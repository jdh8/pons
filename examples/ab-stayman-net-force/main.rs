//! Measure the net-priced Stayman invite/force seams: an A/B duplicate match.
//!
//! Responder over our 1NT forces game on every 9 HCP — A/B-verified better than
//! inviting the 9, and the `probe-nt-invite-eval` screen found raw HCP
//! out-ranks every analytic evaluator at that boundary.  The evaluator net is
//! the first candidate to beat it, and only on the **Stayman class** (a
//! four-card major): +0.030/+0.044 IMPs/board over the whole class at the full
//! 15-17 opener mixture, rising to +0.048/+0.069 opposite exactly-15 openers —
//! the minimum-opener slice where force-vs-invite actually diverges (an opener
//! with extras accepts the invite either way).  The balanced no-major seam
//! stays HCP (the net measured ≈0 there, the third evaluator family to fail).
//!
//! [`set_stayman_net_force`][pons::bidding::american::set_stayman_net_force]
//! (default **off** — this A/B measured a loss; see the knob's docs for the
//! verdict and the forensic seam split) converts exactly the Stayman-rebid
//! seams — with a fit the `4M`/`3M`/`3OM`-slam-try split, without one the
//! `3NT`/`2NT` revert — to "the net clears the game's IMP break-even at the
//! live vulnerability".  Vulnerability-awareness is part of the claim, so
//! measure at `--vulnerability none` **and** `both` before reading a verdict.
//!
//! Each board is bid twice, duplicate style: at table A the net pair sits
//! North/South against a pair without it; at table B the teams swap seats.  Both
//! pairs play the very same books — the per-call thread-local flip serves both
//! from one stance.  Divergent boards are scored two ways from one DD table:
//! [`ns_score_pd`] (perfect defense, prices the road-not-taken as doubled) and
//! [`ns_score_contract`] (plain DD) — the standard bracket; read the verdict
//! from the decision table in `docs/measurement.md`.
//!
//! ```text
//! cargo run --release --example ab-stayman-net-force -- --count 200000
//! cargo run --release --example ab-stayman-net-force -- --count 200000 --vulnerability both
//! cargo run --release --example ab-stayman-net-force -- --count 20000 --show 8
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::Accumulator;
use pons::american;
use pons::bidding::american::set_stayman_net_force;
use pons::bidding::{Family, Stance};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, next_call, seat_to_act};

/// Measure the net-priced Stayman invite/force seams: an A/B duplicate match
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "20000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Deal seed (reproducible boards)
    #[arg(long, default_value = "0")]
    seed: u64,

    /// Print this many divergent boards (auction + contracts) for inspection
    #[arg(long, default_value = "0")]
    show: usize,

    /// Rejection-sample the firing zone: dealer holds a 15-17 balanced hand and
    /// dealer's partner a Stayman-class responder (a four-card major, no
    /// five-card major, not flat 4-3-3-3) with 7-11 HCP.  Unconditioned deals
    /// diverge on only ~0.2% of boards, far too dilute to power the verdict;
    /// sliced boards report IMPs *per sliced board*, not per random deal.
    #[arg(long)]
    slice: bool,
}

/// Balanced: 4-3-3-3, 4-4-3-2, or 5-3-3-2 (no void/singleton, ≤1 doubleton, ≤5 long).
fn is_balanced(h: contract_bridge::Hand) -> bool {
    let lens = contract_bridge::Suit::ASC.map(|s| h[s].len());
    lens.iter().all(|&l| (2..=5).contains(&l)) && lens.iter().filter(|&&l| l == 2).count() <= 1
}

/// The probe's Stayman class: a four-card major, no five-card major, not flat 4-3-3-3.
fn is_stayman_class(h: contract_bridge::Hand) -> bool {
    use contract_bridge::Suit;
    let (hh, ss) = (h[Suit::Hearts].len(), h[Suit::Spades].len());
    let flat4333 = Suit::ASC.into_iter().all(|s| h[s].len() >= 3);
    !flat4333 && (hh == 4 || ss == 4) && hh < 5 && ss < 5
}

fn hcp(h: contract_bridge::Hand) -> u8 {
    contract_bridge::Suit::ASC
        .iter()
        .map(|&s| contract_bridge::eval::hcp::<u8>(h[s]))
        .sum()
}

/// The `--slice` acceptance test (see the flag's doc).
fn in_slice(dealer: Seat, deal: &FullDeal) -> bool {
    let opener = deal[dealer];
    let resp = deal[dealer.partner()];
    is_balanced(opener)
        && (15..=17).contains(&hcp(opener))
        && is_stayman_class(resp)
        && (7..=11).contains(&hcp(resp))
}

