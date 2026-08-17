//! [`Inferences`] and the natural walk that accumulates it
//!
//! The entry point is [`Inferences::read`]: it walks an auction call by call,
//! narrowing each seat's [`Envelope`] under standard 2/1 meanings, then folds in
//! the authored-rule overlay from [`super::projection`] and the hand-written
//! convention readings from [`super::readers`].

use super::envelope::{Envelope, EnvelopeUnion, Range, Relative, Strength, relative_of};
use super::knobs::*;
use super::projection::*;
use super::readers::*;
use super::{LENGTH_CAP, POINTS_CAP};
use crate::bidding::context::Context;
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};

/// A systems-on advance of our 1NT overcall, with their opening stripped
///
/// When `ReadingProfile::nt_overcall_systems_on` is enabled the advancer plays the full
/// opening-1NT structure grafted below `(their 1-of-a-suit) 1NT`, so the
/// artificial Stayman/transfer calls need the *opening-1NT* reading, not the
/// natural walk.  This returns the auction with their opening removed, which
/// reads exactly like an opening 1NT: `(len - index) % 4` is invariant under
/// removing one earlier call, so every later call keeps its relative seat (only
/// their opening — their own natural suit — is lost, which the opponents' system
/// discloses anyway).  [`None`] (the fast path) unless the graft is on and the
/// shape is their 1-suit opening immediately overcalled `1NT`.
pub(super) fn systems_on_overcall_strip(
    auction: &[Call],
    profile: ReadingProfile,
) -> Option<Vec<Call>> {
    if !profile.nt_overcall_systems_on {
        return None;
    }
    let open = auction.iter().position(|&c| c != Call::Pass)?;
    let Call::Bid(opening) = auction[open] else {
        return None;
    };
    if opening.level.get() != 1 || !opening.strain.is_suit() {
        return None;
    }
    if auction.get(open + 1) != Some(&Call::Bid(Bid::new(1, Strain::Notrump))) {
        return None;
    }
    // Only *our* overcall plays the graft.  When they overcall 1NT over our
    // opening the same shape appears, and stripping our opening would read the
    // rest as an opening-1NT auction *by them*: partner's negative double
    // became "our penalty double of their 1NT, 15+", partner's free bid an
    // overcall of a 1NT opening, and our opener's suit vanished.  Phase 2's
    // whole-book loss was this lane (`1x (1NT)`, every seed, every vul); see
    // docs/authored-reading-handoff.md.
    if !profile.strip_side_blind && !(auction.len() - (open + 1)).is_multiple_of(2) {
        return None;
    }
    // Over a MAJOR, Gladiator replaces the opening-1NT graft with a differently
    // shaped structure (cue = Stayman, 2♣ = relay), so the strip identity fails
    // — but only where Gladiator actually *has* that structure.  RHO's call over
    // our 1NT decides:
    //
    // | RHO      | Gladiator plays          | systems-on plays  | strip? |
    // | -------- | ------------------------ | ----------------- | ------ |
    // | pass     | the Gladiator advances   | the 1NT responses | no     |
    // | `2♣`     | the stolen relay (rebase)| systems on        | no     |
    // | `2♦/M`   | Transfer Lebensohl       | its own sohl      | no     |
    // | **X**    | a natural runout         | a natural runout  | yes    |
    // | **3+**   | the floor                | the floor         | yes    |
    //
    // The last two rows are the same auction in both systems, and the floor that
    // answers them is inference-aware — so denying it the stripped picture it
    // was distilled on changes *calls*, not just readings.  That was ~40% of the
    // treatment's measured loss (`vs-X-*` and `contested-other`); see
    // `docs/reading-drift-handoff.md`.
    if profile.nt_overcall_gladiator && matches!(opening.strain, Strain::Hearts | Strain::Spades) {
        let gladiator_owns_it = match auction.get(open + 2) {
            None | Some(&Call::Pass) => true,
            Some(&Call::Bid(rho)) => rho.level.get() == 2,
            Some(&Call::Double | &Call::Redouble) => false,
        };
        if gladiator_owns_it {
            return None;
        }
    }
    let mut stripped = auction.to_vec();
    stripped.remove(open);
    Some(stripped)
}

/// All four players' shown shape and strength, relative to the side to act
///
/// `Vec`-backed [`EnvelopeUnion`] means this is `Clone`, not `Copy` (two convertible call
/// sites: `narrowed_points`, `single_dummy`).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Inferences {
    /// Per-seat bounding-box hull of `unions` — the single-[`Envelope`] reading the
    /// American engine consumes via [`get`][Self::get].  A redundant cache of
    /// `unions[i].hull()` (`ponytail: keeps get()->&Envelope and all readers
    /// unchanged; collapse to get-by-value if the two ever drift`).
    players: [Envelope; 4],
    /// Per-seat union-of-boxes reading; the sampler tests any-box under
    /// [`envelope_union`][field@crate::bidding::ReadingProfile::envelope_union].
    /// Off, projection produces a single box equal to `players[i]`; a reader
    /// may still install an inherently disjunctive foreign disclosure such as
    /// their Multi.
    unions: [EnvelopeUnion; 4],
    /// Per-seat hull of `announced_unions` — the *agreement* twin of `players`, and
    /// what [`features`][crate::bidding::features] hands the nets.  Equal to `players`
    /// unless [`announced`][field@crate::bidding::ReadingProfile::announced]
    /// is on and some rule split the two with
    /// [`announced`][crate::bidding::constraint::announced].
    announced_players: [Envelope; 4],
    /// Per-seat agreement boxes; the twin of `unions` (see `announced_players`).
    announced_unions: [EnvelopeUnion; 4],
    /// The last call the M6.4 classifier read as a control bid: its auction
    /// index and the suit it agrees.  The exact witness for the instinct
    /// signoff — "the named suit is unread" cannot tell a control bid from an
    /// unread to-play bid.  Not part of the shown-range payload
    /// (serialization skips it).
    #[cfg_attr(feature = "serde", serde(skip))]
    control_bid: Option<(u8, Suit)>,
    /// The reading settings this reading was produced under — the gauges and
    /// membership rule [`admits`][Self::admits] tests on.  Carried on the value
    /// so the sampler's acceptance test runs on the partnership's pinned settings
    /// without every sampler entry point growing a profile argument.  Skipped
    /// by serde and refilled with the shipped default on deserialize, matching
    /// how a reading loaded from a corpus is gauged without an attached partnership.
    #[cfg_attr(feature = "serde", serde(skip, default = "ReadingProfile::default"))]
    profile: ReadingProfile,
}

