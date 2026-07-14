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
    /// Whether a one-level Rubens transfer records its meaning (see
    /// [`set_rubens_transfer_reading`]).  On by default; off recovers the
    /// suppress-only reading, where the transfers showed nothing.
    static RUBENS_TRANSFER_READING: Cell<bool> = const { Cell::new(true) };
}

/// Toggle recording the one-level Rubens transfers' meaning (default on).
///
/// The two-level cue-raise always recorded its limit-plus raise; the one-level
/// transfers were suppress-only, leaving the overcaller — and the sampler
/// behind the search floor — blind to the shown support, length, and strength.
/// On, the transfer into partner's suit records three-plus cards in the
/// overcall suit, a new-suit transfer records five-plus in its target, both
/// ten-plus points.  For A/B measurement (`bba-gen --no-ns-rubens-reading`).
pub fn set_rubens_transfer_reading(on: bool) {
    RUBENS_TRANSFER_READING.with(|cell| cell.set(on));
}

fn rubens_transfer_reading() -> bool {
    RUBENS_TRANSFER_READING.with(Cell::get)
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

    /// Whether the projection pass decodes calls authored by *guarded fallbacks*
    /// (every contested convention — transfers, Leaping Michaels, the Lebensohl
    /// cue), not just exact-node classifiers.  **On by default** (BBA A/B: plain
    /// +0.0006/board, +1.03/fired; PD +0.0014, +2.38/fired — both CIs exclude 0).
    /// On, [`project_authored`] re-resolves each prior call's authoring classifier
    /// through the trie's node-then-fallback chain so an alerted call survives later
    /// competition without a per-convention hand reader.  Off restores the
    /// exact-node-only projection (the A/B off arm).
    static FALLBACK_PROJECTION: Cell<bool> = const { Cell::new(true) };
}

/// Toggle decoding fallback-authored conventions in the projection (**default on**)
///
/// Off, `project_authored` sees only exact-node classifiers (via
/// [`common_prefixes`][super::Trie::common_prefixes]), so a contested convention
/// authored by a guarded fallback misreads under second-round intervention unless a
/// hand-written reader covers it.  On, it re-resolves each call's *authoring*
/// classifier (node or fallback) and projects its alerted rule — the general decode
/// for non-natural calls that subsumes the single-suit per-convention readers (the
/// OR-disjunction two-suiters and doubles still need their hand readers).  Read at
/// classification time, per-thread; A/B'd on the BBA match.
pub fn set_fallback_projection(on: bool) {
    FALLBACK_PROJECTION.with(|cell| cell.set(on));
}

/// Whether fallback-authored projection is enabled (default on)
#[must_use]
pub fn fallback_projection_enabled() -> bool {
    FALLBACK_PROJECTION.with(Cell::get)
}

std::thread_local! {
    /// Whether the reading classifies high (four-plus level) new-suit bids as
    /// control bids vs to-play (**on by default**, M6.4).  The deterministic
    /// rule, distilled from Bridge World Standard: such a bid is *natural* iff
    /// the suit could still be the bidder's longest — the bidder has shown no
    /// other suit yet, or is rebidding a suit they themselves showed.
    /// Otherwise it is a control bid agreeing the partnership's most recently
    /// shown suit, and the phantom suit is suppressed rather than floored.
    static CONTROL_BID_READING: Cell<bool> = const { Cell::new(true) };
}

/// Toggle the control-bid reading of high new-suit bids (**default on**, M6.4)
///
/// Off, a four-plus-level new suit falls back to the pre-M6.4 reading (double
/// jumps skipped, notrump-structure bids blanket-suppressed) — the A/B off arm.
pub fn set_control_bid_reading(on: bool) {
    CONTROL_BID_READING.with(|cell| cell.set(on));
}

