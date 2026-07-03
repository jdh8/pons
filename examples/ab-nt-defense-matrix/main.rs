//! GTO-within-a-menu tournament for the defense to their strong 1NT.
//!
//! The value of a 1NT defense depends on the opening side's *counter-strategy*
//! (runout style, the meaning of responder's double, penalty conversions), so a
//! single A/B gives a best response, not an equilibrium.  This harness plays a
//! whole **payoff matrix**: every (our defense, their counters) pair bids the
//! same boards, each board is double-dummy solved once, and every cell is scored
//! against the same datum — the (always-pass, default) cell, so the always-pass
//! row is identically zero and each entry reads "IMPs/board this defense gains
//! over passing throughout, under these counters".  The zero-sum matrix game is
//! then solved by fictitious play for a Nash mixture — the GTO defense *within
//! the menu, under this scoring model*.
//!
//! **The obstruction wall applies** (`project_preemption-dd-negative`): both
//! scorers assume perfect double-dummy cardplay, which prices obstruction and
//! "they sit and die" at zero, so a pass-heavy equilibrium is expected.  Per
//! `reference_pd-vs-plain-dd-bracket` every matrix is reported on **both**
//! plain DD (`ns_score_contract`) and perfect-defense (`ns_score_pd`) scoring.
//!
//! Rows (our defense over their 1NT): always-pass · natural (penalty-X +
//! natural overcalls, the shipped default) · DONT (6+ one-suiter min, the
//! parity config) · Woolsey Multi-Landy.
//! Columns (their counters): shipped defaults · penalty responder-doubles ·
//! soft (takeout doubles, no trap pass, no penalty conversion) · sit (the
//! doubled-1NT runout disabled).
//!
//! ```text
//! cargo run --release --example ab-nt-defense-matrix -- --count 60000
//! cargo run --release --example ab-nt-defense-matrix -- --count 60000 -v both
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Hand, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Family;
use pons::bidding::Stance;
use pons::bidding::american::{
    DoubleStyle, set_always_pass_defense, set_direct_dont, set_direct_dont_one_suiter_min,
    set_direct_landy_double, set_double_style, set_landy, set_natural_defense, set_penalty_pass,
    set_trap_pass, set_unusual_notrump_defense, set_woolsey,
};
use pons::bidding::instinct::{set_one_nt_runout, set_one_nt_runout_universal};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use rayon::prelude::*;
use std::collections::BTreeMap;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Reached, bid_out, hand_hcp, mean_with_ci, seat_to_act};

const ROWS: usize = 4;
const COLS: usize = 4;
const ROW_LABELS: [&str; ROWS] = ["always-pass", "natural", "DONT(6+)", "Woolsey"];
const COL_LABELS: [&str; COLS] = ["default", "penalty-X", "soft", "sit"];

/// GTO-within-a-menu 1NT-defense tournament: defense × counter payoff matrix +
/// fictitious-play equilibrium, on the plain-DD/perfect-defense bracket
#[derive(Parser)]
struct Args {
    /// Number of kept boards (boards where EW actually opens a strong 1NT)
    #[arg(short, long, default_value = "60000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// RNG seed; omitted → taken from the clock and printed (fresh hands per
    /// experiment — pass a seed only to reproduce a run)
    #[arg(long)]
    seed: Option<u64>,

    /// Bootstrap resamples for the equilibrium-support stability check
    #[arg(long, default_value = "200")]
    bootstrap: usize,

    /// Fictitious-play iterations
    #[arg(long, default_value = "200000")]
    fp_iters: usize,

    /// How many worst anchor-cell (natural × default) boards to dump
    #[arg(long, default_value = "10")]
    worst: usize,
}

/// Reset every defense knob (row axis) and counter knob (column axis) to the
/// shipped default, so a book build never inherits a previous build's setting.
fn reset_knobs() {
    // Row axis — our defense over their 1NT.
    set_natural_defense(true);
    set_always_pass_defense(false);
    set_direct_dont(false);
    set_direct_dont_one_suiter_min(5);
    set_woolsey(false);
    set_landy(None);
    set_unusual_notrump_defense(Some((8, 13)));
    set_direct_landy_double(None);
    // Column axis — their counters over our interference.
    set_double_style(DoubleStyle::Optional);
    set_trap_pass(true);
    set_penalty_pass(Some((4, 4, true)));
}

