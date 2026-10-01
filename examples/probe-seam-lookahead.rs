//! Does a rollout lookahead beat the book at one invitation seam, on the
//! **true** deals?  Phase 3b of `docs/exact-posterior.md`, with a *sampled*
//! forecast — the ceiling of any counted one.
//!
//! `probe-rollout-label` (`docs/ai-bidder/logit-calibration.md` §4) already
//! prices rollouts at authored nodes, but against held-out *sampled* layouts and
//! with the net's top-k as candidates.  This probe asks the narrower question
//! Phase 3 turns on:
//!
//! 1. **The seam** (`--seam`, default opener's turn after `1M - 2M -`): a
//!    self-play `american()` walk keeps every deal that reaches it.  Dealer
//!    rotates per deal and vulnerability alternates none/both every four.
//! 2. **The candidates** are the book's own rungs there: the call it made, plus
//!    every call the node made on at least `--min-share` of its decisions.
//! 3. **Two forecasts**, `--layouts` worlds each, both bid out to pass-out by the
//!    real bidder at all four seats and scored double dummy:
//!    - `replay` — worlds on which our own bidder reproduces the auction
//!      (`sample_layouts_replay`): the self-play posterior, hence the ceiling
//!      of *any* lookahead;
//!    - `range` — worlds inside the readings (`sample_layouts`): the ceiling of
//!      a forecast *counted* from the readings.
//! 4. **The selector** deviates from the book's call when a candidate's mean
//!    swing clears `--margin` IMPs — under plain DD, perfect defense, or both
//!    (the same candidate best on each; the pre-registered rule).
//! 5. **The verdict** is read on the true deal: the chosen call's real
//!    continuation against the book's, paired, both scorers.  The true deal is
//!    independent of the sampled worlds, so no winner's curse survives.
//!
//! Two controls ride along.  The **static** table prices "always switch `own`
//! to `alt`" on the true deals — a win there is a mistuned threshold, which
//! needs no lookahead.  The **oracle** row is the hindsight best candidate per
//! deal, the scale no selector can reach.
//!
//! ```sh
//! cargo run --release --example probe-seam-lookahead -- -c 20000 -m 0   # census only
//! cargo run --release --example probe-seam-lookahead -- -c 600000 -s 1790874659 --slam-try --trace 4NT
//! scripts/idle-run.sh cargo run --release --example probe-seam-lookahead -- -c 600000 -s $SEED
//! ```
//!
//! `--trace CALL` skips the pricing and instead bids every rung out on the
//! true deal for the decisions whose book call is `CALL`, tallying the final
//! contracts — how the `4NT` rung over a single raise was found wanting.  The
//! Phase 3b numbers and that trace were taken on the book **before**
//! `response.major_raise_slam_try` shipped; pass `--slam-try` to walk that
//! book.
//!
//! Heavy: one double-dummy solve per sampled world (2 × `--layouts` + 1 per
//! decision, ≈ 7 ms each).

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::agreements::Agreements;
use pons::bidding::context::relative;
use pons::bidding::sampler::sample_layouts;
use pons::bidding::table::select_legal_call;
use pons::bidding::{Bidder, Table};
use pons::scoring::{final_contract, imps, ns_score_bid, ns_score_contract};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;
use std::collections::BTreeMap;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::rollout::{sample_for, swings};
use common::{auction_key, mean_with_ci, seat_to_act, seeded_deals};

const BRACKETS: [&str; 2] = ["plain DD", "perfect defense"];
const ARMS: [&str; 2] = ["range", "replay"];
const RULES: [&str; 3] = ["plain", "PD", "both"];
const VULS: [AbsoluteVulnerability; 2] = [AbsoluteVulnerability::NONE, AbsoluteVulnerability::ALL];
const MARGINS: [f64; 5] = [0.0, 0.25, 0.5, 1.0, 2.0];

