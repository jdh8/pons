//! Measure the keycard ladder's two capability adds — the relocated ask
//! (Kickback/Redwood) and the queen relay — as duplicate A/B matches.
//!
//! Five arms, four knobs, so each coupled change stays attributable:
//!
//! | arm | `set_keycard_minors` | `set_kickback` | `set_keycard_answer_gates` | `set_queen_ask` | what it is |
//! |---|---|---|---|---|---|
//! | `plain` | off | off | off | off | today: majors-only trump, the ask is 4NT |
//! | `minors` | on | off | off | off | minor asks at plain 4NT — round 4's losing arm, re-priced |
//! | `kickback` | on | on | off | off | full Kickback: 4♦/4♥ Redwood, 4♠ over hearts |
//! | `gated` | on | off | on | off | the shipped default |
//! | `queen` | on | off | on | **on** | the shipped default plus the queen relay |
//! | `kickback-queen` | on | on | off | **on** | Kickback plus the queen relay |
//!
//! The relay's two cells are `queen − gated` (the ship decision for the default
//! system) and `kickback-queen − kickback` (the relay where the relocated ask
//! has already bought it room).  Each pair moves exactly one knob.
//!
//! Why the relay should pay, and where: the 1430 answers disclose the trump
//! queen only on the two-keycard rungs, so after a one-or-four or none-or-three
//! answer the asker has been betting six on four keycards blind.  §7.3.4's
//! residual loss class — the relocated arm settling at five where the baseline's
//! forced six makes — is the pre-registered target.  Expect a **grand-heavy**
//! divergent set, so apply `docs/measurement.md`'s slam-boundary shave before
//! calling a thin win.
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
//! **Both knob regimes must be armed.** `set_kickback` and `set_queen_ask` gate
//! rule *presence* at build time — the reading's `alerted` test is structural,
//! so an always-present alerted rule on 4♥/4♠ (or, for the relay, on 5♦/5♥/5NT)
//! would suppress the natural reading of those calls even in the off arm — *and*
//! the recognizers at classification time.  `set_queen_ask` is read at build
//! time twice over, by `instinct()` and by the book's `install_rkcb`.  So: one
//! stance built per arm, and the flags re-set per call by side inside the
//! bidding loop (thread-locals do not cross into rayon workers on their own).
//!
//! ```text
//! cargo run --release --example ab-kickback -- \
//!     --feature kickback --baseline plain --count 10000000 --sd
//! cargo run --release --example ab-kickback -- \
//!     --feature queen --baseline gated --count 10000000 --sd
//! ```

use clap::{Parser, ValueEnum};
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::Accumulator;
use pons::american;
use pons::bidding::Stance;
use pons::bidding::instinct::{
    set_keycard_answer_gates, set_keycard_minors, set_kickback, set_queen_ask,
};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, next_call, seat_to_act, seeded_deals};

/// One arm of the experiment: which of the two knobs it arms
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Arm {
    /// Today's system: majors-only keycard trump, the ask is 4NT
    Plain,
    /// Minor asks at plain 4NT — round 4's losing arm, re-priced
    Minors,
    /// Full Kickback: minor asks relocated to 4♦/4♥ (Redwood), hearts to 4♠.
    /// Carries the gates too, so `kickback` vs `gated` differs by exactly
    /// `set_kickback` and its verdict speaks to a default-on flip.
    Kickback,
    /// The shipped default since 2026-08-01: minors plus
    /// `set_keycard_answer_gates` (the always-present 1430/ROPI/DOPI/DEPO
    /// answer rules confined to a live ask window).  The plain/minors arms
    /// disarm the gates, preserving the readings they were originally
    /// measured under.
    Gated,
    /// The shipped default plus the queen relay — the ship decision for the
    /// default system, measured against `gated`
    Queen,
    /// Kickback plus the queen relay, measured against `kickback` — the
    /// relocated ask is what buys the relay its room
    KickbackQueen,
}

impl Arm {
    /// Whether this arm lifts `keycard_trump`'s majors-only carve
    const fn minors(self) -> bool {
        matches!(
            self,
            Self::Minors | Self::Kickback | Self::Gated | Self::Queen | Self::KickbackQueen
        )
    }

    /// Whether this arm relocates the ask onto the kickback ladder
    const fn kickback(self) -> bool {
        matches!(self, Self::Kickback | Self::KickbackQueen)
    }

    /// Whether this arm face-gates the always-present answer rules
    const fn gates(self) -> bool {
        matches!(
            self,
            Self::Gated | Self::Kickback | Self::Queen | Self::KickbackQueen
        )
    }

    /// Whether this arm carries the queen relay (and the king ask above it)
    const fn queen(self) -> bool {
        matches!(self, Self::Queen | Self::KickbackQueen)
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Minors => "minors",
            Self::Kickback => "kickback",
            Self::Gated => "gated",
            Self::Queen => "queen",
            Self::KickbackQueen => "kickback-queen",
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
}

/// Arm the classify-time half of the knobs for `arm`
fn arm_knobs(arm: Arm) {
    set_keycard_minors(arm.minors());
    set_kickback(arm.kickback());
    set_keycard_answer_gates(arm.gates());
    set_queen_ask(arm.queen());
}

/// Build one stance per arm.  `set_kickback` and `set_queen_ask` are read at
/// build time for rule presence — the latter by the book's `install_rkcb` too —
/// so the arms cannot share a book.
fn build(arm: Arm) -> Stance {
    arm_knobs(arm);
    let stance = american().against();
    arm_knobs(Arm::Plain);
    stance
}

/// Bid one deal, the feature arm seated N-S or E-W.  Both knobs are re-armed
/// before every call, because the stance alone carries only the build-time half.
fn bid_out(
    args: &Args,
    feature: &Stance,
    baseline: &Stance,
    feature_is_ns: bool,
    dealer: Seat,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let feature_side = seat_is_ns == feature_is_ns;
        let (arm, stance) = if feature_side {
            (args.feature, feature)
        } else {
            (args.baseline, baseline)
        };
        arm_knobs(arm);
        auction.push(next_call(
            stance,
            deal[seat],
            dealer,
            args.vulnerability,
            &auction,
        ));
    }
    arm_knobs(Arm::Plain);
    auction
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let feature = build(args.feature);
    let baseline = build(args.baseline);

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

    // The sd-declarer playout row reads with the feature stance on both tables:
    // the arms differ in what they *bid*, and range reading is shared.
    let swings_sd = args.sd.then(|| {
        let mut rng = StdRng::seed_from_u64(args.sd_seed);
        let mut swings = [vec![0i64; args.count], vec![0i64; args.count]];
        arm_knobs(args.feature);
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
            });
            for (k, swing) in swings.iter_mut().enumerate() {
                swing[index] = imps(a[k] - b[k]);
            }
        }
        arm_knobs(Arm::Plain);
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
}
