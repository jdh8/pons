//! Michaels and the unusual `2NT` — our two-suited overcalls, and their advances
//!
//! Two calls naming two suits at once: the cue-bid (Michaels) and the jump to
//! `2NT` (both minors, or the two lowest unbid).  [`set_two_suiter_hcp_floor`]
//! sets the strength both require; [`set_unusual_notrump_defense`] the `2NT`
//! band.

use super::*;

thread_local! {
    /// The both-minors `2NT` overcall of their 1NT: `None` = off (the floor's
    /// natural — and near-useless — 2NT); `Some((lo, hi))` = both minors (5-5) on
    /// `points(lo..=hi)`.  **On by default** at `8..=13`; see
    /// [`set_unusual_notrump_defense`].
    static UNUSUAL_NT: Cell<Option<(u8, u8)>> = const { Cell::new(Some((8, 13))) };
}

/// Configure the both-minors `2NT` overcall of an opponent's 1NT for books built
/// *after* this call (thread-local, read once at book-construction time)
///
/// Independent of [`set_landy`]: a natural `2NT` over their strong 1NT is nearly
/// worthless, so this repurposes the bid as a both-minors (5-5) two-suiter on
/// `points(lo..=hi)` — purely additive, it sacrifices no natural call.  **On by
/// default at `Some((8, 13))`**: A/B'd vs the floor (`examples/landy-ab
/// --ns-minors`) it is a vulnerability-dependent wash on plain double-dummy
/// (≈+0.0001 IMPs/board non-vul, ≈−0.0001 vul), shipped on because it is additive
/// and its obstruction/lead-direction value is invisible to the DD measure; the
/// `8`-floor `13`-ceiling and the 5-5 shape were the best-measured settings
/// (capping strong hands and requiring 5-5 both helped).  `None` reverts to the
/// floor's natural `2NT`.
pub fn set_unusual_notrump_defense(range: Option<(u8, u8)>) {
    UNUSUAL_NT.with(|cell| cell.set(range));
}

/// The configured both-minors `2NT` range, or `None` when off
///
/// Crate-visible so the inference reader can condition partner on the two-suiter.
pub(crate) fn unusual_notrump_range() -> Option<(u8, u8)> {
    UNUSUAL_NT.with(Cell::get)
}

thread_local! {
    /// When `Some(n)`, the Michaels cue-bid and the Unusual 2NT require
    /// `hcp(n..)` on top of the shipped `points(8..)`.  **`Some(8)` by
    /// default** (fix-vs-shipped, 1M boards/vul + 50k sd/vul, 24.pdd
    /// 14.3M–16.3M + 22.4M: plain DD +0.0023 ± 0.0008 NV / +0.0031 ± 0.0010
    /// vul, PD +0.0028/+0.0036, sd-lead +0.0024 ± 0.0035 / +0.0046 ± 0.0043 —
    /// no wall inversion).  See [`set_two_suiter_hcp_floor`].
    static TWO_SUITER_HCP_FLOOR: Cell<Option<u8>> = const { Cell::new(Some(8)) };
}

/// Require `hcp(n..)` on the Michaels cue-bid and the Unusual 2NT for books
/// built *after* this call, on top of the shipped `points(8..)`.
///
/// Both rules are documented "8+ HCP" but were gauged in `points`; rule-of-N+8
/// reads a 5-HCP 6-5 freak 8–9, and those garbage two-suiters cue at weight
/// 2.0 straight into −800 penalty doubles (the point-count remnant's Michaels
/// family, −17..−21 IMPs a board).  `Some(8)` restores the documented floor.
/// **Default `Some(8)`** (measured; see the thread-local above); `None`
/// restores the bare `points(8..)` gate.
pub fn set_two_suiter_hcp_floor(n: Option<u8>) {
    TWO_SUITER_HCP_FLOOR.with(|cell| cell.set(n));
}

pub(crate) fn two_suiter_hcp_floor() -> Option<u8> {
    TWO_SUITER_HCP_FLOOR.with(Cell::get)
}

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
        Suit::Hearts => Rules::new()
            .rule(
                Bid::new(4, Strain::Spades),
                130,
                points(10..) & len(Suit::Spades, 3..),
            )
            .rule(Bid::new(2, Strain::Spades), 50, hcp(0..)),
        // Partner shows hearts + a minor: bid hearts.
        Suit::Spades => Rules::new()
            .rule(
                Bid::new(4, Strain::Hearts),
                130,
                points(10..) & len(Suit::Hearts, 3..),
            )
            .rule(Bid::new(3, Strain::Hearts), 50, hcp(0..)),
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
        gate: |agreements| agreements.build.defense.unusual_notrump_range.is_some(),
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
