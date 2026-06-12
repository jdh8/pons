//! Compare flavors of the `(2♠) X (P)` decision by double-dummy simulation.
//!
//! The scenario is fixed: West opens a weak 2♠ (from the real [`two_over_one`]
//! system), East passes, North makes a takeout double, and South must choose
//! between defending 2♠ doubled and declaring 3NT.  The experiment sweeps two
//! decisions:
//!
//! 1. **Takeout-double flavors** — the rule by which North doubles.  Each
//!    flavor admits a different population of doubled deals; tightening the
//!    double (more values, a guaranteed major fit) shifts the population.  The
//!    double sweep reports, per flavor, how often it fires and how the deals
//!    score under perfect play (the *population stats*).
//! 2. **Response flavors** — South's policy for picking Pass vs 3NT.  Over the
//!    full doubled population, each policy commits to one of the two and is
//!    scored against the per-deal oracle (the *regret*: how much NS score the
//!    policy gives up against always knowing the right answer).
//!
//! Every flavor is written in the crate's constraint vocabulary
//! ([`pons::bidding::constraint`]) and evaluated as a crisp predicate, so the
//! experiment shares its hand-feature language with the system proper.
//!
//! ```text
//! cargo run --example defend-2sx-or-3nt -- --count 300 --vulnerability ns
//! ```

use clap::Parser;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::deck::full_deal;
use contract_bridge::{
    AbsoluteVulnerability, Bid, Contract, FullDeal, Hand, Level, Penalty, Seat, Strain, Suit,
};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::bidding::constraint::{Constraint, hcp, len, stopper_in, top_honors};
use pons::bidding::{Context, Family, Stance, System};
use pons::two_over_one;

const TWO_SPADES: Call = Call::Bid(Bid {
    level: Level::new(2),
    strain: Strain::Spades,
});

/// Compare flavors of the (2♠) X (P) defend-vs-declare decision
#[derive(Parser)]
struct Args {
    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Number of doubled deals to accept (union of the double flavors)
    #[arg(short, long, default_value = "300")]
    count: usize,

    /// Cap on attempts per accepted deal during rejection sampling
    #[arg(long, default_value = "5000")]
    max_attempts_per_deal: usize,
}

// ---------------------------------------------------------------------------
// Flavors, as crisp constraints
// ---------------------------------------------------------------------------

/// A named binary policy over hands, compiled from a crisp constraint
struct Flavor {
    name: &'static str,
    holds: Box<dyn Fn(Hand) -> bool>,
}

/// Compile a crisp constraint into a hand predicate (satisfied ⇒ `true`)
///
/// The flavors here read only the hand (HCP, suit lengths, spade honors and
/// stopper), so an empty auction context suffices.
fn flavor(name: &'static str, constraint: impl Constraint + 'static) -> Flavor {
    Flavor {
        name,
        holds: Box::new(move |hand| {
            let context = Context::new(RelativeVulnerability::NONE, &[]);
            constraint.eval(hand, &context).is_finite()
        }),
    }
}

/// North's takeout-double flavors over the weak 2♠
///
/// Each is a predicate on North's hand; a deal joins a flavor's population when
/// West opens 2♠ and North's hand satisfies it.  `Support` and `Sound` are both
/// subsets of `Shape`, so `Shape` is the broadest doubled population.
fn double_flavors() -> Vec<Flavor> {
    let s = Suit::Spades;
    let h = Suit::Hearts;
    vec![
        flavor("Shape   (12+, <=3S)", hcp(12..) & len(s, ..=3)),
        flavor(
            "Support (12+, <=2S, 3+H)",
            hcp(12..) & len(s, ..=2) & len(h, 3..),
        ),
        flavor("Sound   (14+, <=3S)", hcp(14..) & len(s, ..=3)),
    ]
}

