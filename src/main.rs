mod deal;
mod dds;

fn main() {
    let deals : [deal::Deal; 0] = [];
    dds::solve(&deals, dds::StrainFlags::all());
}
