//! Measure the gated live-search floor (AI-bidder M2.3): A/B duplicate matches.
//!
//! The companion to `instinct-floor` and `neural-floor`, for the *thinking*
//! floor.  The [`SearchFloor`][pons::bidding::search_floor::SearchFloor] safety
//! shell — the distilled net used only as a prior to shortlist calls, then each
//! priced by double-dummy cardplay EV ([`ev_all`][pons::bidding::ev_all]) over
//! sampled layouts before bidding the best — runs as a [`Pair`][pons::Pair] floor
//! ([`american_search`][pons::american_search]) against two opponents,
//! each board bid twice duplicate-style with the seats swapped:
//!
//! 1. **vs the deterministic floor** ([`american`]): the hand-written
//!    `instinct()` ladder.  Beating it (strictly positive IMPs/board) is the
//!    whole point of M2.3 — cardplay-grounded judgement over a static rule.
//! 2. **vs the distilled net** ([`american_neural`]): the raw one-forward-pass
//!    policy the search uses as its *prior*.  Search should beat the policy it
//!    proposes from — "net proposes, search disposes".
//!
//! Both matches run over the **same** boards (so the two verdicts are comparable),
//! scored double dummy on the divergent boards and credited to the search ("home")
//! team as IMPs/board with a 95% confidence interval.  Pass `--seed` to fix the
//! boards across runs (e.g. to compare knob settings on identical deals); `--worst`
//! dumps the boards where search swung the most, the cheapest way to see *why* a
//! number came out the way it did — silly contracts (a bug or coordination flaw)
//! versus sensible-but-unlucky ones (variance).
//!
//! # This is slow
//!
//! Every non-forced decision runs a full double-dummy search (128 layouts × up to
//! 8 candidates by default, ~1.4 s each), so a board costs *thousands* of solves,
//! not one.  The default board count is therefore small — a smoke test, not a
//! verdict.  A tight interval needs thousands of boards and a long, patient run
//! (use `--progress` to watch it tick):
//!
//! ```text
//! cargo run --release --features search --example ab-search-floor -- --count 2000 --progress
//! ```

#![allow(clippy::cast_precision_loss)]

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Contract, FullDeal, Hand, Seat, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::bidding::context::relative;
use pons::bidding::{Family, Stance, System};
use pons::scoring::{final_contract, imps, ns_score_contract};
use pons::{Accumulator, american, american_neural, american_search};
use rand::SeedableRng;
use rand::rngs::StdRng;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{hand_hcp, seat_to_act};

/// Measure the live-search floor: A/B duplicate matches with intervals
#[derive(Parser)]
struct Args {
    /// Number of boards per match (dealer rotates per board)
    ///
    /// Small by default: the search is slow (~1.4 s/decision).  Scale up for a
    /// real interval.
    #[arg(short, long, default_value = "50")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Fix the board RNG seed for reproducible runs (default: entropy)
    ///
    /// The same seed deals the same boards, so two runs at different knob
    /// settings can be compared on identical deals (a paired comparison that
    /// cancels board luck).
    #[arg(short, long)]
    seed: Option<u64>,

    /// Dump this many of search's biggest losing and winning boards per match
    ///
    /// The cheap diagnostic: read the boards where search swung the most to tell
    /// silly contracts (a bug or coordination flaw) from variance.  `0` disables.
    #[arg(short, long, default_value = "5")]
    worst: usize,

    /// Print a progress line to stderr every ~10% of boards while bidding
    #[arg(short, long)]
    progress: bool,

    /// Disable rule-replay layout acceptance for the search floor's sampler.
    ///
    /// Replay is the crate default; pass this to fall back to range-only
    /// sampling.  Affects `ev_all` (the search floor) only; the deterministic and
    /// net opponents are untouched, so the paired A/B is a bare run (replay on)
    /// vs this flag (replay off).
    #[arg(long)]
    no_rule_accept: bool,

    /// Keep only deals that can reach a 1NT opening our side defends (a cheap
    /// shape pre-filter), so the slow DD search lands on boards where the 1NT
    /// defense can diverge. `--count` is then the number of such boards.
    #[arg(long)]
    filter: bool,

    /// Write the per-board "vs deterministic" swings (one integer per line) here.
    ///
    /// With a fixed `--seed` the boards are identical across runs, so dumping the
    /// swings from a flag-off and a flag-on run lets a paired diff isolate the
    /// rule-accept effect (board luck and unchanged boards cancel).
    #[arg(long)]
    swings_out: Option<String>,

