//! Measure the keycard ladder's relocated ask (Kickback/Redwood) as a
//! duplicate A/B match.
//!
//! Five arms over three axes, so each coupled change stays attributable:
//!
//! | arm | `set_rkcb_minors` | `set_kickback` | floor | what it is |
//! |---|---|---|---|---|
//! | `plain` | off | off | v3 | majors-only trump, the ask is 4NT |
//! | `minors` | on | off | v3 | minor asks at plain 4NT — round 4's losing arm, re-priced |
//! | `kickback` | on | on | v3 twin | opt-in again since the gate-2 loss: 4♦/4♥ Redwood, 4♠ over hearts |
//! | `v4` | on | off | **v4** | the configured net, playing `minors`' rules |
//! | `v4-kickback` | on | on | **v4** | the configured net, playing `kickback`'s rules |
//!
//! The last two are `docs/ai-bidder/configured-net.md`'s acceptance gates:
//!
//! | gate | invocation | question |
//! |---|---|---|
//! | 1. quality | `--feature v4 --baseline minors` | is the configured net a better bidder at *fixed* rules? |
//! | 2. convention | `--feature v4-kickback --baseline v4` | what is the relocation worth, alone? |
//!
//! Gate 2 is the fair comparison the first three arms cannot give.  Under the v3
//! floor `set_kickback` swaps the weights as well as the rules, so `kickback −
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
//! **Both knob regimes must be armed.** `set_kickback` gates rule *presence* at
//! build time — the reading's `alerted` test is structural, so an always-present
//! alerted rule on 4♥/4♠ would suppress the natural reading of those calls even
//! in the off arm — *and* the recognizers at classification time.  So: one
//! stance built per arm, and the flags re-set per call by side inside the
//! bidding loop (thread-locals do not cross into rayon workers on their own).
//!
//! ```text
//! cargo run --release --example ab-kickback -- \
//!     --feature kickback --baseline plain --count 10000000 --sd
//! cargo run --release --example ab-kickback -- \
//!     --feature v4 --baseline minors --count 2000000 --sd       # gate 1
//! cargo run --release --example ab-kickback -- \
//!     --feature v4-kickback --baseline v4 --count 2000000 --sd  # gate 2
//! ```

use clap::{Parser, ValueEnum};
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::Accumulator;
use pons::american;
use pons::bidding::Stance;
use pons::bidding::american::american_configured_with;
use pons::bidding::card::{Card, american_card};
use pons::bidding::features::Config;
use pons::bidding::instinct::{keycard_ask_at, kickback_offered_at, set_kickback, set_rkcb_minors};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, next_call, seat_to_act, seeded_deals};

/// One arm of the experiment: which knobs it arms, and which floor it stands on
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Arm {
    /// Today's system: majors-only keycard trump, the ask is 4NT
    Plain,
    /// Minor asks at plain 4NT — round 4's losing arm, re-priced
    Minors,
    /// Full Kickback: minor asks relocated to 4♦/4♥ (Redwood), hearts to 4♠
    Kickback,
    /// `minors`' rules on the configured (v4) floor — gate 1's feature arm
    V4,
    /// `kickback`'s rules on the configured (v4) floor — gate 2's feature arm
    V4Kickback,
}

impl Arm {
    /// Whether this arm lifts `keycard_trump`'s majors-only carve
    const fn minors(self) -> bool {
        matches!(
            self,
            Self::Minors | Self::Kickback | Self::V4 | Self::V4Kickback
        )
    }

    /// Whether this arm relocates the ask onto the kickback ladder
    const fn kickback(self) -> bool {
        matches!(self, Self::Kickback | Self::V4Kickback)
    }

    /// Whether this arm's floor reads its convention card (`american_bba_v4`)
    /// rather than selecting one of the twin v3 artifacts by knob
    const fn configured(self) -> bool {
        matches!(self, Self::V4 | Self::V4Kickback)
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Minors => "minors",
            Self::Kickback => "kickback",
            Self::V4 => "v4",
            Self::V4Kickback => "v4-kickback",
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
    set_rkcb_minors(arm.minors());
    set_kickback(arm.kickback());
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
    arm_knobs(arm);
    let calls: Vec<Call> = auction.iter().copied().collect();
    (0..calls.len()).find_map(|index| {
        let seat = seat_to_act(dealer, index);
        let north_south = matches!(seat, Seat::North | Seat::South);
        if north_south != arm_is_ns {
            return None;
        }
        keycard_ask_at(&calls, index).map(|(_bid, trump, relocated)| {
            // A 4NT ask made while the ladder was offering a relocation is a
            // seat the convention was available for and did not get: the
            // authored book installs RKCB at absolute 4NT and has never been
            // relocated (bba-kickback.md phase 4, still `not started`).
            // Separating it from a lane where the ladder claims nothing is the
            // point — the first is a missed relocation, the second is correct.
            // Keyed to the ask's own trump: a spade ask belongs at 4NT, so a
            // claim in some other suit on the same face is not a miss.
            let offered = !relocated && kickback_offered_at(&calls, index, trump);
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
    arm_knobs(Arm::Plain);

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
    arm_knobs(arm);
    let card = american_card();
    arm_knobs(Arm::Plain);
    card
}

/// Build one stance per arm.  `set_kickback` is read at build time for rule
/// presence, and `set_rkcb_minors` by the book's `install_rkcb`, so the arms
/// cannot share a book.
///
/// A configured arm additionally captures the cell at build time: its own card
/// and `opponent`'s, which is the mixed table these two arms will play.  The
/// cards are read *before* the knobs are armed for the build, because
/// `american_card` reads the same knobs.
fn build(arm: Arm, opponent: Arm) -> Stance {
    let cell = arm
        .configured()
        .then(|| Config::new(&card(arm), &card(opponent)));
    arm_knobs(arm);
    let stance = match cell {
        Some(config) => american_configured_with(config).against(),
        None => american().against(),
    };
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
    per_trump_census(
        &boards,
        &divergent,
        &swings_pd,
        &swings_dd,
        args.feature,
        args.baseline,
    );
}
