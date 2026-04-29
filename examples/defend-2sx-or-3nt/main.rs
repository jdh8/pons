use clap::Parser;
use dds_bridge::solver::{self, NonEmptyStrainFlags, Vulnerability};
use dds_bridge::{
    Bid, Builder, Contract, FullDeal, Hand, Level, Penalty, Rank, Seat, Strain, Suit,
};
use pons::bidding::array::Logits;
use pons::bidding::{Call, RelativeVulnerability, System, Trie};
use pons::eval::{self, HandEvaluator};
use pons::{deck, full_deal};

const TWO_SPADES: Call = Call::Bid(Bid {
    level: Level::new(2),
    strain: Strain::Spades,
});

const THREE_NT: Call = Call::Bid(Bid {
    level: Level::new(3),
    strain: Strain::Notrump,
});

const FIFTHS_THRESHOLD: f64 = 12.0;

/// Compare defending 2♠× and declaring 3NT after `(2♠) X (P)`
#[derive(Parser)]
struct Args {
    /// South's hand in dot-separated suit notation (e.g. AKQ.J92.AT8.K754).
    /// If omitted, South is randomized along with the other seats.
    #[arg(short, long)]
    south: Option<Hand>,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: Vulnerability,

    /// Number of simulated deals
    #[arg(short, long, default_value = "90")]
    count: usize,
}

/// Build the bidding system: a single classifier at `[2♠, X, P]` for South.
///
/// 3NT iff the FIFTHS evaluation is at least the threshold and South holds a
/// spade stopper (4+ spades, or A/K/Q♠). Otherwise, Pass.
fn build_system() -> Trie {
    let mut trie = Trie::new();
    let auction = [TWO_SPADES, Call::Double, Call::Pass];

    trie.insert(&auction, |hand: Hand, _vul: RelativeVulnerability| {
        let strength: f64 = eval::FIFTHS.eval(hand);
        let spades = hand[Suit::Spades];
        let stopper = spades.len() >= 4
            || spades.contains(Rank::A)
            || spades.contains(Rank::K)
            || spades.contains(Rank::Q);

        let mut logits = Logits::new();
        if strength >= FIFTHS_THRESHOLD && stopper {
            *logits.0.get_mut(THREE_NT) = 1.0;
            *logits.0.get_mut(Call::Pass) = 0.0;
        } else {
            *logits.0.get_mut(Call::Pass) = 1.0;
            *logits.0.get_mut(THREE_NT) = 0.0;
        }
        logits
    });

    trie
}

fn decide(system: &Trie, hand: Hand) -> Call {
    let auction = [TWO_SPADES, Call::Double, Call::Pass];
    let logits = system
        .classify(hand, RelativeVulnerability::NONE, &auction)
        .expect("classifier registered for this auction");
    if logits.0[THREE_NT] > logits.0[Call::Pass] {
        THREE_NT
    } else {
        Call::Pass
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut rng = rand::rng();

    let deals: Vec<FullDeal> = match args.south {
        Some(south) => {
            let cards = Builder::new()
                .south(south)
                .build_partial()
                .map_err(|_| anyhow::anyhow!("invalid south hand"))?;
            deck::fill_deals(&mut rng, cards).take(args.count).collect()
        }
        None => core::iter::repeat_with(|| full_deal(&mut rng))
            .take(args.count)
            .collect(),
    };

    let tables = solver::Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    let two_sx = Contract::new(2, Strain::Spades, Penalty::Doubled);
    let three_nt = Contract::new(3, Strain::Notrump, Penalty::Undoubled);
    let ns_vul = args.vulnerability.contains(Vulnerability::NS);
    let ew_vul = args.vulnerability.contains(Vulnerability::EW);

    let system = build_system();
    let mut total_pass: i64 = 0;
    let mut total_3nt: i64 = 0;
    let mut total_system: i64 = 0;
    let mut chose_3nt: usize = 0;

    for (deal, table) in deals.iter().zip(tables.iter()) {
        let tricks_w_spades = u8::from(table[Strain::Spades].get(Seat::West));
        let tricks_s_nt = u8::from(table[Strain::Notrump].get(Seat::South));

        let pass_score = -i64::from(two_sx.score(tricks_w_spades, ew_vul));
        let nt_score = i64::from(three_nt.score(tricks_s_nt, ns_vul));

        let chosen = decide(&system, deal[Seat::South]);
        let system_score = if chosen == THREE_NT {
            chose_3nt += 1;
            nt_score
        } else {
            pass_score
        };

        total_pass += pass_score;
        total_3nt += nt_score;
        total_system += system_score;
    }

    let n = args.count.max(1) as f64;
    let avg_pass = total_pass as f64 / n;
    let avg_3nt = total_3nt as f64 / n;
    let avg_system = total_system as f64 / n;

    if args.south.is_some() {
        let call = if chose_3nt == args.count {
            "3NT"
        } else if chose_3nt == 0 {
            "P"
        } else {
            unreachable!("system is deterministic for a fixed south hand")
        };
        println!("System call: {call}");
    } else {
        let pct = 100.0 * chose_3nt as f64 / n;
        println!(
            "System chose 3NT: {chose_3nt}/{count} ({pct:.0}%)",
            count = args.count
        );
    }

    println!("Sample size: {}", args.count);
    println!("Average NS score:");
    println!("  Always defend 2♠x : {avg_pass:+.0}");
    println!("  Always declare 3NT: {avg_3nt:+.0}");
    println!("  System policy     : {avg_system:+.0}");

    Ok(())
}
