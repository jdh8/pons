//! Measure the Unusual-vs-Unusual structure over 1NT-(2NT both minors): an A/B
//! duplicate match.
//!
//! When an opponent overcalls our 1NT with a both-minors 2NT, the instinct floor
//! has nothing to say — responder passes or guesses.  The UvU structure
//! ([`set_uvu`][pons::bidding::american::set_uvu]) gives responder a penalty `X`,
//! INV+ Stayman/transfer cues (`3♣`/`3♦`), FG+ 5-5-majors splinters (`4♣`/`4♦`)
//! and symmetric Smolen.  Is it worth points, and at what strength floors?
//!
//! Both pairs play `american`; the *environment* is fixed — every defender
//! overcalls a 1NT with the both-minors 2NT
//! ([`set_unusual_notrump_defense`][pons::bidding::american::set_unusual_notrump_defense]),
//! so the `1NT-(2NT)` auction arises at both tables.  The toggled feature is the
//! UvU *responder* structure: the feature pair plays it (at the `--x-floor` /
//! `--cue-floor` swept floors), the other floors the auction.  Each board is bid
//! twice (the feature pair NS at table A, EW at table B); boards whose contracts
//! diverge are scored double dummy ([`ns_score_contract`]) and credited to the
//! feature team.
//!
//! ```text
//! cargo run --release --example ab-uvu -- --x-floor 9 --cue-floor 8 --count 200000
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Hand, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::american::{
    set_unusual_notrump_defense, set_uvu, set_uvu_cue_floor, set_uvu_natural_floor, set_uvu_x_floor,
};
use pons::bidding::{Stance, Tag};
use pons::scoring::{final_contract, imps, ns_score_contract};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;
use std::collections::BTreeMap;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{next_call, seat_to_act};

/// Measure the UvU structure over 1NT-(2NT both minors): an A/B duplicate match
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "20000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Deal seed (reproducible boards)
    #[arg(long, default_value = "0")]
    seed: u64,

    /// Responder's penalty-double HCP floor — the X strength sweep knob
    #[arg(long, default_value = "9")]
    x_floor: u8,

    /// Responder's INV+ cue-bid points floor — the cue strength sweep knob
    #[arg(long, default_value = "8")]
    cue_floor: u8,

    /// Length floor for responder's weak natural 3♥/3♠ escape (6 = a six-bagger;
    /// 5 lets a five-card major escape a bad defence — the sweep knob)
    #[arg(long, default_value = "6")]
    natural_floor: u8,

    /// Opponent both-minors 2NT overcall lower points bound (fixed environment)
    #[arg(long, default_value = "5")]
    opp_lo: u8,

    /// Opponent both-minors 2NT overcall upper points bound (fixed environment)
    #[arg(long, default_value = "11")]
    opp_hi: u8,

    /// Print this many divergent boards (auction + contracts) for inspection
    #[arg(long, default_value = "0")]
    show: usize,

    /// Disable the shape pre-filter (count then means raw deals, not kept ones)
    #[arg(long)]
    no_filter: bool,
}

/// A balanced 15-17 HCP hand — a 1NT-opener candidate
fn is_1nt_opener(hand: Hand) -> bool {
    let len = Suit::ASC.map(|s| hand[s].len());
    let balanced = len.iter().all(|&l| l >= 2) && len.iter().filter(|&&l| l == 2).count() <= 1;
    let hcp: u8 = Suit::ASC.iter().map(|&s| holding_hcp::<u8>(hand[s])).sum();
    balanced && (15..=17).contains(&hcp)
}

/// At least 5-5 in the minors — a both-minors 2NT-overcall candidate
fn is_both_minors(hand: Hand) -> bool {
    hand[Suit::Clubs].len() >= 5 && hand[Suit::Diamonds].len() >= 5
}

/// One side holds a 1NT opener and the *other* holds a 5-5-minors overcaller, so
/// the `1NT-(2NT)` auction can arise (densifies the rare divergent subset)
fn relevant(deal: &FullDeal) -> bool {
    let ns_opener = is_1nt_opener(deal[Seat::North]) || is_1nt_opener(deal[Seat::South]);
    let ew_opener = is_1nt_opener(deal[Seat::East]) || is_1nt_opener(deal[Seat::West]);
    let ns_minors = is_both_minors(deal[Seat::North]) || is_both_minors(deal[Seat::South]);
    let ew_minors = is_both_minors(deal[Seat::East]) || is_both_minors(deal[Seat::West]);
    (ns_opener && ew_minors) || (ew_opener && ns_minors)
}

/// Short bucket label for a call (`P` / `X` / `XX` / the bid)
fn action_label(call: Call) -> String {
    match call {
        Call::Pass => "P".into(),
        Call::Double => "X".into(),
        Call::Redouble => "XX".into(),
        Call::Bid(bid) => bid.to_string(),
    }
}

/// The responder's call to `[1NT, (2NT)]`, plus whether the opener sits NS
///
/// After an opening `1NT` (all prior calls passes) immediately overcalled with
/// `2NT`, the next call is the opener's partner — our UvU responder. Returns that
/// call and the opener's side, so the caller reads the *feature* response from the
/// table where the feature side opened (table A when NS, table B when EW).
fn uvu_response(auction: &[Call], dealer: Seat) -> Option<(Call, bool)> {
    let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
    let two_nt = Call::Bid(Bid::new(2, Strain::Notrump));
    let index = auction.iter().position(|&call| call == one_nt)?;
    if auction[..index].iter().any(|&call| call != Call::Pass) {
        return None;
    }
    if auction.get(index + 1) != Some(&two_nt) {
        return None;
    }
    let opener_ns = matches!(seat_to_act(dealer, index), Seat::North | Seat::South);
    auction.get(index + 2).map(|&call| (call, opener_ns))
}

