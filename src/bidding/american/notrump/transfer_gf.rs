//! Game-forcing continuations after a Jacoby transfer
//!
//! Responder's `3`-level rebid sets up a game force; opener places the
//! contract, and the 5-5 and splinter reroutes send the slam-worthy shapes
//! somewhere better than `3NT`.  Gated by [`set_transfer_gf_majors`] and
//! [`set_transfer_gf_hearts`].

use super::*;

thread_local! {
    /// Responder's game-forcing structure after the spade transfer completes
    /// (`1NT - 2♥ - 2♠`): the natural `3♥` 5-5 slam try, minor side-suits (`3♣`/`3♦`),
    /// and the single-suiter's quantitative `4NT`; **on by default** (A/B vs BBA:
    /// plain +0.0014, PD +0.0016 IMPs/board, both CI ±0.0003, +1.70/+1.90 per fired).
    /// See [`set_transfer_gf_majors`].
    static TRANSFER_GF_MAJORS: Cell<bool> = const { Cell::new(true) };
    /// Mirror the GF structure onto the *heart* transfer (`1NT - 2♦ - 2♥`): minor
    /// side-suits (`3♣`/`3♦`), the `3♠` spade splinter (plus `4♣`/`4♦`), and the
    /// quantitative `4NT` — the single-suited slam try relocating off `3♠`, just as
    /// spades relocated off `3♥`.  The 5-5 slam try needs no heart slot (it rides the
    /// spade transfer).  No-op unless [`set_transfer_gf_majors`] is also on; **on by
    /// default** (A/B vs BBA, two seeds: plain +0.0015/+0.0017, PD +0.0016/+0.0018
    /// IMPs/board, all CI ±0.0003, +1.83/+2.08 per fired).  See [`set_transfer_gf_hearts`].
    static TRANSFER_GF_HEARTS: Cell<bool> = const { Cell::new(true) };
}

/// Author responder's game-forcing structure after the spade transfer for books
/// built *after* this call (thread-local; **on by default**).
///
/// After `1NT - 2♥ - 2♠`, responder's game-forcing hands otherwise fall to the floor's
/// natural raise.  When on: a natural `3♥` shows 5-5 majors with slam interest
/// (rerouted off the capped both-majors `3♦`), `3♣`/`3♦` show a five-spade hand with
/// a four-card minor, and `4NT` is the single-suiter's quantitative slam invite
/// (relocated off the repurposed `3♥`).  Pass `false` to fall back to the floor.
pub fn set_transfer_gf_majors(on: bool) {
    TRANSFER_GF_MAJORS.with(|cell| cell.set(on));
}

/// Whether the post-transfer game-forcing structure is currently authored
pub fn transfer_gf_majors() -> bool {
    TRANSFER_GF_MAJORS.with(Cell::get)
}

/// Mirror the post-transfer game-forcing structure onto the heart transfer for books
/// built *after* this call (thread-local; **on by default**).
///
/// After `1NT - 2♦ - 2♥`, responder shows a five-heart-plus-minor game force (`3♣`/`3♦`),
/// a six-heart splinter (`3♠` short in spades, `4♣`/`4♦` short in a minor), or a
/// single-suited quantitative slam invite (`4NT`, relocated off the `3♠` slam try).
/// The 5-5 majors slam try keeps its single home on the spade transfer.  No effect
/// unless [`set_transfer_gf_majors`] is also on.
pub fn set_transfer_gf_hearts(on: bool) {
    TRANSFER_GF_HEARTS.with(|cell| cell.set(on));
}

/// Whether the heart-transfer mirror is asked for, ignoring the master gate
///
/// The mirror is a no-op unless [`set_transfer_gf_majors`] is also on; the
/// conjunction lives in
/// [`transfer_gf_heart_mirror`](crate::bidding::context::DecisionProfile::transfer_gf_heart_mirror),
/// which is what the book gates on.  This getter returns the raw setting, so
/// arming the mirror under a closed master gate and reading it back gives what
/// was armed.
pub fn transfer_gf_hearts() -> bool {
    TRANSFER_GF_HEARTS.with(Cell::get)
}

