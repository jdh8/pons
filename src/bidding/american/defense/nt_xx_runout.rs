//! A DONT-style runout after their business redouble of our penalty double
//! of their `1NT`
//!
//! `(1NT) X (XX)`, our natural defense only
//! ([`NotrumpDefense::Natural`][crate::bidding::american::NotrumpDefense::Natural]),
//! **direct seat only**, behind `agreements.defense.xx_dont_runout` (default
//! off).  The design and its sources are in
//! `docs/defensive-overcalls.md` § "The DONT-style runout after `(1NT) X (XX)`".
//!
//! The one fact the scheme rests on: their `1NT` is 15+, our `X` is 15+, so
//! the advancer holds at most 40 − 15 − 15 = 10 HCP before their redoubler
//! shows anything, and in practice 0–3.  A values pass ("to play `1NTxx`") has
//! no hands, so the advancer's pass is **forcing** and DONT's escape structure
//! fits verbatim:
//!
//! | Advancer at `(1NT) X (XX)` | Shows |
//! | --- | --- |
//! | `2♣` (alerted) | ♣ and a higher four-card suit, 4-4+ (three-suiters bid it) |
//! | `2♦` (alerted) | ♦ and a four-card major, no four clubs |
//! | `2♥` (alerted) | both majors 4-4+, no four-card minor |
//! | `2♠` | five-plus spades, no other four-card suit, to play |
//! | `-` | forcing: a 5+ one-suiter in ♣/♦/♥, or any 4-3-3-3; at most 10 HCP |
//!
//! The table is a partition of every hand up to 10 HCP, so each call's reading
//! is its exact shape.
//!
//! **Major first** (`agreements.defense.xx_dont_major_first`, default
//! [`XxMajorFirst::Off`] = the table above).  Under `Longer` a five-plus major
//! strictly longer than its four-plus minor, with at most three of the other
//! major, leaves the minor two-suiter: spades bid `2♠`, hearts make the
//! forcing pass and name hearts over the relay.  `AtLeastAsLong` adds the
//! 5-5s.  The table stays a partition; `2♣`/`2♦` keep their old readings (the
//! exclusion overruns the projection's box budget, which costs precision,
//! never soundness).  The forcing pass cannot be alerted (a pass never is); its forcing
//! nature is the doubler's table having no `Pass`, and its meaning is the Pass
//! rule's own constraint, which the pass reading publishes.
//!
//! Over the forcing pass the doubler relays `2♣` (alerted, any hand) and the
//! advancer passes with clubs or names the suit — the five-plus suit, or the
//! four-card suit of a 4-3-3-3 (a four-spade 4-3-3-3 passes first and bids
//! `2♠` now, so a direct `2♠` stays five-plus).  Over the two-suiters the
//! doubler plays DONT's pass-or-correct: pass with three-plus in the shown
//! suit, else relay for the higher one.  If they double a two-suiter, the
//! doubler passes with three-plus or makes an **SOS redouble** ("bid your other
//! suit"); over `2♥` the other suit is known, so the doubler simply passes or
//! bids `2♠` as undoubled.  Every later double of theirs is penalty and we sit:
//! each branch has found a seven-plus-card fit (the doubler is balanced, so it
//! is short in at most one suit), except a 4-3-3-3 that names a four-card suit
//! opposite the doubler's doubleton.
//!
//! **Interaction with `decision.instinct.xx_runout_rails`.**  A book node with
//! finite mass shadows the floor, so with both knobs on this package answers
//! at every node it keys — including `(1NT) X (XX)` and `(1NT) X (XX) - -` —
//! and the rails are inert there.  The two are alternatives, measured as
//! separate arms of `scripts/ab-xx-runout-rails.sh`.
//!
//! **Ours alone.**  `common::mirror_agreements` pins the knob off, so when *we*
//! open `1NT` and redouble their `X`, their pass and runouts keep the floor's
//! reading rather than decoding as this package.

use super::nt_defense::natural_defense_enabled;
use super::nt_dont::{
    dont_one_suiter_direct, name_one_suiter, passed_dont_2c_advance, passed_dont_2c_rebid,
    passed_dont_2d_advance, passed_dont_2d_rebid, passed_dont_2h_advance, passed_dont_x_advance,
};
use super::nt_landy::sit;
use super::*;
use crate::bidding::common::other_major;

