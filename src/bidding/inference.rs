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

std::thread_local! {
    /// Whether [`project_authored`] treats a call as artificial because its
    /// authoring rule carries an [`Alert`][crate::bidding::Alert], on top of the
    /// structural [`artificial`] test.  On by default; turning it off recovers the
    /// pre-alert behaviour where a strength-showing artificial that floors no
    /// foreign suit — the strong 2♣ opening, its 2♦ waiting / 2♥ double negative,
    /// Puppet 3♣ — was misread as a natural suit.  The `ab-alert-reading` example
    /// A/Bs the two.
    static ALERT_READING: Cell<bool> = const { Cell::new(true) };
}

/// Toggle reading an alerted call as artificial (default on).
///
/// This is the per-call defense switch: with it on, the floor recognises every
/// alerted convention — including the strength-showing artificials the structural
/// detector misses — and reads it as the convention rather than as a natural suit,
/// so a player switches its treatment the moment an opponent's alerted call lands.
pub fn set_alert_reading(on: bool) {
    ALERT_READING.with(|cell| cell.set(on));
}

fn alert_reading() -> bool {
    ALERT_READING.with(Cell::get)
}

std::thread_local! {
    /// Whether the layout sampler accepts a candidate hand by *replaying the
    /// rule* — re-running the policy at each prior decision node and keeping the
    /// hand only if the policy would have made the call the player actually made
    /// — instead of projecting the auction into the hand-written [`Inferences`]
    /// ranges.  Off by default; the `ab-landy` example A/Bs the two.  See
    /// [`sample_layouts_replay`][super::sampler::sample_layouts_replay].
    static RULE_ACCEPT: Cell<bool> = const { Cell::new(false) };
}

/// Toggle rule-replay layout acceptance (default off).
///
/// On, the sampler reads each bid by the rule that authored it — the meaning is
/// frozen at the node, surviving later competition — rather than by the
/// per-convention range readers.  Measured on `ab-landy`; see the plan.
pub fn set_rule_accept(on: bool) {
    RULE_ACCEPT.with(|cell| cell.set(on));
}

