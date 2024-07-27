use core::fmt;
use dds_bridge::{deal, solver};
use nalgebra as na;
use pons::eval;
use std::process::ExitCode;

fn calculate_par_suit_tricks(tricks: solver::TricksTable) -> Option<(deal::Suit, deal::Seat, i8)> {
    solver::calculate_par(tricks, solver::Vulnerability::empty(), deal::Seat::North)
        .ok()?
        .contracts
        .into_iter()
        .find_map(|(contract, seat, overtricks)| {
            let suit = deal::Suit::try_from(contract.bid.strain).ok();
            #[allow(clippy::cast_possible_wrap)] // level is always in 1..=7
            suit.map(|suit| (suit, seat, contract.bid.level as i8 + 6 + overtricks))
        })
}

const EVALUATORS: [&'static dyn eval::HandEvaluator<f64>; 5] = [
    &eval::HCP_PLUS,
    &eval::BUMRAP_PLUS,
    &eval::LTC,
    &eval::NLTC,
    &eval::zar,
];

type Columns = na::Const<{ EVALUATORS.len() + 1 }>;
type Evaluation = na::OMatrix<f64, na::Dyn, Columns>;
type Correlation = na::OMatrix<f64, Columns, Columns>;
type Histogram<T> = na::OMatrix<T, na::U8, na::Const<{ EVALUATORS.len() }>>;

fn eval_random_deals(n: usize) -> Result<Evaluation, solver::Error> {
    let deals: Vec<_> = core::iter::repeat_with(|| deal::Deal::new(&mut rand::thread_rng()))
        .take(n)
        .collect();

    let rows: Vec<_> = solver::solve_deals(&deals, solver::StrainFlags::all())?
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
        rows.into_iter()
            .flat_map(|(tricks, eval)| core::iter::once(f64::from(tricks)).chain(eval)),
    ))
}

fn compute_correlation(eval: &Evaluation) -> Correlation {
    let mean = eval.row_mean();
    let centered = eval.map_with_location(|_, j, x| x - mean[j]);
    let moment = centered.adjoint() * centered;
    moment.map_with_location(|i, j, x| x / (moment[(i, i)] * moment[(j, j)]).sqrt())
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Statistics {
    mean: f64,
    sd: f64,
}

impl fmt::Display for Statistics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.mean.fmt(f)?;
        " ± ".fmt(f)?;
        self.sd.fmt(f)
    }
}

fn compute_histogram(eval: &Evaluation) -> Histogram<Statistics> {
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    struct Accumulator {
        count: f64,
        mean: f64,
        moment: f64,
    }

    let stat: Histogram<Accumulator> =
        eval.row_iter().fold(Histogram::default(), |mut stat, row| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let i = (row[0] as usize).max(6) - 6;
            let row = row.fixed_columns::<{ EVALUATORS.len() }>(1);
            stat.row_mut(i).zip_apply(&row, |acc, x| {
                acc.count += 1.0;
                let delta = x - acc.mean;
                acc.mean += delta / acc.count;
                acc.moment += delta * (x - acc.mean);
            });
            stat
        });

    stat.map(|acc| Statistics {
        mean: if acc.count <= 0.5 { f64::NAN } else { acc.mean },
        sd: (acc.moment / (acc.count - 1.0).max(0.0)).sqrt(),
    })
}

#[doc = include_str!("README.md")]
fn main() -> Result<ExitCode, solver::Error> {
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
        "Correlation matrix between `EVALUATORS`: {:.12}",
        compute_correlation(&eval),
    );
    println!(
        "Histogram of eval (mean ± sd) for tricks: {:.6}",
        compute_histogram(&eval),
    );
    Ok(ExitCode::SUCCESS)
}
