//! Both-major agreements — the Stayman `2NT` relay, the five-card max, and `1NT - 3♦`
//!
//! Three ways a both-majors hand is shown: opener's max-only relay over Stayman
//! ([`set_stayman_both_majors`]), the five-card-major maximum jump
//! ([`set_stayman_5card_max`]), and responder's direct `3♦` on 5-5 majors,
//! invitational or better.
use super::*;

/// Responder's relay over opener's max-both-majors `2NT`
///
/// Opener has both four-card majors and a maximum, so responder names *their* own
/// longer major — `3♣` = hearts, `3♦` = spades — asking opener to bid it so the
/// strong concealed hand declares (right-siding).  Both are alerted (artificial).
/// Responder bid Stayman, so always holds a four-card major; the two rules tile
/// every hand, so no catch-all is needed.  A 4-4 tie goes to hearts (the lower
/// major), keeping the most room to escape if an opponent doubles the relay.
fn both_majors_max_responder() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Diamonds),
            100,
            described("spades > hearts", |hand: Hand, _: &Context<'_>| {
                hand[Suit::Spades].len() > hand[Suit::Hearts].len()
            }),
        )
        .alert(BOTH_MAJORS)
        .rule(
            Bid::new(3, Strain::Clubs),
            100,
            described("hearts ≥ spades", |hand: Hand, _: &Context<'_>| {
                hand[Suit::Hearts].len() >= hand[Suit::Spades].len()
            }),
        )
        .alert(BOTH_MAJORS)
}

/// Opener's forced completion of the both-majors relay (right-siding)
///
/// Responder named a major via `3♣`/`3♦`; opener simply bids it so opener declares.
/// Alerted — it completes the relay and shows nothing beyond the `2NT` already did.
fn both_majors_relay_complete(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::from(major)), 100, hcp(0..))
        .alert(BOTH_MAJORS)
}

/// Responder places game over opener's right-siding completion
///
/// Opener's maximum (16-17) and the major fit are both known, so the invite is
/// pre-accepted: bid game when the agreed fit is worth it, else pass the
/// three-level completion (the floor's settle).  The fit is gauged as
/// `points + extra trumps + a fit in the other major`: shape counts now the
/// trump suit is agreed, a fifth trump (the 9-card fit) adds a point, and — since
/// opener showed *both* four-card majors — four in the unnamed major is a known
/// second 4-4 fit worth another.  A flat single 4-4 still needs a full eight; a
/// 5-4 or a double fit reaches game a king lighter.  A bare `points(6..)` on the
/// fifth trump alone overbid the 5-3-3-2 nothing hands this gate now passes.
fn both_majors_relay_placement(major: Suit) -> Rules {
    let other = match major {
        Suit::Spades => Suit::Hearts,
        _ => Suit::Spades,
    };
    Rules::new().rule(
        Bid::new(4, Strain::from(major)),
        130,
        described(
            "game values for the agreed major",
            move |hand: Hand, context: &Context<'_>| {
                let double_fit = usize::from(hand[other].len() >= 4);
                fit_value(context, hand, major) + double_fit >= 8
            },
        ),
    )
}

/// Responder's trump-length-adjusted value for a known `major` fit
///
/// Point count plus one per trump beyond the eighth — the ninth and tenth
/// trump are worth a point apiece now the suit is agreed.  No double-fit term:
/// at a plain Stayman answer opener showed only the one major, so a second fit
/// is unknowable.  ([`both_majors_relay_placement`] adds it back where opener
/// *did* show both majors.)
pub(super) fn fit_value(context: &Context<'_>, hand: Hand, major: Suit) -> usize {
    // Fit-known (the major is agreed), so count shortness as support value —
    // in the side suits only, never in trump itself; the
    // length-beyond-the-eighth term is explicit.
    let profile = context.reading_profile();
    let support = usize::from(support_point_count_in_on(
        profile.support_points(),
        profile.point_scale(),
        hand,
        major,
    ));
    support + hand[major].len().saturating_sub(4)
}

/// Responder's placement over opener's max five-card-major jump (`3♥`/`3♠`)
///
/// With three-card support (an eight-card fit) opposite a maximum, bid game; else
/// sign off in `3NT`.
fn five_card_max_rebid(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 130, len(major, 3..))
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
}

