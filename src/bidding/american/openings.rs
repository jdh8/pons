//! Uncontested openings for every seat
//!
//! This module is the **index** for the base opening table and three child
//! agreements:
//!
//! | Module | Agreement | Knob |
//! | --- | --- | --- |
//! | [`one_notrump`] | the strong `1NT` opening, shape policy, strength gauge, and off-shape treatment | [`OpeningKnobs::open_one_notrump`], [`OpeningKnobs::one_notrump_fifths`], [`OpeningKnobs::notrump_shape`], [`OpeningKnobs::one_notrump_offshape`] |
//! | [`two_notrump`] | the strong `2NT` opening and wide-minor shape treatment | [`two_notrump_wide`][field@crate::bidding::inference::ReadingProfile::two_notrump_wide] |
//! | [`weak_two`] | weak-two strength gauges and wild five-card treatment | [`OpeningKnobs::weak_two_hcp`], [`OpeningKnobs::weak_two_eval`], [`OpeningKnobs::weak_two_wild`] |

use crate::bidding::agreements::Agreements;
use crate::bidding::constraint::{Cons, Constraint, described, hcp, len, nth_seat, or, points};
use crate::bidding::context::Context;
use crate::bidding::rows::{Package, Pattern, compile_into, rows_of};
use crate::bidding::{Alert, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};

mod one_notrump;
mod two_notrump;
mod weak_two;

use one_notrump::with_one_notrump;
use two_notrump::with_two_notrump;
use weak_two::with_weak_twos;

pub use one_notrump::NotrumpShape;
pub use weak_two::WeakTwoEval;

#[cfg(test)]
use one_notrump::notrump_shape;
pub(crate) use two_notrump::two_notrump_wide_shape;

/// The strong, artificial `2♣` opening (22+) — the only artificial opening
const STRONG_2C: Alert = Alert("strong-2c");

/// Better-minor selector: open 1♦ rather than 1♣
///
/// Open the longer minor; with equal length open 1♦ on four-or-more (the
/// standard 4-4 → 1♦, 3-3 → 1♣ split).
fn prefers_diamonds() -> Cons<impl Constraint + Clone> {
    described("prefers diamonds", |hand: Hand, _: &Context<'_>| {
        let clubs = hand[Suit::Clubs].len();
        let diamonds = hand[Suit::Diamonds].len();
        diamonds > clubs || (diamonds == clubs && diamonds >= 4)
    })
}

/// The opening table, shared by every seat
///
/// Strong notrumps (15–17 / 20–21), the artificial 2♣ (22+), five-card majors,
/// better-minor one-of-a-minor openings, weak twos, and three-level preempts.
/// A lighter five-card major is allowed in third and fourth seat.  The 1NT also
/// opens a 5422 or 6322 with a long minor (`wide6322`, the shipped default; see
/// [`openings_with`]).
///
/// Sharp on shape, fuzzy on strength: suit openings gauge upgraded
/// [`points`], notrump ranges gauge [`fifths`][crate::bidding::constraint::fifths].
/// A clean shapely maximum
/// upgrades out of a weak two — it is too good for one.
#[must_use]
pub fn openings(agreements: &Agreements) -> Rules {
    openings_with(NotrumpShape::Wide6322, agreements)
}

