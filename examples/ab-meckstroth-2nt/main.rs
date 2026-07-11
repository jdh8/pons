//! Meckstroth-2NT A/B: natural 18-19 `2NT` rebid vs the artificial GF `2NT`.
//!
//! After `1M – 1NT` (the forcing notrump) opener's strong hands rebid a natural
//! 18-19 balanced `2NT` in the baseline — an unbalanced 18+ has no game-forcing
//! rebid and underbids as a simple two-level suit.  The **real Meckstroth
//! adjunct** turns `2NT` into an artificial 18+ game force of *any* shape, with a
//! `3♣` relay and shape-out continuations toward the right game or slam.  Both
//! arms run the same 2/1 system; the only difference is the [`set_meckstroth_adjunct`]
//! toggle, read once at book-construction time.  (That knob now carries the whole
//! adjunct — the artificial `2NT` *and* the invitational `3m` jumps — so the
//! baseline arm drops both; the `2NT` machine dominates the divergent boards.)
//!
//! Opponents are silenced (East/West always pass), so every auction is
//! constructive start to finish — this measures the *constructive* value of the
//! adjunct.  Each board is bid twice over the same deal, once per arm; boards
//! whose arms reach different contracts are solved double dummy once and scored.
//! A positive IMPs/board favors the artificial game force.
//!
//! ```text
//! cargo run --release --example ab-meckstroth-2nt -- --count 5000
//! ```

use clap::Parser;
use contract_bridge::auction::Auction;
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Contract, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::american::set_meckstroth_adjunct;
use pons::bidding::context::relative;
use pons::bidding::{Family, Inferences, Stance};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use pons::single_dummy::{LeadQuestion, single_dummy_leads};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_uncontested, mean_with_ci, seat_to_act};

/// Meckstroth-2NT A/B: natural `2NT` rebid vs the artificial game force
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "5000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Also price the opening lead single-dummy on divergent boards (slower):
    /// the blind-lead scorer that sits between plain DD and perfect defense
    #[arg(long, default_value_t = false)]
    sd: bool,

    /// Worlds sampled per blind lead (the validated GTO setting is 16)
    #[arg(long, default_value_t = 16)]
    sd_worlds: usize,

    /// Seed for the world-sampling RNG (report it to reproduce a run)
    #[arg(long, default_value_t = 20_240_607)]
    sd_seed: u64,
}

/// One board's two arms: each arm's uncontested auction and its final contract.
type ArmBids = [(Auction, Option<(Contract, Seat)>); 2];

/// Signed-for-NS score of a contract given declarer's (single-dummy) tricks.
/// Copied from `ab-dump-sd` (the promotion to `src/scoring.rs` is still a TODO).
fn ns_score_tricks(
    contract: Contract,
    declarer: Seat,
    tricks: u8,
    vul: AbsoluteVulnerability,
) -> i64 {
    let declarer_vul = vul.contains(match declarer {
        Seat::North | Seat::South => AbsoluteVulnerability::NS,
        Seat::East | Seat::West => AbsoluteVulnerability::EW,
    });
    let score = i64::from(contract.score(tricks, declarer_vul));
    match declarer {
        Seat::North | Seat::South => score,
        Seat::East | Seat::West => -score,
    }
}

