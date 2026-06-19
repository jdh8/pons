//! What the calls have shown, accumulated across an auction
//!
//! [`Context`] gives the *laws-only* facts of an auction; the authored books
//! and the [instinct floor][super::instinct()] read *system intent* off the
//! calls on demand (see [`Interpretation`][super::instinct()]).  This module is
//! the richest such reading: for every player, the range of cards each suit
//! may hold and the range of points shown, derived **purely from the calls**
//! under standard 2/1 meanings.
//!
//! Two consumers want this summary that the per-bid [`Constraint`][crate::bidding::constraint::Constraint]s cannot
//! give them — a `Constraint` is eval-only, so the length a `len(..)` rule
//! asserts can never be read back out:
//!
//! - the [instinct floor][super::instinct()], so a forced auction can pick a
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
//! The meanings encoded here are those of [`american`][super::american()]
//! (five-card majors, strong 15–17 notrump, strong artificial 2♣); like the
//! instinct floor, this reading is tied to that system.

use super::context::Context;
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};
use std::cell::Cell;

/// The largest a suit length range may span
const LENGTH_CAP: u8 = 13;
/// The largest a point range may span (all forty HCP, then some)
const POINTS_CAP: u8 = 37;

std::thread_local! {
    /// Whether [`Inferences::read`] quantifies a natural notrump raise of our own
    /// 1NT opening (2NT invitational, 3NT game).  On by default; turn it off to
    /// reproduce the pre-fix behaviour where opener was blind to responder's
    /// strength and so could not accept an invitation.  The
    /// [`nt-invite-abc`](../../examples) example A/Bs the two.
    static NT_INVITE_INFERENCE: Cell<bool> = const { Cell::new(true) };
}

/// Toggle reading natural notrump raises of our 1NT opening (default on).
///
/// The fix this gates is what lets opener — and the sampler behind the search
/// floor — know responder is invitational (≈8–9) or game-going (10+), so the
/// keyless floor can judge whether game is good without a hand-authored node.
pub fn set_nt_invite_inference(on: bool) {
    NT_INVITE_INFERENCE.with(|cell| cell.set(on));
}