/// Whether rule-replay layout acceptance is enabled (default off).
#[must_use]
pub fn rule_accept_enabled() -> bool {
    RULE_ACCEPT.with(Cell::get)
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

    /// The disjunction of two ranges — the loosest bounds spanning both
    ///
    /// A hand satisfying *either* of two alternatives (an `Or` projection of a
    /// [`Constraint`][super::constraint::Constraint]) has its quantity in one
    /// range or the other, so the sound envelope is their span.  The dual of
    /// [`intersect`][Self::intersect], which keeps the tighter bounds.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
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

    /// Pointwise intersection — the `&` projection (both sets of bounds hold)
    ///
    /// The forward dual of a constraint conjunction: a hand accepted by `a & b`
    /// lies within both envelopes, so each quantity takes the tighter bounds
    /// ([`Range::intersect`]).
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Self {
        let mut out = *self;
        for suit in Suit::ASC {
            out.narrow_length(suit, other.length(suit));
        }
        out.narrow_points(other.points);
        out
    }

    /// Pointwise union — the `|` projection (either set of bounds may hold)
    ///
    /// The forward dual of a constraint disjunction: a hand accepted by `a | b`
    /// lies within one envelope or the other, so each quantity spans both
    /// ([`Range::union`]) — soundness over tightness.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        let mut out = *self;
        for suit in Suit::ASC {
            out.lengths[suit as usize] = out.length(suit).union(other.length(suit));
        }
        out.points = out.points.union(other.points);
        out
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
pub(crate) const fn relative_of(len: usize, index: usize) -> Relative {
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

        // The three declarative conventions — Jacoby transfers over our notrump,
        // Leaping Michaels, and Landy's 2♣ — are read straight off their authored
        // rule's projection rather than re-derived by hand (M6.2c).  `overlay`
        // records each artificial call's projected shape (applied post-walk);
        // `suppressed` is a bitset of the indices whose natural single-suit reading
        // the walk must skip.
        let (overlay, suppressed) = project_authored(context);
        // The one suppression the projection cannot see: the advancer's 2♦ relay /
        // 2♥-2♠ preference over a Landy/Woolsey both-majors 2♣ names no length of its
        // own, so its rule projects nothing — suppress it by hand (the doc's stub).
        let landy_relay = landy_advance_suppress(auction);
        // The Woolsey Multi family: 2♦ (a single 6+ major — its diamond reading
        // suppressed) and the 2♥/2♠ Muiderberg, recorded post-walk.
        let multi = multi_reading(auction);
        // The Woolsey takeout double of their 1NT: the doubler's points are recorded
        // post-walk and the advancer's 2♣ minor relay is suppressed.
        let woolsey_x = woolsey_x_reading(auction);
        // The DONT defense of their 1NT: the artificial X/2♣/2♦/2♥ and the advancer's
        // relay are suppressed; what each genuinely shows is recorded post-walk.
        let dont = dont_reading(auction);
        // Our natural penalty double of their 1NT (15+): a double names no suit, so the
        // generic walk reads it as nothing — the points floor is recorded post-walk.
        let penalty_x = penalty_x_reading(auction);
        // The latch's subsequent penalty doubles: each promises four-plus in the suit
        // it doubles, recorded post-walk so the sampler does not read them as takeout.
        let penalty_latch_doubles = penalty_latch_double_reading(auction);
        // Responder's double of an overcall of our 1NT shows 8+ (every DoubleStyle),
        // recorded post-walk so opener does not undercount the partnership's strength.
        let overcall_double = responder_overcall_double_reading(auction, len);

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
                            || (index < 64 && suppressed >> index & 1 != 0)
                            || landy_relay == Some(index)
                            || multi.is_some_and(|m| m.suppresses(index))
                            || woolsey_x.is_some_and(|w| w.suppresses(index))
                            || dont.is_some_and(|d| d.suppresses(index));

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
                            // 3NT forces game (9+) in both minor schemes; the 2NT
                            // meaning is scheme-dependent — Puppet's 2NT is the
                            // diamond transfer (5+ diamonds), European's is a
                            // balanced invitational ~8 (the size ask).  Stayman, the
                            // major transfers, and the artificial minor calls
                            // (Puppet 2♠/3♣, European 2♠ clubs / 3♣ diamonds) stay
                            // silent here — `project_authored` narrows the single
                            // suits.  This is what lets opener (or the sampler behind
                            // the search floor) judge responder.
                            match bid.level.get() {
                                2 => {
                                    if crate::bidding::american::notrump_minors()
                                        == crate::bidding::american::EUROPEAN
                                    {
                                        players[who].narrow_points(Range::new(8, 9));
                                    } else {
                                        players[who].narrow_length(
                                            Suit::Diamonds,
                                            Range::at_least(5, LENGTH_CAP),
                                        );
                                    }
                                }
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
                                    players[who].narrow_points(Range::new(8, 9));
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

        // The three declarative conventions (Jacoby transfers over our notrump,
        // Leaping Michaels, Landy's 2♣) are recorded from their authored rule's
        // projection — the `overlay` computed above — not a hand-written decoder
        // (M6.2c).  Sound but looser than the old readers: it pins the 2♦ transfer's
        // five-card floor, not the six-card jump upgrade the reader inferred from a
        // later call.  The DONT/Woolsey/Multi conventions below are now transparent
        // `or`/`and` shapes too (M6.2d), so the both-majors family (DONT `2♥`, the
        // direct-Landy `X`) also surfaces in `overlay` here — redundant with, and
        // identical to, the per-suit floors recorded by hand below (an idempotent
        // intersect).  The hand recordings stay: they carry the one-suiter/disjunction
        // floors the `or`-union washes out, which the projection cannot pin.
        for (seat, projected) in overlay.iter().enumerate() {
            players[seat] = players[seat].intersect(projected);
        }

        // A Woolsey Multi-family overcall.  The "6+ major" (2♦) and "4+ minor"
        // (2♥/2♠) are disjunctions the per-suit framework cannot pin to one suit, so
        // they are captured by the *residual*: capping the other three suits forces
        // the sampler to deal the length into the long suit (the same loose handling
        // Landy uses for its 5-4).
        if let Some(multi) = multi {
            let who = relative_of(len, multi.overcall_index) as usize;
            match multi.kind {
                // 2♦ Multi: a true one-suiter, so both minors ≤ 4 (the natural ≥5
                // diamond reading was suppressed above; clubs narrows from full).
                MultiKind::Major => {
                    players[who].narrow_length(Suit::Clubs, Range::new(0, 4));
                    players[who].narrow_length(Suit::Diamonds, Range::new(0, 4));
                }
                // 2♥/2♠ Muiderberg: exactly 5 in the major, ≤ 3 in the other major
                // (refining the natural ≥5 reading); the 4+ minor falls out of the
                // residual.
                MultiKind::Muiderberg(major) => {
                    let other = if major == Suit::Hearts {
                        Suit::Spades
                    } else {
                        Suit::Hearts
                    };
                    players[who].narrow_length(major, Range::new(5, 5));
                    players[who].narrow_length(other, Range::new(0, 3));
                }
            }
            let floor = crate::bidding::american::woolsey_points().0;
            players[who].narrow_points(Range::at_least(floor, POINTS_CAP));
        }

        // A Woolsey takeout double of their 1NT (4-card major + 5-6 card minor).  The
        // shape is a double disjunction the per-suit framework cannot pin, so only the
        // points floor is recorded — enough to stop the floor sampling the doubler as a
        // random weak hand (a double of 1NT is otherwise read as nothing).
        if let Some(woolsey_x) = woolsey_x {
            let who = relative_of(len, woolsey_x.double_index) as usize;
            let floor = crate::bidding::american::woolsey_double_floor();
            players[who].narrow_points(Range::at_least(floor, POINTS_CAP));
        }

        // A DONT overcall of their 1NT.  The X one-suiter and the 2♣/2♦ minor are
        // disjunctions (the long suit / the unknown major) the per-suit framework
        // cannot pin, so only the sound per-suit fact is recorded; the residual carries
        // the rest.  The 2♥ both-majors pins both like Landy.  In each case the points
        // floor stops the floor sampling the overcaller as a random hand.
        if let Some(dont) = dont {
            let who = relative_of(len, dont.overcall_index) as usize;
            match dont.kind {
                // One-suiter in ♣/♦/♥ (spades excluded); the long suit falls out of the
                // residual, only spades ≤ 3 is certain.
                DontKind::OneSuiter => players[who].narrow_length(Suit::Spades, Range::new(0, 3)),
                // 2♣/2♦: a real ≥ 4 minor (the natural ≥ 5 reading was suppressed); the
                // unknown major surfaces naturally if later named, else the residual.
                DontKind::ClubsMajor => {
                    players[who].narrow_length(Suit::Clubs, Range::at_least(4, LENGTH_CAP));
                }
                DontKind::DiamondsMajor => {
                    players[who].narrow_length(Suit::Diamonds, Range::at_least(4, LENGTH_CAP));
                }
                // 2♥: both majors, ≥ 4-4 (the natural ≥ 5 heart reading was suppressed).
                DontKind::BothMajors => {
                    players[who].narrow_length(Suit::Hearts, Range::at_least(4, LENGTH_CAP));
                    players[who].narrow_length(Suit::Spades, Range::at_least(4, LENGTH_CAP));
                }
            }
            players[who].narrow_points(Range::at_least(dont.floor, POINTS_CAP));
        }

        // Our natural penalty double of their 1NT.  The shape gate only widens *which*
        // 15+ hands double, so only the points floor is a sound per-call fact; recording
        // it stops the floor sampling the doubler as a random weak hand and the advancer
        // pulling a phantom suit (cf. the Woolsey double, which records points alone too).
        if let Some(double_index) = penalty_x {
            let who = relative_of(len, double_index) as usize;
            let floor = crate::bidding::american::natural_double_floor();
            players[who].narrow_points(Range::at_least(floor, POINTS_CAP));
        }

        // The latch's later penalty doubles: four-plus in the doubled suit (the
        // floor makes them only on a trump stack), so partner reads them as penalty.
        for (double_index, suit) in penalty_latch_doubles {
            let who = relative_of(len, double_index) as usize;
            players[who].narrow_length(suit, Range::at_least(4, LENGTH_CAP));
        }

        // Responder's double of an overcall of our 1NT: 8+ values (every DoubleStyle).
        if let Some(double_index) = overcall_double {
            let who = relative_of(len, double_index) as usize;
            players[who].narrow_points(Range::at_least(8, POINTS_CAP));
        }

        Self { players }
    }
}

/// Project the authored rule of every artificial prior call into [`Inferences`]
///
/// The generic dual of the per-convention `*_reading` decoders (M6.2b): walk the
/// authored nodes the context's trie carries ([`Context::prefixes`]) and, at each,
/// project the rule of the call actually made.  When that projection floors a suit
/// the call did not name, the call is *artificial* — a transfer, a two-suiter, a
/// Landy 2♣ — and its projected shape is recorded against the bidder's relative
/// seat, exactly as the hand-written readers do, but read straight off the rule.
///
/// A keyless context (no prefixes) or an all-natural auction leaves every seat at
/// [`Inference::unknown`], so this is a sound, loose *overlay* — never the natural
/// reading itself (openings, raises, rebids stay in [`Inferences::read`]).
///
/// The projection in isolation: [`Inferences::read`] folds [`project_authored`]
/// directly, so this thin wrapper now serves only the M6.2b equivalence test.
#[cfg(test)]
#[must_use]
pub(crate) fn authored_reading(context: &Context<'_>) -> Inferences {
    Inferences {
        players: project_authored(context).0,
    }
}

/// Project every artificial prior call into a per-seat overlay, plus a bitset of the
/// artificial calls' auction positions
///
/// The shared walk behind both halves of the retired declarative readers, folded
/// into [`Inferences::read`] (M6.2c): the overlay *records* each artificial call's
/// projected shape against the bidder's seat, and the bitset marks which calls to
/// *suppress* from the natural single-suit reading.  A call is artificial when its
/// projection floors a suit it did not name (see [`artificial`]).
///
/// The bitset indexes by auction position; a position past 64 (never reached by a
/// real auction) is simply left unmarked, falling back to the natural reading.
fn project_authored(context: &Context<'_>) -> ([Inference; 4], u64) {
    let auction = context.auction();
    let len = auction.len();
    let mut players = [Inference::unknown(); 4];
    let mut suppressed = 0u64;

    let Some(prefixes) = context.prefixes() else {
        return (players, suppressed);
    };

    for (prefix, classifier) in prefixes.clone() {
        let index = prefix.len();
        let (Some(&made), Some(rules)) = (auction.get(index), classifier.as_rules()) else {
            continue;
        };

        // The logit of a call is the max over its rules, so a hand could satisfy
        // any one of them — the sound forward envelope is their union.
        let projection = rules
            .rules()
            .iter()
            .filter(|rule| rule.call() == made)
            .map(|rule| rule.project(context))
            .reduce(|acc, p| acc.union(&p));

        // A call is artificial when its authoring rule *alerts* it (the explicit,
        // exhaustive signal — it catches strength-showing artificials like the
        // strong 2♣ opening and Puppet 3♣ that floor no foreign suit), or — as a
        // fallback for any artificial call not yet alerted — when its projection
        // floors a suit it did not name (see [`artificial`]).  The union only adds
        // coverage; it never drops a read the structural test already made.
        let alerted = alert_reading()
            && rules
                .rules()
                .iter()
                .any(|rule| rule.call() == made && rule.alert().is_some());

        if let Some(projection) = projection.filter(|p| alerted || artificial(p, made)) {
            let who = relative_of(len, index) as usize;
            players[who] = players[who].intersect(&projection);
            if index < 64 {
                suppressed |= 1 << index;
            }
        }
    }

    (players, suppressed)
}

/// Whether a call's projection floors a suit other than the one it names
///
/// The artificial-call detector, falling out of the projection itself: a natural
/// bid floors its own strain (1♠ → 5+♠) or no suit (1NT → points only); an
/// artificial one floors a suit it did not name (Jacoby 2♦ → 5+♥, Landy 2♣ → 4-4
/// majors).  A min-length floor of four-plus on a non-named suit is the witness —
/// above any natural by-product, below every convention's real shape.
fn artificial(projection: &Inference, made: Call) -> bool {
    let named = match made {
        Call::Bid(bid) => bid.strain.suit(),
        _ => None,
    };
    Suit::ASC
        .into_iter()
        .any(|suit| Some(suit) != named && projection.length(suit).min >= 4)
}

/// Whether the call at `index` is an artificial relay/puppet/splinter in the
/// minor-suit-response structure over our 1NT opening — so it must not be read as a
/// natural long suit
///
/// Once responder enters a structure as their first call, every later three-level
/// suit bid by our side is an artificial relay or splinter.  Which first calls
/// enter, and the lone exception, depend on the active minor scheme
/// ([`notrump_minors`][crate::bidding::american::notrump_minors]):
///
/// - **Puppet:** 3♣ Puppet, 2NT diamond transfer, or 2♠ two-way relay — except
///   opener's genuine five-card major show over Puppet (`1NT–3♣–3♥/3♠`).
/// - **European:** 2♠ (clubs) or 3♣ (diamonds) transfer — every continuation
///   (opener's completion, responder's splinter) is a relay, no exception; the
///   natural 2NT invite enters nothing.
///
/// Positions assume the standard uncontested auction; a contested one shifts them
/// and matches none.
fn nt_structure_artificial(auction: &[Call], index: usize, opening_index: usize) -> bool {
    let resp_first = auction.get(opening_index + 2);

    if crate::bidding::american::notrump_minors() == crate::bidding::american::EUROPEAN {
        // European: 2♠ (clubs) and 3♣ (diamonds) are transfers; every suit bid in
        // their continuations is a relay, never a natural suit.
        return matches!(
            resp_first,
            Some(&Call::Bid(b))
                if b == Bid::new(2, Strain::Spades) || b == Bid::new(3, Strain::Clubs)
        );
    }

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

/// The advancer's `2♦` relay / `2♥`-`2♠` preference over a Landy/Woolsey both-majors
/// `2♣`, whose natural single-suit reading is suppressed
///
/// The one suppression the projection pass cannot supply: a relay names no length of
/// its own, so its authored rule projects nothing and the artificial detector (which
/// drives the rest of the suppression now, M6.2c) misses it.  The `2♣` overcall
/// itself, and every other retired convention's shape, are read straight off their
/// projected rule; this is the lone hand stub the doc keeps.
///
/// `None` unless Landy or Woolsey is on *and* the defending side's first action over
/// their `1NT` was the both-majors `2♣`, so a natural `2♣` is never mistaken for it.
// ponytail: a relay projects no info, so suppress it by hand; the upgrade path is to
// author the relay's rule with the negated lengths so the detector catches it too.
fn landy_advance_suppress(auction: &[Call]) -> Option<usize> {
    let on = crate::bidding::american::landy_range().is_some()
        || crate::bidding::american::woolsey_enabled();
    if !on {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;

    // The both-majors 2♣ must be the defending side's first action.
    let overcall_index = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Bid(bid) if index % 2 != opener_parity => {
                Some((bid == Bid::new(2, Strain::Clubs)).then_some(index))
            }
            // The opener answered, or a defender did something else — not a 2♣ Landy.
            _ => Some(None),
        })
        .flatten()?;

    advancer_artificial(auction, overcall_index, opener_parity)
}

/// The index of the advancer's first `2♦`/`2♥`/`2♠` response over a both-majors /
/// Multi overcall at `overcall_index` — a relay or a preference among partner's
/// suits, never own length, so its natural reading is suppressed
///
/// The scan jumps over *every* opponent call (pass, double, or a competing suit
/// bid), so a quiet advance and a doubled / contested runout are all covered: a
/// `2♦`/`2♥`/`2♠` is only legal as the *immediate* response (once the auction climbs
/// past `2♠` it can never recur), so the first such call we find is always the
/// preference, whatever the opponents did.  Suppression is sound regardless — it only
/// ever *removes* a possibly-false length, never asserts one.  The suppression then
/// lives for the whole `Inferences::read`.  `None` if our first response was instead
/// an ask (`2NT`) or a genuine raise.
fn advancer_artificial(
    auction: &[Call],
    overcall_index: usize,
    opener_parity: usize,
) -> Option<usize> {
    auction
        .iter()
        .enumerate()
        .skip(overcall_index + 1)
        // Stop at our first *bid* (decide there); jump over everything the opponents do.
        .find_map(|(index, &call)| match call {
            Call::Bid(bid) if index % 2 != opener_parity => Some(
                matches!(
                    bid,
                    b if b == Bid::new(2, Strain::Diamonds)
                        || b == Bid::new(2, Strain::Hearts)
                        || b == Bid::new(2, Strain::Spades)
                )
                .then_some(index),
            ),
            _ => None,
        })
        .flatten()
}

/// Which Woolsey **Multi-family** overcall the defending side made over their 1NT
#[derive(Clone, Copy)]
enum MultiKind {
    /// `2♦` Multi — a single 6+ major (unknown which), nothing else long.  Names a
    /// diamond suit it does not hold, so its natural reading must be suppressed.
    Major,
    /// `2♥`/`2♠` Muiderberg — exactly 5 in the named major, ≤ 3 in the other major
    /// (and a 4+ minor, captured by the residual).  A real major: no suppression.
    Muiderberg(Suit),
}

/// A Woolsey Multi-family overcall and which call it was
#[derive(Clone, Copy)]
struct MultiReading {
    overcall_index: usize,
    kind: MultiKind,
    /// The advancer's `2♥`/`2♠` pass-or-correct over the Multi `2♦` (a preference
    /// among partner's unknown major — not own length), suppressed if present.
    advance_suppress: Option<usize>,
}

impl MultiReading {
    /// Whether the call at `index` is artificial: the `2♦` Multi naming diamonds it
    /// does not hold, or the advancer's `2♥`/`2♠` pass-or-correct (a preference, not
    /// own length).  The Muiderberg `2♥`/`2♠` overcall names a real 5-card major, so
    /// its natural reading is kept.
    fn suppresses(&self, index: usize) -> bool {
        (matches!(self.kind, MultiKind::Major) && self.overcall_index == index)
            || self.advance_suppress == Some(index)
    }
}

/// Read a Woolsey **Multi-family** overcall of their 1NT: the `2♦` Multi (a single
/// 6+ major) or the `2♥`/`2♠` Muiderberg (exactly 5 in the major + a 4+ minor)
///
/// Gated on [`woolsey_enabled`][crate::bidding::american::woolsey_enabled] and the
/// auction being `1NT` then the defending side's first action being that bid.  The
/// both-majors `2♣` is read off its authored rule by the projection pass folded
/// into [`Inferences::read`] (Woolsey = Landy 2♣ + this family).
///
/// ponytail: kept separate so this Multi reading is reusable for a future Multi `2♦`
/// *opening* (an unknown-major weak two) — same shape, no 1NT prefix.
fn multi_reading(auction: &[Call]) -> Option<MultiReading> {
    if !crate::bidding::american::woolsey_enabled() {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;

    // The defending side's FIRST action — a 2♦/2♥/2♠ Multi-family overcall.
    let reading = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Bid(bid) if index % 2 != opener_parity => {
                let kind = if bid == Bid::new(2, Strain::Diamonds) {
                    Some(MultiKind::Major)
                } else if bid == Bid::new(2, Strain::Hearts) {
                    Some(MultiKind::Muiderberg(Suit::Hearts))
                } else if bid == Bid::new(2, Strain::Spades) {
                    Some(MultiKind::Muiderberg(Suit::Spades))
                } else {
                    None
                };
                Some(kind.map(|kind| MultiReading {
                    overcall_index: index,
                    kind,
                    advance_suppress: None,
                }))
            }
            // The opener's side acted (a response), or a defender did something else.
            _ => Some(None),
        })
        .flatten()?;

    // Over the Multi 2♦, the advancer's 2♥/2♠ pass-or-correct picks one of partner's
    // unknown majors — a preference, not own length — so suppress it too (including a
    // doubled runout; the shared helper handles both).
    let advance_suppress = matches!(reading.kind, MultiKind::Major)
        .then(|| advancer_artificial(auction, reading.overcall_index, opener_parity))
        .flatten();

    Some(MultiReading {
        advance_suppress,
        ..reading
    })
}