    /// Skip the second (vs distilled net) match. Halves the runtime when only the
    /// vs-deterministic numbers are wanted (e.g. a paired `--swings-out` A/B).
    #[arg(long)]
    skip_net: bool,
}

/// Cheap shape pre-filter: could this deal reach a 1NT opening our side defends?
///
/// A superset — some seat is a balanced 14-18 opener candidate and an opponent
/// (its LHO or RHO) holds defensive action (a 5+ suit with overcall strength, or
/// a 14+ hand for the penalty double). Concentrates the DD budget on boards where
/// the search floor's 1NT defense can actually diverge.
fn could_defend_1nt(deal: &FullDeal) -> bool {
    Seat::ALL.iter().any(|&opener| {
        let h = deal[opener];
        let lengths = Suit::ASC.map(|s| h[s].len());
        let balanced =
            lengths.iter().all(|&l| l >= 2) && lengths.iter().filter(|&&l| l == 2).count() <= 1;
        if !(balanced && (14..=18).contains(&hand_hcp(h))) {
            return false;
        }
        let lho = Seat::ALL[(opener as usize + 1) % 4];
        let rho = Seat::ALL[(opener as usize + 3) % 4];
        [lho, rho].iter().any(|&d| {
            let hd = deal[d];
            let longest = Suit::ASC.iter().map(|&s| hd[s].len()).max().unwrap_or(0);
            let hcp = hand_hcp(hd);
            (longest >= 5 && (6..=16).contains(&hcp)) || hcp >= 14
        })
    })
}

/// The highest-logit *legal* call, defaulting to a pass
///
/// [`Table::next_call`][pons::bidding::Table::next_call] inlined without the
/// floor-provenance tap (this example measures swing, not activation).
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

/// Bid out one deal: the `home` pair takes the NS seats when `home_is_ns`
fn bid_out(
    home: &Stance,
    away: &Stance,
    home_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();

    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let stance = if seat_is_ns == home_is_ns { home } else { away };
        auction.push(next_call(stance, deal[seat], dealer, vul, &auction));
    }
    auction
}

/// A board to play: a deal and its dealer (dealt once, played by both matches)
struct BoardDeal {
    deal: FullDeal,
    dealer: Seat,
}

/// Deal `count` boards, dealer rotating per board, from the caller's RNG
///
/// With `filter` on, skips deals that can't reach a 1NT defense, so `count` is
/// the number of *kept* boards (dealer rotates over the kept index).
fn make_boards(count: usize, filter: bool, rng: &mut impl rand::Rng) -> Vec<BoardDeal> {
    let mut boards = Vec::with_capacity(count);
    while boards.len() < count {
        let deal = full_deal(rng);
        if filter && !could_defend_1nt(&deal) {
            continue;
        }
        boards.push(BoardDeal {
            dealer: Seat::ALL[boards.len() % 4],
            deal,
        });
    }
    boards
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
    home: &Stance,
    away: &Stance,
    boards: &[BoardDeal],
    vul: AbsoluteVulnerability,
    label: &str,
    progress: bool,
) -> MatchResult {
    // Bid every board at both tables.  Each call may trigger a full double-dummy
    // search inside the floor, so this is the slow part — not the scoring below.
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
    // double dummy in one batch and credit the swing to the home team (NS at
    // table A, EW at table B).
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

    // Perfect-defense doubling is the verdict measure: assume the opponents double
    // every contract that fails (a real defense punishes an overbid).  Scoring a
    // failing overbid undoubled is the wrong model for a bidding A/B — it lets the
    // aggressive search bidder off the hook for exactly the contracts DD over-reaches into.
    let mut swings = vec![0i64; played.len()];
    for (&index, table) in divergent.iter().zip(tables.iter()) {
        let (contract_a, contract_b) = contracts[index];
        let points =
            ns_score_contract(contract_a, table, vul) - ns_score_contract(contract_b, table, vul);
        swings[index] = imps(points);
    }

    MatchResult {
        boards: played,
        swings,
        divergent: divergent.len(),
    }
}

