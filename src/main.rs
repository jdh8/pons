mod deal;
mod dds;

fn main() {
    let deals: Vec<deal::Deal> = (0..100).map(|_| deal::shuffled_standard_52().deal()).collect();
    let solutions = dds::solve(&deals, dds::StrainFlags::all());
    println!("{}\n\n{:?}", solutions.len(), solutions);
}
