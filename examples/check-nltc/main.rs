extern crate dds_bridge as dds;
use std::process::ExitCode;

fn calculate_par_suit_tricks(tricks: dds::TricksTable) -> Option<(dds::Suit, dds::Seat, i8)> {
    dds::calculate_par(tricks, dds::Vulnerability::empty(), dds::Seat::North)
        .ok()?
        .contracts
        .into_iter()
        .find_map(|(contract, seat, overtricks)| {
            #[allow(clippy::cast_possible_wrap)] // level is always in 1..=7
            match contract.bid.strain {
                dds::Strain::Notrump => None,
                dds::Strain::Spades => Some(dds::Suit::Spades),
                dds::Strain::Hearts => Some(dds::Suit::Hearts),
                dds::Strain::Diamonds => Some(dds::Suit::Diamonds),
                dds::Strain::Clubs => Some(dds::Suit::Clubs),
            }
            .map(|suit| (suit, seat, contract.bid.level as i8 + 6 + overtricks))
        })
}

fn analyze_deals(n: usize) -> Result<(), dds::Error> {
    let deals: Vec<_> = core::iter::repeat_with(|| dds::Deal::new(&mut rand::thread_rng()))
        .take(n)
        .collect();
    let tables: Vec<_> = dds::solve_deals(&deals, dds::StrainFlags::NOTRUMP)?
        .into_iter()
        .map(calculate_par_suit_tricks)
        .collect();
    todo!()
}

#[doc = include_str!("README.md")]
fn main() -> Result<ExitCode, dds::Error> {
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
