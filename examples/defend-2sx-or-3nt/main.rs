//! Compare flavors of the `(2♠) X (P)` decision by double-dummy simulation.
//!
//! The scenario: West opens a weak 2♠ (from the real [`american`]
//! system), North makes a takeout double, East passes, and South must choose
//! between defending and declaring 3NT.  Deals are accepted through a
//! four-gate funnel so the studied population is the *live* one:
//!
//! 1. **West opens 2♠** — the system's top call at the table's vulnerability.
//! 2. **North doubles** — by one of the swept takeout-double flavors below.
//! 3. **East passes** — the system's top call over `(2♠) X`; deals where East
//!    would raise or otherwise act never reach South's decision.
//! 4. **South's decision is live** — the system's own advance over
//!    `(2♠) X (P)` is Pass or 3NT; hands that would bid a suit or escape are
//!    not facing this choice.
//!
//! The experiment then sweeps two decisions:
//!
//! 1. **Takeout-double flavors** — the rule by which North doubles.  Each
//!    flavor admits a different population of doubled deals; tightening the
//!    double (more values, a guaranteed major fit) shifts the population.  The
//!    double sweep reports, per flavor, how often it fires over a 2♠ opening
//!    and how the live deals score under perfect play (the *population
//!    stats*).
//! 2. **Response flavors** — South's policy for picking Pass vs 3NT.  Over the
//!    full live population, each policy commits to one of the two and is
//!    scored against the per-deal oracle (the *regret*: how much NS score the
//!    policy gives up against always knowing the right answer).
//!
//! Neither branch assumes the auction is over: after South's call the full
//! table keeps bidding — West may run from the penalty pass, East/West may
//! double 3NT or sacrifice — and the *final* contract is scored double dummy.
//! The response sweep deliberately overrides South's first call in each
//! branch; every later call (including South's) comes from the system.
//!
//! The double and response flavors are written in the crate's constraint
//! vocabulary ([`pons::bidding::constraint`]) and evaluated as crisp
//! predicates, so the experiment shares its hand-feature language with the
//! system proper.
//!
//! ```text
//! cargo run --example defend-2sx-or-3nt -- --count 300 --vulnerability ns
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::deck::full_deal;
use contract_bridge::{
    AbsoluteVulnerability, Bid, Contract, FullDeal, Hand, Penalty, Seat, Strain, Suit,
};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::constraint::{Constraint, hcp, len, stopper_in, top_honors};
use pons::bidding::{Context, Stance, Table};
use pons::scoring::{final_contract, ns_score};
use std::collections::HashMap;

const TWO_SPADES: Call = Call::Bid(Bid::new(2, Strain::Spades));
const THREE_NOTRUMP: Call = Call::Bid(Bid::new(3, Strain::Notrump));

/// Compare flavors of the (2♠) X (P) defend-vs-declare decision
#[derive(Parser)]
struct Args {
    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Number of live doubled deals to accept (union of the double flavors)
    #[arg(short, long, default_value = "300")]
    count: usize,