thread_local! {
    /// Opener jumps to `2NT` over `1NT - 2♣` holding *both* four-card majors and a
    /// *maximum* (16-17); a minimum (15) bids 2♥ naturally.  Responder then names own
    /// major (`3♣` = hearts, `3♦` = spades) and opener completes (`3♥`/`3♠`), so the
    /// strong concealed hand declares the known 4-4 fit (right-siding) instead of
    /// responder declaring after a direct raise.  **On by default** — a paired DD
    /// A/B vs BBA (320k boards/arm, vul none) measured +2.18 IMPs/fired plain
    /// (+0.0035/board, 95% CI excl 0) and +2.29 PD *with garbage on*, +2.68/+2.87
    /// with garbage off — a win in every regime, unlike the earlier strength-step
    /// scheme it replaces.  See [`set_stayman_both_majors`].
    static STAYMAN_BOTH_MAJORS: Cell<bool> = const { Cell::new(true) };
    /// Opener jumps `3♥`/`3♠` over `1NT - 2♣` holding a *five-card* major and a
    /// maximum (16-17), showing the 5-3/5-4 fit plus extras.  **On by default** —
    /// the cleanest of the three: +3.45 IMPs/fired plain (+0.0007/board, 95% CI
    /// excl 0) and +3.33 PD, holding up at +1.47/+0.90 even with garbage on.  See
    /// [`set_stayman_5card_max`].
    static STAYMAN_5CARD_MAX: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's max-only right-siding relay over `1NT - 2♣` with both four-card
/// majors for books built *after* this call (thread-local; **on by default**).
pub fn set_stayman_both_majors(on: bool) {
    STAYMAN_BOTH_MAJORS.with(|cell| cell.set(on));
}

/// Author opener's max five-card-major jump over `1NT - 2♣` for books built *after*
/// this call (thread-local; **on by default**).
pub fn set_stayman_5card_max(on: bool) {
    STAYMAN_5CARD_MAX.with(|cell| cell.set(on));
}

/// Whether opener's both-majors max-only relay is currently authored
pub fn stayman_both_majors() -> bool {
    STAYMAN_BOTH_MAJORS.with(Cell::get)
}

/// Whether opener's max five-card-major jump is currently authored
pub fn stayman_5card_max() -> bool {
    STAYMAN_5CARD_MAX.with(Cell::get)
}

/// Opener's answer to the both-majors 3♦: pick the strain by strength
///
/// With a maximum (17) jump to the eight-card major game, or 3NT when 2-2 in the
/// majors leaves only a seven-card fit.  A minimum (15–16) signs off in three of
/// the better major — spades whenever holding three, else hearts — leaving
/// responder to pass an invitation or raise with game values.  Authored, not
/// floored: the keyless floor misreads 3♦ as natural diamonds and forces game.
//
// ponytail: "better major" is spades-with-three, else hearts — it finds an
// eight-card fit when one exists but prefers spades on a tie (e.g. 3♠ on 3-4
// majors).  Good enough; refine only if the A/B asks for it.
fn five_five_major_answer() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Spades),
            120,
            hcp(17..) & len(Suit::Spades, 3..),
        )
        .rule(
            Bid::new(4, Strain::Hearts),
            120,
            hcp(17..) & len(Suit::Spades, ..3) & len(Suit::Hearts, 3..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            hcp(17..) & len(Suit::Spades, ..3) & len(Suit::Hearts, ..3),
        )
        .rule(Bid::new(3, Strain::Spades), 100, len(Suit::Spades, 3..))
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Spades, ..3))
}

/// Responder's decision over opener's minimum 3-level signoff
///
/// Opener showed 15–16 by signing off in `major`; responder raises to game with
/// the upper half of the invitational+ range and otherwise passes.  Needed
/// because the floor forces responder to game off the 3♦ opening and so could
/// not pass the invitation.  `points` again — responder is the 5-5 hand.
fn five_five_min_rebid(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 100, points(10..))
        .rule(Call::Pass, 90, points(..10))
}

/// Stayman maximum showing both four-card majors, with a right-siding relay
pub(crate) fn both_majors_relay() -> Package {
    Package {
        name: "stayman-both-majors-relay",
        gate: |agreements| agreements.build.notrump.stayman_both_majors,
        entries: |_| {
            let mut entries = rows_of(
                Pattern::node("P* 1NT - 2♣ - 2NT -"),
                both_majors_max_responder(),
            );
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2NT - 3♣ -"),
                both_majors_relay_complete(Suit::Hearts),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2NT - 3♦ -"),
                both_majors_relay_complete(Suit::Spades),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2NT - 3♣ - 3♥ -"),
                both_majors_relay_placement(Suit::Hearts),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2NT - 3♦ - 3♠ -"),
                both_majors_relay_placement(Suit::Spades),
            ));
            entries
        },
    }
}

/// Stayman maximum showing a five-card major
pub(crate) fn five_card_max() -> Package {
    Package {
        name: "stayman-five-card-max",
        gate: |agreements| agreements.build.notrump.stayman_5card_max,
        entries: |_| {
            expand(
                "P* 1NT - 2♣ - 3M -",
                |_| true,
                |b| five_card_max_rebid(b.suit('M')),
            )
        },
    }
}

/// Both-majors 1NT - 3♦ answer and responder's decision over a minimum
pub(crate) fn both_majors_three_diamond() -> Package {
    Package {
        name: "both-majors-three-diamond",
        gate: |_| true,
        entries: |_| {
            let mut entries = rows_of(Pattern::node("P* 1NT - 3♦ -"), five_five_major_answer());
            entries.extend(expand(
                "P* 1NT - 3♦ - 3M -",
                |_| true,
                |b| five_five_min_rebid(b.suit('M')),
            ));
            entries
        },
    }
}

#[cfg(test)]
mod tests;
