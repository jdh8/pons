//! What does BBA infer about *our* 1NT, and which card rows control it?
//!
//! `cards/American.bbsa` carries `1NT opening natural = 0` beside
//! `1NT opening NT style = 1`.  Read literally the first is wrong — our 1NT is a
//! natural 15-17 — and both move 16 decisions in the disclosure sweep
//! (docs/ai-bidder/bba-disclosure-sweep.md), so the pair is load-bearing.
//!
//! But `probe-bba-constraints --mode open` shows BBA's *own* 1NT opening is
//! byte-identical across all four settings (15-17, same hand count).  So these
//! rows govern **interpretation**, not bidding: what EPBot deduces when someone
//! else opens 1NT.  That is exactly what disclosure is for, and it is readable —
//! `BbaOracle::probe` returns the bilans state, whose `seats[0]` is the public
//! band deduced for the opener.
//!
//! So: seat our balanced 15-17 hands as the opener, put BBA on lead over `1NT`,
//! and read the HCP band it deduces under each of the four settings applied to
//! *our* seats.  The setting whose band matches 15-17 is the honest declaration.
//!
//! ```text
//! cargo run --release --example probe-bba-nt-reading
//! ```

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, Bid, Hand, Seat, Strain};
use pons::bidding::context::relative;
use std::ffi::c_int;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::oracle::{BbaOracle, DEFAULT_LIB, EpbotCard, load_bbsa};
use common::{hand_hcp, seeded_deals};

/// Read BBA's deduced band for a disclosed 1NT opening under each card setting
#[derive(Parser)]
struct Args {
    /// Deals to draw balanced 15-17 openers from
    #[arg(long, default_value_t = 20000)]
    deals: usize,

    /// Cap on the opener hands actually probed
    #[arg(long, default_value_t = 300)]
    hands: usize,

    /// Card supplying the rest of the disclosure (its two 1NT rows are overridden)
    #[arg(long, default_value = "cards/American.bbsa")]
    card: String,

    #[arg(long, default_value_t = 0)]
    seed: u64,
}

/// Our 1NT opening shape: balanced 15-17, the range the card declares
fn is_our_1nt(hand: Hand) -> bool {
    let mut lengths: Vec<usize> = contract_bridge::Suit::ASC
        .iter()
        .map(|&suit| hand[suit].len())
        .collect();
    lengths.sort_unstable();
    let balanced = matches!(
        lengths.as_slice(),
        [2, 3, 3, 5] | [2, 3, 4, 4] | [3, 3, 3, 4]
    );
    balanced && (15..=17).contains(&hand_hcp(hand))
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let lib = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    let card = load_bbsa(&args.card)?;
    let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));

    // Openers sit at canonical seat 0; BBA acts at seat 1, over their 1NT.
    let openers: Vec<Hand> = seeded_deals(args.seed, args.deals)
        .into_iter()
        .flat_map(|deal| Seat::ALL.map(|seat| deal[seat]))
        .filter(|&hand| is_our_1nt(hand))
        .take(args.hands)
        .collect();
    anyhow::ensure!(!openers.is_empty(), "no balanced 15-17 hands in the sample");
    println!(
        "{} balanced 15-17 openers; BBA reads position 0 after (1NT)\n",
        openers.len()
    );

    let mut bba = BbaOracle::load(&lib, card.system, Vec::new())?;
    println!(
        "{:>8} {:>6}  {:>12}  {:<24}  variety",
        "natural", "style", "median hcp", "median C D H S length"
    );
    for natural in [0, 1] {
        for style in [0, 1] {
            // The rest of the card rides along, so the two rows are read in the
            // context they would actually ship in.
            let mut toggles = card.toggles.clone();
            for (name, value) in &mut toggles {
                match name.to_bytes() {
                    b"1NT opening natural" => *value = natural,
                    b"1NT opening NT style" => *value = style,
                    _ => {}
                }
            }
            bba = bba.with_opponents(Some(EpbotCard {
                system: card.system,
                toggles,
            }));

            type Read = ((c_int, c_int), [c_int; 4], [c_int; 4]);
            let mut bands: Vec<Read> = openers
                .iter()
                .filter_map(|&hand| {
                    // BBA holds seat 1's hand; only its *read* of seat 0 matters,
                    // so any consistent hand for it does — reuse the opener's.
                    let state = bba.probe(
                        hand,
                        relative(AbsoluteVulnerability::NONE, Seat::East),
                        &[one_nt],
                    )?;
                    let seat = &state.seats[0];
                    // Shape too: the HCP band may be carried by the `1NT opening
                    // range *` rows alone, leaving these two to govern shape.
                    Some((seat.hcp_range(), seat.min_length, seat.max_length))
                })
                .collect();
            anyhow::ensure!(!bands.is_empty(), "EPBot returned no state");
            bands.sort_unstable();
            let (hcp, min, max) = bands[bands.len() / 2];
            let mut distinct: Vec<Read> = bands.clone();
            distinct.dedup();
            // C, D, H, S — EPBot's suit order throughout the info arrays.
            let shape: Vec<String> = (0..4).map(|i| format!("{}-{}", min[i], max[i])).collect();
            println!(
                "{natural:>8} {style:>6}  {:>12}  {:<24}  {} distinct reads",
                format!("{}-{}", hcp.0, hcp.1),
                shape.join(" "),
                distinct.len(),
            );
        }
    }
    println!(
        "\nThe honest setting is the one whose band matches our declared 15-17.\n\
         A band of 0-37 means BBA deduced nothing — the 1NT read as unlimited."
    );
    Ok(())
}