/// Build the four row books (our defense menu) and four column books (their
/// counter menu).  Knobs are thread-local and read at book-construction time,
/// so each build resets everything first.
fn build_books() -> (Vec<Stance>, Vec<Stance>) {
    let build = |configure: &dyn Fn()| {
        reset_knobs();
        configure();
        american().against(Family::NATURAL)
    };
    let rows = vec![
        build(&|| set_always_pass_defense(true)),
        build(&|| ()), // natural: the shipped default defense
        build(&|| {
            // The DONT parity config (docs/ai-bidder/1nt-defense-dont.md):
            // 6+ one-suiter minimum; DONT owns 2♣/2NT, so the Landy/Unusual
            // overlays are overridden the same way ab-landy does.
            set_direct_dont(true);
            set_direct_dont_one_suiter_min(6);
            set_landy(None);
            set_unusual_notrump_defense(Some((8, 14)));
        }),
        build(&|| {
            // Woolsey owns every direct call over their 1NT.
            set_woolsey(true);
            set_natural_defense(false);
            set_unusual_notrump_defense(None);
        }),
    ];
    let cols = vec![
        build(&|| ()),
        build(&|| set_double_style(DoubleStyle::Penalty)),
        build(&|| {
            // Soft: never penalizes our interference.
            set_double_style(DoubleStyle::Takeout);
            set_trap_pass(false);
            set_penalty_pass(None);
        }),
        // Sit: the book is the default one — the difference is the
        // classification-time runout flags, set per cell in the worker.
        build(&|| ()),
    ];
    (rows, cols)
}

/// The "sit" column disables the doubled-1NT runout (a classification-time,
/// per-thread flag), so it must be set in the worker before every cell's bid.
fn set_column_flags(col: usize) {
    let runout = col != 3;
    set_one_nt_runout(runout);
    set_one_nt_runout_universal(runout);
}

/// Balanced shape (no singleton/void, at most one doubleton) with HCP in `lo..=hi`
fn is_balanced_hcp(hand: Hand, lo: u8, hi: u8) -> bool {
    let lengths = Suit::ASC.map(|s| hand[s].len());
    let balanced =
        lengths.iter().all(|&l| l >= 2) && lengths.iter().filter(|&&l| l == 2).count() <= 1;
    balanced && (lo..=hi).contains(&hand_hcp(hand))
}

/// Cheap pre-filter: an E/W hand that could open a strong 1NT (generous band
/// around the 15–17 `fifths` range so no real opener is missed)
fn ew_could_open_1nt(deal: &FullDeal) -> bool {
    [Seat::East, Seat::West]
        .iter()
        .any(|&s| is_balanced_hcp(deal[s], 14, 18))
}

/// The auction's opening bid is 1NT by East or West
fn ew_opened_1nt(auction: &[Call], dealer: Seat) -> bool {
    let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
    auction
        .iter()
        .position(|&c| c != Call::Pass)
        .is_some_and(|i| {
            auction[i] == one_nt && matches!(seat_to_act(dealer, i), Seat::East | Seat::West)
        })
}

/// NS's first non-pass call after the EW 1NT opening — the defensive action a
/// cell's swing is attributed to
fn ns_action_over_1nt(auction: &[Call], dealer: Seat) -> Option<Call> {
    let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
    let i = auction.iter().position(|&c| c == one_nt)?;
    auction[i + 1..].iter().enumerate().find_map(|(off, &c)| {
        let seat = seat_to_act(dealer, i + 1 + off);
        (matches!(seat, Seat::North | Seat::South) && c != Call::Pass).then_some(c)
    })
}

/// A short bucket label for an attributed defensive call
fn action_label(call: Option<Call>) -> String {
    match call {
        None => "(pass)".to_string(),
        Some(Call::Double) => "X".to_string(),
        Some(Call::Bid(bid)) => format!("{bid}"),
        Some(other) => format!("{other:?}"),
    }
}

/// One kept board: the deal, its cell contracts, and what each row's defense did
struct BoardOut {
    deal: FullDeal,
    dealer: Seat,
    /// `contracts[row][col]`; row 0 is the datum replicated across columns
    contracts: [[Reached; COLS]; ROWS],
    /// Each row's first defensive action over the 1NT (column-independent —
    /// the counters only act *after* the interference), read at column 0
    actions: [Option<Call>; ROWS],
    /// The (always-pass, default) datum auction, for the worst-board dump
    datum_auction: Auction,
    /// The (natural, default) anchor auction, for the worst-board dump
    anchor_auction: Auction,
}