/// The (contract, declarer, leader-view inferences) of one auction, read through
/// `stance`; `None` for a pass-out (sd score 0).  Mirrors `ab-dump-sd`.
fn lead_inputs(
    auction: &Auction,
    stance: &Stance,
    dealer: Seat,
    vul: AbsoluteVulnerability,
) -> Option<(Contract, Seat, Inferences)> {
    let (contract, declarer) = final_contract(auction, dealer)?;
    let leader = declarer.lho();
    let cut = (auction.len().saturating_sub(3)..=auction.len())
        .find(|&len| seat_to_act(dealer, len) == leader)
        .expect("one of four consecutive lengths reaches every seat");
    Some((
        contract,
        declarer,
        stance.infer(relative(vul, leader), &auction[..cut]),
    ))
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let mut rng = rand::rng();
    // arm 0 = baseline (natural 2NT, no adjunct), arm 1 = the Meckstroth adjunct
    // (the shipped default).  The toggle is read at book-construction time, so
    // build each arm under its own setting; the baked tries are independent
    // thereafter.
    set_meckstroth_adjunct(false);
    let baseline = american().against(Family::NATURAL);
    set_meckstroth_adjunct(true); // restore the shipped default (on)
    let adjunct = american().against(Family::NATURAL);
    let stances = [baseline, adjunct];

    // Both arms bid the same deal; the only difference is opener's rebid table.
    // Deal sequentially (cheap), then bid in parallel — bidding is pure (the
    // books read their thread-locals at construction), so boards are independent
    // and par_iter preserves order. The DD solver stays on the main thread below.
    let deals: Vec<FullDeal> = (0..args.count).map(|_| full_deal(&mut rng)).collect();
    let vul = args.vulnerability;
    let bids: Vec<ArmBids> = deals
        .par_iter()
        .enumerate()
        .map(|(index, deal)| {
            let dealer = Seat::ALL[index % 4];
            std::array::from_fn(|arm| {
                let auction = bid_uncontested(&stances[arm], dealer, vul, deal);
                let contract = final_contract(&auction, dealer);
                (auction, contract)
            })
        })
        .collect();
    // Contract-only view for the plain/PD scorers below; the retained auctions
    // feed the single-dummy blind-lead pass (each arm read through its own book).
    let contracts: Vec<[Option<(Contract, Seat)>; 2]> =
        bids.iter().map(|b| [b[0].1, b[1].1]).collect();

    // Only boards whose arms diverge can swing; solve those once.
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| contracts[i][0] != contracts[i][1])
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    let mut points = 0i64;
    let mut total_imps = 0i64;
    let mut pd_imps = 0i64;
    for (&i, table) in divergent.iter().zip(tables.iter()) {
        let base = ns_score_contract(contracts[i][0], table, args.vulnerability);
        let adj = ns_score_contract(contracts[i][1], table, args.vulnerability);
        points += adj - base;
        total_imps += imps(adj - base);
        // Perfect-defense read from the same tables — opponents are silenced, so
        // this only ever adds a double to a contract that fails DD (no doubling
        // artifact possible here); plain-DD stays the gate, PD is confirmation.
        let pd_base = ns_score_pd(contracts[i][0], table, args.vulnerability);
        let pd_adj = ns_score_pd(contracts[i][1], table, args.vulnerability);
        pd_imps += imps(pd_adj - pd_base);
    }

    println!(
        "=== Meckstroth-2NT A/B: {} boards, vulnerability {} ===",
        args.count, args.vulnerability,
    );
    println!("(opponents silenced — constructive value only)");
    println!(
        "Divergent boards: {} of {} ({:.1}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "GF 2NT: {points:+} points, {total_imps:+} IMPs ({:+.3} IMPs/board plain)",
        total_imps as f64 / args.count.max(1) as f64,
    );
    println!(
        "        {pd_imps:+} IMPs ({:+.3} IMPs/board PD)",
        pd_imps as f64 / args.count.max(1) as f64,
    );

    if args.sd {
        // Blind-lead pass: on each divergent board both arms' auctions are read
        // for the leader (declarer's LHO) through their own book, the opening
        // lead is chosen single-dummy over `sd_worlds` sampled worlds, then play
        // is double-dummy on the actual deal. Main thread only — the solver is
        // not reentrant, and the plain/PD solve above has already released it.
        let mut pending: Vec<(usize, bool, Contract, Seat)> = Vec::new();
        let mut questions: Vec<LeadQuestion> = Vec::new();
        for &i in &divergent {
            let dealer = Seat::ALL[i % 4];
            for (arm_on, arm) in [(true, 1usize), (false, 0usize)] {
                if let Some((contract, declarer, inferences)) =
                    lead_inputs(&bids[i][arm].0, &stances[arm], dealer, vul)
                {
                    pending.push((i, arm_on, contract, declarer));
                    questions.push(LeadQuestion {
                        deal: deals[i],
                        strain: contract.bid.strain,
                        declarer,
                        inferences,
                    });
                }
            }
        }
        let mut rng = StdRng::seed_from_u64(args.sd_seed);
        let mut on_score = vec![0i64; args.count];
        let mut off_score = vec![0i64; args.count];
        const CHUNK: usize = 4096;
        for (asked, chunk) in pending.chunks(CHUNK).zip(questions.chunks(CHUNK)) {
            let answers = single_dummy_leads(chunk, &mut rng, args.sd_worlds);
            for (&(i, arm_on, contract, declarer), &(_, tricks)) in asked.iter().zip(&answers) {
                let score = ns_score_tricks(contract, declarer, u8::from(tricks), vul);
                if arm_on {
                    on_score[i] = score;
                } else {
                    off_score[i] = score;
                }
            }
        }
        let board_imps: Vec<i64> = (0..args.count)
            .map(|i| imps(on_score[i] - off_score[i]))
            .collect();
        let (mean, ci) = mean_with_ci(&board_imps);
        let total: i64 = board_imps.iter().sum();
        println!(
            "sd-lead GF 2NT ({} worlds, seed {}): {total:+} IMPs, {mean:+.4} IMPs/board [95% CI ±{ci:.4}], {:+.3} IMPs/divergent",
            args.sd_worlds,
            args.sd_seed,
            total as f64 / divergent.len().max(1) as f64,
        );
    }
}
