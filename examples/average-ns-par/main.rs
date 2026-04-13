use clap::Parser;
use dds_bridge::solver::{self, StrainFlags, Vulnerability};
use dds_bridge::{Deal, Hand, Seat};
use pons::{deck, stats};

/// Emulate par score for North-South by simulating random deals
#[derive(Parser)]
struct Args {
    /// North's hand in dot-separated suit notation (e.g. T9762.AT54.JT75.)
    #[arg(short, long)]
    north: Hand,

    /// South's hand in dot-separated suit notation (e.g. A.KQ962.A86.Q642)
    #[arg(short, long)]
    south: Hand,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: Vulnerability,

    /// Dealer seat: N, E, S, W (or full name)
    #[arg(short, long, default_value = "n")]
    dealer: Seat,

    /// Number of simulated deals
    ///
    /// 1. Reduced from 1000 to 90 for reasonable runtime on my i7-9700
    /// 2. Odd multiple of 10 to avoid rounding from 0.5
    #[arg(short, long, default_value = "90")]
    count: usize,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let cards = Deal::new(args.north, Hand::EMPTY, args.south, Hand::EMPTY);
    let solutions = solver::Solver::lock().solve_deals(
        &deck::fill_deals(&mut rand::rng(), cards)?
            .take(args.count)
            .collect::<Vec<_>>(),
        StrainFlags::all(),
    )?;

    let par = stats::average_ns_par(
        solutions.into_iter().collect(),
        args.vulnerability,
        args.dealer,
    )?;

    let Some(par) = par else {
        println!("No data");
        return Ok(());
    };

    match par.contract {
        Some((contract, seat)) => println!("NS par: {contract}{seat}, {:.0}", par.score),
        None => println!("NS par: P, {:.0}", par.score),
    }
    Ok(())
}
