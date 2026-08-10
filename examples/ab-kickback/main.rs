//! Measure the keycard ladder's relocated ask (Kickback/Redwood) as a
//! duplicate A/B match.
//!
//! Three arms over two axes, so each coupled change stays attributable:
//!
//! | arm | `keycard_minors` | `ReadingProfile::rkcb_variant` | what it is |
//! |---|---|---|---|
//! | `plain` | off | plain | majors-only trump, the ask is 4NT |
//! | `minors` | on | plain | minor asks at plain 4NT — round 4's losing arm, re-priced |
//! | `kickback` | on | kickback | opt-in again since the gate-2 loss: 4♦/4♥ Redwood, 4♠ over hearts |
//!
//! Every arm stands on the configured (v4) floor, which is now the only floor
//! `american()` has — so an arm differs from another by a **convention card
//! row**, never by a weights artifact.  That is the separation
//! `docs/ai-bidder/configured-net.md` was written to buy, and it is now
//! structural rather than opt-in.  `--feature kickback --baseline minors` is
//! gate 2 (what the relocation is worth, alone; measured a loss).  **Gate 1**
//! — `v4 − minors`, which priced the floor swap itself — is no longer
//! expressible: its baseline *was* the v3 artifact, deleted with the twins.
//! It reproduces at `f719be9`.
//!
//! Gate 2 is the fair comparison the first three arms cannot give.  Under the v3
//! floor the kickback partnership swaps the weights as well as the rules, so `kickback −
//! minors` prices a convention *and* a differently-trained net — §7.12 measured
//! 93.5% of its divergent boards with no keycard ask by either side.  The v4 arms
//! share one artifact and differ by the `Kickback 1430` row of the card they are
//! handed, so the confound is gone by construction rather than corrected for.
//!
//! Both v4 arms are built for the **mixed table** they actually play: each side's
//! floor is handed its own card *and its opponents'*, which is the asymmetric
//! cell the v4 corpus was dumped to cover.
//!
//! ⚠ Expect divergence away from keycard auctions in gate 2. Flipping one card
//! row perturbs the net's logits on *every* decision, not only on asks — the
//! corpus teaches that the bit is usually irrelevant, which drives the weight
//! toward zero without pinning it there. The census below is what separates
//! that noise from the relocation; read the `no keycard ask` bucket as the
//! measurement's own error bar.
//!
//! The answer gates and the queen relay were arms here until 2026-08-02, when
//! both stopped being knobs — the gates because "off" is a poisoned reading
//! rather than a playable agreement, the relay because there is no partnership
//! that hears a keycard answer and declines to find out about the queen.  Their
//! arms are gone with them; the ledger keeps the cells they measured.
//!
//! For the relocation: `kickback − plain` is its ship decision and runs first;
//! `kickback − minors` prices the relocation on its own; `minors − plain`
//! re-prices the majors-only carve, and is only worth machine time if the first
//! pair is not a clean win.
//!
//! Why the relocation should pay: count the 1430 answers that overshoot five of
//! trump under a plain 4NT ask — three of four in clubs (5♦/5♥/5♠), two of four
//! in diamonds (5♥/5♠), one of four in hearts (5♠), none in spades.  Every
//! relocated ask brings that to zero, because the answers are *steps above the
//! ask*.  The interesting boards are therefore the ones where the answer used to
//! land past the trump suit's own five level and the asker had nowhere to stop.
//!
//! **Both knob regimes must be armed.** `ReadingProfile::rkcb_variant` gates rule *presence* at
//! build time — the reading's `alerted` test is structural, so an always-present
//! alerted rule on 4♥/4♠ would suppress the natural reading of those calls even
//! in the off arm — *and* the recognizers at classification time.  Both halves
//! are captured into the partnership at build, so one partnership per arm carries the
//! whole arm: the bidding loop picks by side and arms nothing.
//!
//! `--dump DIR` writes the divergent boards as a `common::Dump` shard —
//! identical contracts swing zero under **every** scorer, so divergence is
//! the only thing worth disk, and one shard makes the whole existing rescore
//! toolchain apply to a cell that used to evaporate with its stdout.
//! `--rescore DIR` then re-prices a dumped cell without re-bidding: the
//! ask-bucketed census rows under every instrument at once — plain DD,
//! perfect defense, both single-dummy endpoints, their calibrated λ-blend
//! (`common::sd_blend_imps`), and the analytic Pavlicek shave — which is the
//! attribution §7.13's 400k *aggregate* sd row could not give (92% of gate
//! 2's divergent boards saw no keycard ask; the aggregate mostly priced
//! card-row perturbation of the net, not the relocation).
//!
//! ```text
//! cargo run --release --features serde --example ab-kickback -- \
//!     --feature kickback --baseline plain --count 10000000 --sd
//! cargo run --release --features serde --example ab-kickback -- \
//!     --feature kickback --baseline minors --count 2000000 \
//!     --dump /mnt/hdd-data/jdh8/pons-ab-results/kickback-cell-nv  # gate 2
//! cargo run --release --features serde --example ab-kickback -- \
//!     --rescore /mnt/hdd-data/jdh8/pons-ab-results/kickback-cell-nv --show 999999
//! ```

