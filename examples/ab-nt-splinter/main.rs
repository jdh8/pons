//! `1NT - 3♥/3♠` splinter A/B: the shipped empty slot vs the Polish Club treatment.
//!
//! Responder's `3♥`/`3♠` over our 1NT are the only two slots the response ladder
//! leaves empty. [`nt_splinter`][field@pons::bidding::inference::ReadingProfile::nt_splinter]
//! fills them with the Bridge World Standard /
//! Polish Club agreement — shortness in the **bid** major, 2–3 in the other,
//! exactly four diamonds, five or six clubs — whose core hand (`3-1-4-5` and
//! mirrors) has no home at all in the shipped system: too few majors for
//! Stayman, too few diamonds for the `2NT` transfer, too few clubs for the `2♠`
//! transfer, and not `balanced()` for Puppet `3♣`, so at 9+ HCP it blasts `3NT`
//! and at 8 it *passes* 1NT holding a singleton opposite 15-17.
//!
//! Opponents are silenced (East/West always pass), so every auction is
//! constructive start to finish. That is deliberate rather than merely
//! convenient: this convention is measured **in self-play**, not against BBA,
//! because there is no `--advertise-splinter` and BBA's own `1N-3M splinter`
//! toggle is the *GIB* convention (other major pinned at exactly four). A BBA
//! opponent would defend our `3♥` expecting five hearts, and the gain would be
//! partly an exploit of a misinformed opponent — the artifact that got the
//! passed-hand 1NT defense reverted.
//!
//! Each board is bid twice over the same deal, once per arm; boards whose arms
//! reach different contracts are solved double dummy once and scored. A positive
//! IMPs/board favors the splinter (the opt-in arm).
//!
//! The slot is thin by construction, so **IMPs/fired is the primary read** and
//! IMPs/board is the ship gate. Expected fire rate ≈0.11% of boards: the shape
//! mass is `5-4-3-1` (12.93%/24 arrangements × 2 majors ≈ 1.08%) plus `6-4-2-1`
//! (≈0.39%) plus `6-4-3-0` (≈0.11%), times the `hcp(9..)` floor and the
//! low-singleton exclusion, and times the chance partner opened 1NT.
//!
//! ```text
//! cargo run --release --example ab-nt-splinter -- --count 200000 --seed "$SEED_BASE"
//! ```
//!
use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Contract, FullDeal, Seat, Strain};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::context::relative;
use pons::bidding::{Inferences, Partnership};
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
use common::{bid_uncontested, report_sd_brackets, seat_to_act};

/// 1NT - 3M splinter A/B: the Polish Club treatment vs the shipped empty slot
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "5000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Seed for the deal RNG — share one `SEED_BASE` across an experiment's arms
    #[arg(long, default_value_t = 20_240_607)]
    seed: u64,

    /// Responder's HCP floor for the splinter (the 8-vs-9 sweep arm)
    #[arg(long, default_value_t = 9)]
    floor: u8,

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

/// Whether this uncontested auction actually contains the splinter
///
/// Opponents are silenced, so the auction is a run of passes, then the opening,
/// then alternating passes and our calls. Responder's first action is therefore
/// two calls after the opening — the splinter is a `3♥`/`3♠` there over a 1NT.
fn splinter_fired(auction: &Auction) -> bool {
    let Some(open) = auction.iter().position(|call| *call != Call::Pass) else {
        return false;
    };
    let is_one_nt = matches!(
        auction.get(open),
        Some(Call::Bid(bid)) if bid.level.get() == 1 && bid.strain == Strain::Notrump
    );
    is_one_nt
        && matches!(
            auction.get(open + 2),
            Some(Call::Bid(bid))
                if bid.level.get() == 3
                    && matches!(bid.strain, Strain::Hearts | Strain::Spades)
        )
}

