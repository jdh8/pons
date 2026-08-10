//! Size-ask **accept-floor** A/B: opener accepts the balanced-eight size ask with
//! a minimum vs the shipped 17-only — re-priced under SD-PD, with a 15 control.
//!
//! Over our 1NT a balanced eight with no four-card major size-asks (`2♠` in the
//! Puppet scheme, `2NT` in European).  Opener accepts game with a **maximum** and
//! declines to `2NT` with a minimum; the shipped floor is **17** (accept the
//! 25-HCP game, stop at 24 combined).  A double-dummy probe (2026-07-22) rejected
//! lowering it to 16 — `3NT` on 16+8=24 fails often enough under perfect defense.
//! But DD is level-dependently pessimistic on the low **declining** contract
//! (`2NT`) and only slightly on the accepted `3NT`, and DD-PD over-punishes the
//! accepted game with doubled failures a realistic blind lead would dodge — the
//! same two effects that flipped the invite-vs-pass verdict.  This harness
//! re-prices accept-16 under **SD-PD**: the opening lead is chosen single-dummy
//! over auction-consistent worlds, and the trick count scored through the
//! perfect-defense doubling rule (`ns_score_pd_tricks`).  For the gradient the
//! swing is also reported plain-DD, DD-PD, and plain-SD.
//!
//! **Falsification control.**  The two arms are floor 15 (accept every 15-17
//! minimum) vs floor 17 (shipped).  Divergent boards then split by opener HCP: a
//! **16** is the live question, a **15** is a control — accepting on `15 + 8 = 23`
//! turns the invite into a game force, so 15 *should* decline.  The harness is
//! expected to price accept-15 as a clear loss; if instead it prices accept-15 a
//! win, the scorer (not the treatment) is wrong, and the accept-16 verdict can't be
//! trusted either.  17 accepts in both arms, so never diverges.
//!
//! In the **Puppet** scheme (the default) opener's `3♣`/`2NT` max/min signal is
//! shared with the club one-suiter, so a lower floor also up-signals there.  That
//! collateral is real routing, so it is measured — but each divergent board is
//! bucketed by responder type (**invite** = the balanced-eight size ask; **club** =
//! a six-card club one-suiter) so the accept question reads apart from it.
//!
//! Caveat: even SD-PD keeps perfect defense *after* trick one, so it still
//! underscores the accepted `3NT`'s real-world overtricks from fallible defense —
//! it is a lower bound on the accept side's true edge.
//!
//! Arm 0 = **accept** (floor 15), arm 1 = **decline** (floor 17, shipped); the
//! report is **Decline − Accept**, so a **positive** number means declining wins —
//! accepting is bad.  Read the `op16 invite` bucket for the question and the
//! `op15 invite CTRL` bucket for the control.  Opponents are silenced, so this is
//! the constructive value only.
//!
//! ```text
//! cargo run --release --example ab-size-ask-accept -- --count 12000000 --seed "$SEED_BASE"
//! cargo run --release --example ab-size-ask-accept -- --count 12000000 --seed "$SEED_BASE" --vulnerability both
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Contract, FullDeal, Hand, Seat, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::context::relative;
use pons::bidding::{Inferences, Stance};
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

/// Size-ask accept-floor A/B: accept (floor 15) vs decline (17), split by opener HCP
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "12000000")]
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

/// Responder's type: the two meanings of the two-way `2♠` — the balanced-eight
/// size ask, or a six-card club one-suiter (`Other` is a residual that should stay
/// empty on divergent boards).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Invite,
    Club,
    Other,
}

/// Sorted suit lengths of a hand, ascending.
fn sorted_lengths(hand: Hand) -> [usize; 4] {
    let mut lens: [usize; 4] = [0; 4];
    for (i, &s) in Suit::ASC.iter().enumerate() {
        lens[i] = hand[s].len();
    }
    lens.sort_unstable();
    lens
}

/// A balanced hand: no void or singleton and at most one doubleton — i.e. one of
/// 4-3-3-3 / 4-4-3-2 / 5-3-3-2 (sorted lengths `[≥2, ≥3, _, _]`).
fn is_balanced(hand: Hand) -> bool {
    let lens = sorted_lengths(hand);
    lens[0] >= 2 && lens[1] >= 3
}