/// Responder's game-forcing rebid after the spade transfer completes
/// (`1NT - 2♥ - 2♠`), under the GF-majors structure
///
/// `3♥` is a natural 5-5 slam try — the slam end of the both-majors hands, rerouted
/// off the capped `1NT - 3♦` (the `points(17..)` floor tiles against the `3♦` cap of
/// `points(..=16)`).  `4NT` is the single-suiter's quantitative slam invite (16+,
/// denying a fourth heart — the hand the old artificial `3♥` slam try showed, now
/// relocated here).  `3♥` floors only its own strain (the transfer pins the five
/// spades), so it stays unalerted; `4NT` is conventional and carries [`SLAM_TRY`].
/// `3♣`/`3♦` show five spades and a four-card minor — game-forcing (Arm A), or with
/// [`minor_min_to_3nt`] on slam-only (Arm B), the minimums then resting in the
/// floor's choice-of-games `3NT`.  All three natural calls floor only their own
/// strains, so they stay unalerted; `4NT` is conventional and carries [`SLAM_TRY`].
/// Empty off the gate.
pub(super) fn transfer_spade_gf_rebid(agreements: &Agreements) -> Rules {
    if !agreements.decision.transfer_gf_majors {
        return Rules::new();
    }
    // Arm A shows the minor on any game force; Arm B reserves it for slam tries,
    // routing minimum game-forces to the floor's `3NT` (the `minor_min_to_3nt` A/B).
    let minor_floor: u8 = if agreements.notrump.minor_min_to_3nt {
        17
    } else {
        10
    };
    Rules::new()
        // The transfer already pins responder's five spades (`transfer_major_reading`),
        // so these natural rebids restate only their *own* second strain — flooring no
        // un-named suit keeps them off the alert list (the `artificial` invariant).
        .rule(
            Bid::new(3, Strain::Hearts),
            150,
            len(Suit::Hearts, 5..) & points(17..),
        )
        .rule(
            Bid::new(3, Strain::Clubs),
            145,
            len(Suit::Clubs, 4..) & len(Suit::Hearts, ..4) & points(minor_floor..),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            145,
            len(Suit::Diamonds, 4..) & len(Suit::Hearts, ..4) & points(minor_floor..),
        )
        // Choice of games: exactly five spades (the transfer pins the floor; `..6`
        // rules out a six-card one-suiter), game values but short of the 16+ slam
        // quant.  Natural (all upper bounds — floors no un-named suit, so
        // unalerted), and being a recognised undisturbed node it lets opener read
        // 3NT as *balanced* rather than guessing — the read only holds undisturbed,
        // which is why opener's correction gates on it (`has_ruffing_shortness`).
        //
        // `hcp(9..16)`, not `points(10..) & hcp(..16)`, and no minor caps: the
        // gate mirrors the instinct floor's game force (`hcp(9..)` undisturbed —
        // the shipped 9-count seam, `nt_responder_game_floor`).  The old tighter
        // gates sent the 9-count and the 4-card-minor game forces below the
        // minor rules' `points(10..)` through to that floor, which bid the
        // *same* 3NT — and under `set_natural_reading` this rule's projected
        // box then excluded the very hands that bid it.  Every hand the old
        // gates admitted was balanced (shape forced by the caps), where
        // `points(10..)` implies `hcp(10..)`, so this is a pure loosening: the
        // bidder is unchanged (hands newly admitted here bid this same 3NT via
        // the floor before, and every rule sharing a 9..15 hand outweighs 1.4).
        .rule(
            Bid::new(3, Strain::Notrump),
            140,
            len(Suit::Spades, ..6) & len(Suit::Hearts, ..4) & hcp(9..16),
        )
        .rule(
            Bid::new(4, Strain::Notrump),
            140,
            len(Suit::Spades, 5..)
                & len(Suit::Hearts, ..4)
                & len(Suit::Clubs, ..4)
                & len(Suit::Diamonds, ..4)
                & hcp(16..),
        )
        .alert(SLAM_TRY)
        // Six-card-spade slam tries with a side-suit splinter — carved off the direct
        // Texas `4♦` / `4♠` (see `is_major_splinter_slam`).  Artificial (the bid names
        // the *short* suit, not a strain to play), so each carries [`SPLINTER`].
        // Responder's own six-card spade suit is the trump, +6.
        .rule(
            Bid::new(4, Strain::Clubs),
            155,
            len(Suit::Spades, 6..)
                & splinter_short(Suit::Clubs)
                & support_points(Suit::Spades, 16..),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(4, Strain::Diamonds),
            155,
            len(Suit::Spades, 6..)
                & splinter_short(Suit::Diamonds)
                & support_points(Suit::Spades, 16..),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(4, Strain::Hearts),
            155,
            len(Suit::Spades, 6..)
                & splinter_short(Suit::Hearts)
                & support_points(Suit::Spades, 16..),
        )
        .alert(SPLINTER)
}