#[derive(Parser)]
struct Args {
    /// Deals to walk
    #[arg(short, long, default_value_t = 20_000)]
    count: usize,
    /// Seed base; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,
    /// Worlds per forecast arm; 0 prints the seam census and stops
    #[arg(short = 'm', long, default_value_t = 64)]
    layouts: usize,
    /// Seam keys as `render-book` prints them (leading passes stripped)
    #[arg(long, default_values_t = ["1♠ - 2♠ -".to_owned(), "1♥ - 2♥ -".to_owned()])]
    seam: Vec<String>,
    /// A call is a candidate rung when the node made it this often
    #[arg(long, default_value_t = 0.01)]
    min_share: f64,
    /// The pre-registered deviation margin in IMPs (the grid prints regardless)
    #[arg(long, default_value_t = 0.25)]
    margin: f64,
    /// Trace instead of pricing: for every decision whose book call is this
    /// (e.g. `4NT`), bid each rung out on the true deal and tally where it ends
    #[arg(long)]
    trace: Option<String>,
    /// Walk the book as it stood before `response.major_raise_slam_try`
    /// shipped (every 22+ hand asks `4NT`) — the system Phase 3b measured.
    /// With `--trace` this also adds a `knob` row: the whole continuation bid
    /// by today's default
    #[arg(long, default_value_t = false)]
    slam_try: bool,
    /// Traced boards to print in full
    #[arg(long, default_value_t = 10)]
    show: usize,
}

struct Decision {
    key: String,
    hand: Hand,
    seat: Seat,
    dealer: Seat,
    vul: usize,
    prefix: Vec<Call>,
    own: Call,
    truth: FullDeal,
}

/// One priced decision: candidates (own call first), each arm's mean swing per
/// candidate and bracket on its sampled worlds, and the swing on the true deal
struct Priced {
    decision: usize,
    candidates: Vec<Call>,
    forecast: [Vec<[f64; 2]>; 2],
    truth: Vec<[i64; 2]>,
}

fn harvest(
    deal: &FullDeal,
    index: usize,
    policy: &pons::bidding::Partnership,
    seams: &[String],
) -> Option<Decision> {
    let dealer = Seat::ALL[index % 4];
    let vul = (index / 4) % 2;
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let book = policy
            .classify_with_provenance(deal[seat], relative(VULS[vul], seat), &auction)
            .map(|(book, _)| book);
        let own = select_legal_call(book, &auction);
        let key = auction_key(&auction);
        if seams.contains(&key) {
            return Some(Decision {
                key,
                hand: deal[seat],
                seat,
                dealer,
                vul,
                prefix: auction.iter().copied().collect(),
                own,
                truth: *deal,
            });
        }
        auction.push(own);
    }
    None
}

/// The candidate the selector plays (0 = the book's own call)
fn pick(forecast: &[[f64; 2]], rule: usize, margin: f64) -> usize {
    let best = |bracket: usize| {
        (1..forecast.len()).fold(0, |best, c| {
            if forecast[c][bracket] > forecast[best][bracket] {
                c
            } else {
                best
            }
        })
    };
    let clears = |c: usize, bracket: usize| forecast[c][bracket] > margin;
    match rule {
        0 | 1 => Some(best(rule)).filter(|&c| clears(c, rule)),
        _ => Some(best(0)).filter(|&c| c == best(1) && clears(c, 0) && clears(c, 1)),
    }
    .unwrap_or(0)
}

fn show(values: &[i64]) -> String {
    let (mean, ci) = mean_with_ci(values);
    format!("{mean:+.4} ± {ci:.4}")
}

/// True-deal gain per decision of one selector, per bracket, and its fire count
fn verdict(rows: &[&Priced], arm: usize, rule: usize, margin: f64) -> (usize, [Vec<i64>; 2]) {
    let mut gains = [Vec::new(), Vec::new()];
    let mut fired = 0;
    for row in rows {
        let c = pick(&row.forecast[arm], rule, margin);
        fired += usize::from(c != 0);
        for (bracket, gain) in gains.iter_mut().enumerate() {
            gain.push(row.truth[c][bracket]);
        }
    }
    (fired, gains)
}

