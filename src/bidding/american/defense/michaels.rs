//! Michaels and the unusual `2NT` — our two-suited overcalls, and their advances
//!
//! Two calls naming two suits at once: the cue-bid (Michaels) and the jump to
//! `2NT` (both minors, or the two lowest unbid).  `agreements.defense.two_suiter_hcp_floor`
//! sets the strength both require; `agreements.defense.unusual_notrump_range` the `2NT`
//! band.

use super::*;

/// Semi-balanced shape for the penalty double: balanced, or one of 5422/6322/7222
///
/// Authored as the exact 5-box union `{2..=5}⁴ ∪ four {suit 6..=7, rest
/// 2..=3}`: the cube is exactly "no singleton, no six-card suit" (balanced ∪
/// 5422, the 13-card sum excludes 5-5 and worse), and the four pan-handles
/// are the 6322/7222 patterns per long suit.  Eval-equivalence with the
/// closure this replaces is pinned exhaustively by
/// `semi_balanced_boxes_match_closure`.
pub(super) fn semi_balanced() -> Cons<impl Constraint + Clone> {
    let mut boxes = vec![length_box([Range::new(2, 5); 4])];
    boxes.extend(Suit::ASC.map(|suit| long_suit_box(suit, Range::new(6, 7), Range::new(2, 3))));
    shapes("balanced or 5422/6322/7222", boxes)
}

/// Advancer's response to partner's Michaels cue-bid over their opening `t`
pub(super) fn michaels_advances(t: Suit) -> Rules {
    match t {
        // Partner shows both majors: prefer the longer one.
        Suit::Clubs | Suit::Diamonds => {
            let hearts_longer = at_least_as_long(Suit::Hearts, Suit::Spades);
            let spades_longer = longer_suit(Suit::Spades, Suit::Hearts);
            Rules::new()
                .rule(
                    Bid::new(4, Strain::Hearts),
                    130,
                    points(10..) & len(Suit::Hearts, 3..) & hearts_longer.clone(),
                )
                .rule(
                    Bid::new(4, Strain::Spades),
                    130,
                    points(10..) & len(Suit::Spades, 3..) & spades_longer.clone(),
                )
                .rule(Bid::new(2, Strain::Hearts), 100, hearts_longer)
                .rule(Bid::new(2, Strain::Spades), 100, spades_longer)
        }
        // Partner shows spades + a minor: bid spades.
        Suit::Hearts => {
            let game = points(10..) & len(Suit::Spades, 3..);
            Rules::new()
                .rule(Bid::new(4, Strain::Spades), 130, game.clone())
                // A preference for partner's shown five-card suit promises no
                // length.  Its exact reading is the game raise's complement.
                .rule(Bid::new(2, Strain::Spades), 50, !game)
        }
        // Partner shows hearts + a minor: bid hearts.
        Suit::Spades => {
            let game = points(10..) & len(Suit::Hearts, 3..);
            Rules::new()
                .rule(Bid::new(4, Strain::Hearts), 130, game.clone())
                .rule(Bid::new(3, Strain::Hearts), 50, !game)
        }
    }
}

/// The two suits shown by an Unusual 2NT over their opening `t`
///
/// Returns `(a, b)` where `a < b` (lower suit first).
const fn unusual_suits(t: Suit) -> (Suit, Suit) {
    match t {
        Suit::Clubs => (Suit::Diamonds, Suit::Hearts),
        Suit::Diamonds => (Suit::Clubs, Suit::Hearts),
        Suit::Hearts | Suit::Spades => (Suit::Clubs, Suit::Diamonds),
    }
}

/// Advancer's response to partner's Unusual 2NT over their opening `t`
pub(super) fn unusual_nt_advances(t: Suit) -> Rules {
    let (a, b) = unusual_suits(t);
    let a_longer = at_least_as_long(a, b);
    let b_longer = longer_suit(b, a);
    Rules::new()
        .rule(Bid::new(3, Strain::from(a)), 100, a_longer)
        .rule(Bid::new(3, Strain::from(b)), 100, b_longer)
}

/// Advancing partner's both-minors `2NT` over their `1NT`
///
/// Doubled we never sit — sitting in `2NT`-doubled is a loser, the doubler has
/// values behind a 15-17 `1NT` — so both entries just pick the longer minor.
pub(super) fn unusual_notrump_advance_package() -> Package {
    Package {
        name: "unusual-notrump-advance",
        gate: |agreements| agreements.defense.unusual_notrump_range.is_some(),
        entries: |_| {
            let mut entries = rows_of(
                Pattern::node("P* (1NT) 2NT -"),
                unusual_nt_advances(Suit::Spades),
            );
            entries.extend(rows_of(
                Pattern::node("P* (1NT) 2NT (X)"),
                unusual_nt_advances(Suit::Spades),
            ));
            entries
        },
    }
}

#[cfg(test)]
mod tests;
