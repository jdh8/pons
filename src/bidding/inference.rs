//! What the calls have shown, accumulated across an auction
//!
//! [`Context`] gives the *laws-only* facts of an auction; the authored books
//! and the [instinct floor][super::instinct] read *system intent* off the
//! calls on demand (see [`Interpretation`][super::instinct]).  This module is
//! the richest such reading: for every player, the range of cards each suit
//! may hold and the range of points shown, derived **purely from the calls**
//! under standard 2/1 meanings.
//!
//! Two consumers want this summary that the per-bid [`Constraint`]s cannot
//! give them — a `Constraint` is eval-only, so the length a `len(..)` rule
//! asserts can never be read back out:
//!
//! - the [instinct floor][super::instinct], so a forced auction can pick a
//!   known major-suit fit over notrump instead of re-deriving partner's shape
//!   from scratch;
//! - constrained sampling (future), which needs per-player {suit → length} and
//!   points to deal hands consistent with an auction.
//!
//! # Soundness over tightness
//!
//! Every player starts at [`Inference::unknown`] and each call only ever
//! *narrows* a range via [`Range::intersect`].  A rule that is unsure leaves
//! the range wide; a missing rule costs tightness, never soundness.  The
//! guarantee a consumer may rely on is one-directional: a hand that actually
//! made these calls always falls **within** every shown range.  The deriver
//! therefore reads only the meanings that hold robustly — natural suit
//! lengths, raises, rebids, overcalls — and stays silent on the artificial
//! structures (Stayman, transfers, the strong-2♣ responses) that a keyless
//! reading would misread as natural.
//!
//! # One system
//!
//! The meanings encoded here are those of [`two_over_one`][super::two_over_one]
//! (five-card majors, strong 15–17 notrump, strong artificial 2♣); like the
//! instinct floor, this reading is tied to that system.

use super::context::Context;
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// The largest a suit length range may span
const LENGTH_CAP: u8 = 13;
/// The largest a point range may span (all forty HCP, then some)
const POINTS_CAP: u8 = 37;

/// An inclusive `[min, max]` range of a shown quantity — a length or points
///
/// A plain `Copy` pair rather than [`core::ops::RangeInclusive`], so it can be
/// stored, compared, and (de)serialized, and carries [`intersect`][Self::intersect].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Range {
    /// The least the quantity can be
    pub min: u8,
    /// The most the quantity can be
    pub max: u8,
}

impl Range {
    /// Nothing known about a suit length yet: `0..=13`
    pub const FULL_LENGTH: Self = Self {
        min: 0,
        max: LENGTH_CAP,
    };
    /// Nothing known about points yet: `0..=37`
    pub const FULL_POINTS: Self = Self {
        min: 0,
        max: POINTS_CAP,
    };

    /// An inclusive `[min, max]` range
    #[must_use]
    pub const fn new(min: u8, max: u8) -> Self {
        Self { min, max }
    }

    /// `min..=cap` — at least `min`, up to the quantity's natural ceiling
    #[must_use]
    const fn at_least(min: u8, cap: u8) -> Self {
        Self { min, max: cap }
    }

    /// Whether `n` falls within the range
    #[must_use]
    pub const fn contains(self, n: u8) -> bool {
        self.min <= n && n <= self.max
    }

    /// The conjunction of two ranges — the tighter bounds of each
    ///
    /// Two independently sound inferences about the same quantity both hold, so
    /// the truth lies in their intersection.  If the bounds cross (an empty
    /// intersection), some inference was unsound for this auction; rather than
    /// drop the truth, widen to the *union* — soundness over tightness.
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);
        if min <= max {
            Self { min, max }
        } else {
            Self {
                min: self.min.min(other.min),
                max: self.max.max(other.max),
            }
        }
    }
}

