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
    Agreements, CompetitionKnobs, DefenseKnobs, NotrumpKnobs, OpeningKnobs, RebidKnobs,
};
use pons::bidding::american::{EUROPEAN, LebensohlStyle, NotrumpDefense, NotrumpShape, PUPPET};
use pons::bidding::inference::ReadingProfile;
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
        let reading = ReadingProfile::default();
        Self {
            garbage: reading.garbage_stayman,
            xyz: reading.xyz,
            landy: reading.landy_range,
            nmf: RebidKnobs::default().new_minor_forcing,
            offshape: OpeningKnobs::default().one_notrump_offshape,
            super_accept: NotrumpKnobs::default().transfer_super_accept,
            fsf: RebidKnobs::default().fourth_suit_forcing,
            jordan: CompetitionKnobs::default().jordan_truscott,
            leaping: DefenseKnobs::default().leaping_michaels_enabled,
            responsive: DefenseKnobs::default().responsive_takeout_enabled,
            support_x: CompetitionKnobs::default().major_support_double,
            splinter: reading.nt_splinter,
            shape: OpeningKnobs::default().notrump_shape,
            defense: reading.notrump_defense,
            leb: CompetitionKnobs::default().lebensohl_style,
            minors_european: reading.notrump_minors == EUROPEAN,
        }
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
            defense: DefenseKnobs {
                leaping_michaels_enabled: self.leaping,
                responsive_takeout_enabled: self.responsive,
                ..DefenseKnobs::default()
            },
            reading: ReadingProfile {
                garbage_stayman: self.garbage,
                xyz: self.xyz,
                nt_splinter: self.splinter,
                notrump_defense: self.defense,
                notrump_minors: if self.minors_european {
                    EUROPEAN
                } else {
                    PUPPET
                },
                landy_range: self.landy,
                ..ReadingProfile::default()
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
    defense: DefenseKnobs,
    reading: ReadingProfile,
}

impl Knobs {
    /// Paste these onto a fresh capture of the ambient cells
    fn onto_current(self) -> Agreements {
        let mut agreements = Agreements::current();
        agreements.opening = self.opening;
        agreements.rebid = self.rebid;
        agreements.notrump = self.notrump;
        agreements.competition = self.competition;
        agreements.defense = self.defense;
        agreements.decision.reading = self.reading;
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
    ("Garbage Stayman", |d, k| {
        k.reading.garbage_stayman = !d.garbage
    }),
    ("Checkback (NMF)", |d, k| {
        k.rebid.new_minor_forcing = !d.nmf;
    }),
    ("Two Way NMF (XYZ)", |d, k| k.reading.xyz = !d.xyz),
    ("Super acceptance", |d, k| {
        k.notrump.transfer_super_accept = !d.super_accept;
    }),
    ("Fourth suit forcing", |d, k| {
        k.rebid.fourth_suit_forcing = !d.fsf;
    }),
    ("Jordan Truscott 2NT", |d, k| {
        k.competition.jordan_truscott = !d.jordan;
    }),
    ("Leaping Michaels", |d, k| {
        k.defense.leaping_michaels_enabled = !d.leaping;
    }),
    ("Responsive double", |d, k| {
        k.defense.responsive_takeout_enabled = !d.responsive;
    }),
    ("Support double/redouble", |d, k| {
        k.competition.major_support_double = !d.support_x;
    }),
    ("1N-3M splinter", |d, k| k.reading.nt_splinter = !d.splinter),
    ("1NT offshape 4441/5422", |d, k| {
        k.opening.one_notrump_offshape = !d.offshape;
    }),
    ("1NT shape ladder", |d, k| {
        k.opening.notrump_shape = match d.shape {
            NotrumpShape::Balanced => NotrumpShape::Wide6322,
            _ => NotrumpShape::Balanced,
        };
    }),
    ("NT defense (Landy rows)", |d, k| {
        k.reading.notrump_defense = if d.defense == NotrumpDefense::Woolsey {
            NotrumpDefense::Natural
        } else {
            NotrumpDefense::Woolsey
        };
    }),
    ("Lebensohl rows", |d, k| {
        k.competition.lebensohl_style = if d.leb == LebensohlStyle::Off {
            LebensohlStyle::Transfer
        } else {
            LebensohlStyle::Off
        };
    }),
    ("1NT minor scheme", |d, k| {
        k.reading.notrump_minors = if d.minors_european { PUPPET } else { EUROPEAN };
    }),
    ("Landy range", |d, k| {
        k.reading.landy_range = if d.landy.is_some() {
            None
        } else {
            Some((8, 14))
        };
        if let Some(range) = k.reading.landy_range {
            k.reading.woolsey_points = range;
        }
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
            let mut knobs = defaults.knobs();
            if let Some((_, flip)) = arm.checked_sub(1).map(|i| AXES[i]) {
                flip(&defaults, &mut knobs);
            }
            bid_all(&deals, &knobs.onto_current())
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