/// The advancer's ceiling at `(1NT) X (XX)`: 40 − their 15 − our 15
///
/// Only the forcing pass carries it, as its reading of "weak".  A one-suiter or
/// 4-3-3-3 above it cannot occur opposite a 15+ `1NT` and a 15+ double, so it
/// has no rule and is left to the floor rather than to a catch-all whose top
/// reading would erase the pass's.
const ADVANCER_HCP_MAX: u8 = 10;

/// The direct-seat prefix every node of the package extends
///
/// `P*` capped at two leading passes: with three, the doubler is a passed hand,
/// whose double of `1NT` is not the 15+ penalty double (see
/// `penalty_x_reading_with_profile`).
const PREFIX: &str = "P* (1NT) X (XX)";

/// Which weak two-suiters at `(1NT) X (XX)` run to a five-card major instead of
/// the `2♣`/`2♦` two-suiter
///
/// A five-plus major with a four-plus minor and at most three of the other
/// major.  Under [`Off`][Self::Off] it bids the minor two-suiter, and the
/// doubler's pass-or-correct then picks the minor whenever it holds three, so
/// a 5♠-4♣ advancer plays a 4-3 club fit with a 5-2 spade fit available.
/// The A/B that built the knob priced this bucket at −203 plain IMPs over 94
/// boards (`ab-results/xx-runout-rails-2`, the `dont` arm).  Under the other
/// two modes, five-plus spades bid `2♠` and five-plus hearts make the forcing
/// pass, then bid `2♥` over the doubler's relay.  Both majors (5♠-4♥, 5♥-4♠)
/// still bid `2♥`.  Read only while
/// [`xx_dont_runout`][crate::bidding::agreements::DefenseKnobs::xx_dont_runout]
/// is on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum XxMajorFirst {
    /// Every 4-4+ two-suiter with a minor bids the minor (the built package).
    Off,
    /// Only a major strictly longer than each four-plus minor (5-4, 6-4, 6-5);
    /// a 5-5 still bids the minor two-suiter.
    Longer,
    /// Any five-plus major at least as long as each minor, 5-5 included.
    AtLeastAsLong,
}

/// A weak hand that runs to `major` first: five-plus in it, at most
/// three of the other major, a four-plus minor no longer than it (strictly
/// shorter unless `ties`)
fn major_first(major: Suit, ties: bool) -> Cons<impl Constraint + Clone> {
    use Suit::{Clubs, Diamonds};
    let tie = usize::from(ties);
    len(major, 5..)
        & len(other_major(major), ..=3)
        & or([Clubs, Diamonds], 4..)
        & ((len(major, 5..=5) & and([Clubs, Diamonds], ..=4 + tie))
            | (len(major, 6..=6) & and([Clubs, Diamonds], ..=5 + tie))
            | len(major, 7..))
}