/// South's response flavors: a "bid 3NT" predicate (else pass for penalty)
///
/// Each commits the hand to one of the two studied calls, so it scores on every
/// doubled deal.  All three want a spade stopper; they differ in the values
/// required and in how strong a spade holding tips the choice back to defense.
fn response_flavors() -> Vec<Flavor> {
    let s = Suit::Spades;
    let stopper = stopper_in(s);
    vec![
        // Eager penalty: declare only with extra values and no spade holding.
        flavor(
            "Defense  (eager penalty)",
            stopper.clone() & hcp(15..) & !(len(s, 4..) & top_honors(s, 1..)),
        ),
        // Balanced: the current `advance_double` split — penalty-pass a stack.
        flavor(
            "Balanced (current)",
            stopper.clone() & hcp(13..) & !(len(s, 4..) & top_honors(s, 2..)),
        ),
        // Eager 3NT: declare on light values, defend only on a strong stack.
        flavor(
            "Offense  (eager 3NT)",
            stopper & hcp(11..) & !(len(s, 5..) | (len(s, 4..) & top_honors(s, 3..))),
        ),
    ]
}

// ---------------------------------------------------------------------------
// West's opening, from the real system
// ---------------------------------------------------------------------------

/// The 2/1 system bound against natural opponents (used only for West's opening)
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

// ---------------------------------------------------------------------------
// Sampling and scoring
// ---------------------------------------------------------------------------

/// A deal's two outcomes under perfect play, in NS score
#[derive(Clone, Copy)]
struct Outcome {
    /// NS score of defending 2♠ doubled (South passes)
    defend: i64,
    /// NS score of declaring 3NT (South bids)
    declare: i64,
}

impl Outcome {
    /// The better of the two for NS
    fn oracle(self) -> i64 {
        self.defend.max(self.declare)
    }

    /// `true` when 3NT outscores the defense
    fn prefers_3nt(self) -> bool {
        self.declare > self.defend
    }
}

struct Collected {
    deals: Vec<FullDeal>,
    /// Number of sampled deals in which West opened 2♠ (the conditioning event)
    west_opened: usize,
    attempts: usize,
}

fn collect_deals(
    args: &Args,
    system: &Stance,
    flavors: &[Flavor],
    rng: &mut (impl rand::Rng + ?Sized),
) -> Collected {
    let cap = args.count.saturating_mul(args.max_attempts_per_deal);
    let mut deals: Vec<FullDeal> = Vec::with_capacity(args.count);
    let mut west_opened = 0;
    let mut attempts = 0;

    while deals.len() < args.count && attempts < cap {
        let deal = full_deal(rng);
        attempts += 1;
        if !west_opens(system, &deal) {
            continue;
        }
        west_opened += 1;
        let north = deal[Seat::North];
        if flavors.iter().any(|flavor| (flavor.holds)(north)) {
            deals.push(deal);
        }
    }

    Collected {
        deals,
        west_opened,
        attempts,
    }
}

