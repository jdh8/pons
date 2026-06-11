//! Bid out random boards with the basic 2/1 game-forcing system.
//!
//! Both sides play [`pons::two_over_one`], paired into a table with
//! [`System::vs`].  Each turn the player to act classifies their hand against
//! the running auction and makes the highest-logit *legal* call; an auction the
//! book does not cover resolves to a pass, so the bidding always terminates.
//!
//! ```text
//! cargo run --example two-over-one -- --count 3 --dealer south --vulnerability ns
//! ```

use clap::Parser;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Hand, Seat};
use pons::bidding::System;
use pons::bidding::context::relative;
use pons::two_over_one;

/// Bid out random boards with the basic 2/1 game-forcing system
#[derive(Parser)]
struct Args {
    /// Dealer: north, east, south, west
    #[arg(short, long, default_value = "north")]
    dealer: Seat,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Number of boards to bid out
    #[arg(short, long, default_value = "1")]
    count: usize,
}

/// The seat acting `step` calls after the dealer
fn seat_at(dealer: Seat, step: usize) -> Seat {
    Seat::ALL[(dealer as usize + step) % 4]
}

/// The highest bid so far — bids only ascend, so the last one is the highest
fn highest_bid(auction: &[Call]) -> Option<Bid> {
    auction
        .iter()
        .filter_map(|call| match call {
            Call::Bid(bid) => Some(*bid),
            _ => None,
        })
        .next_back()
}

/// The last call that was not a pass
fn last_nonpass(auction: &[Call]) -> Option<Call> {
    auction
        .iter()
        .rev()
        .copied()
        .find(|&call| call != Call::Pass)
}

/// Whether `call` is legal as the next call of `auction`
fn is_legal(call: Call, auction: &[Call]) -> bool {
    match call {
        Call::Pass => true,
        Call::Bid(bid) => highest_bid(auction).is_none_or(|last| bid > last),
        Call::Double => matches!(last_nonpass(auction), Some(Call::Bid(_))),
        Call::Redouble => matches!(last_nonpass(auction), Some(Call::Double)),
    }
}

/// Whether the auction is over: three passes after a call, or four to start
fn auction_ended(auction: &[Call]) -> bool {
    let n = auction.len();
    if n < 4 {
        return false;
    }
    if auction.iter().any(|call| matches!(call, Call::Bid(_))) {
        auction[n - 3..].iter().all(|&call| call == Call::Pass)
    } else {
        true
    }
}

/// The highest-logit legal call the system makes, defaulting to a pass
fn next_call(
    table: &impl System,
    hand: Hand,
    vul: RelativeVulnerability,
    auction: &[Call],
) -> Call {
    let Some(logits) = table.classify(hand, vul, auction) else {
        return Call::Pass;
    };
    let mut scored: Vec<(Call, f32)> = (&logits.0)
        .into_iter()
        .map(|(call, logit)| (call, *logit))
        .filter(|(_, logit)| logit.is_finite())
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
    scored
        .into_iter()
        .map(|(call, _)| call)
        .find(|&call| is_legal(call, auction))
        .unwrap_or(Call::Pass)
}

/// Run the auction from the dealer until it ends
fn bid_out(
    table: &impl System,
    deal: &FullDeal,
    dealer: Seat,
    vul: AbsoluteVulnerability,
) -> Vec<Call> {
    let mut auction = Vec::new();
    while !auction_ended(&auction) {
        let seat = seat_at(dealer, auction.len());
        let call = next_call(table, deal[seat], relative(vul, seat), &auction);
        auction.push(call);
        // Bids ascend, so this is unreachable in practice; a guard all the same.
        if auction.len() >= 64 {
            break;
        }
    }
    auction
}

fn print_board(index: usize, deal: &FullDeal, dealer: Seat, vul: AbsoluteVulnerability) {
    println!(
        "Board {index}: dealer {}, vulnerability {vul}",
        dealer.letter(),
    );
    for seat in Seat::ALL {
        println!("  {}  {}", seat.letter(), deal[seat]);
    }
}

fn print_auction(auction: &[Call], dealer: Seat) {
    println!("  {:>6}{:>6}{:>6}{:>6}", "North", "East", "South", "West");

    // Leading blanks place the dealer's first call in the right column.
    let mut cells: Vec<String> = vec![String::new(); dealer as usize];
    cells.extend(auction.iter().map(|call| format!("{call}")));

    for row in cells.chunks(4) {
        let mut line = String::new();
        for cell in row {
            line.push_str(&format!("{cell:>6}"));
        }
        println!("  {line}");
    }
}

fn main() {
    let args = Args::parse();
    let table = two_over_one().vs(two_over_one());
    let mut rng = rand::rng();

    for index in 1..=args.count {
        let deal = full_deal(&mut rng);
        let auction = bid_out(&table, &deal, args.dealer, args.vulnerability);

        print_board(index, &deal, args.dealer, args.vulnerability);
        print_auction(&auction, args.dealer);
        println!();
    }
}
