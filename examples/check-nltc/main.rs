extern crate dds_bridge as dds;
extern crate nalgebra as na;
use pons::eval::{self, HandEvaluator};
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

type SimpleEvaluator<T> = eval::SimpleEvaluator<T, fn(dds::Holding) -> T>;

const EVALUATORS: [SimpleEvaluator<i32>; 4] = [
    eval::HCP_PLUS,
    eval::CENTI_BUMRAP_PLUS,
    eval::LTC,
    eval::HALF_NLTC,
];

const COLUMNS: usize = EVALUATORS.len() + 1;
type Evaluation = na::OMatrix<f64, na::Dyn, na::Const<COLUMNS>>;
type Correlation = na::OMatrix<f64, na::Const<COLUMNS>, na::Const<COLUMNS>>;

fn eval_random_deals(n: usize) -> Result<Evaluation, dds::Error> {
    let deals: Vec<_> = core::iter::repeat_with(|| dds::Deal::new(&mut rand::thread_rng()))
        .take(n)
        .collect();

    let rows: Vec<_> = dds::solve_deals(&deals, !dds::StrainFlags::NOTRUMP)?
        .into_iter()
        .map(calculate_par_suit_tricks)
        .enumerate()
        .filter_map(|(i, x)| {
            x.map(|(_, seat, tricks)| {
                let hands = [deals[i][seat], deals[i][seat + core::num::Wrapping(2)]];
                (tricks, EVALUATORS.map(|f| f.eval_pair(hands)))
            })
        })
        .collect();

    Ok(Evaluation::from_row_iterator(
        rows.len(),
        rows.into_iter().flat_map(|(tricks, eval)| {
            core::iter::once(f64::from(tricks)).chain(eval.into_iter().map(f64::from))
        }),
    ))
}

fn compute_correlation(eval: &Evaluation) -> Correlation {
    let mean = eval.row_mean();
    let centered: Vec<_> = eval.row_iter().map(|row| row - mean).collect();
    let centered = Evaluation::from_rows(&centered);

    #[allow(clippy::cast_precision_loss)]
    let covariance = centered.adjoint() * &centered / (centered.nrows() - 1) as f64;

    Correlation::from_fn(|i, j| {
        covariance[(i, j)] / (covariance[(i, i)] * covariance[(j, j)]).sqrt()
    })
}

#[doc = include_str!("README.md")]
fn main() -> Result<ExitCode, dds::Error> {
    let n = match std::env::args().nth(1) {
        Some(string) => {
            if let Ok(n) = string.parse::<usize>() {
                n
            } else {
                eprintln!("{}", include_str!("README.md"));
                return Ok(ExitCode::FAILURE);
            }
        }
        None => 100,
    };
    let eval = eval_random_deals(n)?;

    println!(
        "Correlation matrix between `EVALUATORS`: {}",
        compute_correlation(&eval),
    );
    Ok(ExitCode::SUCCESS)
}
