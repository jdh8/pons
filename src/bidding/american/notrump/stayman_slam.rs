//! Slam machinery over Stayman — the cue continuation and the minor slam try
//!
//! Two agreements above the game level once opener has answered:
//! [`NotrumpKnobs::stayman_cue_continuation`][crate::bidding::agreements::NotrumpKnobs::stayman_cue_continuation] (responder cue-bids toward a major slam)
//! and [`NotrumpKnobs::stayman_minor_slam_try`][crate::bidding::agreements::NotrumpKnobs::stayman_minor_slam_try] (the `2♦` denial's minor-fit route into
//! keycard).

use super::*;

/// Opener's answer to a direct four-of-a-major slam try (`1NT - 4♥/4♠`)
///
/// Non-forcing: a **maximum** (17) accepts by launching RKCB (`4NT`); a minimum
/// signs off by passing the major game.  The 1430 ladder ([`slam`]) then exchanges
/// keycards and places `6M`, or `5M` when the partnership is missing two.
pub(crate) fn slam_try_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, hcp(17..))
        .alert(slam::RKCB)
        .rule(Call::Pass, 0, hcp(..17))
}

/// Opener holds a first- or second-round honour control (an ace or king) in `suit`
fn control_in(suit: Suit) -> Cons<impl Constraint + Clone> {
    // ponytail: A/K only — ignores shortness controls (void/singleton).  A full
    // cue scheme would add them, but a balanced 1NT opener rarely holds one.
    described(
        format!("control in {suit}"),
        move |hand: Hand, _: &Context<'_>| {
            let holding = hand[suit];
            holding.contains(Rank::A) || holding.contains(Rank::K)
        },
    )
}

/// Opener's reply to responder's `3OM` slam try / choice of game
///
/// A flat `(4333)` chooses notrump (`3NT`); a maximum (17) cue-bids the cheapest
/// honour control to cooperate; otherwise opener signs off in the major game.
pub(super) fn stayman_slam_try_answer(major: Suit) -> Rules {
    let mut rules = Rules::new().rule(Bid::new(3, Strain::Notrump), 140, flat_4333());
    // Cheapest control cue with a maximum: each suit ranking below the major.
    let mut weight = 130;
    for cue in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        if Strain::from(cue) < Strain::from(major) {
            rules = rules.rule(
                Bid::new(4, Strain::from(cue)),
                weight,
                hcp(17..) & control_in(cue),
            );
            weight -= 5;
        }
    }
    // Minimum, or a maximum without a cheap control: sign off in game.
    rules.rule(Bid::new(4, Strain::from(major)), 100, hcp(0..))
}

/// Responder's rebid after opener cooperates with the `3OM` slam try by cue-bidding
///
/// Opener's cue (a control ranking below the trump `major`) showed a **maximum**
/// (17) plus slam interest — see [`stayman_slam_try_answer`].  Responder's `3OM` was
/// a wide choice-of-game *or* slam try, so responder resolves it here: a slam-worthy
/// hand keycards (`4NT` RKCB, the [`slam`] 1430 ladder placing the contract),
/// everything else signs off in the major game.  Without this node opener's cue was
/// passed out — often *below* game — the dominant Stayman leak this fixes.  Gated by
/// [`NotrumpKnobs::stayman_cue_continuation`][crate::bidding::agreements::NotrumpKnobs::stayman_cue_continuation] (on by default).
fn stayman_cue_rebid(major: Suit) -> Rules {
    Rules::new()
        // Slam values opposite a known maximum plus the shown control: keycard.
        .rule(Bid::new(4, Strain::Notrump), 120, hcp(14..))
        .alert(slam::RKCB)
        // Otherwise the 3OM was only choosing the game: sign off in the major.
        .rule(Bid::new(4, Strain::from(major)), 100, hcp(0..))
}

/// Opener's reply to responder's Stayman-then-minor slam try (`…3♣` / `…3♦`)
///
/// Responder showed a natural 5+ `minor` with slam values (14+) and no major fit.
/// With four-card support *and* a maximum (16-17) opener cooperates by raising to
/// `4m`, setting trump for responder's keycard ask; otherwise — no fit, or a
/// minimum — opener signs off in `3NT`, the game responder's values guarantee.
/// The `3NT` catch-all keeps the table total (the finite-fallback invariant).
fn stayman_minor_answer(minor: Suit) -> Rules {
    Rules::new()
        // Fit + maximum: raise the minor, inviting the keycard ask.
        .rule(
            Bid::new(4, Strain::from(minor)),
            130,
            len(minor, 4..) & hcp(16..),
        )
        // No fit, or a minimum: place game in notrump.
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
}

/// Responder's keycard ask after opener raises the Stayman-then-minor slam try
/// (`…3m - 4m`)
///
/// Opener confirmed a four-card fit and a maximum, so responder — who opened the
/// slam try with 14+ — keycards (`4NT` RKCB, the [`slam`] 1430 ladder placing the
/// minor slam or signing off in `5m` when a keycard is missing).  Both hands are
/// known non-minimum before the ask, so — unlike the transfer-then-minor path
/// ([`gf_minor_answer`]) — the five-level response is safe.  Artificial, so the
/// `4NT` carries the [`slam::RKCB`] alert.
fn stayman_minor_slam_rkcb() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, hcp(0..))
        .alert(slam::RKCB)
}

/// Stayman cue-bid continuations and their RKCB subtrees
pub(crate) fn cue() -> Package {
    Package {
        name: "stayman-cue-continuation",
        gate: |agreements| agreements.notrump.stayman_cue_continuation,
        entries: |_| {
            let two_h = call(2, Strain::Hearts);
            let two_s = call(2, Strain::Spades);
            let three_h = call(3, Strain::Hearts);
            let three_s = call(3, Strain::Spades);
            let mut entries = Vec::new();

            for (answer, three_om, major) in [
                (two_h, three_s, Suit::Hearts),
                (two_s, three_h, Suit::Spades),
            ] {
                for cue_suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
                    if Strain::from(cue_suit) >= Strain::from(major) {
                        continue;
                    }
                    let path = format!(
                        "P* 1NT - 2♣ - {answer} - {three_om} - {} -",
                        call(4, Strain::from(cue_suit)),
                    );
                    entries.extend(rows_of(Pattern::node(&path), stayman_cue_rebid(major)));
                    entries.extend(slam::rkcb_rows(&path, major));
                }
            }

            entries
        },
    }
}

/// Stayman minor slam tries and their RKCB subtrees
pub(crate) fn minor_slam() -> Package {
    Package {
        name: "stayman-minor-slam-try",
        gate: |agreements| agreements.notrump.stayman_minor_slam_try,
        entries: |_| {
            let two_d = call(2, Strain::Diamonds);
            let two_h = call(2, Strain::Hearts);
            let two_s = call(2, Strain::Spades);
            let three_c = call(3, Strain::Clubs);
            let three_d = call(3, Strain::Diamonds);
            let mut entries = Vec::new();

            for answer in [two_h, two_s, two_d] {
                for (three_m, minor) in [(three_c, Suit::Clubs), (three_d, Suit::Diamonds)] {
                    let prefix = format!("P* 1NT - 2♣ - {answer} - {three_m} -");
                    entries.extend(rows_of(Pattern::node(&prefix), stayman_minor_answer(minor)));

                    let path = format!("{prefix} {} -", call(4, Strain::from(minor)));
                    entries.extend(rows_of(Pattern::node(&path), stayman_minor_slam_rkcb()));
                    entries.extend(slam::rkcb_rows(&path, minor));
                }
            }

            entries
        },
    }
}

#[cfg(test)]
mod tests;