/// [`openings`] with the 1NT [`NotrumpShape`] policy selectable
///
/// `openings()` ships [`NotrumpShape::Wide`] (a 5422 with a five-card minor also
/// opens 1NT); [`NotrumpShape::Balanced`] is the classic baseline and
/// [`NotrumpShape::Wide6322`] the experimental superset.
#[must_use]
pub fn openings_with(shape: NotrumpShape, agreements: &Agreements) -> Rules {
    let knobs = &agreements.opening;
    let wide = knobs.wide_one_club;
    let mut rules = if wide {
        // The wide 1♣'s strong `2♣!`: 21–23 with a five-card major or a
        // six-card minor, or any 24+.  Everything else at 21–23 opens a minor
        // and clarifies over the `1♦!` relay (`wide_one_club.rs`).
        let majors = [Suit::Hearts, Suit::Spades];
        let minors = [Suit::Clubs, Suit::Diamonds];
        Rules::new()
            .rule(
                Bid::new(2, Strain::Clubs),
                300,
                (hcp(21..=23) & (or(majors, 5..) | or(minors, 6..))) | hcp(24..),
            )
            .alert(STRONG_2C)
    } else {
        // Strong, artificial 2♣ — top priority.  The `hcp` leg is exact cover
        // for the plain rule-of-N+8 opt-in scale's flat hole: a 4-3-3-3
        // 22-count reads 21 points there and would otherwise demote a game
        // force to a passable 1♣ (the shipped floored scale reads it 22, and
        // unbalanced 22-HCP hands read 22+ points on every scale, so the
        // union adds nothing else — it's redundant-but-exact by default).
        Rules::new()
            .rule(Bid::new(2, Strain::Clubs), 300, points(22..) | hcp(22..))
            .alert(STRONG_2C)
    };
    rules = with_one_notrump(rules, shape, knobs);
    rules = with_two_notrump(rules, agreements);
    // One-level suit openings.  Every band carries an explicit `hcp` floor.
    // On the default PointCount scale the shape [`upgrade`] caps at 2, so
    // `points(N..)` already implies `hcp(N−2..)` and the floor is redundant —
    // but it is retained as the belt for the rule-of-N+8 opt-out, where a
    // count is `hcp + max(0, L1+L2 − 8)`, `L1+L2` reaches 13 on a 7-6-0-0, and
    // `points(N..)` bottoms out at `N − 5` HCP (`points(12..)` would admit a
    // 7-count).  WBF Systems Policy 2024 §2.3.1(c) makes a system a HUM if a
    // one-level opening in first or second seat "may be made with 7 high card
    // points or less", so an envelope that *touches* 0..=7 is caught however
    // rare the hand.  EBU Blue Book §7A3/§8A4 impose the same absolute 8 in
    // every seat, third and fourth included, where the WBF does not reach.
    //
    // First/second seat takes `hcp(10..)` rather than the bare legal minimum —
    // standard practice, since partner must be able to aim at 3NT.  Third and
    // fourth seat open lighter on `points(11..)`, exactly the Rule of 19 (=
    // ACBL "Average Strength", which those charts require in all four seats),
    // floored at the legal 8.
    //
    // The ceiling is `points(..=21)` — above it the hand is the `2♣!` — except
    // under the wide 1♣, where the strong `2♣!` is narrower and the ceiling
    // moves onto raw HCP: majors run to 20 (a 21+ five-card major opens
    // `2♣!`), minors to 23 (`wide_one_club`).
    //
    // Five-card majors; 1♠ ranks just above 1♥ so 5-5 opens the higher.
    let early = nth_seat(1) | nth_seat(2);
    let late = nth_seat(3) | nth_seat(4);
    rules = one_level(
        rules,
        Bid::new(1, Strain::Spades),
        160,
        wide,
        (12, 10, 20),
        len(Suit::Spades, 5..) & early.clone(),
    );
    rules = one_level(
        rules,
        Bid::new(1, Strain::Hearts),
        150,
        wide,
        (12, 10, 20),
        len(Suit::Hearts, 5..) & early,
    );
    // Lighter five-card majors in third/fourth seat.
    rules = one_level(
        rules,
        Bid::new(1, Strain::Spades),
        260,
        wide,
        (11, 8, 20),
        len(Suit::Spades, 5..) & late.clone(),
    );
    rules = one_level(
        rules,
        Bid::new(1, Strain::Hearts),
        250,
        wide,
        (11, 8, 20),
        len(Suit::Hearts, 5..) & late,
    );
    // One-of-a-minor openings (deny a five-card major): better minor, or the
    // 5542 partition — `1♦` on four-plus diamonds (so a (xx)45 opens `1♦` as a
    // canapé), `1♣` on two-plus clubs otherwise (`five_five_four_two`).
    let no_major = || len(Suit::Hearts, ..5) & len(Suit::Spades, ..5);
    let minor = (12, 10, 23);
    if knobs.five_five_four_two {
        rules = one_level(
            rules,
            Bid::new(1, Strain::Diamonds),
            100,
            wide,
            minor,
            len(Suit::Diamonds, 4..) & no_major(),
        );
        rules = one_level(
            rules,
            Bid::new(1, Strain::Clubs),
            100,
            wide,
            minor,
            len(Suit::Clubs, 2..) & len(Suit::Diamonds, ..4) & no_major(),
        );
    } else {
        rules = one_level(
            rules,
            Bid::new(1, Strain::Diamonds),
            100,
            wide,
            minor,
            prefers_diamonds() & no_major(),
        );
        rules = one_level(
            rules,
            Bid::new(1, Strain::Clubs),
            100,
            wide,
            minor,
            len(Suit::Clubs, 3..) & !prefers_diamonds() & no_major(),
        );
    }

    if knobs.multi_two_diamonds {
        // One artificial `2♦!` replaces all three weak twos: 4-10 HCP with
        // exactly one six-card major — no strong variant, at any vulnerability
        // (BBA's book and the independent BBA-WJ harvest agree).  Seven-card
        // majors keep falling to the three-level preempts below, and a
        // six-card *diamond* suit now has no opening at all (`multi.rs`).
        rules = rules
            .rule(
                Bid::new(2, Strain::Diamonds),
                100,
                hcp(4..=10) & (len(Suit::Hearts, 6..=6) | len(Suit::Spades, 6..=6)) & !nth_seat(4),
            )
            .alert(super::multi::MULTI_2D);
    } else {
        rules = with_weak_twos(rules, knobs);
    }
    // Three-level preempts (seven-card suit, not in fourth seat).
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        rules = rules.rule(
            Bid::new(3, Strain::from(suit)),
            90,
            len(suit, 7..) & points(..12) & !nth_seat(4),
        );
    }
    rules.rule(Call::Pass, 0, points(..12))
}

