//! Major game tries after a single raise: `1M - 2M`
//!
//! Responder's single raise promises three-plus trumps and 6–9 points, so
//! opener needs real extras to move: a long-suit try, the general re-raise, or
//! a keycard-asking maximum.  Gated by [`set_major_game_tries`], default on
//! (+0.042/+0.065 IMPs/board NV/vul, silenced-opponent A/B, 200k boards/vul,
//! plain-DD + perfect-defense both winning).

use super::*;

std::thread_local! {
    /// Whether opener's long-suit game tries after a single raise (`1M - 2M`)
    /// are authored.  Default on (measured +0.042/+0.065 IMPs/board NV/vul).
    static MAJOR_GAME_TRIES: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's major game tries after `1M - 2M` for books built *after*
/// this call
///
/// Read at book construction; **default on** (`--no-ns-major-game-tries` in
/// `bba-gen` for the off arm).
pub fn set_major_game_tries(on: bool) {
    MAJOR_GAME_TRIES.with(|cell| cell.set(on));
}

/// Whether major game tries are currently authored
pub(crate) fn major_game_tries() -> bool {
    MAJOR_GAME_TRIES.with(Cell::get)
}

/// The level of the cheapest available call in `suit` over `2` of `major`
///
/// A suit ranked above the major is still open at the two level; a suit
/// ranked below it must jump to the three level to be bid at all.
fn try_level(major: Suit, suit: Suit) -> u8 {
    if Strain::from(suit) > Strain::from(major) {
        2
    } else {
        3
    }
}

/// The three side suits available as a long-suit game try, cheapest first
///
/// At most one suit outranks the major (the other major, over `1♥`), so the
/// order is: that suit at the two level, if any, then the rest at the three
/// level in ascending rank.  Hearts: `[♠, ♣, ♦]`; spades: `[♣, ♦, ♥]`.
fn game_try_suits(major: Suit) -> Vec<Suit> {
    let major_strain = Strain::from(major);
    let mut above = Vec::new();
    let mut below = Vec::new();
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == major {
            continue;
        }
        if Strain::from(suit) > major_strain {
            above.push(suit);
        } else {
            below.push(suit);
        }
    }
    above.into_iter().chain(below).collect()
}

/// Opener's continuation after `1M - 2M -`: game tries toward a
/// non-forcing raise
///
/// Responder's single raise promises three-plus trumps and 6–9 points, so
/// opener needs real extras to move: a maximum drives to game outright (or
/// asks for keycards on a huge hand), 16–18 explores with a long-suit game
/// try or the general re-raise, and anything below settles in the part score.
///
/// | Call | Meaning |
/// |---|---|
/// | 4NT | RKCB ask (22+) |
/// | 4M | Non-asking maximum (19+) |
/// | 2♠/3♣/3♦ (hearts) or 3♣/3♦/3♥ (spades) | Long-suit game try (16–18, 4+ in the suit) |
/// | 3M | The general re-raise try (16–18), below every suit try in weight |
/// | Pass | Minimum, nothing more to show |
#[must_use]
fn opener_after_raise(major: Suit) -> Rules {
    let trump = Strain::from(major);

    // Opener's seat throughout: the trump is the own five-card major, +5.
    let mut rules = Rules::new()
        // 4NT: RKCB ask on a maximum.
        .rule(
            Bid::new(4, Strain::Notrump),
            260,
            support_points(major, 22..),
        )
        .alert(slam::RKCB)
        // 4M: a non-asking maximum.
        .rule(Bid::new(4, trump), 220, support_points(major, 19..));

    // Long-suit game tries, cheapest first: natural, no alert.
    for (suit, weight) in game_try_suits(major).into_iter().zip([150_i16, 145, 140]) {
        rules = rules.rule(
            Bid::new(try_level(major, suit), Strain::from(suit)),
            weight,
            len(suit, 4..) & support_points(major, 16..=18),
        );
    }

    rules
        // 3M: the general re-raise try, deliberately below the suit tries.
        .rule(Bid::new(3, trump), 120, support_points(major, 16..=18))
        // Pass: a minimum, the finite catch-all.
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's answer to a long-suit game try: accept with a maximum, a
/// shortage, or two top honors in the tried suit — decline otherwise
///
/// Forcing by omission below `3M`: every try sits under it, so the decline is
/// always legal.
#[must_use]
fn responder_after_try(major: Suit, try_suit: Suit) -> Rules {
    let trump = Strain::from(major);
    Rules::new()
        // Accept: a maximum single raise, or good shape in the try suit.
        // Responder's seat: the single raise promised 3+ trumps, +3.
        .rule(
            Bid::new(4, trump),
            100,
            support_points(major, 8..=9) | len(try_suit, ..=1) | top_honors(try_suit, 2..),
        )
        // Decline, guaranteed legal (every try sits below 3M).
        .rule(Bid::new(3, trump), 50, hcp(0..))
}

/// Responder's answer to the general re-raise try: accept with a maximum,
/// passable
#[must_use]
fn responder_after_general_try(major: Suit) -> Rules {
    Rules::new()
        // Responder's seat: the single raise promised 3+ trumps, +3.
        .rule(
            Bid::new(4, Strain::from(major)),
            100,
            support_points(major, 8..=9),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's follow-up after a long-suit try is declined: push on with
/// extras, passable
#[must_use]
fn opener_after_decline(major: Suit) -> Rules {
    Rules::new()
        // Opener's seat: the trump is the own five-card major, +5.
        .rule(
            Bid::new(4, Strain::from(major)),
            100,
            support_points(major, 18..),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Major game tries after `1M - 2M`, with every answer and RKCB subtree
pub(crate) fn major_game_try_continuations() -> Package {
    Package {
        name: "major-game-try-continuations",
        gate: major_game_tries,
        entries: || {
            let mut entries = Vec::new();
            for major in [Suit::Hearts, Suit::Spades] {
                let trump = Strain::from(major);
                let prefix = format!("P* {} - {} -", call(1, trump), call(2, trump));
                entries.extend(rows_of(Pattern::node(&prefix), opener_after_raise(major)));
                entries.extend(slam::rkcb_rows(&prefix, major));

                for suit in game_try_suits(major) {
                    let try_call = call(try_level(major, suit), Strain::from(suit));
                    let tried = format!("{prefix} {try_call} -");
                    entries.extend(rows_of(
                        Pattern::node(&tried),
                        responder_after_try(major, suit),
                    ));

                    let declined = format!("{tried} {} -", call(3, trump));
                    entries.extend(rows_of(
                        Pattern::node(&declined),
                        opener_after_decline(major),
                    ));
                }

                let general = format!("{prefix} {} -", call(3, trump));
                entries.extend(rows_of(
                    Pattern::node(&general),
                    responder_after_general_try(major),
                ));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
