//! Probe the distilled neural floor: for a given auction, what hands does it
//! assign to each call?
//!
//! The net's output is a function of `(actor hand, auction context)` *only* — it
//! never sees the other three hands. So we characterise its judgement by
//! **sample-and-probe**: hold the auction fixed, deal random actor hands
//! (consistent with what the actor has already shown), run the *real* bidder
//! ([`american_neural_search`] and friends — legality mask and forced rails
//! included), bucket each hand by the call it produces, and summarise every
//! bucket in the DSL's own vocabulary (HCP / suit length / balanced).
//!
//! Because the auction is whatever you pass, competitive sequences work the same
//! as constructive ones — `1♦ 1♠` just means the opponents overcalled, and the
//! [`Context`] / [`Inferences`] already encode that. Off-book (the whole
//! competitive zone) the bidder *is* the trained net, so its buckets are the
//! net's conditions.
//!
//! The `sketch:` line per call is a *candidate* constraint to verify with
//! `bidding::verify` and hand-author, not a proof of the net's internal logic.
//!
//! ```text
//! cargo run --release --features search --example probe-extract-constraints -- \
//!   --auction "1♦ 1♠" --samples 40000
//! ```

#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use anyhow::{Context as _, Result, bail};
use clap::Parser;
use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::deck::fill_deals;
use contract_bridge::eval::{self, HandEvaluator, SimpleEvaluator};
use contract_bridge::{Bid, Builder, Hand, Level, Seat, Strain, Suit};
use pons::bidding::array::Logits;
use pons::bidding::context::Context;
use pons::bidding::inference::{Inferences, Range};
use pons::bidding::{Family, System};
use pons::{Pair, american_neural, american_neural_search, american_neural_v2};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::HashMap;
use std::fmt::Write as _;

#[derive(Parser)]
#[command(about = "Probe the neural floor for the hand conditions behind each call")]
struct Args {
    /// Auction prefix in table-call format; the actor is next to act.
    /// Empty (the default) probes a first-seat opening.  Example: "1♦ 1♠".
    #[arg(long, default_value = "")]
    auction: String,

    /// Which distilled net to probe: search (champion) | v2 (tag-augmented) | neural (v1)
    #[arg(long, default_value = "search")]
    net: String,

    /// Vulnerability relative to the actor: none | we | they | both
    #[arg(long, default_value = "none")]
    vul: String,

    /// Number of probe hands to keep (after the actor-shape filter)
    #[arg(long, default_value_t = 20_000)]
    samples: usize,

    /// Two-sided percentile trim for the reported HCP range (0.05 = 5th–95th)
    #[arg(long, default_value_t = 0.05)]
    trim: f64,

    /// Skip calls chosen on fewer than this fraction of probe hands
    #[arg(long, default_value_t = 0.01)]
    min_share: f64,

    /// RNG seed (fixed by default, so runs are reproducible)
    #[arg(long, default_value_t = 0)]
    seed: u64,

    /// Optional output file (default: stdout)
    #[arg(long)]
    out: Option<String>,
}

/// Per-call accumulator: every probe hand the net mapped to this call.
#[derive(Default)]
struct Bucket {
    hcp: Vec<u8>,
    /// Suit lengths, indexed in [`Suit::ASC`] order (♣ ♦ ♥ ♠).
    len: [Vec<u8>; 4],
    balanced: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if !(0.0..0.5).contains(&args.trim) {
        bail!("--trim must be in [0.0, 0.5)");
    }

    let auction = parse_auction(&args.auction)?;
    let vul = parse_vul(&args.vul)?;
    let stance = pick_net(&args.net)?.against(Family::NATURAL);

    let context = Context::new(vul, &auction);
    let inferences = Inferences::read(&context);
    let actor_shape = inferences.me().lengths; // [Range; 4], ♣ ♦ ♥ ♠

    // The net only depends on the actor's hand + the fixed auction, so we deal
    // random hands and keep one seat's — filtered to the actor's own shown
    // shape so we don't probe with hands that contradict its prior bids.
    let mut rng = StdRng::seed_from_u64(args.seed);
    let empty = Builder::new()
        .build_partial()
        .expect("an empty builder is a valid (all-unknown) partial deal");

    // The prefix auction is the same for every probe hand; build it once for
    // the legality re-check in `best_legal`.
    let mut prefix = Auction::new();
    for &call in &auction {
        prefix.push(call);
    }