/// One one-level suit opening: `shape` under the strength band
///
/// `(lo, floor, cap)` is the `points` floor, the raw-HCP floor, and the
/// wide-1♣ raw-HCP ceiling.  Off the wide 1♣ the ceiling is american's
/// `points(..=21)` and `cap` is unused, so the shipped description is
/// unchanged.
fn one_level<S: Constraint + 'static>(
    rules: Rules,
    bid: Bid,
    weight: i16,
    wide: bool,
    (lo, floor, cap): (u8, u8, u8),
    shape: Cons<S>,
) -> Rules {
    if wide {
        rules.rule(bid, weight, points(lo..) & hcp(floor..=cap) & shape)
    } else {
        rules.rule(bid, weight, points(lo..=21) & hcp(floor..) & shape)
    }
}

/// The opening table as a row package
///
/// The whole file is one node — the empty auction, fanned over the four seats
/// — so the package is a single row group.  The 1NT shape policy now rides the
/// captured value like every other opening knob, so [`Package::entries`] reads
/// it off `agreements` instead of fetching it from the thread.
pub(super) fn package() -> Package {
    Package {
        name: "openings",
        gate: |_| true,
        entries: |agreements| {
            rows_of(
                Pattern::node("P*"),
                openings_with(agreements.opening.notrump_shape, agreements),
            )
        },
    }
}

/// Register the opening table in the constructive book
pub(super) fn register(book: &mut Trie, agreements: &Agreements) {
    compile_into(book, agreements, &[package()]);
}

#[cfg(test)]
mod tests;