/// The sound legacy-`points` image of a fit-known support-scale band
///
/// The two scales share raw HCP and diverge by side-suit shortness credit
/// plus the double-fit term against the capped shape [`upgrade`]
/// [`point_count`] adds.  With three-plus trumps (every fit-known site) the
/// skew is bounded both ways: the support count exceeds [`point_count`] by at
/// most **5** (two side voids credit 6 and the double fit 1, while that shape
/// necessarily earns the full upgrade of 2 with nothing wasted), and trails
/// it by at most **1** (an unbalanced hand whose only short suits are working
/// doubletons upgrades 1 with no shortness credit).  So a support promise
/// `[F, C]` pins the legacy scale only to `[F − 5, C + 1]` — publishing the
/// band verbatim excluded the shapely light raises that measurably make it
/// (the `1♠ - 2♠` divergence-meter defect: observed point counts 4–10
/// against a published 6..=10), and [`Envelope::admits`] gauges the legacy
/// axis unconditionally, so the sampler refused to deal partner those hands.
/// Pinned by `support_band_points_image_is_sound`.
///
/// [`point_count`]: crate::bidding::constraint::point_count
/// [`upgrade`]: crate::bidding::constraint::upgrade
pub(super) fn support_band_to_points(band: Range) -> Range {
    Range::new(
        band.min.saturating_sub(5),
        band.max.saturating_add(1).min(POINTS_CAP),
    )
}

impl Inferences {
    /// The shown shape and strength of one relative seat (the hull)
    #[must_use]
    pub const fn get(&self, who: Relative) -> &Envelope {
        &self.players[who as usize]
    }

    /// What one relative seat's calls **announce** — the partnership agreement
    ///
    /// The disclosure twin of [`get`][Self::get].  Equal to it unless
    /// [`announced`][field@crate::bidding::ReadingProfile::announced] is on
    /// and a rule split the two with
    /// [`announced`][crate::bidding::constraint::announced], which is how a call decided
    /// by the evaluator net can still say what it means: the sound projection
    /// stays ⊤ for the sampler while this carries the agreement.
    ///
    /// Consume this for disclosure and for anything that *reasons* about the
    /// auction (the nets' feature vectors); consume [`get`][Self::get] wherever
    /// a hand must be tested for consistency, because only that one is bound to
    /// contain the truth.
    #[must_use]
    pub const fn announced(&self, who: Relative) -> &Envelope {
        &self.announced_players[who as usize]
    }

    /// One relative seat's announced **union of boxes** — the unhulled twin of
    /// [`announced`][Self::announced]
    ///
    /// The hull is what the nets have always read, and hulling is where the
    /// information goes: `♥5..13` and `♥5..8` are the same claim, yet their
    /// endpoints differ by a third of the column's range.  Anything that wants
    /// the *distribution* a reading describes rather than its bounding box —
    /// [`features_eval_shape`][crate::bidding::features::features_eval_shape] — reads
    /// the boxes here and tests membership atom by atom.
    #[must_use]
    pub const fn announced_union(&self, who: Relative) -> &EnvelopeUnion {
        &self.announced_unions[who as usize]
    }

    /// Assemble a reading from the natural walk's hull and the two overlays
    ///
    /// The agreement side reuses `players`, so a box it *widens* cannot show
    /// through — an announce looser than its projection is silently clipped
    /// back.  Sound for the pilot, whose agreement is strictly tighter than ⊤.
    // ponytail: re-running the walk per overlay is the fix if a looser
    // agreement ever needs to show through; nothing wants one yet.
    fn assemble(
        players: [Envelope; 4],
        overlay: &[EnvelopeUnion; 4],
        agreement: &[EnvelopeUnion; 4],
        control_bid: Option<(u8, Suit)>,
        profile: ReadingProfile,
    ) -> Self {
        let announced_unions = intersect_overlay(&players, agreement, profile);
        let mut this = Self {
            unions: intersect_overlay(&players, overlay, profile),
            announced_players: std::array::from_fn(|i| announced_unions[i].hull()),
            announced_unions,
            players,
            control_bid,
            profile,
        };
        if profile.blind_opponents {
            for who in [Relative::Lho, Relative::Rho] {
                let i = who as usize;
                this.players[i] = Envelope::unknown();
                this.announced_players[i] = Envelope::unknown();
                this.unions[i] = EnvelopeUnion::unknown();
                this.announced_unions[i] = EnvelopeUnion::unknown();
            }
        }
        this
    }

    /// Whether `hand` is consistent with one seat's reading
    ///
    /// Under [`envelope_union`][field@crate::bidding::inference::ReadingProfile::envelope_union],
    /// or when a reader explicitly preserved more than one box, a hand must
    /// lie in *some* box of that seat's union. Otherwise it need only lie in
    /// the bounding-box hull. The sampler's per-seat test.
    #[must_use]
    pub fn admits(&self, who: Relative, hand: Hand) -> bool {
        let union = &self.unions[who as usize];
        if self.profile.envelope_union() || union.boxes().len() > 1 {
            union.contains_on(hand, self.profile)
        } else {
            self.players[who as usize].admits_on(hand, self.profile)
        }
    }

    /// What the player to act has shown by their own prior calls
    #[must_use]
    pub const fn me(&self) -> &Envelope {
        self.get(Relative::Me)
    }

    /// What partner has shown
    #[must_use]
    pub const fn partner(&self) -> &Envelope {
        self.get(Relative::Partner)
    }

    /// What the left-hand opponent has shown
    #[must_use]
    pub const fn lho(&self) -> &Envelope {
        self.get(Relative::Lho)
    }

    /// What the right-hand opponent has shown
    #[must_use]
    pub const fn rho(&self) -> &Envelope {
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
        let profile = ReadingProfile::default();
        let mut copy = self.clone();
        let i = who as usize;
        copy.players[i].strength.points = copy.players[i].strength.points.intersect(points);
        // Narrow the points of every box in the union to keep `unions` == the hull's
        // source (a points-only slab drops no box: it never crosses a length axis).
        let slab = Envelope {
            strength: Strength {
                points,
                ..Strength::unknown()
            },
            ..Envelope::unknown()
        };
        copy.unions[i].intersect_assign(&slab.into(), profile);
        // An externally-imposed points slice is a fact about the hand, not a
        // reading of a call, so it narrows the agreement side identically —
        // otherwise the two drift apart on the one axis the caller sliced.
        copy.announced_players[i].strength.points =
            copy.announced_players[i].strength.points.intersect(points);
        copy.announced_unions[i].intersect_assign(&slab.into(), profile);
        copy
    }

