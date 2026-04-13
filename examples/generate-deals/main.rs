use dds_bridge::Seat;
use pons::full_deal;
use std::process::ExitCode;

fn generate_deals(n: usize) {
    (0..n).for_each(|_| println!("{}", full_deal(&mut rand::rng()).display(Seat::North)));
}

#[doc = include_str!("README.md")]
fn main() -> ExitCode {
    match std::env::args().nth(1) {
        Some(string) => {
            if let Ok(n) = string.parse::<usize>() {
                generate_deals(n);
            } else {
                eprintln!("{}", include_str!("README.md"));
                return ExitCode::FAILURE;
            }
        }
        None => generate_deals(100),
    }
    ExitCode::SUCCESS
}