/// What the calls have shown about one player, hand-independently
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Inference {
    /// Shown length range per suit, indexed by `suit as usize` (the ascending
    /// [`Suit::ASC`] order: clubs, diamonds, hearts, spades)
    pub lengths: [Range; 4],
    /// Shown point range, on the upgraded [`points`][super::constraint::points]
    /// scale the suit-oriented rules gauge (raw HCP for the balanced openings)
    pub points: Range,
}

impl Inference {
    /// Nothing shown yet: every suit `0..=13`, points `0..=37`
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            lengths: [Range::FULL_LENGTH; 4],
            points: Range::FULL_POINTS,
        }
    }

    /// The shown length range of a suit
    #[must_use]
    pub const fn length(&self, suit: Suit) -> Range {
        self.lengths[suit as usize]
    }

    /// Narrow a suit's shown length by intersecting in `range`
    fn narrow_length(&mut self, suit: Suit, range: Range) {
        let slot = &mut self.lengths[suit as usize];
        *slot = slot.intersect(range);
    }

    /// Narrow the shown points by intersecting in `range`
    fn narrow_points(&mut self, range: Range) {
        self.points = self.points.intersect(range);
    }
}

/// A seat relative to the player about to act, clockwise
///
/// The discriminant is the seating order from the actor — the natural index
/// for a sampler walking the four hands — *not* the auction's parity, so map
/// an auction position through [`relative_of`] rather than casting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Relative {
    /// The player about to act
    Me = 0,
    /// Left-hand opponent (the next to act)
    Lho = 1,
    /// Partner
    Partner = 2,
    /// Right-hand opponent (the previous to act)
    Rho = 3,
}

/// The relative seat of the call at `index` in an auction of length `len`
///
/// Mirrors [`Context`]'s parity: the call before the actor's (`len - index ==
/// 1`) is RHO, two before is partner, three before is LHO, four before is the
/// actor again.
const fn relative_of(len: usize, index: usize) -> Relative {
    match (len - index) % 4 {
        0 => Relative::Me,
        1 => Relative::Rho,
        2 => Relative::Partner,
        _ => Relative::Lho,
    }
}

/// All four players' shown shape and strength, relative to the side to act
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Inferences {
    players: [Inference; 4],
}

impl Inferences {
    /// The shown shape and strength of one relative seat
    #[must_use]
    pub const fn get(&self, who: Relative) -> &Inference {
        &self.players[who as usize]
    }

    /// What the player to act has shown by their own prior calls
    #[must_use]
    pub const fn me(&self) -> &Inference {
        self.get(Relative::Me)
    }

    /// What partner has shown
    #[must_use]
    pub const fn partner(&self) -> &Inference {
        self.get(Relative::Partner)
    }

    /// What the left-hand opponent has shown
    #[must_use]
    pub const fn lho(&self) -> &Inference {
        self.get(Relative::Lho)
    }

    /// What the right-hand opponent has shown
    #[must_use]
    pub const fn rho(&self) -> &Inference {
        self.get(Relative::Rho)
    }