    /// Cap on attempts per accepted deal during rejection sampling
    #[arg(long, default_value = "20000")]
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
/// stopper) through context-free primitives, so an empty auction context is
/// exact.  The *live* decisions in the funnel — West's opening, East's pass,
/// South's system advance — are taken by the table at the real vulnerability.
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
/// Each is a predicate on North's hand; a deal joins a flavor's population
/// when it survives the funnel and North's hand satisfies it.  `Support` and
/// `Sound` are both subsets of `Shape`, so `Shape` is the broadest doubled
/// population.
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
/// Each commits the hand to one of the two studied calls, so it scores on
/// every live deal.  All three want a spade stopper; they differ in the
/// values required and in how strong a spade holding tips the choice back to
/// defense.
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
// The table, from the real system
// ---------------------------------------------------------------------------

/// Both pairs play 2/1 at a table with West dealing, at the CLI vulnerability
fn build_table(vul: AbsoluteVulnerability) -> Table<Stance, Stance> {
    let pair = american();
    Table::of_pairs(&pair, &pair, Seat::West, vul)
}

/// The auction seeded with `(2♠) X (P)` and South's `decision`
fn seeded_auction(decision: Call) -> Auction {
    let mut auction = Auction::new();
    auction
        .try_extend([TWO_SPADES, Call::Double, Call::Pass, decision])
        .expect("the seeded prefix is legal from an empty auction");
    auction
}

// ---------------------------------------------------------------------------
// Sampling: the four-gate funnel
// ---------------------------------------------------------------------------

struct Collected {
    deals: Vec<FullDeal>,
    attempts: usize,
    /// Deals in which West opened 2♠ (gate 1)
    west_opened: usize,
    /// Per-flavor double counts among `west_opened` (the true firing rates)
    flavor_fired: Vec<usize>,
    /// Deals surviving the double-flavor union (gate 2)
    north_doubled: usize,
    /// Deals in which East also passed over the double (gate 3)
    east_passed: usize,
}

fn collect_deals(
    args: &Args,
    table: &Table<Stance, Stance>,
    flavors: &[Flavor],
    rng: &mut (impl rand::Rng + ?Sized),
) -> Collected {
    let cap = args.count.saturating_mul(args.max_attempts_per_deal);
    let mut collected = Collected {
        deals: Vec::with_capacity(args.count),
        attempts: 0,
        west_opened: 0,
        flavor_fired: vec![0; flavors.len()],
        north_doubled: 0,
        east_passed: 0,
    };

    while collected.deals.len() < args.count && collected.attempts < cap {
        let deal = full_deal(rng);
        collected.attempts += 1;

        // Gate 1: West opens 2♠ per the system.
        if table.next_call(deal[Seat::West], &Auction::new()) != TWO_SPADES {
            continue;
        }
        collected.west_opened += 1;

        // Gate 2: North doubles by at least one swept flavor.
        let north = deal[Seat::North];
        let mut doubled = false;
        for (flavor, fired) in flavors.iter().zip(&mut collected.flavor_fired) {
            if (flavor.holds)(north) {
                *fired += 1;
                doubled = true;
            }
        }
        if !doubled {
            continue;
        }
        collected.north_doubled += 1;

        // Gate 3: East passes over the double per the system.
        let mut auction = Auction::new();
        auction
            .try_extend([TWO_SPADES, Call::Double])
            .expect("the seeded prefix is legal from an empty auction");
        if table.next_call(deal[Seat::East], &auction) != Call::Pass {
            continue;
        }
        collected.east_passed += 1;

        // Gate 4: South's decision is live — the system itself would pass
        // for penalty or bid 3NT, not run to a suit or escape.
        auction.push(Call::Pass);
        let advance = table.next_call(deal[Seat::South], &auction);
        if advance != Call::Pass && advance != THREE_NOTRUMP {
            continue;
        }

        collected.deals.push(deal);
    }
    collected
}

// ---------------------------------------------------------------------------
// Scoring: bid both branches out and price the final contracts
// ---------------------------------------------------------------------------

/// A deal's two outcomes under perfect play, in NS score
#[derive(Clone, Copy)]
struct Outcome {
    /// NS score of the final contract after South passes
    defend: i64,
    /// NS score of the final contract after South bids 3NT
    declare: i64,
}

impl Outcome {
    /// The better of the two for NS
    fn oracle(self) -> i64 {
        self.defend.max(self.declare)
    }

