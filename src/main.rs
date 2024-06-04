use dds_bridge::contract::Strain;
use dds_bridge::deal::{Deal, Seat};
use dds_bridge::solver;

#[derive(Clone, Copy, Debug, Default)]
struct Accumulator {
    ns: usize,
    ew: usize,
    total: usize,
}

fn analyze_deals(n: usize) -> Result<(), solver::Error> {
    let deals: Vec<_> = core::iter::repeat_with(|| Deal::new(&mut rand::thread_rng()))
        .take(n)
        .collect();

    let solution = solver::solve_deals(&deals, solver::StrainFlags::NOTRUMP)?
        .into_iter()
        .map(|table| table[Strain::Notrump])
        .fold(Accumulator::default(), |mut acc, sol| {
            let (n, e, s, w) = (
                sol.at(Seat::North),
                sol.at(Seat::East),
                sol.at(Seat::South),
                sol.at(Seat::West),
            );
            acc.ns += usize::from(n.max(s));
            acc.ew += usize::from(e.max(w));
            acc.total += usize::from(n + e + s + w);
            acc
        });

    dbg!(solution);
    Ok(())
}

fn main() -> Result<(), solver::Error> {
    std::env::args().nth(1).map_or_else(
        || analyze_deals(100),
        |string| string.parse::<usize>().map_or_else(|_| todo!(), analyze_deals))
}
