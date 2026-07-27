//! Census the instinct floor's 4NT RKCB ask: how often it fires, and what a
//! *static* own-hand floor could announce there.
//!
//! Step 5 of the `announced()` pilot. The ask
//! ([`instinct.rs`](../src/bidding/instinct.rs), the `RKCB_FLOOR` rule) is the
//! one bilans-converted milestone that is not a final contract, and it is the
//! worst reading in the tree: `.alert(RKCB_FLOOR)` suppresses the natural
//! reading, and its gate `slam_entry_reached()` is a whole-`pred`, so the
//! projection is ⊤ — the ask announces **nothing at all**.
//!
//! The pilot wants to announce the arithmetic the net replaced:
//! `FLOOR_SLAM_ENTRY = 29` combined support points. A projection cannot read
//! partner's shown minimum without recursing through `Inferences::read`, so the
//! pilot announces a *static* own-hand floor `support_points(k..)` instead, and
//! `k` has to come from the population. This probe supplies it.
//!
//! Two numbers decide whether the pilot is worth building:
//!
//! - **the firing rate** — if the ask is rare the whole split buys nothing;
//! - **`k` at the reported percentile** — if it lands near 0 the agreement says
//!   nothing and the pilot should move to another site.
//!
//! Reported as percentiles of the *actor's own* support points in the keycard
//! trump at firing nodes, plus the distribution of `29 − partner_min` (what an
//! auction-aware announce could have claimed) for the follow-up's sake.
//!
//! ```sh
//! cargo run --release --example probe-announced-rkcb -- --count 200000
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Bid, Hand, Seat, Strain, Suit};
use pons::american;
use pons::bidding::constraint::{point_count, support_point_count_in};
use pons::bidding::context::relative;
use pons::bidding::inference::set_announced_reading;
use pons::bidding::instinct::set_rkcb_announce;
use pons::bidding::{Inferences, Stance, System};
use rayon::prelude::*;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{seat_to_act, seeded_deals};

/// The floor's combined support-point entry threshold — `FLOOR_SLAM_ENTRY`.
/// Not importable (the constant is private), so it is mirrored here; the probe
/// only uses it to report `29 − partner_min`, never to gate.
const FLOOR_SLAM_ENTRY: u8 = 29;

/// One firing of the ask.
struct Firing {
    /// The actor's own support points in the keycard trump — the axis
    /// `slam_entry_reached`'s own arithmetic counts on.
    own: u8,
    /// The actor's own `point_count` — the axis an announce must ride, because
    /// `features::push_inference` encodes only lengths and `points`. A box on
    /// `support_points` would be invisible to every net that reads the auction.
    own_points: u8,
    /// Partner's shown support floor in that trump, else its shown point floor.
    partner_min: u8,
    /// What the asker's *own* seat already reads as, before the ask is added.
    /// The announce only buys `max(0, k - this)` — an asker who already opened
    /// is floored by the natural walk, and the agreement adds nothing there.
    own_read_floor: u8,
    /// Whether the **floor** authored the ask. A book node with finite mass
    /// shadows the floor, and only the floor's rule carries the `announced()`
    /// wrap — so a book-authored ask is outside the pilot entirely.
    from_floor: bool,
    /// The auction prefix and the asker's hand, for `--dump` (pinning a real
    /// floor-authored node in a unit test).
    witness: Option<(Vec<Call>, Hand)>,
}

/// The highest-logit legal call, defaulting to a pass (the harness rule).
fn argmax_legal(logits: &pons::bidding::array::Logits, auction: &Auction) -> Call {
    let mut scored: Vec<(Call, f32)> = logits
        .iter()
        .map(|(call, &logit)| (call, logit))
        .filter(|&(_, logit)| logit.is_finite())
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
    scored
        .into_iter()
        .map(|(call, _)| call)
        .find(|&call| auction.can_push(call).is_ok())
        .unwrap_or(Call::Pass)
}

/// The trump the ask would keycard for: a suit someone genuinely showed
/// five-plus of, mirroring the ask's own decodability gate. Approximates
/// `keycard_trump` (private) by taking the longest shown combined fit, which
/// agrees with it wherever exactly one suit qualifies.
fn keycard_trump(inferences: &Inferences) -> Option<Suit> {
    Suit::ASC
        .into_iter()
        .filter(|&suit| {
            let me = inferences.me().length(suit).min;
            let partner = inferences.partner().length(suit).min;
            me.max(partner) >= 5
        })
        .max_by_key(|&suit| {
            inferences.me().length(suit).min + inferences.partner().length(suit).min
        })
}

