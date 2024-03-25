mod contract;
mod deal;
mod dds;
mod eval;
mod test;

#[derive(Clone, Copy, Debug, Default)]
struct Accumulator {
    ns: usize,
    ew: usize,
    total: usize,
}

fn analyze_deals(n: usize) {
    let deals: Vec<_> = (0..n).map(|_| deal::shuffled_standard_52_deck().deal()).collect();
    let solution = dds::solve(&deals, dds::StrainFlags::NOTRUMP)
        .into_iter()
        .map(|table| table[contract::Strain::Notrump])
        .fold(Accumulator::default(), |mut acc, sol| {
            let (n, e, s, w) = (
                sol.at(deal::Seat::North),
                sol.at(deal::Seat::East),
                sol.at(deal::Seat::South),
                sol.at(deal::Seat::West),
            );
            acc.ns += usize::from(n.max(s));
            acc.ew += usize::from(e.max(w));
            acc.total += usize::from(n + e + s + w);
            acc
        });

    dbg!(solution);
}

fn main() {
    std::env::args().nth(1).map_or_else(
        || analyze_deals(100),
        |string| string.parse::<usize>().map_or_else(|_| todo!(), analyze_deals));
}
