//! Size-ask-eight A/B: **invite** the balanced 8 (no four-card major) over our
//! 1NT vs **pass** it — re-priced under SD-PD.
//!
//! Over our 1NT a balanced eight with no four-card major can size-ask (`2♠` in the
//! Puppet scheme, `2NT` in European): an invite landing `2NT` opposite a 15-16
//! minimum and `3NT` opposite a 17 maximum.  On 2026-07-03 we carved the flat
//! 4-3-3-3 eight out of that invite (it passes), on the strength of
//! `examples/probe-uninvite-4333` — a **plain double-dummy** probe (+0.64 IMPs/board
//! for passing the flat class).  But DD is level-dependently pessimistic on the low
//! contracts in play — very on 1NT (the pass outcome), only slightly on 3NT (the
//! max-invite) — so a DD verdict on pass-vs-invite mis-weights the very levels the
//! decision turns on.
//!
//! This harness re-runs the original invite-vs-pass experiment for the whole class
//! (`hcp(8) & balanced()` no four-card major) under **SD-PD**: the opening lead is
//! chosen single-dummy over auction-consistent worlds (so 1NT/2NT stop being scored
//! as if the defenders saw all 26 cards), and the resulting trick count is scored
//! through the perfect-defense doubling rule (`ns_score_pd_tricks`) so an overreached
//! invite that fails still pays the doubled penalty a real opponent would exact.  For
//! the gradient, the swing is also reported plain-DD, DD-PD, and plain-SD.
//!
//! Caveat: even SD-PD keeps perfect defense *after* trick one, so it still underscores
//! 1NT's real-world overtricks from fallible defense — it is a lower bound on the pass
//! side's true edge.
//!
//! Arm 0 = **invite** (the whole class size-asks), arm 1 = **pass** (the whole class
//! passes); the report is **Pass − Invite**, so a **positive** number means passing
//! wins — the size ask is bad.  The two knob poles subsume the shipped split (flat
//! passes, shapely eights invite): read each shape off the flat / non-flat breakdown.
//! Opponents are silenced, so this is the constructive value only.
//!
//! ```text
//! cargo run --release --example ab-size-ask-eight -- --count 2000000 --seed "$SEED_BASE"
//! cargo run --release --example ab-size-ask-eight -- --count 2000000 --seed "$SEED_BASE" --vulnerability both
//! ```

use clap::Parser;
use contract_bridge::auction::Auction;
use contract_bridge::{AbsoluteVulnerability, Contract, FullDeal, Hand, Seat, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::american::{SizeAskEight, set_size_ask_eight};
use pons::bidding::context::relative;
use pons::bidding::{Family, Inferences, Stance};
use pons::scoring::{
    final_contract, imps, ns_score_contract, ns_score_pd, ns_score_pd_tricks, ns_score_tricks,
};
use pons::single_dummy::{LeadQuestion, single_dummy_leads};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_uncontested, hand_hcp, mean_with_ci, seat_to_act, seeded_deals};

/// Size-ask-eight A/B: invite the balanced 8 vs pass it, priced four ways
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "2000000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Base seed — fresh per experiment (`SEED_BASE=$(date +%s)`), shared across
    /// arms/vuls; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,

    /// Worlds sampled per blind lead (the validated GTO setting is 16)
    #[arg(long, default_value_t = 16)]
    sd_worlds: usize,

    /// Seed for the world-sampling RNG (report it to reproduce a run)
    #[arg(long, default_value_t = 20_240_607)]
    sd_seed: u64,
}

/// One board's two arms: each arm's uncontested auction and its final contract.
type ArmBids = [(Auction, Option<(Contract, Seat)>); 2];

/// Exactly one four-card suit and three three-card suits.
fn is_flat_4333(hand: Hand) -> bool {
    let mut lens: [usize; 4] = [0; 4];
    for (i, &s) in Suit::ASC.iter().enumerate() {
        lens[i] = hand[s].len();
    }
    lens.sort_unstable();
    lens == [3, 3, 3, 4]
}

/// Is the responder's size-ask eight a flat 4-3-3-3?  On a divergent board the sole
/// difference between the arms is the size-ask-eight routing, so the responder (the
/// North/South hand with exactly eight HCP) is the class hand; classify its shape.
fn responder_is_flat(deal: &FullDeal) -> Option<bool> {
    [Seat::North, Seat::South]
        .into_iter()
        .find(|&s| hand_hcp(deal[s]) == 8)
        .map(|s| is_flat_4333(deal[s]))
}

/// The (contract, declarer, leader-view inferences) of one auction, read through
/// `stance`; `None` for a pass-out (sd score 0).  Mirrors `ab-notrump-minors`.
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

/// Mean IMPs/board (over `count`) and mean IMPs/divergent, each with a 95% CI, for
/// the subset of divergent boards flagged by `keep`.  `per_board` is the full-length
/// swing vector (0 on non-divergent boards).
fn summarise(
    per_board: &[i64],
    divergent: &[usize],
    keep: impl Fn(usize) -> bool,
) -> (usize, (f64, f64), (f64, f64)) {
    let fired: Vec<i64> = divergent
        .iter()
        .copied()
        .filter(|&i| keep(i))
        .map(|i| per_board[i])
        .collect();
    // IMPs/board keeps the full denominator only when scoring the whole set; for a
    // shape subset the natural rate is per-fired, so report the subset both ways off
    // its own fired vector padded to `count` for the board rate.
    let mut padded = vec![0i64; per_board.len()];
    for &i in divergent.iter().filter(|&&i| keep(i)) {
        padded[i] = per_board[i];
    }
    (fired.len(), mean_with_ci(&padded), mean_with_ci(&fired))
}