    /// Derive, hand-independently, what every player's calls have shown under
    /// standard 2/1 meanings, relative to the side to act
    #[must_use]
    pub fn read(context: &Context<'_>) -> Self {
        let auction = context.auction();
        let len = auction.len();
        let mut players = [Inference::unknown(); 4];

        let Some(opening_index) = auction.iter().position(|&c| c != Call::Pass) else {
            return Self { players };
        };
        let Call::Bid(opening_bid) = auction[opening_index] else {
            return Self { players };
        };
        let opener_lane = opening_index % 4;
        // SAFETY: at most three passes precede the opening, so the cast is safe.
        #[allow(clippy::cast_possible_truncation)]
        let opener_seat = opening_index as u8 + 1;
        let opening_artificial =
            opening_bid.strain == Strain::Notrump || opening_bid == Bid::new(2, Strain::Clubs);
        let defending_parity = (opener_lane + 1) % 2;

        // Suits bid and the count of bids made, per auction lane (`index % 4`);
        // lanes of equal parity are partners, the same side.
        let mut lane_suits = [0u8; 4];
        let mut lane_bids = [0u8; 4];
        let mut side_acted = [false; 2];
        let mut highest: Option<Bid> = None;

        for (index, &call) in auction.iter().enumerate() {
            let lane = index % 4;
            let who = relative_of(len, index) as usize;
            let is_opening_side = lane % 2 == opener_lane % 2;
            let first_action_of_side = !side_acted[lane % 2];

            match call {
                Call::Pass | Call::Redouble => {}
                Call::Double => {
                    // A direct double of a natural suit opening, the defending
                    // side's first action, reads as takeout: opening values.
                    if !is_opening_side
                        && first_action_of_side
                        && index != opening_index
                        && opening_bid.strain.is_suit()
                    {
                        players[who].narrow_points(Range::at_least(11, POINTS_CAP));
                    }
                    side_acted[lane % 2] = true;
                }
                Call::Bid(bid) => {
                    if index == opening_index {
                        apply_opening(&mut players[who], bid, opener_seat);
                    } else if let Some(suit) = bid.strain.suit() {
                        // A three-level suit bid over our 1NT is off-book and
                        // forcing — the instinct reading takes it as natural,
                        // five-plus (see `opener_forced_past_invitation`).  The
                        // two-level responses are Stayman and transfers.
                        let over_one_notrump = is_opening_side
                            && opening_bid == Bid::new(1, Strain::Notrump)
                            && bid.level.get() == 3;
                        let suppress = is_opening_side && opening_artificial && !over_one_notrump;

                        if !suppress {
                            let jump = bid
                                .level
                                .get()
                                .saturating_sub(cheapest_level(highest, bid.strain));
                            let mask = 1u8 << suit as u8;
                            let i_bid_it = lane_suits[lane] & mask != 0;
                            let partner_bid_it = lane_suits[(lane + 2) % 4] & mask != 0;

                            if i_bid_it {
                                // Rebidding our own suit shows a sixth card.
                                players[who].narrow_length(suit, Range::at_least(6, LENGTH_CAP));
                            } else if partner_bid_it {
                                // Raising partner's suit shows three-card support.
                                players[who].narrow_length(suit, Range::at_least(3, LENGTH_CAP));
                            } else if over_one_notrump {
                                // Natural, forcing five-card suit over our 1NT.
                                players[who].narrow_length(suit, Range::at_least(5, LENGTH_CAP));
                            } else if !is_opening_side && first_action_of_side {
                                // The defending side's first suit bid is an
                                // overcall: a five-card suit (six if jumping),
                                // opening values at the cheapest level.
                                let min = if jump >= 1 { 6 } else { 5 };
                                players[who].narrow_length(suit, Range::at_least(min, LENGTH_CAP));
                                if jump == 0 {
                                    players[who].narrow_points(Range::at_least(8, POINTS_CAP));
                                }
                            } else if jump >= 1 {
                                // A single jump in a new suit is a weak jump:
                                // a six-card suit.  Skip splinters (double jumps).
                                if jump == 1 {
                                    players[who]
                                        .narrow_length(suit, Range::at_least(6, LENGTH_CAP));
                                }
                            } else {
                                // A natural new suit at the cheapest level: four-plus.
                                players[who].narrow_length(suit, Range::at_least(4, LENGTH_CAP));
                                apply_response_points(
                                    &mut players[who],
                                    bid,
                                    opening_bid,
                                    is_opening_side
                                        && lane == (opener_lane + 2) % 4
                                        && lane_bids[lane] == 0
                                        && !side_acted[defending_parity],
                                );
                            }
                        }
                    }

                    if let Some(suit) = bid.strain.suit() {
                        lane_suits[lane] |= 1 << suit as u8;
                    }
                    lane_bids[lane] += 1;
                    side_acted[lane % 2] = true;
                    if highest.is_none_or(|h| outranks(bid, h)) {
                        highest = Some(bid);
                    }
                }
            }
        }

        Self { players }
    }
}

