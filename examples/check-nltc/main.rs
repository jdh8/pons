use dds_bridge as dds;
use nalgebra as na;
use pons::eval::{self, HandEvaluator as _};
use std::process::ExitCode;

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
type Coefficients = na::OMatrix<f64, na::U2, na::Const<{ EVALUATORS.len() }>>;

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
    let centered = eval.map_with_location(|_, j, x| x - mean[j]);

    #[allow(clippy::cast_precision_loss)]
    let covariance = centered.adjoint() * centered / (eval.nrows() - 1) as f64;

    Correlation::from_fn(|i, j| {
        covariance[(i, j)] / (covariance[(i, i)] * covariance[(j, j)]).sqrt()
    })
}

fn compute_linear_regression(eval: &Evaluation) -> Coefficients {
    let tricks = eval.column(0);
    let columns: Vec<_> = eval
        .fixed_columns::<{ EVALUATORS.len() }>(1)
        .column_iter()
        .map(|col| {
            let matrix = na::OMatrix::<f64, na::Dyn, na::U2>::from_columns(&[
                col.into(),
                na::DVector::from_element(eval.nrows(), 1.0),
            ]);
            let (q, r) = matrix.qr().unpack();
            let q = q.fixed_columns::<2>(0);
            let r = r.fixed_rows::<2>(0);
            r.solve_upper_triangular(&(q.transpose() * tricks))
                .expect("Same evaluation for all deals")
        })
        .collect();
    Coefficients::from_columns(&columns)
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
        "Linear regression coefficients: {}",
        compute_linear_regression(&eval),
    );
    Ok(ExitCode::SUCCESS)
}