    // ponytail: rejection budget for the shape filter; ample for the loose
    // ranges Inferences produces. A no-op when the actor hasn't bid yet.
    let budget = args.samples.saturating_mul(64).max(args.samples);
    let mut buckets: HashMap<Call, Bucket> = HashMap::new();
    let mut probed = 0usize;

    for deal in fill_deals(&mut rng, empty).take(budget) {
        let hand = deal[Seat::North];
        if !shape_ok(hand, &actor_shape) {
            continue;
        }
        let Some(logits) = stance.classify(hand, vul, &auction) else {
            continue;
        };
        let Some(call) = best_legal(&logits, &prefix) else {
            continue;
        };

        let lengths = Suit::ASC.map(|suit| hand[suit].len() as u8);
        let entry = buckets.entry(call).or_default();
        entry.hcp.push(hcp(hand));
        for (slot, &l) in entry.len.iter_mut().zip(&lengths) {
            slot.push(l);
        }
        if is_balanced(lengths) {
            entry.balanced += 1;
        }

        probed += 1;
        if probed == args.samples {
            break;
        }
    }

    if probed == 0 {
        bail!("no probe hands survived the actor-shape filter — auction may be jointly infeasible");
    }

    let report = render(&args, &auction, probed, &mut buckets);
    if let Some(path) = &args.out {
        std::fs::write(path, &report)?;
        eprintln!("wrote {path}");
    } else {
        print!("{report}");
    }
    Ok(())
}

/// Build the per-call report.  Takes `&mut` only to sort each bucket in place.
fn render(
    args: &Args,
    auction: &[Call],
    probed: usize,
    buckets: &mut HashMap<Call, Bucket>,
) -> String {
    let mut report = String::new();
    let _ = writeln!(
        report,
        "# Neural floor probe\n\nnet: {}\nauction: {}\nvul: {}\nprobe hands: {probed}\n",
        args.net,
        if auction.is_empty() {
            "(opening seat)".to_string()
        } else {
            format_auction(auction)
        },
        args.vul,
    );
    let _ = writeln!(
        report,
        "Each call's hands summarised in DSL vocabulary; `sketch` is a *candidate* to verify, not a proof.\n"
    );
    // Order calls by how often the net chose them.
    let mut by_share: Vec<(&Call, &Bucket)> = buckets.iter().collect();
    by_share.sort_by_key(|(_, b)| std::cmp::Reverse(b.hcp.len()));

    for (call, bucket) in by_share {
        let n = bucket.hcp.len();
        let share = n as f64 / probed as f64;
        if share < args.min_share {
            continue;
        }

        let mut hcp = bucket.hcp.clone();
        hcp.sort_unstable();
        let hcp_lo = pct(&hcp, args.trim);
        let hcp_hi = pct(&hcp, 1.0 - args.trim);

        let _ = writeln!(report, "## {call}   (chosen {:.1}%, n={n})", 100.0 * share);
        let _ = writeln!(
            report,
            "- hcp: {hcp_lo}–{hcp_hi} (median {})",
            pct(&hcp, 0.5)
        );

        // One band line per suit, and collect the salient length clauses.
        let mut clauses = vec![format!("hcp({hcp_lo}..={hcp_hi})")];
        for (i, suit) in Suit::ASC.into_iter().enumerate() {
            let mut col = bucket.len[i].clone();
            col.sort_unstable();
            let (lo, mid, hi) = (pct(&col, 0.10), pct(&col, 0.5), pct(&col, 0.90));
            let _ = writeln!(report, "- {suit:?}: {lo}–{hi} (median {mid})");
            // A real length requirement: even the short tail holds 4+.
            if lo >= 4 {
                clauses.push(format!("len({suit:?}, {lo}..)"));
            }
        }

        let bal = bucket.balanced as f64 / n as f64;
        let _ = writeln!(report, "- balanced: {:.0}%", 100.0 * bal);
        if bal > 0.8 {
            clauses.push("balanced()".to_string());
        }

        let _ = writeln!(report, "- sketch: {}\n", clauses.join(" & "));
    }

    // Cheap durable invariants: the smallest things that fail if the wiring
    // breaks (HCP escapes its range, or shares don't add up).
    let total: usize = buckets.values().map(|b| b.hcp.len()).sum();
    assert_eq!(total, probed, "bucket sizes must partition the probe hands");
    assert!(
        buckets.values().flat_map(|b| &b.hcp).all(|&h| h <= 37),
        "HCP out of range — feature/eval wiring is wrong"
    );
    report
}

