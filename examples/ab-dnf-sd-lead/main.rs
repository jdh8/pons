//! DNF sd-lead A/B: the **same** auction scored four ways — the
//! [`pons::bidding::ReadingProfile::envelope_union`] ×
//! [`pons::bidding::ReadingProfile::gauge_membership`]
//! matrix (off / dnf / gauge /
//! both), all in one process so every arm prices the identical lead question.
//!
//! Both knobs are inert for the floor+net bidder — it never samples during
//! bidding (`sample_layouts` is reached only by `ev_all` and this sd-lead
//! scorer). So their value surfaces only where a sampler runs. The sd-lead
//! scorer is the shipped one: the leader (declarer's LHO) picks a card
//! single-dummy over `sd_worlds` auction-consistent layouts, then play is
//! double-dummy on the real deal (`single_dummy_leads`). `dnf` keeps a
//! disjunctive reading's separate boxes (a two-suiter overcall pins to a true
//! shape instead of the bounding box); `gauge` makes the sampled hands respect
//! the raw-HCP and support-points bands too (a 15–17 1NT stops admitting
//! 13-counts the `points` scale upgraded) — independent mechanisms, hence the
//! matrix.
//!
//! Bidding is **contested** — all four seats bid `american()` — so the default-on
//! two-suiter overcalls (Michaels, Unusual NT) fire and the leader actually reads
//! a disjunction. The auction is identical in every arm (the bidder is
//! knob-independent); only the leader's sampled model changes, so an sd swing is
//! pure reading value. Uncontested american barely disjoins, hence contested here.
//!
//! **No SD-PD bracket here, deliberately.** Every other sd harness reports the
//! same trick count twice — plain ([`ns_score_tricks`][pons::scoring::ns_score_tricks])
//! and perfect-defense ([`ns_score_pd_tricks`][pons::scoring::ns_score_pd_tricks],
//! a contract failing on those tricks priced doubled) — because plain SD
//! flatters aggression. That correction is meaningless *here*: all four arms
//! reach the identical contract, and for a fixed contract both scorers are
//! nondecreasing step functions of tricks, so every board's swing keeps its
//! sign under PD and only magnitudes change. It could never produce a
//! per-board disagreement, and it would misattribute the opponents' doubling
//! judgment to the leader's world model. A verdict resting on this harness
//! alone therefore needs a **plain+PD** re-adjudication (`scripts/ab-dnf-flip2.sh`),
//! not an SD-PD row.
//!
//! ```text
//! cargo run --release --example ab-dnf-sd-lead -- --count 20000 --sd-worlds 16
//! cargo run --release --example ab-dnf-sd-lead -- --count 20000 --vulnerability both
//! ```

use clap::Parser;
use contract_bridge::auction::Auction;
use contract_bridge::{AbsoluteVulnerability, Contract, Seat};
use pons::american;
use pons::bidding::context::relative;
use pons::bidding::{Inferences, Stance};
use pons::scoring::{final_contract, imps, ns_score_tricks};
use pons::single_dummy::{LeadQuestion, single_dummy_leads};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_out, mean_with_ci, seat_to_act, seeded_deals};

/// DNF sd-lead A/B: one contested auction, its opening lead priced off vs on
#[derive(Parser)]
struct Args {
    /// Number of boards (dealer rotates per board)
    #[arg(short, long, default_value = "20000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Deal seed — fresh per experiment (`SEED_BASE=$(date +%s)`), shared across
    /// vuls; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,

    /// Worlds sampled per blind lead (the validated GTO setting is 16)
    #[arg(long, default_value_t = 16)]
    sd_worlds: usize,

    /// Seed for the world-sampling RNG (report it to reproduce a run)
    #[arg(long, default_value_t = 20_240_607)]
    sd_seed: u64,
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

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = args.vulnerability;
    let stance = american(&pons::bidding::agreements::Agreements::current()).against();
    let deals = seeded_deals(base, args.count);

    // Bidding is knob-independent (the floor+net bidder never samples), so bid
    // once.  All four seats bid `american` (both `bid_out` stances identical), so
    // the auction is fully contested and two-suiter overcalls fire.
    let auctions: Vec<Auction> = deals
        .par_iter()
        .enumerate()
        .map(|(i, deal)| bid_out(&stance, &stance, true, Seat::ALL[i % 4], vul, deal))
        .collect();

    // Price the opening lead for one arm: read the leader's inferences and
    // sample worlds *under the arm's knobs* (disjunctive boxes and/or gauge
    // membership).  Both knobs are captured into a stance at build, so the arm
    // gets its own `reader`; the sampler inside `single_dummy_leads` runs on the
    // main thread and follows the `Inferences` that reader produced.
    let score_arm = |envelope_union: bool, gauge: bool| -> Vec<i64> {
        let mut agreements = pons::bidding::agreements::Agreements::current();
        agreements.decision.reading.envelope_union = envelope_union;
        agreements.decision.reading.gauge_membership = gauge;
        let reader = american(&agreements).against();
        let mut pending: Vec<(usize, Contract, Seat)> = Vec::new();
        let mut questions: Vec<LeadQuestion> = Vec::new();
        for (i, auction) in auctions.iter().enumerate() {
            let dealer = Seat::ALL[i % 4];
            if let Some((contract, declarer, inferences)) =
                lead_inputs(auction, &reader, dealer, vul)
            {
                pending.push((i, contract, declarer));
                questions.push(LeadQuestion {
                    deal: deals[i],
                    strain: contract.bid.strain,
                    declarer,
                    inferences,
                });
            }
        }
        let mut rng = StdRng::seed_from_u64(args.sd_seed);
        let mut score = vec![0i64; args.count];
        const CHUNK: usize = 4096;
        for (asked, chunk) in pending.chunks(CHUNK).zip(questions.chunks(CHUNK)) {
            let answers = single_dummy_leads(chunk, &mut rng, args.sd_worlds);
            for (&(i, contract, declarer), &(_, tricks)) in asked.iter().zip(&answers) {
                score[i] = ns_score_tricks(contract, declarer, u8::from(tricks), vul);
            }
        }
        score
    };

    let off = score_arm(false, false);
    let union = score_arm(true, false);
    let gauge = score_arm(false, true);
    let both = score_arm(true, true);

    println!(
        "=== DNF×gauge sd-lead A/B (contested): {} boards, vulnerability {}, deal seed {} ===",
        args.count, vul, base,
    );
    println!(
        "(knob matrix on one bid-out; bidding identical — only the leader's sampled model changes; {} worlds, sd seed {})",
        args.sd_worlds, args.sd_seed,
    );

    // NS-signed sd score, so IMPs read from NS's chair; a defensive gain on a
    // board NS defends and a declaring gain where NS declares both count.
    let report = |label: &str, arm: &[i64], baseline: &[i64]| {
        let board_imps: Vec<i64> = (0..args.count)
            .map(|i| imps(arm[i] - baseline[i]))
            .collect();
        let divergent = board_imps.iter().filter(|&&d| d != 0).count();
        let (mean, ci) = mean_with_ci(&board_imps);
        let total: i64 = board_imps.iter().sum();
        println!(
            "{label}: {total:+} IMPs, {mean:+.4} IMPs/board [95% CI ±{ci:.4}], divergent {divergent} ({:.2}%)",
            100.0 * divergent as f64 / args.count.max(1) as f64,
        );
    };
    report("dnf   vs off", &union, &off);
    report("gauge vs off", &gauge, &off);
    report("both  vs off", &both, &off);
    report("both  vs dnf", &both, &union);
}
