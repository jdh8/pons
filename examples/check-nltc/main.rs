use dds_bridge::{contract, deal, solver};
use std::process::ExitCode;

fn calculate_par_suit_tricks(tricks: solver::TricksTable) -> Option<(deal::Suit, deal::Seat, i8)> {
    solver::calculate_par(tricks, solver::Vulnerability::empty(), deal::Seat::North)
        .ok()?
        .contracts
        .into_iter()
        .find_map(|(contract, seat, overtricks)| {
            #[allow(clippy::cast_possible_wrap)] // level is always in 1..=7
            match contract.bid.strain {
                contract::Strain::Notrump => None,
                contract::Strain::Spades => Some(deal::Suit::Spades),
                contract::Strain::Hearts => Some(deal::Suit::Hearts),
                contract::Strain::Diamonds => Some(deal::Suit::Diamonds),
                contract::Strain::Clubs => Some(deal::Suit::Clubs),
            }
            .map(|suit| (suit, seat, contract.bid.level as i8 + 6 + overtricks))
        })
}

fn analyze_deals(n: usize) -> Result<(), solver::Error> {
    let deals: Vec<_> = core::iter::repeat_with(|| deal::Deal::new(&mut rand::thread_rng()))
        .take(n)
        .collect();
    let tables: Vec<_> = solver::solve_deals(&deals, solver::StrainFlags::NOTRUMP)?
        .into_iter()
        .map(calculate_par_suit_tricks)
        .collect();
    todo!()
}

#[doc = include_str!("README.md")]
fn main() -> Result<ExitCode, solver::Error> {
    match std::env::args().nth(1) {
        Some(string) => {
            if let Ok(n) = string.parse::<usize>() {
                analyze_deals(n)
            } else {
                eprintln!("{}", include_str!("README.md"));
                return Ok(ExitCode::FAILURE);
            }
        }
        None => analyze_deals(100),
    }?;
    Ok(ExitCode::SUCCESS)
}