/// Our Woolsey takeout **double** of their 1NT and the advancer's `2♣` minor relay
///
/// The double shows a 4-card major plus a 5-6 card minor with the
/// [`woolsey_double_floor`][crate::bidding::american::woolsey_double_floor] points
/// floor.  The shape is a *double* disjunction (either major, either minor) the
/// per-suit framework cannot pin, so only the points floor is recorded post-walk —
/// but that alone matters: a double of 1NT names no suit, so the generic walk reads
/// it as *nothing* (the takeout-of-a-suit branch needs a suit opening), leaving the
/// floor to sample the doubler as a random hand.
///
/// The advancer's `2♣` over the double is a "name your minor" relay, not own clubs,
/// so its natural reading is suppressed.  Our own `2♥`/`2♠` advances are natural
/// majors and `2NT` is the notrump game-ask, so neither needs suppression.
#[derive(Clone, Copy)]
struct WoolseyXReading {
    double_index: usize,
    relay_suppress: Option<usize>,
}

impl WoolseyXReading {
    fn suppresses(&self, index: usize) -> bool {
        self.relay_suppress == Some(index)
    }
}

fn woolsey_x_reading(auction: &[Call]) -> Option<WoolseyXReading> {
    if !crate::bidding::american::woolsey_enabled() {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;

    // The double must be the defending side's FIRST action over their 1NT.
    let double_index = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Double if index % 2 != opener_parity => Some(Some(index)),
            // The opener's side acted, or a defender did something else (an overcall)
            // — not our takeout double.
            _ => Some(None),
        })
        .flatten()?;

    // The advancer's first bid; suppress it only if it is the 2♣ minor relay.  Jump
    // over every opponent call so a contested relay is covered too (the 2♣ relay is
    // only legal as the immediate response, so the first such call is always it).
    let relay_suppress = auction
        .iter()
        .enumerate()
        .skip(double_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Bid(bid) if index % 2 != opener_parity => {
                Some((bid == Bid::new(2, Strain::Clubs)).then_some(index))
            }
            _ => None,
        })
        .flatten();

    Some(WoolseyXReading {
        double_index,
        relay_suppress,
    })
}