fn score_deals(deals: &[FullDeal], vulnerability: AbsoluteVulnerability) -> Vec<Outcome> {
    let tables = Solver::lock().solve_deals(deals, NonEmptyStrainFlags::ALL);
    let two_sx = Contract::new(2, Strain::Spades, Penalty::Doubled);
    let three_nt = Contract::new(3, Strain::Notrump, Penalty::Undoubled);
    let ns_vul = vulnerability.contains(AbsoluteVulnerability::NS);
    let ew_vul = vulnerability.contains(AbsoluteVulnerability::EW);

    tables
        .iter()
        .map(|table| {
            let tricks_w_spades = u8::from(table[Strain::Spades].get(Seat::West));
            let tricks_s_nt = u8::from(table[Strain::Notrump].get(Seat::South));
            Outcome {
                defend: -i64::from(two_sx.score(tricks_w_spades, ew_vul)),
                declare: i64::from(three_nt.score(tricks_s_nt, ns_vul)),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

#[allow(clippy::cast_precision_loss)]
fn mean(values: impl Iterator<Item = i64>) -> f64 {
    let (sum, n) = values.fold((0i64, 0usize), |(sum, n), v| (sum + v, n + 1));
    if n == 0 { 0.0 } else { sum as f64 / n as f64 }
}

/// Per-flavor population stats: how favorable is the doubled situation?
#[allow(clippy::cast_precision_loss)]
fn print_double_sweep(
    flavors: &[Flavor],
    deals: &[FullDeal],
    scores: &[Outcome],
    west_opened: usize,
) {
    println!("=== Takeout-double flavors (West opened 2S {west_opened}x; population stats) ===");
    println!(
        "  {:<26}{:>6}{:>8}{:>8}{:>10}{:>10}{:>10}",
        "Flavor", "n", "of 2S", "3NT%", "defend", "declare", "oracle",
    );
    for flavor in flavors {
        let members: Vec<usize> = (0..deals.len())
            .filter(|&i| (flavor.holds)(deals[i][Seat::North]))
            .collect();
        let n = members.len();
        if n == 0 {
            println!("  {:<26}{:>6}{:>8}", flavor.name, 0, "--");
            continue;
        }
        let of_2s = 100.0 * n as f64 / west_opened as f64;
        let pct_3nt =
            100.0 * members.iter().filter(|&&i| scores[i].prefers_3nt()).count() as f64 / n as f64;
        let defend = mean(members.iter().map(|&i| scores[i].defend));
        let declare = mean(members.iter().map(|&i| scores[i].declare));
        let oracle = mean(members.iter().map(|&i| scores[i].oracle()));
        println!(
            "  {:<26}{:>6}{:>7.1}%{:>7.0}%{:>+10.0}{:>+10.0}{:>+10.0}",
            flavor.name, n, of_2s, pct_3nt, defend, declare, oracle,
        );
    }
}

/// Response policies over the full doubled population, scored by regret
#[allow(clippy::cast_precision_loss)]
fn print_response_sweep(policies: &[Flavor], deals: &[FullDeal], scores: &[Outcome]) {
    let n = deals.len() as f64;
    let oracle = mean(scores.iter().map(|o| o.oracle()));
    println!(
        "\n=== South advance policies (doubled population, n={}) ===",
        deals.len()
    );
    println!(
        "  {:<26}{:>8}{:>10}{:>10}",
        "Policy", "3NT%", "avg NS", "regret"
    );

    let row = |name: &str, rate: f64, avg: f64| {
        println!("  {name:<26}{rate:>7.0}%{avg:>+10.0}{:>10.0}", oracle - avg);
    };

    // Trivial baselines first, then the smart flavors, then the oracle.
    row(
        "Always defend 2Sx",
        0.0,
        mean(scores.iter().map(|o| o.defend)),
    );
    row(
        "Always declare 3NT",
        100.0,
        mean(scores.iter().map(|o| o.declare)),
    );
    for policy in policies {
        let choices: Vec<bool> = deals
            .iter()
            .map(|d| (policy.holds)(d[Seat::South]))
            .collect();
        let rate = 100.0 * choices.iter().filter(|&&b| b).count() as f64 / n;
        let avg = mean(choices.iter().zip(scores).map(
            |(&three_nt, o)| {
                if three_nt { o.declare } else { o.defend }
            },
        ));
        row(policy.name, rate, avg);
    }
    let oracle_rate = 100.0 * scores.iter().filter(|o| o.prefers_3nt()).count() as f64 / n;
    row("Oracle (best per deal)", oracle_rate, oracle);
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut rng = rand::rng();
    let system = build_system();
    let doubles = double_flavors();
    let responses = response_flavors();

    let collected = collect_deals(&args, &system, &doubles, &mut rng);
    if collected.deals.is_empty() {
        anyhow::bail!(
            "no deals reached (2S) X (P) in {} attempts; try raising --max-attempts-per-deal",
            collected.attempts,
        );
    }

    let scores = score_deals(&collected.deals, args.vulnerability);
    println!(
        "Sample: {got}/{target} doubled deals ({attempts} attempts)\n",
        got = collected.deals.len(),
        target = args.count,
        attempts = collected.attempts,
    );
    print_double_sweep(&doubles, &collected.deals, &scores, collected.west_opened);
    print_response_sweep(&responses, &collected.deals, &scores);
    Ok(())
}