/// Whether `bid` is higher than the standing `highest` contract
fn outranks(bid: Bid, highest: Bid) -> bool {
    bid.strain > highest.strain
        || (bid.strain == highest.strain && bid.level.get() > highest.level.get())
}

/// The cheapest level a strain can be bid over the standing `highest` contract
const fn cheapest_level(highest: Option<Bid>, strain: Strain) -> u8 {
    match highest {
        None => 1,
        Some(h) if strain as u8 > h.strain as u8 => h.level.get(),
        Some(h) => h.level.get() + 1,
    }
}

/// Apply the meaning of the opening bid (the first non-pass call)
fn apply_opening(inf: &mut Inference, bid: Bid, seat: u8) {
    let majors_light = if seat >= 3 {
        Range::new(9, 21)
    } else {
        Range::new(12, 21)
    };
    match (bid.level.get(), bid.strain) {
        (1, Strain::Hearts) => {
            inf.narrow_length(Suit::Hearts, Range::at_least(5, LENGTH_CAP));
            inf.narrow_points(majors_light);
        }
        (1, Strain::Spades) => {
            inf.narrow_length(Suit::Spades, Range::at_least(5, LENGTH_CAP));
            inf.narrow_points(majors_light);
        }
        (1, Strain::Diamonds) => {
            inf.narrow_length(Suit::Diamonds, Range::at_least(3, LENGTH_CAP));
            inf.narrow_length(Suit::Hearts, Range::new(0, 4));
            inf.narrow_length(Suit::Spades, Range::new(0, 4));
            inf.narrow_points(Range::new(12, 21));
        }
        (1, Strain::Clubs) => {
            inf.narrow_length(Suit::Clubs, Range::at_least(3, LENGTH_CAP));
            inf.narrow_length(Suit::Hearts, Range::new(0, 4));
            inf.narrow_length(Suit::Spades, Range::new(0, 4));
            inf.narrow_points(Range::new(12, 21));
        }
        (1, Strain::Notrump) => {
            balanced(inf);
            inf.narrow_points(Range::new(14, 18));
        }
        (2, Strain::Clubs) => {
            // Strong and artificial: 22+ points, but nothing about shape.
            inf.narrow_points(Range::at_least(20, POINTS_CAP));
        }
        (2, Strain::Notrump) => {
            balanced(inf);
            inf.narrow_points(Range::new(19, 22));
        }
        (2, strain) if strain.is_suit() => {
            inf.narrow_length(strain.suit().unwrap(), Range::new(6, 6));
            inf.narrow_points(Range::new(5, 10));
        }
        (3, strain) if strain.is_suit() => {
            inf.narrow_length(strain.suit().unwrap(), Range::at_least(7, LENGTH_CAP));
            inf.narrow_points(Range::new(0, 11));
        }
        _ => {}
    }
}

/// Narrow a balanced opener: two to five cards in every suit
fn balanced(inf: &mut Inference) {
    for suit in Suit::ASC {
        inf.narrow_length(suit, Range::new(2, 5));
    }
}

/// The point floor a responder's first natural new suit shows, when uncontested
///
/// A one-level new suit promises six-plus points; a game-forcing 2/1 (a
/// two-level new suit over a one-of-a-major opening, or `1♦`–`2♣`) promises
/// thirteen-plus.
fn apply_response_points(inf: &mut Inference, response: Bid, opening: Bid, eligible: bool) {
    if !eligible {
        return;
    }
    match response.level.get() {
        1 => inf.narrow_points(Range::at_least(6, POINTS_CAP)),
        2 if is_two_over_one(opening, response) => {
            inf.narrow_points(Range::at_least(13, POINTS_CAP));
        }
        _ => {}
    }
}

