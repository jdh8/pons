//! Is the *shipped* raw-`hcp(13..)` no-fit 2/1 floor itself sound at its own
//! margin — or does it need tightening too, the same way `hcp12` failed at
//! its margin?
//!
//! Mirrors `probe-two-over-one-hcp12-3nt` one rung up: baseline is the
//! hypothetical stricter `TwoOverOneGate::Hcp14`, candidate is the shipped
//! `Hcp13`. Opener is filtered to raw HCP 10-12 (a genuinely light, shapely
//! minimum — `points(12..)` is automatic once they actually open, since
//! that's the opening rule itself). Responder is filtered to raw HCP exactly
//! 13 — the marginal slice `hcp13` admits and a hypothetical `hcp14` would
//! not — with **no fit** (at most two cards in opener's major in *either*
//! major, so the fit leg's `support_points(13..)` cannot be what admits the
//! hand). Bids each qualifying deal twice and solves double dummy, so every
//! board shows what the shipped floor is buying at its own edge.
//!
//! ```text
//! cargo run --release --example probe-two-over-one-hcp13-3nt -- 2000000 0
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
    set_two_over_one_gate(TwoOverOneGate::Hcp14);
    let baseline_stance = american().against(Family::NATURAL);
    set_two_over_one_gate(TwoOverOneGate::Hcp13);
    let candidate_stance = american().against(Family::NATURAL);
    set_two_over_one_gate(TwoOverOneGate::Hcp13);

    let deals = seeded_deals(seed, count);
    let mut qualifying: Vec<Board> = Vec::new();
    for (index, deal) in deals.iter().enumerate() {
        let dealer = Seat::ALL[index % 4];
        let opener = dealer;
        let responder = opener.partner();

        // Cheap hand-level pre-filter before touching the bidding engine at
        // all: opener a genuinely light shapely minimum (raw HCP 10-12 —
        // `points(12..)` is automatic once they actually open, since that's
        // the opening rule itself), responder at the exact hcp13-vs-hcp14
        // margin (raw HCP exactly 13), and no fit (deny 3-card-plus support
        // in *either* major so the fit leg cannot be what admits the hand).
        if !(10..=12).contains(&hand_hcp(deal[opener])) {
            continue;
        }
        let resp: Hand = deal[responder];
        if hand_hcp(resp) != 13 || resp[Suit::Hearts].len() >= 3 || resp[Suit::Spades].len() >= 3 {
            continue;
        }

        let baseline = bid_uncontested(&baseline_stance, dealer, vul, deal);
        let Some(major) = dealer_opens_major(&baseline) else {
            continue;
        };
        let candidate = bid_uncontested(&candidate_stance, dealer, vul, deal);

        // Only the boards where the candidate floor actually admits the hand
        // into the 2/1 (baseline does not — else this isn't a "shipped
        // floor's own margin" board at all).
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
        "{count} deals scanned, {} qualifying boards (opener hcp 10-12, responder hcp==13 & \
         <=2-card support in either major, hcp13 admits / hcp14 doesn't)",
        qualifying.len()
    );

    let solve_deals: Vec<FullDeal> = qualifying.iter().map(|b| b.deal).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    let mut landed_3nt = 0usize;
    let mut made_3nt = 0usize;
    let mut baseline_contract_kinds: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut candidate_contract_kinds: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut swing_total = 0i64;
    let mut pd_swing_total = 0i64;

    for (board, table) in qualifying.iter().zip(&tables) {
        let base_result = final_contract(&board.baseline, board.dealer);
        let cand_result = final_contract(&board.candidate, board.dealer);
        let kind_of = |r: Option<(contract_bridge::Contract, Seat)>| {
            r.map_or_else(
                || "pass-out".to_owned(),
                |(c, _)| format!("{} {}", c.bid.level, c.bid.strain),
            )
        };
        *baseline_contract_kinds
            .entry(kind_of(base_result))
            .or_insert(0) += 1;
        *candidate_contract_kinds
            .entry(kind_of(cand_result))
            .or_insert(0) += 1;

        let base_score = pons::scoring::ns_score_contract(base_result, table, vul);
        let cand_score = pons::scoring::ns_score_contract(cand_result, table, vul);
        swing_total += pons::scoring::imps(cand_score - base_score);

        let base_pd = pons::scoring::ns_score_pd(base_result, table, vul);
        let cand_pd = pons::scoring::ns_score_pd(cand_result, table, vul);
        pd_swing_total += pons::scoring::imps(cand_pd - base_pd);

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
        "plain-DD swing over hcp14 baseline:      {swing_total:+} IMPs total, {:+.3} IMPs/board",
        swing_total as f64 / qualifying.len().max(1) as f64,
    );
    println!(
        "perfect-defense swing over hcp14 baseline: {pd_swing_total:+} IMPs total, {:+.3} IMPs/board",
        pd_swing_total as f64 / qualifying.len().max(1) as f64,
    );

    println!("\nwhat a stricter hcp14 gate would do instead on these same boards:");
    let mut kinds: Vec<(&String, &usize)> = baseline_contract_kinds.iter().collect();
    kinds.sort_by_key(|&(_, &n)| std::cmp::Reverse(n));
    for (kind, n) in kinds {
        println!("  {n:6}  {kind}");
    }

    println!("\nwhat the shipped hcp13 gate reaches on these same boards:");
    let mut cand_kinds: Vec<(&String, &usize)> = candidate_contract_kinds.iter().collect();
    cand_kinds.sort_by_key(|&(_, &n)| std::cmp::Reverse(n));
    for (kind, n) in cand_kinds {
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
            "opener {} (hcp {}, points {}) {}\nresponder {} (hcp {}, points {}, {}={} cards) {}\n  hcp14: {} = {}\n  hcp13: {} = {}",
            board.dealer,
            hand_hcp(board.deal[board.dealer]),
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