/// The size-ask class: a balanced eight with no four-card major.
fn is_size_ask(hand: Hand) -> bool {
    hand_hcp(hand) == 8
        && hand[Suit::Hearts].len() < 4
        && hand[Suit::Spades].len() < 4
        && is_balanced(hand)
}

/// The 1NT opener's seat: the seat that made the first (opening) bid.  On a
/// divergent board that opening is always 1NT (only `1NT - 2♠` auctions feel the
/// accept floor), and the opener holds 15 or 16 — the two strengths where floor 15
/// and floor 17 split.  The auction may sit on either axis, so this is *not*
/// restricted to North/South; `ns_score_*` prices the contract NS-relative anyway.
fn opener_seat(auction: &Auction, dealer: Seat) -> Option<Seat> {
    (0..auction.len())
        .find(|&i| matches!(auction[i], Call::Bid(_)))
        .map(|i| seat_to_act(dealer, i))
}

/// Tag a divergent board with `(opener HCP, responder type)`.  Opener 15 is the
/// falsification control (accept-15 should decline), opener 16 the live question.
fn board_tag(deal: &FullDeal, auction: &Auction, dealer: Seat) -> Option<(u8, Class)> {
    let opener = opener_seat(auction, dealer)?;
    let responder = deal[opener.partner()];
    let class = if is_size_ask(responder) {
        Class::Invite
    } else if responder[Suit::Clubs].len() >= 6 {
        Class::Club
    } else {
        Class::Other
    };
    Some((hand_hcp(deal[opener]), class))
}

/// The (contract, declarer, leader-view inferences) of one auction, read through
/// `stance`; `None` for a pass-out (sd score 0).  Mirrors `ab-size-ask-eight`.
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
/// the subset of divergent boards flagged by `keep`.
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

    // arm 0 = accept (floor 15: accept every 15-17 minimum), arm 1 = decline
    // (floor 17, shipped).  Divergent boards then split by opener HCP: 15 (the
    // falsification control — accept-15 should lose) and 16 (the live question).
    // 17 accepts in both arms, so never diverges.  The floor is baked into the
    // book, so build each arm from its own agreements; the tries are independent
    // thereafter.
    let mut arm = pons::bidding::agreements::Agreements::default();
    arm.notrump.size_ask_accept_floor = 15;
    let accept = american(&arm).against();
    arm.notrump.size_ask_accept_floor = 17;
    let decline = american(&arm).against();
    let stances = [accept, decline];

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

    // Plain-DD and DD-PD swings (Decline − Accept = arm1 − arm0).
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

    // Single-dummy pass: read each arm's auction for the leader, choose the opening
    // lead single-dummy over `sd_worlds` sampled worlds, play double-dummy, score
    // the trick count both plain (`ns_score_tricks`) and SD-PD (`ns_score_pd_tricks`).
    // Main thread only — the DD solve above released the solver lock.
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

    // (opener HCP, responder type) tag per divergent board; None off the size ask.
    let tag: Vec<Option<(u8, Class)>> = (0..args.count)
        .map(|i| board_tag(&deals[i], &bids[i][0].0, Seat::ALL[i % 4]))
        .collect();
    let is = |i: usize, hcp: u8, class: Class| tag[i] == Some((hcp, class));

    println!(
        "=== size-ask accept-floor A/B (Decline − Accept): {} boards, vulnerability {}, seed {} ===",
        args.count, vul, base,
    );
    println!(
        "(opponents silenced; + ⇒ declining wins ⇒ accepting is bad. opener 15 = control \
         (should be +), 16 = question. sd {} worlds, sd seed {})",
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
    let groups: [(&str, &dyn Fn(usize) -> bool); 6] = [
        ("all              ", &|_| true),
        ("op16 invite (8) ", &|i| is(i, 16, Class::Invite)),
        ("op16 club       ", &|i| is(i, 16, Class::Club)),
        ("op15 invite CTRL", &|i| is(i, 15, Class::Invite)),
        ("op15 club       ", &|i| is(i, 15, Class::Club)),
        ("other/off-axis  ", &|i| {
            tag[i].is_none_or(|(_, c)| c == Class::Other)
        }),
    ];
    for (group_label, keep) in &groups {
        let fired = divergent.iter().filter(|&&i| keep(i)).count();
        println!("--- {group_label} ({fired} divergent) ---");
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
