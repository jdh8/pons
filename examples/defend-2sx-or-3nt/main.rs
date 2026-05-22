use clap::Parser;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::deck::{self, full_deal};
use contract_bridge::eval::{self, HandEvaluator};
use contract_bridge::{
    Bid, Builder, Contract, FullDeal, Hand, Level, Penalty, Rank, Seat, Strain, Suit,
};
use dds_bridge::{Vulnerability, solve_deals};
use pons::bidding::array::Logits;
use pons::bidding::{System, Trie};

const TWO_SPADES: Call = Call::Bid(Bid {
    level: Level::new(2),
    strain: Strain::Spades,
});

const THREE_NT: Call = Call::Bid(Bid {
    level: Level::new(3),
    strain: Strain::Notrump,
});

const TWO_NT: Call = Call::Bid(Bid {
    level: Level::new(2),
    strain: Strain::Notrump,
});

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

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
    #[arg(long, default_value = "5000")]
    max_attempts_per_deal: usize,
}

fn hcp_total(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| eval::hcp::<u8>(hand[s])).sum()
}

/// West's natural opening at `[]`: 2♠ iff a textbook weak two
/// (exactly six spades, fewer than four hearts, 5–10 HCP).
fn west_call(hand: Hand) -> Call {
    let spades = hand[Suit::Spades];
    let hearts = hand[Suit::Hearts];
    let hcp = hcp_total(hand);
    if spades.len() == 6 && hearts.len() < 4 && (5..=10).contains(&hcp) {
        TWO_SPADES
    } else {
        Call::Pass
    }
}

/// North's natural call after `[2♠]`: takeout double iff 12+ HCP, ≤2 spades,
/// ≥3 hearts.
fn north_call(hand: Hand) -> Call {
    let hcp = hcp_total(hand);
    let spades = hand[Suit::Spades].len();
    let hearts = hand[Suit::Hearts].len();
    if hcp >= 12 && spades <= 2 && hearts >= 3 {
        Call::Double
    } else {
        Call::Pass
    }
}

/// South's natural call after `[2♠, X, P]`. Hands that would naturally bid a
/// new suit, jump-raise hearts, or escape via Lebensohl 2NT all return calls
/// other than `Pass` / `3NT`, and the example rejects those deals so the
/// reported averages only cover the P-vs-3NT decision.
fn south_call(hand: Hand) -> Call {
    let spades = hand[Suit::Spades];
    let hearts = hand[Suit::Hearts];
    let clubs = hand[Suit::Clubs];
    let diamonds = hand[Suit::Diamonds];
    let hcp = hcp_total(hand);
    let strength: f64 = eval::FIFTHS.eval(hand);

    if hearts.len() >= 4 {
        let level = if hcp >= 10 { 4 } else { 3 };
        return bid(level, Strain::Hearts);
    }
    if clubs.len() >= 5 {
        return bid(3, Strain::Clubs);
    }
    if diamonds.len() >= 5 {
        return bid(3, Strain::Diamonds);
    }

    let spade_honors = u8::from(spades.contains(Rank::A))
        + u8::from(spades.contains(Rank::K))
        + u8::from(spades.contains(Rank::Q))
        + u8::from(spades.contains(Rank::J));
    if spades.len() >= 4 && spade_honors >= 2 && hcp >= 6 {
        return Call::Pass;
    }

    let stopper = spades.contains(Rank::A) || spades.contains(Rank::K) || spades.contains(Rank::Q);
    if spades.len() <= 3 && strength >= FIFTHS_THRESHOLD && stopper {
        return THREE_NT;
    }

    TWO_NT
}