/// Whether the opponents have only passed so far — the ask's `undisturbed()`.
fn undisturbed(auction: &[Call]) -> bool {
    auction
        .iter()
        .enumerate()
        .filter(|(index, _)| (auction.len() - index) % 2 == 1)
        .all(|(_, &call)| call == Call::Pass)
}

/// Partner's last call, two back and every four before that.
fn partner_last_call(auction: &[Call]) -> Option<Bid> {
    auction
        .iter()
        .rev()
        .skip(1)
        .step_by(4)
        .find_map(|&call| match call {
            Call::Bid(bid) => Some(bid),
            _ => None,
        })
}

/// Bid one deal out under `stance` and collect every RKCB-ask firing.
fn walk(
    stance: &Stance,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &contract_bridge::FullDeal,
    out: &mut Vec<Firing>,
) {
    let ask = Call::Bid(Bid::new(4, Strain::Notrump));
    let mut auction = Auction::new();

    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let hand: Hand = deal[seat];
        let rel = relative(vul, seat);

        let (call, from_floor) = match stance.classify_with_provenance(hand, rel, &auction) {
            // `depth == 0 && fallback.is_some()` is the floor firing — the same
            // test `american_floored` uses. A book node shadows the floor, and
            // only the floor's rule carries the `announced()` wrap.
            Some((logits, provenance)) => (
                argmax_legal(&logits, &auction),
                provenance.depth == 0 && provenance.fallback.is_some(),
            ),
            None => (Call::Pass, false),
        };

        if call == ask && undisturbed(&auction) {
            let inferences = stance.infer(rel, &auction);
            // The ask is not quantitative: partner must not have bid notrump.
            let quantitative =
                partner_last_call(&auction).is_some_and(|bid| bid.strain == Strain::Notrump);
            if !quantitative && let Some(trump) = keycard_trump(&inferences) {
                let partner = inferences.partner();
                out.push(Firing {
                    own: support_point_count_in(hand, trump),
                    own_points: point_count(hand),
                    partner_min: partner
                        .strength
                        .support_floor(trump)
                        .unwrap_or(partner.strength.points.min),
                    own_read_floor: inferences.me().strength.points.min,
                    from_floor,
                    witness: (from_floor && inferences.me().strength.points.min == 0)
                        .then(|| (auction.to_vec(), hand)),
                });
            }
        }

        auction.push(call);
    }
}

/// Bid one deal out and return the finished auction — the divergence smoke's
/// half. Same argmax rule as [`walk`], no census.
fn bid_out(
    stance: &Stance,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &contract_bridge::FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let call = match stance.classify(deal[seat], relative(vul, seat), &auction) {
            Some(logits) => argmax_legal(&logits, &auction),
            None => Call::Pass,
        };
        auction.push(call);
    }
    auction
}

/// `values` must be sorted; returns the value at `percent` of the way through.
fn percentile(values: &[u8], percent: usize) -> u8 {
    if values.is_empty() {
        return 0;
    }
    let index = (values.len() - 1) * percent / 100;
    values[index]
}

fn report(label: &str, mut values: Vec<u8>) {
    values.sort_unstable();
    println!("\n{label} (n = {})", values.len());
    if values.is_empty() {
        println!("  no firings — nothing to announce");
        return;
    }
    print!("  ");
    for percent in [0, 5, 10, 25, 50, 75, 100] {
        print!("p{percent}={:<4}", percentile(&values, percent));
    }
    let mean = values.iter().map(|&v| u32::from(v)).sum::<u32>() as f64 / values.len() as f64;
    println!("\n  mean={mean:.2}");
}

/// Census the floor's 4NT RKCB ask for the `announced()` pilot
#[derive(Parser)]
#[command(about = "Census the floor's 4NT RKCB ask: firing rate and announceable floor")]
struct Args {
    /// Number of boards to bid out
    #[arg(short, long, default_value_t = 200_000)]
    count: usize,

    /// Seed base for the deal stream
    #[arg(long, default_value_t = 20_260_726)]
    seed: u64,

    /// Vulnerability (the ask's gate has no vulnerability term; here for the record)
    #[arg(long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,
}

