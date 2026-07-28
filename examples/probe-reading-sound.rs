//! Does a reading of a **hidden** seat actually contain that seat's hand?
//!
//! [`Inferences::read`]'s module doc promises soundness over tightness: *"a hand
//! that actually made these calls always falls within every shown range"*. The
//! same doc then says the meanings encoded are [`american`]'s. Both halves are
//! load-bearing, and together they leave a hole: at our own two seats the
//! guarantee is *derived* — partner really did bid our system — but LHO and RHO
//! are bidding something else, so for them it is merely *assumed*.
//!
//! Every consumer of a reading rests on that assumption. The evaluator net is
//! handed each hidden seat's box as features; the sampler rejects worlds outside
//! it. A box that excludes the truth is not a loose prior, it is a wrong one,
//! and no amount of extra columns describing it helps.
//!
//! So: bid a mixed table with BBA at the opponent seats, and at every decision
//! node of *our* seats test `announced_dnf(who).contains(true_hand)`.
//!
//! - **partner ≈ 0%** is the self-check. The membership test and the harness
//!   are only trustworthy if our own side comes out clean; a material rate there
//!   is a bug in the reading, not a finding about opponents.
//! - **LHO / RHO** is the measurement. Whatever it is above partner's rate is
//!   the price of reading a foreign system with our own meanings.
//!
//! Note [`features::shape_of`] already falls back when a reading admits *zero*
//! shapes, so total exclusion is survivable; a box that is merely wrong is not
//! caught anywhere, which is what this counts.
//!
//! No double-dummy, no solver. EPBot is not thread-safe here, so this is serial.
//!
//! ```sh
//! cargo run --release --example probe-reading-sound -- -c 2000
//! ```
//!
//! [`american`]: pons::american

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use pons::american;
use pons::bidding::context::relative;
use pons::bidding::{Relative, Stance};
use std::collections::HashMap;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::oracle::{BbaOracle, ConventionCard, DEFAULT_LIB, SYSTEM_2_OVER_1, bid_out};
use common::{auction_key, seat_to_act, seeded_deals};

/// The hidden seats, in report order, with how far back each one last acted
const HIDDEN: [(&str, Relative, usize); 3] = [
    ("LHO (BBA)", Relative::Lho, 3),
    ("partner (ours)", Relative::Partner, 2),
    ("RHO (BBA)", Relative::Rho, 1),
];

/// One-letter tag per hidden seat, prefixed onto a worklist key so a hot spot
/// says *whose* call was misread as well as which
const TAG: [&str; 3] = ["L", "P", "R"];

#[derive(Parser)]
struct Args {
    /// Deals to bid
    #[arg(short, long, default_value = "2000")]
    count: usize,

    /// Seed base; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,

    /// Offending keys to list
    #[arg(long, default_value = "20")]
    top: usize,

    /// Do not disclose our card to BBA (the `bba-gen --disclose off` arm)
    #[arg(long)]
    no_disclose: bool,
}

/// Readings taken, and how many excluded the truth
#[derive(Default, Clone, Copy)]
struct Cell {
    readings: u64,
    bad: u64,
}

impl Cell {
    fn add(&mut self, sound: bool) {
        self.readings += 1;
        self.bad += u64::from(!sound);
    }

    fn pct(self) -> f64 {
        if self.readings == 0 {
            return 0.0;
        }
        100.0 * self.bad as f64 / self.readings as f64
    }
}

/// Our generated card, so BBA reads our calls the way `bba-gen` has it read them
fn our_card() -> ConventionCard {
    let card = pons::bidding::card::american_card();
    ConventionCard {
        system: card.system,
        toggles: card
            .rows
            .iter()
            .map(|(name, value)| {
                (
                    std::ffi::CString::new(*name).expect("a card row name has no NUL"),
                    *value,
                )
            })
            .collect(),
    }
}

/// Every decision node of one bid-out deal, charged into `seats` and `keys`
fn census(
    stance: &Stance,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
    auction: &[Call],
    seats: &mut [Cell; 3],
    keys: &mut HashMap<String, Cell>,
) {
    for cut in 1..auction.len() {
        let seat = seat_to_act(dealer, cut);
        // Only our own seats: a reading taken at an EW seat is one no consumer
        // in the crate ever takes.
        if !matches!(seat, Seat::North | Seat::South) {
            continue;
        }
        let read = stance.infer(relative(vul, seat), &auction[..cut]);
        for (slot, (_, who, back)) in HIDDEN.into_iter().enumerate() {
            let Some(last) = cut.checked_sub(back) else {
                continue; // the seat has not called yet
            };
            // `back` calls ago is `back` seats counter-clockwise from the actor.
            let hand = deal[Seat::ALL[(seat as usize + 4 - back) % 4]];
            let sound = read.announced_dnf(who).contains(hand);
            seats[slot].add(sound);
            keys.entry(format!("{} {}", TAG[slot], auction_key(&auction[..=last])))
                .or_default()
                .add(sound);
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = AbsoluteVulnerability::NONE;
    let stance = american().against();

    let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    let bba = BbaOracle::load(&path, SYSTEM_2_OVER_1, Vec::new())
        .map_err(|error| {
            anyhow::anyhow!(
                "could not load EPBot native lib at `{path}`: {error}\n\
                 Fetch it with `git submodule update --init vendor/bba`, or set BBA_LIB."
            )
        })?
        .with_opponents((!args.no_disclose).then(our_card));

    let mut seats = [Cell::default(); 3];
    let mut keys: HashMap<String, Cell> = HashMap::new();
    for (board, deal) in seeded_deals(base, args.count).iter().enumerate() {
        let dealer = Seat::ALL[board % 4];
        let auction = bid_out(&stance, &bba, true, dealer, vul, deal);
        census(&stance, dealer, vul, deal, &auction, &mut seats, &mut keys);
    }

    println!("boards {}  seed {base}", args.count);
    println!(
        "disclosure {}\n",
        if args.no_disclose { "off" } else { "generated" },
    );
    println!(
        "{:<16} {:>10} {:>10} {:>10}",
        "seat", "readings", "excluded", "%"
    );
    for (slot, (label, _, _)) in HIDDEN.into_iter().enumerate() {
        let cell = seats[slot];
        println!(
            "{label:<16} {:>10} {:>10} {:>9.3}%",
            cell.readings,
            cell.bad,
            cell.pct(),
        );
    }

    let mut keys: Vec<_> = keys.into_iter().collect();
    let n = keys.len();
    keys.sort_unstable_by(|a, b| b.1.bad.cmp(&a.1.bad).then_with(|| a.0.cmp(&b.0)));
    println!(
        "\ntop {} keys by excluded readings (of {n} distinct)\n",
        args.top.min(n),
    );
    println!("{:>10} {:>10} {:>9}  key", "excluded", "readings", "%");
    for (key, cell) in keys.iter().take(args.top) {
        println!(
            "{:>10} {:>10} {:>8.2}%  {key}",
            cell.bad,
            cell.readings,
            cell.pct(),
        );
    }
    Ok(())
}