/// Bid out one table; the feature pair plays UvU, the other floors it
///
/// Both stances are baked: the UvU responder structure and the opponents'
/// both-minors 2NT overcall are read at book construction, so picking the stance
/// per seat is all that is needed (safe under Rayon — no per-call thread-locals).
fn bid_out(
    feature: &Stance,
    baseline: &Stance,
    feature_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let stance = if seat_is_ns == feature_is_ns {
            feature
        } else {
            baseline
        };
        auction.push(next_call(stance, deal[seat], dealer, vul, &auction));
    }
    auction
}

/// One board: the deal and both tables' auctions
struct Board {
    deal: FullDeal,
    dealer: Seat,
    table_a: Auction,
    table_b: Auction,
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();

    // Two stances differing only in the UvU responder structure; the opponents'
    // both-minors 2NT overcall (the environment that creates the auction) is on
    // for both, so the divergence isolates the UvU response.
    let range = Some((args.opp_lo, args.opp_hi));
    set_unusual_notrump_defense(range);
    set_uvu(false);
    let baseline = american().against(Tag::NATURAL);
    set_unusual_notrump_defense(range);
    set_uvu(true);
    set_uvu_x_floor(args.x_floor);
    set_uvu_cue_floor(args.cue_floor);
    set_uvu_natural_floor(args.natural_floor);
    let feature = american().against(Tag::NATURAL);

    // Deal sequentially (seeded, reproducible); keep only deals where the
    // auction can arise, until `count` pass; bid both tables in parallel.
    let mut rng = StdRng::seed_from_u64(args.seed);
    let mut scanned = 0usize;
    let mut kept: Vec<FullDeal> = Vec::with_capacity(args.count);
    while kept.len() < args.count {
        let deal = full_deal(&mut rng);
        scanned += 1;
        if args.no_filter || relevant(&deal) {
            kept.push(deal);
        }
    }
    eprintln!(
        "scanned {scanned} for {} boards ({:.1}% kept); bidding...",
        kept.len(),
        100.0 * kept.len() as f64 / scanned.max(1) as f64,
    );
    let deals: Vec<(Seat, FullDeal)> = kept
        .into_iter()
        .enumerate()
        .map(|(index, deal)| (Seat::ALL[index % 4], deal))
        .collect();
    let boards: Vec<Board> = deals
        .par_iter()
        .map(|&(dealer, deal)| Board {
            deal,
            dealer,
            table_a: bid_out(&feature, &baseline, true, dealer, args.vulnerability, &deal),
            table_b: bid_out(
                &feature,
                &baseline,
                false,
                dealer,
                args.vulnerability,
                &deal,
            ),
        })
        .collect();

    // Only boards whose tables reach different contracts can swing; solve those
    // double dummy (on the main thread) and credit the swing to the UvU team.
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
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&index| boards[index].deal).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    let mut total_points = 0i64;
    let mut total_imps = 0i64;
    let mut by_call: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    let mut shown = 0;
    for (&index, table) in divergent.iter().zip(tables.iter()) {
        let (contract_a, contract_b) = contracts[index];
        let swing = ns_score_contract(contract_a, table, args.vulnerability)
            - ns_score_contract(contract_b, table, args.vulnerability);
        total_points += swing;
        total_imps += imps(swing);

        // Attribute the swing to the feature side's UvU response, read from the
        // table where the feature side opened (A when NS opened, else B).
        let board = &boards[index];
        let response = uvu_response(&board.table_a, board.dealer).and_then(|(_, opener_ns)| {
            let uvu_table = if opener_ns {
                &board.table_a
            } else {
                &board.table_b
            };
            uvu_response(uvu_table, board.dealer).map(|(call, _)| call)
        });
        if let Some(call) = response {
            let entry = by_call.entry(action_label(call)).or_default();
            entry.0 += 1;
            entry.1 += imps(swing);
        }

        if shown < args.show {
            shown += 1;
            let board = &boards[index];
            let calls: Vec<Call> = board.table_a.iter().copied().collect();
            println!(
                "[{shown}] dealer {:?}  A {calls:?} -> {contract_a:?}  vs  B -> {contract_b:?}  (swing {swing:+})",
                board.dealer,
            );
        }
    }

    println!(
        "=== UvU 1NT-(2NT) A/B: x-floor {}, cue-floor {}, natural-floor {}, opp 2NT {}-{}, {} boards, vulnerability {} ===",
        args.x_floor,
        args.cue_floor,
        args.natural_floor,
        args.opp_lo,
        args.opp_hi,
        args.count,
        args.vulnerability,
    );
    println!(
        "Divergent boards: {} of {} ({:.2}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "UvU team: {total_points:+} points, {total_imps:+} IMPs ({:+.3} IMPs/board, {:+.3} IMPs/divergent)",
        total_imps as f64 / args.count.max(1) as f64,
        total_imps as f64 / divergent.len().max(1) as f64,
    );

    println!("\nBy UvU response (IMPs each counter-measure gains vs the floor):");
    for (call, &(boards_n, imps_won)) in &by_call {
        println!(
            "  {call:<4} {boards_n:>5} boards  {imps_won:+6} IMPs  ({:+.3} IMPs/board)",
            imps_won as f64 / boards_n.max(1) as f64,
        );
    }
}