use clap::{Parser, ValueEnum};
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::Accumulator;
use pons::bidding::Partnership;
use pons::bidding::american::american_with_config;
use pons::bidding::card::{Card, american_card};
use pons::bidding::features::Config;
use pons::bidding::instinct::{RkcbVariant, keycard_ask_at, kickback_offered_at};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, next_call, seat_to_act, seeded_deals};
use std::path::PathBuf;

/// One arm of the experiment: which knobs it arms
///
/// The `v4`/`v4-kickback` arms are gone with the twin artifacts: every arm now
/// stands on the configured floor, so `minors` *is* the old `v4` and `kickback`
/// *is* the old `v4-kickback`.  Gate 1 (`v4 − minors`) priced the floor swap and
/// is no longer expressible here; it reproduces at `f719be9`.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Arm {
    /// Today's system: majors-only keycard trump, the ask is 4NT
    Plain,
    /// Minor asks at plain 4NT — round 4's losing arm, re-priced
    Minors,
    /// Full Kickback: minor asks relocated to 4♦/4♥ (Redwood), hearts to 4♠
    Kickback,
}

impl Arm {
    /// Whether this arm lifts `keycard_trump`'s majors-only carve
    const fn minors(self) -> bool {
        matches!(self, Self::Minors | Self::Kickback)
    }

    /// The relocation stance this arm plays
    const fn variant(self) -> RkcbVariant {
        match self {
            Self::Kickback => RkcbVariant::Kickback,
            Self::Plain | Self::Minors => RkcbVariant::Plain,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Minors => "minors",
            Self::Kickback => "kickback",
        }
    }
}

/// Measure the relocated keycard ask (Kickback/Redwood): an A/B duplicate match
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "1000000")]
    count: usize,

    /// The arm under test
    #[arg(long, default_value = "kickback")]
    feature: Arm,

    /// The arm it is measured against
    #[arg(long, default_value = "plain")]
    baseline: Arm,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Deal seed base (board i seeded base+i; fresh per experiment, shared across arms)
    #[arg(long, default_value = "0")]
    seed: u64,

    /// Print this many divergent boards (auction + contracts) for inspection
    #[arg(long, default_value = "0")]
    show: usize,

    /// Add the sd-declarer playout row (blind lead + fallible declarer)
    #[arg(long, default_value_t = false)]
    sd: bool,

    /// Worlds per blind lead and per declarer decision (with --sd)
    #[arg(long, default_value_t = 16)]
    sd_worlds: usize,

    /// Seed for the sd world-sampling RNG (report it to reproduce a run)
    #[arg(long, default_value_t = 20_240_607)]
    sd_seed: u64,

    /// Write the divergent boards (only — identical contracts swing zero
    /// under every scorer) as a `common::Dump` shard under this directory
    #[arg(long)]
    dump: Option<PathBuf>,

    /// Re-price a `--dump` directory instead of bidding: the ask-bucketed
    /// census under every instrument (arms and vulnerability come from the
    /// dump; --sd-worlds/--sd-seed/--show still apply)
    #[arg(long, conflicts_with_all = ["dump", "feature", "baseline"])]
    rescore: Option<String>,

    /// With --rescore: price the sd endpoints only on boards where either
    /// arm made a keycard ask.  A fair cell's no-ask bucket is ~92% of the
    /// dump and pure twin noise, at ~0.5 s of playout per board; the DD-side
    /// instruments still cover it, and --show then details ask boards only
    #[arg(long, requires = "rescore", default_value_t = false)]
    sd_ask_only: bool,
}