fn main() {
    let args = Args::parse();
    let stance = american().against();
    let deals = seeded_deals(args.seed, args.count);
    eprintln!("announced-rkcb: {} boards", deals.len());

    // Divergence smoke: the same deals bid knob-off and knob-on. The pilot moves
    // no criterion — only what the seats *read* — so every divergent board is a
    // net reacting to a tighter box, and the count is what an A/B would score.
    // `set_announced_reading` is a thread-local, so each rayon worker sets it.
    let diverge = |pilot: bool| -> usize {
        deals
            .par_iter()
            .enumerate()
            .filter(|(index, deal)| {
                let dealer = Seat::ALL[index % 4];
                set_announced_reading(false);
                let off = bid_out(&stance, dealer, args.vulnerability, deal);
                set_announced_reading(true);
                set_rkcb_announce(pilot);
                let on = bid_out(&stance, dealer, args.vulnerability, deal);
                set_announced_reading(false);
                set_rkcb_announce(true);
                *off != *on
            })
            .count()
    };
    let (bare, diverged) = (diverge(false), diverge(true));

    let firings: Vec<Firing> = deals
        .par_iter()
        .enumerate()
        .flat_map_iter(|(index, deal)| {
            let dealer = Seat::ALL[index % 4];
            let mut out = Vec::new();
            walk(&stance, dealer, args.vulnerability, deal, &mut out);
            out
        })
        .collect();

    println!("boards          {}", args.count);
    println!(
        "divergence     alerted-only union alone: {bare} ({:.3}%)  |  + RKCB pilot: {diverged} ({:.3}%)",
        100.0 * bare as f64 / args.count as f64,
        100.0 * diverged as f64 / args.count as f64
    );
    println!("ask firings     {}", firings.len());
    println!(
        "firing rate     {:.3}% of boards",
        100.0 * firings.len() as f64 / args.count as f64
    );

    // Only the floor's rule carries the `announced()` wrap; a book-authored ask
    // shadows it entirely. This split is the pilot's real reach.
    let floored: Vec<&Firing> = firings.iter().filter(|f| f.from_floor).collect();
    println!(
        "FLOOR-authored  {} ({:.1}% of asks, {:.3}% of boards) <- the pilot's reach",
        floored.len(),
        100.0 * floored.len() as f64 / firings.len().max(1) as f64,
        100.0 * floored.len() as f64 / args.count as f64,
    );

    // Floor-authored asks where the asker's own seat reads *nothing* yet — the
    // cases the agreement buys the most, and the ones worth pinning in a test.
    for firing in firings.iter().filter_map(|f| f.witness.as_ref()).take(5) {
        println!("\n  witness: {:?}\n    hand: {}", firing.0, firing.1);
    }

    report(
        "own support points in the keycard trump",
        firings.iter().map(|f| f.own).collect(),
    );
    report(
        "own point_count (THE ANNOUNCEABLE AXIS — what push_inference encodes)",
        firings.iter().map(|f| f.own_points).collect(),
    );
    report(
        "partner's shown floor",
        firings.iter().map(|f| f.partner_min).collect(),
    );
    report(
        "29 - partner_min (what an auction-aware announce could claim)",
        firings
            .iter()
            .map(|f| FLOOR_SLAM_ENTRY.saturating_sub(f.partner_min))
            .collect(),
    );

    // The pilot's answer: the largest static floor that still covers 90% / 95%
    // of firings. This is an *agreement*, not a bound — the net fires below it
    // by design, which is exactly why it rides `announce` and not `project`.
    let mut own: Vec<u8> = firings.iter().map(|f| f.own_points).collect();
    own.sort_unstable();
    report(
        "asker's own seat floor ALREADY read before the ask",
        firings.iter().map(|f| f.own_read_floor).collect(),
    );

    // The pilot only buys `max(0, k - already_read)`. An asker who opened is
    // floored by the natural walk, so the agreement adds nothing there; the
    // announce bites on askers who have shown little — a responder, a passed
    // hand, an auction the walk reads loosely.
    for k in [11u8, 12, 14, 16] {
        let bites = firings.iter().filter(|f| k > f.own_read_floor).count();
        let gained: u32 = firings
            .iter()
            .map(|f| u32::from(k.saturating_sub(f.own_read_floor)))
            .sum();
        println!(
            "k = {k:<3} bites on {bites:>5} / {} firings ({:.1}%), mean points gained {:.2}",
            firings.len(),
            100.0 * bites as f64 / firings.len() as f64,
            f64::from(gained) / firings.len() as f64,
        );
    }

    println!("\nstatic k on the `points` axis (the pilot's announce box):");
    println!(
        "  points(k..) covering 90% of firings: k = {}",
        percentile(&own, 10)
    );
    println!(
        "  points(k..) covering 95% of firings: k = {}",
        percentile(&own, 5)
    );
    println!(
        "  points(k..) covering 99% of firings: k = {}",
        percentile(&own, 1)
    );
}