/// Whether a two-level new suit is a game-forcing 2/1 over `opening`
fn is_two_over_one(opening: Bid, response: Bid) -> bool {
    response.level.get() == 2
        && match opening.strain {
            Strain::Hearts | Strain::Spades => true,
            Strain::Diamonds => response.strain == Strain::Clubs,
            _ => false,
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::constraint::point_count;
    use contract_bridge::auction::RelativeVulnerability;
    use contract_bridge::{Bid, Hand, Level};
    use proptest::prelude::*;

    const fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid {
            level: Level::new(level),
            strain,
        })
    }

    fn read(auction: &[Call]) -> Inferences {
        Inferences::read(&Context::new(RelativeVulnerability::NONE, auction))
    }

    #[test]
    fn opening_shapes() {
        // [1♥]: the opener sits to our right (the call just before ours).
        let one_heart = read(&[bid(1, Strain::Hearts)]);
        assert_eq!(one_heart.rho().length(Suit::Hearts), Range::new(5, 13));
        assert_eq!(one_heart.rho().points, Range::new(12, 21));

        // A strong notrump is balanced; an artificial 2♣ says only "strong".
        let one_nt = read(&[bid(1, Strain::Notrump)]);
        assert_eq!(one_nt.rho().length(Suit::Spades), Range::new(2, 5));
        assert_eq!(one_nt.rho().points, Range::new(14, 18));

        let two_clubs = read(&[bid(2, Strain::Clubs)]);
        assert_eq!(two_clubs.rho().length(Suit::Spades), Range::FULL_LENGTH);
        assert_eq!(two_clubs.rho().points, Range::new(20, 37));

        // Weak two: exactly six; three-level preempt: seven-plus.
        let weak_two = read(&[bid(2, Strain::Spades)]);
        assert_eq!(weak_two.rho().length(Suit::Spades), Range::new(6, 6));
        assert_eq!(weak_two.rho().points, Range::new(5, 10));
        let preempt = read(&[bid(3, Strain::Diamonds)]);
        assert_eq!(preempt.rho().length(Suit::Diamonds), Range::new(7, 13));

        // A 1♣ opening denies a five-card major.
        let one_club = read(&[bid(1, Strain::Clubs)]);
        assert_eq!(one_club.rho().length(Suit::Clubs), Range::new(3, 13));
        assert_eq!(one_club.rho().length(Suit::Hearts), Range::new(0, 4));
    }

    #[test]
    fn third_seat_openings_are_light() {
        // [P, P, 1♠]: a third-seat opener may be down to nine points.
        let third = read(&[Call::Pass, Call::Pass, bid(1, Strain::Spades)]);
        assert_eq!(third.rho().points, Range::new(9, 21));
    }

    #[test]
    fn responses_narrow_partner_and_opener() {
        // [1♥, P, 2♣, P]: we opened 1♥ (partner is us at index 0... no — at
        // len 4, index 0 is Me), partner responded 2♣ (game-forcing 2/1).
        let auction = [
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ];
        let inf = read(&auction);
        // Index 0 (1♥) is four before the actor → Me, the opener.
        assert_eq!(inf.me().length(Suit::Hearts), Range::new(5, 13));
        // Index 2 (2♣) is two before → Partner, the 2/1 responder.
        assert_eq!(inf.partner().length(Suit::Clubs), Range::new(4, 13));
        assert_eq!(inf.partner().points, Range::new(13, 37));
    }

    #[test]
    fn opener_rebid_shows_sixth_card() {
        // [1♥, P, 1♠, P, 2♥, P]: at length 6 the opener (who bid 1♥ and rebid
        // 2♥) sits as partner, and the 1♠ responder is us.
        let auction = [
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ];
        let inf = read(&auction);
        // Partner opened 1♥ then rebid hearts, showing six.
        assert_eq!(inf.partner().length(Suit::Hearts), Range::new(6, 13));
        // Our 1♠ response showed four spades and six-plus points.
        assert_eq!(inf.me().length(Suit::Spades), Range::new(4, 13));
        assert_eq!(inf.me().points, Range::new(6, 37));
    }

    #[test]
    fn overcall_shows_five_cards() {
        // [1♦, 1♠]: their 1♦ opening, our partner's... no — at len 2, index 1
        // (1♠) is RHO.  Their 1♦ is two before → Partner? recompute below.
        let auction = [bid(1, Strain::Diamonds), bid(1, Strain::Spades)];
        let inf = read(&auction);
        // Index 0 (1♦ opening) → Partner; index 1 (1♠ overcall) → Rho.
        assert_eq!(inf.partner().length(Suit::Diamonds), Range::new(3, 13));
        assert_eq!(inf.rho().length(Suit::Spades), Range::new(5, 13));
        assert_eq!(inf.rho().points, Range::new(8, 37));
    }

    #[test]
    fn transfers_are_not_read_as_natural() {
        // [1NT, P, 2♦, P]: 2♦ is a Jacoby transfer, not diamonds — the
        // opening side's artificial response leaves shape unknown.
        let auction = [
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
        ];
        let inf = read(&auction);
        assert_eq!(inf.partner().length(Suit::Diamonds), Range::FULL_LENGTH);
    }

    #[test]
    fn three_level_suit_over_one_notrump_is_natural() {
        // [1NT, P, 3♥, P]: a three-level suit bid over 1NT is forcing and
        // natural in the instinct reading — five-plus hearts.
        let auction = [
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(3, Strain::Hearts),
            Call::Pass,
        ];
        let inf = read(&auction);
        assert_eq!(inf.partner().length(Suit::Hearts), Range::new(5, 13));
    }

    #[test]
    fn relative_seat_tracks_the_actor() {
        // The same 1♥ opening lands on a different relative seat as the
        // auction grows by one call.
        assert_eq!(
            read(&[bid(1, Strain::Hearts)]).rho().points,
            Range::new(12, 21)
        );
        assert_eq!(
            read(&[bid(1, Strain::Hearts), Call::Pass]).partner().points,
            Range::new(12, 21)
        );
    }

    #[test]
    fn range_intersect_widens_on_conflict() {
        // Disjoint ranges cannot both hold; widen to the union, never empty.
        assert_eq!(
            Range::new(5, 13).intersect(Range::new(6, 13)),
            Range::new(6, 13)
        );
        assert_eq!(
            Range::new(0, 3).intersect(Range::new(6, 13)),
            Range::new(0, 13)
        );
    }

    proptest! {
        /// Soundness: a hand that opens the book's choice falls within the
        /// opening inference.  Tests rule 1 (the opening table) over random hands.
        #[test]
        fn opening_inference_contains_the_opener(seed in any::<u64>()) {
            use crate::bidding::trie::Classifier;
            use crate::bidding::two_over_one::openings;
            use contract_bridge::deck::full_deal;
            use rand::SeedableRng;

            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            let deal = full_deal(&mut rng);
            let hand: Hand = deal[contract_bridge::Seat::North];

            let context = Context::new(RelativeVulnerability::NONE, &[]);
            let logits = openings().classify(hand, &context);
            let Some((call, _)) = (&logits.0)
                .into_iter()
                .filter(|(_, l)| l.is_finite())
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("not NaN"))
            else {
                return Ok(());
            };
            let Call::Bid(_) = call else { return Ok(()); };

            // The opener sits to the actor's right after a single call.
            let inf = read(&[call]);
            let opener = inf.rho();
            let points = point_count(hand);
            prop_assert!(
                opener.points.contains(points),
                "{call} opener with {points} points outside {:?}",
                opener.points
            );
            for suit in Suit::ASC {
                let length = hand[suit].len();
                // SAFETY: a suit length is at most 13.
                #[allow(clippy::cast_possible_truncation)]
                let length = length as u8;
                prop_assert!(
                    opener.length(suit).contains(length),
                    "{call} opener with {length} {suit:?} outside {:?}",
                    opener.length(suit)
                );
            }
        }
    }
}
