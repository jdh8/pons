//! Book-extraction probe over a brl self-play corpus.
//!
//! brl (github.com/harukaki/brl) is a neural bidder playing a *learned,
//! non-human* system; its policy is a pure function of (own hand, auction,
//! vul). `vendor/brl/dump_selfplay.py` records its greedy self-play; this
//! probe asks the feasibility question of the extraction campaign: **is that
//! system compressible into disclosure-vocabulary rules** (HCP bands, suit
//! lengths, balance) — is it "ruly"?
//!
//! Per auction node (prefix of table calls, depth ≤ `--depth`) it:
//! - runs the **paired vul-flip test** (same deal under all four vul combos —
//!   the policy is deterministic, so any argmax change is exact evidence of
//!   vul-conditioning);
//! - fits one axis-aligned box per call — HCP × per-suit lengths at a small
//!   quantile trim chosen on the fit split, plus a `balanced()` conjunct at
//!   ≥99% purity — and resolves overlaps by `Rules` semantics: weight =
//!   ln(bucket share), argmax of (weight | box contains hand), deterministic
//!   tie-break; a hand no box contains is a **miss** (coverage reported);
//! - scores held-out fidelity (split **by deal**, all four vul variants on
//!   one side) as pooled top-1, conditional-on-non-Pass, confident-row
//!   (top-1 prob ≥ τ), plus the **expressiveness ceiling** — the majority
//!   call per exact (hcp, lengths) tuple learned on the fit split — which
//!   separates "fitter too weak" (ceiling ≫ fit) from "genuinely non-ruly"
//!   (ceiling low).
//!
//! Ingest asserts: every auction replays legally and the chosen call matches
//! the dumped top-1. The opening-rate-by-HCP table is informational — brl's
//! is **anti-monotone** (dealer Pass is an informative strong-ish call in its
//! learned system; near-yarboroughs almost always open). That this is genuine
//! and not an encoder scramble was validated by replaying corpus boards
//! through pgx's own PBN parsing and reproducing every dumped call
//! (vendor/brl/PIN.md). The `sketch:` lines are candidate DSL constraints for
//! a later authored book, not proofs.
//!
//! ```text
//! cargo run --example probe-brl-book -- --corpus ~/brl/corpus/brl-selfplay-200k.jsonl
//! ```

#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use anyhow::{Context as _, Result, bail};
use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{Bid, Level, Strain};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{BufRead, BufReader};

/// Call tokens in pgx/pons action order: Pass, X, XX, then 1C..7NT.
const TOKENS: [&str; 38] = [
    "P", "X", "XX", "1C", "1D", "1H", "1S", "1NT", "2C", "2D", "2H", "2S", "2NT", "3C", "3D", "3H",
    "3S", "3NT", "4C", "4D", "4H", "4S", "4NT", "5C", "5D", "5H", "5S", "5NT", "6C", "6D", "6H",
    "6S", "6NT", "7C", "7D", "7H", "7S", "7NT",
];
/// Suit labels in the corpus PBN order (spades first).
const SUITS: [char; 4] = ['S', 'H', 'D', 'C'];
const VULS: [&str; 4] = ["none", "ns", "ew", "both"];

#[derive(Parser)]
#[command(about = "Extract candidate books from a brl self-play corpus and measure their fidelity")]
struct Args {
    /// JSONL corpus from vendor/brl/dump_selfplay.py
    #[arg(long)]
    corpus: String,

    /// Maximum node depth (auction-prefix length) to fit
    #[arg(long, default_value_t = 2)]
    depth: usize,

    /// Minimum rows at a node to fit rules (smaller nodes are only mass-listed)
    #[arg(long, default_value_t = 2000)]
    min_rows: usize,

    /// Paired vul-flip rate above which a node counts as vul-conditioned
    #[arg(long, default_value_t = 0.02)]
    flip_eps: f64,

    /// Top-1 probability threshold for the confident-row fidelity slice
    #[arg(long, default_value_t = 0.9)]
    confident: f64,