/// Bid one candidate deal through every cell; `None` if EW never opened 1NT
fn bid_board(
    rows: &[Stance],
    cols: &[Stance],
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: FullDeal,
) -> Option<BoardOut> {
    set_column_flags(0);
    let datum_auction = bid_out(&rows[0], &cols[0], true, dealer, vul, &deal);
    if !ew_opened_1nt(&datum_auction, dealer) {
        return None;
    }
    let datum = final_contract(&datum_auction, dealer);
    let mut contracts = [[datum; COLS]; ROWS];
    let mut actions = [None; ROWS];
    let mut anchor_auction = Auction::new();
    for row in 1..ROWS {
        for col in 0..COLS {
            set_column_flags(col);
            let auction = bid_out(&rows[row], &cols[col], true, dealer, vul, &deal);
            contracts[row][col] = final_contract(&auction, dealer);
            if col == 0 {
                actions[row] = ns_action_over_1nt(&auction, dealer);
                if row == 1 {
                    anchor_auction = auction;
                }
            }
        }
    }
    Some(BoardOut {
        deal,
        dealer,
        contracts,
        actions,
        datum_auction,
        anchor_auction,
    })
}

/// Solve the zero-sum matrix game (row maximizes, column minimizes) by
/// fictitious play.  Returns the average mixed strategies and the
/// exploitability gap `max_i (M·ȳ)_i − min_j (x̄ᵀ·M)_j` (0 at an exact
/// equilibrium; the game value lies inside the gap).
fn fictitious_play(m: &[Vec<f64>], iters: usize) -> (Vec<f64>, Vec<f64>, f64, f64) {
    let (nr, nc) = (m.len(), m[0].len());
    // Cumulative payoff of each pure strategy against the opponent's history.
    let mut row_payoff = vec![0.0; nr];
    let mut col_payoff = vec![0.0; nc];
    let mut row_count = vec![0.0; nr];
    let mut col_count = vec![0.0; nc];
    let argmax = |v: &[f64]| -> usize {
        v.iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).expect("payoffs are finite"))
            .expect("non-empty")
            .0
    };
    for _ in 0..iters {
        let i = argmax(&row_payoff);
        row_count[i] += 1.0;
        for (j, p) in col_payoff.iter_mut().enumerate() {
            *p -= m[i][j]; // the column player minimizes
        }
        let j = argmax(&col_payoff);
        col_count[j] += 1.0;
        for (i2, p) in row_payoff.iter_mut().enumerate() {
            *p += m[i2][j];
        }
    }
    let normalize = |counts: Vec<f64>| -> Vec<f64> {
        let total: f64 = counts.iter().sum();
        counts.into_iter().map(|c| c / total).collect()
    };
    let x = normalize(row_count);
    let y = normalize(col_count);
    // Best pure responses against the average mixtures.
    let row_best = (0..nr)
        .map(|i| (0..nc).map(|j| m[i][j] * y[j]).sum::<f64>())
        .fold(f64::NEG_INFINITY, f64::max);
    let col_best = (0..nc)
        .map(|j| (0..nr).map(|i| x[i] * m[i][j]).sum::<f64>())
        .fold(f64::INFINITY, f64::min);
    let value = 0.5 * (row_best + col_best);
    (x, y, value, row_best - col_best)
}

/// Fictitious play must solve matching pennies (value 0, uniform mixtures) —
/// the cheap self-check that the solver isn't broken.
fn fp_self_check() {
    let pennies = vec![vec![1.0, -1.0], vec![-1.0, 1.0]];
    let (x, y, value, gap) = fictitious_play(&pennies, 100_000);
    assert!(
        value.abs() < 0.01 && gap < 0.02,
        "FP fails matching pennies"
    );
    assert!(
        (x[0] - 0.5).abs() < 0.05 && (y[0] - 0.5).abs() < 0.05,
        "FP mixture off uniform on matching pennies"
    );
}

/// Render a mixture as "label 0.87 · label 0.13", dropping sub-1% entries
fn mixture(weights: &[f64], labels: &[&str]) -> String {
    let parts: Vec<String> = weights
        .iter()
        .zip(labels)
        .filter(|(w, _)| **w >= 0.01)
        .map(|(w, l)| format!("{l} {w:.2}"))
        .collect();
    parts.join(" · ")
}

