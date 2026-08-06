//! The invitational `2NT` continuation in the rich advance of partner's takeout double
//!
//! The accept-or-decline structure is gated by [`set_advance_2nt_continuation`].

use super::advance_minor_jump::advance_minor_jump_rebid;
use super::*;

thread_local! {
    /// Whether the doubler answers the advancer's invitational `2NT` with an
    /// authored accept/decline instead of falling to the instinct floor (which
    /// passes even game-going hands); see [`set_advance_2nt_continuation`].  **On
    /// by default** — a wash-positive A/B fix to a strict floor-pass in the
    /// default-on rich advance.  No effect unless [`RICH_ADVANCE_DOUBLE`] is on.
    static ADVANCE_2NT_CONTINUATION: Cell<bool> = const { Cell::new(true) };
}

/// Toggle the doubler's **accept/decline of the advancer's invitational `2NT`**
/// on the rich advance of a takeout double for books built *after* this call
/// (thread-local, read at book-construction time)
///
/// **On by default**, and a no-op unless [`set_rich_advance_double`] is on. The
/// advancer's `2NT` (`(1t) X - 2NT`) is a limited balanced 11–12 invite with a
/// stopper, but with no authored continuation the doubler falls to the instinct
/// floor, which treats `2NT` as non-forcing and *passes it even holding a game*.
/// When on, the doubler answers the invite naturally: **Pass** declines with a
/// minimum, **`3NT`** accepts to play, and a **new 5-card major** accepts
/// game-forcing so the advancer can pick the 4-4/5-3 major game.  Fixing this
/// floor-pass measured wash-positive on all four cells (NV/vul × plain/PD),
/// which earns the default-on flip.  Off-switch `bba-gen
/// --no-ns-advance-2nt-continuation`.
pub fn set_advance_2nt_continuation(on: bool) {
    ADVANCE_2NT_CONTINUATION.with(|cell| cell.set(on));
}

/// Whether the doubler's answer to the advancer's `2NT` invite is authored
fn advance_2nt_continuation_enabled() -> bool {
    ADVANCE_2NT_CONTINUATION.with(Cell::get)
}

/// Doubler's accept-or-decline of the advancer's invitational `2NT`
/// (`(1t) X - 2NT { - | (X) } ?`, gated by [`set_advance_2nt_continuation`])
///
/// The `2NT` invite is a limited balanced 11–12 with a stopper (the advancer
/// supplies the notrump stopper), so the doubler — sitting on the wide takeout
/// range — simply answers a natural invite: **Pass** declines with a minimum,
/// **`3NT`** accepts to play, and a **new 5-card major** accepts game-forcing so
/// the advancer can choose the 4-4/5-3 major game over `3NT` (the advancer places
/// it — [`advance_minor_jump_rebid`], the same accept-a-forcing-suit logic).  A
/// 5-card *minor* is not shown: with the advancer's stopper `3NT` is almost
/// always right, so only the fit-seeking majors are worth the detour.  All
/// natural; nothing artificial to alert.
fn answer_advance_2nt(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let mut rules = Rules::new()
        // Accept to play: 3NT with a maximum (the advancer holds the stopper).
        .rule(Bid::new(3, Strain::Notrump), 120, hcp(14..))
        // Minimum: decline the invite, play 2NT.
        .rule(Call::Pass, 0, hcp(0..));
    // Accept game-forcing by showing a 5-card major to seek the fit.
    for major in [Suit::Hearts, Suit::Spades] {
        let s = Strain::from(major);
        if s == theirs {
            continue;
        }
        rules = rules.rule(Bid::new(3, s), 130, points(14..) & len(major, 5..));
    }
    rules
}

/// Invitational-`2NT` continuation rows for one opening suit
pub(super) fn advance_2nt_rows(base: &str, theirs: Strain, opening: Bid) -> Vec<Entry> {
    let mut entries = Vec::new();
    if advance_2nt_continuation_enabled() {
        for rho in ["-", "(X)"] {
            let after_2nt = format!("{base} 2NT {rho}");
            entries.extend(rows_of(
                Pattern::node(&after_2nt),
                answer_advance_2nt(opening),
            ));
            // The advancer places game over each forcing major the
            // doubler can show (an unbid major at the three level).
            for major in [Suit::Hearts, Suit::Spades] {
                let s = Strain::from(major);
                if s == theirs {
                    continue;
                }
                let bid = Bid::new(3, s);
                for rho2 in ["-", "(X)"] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("{after_2nt} {bid} {rho2}")),
                        advance_minor_jump_rebid(major),
                    ));
                }
            }
        }
    }
    entries
}

#[cfg(test)]
mod tests;