    /// Derive, hand-independently, what every player's calls have shown under
    /// standard 2/1 meanings, relative to the side to act
    ///
    /// **A bare `Context::new` reads far less than the bidder does.**
    /// Projection-based reading — the pass bands, the authored-rule overlay —
    /// needs the convention keys that only `Partnership::prefixed_context` attaches,
    /// so on a keyless context every one of them is skipped silently: no error,
    /// and `0..=37` is a perfectly well-formed answer. A pass that the bidder
    /// reads as `0..=11` comes back vacuous here. Diagnostics that want *what
    /// the bidder actually sees* must go through `Partnership::infer`; this entry
    /// point is for the hand-coded walk alone.
    #[must_use]
    pub fn read(context: &Context<'_>) -> Self {
        // A systems-on advance of our 1NT overcall reads as an opening-1NT
        // auction with their opening stripped: the advancer plays the grafted
        // 1NT structure, so the hand-coded notrump walk reads its artificial
        // Stayman/transfer calls instead of the natural walk raising a phantom
        // suit.  Re-key the stripped auction through the attached partnership so the
        // projection overlay survives the strip — a bare `Context::new` has no
        // trie prefixes, so `project_authored` silently skips every authored
        // rule, and the calls only the *book* knows are conventional (the
        // alerted both-majors `3♦`, which the walk's off-book arm reads as
        // natural `♦ 5..`) excluded their own bidders (`readings_admit_the_
        // bidder`).  The stripped opening is 1NT, which the strip never fires
        // on, so this recurses at most once.
        if let Some(stripped) =
            systems_on_overcall_strip(context.auction(), context.reading_profile())
        {
            let mut reading = match context.own_system() {
                Some(partnership) => {
                    // The stripped 1NT is an *overcall*: their next call is by
                    // the side that OPENED, so the `two_clubs_landy` disclosure
                    // (their defense to our 1NT *opening*) must not extrapolate
                    // through the strip — over our overcall their 2♣ is
                    // responder's natural call, never Landy.  The first A/B's
                    // worst boards were exactly this leak.
                    let mut profile = context.decision_profile();
                    profile.their.two_clubs_landy = false;
                    profile.their.two_diamonds_multi = false;
                    Self::read(
                        &partnership
                            .prefixed_context(context.vul(), &stripped)
                            .with_profile(profile),
                    )
                }
                None => Self::read(&Context::new(context.vul(), &stripped)),
            };
            if context.reading_profile().scope != ReadingScope::All {
                return reading;
            }
            // Stripping makes the overcall look like an opening 1NT. Preserve
            // the grafted advance reading, but restore the overcaller's own
            // 15–18 authored box from the unstripped auction.
            let mut profile = context.decision_profile();
            profile.reading.nt_overcall_systems_on = false;
            let unstripped = match context.own_system() {
                Some(partnership) => Self::read(
                    &partnership
                        .prefixed_context(context.vul(), context.auction())
                        .with_profile(profile),
                ),
                None => Self::read(
                    &Context::new(context.vul(), context.auction()).with_profile(profile),
                ),
            };
            let opening = context
                .auction()
                .iter()
                .position(|&call| call != Call::Pass)
                .expect("the strip found an opening");
            let overcaller = relative_of(context.auction().len(), opening + 1) as usize;
            reading.players[overcaller] = unstripped.players[overcaller];
            reading.unions[overcaller] = unstripped.unions[overcaller].clone();
            reading.announced_players[overcaller] = unstripped.announced_players[overcaller];
            reading.announced_unions[overcaller] = unstripped.announced_unions[overcaller].clone();
            return reading;
        }
        let profile = context.reading_profile();
        // The *opponents'* agreement, for the sites that decode a call off what
        // the bidder's own side plays rather than off ours.  `Partnership::opponents`
        // is ours again unless a foreign book was declared
        // ([`Partnership::with_opponents`]), so an undeclared table reads
        // bit-for-bit as before.
        let their_profile = context
            .their_system()
            .map_or(profile, |them| them.profile().reading);
        let auction = context.auction();
        let len = auction.len();
        let mut players = [Envelope::unknown(); 4];
        // The disjunctive overlay, folded into `players` (the hull) below and into
        // `unions` (the boxes) at each return.  Unknown until `project_authored` runs.
        let mut overlay_unions: [EnvelopeUnion; 4] =
            std::array::from_fn(|_| EnvelopeUnion::unknown());
        // The agreement twin of `overlay_unions`; a clone of it unless
        // `ReadingProfile::announced` is on (see [`project_authored`]).
        let mut agreement_unions: [EnvelopeUnion; 4] =
            std::array::from_fn(|_| EnvelopeUnion::unknown());
        let mut control_bid = None;

        let Some(opening_index) = auction.iter().position(|&c| c != Call::Pass) else {
            // Nothing but passes so far — each is still a call with a reading:
            // a no-open pass declines the whole opening table, decoded by the
            // projection pass off the table's own Pass gate (`points(..12)` —
            // see `ReadingProfile::pass`).  The walk below needs an opening, so
            // apply the overlay here and return.
            if profile.pass {
                let projection = project_authored(context);
                for (player, projected) in players.iter_mut().zip(&projection.unions) {
                    *player = player.intersect(&projected.hull());
                }
                overlay_unions = projection.unions;
                agreement_unions = projection.announced_unions;
            }
            return Self::assemble(
                players,
                &overlay_unions,
                &agreement_unions,
                control_bid,
                profile,
            );
        };
        let Call::Bid(opening_bid) = auction[opening_index] else {
            return Self::assemble(
                players,
                &overlay_unions,
                &agreement_unions,
                control_bid,
                profile,
            );
        };
        let opener_lane = opening_index % 4;
        // Whose scheme the opening side's 1NT structure runs on.  `is_opening_side`
        // below is parity relative to the *opener*, not to us, so the notrump
        // sites it gates fire just as happily on the opponents' auction — and
        // must then answer from their agreement, not ours.  This is the whole
        // point of an opponent model like `european.rs`.
        let side_profile = if opener_lane % 2 == len % 2 {
            profile
        } else {
            their_profile
        };
        // SAFETY: at most three passes precede the opening, so the cast is safe.
        #[allow(clippy::cast_possible_truncation)]
        let opener_seat = opening_index as u8 + 1;
        let opening_artificial =
            opening_bid.strain == Strain::Notrump || opening_bid == Bid::new(2, Strain::Clubs);
        let defending_parity = (opener_lane + 1) % 2;
        let read_nt_invite = profile.nt_invite;
        // A 1NT - 2♣ Stayman auction (opponents silent): opener's major answer and
        // responder's strength are read below so the floor judges the fit and
        // accepts or declines invitations.  The artificial 3OM / Smolen jumps are
        // suppressed from the natural suit reading rather than re-derived.
        let stayman = opening_bid == Bid::new(1, Strain::Notrump)
            && auction.get(opening_index + 2) == Some(&Call::Bid(Bid::new(2, Strain::Clubs)));

        // Suits bid and the count of bids made, per auction lane (`index % 4`);
        // lanes of equal parity are partners, the same side.
        let mut lane_suits = [0u8; 4];
        // The subset of `lane_suits` the walk actually read as a natural
        // holding — a cue names a suit without ever showing it.
        let mut natural_lane_suits = [0u8; 4];
        // The subset of `natural_lane_suits` the lane has shown *twice* (a
        // rebid or jump-rebid suit, six long or a good five) — a raise of one
        // is routinely made on a doubleton, even jumping to game.
        let mut rebid_lane_suits = [0u8; 4];
        let mut lane_bids = [0u8; 4];
        let mut lane_doubled = [false; 4];
        let mut side_acted = [false; 2];
        let mut highest: Option<Bid> = None;
        let read_cues = profile.cue;
        let sound_lengths = profile.length_soundness;

        // Every hand-written convention reader, run once over the auction: which
        // calls name a suit their bidder need not hold, and what each really
        // showed (recorded post-walk).  `docs/reader-retirement.md` is the ledger.
        let readings = Readings::read(auction, len, profile, context.decision_profile().their);

        // Authored calls with an informative projection own their reading.  The
        // projection supplies both the post-walk overlay and the per-call masks
        // that let the walk retain only mechanical lane bookkeeping.  A live
        // unalerted authored rule whose projection is top falls back to the
        // walk; an alerted top stays suppressed as artificial.
        let projection = project_authored(context);
        let masks = projection.masks;
        let suppressed = masks.suppressed;
        overlay_unions = projection.unions;
        agreement_unions = projection.announced_unions;
        // The hulled overlay the natural walk consumes (`shown_suit`, the post-walk
        // intersect); the boxes are re-combined into `unions` at the return.
        let overlay: [Envelope; 4] = std::array::from_fn(|i| overlay_unions[i].hull());

        // Which calls the walk has suppressed so far (any reason: projection,
        // convention readers, the notrump-structure blanket, control bids) —
        // the control-bid classifier scans it for the agreed suit (M6.4).
        let mut suppressed_so_far = 0u64;
        // Suits a substituted projection has promised at three-plus.  Four-plus
        // is a naturally shown suit; matching three-plus promises by partners
        // establish a fit even when a short minor opening itself promised only
        // three.
        let mut projected_lane_lengths = [[0u8; 4]; 4];
        // A call whose authoring rule promised nothing reads its shape off the
        // walk as it always did, but its *strength* is the exclusion fold's to
        // state (`CallMasks::walk_shape`): the walk's guess is rolled back at
        // the top of the next iteration, which is where every `continue` in
        // the arms below lands.
        let mut roll_back: Option<(usize, Strength)> = None;

        for (index, &call) in auction.iter().enumerate() {
            if let Some((seat, strength)) = roll_back.take() {
                players[seat].strength = strength;
            }
            let lane = index % 4;
            let who = relative_of(len, index) as usize;
            if index < 64 && masks.walk_shape >> index & 1 != 0 {
                roll_back = Some((who, players[who].strength));
            }
            let is_opening_side = lane % 2 == opener_lane % 2;
            let first_action_of_side = !side_acted[lane % 2];
            let substituted_call = index < 64 && masks.substituted >> index & 1 != 0;
            let artificial_call = index < 64 && masks.artificial >> index & 1 != 0;
            let authored_call = index < 64 && masks.authored >> index & 1 != 0;

            if substituted_call {
                let mut three_plus = 0u8;
                let mut four_plus = 0u8;
                for suit in Suit::ASC {
                    let mask = 1u8 << suit as u8;
                    let floor = masks.floor(index, suit);
                    projected_lane_lengths[lane][suit as usize] =
                        projected_lane_lengths[lane][suit as usize].max(floor);
                    if floor >= 3 {
                        three_plus |= mask;
                    }
                    if floor >= 4 {
                        four_plus |= mask;
                    }
                }
                let partner_lane = (lane + 2) % 4;
                let partner_projected = Suit::ASC.into_iter().fold(0u8, |mask, suit| {
                    mask | (u8::from(projected_lane_lengths[partner_lane][suit as usize] >= 3)
                        << suit as u8)
                });
                let fit = three_plus & (natural_lane_suits[partner_lane] | partner_projected);
                let shown = four_plus | fit;
                rebid_lane_suits[lane] |= four_plus & natural_lane_suits[lane];
                lane_suits[lane] |= shown;
                natural_lane_suits[lane] |= shown;
                // The fit is *this* lane's showing; it is deliberately not
                // written back into partner's lane sets.  Half of such a
                // write-back is a no-op by construction — `natural_lane_suits`
                // is a subset of `lane_suits` at every site, so a `fit` sourced
                // from partner's *natural* set is already in both — and the
                // other half, a `fit` sourced from `partner_projected`, needs a
                // substituted call that floors a suit at three while neither
                // naming it (the face-suit record below) nor projecting four
                // (`four_plus`).  No call in the shipped book does that, and
                // dropping the write-back measured **0 diverging boards in
                // 1,228,800** with `smoke-default` byte-identical
                // (`docs/authored-reading-handoff.md`, 2026-08-17).  So the
                // deletion settles the open question in the loosening direction
                // for any future book that does reach it: a suit partner
                // promised only through a projection never becomes a suit
                // partner *showed*.
                // The suit the call *named*, in the lane's mechanical
                // bid-history only.  Meaning comes from the projection above,
                // but "this lane has bid diamonds" is what happened at the
                // table, and the walk's later rebid/raise/cue arms key on it:
                // a `1♦` opening whose rule-union admits a three-card diamond
                // projects no length floor at all, so without this its own
                // three-level rebid read as a *first* showing (♦4+, not ♦6+)
                // and the raise ladder lost a level.  Artificial calls are
                // excluded — a transfer's face suit is exactly the phantom
                // holding suppression exists to kill.
                if !artificial_call
                    && let Call::Bid(bid) = call
                    && let Some(suit) = bid.strain.suit()
                {
                    lane_suits[lane] |= 1 << suit as u8;
                }
                suppressed_so_far |= 1 << index;
            }

            match call {
                Call::Pass | Call::Redouble => {}
                Call::Double => {
                    // A direct double of a natural suit opening, the defending
                    // side's first action, reads as takeout: opening values.
                    if !substituted_call
                        && !is_opening_side
                        && first_action_of_side
                        && index != opening_index
                        && opening_bid.strain.is_suit()
                    {
                        players[who].narrow_points(Range::at_least(11, POINTS_CAP));
                    }
                    lane_doubled[lane] = true;
                    side_acted[lane % 2] = true;
                }
                Call::Bid(bid) => {
                    if substituted_call {
                        lane_bids[lane] += 1;
                        side_acted[lane % 2] = true;
                        if highest.is_none_or(|h| outranks(bid, h)) {
                            highest = Some(bid);
                        }
                        continue;
                    }
                    // Their direct 1NT overcall of our one-suit opening: natural,
                    // the strong-notrump band, read off their scheme's opening
                    // 1NT.  Until 2026-08-16 `systems_on_overcall_strip` fired
                    // on this shape too and delivered exactly this box by
                    // reading the stripped auction as their 1NT *opening* — while
                    // misreading every call of ours in the lane; the strip is
                    // now ours-only, so the walk keeps their box explicitly.
                    // Our own 1NT overcall is authored (15–18) and stays with
                    // the projection.
                    let their_direct_nt_overcall = index == opening_index + 1
                        && bid == Bid::new(1, Strain::Notrump)
                        && opening_bid.level.get() == 1
                        && opening_bid.strain.is_suit()
                        && matches!(relative_of(len, index), Relative::Lho | Relative::Rho);
                    if index == opening_index {
                        apply_opening(&mut players[who], bid, opener_seat, profile);
                    } else if their_direct_nt_overcall {
                        apply_opening(&mut players[who], bid, 1, their_profile);
                    } else if let Some(suit) = bid.strain.suit() {
                        // A three-level suit bid over our 1NT is off-book and
                        // forcing — the instinct reading takes it as natural,
                        // five-plus (see `opener_forced_past_invitation`).  The
                        // two-level responses are Stayman and transfers.
                        //
                        // Our 1NT *overcall* is the same structure one seat
                        // over, so the advancer's three-level suit is natural
                        // and forcing too — never the weak six-card jump the
                        // `jump >= 1` arm below would read.  Systems-on gets
                        // this free (`systems_on_overcall_strip` deletes their
                        // opening and the auction reads as an opening 1NT);
                        // Gladiator turns the strip off because its advances
                        // differ, so the walk has to recognise the overcall
                        // itself — `gladiator_advances` authors the game-forcing
                        // `3♣`/`3♦`/`3O` as `len(suit, 5..)`, and a 6+ reading
                        // excluded every five-card advancer from its own box.
                        let one_nt = Bid::new(1, Strain::Notrump);
                        let our_one_nt_overcall = !is_opening_side
                            && opening_bid.level.get() == 1
                            && opening_bid.strain.is_suit()
                            && auction.get(opening_index + 1) == Some(&Call::Bid(one_nt))
                            && index > opening_index + 1
                            && (index - opening_index - 1) % 4 == 2;
                        // Only the lane's *first* bid: a three-level call made
                        // after an earlier (artificial) response — a second
                        // suit behind a Jacoby transfer, a super-accept — is
                        // structure, not the direct natural-forcing response,
                        // and reading it five-plus excluded the four-card
                        // second-suiters from their own box.  Gated out here it
                        // falls back under the notrump-structure blanket.
                        let over_one_notrump = bid.level.get() == 3
                            && lane_bids[lane] == 0
                            && ((is_opening_side && opening_bid == one_nt) || our_one_nt_overcall);
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
                        // Responder's 1NT - 3M splinter, when authored, is the
                        // shortest possible major: never a natural five-plus.
                        // Suppressing this *one* index (rather than routing it
                        // through `nt_structure_artificial`, whose `entered` set
                        // marks the whole continuation subtree) leaves opener's
                        // natural `3♠`/`4♣`/`4♦` rebids reading off the walk.
                        let nt_splinter_artificial = profile.nt_splinter
                            && is_opening_side
                            && opening_bid == Bid::new(1, Strain::Notrump)
                            && index == opening_index + 2
                            && bid.level.get() == 3
                            && matches!(bid.strain, Strain::Hearts | Strain::Spades);
                        // The blanket and its structural exceptions are guesses
                        // about calls nothing is known about.  An authored call
                        // reaching here has a live rule and no alert, so
                        // `artificial_calls_are_alerted` makes it natural —
                        // `top_authored_projection_falls_back_to_the_walk`.
                        let nt_blanket = !authored_call
                            && is_opening_side
                            && opening_artificial
                            && !over_one_notrump;
                        let chain = (!authored_call
                            && (stayman_artificial
                                || nt_splinter_artificial
                            // No `is_opening_side` gate, unlike its two neighbours
                            // above — see `nt_structure_artificial`'s own doc for
                            // why adding one is an A/B, not a cleanup.
                                || nt_structure_artificial(
                                    auction,
                                    index,
                                    opening_index,
                                    side_profile,
                                )))
                            || (index < 64 && suppressed >> index & 1 != 0)
                            || readings.suppresses(index);

                        // M6.4: a four-plus-level suit bid in the slam zone is
                        // classified control-bid vs to-play before the natural
                        // walk (see [`classify_high_bid`]).  It may punch
                        // through the notrump-structure blanket (the
                        // post-transfer 4♠ control) — but only when the
                        // projection is present to have claimed the genuinely
                        // artificial calls (Texas transfers) first.
                        let slam = if profile.control_bid
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
                                profile,
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
                        let opener_ladder_rebid = profile.opener_extras_ladder
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
                            // A bid of a suit only the opponents have naturally
                            // shown is a cue, never a holding (`ReadingProfile::cue`).
                            let opponents_natural = natural_lane_suits[(lane + 1) % 4]
                                | natural_lane_suits[(lane + 3) % 4];
                            let opponents_shown_it = read_cues && opponents_natural & mask != 0;
                            // A slam-zone bid made with a *different* suit
                            // already agreed in this lane pair is a control
                            // bid, and a control bid claims no length in the
                            // suit it names.  [`classify_high_bid`] (M6.4) is
                            // the real classifier, but it is gated off the
                            // moment the opponents act
                            // (`!side_acted[defending_parity]`), so in
                            // competition every slam-zone bid fell through to
                            // the rebid or raise arm below and claimed length.
                            // The Phase 3 A′′ worst board is exactly that: with
                            // hearts agreed, opener's floor `4♣` read as a club
                            // *rebid* (♣6+, `read.rs`'s floor of 6), so the
                            // keycard ask keyed on a seven-card club fit over
                            // an eight-card heart fit holding AKQ, and the slam
                            // went two down doubled for −18 IMPs.  Which call
                            // the floor makes instead is the floor's business;
                            // this only stops the reading inventing the suit.
                            let control_bid = (4..=5).contains(&bid.level.get())
                                && lane_suits[lane] & lane_suits[(lane + 2) % 4] & !mask != 0;

                            if control_bid && (i_bid_it || partner_bid_it) {
                                // No length claim, either way: a control bid in
                                // partner's suit is not a raise of it.
                            } else if i_bid_it {
                                // Rebidding our own suit shows a sixth card —
                                // except (`ReadingProfile::length_soundness`) a re-raise of
                                // a suit partner has also bid (agreed, so no new
                                // length) and opener's immediate two-level rebid
                                // of the opened suit, routinely a good five
                                // (a minor, or a major stuck over the forcing
                                // notrump).
                                let agreed_re_raise = sound_lengths && partner_bid_it;
                                let five_card_rebid = sound_lengths
                                    && lane == opener_lane
                                    && lane_bids[lane] == 1
                                    && bid.level.get() == 2
                                    && opening_bid.strain.suit() == Some(suit);
                                // Under XYZ (`ReadingProfile::xyz`) responder's two-level
                                // rebid of the one-level major is authored
                                // five-plus, both routes: the direct 2M weak
                                // sign-off and the invitational 2M through the
                                // 2♣ relay (`xyz_responder`/`xyz_after_relay`).
                                // Reading a sixth card excluded every five-card
                                // responder from their own box.
                                let xyz_rebid = profile.xyz
                                    && !side_acted[defending_parity]
                                    && is_opening_side
                                    && lane != opener_lane
                                    && matches!(suit, Suit::Hearts | Suit::Spades)
                                    && bid.level.get() == 2
                                    && opening_bid.level.get() == 1
                                    && opening_bid.strain.is_suit()
                                    && auction.get(opening_index + 2)
                                        == Some(&Call::Bid(Bid::new(1, Strain::from(suit))))
                                    && matches!(
                                        auction.get(opening_index + 4),
                                        Some(Call::Bid(rebid)) if rebid.level.get() == 1
                                    )
                                    && (index == opening_index + 6
                                        || (index == opening_index + 10
                                            && auction.get(opening_index + 6)
                                                == Some(&Call::Bid(Bid::new(2, Strain::Clubs)))
                                            && auction.get(opening_index + 8)
                                                == Some(&Call::Bid(Bid::new(
                                                    2,
                                                    Strain::Diamonds,
                                                )))));
                                if !agreed_re_raise {
                                    let floor = if five_card_rebid || xyz_rebid { 5 } else { 6 };
                                    players[who]
                                        .narrow_length(suit, Range::at_least(floor, LENGTH_CAP));
                                }
                                if natural_lane_suits[lane] & mask != 0 {
                                    rebid_lane_suits[lane] |= mask;
                                }
                                natural_lane_suits[lane] |= mask;
                            } else if partner_bid_it {
                                // Raising partner's suit shows three-card support
                                // — unless partner has already shown six-plus (a
                                // preempt, a weak jump, a jump-rebid suit): a
                                // raise of a known-long suit to game is routinely
                                // made on a doubleton or stiff honour, so no
                                // length claim is sound there.  A *delayed*
                                // return to a suit partner has shown five-plus
                                // (the opened major back over the forcing
                                // notrump, an XYZ five-card rebid) floors at two
                                // — the false preference on Hx is the norm, not
                                // the exception.  A preference takes the cheapest
                                // route, so a jump return qualifies only when
                                // partner has bid the suit twice; direct raises
                                // and raises of 4-card or unknown-length suits
                                // keep the three-card claim.
                                let partner_length = players[(who + 2) % 4]
                                    .length(suit)
                                    .min
                                    .max(projected_lane_lengths[(lane + 2) % 4][suit as usize]);
                                if partner_length < 6 {
                                    let partner_rebid_it =
                                        rebid_lane_suits[(lane + 2) % 4] & mask != 0;
                                    let delayed = (jump == 0 || partner_rebid_it)
                                        && lane_bids[lane] >= 1
                                        && partner_length >= 5;
                                    let floor = if delayed { 2 } else { 3 };
                                    players[who]
                                        .narrow_length(suit, Range::at_least(floor, LENGTH_CAP));
                                }
                                natural_lane_suits[lane] |= mask;
                            } else if opponents_shown_it {
                                // A cue: no length in the named suit.  Record the
                                // two meanings that hold robustly across natural
                                // systems; anything else stays silent (soundness
                                // over tightness).
                                let partner_natural = natural_lane_suits[(lane + 2) % 4];
                                let michaels = !is_opening_side
                                    && first_action_of_side
                                    && partner_natural == 0
                                    && opponents_natural == mask
                                    && opening_bid.strain.suit() == Some(suit)
                                    && matches!(suit, Suit::Clubs | Suit::Diamonds)
                                    && (jump == 0 || (opening_bid.level.get() == 2 && jump == 1));
                                if michaels {
                                    // A direct cue of their minor opening —
                                    // Michaels (or Leaping Michaels over the weak
                                    // two): both majors, five-five.  Strength
                                    // stays open (mini-max styles run wide).
                                    players[who].narrow_length(
                                        Suit::Hearts,
                                        Range::at_least(5, LENGTH_CAP),
                                    );
                                    players[who].narrow_length(
                                        Suit::Spades,
                                        Range::at_least(5, LENGTH_CAP),
                                    );
                                } else if jump == 0 && partner_natural.count_ones() == 1 {
                                    // A non-jump cue opposite one natural suit:
                                    // the limit-plus cue-raise (mirrors the
                                    // Rubens cue-raise floors).
                                    let agreed =
                                        Suit::ASC[partner_natural.trailing_zeros() as usize];
                                    players[who]
                                        .narrow_length(agreed, Range::at_least(3, LENGTH_CAP));
                                    // Fit agreed (the cue names partner's suit), so
                                    // the raise's point promise is a support-scale
                                    // one; the legacy axis takes only its sound
                                    // image.
                                    let band = Range::at_least(10, POINTS_CAP);
                                    players[who].narrow_points(support_band_to_points(band));
                                    players[who].narrow_support_points(agreed, band);
                                }
                            } else if over_one_notrump {
                                // Natural, forcing five-card suit over our 1NT.
                                players[who].narrow_length(suit, Range::at_least(5, LENGTH_CAP));
                                natural_lane_suits[lane] |= mask;
                            } else if !is_opening_side && first_action_of_side {
                                // The defending side's first suit bid is an
                                // overcall: a five-card suit (six if jumping),
                                // opening values at the cheapest level.
                                let min = if jump >= 1 { 6 } else { 5 };
                                players[who].narrow_length(suit, Range::at_least(min, LENGTH_CAP));
                                natural_lane_suits[lane] |= mask;
                                if jump == 0 {
                                    players[who].narrow_points(Range::at_least(8, POINTS_CAP));
                                }
                            } else if jump >= 1 {
                                // A single jump in a new suit is a weak jump: a
                                // six-card suit.  Skip splinters (double jumps)
                                // — and, under `ReadingProfile::length_soundness`, a player
                                // who has doubled (their jump is strength on as
                                // few as three cards; claim nothing).  Opener's
                                // extras-ladder jump-shift is instead a strong
                                // 5-4, so the jumped suit is only 4+.
                                if jump == 1 && !(sound_lengths && lane_doubled[lane]) {
                                    let floor = if opener_ladder_rebid { 4 } else { 6 };
                                    players[who]
                                        .narrow_length(suit, Range::at_least(floor, LENGTH_CAP));
                                    natural_lane_suits[lane] |= mask;
                                }
                            } else {
                                // A natural new suit at the cheapest level: four-plus.
                                players[who].narrow_length(suit, Range::at_least(4, LENGTH_CAP));
                                natural_lane_suits[lane] |= mask;
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
                                    if side_profile.notrump_minors
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
                            // jump raise invitational.  One-level openings only:
                            // a raise of a preempt is two-way — furthering the
                            // preempt on nothing OR bidding a game to make on
                            // 16+ — so no strength band is sound there (the
                            // `1..=11` image of the constructive band excluded
                            // every to-make raiser of `3♥ - 4♥` from its own
                            // box).
                            let partner_bid_it =
                                lane_suits[(lane + 2) % 4] & (1 << suit as u8) != 0;
                            if responder_first && partner_bid_it && opening_one_suit {
                                let jump = bid
                                    .level
                                    .get()
                                    .saturating_sub(cheapest_level(highest, bid.strain));
                                // Fit agreed (raising opener's suit), so the raise
                                // strength is a support-scale promise — the
                                // support gauge carries it exactly; the legacy
                                // axis takes only its sound image
                                // (`support_band_to_points`).
                                let band = match jump {
                                    0 => Some(Range::new(6, 10)),
                                    1 => Some(Range::new(10, 12)),
                                    _ => None,
                                };
                                if let Some(band) = band {
                                    players[who].narrow_points(support_band_to_points(band));
                                    players[who].narrow_support_points(suit, band);
                                }
                            }
                        }
                    }

                    // Opener's extras-ladder rebid shows extras and — for a
                    // new-suit rung — a five-card opened suit.  Sound floors: the
                    // jump-rebid is 16+, the reverse 17+, the jump-shift 18+.
                    if profile.opener_extras_ladder
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

                    // Opener's major jump-rebid (`ReadingProfile::opener_major_jump_rebid`):
                    // a 3M jump in opener's own opened major over `1♥ - 1♠` / `1M - 1NT`
                    // shows 16+.  Natural, so the six-card length is read above
                    // (the `i_bid_it` branch); add the strength floor here.
                    if profile.opener_major_jump_rebid
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
                            if !profile.garbage_stayman && !profile.crawling_stayman {
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
                        // A natural suit opening is a shown holding (the strong
                        // 2♣ is not); later calls set their bits in the walk.
                        if index == opening_index && !opening_artificial {
                            natural_lane_suits[lane] |= 1 << suit as u8;
                        }
                    }
                    lane_bids[lane] += 1;
                    side_acted[lane % 2] = true;
                    if highest.is_none_or(|h| outranks(bid, h)) {
                        highest = Some(bid);
                    }
                }
            }
        }

        if let Some((seat, strength)) = roll_back {
            players[seat].strength = strength;
        }

        // Record what the suppressed conventional calls genuinely showed.  The
        // block order inside `apply` is load-bearing — see its doc comment.
        readings.apply(
            &mut players,
            &overlay,
            &mut overlay_unions,
            &mut agreement_unions,
            len,
            profile,
        );

        // The vacuous-scoped probed overlay (`ReadingProfile::probed_vacuous`):
        // own-side calls in *contested* prefixes only, folded onto axes every
        // symbolic source above — walk stamps, projections, and hand
        // recordings alike — left fully open.  It runs here, last, because
        // the mask must be judged against the complete symbolic reading (the
        // full fold's home, `project_authored`, runs before the walk stamps).
        // Latest call first: the longest prefix is the sharpest conditioning,
        // and once it fills an axis, earlier keys leave it alone.  The
        // contested gate scopes the fold to the measured coverage hole
        // (contested free bids the walk stamps nothing for): filling
        // constructive ⊤ axes moved ~23% of boards in the 2026-07-31 smoke,
        // all net-OOD grand blasts — the σ-shrink signature of the exclusion
        // retrain.  Under the full fold this is redundant — every probed box
        // already folded unmasked.
        if profile.probed_vacuous
            && !profile.probed
            && let Some(them) = context.own_system()
        {
            // The first index at which both sides have acted: keys through a
            // call before it read purely constructive traffic — not the hole.
            let contested_from = {
                let mut acted = [usize::MAX; 2];
                for (index, call) in auction.iter().enumerate() {
                    if *call != Call::Pass {
                        let side = &mut acted[index % 2];
                        if *side == usize::MAX {
                            *side = index;
                        }
                    }
                }
                acted[0].max(acted[1])
            };
            for index in (contested_from..len).rev() {
                if index % 2 != len % 2 {
                    continue;
                }
                let Some(&box_) = them.probed_box(&auction[..=index]) else {
                    continue;
                };
                let who = relative_of(len, index) as usize;
                let mut masked = Envelope::unknown();
                if players[who].strength.points == Range::FULL_POINTS {
                    masked.strength.points = box_.strength.points;
                }
                for suit in 0..4 {
                    if players[who].lengths[suit] == Range::FULL_LENGTH {
                        masked.lengths[suit] = box_.lengths[suit];
                    }
                }
                if masked != Envelope::unknown() {
                    players[who] = players[who].intersect(&masked);
                }
            }
        }

        Self::assemble(
            players,
            &overlay_unions,
            &agreement_unions,
            control_bid,
            profile,
        )
    }