/// Responder's game-forcing rebid after the *heart* transfer completes (`1NT - 2♦ - 2♥`)
///
/// The heart mirror of [`transfer_spade_gf_rebid`], tighter because `2♠`/`2NT` are the
/// single-suited/`5♥4♠` invites and `3♥` is the six-card invite.  So there is **no**
/// 5-5 slot here — a 5-5 slam try rides the spade transfer ([`slam_55_reroute`]) — and
/// the spade shortness splinter drops to a cheap `3♠` (below `4♥`), freed by evicting
/// the single-suited slam try to `4NT`:
///
/// - `3♣`/`3♦` — five hearts and a four-card minor, game-forcing (Arm A).  Natural
///   (the transfer pins the hearts), flooring only the minor, so unalerted.
/// - `3♠` / `4♣` / `4♦` — a six-heart splinter short in spades / clubs / diamonds,
///   carried onto the transfer off the direct Texas `4♣` / `4♥`; each carries
///   [`SPLINTER`].
/// - `4NT` — the single-suiter's quantitative slam invite, [`SLAM_TRY`].
///
/// Empty off the heart gate.
pub(super) fn transfer_heart_gf_rebid(agreements: &Agreements) -> Rules {
    if !agreements.decision.transfer_gf_heart_mirror() {
        return Rules::new();
    }
    let minor_floor: u8 = if agreements.notrump.minor_min_to_3nt {
        17
    } else {
        10
    };
    // The transfer pins responder's five hearts, so these natural rebids restate only
    // their own second strain (floor no un-named suit → off the alert list).
    Rules::new()
        .rule(
            Bid::new(3, Strain::Clubs),
            145,
            len(Suit::Clubs, 4..) & len(Suit::Spades, ..4) & points(minor_floor..),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            145,
            len(Suit::Diamonds, 4..) & len(Suit::Spades, ..4) & points(minor_floor..),
        )
        // Choice of games: exactly five hearts (`..6` rules out a six-card one-suiter),
        // game values short of the 16+ slam quant.  Natural (upper bounds
        // only — unalerted), the undisturbed node that lets opener read 3NT as
        // *balanced* for the ruff-gated correction.  `hcp(9..16)` mirrors the
        // instinct floor's game force so the rule owns every 3NT this node
        // actually produces — see the spade mirror (`transfer_spade_gf_rebid`).
        .rule(
            Bid::new(3, Strain::Notrump),
            140,
            len(Suit::Hearts, ..6) & len(Suit::Spades, ..4) & hcp(9..16),
        )
        // Six-card-heart slam tries with a side-suit splinter — the spade shortness at
        // the cheap `3♠`, the minors at the four level.  Artificial, so each is alerted.
        // Responder's own six-card heart suit is the trump, +6.
        .rule(
            Bid::new(3, Strain::Spades),
            155,
            len(Suit::Hearts, 6..)
                & splinter_short(Suit::Spades)
                & support_points(Suit::Hearts, 16..),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(4, Strain::Clubs),
            155,
            len(Suit::Hearts, 6..)
                & splinter_short(Suit::Clubs)
                & support_points(Suit::Hearts, 16..),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(4, Strain::Diamonds),
            155,
            len(Suit::Hearts, 6..)
                & splinter_short(Suit::Diamonds)
                & support_points(Suit::Hearts, 16..),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(4, Strain::Notrump),
            140,
            len(Suit::Hearts, 5..)
                & len(Suit::Spades, ..4)
                & len(Suit::Clubs, ..4)
                & len(Suit::Diamonds, ..4)
                & hcp(16..),
        )
        .alert(SLAM_TRY)
}