/// The advancer's runout at `(1NT) X (XX)`
///
/// An exact partition: two four-plus suits bid the cheapest two-suiter that
/// holds them, five-plus spades alone bid `2♠`, and everything else — a 5+
/// one-suiter in ♣/♦/♥ or a 4-3-3-3 — makes the forcing pass.  Under a
/// [`XxMajorFirst`] mode other than `Off`, the [`major_first`] hands leave the
/// minor two-suiters: spades join `2♠`, hearts join the forcing pass.
fn advancer_runout(mode: XxMajorFirst) -> Rules {
    use Suit::{Clubs, Diamonds, Hearts, Spades};
    let one_suiter_or_flat =
        dont_one_suiter_direct(5) | and([Clubs, Diamonds, Hearts, Spades], 3..=4);
    let both_majors = and([Hearts, Spades], 4..) & and([Clubs, Diamonds], ..=3);
    let clubs = len(Clubs, 4..) & or([Diamonds, Hearts, Spades], 4..);
    let diamonds = len(Diamonds, 4..) & or([Hearts, Spades], 4..) & len(Clubs, ..=3);
    let spades = len(Spades, 5..) & and([Clubs, Diamonds, Hearts], ..=3);
    let weak = hcp(..=ADVANCER_HCP_MAX);
    if mode == XxMajorFirst::Off {
        return Rules::new()
            .rule(Bid::new(2, Strain::Clubs), 100, clubs)
            .alert(XX_RUNOUT_2C)
            .rule(Bid::new(2, Strain::Diamonds), 100, diamonds)
            .alert(DONT_2D)
            .rule(Bid::new(2, Strain::Hearts), 100, both_majors)
            .alert(DONT_2H)
            .rule(Bid::new(2, Strain::Spades), 100, spades)
            .rule(Call::Pass, 100, one_suiter_or_flat & weak);
    }
    let ties = mode == XxMajorFirst::AtLeastAsLong;
    let neither = !major_first(Spades, ties) & !major_first(Hearts, ties);
    Rules::new()
        .rule(Bid::new(2, Strain::Clubs), 100, clubs & neither.clone())
        .alert(XX_RUNOUT_2C)
        .rule(Bid::new(2, Strain::Diamonds), 100, diamonds & neither)
        .alert(DONT_2D)
        .rule(Bid::new(2, Strain::Hearts), 100, both_majors)
        .alert(DONT_2H)
        .rule(
            Bid::new(2, Strain::Spades),
            100,
            spades | major_first(Spades, ties),
        )
        .rule(
            Call::Pass,
            100,
            (one_suiter_or_flat | major_first(Hearts, ties)) & weak,
        )
}

