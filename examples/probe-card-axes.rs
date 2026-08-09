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
//! One rayon task per arm, each arming on its own thread: the knobs it flips
//! are thread-local and read at book construction, and rayon reuses pool
//! threads across tasks, so every task first restores the captured defaults,
//! then applies its own flip, then builds its stance — which pins the
//! classify-time state, so nothing is read off the thread after that.
//!
//! ```text
//! cargo run --release --example probe-card-axes -- --count 20000 --seed 1
//! ```

use clap::Parser;
use contract_bridge::auction::Auction;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use pons::american;
use pons::bidding::agreements::{
    Agreements, CompetitionKnobs, NotrumpKnobs, OpeningKnobs, RebidKnobs,
};
use pons::bidding::american::{
    EUROPEAN, LebensohlStyle, NotrumpDefense, NotrumpShape, PUPPET, garbage_stayman,
    leaping_michaels_enabled, notrump_defense, notrump_minors, nt_splinter,
    responsive_takeout_enabled, set_garbage_stayman, set_landy, set_leaping_michaels,
    set_notrump_defense, set_notrump_minors, set_nt_splinter, set_responsive_takeout, set_xyz,
};
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

/// The shipped default of every probed knob, captured off the untouched main
/// thread's readers so a default drift can never silently invert a flip.
#[derive(Clone, Copy)]
struct Defaults {
    garbage: bool,
    nmf: bool,
    xyz: bool,
    super_accept: bool,
    fsf: bool,
    jordan: bool,
    leaping: bool,
    responsive: bool,
    support_x: bool,
    splinter: bool,
    offshape: bool,
    shape: NotrumpShape,
    defense: NotrumpDefense,
    leb: LebensohlStyle,
    minors_european: bool,
    landy: Option<(u8, u8)>,
}

impl Defaults {
    fn capture() -> Self {
        Self {
            garbage: garbage_stayman(),
            // These two have crate-private readers (not web-settable), so
            // their shipped defaults are transcribed from the `Cell::new`
            // literals: xyz.rs:42, nt_landy.rs:17.
            xyz: true,
            landy: None,
            nmf: RebidKnobs::default().new_minor_forcing,
            offshape: OpeningKnobs::default().one_notrump_offshape,
            super_accept: NotrumpKnobs::default().transfer_super_accept,
            fsf: RebidKnobs::default().fourth_suit_forcing,
            jordan: CompetitionKnobs::default().jordan_truscott,
            leaping: leaping_michaels_enabled(),
            responsive: responsive_takeout_enabled(),
            support_x: CompetitionKnobs::default().major_support_double,
            splinter: nt_splinter(),
            shape: OpeningKnobs::default().notrump_shape,
            defense: notrump_defense(),
            leb: CompetitionKnobs::default().lebensohl_style,
            minors_european: notrump_minors() == EUROPEAN,
        }
    }

    fn apply(&self) {
        set_garbage_stayman(self.garbage);
        set_xyz(self.xyz);
        set_leaping_michaels(self.leaping);
        set_responsive_takeout(self.responsive);
        set_nt_splinter(self.splinter);
        set_notrump_defense(self.defense);
        set_notrump_minors(if self.minors_european {
            EUROPEAN
        } else {
            PUPPET
        });
        set_landy(self.landy);
    }

    /// The axes that are fields of [`Agreements`] rather than thread cells
    fn knobs(&self) -> Knobs {
        Knobs {
            opening: OpeningKnobs {
                one_notrump_offshape: self.offshape,
                notrump_shape: self.shape,
                ..OpeningKnobs::default()
            },
            rebid: RebidKnobs {
                new_minor_forcing: self.nmf,
                fourth_suit_forcing: self.fsf,
                ..RebidKnobs::default()
            },
            notrump: NotrumpKnobs {
                transfer_super_accept: self.super_accept,
                ..NotrumpKnobs::default()
            },
            competition: CompetitionKnobs {
                jordan_truscott: self.jordan,
                major_support_double: self.support_x,
                lebensohl_style: self.leb,
                ..CompetitionKnobs::default()
            },
        }
    }
}