/// The index of our natural **penalty** double of their 1NT (15+ HCP), or `None`
///
/// A double of 1NT names no suit, so the generic walk's takeout branch (which needs
/// a suit opening) reads it as nothing.  Returns the doubler's index so the post-walk
/// pass records the [`natural_double_floor`][crate::bidding::american::natural_double_floor]
/// points floor.  Mirrors [`woolsey_x_reading`].
///
/// Fires only when a double of their 1NT actually *means* the natural penalty double:
/// the natural defense is on and no convention has repurposed the double (DONT = a
/// one-suiter, direct Landy / Woolsey = both majors — each has its own reading).  A
/// *passed* doubler cannot hold 15+, so their double is the both-majors passed-hand
/// call, not penalty; an unpassed doubler is identified by lane (a seat that passed
/// before the opening occupies a lane below `opening_index`).
pub(super) fn penalty_x_reading(auction: &[Call]) -> Option<usize> {
    use crate::bidding::american as a;
    if !a::natural_defense_enabled()
        || a::direct_dont_enabled()
        || a::direct_landy_double().is_some()
        || a::woolsey_enabled()
    {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;

    // The double must be the defending side's FIRST action over their 1NT.
    let double_index = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Double if index % 2 != opener_parity => Some(Some(index)),
            // The opener's side acted, or a defender overcalled — not the penalty double.
            _ => Some(None),
        })
        .flatten()?;

    // A passed doubler's double is the both-majors passed-hand call, never 15+ penalty.
    // Seats that passed before the opening fill lanes `0..opening_index` (all the calls
    // there are passes), so an unpassed doubler's lane is at or beyond `opening_index`.
    (double_index % 4 >= opening_index).then_some(double_index)
}

