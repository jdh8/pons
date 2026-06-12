//! Compare defending 2♠× and declaring 3NT after `(2♠) X (P)`.
//!
//! West opens a weak 2♠, North makes a takeout double, East passes, and South
//! must choose between defending 2♠ doubled and declaring 3NT.  All three
//! decisions are taken by the real `two_over_one` system: West's weak-two
//! opening from the constructive book, North's takeout double and South's
//! advance from the defensive book (`defense_to_weak_two` and
//! `advance_double`).
//!
//! Rejection sampling keeps only the deals that actually reach this auction —
//! West opens 2♠, North doubles, and South's advance is `Pass` or `3NT` — so
//! the reported averages cover exactly the P-vs-3NT decision.  Hands the system
//! advances with a new suit, a major-suit game, or a lebensohl 2NT fall out of
//! scope and are discarded.

use clap::Parser;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::deck::{self, full_deal};
use contract_bridge::{
    AbsoluteVulnerability, Bid, Builder, Contract, FullDeal, Hand, Level, Penalty, Seat, Strain,
};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::bidding::{Family, Stance, System};
use pons::two_over_one;

const TWO_SPADES: Call = Call::Bid(Bid {
    level: Level::new(2),
    strain: Strain::Spades,
});

const THREE_NT: Call = Call::Bid(Bid {
    level: Level::new(3),
    strain: Strain::Notrump,
});

/// Compare defending 2♠× and declaring 3NT after `(2♠) X (P)`
#[derive(Parser)]
struct Args {
    /// South's hand in dot-separated suit notation (e.g. AKQ.J92.AT8.K754).
    /// If omitted, South is randomized along with the other seats.
    #[arg(short, long)]
    south: Option<Hand>,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Number of valid deals to accept
    #[arg(short, long, default_value = "90")]
    count: usize,

    /// Cap on attempts per accepted deal during rejection sampling
    #[arg(long, default_value = "5000")]
    max_attempts_per_deal: usize,
}

/// The 2/1 system bound against natural opponents
///
/// One [`Stance`] answers all three seats: West opens from the constructive
/// book, North and South act from the defensive book.  The auction is keyed as
/// an analysis fragment — West sits at index 0, so `[]` is West's opening,
/// `[2♠]` is North's call, and `[2♠, X, P]` is South's advance.
fn build_system() -> Stance {
    two_over_one().against(Family::NATURAL)
}

/// The highest-logit legal call the system assigns the hand for the auction
fn decide_call(system: &Stance, auction: &[Call], hand: Hand) -> Call {
    let logits = system
        .classify(hand, RelativeVulnerability::NONE, auction)
        .expect("the 2/1 system covers this auction");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("Array<f32> always has 38 entries")
}

fn west_opens(system: &Stance, deal: &FullDeal) -> bool {
    decide_call(system, &[], deal[Seat::West]) == TWO_SPADES
}

fn north_doubles(system: &Stance, deal: &FullDeal) -> bool {
    decide_call(system, &[TWO_SPADES], deal[Seat::North]) == Call::Double
}

fn south_in_scope(system: &Stance, deal: &FullDeal) -> bool {
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
    system: &Stance,
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

fn score_deals(deals: &[FullDeal], vulnerability: AbsoluteVulnerability) -> Totals {
    let tables = Solver::lock().solve_deals(deals, NonEmptyStrainFlags::ALL);
    let two_sx = Contract::new(2, Strain::Spades, Penalty::Doubled);
    let three_nt = Contract::new(3, Strain::Notrump, Penalty::Undoubled);
    let ns_vul = vulnerability.contains(AbsoluteVulnerability::NS);
    let ew_vul = vulnerability.contains(AbsoluteVulnerability::EW);

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