#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
fn main() {
    let args = Args::parse();
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = args.vulnerability;

    // arm 0 = invite the whole class, arm 1 = pass the whole class.  The routing is
    // read at book-construction time, so build each arm under its own setting; the
    // baked tries are independent thereafter.  Restore the shipped default after.
    set_size_ask_eight(SizeAskEight::Invite);
    let invite = american().against(Family::NATURAL);
    set_size_ask_eight(SizeAskEight::Pass);
    let pass = american().against(Family::NATURAL);
    set_size_ask_eight(SizeAskEight::Shipped);
    let stances = [invite, pass];

    let deals = seeded_deals(base, args.count);
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

    let contracts: Vec<[Option<(Contract, Seat)>; 2]> =
        bids.iter().map(|b| [b[0].1, b[1].1]).collect();

    // Only boards whose arms diverge can swing; solve those once (double dummy).
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| contracts[i][0] != contracts[i][1])
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
    let tables = Solver::lock(None).solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    // Plain-DD and DD-PD swings from the DD tables (Pass − Invite = arm1 − arm0).
    let mut dd = vec![0i64; args.count];
    let mut ddpd = vec![0i64; args.count];
    for (&i, table) in divergent.iter().zip(tables.iter()) {
        dd[i] = imps(
            ns_score_contract(contracts[i][1], table, vul)
                - ns_score_contract(contracts[i][0], table, vul),
        );
        ddpd[i] = imps(
            ns_score_pd(contracts[i][1], table, vul) - ns_score_pd(contracts[i][0], table, vul),
        );
    }

    // Single-dummy pass: on each divergent board, read each arm's auction for the
    // leader (declarer's LHO) through that arm's book, choose the opening lead
    // single-dummy over `sd_worlds` sampled worlds, then play double-dummy on the
    // actual deal.  Score the resulting trick count both plain (`ns_score_tricks`)
    // and SD-PD (`ns_score_pd_tricks`, a failing contract priced doubled).  Main
    // thread only — the solver is not reentrant and the DD solve above released it.
    let mut pending: Vec<(usize, usize, Contract, Seat)> = Vec::new();
    let mut questions: Vec<LeadQuestion> = Vec::new();
    for &i in &divergent {
        let dealer = Seat::ALL[i % 4];
        for arm in [0usize, 1usize] {
            if let Some((contract, declarer, inferences)) =
                lead_inputs(&bids[i][arm].0, &stances[arm], dealer, vul)
            {
                pending.push((i, arm, contract, declarer));
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
    // Per arm: plain-SD and SD-PD NS scores, indexed by board.
    let mut sd_score = [vec![0i64; args.count], vec![0i64; args.count]];
    let mut sdpd_score = [vec![0i64; args.count], vec![0i64; args.count]];
    const CHUNK: usize = 4096;
    for (asked, chunk) in pending.chunks(CHUNK).zip(questions.chunks(CHUNK)) {
        let answers = single_dummy_leads(chunk, &mut rng, args.sd_worlds);
        for (&(i, arm, contract, declarer), &(_, tricks)) in asked.iter().zip(&answers) {
            let t = u8::from(tricks);
            sd_score[arm][i] = ns_score_tricks(contract, declarer, t, vul);
            sdpd_score[arm][i] = ns_score_pd_tricks(contract, declarer, t, vul);
        }
    }
    let mut sd = vec![0i64; args.count];
    let mut sdpd = vec![0i64; args.count];
    for &i in &divergent {
        sd[i] = imps(sd_score[1][i] - sd_score[0][i]);
        sdpd[i] = imps(sdpd_score[1][i] - sdpd_score[0][i]);
    }

    // Shape tag per divergent board: flat 4-3-3-3 responder vs non-flat.
    let flat: Vec<bool> = (0..args.count)
        .map(|i| responder_is_flat(&deals[i]).unwrap_or(false))
        .collect();

    println!(
        "=== size-ask-eight A/B (Pass − Invite): {} boards, vulnerability {}, seed {} ===",
        args.count, vul, base,
    );
    println!(
        "(opponents silenced; + ⇒ passing wins ⇒ the size ask is bad. sd {} worlds, sd seed {})",
        args.sd_worlds, args.sd_seed,
    );
    println!(
        "Divergent boards: {} of {} ({:.3}%)\n",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );

    let brackets: [(&str, &[i64]); 4] = [
        ("plain-DD", &dd),
        ("DD-PD", &ddpd),
        ("plain-SD", &sd),
        ("SD-PD  ", &sdpd),
    ];
    let shapes: [(&str, &dyn Fn(usize) -> bool); 3] = [
        ("all      ", &|_| true),
        ("flat-4333", &|i| flat[i]),
        ("non-flat ", &|i| !flat[i]),
    ];
    for (shape_label, keep) in &shapes {
        let fired = divergent.iter().filter(|&&i| keep(i)).count();
        println!("--- {shape_label} ({fired} divergent) ---");
        for (label, per_board) in &brackets {
            let (n, (board_mean, board_ci), (fired_mean, fired_ci)) =
                summarise(per_board, &divergent, keep);
            println!(
                "{label}: {board_mean:+.4} ± {board_ci:.4} IMPs/board  |  \
                 {fired_mean:+.3} ± {fired_ci:.3} IMPs/divergent  (n={n})",
            );
        }
        println!();
    }
}
