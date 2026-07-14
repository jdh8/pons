//! Measure a point-count scale change: an A/B duplicate match.  Each board is
//! bid twice, duplicate style: at table A the candidate pair sits North/South
//! against a baseline pair (table B swaps seats), the per-seat knob flipped
//! just before each call.  Boards whose two auctions reach different
//! contracts are solved double dummy once and scored with plain DD and
//! perfect defense; `--sd` adds the blind-lead single-dummy bracket that sits
//! between the two (DD is too pessimistic on part-scores), crediting the
//! swing to the candidate team.
//!
//! Two treatments share the harness:
//!
//! - Default: the fit-known
//!   [`support_points`][pons::bidding::constraint::support_points] scale
//!   (shipped) vs legacy, via
//!   [`set_support_points`][pons::bidding::constraint::set_support_points].
//! - `--candidate hcp|rule`: the **global**
//!   [`PointScale`][pons::bidding::constraint::PointScale] deprecation A/B/C —
//!   every `points` gate, the sampler, and the floor's combined counts swap
//!   scale together per acting side, against `--baseline` (default legacy).
//!
//! `--deals <bank.pdd> --offset <rows>` bids a slice of a pre-solved binary
//! deal bank instead of seeded deals: plain DD and perfect defense then score
//! from the stored tables with **no live solving** (only `--sd` solves), so
//! million-board runs are bidding-bound.  `--show N` prints the worst
//! divergent boards and the first-divergence buckets for gate forensics.
//!
//! ```text
//! cargo run --example ab-point-count -- --count 1000 --vulnerability ns --seed "$SEED_BASE"
//! cargo run --example ab-point-count -- --count 1000 --sd --seed "$SEED_BASE"
//! cargo run --release --example ab-point-count -- --candidate rule \
//!     --deals /nfs2/jdh8/24.pdd --offset 0 --count 1000000 --show 20
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Contract, FullDeal, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::american;
use pons::bidding::constraint::{PointScale, set_point_scale, set_support_points};
use pons::bidding::context::relative;
use pons::bidding::{Family, Inferences, Stance, System};
use pons::scoring::{final_contract, imps, ns_score_contract};
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

    /// Candidate **global** point scale (the deprecation A/B/C); when omitted
    /// the runner measures the fit-known support_points scale instead
    #[arg(long, value_enum)]
    candidate: Option<Scale>,

    /// Baseline global point scale for the other side (with --candidate)
    #[arg(long, value_enum, default_value = "legacy")]
    baseline: Scale,

    /// Bid a slice of this pre-solved binary `.pdd` deal bank instead of
    /// seeded deals; plain DD/PD then score its stored tables, no live solve
    #[arg(long)]
    deals: Option<std::path::PathBuf>,

    /// Row offset into --deals — the slice ledger's cursor, never replayed
    #[arg(long, default_value_t = 0)]
    offset: u64,

    /// Print this many worst divergent boards (by candidate plain-DD loss)
    /// plus the first-divergence buckets over all divergent boards
    #[arg(long, default_value_t = 0)]
    show: usize,
}

/// A CLI name for each global [`PointScale`] arm
#[derive(Clone, Copy, PartialEq, Eq, Debug, clap::ValueEnum)]
enum Scale {
    /// Legacy raw HCP + upgrade (deposed 2026-07-14; the opt-out)
    Legacy,
    /// Raw Milton Work HCP
    Hcp,
    /// Rule of N+8: HCP + two longest suit lengths − 8 (opt-in since the
    /// 4333-floor A/B)
    Rule,
    /// Rule of N+8 floored at raw HCP: flat 4-3-3-3 keeps its HCP (the
    /// shipped default)
    RuleFloored,
}

impl From<Scale> for PointScale {
    fn from(scale: Scale) -> Self {
        match scale {
            Scale::Legacy => Self::PointCount,
            Scale::Hcp => Self::Hcp,
            Scale::Rule => Self::RuleOfN,
            Scale::RuleFloored => Self::RuleOfNFloored,
        }
    }
}

/// Which knob the duplicate match flips per acting side
#[derive(Clone, Copy)]
enum Arms {
    /// The fit-known `support_points` scale on/off (the shipped scale's A/B)
    SupportPoints,
    /// The global point scale: candidate vs baseline (the deprecation A/B/C)
    PointScale {
        candidate: PointScale,
        baseline: PointScale,
    },
}