/// Opener's answer to responder's quantitative `4NT` (`1NT - 2♥ - 2♠ - 4NT`)
///
/// Responder is a balanced 16+ single-suited five-spade hand inviting slam.  A
/// maximum (17) accepts — `6M` on three-card support (the known eight-card fit),
/// else `6NT`; a minimum declines by passing `4NT`.  ponytail: blasts the small slam
/// rather than RKCB — the invite is balanced and quantitative, so a keycard detour
/// buys little.
fn gf_quant_answer(major: Suit) -> Rules {
    Rules::new()
        .rule(
            Bid::new(6, Strain::from(major)),
            130,
            hcp(17..) & len(major, 3..),
        )
        .rule(Bid::new(6, Strain::Notrump), 120, hcp(17..))
        .rule(Call::Pass, 0, hcp(..17))
}

/// Opener's answer to responder's five-spade-plus-minor game force (`…3♣` / `…3♦`)
///
/// The five-three major fit is the anchor, and the minor is game-forcing but
/// *undifferentiated* (minimum through slam in Arm A), so opener places game rather
/// than keycarding: `4M` on three-card support — the 5-3 fit's ruffing value
/// out-scores an un-pulled `3NT` — else `3NT`.  ponytail: no RKCB over the minor. A
/// paired A/B caught opener blasting slam on its own maximum opposite a possible bare
/// minimum, doubled into the artificial five-level keycard response; the minor's own
/// slam is left to the (near-impossible) rare hand this treatment already discounts.
fn gf_minor_answer(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 110, len(major, 3..))
        .rule(Bid::new(3, Strain::Notrump), 100, len(major, ..3))
}

/// Opener's answer to responder's six-card-spade splinter (`…4♣` / `…4♦` / `…4♥`)
///
/// The major is agreed (responder showed six) and the shortness is known, so this is
/// game-forcing: a maximum (17) cooperates by launching RKCB in the major (`4NT`), a
/// minimum signs off in the major game (`4M`).  ponytail: the shortness-versus-values
/// judgment (are opener's honors opposite the splinter wasted?) is left to raw
/// strength — a finer wasted-value gate is the upgrade path if the A/B wants it.
fn gf_splinter_answer(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, hcp(17..))
        .alert(slam::RKCB)
        .rule(Bid::new(4, Strain::from(major)), 0, hcp(..17))
}

/// The `longer` major strictly outnumbers `shorter` — the transfer names the
/// longer major (see [`NotrumpKnobs::transfer_longer_major`][crate::bidding::agreements::NotrumpKnobs::transfer_longer_major])
pub(super) fn longer_major(longer: Suit, shorter: Suit) -> Cons<impl Constraint + Clone> {
    longer_suit(longer, shorter)
}

