//! AI-bidder **Side-track S.1** — the external eval anchor, *scoring* half.
//!
//! Reads a `Dump` of boards bid by [`bba-gen`](../bba-gen/main.rs), solves the
//! divergent boards double dummy (`ddss`, the only parallel part), and prints the
//! IMPs/board headline, the per-bucket 1NT/defense/UvU breakdowns, and the worst
//! boards.  **No EPBot / no FFI**: it never loads `libEPBot.so`, so it can
//! saturate the box while a fresh `bba-gen` (single-threaded) runs alongside, and
//! a cached board file can be re-scored many ways (plain vs PD) without paying the
//! slow FFI bidding again.
//!
//! ```text
//! cargo run --release --features serde --example bba-gen -- --count 1000 \
//!   | cargo run --release --features serde --example bba-score
//! cargo run --release --features serde --example bba-score -- boards.json --score pd
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Bid, Contract, FullDeal, Seat, Strain};
use pons::scoring::{final_contract, ns_score_bid, ns_score_contract};
use std::collections::BTreeMap;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Dump, mean_with_ci, score_boards, seat_to_act};

/// Score a `bba-gen` board dump and report IMPs/board (the scoring half of the
/// A/B duplicate match)
#[derive(Parser)]
struct Args {
    /// Board dump from `bba-gen` (default: stdin)
    input: Option<String>,

    /// Score with plain DD (`plain`, the contract's bid penalty) or
    /// perfect-defense (`pd`, double any contract that fails double dummy)
    #[arg(long, default_value = "plain")]
    score: String,

    /// Re-price at this vulnerability instead of the dump's (a what-if — the
    /// boards were *bid* at the dump's vulnerability and are not re-bid)
    #[arg(short, long)]
    vulnerability: Option<AbsoluteVulnerability>,

    /// Number of worst (most-lost) divergent boards to dump
    #[arg(short, long, default_value = "15")]
    top: usize,

    /// Filter the we-defend worst-board dump to boards whose first NS call after
    /// their 1NT matches this label (e.g. `X` for penalty-double boards only).
    #[arg(long)]
    action: Option<String>,
}

