//! The raw-hand enrichment scans behind `bba-gen --filter-*`, shared with
//! `filter-coverage` so the generator and the coverage check read one
//! definition.
//!
//! Each scan looks at the four hands only — no bidding, no FFI, no solver — so
//! accepting a deal can never depend on what the bidder does with it.  That is
//! also its limit: a filtered A/B measures `E[Δ | divergence, filter]`, which
//! equals `E[Δ | divergence]` only when the scan covers the divergent boards.
//! `filter-coverage` counts how many it misses on an unfiltered pair of arms
//! (docs/measurement.md, coverage).
//!
//! Changing any predicate here changes `bba-gen`'s accepted set, so the
//! persisted seeds of every enriched run stop reproducing.  Add a new
//! predicate instead, as `is_wide_1nt_opener` was added beside
//! `is_1nt_opener`.

use super::hand_hcp;
use contract_bridge::{FullDeal, Hand, Seat, Suit};

#[cfg(test)]
mod tests;

/// Balanced (no singleton/void, at most one doubleton) with 15-17 HCP — a strict
/// 1NT-opener gate for the cheap `--filter-1nt` pre-filter.
pub fn is_1nt_opener(hand: Hand) -> bool {
    let len = Suit::ASC.map(|s| hand[s].len());
    let balanced = len.iter().all(|&l| l >= 2) && len.iter().filter(|&&l| l == 2).count() <= 1;
    balanced && (15..=17).contains(&hand_hcp(hand))
}

/// 15–17 HCP with no suit shorter than two: a superset of the default
/// `NotrumpShape::Wide6322` opener, for `--wide-1nt`, `--filter-multi-slam`
/// and `--filter-lebensohl-raise`.
///
/// [`is_1nt_opener`] stops at one doubleton, so it drops the 5m422 and 6m322
/// hands the default opens 1NT, and those are the shapes that upgrade a point.
/// This also admits the long-major 5422/6322 and the 7222 that open a suit:
/// loose is the safe direction for a *scan* (see `--filter-landy`).
/// [`is_1nt_opener`] stays strict because the older filters' persisted seeds
/// must keep reproducing their accepted sets.
pub fn is_wide_1nt_opener(hand: Hand) -> bool {
    Suit::ASC.iter().all(|&s| hand[s].len() >= 2) && (15..=17).contains(&hand_hcp(hand))
}

/// Landy-shaped: 4+ in both majors with the longer 5+, and overcall values —
/// the raw-hand twin of `landy_2c`'s `five_four(♥, ♠) & points(..)`
/// (`src/bidding/american/defense/nt_landy.rs`), for `--filter-landy`.
///
/// HCP rather than upgraded points, and 8+ uncapped, because this gates a
/// *scan*, not a call: it must not reject a hand the opponents' own band would
/// overcall on.  See `--filter-landy` for why loose is the safe direction.
pub fn is_landy_shaped(hand: Hand) -> bool {
    let (hearts, spades) = (hand[Suit::Hearts].len(), hand[Suit::Spades].len());
    hearts.min(spades) >= 4 && hearts.max(spades) >= 5 && hand_hcp(hand) >= 8
}

/// Preempt-shaped: a seven-card suit and no more than 12 HCP — the raw-hand
/// twin of BBA's natural three-level overcall of a 1NT opening, for
/// `--filter-preempt`.
///
/// HCP rather than upgraded points, and no floor, because this gates a *scan*,
/// not a call.  See `--filter-preempt` for why loose is the safe direction.
pub fn is_preempt_shaped(hand: Hand) -> bool {
    Suit::ASC.iter().any(|&suit| hand[suit].len() >= 7) && hand_hcp(hand) <= 12
}

/// Multi-shaped: a six-card major, for `--filter-multi-slam`.  No HCP gate —
/// see that flag for why.
pub fn is_multi_shaped(hand: Hand) -> bool {
    hand[Suit::Hearts].len() >= 6 || hand[Suit::Spades].len() >= 6
}

