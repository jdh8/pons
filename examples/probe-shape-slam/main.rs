//! Shape-slam seam probe — how fat is the "stop in 4M but 6M is cold" gap?
//!
//! Before building a cue-bidding subsystem for the instinct floor (to reach
//! shape-driven major slams on a known 4-4 fit *below* the 33-combined-HCP
//! threshold the floor's slam milestones currently gate on), we measure the
//! population: bid every board out with the real system uncontested, find the
//! boards where we **stop in a major game (4♥/4♠)** yet the **small slam makes
//! double-dummy**, and bucket them by combined HCP, fit shape, and shortness.
//!
//! The decision it informs: is the *sub-33 shapely* slice (what a cue ladder
//! would target) fat, or are the missed slams mostly 33+ (a plain evaluator /
//! decodability fix, not a cue subsystem)?  It also prices the discrimination
//! problem — how often a blind "bid slam on a shapely fit" trigger would
//! **overreach** into a game that already fails to make slam.
//!
//! ```text
//! cargo run --release --example probe-shape-slam -- --count 20000 --seed 1
//! ```

use clap::Parser;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Rank, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Family;
use pons::bidding::constraint::point_count;
use pons::scoring::final_contract;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{hand_hcp, seat_to_act, seeded_deals};

#[derive(Parser)]
#[command(about = "Size the 4M-stop / 6M-cold shape-slam seam on known major fits")]
struct Args {
    /// Boards to bid and classify
    #[arg(short, long, default_value_t = 20_000)]
    count: usize,
    /// Deal seed base (board i seeded base+i); fresh per experiment
    #[arg(short, long, default_value_t = 1)]
    seed: u64,
    /// Vulnerability the deals are bid at (affects nothing but routing symmetry)
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,
    /// Print this many example seam boards
    #[arg(long, default_value_t = 8)]
    examples: usize,
}

/// The trump Suit of a major strain, else `None`.
fn major(strain: Strain) -> Option<Suit> {
    matches!(strain, Strain::Hearts | Strain::Spades)
        .then(|| Suit::try_from(strain).expect("major is a suit"))
}

/// New Losing Trick Count (Koelman): within the top `min(len, 3)` slots of a
/// suit, a missing ace is 1.5 losers, a missing king 1.0, a missing queen 0.5.
/// Jacks are ignored; short suits self-cap (a void is 0, a singleton ≤1.5).
fn nltc(hand: Hand) -> f64 {
    Suit::ASC
        .iter()
        .map(|&s| {
            let h = hand[s];
            let len = h.len();
            f64::from(u8::from(len >= 1 && !h.contains(Rank::A))) * 1.5
                + f64::from(u8::from(len >= 2 && !h.contains(Rank::K)))
                + f64::from(u8::from(len >= 3 && !h.contains(Rank::Q))) * 0.5
        })
        .sum()
}

/// Shortest suit length across a hand — the ruffing-shortness signal.
fn min_suit_len(deal: &FullDeal, seat: Seat) -> usize {
    Suit::ASC
        .iter()
        .map(|&s| deal[seat][s].len())
        .min()
        .expect("four suits")
}

/// One classified 4M-stop board.
struct Board {
    idx: usize,
    trump: Suit,
    declarer: Seat,
    combined_hcp: u8,
    /// combined upgraded point_count (raw HCP + fuzzy shape upgrade) — the axis
    /// the slam gate actually reads (`combined_points`)
    combined_pts: u8,
    /// combined New Losing Trick Count — lower is stronger (slam-oriented)
    combined_nltc: f64,
    n_len: usize,
    s_len: usize,
    /// min suit length across the two hands — <=2 is ruffing shortness
    short: usize,
    slam_makes: bool,
}

