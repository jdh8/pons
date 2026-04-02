use clap::Parser;
use dds_bridge::deal::{Hand, Seat};
use dds_bridge::solver::Vulnerability;
use pons::solver;

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
    #[arg(short, long, default_value = "1000")]
    count: usize,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let (score, contract) = solver::emulate_par(
        args.north,
        args.south,
        args.vulnerability,
        args.dealer,
        args.count,
    )?;

    match contract {
        Some((contract, seat)) => {
            println!(
                "NS par: {}{}{}{}, {score:.0}",
                contract.bid.level,
                contract.bid.strain.unicode(),
                contract.penalty,
                char::from(seat)
            );
        }
        None => println!("NS par: P, {score:.0}"),
    }
    Ok(())
}