/// Capture both halves of `arm` in one agreements value.
fn arm_agreements(arm: Arm) -> pons::bidding::agreements::Agreements {
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.decision.reading.rkcb_variant = arm.variant();
    agreements.decision.instinct.keycard_minors = arm.minors();
    agreements
}

/// The keycard ask `arm` made at one table, if any — the trump it asked in and
/// whether the ask was relocated.
///
/// **Both arms bid at every table.** The feature sits N-S at table A and E-W at
/// table B, so an auction is a conversation between the two arms and a scan
/// that ignores *who* called attributes the opponents' asks to whichever arm it
/// happened to arm.  `arm_is_ns` says where this arm sits at this table, and
/// only calls by that side are considered — with that arm's knobs set, since
/// `keycard_ask_at` reads them to recognise a relocation.
///
/// Scans the whole auction rather than a fixed ply: the ask sits wherever the
/// conversation reached it.  The first one wins, because a second ask on the
/// same auction is a later rung of the same conversation, which
/// `keycard_ask_at` already declines to call an ask.
fn table_ask(
    auction: &Auction,
    dealer: Seat,
    arm: Arm,
    arm_is_ns: bool,
) -> Option<(Suit, bool, bool)> {
    let profile = arm_agreements(arm).decision.reading;
    let calls: Vec<Call> = auction.iter().copied().collect();
    (0..calls.len()).find_map(|index| {
        let seat = seat_to_act(dealer, index);
        let north_south = matches!(seat, Seat::North | Seat::South);
        if north_south != arm_is_ns {
            return None;
        }
        keycard_ask_at(profile, &calls, index).map(|(_bid, trump, relocated)| {
            // A 4NT ask made while the ladder was offering a relocation is a
            // seat the convention was available for and did not get: the
            // authored book installs RKCB at absolute 4NT and has never been
            // relocated (bba-kickback.md phase 4, still `not started`).
            // Separating it from a lane where the ladder claims nothing is the
            // point — the first is a missed relocation, the second is correct.
            // Keyed to the ask's own trump: a spade ask belongs at 4NT, so a
            // claim in some other suit on the same face is not a miss.
            let offered = !relocated && kickback_offered_at(profile, &calls, index, trump);
            (trump, relocated, offered)
        })
    })
}

/// The keycard ask `arm` made on this board, at whichever table it made one.
fn arm_ask(board: &Board, arm: Arm, is_feature: bool) -> Option<(Suit, bool, bool)> {
    // The feature is N-S at table A and E-W at table B; the baseline mirrors it.
    table_ask(&board.table_a, board.dealer, arm, is_feature)
        .or_else(|| table_ask(&board.table_b, board.dealer, arm, !is_feature))
}

