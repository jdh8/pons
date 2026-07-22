//! Does lightening the major no-fit 2/1 floor to `points(12..)` (Rule of 20,
//! `TwoOverOneGate::Points12`) reach *good* 3NTs when both sides are minimum —
//! or does it force game on hands that belong in a non-forcing 1NT?
//!
//! No self-play A/B here: filters to the exact marginal slice — opener's own
//! minimum (12-13 points), responder's own gate minimum (raw `hcp < 13` so
//! the shipped `hcp13` floor would *not* admit the hand, `points` in 12..=13
//! so the new floor barely does), and **no fit** (responder holds at most two
//! cards in opener's major — the fit leg's `support_points(13..)` must play no
//! part). Bids each qualifying deal twice (the shipped `hcp13` gate and the
//! candidate `points12` gate) and solves double dummy, so every board shows
//! what the new floor changed and whether the result made.
//!
//! ```text
//! cargo run --release --example probe-two-over-one-nofit-3nt -- 2000000 0
//! ```
//! Args (positional, optional): deal `count` (default 2,000,000), `seed` (default 0).

use contract_bridge::auction::Auction;
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Hand, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Family;
use pons::bidding::american::{TwoOverOneGate, set_two_over_one_gate};
use pons::bidding::constraint::point_count;
use pons::scoring::final_contract;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_uncontested, hand_hcp, seeded_deals};

/// The major opener shows at first call: `Some(suit)` if `auction[0]` is a
/// five-card 1♥/1♠ opening by the dealer, else `None`
fn dealer_opens_major(auction: &Auction) -> Option<Suit> {
    match auction.first().copied()? {
        contract_bridge::auction::Call::Bid(Bid {
            level,
            strain: Strain::Hearts,
        }) if level.get() == 1 => Some(Suit::Hearts),
        contract_bridge::auction::Call::Bid(Bid {
            level,
            strain: Strain::Spades,
        }) if level.get() == 1 => Some(Suit::Spades),
        _ => None,
    }
}

/// Responder's first call is a new suit at the two level — the 2/1 entry,
/// whichever gate admitted it — rather than the forcing 1NT catch-all
fn responder_enters_two_over_one(auction: &Auction) -> bool {
    matches!(
        auction.get(2).copied(),
        Some(contract_bridge::auction::Call::Bid(Bid { level, .. })) if level.get() == 2
    )
}

struct Board {
    deal: FullDeal,
    dealer: Seat,
    major: Suit,
    baseline: Auction,
    candidate: Auction,
}