fn main() {
    let args = Args::parse();
    let stance = american().against(Family::NATURAL);
    let deals = seeded_deals(args.seed, args.count);
    let vul = args.vulnerability;

    // Bid every board uncontested (constructive seam), keep the boards that stop
    // in a *NS-declared major game*.  Bidding is pure, so parallelize; the DD
    // solver stays on the main thread below.
    let stops: Vec<(usize, Suit, Seat)> = deals
        .par_iter()
        .enumerate()
        .filter_map(|(idx, deal)| {
            let dealer = Seat::ALL[idx % 4];
            let mut auction = contract_bridge::auction::Auction::new();
            while !auction.has_ended() {
                let seat = seat_to_act(dealer, auction.len());
                let call = if matches!(seat, Seat::East | Seat::West) {
                    contract_bridge::auction::Call::Pass
                } else {
                    common::next_call(&stance, deal[seat], dealer, vul, &auction)
                };
                auction.push(call);
            }
            let (contract, declarer) = final_contract(&auction, dealer)?;
            let trump = major(contract.bid.strain)?;
            (contract.bid.level.get() == 4 && matches!(declarer, Seat::North | Seat::South))
                .then_some((idx, trump, declarer))
        })
        .collect();

    // Solve the actual deals of the 4M stops (one table each) on the main thread.
    let solve: Vec<FullDeal> = stops.iter().map(|&(idx, ..)| deals[idx]).collect();
    let tables = Solver::lock().solve_deals(&solve, NonEmptyStrainFlags::ALL);

    let boards: Vec<Board> = stops
        .iter()
        .zip(&tables)
        .map(|(&(idx, trump, declarer), table)| {
            let strain = Strain::from(trump);
            let slam_makes = u8::from(table[strain].get(Seat::North))
                .max(u8::from(table[strain].get(Seat::South)))
                >= 12;
            Board {
                idx,
                trump,
                declarer,
                combined_hcp: hand_hcp(deals[idx][Seat::North]) + hand_hcp(deals[idx][Seat::South]),
                combined_pts: point_count(deals[idx][Seat::North])
                    + point_count(deals[idx][Seat::South]),
                combined_nltc: nltc(deals[idx][Seat::North]) + nltc(deals[idx][Seat::South]),
                n_len: deals[idx][Seat::North][trump].len(),
                s_len: deals[idx][Seat::South][trump].len(),
                short: min_suit_len(&deals[idx], Seat::North)
                    .min(min_suit_len(&deals[idx], Seat::South)),
                slam_makes,
            }
        })
        .collect();

    report(&args, &boards);
}

