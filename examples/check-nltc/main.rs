use dds_bridge as dds;
use nalgebra as na;
use pons::eval::{self, HandEvaluator as _};
use std::process::ExitCode;
use core::ops::AddAssign as _;

fn calculate_par_suit_tricks(tricks: dds::TricksTable) -> Option<(dds::Suit, dds::Seat, i8)> {
    dds::calculate_par(tricks, dds::Vulnerability::empty(), dds::Seat::North)
        .ok()?
        .contracts
        .into_iter()
        .find_map(|(contract, seat, overtricks)| {
            let suit = dds::Suit::try_from(contract.bid.strain).ok();
            #[allow(clippy::cast_possible_wrap)] // level is always in 1..=7
            suit.map(|suit| (suit, seat, contract.bid.level as i8 + 6 + overtricks))
        })
}

type SimpleEvaluator<T> = eval::SimpleEvaluator<T, fn(dds::Holding) -> T>;

const EVALUATORS: [SimpleEvaluator<i32>; 4] = [
    eval::HCP_PLUS,
    eval::CENTI_BUMRAP_PLUS,
    eval::LTC,
    eval::HALF_NLTC,
];

type Columns = na::Const<{ EVALUATORS.len() + 1 }>;
type Evaluation = na::OMatrix<f64, na::Dyn, Columns>;
type Correlation = na::OMatrix<f64, Columns, Columns>;
type Histogram = na::OMatrix<f64, na::U8, na::Const<{ EVALUATORS.len() }>>;

fn eval_random_deals(n: usize) -> Result<Evaluation, dds::Error> {
    let deals: Vec<_> = core::iter::repeat_with(|| dds::Deal::new(&mut rand::thread_rng()))
        .take(n)
        .collect();

    let rows: Vec<_> = dds::solve_deals(&deals, dds::StrainFlags::all())?
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
    let centered = eval.map_with_location(|_, j, x| x - mean[j]);
    let moment = centered.adjoint() * centered;
    moment.map_with_location(|i, j, x| x / (moment[(i, i)] * moment[(j, j)]).sqrt())
}

fn compute_mean_historgram(eval: &Evaluation) -> Histogram {
    let mut sum = Histogram::zeros();
    let mut count = Histogram::zeros();

    for row in eval.row_iter() {
        let i = (row[0] as usize).max(6) - 6;
        sum.row_mut(i).add_assign(row.fixed_columns::<{ EVALUATORS.len() }>(1));
        count.row_mut(i).add_scalar_mut(1.0);
    }
    sum.component_div(&count)
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
    let tricks = eval.column(0);
    let mean = tricks.mean();

    #[allow(clippy::cast_precision_loss)]
    let variance = tricks
        .iter()
        .map(|&x| {
            let x = x - mean;
            x * x
        })
        .sum::<f64>()
        / (tricks.len() - 1) as f64;

    println!("The number of valid deals: {}", tricks.len());
    println!("Average tricks of the best suit contract: {mean}");
    println!("Standard deviation of the tricks: {}\n", variance.sqrt());
    println!(
        "Correlation matrix between `EVALUATORS`: {}",
        compute_correlation(&eval),
    );
    println!(
        "Histogram of mean eval for tricks: {}",
        compute_mean_historgram(&eval),
    );
    Ok(ExitCode::SUCCESS)
}