impl Arms {
    /// Arm the acting side's knob (thread-local, set just before classifying)
    fn apply(self, is_candidate: bool) {
        match self {
            Self::SupportPoints => set_support_points(is_candidate),
            Self::PointScale {
                candidate,
                baseline,
            } => set_point_scale(if is_candidate { candidate } else { baseline }),
        }
    }
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
/// The ablation knob is thread-local and set just before each classification,
/// so this stays correct whether it runs on the main thread or a rayon worker
/// (each board bids on a single thread).
fn bid_out(
    stance: &Stance,
    arms: Arms,
    candidate_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();

    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        arms.apply(seat_is_ns == candidate_is_ns);
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

/// Forensics for `--show`: rank divergent boards by candidate plain-DD swing
/// and bucket them all by the first differing call — a remnant gate shows up
/// as a bucket where the baseline side keeps winning.
fn show_divergences(
    show: usize,
    divergent: &[usize],
    tables: &[TrickCountTable],
    auctions: &[AuctionPair],
    contracts: &[ContractPair],
    deals: &[FullDeal],
    vul: AbsoluteVulnerability,
) {
    let reached = |contract: Option<(Contract, Seat)>| {
        contract.map_or_else(|| "pass-out".to_owned(), |(c, s)| format!("{c} by {s}"))
    };

    // (candidate IMP swing, position in `divergent`), plus per-bucket
    // (count, IMP total, IMP sum of squares) for a per-bucket mean ± CI —
    // the remnant criterion is a negative bucket whose CI clears zero.
    let mut ranked: Vec<(i64, usize)> = Vec::with_capacity(divergent.len());
    let mut buckets: std::collections::HashMap<String, (usize, i64, i64)> =
        std::collections::HashMap::new();
    for (position, (&i, table)) in divergent.iter().zip(tables).enumerate() {
        let swing = imps(
            ns_score_contract(contracts[i][1], table, vul)
                - ns_score_contract(contracts[i][0], table, vul),
        );
        ranked.push((swing, position));

        let [off, on] = &auctions[i];
        let split = (0..off.len().min(on.len()))
            .find(|&n| off[n] != on[n])
            .unwrap_or_else(|| off.len().min(on.len()));
        let prefix = off
            .iter()
            .take(split)
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        let next = |auction: &Auction| {
            auction
                .get(split)
                .map_or_else(|| "(end)".to_owned(), ToString::to_string)
        };
        let entry = buckets
            .entry(format!("[{prefix}] {} → {}", next(off), next(on)))
            .or_insert((0, 0, 0));
        entry.0 += 1;
        entry.1 += swing;
        entry.2 += swing * swing;
    }

    ranked.sort_unstable();
    println!("\nWorst divergent boards for the candidate (plain DD):");
    for &(swing, position) in ranked.iter().take(show) {
        let i = divergent[position];
        let [off, on] = &auctions[i];
        println!(
            "board {i} ({swing:+} IMPs) dealer {} deal {}\n  off: {off} = {}\n   on: {on} = {}",
            Seat::ALL[i % 4],
            deals[i],
            reached(contracts[i][0]),
            reached(contracts[i][1]),
        );
    }

    let mut sorted: Vec<(&String, &(usize, i64, i64))> = buckets.iter().collect();
    // Tiebreak on the key so equal totals print in a deterministic order.
    sorted.sort_by_key(|&(key, &(_, total, _))| (total, key));
    println!("\nFirst-divergence buckets (worst {show} by candidate IMPs; off-call → on-call):");
    for &(key, &(n, total, sumsq)) in sorted.iter().take(show) {
        // Per-bucket 95% CI on the mean swing per divergent board.
        #[allow(clippy::cast_precision_loss)]
        let (n_f, total_f, sumsq_f) = (n as f64, total as f64, sumsq as f64);
        let mean = total_f / n_f;
        let var = (sumsq_f - n_f * mean * mean) / (n_f - 1.0).max(1.0);
        let ci = 1.96 * (var / n_f).sqrt();
        // A remnant candidate: legacy keeps winning and the CI clears zero.
        let flag = if mean + ci < 0.0 { "  ⚠ remnant" } else { "" };
        println!("{total:+7} IMPs  ×{n:<6} {mean:+.2} ± {ci:.2}  {key}{flag}");
    }
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

    let arms = match args.candidate {
        Some(candidate) => Arms::PointScale {
            candidate: candidate.into(),
            baseline: args.baseline.into(),
        },
        None => Arms::SupportPoints,
    };

    // Deal source: the seeded stream (base + index) so every arm/vul replays
    // the identical stream, or a slice of a pre-solved .pdd deal bank whose
    // stored DD tables replace the live solve below (arms then pair by
    // sharing the same --offset instead of the same seed).
    let (deals, stored): (Vec<FullDeal>, Option<Vec<TrickCountTable>>) = match &args.deals {
        Some(path) => {
            let rows = pons::pdd::load_slice(path, args.offset, args.count)
                .unwrap_or_else(|e| panic!("load {}: {e}", path.display()));
            assert_eq!(
                rows.len(),
                args.count,
                "deal bank exhausted: {} rows from offset {}",
                rows.len(),
                args.offset,
            );
            let (deals, tables) = rows.into_iter().unzip();
            (deals, Some(tables))
        }
        None => (seeded_deals(base, args.count), None),
    };

    // Each bid_out sets its own thread-local per call, so board bidding
    // parallelizes; the DD solver stays on the main thread below.  Retain both
    // tables' auctions (index 0 = table_b/off, 1 = table_a/on — the same order
    // as `contracts`) so the single-dummy pass can read each auction from the
    // leader's view.
    let (auctions, contracts): (Vec<AuctionPair>, Vec<ContractPair>) = deals
        .par_iter()
        .enumerate()
        .map(|(index, deal)| {
            let dealer = Seat::ALL[index % 4];
            let table_a = bid_out(&stance, arms, true, dealer, vul, deal);
            let table_b = bid_out(&stance, arms, false, dealer, vul, deal);
            // Credit the candidate team: [off = table_b (candidate EW),
            // on = table_a (candidate NS)], matching report_brackets' on − off.
            let contracts = [
                final_contract(&table_b, dealer),
                final_contract(&table_a, dealer),
            ];
            ([table_b, table_a], contracts)
        })
        .unzip();

    // Only boards whose tables reach different results can swing; look their
    // tables up in the deal bank, or solve them once, and score both brackets
    // (plain DD + perfect defense) from the same tables.
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| contracts[i][0] != contracts[i][1])
        .collect();
    let tables: Vec<TrickCountTable> = match &stored {
        Some(all) => divergent.iter().map(|&i| all[i]).collect(),
        None => {
            let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
            Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL)
        }
    };

