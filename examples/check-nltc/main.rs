use dds_bridge::{Seat, Suit, solver};
use nalgebra as na;
use pons::deck;
use pons::eval;
use pons::{Accumulator, Statistics};
use std::process::ExitCode;

fn calculate_par_suit_tricks(tricks: solver::TrickCountTable) -> Option<(Suit, Seat, i8)> {
    solver::calculate_par(tricks, solver::Vulnerability::empty(), Seat::North)
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

const EVALUATORS: [&'static dyn eval::HandEvaluator<f64>; 5] = [
    &eval::SimpleEvaluator(eval::hcp_plus::<f64>),
    &eval::BUMRAP_PLUS,
    &eval::SimpleEvaluator(eval::ltc::<f64>),
    &eval::NLTC,
    &eval::zar,
];

type Columns = na::Const<{ EVALUATORS.len() + 1 }>;
type Evaluation = na::OMatrix<f64, na::Dyn, Columns>;
type Correlation = na::OMatrix<f64, Columns, Columns>;
type Histogram<T> = na::OMatrix<T, na::U8, na::Const<{ EVALUATORS.len() }>>;

fn eval_random_deals(n: usize) -> Evaluation {
    let deals: Vec<_> = core::iter::repeat_with(|| deck::full_deal(&mut rand::rng()))
        .take(n)
        .collect();

    let rows: Vec<_> = solver::Solver::lock()
        .solve_deals(&deals, solver::StrainFlags::all())
        .into_iter()
        .map(calculate_par_suit_tricks)
        .enumerate()
        .filter_map(|(i, x)| {
            x.map(|(_, seat, tricks)| {
                let hands = [deals[i][seat], deals[i][seat.partner()]];
                (tricks, EVALUATORS.map(|f| f.eval_pair(hands)))
            })
        })
        .collect();

    Evaluation::from_row_iterator(
        rows.len(),
        rows.into_iter()
            .flat_map(|(tricks, eval)| core::iter::once(f64::from(tricks)).chain(eval)),
    )
}

fn compute_correlation(eval: &Evaluation) -> Correlation {
    let mean = eval.row_mean();
    let centered = eval.map_with_location(|_, j, x| x - mean[j]);
    let moment = centered.adjoint() * centered;
    moment.map_with_location(|i, j, x| x / (moment[(i, i)] * moment[(j, j)]).sqrt())
}

fn compute_histogram(eval: &Evaluation) -> Histogram<Statistics> {
    eval.row_iter()
        .fold(Histogram::default(), |mut acc, row| {
            // SAFETY: `row[0]` is tricks since the beginning.  It is stored as
            // `f64` for `nalgebra`, but it is always an integer in 0..=13, so
            // the cast is safe.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let i = (row[0] as usize).max(6) - 6;
            let row = row.fixed_columns::<{ EVALUATORS.len() }>(1);
            acc.row_mut(i).zip_apply(&row, Accumulator::push);
            acc
        })
        .map(Accumulator::sample)
}

#[doc = include_str!("README.md")]
fn main() -> ExitCode {
    let n = match std::env::args().nth(1) {
        Some(string) => {
            if let Ok(n) = string.parse::<usize>() {
                n
            } else {
                eprintln!("{}", include_str!("README.md"));
                return ExitCode::FAILURE;
            }
        }
        None => 100,
    };
    let eval = eval_random_deals(n);
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
    ExitCode::SUCCESS
}
