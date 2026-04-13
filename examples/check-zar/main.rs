use dds_bridge::{Suit, deal, solver};
use pons::deck;
use pons::eval::{self, HandEvaluator as _};
use pons::stats::{Accumulator, Statistics};
use std::process::ExitCode;

fn calculate_par_suit_tricks(tricks: solver::TricksTable) -> Option<(Suit, deal::Seat, i8)> {
    solver::calculate_par(tricks, solver::Vulnerability::empty(), deal::Seat::North)
        .ok()?
        .contracts
        .into_iter()
        .find_map(|pc| {
            let suit = Suit::try_from(pc.contract.bid.strain).ok();
            #[allow(clippy::cast_possible_wrap)] // level is always in 1..=7
            suit.map(|suit| {
                (
                    suit,
                    pc.declarer,
                    pc.contract.bid.level.get() as i8 + 6 + pc.overtricks,
                )
            })
        })
}

fn eval_random_deals(n: usize) -> Result<[Statistics; 64], solver::SystemError> {
    let deals: Vec<_> = core::iter::repeat_with(|| deck::full_deal(&mut rand::rng()))
        .take(n)
        .collect();

    Ok(solver::Solver::lock()
        .solve_deals(&deals, solver::StrainFlags::all())?
        .into_iter()
        .map(calculate_par_suit_tricks)
        .enumerate()
        .filter_map(|(i, x)| {
            x.map(|(_, seat, tricks)| {
                let hands = [deals[i][seat], deals[i][seat.partner()]];
                (eval::zar::<u8>.eval_pair(hands), tricks)
            })
        })
        .fold([Accumulator::default(); 64], |mut acc, (eval, tricks)| {
            acc[(eval - 16).min(64) as usize].push(tricks.into());
            acc
        })
        .map(Accumulator::sample))
}

fn main() -> Result<ExitCode, solver::SystemError> {
    let n = match std::env::args().nth(1) {
        Some(string) => {
            if let Ok(n) = string.parse::<usize>() {
                n
            } else {
                //eprintln!("{}", include_str!("README.md"));
                return Ok(ExitCode::FAILURE);
            }
        }
        None => 100,
    };

    for (i, stat) in eval_random_deals(n)?.into_iter().enumerate() {
        println!("{}: {stat}", i + 16);
    }

    Ok(ExitCode::SUCCESS)
}