/// A six-card minor with 9+ HCP, for `--filter-multi-slam`: looser than the
/// `points(13..) & len(minor, 6..)` slam try it stands in for.
pub fn is_minor_slam_shaped(hand: Hand) -> bool {
    (hand[Suit::Clubs].len() >= 6 || hand[Suit::Diamonds].len() >= 6) && hand_hcp(hand) >= 9
}

/// The weak heart sign-off hand, for `--filter-lebensohl-raise`: looser than
/// the relay's `hcp(6..)` floor and its weak ceiling.
pub fn is_weak_heart_signoff(hand: Hand) -> bool {
    hand[Suit::Hearts].len() >= 5 && hand_hcp(hand) <= 11
}

/// One `bba-gen --filter-*` scan.  `bba-gen` keeps a deal only when every
/// scan it was given accepts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    /// `--filter-1nt`: some seat holds the 1NT candidate.
    OneNt,
    /// `--filter-landy`: some 1NT candidate's LHO is Landy-shaped.
    Landy,
    /// `--filter-preempt`: some 1NT candidate's LHO is preempt-shaped.
    Preempt,
    /// `--filter-multi-slam`: a wide candidate, its Multi-shaped LHO and its
    /// minor-slam-shaped partner.
    MultiSlam,
    /// `--filter-lebensohl-raise`: a wide candidate with three-plus hearts,
    /// its five-plus-spade LHO and its weak-heart partner.
    LebensohlRaise,
}

impl Filter {
    /// Every scan, in `bba-gen`'s flag order
    pub const ALL: [Self; 5] = [
        Self::OneNt,
        Self::Landy,
        Self::Preempt,
        Self::MultiSlam,
        Self::LebensohlRaise,
    ];

    /// The `bba-gen` flag that arms this scan
    pub const fn flag(self) -> &'static str {
        match self {
            Self::OneNt => "--filter-1nt",
            Self::Landy => "--filter-landy",
            Self::Preempt => "--filter-preempt",
            Self::MultiSlam => "--filter-multi-slam",
            Self::LebensohlRaise => "--filter-lebensohl-raise",
        }
    }

    /// Whether `--wide-1nt` changes this scan's 1NT candidate.
    /// `--filter-multi-slam` and `--filter-lebensohl-raise` are always wide.
    pub const fn honours_wide(self) -> bool {
        matches!(self, Self::OneNt | Self::Landy | Self::Preempt)
    }

    /// Whether this scan accepts `deal`; `wide` is `--wide-1nt`.
    ///
    /// Each multi-seat scan is **paired**: one seat must be the 1NT candidate
    /// *and* have the other halves at its LHO / partner, or two unrelated
    /// seats could satisfy the halves.  LHO, not RHO: the hand that acts
    /// over a 1NT opening is the opener's left-hand opponent.  The geometry
    /// is dealer-independent, which matters because `bba-gen` assigns the
    /// dealer only after the scan.
    pub fn accepts(self, deal: &FullDeal, wide: bool) -> bool {
        let candidate: fn(Hand) -> bool = if wide {
            is_wide_1nt_opener
        } else {
            is_1nt_opener
        };
        Seat::ALL.iter().any(|&seat| match self {
            Self::OneNt => candidate(deal[seat]),
            Self::Landy => candidate(deal[seat]) && is_landy_shaped(deal[seat.lho()]),
            Self::Preempt => candidate(deal[seat]) && is_preempt_shaped(deal[seat.lho()]),
            Self::MultiSlam => {
                is_wide_1nt_opener(deal[seat])
                    && is_multi_shaped(deal[seat.lho()])
                    && is_minor_slam_shaped(deal[seat.partner()])
            }
            Self::LebensohlRaise => {
                is_wide_1nt_opener(deal[seat])
                    && deal[seat][Suit::Hearts].len() >= 3
                    && deal[seat.lho()][Suit::Spades].len() >= 5
                    && is_weak_heart_signoff(deal[seat.partner()])
            }
        })
    }
}