/// Where each rung ends on the true deal, for the decisions whose book call
/// displays as `own`: final contract, how often it makes, and its mean swing
/// against the book's call.  The first `show` boards print in full.
#[allow(clippy::cast_precision_loss)]
fn trace(
    decisions: &[Decision],
    rungs: &BTreeMap<&str, Vec<Call>>,
    policy: &pons::bidding::Partnership,
    knob: Option<&pons::bidding::Partnership>,
    own: &str,
    show: usize,
) {
    let rows: Vec<&Decision> = decisions
        .iter()
        .filter(|d| d.own.to_string() == own)
        .collect();
    let truths: Vec<FullDeal> = rows.iter().map(|d| d.truth).collect();
    let tables = Solver::lock(None).solve_deals(&truths, NonEmptyStrainFlags::ALL);
    // (seam, rung) -> final contract -> [count, made, plain IMPs, PD IMPs]
    let mut tally: BTreeMap<(String, String), BTreeMap<String, [i64; 4]>> = BTreeMap::new();
    for (n, (d, tricks)) in rows.iter().zip(&tables).enumerate() {
        let bidder: &dyn Bidder = policy;
        let table = Table::new(bidder, bidder, d.dealer, VULS[d.vul]);
        let sign = if matches!(d.seat, Seat::North | Seat::South) {
            1
        } else {
            -1
        };
        let knob_table = knob.map(|k| {
            let k: &dyn Bidder = k;
            Table::new(k, k, d.dealer, VULS[d.vul])
        });
        // `None` lets the knob system pick its own call at the seam.
        let reach = |call: Option<Call>| {
            let mut seed = Auction::new();
            seed.try_extend(d.prefix.iter().copied())
                .expect("the walk's prefix is legal");
            let auction = match (call, &knob_table) {
                (Some(call), _) => {
                    seed.try_push(call).expect("a rung is legal at its seam");
                    table.bid_out_from(&d.truth, seed)
                }
                (None, Some(knob)) => knob.bid_out_from(&d.truth, seed),
                (None, None) => unreachable!("the knob row needs --slam-try"),
            };
            let reached = final_contract(&auction, d.dealer);
            let scores = [
                sign * ns_score_contract(reached, tricks, VULS[d.vul]),
                sign * ns_score_bid(reached, tricks, VULS[d.vul]),
            ];
            (auction, reached, scores)
        };
        let (_, _, base) = reach(Some(d.own));
        if n < show {
            let partner = Seat::ALL[(d.seat as usize + 2) % 4];
            println!(
                "\n#{n} {} vul {}: opener {}  partner {}",
                d.key,
                ["none", "both"][d.vul],
                d.hand,
                d.truth[partner]
            );
        }
        let rows = rungs[d.key.as_str()]
            .iter()
            .map(|&call| (call.to_string(), Some(call)))
            .chain(knob.map(|_| ("knob".to_owned(), None)));
        for (call, rung) in rows {
            let (auction, reached, scores) = reach(rung);
            let (label, took, made) =
                reached.map_or(("passed out".to_owned(), 0, true), |(c, by)| {
                    let took = u8::from(tricks[c.bid.strain].get(by));
                    (
                        Call::Bid(c.bid).to_string(),
                        took,
                        took >= 6 + c.bid.level.get(),
                    )
                });
            let swing = [imps(scores[0] - base[0]), imps(scores[1] - base[1])];
            // Whether opener holds a second suit to make a long-suit try in.
            let side = if Suit::ASC.iter().filter(|&&s| d.hand[s].len() >= 4).count() > 1 {
                "4+ side suit"
            } else {
                "one-suited"
            };
            let cell = tally
                .entry((format!("{} [{side}]", d.key), call.clone()))
                .or_default()
                .entry(label.clone())
                .or_default();
            for (sum, add) in cell
                .iter_mut()
                .zip([1, i64::from(made), swing[0], swing[1]])
            {
                *sum += add;
            }
            if n < show {
                println!(
                    "  {call:<4} {:<44} {label} takes {took:2}  {:+5}  swing {:+3} / {:+3}",
                    auction_key(&auction),
                    scores[0],
                    swing[0],
                    swing[1]
                );
            }
        }
    }
    println!(
        "\n{} decisions with book call {own}; per rung: contract ×n, made, mean swing plain / PD",
        rows.len()
    );
    for ((key, call), contracts) in &tally {
        let total: [i64; 4] = contracts.values().fold([0; 4], |mut sum, cell| {
            for (s, c) in sum.iter_mut().zip(cell) {
                *s += c;
            }
            sum
        });
        println!(
            "{key}  {call}: {:+.3} / {:+.3}",
            total[2] as f64 / total[0] as f64,
            total[3] as f64 / total[0] as f64
        );
        for (contract, [n, made, plain, pd]) in contracts {
            println!(
                "    {contract:<10} ×{n:<4} made {:5.1}%  {:+.2} / {:+.2}",
                100.0 * *made as f64 / *n as f64,
                *plain as f64 / *n as f64,
                *pd as f64 / *n as f64
            );
        }
    }
}