/// The advancer naming its suit over the doubler's `2♣` relay under a
/// [`XxMajorFirst`] mode other than `Off`
///
/// [`name_one_suiter`]`(4)` with one change: `2♦` caps hearts at four, so a
/// heart-first hand holding four-plus diamonds names hearts (the reason it
/// passed), and each call's reading stays its exact shape.
fn name_major_first() -> Rules {
    Rules::new()
        .rule(
            Bid::new(2, Strain::Diamonds),
            100,
            len(Suit::Diamonds, 4..) & len(Suit::Hearts, ..=4),
        )
        .rule(Bid::new(2, Strain::Hearts), 100, len(Suit::Hearts, 4..))
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 4..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// The doubler over a doubled two-suiter (`… 2♣ (X)` / `… 2♦ (X)`): pass with
/// three-plus in the shown suit, else the SOS redouble — "bid your other suit"
fn sos_or_sit(shown: Suit) -> Rules {
    Rules::new()
        .rule(Call::Redouble, 100, len(shown, ..=2))
        .alert(XX_RUNOUT_SOS)
        .rule(Call::Pass, 0, hcp(0..))
}

/// The advancer over the SOS redouble (`… 2♣ (X) XX -`): bid the other suit,
/// the cheapest four-plus suit above the one shown
///
/// The doubler is balanced and at most doubleton in the shown suit, so it holds
/// three-plus in every other suit and the cheapest is the safest.  The `Pass`
/// is the finite catch-all; a two-suiter always has a rung above it.
fn other_suit_after_sos(shown: Suit) -> Rules {
    let mut rules = Rules::new();
    let mut weight = 100;
    for suit in Suit::ASC.into_iter().filter(|&suit| suit > shown) {
        rules = rules.rule(Bid::new(2, Strain::from(suit)), weight, len(suit, 4..));
        weight -= 10;
    }
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// The package's tables, keyed by the auction after [`PREFIX`]
fn tables(mode: XxMajorFirst) -> Vec<(&'static str, Rules)> {
    let name = || {
        if mode == XxMajorFirst::Off {
            name_one_suiter(4)
        } else {
            name_major_first()
        }
    };
    vec![
        ("", advancer_runout(mode)),
        // The forcing pass: the doubler relays, the advancer names the suit.
        ("- -", passed_dont_x_advance()),
        ("- - 2♣ -", name()),
        ("- - 2♣ (X)", name()),
        // 2♣: clubs + a higher suit.
        ("2♣ -", passed_dont_2c_advance()),
        ("2♣ (X)", sos_or_sit(Suit::Clubs)),
        ("2♣ - 2♦ -", passed_dont_2c_rebid()),
        ("2♣ - 2♦ (X)", passed_dont_2c_rebid()),
        ("2♣ (X) XX -", other_suit_after_sos(Suit::Clubs)),
        // 2♦: diamonds + a major.
        ("2♦ -", passed_dont_2d_advance()),
        ("2♦ (X)", sos_or_sit(Suit::Diamonds)),
        ("2♦ - 2♥ -", passed_dont_2d_rebid()),
        ("2♦ - 2♥ (X)", passed_dont_2d_rebid()),
        ("2♦ (X) XX -", other_suit_after_sos(Suit::Diamonds)),
        // 2♥: both majors — the other suit is known, so no SOS over a double.
        ("2♥ -", passed_dont_2h_advance()),
        ("2♥ (X)", passed_dont_2h_advance()),
    ]
}

/// Our bids that partner still answers (pass-or-correct or a relay), after
/// [`PREFIX`]: their `-` and `(X)` continuations are tables above
const OPEN_BIDS: &[&str] = &["2♣", "2♦", "2♥", "- - 2♣", "2♣ - 2♦", "2♦ - 2♥"];

/// Our bids that end the search for a strain, after [`PREFIX`]: partner sits
/// whether they pass or double
const FINAL_BIDS: &[&str] = &[
    "2♠",
    "- - 2♣ - 2♦",
    "- - 2♣ - 2♥",
    "- - 2♣ - 2♠",
    "- - 2♣ (X) 2♦",
    "- - 2♣ (X) 2♥",
    "- - 2♣ (X) 2♠",
    "2♣ - 2♦ - 2♥",
    "2♣ - 2♦ - 2♠",
    "2♣ - 2♦ (X) 2♥",
    "2♣ - 2♦ (X) 2♠",
    "2♣ (X) XX - 2♦",
    "2♣ (X) XX - 2♥",
    "2♣ (X) XX - 2♠",
    "2♦ - 2♥ - 2♠",
    "2♦ - 2♥ (X) 2♠",
    "2♦ (X) XX - 2♥",
    "2♦ (X) XX - 2♠",
    "2♥ - 2♠",
    "2♥ (X) 2♠",
];

/// Every node where our side sits for a contract already found, after
/// [`PREFIX`]
///
/// For each of our bids: their double after partner accepted it (`- - (X)`),
/// partner's pass of their double (`(X) - -`), and the round after that.  A
/// final bid adds partner's own turn over their pass or double.  Every double
/// of theirs here is penalty (their redouble was business), and our side has a
/// seven-plus-card fit on every branch but one, so nobody runs.
fn sit_suffixes() -> Vec<String> {
    let mut suffixes = Vec::new();
    for bid in OPEN_BIDS.iter().chain(FINAL_BIDS) {
        for tail in ["(X) - -", "- - (X)", "- - (X) - -"] {
            suffixes.push(format!("{bid} {tail}"));
        }
    }
    for bid in FINAL_BIDS {
        for tail in ["-", "(X)"] {
            suffixes.push(format!("{bid} {tail}"));
        }
    }
    suffixes
}

/// The node key for a suffix after [`PREFIX`], capped at two leading passes
fn node(suffix: &str) -> Pattern {
    let key = if suffix.is_empty() {
        PREFIX.to_string()
    } else {
        format!("{PREFIX} {suffix}")
    };
    Pattern::node(&key).with_fan(2)
}

/// The DONT-style runout after `(1NT) X (XX)`, when
/// `agreements.defense.xx_dont_runout` is on and the natural defense is active
pub(super) fn xx_dont_runout_package() -> Package {
    Package {
        name: "xx-dont-runout",
        gate: |agreements| agreements.defense.xx_dont_runout && natural_defense_enabled(agreements),
        entries: |agreements| {
            let mut entries: Vec<Entry> = tables(agreements.defense.xx_dont_major_first)
                .into_iter()
                .flat_map(|(suffix, rules)| rows_of(node(suffix), rules))
                .collect();
            for suffix in sit_suffixes() {
                entries.extend(rows_of(node(&suffix), sit()));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
