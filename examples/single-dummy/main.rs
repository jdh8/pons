use clap::Parser;
use contract_bridge::{Bid, Hand, Level, Seat, Strain};
use pons::single_dummy;
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Estimate single-dummy tricks for a declarer + dummy over random defender hands
#[derive(Parser)]
struct Args {
    /// Declarer's hand in dot-separated suit notation (e.g. AKQJT98.AK.AK.AK)
    #[arg(short, long)]
    declarer: Hand,

    /// Dummy's hand in dot-separated suit notation (e.g. 765432.QJT.QJ.QJ)
    #[arg(short = 'm', long)]
    dummy: Hand,

    /// Declarer's seat: N, E, S, W (or full name)
    #[arg(short, long, default_value = "n")]
    seat: Seat,

    /// Number of sampled defender layouts
    #[arg(short, long, default_value = "1000")]
    count: usize,

    /// RNG seed (fixed for reproducibility)
    #[arg(long, default_value = "0")]
    seed: u64,
}

fn main() {
    let args = Args::parse();
    let mut rng = StdRng::seed_from_u64(args.seed);
    let hist = single_dummy(args.declarer, args.dummy, args.seat, &mut rng, args.count);

    println!("Declarer {}, {} sampled layouts", args.seat, args.count);
    println!("strain  E[tricks]  P(game)");
    for strain in Strain::ASC {
        // Game is 5 of a minor, 4 of a major, 3NT.
        let level = match strain {
            Strain::Clubs | Strain::Diamonds => 5,
            Strain::Hearts | Strain::Spades => 4,
            Strain::Notrump => 3,
        };
        let bid = Bid {
            level: Level::new(level),
            strain,
        };
        println!(
            "{strain:>6}  {:>9.2}  {:>7.1}%",
            hist.expected_tricks(args.seat, strain),
            100.0 * hist.make_probability(args.seat, bid),
        );
    }
}