/// Whether the control-bid reading is enabled (default on); shared with the
/// instinct floor so the reader and the signoff rules flip together
#[must_use]
pub(super) fn control_bid_reading() -> bool {
    CONTROL_BID_READING.with(Cell::get)
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

/// A systems-on advance of our 1NT overcall, with their opening stripped
///
/// When `set_nt_overcall_systems_on` is enabled the advancer plays the full
/// opening-1NT structure grafted below `[their 1-of-a-suit, our 1NT]`, so the
/// artificial Stayman/transfer calls need the *opening-1NT* reading, not the
/// natural walk.  This returns the auction with their opening removed, which
/// reads exactly like an opening 1NT: `(len - index) % 4` is invariant under
/// removing one earlier call, so every later call keeps its relative seat (only
/// their opening — their own natural suit — is lost, which the opponents' system
/// discloses anyway).  [`None`] (the fast path) unless the graft is on and the
/// shape is their 1-suit opening immediately overcalled `1NT`.
fn systems_on_overcall_strip(auction: &[Call]) -> Option<Vec<Call>> {
    if !crate::bidding::american::nt_overcall_systems_on() {
        return None;
    }
    let open = auction.iter().position(|&c| c != Call::Pass)?;
    let Call::Bid(opening) = auction[open] else {
        return None;
    };
    if opening.level.get() != 1 || !opening.strain.is_suit() {
        return None;
    }
    // Over a MAJOR, Gladiator replaces the opening-1NT graft with a differently
    // shaped structure (cue = Stayman, 2♣ = relay), so the strip identity no
    // longer holds — leave those auctions to `gladiator_reading` / the walk.
    if crate::bidding::american::nt_overcall_gladiator()
        && matches!(opening.strain, Strain::Hearts | Strain::Spades)
    {
        return None;
    }
    if auction.get(open + 1) != Some(&Call::Bid(Bid::new(1, Strain::Notrump))) {
        return None;
    }
    let mut stripped = auction.to_vec();
    stripped.remove(open);
    Some(stripped)
}

/// All four players' shown shape and strength, relative to the side to act
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Inferences {
    players: [Inference; 4],
    /// The last call the M6.4 classifier read as a control bid: its auction
    /// index and the suit it agrees.  The exact witness for the instinct
    /// signoff — "the named suit is unread" cannot tell a control bid from an
    /// unread to-play bid.  Not part of the shown-range payload
    /// (serialization skips it).
    #[cfg_attr(feature = "serde", serde(skip))]
    control_bid: Option<(u8, Suit)>,
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
        // A systems-on advance of our 1NT overcall reads as an opening-1NT
        // auction with their opening stripped: the advancer plays the grafted
        // 1NT structure, so the hand-coded notrump walk reads its artificial
        // Stayman/transfer calls instead of the natural walk raising a phantom
        // suit.  Re-read keyless (projection dropped — soundness over tightness;
        // `read` ignores vul, and the stripped opening is 1NT so this recurses at
        // most once).
        if let Some(stripped) = systems_on_overcall_strip(context.auction()) {
            return Self::read(&Context::new(context.vul(), &stripped));
        }
        let auction = context.auction();
        let len = auction.len();
        let mut players = [Inference::unknown(); 4];
        let mut control_bid = None;

        let Some(opening_index) = auction.iter().position(|&c| c != Call::Pass) else {
            return Self {
                players,
                control_bid,
            };
        };
        let Call::Bid(opening_bid) = auction[opening_index] else {
            return Self {
                players,
                control_bid,
            };
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
        let (rubens_suppress, rubens_cue, rubens_transfer) = rubens_reading(auction);

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
        // The Meckwell defense of their 1NT: the two-way X (single minor OR both
        // majors) records points only; the 2♣/2♦ minor + major and the advancer's
        // relay are suppressed like DONT's.
        let meckwell = meckwell_reading(auction);
        // Their two-suiter over our 1M: the Michaels cue of our own major is
        // suppressed (it is not a natural suit in our major); what each call
        // genuinely shows is recorded post-walk.
        let two_suiter = two_suiter_reading(auction);
        // Our natural penalty double of their 1NT (15+): a double names no suit, so the
        // generic walk reads it as nothing — the points floor is recorded post-walk.
        let penalty_x = penalty_x_reading(auction);
        // The latch's subsequent penalty doubles: each promises four-plus in the suit
        // it doubles, recorded post-walk so the sampler does not read them as takeout.
        let penalty_latch_doubles = penalty_latch_double_reading(auction);
        // Responder's double of an overcall of our 1NT shows 8+ (every DoubleStyle),
        // recorded post-walk so opener does not undercount the partnership's strength.
        let overcall_double = responder_overcall_double_reading(auction, len);
        // Our Gladiator advance of a 1NT overcall of their major: the 2♣ relay (and
        // its forced 2♦), the cue-Stayman, the 3M splinter, and the 4M both-minor
        // Leaping Michaels are bids of a suit the caller lacks — suppressed here,
        // real shape recorded post-walk.
        let gladiator = gladiator_reading(auction);

        // Which calls the walk has suppressed so far (any reason: projection,
        // convention readers, the notrump-structure blanket, control bids) —
        // the control-bid classifier scans it for the agreed suit (M6.4).
        let mut suppressed_so_far = 0u64;

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
                        let nt_blanket = is_opening_side && opening_artificial && !over_one_notrump;
                        let chain = stayman_artificial
                            || nt_structure_artificial(auction, index, opening_index)
                            || rubens_suppress.contains(&Some(index))
                            || (index < 64 && suppressed >> index & 1 != 0)
                            || landy_relay == Some(index)
                            || multi.is_some_and(|m| m.suppresses(index))
                            || woolsey_x.is_some_and(|w| w.suppresses(index))
                            || dont.is_some_and(|d| d.suppresses(index))
                            || meckwell.is_some_and(|m| m.suppresses(index))
                            || two_suiter.is_some_and(|t| t.suppresses(index))
                            || gladiator.is_some_and(|g| g.suppresses(index));

                        // M6.4: a four-plus-level suit bid in the slam zone is
                        // classified control-bid vs to-play before the natural
                        // walk (see [`classify_high_bid`]).  It may punch
                        // through the notrump-structure blanket (the
                        // post-transfer 4♠ control) — but only when the
                        // projection is present to have claimed the genuinely
                        // artificial calls (Texas transfers) first.
                        let slam = if control_bid_reading()
                            && index != opening_index
                            && is_opening_side
                            && !side_acted[defending_parity]
                            && (4..=5).contains(&bid.level.get())
                            && !chain
                            && (!nt_blanket || context.prefixes().is_some())
                        {
                            classify_high_bid(
                                auction,
                                index,
                                bid,
                                len,
                                opening_index,
                                &players,
                                &overlay,
                                suppressed_so_far,
                            )
                        } else {
                            HighBid::Unclaimed
                        };

                        let suppress = match slam {
                            // To play (or an unreadable splinter): no record —
                            // flooring a six here rerouted combined-33 hands
                            // from the winning 6NT power-blast into thin 6-2
                            // suit slams (round 4 of the A/B).
                            HighBid::ToPlay => true,
                            HighBid::Control { trump, shower } => {
                                // A control bid: the bid suit is a control, not
                                // length — it agrees `trump`.  Agreeing one's
                                // own shown suit past game promises a sixth
                                // card; agreeing partner's promises support.
                                // Either way the slam try shows opening values
                                // and up (a sound floor; the real hand is
                                // stronger).
                                let floor = if shower == who { 6 } else { 3 };
                                players[who]
                                    .narrow_length(trump, Range::at_least(floor, LENGTH_CAP));
                                players[who].narrow_points(Range::at_least(13, POINTS_CAP));
                                #[allow(clippy::cast_possible_truncation)]
                                {
                                    control_bid = Some((index as u8, trump));
                                }
                                true
                            }
                            HighBid::Unclaimed => nt_blanket || chain,
                        };
                        if suppress && index < 64 {
                            suppressed_so_far |= 1 << index;
                        }

                        // Opener's extras-ladder rebid: a minor opening, opener's
                        // first rebid, opponents silent.  The jump-shift and
                        // reverse rungs name a real 4-card second suit and show
                        // extras — read below, not as a weak jump.
                        let opener_ladder_rebid = crate::bidding::american::opener_extras_ladder()
                            && !side_acted[defending_parity]
                            && is_opening_side
                            && lane == opener_lane
                            && lane_bids[lane] == 1
                            && opening_bid.level.get() == 1
                            && matches!(opening_bid.strain, Strain::Clubs | Strain::Diamonds);

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
                                // A single jump in a new suit is a weak jump: a
                                // six-card suit.  Skip splinters (double jumps).
                                // Opener's extras-ladder jump-shift is instead a
                                // strong 5-4, so the jumped suit is only 4+.
                                if jump == 1 {
                                    let floor = if opener_ladder_rebid { 4 } else { 6 };
                                    players[who]
                                        .narrow_length(suit, Range::at_least(floor, LENGTH_CAP));
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

                    // Opener's extras-ladder rebid shows extras and — for a
                    // new-suit rung — a five-card opened suit.  Sound floors: the
                    // jump-rebid is 16+, the reverse 17+, the jump-shift 18+.
                    if crate::bidding::american::opener_extras_ladder()
                        && !side_acted[defending_parity]
                        && is_opening_side
                        && lane == opener_lane
                        && lane_bids[lane] == 1
                        && opening_bid.level.get() == 1
                        && matches!(opening_bid.strain, Strain::Clubs | Strain::Diamonds)
                        && let (Some(bid_suit), Some(opened)) =
                            (bid.strain.suit(), opening_bid.strain.suit())
                    {
                        let jump = bid
                            .level
                            .get()
                            .saturating_sub(cheapest_level(highest, bid.strain));
                        let responder_bid_it =
                            lane_suits[(lane + 2) % 4] & (1 << bid_suit as u8) != 0;
                        if bid_suit == opened {
                            // Jump-rebid of opener's own suit.
                            if jump >= 1 {
                                players[who].narrow_points(Range::at_least(16, POINTS_CAP));
                            }
                        } else if !responder_bid_it {
                            // Reverse (non-jump two-level, higher suit) or
                            // jump-shift (single jump): a five-card opened suit.
                            let reverse = jump == 0
                                && bid.level.get() == 2
                                && (bid.strain as u8) > (Strain::from(opened) as u8);
                            let jump_shift = jump == 1;
                            if reverse || jump_shift {
                                players[who].narrow_length(opened, Range::at_least(5, LENGTH_CAP));
                                let floor = if jump_shift { 18 } else { 17 };
                                players[who].narrow_points(Range::at_least(floor, POINTS_CAP));
                            }
                        }
                    }

                    // Opener's major jump-rebid (set_opener_major_jump_rebid):
                    // a 3M jump in opener's own opened major over 1♥-1♠ / 1M-1NT
                    // shows 16+.  Natural, so the six-card length is read above
                    // (the `i_bid_it` branch); add the strength floor here.
                    if crate::bidding::american::opener_major_jump_rebid()
                        && !side_acted[defending_parity]
                        && is_opening_side
                        && lane == opener_lane
                        && lane_bids[lane] == 1
                        && opening_bid.level.get() == 1
                        && matches!(opening_bid.strain, Strain::Hearts | Strain::Spades)
                        && bid.strain == opening_bid.strain
                        && bid
                            .level
                            .get()
                            .saturating_sub(cheapest_level(highest, bid.strain))
                            >= 1
                    {
                        players[who].narrow_points(Range::at_least(16, POINTS_CAP));
                    }

                    // Stayman: read opener's major answer and responder's
                    // strength (opponents silent) so the floor judges the fit and
                    // accepts or declines invitations.
                    if stayman && is_opening_side && !side_acted[defending_parity] {
                        let responder_lane = (opener_lane + 2) % 4;
                        if index == opening_index + 2 {
                            // Responder's 2♣ Stayman shows invitational+ values —
                            // unless garbage or crawling Stayman is on, where a weak
                            // hand may bid 2♣ to escape, so the floor must not assume
                            // 8+.
                            if !crate::bidding::american::garbage_stayman()
                                && !crate::bidding::american::crawling_stayman()
                            {
                                players[who].narrow_points(Range::at_least(8, POINTS_CAP));
                            }
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

        // A one-level Rubens transfer records its meaning likewise (see
        // [`set_rubens_transfer_reading`]) — but only for the advancer's own
        // side: the transfer semantics are *our* agreement, and an opponent's
        // in-band advance may be a genuine suit (asserting length in the suit
        // above would poison the sampler).  Suppression above stays side-blind:
        // it only loses information, never asserts any.
        if let Some((transfer_index, suit, min_len)) = rubens_transfer {
            let who = relative_of(len, transfer_index);
            if matches!(who, Relative::Me | Relative::Partner) {
                let who = who as usize;
                players[who].narrow_length(suit, Range::at_least(min_len, LENGTH_CAP));
                players[who].narrow_points(Range::at_least(10, POINTS_CAP));
            }
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

        // A Meckwell overcall of their 1NT.  The two-way X (single 6+ minor OR both
        // majors) is a disjunction that shares no sound per-suit fact — the one-suiter
        // arm holds short majors, the both-majors arm long majors — so only the points
        // floor is recorded (as the Woolsey / penalty double).  The 2♣/2♦ pin the real
        // ≥4 minor (the natural ≥5 reading was suppressed); the unknown major surfaces
        // from the residual.  Natural 2♥/2♠ and the 2NT both-minors are read elsewhere.
        if let Some(meckwell) = meckwell {
            let who = relative_of(len, meckwell.overcall_index) as usize;
            match meckwell.kind {
                MeckwellKind::TwoWayDouble => {}
                MeckwellKind::ClubsMajor => {
                    players[who].narrow_length(Suit::Clubs, Range::at_least(4, LENGTH_CAP));
                }
                MeckwellKind::DiamondsMajor => {
                    players[who].narrow_length(Suit::Diamonds, Range::at_least(4, LENGTH_CAP));
                }
            }
            players[who].narrow_points(Range::at_least(meckwell.floor, POINTS_CAP));
        }

        // Their two-suiter over our 1M.  A Michaels cue records the other major's
        // 5-card floor (the unknown minor is a disjunction left to the residual);
        // the both-minors (2NT) pins both.  No points floor — mini-maxi Michaels
        // styles run too wide for a sound one.
        if let Some(two_suiter) = two_suiter {
            let who = relative_of(len, two_suiter.index) as usize;
            match two_suiter.michaels_om {
                Some(om) => players[who].narrow_length(om, Range::at_least(5, LENGTH_CAP)),
                None => {
                    players[who].narrow_length(Suit::Clubs, Range::at_least(5, LENGTH_CAP));
                    players[who].narrow_length(Suit::Diamonds, Range::at_least(5, LENGTH_CAP));
                }
            }
        }

        // Our Gladiator advance: record the real shape the suppressed call hid.
        // Guarded to our own side (the advance is our agreement) — an opponent's
        // in-band call must never be narrowed to the phantom suit.
        if let Some(gladiator) = gladiator {
            let who = relative_of(len, gladiator.index);
            if matches!(who, Relative::Me | Relative::Partner) {
                let who = who as usize;
                match gladiator.advance {
                    // The relay is weak-or-invitational (< game); only the point
                    // cap is a sound per-call fact (the suit is revealed by the
                    // XYZ-style rebid over 2♦, read naturally).
                    GladiatorAdvance::Relay => {
                        players[who].narrow_points(Range::new(0, 9));
                    }
                    GladiatorAdvance::Cue { o } => {
                        players[who].narrow_length(o, Range::at_least(4, LENGTH_CAP));
                        players[who].narrow_points(Range::at_least(8, POINTS_CAP));
                    }
                    // Delayed cue: exactly 3 in the unbid major, INV+ (checks the
                    // 5-3 fit an exactly-5-major overcall can hold).
                    GladiatorAdvance::DelayedCue { o } => {
                        players[who].narrow_length(o, Range::new(3, 3));
                        players[who].narrow_points(Range::at_least(8, POINTS_CAP));
                    }
                    GladiatorAdvance::Splinter { o, m } => {
                        players[who].narrow_length(o, Range::at_least(4, LENGTH_CAP));
                        players[who].narrow_length(m, Range::new(0, 1));
                        players[who].narrow_points(Range::at_least(10, POINTS_CAP));
                    }
                    GladiatorAdvance::BothMinors => {
                        players[who].narrow_length(Suit::Clubs, Range::at_least(5, LENGTH_CAP));
                        players[who].narrow_length(Suit::Diamonds, Range::at_least(5, LENGTH_CAP));
                        players[who].narrow_points(Range::at_least(10, POINTS_CAP));
                    }
                    GladiatorAdvance::Minor { o, minor } => {
                        players[who].narrow_length(o, Range::at_least(5, LENGTH_CAP));
                        players[who].narrow_length(minor, Range::at_least(5, LENGTH_CAP));
                        players[who].narrow_points(Range::at_least(10, POINTS_CAP));
                    }
                    // 2NT = weak transfer to clubs: 6+ clubs, sub-invitational.
                    GladiatorAdvance::ClubTransfer => {
                        players[who].narrow_length(Suit::Clubs, Range::at_least(6, LENGTH_CAP));
                        players[who].narrow_points(Range::new(0, 7));
                    }
                }
            }
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

        Self {
            players,
            control_bid,
        }
    }

    /// The last call the M6.4 classifier read as a control bid: its auction
    /// index and the agreed suit (see [`classify_high_bid`])
    #[must_use]
    pub(super) fn control_bid(&self) -> Option<(u8, Suit)> {
        self.control_bid
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
        control_bid: None,
    }
}

/// How a four-plus-level suit bid in the slam zone reads (M6.4)
enum HighBid {
    /// To play (or an unreadable splinter) — suppressed from the natural
    /// walk, nothing recorded: the honest envelope is wide (a preempt, a
    /// two-suiter picture jump, a fast-arrival sign-off), and flooring a six
    /// here measurably rerouted 33-count hands into thin suit slams
    ToPlay,
    /// A control bid agreeing `trump`, most recently shown by seat `shower`
    Control { trump: Suit, shower: usize },
    /// Not the classifier's call — fall through to the generic walk
    Unclaimed,
}

/// Classify an unalerted suit bid at the four level or higher: control bid or
/// to play (M6.4)
///
/// The deterministic rule, calibrated to what this system actually bids: the
/// bid is a **control bid** iff the bidder *bypassed* the suit — it was
/// biddable more cheaply (same level, lower strain) at their first
/// suit-showing call and they chose another suit (`1♦–1♠–2♦–4♥`: 1♥ was
/// available under 1♠, so hearts are short and 4♥ agrees diamonds — the
/// partnership's most recently shown suit, BWS's priority).  A suit *above*
/// the first-shown one was never denied: both the book and the floor bid the
/// cheaper suit first holding a longer higher one (a 1♥ response or a heart
/// transfer on 6♠5♥ is real traffic — the first A/B bled six IMPs a fired
/// board pulling those natural 4♠s to the "agreed" minor), so it reads to
/// play: suppressed, but with nothing floored.
///
/// "Shown" folds the walk's floors so far with the projection overlay, so a
/// transferred suit counts for its transferee.  (The overlay is the
/// full-auction fold, so an artificial call *after* `index` could in principle
/// leak into the test; slam-zone auctions all but never continue artificially
/// after an unalerted four-level bid, and the leak can only re-label a control
/// bid — it never floors a phantom suit.)
#[allow(clippy::too_many_arguments)]
fn classify_high_bid(
    auction: &[Call],
    index: usize,
    bid: Bid,
    len: usize,
    opening_index: usize,
    players: &[Inference; 4],
    overlay: &[Inference; 4],
    suppressed_so_far: u64,
) -> HighBid {
    let Some(suit) = bid.strain.suit() else {
        return HighBid::Unclaimed;
    };
    let who = relative_of(len, index) as usize;
    let partner = (who + 2) % 4;
    let shown =
        |seat: usize, s: Suit| players[seat].length(s).min >= 4 || overlay[seat].length(s).min >= 4;

    // Rebids of one's own suit and raises of partner's stay with the generic
    // walk (six-plus / support) — both are to play.
    if shown(who, suit) || shown(partner, suit) {
        return HighBid::Unclaimed;
    }

    if !Suit::ASC.into_iter().any(|s| s != suit && shown(who, s)) {
        // The bidder has shown nothing: the suit can be their longest — to
        // play (which covers the possible splinter below game in partner's
        // major, `1♥–4♣`, since nothing is recorded either way).
        return HighBid::ToPlay;
    }

    // The bidder's first suit-showing call: its shown suit `r` and level — a
    // natural bid's own suit, or an artificial call's *projected* one (the
    // transfer's major, not its named diamond; the fold of the seat's floors
    // stands in for the per-call projection, a recency approximation).  Track
    // the highest bid standing before it for the bypass legality test.
    let mut first_shown: Option<(Suit, u8)> = None;
    let mut highest_before: Option<Bid> = None;
    for (j, &call) in auction.iter().enumerate().take(index).skip(opening_index) {
        let Call::Bid(prior) = call else {
            continue;
        };
        if j % 4 == index % 4 {
            let shown_suit = if j < 64 && suppressed_so_far >> j & 1 != 0 {
                Suit::ASC
                    .into_iter()
                    .filter(|&s| overlay[who].length(s).min >= 4)
                    .max_by_key(|&s| (overlay[who].length(s).min, s as u8))
            } else {
                prior.strain.suit()
            };
            if let Some(r) = shown_suit {
                first_shown = Some((r, prior.level.get()));
                break;
            }
        }
        if highest_before.is_none_or(|h| outranks(prior, h)) {
            highest_before = Some(prior);
        }
    }
    let Some((r, r_level)) = first_shown else {
        return HighBid::Unclaimed;
    };

    // The longer-major response discipline (`set_longer_major_response`)
    // swaps two verdicts when the bidder's first call was a one-level major
    // response to partner's minor opening: a 1♥ response denies longer
    // spades, so a later spade bid *is* a bypass (control) even though it
    // sits above the response; and a 1♠ response may conceal equal-length
    // five-plus hearts (5-5 responds 1♠), so the skipped 1♥ no longer proves
    // shortness (to play).
    let response_to_partners_minor = r_level == 1
        && relative_of(len, opening_index) as usize == partner
        && matches!(auction[opening_index], Call::Bid(opening)
            if opening.level.get() == 1
                && matches!(opening.strain.suit(), Some(Suit::Clubs | Suit::Diamonds)));
    let discipline =
        response_to_partners_minor && crate::bidding::american::longer_major_response();

    // Bypassed: the bid suit sat below the first-shown suit at the same level
    // and the bidder skipped it — it cannot be long, so this is a control bid.
    // Otherwise the suit was never denied and reads to play.
    let bypassed = match (discipline, r, suit) {
        (true, Suit::Hearts, Suit::Spades) => true,
        (true, Suit::Spades, Suit::Hearts) => false,
        _ => {
            (suit as u8) < (r as u8)
                && highest_before.is_none_or(|h| outranks(Bid::new(r_level, bid.strain), h))
        }
    };
    if !bypassed {
        return HighBid::ToPlay;
    }

    // The agreed suit: the partnership's most recently shown one.
    for j in (opening_index..index).rev() {
        if j % 2 != index % 2 {
            continue; // the opponents' calls agree nothing for us
        }
        let Call::Bid(prior) = auction[j] else {
            continue;
        };
        let seat = relative_of(len, j) as usize;
        if j < 64 && suppressed_so_far >> j & 1 != 0 {
            let candidate = Suit::ASC
                .into_iter()
                .filter(|&s| {
                    s != suit
                        && (overlay[seat].length(s).min >= 4 || players[seat].length(s).min >= 4)
                })
                .max_by_key(|&s| {
                    (
                        overlay[seat].length(s).min.max(players[seat].length(s).min),
                        s as u8,
                    )
                });
            if let Some(trump) = candidate {
                return HighBid::Control {
                    trump,
                    shower: seat,
                };
            }
        } else if let Some(s) = prior.strain.suit()
            && s != suit
        {
            return HighBid::Control {
                trump: s,
                shower: seat,
            };
        }
    }
    HighBid::Unclaimed
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

    // Project the call made at `index`, authored by `classifier`, into the overlay.
    let mut project_call = |index: usize, classifier: &dyn super::trie::Classifier| {
        let (Some(&made), Some(rules)) = (auction.get(index), classifier.as_rules()) else {
            return;
        };

        // The logit of a call is the max over its rules, so a hand could satisfy
        // any one of them — the sound forward envelope is their union.
        let projection = rules
            .rules()
            .iter()
            .filter(|rule| rule.call() == made)
            .map(|rule| rule.project(context))
            .reduce(|acc, p| acc.union(&p));

        // A call is artificial — decode it — when its authoring rule *alerts* it.
        // The alert is now the complete, exhaustive signal: every artificial call
        // in the book carries one (guarded by the `artificial_calls_are_alerted`
        // invariant test), so the old structural `artificial(p, made)` fallback
        // has been retired (alert-by-disclosed-meaning, the move modern bridge
        // made retiring "X is self-alerting").
        let alerted = alert_reading()
            && rules
                .rules()
                .iter()
                .any(|rule| rule.call() == made && rule.alert().is_some());

        if let Some(projection) = projection.filter(|_| alerted) {
            let who = relative_of(len, index) as usize;
            players[who] = players[who].intersect(&projection);
            if index < 64 {
                suppressed |= 1 << index;
            }
        }
    };

    if fallback_projection_enabled() {
        // Decode every prior call by the classifier that *authored* it — node or
        // guarded fallback — so contested conventions (transfers, Leaping Michaels,
        // the Lebensohl cue) survive later competition without a per-convention reader.
        let trie = prefixes.root();
        for index in 0..len {
            if let Some(classifier) = trie.authoring_classifier(context, &auction[..index]) {
                project_call(index, classifier);
            }
        }
    } else {
        // Exact-node classifiers only — the shipped default; fallback-authored
        // conventions are read by the hand-written readers in [`Inferences::read`].
        for (prefix, classifier) in prefixes.clone() {
            project_call(prefix.len(), classifier);
        }
    }

    (players, suppressed)
}

/// Whether a call's projection floors a suit other than the one it names
///
/// The structural artificial-call detector, falling out of the projection itself:
/// a natural call floors its own strain (1♠ → 5+♠) or no suit (1NT → points only);
/// an artificial one floors a suit it did not name (Jacoby 2♦ → 5+♥, Landy 2♣ →
/// 4-4 majors).  A min-length floor of four-plus on a non-named suit is the witness
/// — above any natural by-product, below every convention's real shape.
///
/// **The "named" suit generalizes past bids.**  A call is natural when it offers to
/// play what it declares, artificial when it points partner at some *other* suit:
/// - a **bid** names its own strain;
/// - a **double / redouble** names the *doubled strain* — the contract it offers to
///   defend.  A penalty double floors that strain (or nothing) → natural; a takeout
///   double floors an *unbid* suit (support for where it sends partner) → artificial;
/// - a **pass** redirects from nothing → never artificial (a trap pass defends what
///   is on the table);
/// - a **transfer completion** names the suit it will play, flooring no other →
///   natural, so already `false`.
///
/// This is a *sound sufficient* witness, not a complete one: a takeout double whose
/// authoring rule floors nothing (opaque shape predicates, e.g. the direct takeout
/// double) reads `false` here though it is takeout by meaning — such calls are
/// classified artificial by their `.alert(...)` instead, exactly as shape-only
/// artificial bids are.
///
/// **Retired from the decode gate** — alerts now carry the signal exhaustively.
/// This survives test-only, as the `artificial_calls_are_alerted` invariant guard:
/// any future artificial call added without an `.alert(...)` must fail that test
/// rather than silently lose its decoding.
#[cfg(test)]
fn artificial(projection: &Inference, made: Call, doubled: Option<Strain>) -> bool {
    // The "named" suit is the one the call offers to play: a bid names its own
    // strain; a double/redouble "names" the doubled strain it offers to defend.
    // Artificial = the projection floors some *other* suit ≥4 — the call is really
    // pointing partner at a suit it did not name.  A pass redirects from nothing.
    let named = match made {
        Call::Bid(bid) => bid.strain.suit(),
        Call::Double | Call::Redouble => doubled.and_then(|strain| strain.suit()),
        Call::Pass => return false,
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

/// The Rubens-artificial calls of an advance, and the advance's strength reading
///
/// In a [Rubens advance][super::instinct::overcall_shape] of a simple overcall,
/// some calls *name* a suit they do not *hold*: the advancer's transfer (a relay
/// to the next suit up) or cue-raise, and the overcaller's forced completion.
/// Returns `(suppress, cue, transfer)` — `suppress` lists those indices, whose
/// bid suit must not be read as natural length; `cue` is `(index, Y)` of a
/// two-level cue-raise, read separately as a limit-plus raise (three-plus cards
/// in partner's overcall `Y`, ten-plus points); `transfer` is
/// `(index, suit, min-len)` of a one-level transfer's meaning — the transfer
/// into partner's suit is the same limit-plus raise (`(index, Y, 3)`), a
/// new-suit transfer shows its own five-card target (`(index, target, 5)`),
/// both ten-plus points ([`set_rubens_transfer_reading`], recorded post-walk
/// for the advancer's *own side only* — an opponent's in-band advance may be
/// natural).
///
/// The shown values are what let the overcaller judge game — and the completion
/// is a forced relay, still never read as length (soundness over tightness, as
/// with transfers over our own notrump).
#[allow(clippy::type_complexity)]
fn rubens_reading(
    auction: &[Call],
) -> (
    [Option<usize>; 2],
    Option<(usize, Suit)>,
    Option<(usize, Suit, u8)>,
) {
    let none = ([None, None], None, None);
    // The bidder's knob governs the reading too: with Rubens advances off, an
    // advance in the band is a genuine suit and must be read naturally.
    if !super::instinct::rubens_advances_enabled() {
        return none;
    }
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
            ([Some(advance_index), None], Some((advance_index, y)), None)
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
    let target_suit = Suit::ASC[(source as u8 + 1) as usize];
    let target = Strain::from(target_suit);
    // The overcaller completes through opener's lead-directing double too, so
    // the completion stays a relay (never a holding) in both shapes.
    let completion = (matches!(
        auction.get(advance_index + 1),
        Some(Call::Pass | Call::Double)
    ) && auction.get(advance_index + 2) == Some(&Call::Bid(Bid::new(2, target))))
    .then_some(advance_index + 2);
    // The transfer's meaning, fixed the moment it is made (the completion is
    // not required): into partner's suit = the limit-plus raise, a new suit =
    // the advancer's own five-card target.
    let transfer = rubens_transfer_reading().then_some(if target_suit == y {
        (advance_index, y, 3)
    } else {
        (advance_index, target_suit, 5)
    });
    ([Some(advance_index), completion], None, transfer)
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

/// Their two-suiter over our 1♥/1♠ opening
/// ([`set_uvu_over_majors`][crate::bidding::american::set_uvu_over_majors])
///
/// The defenders' *direct* action over our major opening, read as the
/// NATURAL-family two-suiters: a `2M` cue of our own major is Michaels (5+ in
/// the other major plus 5+ in an unknown minor), a `(2NT)` jump is unusual
/// (both minors).  Without this reading the natural walk takes the Michaels
/// cue as a genuine 5-card suit *in our own major* — the sampler then deals
/// the cue-bidder length in the one suit the convention all but denies.  The
/// unknown Michaels minor is a disjunction the per-suit ranges cannot pin, so
/// only the other-major floor is recorded; no points floor either (mini-maxi
/// Michaels styles run too wide for a sound one).
#[derive(Clone, Copy)]
struct TwoSuiterReading {
    /// Index of their two-suited call
    index: usize,
    /// The other major shown by a Michaels cue of our opened major, or
    /// [`None`] for the both-minors `(2NT)` (which needs no suppression — a
    /// notrump bid never enters the walk's natural suit reading)
    michaels_om: Option<Suit>,
}

impl TwoSuiterReading {
    const fn suppresses(self, index: usize) -> bool {
        self.michaels_om.is_some() && index == self.index
    }
}

fn two_suiter_reading(auction: &[Call]) -> Option<TwoSuiterReading> {
    if !crate::bidding::american::uvu_over_majors() {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    let Call::Bid(opening) = auction[opening_index] else {
        return None;
    };
    if opening.level.get() != 1 || !matches!(opening.strain, Strain::Hearts | Strain::Spades) {
        return None;
    }
    let Some(&Call::Bid(direct)) = auction.get(opening_index + 1) else {
        return None;
    };

    let index = opening_index + 1;
    if direct == Bid::new(2, opening.strain) {
        let om = if opening.strain == Strain::Hearts {
            Suit::Spades
        } else {
            Suit::Hearts
        };
        Some(TwoSuiterReading {
            index,
            michaels_om: Some(om),
        })
    } else if direct == Bid::new(2, Strain::Notrump) {
        Some(TwoSuiterReading {
            index,
            michaels_om: None,
        })
    } else {
        None
    }
}

/// Our **Gladiator** advance of a 1NT overcall of their major
/// ([`set_nt_overcall_gladiator`][crate::bidding::american::set_nt_overcall_gladiator])
///
/// The advancer's artificial calls under `[1M, 1NT, P, ?]` — the `2♣` relay (and
/// its forced `2♦` completion), the cue of their major (Stayman for the unbid
/// major), the `3M` splinter, and the `4M` both-minor Leaping Michaels — are bids
/// of a suit the caller does *not* hold; the natural walk would floor a phantom
/// suit.  Their indices are suppressed and the real shape recorded post-walk.  The
/// natural advances (`2♦`/`2O`, the 3-level naturals, `4O`) read off the walk and
/// never enter here.
#[derive(Clone, Copy)]
enum GladiatorAdvance {
    /// `2♣` relay (weak / invitational) — no sound per-suit floor.
    Relay,
    /// Cue of their major = Stayman: 4+ in the unbid major `o`, INV+.
    Cue { o: Suit },
    /// Delayed cue (`2♣` relay → forced `2♦` → cue of their major): exactly 3 in
    /// the unbid major `o`, INV+ — the 5-3-fit check.
    DelayedCue { o: Suit },
    /// `3M` splinter: 4+ `o`, 0–1 in their major `m`, GF.
    Splinter { o: Suit, m: Suit },
    /// `4M` Leaping Michaels: both minors 5+, GF.
    BothMinors,
    /// `4♣`/`4♦` Leaping Michaels: 5+ `o` + 5+ the named `minor`, GF.
    Minor { o: Suit, minor: Suit },
    /// `2NT`: a weak transfer to clubs (6+♣) — not a balanced notrump.
    ClubTransfer,
}

#[derive(Clone, Copy)]
struct GladiatorReading {
    /// Index of the advancer's Gladiator call
    index: usize,
    advance: GladiatorAdvance,
    /// Bitset of indices whose natural suit reading the walk must skip
    suppress: u64,
}

impl GladiatorReading {
    const fn suppresses(self, index: usize) -> bool {
        index < 64 && self.suppress >> index & 1 != 0
    }
}

fn gladiator_reading(auction: &[Call]) -> Option<GladiatorReading> {
    if !crate::bidding::american::nt_overcall_gladiator() {
        return None;
    }
    let open = auction.iter().position(|&c| c != Call::Pass)?;
    let Call::Bid(opening) = auction[open] else {
        return None;
    };
    let m = opening.strain.suit()?;
    if opening.level.get() != 1 || !matches!(m, Suit::Hearts | Suit::Spades) {
        return None;
    }
    // Our 1NT overcall, then the advancer.  RHO usually passes; over RHO's (2♣)
    // systems-on overcall we mirror the book rebase — their 2♣ maps to a pass and
    // advancer's Double to the stolen 2♣ relay — and re-read, so every (2♣)
    // continuation (relay, delayed cue, cue-Stayman, club transfer) decodes
    // through the uncontested logic below with the same call indices.  Any other
    // RHO action leaves it to the natural walk.
    if auction.get(open + 1) != Some(&Call::Bid(Bid::new(1, Strain::Notrump))) {
        return None;
    }
    if auction.get(open + 2) == Some(&Call::Bid(Bid::new(2, Strain::Clubs))) {
        let mut stripped = auction.to_vec();
        stripped[open + 2] = Call::Pass;
        if auction.get(open + 3) == Some(&Call::Double) {
            stripped[open + 3] = Call::Bid(Bid::new(2, Strain::Clubs));
        }
        return gladiator_reading(&stripped);
    }
    if auction.get(open + 2) != Some(&Call::Pass) {
        return None;
    }
    let index = open + 3;
    let Some(&Call::Bid(bid)) = auction.get(index) else {
        return None;
    };
    let o = if m == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };

    // `index ≤ 6` (at most three leading passes), so the shifts never overflow.
    let mut suppress = 0u64;
    let advance = if bid == Bid::new(2, Strain::Clubs) {
        suppress |= 1 << index;
        // The overcaller's forced 2♦ completion (relay, P, 2♦) says nothing of
        // diamonds — suppress it too.
        let mut delayed = false;
        if auction.get(index + 2) == Some(&Call::Bid(Bid::new(2, Strain::Diamonds))) {
            suppress |= 1 << (index + 2);
            // Delayed cue at index+4 (relay, P, 2♦, P, cue-of-their-major): a
            // phantom-suit call too (advancer holds exactly 3 `o`, not `m`).
            if auction.get(index + 4) == Some(&Call::Bid(Bid::new(2, opening.strain))) {
                suppress |= 1 << (index + 4);
                delayed = true;
            }
        }
        if delayed {
            GladiatorAdvance::DelayedCue { o }
        } else {
            GladiatorAdvance::Relay
        }
    } else if bid == Bid::new(2, opening.strain) {
        suppress |= 1 << index;
        GladiatorAdvance::Cue { o }
    } else if bid == Bid::new(3, opening.strain) {
        suppress |= 1 << index;
        GladiatorAdvance::Splinter { o, m }
    } else if bid == Bid::new(4, opening.strain) {
        suppress |= 1 << index;
        GladiatorAdvance::BothMinors
    } else if bid == Bid::new(2, Strain::Notrump) {
        suppress |= 1 << index;
        // The overcaller's forced 3♣ transfer completion says nothing of clubs.
        if auction.get(index + 2) == Some(&Call::Bid(Bid::new(3, Strain::Clubs))) {
            suppress |= 1 << (index + 2);
        }
        GladiatorAdvance::ClubTransfer
    } else if bid == Bid::new(4, Strain::Clubs) {
        GladiatorAdvance::Minor {
            o,
            minor: Suit::Clubs,
        }
    } else if bid == Bid::new(4, Strain::Diamonds) {
        GladiatorAdvance::Minor {
            o,
            minor: Suit::Diamonds,
        }
    } else {
        return None;
    };

    Some(GladiatorReading {
        index,
        advance,
        suppress,
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
        || a::meckwell_enabled()
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

/// Which Meckwell defense call the defending side made over their 1NT
#[derive(Clone, Copy)]
enum MeckwellKind {
    /// `X` — a single 6+ minor OR both majors.  A double naming no suit, and a
    /// disjunction (short majors OR long majors) the per-suit framework cannot pin, so
    /// only the points floor is a sound fact (as the Woolsey / penalty double).
    TwoWayDouble,
    /// `2♣` — clubs (real, ≥ 4) + an unknown major.  As DONT's `ClubsMajor`.
    ClubsMajor,
    /// `2♦` — diamonds (real, ≥ 4) + an unknown major.  As DONT's `DiamondsMajor`.
    DiamondsMajor,
}

/// A Meckwell overcall of their 1NT (`X`/`2♣`/`2♦`) and the advancer's relay
///
/// Meckwell's natural `2♥`/`2♠` single-suiters name real suits (read by the generic
/// walk) and the `2NT` both-minors is the Unusual overlay, so only the two-way `X` and
/// the `2♣`/`2♦` minor + major are decoded here.  Mirrors [`dont_reading`].
#[derive(Clone, Copy)]
struct MeckwellReading {
    overcall_index: usize,
    kind: MeckwellKind,
    floor: u8,
    /// The advancer's relay — `2♣` over the `X`, or the `2♦`/`2♥`/`2♠` pass-or-correct
    /// over `2♣`/`2♦` (a preference among partner's suits, not own length).
    advance_suppress: Option<usize>,
}

impl MeckwellReading {
    /// The `X` (a double) names no suit, so only the `2♣`/`2♦` overcalls suppress
    /// their own natural reading; the advancer's relay is always suppressed.
    fn suppresses(&self, index: usize) -> bool {
        (!matches!(self.kind, MeckwellKind::TwoWayDouble) && self.overcall_index == index)
            || self.advance_suppress == Some(index)
    }
}

/// Read a Meckwell overcall of their 1NT, gated on
/// [`meckwell_enabled`][crate::bidding::american::meckwell_enabled] and the auction
/// being `1NT` then the defending side's first action being a Meckwell call
fn meckwell_reading(auction: &[Call]) -> Option<MeckwellReading> {
    if !crate::bidding::american::meckwell_enabled() {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;
    let floor = crate::bidding::american::natural_overcall_points().0;

    // The defending side's FIRST action — a Meckwell `X`/`2♣`/`2♦` (natural `2♥`/`2♠`
    // and anything else fall through to the generic reading).
    let (overcall_index, kind) = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Double if index % 2 != opener_parity => {
                Some(Some((index, MeckwellKind::TwoWayDouble)))
            }
            Call::Bid(bid) if index % 2 != opener_parity => {
                let kind = if bid == Bid::new(2, Strain::Clubs) {
                    Some(MeckwellKind::ClubsMajor)
                } else if bid == Bid::new(2, Strain::Diamonds) {
                    Some(MeckwellKind::DiamondsMajor)
                } else {
                    None
                };
                Some(kind.map(|kind| (index, kind)))
            }
            // The opener's side acted (a response), or a defender did something else.
            _ => Some(None),
        })
        .flatten()?;

    // The advancer's relay: `2♣` over the `X` (names a minor, not own clubs), or the
    // `2♦`/`2♥`/`2♠` preference over a two-suiter.  Both scans jump over every opponent
    // call so a contested relay is covered (the relay is only legal as the immediate
    // response).
    let advance_suppress = match kind {
        MeckwellKind::TwoWayDouble => auction
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

    Some(MeckwellReading {
        overcall_index,
        kind,
        floor,
        advance_suppress,
    })
}

/// Apply the meaning of the opening bid (the first non-pass call)
fn apply_opening(inf: &mut Inference, bid: Bid, seat: u8) {
    // Rule-of-20 light openers (default on) drop the one-level suit point floor
    // from 12 to 10 in any seat; third/fourth seat opens majors lighter still (9).
    let light = crate::bidding::american::rule_of_20_enabled();
    let major_floor = if seat >= 3 {
        9
    } else if light {
        10
    } else {
        12
    };
    let minor_floor = if light { 10 } else { 12 };
    let majors_light = Range::new(major_floor, 21);
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
            inf.narrow_points(Range::new(minor_floor, 21));
        }
        (1, Strain::Clubs) => {
            inf.narrow_length(Suit::Clubs, Range::at_least(3, LENGTH_CAP));
            inf.narrow_length(Suit::Hearts, Range::new(0, 4));
            inf.narrow_length(Suit::Spades, Range::new(0, 4));
            inf.narrow_points(Range::new(minor_floor, 21));
        }
        (1, Strain::Notrump) => {
            // Balanced, OR — since the shipped `Wide6322` shape also opens 1NT
            // on a 6322 with a six-card minor — a minor running to six.  Majors
            // stay 2–5 (a balanced 5332 major); minors widen to 2–6.  Set the
            // four suits directly: `narrow_length` only intersects, so clamping
            // via `balanced()` first would pin the minors back to five.
            inf.narrow_length(Suit::Spades, Range::new(2, 5));
            inf.narrow_length(Suit::Hearts, Range::new(2, 5));
            inf.narrow_length(Suit::Clubs, Range::new(2, 6));
            inf.narrow_length(Suit::Diamonds, Range::new(2, 6));
            // Plain HCP 15–17 gates the opening (fifths archived).  The shipped
            // rule-of-N+8 scale reads a flat 4-3-3-3 one under its HCP and a
            // 5422/6322 one over (9-card long suits − 8); the legacy upgrade
            // scale adds at most +1 the same way.  Sound band 15−slack..18 —
            // the slack term keeps the legacy opt-out arm exact.  ponytail:
            // exact for the shipped plain-HCP gauge; the archived
            // `set_one_notrump_fifths` knob, if ever revived, would re-widen
            // this to 14–19.
            let slack = crate::bidding::constraint::flat_hcp_slack();
            inf.narrow_points(Range::new(15 - slack, 18));
        }
        (2, Strain::Clubs) => {
            // Strong and artificial: 22+ points, but nothing about shape.
            inf.narrow_points(Range::at_least(20, POINTS_CAP));
        }
        (2, Strain::Notrump) => {
            balanced(inf);
            // As with 1NT: `fifths(20.0..22.0)` admits a quack-heavy 23-count
            // (fifths within 1.6 of raw HCP), so the sound point envelope is
            // 19–23, not 19–22 — and rule of N+8 gives a flat 4-3-3-3 floor
            // another point back.
            let slack = crate::bidding::constraint::flat_hcp_slack();
            inf.narrow_points(Range::new(19 - slack, 23));
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
        // Rule of 20 (default on) opens sound 10-11 counts, so the floor is 10.
        assert_eq!(one_heart.rho().points, Range::new(10, 21));

        // A strong notrump is balanced-or-6322-minor (the shipped Wide6322): a
        // major stays 2–5 (a balanced 5332 major), a minor widens to 2–6 (the
        // 6322's six-card minor); an artificial 2♣ says only "strong".
        let one_nt = read(&[bid(1, Strain::Notrump)]);
        assert_eq!(one_nt.rho().length(Suit::Spades), Range::new(2, 5));
        assert_eq!(one_nt.rho().length(Suit::Diamonds), Range::new(2, 6));
        // Plain HCP 15–17: a flat 4333 reads one under on the shipped
        // rule-of-N+8 scale, a semi-balanced 5422/6322 one over → 14–18.
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
    fn opener_extras_ladder_reads_extras() {
        use crate::bidding::american::set_opener_extras_ladder;
        let d = bid(1, Strain::Diamonds);
        let s = bid(1, Strain::Spades);
        let p = Call::Pass;
        set_opener_extras_ladder(true);
        // Opener (partner of the hero to act) after 1♦ – 1♠ – X.
        // Jump-rebid 3♦: a self-sufficient six-plus diamonds, 16+.
        let jr = read(&[d, p, s, p, bid(3, Strain::Diamonds), p]);
        assert!(jr.partner().length(Suit::Diamonds).min >= 6);
        assert!(jr.partner().points.min >= 16);
        // Reverse 2♥: five-plus diamonds, four-plus hearts, 17+.
        let rev = read(&[d, p, s, p, bid(2, Strain::Hearts), p]);
        assert!(rev.partner().length(Suit::Diamonds).min >= 5);
        assert!(rev.partner().length(Suit::Hearts).min >= 4);
        assert!(rev.partner().points.min >= 17);
        // Jump-shift 3♣: five-plus diamonds, 18+, and clubs read as the strong
        // 4+ second suit — NOT the weak-jump six (the phantom-suit fix).
        let js = read(&[d, p, s, p, bid(3, Strain::Clubs), p]);
        assert!(js.partner().length(Suit::Diamonds).min >= 5);
        assert!(js.partner().points.min >= 18);
        assert_eq!(
            js.partner().length(Suit::Clubs),
            Range::at_least(4, LENGTH_CAP)
        );
        set_opener_extras_ladder(true);
    }

    #[test]
    fn opener_major_jump_rebid_reads_extras() {
        use crate::bidding::american::set_opener_major_jump_rebid;
        let h = bid(1, Strain::Hearts);
        let s = bid(1, Strain::Spades);
        let p = Call::Pass;
        set_opener_major_jump_rebid(true);
        // Opener after 1♥ – 1♠ – 3♥: jump-rebid of a six-plus major, 16+.
        let jr = read(&[h, p, s, p, bid(3, Strain::Hearts), p]);
        assert!(jr.partner().length(Suit::Hearts).min >= 6);
        assert!(jr.partner().points.min >= 16);
        set_opener_major_jump_rebid(true);
    }

    /// The M6.4 deterministic rule on its canonical auctions: a
    /// four-plus-level new suit is a control bid iff the bidder *bypassed*
    /// it (available below their first-shown suit at the same level);
    /// everything else stays to play — suppressed, nothing floored.
    #[test]
    fn high_bid_control_vs_natural() {
        use crate::bidding::american::set_longer_major_response;
        // Pin the historic hearts-first reading (knob off): these
        // minor-response verdicts are the knob-off ones — the longer-major
        // default is covered by `high_bid_under_longer_major_response`, and the
        // 1NT-transfer sub-cases below are knob-independent.
        set_longer_major_response(false);
        // 1♦–1♠–2♦–4♥: responder bid spades first, so hearts cannot be their
        // longest — a control bid agreeing diamonds.  Hearts stays unfloored;
        // diamond support and slam-try values are recorded instead.
        let control = read(&[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ]);
        assert_eq!(control.partner().length(Suit::Hearts).min, 0);
        assert!(control.partner().length(Suit::Diamonds).min >= 3);
        assert!(control.partner().points.min >= 13);

        // 1♦–1♠–2♦–4♠: rebidding one's own suit is natural — six-plus spades.
        let rebid = read(&[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ]);
        assert!(rebid.partner().length(Suit::Spades).min >= 6);

        // 1♦–4♥: the bidder has shown nothing, so hearts can be their
        // longest — to play, no control machinery (and no phantom floor:
        // the honest envelope of an unread jump stays wide).
        let preempt = read(&[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ]);
        assert!(preempt.control_bid().is_none());

        // 1♣–1♥–2♣–4♠: spades sit *above* the first-shown hearts, so they were
        // never denied — this system's response and transfer styles bid the
        // cheaper suit first holding a longer higher one (the first M6.4 A/B
        // bled six IMPs a fired board pulling these to the "agreed" minor).
        // To play, not a control bid.
        let above = read(&[
            bid(1, Strain::Clubs),
            Call::Pass,
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ]);
        assert!(above.control_bid().is_none());

        // 1NT–2♦–2♥–4♠: same shape through a transfer (the overlay attributes
        // the hearts to the bidder) — spades were never denied, so to play.
        let post_transfer = read_booked(&[
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ]);
        assert!(post_transfer.control_bid().is_none());
        assert!(post_transfer.partner().length(Suit::Hearts).min >= 5);

        // 1NT–2♥–2♠–4♥ — the mirror: hearts sit *below* the transferred
        // spades and the cheaper heart transfer was bypassed, so 4♥ cannot be
        // long — a control bid agreeing spades, promising a sixth.
        let mirror = read_booked(&[
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Spades),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ]);
        assert_eq!(mirror.partner().length(Suit::Hearts).min, 0);
        assert!(mirror.partner().length(Suit::Spades).min >= 6);
        set_longer_major_response(true); // restore the shipped default
    }

    /// The longer-major response discipline swaps the M6.4 verdicts on the
    /// two major-response auctions: a 1♥ response denies longer spades (so
    /// the spade jump becomes a control bid), and a 1♠ response may conceal
    /// equal-length five-plus hearts (so the heart jump reads to play).
    #[test]
    fn high_bid_under_longer_major_response() {
        use crate::bidding::american::set_longer_major_response;

        // 1♣–1♥–2♣–4♠, discipline on: 1♥ denied longer spades, so 4♠ is a
        // bypass — a control bid agreeing clubs, spades left unfloored.
        set_longer_major_response(true);
        let control = read(&[
            bid(1, Strain::Clubs),
            Call::Pass,
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ]);
        // The mirror 1♣–1♠–2♣–4♥: a 1♠ response no longer proves short
        // hearts (5-5 responds 1♠), so the heart jump reads to play.
        let to_play = read(&[
            bid(1, Strain::Clubs),
            Call::Pass,
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ]);
        assert_eq!(control.partner().length(Suit::Spades).min, 0);
        assert!(control.partner().length(Suit::Clubs).min >= 3);
        assert!(control.partner().points.min >= 13);
        assert!(to_play.control_bid().is_none());

        // Knob off (the historic hearts-first opt-in): the original verdicts
        // stand — the spade jump above the 1♥ response is to play.
        set_longer_major_response(false);
        let above = read(&[
            bid(1, Strain::Clubs),
            Call::Pass,
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ]);
        set_longer_major_response(true); // restore the shipped default
        assert!(above.control_bid().is_none());
    }

    #[test]
    fn gambling_3nt_over_double_reads_unbalanced() {
        use crate::bidding::instinct::set_gambling_3nt_over_double;
        // [1NT,(X),3NT,P]: opener reads partner's gambling 3NT.  The floor alerts the
        // call as the long-minor gamble, so the natural balanced-3NT reading is
        // suppressed and a six-card minor stays within range — the search sampler must
        // be free to deal responder its running suit, not pin it to a flat hand.
        set_gambling_3nt_over_double(true);
        let read = read_booked(&[
            bid(1, Strain::Notrump),
            Call::Double,
            bid(3, Strain::Notrump),
            Call::Pass,
        ]);
        assert!(read.partner().length(Suit::Clubs).contains(6));
        assert!(read.partner().length(Suit::Diamonds).contains(6));
        set_gambling_3nt_over_double(false);
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
    fn artificial_witness_covers_doubles() {
        // A projection that floors a suit it would not name — the witness a transfer
        // or two-suiter trips (5+ hearts).
        let mut floors_hearts = Inference::unknown();
        floors_hearts.narrow_length(Suit::Hearts, Range::at_least(5, LENGTH_CAP));

        // A *bid* that did not name hearts is artificial (Jacoby 2♦ → 5+♥); a bid
        // naming its own suit is natural (1♥ → 5+♥).
        assert!(artificial(&floors_hearts, bid(2, Strain::Diamonds), None));
        assert!(!artificial(&floors_hearts, bid(1, Strain::Hearts), None));

        // A pass redirects from nothing → never artificial, even flooring a suit.
        assert!(!artificial(
            &floors_hearts,
            Call::Pass,
            Some(Strain::Spades)
        ));

        // A double/redouble "names" the *doubled strain*.  Doubling spades while the
        // projection floors hearts is takeout — it points partner at hearts → artificial;
        // doubling hearts while flooring hearts defends the doubled strain → natural
        // (penalty).  A redouble inherits the same doubled strain.
        assert!(artificial(
            &floors_hearts,
            Call::Double,
            Some(Strain::Spades)
        ));
        assert!(!artificial(
            &floors_hearts,
            Call::Double,
            Some(Strain::Hearts)
        ));
        assert!(artificial(
            &floors_hearts,
            Call::Redouble,
            Some(Strain::Spades)
        ));
        assert!(!artificial(
            &floors_hearts,
            Call::Redouble,
            Some(Strain::Hearts)
        ));

        // A double of notrump defends no suit, so any floored side suit is takeout.
        assert!(artificial(
            &floors_hearts,
            Call::Double,
            Some(Strain::Notrump)
        ));
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
    fn meckwell_overcalls_and_advances_read() {
        use crate::bidding::american::{set_landy, set_meckwell, set_unusual_notrump_defense};
        set_landy(None);
        set_unusual_notrump_defense(None);
        set_meckwell(true);

        // (1NT)–X–(P): the two-way double (single 6+ minor OR both majors) shares no
        // sound per-suit fact, so ONLY the points floor is recorded — no length is
        // narrowed (unlike DONT's X, which pins spades ≤ 3).
        let x = read(&[bid(1, Strain::Notrump), Call::Double, Call::Pass]);
        assert_eq!(x.partner().points, Range::new(8, 37));
        assert_eq!(x.partner().length(Suit::Spades), Range::FULL_LENGTH);
        assert_eq!(x.partner().length(Suit::Hearts), Range::FULL_LENGTH);

        // (1NT)–X–(P)–2♣–(P): the advancer's 2♣ is a "name your suit" relay, not own
        // clubs, so its natural ≥ 4 reading is suppressed.
        let relay = read(&[
            bid(1, Strain::Notrump),
            Call::Double,
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ]);
        assert_eq!(relay.partner().length(Suit::Clubs), Range::FULL_LENGTH);

        // (1NT)–2♣–(P): a real ≥ 4 club suit + an unknown major.  The natural ≥ 5
        // reading is suppressed (a 4-club / 5-major hand makes this call), re-pinned ≥ 4.
        let two_c = read(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
        assert_eq!(two_c.partner().length(Suit::Clubs), Range::new(4, 13));
        assert_eq!(two_c.partner().points, Range::new(8, 37));

        // (1NT)–2♦–(P): diamonds + a major, real ≥ 4.
        let two_d = read(&[
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            Call::Pass,
        ]);
        assert_eq!(two_d.partner().length(Suit::Diamonds), Range::new(4, 13));

        // (1NT)–2♥–(P): NATURAL hearts (Meckwell's 2♥ is a single-suiter, not DONT's
        // both-majors), so spades are not floored — the DONT-vs-Meckwell fork.
        let two_h = read(&[bid(1, Strain::Notrump), bid(2, Strain::Hearts), Call::Pass]);
        assert_eq!(
            two_h.partner().length(Suit::Spades).min,
            0,
            "natural 2♥ shows no spades",
        );

        // Off: the 2♣ reads as a natural club one-suiter again (≥ 5) — no leak.
        set_meckwell(false);
        let off = read(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
        assert_eq!(off.partner().length(Suit::Clubs), Range::new(5, 13));

        set_unusual_notrump_defense(Some((8, 13)));
    }

    #[test]
    fn narrowed_points_intersects_one_player() {
        // 1NT shows 14-18; narrow the opener (here our RHO) to the upper half.
        let inf = read(&[bid(1, Strain::Notrump)]);
        assert_eq!(inf.rho().points, Range::new(14, 18));

        let upper = inf.narrowed_points(Relative::Rho, Range::new(17, 18));
        assert_eq!(
            upper.rho().points,
            Range::new(17, 18),
            "narrowed to the half"
        );
        assert_eq!(inf.rho().points, Range::new(14, 18), "original unchanged");
        // Shape and the other players are untouched.
        assert_eq!(
            upper.rho().length(Suit::Spades),
            inf.rho().length(Suit::Spades)
        );
        assert_eq!(upper.partner().points, inf.partner().points);

        // Intersection, not replacement: a wider request cannot widen what was shown.
        let clamped = inf.narrowed_points(Relative::Rho, Range::new(0, POINTS_CAP));
        assert_eq!(clamped.rho().points, Range::new(14, 18));
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
    fn competitive_opener_rebid_shows_sixth_card() {
        // [1♦, 1♥, P, 2♥, 3♦, P]: partner opened 1♦ and, over the opponents'
        // heart auction, rebid 3♦ (the opt-in `set_competitive_rebid` floor).
        // The natural length reading applies in competition too — only the
        // *strength* reading is suppressed when opponents act — so partner is
        // still read with six-plus diamonds, keeping the sampler and any further
        // interference sound.  Knob-independent: `read` interprets the auction.
        let auction = [
            bid(1, Strain::Diamonds),
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Hearts),
            bid(3, Strain::Diamonds),
            Call::Pass,
        ];
        let inf = read(&auction);
        assert_eq!(inf.partner().length(Suit::Diamonds), Range::new(6, 13));
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
    fn systems_on_overcall_transfer_is_not_read_as_diamonds() {
        // [1♦, 1NT, P, 2♦, P]: their 1♦, our 1NT overcall, the advancer's 2♦ is a
        // Jacoby transfer (grafted opening-1NT structure), not natural diamonds.
        // Stripping their opening reads it as [1NT, P, 2♦, P], so the floor never
        // raises a phantom diamond suit into a doubled disaster (the iron rule).
        let auction = [
            bid(1, Strain::Diamonds),
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
        ];
        let inf = read(&auction);
        assert_eq!(inf.partner().length(Suit::Diamonds), Range::FULL_LENGTH);
    }

    #[test]
    fn gladiator_cue_is_not_read_as_their_major() {
        // [1♠, 1NT, P, 2♠, P]: our 1NT overcall of their 1♠; the advancer's 2♠ is
        // Gladiator Stayman for hearts (exactly 4, INV+) — NOT a natural spade
        // suit.  The major-strip is suppressed for Gladiator, so `gladiator_reading`
        // reads the cue.
        crate::bidding::american::set_nt_overcall_gladiator(true);
        let auction = [
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Spades),
            Call::Pass,
        ];
        let inf = read(&auction);
        crate::bidding::american::set_nt_overcall_gladiator(false);
        // Their major is never floored into the advancer's hand (the iron rule)...
        assert_eq!(inf.partner().length(Suit::Spades), Range::FULL_LENGTH);
        // ...and the cue pins the four-card heart holding it promised.
        assert_eq!(inf.partner().length(Suit::Hearts), Range::new(4, 13));
    }

    #[test]
    fn gladiator_relay_is_not_read_as_clubs() {
        // [1♠, 1NT, P, 2♣, P]: the advancer's 2♣ is the Gladiator relay (weak /
        // invitational, any suit), not a natural club suit.
        crate::bidding::american::set_nt_overcall_gladiator(true);
        let auction = [
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ];
        let inf = read(&auction);
        crate::bidding::american::set_nt_overcall_gladiator(false);
        assert_eq!(inf.partner().length(Suit::Clubs), Range::FULL_LENGTH);
    }

    #[test]
    fn gladiator_delayed_cue_is_read_as_exactly_three_not_spades() {
        // [1♠,1NT,P,2♣,P,2♦,P,2♠,P]: the advancer's SECOND 2♠ (after the 2♣ relay
        // and forced 2♦) is the Gladiator delayed cue — exactly 3 hearts, INV+ —
        // NOT a natural spade suit.  The suppression must cover it too, else the
        // floor raises a phantom spade suit into a doubled disaster (the iron rule).
        crate::bidding::american::set_nt_overcall_gladiator(true);
        let auction = [
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Spades),
            Call::Pass,
        ];
        let inf = read(&auction);
        crate::bidding::american::set_nt_overcall_gladiator(false);
        // Their major is never floored into the advancer's hand...
        assert_eq!(inf.partner().length(Suit::Spades), Range::FULL_LENGTH);
        // ...and the delayed cue pins exactly 3 hearts.
        assert_eq!(inf.partner().length(Suit::Hearts), Range::new(3, 3));
    }

    #[test]
    fn gladiator_stolen_relay_double_is_read_as_the_relay() {
        // [1♠, 1NT, (2♣), X, P]: over RHO's systems-on 2♣, the advancer's Double is
        // the stolen Gladiator relay (weak-or-invitational, any suit) — NOT a
        // penalty double naming clubs.  The reader mirrors the book rebase.
        crate::bidding::american::set_nt_overcall_gladiator(true);
        let auction = [
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Double,
            Call::Pass,
        ];
        let inf = read(&auction);
        crate::bidding::american::set_nt_overcall_gladiator(false);
        // No phantom club suit raised from the doubled strain...
        assert_eq!(inf.partner().length(Suit::Clubs), Range::FULL_LENGTH);
        // ...and the relay's sub-game point cap is recorded.
        assert_eq!(inf.partner().points, Range::new(0, 9));
    }

    #[test]
    fn gladiator_contested_transfer_lebensohl_pins_the_target() {
        // [1♠, 1NT, (2♥), 3♦, P]: over RHO's 2♥ there is no room for the relay
        // tree, so advancer plays Transfer Lebensohl; 3♦ transfers up through their
        // hearts (showing spades), read via the builders' alerts — opener must not
        // raise a phantom diamond suit.
        crate::bidding::american::set_nt_overcall_gladiator(true);
        let auction = [
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            bid(2, Strain::Hearts),
            bid(3, Strain::Diamonds),
            Call::Pass,
        ];
        let inf = read_booked(&auction);
        crate::bidding::american::set_nt_overcall_gladiator(false);
        assert!(
            inf.partner().length(Suit::Spades).min >= 5,
            "transfer target pinned"
        );
        assert!(
            inf.partner().length(Suit::Diamonds).min < 5,
            "phantom suit not read"
        );
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
    fn contested_transfer_lebensohl_reads_the_target_under_intervention() {
        // Board 881510: [1NT, (2♠), 3♦, (3♠)] — responder's 3♦ is a Transfer-
        // Lebensohl transfer to hearts (up the line through their spade suit).  RHO's
        // (3♠) skips opener's completion node; the default-on fallback projection
        // re-resolves 3♦'s authoring rule and pins hearts, so opener does not read it
        // as natural diamonds and raise the phantom suit to 5♦x.  Needs the prefixed
        // `read_booked` (the projection reads the rule off the book).
        let auction = [
            bid(1, Strain::Notrump),
            bid(2, Strain::Spades),
            bid(3, Strain::Diamonds),
            bid(3, Strain::Spades),
        ];
        let inf = read_booked(&auction);
        assert!(
            inf.partner().length(Suit::Hearts).min >= 5,
            "transfer target pinned"
        );
        assert!(
            inf.partner().length(Suit::Diamonds).min < 5,
            "phantom suit not read"
        );
    }

    #[test]
    fn fallback_projection_decodes_contested_leaping_michaels() {
        // [1NT, (2♦), 4♦, (P)]: Leaping Michaels = both majors 5-5, authored as a
        // *guarded fallback* in the (2♦) Transfer block — invisible to the exact-node
        // projection, and with no hand reader.  The default-on fallback projection
        // re-resolves its authoring rule and pins both majors (no reader involved).
        let auction = [
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            bid(4, Strain::Diamonds),
            Call::Pass,
        ];
        let inf = read_booked(&auction);
        assert!(
            inf.partner().length(Suit::Hearts).min >= 5
                && inf.partner().length(Suit::Spades).min >= 5,
            "fallback projection pins both majors for contested Leaping Michaels"
        );
    }

    #[test]
    fn contested_transfer_lebensohl_direct_jacoby_over_2d() {
        // Over (2♦) the transfers are direct Jacoby: 3♦→♥.  [1NT, (2♦), 3♦, (X)].
        let auction = [
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            bid(3, Strain::Diamonds),
            Call::Double,
        ];
        let inf = read_booked(&auction);
        assert!(inf.partner().length(Suit::Hearts).min >= 5);
    }

    #[test]
    fn contested_transfer_lebensohl_cue_is_not_a_transfer() {
        // The cue of their suit is Stayman (a 4-card unbid major), not a 5+ transfer:
        // [1NT, (2♠), 3♠, (P)] projects hearts as only 4-card interest, and the
        // natural-spades reading of the cue is suppressed (not a long spade suit).
        let auction = [
            bid(1, Strain::Notrump),
            bid(2, Strain::Spades),
            bid(3, Strain::Spades),
            Call::Pass,
        ];
        let inf = read_booked(&auction);
        assert!(inf.partner().length(Suit::Hearts).min < 5);
        assert!(inf.partner().length(Suit::Spades).min < 5);
    }

    #[test]
    fn relative_seat_tracks_the_actor() {
        // The same 1♥ opening lands on a different relative seat as the
        // auction grows by one call.
        assert_eq!(
            read(&[bid(1, Strain::Hearts)]).rho().points,
            Range::new(10, 21)
        );
        assert_eq!(
            read(&[bid(1, Strain::Hearts), Call::Pass]).partner().points,
            Range::new(10, 21)
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
        // minimum — it must not be read as the 18–19 jump.  Opener stays at the
        // opening floor (10–21 with Rule of 20 on).
        let inf = read(&[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Notrump),
            Call::Pass,
        ]);
        assert_eq!(inf.partner().points, Range::new(10, 21));
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
        // the quiet 12–16 rebid — leave the strength at the opening floor (10–21
        // with Rule of 20 on).
        let inf = read(&[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Hearts),
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            Call::Pass,
        ]);
        assert_eq!(inf.partner().points, Range::new(10, 21));
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
    fn rubens_reading_respects_the_knob() {
        // With Rubens advances off (`set_rubens_advances`), the same 2♣ is a
        // genuine club suit — the suppression lifts and it reads naturally.
        crate::bidding::instinct::set_rubens_advances(false);
        let inf = read(&[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ]);
        assert!(inf.partner().length(Suit::Clubs).min >= 4);
        crate::bidding::instinct::set_rubens_advances(true);
    }

    #[test]
    fn rubens_limit_raise_transfer_records_support() {
        // (1♣) 1♠ (P) 2♥ (P): partner's transfer into our spades is the
        // limit-plus raise — the overcaller reads three-plus spades and
        // ten-plus points, while the named hearts stay unread (a relay).
        let inf = read(&[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ]);
        assert!(inf.partner().length(Suit::Spades).min >= 3);
        assert!(inf.partner().points.min >= 10);
        assert_eq!(inf.partner().length(Suit::Hearts), Range::FULL_LENGTH);
    }

    #[test]
    fn rubens_new_suit_transfer_records_the_target() {
        // (1♣) 1♠ (P) 2♣ (P): the new-suit transfer shows the advancer's own
        // five-card diamond suit and ten-plus points; clubs stay unread.
        let inf = read(&[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ]);
        assert!(inf.partner().length(Suit::Diamonds).min >= 5);
        assert!(inf.partner().points.min >= 10);
        assert_eq!(inf.partner().length(Suit::Clubs), Range::FULL_LENGTH);
    }

    #[test]
    fn rubens_transfer_records_despite_intervention() {
        // (1♣) 1♠ (P) 2♥ (X): opener doubles the transfer — the completion
        // never comes, but the shown limit raise is exactly what the
        // overcaller needs for the competitive decision.
        let inf = read(&[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Double,
        ]);
        assert!(inf.partner().length(Suit::Spades).min >= 3);
        assert!(inf.partner().points.min >= 10);
    }

    #[test]
    fn rubens_transfer_is_not_read_for_the_opponents() {
        // Same auction read from the opening side (the advancer is now our
        // LHO): the opponents' agreement is not assumed — an in-band advance
        // from the other side may be a genuine suit, so nothing is recorded.
        let inf = read(&[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            Call::Pass,
        ]);
        assert_eq!(inf.lho().length(Suit::Spades), Range::FULL_LENGTH);
        assert_eq!(inf.lho().points, Range::FULL_POINTS);
    }

    #[test]
    fn michaels_cue_over_our_major_reads_the_other_major() {
        use crate::bidding::american::set_uvu_over_majors;

        // [1♥, (2♥)]: their direct cue of our opened major is Michaels — 5+
        // spades, and NOT a natural heart suit (the walk's misread suppressed).
        set_uvu_over_majors(true);
        let inf = read(&[bid(1, Strain::Hearts), bid(2, Strain::Hearts)]);
        assert!(inf.rho().length(Suit::Spades).min >= 5, "the shown major");
        assert_eq!(
            inf.rho().length(Suit::Hearts),
            Range::FULL_LENGTH,
            "the cue is not natural hearts"
        );

        // Knob off: the pre-package natural reading is preserved verbatim.
        set_uvu_over_majors(false);
        let inf = read(&[bid(1, Strain::Hearts), bid(2, Strain::Hearts)]);
        assert!(inf.rho().length(Suit::Hearts).min >= 5);
        assert_eq!(inf.rho().length(Suit::Spades), Range::FULL_LENGTH);
        set_uvu_over_majors(true);
    }

    #[test]
    fn unusual_2nt_over_our_major_reads_both_minors() {
        use crate::bidding::american::set_uvu_over_majors;

        set_uvu_over_majors(true);
        let inf = read(&[bid(1, Strain::Spades), bid(2, Strain::Notrump)]);
        assert!(inf.rho().length(Suit::Clubs).min >= 5);
        assert!(inf.rho().length(Suit::Diamonds).min >= 5);
        set_uvu_over_majors(false);

        // Knob off: nothing recorded for their 2NT.
        let inf = read(&[bid(1, Strain::Spades), bid(2, Strain::Notrump)]);
        assert_eq!(inf.rho().length(Suit::Clubs), Range::FULL_LENGTH);
        assert_eq!(inf.rho().length(Suit::Diamonds), Range::FULL_LENGTH);
        set_uvu_over_majors(true);
    }

    #[test]
    fn uvu_major_cue_projects_the_raise() {
        use crate::bidding::american::set_uvu_over_majors;

        // [1♥, (2NT), 3♣, (P)] from opener's seat: partner's cheap cue is the
        // alerted limit-plus raise — decoded off its authored rule's
        // projection (3+ hearts, 10+), not as natural clubs.
        set_uvu_over_majors(true);
        let inf = read_booked(&[
            bid(1, Strain::Hearts),
            bid(2, Strain::Notrump),
            bid(3, Strain::Clubs),
            Call::Pass,
        ]);
        let cue_bidder = inf.partner();
        assert!(
            cue_bidder.length(Suit::Hearts).min >= 3,
            "the projected fit"
        );
        assert!(cue_bidder.points.min >= 10, "the projected strength");
        assert_eq!(
            cue_bidder.length(Suit::Clubs),
            Range::FULL_LENGTH,
            "not natural clubs"
        );
    }

    #[test]
    fn rubens_transfer_reading_knob_recovers_suppress_only() {
        // Stage-2 knob off: the transfer is still suppressed (not natural
        // hearts) but records nothing — the pre-fix shape.
        set_rubens_transfer_reading(false);
        let inf = read(&[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ]);
        assert_eq!(inf.partner().length(Suit::Spades), Range::FULL_LENGTH);
        assert_eq!(inf.partner().length(Suit::Hearts), Range::FULL_LENGTH);
        assert_eq!(inf.partner().points, Range::FULL_POINTS);
        set_rubens_transfer_reading(true);
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

    /// Retirement invariant for [`artificial`]: every call the structural
    /// detector would read as artificial is *also* alerted by its authoring rule.
    ///
    /// `artificial(project(rule), call) ⟹ rule.alert().is_some()`, walked over
    /// every authored rule in the shipped `american()` book (all three phase
    /// tries).  This now holds with zero counterexamples, so `|| artificial(p,
    /// made)` has been dropped from the decode gate: alerts alone carry the "decode
    /// this call" signal (alert-by-disclosed-meaning, the move modern bridge made
    /// retiring "X is self-alerting").
    ///
    /// Kept as a **permanent regression guard**: a future artificial bid added
    /// without an `.alert(...)` makes this fail (the panic lists the exact call),
    /// rather than silently losing its decoding now that the structural fallback is
    /// gone.
    #[test]
    fn artificial_calls_are_alerted() {
        use crate::bidding::american::american;

        let pair = american();
        let tries = [
            ("constructive", &pair.constructive.0),
            ("competitive", &pair.competitive.0),
            ("defensive", &pair.defensive.0),
        ];

        let mut worklist: Vec<String> = Vec::new();
        for (phase, trie) in tries {
            for (auction, classifier) in trie {
                let auction: &[Call] = &auction;
                let Some(rules) = classifier.as_rules() else {
                    continue;
                };
                let context = Context::new(RelativeVulnerability::NONE, auction)
                    .with_prefixes(trie.common_prefixes(auction));
                for rule in rules.rules() {
                    let made = rule.call();
                    let doubled = context.last_bid().map(|last| last.strain);
                    if super::artificial(&rule.project(&context), made, doubled)
                        && rule.alert().is_none()
                    {
                        worklist.push(format!(
                            "{phase}: [{}] {made}  (label: {:?})",
                            auction
                                .iter()
                                .map(ToString::to_string)
                                .collect::<Vec<_>>()
                                .join(" "),
                            rule.label(),
                        ));
                    }
                }
            }
        }

        worklist.sort();
        worklist.dedup();
        assert!(
            worklist.is_empty(),
            "{} artificial calls lack an alert (the retirement worklist):\n{}",
            worklist.len(),
            worklist.join("\n"),
        );
    }

    /// The same alert invariant, but for the opt-in Gladiator book (off by default,
    /// so the walk above never sees it).  A Gladiator artificial call added without
    /// `.alert(...)` fails here.
    #[test]
    fn gladiator_artificial_calls_are_alerted() {
        use crate::bidding::american::{american, set_nt_overcall_gladiator};

        set_nt_overcall_gladiator(true);
        let pair = american();
        set_nt_overcall_gladiator(false);

        let trie = &pair.defensive.0;
        let mut worklist: Vec<String> = Vec::new();
        for (auction, classifier) in trie {
            let auction: &[Call] = &auction;
            let Some(rules) = classifier.as_rules() else {
                continue;
            };
            let context = Context::new(RelativeVulnerability::NONE, auction)
                .with_prefixes(trie.common_prefixes(auction));
            for rule in rules.rules() {
                let made = rule.call();
                let doubled = context.last_bid().map(|last| last.strain);
                if super::artificial(&rule.project(&context), made, doubled)
                    && rule.alert().is_none()
                {
                    worklist.push(format!(
                        "[{}] {made}  (label: {:?})",
                        auction
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(" "),
                        rule.label(),
                    ));
                }
            }
        }

        worklist.sort();
        worklist.dedup();
        assert!(
            worklist.is_empty(),
            "{} Gladiator artificial calls lack an alert:\n{}",
            worklist.len(),
            worklist.join("\n"),
        );
    }

    /// The same alert invariant for the opt-in New Minor Forcing book (off by
    /// default, so the shipped-system walk never sees it).  Guards the one
    /// artificial call NMF adds — responder's `2`-of-the-new-minor checkback —
    /// against losing its `.alert(...)` and reading as a phantom minor suit.
    #[test]
    fn new_minor_forcing_artificial_calls_are_alerted() {
        use crate::bidding::american::{american, set_new_minor_forcing};

        set_new_minor_forcing(true);
        let pair = american();
        set_new_minor_forcing(false);

        let trie = &pair.constructive.0;
        let mut worklist: Vec<String> = Vec::new();
        for (auction, classifier) in trie {
            let auction: &[Call] = &auction;
            let Some(rules) = classifier.as_rules() else {
                continue;
            };
            let context = Context::new(RelativeVulnerability::NONE, auction)
                .with_prefixes(trie.common_prefixes(auction));
            for rule in rules.rules() {
                let made = rule.call();
                let doubled = context.last_bid().map(|last| last.strain);
                if super::artificial(&rule.project(&context), made, doubled)
                    && rule.alert().is_none()
                {
                    worklist.push(format!(
                        "[{}] {made}  (label: {:?})",
                        auction
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(" "),
                        rule.label(),
                    ));
                }
            }
        }

        worklist.sort();
        worklist.dedup();
        assert!(
            worklist.is_empty(),
            "{} New Minor Forcing artificial calls lack an alert:\n{}",
            worklist.len(),
            worklist.join("\n"),
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