/// Both majors of exactly equal length (the 5-5/6-6 two-suiters)
pub(super) fn equal_majors() -> Cons<impl Constraint + Clone> {
    equal_length("equal-length majors", Suit::Hearts, Suit::Spades)
}

/// A 5-5 majors hand strong enough to drive slam (`point_count ≥ 17`)
///
/// Capped off the both-majors `3♦` (`points(..=16)`) when the GF-majors structure
/// is on, such a hand instead transfers and rebids the natural `3♥` slam try, so
/// this widens the spade-transfer guard to admit it.  Always false off the gate, so
/// the baseline transfer guard is unchanged.
pub(super) fn slam_55_reroute() -> Cons<impl Constraint + Clone> {
    let legacy = described(
        "5-5 slam reroute to the spade transfer",
        |hand: Hand, context: &Context<'_>| {
            let profile = context.decision_profile();
            profile.transfer_gf_majors
                && hand[Suit::Hearts].len() >= 5
                && usize::from(point_count_on(profile.reading.point_scale(), hand)) >= 17
        },
    );
    // Whenever the gate accepts at all it requires 5+ hearts and 17+ points
    // (and off its treatment it accepts nothing), so the box is sound in both
    // knob states.
    let mut envelope = long_suit_box(
        Suit::Hearts,
        Range::new(5, Range::FULL_LENGTH.max),
        Range::FULL_LENGTH,
    );
    envelope.strength.points = Range::new(17, Range::FULL_POINTS.max);
    envelope_union_upgrade(legacy, EnvelopeUnion::from(envelope))
}

/// A void or low singleton — the shortness a splinter shows
///
/// A singleton ace or king is a *working* honor, not shortness to advertise, so it
/// is excluded (the same wasted-honor principle as [`upgrade`]'s `wasted`).
fn is_splinter_holding(holding: Holding) -> bool {
    holding.is_empty()
        || (holding.len() == 1 && !holding.contains(Rank::A) && !holding.contains(Rank::K))
}

/// Void or a low singleton in `suit`, as a constraint (see [`is_splinter_holding`])
pub(super) fn splinter_short(suit: Suit) -> Cons<impl Constraint + Clone> {
    let legacy = described(
        "void or a low singleton",
        move |hand: Hand, _: &Context<'_>| is_splinter_holding(hand[suit]),
    );
    // Sound over-approximation: the singleton-A/K exclusion is honor
    // location, which no `Envelope` axis holds — the box keeps `suit ≤ 1`.
    envelope_union_upgrade(
        legacy,
        EnvelopeUnion::from(long_suit_box(suit, Range::new(0, 1), Range::FULL_LENGTH)),
    )
}

/// A six-card-`major` slam hand (16+ points) with a describable side-suit splinter
///
/// Under the GF-majors gate this hand is carved off the direct Texas transfer and
/// direct game-jump onto the Jacoby transfer, where it splinters (spades at the four
/// level, hearts as low as `3♠`).  Gated per major — spades by the master flag, hearts
/// by the heart mirror — so each side is false until its own structure is on.
fn is_major_splinter_slam(context: &Context<'_>, hand: Hand, major: Suit) -> bool {
    let decision = context.decision_profile();
    let profile = decision.reading;
    let active = if major == Suit::Hearts {
        decision.transfer_gf_heart_mirror()
    } else {
        decision.transfer_gf_majors
    };
    active
        && hand[major].len() >= 6
        // Fit-known: 6-card major opposite 1NT's 2+ is an 8-card fit, and this
        // hand has a side-suit splinter — count the shortness as support
        // value, in lockstep with the splinter gates'
        // `support_points(major, 16..)` and the reroute boxes.
        && usize::from(support_point_count_in_on(
            profile.support_points(),
            profile.point_scale(),
            hand,
            major,
        )) >= 16
        && Suit::ASC
            .into_iter()
            .filter(|&suit| suit != major)
            .any(|suit| is_splinter_holding(hand[suit]))
}

