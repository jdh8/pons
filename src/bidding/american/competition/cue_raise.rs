//! The cue-bid raise, and opener's answer to it
//!
//! Responder's cue of their suit is a limit-plus raise of our opening.  Opener
//! answers under [`set_cue_raise_answer`] for a major and
//! [`set_cue_minor_raise_answer`] for a minor — different agreements, because a
//! minor cue-raise looks for `3NT` and a major one for `4M`.  The *delayed* cue
//! ([`set_delayed_cue`]) is the same raise one round later.

use super::*;

thread_local! {
    /// Whether opener's answer to partner's cue-raise (`1M (ovc) cue -`)
    /// is authored. Default on — without it the cue-raise falls through to the
    /// keyless floor, which cannot act on a bid whose *named* suit (the cue)
    /// differs from its *shown* suit (the major), so opener passes and the
    /// cuebid is left in as the contract.
    static CUE_RAISE_ANSWER: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's answer to partner's cue-raise for books built *after* this
/// call (thread-local)
///
/// **Default on** (`--no-ns-cue-raise-answer` in `bba-gen` for the off arm).
pub fn set_cue_raise_answer(on: bool) {
    CUE_RAISE_ANSWER.with(|cell| cell.set(on));
}

/// Whether opener's answer to a cue-raise is currently authored
pub fn cue_raise_answer() -> bool {
    CUE_RAISE_ANSWER.with(Cell::get)
}

thread_local! {
    /// Whether opener's answer to a *minor*-opening cue-raise
    /// (`1m (ovc) cue -`) is authored. The minor twin of
    /// [`CUE_RAISE_ANSWER`]; separate knob so the A/B can isolate the minor
    /// contribution over the already-shipped major answer. Default on.
    static CUE_MINOR_RAISE_ANSWER: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's answer to a minor-opening cue-raise for books built *after*
/// this call (thread-local)
///
/// **Default on** (`--no-ns-cue-minor-raise-answer` in `bba-gen` for the off
/// arm). Independent of [`set_cue_raise_answer`], which governs the majors.
pub fn set_cue_minor_raise_answer(on: bool) {
    CUE_MINOR_RAISE_ANSWER.with(|cell| cell.set(on));
}

/// Whether opener's answer to a minor-opening cue-raise is currently authored
pub fn cue_minor_raise_answer() -> bool {
    CUE_MINOR_RAISE_ANSWER.with(Cell::get)
}

thread_local! {
    /// Whether the Transfer-Lebensohl cue is split by stopper (see
    /// [`set_delayed_cue`]).
    static DELAYED_CUE: Cell<bool> = const { Cell::new(false) };
}

/// Enable the stopper-split cue for books built *after* this call (thread-local,
/// read once at book-construction time)
///
/// Larry Cohen's fast-denies / slow-shows, adapted to our Transfer Lebensohl:
/// the *direct* cue of their suit denies a stopper, while a *delayed* cue (relay
/// through `2NT`, then their suit) is Stayman *with* a stopper. It also denies a
/// 5-card unbid major (Smolen / Leaping Michaels handle those). Only the
/// single-unbid-major contexts — over `(2♥)` and `(2♠)` — are affected. Off by
/// default; gated behind this toggle for A/B measurement.
pub fn set_delayed_cue(on: bool) {
    DELAYED_CUE.with(|cell| cell.set(on));
}

/// Whether the stopper-split cue is enabled
pub fn delayed_cue() -> bool {
    DELAYED_CUE.with(Cell::get)
}

/// Opener's answer after `1M (ovc) cue -` (partner cue-raised to a
/// limit-plus raise of the opening major): accept to game or decline
///
/// The contested twin of [`opener_after_limit_raise`][super::raises], minus the
/// keycard ask — offering `4NT` RKCB here would strand it, because the contested
/// node has no authored keycard *responses* (the uncontested package includes
/// `slam::rkcb_rows`; this one does not). So a strong opener blasts `4M`
/// game rather than pass out a 4NT nobody answers; slam exploration is a later
/// opt-in.
///
/// The one difference from the uncontested `1M - 3M` version is the **decline**:
/// there partner already *bid* the major, so opener passes to play it; after a
/// cuebid partner named the *opponents'* suit, so opener must actively **sign off
/// in 3M** — passing would leave the cuebid in as the contract (the very bug this
/// table fixes). The point-gate does the work: a minimum opener fails
/// `points(13..)` and takes the 3M catch-all.
pub(super) fn answer_cue_raise(major: Suit) -> Rules {
    let trump = Strain::from(major);
    Rules::new()
        // 4M: accept → game.
        .rule(Bid::new(4, trump), 100, points(13..))
        // 3M: decline → sign off in the major (catch-all).
        //
        // ponytail: decline assumes 3M is legal, which holds for every cue below
        // 3M — all cues over 1♠, and cues over 1♥ except a 3♠ cue. A 3♠ cue over
        // 1♥ with a minimum opener has 3♥ illegal and falls back through to the
        // floor pass; rare, revisit if the A/B surfaces it.
        .rule(Bid::new(3, trump), 0, hcp(0..))
}

/// Opener's answer after `1m (ovc) cue -` (partner cue-raised to a
/// limit-plus raise of the opening minor): bid the best game or sign off
///
/// The minor twin of [`answer_cue_raise`]. Two differences from the major
/// version, both because minor game (`5m`) is remote:
///
/// * **Accept is `3NT`, not `5m`** — gated on values *and* a stopper in their
///   suit (`stopper_in_their_suits`), so we don't get run in the overcall suit.
///   `3NT` outranks any in-scope cue (`≤ 3♠`), so it is always legal.
/// * **Decline is our minor, but its level floats.** After a 1-level overcall
///   the cue is at the 2 level and `3m` signs off; after a 2-level overcall the
///   cue is at the 3 level and `3m` sits *below* it for a club opening, so `4m`
///   is the only sign-off. The engine does **not** mask illegal calls, so each
///   decline rung is legality-anchored with `min_level_is`: exactly one of `3m`
///   / `4m` is the cheapest our-minor bid in any given auction, and only that
///   one fires.
pub(super) fn answer_cue_minor_raise(minor: Suit) -> Rules {
    let trump = Strain::from(minor);
    Rules::new()
        // 3NT: accept to the best game — needs values and their suit stopped.
        //
        // ponytail: always 3NT, never 5m. A single stopper is thin against a
        // 6-card overcall suit, and a 10-card minor fit sometimes plays 5m when
        // 3NT gets run. The A/B win is net of that tail; splitting 3NT-vs-5m on
        // fit length is the upgrade path if a re-measure wants the last IMPs.
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            points(14..) & stopper_in_their_suits(),
        )
        // 3m: decline when our minor is still available at the 3 level.
        .rule(Bid::new(3, trump), 50, min_level_is(3, trump))
        // 4m: decline when 3m sits below the cue (club opening, 3-level cue).
        .rule(Bid::new(4, trump), 50, min_level_is(4, trump))
}

/// Section 4b as a row package: opener answers partner's cue-raise of the
/// opening major ([`set_cue_raise_answer`][super::set_cue_raise_answer])
///
/// The columns after the opening are their overcall at level `i` in suit `x`,
/// our cue *of that suit* at level `j`, their pass.  The shared letter is what
/// makes it a cue, and ascension supplies `j > i` — enumerating every legal cue
/// height is the point, not an accident.  The overcall is capped at `2♠`, the
/// cue-raise's authored ceiling in [`over_their_overcall`], and `x` excludes
/// our own major: when the opponents cue-bid *our* suit (a Michaels
/// `1♠ (2♠)`), responder's `3♠` is a natural raise, not a cue-raise, and this
/// table must not hijack it.  Their `1NT` overcall gets the notrump twin, where
/// the cue is `2NT` and up — `1NT` is the only NT overcall under the cap, since
/// `2NT` outranks `2♠` (their `2NT` over our major is the UvU package's).
///
/// Without the node the cue-raise falls through to the keyless floor — whose
/// raise ladder needs partner's *named* and *shown* suit to agree, which a cue
/// decouples — so opener passes and the cuebid is left in as the contract.
pub(super) fn cue_raise_answer_package() -> Package {
    Package {
        name: "cue-raise-answer",
        gate: |agreements| agreements.competition.cue_raise_answer,
        entries: |_| {
            [Suit::Hearts, Suit::Spades]
                .into_iter()
                .flat_map(|major| {
                    let key = format!("P* 1{}", Strain::from(major));
                    let capped = Bid::new(2, Strain::Spades);
                    let mut entries = expand(
                        &format!("{key} (ix) jx -"),
                        move |b: &Bindings| {
                            Bid::new(b.level('i').get(), b.suit('x').into()) <= capped
                                && b.suit('x') != major
                        },
                        move |_: &Bindings| answer_cue_raise(major),
                    );
                    entries.extend(expand(
                        &format!("{key} (iN) jN -"),
                        move |b: &Bindings| Bid::new(b.level('i').get(), Strain::Notrump) <= capped,
                        move |_: &Bindings| answer_cue_raise(major),
                    ));
                    entries
                })
                .collect()
        },
    }
}

/// Section 4c as a row package: the minor twin of
/// [`cue_raise_answer_package`] ([`set_cue_minor_raise_answer`][super::set_cue_minor_raise_answer])
///
/// A minor-opening cue-raise passes out the same way.  The cue may sit as high
/// as `3♠` (a 2-level overcall forces the cue to the 3 level), so the ceiling
/// rides the *cue* — `j·x ≤ 3♠` — rather than the overcall.  `x != minor` again
/// excludes a cue of our own suit (`1♣ (2♣)` Michaels: responder's `3♣` is a
/// raise).  Under the cue cap the notrump twin holds a single column,
/// `(1NT) 2NT`.
pub(super) fn cue_minor_raise_answer_package() -> Package {
    Package {
        name: "cue-minor-raise-answer",
        gate: |agreements| agreements.competition.cue_minor_raise_answer,
        entries: |_| {
            [Suit::Clubs, Suit::Diamonds]
                .into_iter()
                .flat_map(|minor| {
                    let key = format!("P* 1{}", Strain::from(minor));
                    let capped = Bid::new(3, Strain::Spades);
                    let mut entries = expand(
                        &format!("{key} (ix) jx -"),
                        move |b: &Bindings| {
                            Bid::new(b.level('j').get(), b.suit('x').into()) <= capped
                                && b.suit('x') != minor
                        },
                        move |_: &Bindings| answer_cue_minor_raise(minor),
                    );
                    entries.extend(expand(
                        &format!("{key} (iN) jN -"),
                        move |b: &Bindings| Bid::new(b.level('j').get(), Strain::Notrump) <= capped,
                        move |_: &Bindings| answer_cue_minor_raise(minor),
                    ));
                    entries
                })
                .collect()
        },
    }
}

/// The retired guarded wirings of [`cue_raise_answer_package`] and
/// [`cue_minor_raise_answer_package`], kept as the resolution-equivalence
/// oracles for `converted_packages_match_legacy`
#[cfg(test)]
fn cue_raise_legacy_rows(minors: bool) -> Vec<Entry> {
    let trumps: [Suit; 2] = if minors {
        [Suit::Clubs, Suit::Diamonds]
    } else {
        [Suit::Hearts, Suit::Spades]
    };
    trumps
        .into_iter()
        .flat_map(|our| {
            let trump = Strain::from(our);
            // The sample's overcall must not be in our own suit.
            let sample = if our == Suit::Spades {
                "(2♥) 3♥ -"
            } else {
                "(1♠) 2♠ -"
            };
            rows_of(
                Pattern::guarded(
                    &format!("P* 1{trump}"),
                    sample,
                    described_guard(
                        "(overcall) cue -",
                        guard(move |_: &Context<'_>, suffix: &[Call]| {
                            matches!(
                                suffix,
                                [Call::Bid(ovc), Call::Bid(cue), Call::Pass]
                                    if cue.strain == ovc.strain
                                        && cue > ovc
                                        && (if minors {
                                            *cue <= Bid::new(3, Strain::Spades)
                                        } else {
                                            *ovc <= Bid::new(2, Strain::Spades)
                                        })
                                        && ovc.strain != trump
                            )
                        }),
                    ),
                ),
                if minors {
                    answer_cue_minor_raise(our)
                } else {
                    answer_cue_raise(our)
                },
            )
        })
        .collect()
}

#[cfg(test)]
pub(super) fn cue_raise_answer_package_legacy() -> Package {
    Package {
        name: "cue-raise-answer",
        gate: |agreements| agreements.competition.cue_raise_answer,
        entries: |_| cue_raise_legacy_rows(false),
    }
}

#[cfg(test)]
pub(super) fn cue_minor_raise_answer_package_legacy() -> Package {
    Package {
        name: "cue-minor-raise-answer",
        gate: |agreements| agreements.competition.cue_minor_raise_answer,
        entries: |_| cue_raise_legacy_rows(true),
    }
}

#[cfg(test)]
mod tests;