/// The index of responder's double of an opponent's overcall of *our* 1NT
/// (`[1NT,(2X),X]`), or `None`
///
/// Every [`DoubleStyle`][crate::bidding::american::DoubleStyle] makes this double
/// show **8+ values** (takeout ≤3/8, penalty 4+/9, optional 2-3/8), so the post-walk
/// records that points floor — without it the double reads as nothing and opener
/// undercounts the partnership's strength.  Fires only for our own 1NT (the opener
/// shares the actor's parity); their responder's double of our overcall is their
/// convention, not ours.
fn responder_overcall_double_reading(auction: &[Call], len: usize) -> Option<usize> {
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump))
        || opening_index % 2 != len % 2
    {
        return None;
    }
    // The opponent's suit overcall, then our responder's immediate double of it.
    match auction.get(opening_index + 1) {
        Some(Call::Bid(bid)) if bid.strain.is_suit() => {}
        _ => return None,
    }
    (auction.get(opening_index + 2) == Some(&Call::Double)).then_some(opening_index + 2)
}

/// Our side's *subsequent* penalty doubles after the natural penalty X of their
/// 1NT — the latch's later doubles — each paired with the suit it doubles
///
/// The penalty latch ([`set_penalty_latch`][crate::bidding::instinct::set_penalty_latch])
/// makes these via the trump-stack rule, so each promises four-plus cards in the
/// doubled suit.  Recording that length stops the sampler reading the double as
/// takeout — without it the advancer pulls a penalty double thinking partner is
/// short, the phantom-suit leak the [`penalty_x_reading`] doc names.  Empty unless
/// the latch is on, so it agrees with the floor on when a later double is penalty.
///
/// Once we penalty-double their 1NT the penalty stance holds for the rest of the
/// auction (mirrors `penalty_latched`) — a bid of our own does *not* un-latch it,
/// it only updates the suit a later penalty double refers to.
fn penalty_latch_double_reading(auction: &[Call]) -> Vec<(usize, Suit)> {
    if !crate::bidding::instinct::penalty_latch_enabled() {
        return Vec::new();
    }
    let Some(x_index) = penalty_x_reading(auction) else {
        return Vec::new();
    };
    let our_parity = x_index % 2;
    let mut out = Vec::new();
    let mut last_suit_bid: Option<(Suit, usize)> = None; // (suit, the bidder's parity)
    for (index, &call) in auction.iter().enumerate().skip(x_index + 1) {
        match call {
            // Our own bid does not un-latch the penalty stance; it just updates the
            // suit a later penalty double would refer to.
            Call::Bid(bid) => {
                last_suit_bid = bid.strain.suit().map(|suit| (suit, index % 2));
            }
            // Our double of their suit runout is penalty: four-plus in that suit.
            Call::Double if index % 2 == our_parity => {
                if let Some((suit, bidder_parity)) = last_suit_bid
                    && bidder_parity != our_parity
                {
                    out.push((index, suit));
                }
            }
            _ => {}
        }
    }
    out
}