/// Per-trump attribution over the divergent boards.
///
/// Bucketing by the strain of the *final contract* — the first cut of this
/// analysis — conflates the lane the ask was made in with wherever the auction
/// landed, and double-counts a board whose two arms landed in different
/// strains.  Bucketing by the **ask** does neither, and it exposes the bucket
/// that decides how much of a cell is even attributable: boards where neither
/// arm asked for keycards at all, which under a knob that also swaps the
/// floor's weights is the net's contribution and nothing else.
fn per_trump_census(
    boards: &[Board],
    divergent: &[usize],
    swings_pd: &[i64],
    swings_dd: &[i64],
    feature: Arm,
    baseline: Arm,
) {
    // (count, pd, dd) per label, in a fixed order so cells are comparable.
    let mut rows: Vec<(String, [i64; 3])> = Vec::new();
    let mut bump = |label: String, pd: i64, dd: i64| {
        let slot = match rows.iter_mut().find(|(name, _)| *name == label) {
            Some((_, slot)) => slot,
            None => {
                rows.push((label, [0; 3]));
                &mut rows.last_mut().expect("just pushed").1
            }
        };
        slot[0] += 1;
        slot[1] += pd;
        slot[2] += dd;
    };

    for &index in divergent {
        let board = &boards[index];
        let ask_a = arm_ask(board, feature, true);
        let ask_b = arm_ask(board, baseline, false);
        // Attribute to the feature arm's ask where it made one; otherwise to
        // the baseline's, so a board the feature *stopped* asking on is still
        // charged to that lane rather than hidden in the net bucket.
        let label = match (ask_a, ask_b) {
            (Some((trump, true, _)), _) => format!("{trump:?} relocated"),
            (Some((trump, false, true)), _) => format!("{trump:?} 4NT *ladder offered*"),
            (Some((trump, false, false)), _) => format!("{trump:?} 4NT (no claim)"),
            (None, Some((trump, ..))) => format!("{trump:?} ask only in baseline"),
            (None, None) => "no keycard ask (net alone)".to_string(),
        };
        bump(label, swings_pd[index], swings_dd[index]);
    }
    rows.sort_by(|a, b| b.1[0].cmp(&a.1[0]));
    let total = divergent.len().max(1) as f64;
    println!("\n-- per-trump attribution, bucketed by the ask (all divergent boards) --");
    println!(
        "{:<28} {:>9} {:>7} {:>10} {:>8} {:>10} {:>8}",
        "bucket", "boards", "share", "PD", "PD/bd", "DD", "DD/bd",
    );
    for (label, [count, pd, dd]) in &rows {
        let n = *count as f64;
        println!(
            "{label:<28} {count:>9} {:>6.1}% {pd:>+10} {:>+8.3} {dd:>+10} {:>+8.3}",
            100.0 * n / total,
            *pd as f64 / n,
            *dd as f64 / n,
        );
    }
}

/// The convention card `arm` discloses — its knobs, read through `card.rs`
fn card(arm: Arm) -> Card {
    american_card(&arm_agreements(arm))
}

/// Build one partnership per arm. `ReadingProfile::rkcb_variant` is read at build time for rule
/// presence, and `keycard_minors` by the book's RKCB row packages, so the
/// arms cannot share a book.
///
/// Every arm captures the cell at build time: its own card and `opponent`'s,
/// which is the mixed table these two arms will play.  Bare [`american`] would
/// be *wrong* here — it declares the opponents as playing our own card, so each
/// arm would claim the other relocates exactly as it does, on precisely the
/// mixed boards this experiment measures.  The cards are read *before* the knobs
/// are armed for the build, because `american_card` reads the same knobs.
fn build(arm: Arm, opponent: Arm) -> Partnership {
    let cell = Config::new(&card(arm), &card(opponent));
    american_with_config(&arm_agreements(arm), cell).bind()
}

/// Bid one deal, the feature arm seated N-S or E-W.  Each partnership carries both
/// halves of its arm's knobs — build-time and classify-time — from [`build`].
fn bid_out(
    args: &Args,
    feature: &Partnership,
    baseline: &Partnership,
    feature_is_ns: bool,
    dealer: Seat,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let feature_side = seat_is_ns == feature_is_ns;
        let partnership = if feature_side { feature } else { baseline };
        auction.push(next_call(
            partnership,
            deal[seat],
            dealer,
            args.vulnerability,
            &auction,
        ));
    }
    auction
}

