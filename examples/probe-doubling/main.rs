//! probe-doubling — were the doubles right? (pre-A/B gate 2b)
//!
//! The calibration half of `docs/ai-bidder/doubling-calibration.md`. The
//! competitive accountant prices the failing branch of a contract we bid as
//! doubled with probability `q`, and `q` is only meaningful if the opponents'
//! doubles are *failure-conditioned*. This converts the marginal
//! `P(doubled | we declare)` that `scripts/q-table.py` counts into the two
//! numbers the gate actually needs — `P(X | we fail)` and `P(X | we make)` —
//! and prices the doubles themselves in IMPs.
//!
//! Method: read retained `bba-gen` arm dumps, take every table-auction's final
//! contract (`pons::scoring::final_contract`), stride-sample by board so the
//! solve is bounded, solve those deals double-dummy, and tabulate. Both tables
//! are read: `bba-gen` seats our system N/S at table A and E/W at table B
//! (`bid_out(..., conv_is_ns, ...)`), so table B needs the seat-parity flip.
//!
//! **The solver is a process-global lock used from the main thread only**, so
//! this is single-threaded around `solve_deals` and bounded by `--limit`. One
//! directory is loaded, sampled, solved and folded at a time — the four
//! retained arms together are ~290 MB of JSON.
//!
//! ```text
//! scripts/idle-run.sh cargo run --release --features serde,dd --example probe-doubling -- \
//!     ab-results/two-level-minor-overcall-refresh/off-none \
//!     ab-results/two-level-minor-overcall-refresh/off-both
//! ```
//!
//! Reads dumps only; writes nothing. `examples/ab-dump-diff` is deliberately
//! left alone — `scripts/sd-pd-dumps.sh` gates on its output not drifting.

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, Contract, FullDeal, Penalty, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::scoring::{final_contract, imps};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{load_dump, seat_to_act};

/// The accountant's trigger, read off an auction prefix from the seat to act
///
/// Mirrors `instinct::their_live_bid_at_most` with the comparison inverted, and
/// is the same predicate `examples/eval-columns` slices its columns with: the
/// last live call is *their* bid (X and XX are not `Call::Bid`, so a doubled
/// contract is excluded) at level ≥ 4, and our side has already named a strain.
fn gate_node(prefix: &[Call]) -> bool {
    let Some(index) = prefix.iter().rposition(|&call| call != Call::Pass) else {
        return false;
    };
    if (prefix.len() - index).is_multiple_of(2) {
        return false;
    }
    let Call::Bid(bid) = prefix[index] else {
        return false;
    };
    bid.level.get() >= 4
        && prefix
            .iter()
            .enumerate()
            .any(|(i, call)| matches!(call, Call::Bid(_)) && (prefix.len() - i).is_multiple_of(2))
}

/// Did one of *our* seats ever face the trigger in this auction?
///
/// q is applied at that node, so the calibration has to be conditioned there
/// too — the marginal population mixes in uncontested auctions that land in
/// 5♦/5♥ with nobody placed to double, and that dilution is what made the
/// marginal q table read a level gradient that is not really there
/// (`docs/ai-bidder/doubling-calibration.md`).
fn gate_reached(auction: &[Call], dealer: Seat, ours_is_ns: bool) -> bool {
    (0..auction.len()).any(|length| {
        matches!(seat_to_act(dealer, length), Seat::North | Seat::South) == ours_is_ns
            && gate_node(&auction[..length])
    })
}

#[derive(Parser)]
#[command(about = "Failure-conditioning and IMP value of the doubles in retained arm dumps")]
struct Args {
    /// Arm dump directories (or files) — the *default* arms, e.g. `off-none off-both`
    dirs: Vec<String>,
    /// Ignore contracts below this level; the gate reads q at 4+
    #[arg(long, default_value_t = 1)]
    min_level: u8,
    /// Boards to sample per directory (stride-sampled, so both tables of a kept
    /// board stay together and the sample is deterministic)
    #[arg(long, default_value_t = 40_000)]
    limit: usize,
}

/// Level buckets, matching `scripts/q-table.py`
const LEVELS: [&str; 4] = ["1-2", "3", "4", "5+"];

const fn bucket(level: u8) -> usize {
    match level {
        ..=2 => 0,
        3 => 1,
        4 => 2,
        _ => 3,
    }
}

/// Counts for one cell of the published tables
#[derive(Default, Clone, Copy)]
struct Row {
    n: u64,
    doubled: u64,
    failed: u64,
    doubled_failed: u64,
    made_doubled: u64,
    down1: u64,
    down2: u64,
    /// IMPs the doubles in this cell earned the side that made them
    imps: i64,
}

impl Row {
    fn push(&mut self, doubled: bool, down: i32, gain: i64) {
        self.n += 1;
        self.failed += u64::from(down > 0);
        if !doubled {
            return;
        }
        self.doubled += 1;
        self.imps += gain;
        if down > 0 {
            self.doubled_failed += 1;
            if down == 1 {
                self.down1 += 1;
            } else {
                self.down2 += 1;
            }
        } else {
            self.made_doubled += 1;
        }
    }
}

/// The published tables, folded in one pass
#[derive(Default)]
struct Tally {
    /// `[level bucket][our vul]`, **our** declared contracts, all auctions
    levels: [[Row; 2]; 4],
    /// The same, restricted to auctions that reached the gate's trigger — the
    /// population q is actually applied to
    gate: [[Row; 2]; 4],
    /// `[we declare / they declare][our vul]`
    directions: [[Row; 2]; 2],
    boards: usize,
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        f64::NAN
    } else {
        numerator as f64 / denominator as f64
    }
}