    /// The last call the M6.4 classifier read as a control bid: its auction
    /// index and the agreed suit (see [`classify_high_bid`])
    #[must_use]
    pub(in crate::bidding) fn control_bid(&self) -> Option<(u8, Suit)> {
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
/// [`Envelope::unknown`], so this is a sound, loose *overlay* — never the natural
/// reading itself (openings, raises, rebids stay in [`Inferences::read`]).
///
/// The projection in isolation: [`Inferences::read`] folds [`project_authored`]
/// directly, so this thin wrapper now serves only the M6.2b equivalence test.
#[cfg(test)]
#[must_use]
pub(crate) fn authored_reading(context: &Context<'_>) -> Inferences {
    let projection = project_authored(context);
    let unions = projection.unions;
    let announced_unions = projection.announced_unions;
    Inferences {
        players: std::array::from_fn(|i| unions[i].hull()),
        announced_players: std::array::from_fn(|i| announced_unions[i].hull()),
        unions,
        announced_unions,
        control_bid: None,
        profile: context.reading_profile(),
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
/// suit-showing call and they chose another suit (`1♦ - 1♠ - 2♦ - 4♥`: 1♥ was
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
    players: &[Envelope; 4],
    overlay: &[Envelope; 4],
    suppressed_so_far: u64,
    profile: ReadingProfile,
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
        // major, `1♥ - 4♣`, since nothing is recorded either way).
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

    // The longer-major response discipline (`ReadingProfile::longer_major_response`)
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
    let discipline = response_to_partners_minor && profile.longer_major_response;

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
/// Combine the final hulled `players` with the disjunctive `overlay` into the
/// per-seat envelope union the sampler consumes
///
/// `players[i]` already folds `overlay[i].hull()` and every hand-walk narrowing,
/// and each overlay box is `⊆` that hull, so re-intersecting recovers exactly
/// `⋃(hand-walk ∩ boxₖ)` — the tight union — while dropping boxes the walk
/// contradicts.  With
/// [`envelope_union`][field@crate::bidding::ReadingProfile::envelope_union]
/// off each overlay is one box,
/// so the result is the single box `players[i]` and
/// `unions[i].hull() == players[i]` (byte-identical).
fn intersect_overlay(
    players: &[Envelope; 4],
    overlay: &[EnvelopeUnion; 4],
    profile: ReadingProfile,
) -> [EnvelopeUnion; 4] {
    std::array::from_fn(|i| EnvelopeUnion::from(players[i]).intersect_owned(&overlay[i], profile))
}

/// Whether the call at `index` is an artificial relay/puppet/splinter in the
/// minor-suit-response structure over our 1NT opening — so it must not be read as a
/// natural long suit
///
/// Once responder enters a structure as their first call, every later three-level
/// suit bid by our side is an artificial relay or splinter.  Which first calls
/// enter, and the lone exception, depend on the active minor scheme
/// ([`notrump_minors`][field@crate::bidding::inference::ReadingProfile::notrump_minors]):
///
/// - **Puppet:** 3♣ Puppet, 2NT diamond transfer, or 2♠ two-way relay — except
///   opener's genuine five-card major show over Puppet (`1NT - 3♣ - 3♥/3♠`).
/// - **European:** 2♠ (clubs) or 3♣ (diamonds) transfer — every continuation
///   (opener's completion, responder's splinter) is a relay, no exception; the
///   natural 2NT invite enters nothing.
///
/// Positions assume the standard uncontested auction; a contested one shifts them
/// and matches none.
///
/// `profile` is the **opening side's** scheme (`side_profile` at the call site),
/// not necessarily ours: a European opponent's `1NT - 2♠` is their club transfer
/// however we play the slot.
///
/// # Two known defects, both A/B-gated
///
/// This is measurably wrong twice over, and neither is fixable inside a
/// fidelity-gated change (`docs/ai-bidder/bba-1nt-minors.md`):
///
/// 1. **No `opening_bid == 1NT` gate.** `entered` looks only at
///    `opening_index + 2`, so `1♣ (1♠) 3♣` and `1♠ (2♦) 2♠` enter the notrump
///    minor structure and get their whole continuation blanketed as relays.
/// 2. **No `is_opening_side` gate,** unlike `nt_splinter_artificial` and
///    `nt_blanket` beside it — so the *defenders'* suit bids are suppressed too.
///
/// Adding gate 2 alone moves **826 of 40000** boards of the shipped default
/// (`smoke-default --count 40000 --seed 1`), 680 of them in auctions containing
/// no 1NT at all — i.e. mostly defect 1 leaking through. That is a live bidding
/// change and needs the A/B the iron rules demand, so both stay recorded here
/// rather than half-fixed.
fn nt_structure_artificial(
    auction: &[Call],
    index: usize,
    opening_index: usize,
    profile: ReadingProfile,
) -> bool {
    let resp_first = auction.get(opening_index + 2);

    if profile.notrump_minors == crate::bidding::american::EUROPEAN {
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

#[cfg(test)]
mod tests;