#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
fn main() {
    fp_self_check();
    let args = Args::parse();
    let seed = args.seed.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after 1970")
            .as_secs()
    });
    let mut rng = StdRng::seed_from_u64(seed);
    let vul = args.vulnerability;
    let (rows, cols) = build_books();

    // Deal → cheap shape pre-filter → bid the datum cell → keep iff EW actually
    // opened 1NT.  Chunked so the sequential RNG stays reproducible while the
    // bidding fans out over Rayon.
    let mut boards: Vec<BoardOut> = Vec::with_capacity(args.count);
    let mut scanned = 0usize;
    let mut candidates = 0usize;
    while boards.len() < args.count {
        let need = (args.count - boards.len()).max(64);
        let mut chunk: Vec<(usize, FullDeal)> = Vec::with_capacity(need * 2);
        while chunk.len() < need * 2 {
            let deal = full_deal(&mut rng);
            scanned += 1;
            if ew_could_open_1nt(&deal) {
                chunk.push((candidates, deal));
                candidates += 1;
            }
        }
        boards.par_extend(
            chunk
                .into_par_iter()
                .filter_map(|(idx, deal)| bid_board(&rows, &cols, Seat::ALL[idx % 4], vul, deal)),
        );
    }
    boards.truncate(args.count);
    let n = boards.len();
    eprintln!("scanned {scanned} deals, {candidates} candidates, kept {n}; solving...");

    // One DD solve per board serves every cell; only boards where some cell
    // left the datum contract can swing.
    let need_solve: Vec<usize> = (0..n)
        .filter(|&b| {
            let datum = boards[b].contracts[0][0];
            boards[b].contracts.iter().flatten().any(|&c| c != datum)
        })
        .collect();
    let deals: Vec<FullDeal> = need_solve.iter().map(|&b| boards[b].deal).collect();
    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    // Per-cell per-board IMP swings vs the datum, on both scorers.
    let mut plain: Vec<Vec<Vec<i64>>> = vec![vec![vec![0; n]; COLS]; ROWS];
    let mut pd: Vec<Vec<Vec<i64>>> = vec![vec![vec![0; n]; COLS]; ROWS];
    let mut divergent = [[0usize; COLS]; ROWS];
    let mut buckets: Vec<Vec<BTreeMap<String, (i64, i64)>>> =
        vec![vec![BTreeMap::new(); COLS]; ROWS];
    for (&b, table) in need_solve.iter().zip(&tables) {
        let board = &boards[b];
        let datum = board.contracts[0][0];
        let datum_plain = ns_score_contract(datum, table, vul);
        let datum_pd = ns_score_pd(datum, table, vul);
        for row in 0..ROWS {
            for col in 0..COLS {
                let reached = board.contracts[row][col];
                if reached == datum {
                    continue;
                }
                let swing_plain = imps(ns_score_contract(reached, table, vul) - datum_plain);
                plain[row][col][b] = swing_plain;
                pd[row][col][b] = imps(ns_score_pd(reached, table, vul) - datum_pd);
                divergent[row][col] += 1;
                let bucket = buckets[row][col]
                    .entry(action_label(board.actions[row]))
                    .or_default();
                bucket.0 += 1;
                bucket.1 += swing_plain;
            }
        }
    }

    println!(
        "=== 1NT-defense matrix: {n} boards (EW opens a strong 1NT), vul {vul}, seed {seed} ===",
    );
    println!(
        "rows = our defense, cols = their counters; entries = IMPs/board vs the (always-pass, default) datum",
    );
    let print_matrix = |name: &str, swings: &[Vec<Vec<i64>>]| {
        println!("--- {name} ---");
        print!("{:<13}", "");
        for label in COL_LABELS {
            print!("{label:>16}");
        }
        println!();
        for (row, cells) in swings.iter().enumerate() {
            print!("{:<13}", ROW_LABELS[row]);
            for cell in cells {
                let (mean, ci) = mean_with_ci(cell);
                print!("  {mean:+.3}\u{b1}{ci:.3}");
            }
            println!();
        }
    };
    print_matrix("plain DD (ns_score_contract)", &plain);
    print_matrix("perfect defense (ns_score_pd)", &pd);
    println!("--- divergence from datum (% of boards) ---");
    print!("{:<13}", "");
    for label in COL_LABELS {
        print!("{label:>16}");
    }
    println!();
    for (row, cells) in divergent.iter().enumerate() {
        print!("{:<13}", ROW_LABELS[row]);
        for &count in cells {
            print!("{:>15.1}%", 100.0 * count as f64 / n.max(1) as f64);
        }
        println!();
    }

    // The equilibrium of the empirical matrix, per scorer.
    let mean_matrix = |swings: &[Vec<Vec<i64>>]| -> Vec<Vec<f64>> {
        swings
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| cell.iter().sum::<i64>() as f64 / n.max(1) as f64)
                    .collect()
            })
            .collect()
    };
    for (name, swings) in [("plain", &plain), ("pd", &pd)] {
        let m = mean_matrix(swings);
        let (x, y, value, gap) = fictitious_play(&m, args.fp_iters);
        println!("--- equilibrium ({name}) ---");
        println!("  defense mixture:  {}", mixture(&x, &ROW_LABELS));
        println!("  counter mixture:  {}", mixture(&y, &COL_LABELS));
        println!("  value {value:+.4} IMPs/board (exploitability gap {gap:.4})");
    }

    // Bootstrap the equilibrium support over boards: does the mixture survive
    // resampling, or does it flip inside the noise?
    if args.bootstrap > 0 {
        let mut boot_rng = StdRng::seed_from_u64(seed.wrapping_add(1));
        for (name, swings) in [("plain", &plain), ("pd", &pd)] {
            let mut row_support = [0usize; ROWS];
            let mut col_support = [0usize; COLS];
            let mut values: Vec<f64> = Vec::with_capacity(args.bootstrap);
            for _ in 0..args.bootstrap {
                let sample: Vec<usize> = (0..n).map(|_| boot_rng.random_range(0..n)).collect();
                let m: Vec<Vec<f64>> = (0..ROWS)
                    .map(|row| {
                        (0..COLS)
                            .map(|col| {
                                sample.iter().map(|&b| swings[row][col][b]).sum::<i64>() as f64
                                    / n as f64
                            })
                            .collect()
                    })
                    .collect();
                let (x, y, value, _) = fictitious_play(&m, args.fp_iters / 10);
                values.push(value);
                for (i, w) in x.iter().enumerate() {
                    row_support[i] += usize::from(*w >= 0.05);
                }
                for (j, w) in y.iter().enumerate() {
                    col_support[j] += usize::from(*w >= 0.05);
                }
            }
            values.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
            let pct = |c: usize| 100.0 * c as f64 / args.bootstrap as f64;
            println!("--- bootstrap ({name}, {} resamples) ---", args.bootstrap);
            print!("  defense in support:");
            for (i, label) in ROW_LABELS.iter().enumerate() {
                print!(" {label} {:.0}%", pct(row_support[i]));
            }
            println!();
            print!("  counter in support:");
            for (j, label) in COL_LABELS.iter().enumerate() {
                print!(" {label} {:.0}%", pct(col_support[j]));
            }
            println!();
            println!(
                "  value 95% band [{:+.4}, {:+.4}]",
                values[args.bootstrap * 25 / 1000],
                values[(args.bootstrap * 975 / 1000).min(args.bootstrap - 1)],
            );
        }
    }

    println!("--- per-action buckets (plain-DD swings vs datum, per cell) ---");
    for row in 1..ROWS {
        for col in 0..COLS {
            let cells: Vec<String> = buckets[row][col]
                .iter()
                .map(|(action, (count, swing))| {
                    format!(
                        "{action} {count}b {swing:+} ({:+.2}/b)",
                        *swing as f64 / (*count).max(1) as f64
                    )
                })
                .collect();
            println!(
                "  [{} \u{d7} {}]  {}",
                ROW_LABELS[row],
                COL_LABELS[col],
                cells.join("; ")
            );
        }
    }

    // Worst anchor-cell boards: sanity-check the natural defense's auctions.
    let mut worst: Vec<(i64, usize)> = (0..n).map(|b| (plain[1][0][b], b)).collect();
    worst.sort_unstable();
    eprintln!(
        "=== worst {} (natural \u{d7} default) boards ===",
        args.worst
    );
    for &(swing, b) in worst.iter().take(args.worst) {
        let board = &boards[b];
        eprintln!(
            "[{swing:+} IMP] dealer {:?}  {}\n  datum:  {} -> {:?}\n  natural: {} -> {:?}",
            board.dealer,
            board.deal,
            board.datum_auction,
            board.contracts[0][0],
            board.anchor_auction,
            board.contracts[1][0],
        );
    }
}
