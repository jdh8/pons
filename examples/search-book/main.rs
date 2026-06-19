//! Measure the leaf-pricing search bidder (AI-bidder M7.0): A/B duplicate matches.
//!
//! The companion to `search-floor`, for "search at every leaf".  Today
//! [`american_search`][pons::american_search] runs the double-dummy search only
//! where the book is *silent* (the contested fallback floor); M7.0's
//! [`SearchBook`][pons::bidding::search_floor::SearchBook]
//! ([`american_search_book`][pons::american_search_book]) widens it to every
//! non-forced *authored book leaf* — the leaf's logits become the search prior,
//! so cardplay re-judges among the calls the rule proposed (∪ the net's natural
//! alternatives).  "Rules propose, DD disposes."
//!
//! The two are the A/B arms of M7: `american_search_book` (DD on leaves too,
//! the **treatment**, "home") is measured against
//!
//! 1. **`american_search`** (DD off-book only, the **baseline** arm): the M7.0
//!    win condition is *parity-or-better* — leaf-pricing should not lose, and
//!    should win where cardplay sees through an inflexible authored weight.
//! 2. **`american`** (the deterministic `instinct()` floor): the absolute
//!    reference both search arms share underneath.
//!
//! Both matches run over the **same** boards (so the two verdicts are comparable),
//! scored double dummy on the divergent boards, perfect-defense default (failing
//! contracts priced doubled), and credited to the leaf-pricing ("home") team as
//! IMPs/board with a 95% confidence interval.  Pass `--seed` to fix the boards;
//! `--worst` dumps the boards where leaf-pricing swung the most.
//!
//! # This is *very* slow
//!
//! Unlike `search-floor` (which searches only the off-book fall-through), this
//! runs a full double-dummy search at **every non-forced on-book decision** — far
//! more searches per board.  The default board count is therefore tiny — a smoke
//! test, not a verdict.  A real interval needs a long, patient run:
//!
//! ```text
//! cargo run --release --features search --example search-book -- --count 2000 --progress
//! ```

#![allow(clippy::cast_precision_loss)]

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Contract, FullDeal, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::bidding::context::relative;
use pons::bidding::{Family, System};
use pons::scoring::{final_contract, imps, ns_score};
use pons::{Accumulator, american, american_search, american_search_book};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Measure the leaf-pricing search bidder: A/B duplicate matches with intervals
#[derive(Parser)]
struct Args {
    /// Number of boards per match (dealer rotates per board)
    ///
    /// Tiny by default: every non-forced on-book decision runs a DD search, so
    /// this is slower than `search-floor`.  Scale up for a real interval.
    #[arg(short, long, default_value = "20")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Fix the board RNG seed for reproducible runs (default: entropy)
    #[arg(short, long)]
    seed: Option<u64>,

    /// Dump this many of the biggest losing and winning boards per match
    #[arg(short, long, default_value = "5")]
    worst: usize,

    /// Print a progress line to stderr every ~10% of boards while bidding
    #[arg(short, long)]
    progress: bool,
}

/// The seat acting after `len` calls from `dealer`
const fn seat_to_act(dealer: Seat, len: usize) -> Seat {
    Seat::ALL[(dealer as usize + len) % 4]
}

