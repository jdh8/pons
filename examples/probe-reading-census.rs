//! Which calls read as **nothing**, weighted by how often they fire?
//!
//! `set_blind_inference` priced the whole reading programme by deleting every
//! reading: −0.65 … −1.27 IMPs/board, about the entire remaining gap to BBA
//! ([ai-bidder/sampled-projection.md]). That is the *ceiling*. This probe is
//! the census that says how much of it is still on the table — the one the
//! design asks for before any further build.
//!
//! The surface is exactly what the nets are handed. `features::push_inference`
//! gives each seat ten floats: four suit lengths and `points`, as `{min, max}`
//! endpoints of the seat's *announced* envelope. An axis is **⊤** when it is
//! still at its `Envelope::unknown` value (`0..=13`, `0..=37`) — the call said
//! nothing about it. Counting ⊤s on this surface keeps the census
//! commensurable with the negative control, which moved the same floats.
//!
//! Two numbers matter, and they say different things:
//!
//! - **⊤ axes per hidden seat**, split by whether the seat has actually *bid*.
//!   A seat that has only passed reads ⊤ unless the pass bands fire, so the
//!   split separates missing readings from absent information.
//! - **blind %** — readings that are ⊤ on all five axes *despite* the seat
//!   having bid. Those calls contributed no projection at all, which is the
//!   signature of the layers the `authored_calls_read_what_they_gate` meter
//!   cannot see: the `Fallback::classify` competitive layer and the instinct
//!   floor, whose milestone gates are `pred`s and project ⊤ by construction.
//!
//! Every decision node counts once over real self-play auctions, so firing
//! frequency *is* the weight — no separate reach pass. Attribution charges a
//! seat's ⊤s to that seat's last call, keyed by the auction through it with
//! leading passes stripped, which is directly greppable against
//! `examples/render-book`.
//!
//! No double-dummy, no solver.
//!
//! ```sh
//! cargo run --release --example probe-reading-census -- -c 5000
//! ```
//!
//! [ai-bidder/sampled-projection.md]: ../docs/ai-bidder/sampled-projection.md

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, Seat, Suit};
use pons::american;
use pons::bidding::context::relative;
use pons::bidding::{Context, Envelope, Inferences, Range, Relative, Stance};
use rayon::prelude::*;
use std::collections::HashMap;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_out, seat_to_act, seeded_deals};

/// The axes the nets read per seat: four suit lengths and `points`
const AXES: u32 = 5;

#[derive(Parser)]
struct Args {
    /// Deals to bid
    #[arg(short, long, default_value = "5000")]
    count: usize,

    /// Seed base; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,

    /// Keys to list in the ranked worklist
    #[arg(long, default_value = "30")]
    top: usize,
}

/// How many of the five axes the nets read carry no information
fn top_axes(shown: &Envelope) -> u32 {
    let mut top = u32::from(shown.strength.points == Range::FULL_POINTS);
    for suit in Suit::ASC {
        top += u32::from(shown.length(suit) == Range::FULL_LENGTH);
    }
    top
}

/// One display key per decision: leading passes stripped, calls joined
///
/// Stripping merges the four dealer rotations of one decision into one line —
/// leading passes only encode the seat, which the books already fan over.
fn auction_key(auction: &[Call]) -> String {
    let key = auction
        .iter()
        .skip_while(|&&call| call == Call::Pass)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    if key.is_empty() {
        "(opening passes)".to_owned()
    } else {
        key
    }
}

/// Seat-readings observed, ⊤ axes summed over them, and how many were ⊤ on all
/// five
#[derive(Default, Clone, Copy)]
struct Cell {
    readings: u64,
    top: u64,
    blind: u64,
}

impl Cell {
    fn add(&mut self, top: u32) {
        self.readings += 1;
        self.top += u64::from(top);
        self.blind += u64::from(top == AXES);
    }

    fn merge(&mut self, other: Self) {
        self.readings += other.readings;
        self.top += other.top;
        self.blind += other.blind;
    }

    /// ⊤ axes per reading, of [`AXES`]
    fn per_reading(self) -> f64 {
        if self.readings == 0 {
            return 0.0;
        }
        self.top as f64 / self.readings as f64
    }

    fn blind_pct(self) -> f64 {
        if self.readings == 0 {
            return 0.0;
        }
        100.0 * self.blind as f64 / self.readings as f64
    }
}

#[derive(Default)]
struct Census {
    /// Per-key tallies, charged to the read seat's last call
    keys: HashMap<String, Cell>,
    /// Seats that have bid — the readings the programme can improve
    bid: Cell,
    /// Seats that have only passed
    passed: Cell,
    /// The same readings taken through a bare `Context::new`, which skips every
    /// projection-based reading silently.  The self-check below leans on it.
    bare: Cell,
}