    // The deal seed is meaningless (and randomly noisy) when a deal bank
    // supplies the boards; the bank line below carries the slice instead.
    if args.deals.is_some() {
        println!(
            "=== Point-count A/B match: {} boards, vulnerability {} ===",
            args.count, vul,
        );
    } else {
        println!(
            "=== Point-count A/B match: {} boards, vulnerability {}, seed {} ===",
            args.count, vul, base,
        );
    }
    match arms {
        Arms::SupportPoints => println!("arms: fit-known support_points on vs off"),
        Arms::PointScale { .. } => println!(
            "arms: global point scale {:?} vs {:?} baseline",
            args.candidate.expect("PointScale arms imply --candidate"),
            args.baseline,
        ),
    }
    if let Some(path) = &args.deals {
        println!(
            "deal bank: {} rows {}..{} (stored DD tables, no live solve)",
            path.display(),
            args.offset,
            args.offset + args.count as u64,
        );
    }
    println!(
        "Divergent boards: {} of {} ({:.2}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );

    report_brackets(args.count, &divergent, &tables, &contracts, vul);

    if args.show > 0 {
        show_divergences(
            args.show, &divergent, &tables, &auctions, &contracts, &deals, vul,
        );
    }

    if args.sd {
        // One default-flag reading serves both arms (see infer_stance above);
        // pin the main thread back to the shipped defaults in case rayon ran a
        // bid_out here and left an arm's knob set.
        set_support_points(true);
        set_point_scale(PointScale::RuleOfNFloored);
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