/// Which DONT defense call the defending side made over their 1NT
#[derive(Clone, Copy)]
enum DontKind {
    /// `X` — a one-suiter in ♣/♦/♥ (a spade one-suiter bids the natural `2♠`), so
    /// spades are short.  The long suit is a triple disjunction the per-suit
    /// framework cannot pin; only `spades ≤ 3` is a sound per-suit fact.
    OneSuiter,
    /// `2♣` — clubs (real, ≥ 4) + an unknown higher major.  Names a real club suit,
    /// but the natural ≥ 5 reading is unsound (the 4-major-5-club hand has 4 clubs).
    ClubsMajor,
    /// `2♦` — diamonds (real, ≥ 4) + an unknown major.  As `ClubsMajor` for diamonds.
    DiamondsMajor,
    /// `2♥` — both majors, ≥ 5-4.  Exactly a Landy two-suiter on the `2♥` bid.
    BothMajors,
}

/// A DONT overcall of their 1NT (`X`/`2♣`/`2♦`/`2♥`) and the advancer's relay
///
/// DONT's calls name suits the hand may not hold (`X` names none; `2♣`/`2♦`/`2♥` can
/// be only 4 cards in the named suit) or are relays, so the generic walk misreads
/// them — leaving the floor to raise a phantom suit or sample a random hand.  The
/// natural `2♠` is a genuine spade suit and needs no reading.  Mirrors
/// [`multi_reading`] / [`woolsey_x_reading`].
#[derive(Clone, Copy)]
struct DontReading {
    overcall_index: usize,
    kind: DontKind,
    floor: u8,
    /// The advancer's relay — `2♣` over the `X`, or the `2♦`/`2♥`/`2♠` pass-or-correct
    /// over `2♣`/`2♦`/`2♥` (a preference among partner's suits, not own length).
    advance_suppress: Option<usize>,
}

impl DontReading {
    /// Whether the call at `index` is artificial.  The `X` (a double) names no suit,
    /// so only the `2♣`/`2♦`/`2♥` overcalls suppress their own natural reading; the
    /// advancer's relay is always suppressed.
    fn suppresses(&self, index: usize) -> bool {
        (!matches!(self.kind, DontKind::OneSuiter) && self.overcall_index == index)
            || self.advance_suppress == Some(index)
    }
}