#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
fn main() {
    let args = Args::parse();
    let base = args.seed.unwrap_or_else(rand::random);
    let mut agreements = Agreements::default();
    agreements.response.major_raise_slam_try &= !args.slam_try;
    let policy = american(&agreements).bind();
    let started = std::time::Instant::now();

    let decisions: Vec<Decision> = seeded_deals(base, args.count)
        .par_iter()
        .enumerate()
        .filter_map(|(index, deal)| harvest(deal, index, &policy, &args.seam))
        .collect();

    // The node's own rungs: every call it made often enough, per key.
    let mut made: BTreeMap<&str, Vec<(Call, usize)>> = BTreeMap::new();
    for decision in &decisions {
        let calls = made.entry(&decision.key).or_default();
        match calls.iter_mut().find(|(call, _)| *call == decision.own) {
            Some((_, n)) => *n += 1,
            None => calls.push((decision.own, 1)),
        }
    }
    println!(
        "seed {base}, {} deals, {} seam decisions ({:.2}% of deals), walk {:.1?}",
        args.count,
        decisions.len(),
        100.0 * decisions.len() as f64 / args.count as f64,
        started.elapsed()
    );
    let mut rungs: BTreeMap<&str, Vec<Call>> = BTreeMap::new();
    for (key, calls) in &mut made {
        calls.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
        let total: usize = calls.iter().map(|&(_, n)| n).sum();
        let census: Vec<String> = calls
            .iter()
            .map(|(call, n)| format!("{call} ×{n}"))
            .collect();
        println!("  {key}: {}", census.join(", "));
        rungs.insert(
            key,
            calls
                .iter()
                .filter(|&&(_, n)| n as f64 >= args.min_share * total as f64)
                .map(|&(call, _)| call)
                .collect(),
        );
    }
    if let Some(own) = &args.trace {
        let knob = args
            .slam_try
            .then(|| american(&Agreements::default()).bind());
        trace(&decisions, &rungs, &policy, knob.as_ref(), own, args.show);
        return;
    }
    let m = args.layouts;
    if m == 0 {
        return;
    }

    // Draw both arms; a decision either arm starves on is counted and skipped.
    let drawn: Vec<[Vec<FullDeal>; 2]> = decisions
        .par_iter()
        .enumerate()
        .map(|(i, d)| {
            let seed = base.wrapping_add(args.count as u64).wrapping_add(i as u64);
            let inferences = policy.infer(relative(VULS[d.vul], d.seat), &d.prefix);
            let mut rng = StdRng::seed_from_u64(seed.wrapping_add(args.count as u64));
            let range = sample_layouts(d.hand, d.seat, &inferences, &mut rng, m);
            let replay = sample_for(d.hand, d.seat, &policy, VULS[d.vul], &d.prefix, m, seed);
            [range, replay]
        })
        .collect();
    for (arm, name) in ARMS.iter().enumerate() {
        let short = drawn.iter().filter(|d| d[arm].len() < m).count();
        println!("  {name}: {short} short draws");
    }
    let kept: Vec<usize> = (0..decisions.len())
        .filter(|&i| drawn[i].iter().all(|layouts| layouts.len() == m))
        .collect();
    let drew = started.elapsed();

    // One main-thread solve: per kept decision, `range`, `replay`, the truth.
    let stride = 2 * m + 1;
    let worlds: Vec<FullDeal> = kept
        .iter()
        .flat_map(|&i| {
            drawn[i]
                .iter()
                .flatten()
                .copied()
                .chain(std::iter::once(decisions[i].truth))
        })
        .collect();
    let tables = Solver::lock(None).solve_deals(&worlds, NonEmptyStrainFlags::ALL);
    let solved = started.elapsed();

    let priced: Vec<Priced> = kept
        .par_iter()
        .enumerate()
        .map(|(slot, &i)| {
            let d = &decisions[i];
            let mut candidates = vec![d.own];
            candidates.extend(rungs[d.key.as_str()].iter().filter(|&&call| call != d.own));
            let price = |from: usize, to: usize| {
                let at = slot * stride;
                swings(
                    &candidates,
                    &d.prefix,
                    d.dealer,
                    d.seat,
                    &worlds[at + from..at + to],
                    &tables[at + from..at + to],
                    &policy,
                    &policy,
                    VULS[d.vul],
                )
            };
            let mean = |swings: Vec<Vec<[i64; 2]>>| -> Vec<[f64; 2]> {
                swings
                    .iter()
                    .map(|layouts| {
                        [0, 1].map(|b| layouts.iter().map(|s| s[b]).sum::<i64>() as f64 / m as f64)
                    })
                    .collect()
            };
            Priced {
                decision: i,
                forecast: [mean(price(0, m)), mean(price(m, 2 * m))],
                truth: price(2 * m, stride).iter().map(|c| c[0]).collect(),
                candidates,
            }
        })
        .collect();
    println!(
        "{} priced, M = {m}; draw {:.1?}, solve {:.1?}, price {:.1?}\n",
        priced.len(),
        drew,
        solved - drew,
        started.elapsed() - solved
    );

    let all: Vec<&Priced> = priced.iter().collect();
    println!("True-deal IMPs per seam decision against the book's call, ± 95%:");
    for (arm, name) in ARMS.iter().enumerate() {
        println!("\n── forecast: {name} ──");
        println!(
            "  margin  rule   fired   {:<20} {:<20}",
            BRACKETS[0], BRACKETS[1]
        );
        for margin in MARGINS {
            for (rule, rule_name) in RULES.iter().enumerate() {
                let (fired, gains) = verdict(&all, arm, rule, margin);
                println!(
                    "  {margin:<6}  {rule_name:<5}  {:5.1}%  {:<20} {:<20}",
                    100.0 * fired as f64 / all.len() as f64,
                    show(&gains[0]),
                    show(&gains[1])
                );
            }
        }
        // The pre-registered rule, by vulnerability cell.
        for (vul, cell) in ["none", "both"].iter().enumerate() {
            let rows: Vec<&Priced> = all
                .iter()
                .copied()
                .filter(|row| decisions[row.decision].vul == vul)
                .collect();
            let (fired, gains) = verdict(&rows, arm, 2, args.margin);
            println!(
                "  gate rule (both, {}) vul {cell}: {} decisions, {fired} fired, {} / {}",
                args.margin,
                rows.len(),
                show(&gains[0]),
                show(&gains[1])
            );
        }
        // §5's original wording: tune the margin on one half, read the other.
        let (even, odd): (Vec<_>, Vec<_>) =
            all.iter().copied().partition(|row| row.decision % 2 == 0);
        let tuned = MARGINS
            .into_iter()
            .max_by_key(|&margin| {
                let (_, gains) = verdict(&even, arm, 2, margin);
                gains.iter().flatten().sum::<i64>()
            })
            .expect("the grid is not empty");
        let (fired, gains) = verdict(&odd, arm, 2, tuned);
        println!(
            "  margin tuned on the even half = {tuned}; odd half: {} decisions, {fired} fired, {} / {}",
            odd.len(),
            show(&gains[0]),
            show(&gains[1])
        );
        // What the gate rule swaps, and what each swap earned on the true deals.
        let mut swaps: BTreeMap<String, [Vec<i64>; 2]> = BTreeMap::new();
        for row in &all {
            let c = pick(&row.forecast[arm], 2, args.margin);
            if c != 0 {
                let d = &decisions[row.decision];
                let entry = swaps
                    .entry(format!("{}  {} -> {}", d.key, d.own, row.candidates[c]))
                    .or_default();
                for (bracket, gains) in entry.iter_mut().enumerate() {
                    gains.push(row.truth[c][bracket]);
                }
            }
        }
        for (swap, gains) in &swaps {
            println!(
                "    {swap}  ×{}  {} / {}",
                gains[0].len(),
                show(&gains[0]),
                show(&gains[1])
            );
        }
    }

    let agree = all
        .iter()
        .filter(|row| {
            pick(&row.forecast[0], 2, args.margin) == pick(&row.forecast[1], 2, args.margin)
        })
        .count();
    println!(
        "\nThe two forecasts pick the same call on {:.1}% of decisions (gate rule).",
        100.0 * agree as f64 / all.len() as f64
    );
    let oracle: [Vec<i64>; 2] = [0, 1].map(|b| {
        all.iter()
            .map(|row| row.truth.iter().map(|t| t[b]).max().unwrap_or(0))
            .collect()
    });
    println!(
        "Oracle (hindsight best rung per deal): {} / {}",
        show(&oracle[0]),
        show(&oracle[1])
    );

    println!("\nStatic control — always switch `own` to `alt`, true deals (n ≥ 30):");
    let mut remaps: BTreeMap<String, [Vec<i64>; 2]> = BTreeMap::new();
    for row in &all {
        let d = &decisions[row.decision];
        for (c, alt) in row.candidates.iter().enumerate().skip(1) {
            let entry = remaps
                .entry(format!("{}  {} -> {alt}", d.key, d.own))
                .or_default();
            for (bracket, gains) in entry.iter_mut().enumerate() {
                gains.push(row.truth[c][bracket]);
            }
        }
    }
    for (remap, gains) in remaps.iter().filter(|(_, gains)| gains[0].len() >= 30) {
        println!(
            "  {remap}  ×{}  {} / {}",
            gains[0].len(),
            show(&gains[0]),
            show(&gains[1])
        );
    }
}