/// Print one match's IMPs/board with a 95% CI and a "should beat" verdict
///
/// M2.3's win condition is *strictly positive* IMPs/board: the search team
/// should be ahead, with the interval excluding zero once enough boards are in.
fn report(label: &str, opponent: &str, result: &MatchResult, vul: AbsoluteVulnerability) {
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
    println!("Home (search) team: {total:+} IMPs, {mean:+.3} IMPs/board");
    println!("  95% CI: [{lo:+.3}, {hi:+.3}]  (SE {se:.3}, n = {count})");

    let excludes_zero = !(lo..=hi).contains(&0.0);
    let verdict = match (excludes_zero, mean > 0.0) {
        (true, true) => "search ahead, CI excludes 0 — beats it (the M2.3 goal)",
        (true, false) => "search BEHIND, CI excludes 0 — inspect divergent boards",
        (false, _) => "CI contains 0 — too few boards to call; collect more",
    };
    println!("  Should beat {opponent} (strictly positive): {verdict}");
}

/// A contract and its declarer rendered for the board dump
fn fmt_contract(contract: Option<(Contract, Seat)>) -> String {
    match contract {
        Some((contract, declarer)) => format!("{contract} by {declarer}"),
        None => "passed out".to_owned(),
    }
}

/// Dump the boards where search swung the most — the cheap "why" diagnostic
///
/// Shows the `worst` biggest losses and biggest gains: a few disasters with sane
/// gains points to variance, a systematic tilt or absurd contracts to a bug or
/// the partner-coordination gap (the rollout assumes a net partner, but partner
/// is search).
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
        &format!("{} worst for search vs {opponent} (losses)", losses.len()),
        result,
        &losses,
    );
    dump_boards(
        &format!("{} best for search vs {opponent} (gains)", gains.len()),
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
            "\n  Board {index} (dealer {}, swing {:+} IMPs to search)",
            board.dealer, result.swings[index],
        );
        println!("    deal: {}", board.deal);
        println!(
            "    A (search N/S): {}  ->  {}",
            board.table_a,
            fmt_contract(final_contract(&board.table_a, board.dealer)),
        );
        println!(
            "    B (search E/W): {}  ->  {}",
            board.table_b,
            fmt_contract(final_contract(&board.table_b, board.dealer)),
        );
    }
}

fn main() {
    let args = Args::parse();

    // The flag is a thread-local read inside `ev_all`; this example bids on the
    // main thread (the sequential `duplicate_match` loop), so one call here covers
    // every decision. Replay is the crate default; `--no-rule-accept` opts out.
    // The DD solver below is separate and untouched by it.
    pons::bidding::inference::set_rule_accept(!args.no_rule_accept);

    let search = american_search().against(Family::NATURAL);
    let deterministic = american().against(Family::NATURAL);

    // Deal the boards once so both matches play identical deals (and a `--seed`
    // makes them reproducible across runs).
    let boards = match args.seed {
        Some(seed) => make_boards(args.count, args.filter, &mut StdRng::seed_from_u64(seed)),
        None => make_boards(args.count, args.filter, &mut rand::rng()),
    };

    println!(
        "AI-bidder M2.3: live double-dummy search floor vs the deterministic floor and the \
         distilled net\n({} boards per match{}; the search is slow — thousands of boards need a \
         long run)",
        args.count,
        args.seed.map_or(String::new(), |s| format!(", seed {s}")),
    );

    let vs_deterministic = duplicate_match(
        &search,
        &deterministic,
        &boards,
        args.vulnerability,
        "vs deterministic",
        args.progress,
    );
    report(
        "Search floor vs deterministic floor",
        "the deterministic floor",
        &vs_deterministic,
        args.vulnerability,
    );
    dump_extremes("the deterministic floor", &vs_deterministic, args.worst);

    // Dump the vs-deterministic per-board swings for a paired off-vs-on diff.
    if let Some(path) = &args.swings_out {
        let lines: String = vs_deterministic
            .swings
            .iter()
            .map(|s| format!("{s}\n"))
            .collect();
        std::fs::write(path, lines).expect("write swings file");
    }

    if !args.skip_net {
        let neural = american_neural().against(Family::NATURAL);
        let vs_neural = duplicate_match(
            &search,
            &neural,
            &boards,
            args.vulnerability,
            "vs net",
            args.progress,
        );
        report(
            "Search floor vs distilled net",
            "the raw net prior",
            &vs_neural,
            args.vulnerability,
        );
        dump_extremes("the raw net prior", &vs_neural, args.worst);
    }
}