/// The highest-logit *legal* call, defaulting to a pass
///
/// [`Table::next_call`][pons::bidding::Table::next_call] inlined without the
/// floor-provenance tap, generalized over any [`System`] so the leaf-pricing
/// wrapper and a bound [`Stance`][pons::Stance] seat the same way.
fn next_call(
    system: &dyn System,
    hand: Hand,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    auction: &Auction,
) -> Call {
    let seat = seat_to_act(dealer, auction.len());
    let Some(logits) = system.classify(hand, relative(vul, seat), auction) else {
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

/// Bid out one deal: the `home` pair takes the NS seats when `home_is_ns`
fn bid_out(
    home: &dyn System,
    away: &dyn System,
    home_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();

    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let system = if seat_is_ns == home_is_ns { home } else { away };
        auction.push(next_call(system, deal[seat], dealer, vul, &auction));
    }
    auction
}

/// A board to play: a deal and its dealer (dealt once, played by both matches)
struct BoardDeal {
    deal: FullDeal,
    dealer: Seat,
}

/// Deal `count` boards, dealer rotating per board, from the caller's RNG
fn make_boards(count: usize, rng: &mut impl rand::Rng) -> Vec<BoardDeal> {
    (0..count)
        .map(|index| BoardDeal {
            dealer: Seat::ALL[index % 4],
            deal: full_deal(rng),
        })
        .collect()
}

/// One played board: the deal and both tables' auctions
struct Board {
    deal: FullDeal,
    dealer: Seat,
    /// Table A: home pair sits North/South
    table_a: Auction,
    /// Table B: home pair sits East/West
    table_b: Auction,
}

/// The outcome of one duplicate match
struct MatchResult {
    /// Every played board, for the worst/best-board dump
    boards: Vec<Board>,
    /// Per-board IMP swing to the home team (0 on non-divergent boards)
    swings: Vec<i64>,
    /// Boards whose two tables reached different contracts
    divergent: usize,
}

/// Run a duplicate match over `boards`, crediting swings to `home`
fn duplicate_match(
    home: &dyn System,
    away: &dyn System,
    boards: &[BoardDeal],
    vul: AbsoluteVulnerability,
    label: &str,
    progress: bool,
) -> MatchResult {
    // Bid every board at both tables.  Each call may trigger a full double-dummy
    // search, so this is the slow part — not the scoring below.
    let step = (boards.len() / 10).max(1);
    let played: Vec<Board> = boards
        .iter()
        .enumerate()
        .map(|(index, board)| {
            if progress && index > 0 && index % step == 0 {
                eprintln!("  [{label}] {index}/{} boards bid", boards.len());
            }
            Board {
                deal: board.deal,
                dealer: board.dealer,
                table_a: bid_out(home, away, true, board.dealer, vul, &board.deal),
                table_b: bid_out(home, away, false, board.dealer, vul, &board.deal),
            }
        })
        .collect();

    // Only boards whose tables reach different results can swing; solve those
    // double dummy in one batch and credit the swing to the home team.
    let contracts: Vec<_> = played
        .iter()
        .map(|board| {
            (
                final_contract(&board.table_a, board.dealer),
                final_contract(&board.table_b, board.dealer),
            )
        })
        .collect();
    let divergent: Vec<usize> = (0..played.len())
        .filter(|&index| contracts[index].0 != contracts[index].1)
        .collect();
    let deals: Vec<FullDeal> = divergent.iter().map(|&index| played[index].deal).collect();
    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    let mut swings = vec![0i64; played.len()];
    for (&index, table) in divergent.iter().zip(tables.iter()) {
        let (contract_a, contract_b) = contracts[index];
        let points = ns_score(contract_a, table, vul) - ns_score(contract_b, table, vul);
        swings[index] = imps(points);
    }

    MatchResult {
        boards: played,
        swings,
        divergent: divergent.len(),
    }
}

/// Print one match's IMPs/board with a 95% CI and a verdict
fn report(
    label: &str,
    opponent: &str,
    parity_ok: bool,
    result: &MatchResult,
    vul: AbsoluteVulnerability,
) {
    let count = result.swings.len();
    let mut acc = Accumulator::new();
    for &swing in &result.swings {
        acc.push(swing as f64);
    }
    let stats = acc.sample();
    let mean = stats.mean();
    let se = stats.sd() / (count.max(1) as f64).sqrt();
    let (lo, hi) = (mean - 1.96 * se, mean + 1.96 * se);
    let total: i64 = result.swings.iter().sum();

    println!("\n=== {label}: {count} boards, vulnerability {vul} ===");
    println!(
        "Divergent boards: {} of {count} ({:.0}%)",
        result.divergent,
        100.0 * result.divergent as f64 / count.max(1) as f64,
    );
    println!("Home (leaf-pricing) team: {total:+} IMPs, {mean:+.3} IMPs/board");
    println!("  95% CI: [{lo:+.3}, {hi:+.3}]  (SE {se:.3}, n = {count})");

    let excludes_zero = !(lo..=hi).contains(&0.0);
    // The baseline match (`american_search`) wants *parity-or-better*; the
    // deterministic match wants a strict win.
    let verdict = if parity_ok {
        match (excludes_zero, mean >= 0.0) {
            (false, _) => {
                "CI contains 0 — parity (the M7.0 floor); collect more for a sharper read"
            }
            (true, true) => "ahead, CI excludes 0 — leaf-pricing wins (beyond parity)",
            (true, false) => {
                "BEHIND, CI excludes 0 — leaf-pricing regresses; inspect divergent boards"
            }
        }
    } else {
        match (excludes_zero, mean > 0.0) {
            (true, true) => "ahead, CI excludes 0 — beats it",
            (true, false) => "BEHIND, CI excludes 0 — inspect divergent boards",
            (false, _) => "CI contains 0 — too few boards to call; collect more",
        }
    };
    println!("  vs {opponent}: {verdict}");
}

/// A contract and its declarer rendered for the board dump
fn fmt_contract(contract: Option<(Contract, Seat)>) -> String {
    match contract {
        Some((contract, declarer)) => format!("{contract} by {declarer}"),
        None => "passed out".to_owned(),
    }
}

/// Dump the boards where leaf-pricing swung the most — the cheap "why" diagnostic
fn dump_extremes(opponent: &str, result: &MatchResult, worst: usize) {
    if worst == 0 {
        return;
    }
    let mut swung: Vec<usize> = (0..result.swings.len())
        .filter(|&index| result.swings[index] != 0)
        .collect();
    if swung.is_empty() {
        println!("\n  (no boards swung — nothing to dump)");
        return;
    }
    swung.sort_by_key(|&index| result.swings[index]); // ascending: worst losses first

    let losses: Vec<usize> = swung.iter().take(worst).copied().collect();
    let gains: Vec<usize> = swung.iter().rev().take(worst).copied().collect();
    dump_boards(
        &format!(
            "{} worst for leaf-pricing vs {opponent} (losses)",
            losses.len()
        ),
        result,
        &losses,
    );
    dump_boards(
        &format!(
            "{} best for leaf-pricing vs {opponent} (gains)",
            gains.len()
        ),
        result,
        &gains,
    );
}

/// Print the deal, both auctions, and both contracts for the given boards
fn dump_boards(title: &str, result: &MatchResult, indices: &[usize]) {
    println!("\n  --- {title} ---");
    for &index in indices {
        let board = &result.boards[index];
        println!(
            "\n  Board {index} (dealer {}, swing {:+} IMPs to leaf-pricing)",
            board.dealer, result.swings[index],
        );
        println!("    deal: {}", board.deal);
        println!(
            "    A (leaf-pricing N/S): {}  ->  {}",
            board.table_a,
            fmt_contract(final_contract(&board.table_a, board.dealer)),
        );
        println!(
            "    B (leaf-pricing E/W): {}  ->  {}",
            board.table_b,
            fmt_contract(final_contract(&board.table_b, board.dealer)),
        );
    }
}

fn main() {
    let args = Args::parse();

    let book = american_search_book(Family::NATURAL);
    let baseline = american_search().against(Family::NATURAL);
    let deterministic = american().against(Family::NATURAL);

    // Deal the boards once so both matches play identical deals.
    let boards = match args.seed {
        Some(seed) => make_boards(args.count, &mut StdRng::seed_from_u64(seed)),
        None => make_boards(args.count, &mut rand::rng()),
    };

    println!(
        "AI-bidder M7.0: search at every authored leaf vs DD-off-book-only and the \
         deterministic floor\n({} boards per match{}; this searches every non-forced on-book \
         decision — very slow, thousands of boards need a long run)",
        args.count,
        args.seed.map_or(String::new(), |s| format!(", seed {s}")),
    );

    let vs_baseline = duplicate_match(
        &book,
        &baseline,
        &boards,
        args.vulnerability,
        "vs search-floor",
        args.progress,
    );
    report(
        "Leaf-pricing vs DD-off-book-only (american_search)",
        "american_search (the M7.0 parity bar)",
        true,
        &vs_baseline,
        args.vulnerability,
    );
    dump_extremes("american_search", &vs_baseline, args.worst);

    let vs_deterministic = duplicate_match(
        &book,
        &deterministic,
        &boards,
        args.vulnerability,
        "vs deterministic",
        args.progress,
    );
    report(
        "Leaf-pricing vs deterministic floor (american)",
        "the deterministic floor",
        false,
        &vs_deterministic,
        args.vulnerability,
    );
    dump_extremes("the deterministic floor", &vs_deterministic, args.worst);
}