    /// `true` when bidding 3NT outscores passing
    fn prefers_3nt(self) -> bool {
        self.declare > self.defend
    }
}

/// Where the bid-out branches diverged from the nominal contracts
#[derive(Default)]
struct BranchTelemetry {
    /// Defend branches whose final contract is not 2♠× by West (West ran)
    defend_diverged: usize,
    /// Declare branches whose final contract is not 3NT by South
    declare_diverged: usize,
    /// Divergent defend finals, e.g. "3♣ by W" → count
    defend_contracts: HashMap<String, usize>,
    /// Divergent declare finals, e.g. "3NT× by S" → count
    declare_contracts: HashMap<String, usize>,
}

/// One display key per final contract, e.g. "3♣ by W" ("pass-out" never
/// happens here: 2♠ has been bid)
fn contract_key(result: Option<(Contract, Seat)>) -> String {
    result.map_or_else(
        || "pass-out".to_owned(),
        |(contract, declarer)| format!("{contract} by {declarer}"),
    )
}

fn score_deals(
    table: &Table<Stance, Stance>,
    deals: &[FullDeal],
    vulnerability: AbsoluteVulnerability,
) -> (Vec<Outcome>, BranchTelemetry) {
    let tricks = Solver::lock().solve_deals(deals, NonEmptyStrainFlags::ALL);
    let two_sx = Some((
        Contract::new(2, Strain::Spades, Penalty::Doubled),
        Seat::West,
    ));
    let three_nt = Some((
        Contract::new(3, Strain::Notrump, Penalty::Undoubled),
        Seat::South,
    ));
    let mut telemetry = BranchTelemetry::default();

    let outcomes = deals
        .iter()
        .zip(&tricks)
        .map(|(deal, tricks)| {
            let defend = final_contract(
                &table.bid_out_from(deal, seeded_auction(Call::Pass)),
                Seat::West,
            );
            let declare = final_contract(
                &table.bid_out_from(deal, seeded_auction(THREE_NOTRUMP)),
                Seat::West,
            );
            if defend != two_sx {
                telemetry.defend_diverged += 1;
                *telemetry
                    .defend_contracts
                    .entry(contract_key(defend))
                    .or_default() += 1;
            }
            if declare != three_nt {
                telemetry.declare_diverged += 1;
                *telemetry
                    .declare_contracts
                    .entry(contract_key(declare))
                    .or_default() += 1;
            }
            Outcome {
                defend: ns_score(defend, tricks, vulnerability),
                declare: ns_score(declare, tricks, vulnerability),
            }
        })
        .collect();
    (outcomes, telemetry)
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

#[allow(clippy::cast_precision_loss)]
fn mean(values: impl Iterator<Item = i64>) -> f64 {
    let (sum, n) = values.fold((0i64, 0usize), |(sum, n), v| (sum + v, n + 1));
    if n == 0 { 0.0 } else { sum as f64 / n as f64 }
}

#[allow(clippy::cast_precision_loss)]
fn percent(part: usize, whole: usize) -> f64 {
    100.0 * part as f64 / whole.max(1) as f64
}

/// The funnel: how many sampled deals survived each gate
fn print_funnel(collected: &Collected) {
    println!(
        "Funnel: {attempts} attempts -> {west} West 2S -> {north} N double \
         -> {east} E pass -> {live} S live (accepted)\n",
        attempts = collected.attempts,
        west = collected.west_opened,
        north = collected.north_doubled,
        east = collected.east_passed,
        live = collected.deals.len(),
    );
}

/// Per-flavor population stats: how favorable is the live doubled situation?
fn print_double_sweep(flavors: &[Flavor], collected: &Collected, scores: &[Outcome]) {
    let deals = &collected.deals;
    println!(
        "=== Takeout-double flavors (West opened 2S {}x; live population stats) ===",
        collected.west_opened,
    );
    println!(
        "  {:<26}{:>6}{:>8}{:>8}{:>10}{:>10}{:>10}",
        "Flavor", "n", "of 2S", "3NT%", "defend", "declare", "oracle",
    );
    for (flavor, &fired) in flavors.iter().zip(&collected.flavor_fired) {
        let members: Vec<usize> = (0..deals.len())
            .filter(|&i| (flavor.holds)(deals[i][Seat::North]))
            .collect();
        let n = members.len();
        if n == 0 {
            println!("  {:<26}{:>6}{:>8}", flavor.name, 0, "--");
            continue;
        }
        let of_2s = percent(fired, collected.west_opened);
        let pct_3nt = percent(
            members.iter().filter(|&&i| scores[i].prefers_3nt()).count(),
            n,
        );
        let defend = mean(members.iter().map(|&i| scores[i].defend));
        let declare = mean(members.iter().map(|&i| scores[i].declare));
        let oracle = mean(members.iter().map(|&i| scores[i].oracle()));
        println!(
            "  {:<26}{:>6}{:>7.1}%{:>7.0}%{:>+10.0}{:>+10.0}{:>+10.0}",
            flavor.name, n, of_2s, pct_3nt, defend, declare, oracle,
        );
    }
}

/// Where the bid-out branches left the nominal contracts
fn print_branch_telemetry(telemetry: &BranchTelemetry, n: usize) {
    let top = |contracts: &HashMap<String, usize>| {
        let mut sorted: Vec<(&String, &usize)> = contracts.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        sorted
            .into_iter()
            .take(3)
            .map(|(key, count)| format!("{key} ({count}x)"))
            .collect::<Vec<_>>()
            .join(", ")
    };

    println!("\n=== Bid-out divergence (the auction did not stop where seeded) ===");
    println!(
        "  After Pass: {:>4} of {n} ({:>3.0}%) ended off 2Sx by W{}",
        telemetry.defend_diverged,
        percent(telemetry.defend_diverged, n),
        if telemetry.defend_diverged == 0 {
            String::new()
        } else {
            format!(": {}", top(&telemetry.defend_contracts))
        },
    );
    println!(
        "  After 3NT:  {:>4} of {n} ({:>3.0}%) ended off 3NT by S{}",
        telemetry.declare_diverged,
        percent(telemetry.declare_diverged, n),
        if telemetry.declare_diverged == 0 {
            String::new()
        } else {
            format!(": {}", top(&telemetry.declare_contracts))
        },
    );
}

/// Response policies over the full live population, scored by regret
#[allow(clippy::cast_precision_loss)]
fn print_response_sweep(policies: &[Flavor], deals: &[FullDeal], scores: &[Outcome]) {
    let n = deals.len() as f64;
    let oracle = mean(scores.iter().map(|o| o.oracle()));
    println!(
        "\n=== South advance policies (live population, n={}) ===",
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
    let table = build_table(args.vulnerability);
    let doubles = double_flavors();
    let responses = response_flavors();

    let collected = collect_deals(&args, &table, &doubles, &mut rng);
    if collected.deals.is_empty() {
        anyhow::bail!(
            "no deals survived the (2S) X (P) funnel in {} attempts; \
             try raising --max-attempts-per-deal",
            collected.attempts,
        );
    }

    let (scores, telemetry) = score_deals(&table, &collected.deals, args.vulnerability);
    println!(
        "Sample: {got}/{target} live doubled deals\n",
        got = collected.deals.len(),
        target = args.count,
    );
    print_funnel(&collected);
    print_double_sweep(&doubles, &collected, &scores);
    print_branch_telemetry(&telemetry, collected.deals.len());
    print_response_sweep(&responses, &collected.deals, &scores);
    Ok(())
}