    /// How many nodes to list in the mass table
    #[arg(long, default_value_t = 40)]
    mass_top: usize,

    /// Optional output file (default: stdout)
    #[arg(long)]
    out: Option<String>,
}

/// One decision by one actor at one node.
#[derive(Clone, Copy)]
struct Row {
    deal: u32,
    /// Index into [`VULS`] (absolute vulnerability).
    vul: u8,
    hcp: u8,
    /// Suit lengths in [`SUITS`] order (spades first).
    len: [u8; 4],
    /// Token id of the call brl made.
    call: u8,
    /// brl's top-1 probability for that call.
    p1: f32,
    ent: f32,
}

#[derive(Default)]
struct Node {
    mass: u64,
    rows: Vec<Row>,
}

/// One fitted box: `Rules`-style rule with weight = ln(bucket share).
struct RuleBox {
    call: u8,
    weight: f32,
    hcp: (u8, u8),
    /// Per-suit inclusive length bounds, [`SUITS`] order.
    len: [(u8, u8); 4],
    balanced: bool,
}

impl RuleBox {
    fn contains(&self, r: &Row) -> bool {
        (self.hcp.0..=self.hcp.1).contains(&r.hcp)
            && self
                .len
                .iter()
                .zip(&r.len)
                .all(|(&(lo, hi), l)| (lo..=hi).contains(l))
            && (!self.balanced || is_balanced(r.len))
    }

