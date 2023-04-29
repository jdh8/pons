mod contract;
mod deal;
mod dds;
mod eval;
mod test;

fn analyze_deals(n: usize) {
    let deals: Vec<deal::Deal> = (0..n).map(|_| deal::shuffled_standard_52_deck().deal()).collect();
    let solutions = dds::solve(&deals, dds::StrainFlags::all());

    for (deal, sol) in deals.iter().zip(solutions) {
        println!("{} {}", deal, sol)
    }
}

fn main() {
    match std::env::args().nth(1) {
        None => analyze_deals(100),
        Some(string) => match string.parse::<usize>() {
            Ok(n) => analyze_deals(n),
            Err(_) => todo!(),
        }
    }
}
