//! Responder's rebid after opener raises the two-over-one hearts,
//! `1♠ - 2♥ - 3♥`
//!
//! Gated by [`GameForceKnobs::heart_raise_keycard`].  The raise shows four
//! hearts facing five, a nine-card fit, and the base table's only heart call,
//! the six-card `3♥` rebid, is illegal there and dropped when the book is
//! built: responder bids `3♠` on three spades, the no-fit notrump ladder, or
//! leaves a six-card minimum to the floor's `4♥`.
//! On, hearts are trump: responder asks keycards on extras or signs off in
//! `4♥`, and opener may ask over the sign-off.

use super::*;
use crate::bidding::american::slam;

/// Opener's points from which it asks over responder's `4♥` sign-off
///
/// ponytail: fixed; make it the knob's second term if the ask is swept.
const OPENER_ASK: u8 = 17;

/// Responder's rebid at `1♠ - 2♥ - 3♥ -`: the keycard ask on `ask`+ points,
/// else `4♥`
///
/// No [`Pass`][Call::Pass] rule: the auction is still game forcing.
fn responder(ask: u8) -> Rules {
    Rules::new()
        .rule(call(4, Strain::Notrump), 100, points(ask..))
        .alert(slam::RKCB)
        .rule(call(4, Strain::Hearts), 50, hcp(0..))
}

/// Opener over the `4♥` sign-off: the keycard ask on extras, else pass
fn opener() -> Rules {
    Rules::new()
        .rule(call(4, Strain::Notrump), 100, points(OPENER_ASK..))
        .alert(slam::RKCB)
        .rule(Call::Pass, 0, hcp(0..))
}

/// Every row of the agreement, replacing the base table at `1♠ - 2♥ - 3♥ -`
pub(super) fn rows(ask: u8) -> Vec<Entry> {
    let agreed = "P* 1♠ - 2♥ - 3♥ -";
    let signed_off = format!("{agreed} 4♥ -");
    let mut entries = rows_of(Pattern::node(agreed), responder(ask));
    entries.extend(slam::rkcb_rows(agreed, Suit::Hearts));
    entries.extend(rows_of(Pattern::node(&signed_off), opener()));
    entries.extend(slam::rkcb_rows(&signed_off, Suit::Hearts));
    entries
}