    fn sketch(&self) -> String {
        let mut parts = vec![format!("hcp({}..={})", self.hcp.0, self.hcp.1)];
        for (i, &(lo, hi)) in self.len.iter().enumerate() {
            match (lo, hi) {
                (0, 13) => {}
                (0, _) => parts.push(format!("len({}, ..={hi})", SUITS[i])),
                (_, 13) => parts.push(format!("len({}, {lo}..)", SUITS[i])),
                _ => parts.push(format!("len({}, {lo}..={hi})", SUITS[i])),
            }
        }
        if self.balanced {
            parts.push("balanced()".into());
        }
        parts.join(" & ")
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut nodes: HashMap<String, Node> = HashMap::new();
    let mut boards = 0u64;
    let mut decisions = 0u64;
    // Opening rate by dealer-HCP band — the obs-scramble tripwire.
    let mut open_by_hcp = [(0u64, 0u64); 5];

    let file = std::fs::File::open(&args.corpus)
        .with_context(|| format!("cannot open corpus {}", args.corpus))?;
    for line in BufReader::new(file).lines() {
        let line = line?;
        let v: Value = serde_json::from_str(&line)?;
        let deal = v["deal"].as_u64().context("deal")? as u32;
        let vul_str = v["vul"].as_str().context("vul")?;
        let vul = VULS
            .iter()
            .position(|&s| s == vul_str)
            .context("vul label")? as u8;
        let seats = parse_pbn(v["pbn"].as_str().context("pbn")?)?;
        let calls: Vec<u8> = v["calls"]
            .as_array()
            .context("calls")?
            .iter()
            .map(|c| token_id(c.as_str().unwrap_or_default()))
            .collect::<Result<_>>()?;
        let top3 = v["top3"].as_array().context("top3")?;
        let ent = v["ent"].as_array().context("ent")?;

        // Legality replay + argmax consistency.
        let mut auction = Auction::new();
        for (t, &call) in calls.iter().enumerate() {
            let c = to_call(call);
            if auction.can_push(c).is_err() {
                bail!(
                    "illegal call {} at turn {t} of deal {deal}",
                    TOKENS[call as usize]
                );
            }
            auction.push(c);
            let top = &top3[t].as_array().context("top3 row")?[0];
            let top_tok = top[0].as_str().context("top token")?;
            if top_tok != TOKENS[call as usize] {
                bail!("argmax mismatch at deal {deal} turn {t}: {top_tok} vs chosen");
            }
        }
        decisions += calls.len() as u64;

        let dealer = &seats[0];
        let band = (dealer.0 / 4).min(4) as usize;
        open_by_hcp[band].1 += 1;
        open_by_hcp[band].0 += u64::from(calls[0] != 0);

        for d in 0..=args.depth.min(calls.len() - 1) {
            let key = calls[..d]
                .iter()
                .map(|&c| TOKENS[c as usize])
                .collect::<Vec<_>>()
                .join(" ");
            let node = nodes.entry(key).or_default();
            node.mass += 1;
            let (hcp, len) = seats[d % 4];
            node.rows.push(Row {
                deal,
                vul,
                hcp,
                len,
                call: calls[d],
                p1: top3[d].as_array().context("top3 row")?[0][1]
                    .as_f64()
                    .context("top prob")? as f32,
                ent: ent[d].as_f64().context("ent")? as f32,
            });
        }
        boards += 1;
    }

    let mut report = String::new();
    let _ = writeln!(
        report,
        "# brl book-extraction probe\n\ncorpus: {} ({boards} boards, {decisions} decisions)\n",
        args.corpus
    );

    // Informational: brl's opening rate is genuinely ANTI-monotone in HCP
    // (dealer Pass is a strong-ish call in its learned system) — validated by
    // independent replay through pgx's own PBN parsing, so no monotone assert.
    let _ = writeln!(report, "## Ingest checks\n\nlegality + argmax: ok");
    let rates: Vec<f64> = open_by_hcp
        .iter()
        .map(|&(o, n)| o as f64 / n.max(1) as f64)
        .collect();
    let _ = writeln!(
        report,
        "opening rate by dealer HCP band (0-3, 4-7, 8-11, 12-15, 16+): {}",
        rates
            .iter()
            .map(|r| format!("{:.1}%", 100.0 * r))
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Mass table.
    let mut by_mass: Vec<(&String, &Node)> = nodes.iter().collect();
    by_mass.sort_by_key(|(k, n)| (std::cmp::Reverse(n.mass), (*k).clone()));
    let _ = writeln!(
        report,
        "\n## Node mass (top {}, depth <= {})\n",
        args.mass_top, args.depth
    );
    for (key, node) in by_mass.iter().take(args.mass_top) {
        let label = if key.is_empty() { "(root)" } else { key };
        let _ = writeln!(
            report,
            "- `{label}`: {} rows ({:.2}% of boards)",
            node.mass,
            100.0 * node.mass as f64 / boards as f64
        );
    }

    // Fit every node big enough, in mass order.
    for (key, node) in by_mass
        .iter()
        .filter(|(_, n)| n.rows.len() >= args.min_rows)
    {
        analyze_node(&mut report, &args, key, node);
    }

    if let Some(path) = &args.out {
        std::fs::write(path, &report)?;
        eprintln!("wrote {path}");
    } else {
        print!("{report}");
    }
    Ok(())
}

fn analyze_node(report: &mut String, args: &Args, key: &str, node: &Node) {
    let label = if key.is_empty() { "(root)" } else { key };
    let seat =
        ["N (dealer)", "E (LHO)", "S (partner)", "W (RHO)"][key.split_whitespace().count() % 4];
    let _ = writeln!(
        report,
        "\n## Node `{label}` — {} rows, actor {seat}\n",
        node.rows.len()
    );

    // Paired vul-flip: same deal, different vul, both reaching this node.
    let mut per_deal: HashMap<u32, [Option<u8>; 4]> = HashMap::new();
    for r in &node.rows {
        per_deal.entry(r.deal).or_default()[r.vul as usize] = Some(r.call);
    }
    const PAIRS: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let (mut pairs, mut flips) = (0u64, 0u64);
    let mut per_pair = [(0u64, 0u64); 6];
    for (slot, &(a, b)) in PAIRS.iter().enumerate() {
        for calls in per_deal.values() {
            if let (Some(x), Some(y)) = (calls[a], calls[b]) {
                per_pair[slot].1 += 1;
                per_pair[slot].0 += u64::from(x != y);
                pairs += 1;
                flips += u64::from(x != y);
            }
        }
    }
    let flip_rate = flips as f64 / pairs.max(1) as f64;
    let mut pair_desc = Vec::new();
    for (&(a, b), &(f, n)) in PAIRS.iter().zip(&per_pair) {
        if n > 0 {
            pair_desc.push(format!(
                "{}~{} {:.1}%",
                VULS[a],
                VULS[b],
                100.0 * f as f64 / n as f64
            ));
        }
    }
    let _ = writeln!(
        report,
        "vul-flip: {:.2}% over {pairs} pairs ({}) => {}",
        100.0 * flip_rate,
        pair_desc.join(", "),
        if flip_rate < args.flip_eps {
            "merge vul strata"
        } else {
            "VUL-CONDITIONED"
        }
    );

    // Split by deal: all four vul variants of a deal land on one side.
    let fit: Vec<Row> = node
        .rows
        .iter()
        .filter(|r| r.deal % 4 != 0)
        .copied()
        .collect();
    let hold: Vec<Row> = node
        .rows
        .iter()
        .filter(|r| r.deal % 4 == 0)
        .copied()
        .collect();

    let (boxes, q) = fit_boxes(&fit);
    let m = Metrics::score(&boxes, &fit, &hold, args.confident);
    let _ = writeln!(
        report,
        "fit: q={q} | holdout fidelity: pooled {:.1}%, non-Pass {:.1}%, confident(p>={}) {:.1}%, coverage {:.1}%",
        100.0 * m.pooled,
        100.0 * m.non_pass,
        args.confident,
        100.0 * m.confident,
        100.0 * m.coverage,
    );
    let (ceil, ceil_cov) = ceiling(&fit, &hold, false);
    let (ceil_v, ceil_v_cov) = ceiling(&fit, &hold, true);
    let _ = writeln!(
        report,
        "ceiling (exact-tuple majority): {:.1}% among the {:.1}% of holdout with a seen tuple; vul-in-tuple {:.1}% among {:.1}% | self-fidelity on fit split: {:.1}%",
        100.0 * ceil,
        100.0 * ceil_cov,
        100.0 * ceil_v,
        100.0 * ceil_v_cov,
        100.0 * m.self_fid,
    );

    // A vul-conditioned node wants a per-stratum book: refit inside each vul
    // combo (split by deal within the stratum) and report what that buys.
    if flip_rate >= args.flip_eps {
        let mut parts = Vec::new();
        let mut hits = 0.0;
        let mut total = 0usize;
        for (i, name) in VULS.iter().enumerate() {
            let sf: Vec<Row> = fit.iter().filter(|r| r.vul == i as u8).copied().collect();
            let sh: Vec<Row> = hold.iter().filter(|r| r.vul == i as u8).copied().collect();
            if sf.len() < 500 || sh.is_empty() {
                parts.push(format!("{name} (thin)"));
                continue;
            }
            let (b, _) = fit_boxes(&sf);
            let sm = Metrics::score(&b, &sf, &sh, args.confident);
            hits += sm.pooled * sh.len() as f64;
            total += sh.len();
            parts.push(format!("{name} {:.1}%", 100.0 * sm.pooled));
        }
        if total > 0 {
            let _ = writeln!(
                report,
                "stratified holdout fidelity: pooled {:.1}% ({})",
                100.0 * hits / total as f64,
                parts.join(", ")
            );
        }
    }
    let mut ents: Vec<f32> = node.rows.iter().map(|r| r.ent).collect();
    ents.sort_by(f32::total_cmp);
    let _ = writeln!(
        report,
        "entropy: median {:.2}, p90 {:.2} nats\n",
        ents[ents.len() / 2],
        ents[ents.len() * 9 / 10],
    );

    // Per-call table, share-descending.
    let mut shares: HashMap<u8, usize> = HashMap::new();
    for r in &node.rows {
        *shares.entry(r.call).or_default() += 1;
    }
    let mut order: Vec<(u8, usize)> = shares.into_iter().collect();
    order.sort_by_key(|&(c, n)| (std::cmp::Reverse(n), c));
    for (call, n) in order.iter().take(12) {
        let share = *n as f64 / node.rows.len() as f64;
        let sketch = boxes
            .iter()
            .find(|b| b.call == *call)
            .map_or("(below min share)".to_string(), RuleBox::sketch);
        let (mut hit, mut of) = (0u64, 0u64);
        for r in &hold {
            if r.call == *call {
                of += 1;
                hit += u64::from(classify(&boxes, r) == Some(*call));
            }
        }
        let _ = writeln!(
            report,
            "- **{}** {:.1}% (n={n}) recall {:.1}%: `{sketch}`",
            TOKENS[*call as usize],
            100.0 * share,
            100.0 * hit as f64 / of.max(1) as f64,
        );
    }
}

/// Fit one box per call from the fit split; pick the trim quantile on fit
/// agreement. ponytail: quantile boxes + q grid, no coordinate refinement —
/// escalate only if the ceiling says the fitter is the bottleneck.
fn fit_boxes(fit: &[Row]) -> (Vec<RuleBox>, f64) {
    let mut best: Option<(Vec<RuleBox>, f64, f64)> = None;
    for q in [0.005, 0.02, 0.05] {
        let boxes = quantile_boxes(fit, q);
        let agree = agreement(&boxes, fit);
        if best.as_ref().is_none_or(|(_, _, a)| agree > *a) {
            best = Some((boxes, q, agree));
        }
    }
    let (boxes, q, _) = best.expect("q grid is non-empty");
    (boxes, q)
}

fn quantile_boxes(fit: &[Row], q: f64) -> Vec<RuleBox> {
    let mut buckets: HashMap<u8, Vec<&Row>> = HashMap::new();
    for r in fit {
        buckets.entry(r.call).or_default().push(r);
    }
    let mut boxes: Vec<RuleBox> = buckets
        .iter()
        .filter(|(_, rows)| rows.len() >= 20) // below this, quantiles are noise
        .map(|(&call, rows)| {
            let grab = |f: &dyn Fn(&Row) -> u8| {
                let mut v: Vec<u8> = rows.iter().map(|r| f(r)).collect();
                v.sort_unstable();
                (pct(&v, q), pct(&v, 1.0 - q))
            };
            let balanced = rows.iter().filter(|r| is_balanced(r.len)).count() as f64
                >= 0.99 * rows.len() as f64;
            RuleBox {
                call,
                weight: (rows.len() as f32 / fit.len() as f32).ln(),
                hcp: grab(&|r| r.hcp),
                len: [0, 1, 2, 3].map(|i| grab(&|r| r.len[i])),
                balanced,
            }
        })
        .collect();
    // Deterministic max-weight resolution: heaviest first, call id breaks ties.
    boxes.sort_by(|a, b| b.weight.total_cmp(&a.weight).then(a.call.cmp(&b.call)));
    boxes
}

/// `Rules` semantics: the heaviest box containing the hand wins; none = abstain.
fn classify(boxes: &[RuleBox], r: &Row) -> Option<u8> {
    boxes.iter().find(|b| b.contains(r)).map(|b| b.call)
}

fn agreement(boxes: &[RuleBox], rows: &[Row]) -> f64 {
    let hits = rows
        .iter()
        .filter(|r| classify(boxes, r) == Some(r.call))
        .count();
    hits as f64 / rows.len().max(1) as f64
}

struct Metrics {
    pooled: f64,
    non_pass: f64,
    confident: f64,
    coverage: f64,
    self_fid: f64,
}

impl Metrics {
    fn score(boxes: &[RuleBox], fit: &[Row], hold: &[Row], tau: f64) -> Self {
        let (mut hit, mut np_hit, mut np, mut conf_hit, mut conf, mut covered) =
            (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
        for r in hold {
            let got = classify(boxes, r);
            covered += u64::from(got.is_some());
            let ok = got == Some(r.call);
            hit += u64::from(ok);
            if r.call != 0 {
                np += 1;
                np_hit += u64::from(ok);
            }
            if f64::from(r.p1) >= tau {
                conf += 1;
                conf_hit += u64::from(ok);
            }
        }
        let n = hold.len().max(1) as f64;
        Metrics {
            pooled: hit as f64 / n,
            non_pass: np_hit as f64 / np.max(1) as f64,
            confident: conf_hit as f64 / conf.max(1) as f64,
            coverage: covered as f64 / n,
            self_fid: agreement(boxes, fit),
        }
    }
}

/// Expressiveness ceiling: majority call per exact (hcp, lengths[, vul])
/// tuple, learned on the fit split, scored on the holdout rows whose tuple
/// was seen (their share is returned as the second value — the number is
/// meaningless without it on thin nodes). An upper bound for ANY rule over
/// these features; biased high on thin tuples. `with_vul` adds the vul
/// stratum to the tuple — the honest ceiling for a vul-conditioned node,
/// at the price of ~4x thinner tuples.
fn ceiling(fit: &[Row], hold: &[Row], with_vul: bool) -> (f64, f64) {
    let key = |r: &Row| (if with_vul { r.vul } else { 0 }, r.hcp, r.len);
    let mut table: HashMap<(u8, u8, [u8; 4]), HashMap<u8, u32>> = HashMap::new();
    for r in fit {
        *table.entry(key(r)).or_default().entry(r.call).or_default() += 1;
    }
    let majority: HashMap<(u8, u8, [u8; 4]), u8> = table
        .into_iter()
        .map(|(k, votes)| {
            let call = votes
                .into_iter()
                .max_by_key(|&(c, n)| (n, std::cmp::Reverse(c)))
                .expect("non-empty votes")
                .0;
            (k, call)
        })
        .collect();
    let (mut hits, mut covered) = (0u64, 0u64);
    for r in hold {
        if let Some(&call) = majority.get(&key(r)) {
            covered += 1;
            hits += u64::from(call == r.call);
        }
    }
    (
        hits as f64 / covered.max(1) as f64,
        covered as f64 / hold.len().max(1) as f64,
    )
}

/// Balanced: every suit >= 2 and at most one doubleton (matches `features::is_balanced`).
fn is_balanced(len: [u8; 4]) -> bool {
    len.iter().all(|&l| l >= 2) && len.iter().filter(|&&l| l == 2).count() <= 1
}

/// Value at quantile `q` of a pre-sorted slice (nearest-rank).
fn pct(sorted: &[u8], q: f64) -> u8 {
    let idx = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Parse "N:S.H.D.C E S W" into per-seat (hcp, suit lengths), NESW order.
fn parse_pbn(pbn: &str) -> Result<[(u8, [u8; 4]); 4]> {
    let body = pbn.strip_prefix("N:").context("pbn must start with N:")?;
    let mut seats = [(0u8, [0u8; 4]); 4];
    for (seat, hand) in body.split_whitespace().enumerate() {
        if seat >= 4 {
            bail!("more than four hands in {pbn:?}");
        }
        let mut hcp = 0u8;
        let mut len = [0u8; 4];
        for (suit, cards) in hand.split('.').enumerate() {
            if suit >= 4 {
                bail!("more than four suits in {hand:?}");
            }
            len[suit] = cards.len() as u8;
            for c in cards.chars() {
                hcp += match c {
                    'A' => 4,
                    'K' => 3,
                    'Q' => 2,
                    'J' => 1,
                    _ => 0,
                };
            }
        }
        seats[seat] = (hcp, len);
    }
    Ok(seats)
}

fn token_id(tok: &str) -> Result<u8> {
    TOKENS
        .iter()
        .position(|&t| t == tok)
        .map(|i| i as u8)
        .with_context(|| format!("unknown call token {tok:?}"))
}

fn to_call(id: u8) -> Call {
    match id {
        0 => Call::Pass,
        1 => Call::Double,
        2 => Call::Redouble,
        _ => {
            let b = id - 3;
            Call::Bid(Bid {
                level: Level::new(b / 5 + 1),
                strain: [
                    Strain::Clubs,
                    Strain::Diamonds,
                    Strain::Hearts,
                    Strain::Spades,
                    Strain::Notrump,
                ][usize::from(b % 5)],
            })
        }
    }
}
