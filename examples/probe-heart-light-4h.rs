//! Does the `1♠ - 2♥` heart-light gate (`two_over_one_heart_light`) route the
//! flat-twelve, five-heart hands it admits into a **`4♥` on the 5-3 fit** rather
//! than a thin 3NT — the strain-location bet behind lightening the gate on the
//! ensured five-card major?
//!
//! Isolates the pure *entry adds*: responder holds exactly 12 HCP, 5+ hearts and
//! at most two spades (no spade fit — the fit leg cannot be what admits the
//! hand), opener opens `1♠`, and the candidate (heart-light on) enters `2♥`
//! while the shipped baseline does **not** (it holds the flat twelve at a forcing
//! 1NT, since `points == hcp == 12` carries no upgrade).  That excludes the
//! reading-only slam cascades a bank A/B also shows — both books bid `2♥`, only
//! the box floor differs — so every board here is a hand the gate genuinely
//! moves.  Splits the landing by opener's heart length: **3+ = the 5-3 fit**
//! (expect `4♥`), **≤2 = no fit** (expect 3NT), and prints opener's rebid so the
//! "2♠ relay vs 2NT" continuation is visible.
//!
//! ```text
//! cargo run --release --example probe-heart-light-4h -- 4000000 0
//! ```
//! Args (positional, optional): deal `count` (default 4,000,000), `seed` (default 0).

use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Bid, Contract, FullDeal, Hand, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::agreements::Agreements;
use pons::bidding::constraint::point_count;
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_uncontested, hand_hcp, seeded_deals};

/// Responder's first call is a two-level new suit — the 2/1 entry, whichever
/// gate admitted it — rather than the forcing 1NT catch-all
fn enters_two_over_one(auction: &Auction) -> bool {
    matches!(
        auction.get(2).copied(),
        Some(Call::Bid(Bid { level, .. })) if level.get() == 2
    )
}

/// Opener's rebid as a display string — index 4 (`1♠ - 2♥ - ?`): the
/// uncontested auction carries the opponents' passes, so opener's second call
/// sits four slots in, not three.
fn opener_rebid(auction: &Auction) -> String {
    auction
        .get(4)
        .map_or_else(|| "(none)".to_owned(), ToString::to_string)
}

struct Board {
    deal: FullDeal,
    dealer: Seat,
    opener_hearts: usize,
    baseline: Auction,
    candidate: Auction,
}

/// A landing tally for one opener-heart-length split
#[derive(Default)]
struct Split {
    boards: usize,
    hearts_game: usize, // candidate lands in 4♥+ (the fit target)
    notrump: usize,     // candidate lands in some notrump
    made: usize,        // candidate contract makes double dummy
    plain: i64,
    pd: i64,
    rebids: std::collections::HashMap<String, usize>,
    contracts: std::collections::HashMap<String, usize>,
}