/// Build the bidding system: three classifiers wired to the same `Trie`.
///
/// * `[]`              — West's opening (2♠ or Pass)
/// * `[2♠]`            — North's call (Double or Pass)
/// * `[2♠, X, P]`      — South's natural call (Pass, 3NT, or out-of-scope)
fn build_system() -> Trie {
    let mut trie = Trie::new();

    trie.insert(&[], |hand: Hand, _vul: RelativeVulnerability| {
        let mut logits = Logits::new();
        *logits.0.get_mut(west_call(hand)) = 1.0;
        logits
    });

    trie.insert(&[TWO_SPADES], |hand: Hand, _vul: RelativeVulnerability| {
        let mut logits = Logits::new();
        *logits.0.get_mut(north_call(hand)) = 1.0;
        logits
    });

    trie.insert(
        &[TWO_SPADES, Call::Double, Call::Pass],
        |hand: Hand, _vul: RelativeVulnerability| {
            let mut logits = Logits::new();
            *logits.0.get_mut(south_call(hand)) = 1.0;
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

fn west_opens(system: &Trie, deal: &FullDeal) -> bool {
    decide_call(system, &[], deal[Seat::West]) == TWO_SPADES
}

fn north_doubles(system: &Trie, deal: &FullDeal) -> bool {
    decide_call(system, &[TWO_SPADES], deal[Seat::North]) == Call::Double
}

fn south_in_scope(system: &Trie, deal: &FullDeal) -> bool {
    let south_auction = [TWO_SPADES, Call::Double, Call::Pass];
    let call = decide_call(system, &south_auction, deal[Seat::South]);
    call == Call::Pass || call == THREE_NT
}

struct Totals {
    pass: i64,
    nt: i64,
    oracle: i64,
    oracle_chose_3nt: usize,
}

fn collect_deals(
    args: &Args,
    system: &Trie,
    rng: &mut (impl rand::Rng + ?Sized),
) -> anyhow::Result<(Vec<FullDeal>, usize)> {
    let cap = args.count.saturating_mul(args.max_attempts_per_deal);
    let mut deals: Vec<FullDeal> = Vec::with_capacity(args.count);
    let mut attempts: usize = 0;

    let accept = |deal: FullDeal, deals: &mut Vec<FullDeal>| {
        if west_opens(system, &deal)
            && north_doubles(system, &deal)
            && south_in_scope(system, &deal)
        {
            deals.push(deal);
        }
    };

    match args.south {
        Some(south) => {
            let cards = Builder::new()
                .south(south)
                .build_partial()
                .map_err(|_| anyhow::anyhow!("invalid south hand"))?;
            let mut iter = deck::fill_deals(rng, cards);
            while deals.len() < args.count && attempts < cap {
                let deal = iter.next().expect("fill_deals iterator is infinite");
                attempts += 1;
                accept(deal, &mut deals);
            }
        }
        None => {
            while deals.len() < args.count && attempts < cap {
                let deal = full_deal(rng);
                attempts += 1;
                accept(deal, &mut deals);
            }
        }
    }

    Ok((deals, attempts))
}

fn score_deals(deals: &[FullDeal], vulnerability: Vulnerability) -> Totals {
    let tables = solve_deals(deals);
    let two_sx = Contract::new(2, Strain::Spades, Penalty::Doubled);
    let three_nt = Contract::new(3, Strain::Notrump, Penalty::Undoubled);
    let ns_vul = vulnerability.contains(Vulnerability::NS);
    let ew_vul = vulnerability.contains(Vulnerability::EW);

    let mut totals = Totals {
        pass: 0,
        nt: 0,
        oracle: 0,
        oracle_chose_3nt: 0,
    };

    for table in &tables {
        let tricks_w_spades = u8::from(table[Strain::Spades].get(Seat::West));
        let tricks_s_nt = u8::from(table[Strain::Notrump].get(Seat::South));
        let pass_score = -i64::from(two_sx.score(tricks_w_spades, ew_vul));
        let nt_score = i64::from(three_nt.score(tricks_s_nt, ns_vul));

        let (oracle_score, picked_3nt) = if nt_score > pass_score {
            (nt_score, true)
        } else {
            (pass_score, false)
        };

        totals.pass += pass_score;
        totals.nt += nt_score;
        totals.oracle += oracle_score;
        totals.oracle_chose_3nt += usize::from(picked_3nt);
    }

    totals
}

#[allow(clippy::cast_precision_loss)]
fn print_summary(args: &Args, deals: &[FullDeal], attempts: usize, totals: &Totals) {
    let n = deals.len() as f64;
    let avg_pass = totals.pass as f64 / n;
    let avg_3nt = totals.nt as f64 / n;
    let avg_oracle = totals.oracle as f64 / n;

    println!(
        "Sample size: {got} / {target} valid deals ({attempts} attempts)",
        got = deals.len(),
        target = args.count,
    );
    let pct = 100.0 * totals.oracle_chose_3nt as f64 / n;
    println!(
        "Oracle chose 3NT: {chose_3nt}/{got} ({pct:.0}%)",
        chose_3nt = totals.oracle_chose_3nt,
        got = deals.len()
    );
    println!("Average NS score:");
    println!("  Always defend 2♠x     : {avg_pass:+.0}");
    println!("  Always declare 3NT    : {avg_3nt:+.0}");
    println!("  Oracle (best per deal): {avg_oracle:+.0}");
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut rng = rand::rng();
    let system = build_system();

    if let Some(south) = args.south {
        let south_auction = [TWO_SPADES, Call::Double, Call::Pass];
        let call = decide_call(&system, &south_auction, south);
        if call != Call::Pass && call != THREE_NT {
            anyhow::bail!(
                "this South hand naturally bids {call}, not P or 3NT — \
                 pick a hand whose call is in scope"
            );
        }
    }

    let (deals, attempts) = collect_deals(&args, &system, &mut rng)?;
    if deals.is_empty() {
        anyhow::bail!(
            "no deals reached the (2♠) X (P) → P/3NT auction in {attempts} attempts; \
             try raising --max-attempts-per-deal"
        );
    }

    let totals = score_deals(&deals, args.vulnerability);
    print_summary(&args, &deals, attempts, &totals);
    Ok(())
}