/// Read a DONT overcall of their 1NT, gated on
/// [`direct_dont_enabled`][crate::bidding::american::direct_dont_enabled] and the
/// auction being `1NT` then the defending side's first action being a DONT call
fn dont_reading(auction: &[Call]) -> Option<DontReading> {
    if !crate::bidding::american::direct_dont_enabled() {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;
    let floor = crate::bidding::american::natural_overcall_points().0;

    // The defending side's FIRST action — a DONT `X`/`2♣`/`2♦`/`2♥` (the natural `2♠`
    // and anything else fall through to the generic reading).
    let (overcall_index, kind) = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Double if index % 2 != opener_parity => Some(Some((index, DontKind::OneSuiter))),
            Call::Bid(bid) if index % 2 != opener_parity => {
                let kind = if bid == Bid::new(2, Strain::Clubs) {
                    Some(DontKind::ClubsMajor)
                } else if bid == Bid::new(2, Strain::Diamonds) {
                    Some(DontKind::DiamondsMajor)
                } else if bid == Bid::new(2, Strain::Hearts) {
                    Some(DontKind::BothMajors)
                } else {
                    None
                };
                Some(kind.map(|kind| (index, kind)))
            }
            // The opener's side acted (a response), or a defender did something else.
            _ => Some(None),
        })
        .flatten()?;

    // The advancer's relay: `2♣` over the `X` (it names a minor, not own clubs), or the
    // `2♦`/`2♥`/`2♠` preference over a two-suiter (one of partner's suits, not own
    // length).  Both scans jump over every opponent call so a contested relay is
    // covered (the relay is only legal as the immediate response).
    let advance_suppress = match kind {
        DontKind::OneSuiter => auction
            .iter()
            .enumerate()
            .skip(overcall_index + 1)
            .find_map(|(index, &call)| match call {
                Call::Bid(bid) if index % 2 != opener_parity => {
                    Some((bid == Bid::new(2, Strain::Clubs)).then_some(index))
                }
                _ => None,
            })
            .flatten(),
        _ => advancer_artificial(auction, overcall_index, opener_parity),
    };

    Some(DontReading {
        overcall_index,
        kind,
        floor,
        advance_suppress,
    })
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

    /// Read on a *prefixed* context, the trie access the projection pass needs to
    /// read a convention off its authored rule — what the production search floor
    /// hands `Inferences::read` (cf. `Stance::prefixed_context`).  The plain `read`
    /// above is keyless, so it sees no convention overlay.
    fn read_booked(auction: &[Call]) -> Inferences {
        let stance = crate::american().against(crate::bidding::Family::NATURAL);
        Inferences::read(&stance.prefixed_context(RelativeVulnerability::NONE, auction))
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
        let advance = read_booked(&[bid(2, Strain::Hearts), bid(4, Strain::Clubs), Call::Pass]);
        assert_eq!(advance.partner().length(Suit::Clubs), Range::new(5, 13));
        assert_eq!(advance.partner().length(Suit::Spades), Range::new(5, 13));
        assert_eq!(advance.partner().points, Range::new(14, 37));

        // Over 2♦, the 4♦ cue shows both majors; 4♣ shows clubs + an unknown
        // major, so only clubs is pinned.
        let cue = read_booked(&[
            bid(2, Strain::Diamonds),
            bid(4, Strain::Diamonds),
            Call::Pass,
        ]);
        assert_eq!(cue.partner().length(Suit::Hearts), Range::new(5, 13));
        assert_eq!(cue.partner().length(Suit::Spades), Range::new(5, 13));

        // Disabled (the default): a 4♣ jump reads as a natural one-suiter, so
        // spades stay unconstrained — the convention must not leak when off.
        set_leaping_michaels(false);
        let off = read_booked(&[bid(2, Strain::Hearts), bid(4, Strain::Clubs), Call::Pass]);
        assert_eq!(off.partner().length(Suit::Spades), Range::FULL_LENGTH);
    }

    #[test]
    fn landy_conditions_partner() {
        use crate::bidding::american::{set_landy, set_unusual_notrump_defense};

        // (1NT)–2♣–(P): the advancer reads partner's both-majors two-suiter (at
        // least 4-4 in the majors, 8+ points) rather than a natural club suit.
        set_landy(Some((8, 15)));
        set_unusual_notrump_defense(Some((8, 15)));
        let advance = read_booked(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
        assert_eq!(advance.partner().length(Suit::Hearts), Range::new(4, 13));
        assert_eq!(advance.partner().length(Suit::Spades), Range::new(4, 13));
        assert_eq!(advance.partner().length(Suit::Clubs), Range::FULL_LENGTH);
        assert_eq!(advance.partner().points, Range::new(8, 37));

        // (1NT)–2NT–(P): both minors, 5-5 (the independent unusual-2NT toggle).
        let minors = read_booked(&[bid(1, Strain::Notrump), bid(2, Strain::Notrump), Call::Pass]);
        assert_eq!(minors.partner().length(Suit::Clubs), Range::new(5, 13));
        assert_eq!(minors.partner().length(Suit::Diamonds), Range::new(5, 13));

        // The advancer's 2♦ relay is artificial — read from the overcaller's seat,
        // partner's (the relayer's) diamonds stay unconstrained.
        let relay = read_booked(&[
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
        ]);
        assert_eq!(relay.partner().length(Suit::Diamonds), Range::FULL_LENGTH);

        // Disabled: 2♣ reads as a natural club one-suiter, so spades stay
        // unconstrained — the convention must not leak when off.
        set_landy(None);
        set_unusual_notrump_defense(None);
        let off = read_booked(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
        assert_eq!(off.partner().length(Suit::Spades), Range::FULL_LENGTH);

        // Restore the shipped defaults so sibling tests on this thread are unaffected
        // (unusual 2NT ships on; Landy 2♣ ships off).
        set_unusual_notrump_defense(Some((8, 13)));
    }

    #[test]
    fn woolsey_conditions_partner() {
        use crate::bidding::american::{
            set_landy, set_unusual_notrump_defense, set_woolsey, set_woolsey_points,
        };
        // Landy off, Woolsey on: the 2♣ must read through the Woolsey path.
        set_landy(None);
        set_unusual_notrump_defense(None);
        set_woolsey(true);
        set_woolsey_points(10, 19);

        // (1NT)–2♣–(P): Woolsey's 2♣ is both majors, 10+, never a natural club suit.
        // Read off the authored rule's projection (on a prefixed/booked context),
        // which pins each major to 4-5 exactly — Woolsey sends a six-card major to
        // the Multi/Muiderberg calls, a distinction the old loose reader missed.
        let two_c = read_booked(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
        assert_eq!(two_c.partner().length(Suit::Hearts), Range::new(4, 5));
        assert_eq!(two_c.partner().length(Suit::Spades), Range::new(4, 5));
        assert_eq!(two_c.partner().length(Suit::Clubs), Range::FULL_LENGTH);
        assert_eq!(two_c.partner().points, Range::new(10, 37));

        // (1NT)–2♦–(P): the Multi names diamonds it does NOT hold, so the natural
        // ≥5 reading is suppressed and BOTH minors narrow to ≤4 — the floor can no
        // longer "raise diamonds" into a doubled 5♦ (the 6+ major falls out of the
        // residual the per-suit framework cannot pin).
        let multi = read(&[
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            Call::Pass,
        ]);
        assert_eq!(multi.partner().length(Suit::Diamonds), Range::new(0, 4));
        assert_eq!(multi.partner().length(Suit::Clubs), Range::new(0, 4));

        // (1NT)–2♥–(P): Muiderberg — exactly 5 hearts, ≤3 spades.
        let muiderberg = read(&[bid(1, Strain::Notrump), bid(2, Strain::Hearts), Call::Pass]);
        assert_eq!(muiderberg.partner().length(Suit::Hearts), Range::new(5, 5));
        assert_eq!(muiderberg.partner().length(Suit::Spades), Range::new(0, 3));

        // The advancer's 2♥/2♠ over 2♣ (both majors) or 2♦ (Multi) is a PREFERENCE
        // among partner's two majors — not own length — so its natural ≥4 reading is
        // suppressed throughout (here, read from the advancer's seat as partner).
        let pref_2c = read(&[
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ]);
        assert_eq!(pref_2c.partner().length(Suit::Hearts), Range::FULL_LENGTH);
        let pref_2d = read(&[
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Spades),
            Call::Pass,
        ]);
        assert_eq!(pref_2d.partner().length(Suit::Spades), Range::FULL_LENGTH);

        // Off: the Multi 2♦ reads as a natural diamond one-suiter again (≥5) — the
        // convention must not leak when disabled.
        set_woolsey(false);
        let off = read(&[
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            Call::Pass,
        ]);
        assert_eq!(off.partner().length(Suit::Diamonds), Range::new(5, 13));

        // Restore the shipped default (unusual 2NT ships on).
        set_unusual_notrump_defense(Some((8, 13)));
    }

    #[test]
    fn woolsey_double_and_advances_read() {
        use crate::bidding::american::{
            set_landy, set_unusual_notrump_defense, set_woolsey, set_woolsey_double_floor,
            set_woolsey_points,
        };
        set_landy(None);
        set_unusual_notrump_defense(None);
        set_woolsey(true);
        set_woolsey_points(10, 19);
        set_woolsey_double_floor(12);

        // (1NT)–X–(P): the takeout double names no suit, so nothing is misread — but
        // the doubler's strength (12+) is recorded, where a bare double of 1NT would
        // otherwise read as nothing.
        let x = read(&[bid(1, Strain::Notrump), Call::Double, Call::Pass]);
        assert_eq!(x.partner().points, Range::new(12, 37));

        // (1NT)–X–(P)–2♣–(P): the advancer's 2♣ is a "name your minor" relay, not own
        // clubs, so its natural ≥4 reading is suppressed (read from the advancer seat).
        let relay = read(&[
            bid(1, Strain::Notrump),
            Call::Double,
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ]);
        assert_eq!(relay.partner().length(Suit::Clubs), Range::FULL_LENGTH);

        // (1NT)–2♥–(P)–2NT–(P): the Muiderberg minor-ask 2NT is a relay in a
        // COMPETITIVE auction (our side already overcalled), so it is never read as a
        // natural notrump invite — the advancer's points stay unconstrained.
        let ask = read(&[
            bid(1, Strain::Notrump),
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Notrump),
            Call::Pass,
        ]);
        assert_eq!(ask.partner().points, Range::new(0, 37));

        // Off: the Woolsey 12+ reading must not leak — the double now falls through to
        // the default-on natural penalty reading (15+), not Woolsey's 12+.
        set_woolsey(false);
        let off = read(&[bid(1, Strain::Notrump), Call::Double, Call::Pass]);
        assert_eq!(off.partner().points, Range::new(15, 37));

        set_unusual_notrump_defense(Some((8, 13)));
    }

    #[test]
    fn dont_overcalls_and_advances_read() {
        use crate::bidding::american::{set_direct_dont, set_landy, set_unusual_notrump_defense};
        set_landy(None);
        set_unusual_notrump_defense(None);
        set_direct_dont(true);

        // (1NT)–X–(P): a one-suiter in ♣/♦/♥ — spades short (≤3, the one sound fact),
        // strength recorded (the default 8+ overcall floor) where a bare double of 1NT
        // would otherwise read as nothing.
        let x = read(&[bid(1, Strain::Notrump), Call::Double, Call::Pass]);
        assert_eq!(x.partner().length(Suit::Spades), Range::new(0, 3));
        assert_eq!(x.partner().points, Range::new(8, 37));

        // (1NT)–X–(P)–2♣–(P): the advancer's 2♣ is a "name your suit" relay, not own
        // clubs, so its natural ≥4 reading is suppressed (read from the advancer seat).
        let relay = read(&[
            bid(1, Strain::Notrump),
            Call::Double,
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ]);
        assert_eq!(relay.partner().length(Suit::Clubs), Range::FULL_LENGTH);

        // (1NT)–2♣–(P): a real ≥4 club suit + an unknown major.  The natural ≥5 reading
        // is suppressed (a 4-club / 5-major DONT hand makes this call), re-pinned to ≥4.
        let two_c = read(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
        assert_eq!(two_c.partner().length(Suit::Clubs), Range::new(4, 13));
        assert_eq!(two_c.partner().points, Range::new(8, 37));

        // (1NT)–2♣–(P)–2♦–(P): the advancer's 2♦ is a "name your higher suit" relay,
        // not own diamonds — suppressed.
        let pref = read(&[
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
        ]);
        assert_eq!(pref.partner().length(Suit::Diamonds), Range::FULL_LENGTH);

        // (1NT)–2♥–(P): both majors, ≥4-4 — exactly a Landy two-suiter on the 2♥ bid.
        let two_h = read(&[bid(1, Strain::Notrump), bid(2, Strain::Hearts), Call::Pass]);
        assert_eq!(two_h.partner().length(Suit::Hearts), Range::new(4, 13));
        assert_eq!(two_h.partner().length(Suit::Spades), Range::new(4, 13));

        // Off: the 2♣ reads as a natural club one-suiter again (≥5) — no leak.
        set_direct_dont(false);
        let off = read(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
        assert_eq!(off.partner().length(Suit::Clubs), Range::new(5, 13));
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
        let inf = read_booked(&auction);
        assert_eq!(inf.me().length(Suit::Hearts), Range::new(5, 13));
        assert_eq!(inf.me().length(Suit::Diamonds), Range::FULL_LENGTH);
    }

    #[test]
    fn transfer_jump_to_game_shows_at_least_five() {
        // [1NT, P, 2♦, P, 2♥, P, 4♥, P]: partner transferred then jumped to 4♥.
        // The projection reads the 2♦ transfer's authored rule — a five-card floor;
        // the old reader's six-card upgrade off the jump is dropped (soundness over
        // tightness, M6.2c).  At length 8 the responder sits as Partner.
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
        let inf = read_booked(&auction);
        assert_eq!(inf.partner().length(Suit::Hearts), Range::new(5, 13));
    }

    #[test]
    fn transfer_then_three_major_shows_at_least_five() {
        // [1NT, P, 2♦, P, 2♥, P, 3♥, P]: a raise of the transferred suit.  The
        // projection pins the transfer's five-card floor; the old reader's six-card
        // upgrade and the 8–9 invitational points are dropped (soundness over
        // tightness, M6.2c).
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
        let inf = read_booked(&auction);
        assert!(inf.partner().length(Suit::Hearts).min >= 5);
    }

    #[test]
    fn transfer_projection_covers_spades_and_two_notrump() {
        // Spade transfer (2♥ → 2♠) jumped to 4♠: the 2♥ transfer rule projects a
        // five-card spade floor.
        let spades = read_booked(&[
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Spades),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ]);
        assert_eq!(spades.partner().length(Suit::Spades), Range::new(5, 13));

        // The same shape over a 2NT opening (3♦ → 3♥, jump 4♥).
        let two_nt = read_booked(&[
            bid(2, Strain::Notrump),
            Call::Pass,
            bid(3, Strain::Diamonds),
            Call::Pass,
            bid(3, Strain::Hearts),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ]);
        assert_eq!(two_nt.partner().length(Suit::Hearts), Range::new(5, 13));
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
