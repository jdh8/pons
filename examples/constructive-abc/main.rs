//! Constructive-only floor A/B/C: instinct vs SearchFloor vs NeuralFloorSearch.
//!
//! The three learned-vs-deterministic floors normally partition the auction —
//! the neural/search floors own only the contested books, while the constructive
//! book is always floored by the deterministic
//! [`instinct`][pons::bidding::instinct] ladder (see
//! [`american_constructive_floor`][pons::bidding::american::american_constructive_floor]).
//! This harness lifts that partition to ask one question: *if* we let the search
//! floors answer the unbooked constructive nodes, do they bid them better than
//! the milestone heuristics?
//!
//! It isolates the constructive phase by **silencing the opponents** — East and
//! West always pass — so every auction is constructive start to finish (when
//! opener takes its second turn, only responder has bid).  Each board is bid
//! three times, once per floor, over the *same* deal; the deal is solved double
//! dummy once and the three final contracts scored against it.  The pairwise
//! score swings become IMPs: a positive `X vs Y` favors `X`.
//!
//! Because the only difference between arms is the constructive floor and the
//! opponents are passive across all three, no seat swap is needed — the
//! comparison is already duplicate-clean.
//!
//! ```text
//! cargo run --release --features search --example constructive-abc -- --count 200
//! ```
//!
//! The `search` arm runs a double-dummy rollout per non-forced decision, so it
//! dominates the runtime (~seconds per board at the default knobs); shrink
//! `--layouts`/`--shortlist` for a faster, noisier run, and prefer
//! `scripts/idle-run.sh` for a large `--count` on the shared box.

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::bidding::american::american_constructive_floor;
use pons::bidding::context::relative;
use pons::bidding::neural_floor::NeuralFloorSearch;
use pons::bidding::search_floor::SearchFloor;
use pons::bidding::{Family, Stance, System};
use pons::instinct;
use pons::scoring::{final_contract, imps, ns_score_contract};

/// Constructive-only floor A/B/C: instinct vs SearchFloor vs NeuralFloorSearch
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "200")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// `SearchFloor` layouts sampled and solved per decision
    #[arg(short, long, default_value = "128")]
    layouts: usize,

    /// `SearchFloor` top-k calls actually scored by EV
    #[arg(short, long, default_value = "8")]
    shortlist: usize,
}

/// The seat acting after `len` calls from `dealer`
const fn seat_to_act(dealer: Seat, len: usize) -> Seat {
    Seat::ALL[(dealer as usize + len) % 4]
}

/// The highest-logit *legal* call, defaulting to a pass
fn next_call(
    stance: &Stance,
    hand: Hand,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    auction: &Auction,
) -> Call {
    let seat = seat_to_act(dealer, auction.len());
    let Some(logits) = stance.classify(hand, relative(vul, seat), auction) else {
        return Call::Pass;
    };
    let mut scored: Vec<(Call, f32)> = logits
        .iter()
        .map(|(call, &logit)| (call, logit))
        .filter(|&(_, logit)| logit.is_finite())
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
    scored
        .into_iter()
        .map(|(call, _)| call)
        .find(|&call| auction.can_push(call).is_ok())
        .unwrap_or(Call::Pass)
}

/// Bid one deal with the opponents (East/West) forced to pass throughout
fn bid_uncontested(
    stance: &Stance,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let call = if matches!(seat, Seat::East | Seat::West) {
            Call::Pass
        } else {
            next_call(stance, deal[seat], dealer, vul, &auction)
        };
        auction.push(call);
    }
    auction
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let mut rng = rand::rng();

    // Three arms, identical but for the constructive floor; opponents silenced,
    // so the contested books they each leave bare are never reached.
    let names = ["instinct", "search", "neural"];
    let stances = [
        american_constructive_floor(instinct()).against(Family::NATURAL),
        american_constructive_floor(SearchFloor {
            layouts: args.layouts,
            shortlist: args.shortlist,
            ..SearchFloor::default()
        })
        .against(Family::NATURAL),
        american_constructive_floor(NeuralFloorSearch).against(Family::NATURAL),
    ];

    // Bid every board with all three arms over the same deal.
    let mut deals: Vec<FullDeal> = Vec::with_capacity(args.count);
    let mut contracts = Vec::with_capacity(args.count);
    for index in 0..args.count {
        let dealer = Seat::ALL[index % 4];
        let deal = full_deal(&mut rng);
        let board: [_; 3] = std::array::from_fn(|arm| {
            let auction = bid_uncontested(&stances[arm], dealer, args.vulnerability, &deal);
            final_contract(&auction, dealer)
        });
        deals.push(deal);
        contracts.push(board);
        eprint!("\rbid {}/{}", index + 1, args.count);
    }
    eprintln!();

    // Only boards whose three arms diverge can swing; solve those once and reuse
    // the table for every pairing.
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| {
            let c = &contracts[i];
            c[0] != c[1] || c[1] != c[2]
        })
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    // (first, second) arm indices; a positive swing favors `first`.
    let pairings = [(0, 1), (0, 2), (1, 2)];
    let mut points = [0i64; 3];
    let mut imp_totals = [0i64; 3];
    for (&i, table) in divergent.iter().zip(tables.iter()) {
        let scores: [i64; 3] = std::array::from_fn(|arm| {
            ns_score_contract(contracts[i][arm], table, args.vulnerability)
        });
        for (p, &(x, y)) in pairings.iter().enumerate() {
            let swing = scores[x] - scores[y];
            points[p] += swing;
            imp_totals[p] += imps(swing);
        }
    }

    println!(
        "=== Constructive-only floor A/B/C: {} boards, vulnerability {} ===",
        args.count, args.vulnerability,
    );
    println!("(opponents silenced — every auction is constructive)");
    println!(
        "SearchFloor knobs: layouts={}, shortlist={}",
        args.layouts, args.shortlist,
    );
    println!(
        "Divergent boards: {} of {} ({:.0}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "\n  {:>8} vs {:<8}  points     IMPs   IMPs/board",
        "first", "second"
    );
    for (p, &(x, y)) in pairings.iter().enumerate() {
        println!(
            "  {:>8} vs {:<8}  {:+6}  {:+6}     {:+.3}",
            names[x],
            names[y],
            points[p],
            imp_totals[p],
            imp_totals[p] as f64 / args.count.max(1) as f64,
        );
    }
    println!("\n(a positive row favors the first-named floor)");
}
