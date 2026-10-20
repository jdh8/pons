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
//! is its exact shape.  The forcing pass cannot be alerted (a pass never is); its forcing
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

/// The advancer's runout at `(1NT) X (XX)`
///
/// An exact partition: two four-plus suits bid the cheapest two-suiter that
/// holds them, five-plus spades alone bid `2♠`, and everything else — a 5+
/// one-suiter in ♣/♦/♥ or a 4-3-3-3 — makes the forcing pass.
fn advancer_runout() -> Rules {
    use Suit::{Clubs, Diamonds, Hearts, Spades};
    let one_suiter_or_flat =
        dont_one_suiter_direct(5) | and([Clubs, Diamonds, Hearts, Spades], 3..=4);
    Rules::new()
        .rule(
            Bid::new(2, Strain::Clubs),
            100,
            len(Clubs, 4..) & or([Diamonds, Hearts, Spades], 4..),
        )
        .alert(XX_RUNOUT_2C)
        .rule(
            Bid::new(2, Strain::Diamonds),
            100,
            len(Diamonds, 4..) & or([Hearts, Spades], 4..) & len(Clubs, ..=3),
        )
        .alert(DONT_2D)
        .rule(
            Bid::new(2, Strain::Hearts),
            100,
            and([Hearts, Spades], 4..) & and([Clubs, Diamonds], ..=3),
        )
        .alert(DONT_2H)
        .rule(
            Bid::new(2, Strain::Spades),
            100,
            len(Spades, 5..) & and([Clubs, Diamonds, Hearts], ..=3),
        )
        .rule(
            Call::Pass,
            100,
            one_suiter_or_flat & hcp(..=ADVANCER_HCP_MAX),
        )
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
fn tables() -> Vec<(&'static str, Rules)> {
    vec![
        ("", advancer_runout()),
        // The forcing pass: the doubler relays, the advancer names the suit.
        ("- -", passed_dont_x_advance()),
        ("- - 2♣ -", name_one_suiter(4)),
        ("- - 2♣ (X)", name_one_suiter(4)),
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
        entries: |_| {
            let mut entries: Vec<Entry> = tables()
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
