//! One worklist key, replayed: which call at this node excludes its own bidder?
//!
//! `probe-reading-sound` ranks *nodes* — an auction through partner's call and
//! how often partner's reading of it failed to contain partner's hand. That is
//! a scalar per node; the repair needs the witness. This replays the seat that
//! made the key's last call over seeded hands (its earlier calls in the script
//! must be the ones it actually chooses — the `readings_admit_the_bidder`
//! route filter), then reads the call back and reports, per chosen call, how
//! often the reading excludes the hand and **which axis** does the excluding.
//!
//! ```sh
//! cargo run --release --example probe-admit-node -- "2♥ - 2NT - 3♣" "1♠ - - X"
//! ```
//!
//! The reading is taken exactly as the sweep takes it — the key's calls, then
//! the bidder's call, then one pass, from the bidder's partner's seat, under
//! default knobs. Membership is `Envelope::admits`: four suit lengths and the
//! `points` gauge (raw HCP and the support/suit-HCP bands have no teeth until
//! `gauge_membership`), so those are the axes printed.

use clap::Parser;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Hand, Seat, Suit};
use pons::american;
use pons::bidding::inference::Envelope;
use pons::bidding::{Partnership, Relative};
use std::collections::BTreeMap;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::seeded_deals;

#[derive(Parser)]
struct Args {
    /// Worklist keys, e.g. `"2♥ - 2NT - 3♣"`; the last call of each is the one
    /// whose reading is tested. `-`/`P` is a pass, parentheses are ignored.
    auctions: Vec<String>,

    /// Hands to replay per key
    #[arg(short, long, default_value = "4000")]
    count: usize,

    /// Seed for the replayed hands
    #[arg(short, long, default_value = "20260824")]
    seed: u64,

    /// Witnesses to print per excluded call
    #[arg(long, default_value = "3")]
    witnesses: usize,
}

/// A box's axes as the membership test sees them, marking the ones `hand` fails
fn render(shown: &Envelope, hand: Option<Hand>) -> String {
    let points = hand.map(pons::bidding::constraint::point_count);
    let bad = |ok: bool| if ok { "" } else { " ✗" };
    let mut out = format!(
        "points {:?}{}",
        shown.strength.points,
        bad(points.is_none_or(|p| shown.strength.points.contains(p))),
    );
    for suit in Suit::ASC {
        // SAFETY: a suit length is at most 13, so the cast cannot truncate.
        #[allow(clippy::cast_possible_truncation)]
        let length = hand.map(|hand| hand[suit].len() as u8);
        out += &format!(
            "  {suit} {:?}{}",
            shown.length(suit),
            bad(length.is_none_or(|n| shown.length(suit).contains(n))),
        );
    }
    out
}

/// The hand's own shape and count, for reading against the box above
fn describe(hand: Hand) -> String {
    let shape = Suit::DESC
        .into_iter()
        .map(|suit| hand[suit].len().to_string())
        .collect::<Vec<_>>()
        .join("-");
    format!(
        "{hand}  {shape}  points {}",
        pons::bidding::constraint::point_count(hand),
    )
}

/// The system's own choice at `auction` — the highest finite logit, book and
/// floor together (the in-crate `chosen_call` of the admits sweep)
fn chosen_call(partnership: &Partnership, hand: Hand, auction: &[Call]) -> Call {
    let Some((logits, _)) =
        partnership.classify_with_provenance(hand, RelativeVulnerability::NONE, auction)
    else {
        return Call::Pass;
    };
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map_or(Call::Pass, |(call, _)| call)
}

/// Every hand's verdict at one key, tallied by the call it actually chose
fn replay(partnership: &Partnership, hands: &[Hand], prefix: &[Call]) -> BTreeMap<String, Tally> {
    let mut tally: BTreeMap<String, Tally> = BTreeMap::new();
    for &hand in hands {
        // Honest route: the seat's own earlier calls in the script must be the
        // ones this hand chooses, so the reading is tested against hands that
        // actually bid the lane.
        if (prefix.len() % 4..prefix.len())
            .step_by(4)
            .any(|i| chosen_call(partnership, hand, &prefix[..i]) != prefix[i])
        {
            continue;
        }
        let made = chosen_call(partnership, hand, prefix);
        let mut read: Vec<Call> = prefix.to_vec();
        read.push(made);
        read.push(Call::Pass);
        let inferences = partnership.infer(RelativeVulnerability::NONE, &read);
        let cell = tally
            .entry(contract_bridge::auction::display_calls(&[made]).to_string())
            .or_default();
        cell.chosen += 1;
        cell.points
            .push(pons::bidding::constraint::point_count(hand));
        if !inferences.admits(Relative::Partner, hand) {
            cell.excluded += 1;
            cell.witnesses
                .push((hand, *inferences.get(Relative::Partner)));
        } else if cell.admitted.is_none() {
            cell.admitted = Some(*inferences.get(Relative::Partner));
        }
    }
    tally
}

#[derive(Default)]
struct Tally {
    chosen: usize,
    excluded: usize,
    witnesses: Vec<(Hand, Envelope)>,
    admitted: Option<Envelope>,
    /// Every chooser's point count — the "what floor would be sound here"
    /// number a loosening repair needs when it replaces a wrong claim with a
    /// weaker one rather than with nothing.
    points: Vec<u8>,
}

impl Tally {
    /// The observed point floor and its 1st percentile, for reading a sound
    /// `points(n..)` off the population that actually makes the call
    fn floors(&self) -> (u8, u8) {
        let mut points = self.points.clone();
        points.sort_unstable();
        let first = points.first().copied().unwrap_or(0);
        let pct = points.get(points.len() / 100).copied().unwrap_or(first);
        (first, pct)
    }
}

fn main() {
    let args = Args::parse();
    let partnership = american(&pons::bidding::agreements::Agreements::default()).bind();
    let hands: Vec<Hand> = seeded_deals(args.seed, args.count)
        .iter()
        .map(|deal| deal[Seat::North])
        .collect();

    for text in &args.auctions {
        let calls: Vec<Call> = text
            .split_whitespace()
            .map(|token| match token.trim_matches(['(', ')']) {
                "-" | "P" => Call::Pass,
                other => other.parse().expect("a call"),
            })
            .collect();
        let (prefix, last) = calls.split_at(calls.len() - 1);
        let key = contract_bridge::auction::display_calls(last).to_string();
        let tally = replay(&partnership, &hands, prefix);
        let routed: usize = tally.values().map(|cell| cell.chosen).sum();
        println!(
            "\n=== {text}   ({routed} of {} hands take this route)",
            hands.len()
        );
        for (made, cell) in &tally {
            let marker = if *made == key {
                " <- the key's call"
            } else {
                ""
            };
            let (min, pct) = cell.floors();
            println!(
                "  {made:<4} chosen {:>5}   excluded {:>5}  {:>6.2}%   points min {min:>2} p1 {pct:>2}{marker}",
                cell.chosen,
                cell.excluded,
                100.0 * cell.excluded as f64 / cell.chosen as f64,
            );
            if let Some(box_) = &cell.admitted
                && cell.excluded > 0
            {
                println!("        admitted box: {}", render(box_, None));
            }
            for (hand, box_) in cell.witnesses.iter().take(args.witnesses) {
                println!("        {}", describe(*hand));
                println!("        read as       {}", render(box_, Some(*hand)));
            }
        }
    }
}