#[allow(clippy::cast_precision_loss)]
fn report(args: &Args, boards: &[Board]) {
    let count = args.count as f64;
    let n_4m = boards.len();
    let tally = |f: &dyn Fn(&Board) -> bool| boards.iter().filter(|b| f(b)).count();
    let pct = |n: usize, d: usize| {
        if d == 0 {
            0.0
        } else {
            100.0 * n as f64 / d as f64
        }
    };

    let missed = tally(&|b| b.slam_makes);
    let shapely = |b: &Board| b.combined_hcp < 33 && b.short <= 2;
    let seam = tally(&|b| b.slam_makes && shapely(b));
    let seam_44 = tally(&|b| b.slam_makes && shapely(b) && b.n_len == 4 && b.s_len == 4);
    let overreach = tally(&|b| !b.slam_makes && shapely(b));

    println!(
        "=== probe-shape-slam ({} boards, seed {}) ===",
        args.count, args.seed
    );
    println!(
        "4M stops (NS major game): {n_4m}  ({:.2}% of boards)",
        pct(n_4m, args.count)
    );
    println!(
        "  of which 6M makes DD:   {missed}  ({:.1}% of 4M stops, {:.3}% of boards)",
        pct(missed, n_4m),
        100.0 * missed as f64 / count
    );

    // Combined-HCP band of the MISSED slams — the load-bearing split.
    println!("\n  missed-slam combined-HCP bands:");
    for (lo, hi, tag) in [
        (0, 29, "<29"),
        (29, 31, "29-30"),
        (31, 33, "31-32"),
        (33, 41, "33+"),
    ] {
        let n = tally(&|b| b.slam_makes && b.combined_hcp >= lo && b.combined_hcp < hi);
        println!("    {tag:>6}: {n:>6}  ({:4.1}% of missed)", pct(n, missed));
    }

    // Same missed slams, bucketed by UPGRADED combined points (raw HCP + fuzzy
    // shape upgrade) — the axis `combined_points(33)` actually gates on.  The
    // gap between this table and the HCP one above is exactly how much the
    // existing shape upgrade already lifts these hands toward the 33 line.
    println!("\n  missed-slam combined-POINTS bands (raw HCP + shape upgrade):");
    for (lo, hi, tag) in [
        (0, 29, "<29"),
        (29, 31, "29-30"),
        (31, 33, "31-32"),
        (33, 41, "33+"),
    ] {
        let n = tally(&|b| b.slam_makes && b.combined_pts >= lo && b.combined_pts < hi);
        println!("    {tag:>6}: {n:>6}  ({:4.1}% of missed)", pct(n, missed));
    }
    // Decisive for "is the evaluator too harsh": shapely slam-makers (raw HCP
    // <33 + shortness) that the upgrade ALREADY carries to combined_pts>=33 —
    // the gate would pass them, so the block is elsewhere (RKCB decodability /
    // partner's advertised min), NOT the point count.
    let lifted = tally(&|b| b.slam_makes && shapely(b) && b.combined_pts >= 33);
    let seam_now = tally(&|b| b.slam_makes && shapely(b));
    println!(
        "\n  shapely slam-makers already at combined_pts>=33 (upgrade carries them over): {lifted}  ({:4.1}% of the {seam_now} shapely)",
        pct(lifted, seam_now)
    );

    println!(
        "\n  --> SEAM (missed & <33 combined & shortness<=2): {seam}  ({:4.1}% of missed, {:.3}% of boards)",
        pct(seam, missed),
        100.0 * seam as f64 / count
    );
    println!(
        "        of those a bare 4-4 trump fit: {seam_44}  ({:4.1}% of seam)",
        pct(seam_44, seam)
    );
    // Ceiling: if every seam board converted 4M->6M, at ~12 IMPs a slam.
    println!(
        "        rough IMP ceiling (perfect conversion @~12): {:+.3} IMPs/board",
        12.0 * seam as f64 / count
    );

    // Discrimination: shapely 4M stops where 6M does NOT make — a blind trigger's
    // overreach set.  A cue ladder must out-discriminate this.
    println!(
        "\n  discrimination — shapely 4M stops where 6M FAILS (blind-trigger overreach): {overreach}"
    );
    println!(
        "        shapely stops total: {}  → slam-make rate among shapely stops: {:.1}%",
        seam + overreach,
        pct(seam, seam + overreach)
    );

    // The build-vs-skip cross-tab: of the 4M stops, what fraction have a cold 6M,
    // sliced by combined HCP × shortness class.  A cue ladder is worth building
    // only where the make-rate is high enough to bid toward — the low-HCP / flat
    // cells are DD freaks (voids carrying wild single-suiters), unbiddable.  The
    // right-hand 4-4-only columns are the user's exemplar class specifically.
    let short_class = |b: &Board| match b.short {
        0..=1 => "sing/void",
        2 => "doubleton",
        _ => "flat 3+",
    };
    let cell = |lo: u8, hi: u8, sc: &str, only44: bool| {
        let denom = tally(&|b| {
            b.combined_hcp >= lo
                && b.combined_hcp < hi
                && short_class(b) == sc
                && (!only44 || (b.n_len == 4 && b.s_len == 4))
        });
        let made = tally(&|b| {
            b.slam_makes
                && b.combined_hcp >= lo
                && b.combined_hcp < hi
                && short_class(b) == sc
                && (!only44 || (b.n_len == 4 && b.s_len == 4))
        });
        (made, denom)
    };
    println!("\n  make-rate of 6M among 4M stops   [ ALL fits | 4-4 only ]   (made/stops):");
    println!("            HCP     sing/void        doubleton         flat 3+");
    for (lo, hi, tag) in [
        (0, 29, "<29"),
        (29, 31, "29-30"),
        (31, 33, "31-32"),
        (33, 41, "33+"),
    ] {
        let fmt = |sc: &str| {
            let (m, d) = cell(lo, hi, sc, false);
            let (m4, d4) = cell(lo, hi, sc, true);
            format!(
                "{:>3}/{:<4}{:>3.0}% | {:>2}/{:<3}",
                m,
                format!("{d}"),
                pct(m, d),
                m4,
                format!("{d4}")
            )
        };
        println!(
            "         {tag:>6}  {}  {}  {}",
            fmt("sing/void"),
            fmt("doubleton"),
            fmt("flat 3+")
        );
    }

    // Evaluator bake-off: as a solo slam trigger over the 4M stops, which metric
    // separates the {missed} cold 6M from the games that stay games?  Precision =
    // makes/fires (how clean — the inverse of overreach); recall = makes/missed
    // (coverage of cold slams).  A better slam evaluator dominates: higher
    // precision at equal recall.  NLTC fires LOW (strong = few losers); HCP and
    // upgraded points fire HIGH.
    println!(
        "\n  === evaluator bake-off (solo slam trigger over {n_4m} stops, {missed} cold 6M) ==="
    );
    println!("    trigger            fires   makes   precision   recall");
    let row = |label: &str, pred: &dyn Fn(&Board) -> bool| {
        let fires = tally(pred);
        let makes = tally(&|b| b.slam_makes && pred(b));
        println!(
            "    {label:<16} {fires:>6}  {makes:>6}    {:>6.1}%   {:>6.1}%",
            pct(makes, fires),
            pct(makes, missed)
        );
    };
    for t in [11.0, 12.0, 13.0, 14.0, 15.0] {
        row(&format!("NLTC <= {t:>4.1}"), &|b| b.combined_nltc <= t);
    }
    for t in [33, 31, 30, 29, 28] {
        row(&format!("HCP  >= {t}"), &|b| b.combined_hcp >= t);
    }
    for t in [33, 31, 30, 29] {
        row(&format!("pts  >= {t}"), &|b| b.combined_pts >= t);
    }
    // And the honest slam trigger: an evaluator AND a known-fit ruffing shape.
    // The floor never bids slam on values alone; it needs the fit.  Cross NLTC
    // with the shortness gate the earlier cross-tab already validated.
    // NLTC's trick formula presumes a trump fit — a bake-off over misfit 4M
    // contracts (5-2, 4-3 moysians) tests it out of domain.  Restrict to genuine
    // 8+ card fits (what the floor's `known_eight_card_fit` gate requires anyway).
    // This is the fair NLTC test and the decision-relevant comparison.
    let fit = |b: &Board| b.n_len + b.s_len >= 8;
    let n_fit = tally(&fit);
    let makes_fit = tally(&|b| b.slam_makes && fit(b));
    println!(
        "\n  4M stops by trump length: 8+ fit = {n_fit} ({makes_fit} cold 6M)  |  <8 misfit = {} ({} cold 6M)",
        n_4m - n_fit,
        missed - makes_fit
    );
    println!("    -- bake-off WITHIN 8+ fits (recall over {makes_fit} in-fit cold 6M) --");
    let row_fit = |label: &str, pred: &dyn Fn(&Board) -> bool| {
        let fires = tally(&|b| pred(b) && fit(b));
        let makes = tally(&|b| b.slam_makes && pred(b) && fit(b));
        println!(
            "    {label:<16} {fires:>6}  {makes:>6}    {:>6.1}%   {:>6.1}%",
            pct(makes, fires),
            pct(makes, makes_fit)
        );
    };
    for t in [11.0, 12.0, 13.0, 14.0] {
        row_fit(&format!("NLTC <= {t:>4.1}"), &|b| b.combined_nltc <= t);
    }
    for t in [31, 30, 29] {
        row_fit(&format!("HCP  >= {t}"), &|b| b.combined_hcp >= t);
    }
    for t in [31, 30, 29, 28, 27] {
        row_fit(&format!("pts  >= {t}"), &|b| b.combined_pts >= t);
    }
    // The literal proposed gate: known 8+ fit AND ruffing shortness.  Requiring a
    // short suit strips flat no-ruff hands (few of which make), so precision
    // should hold higher as the point floor drops — the >50% "biddable slam"
    // line is what we're chasing.
    println!("    -- proposed gate: pts >= t & fit & shortness<=2 --");
    for t in [30, 29, 28, 27, 26] {
        row_fit(&format!("pts>={t} & short"), &|b| {
            b.combined_pts >= t && b.short <= 2
        });
    }

    if args.examples > 0 {
        println!("\n  example seam boards:");
        for b in boards
            .iter()
            .filter(|b| b.slam_makes && shapely(b))
            .take(args.examples)
        {
            println!(
                "    #{:<6} {:?}  {:?}-fit {}+{}  combined {} HCP  short {}  (6{:?} makes)",
                b.idx, b.declarer, b.trump, b.n_len, b.s_len, b.combined_hcp, b.short, b.trump
            );
        }
    }
}