fn nt_invite_inference() -> bool {
    NT_INVITE_INFERENCE.with(Cell::get)
}

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
/// an auction position through `relative_of` rather than casting.
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

    /// A copy with one player's shown points intersected down to `points`
    ///
    /// Splits a shown range into halves for what-if sampling: narrowing an
    /// opener's points to the upper or lower half of what they have shown lets a
    /// caller deal layouts from each half and ask, double-dummy, whether game is
    /// good opposite a maximum but not a minimum — the meaning of an invitation.
    /// Intersects (never widens), so the result stays within what was shown.
    #[must_use]
    pub fn narrowed_points(&self, who: Relative, points: Range) -> Self {
        let mut copy = *self;
        copy.players[who as usize].points = copy.players[who as usize].points.intersect(points);
        copy
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
        let read_nt_invite = nt_invite_inference();
        // A 1NT–2♣ Stayman auction (opponents silent): opener's major answer and
        // responder's strength are read below so the floor judges the fit and
        // accepts or declines invitations.  The artificial 3OM / Smolen jumps are
        // suppressed from the natural suit reading rather than re-derived.
        let stayman = opening_bid == Bid::new(1, Strain::Notrump)
            && auction.get(opening_index + 2) == Some(&Call::Bid(Bid::new(2, Strain::Clubs)));

        // Suits bid and the count of bids made, per auction lane (`index % 4`);
        // lanes of equal parity are partners, the same side.
        let mut lane_suits = [0u8; 4];
        let mut lane_bids = [0u8; 4];
        let mut side_acted = [false; 2];
        let mut highest: Option<Bid> = None;

        // Rubens advances name relay suits; identify them so the natural reading
        // skips them, and capture a cue-raise's strength to apply afterwards.
        let (rubens_suppress, rubens_cue) = rubens_reading(auction);

        // A Leaping Michaels jump (4♣/4♦ over their weak two) shows two suits, so
        // its natural single-suit reading is suppressed and the pair recorded
        // post-walk (cf. the Rubens cue).
        let leaping_michaels = leaping_michaels_reading(auction);

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
                        // Responder's 3OM slam try and Smolen jumps are
                        // artificial three-level majors in a new suit (partner
                        // never bid it); never read them as a natural long suit.
                        let stayman_artificial = stayman
                            && is_opening_side
                            && lane != opener_lane
                            && lane_bids[lane] >= 1
                            && bid.level.get() == 3
                            && matches!(bid.strain, Strain::Hearts | Strain::Spades)
                            && lane_suits[(lane + 2) % 4] & (1u8 << suit as u8) == 0;
                        let suppress = (is_opening_side && opening_artificial && !over_one_notrump)
                            || stayman_artificial
                            || nt_structure_artificial(auction, index, opening_index)
                            || rubens_suppress.contains(&Some(index))
                            || leaping_michaels.is_some_and(|(i, _, _)| i == index);

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

                    // Strength shown by limited natural rebids and raises, read
                    // only when the opponents have stayed silent (a competitive
                    // 2NT or raise can be off-meaning).  Every branch narrows by
                    // a sound bound — the true point count always falls within.
                    if index != opening_index && !side_acted[defending_parity] {
                        let responder_lane = (opener_lane + 2) % 4;
                        let opener_rebid =
                            is_opening_side && lane == opener_lane && lane_bids[lane] == 1;
                        let responder_first =
                            is_opening_side && lane == responder_lane && lane_bids[lane] == 0;
                        let opening_one_suit =
                            opening_bid.level.get() == 1 && opening_bid.strain.is_suit();

                        if read_nt_invite
                            && bid.strain == Strain::Notrump
                            && opening_bid == Bid::new(1, Strain::Notrump)
                            && responder_first
                        {
                            // Responder's notrump action over our 1NT opening.
                            // 2NT is now the diamond transfer (5+ diamonds), not a
                            // points raise; 3NT still forces game (9+).  Stayman,
                            // the major transfers, and the two-way 2♠ are
                            // artificial and stay silent.  This is what lets opener
                            // (or the sampler behind the search floor) judge
                            // responder.
                            match bid.level.get() {
                                2 => players[who]
                                    .narrow_length(Suit::Diamonds, Range::at_least(5, LENGTH_CAP)),
                                3 => players[who].narrow_points(Range::at_least(9, POINTS_CAP)),
                                _ => {}
                            }
                        } else if bid.strain == Strain::Notrump && opening_one_suit {
                            if opener_rebid {
                                // A balanced rebid.  1NT is a minimum (12–16: a
                                // 17 would open the strong notrump); a *jump* to
                                // 2NT is the strong 18–19 rebid.  A non-jump 2NT
                                // (over a two-level response) is a minimum and is
                                // left to the opening's bound.
                                let nt_jump = bid
                                    .level
                                    .get()
                                    .saturating_sub(cheapest_level(highest, Strain::Notrump));
                                if bid.level.get() == 1 {
                                    players[who].narrow_points(Range::new(12, 16));
                                } else if bid.level.get() == 2 && nt_jump >= 1 {
                                    players[who].narrow_points(Range::new(18, 21));
                                }
                            } else if responder_first && bid.level.get() == 1 {
                                // A 1NT response: a natural or forcing notrump.
                                players[who].narrow_points(Range::new(6, 12));
                            }
                        } else if let Some(suit) = bid.strain.suit() {
                            // Responder raising opener's suit shows limited
                            // support strength: a single raise constructive, a
                            // jump raise invitational.
                            let partner_bid_it =
                                lane_suits[(lane + 2) % 4] & (1 << suit as u8) != 0;
                            if responder_first && partner_bid_it {
                                let jump = bid
                                    .level
                                    .get()
                                    .saturating_sub(cheapest_level(highest, bid.strain));
                                match jump {
                                    0 => players[who].narrow_points(Range::new(6, 10)),
                                    1 => players[who].narrow_points(Range::new(10, 12)),
                                    _ => {}
                                }
                            }
                        }
                    }

                    // Stayman: read opener's major answer and responder's
                    // strength (opponents silent) so the floor judges the fit and
                    // accepts or declines invitations.
                    if stayman && is_opening_side && !side_acted[defending_parity] {
                        let responder_lane = (opener_lane + 2) % 4;
                        if index == opening_index + 2 {
                            // Responder's 2♣ Stayman shows invitational+ values.
                            players[who].narrow_points(Range::at_least(8, POINTS_CAP));
                        } else if index == opening_index + 4 && lane == opener_lane {
                            // Opener's answer names or denies a four-card major.
                            match bid.strain {
                                Strain::Hearts => players[who]
                                    .narrow_length(Suit::Hearts, Range::at_least(4, LENGTH_CAP)),
                                Strain::Spades => {
                                    players[who].narrow_length(
                                        Suit::Spades,
                                        Range::at_least(4, LENGTH_CAP),
                                    );
                                    players[who].narrow_length(Suit::Hearts, Range::new(0, 3));
                                }
                                Strain::Diamonds => {
                                    players[who].narrow_length(Suit::Hearts, Range::new(0, 3));
                                    players[who].narrow_length(Suit::Spades, Range::new(0, 3));
                                }
                                _ => {}
                            }
                        } else if index == opening_index + 6 && lane == responder_lane {
                            // Responder's invitational continuations pin strength
                            // for opener's accept/decline; game and quantitative
                            // calls speak for themselves.
                            let raise_of_major = bid
                                .strain
                                .suit()
                                .is_some_and(|s| lane_suits[opener_lane] & (1u8 << s as u8) != 0);
                            match (bid.level.get(), bid.strain) {
                                (2, Strain::Notrump) => {
                                    players[who].narrow_points(Range::new(8, 9))
                                }
                                (3, Strain::Notrump) => {
                                    players[who].narrow_points(Range::at_least(9, POINTS_CAP));
                                }
                                (3, s) if s.is_suit() && raise_of_major => {
                                    players[who].narrow_points(Range::new(8, 9));
                                }
                                _ => {}
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

        // A two-level cue-raise shows a limit-plus raise: three-plus cards in
        // partner's overcall and opening values.  Recorded after the walk (the
        // cue itself named the opponents' suit, suppressed above).
        if let Some((cue_index, overcall_suit)) = rubens_cue {
            let who = relative_of(len, cue_index) as usize;
            players[who].narrow_length(overcall_suit, Range::at_least(3, LENGTH_CAP));
            players[who].narrow_points(Range::at_least(10, POINTS_CAP));
        }

        // A completed Jacoby major transfer over our own strong notrump shows
        // responder's suit; a follow-up jump to game or raise of the transferred
        // suit upgrades it to a six-card suit (and pins invitational strength).
        // The generic walk suppresses the artificial transfer and its
        // completion, so derive this here (soundness over tightness, as with the
        // Rubens advances above).
        if let Some((responder_index, major, min_length, points)) =
            transfer_major_reading(auction, opening_index)
        {
            let who = relative_of(len, responder_index) as usize;
            players[who].narrow_length(major, Range::at_least(min_length, LENGTH_CAP));
            if let Some(points) = points {
                players[who].narrow_points(points);
            }
        }

        // A Leaping Michaels overcall of a weak two shows a 5-5 two-suiter with
        // game-forcing values (point_count ≥ 14, matching the overcall gate).
        // Over 2♦, 4♣'s major is unknown, so only clubs is pinned.  The jump's
        // natural single-suit reading was suppressed above so this records the
        // pair — the signal the search sampler needs to condition partner.
        if let Some((overcall_index, primary, secondary)) = leaping_michaels {
            let who = relative_of(len, overcall_index) as usize;
            players[who].narrow_length(primary, Range::at_least(5, LENGTH_CAP));
            if let Some(secondary) = secondary {
                players[who].narrow_length(secondary, Range::at_least(5, LENGTH_CAP));
            }
            players[who].narrow_points(Range::at_least(14, POINTS_CAP));
        }

        Self { players }
    }
}

/// Whether the call at `index` is an artificial relay/puppet/splinter in the
/// Puppet-Stayman or minor-suit-transfer structures over our 1NT opening — so it
/// must not be read as a natural long suit
///
/// Once responder enters a new structure (a 3♣ Puppet, 2NT diamond transfer, or
/// 2♠ relay as their first call), every later three-level suit bid by our side is
/// an artificial relay or splinter — except opener's genuine five-card major show
/// over Puppet (`1NT–3♣–3♥/3♠`).  Positions assume the standard uncontested
/// auction; a contested one shifts them and matches none.
fn nt_structure_artificial(auction: &[Call], index: usize, opening_index: usize) -> bool {
    let resp_first = auction.get(opening_index + 2);
    let entered = matches!(
        resp_first,
        Some(&Call::Bid(b))
            if b == Bid::new(3, Strain::Clubs)
                || b == Bid::new(2, Strain::Notrump)
                || b == Bid::new(2, Strain::Spades)
    );
    if !entered {
        return false;
    }
    // Opener's natural five-card major show over Puppet stays a real suit.
    let opener_puppet_major = index == opening_index + 4
        && resp_first == Some(&Call::Bid(Bid::new(3, Strain::Clubs)))
        && matches!(
            auction.get(index),
            Some(&Call::Bid(b))
                if b.level.get() == 3 && matches!(b.strain, Strain::Hearts | Strain::Spades)
        );
    !opener_puppet_major
}

/// Whether `bid` is higher than the standing `highest` contract
///
/// Bridge contracts rank by level first, then strain — `2♣` outranks `1♠`.
fn outranks(bid: Bid, highest: Bid) -> bool {
    bid.level.get() > highest.level.get()
        || (bid.level.get() == highest.level.get() && bid.strain > highest.strain)
}

/// The cheapest level a strain can be bid over the standing `highest` contract
const fn cheapest_level(highest: Option<Bid>, strain: Strain) -> u8 {
    match highest {
        None => 1,
        Some(h) if strain as u8 > h.strain as u8 => h.level.get(),
        Some(h) => h.level.get() + 1,
    }
}

/// The Rubens-artificial calls of an advance, and a cue-raise's strength reading
///
/// In a [Rubens advance][super::instinct::overcall_shape] of a simple overcall,
/// some calls *name* a suit they do not *hold*: the advancer's transfer (a relay
/// to the next suit up) or cue-raise, and the overcaller's forced completion.
/// Returns `(suppress, cue)` — `suppress` lists those indices, whose bid suit
/// must not be read as natural length; `cue` is `(index, Y)` of a two-level
/// cue-raise, read separately as a limit-plus raise (three-plus cards in
/// partner's overcall `Y`, ten-plus points).
///
/// A new-suit transfer's target is the *advancer's* own suit and the completion
/// is a forced relay, so neither is read as length (soundness over tightness, as
/// with transfers over our own notrump).  The cue-raise's *strength*, by
/// contrast, is what lets the overcaller reach game, so it is recorded.
fn rubens_reading(auction: &[Call]) -> ([Option<usize>; 2], Option<(usize, Suit)>) {
    let none = ([None, None], None);
    let Some((x, y, overcall_index, level)) = super::instinct::overcall_shape(auction) else {
        return none;
    };
    // The advance comes after the overcaller's partner (RHO of the overcaller)
    // passes; the advancer's call sits two past the overcall.
    if auction.get(overcall_index + 1) != Some(&Call::Pass) {
        return none;
    }
    let advance_index = overcall_index + 2;
    let Some(&Call::Bid(advance)) = auction.get(advance_index) else {
        return none;
    };
    if level == 2 {
        // Two-level overcall: the cue-raise (2X) is the lone artificial call.
        return if advance == Bid::new(2, Strain::from(x)) {
            ([Some(advance_index), None], Some((advance_index, y)))
        } else {
            none
        };
    }
    // One-level overcall: a transfer 2S (X ≤ S < Y), then the completion 2(S+1).
    let Some(source) = advance.strain.suit() else {
        return none;
    };
    if advance.level.get() != 2 || (source as u8) < (x as u8) || (source as u8) >= (y as u8) {
        return none;
    }
    let target = Strain::from(Suit::ASC[(source as u8 + 1) as usize]);
    let completion = (auction.get(advance_index + 1) == Some(&Call::Pass)
        && auction.get(advance_index + 2) == Some(&Call::Bid(Bid::new(2, target))))
    .then_some(advance_index + 2);
    ([Some(advance_index), completion], None)
}

/// What a Leaping Michaels jump shows about the overcaller
///
/// Returns `(overcall_index, primary, secondary)`: the overcaller holds five-plus
/// in `primary` (and in `secondary` when known), game-forcing values.  Over a
/// major the jump is a minor + the *other* major (both known); over `2♦` the `4♦`
/// cue shows both majors, while `4♣` shows clubs + an *unknown* major, so only
/// clubs is pinned (`secondary` is `None`).  The jump's natural single-suit
/// reading is suppressed in the walk so the pair is recorded post-walk — mirrors
/// [`rubens_reading`].
///
/// Returns `None` unless Leaping Michaels is enabled *and* the auction is a weak
/// two followed by the defending side's first action being a `4♣`/`4♦` jump, so a
/// natural four-level bid is never mistaken for the convention.
fn leaping_michaels_reading(auction: &[Call]) -> Option<(usize, Suit, Option<Suit>)> {
    if !crate::bidding::american::leaping_michaels_enabled() {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    let Call::Bid(opening) = auction[opening_index] else {
        return None;
    };
    let theirs = opening.strain.suit()?;
    // A weak two (2♦/2♥/2♠); 2♣ is the strong artificial opening, not a weak two.
    if opening.level.get() != 2 || theirs == Suit::Clubs {
        return None;
    }
    // The overcall is the defending side's *first* action: a jump to 4♣ or 4♦.
    let opener_parity = opening_index % 2;
    for (index, &call) in auction.iter().enumerate().skip(opening_index + 1) {
        match call {
            Call::Pass => {}
            Call::Bid(bid) if index % 2 != opener_parity => {
                let lm = bid.strain.suit()?;
                if bid.level.get() != 4 || !matches!(lm, Suit::Clubs | Suit::Diamonds) {
                    return None;
                }
                let (primary, secondary) = leaping_michaels_suits(theirs, lm);
                return Some((index, primary, secondary));
            }
            // The defending side did something else first — not a Leaping Michaels.
            _ => return None,
        }
    }
    None
}

/// The suit(s) a Leaping Michaels jump `lm` (clubs or diamonds) shows over their
/// weak two `theirs`, as `(primary, secondary)`
fn leaping_michaels_suits(theirs: Suit, lm: Suit) -> (Suit, Option<Suit>) {
    match theirs {
        // Over a major: lm + the OTHER major.
        Suit::Hearts => (lm, Some(Suit::Spades)),
        Suit::Spades => (lm, Some(Suit::Hearts)),
        // Over 2♦: 4♦ cue = both majors; 4♣ = clubs + an unknown major.
        Suit::Diamonds if lm == Suit::Diamonds => (Suit::Hearts, Some(Suit::Spades)),
        Suit::Diamonds => (Suit::Clubs, None),
        Suit::Clubs => (lm, None),
    }
}

/// The bid at `index`, if the call there is a bid (not a pass/double/redouble)
fn bid_at(auction: &[Call], index: usize) -> Option<Bid> {
    match auction.get(index) {
        Some(&Call::Bid(bid)) => Some(bid),
        _ => None,
    }
}

/// What a completed Jacoby *major* transfer over our own strong notrump shows
/// about responder, returned as `(responder_index, major, min_length, points)`
///
/// The generic walk suppresses the artificial transfer and its completion, so
/// this reads them after the fact:
///
/// - a completed transfer shows responder holds **five-plus** in the major;
/// - a follow-up raise of the transferred suit (`1NT–2♦–2♥–3♥`) or jump to game
///   (`…–4♥`, or `2NT–3♦–3♥–4♥`) shows **six-plus** — responder bypassed the
///   choice-of-games `3NT` (which would be exactly five) for a known long suit;
/// - the invitational `3M` raise also pins invitational strength (8–9, the same
///   bound the Stayman reading uses for a major raise).
///
/// South African Texas is a direct transfer to game (no choice of games to
/// bypass) and the minor transfers are out of scope, so neither is read here.
/// Positions assume the standard uncontested auction (opponents passing); a
/// contested one shifts them and matches none — those continuations fall outside
/// the floor's natural-only scope.
fn transfer_major_reading(
    auction: &[Call],
    opening_index: usize,
) -> Option<(usize, Suit, u8, Option<Range>)> {
    // Each `(opening, responder's transfer, opener's completion)`; game in the
    // major is always the four level.
    const MAJORS: [(Bid, Bid, Bid); 4] = [
        (
            Bid::new(1, Strain::Notrump),
            Bid::new(2, Strain::Diamonds),
            Bid::new(2, Strain::Hearts),
        ),
        (
            Bid::new(1, Strain::Notrump),
            Bid::new(2, Strain::Hearts),
            Bid::new(2, Strain::Spades),
        ),
        (
            Bid::new(2, Strain::Notrump),
            Bid::new(3, Strain::Diamonds),
            Bid::new(3, Strain::Hearts),
        ),
        (
            Bid::new(2, Strain::Notrump),
            Bid::new(3, Strain::Hearts),
            Bid::new(3, Strain::Spades),
        ),
    ];

    let opening = bid_at(auction, opening_index)?;
    // The opponents must stay silent for these positions to hold.
    if auction.get(opening_index + 1) != Some(&Call::Pass) {
        return None;
    }
    let transfer = bid_at(auction, opening_index + 2)?;
    let &(_, _, completion) = MAJORS
        .iter()
        .find(|&&(o, t, _)| o == opening && t == transfer)?;
    let major = completion.strain.suit()?;
    let responder_index = opening_index + 2;

    // The transfer alone shows a five-card major.
    let mut min_length = 5;
    let mut points = None;

    // Opener's completion, then responder's continuation, each after a pass.
    if auction.get(opening_index + 3) == Some(&Call::Pass)
        && bid_at(auction, opening_index + 4) == Some(completion)
        && auction.get(opening_index + 5) == Some(&Call::Pass)
        && let Some(follow) = bid_at(auction, opening_index + 6)
    {
        let raise_to_three = follow == Bid::new(3, completion.strain);
        let jump_to_game = follow == Bid::new(4, completion.strain);
        if raise_to_three || jump_to_game {
            min_length = 6;
        }
        if raise_to_three {
            // Invitational, like the Stayman major raise.
            points = Some(Range::new(8, 9));
        }
    }

    Some((responder_index, major, min_length, points))
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
            // The opening gates on `fifths(15.0..18.0)`, which downgrades
            // quack-heavy hands (K/Q worth less than HCP).  Since
            // `hcp - fifths = 0.2·(#K+#Q) - 0.4·(#T) ≤ 1.6` and a balanced
            // hand's `point_count` is its raw HCP, a 1NT opener can hold up to
            // 19 points (e.g. ♠KQJx ♥KQx ♦KQx ♣Kxx: 19 HCP, 17.6 fifths).
            inf.narrow_points(Range::new(14, 19));
        }
        (2, Strain::Clubs) => {
            // Strong and artificial: 22+ points, but nothing about shape.
            inf.narrow_points(Range::at_least(20, POINTS_CAP));
        }
        (2, Strain::Notrump) => {
            balanced(inf);
            // As with 1NT: `fifths(20.0..22.0)` admits a quack-heavy 23-count
            // (fifths within 1.6 of raw HCP), so the sound point envelope is
            // 19–23, not 19–22.
            inf.narrow_points(Range::new(19, 23));
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
        2 if is_american(opening, response) => {
            inf.narrow_points(Range::at_least(13, POINTS_CAP));
        }
        _ => {}
    }
}

/// Whether a two-level new suit is a game-forcing 2/1 over `opening`
fn is_american(opening: Bid, response: Bid) -> bool {
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
        assert_eq!(one_nt.rho().points, Range::new(14, 19));

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
    fn leaping_michaels_conditions_partner() {
        use crate::bidding::american::set_leaping_michaels;

        // (2♥)–4♣–(P): the advancer reads partner's two-suiter — five-plus clubs
        // AND five-plus spades, game-forcing — so the search sampler deals partner
        // the right shape rather than a natural club one-suiter.
        set_leaping_michaels(true);
        let advance = read(&[bid(2, Strain::Hearts), bid(4, Strain::Clubs), Call::Pass]);
        assert_eq!(advance.partner().length(Suit::Clubs), Range::new(5, 13));
        assert_eq!(advance.partner().length(Suit::Spades), Range::new(5, 13));
        assert_eq!(advance.partner().points, Range::new(14, 37));

        // Over 2♦, the 4♦ cue shows both majors; 4♣ shows clubs + an unknown
        // major, so only clubs is pinned.
        let cue = read(&[
            bid(2, Strain::Diamonds),
            bid(4, Strain::Diamonds),
            Call::Pass,
        ]);
        assert_eq!(cue.partner().length(Suit::Hearts), Range::new(5, 13));
        assert_eq!(cue.partner().length(Suit::Spades), Range::new(5, 13));

        // Disabled (the default): a 4♣ jump reads as a natural one-suiter, so
        // spades stay unconstrained — the convention must not leak when off.
        set_leaping_michaels(false);
        let off = read(&[bid(2, Strain::Hearts), bid(4, Strain::Clubs), Call::Pass]);
        assert_eq!(off.partner().length(Suit::Spades), Range::FULL_LENGTH);
    }

    #[test]
    fn narrowed_points_intersects_one_player() {
        // 1NT shows 14-19; narrow the opener (here our RHO) to the upper half.
        let inf = read(&[bid(1, Strain::Notrump)]);
        assert_eq!(inf.rho().points, Range::new(14, 19));

        let upper = inf.narrowed_points(Relative::Rho, Range::new(17, 19));
        assert_eq!(
            upper.rho().points,
            Range::new(17, 19),
            "narrowed to the half"
        );
        assert_eq!(inf.rho().points, Range::new(14, 19), "original unchanged");
        // Shape and the other players are untouched.
        assert_eq!(
            upper.rho().length(Suit::Spades),
            inf.rho().length(Suit::Spades)
        );
        assert_eq!(upper.partner().points, inf.partner().points);

        // Intersection, not replacement: a wider request cannot widen what was shown.
        let clamped = inf.narrowed_points(Relative::Rho, Range::new(0, POINTS_CAP));
        assert_eq!(clamped.rho().points, Range::new(14, 19));
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
    fn completed_major_transfer_shows_five() {
        // [1NT, P, 2♦, P, 2♥, P]: partner transferred to hearts and we
        // completed; at length 6 the responder is us (Me).  The transfer shows a
        // five-card major even before a jump confirms the sixth, while the
        // transferred-*from* suit stays unread.
        let auction = [
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ];
        let inf = read(&auction);
        assert_eq!(inf.me().length(Suit::Hearts), Range::new(5, 13));
        assert_eq!(inf.me().length(Suit::Diamonds), Range::FULL_LENGTH);
    }

    #[test]
    fn transfer_jump_to_game_shows_six() {
        // [1NT, P, 2♦, P, 2♥, P, 4♥, P]: partner transferred then jumped past
        // 3NT to 4♥, showing a six-card major (the M6.1 canonical case).  At
        // length 8 the responder sits as Partner.
        let auction = [
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ];
        let inf = read(&auction);
        assert_eq!(inf.partner().length(Suit::Hearts), Range::new(6, 13));
    }

    #[test]
    fn transfer_then_three_major_invites_with_six() {
        // [1NT, P, 2♦, P, 2♥, P, 3♥, P]: a raise of the transferred suit is
        // invitational with a six-card major.
        let auction = [
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(3, Strain::Hearts),
            Call::Pass,
        ];
        let inf = read(&auction);
        assert_eq!(inf.partner().length(Suit::Hearts), Range::new(6, 13));
        assert_eq!(inf.partner().points, Range::new(8, 9));
    }

    #[test]
    fn transfer_major_reading_covers_spades_and_two_notrump() {
        // Spade transfer (2♥ → 2♠) jumped to 4♠.
        let spades = read(&[
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Spades),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ]);
        assert_eq!(spades.partner().length(Suit::Spades), Range::new(6, 13));

        // The same shape over a 2NT opening (3♦ → 3♥, jump 4♥).
        let two_nt = read(&[
            bid(2, Strain::Notrump),
            Call::Pass,
            bid(3, Strain::Diamonds),
            Call::Pass,
            bid(3, Strain::Hearts),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ]);
        assert_eq!(two_nt.partner().length(Suit::Hearts), Range::new(6, 13));
    }

    #[test]
    fn contested_transfer_auction_is_not_specially_read() {
        // [1NT, 2♣, 2♦, P, 2♥, P, 4♥, P]: with the opponents in, the transfer
        // positions shift, so the special reading must not pin a six-card suit.
        let auction = [
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ];
        let inf = read(&auction);
        assert!(inf.partner().length(Suit::Hearts).min < 6);
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
    fn limited_notrump_rebids_narrow_strength() {
        // [1♦, P, 1♥, P, 1NT, P]: the opener (partner) showed a 12–16 minimum.
        let one_nt = read(&[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(1, Strain::Notrump),
            Call::Pass,
        ]);
        assert_eq!(one_nt.partner().points, Range::new(12, 16));

        // A jump to 2NT is the strong 18–19 rebid (sound bound 18–21).
        let two_nt = read(&[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Notrump),
            Call::Pass,
        ]);
        assert_eq!(two_nt.partner().points, Range::new(18, 21));
    }

    #[test]
    fn cheapest_two_notrump_over_a_response_is_not_strong() {
        // [1♦, P, 2♣, P, 2NT, P]: 2NT is the *cheapest* notrump over a 2/1, a
        // minimum — it must not be read as the 18–19 jump.  Opener stays 12–21.
        let inf = read(&[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Notrump),
            Call::Pass,
        ]);
        assert_eq!(inf.partner().points, Range::new(12, 21));
    }

    #[test]
    fn raises_and_one_notrump_response_narrow_the_responder() {
        // [1♥, P, 2♥, P]: a single raise is 6–10.
        let single = read(&[
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ]);
        assert_eq!(single.partner().points, Range::new(6, 10));
        // [1♥, P, 3♥, P]: a limit (jump) raise is 10–12.
        let limit = read(&[
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(3, Strain::Hearts),
            Call::Pass,
        ]);
        assert_eq!(limit.partner().points, Range::new(10, 12));
        // [1♥, P, 1NT, P]: a 1NT response is 6–12.
        let one_nt = read(&[
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(1, Strain::Notrump),
            Call::Pass,
        ]);
        assert_eq!(one_nt.partner().points, Range::new(6, 12));
    }

    #[test]
    fn competition_suppresses_the_limited_rebid_reading() {
        // [1♦, P, 1♥, 1♠, 1NT, P]: with the opponents in, opener's 1NT is not
        // the quiet 12–16 rebid — leave the strength at the opening's 12–21.
        let inf = read(&[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Hearts),
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            Call::Pass,
        ]);
        assert_eq!(inf.partner().points, Range::new(12, 21));
    }

    #[test]
    fn rubens_cue_raise_shows_support() {
        // (1♠) 2♣ (P) 2♠ (P): we overcalled 2♣, partner cue-raised 2♠ — a
        // limit-plus club raise.  The overcaller reads three-plus clubs and
        // ten-plus points, but no spade length (the cue is a relay).
        let inf = read(&[
            bid(1, Strain::Spades),
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Spades),
            Call::Pass,
        ]);
        assert!(inf.partner().length(Suit::Clubs).min >= 3);
        assert!(inf.partner().points.min >= 10);
        assert_eq!(inf.partner().length(Suit::Spades), Range::FULL_LENGTH);
    }

    #[test]
    fn rubens_transfer_is_not_read_as_natural() {
        // (1♣) 1♠ (P) 2♣ (P): we overcalled 1♠, partner transferred 2♣ (a relay
        // to diamonds).  The bid suit must not be read as a club holding.
        let inf = read(&[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ]);
        assert_eq!(inf.partner().length(Suit::Clubs), Range::FULL_LENGTH);
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
            use crate::bidding::american::openings;
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
