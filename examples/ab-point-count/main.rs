//! Measure the fit-known
//! [`support_points`][pons::bidding::constraint::support_points] scale: an A/B
//! duplicate match of legacy [`point_count`][pons::bidding::constraint::point_count]
//! (shipped) against `hcp_plus` (HCP plus useful shortness) plus the bare
//! long-suit length term, wired into the fit-known gates only.  Each board is
//! bid twice, duplicate style: at table A the candidate pair sits North/South
//! against a pair on the shipped scale (table B swaps seats). The
//! [`set_support_points`][pons::bidding::constraint::set_support_points]
//! ablation hook flips the scale per acting side. Boards whose two auctions
//! reach different contracts are solved double dummy once and scored with
//! plain DD and perfect defense; `--sd` adds the blind-lead single-dummy
//! bracket that sits between the two (DD is too pessimistic on part-scores),
//! crediting the swing to the candidate team.
//!
//! ```text
//! cargo run --example ab-point-count -- --count 1000 --vulnerability ns --seed "$SEED_BASE"
//! cargo run --example ab-point-count -- --count 1000 --sd --seed "$SEED_BASE"
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Contract, FullDeal, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::constraint::set_support_points;
use pons::bidding::context::relative;
use pons::bidding::{Family, Inferences, Stance, System};
use pons::scoring::{final_contract, imps};
use pons::single_dummy::{LeadQuestion, single_dummy_leads};

use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{mean_with_ci, report_brackets, seat_to_act, seeded_deals};

/// Measure the candidate point-count scale: an A/B duplicate match
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "200")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Base seed — fresh per experiment (`SEED_BASE=$(date +%s)`), shared
    /// across arms/vuls; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,

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

/// Bid out one deal, switching the point-count scale per acting side
///
/// The ablation flag is thread-local and set just before each classification,
/// so this stays correct whether it runs on the main thread or a rayon worker
/// (each board bids on a single thread).
fn bid_out(
    stance: &Stance,
    candidate_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();

    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        set_support_points(seat_is_ns == candidate_is_ns);
        auction.push(next_call(stance, deal[seat], dealer, vul, &auction));
    }
    auction
}

/// One board's two tables' auctions, `[table_b (off), table_a (on)]`.
type AuctionPair = [Auction; 2];

/// One board's two tables' reached contracts, `[off, on]` — same order as
/// [`AuctionPair`], so the DD/PD and single-dummy paths line up.
type ContractPair = [Option<(Contract, Seat)>; 2];

/// Signed-for-NS score of a contract given declarer's (single-dummy) tricks.
/// Copied from `ab-fuzzy-strength` (the promotion to `src/scoring.rs` is a TODO).
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
/// `stance`; `None` for a pass-out (sd score 0).  Mirrors `ab-fuzzy-strength`.
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
    let stance = american().against(Family::NATURAL);
    // One default-flag book reads the leader's view for the blind-lead pass: the
    // point-count scale barely shifts disclosed meaning, so a single reading
    // serves both arms (a deliberate simplification — we do not flip the scale
    // for inference, unlike per-call bidding above).
    let infer_stance = american().against(Family::NATURAL);

    // Deals are seeded per board (base + index) so every arm/vul replays the
    // identical stream. Each bid_out sets its own thread-local per call, so
    // board bidding parallelizes; the DD solver stays on the main thread below.
    // Retain both tables' auctions (index 0 = table_b/off, 1 = table_a/on — the
    // same order as `contracts`) so the single-dummy pass can read each auction
    // from the leader's view.
    let deals = seeded_deals(base, args.count);
    let (auctions, contracts): (Vec<AuctionPair>, Vec<ContractPair>) = deals
        .par_iter()
        .enumerate()
        .map(|(index, deal)| {
            let dealer = Seat::ALL[index % 4];
            let table_a = bid_out(&stance, true, dealer, vul, deal);
            let table_b = bid_out(&stance, false, dealer, vul, deal);
            // Credit the candidate team: [off = table_b (candidate EW),
            // on = table_a (candidate NS)], matching report_brackets' on − off.
            let contracts = [
                final_contract(&table_b, dealer),
                final_contract(&table_a, dealer),
            ];
            ([table_b, table_a], contracts)
        })
        .unzip();

    // Only boards whose tables reach different results can swing; solve those
    // once and score both brackets (plain DD + perfect defense) from the tables.
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| contracts[i][0] != contracts[i][1])
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    println!(
        "=== Point-count A/B match: {} boards, vulnerability {}, seed {} ===",
        args.count, vul, base,
    );
    println!(
        "Divergent boards: {} of {} ({:.2}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );

    report_brackets(args.count, &divergent, &tables, &contracts, vul);

    if args.sd {
        // Blind-lead pass: on each divergent board price both arms' auctions —
        // the opening lead is chosen single-dummy over `sd_worlds` sampled worlds
        // (read from the leader's view through the default-flag book), then play
        // is double-dummy on the actual deal. Main thread only — the solver is
        // not reentrant, and the plain/PD solve above has already released it.
        let mut pending: Vec<(usize, bool, Contract, Seat)> = Vec::new();
        let mut questions: Vec<LeadQuestion> = Vec::new();
        for &i in &divergent {
            let dealer = Seat::ALL[i % 4];
            // (is_on, table index): 1 = table_a/on (candidate NS), 0 = table_b/off.
            for (is_on, idx) in [(true, 1usize), (false, 0usize)] {
                if let Some((contract, declarer, inferences)) =
                    lead_inputs(&auctions[i][idx], &infer_stance, dealer, vul)
                {
                    pending.push((i, is_on, contract, declarer));
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
            for (&(i, is_on, contract, declarer), &(_, tricks)) in asked.iter().zip(&answers) {
                let score = ns_score_tricks(contract, declarer, u8::from(tricks), vul);
                if is_on {
                    on_score[i] = score;
                } else {
                    off_score[i] = score;
                }
            }
        }
        // Positive = candidate team (ON, sitting NS at table A) gains under the
        // blind lead. ns_score_tricks already flips sign for an EW declarer, so
        // on_score − off_score credits the candidate exactly as the DD path's
        // [table_b (off), table_a (on)] ordering does.
        let board_imps: Vec<i64> = (0..args.count)
            .map(|i| imps(on_score[i] - off_score[i]))
            .collect();
        let (mean, ci) = mean_with_ci(&board_imps);
        let total: i64 = board_imps.iter().sum();
        println!(
            "sd-lead candidate ({} worlds, seed {}): {total:+} IMPs, {mean:+.4} IMPs/board [95% CI ±{ci:.4}], {:+.3} IMPs/divergent",
            args.sd_worlds,
            args.sd_seed,
            total as f64 / divergent.len().max(1) as f64,
        );
    }
}