/// Sample, solve and fold one arm directory
fn fold(path: &str, args: &Args, tally: &mut Tally) {
    let dump = load_dump(path);
    let vul = dump.vulnerability;
    let stride = dump.boards.len().div_ceil(args.limit.max(1)).max(1);
    let kept: Vec<usize> = (0..dump.boards.len()).step_by(stride).collect();
    let deals: Vec<FullDeal> = kept.iter().map(|&index| dump.boards[index].deal).collect();
    eprintln!(
        "probe-doubling: {path} — {} boards, solving {} (stride {stride})",
        dump.boards.len(),
        deals.len()
    );
    let tables = Solver::lock(None).solve_deals(&deals, NonEmptyStrainFlags::ALL);
    tally.boards += kept.len();

    for (&index, table) in kept.iter().zip(&tables) {
        let board = &dump.boards[index];
        // Table A seats our system N/S; table B seats it E/W.
        for (auction, ours_is_ns) in [(&board.table_a, true), (&board.table_b, false)] {
            let Some((contract, declarer)) = final_contract(auction, board.dealer) else {
                continue;
            };
            let level = contract.bid.level.get();
            if level < args.min_level {
                continue;
            }
            let declarer_is_ns = matches!(declarer, Seat::North | Seat::South);
            let side = |is_ns: bool| {
                vul.contains(if is_ns {
                    AbsoluteVulnerability::NS
                } else {
                    AbsoluteVulnerability::EW
                })
            };
            let tricks = u8::from(table[contract.bid.strain].get(declarer));
            let down = i32::from(level) + 6 - i32::from(tricks);
            let doubled = contract.penalty != Penalty::Undoubled;
            // What the (re)double was worth to the side that made it: the
            // declaring side's loss from the penalty actually on the contract.
            let plain = Contract {
                bid: contract.bid,
                penalty: Penalty::Undoubled,
            }
            .score(tricks, side(declarer_is_ns));
            let gain = if doubled {
                imps(i64::from(plain) - i64::from(contract.score(tricks, side(declarer_is_ns))))
            } else {
                0
            };

            let ours = declarer_is_ns == ours_is_ns;
            let vul_we = usize::from(side(ours_is_ns));
            tally.directions[usize::from(!ours)][vul_we].push(doubled, down, gain);
            if ours {
                tally.levels[bucket(level)][vul_we].push(doubled, down, gain);
                if gate_reached(auction, board.dealer, ours_is_ns) {
                    tally.gate[bucket(level)][vul_we].push(doubled, down, gain);
                }
            }
        }
    }
}

fn main() {
    let args = Args::parse();
    assert!(!args.dirs.is_empty(), "usage: probe-doubling <arm-dir>...");
    let mut tally = Tally::default();
    for path in &args.dirs {
        fold(path, &args, &mut tally);
    }

    println!(
        "boards solved {}, min level {}\n",
        tally.boards, args.min_level
    );

    println!("## were the doubles right\n");
    println!("| slice | n doubled | made anyway | down 1 | down 2+ | mean IMPs of the X |");
    println!("| --- | ---: | ---: | ---: | ---: | ---: |");
    for (side, label) in [(0, "we declare"), (1, "they declare")] {
        for (vul, vul_label) in [(0, "NV"), (1, "vul")] {
            let row = tally.directions[side][vul];
            if row.doubled == 0 {
                continue;
            }
            println!(
                "| {label}, {vul_label} | {} | {} ({:.1}%) | {} | {} | {:+.2} |",
                row.doubled,
                row.made_doubled,
                100.0 * ratio(row.made_doubled, row.doubled),
                row.down1,
                row.down2,
                row.imps as f64 / row.doubled as f64,
            );
        }
    }

    for (table, title) in [
        (
            &tally.gate,
            "auctions that reached the gate's trigger — the shipping slice",
        ),
        (&tally.levels, "all auctions — the marginal contrast"),
    ] {
        println!("\n## does q belong on the failing branch only? ({title})\n");
        println!(
            "| level | our vul | n | P(fail) | q = P(X) | P(X \\| fail) | P(X \\| make) | lift |"
        );
        println!("| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |");
        for (vul, vul_label) in [(0, "none"), (1, "both")] {
            for (index, level) in LEVELS.iter().enumerate() {
                let row = table[index][vul];
                if row.n == 0 {
                    continue;
                }
                let p_x_fail = ratio(row.doubled_failed, row.failed);
                let p_x_make = ratio(row.doubled - row.doubled_failed, row.n - row.failed);
                println!(
                    "| {level} | {vul_label} | {} | {:.3} | {:.3} | {:.3} | {:.3} | {:.1}× |",
                    row.n,
                    ratio(row.failed, row.n),
                    ratio(row.doubled, row.n),
                    p_x_fail,
                    p_x_make,
                    p_x_fail / p_x_make,
                );
            }
        }
    }
    println!(
        "\nA lift far above 1 means the doubles are failure-conditioned, so q applies to the\n\
         failing branch only; a lift near 1 means they are indiscriminate and q applies to both.\n\
         Read the gate table: the marginal one mixes in uncontested auctions the gate never sees."
    );
}