fn main() {
    let mut argv = std::env::args().skip(1);
    let count: usize = argv
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4_000_000);
    let seed: u64 = argv.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let vul = AbsoluteVulnerability::NONE;

    // Two books, built once — the knob is read at construction, not classify time.
    let baseline = american(&Agreements::default()).bind();
    let mut light = Agreements::default();
    light.response.two_over_one_heart_light = true;
    let candidate = american(&light).bind();

    let deals = seeded_deals(seed, count);
    let mut qualifying: Vec<Board> = Vec::new();
    for (index, deal) in deals.iter().enumerate() {
        let dealer = Seat::ALL[index % 4];
        let responder = dealer.partner();
        let resp: Hand = deal[responder];

        // The pure heart-light add: flat twelve, five+ hearts, at most two spades
        // (no fit).  points==12 (== hcp here) is what the shipped points13 gate
        // misses, so these are exactly the boards the candidate gate moves.
        if hand_hcp(resp) != 12
            || point_count(resp) != 12
            || resp[Suit::Hearts].len() < 5
            || resp[Suit::Spades].len() >= 3
        {
            continue;
        }

        let base = bid_uncontested(&baseline, dealer, vul, deal);
        // Opener must actually open 1♠ for 2♥ to be the five-card-major 2/1.
        if !matches!(base.first().copied(), Some(Call::Bid(b)) if b == Bid::new(1, Strain::Spades))
        {
            continue;
        }
        let cand = bid_uncontested(&candidate, dealer, vul, deal);

        // Only the entry adds: candidate enters the 2/1, baseline stays in 1NT.
        if enters_two_over_one(&cand) && !enters_two_over_one(&base) {
            qualifying.push(Board {
                deal: *deal,
                dealer,
                opener_hearts: deal[dealer][Suit::Hearts].len(),
                baseline: base,
                candidate: cand,
            });
        }
    }

    println!(
        "{count} deals scanned, {} qualifying entry-adds (opener 1♠, responder hcp==points==12 & \
         5+ hearts & <=2 spades; heart-light enters 2♥, shipped stays 1NT)",
        qualifying.len()
    );

    let solve_deals: Vec<FullDeal> = qualifying.iter().map(|b| b.deal).collect();
    let tables = Solver::lock(None).solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    // Split by opener's heart length: 3+ = the 5-3 fit, ≤2 = no fit.
    let mut fit = Split::default();
    let mut nofit = Split::default();
    for (board, table) in qualifying.iter().zip(&tables) {
        let split = if board.opener_hearts >= 3 {
            &mut fit
        } else {
            &mut nofit
        };
        split.boards += 1;
        *split
            .rebids
            .entry(opener_rebid(&board.candidate))
            .or_insert(0) += 1;

        let base_result = final_contract(&board.baseline, board.dealer);
        let cand_result = final_contract(&board.candidate, board.dealer);
        *split
            .contracts
            .entry(cand_result.map_or_else(
                || "pass-out".to_owned(),
                |(c, _)| format!("{} {}", c.bid.level, c.bid.strain),
            ))
            .or_insert(0) += 1;

        split.plain += imps(
            ns_score_contract(cand_result, table, vul) - ns_score_contract(base_result, table, vul),
        );
        split.pd +=
            imps(ns_score_pd(cand_result, table, vul) - ns_score_pd(base_result, table, vul));

        if let Some((contract, declarer)) = cand_result {
            let strain = contract.bid.strain;
            if matches!(strain, Strain::Hearts) && contract.bid.level.get() >= 4 {
                split.hearts_game += 1;
            }
            if strain == Strain::Notrump {
                split.notrump += 1;
            }
            let tricks = u8::from(table[strain].get(declarer));
            if tricks >= 6 + contract.bid.level.get() {
                split.made += 1;
            }
        }
    }

    let report = |name: &str, s: &Split| {
        println!(
            "\n=== opener {name} ({} boards) ===\n  candidate lands in 4♥+: {} ({:.0}%) | notrump: {} ({:.0}%) | makes DD: {} ({:.0}%)\n  plain-DD swing {:+} IMPs ({:+.3}/add) | perfect-defense {:+} IMPs ({:+.3}/add)",
            s.boards,
            s.hearts_game,
            100.0 * s.hearts_game as f64 / s.boards.max(1) as f64,
            s.notrump,
            100.0 * s.notrump as f64 / s.boards.max(1) as f64,
            s.made,
            100.0 * s.made as f64 / s.boards.max(1) as f64,
            s.plain,
            s.plain as f64 / s.boards.max(1) as f64,
            s.pd,
            s.pd as f64 / s.boards.max(1) as f64,
        );
        let mut rebids: Vec<(&String, &usize)> = s.rebids.iter().collect();
        rebids.sort_by_key(|&(_, &n)| std::cmp::Reverse(n));
        print!("  opener's rebid after 1♠ - 2♥ - ?:");
        for (call, n) in rebids {
            print!("  {call}×{n}");
        }
        println!();
        let mut contracts: Vec<(&String, &usize)> = s.contracts.iter().collect();
        contracts.sort_by_key(|&(_, &n)| std::cmp::Reverse(n));
        print!("  candidate contract:");
        for (c, n) in contracts.iter().take(8) {
            print!("  {c}×{n}");
        }
        println!();
    };
    report("holds 3+ hearts — the 5-3 fit", &fit);
    report("holds <=2 hearts — no fit", &nofit);

    println!("\nsample fit boards (opener 3+ hearts):");
    for (board, table) in qualifying
        .iter()
        .zip(&tables)
        .filter(|(b, _)| b.opener_hearts >= 3)
        .take(12)
    {
        let cand_result = final_contract(&board.candidate, board.dealer);
        let reached = |r: Option<(Contract, Seat)>| {
            r.map_or_else(
                || "pass-out".to_owned(),
                |(c, s)| {
                    format!(
                        "{c} by {s} ({} tricks DD)",
                        u8::from(table[c.bid.strain].get(s))
                    )
                },
            )
        };
        let resp = board.dealer.partner();
        println!(
            "opener {} ({}=♥, {} hcp) {}\nresponder {} ({}=♥ {}=♠, {} hcp) {}\n  1NT: {} = {}\n   2♥: {} = {}",
            board.dealer,
            board.opener_hearts,
            hand_hcp(board.deal[board.dealer]),
            board.deal[board.dealer],
            resp,
            board.deal[resp][Suit::Hearts].len(),
            board.deal[resp][Suit::Spades].len(),
            hand_hcp(board.deal[resp]),
            board.deal[resp],
            board.baseline,
            reached(final_contract(&board.baseline, board.dealer)),
            board.candidate,
            reached(cand_result),
        );
    }
}