/// The analytic Pavlicek shave's phantom-win probability per contract level:
/// the chance a slam that *makes double-dummy* would have failed at the
/// table.  Pavlicek's full-deal table (rpbridge.net/8j45.htm): the 6-level
/// makes 66.65% actual vs 67.99% DD, grands 64.28% vs 70.31% — the doctrine
/// band is q ≈ 1–3% at six and 3–10% at seven (docs/measurement.md, the
/// slam-boundary addendum); the midpoints are used here.  A board decided by
/// a DD-making slam contributes `swing × (1 − 2q)` — with probability q the
/// win was a phantom and the swing roughly negates.
const fn shave_q(level: u8) -> f64 {
    match level {
        6 => 0.02,
        7 => 0.06,
        _ => 0.0,
    }
}

/// The `(1 − 2q)` factor of one board: the largest shave among the tables
/// whose contract is a slam that makes double-dummy (a grand outranks a small
/// slam when both make — it is the shakier win).
fn shave_factor(
    contracts: (common::Reached, common::Reached),
    table: &ddss::TrickCountTable,
) -> f64 {
    let q = [contracts.0, contracts.1]
        .into_iter()
        .flatten()
        .filter_map(|(contract, declarer)| {
            let level = contract.bid.level.get();
            let makes = u8::from(table[contract.bid.strain].get(declarer)) >= 6 + level;
            (level >= 6 && makes).then(|| shave_q(level))
        })
        .fold(0.0f64, f64::max);
    1.0 - 2.0 * q
}