/// Render an auction with leading passes kept, calls space-joined
fn show_auction(auction: &Auction) -> String {
    auction
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Short bucket label for a responder/defender call (`P`, `2♣`, `X`, …)
fn action_label(call: Call) -> String {
    match call {
        Call::Pass => "P".into(),
        Call::Double => "X".into(),
        Call::Redouble => "XX".into(),
        Call::Bid(bid) => bid.to_string(),
    }
}

/// If this auction's *opening* call is 1NT, its index and whether the opener is
/// North/South.  The opening requirement (all prior calls passes) excludes a
/// `1♣-P-1NT` rebid — we want 1NT *openings* only.
fn opening_1nt(auction: &[Call], dealer: Seat) -> Option<(usize, bool)> {
    let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
    let index = auction.iter().position(|&call| call == one_nt)?;
    if auction[..index].iter().any(|&call| call != Call::Pass) {
        return None;
    }
    let opener_ns = matches!(seat_to_act(dealer, index), Seat::North | Seat::South);
    Some((index, opener_ns))
}

/// Our first call after the 1NT opening.  At table A our pair sits North/South,
/// so this is our action whether we opened (responder) or defended (overcaller),
/// skipping any opposing call in between.  Captures `Pass` too.
fn first_ns_call_after(auction: &[Call], dealer: Seat, nt_index: usize) -> Option<Call> {
    auction[nt_index + 1..]
        .iter()
        .enumerate()
        .find_map(|(off, &call)| {
            matches!(
                seat_to_act(dealer, nt_index + 1 + off),
                Seat::North | Seat::South
            )
            .then_some(call)
        })
}

/// The 1NT opener's partner's (responder's) first call after the opening — i.e.
/// what the opponents responded once we did *not* overcall.  Their partner sits
/// two seats after the opener.  `None` if the responder never gets to call.
fn responder_call_after(auction: &[Call], dealer: Seat, nt_index: usize) -> Option<Call> {
    let responder = seat_to_act(dealer, nt_index + 2);
    auction[nt_index + 1..]
        .iter()
        .enumerate()
        .find_map(|(off, &call)| {
            (seat_to_act(dealer, nt_index + 1 + off) == responder).then_some(call)
        })
}

#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let dump: Dump = match args.input.as_deref() {
        Some(path) => serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(path)?))?,
        None => serde_json::from_reader(std::io::stdin().lock())?,
    };
    let boards = &dump.boards;
    let count = boards.len();
    // The boards were bid at `dump.vulnerability`; `--vulnerability` re-prices the
    // same contracts at a different one (a what-if — it does not re-bid).
    let vul = args.vulnerability.unwrap_or(dump.vulnerability);
    let pd = match args.score.as_str() {
        "plain" => false,
        "pd" => true,
        other => anyhow::bail!("--score must be plain|pd, got {other:?}"),
    };
    // Plain DD prices the contract's actual penalty; PD doubles any contract that
    // fails double-dummy (perfect-defense), which punishes a weak overbid.
    let score = |c: Option<(Contract, Seat)>, table: &_, vul| {
        if pd {
            ns_score_bid(c.map(|(ct, s)| (ct.bid, s)), table, vul)
        } else {
            ns_score_contract(c, table, vul)
        }
    };

    let contracts: Vec<_> = boards
        .iter()
        .map(|board| {
            (
                final_contract(&board.table_a, board.dealer),
                final_contract(&board.table_b, board.dealer),
            )
        })
        .collect();
    let deals: Vec<FullDeal> = boards.iter().map(|board| board.deal).collect();
    let scored = score_boards(&contracts, &deals, vul, score);
    let mut swings = scored.swings;

    let (mean, half_width) = mean_with_ci(&scored.board_imps);
    println!(
        "=== {} (us) vs {} (them): {count} boards, vulnerability {} ===",
        dump.our_label, dump.their_label, vul,
    );
    println!(
        "Divergent boards: {} of {count} ({:.0}%)",
        scored.divergent.len(),
        100.0 * scored.divergent.len() as f64 / count.max(1) as f64,
    );
    println!(
        "Our pair: {:+} points, {:+} IMPs\n\
         IMPs/board: {mean:+.3}  (95% CI [{:+.3}, {:+.3}])",
        scored.total_points,
        scored.total_imps,
        mean - half_width,
        mean + half_width,
    );
    if vul != dump.vulnerability {
        println!(
            "(re-priced at vulnerability {vul}; the boards were bid at {} and are not re-bid)",
            dump.vulnerability,
        );
    }

    // Isolate the 1NT subset, keyed on table A (where our pair sits NS): boards
    // whose opening call is 1NT, split by who opened (NS = our opening, EW = our
    // defense), and bucketed by our first call so a leak localizes to a single
    // continuation.
    let mut open = (0i64, 0i64);
    let mut open_by: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    let mut defend = (0i64, 0i64);
    let mut defend_by: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    // Same we-defend boards, re-bucketed: DIRECT (we acted over 1NT), CONT <call>
    // (we passed, they responded with <call>), or QUIET (we passed, they passed).
    let mut defend_shape_by: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    // Focus subset: we open 1NT and the overcall is exactly 2NT — the UvU subset.
    let two_nt = Call::Bid(Bid::new(2, Strain::Notrump));
    let mut uvu = (0i64, 0i64);
    let mut uvu_by: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    for &(index, _points, imp) in &swings {
        let board = &boards[index];
        let Some((nt_index, opener_ns)) = opening_1nt(&board.table_a, board.dealer) else {
            continue;
        };
        let our_direct = first_ns_call_after(&board.table_a, board.dealer, nt_index);
        let key = our_direct.map_or_else(|| "(none)".into(), action_label);
        let is_uvu = opener_ns && board.table_a.get(nt_index + 1) == Some(&two_nt);
        let (sum, by) = if opener_ns {
            (&mut open, &mut open_by)
        } else {
            (&mut defend, &mut defend_by)
        };
        sum.0 += 1;
        sum.1 += imp;
        let entry = by.entry(key.clone()).or_default();
        entry.0 += 1;
        entry.1 += imp;
        if !opener_ns {
            let shape = match our_direct {
                Some(call) if call != Call::Pass => "DIRECT (we bid over 1NT)".to_string(),
                _ => match responder_call_after(&board.table_a, board.dealer, nt_index) {
                    Some(call) if call != Call::Pass => format!("CONT {}", action_label(call)),
                    _ => "QUIET (we passed, they passed)".to_string(),
                },
            };
            let entry = defend_shape_by.entry(shape).or_default();
            entry.0 += 1;
            entry.1 += imp;
        }
        if is_uvu {
            uvu.0 += 1;
            uvu.1 += imp;
            let entry = uvu_by.entry(key).or_default();
            entry.0 += 1;
            entry.1 += imp;
        }
    }
    // Print a bucket only when it has data (which buckets apply is implicit in how
    // the boards were generated, so an empty one is just noise here).
    let report = |title: &str, sum: (i64, i64), by: &BTreeMap<String, (i64, i64)>| {
        if sum.0 == 0 {
            return;
        }
        println!(
            "\n=== {title} === ({} divergent boards, {:+} IMPs, {:+.3} IMPs/board)",
            sum.0,
            sum.1,
            sum.1 as f64 / sum.0.max(1) as f64,
        );
        for (action, &(boards_n, imps_won)) in by {
            println!(
                "  {action:<5} {boards_n:>5} boards  {imps_won:+6} IMPs  ({:+.3} IMPs/board)",
                imps_won as f64 / boards_n.max(1) as f64,
            );
        }
    };
    report("OUR 1NT openings (we open 1NT)", open, &open_by);
    report(
        "OUR defense vs their 1NT (they open 1NT)",
        defend,
        &defend_by,
    );
    report(
        "OUR defense vs their 1NT, by auction shape (DIRECT vs CONTinuation)",
        defend,
        &defend_shape_by,
    );
    report("OUR 1NT-(2NT) responses (focus)", uvu, &uvu_by);

    // The boards we lost by the most: where their side out-bid ours.  Sort by IMP
    // swing ascending (most negative first), break ties by points.  One renderer,
    // used for the global ranking and the we-defend-1NT subset.
    let dump_rows = |title: &str, rows: &[(usize, i64, i64)]| {
        println!("\n=== {title} ===");
        for &(index, points, imp) in rows {
            let board = &boards[index];
            let (contract_a, contract_b) = contracts[index];
            println!(
                "\n[board {index}] dealer {:?}, swing {points:+} pts / {imp:+} IMPs",
                board.dealer
            );
            println!("  {}", board.deal.display(Seat::North));
            println!(
                "  ours NS @ A: {}  -> {}",
                show_auction(&board.table_a),
                contract_a.map_or("passed out".into(), |(c, s)| format!("{c} by {s:?}")),
            );
            println!(
                "  ours EW @ B: {}  -> {}",
                show_auction(&board.table_b),
                contract_b.map_or("passed out".into(), |(c, s)| format!("{c} by {s:?}")),
            );
        }
    };
    swings.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| a.1.cmp(&b.1)));
    let worst: Vec<_> = swings.iter().take(args.top).copied().collect();
    dump_rows(
        &format!("Worst {} divergent boards for us (their edge)", worst.len()),
        &worst,
    );
    let worst_defend: Vec<_> = swings
        .iter()
        .filter(|&&(index, ..)| {
            let board = &boards[index];
            let Some((nt_index, false)) = opening_1nt(&board.table_a, board.dealer) else {
                return false;
            };
            // Optional: keep only boards whose first NS call after 1NT matches.
            args.action.as_deref().is_none_or(|want| {
                first_ns_call_after(&board.table_a, board.dealer, nt_index)
                    .is_some_and(|call| action_label(call) == want)
            })
        })
        .take(args.top)
        .copied()
        .collect();
    dump_rows(
        &format!(
            "Worst {} we-defend-1NT boards (BBA opens 1NT, we defend){}",
            worst_defend.len(),
            args.action
                .as_deref()
                .map_or_else(String::new, |a| format!(", first NS call = {a}")),
        ),
        &worst_defend,
    );
    Ok(())
}