impl Census {
    fn merge(mut self, other: Self) -> Self {
        for (key, cell) in other.keys {
            self.keys.entry(key).or_default().merge(cell);
        }
        self.bid.merge(other.bid);
        self.passed.merge(other.passed);
        self.bare.merge(other.bare);
        self
    }
}

/// Census every decision node of one bid-out auction
fn census_auction(
    stance: &Stance,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    auction: &[Call],
) -> Census {
    let mut out = Census::default();

    for cut in 1..auction.len() {
        let rel = relative(vul, seat_to_act(dealer, cut));
        let prefix = &auction[..cut];
        let read = stance.infer(rel, prefix);
        // The control: the same auction without the trie prefixes `Stance`
        // attaches, so every projection-based reading is skipped.
        let bare = Inferences::read(&Context::new(rel, prefix));

        // A hidden seat's own calls sit `back` before the actor and every four
        // before that (`Relative`'s parity, counted back from the reader).
        for (who, back) in [
            (Relative::Rho, 1_usize),
            (Relative::Partner, 2),
            (Relative::Lho, 3),
        ] {
            let Some(last) = cut.checked_sub(back) else {
                continue; // the seat has not called yet
            };
            let top = top_axes(read.announced(who));
            if (0..=last)
                .rev()
                .step_by(4)
                .any(|i| auction[i] != Call::Pass)
            {
                out.bid.add(top);
            } else {
                out.passed.add(top);
            }
            out.keys
                .entry(auction_key(&auction[..=last]))
                .or_default()
                .add(top);
            out.bare.add(top_axes(bare.announced(who)));
        }
    }
    out
}

fn main() {
    let args = Args::parse();
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = AbsoluteVulnerability::NONE;
    let stance = american().against();

    let census = seeded_deals(base, args.count)
        .par_iter()
        .enumerate()
        .map(|(board, deal)| {
            let dealer = Seat::ALL[board % 4];
            let auction = bid_out(&stance, &stance, true, dealer, vul, deal);
            census_auction(&stance, dealer, vul, &auction)
        })
        .reduce(Census::default, Census::merge);

    let mut all = census.bid;
    all.merge(census.passed);

    // Self-check: a bare context reads *less* than the bidder does, so it can
    // only find more ⊤.  If this trips, the prefixed read path is not wired and
    // the census would report zero headroom for entirely the wrong reason.
    assert!(
        census.bare.top >= all.top,
        "bare context read tighter than Stance::infer ({} vs {} ⊤ axes) — read path broken",
        census.bare.top,
        all.top,
    );

    println!("boards {}  seed {base}", args.count);
    println!(
        "hidden-seat readings {}  (⊤ of {AXES} axes; bare-context control {:.3})\n",
        all.readings,
        census.bare.per_reading(),
    );

    println!(
        "{:<12} {:>10} {:>8} {:>9}",
        "", "readings", "⊤/seat", "blind %"
    );
    for (label, cell) in [("has bid", census.bid), ("passes only", census.passed)] {
        println!(
            "{label:<12} {:>10} {:>8.3} {:>8.2}%",
            cell.readings,
            cell.per_reading(),
            cell.blind_pct(),
        );
    }

    // Two rankings, because they answer different questions.  Total ⊤ mass is
    // dominated by calls that *cannot* say more — an opening pass constrains no
    // suit, so its four ⊤ length axes are exact rather than a leak.  Readings
    // that are ⊤ on all five axes are the ones where the call contributed no
    // projection at all, and that is the worklist.
    let mut keys: Vec<_> = census.keys.into_iter().collect();
    let n = keys.len();
    for (label, rank) in [
        ("total ⊤ axes", (|c: &Cell| c.top) as fn(&Cell) -> u64),
        ("fully-blind readings", |c: &Cell| c.blind),
    ] {
        keys.sort_unstable_by(|a, b| rank(&b.1).cmp(&rank(&a.1)).then_with(|| a.0.cmp(&b.0)));
        println!(
            "\ntop {} keys by {label} (of {n} distinct)\n",
            args.top.min(n),
        );
        println!(
            "{:>10} {:>10} {:>8} {:>9} {:>8}  key",
            "⊤ axes", "readings", "⊤/seat", "blind %", "blind",
        );
        for (key, cell) in keys.iter().take(args.top) {
            println!(
                "{:>10} {:>10} {:>8.3} {:>8.2}% {:>8}  {key}",
                cell.top,
                cell.readings,
                cell.per_reading(),
                cell.blind_pct(),
                cell.blind,
            );
        }
    }
}