/// The axes that live in the captured [`Agreements`] rather than in a cell
#[derive(Clone, Copy)]
struct Knobs {
    opening: OpeningKnobs,
    rebid: RebidKnobs,
    notrump: NotrumpKnobs,
    competition: CompetitionKnobs,
}

impl Knobs {
    /// Paste these onto a fresh capture of the ambient cells
    fn onto_current(self) -> Agreements {
        let mut agreements = Agreements::current();
        agreements.opening = self.opening;
        agreements.rebid = self.rebid;
        agreements.notrump = self.notrump;
        agreements.competition = self.competition;
        agreements
    }
}

/// A knob flip away from the shipped defaults, applied on the bidding thread.
///
/// Most axes are thread cells, flipped in place; the rest live in the
/// [`Agreements`] value, so the flip is also handed the [`Knobs`] the build
/// will carry.  The cells are re-read after the flip, so a flip may do either.
type Flip = fn(&Defaults, &mut Knobs);

/// Every probed axis: the card-block name(s) it moves, and the flip away from
/// the shipped default.  Radio groups (one knob, several rows) probe as one.
const AXES: [(&str, Flip); 16] = [
    ("Garbage Stayman", |d, _| set_garbage_stayman(!d.garbage)),
    ("Checkback (NMF)", |d, k| {
        k.rebid.new_minor_forcing = !d.nmf;
    }),
    ("Two Way NMF (XYZ)", |d, _| set_xyz(!d.xyz)),
    ("Super acceptance", |d, k| {
        k.notrump.transfer_super_accept = !d.super_accept;
    }),
    ("Fourth suit forcing", |d, k| {
        k.rebid.fourth_suit_forcing = !d.fsf;
    }),
    ("Jordan Truscott 2NT", |d, k| {
        k.competition.jordan_truscott = !d.jordan;
    }),
    ("Leaping Michaels", |d, _| set_leaping_michaels(!d.leaping)),
    ("Responsive double", |d, _| {
        set_responsive_takeout(!d.responsive)
    }),
    ("Support double/redouble", |d, k| {
        k.competition.major_support_double = !d.support_x;
    }),
    ("1N-3M splinter", |d, _| set_nt_splinter(!d.splinter)),
    ("1NT offshape 4441/5422", |d, k| {
        k.opening.one_notrump_offshape = !d.offshape;
    }),
    ("1NT shape ladder", |d, k| {
        k.opening.notrump_shape = match d.shape {
            NotrumpShape::Balanced => NotrumpShape::Wide6322,
            _ => NotrumpShape::Balanced,
        };
    }),
    ("NT defense (Landy rows)", |d, _| {
        set_notrump_defense(if d.defense == NotrumpDefense::Woolsey {
            NotrumpDefense::Natural
        } else {
            NotrumpDefense::Woolsey
        });
    }),
    ("Lebensohl rows", |d, k| {
        k.competition.lebensohl_style = if d.leb == LebensohlStyle::Off {
            LebensohlStyle::Transfer
        } else {
            LebensohlStyle::Off
        };
    }),
    ("1NT minor scheme", |d, _| {
        set_notrump_minors(if d.minors_european { PUPPET } else { EUROPEAN });
    }),
    ("Landy range", |d, _| {
        set_landy(if d.landy.is_some() {
            None
        } else {
            Some((8, 14))
        });
    }),
];

/// Bid every deal sequentially on the current thread under the armed knobs.
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
    let defaults = Defaults::capture();
    let deals = seeded_deals(args.seed, args.count);

    // Arm 0 is the baseline (defaults, no flip); arms 1.. are the axes.
    let arms: Vec<Vec<String>> = (0..=AXES.len())
        .into_par_iter()
        .map(|arm| {
            defaults.apply();
            let mut knobs = defaults.knobs();
            if let Some((_, flip)) = arm.checked_sub(1).map(|i| AXES[i]) {
                flip(&defaults, &mut knobs);
            }
            // The ambient half is read *after* the flip, which may have written
            // a cell; the value half is what the flip just edited.
            let auctions = bid_all(&deals, &knobs.onto_current());
            defaults.apply();
            auctions
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