fn main() {
    let mut argv = std::env::args().skip(1);
    let count: usize = argv
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2_000_000);
    let seed: u64 = argv.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let vul = AbsoluteVulnerability::NONE;

    // Two books, built once — `set_two_over_one_gate` is read at book
    // construction, never at classify time.
    set_two_over_one_gate(TwoOverOneGate::Hcp13);
    let baseline_stance = american().against(Family::NATURAL);
    set_two_over_one_gate(TwoOverOneGate::Points12);
    let candidate_stance = american().against(Family::NATURAL);
    set_two_over_one_gate(TwoOverOneGate::Hcp13);

    let deals = seeded_deals(seed, count);
    let mut qualifying: Vec<Board> = Vec::new();
    for (index, deal) in deals.iter().enumerate() {
        let dealer = Seat::ALL[index % 4];
        let opener = dealer;
        let responder = opener.partner();

        // Cheap hand-level pre-filter before touching the bidding engine at
        // all: opener's own minimum (12-13 points), responder's exact
        // marginal slice (raw HCP short of the shipped hcp13 floor, points
        // barely clearing the candidate points12 floor), and no fit (deny
        // 3-card-plus support in *either* major so the fit leg's
        // `support_points(13..)` cannot be what admits the hand).
        if !(12..=13).contains(&point_count(deal[opener])) {
            continue;
        }
        let resp: Hand = deal[responder];
        if hand_hcp(resp) >= 13
            || !(12..=13).contains(&point_count(resp))
            || resp[Suit::Hearts].len() >= 3
            || resp[Suit::Spades].len() >= 3
        {
            continue;
        }

        let baseline = bid_uncontested(&baseline_stance, dealer, vul, deal);
        let Some(major) = dealer_opens_major(&baseline) else {
            continue;
        };
        let candidate = bid_uncontested(&candidate_stance, dealer, vul, deal);

        // Only the boards where the candidate floor actually admits the hand
        // into the 2/1 (baseline does not — else this isn't a "my gate"
        // board at all).
        if responder_enters_two_over_one(&candidate) && !responder_enters_two_over_one(&baseline) {
            qualifying.push(Board {
                deal: *deal,
                dealer,
                major,
                baseline,
                candidate,
            });
        }
    }

    println!(
        "{count} deals scanned, {} qualifying boards (opener 12-13 points, responder hcp<13 & \
         points 12-13 & <=2-card support in opener's major, points12 admits / hcp13 doesn't)",
        qualifying.len()
    );

    let solve_deals: Vec<FullDeal> = qualifying.iter().map(|b| b.deal).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    let mut landed_3nt = 0usize;
    let mut made_3nt = 0usize;
    let mut baseline_contract_kinds: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut swing_total = 0i64;

    for (board, table) in qualifying.iter().zip(&tables) {
        let base_result = final_contract(&board.baseline, board.dealer);
        let cand_result = final_contract(&board.candidate, board.dealer);
        *baseline_contract_kinds
            .entry(base_result.map_or_else(
                || "pass-out".to_owned(),
                |(c, _)| format!("{} {}", c.bid.level, c.bid.strain),
            ))
            .or_insert(0) += 1;

        let base_score = pons::scoring::ns_score_contract(base_result, table, vul);
        let cand_score = pons::scoring::ns_score_contract(cand_result, table, vul);
        swing_total += pons::scoring::imps(cand_score - base_score);

        if let Some((contract, declarer)) = cand_result
            && contract.bid.strain == Strain::Notrump
        {
            landed_3nt += 1;
            let tricks = u8::from(table[Strain::Notrump].get(declarer));
            if tricks >= 6 + contract.bid.level.get() {
                made_3nt += 1;
            }
        }
    }

    println!(
        "\ncandidate lands in notrump on {landed_3nt}/{} qualifying boards ({} made, {:.1}%)",
        qualifying.len(),
        made_3nt,
        100.0 * made_3nt as f64 / landed_3nt.max(1) as f64,
    );
    println!(
        "plain-DD swing over baseline: {swing_total:+} IMPs total, {:+.3} IMPs/board",
        swing_total as f64 / qualifying.len().max(1) as f64,
    );

    println!("\nwhat the shipped hcp13 gate does instead on these same boards:");
    let mut kinds: Vec<(&String, &usize)> = baseline_contract_kinds.iter().collect();
    kinds.sort_by_key(|&(_, &n)| std::cmp::Reverse(n));
    for (kind, n) in kinds {
        println!("  {n:6}  {kind}");
    }

    println!("\nsample boards:");
    for (board, table) in qualifying.iter().zip(&tables).take(15) {
        let base_result = final_contract(&board.baseline, board.dealer);
        let cand_result = final_contract(&board.candidate, board.dealer);
        let reached = |r: Option<(contract_bridge::Contract, Seat)>| {
            r.map_or_else(
                || "pass-out".to_owned(),
                |(c, s)| {
                    let tricks = u8::from(table[c.bid.strain].get(s));
                    format!("{c} by {s} ({tricks} tricks DD)")
                },
            )
        };
        let resp = board.dealer.partner();
        println!(
            "opener {} ({} pts) {}\nresponder {} (hcp {}, points {}, {}={} cards) {}\n  hcp13:    {} = {}\n  points12: {} = {}",
            board.dealer,
            point_count(board.deal[board.dealer]),
            board.deal[board.dealer],
            resp,
            hand_hcp(board.deal[resp]),
            point_count(board.deal[resp]),
            board.major,
            board.deal[resp][board.major].len(),
            board.deal[resp],
            board.baseline,
            reached(base_result),
            board.candidate,
            reached(cand_result),
        );
    }
}
