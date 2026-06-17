//! Bid out random boards with the basic 2/1 game-forcing system.
//!
//! Both sides play [`pons::american()`], bound against each other and seated
//! into a [`Table`].  Each turn the player to act classifies their hand against
//! the running auction and makes the highest-logit *legal* call; an auction the
//! book does not cover resolves to a pass, so the bidding always terminates.
//!
//! ```text
//! cargo run --example american -- --count 3 --dealer south --vulnerability ns
//! ```

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use pons::american;
use pons::bidding::Table;

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
    let ns = american();
    let ew = american();
    let table = Table::of_pairs(&ns, &ew, args.dealer, args.vulnerability);
    let mut rng = rand::rng();

    for index in 1..=args.count {
        let deal = full_deal(&mut rng);
        let auction = table.bid_out(&deal);

        print_board(index, &deal, args.dealer, args.vulnerability);
        print_auction(&auction, args.dealer);
        println!();
    }
}