/// The splinter reroute as a positive constraint (widens the major's transfer guard)
pub(super) fn major_splinter_reroute(major: Suit) -> Cons<impl Constraint + Clone> {
    let legacy = described(
        "6-card-major splinter reroute to the transfer",
        move |hand: Hand, context: &Context<'_>| is_major_splinter_slam(context, hand, major),
    );
    // One box per short side suit: 6+ in the major, 16+ support points, that
    // side suit at most one card (the singleton-honor exclusion is
    // over-approximated away, as in `splinter_short`).  Sound in both
    // treatment states; raw `union`, never `disjoin`, for authoring-time
    // boxes (the C2 precedent — `disjoin` would hull them while the knob is
    // off during book construction).
    let boxes = Suit::ASC
        .into_iter()
        .filter(|&suit| suit != major)
        .map(|short| {
            let mut envelope = long_suit_box(
                major,
                Range::new(6, Range::FULL_LENGTH.max),
                Range::FULL_LENGTH,
            );
            envelope.lengths[short as usize] = Range::new(0, 1);
            envelope.narrow_support_points(major, Range::new(16, Range::FULL_POINTS.max));
            EnvelopeUnion::from(envelope)
        })
        .reduce(EnvelopeUnion::union)
        .unwrap_or_else(EnvelopeUnion::unknown);
    envelope_union_upgrade(legacy, boxes)
}

/// The splinter reroute negated (carves the direct Texas and direct game-jump)
pub(super) fn not_major_splinter_slam(major: Suit) -> Cons<impl Constraint + Clone> {
    described(
        "not a describable 6-card-major splinter slam",
        move |hand: Hand, context: &Context<'_>| !is_major_splinter_slam(context, hand, major),
    )
}

/// Game-forcing spade-transfer continuations and their RKCB subtrees
pub(crate) fn spade_transfer_game_force() -> Package {
    Package {
        name: "spade-transfer-game-force",
        gate: |agreements| agreements.decision.transfer_gf_majors,
        entries: |_| {
            let mut entries = rows_of(
                Pattern::node("P* 1NT - 2♥ - 2♠ - 4NT -"),
                gf_quant_answer(Suit::Spades),
            );
            entries.extend(expand(
                "P* 1NT - 2♥ - 2♠ - 3m -",
                |_| true,
                |_| gf_minor_answer(Suit::Spades),
            ));
            for short in [Strain::Clubs, Strain::Diamonds, Strain::Hearts] {
                let path = format!("P* 1NT - 2♥ - 2♠ - {} -", call(4, short),);
                entries.extend(rows_of(
                    Pattern::node(&path),
                    gf_splinter_answer(Suit::Spades),
                ));
                entries.extend(slam::rkcb_rows(&path, Suit::Spades));
            }
            entries
        },
    }
}

/// Game-forcing heart-transfer continuations and their RKCB subtrees
pub(crate) fn heart_transfer_game_force() -> Package {
    Package {
        name: "heart-transfer-game-force",
        gate: |agreements| agreements.decision.transfer_gf_heart_mirror(),
        entries: |_| {
            let mut entries = rows_of(
                Pattern::node("P* 1NT - 2♦ - 2♥ - 4NT -"),
                gf_quant_answer(Suit::Hearts),
            );
            entries.extend(expand(
                "P* 1NT - 2♦ - 2♥ - 3m -",
                |_| true,
                |_| gf_minor_answer(Suit::Hearts),
            ));
            for splinter in [
                call(3, Strain::Spades),
                call(4, Strain::Clubs),
                call(4, Strain::Diamonds),
            ] {
                let path = format!("P* 1NT - 2♦ - 2♥ - {splinter} -");
                entries.extend(rows_of(
                    Pattern::node(&path),
                    gf_splinter_answer(Suit::Hearts),
                ));
                entries.extend(slam::rkcb_rows(&path, Suit::Hearts));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