/// Bid out one deal, enabling the net seams only for the feature side
///
/// The thread-local is set just before each classification, so this is safe under
/// rayon: the worker sets and reads it on its own thread.  The bilans floor
/// stays at its shipped default for **both** sides — only the Stayman seams move.
fn bid_out(
    stance: &Stance,
    args: &Args,
    feature_is_ns: bool,
    dealer: Seat,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        set_stayman_net_force(seat_is_ns == feature_is_ns);
        auction.push(next_call(
            stance,
            deal[seat],
            dealer,
            args.vulnerability,
            &auction,
        ));
    }
    auction
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let stance = american().against(Family::NATURAL);

    // Deal sequentially (seeded, reproducible); bid both tables in parallel.
    let mut rng = StdRng::seed_from_u64(args.seed);
    let deals: Vec<(Seat, FullDeal)> = (0..args.count)
        .map(|index| {
            let dealer = Seat::ALL[index % 4];
            loop {
                let deal = full_deal(&mut rng);
                if !args.slice || in_slice(dealer, &deal) {
                    return (dealer, deal);
                }
            }
        })
        .collect();
    let boards: Vec<Board> = deals
        .par_iter()
        .map(|&(dealer, deal)| Board {
            deal,
            dealer,
            table_a: bid_out(&stance, &args, true, dealer, &deal),
            table_b: bid_out(&stance, &args, false, dealer, &deal),
        })
        .collect();

    // Only boards whose tables reach different contracts can swing; solve those
    // double dummy (on the main thread) and credit the swing to the net team
    // (NS at table A, EW at table B).
    let contracts: Vec<_> = boards
        .iter()
        .map(|board| {
            (
                final_contract(&board.table_a, board.dealer),
                final_contract(&board.table_b, board.dealer),
            )
        })
        .collect();
    let divergent: Vec<usize> = (0..boards.len())
        .filter(|&index| contracts[index].0 != contracts[index].1)
        .collect();
    let deals: Vec<FullDeal> = divergent.iter().map(|&index| boards[index].deal).collect();
    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    // Per-board IMP swing to the net team (0 on non-divergent boards), scored
    // both ways from the same DD table per the measurement doctrine.
    let mut swings_pd = vec![0i64; args.count];
    let mut swings_dd = vec![0i64; args.count];
    let mut shown = 0;
    for (&index, table) in divergent.iter().zip(tables.iter()) {
        let (contract_a, contract_b) = contracts[index];
        let points_pd = ns_score_pd(contract_a, table, args.vulnerability)
            - ns_score_pd(contract_b, table, args.vulnerability);
        let points_dd = ns_score_contract(contract_a, table, args.vulnerability)
            - ns_score_contract(contract_b, table, args.vulnerability);
        swings_pd[index] = imps(points_pd);
        swings_dd[index] = imps(points_dd);

        if shown < args.show {
            shown += 1;
            let board = &boards[index];
            let calls: Vec<Call> = board.table_a.iter().copied().collect();
            println!(
                "[{shown}] dealer {:?}  A {calls:?} -> {contract_a:?}  vs  B -> {contract_b:?}  (PD {:+}, DD {:+})",
                board.dealer,
                imps(points_pd),
                imps(points_dd),
            );
        }
    }

    println!(
        "\n=== Stayman-net-force A/B: {} boards, vulnerability {} ===",
        args.count, args.vulnerability,
    );
    println!(
        "Divergent boards: {} of {} ({:.2}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    for (label, swings) in [
        ("ns_score_pd  (PD)", &swings_pd),
        ("ns_score_cnt (DD)", &swings_dd),
    ] {
        let total: i64 = swings.iter().sum();
        let mut acc = Accumulator::new();
        for &swing in swings.iter() {
            acc.push(swing as f64);
        }
        let stats = acc.sample();
        let mean = stats.mean();
        let se = stats.sd() / (args.count.max(1) as f64).sqrt();
        let (lo, hi) = (mean - 1.96 * se, mean + 1.96 * se);
        let verdict = if (lo..=hi).contains(&0.0) {
            "parity"
        } else if mean > 0.0 {
            "net ahead"
        } else {
            "net behind"
        };
        println!(
            "{label}: {total:+} IMPs, {mean:+.3}/board  95% CI [{lo:+.3}, {hi:+.3}]  ({verdict})",
        );
    }
}