/// The (contract, declarer, leader-view inferences) of one auction, read through
/// `partnership`; `None` for a pass-out (sd score 0).  Mirrors `ab-notrump-minors`.
fn lead_inputs(
    auction: &Auction,
    partnership: &Partnership,
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
        partnership.infer(relative(vul, leader), &auction[..cut]),
    ))
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let mut rng = StdRng::seed_from_u64(args.seed);
    // arm 0 = off (the shipped system, slot empty), arm 1 = on (the splinter).
    // Both keep every other shipped default. The knob is read at book-
    // construction time, so build each arm under its own setting; the baked
    // tries are independent thereafter.
    let mut off_agreements = pons::bidding::agreements::Agreements::default();
    off_agreements.decision.reading.nt_splinter = false;
    let off = american(&off_agreements).bind();
    let mut armed = pons::bidding::agreements::Agreements::default();
    armed.decision.reading.nt_splinter = true;
    armed.notrump.nt_splinter_floor = args.floor;
    let on = american(&armed).bind();
    let partnerships = [off, on];

    // Both arms bid the same deal; the only difference is the two 3M rules.
    // Deal sequentially (cheap, and seeded so arms are reproducible), then bid
    // in parallel — bidding is pure (the books captured their agreements at
    // construction), so boards are independent and par_iter preserves order.
    // The DD solver stays on the main thread below.
    let deals: Vec<FullDeal> = (0..args.count).map(|_| full_deal(&mut rng)).collect();
    let vul = args.vulnerability;
    let bids: Vec<ArmBids> = deals
        .par_iter()
        .enumerate()
        .map(|(index, deal)| {
            let dealer = Seat::ALL[index % 4];
            std::array::from_fn(|arm| {
                let auction = bid_uncontested(&partnerships[arm], dealer, vul, deal);
                let contract = final_contract(&auction, dealer);
                (auction, contract)
            })
        })
        .collect();
    let contracts: Vec<[Option<(Contract, Seat)>; 2]> =
        bids.iter().map(|b| [b[0].1, b[1].1]).collect();

    // Boards where the convention actually fired — the denominator that matters
    // for a slot this thin. A fired board need not diverge (opener may land in
    // the same 3NT), and a divergent board is always a fired one here.
    let fired = bids.iter().filter(|b| splinter_fired(&b[1].0)).count();

    // Only boards whose arms diverge can swing; solve those once.
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| contracts[i][0] != contracts[i][1])
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
    let tables = Solver::lock(None).solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

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
        "=== 1NT - 3M splinter A/B: {} boards, vulnerability {}, floor {}, seed {} ===",
        args.count, args.vulnerability, args.floor, args.seed,
    );
    println!("(opponents silenced — constructive value only)");
    println!(
        "Fired: {fired} of {} ({:.3}%)   Divergent: {} ({:.3}%)",
        args.count,
        100.0 * fired as f64 / args.count.max(1) as f64,
        divergent.len(),
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "Splinter (vs empty slot): {points:+} points, {total_imps:+} IMPs ({:+.4} IMPs/board plain, {:+.2} IMPs/fired)",
        total_imps as f64 / args.count.max(1) as f64,
        total_imps as f64 / fired.max(1) as f64,
    );
    println!(
        "                          {pd_imps:+} IMPs ({:+.4} IMPs/board PD, {:+.2} IMPs/fired)",
        pd_imps as f64 / args.count.max(1) as f64,
        pd_imps as f64 / fired.max(1) as f64,
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
                    lead_inputs(&bids[i][arm].0, &partnerships[arm], dealer, vul)
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
        let mut on_score = vec![[0i64; 2]; args.count];
        let mut off_score = vec![[0i64; 2]; args.count];
        const CHUNK: usize = 4096;
        for (asked, chunk) in pending.chunks(CHUNK).zip(questions.chunks(CHUNK)) {
            let answers = single_dummy_leads(chunk, &mut rng, args.sd_worlds);
            for (&(i, arm_on, contract, declarer), &(_, tricks)) in asked.iter().zip(&answers) {
                let t = u8::from(tricks);
                let score = [
                    ns_score_tricks(contract, declarer, t, vul),
                    ns_score_pd_tricks(contract, declarer, t, vul),
                ];
                if arm_on {
                    on_score[i] = score;
                } else {
                    off_score[i] = score;
                }
            }
        }
        report_sd_brackets(
            "sd-lead splinter",
            args.sd_worlds,
            args.sd_seed,
            &on_score,
            &off_score,
            divergent.len(),
        );
    }
}
