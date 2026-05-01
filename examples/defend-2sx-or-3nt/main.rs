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

    /// Number of valid deals to accept
    #[arg(short, long, default_value = "90")]
    count: usize,

    /// Cap on attempts per accepted deal during rejection sampling
    #[arg(long, default_value = "2000")]
    max_attempts_per_deal: usize,
}

fn hcp_total(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| eval::hcp::<u8>(hand[s])).sum()
}

/// Build the bidding system: three classifiers wired to the same `Trie`.
///
/// * `[]`              — does West open 2♠ as a weak two?
/// * `[2♠]`            — does North takeout-double?
/// * `[2♠, X, P]`      — does South pick 3NT or Pass?
fn build_system() -> Trie {
    let mut trie = Trie::new();

    // West: weak two opener iff exactly six spades, fewer than four hearts,
    // and 5–10 HCP.
    trie.insert(&[], |hand: Hand, _vul: RelativeVulnerability| {
        let spades = hand[Suit::Spades];
        let hearts = hand[Suit::Hearts];
        let hcp = hcp_total(hand);
        let weak_two = spades.len() == 6 && hearts.len() < 4 && (5..=10).contains(&hcp);

        let mut logits = Logits::new();
        if weak_two {
            *logits.0.get_mut(TWO_SPADES) = 1.0;
            *logits.0.get_mut(Call::Pass) = 0.0;
        } else {
            *logits.0.get_mut(Call::Pass) = 1.0;
            *logits.0.get_mut(TWO_SPADES) = 0.0;
        }
        logits
    });

    // North: takeout double iff 12+ HCP, ≤2 spades, ≥3 hearts.
    trie.insert(&[TWO_SPADES], |hand: Hand, _vul: RelativeVulnerability| {
        let hcp = hcp_total(hand);
        let spades = hand[Suit::Spades].len();
        let hearts = hand[Suit::Hearts].len();
        let takeout = hcp >= 12 && spades <= 2 && hearts >= 3;

        let mut logits = Logits::new();
        if takeout {
            *logits.0.get_mut(Call::Double) = 1.0;
            *logits.0.get_mut(Call::Pass) = 0.0;
        } else {
            *logits.0.get_mut(Call::Pass) = 1.0;
            *logits.0.get_mut(Call::Double) = 0.0;
        }
        logits
    });

    // South: 3NT iff FIFTHS ≥ threshold and a spade stopper.
    trie.insert(
        &[TWO_SPADES, Call::Double, Call::Pass],
        |hand: Hand, _vul: RelativeVulnerability| {
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
        },
    );

    trie
}

fn decide_call(system: &Trie, auction: &[Call], hand: Hand) -> Call {
    let logits = system
        .classify(hand, RelativeVulnerability::NONE, auction)
        .expect("classifier registered for this auction");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("Array<f32> always has 38 entries")
}

fn auction_reached(system: &Trie, deal: &FullDeal) -> bool {
    decide_call(system, &[], deal[Seat::West]) == TWO_SPADES
        && decide_call(system, &[TWO_SPADES], deal[Seat::North]) == Call::Double
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut rng = rand::rng();
    let system = build_system();

    let cap = args.count.saturating_mul(args.max_attempts_per_deal);
    let mut deals: Vec<FullDeal> = Vec::with_capacity(args.count);
    let mut attempts: usize = 0;

    match args.south {
        Some(south) => {
            let cards = Builder::new()
                .south(south)
                .build_partial()
                .map_err(|_| anyhow::anyhow!("invalid south hand"))?;
            let mut iter = deck::fill_deals(&mut rng, cards);
            while deals.len() < args.count && attempts < cap {
                let deal = iter.next().expect("fill_deals iterator is infinite");
                attempts += 1;
                if auction_reached(&system, &deal) {
                    deals.push(deal);
                }
            }
        }
        None => {
            while deals.len() < args.count && attempts < cap {
                let deal = full_deal(&mut rng);
                attempts += 1;
                if auction_reached(&system, &deal) {
                    deals.push(deal);
                }
            }
        }
    }

    if deals.is_empty() {
        anyhow::bail!(
            "no deals reached the (2♠) X (P) auction in {attempts} attempts; \
             try raising --max-attempts-per-deal"
        );
    }

    let tables = solver::Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    let two_sx = Contract::new(2, Strain::Spades, Penalty::Doubled);
    let three_nt = Contract::new(3, Strain::Notrump, Penalty::Undoubled);
    let ns_vul = args.vulnerability.contains(Vulnerability::NS);
    let ew_vul = args.vulnerability.contains(Vulnerability::EW);

    let mut total_pass: i64 = 0;
    let mut total_3nt: i64 = 0;
    let mut total_system: i64 = 0;
    let mut chose_3nt: usize = 0;

    let south_auction = [TWO_SPADES, Call::Double, Call::Pass];
    for (deal, table) in deals.iter().zip(tables.iter()) {
        let tricks_w_spades = u8::from(table[Strain::Spades].get(Seat::West));
        let tricks_s_nt = u8::from(table[Strain::Notrump].get(Seat::South));

        let pass_score = -i64::from(two_sx.score(tricks_w_spades, ew_vul));
        let nt_score = i64::from(three_nt.score(tricks_s_nt, ns_vul));

        let chosen = decide_call(&system, &south_auction, deal[Seat::South]);
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

    let n = deals.len() as f64;
    let avg_pass = total_pass as f64 / n;
    let avg_3nt = total_3nt as f64 / n;
    let avg_system = total_system as f64 / n;

    println!(
        "Sample size: {got} / {target} valid deals ({attempts} attempts)",
        got = deals.len(),
        target = args.count,
    );
    if args.south.is_some() {
        let call = if chose_3nt == deals.len() {
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
            "System chose 3NT: {chose_3nt}/{got} ({pct:.0}%)",
            got = deals.len()
        );
    }
    println!("Average NS score:");
    println!("  Always defend 2♠x : {avg_pass:+.0}");
    println!("  Always declare 3NT: {avg_3nt:+.0}");
    println!("  System policy     : {avg_system:+.0}");

    Ok(())
}