/// Re-price a `--dump` directory: every board in it is a divergent board of
/// the original run, and every instrument prices the same reached contracts
/// off the same solves — plain DD and perfect defense from one DD fan-out,
/// the two single-dummy endpoints from one composed run per table, their
/// λ-blend ([`common::sd_blend_imps`]), and the analytic Pavlicek shave.
/// Buckets are the ask census of the original run ([`per_trump_census`]'s
/// labels), which is the attribution the §7.13 aggregate rows could not give.
#[allow(clippy::cast_precision_loss)]
fn rescore(args: &Args, path: &str) {
    let dump = common::load_dump(path);
    let parse = |label: &str| -> Arm {
        ValueEnum::from_str(label, true)
            .unwrap_or_else(|e| panic!("dump label {label:?} is not an arm: {e}"))
    };
    let feature = parse(&dump.our_label);
    let baseline = parse(&dump.their_label);
    let vul = dump.vulnerability;
    let count = dump
        .gen_args
        .iter()
        .position(|a| a == "--count" || a == "-c")
        .and_then(|i| dump.gen_args.get(i + 1))
        .and_then(|v| v.parse::<usize>().ok());
    println!(
        "=== rescore: {} vs {}, {} divergent boards of {} bid, vulnerability {}, gen seed {:?} ===",
        dump.our_label,
        dump.their_label,
        dump.boards.len(),
        count.map_or_else(|| "?".to_owned(), |c| c.to_string()),
        vul,
        dump.seed,
    );
    println!("gen args: {:?}", dump.gen_args);

    let contracts: Vec<_> = dump
        .boards
        .iter()
        .map(|board| {
            (
                final_contract(&board.table_a, board.dealer),
                final_contract(&board.table_b, board.dealer),
            )
        })
        .collect();
    let solve: Vec<FullDeal> = dump.boards.iter().map(|board| board.deal).collect();
    let tables = Solver::lock(None).solve_deals(&solve, NonEmptyStrainFlags::ALL);

    // Bucket labels first: under --sd-ask-only they decide which boards the
    // sd pass prices.  `table_ask` arms each arm's own knobs itself.
    let labels: Vec<String> = dump
        .boards
        .iter()
        .map(|board| {
            let ask_a = arm_ask(board, feature, true);
            let ask_b = arm_ask(board, baseline, false);
            match (ask_a, ask_b) {
                (Some((trump, true, _)), _) => format!("{trump:?} relocated"),
                (Some((trump, false, true)), _) => format!("{trump:?} 4NT *ladder offered*"),
                (Some((trump, false, false)), _) => format!("{trump:?} 4NT (no claim)"),
                (None, Some((trump, ..))) => format!("{trump:?} ask only in baseline"),
                (None, None) => "no keycard ask (net alone)".to_string(),
            }
        })
        .collect();
    let priced: Vec<bool> = labels
        .iter()
        .map(|label| !args.sd_ask_only || label != "no keycard ask (net alone)")
        .collect();

    // The sd endpoints, sequential per board (the playout cannot pool); the
    // feature partnership reads for both tables, as in the live --sd row.
    let partnership = build(feature, baseline);
    let mut rng = StdRng::seed_from_u64(args.sd_seed);
    let sd: Vec<Option<[common::SdScores; 2]>> = dump
        .boards
        .iter()
        .enumerate()
        .map(|(index, board)| {
            priced[index].then(|| {
                [&board.table_a, &board.table_b].map(|auction| {
                    common::sd_declarer_ns_score(
                        auction,
                        board.dealer,
                        &board.deal,
                        &partnership,
                        vul,
                        &mut rng,
                        args.sd_worlds,
                        args.sd_worlds,
                    )
                })
            })
        })
        .collect();

    // Per-board swings (feature − baseline, IMPs) under every instrument.
    let n = dump.boards.len();
    let mut swings: Vec<(&str, Vec<f64>)> = [
        "plain DD",
        "perfect defense",
        "sd-lead",
        "sd-lead + PD",
        "sd-playout",
        "sd-playout + PD",
        "sd-blend",
        "sd-blend + PD",
        "DD shaved (q6 2%, q7 6%)",
    ]
    .into_iter()
    .map(|label| (label, Vec::with_capacity(n)))
    .collect();
    for index in 0..n {
        let (a, b) = contracts[index];
        let table = &tables[index];
        let dd = imps(ns_score_contract(a, table, vul) - ns_score_contract(b, table, vul));
        let pd = imps(ns_score_pd(a, table, vul) - ns_score_pd(b, table, vul));
        swings[0].1.push(dd as f64);
        swings[1].1.push(pd as f64);
        match &sd[index] {
            Some([sd_a, sd_b]) => {
                swings[2].1.push(imps(sd_a.lead[0] - sd_b.lead[0]) as f64);
                swings[3].1.push(imps(sd_a.lead[1] - sd_b.lead[1]) as f64);
                swings[4]
                    .1
                    .push(imps(sd_a.playout[0] - sd_b.playout[0]) as f64);
                swings[5]
                    .1
                    .push(imps(sd_a.playout[1] - sd_b.playout[1]) as f64);
                swings[6].1.push(common::sd_blend_imps(sd_a, sd_b, 0));
                swings[7].1.push(common::sd_blend_imps(sd_a, sd_b, 1));
            }
            // An unpriced board carries NaN so every table stays
            // board-parallel; the printers filter to finite values.
            None => (2..=7).for_each(|k| swings[k].1.push(f64::NAN)),
        }
        swings[8]
            .1
            .push(dd as f64 * shave_factor(contracts[index], table));
    }

    println!(
        "\n-- instruments over the dumped boards ({} worlds, sd seed {}{}) --",
        args.sd_worlds,
        args.sd_seed,
        if args.sd_ask_only {
            format!(
                "; sd rows over the {} ask boards only",
                priced.iter().filter(|&&p| p).count(),
            )
        } else {
            String::new()
        },
    );
    for (label, values) in &swings {
        let finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
        if finite.is_empty() {
            println!("{label:<26} (no priced boards)");
            continue;
        }
        let total: f64 = finite.iter().sum();
        let (mean, ci) = common::mean_with_ci_f64(&finite);
        let scope = if finite.len() == n {
            count.map_or_else(String::new, |c| {
                format!("  {:+.5}/board of {c}", total / c as f64)
            })
        } else {
            format!("  (n={})", finite.len())
        };
        println!("{label:<26} {total:+9.1} IMPs  {mean:+.4} ± {ci:.4} /divergent{scope}",);
    }

    // The ask-bucketed census × every instrument: per-board means per bucket.
    let mut buckets: Vec<(String, Vec<usize>)> = Vec::new();
    for (index, label) in labels.iter().enumerate() {
        match buckets.iter_mut().find(|(name, _)| name == label) {
            Some((_, indices)) => indices.push(index),
            None => buckets.push((label.clone(), vec![index])),
        }
    }
    buckets.sort_by_key(|(_, indices)| std::cmp::Reverse(indices.len()));

    println!("\n-- ask-bucketed census × instrument (per-divergent-board mean IMPs) --");
    print!("{:<28} {:>7} {:>6}", "bucket", "boards", "share");
    for (label, _) in &swings {
        print!(
            " {:>9}",
            label
                .replace("perfect defense", "PD")
                .replace("DD shaved (q6 2%, q7 6%)", "DD shaved")
                .replace(" + PD", "+PD")
                .replace("sd-", "")
        );
    }
    println!();
    for (label, indices) in &buckets {
        print!(
            "{label:<28} {:>7} {:>5.1}%",
            indices.len(),
            100.0 * indices.len() as f64 / n.max(1) as f64,
        );
        for (_, values) in &swings {
            let vals: Vec<f64> = indices
                .iter()
                .map(|&i| values[i])
                .filter(|v| v.is_finite())
                .collect();
            if vals.is_empty() {
                print!(" {:>9}", "-");
            } else {
                let mean = vals.iter().sum::<f64>() / vals.len() as f64;
                print!(" {mean:>+9.3}");
            }
        }
        println!();
    }

    // Full per-board detail for the classification pass: contracts, both
    // endpoints' tricks, and every instrument's swing.  Only priced boards
    // print — under --sd-ask-only that is exactly the classification set.
    for (shown, index) in (0..n)
        .filter(|&index| priced[index])
        .take(args.show)
        .enumerate()
    {
        let board = &dump.boards[index];
        let (a, b) = contracts[index];
        let Some([sd_a, sd_b]) = &sd[index] else {
            unreachable!("priced boards carry sd scores");
        };
        let calls_a: Vec<Call> = board.table_a.iter().copied().collect();
        let calls_b: Vec<Call> = board.table_b.iter().copied().collect();
        println!(
            "\n[{shown}] {}  dealer {:?}  A {calls_a:?} -> {a:?} sd {:?}t  vs  B {calls_b:?} -> {b:?} sd {:?}t",
            labels[index], board.dealer, sd_a.tricks, sd_b.tricks,
        );
        for seat in Seat::ALL {
            println!("    {seat:?}: {}", board.deal[seat]);
        }
        print!("   ");
        for (label, values) in &swings {
            print!(
                " {}={:+.2}",
                label
                    .replace("perfect defense", "PD")
                    .replace("DD shaved (q6 2%, q7 6%)", "shave")
                    .replace(" + PD", "+PD"),
                values[index],
            );
        }
        println!();
    }
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    if let Some(path) = &args.rescore {
        rescore(&args, path);
        return;
    }
    let feature = build(args.feature, args.baseline);
    let baseline = build(args.baseline, args.feature);

    let deals: Vec<(Seat, FullDeal)> = seeded_deals(args.seed, args.count)
        .into_iter()
        .enumerate()
        .map(|(index, deal)| (Seat::ALL[index % 4], deal))
        .collect();
    let boards: Vec<Board> = deals
        .par_iter()
        .map(|&(dealer, deal)| Board {
            deal,
            dealer,
            table_a: bid_out(&args, &feature, &baseline, true, dealer, &deal),
            table_b: bid_out(&args, &feature, &baseline, false, dealer, &deal),
        })
        .collect();

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

    if let Some(dir) = &args.dump {
        std::fs::create_dir_all(dir).expect("create dump directory");
        let shard = common::Dump {
            our_label: args.feature.label().to_owned(),
            their_label: args.baseline.label().to_owned(),
            vulnerability: args.vulnerability,
            seed: Some(args.seed),
            gen_args: std::env::args().skip(1).collect(),
            boards: divergent
                .iter()
                .map(|&index| {
                    let board = &boards[index];
                    Board {
                        deal: board.deal,
                        dealer: board.dealer,
                        table_a: board.table_a.clone(),
                        table_b: board.table_b.clone(),
                    }
                })
                .collect(),
        };
        let path = dir.join("shard-000.json");
        serde_json::to_writer(
            std::io::BufWriter::new(std::fs::File::create(&path).expect("create dump shard")),
            &shard,
        )
        .expect("write dump shard");
        println!(
            "dumped {} divergent boards to {}",
            shard.boards.len(),
            path.display(),
        );
    }

    let solve: Vec<FullDeal> = divergent.iter().map(|&index| boards[index].deal).collect();
    let tables = Solver::lock(None).solve_deals(&solve, NonEmptyStrainFlags::ALL);

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
            let calls_b: Vec<Call> = board.table_b.iter().copied().collect();
            println!("    B {calls_b:?}");
            for seat in Seat::ALL {
                println!("    {seat:?}: {}", board.deal[seat]);
            }
        }
    }

    // The sd-declarer playout row reads with the feature partnership on both tables:
    // the arms differ in what they *bid*, and range reading is shared.
    let swings_sd = args.sd.then(|| {
        let mut rng = StdRng::seed_from_u64(args.sd_seed);
        let mut swings = [vec![0i64; args.count], vec![0i64; args.count]];
        for &index in &divergent {
            let board = &boards[index];
            let [a, b] = [&board.table_a, &board.table_b].map(|auction| {
                common::sd_declarer_ns_score(
                    auction,
                    board.dealer,
                    &board.deal,
                    &feature,
                    args.vulnerability,
                    &mut rng,
                    args.sd_worlds,
                    args.sd_worlds,
                )
                .playout
            });
            for (k, swing) in swings.iter_mut().enumerate() {
                swing[index] = imps(a[k] - b[k]);
            }
        }
        swings
    });

    println!(
        "\n=== Kickback A/B: {} vs {}, {} boards, vulnerability {}, seed {} ===",
        args.feature.label(),
        args.baseline.label(),
        args.count,
        args.vulnerability,
        args.seed,
    );
    println!(
        "Divergent boards: {} of {} ({:.4}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    let mut rows = vec![
        ("ns_score_pd  (PD)", &swings_pd),
        ("ns_score_cnt (DD)", &swings_dd),
    ];
    // Slam gains are contract-boundary effects plain DD can see, so a PD-only
    // win here is a doubling artifact, not a ship.  Read the PD row as a bound.
    if let Some([sd, sd_pd]) = &swings_sd {
        rows.push(("sd-declarer  (SD)", sd));
        rows.push(("sd-decl + PD (SD)", sd_pd));
    }
    for (label, swings) in rows {
        let total: i64 = swings.iter().sum();
        let mut acc = Accumulator::new();
        for &swing in swings.iter() {
            acc.push(swing as f64);
        }
        let stats = acc.sample();
        let mean = stats.mean();
        let se = stats.sd() / (args.count.max(1) as f64).sqrt();
        let (lo, hi) = (mean - 1.96 * se, mean + 1.96 * se);
        let per_div = total as f64 / divergent.len().max(1) as f64;
        let verdict = if (lo..=hi).contains(&0.0) {
            "parity"
        } else if mean > 0.0 {
            "feature ahead"
        } else {
            "feature behind"
        };
        println!(
            "{label}: {total:+} IMPs, {mean:+.5}/board  95% CI [{lo:+.5}, {hi:+.5}]  {per_div:+.2}/divergent  ({verdict})",
        );
    }
    per_trump_census(
        &boards,
        &divergent,
        &swings_pd,
        &swings_dd,
        args.feature,
        args.baseline,
    );
}