/// Highest-logit legal call, mirroring `Table::next_call`.  The floor masks
/// illegal calls to `-∞`, so finite logits are already legal; the `can_push`
/// re-check guards on-book nodes that may not mask.
fn best_legal(logits: &Logits, prefix: &Auction) -> Option<Call> {
    let mut scored: Vec<(Call, f32)> = logits
        .iter()
        .map(|(call, &logit)| (call, logit))
        .filter(|&(_, logit)| logit.is_finite())
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
    scored
        .into_iter()
        .map(|(call, _)| call)
        .find(|&call| prefix.can_push(call).is_ok())
}

/// HCP via the same evaluator [`features`] uses, so the numbers match the net's.
fn hcp(hand: Hand) -> u8 {
    SimpleEvaluator(eval::hcp::<u8>).eval(hand)
}

/// Balanced: every suit ≥ 2 and at most one doubleton (matches `features::is_balanced`).
fn is_balanced(len: [u8; 4]) -> bool {
    len.iter().all(|&l| l >= 2) && len.iter().filter(|&&l| l == 2).count() <= 1
}

/// Whether each suit length falls inside the actor's shown ranges.
fn shape_ok(hand: Hand, shape: &[Range; 4]) -> bool {
    Suit::ASC.into_iter().enumerate().all(|(i, suit)| {
        let l = hand[suit].len() as u8;
        shape[i].min <= l && l <= shape[i].max
    })
}

/// Value at quantile `q` of a pre-sorted slice (nearest-rank).
fn pct(sorted: &[u8], q: f64) -> u8 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn pick_net(name: &str) -> Result<Pair> {
    Ok(match name {
        "search" => american_neural_search(),
        "v2" => american_neural_v2(),
        "neural" | "v1" => american_neural(),
        other => bail!("--net must be search|v2|neural, got {other:?}"),
    })
}

fn parse_vul(s: &str) -> Result<RelativeVulnerability> {
    let mut v = RelativeVulnerability::NONE;
    match s {
        "none" => {}
        "we" => v.set(RelativeVulnerability::WE, true),
        "they" => v.set(RelativeVulnerability::THEY, true),
        "both" => {
            v.set(RelativeVulnerability::WE, true);
            v.set(RelativeVulnerability::THEY, true);
        }
        other => bail!("--vul must be none|we|they|both, got {other:?}"),
    }
    Ok(v)
}

fn parse_auction(text: &str) -> Result<Vec<Call>> {
    text.split_whitespace().map(parse_call_token).collect()
}

fn parse_call_token(token: &str) -> Result<Call> {
    let t = token.trim();
    let upper = t.to_ascii_uppercase();
    match upper.as_str() {
        "P" | "PASS" => return Ok(Call::Pass),
        "X" | "D" | "DBL" | "DOUBLE" => return Ok(Call::Double),
        "XX" | "R" | "RDBL" | "REDOUBLE" => return Ok(Call::Redouble),
        _ => {}
    }

    let mut chars = t.chars();
    let level_ch = chars
        .next()
        .with_context(|| format!("empty call token: {t:?}"))?;
    let level_digit = level_ch
        .to_digit(10)
        .with_context(|| format!("bad bid level: {t:?}"))?;
    if !(1..=7).contains(&level_digit) {
        bail!("bid level out of range: {t}");
    }
    let strain = parse_strain(chars.as_str().trim())?;
    Ok(Call::Bid(Bid {
        level: Level::new(level_digit as u8),
        strain,
    }))
}

fn parse_strain(tok: &str) -> Result<Strain> {
    Ok(match tok.to_ascii_uppercase().as_str() {
        "C" | "♣" => Strain::Clubs,
        "D" | "♦" => Strain::Diamonds,
        "H" | "♥" => Strain::Hearts,
        "S" | "♠" => Strain::Spades,
        "N" | "NT" => Strain::Notrump,
        other => bail!("unknown strain: {other:?}"),
    })
}

fn format_auction(calls: &[Call]) -> String {
    calls
        .iter()
        .map(|c| format!("{c}"))
        .collect::<Vec<_>>()
        .join(" ")
}
