//! Frequency gate for the v5 card-manifold axis probe: how often does flipping
//! each card-expressible knob move a default-system auction?
//!
//! Gate 3 of `docs/ai-bidder/card-manifold.md` §"Axis selection": a bit that
//! decides ~0.05% of boards leaves its weight near initialisation, so the v5
//! corpus should thaw high-frequency axes first.  This ranks every candidate —
//! the ~16 knobs behind the 24 live `american_row` arms, minus `Kickback 1430`
//! (already trained) and `Two way game tries` (honest value is
//! knob-independent) — by bidding the same seeded deals under the shipped
//! defaults and under the flip, all four seats our system, and counting
//! differing auctions.  Gate 1's book half falls out for free (a knob that
//! moves no auction fails truthfulness); gate 2 (EPBot stickiness) is enforced
//! at dump time by `verify_card`.
//!
//! One rayon task per arm.  Each starts from `Agreements::default()`, applies
//! its own flip to that value, and builds a stance whose classify-time state is
//! pinned independently of the worker thread.
//!
//! ```text
//! cargo run --release --example probe-card-axes -- --count 20000 --seed 1
//! ```

use clap::Parser;
use contract_bridge::auction::Auction;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use pons::american;
use pons::bidding::agreements::Agreements;
use pons::bidding::american::{EUROPEAN, LebensohlStyle, NotrumpDefense, NotrumpShape, PUPPET};
use rayon::prelude::*;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{next_call, seat_to_act, seeded_deals};

#[derive(Parser)]
struct Args {
    /// Number of boards bid per axis (dealer and vulnerability rotate)
    #[arg(short, long, default_value = "20000")]
    count: usize,

    /// Deal seed base (board i seeded base+i)
    #[arg(long, default_value = "1")]
    seed: u64,
}

/// A knob flip away from the shipped defaults
type Flip = fn(&mut Agreements);

/// Every probed axis: the card-block name(s) it moves, and the flip away from
/// the shipped default.  Radio groups (one knob, several rows) probe as one.
const AXES: [(&str, Flip); 16] = [
    ("Garbage Stayman", |a| {
        a.decision.reading.garbage_stayman = !a.decision.reading.garbage_stayman
    }),
    ("Checkback (NMF)", |a| {
        a.rebid.new_minor_forcing = !a.rebid.new_minor_forcing;
    }),
    ("Two Way NMF (XYZ)", |a| {
        a.decision.reading.xyz = !a.decision.reading.xyz
    }),
    ("Super acceptance", |a| {
        a.notrump.transfer_super_accept = !a.notrump.transfer_super_accept;
    }),
    ("Fourth suit forcing", |a| {
        a.rebid.fourth_suit_forcing = !a.rebid.fourth_suit_forcing;
    }),
    ("Jordan Truscott 2NT", |a| {
        a.competition.jordan_truscott = !a.competition.jordan_truscott;
    }),
    ("Leaping Michaels", |a| {
        a.defense.leaping_michaels_enabled = !a.defense.leaping_michaels_enabled;
    }),
    ("Responsive double", |a| {
        a.defense.responsive_takeout_enabled = !a.defense.responsive_takeout_enabled;
    }),
    ("Support double/redouble", |a| {
        a.competition.major_support_double = !a.competition.major_support_double;
    }),
    ("1N-3M splinter", |a| {
        a.decision.reading.nt_splinter = !a.decision.reading.nt_splinter
    }),
    ("1NT offshape 4441/5422", |a| {
        a.opening.one_notrump_offshape = !a.opening.one_notrump_offshape;
    }),
    ("1NT shape ladder", |a| {
        a.opening.notrump_shape = match a.opening.notrump_shape {
            NotrumpShape::Balanced => NotrumpShape::Wide6322,
            _ => NotrumpShape::Balanced,
        };
    }),
    ("NT defense (Landy rows)", |a| {
        a.decision.reading.notrump_defense =
            if a.decision.reading.notrump_defense == NotrumpDefense::Woolsey {
                NotrumpDefense::Natural
            } else {
                NotrumpDefense::Woolsey
            };
    }),
    ("Lebensohl rows", |a| {
        a.competition.lebensohl_style = if a.competition.lebensohl_style == LebensohlStyle::Off {
            LebensohlStyle::Transfer
        } else {
            LebensohlStyle::Off
        };
    }),
    ("1NT minor scheme", |a| {
        a.decision.reading.notrump_minors = if a.decision.reading.notrump_minors == EUROPEAN {
            PUPPET
        } else {
            EUROPEAN
        };
    }),
    ("Landy range", |a| {
        a.decision.reading.landy = !a.decision.reading.landy;
        if a.decision.reading.landy {
            // The axis is named for a range, so it moves the shared band too.
            a.decision.reading.convention_points = (8, 14);
        }
    }),
];

/// Bid every deal sequentially under one agreements value
fn bid_all(deals: &[FullDeal], agreements: &Agreements) -> Vec<String> {
    let stance = american(agreements).against();
    deals
        .iter()
        .enumerate()
        .map(|(index, deal)| {
            let dealer = Seat::ALL[index % 4];
            let vul = [
                AbsoluteVulnerability::NONE,
                AbsoluteVulnerability::NS,
                AbsoluteVulnerability::EW,
                AbsoluteVulnerability::ALL,
            ][(index / 4) % 4];
            let mut auction = Auction::new();
            while !auction.has_ended() {
                let seat = seat_to_act(dealer, auction.len());
                auction.push(next_call(&stance, deal[seat], dealer, vul, &auction));
            }
            auction
                .iter()
                .map(|call| format!("{call}"))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

fn main() {
    let args = Args::parse();
    let deals = seeded_deals(args.seed, args.count);

    // Arm 0 is the baseline (defaults, no flip); arms 1.. are the axes.
    let arms: Vec<Vec<String>> = (0..=AXES.len())
        .into_par_iter()
        .map(|arm| {
            let mut agreements = Agreements::default();
            if let Some((_, flip)) = arm.checked_sub(1).map(|i| AXES[i]) {
                flip(&mut agreements);
            }
            bid_all(&deals, &agreements)
        })
        .collect();

    let baseline = &arms[0];
    let mut moved: Vec<(usize, &str)> = AXES
        .iter()
        .zip(&arms[1..])
        .map(|((name, _), auctions)| {
            let n = auctions
                .iter()
                .zip(baseline)
                .filter(|(a, b)| a != b)
                .count();
            (n, *name)
        })
        .collect();
    moved.sort_unstable_by(|a, b| b.cmp(a));

    println!("axis\tmoved\tof {}\tpct", args.count);
    for (n, name) in moved {
        #[allow(clippy::cast_precision_loss)]
        let pct = 100.0 * n as f64 / args.count as f64;
        println!("{name}\t{n}\t{pct:.2}%");
    }
}
