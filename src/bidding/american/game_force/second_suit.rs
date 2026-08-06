//! Opener's third call after responder raises opener's second suit
//!
//! Gated by [`set_second_suit_agreement`].  Read at book-construction time;
//! `1M - 2r - 2x - 3x` gets an opener rebid (RKCB on extras, else sign off)
//! instead of falling to the game backstop.

use super::*;
use crate::bidding::american::slam;

std::thread_local! {
    /// Whether opener authors a third-call table after responder raises
    /// opener's second suit (`1M - 2r - 2x - 3x`).  On by default — shipped
    /// (+0.0012 plain / +0.0014 PD NV, +0.0015 / +0.0018 vul IMPs/board vs BBA);
    /// see [`set_second_suit_agreement`].  When off, that node falls through to
    /// the floor (it fell to the game backstop until that was deleted).
    static SECOND_SUIT_AGREEMENT: Cell<bool> = const { Cell::new(true) };
}

/// Toggle opener's third call after responder agrees the second suit
///
/// Read at book-construction time; `1M - 2r - 2x - 3x` gets an opener rebid
/// (RKCB on extras, else sign off) instead of falling to the game backstop.
pub fn set_second_suit_agreement(on: bool) {
    SECOND_SUIT_AGREEMENT.with(|cell| cell.set(on));
}

fn second_suit_agreement() -> bool {
    SECOND_SUIT_AGREEMENT.with(Cell::get)
}

/// Opener's third call after responder raises opener's second suit
///
/// `1M - 2r - 2x - 3x`: responder has agreed opener's second suit `x` as trump in a
/// still-forcing auction (the two-suiter's second fit).  Opener asks with 4NT
/// RKCB on extras, else signs off in game — four of an agreed major, or `3NT`
/// (with `5x` as the deep fallback) when `x` is a minor.  Without this the node
/// falls to [`game_backstop`], which reverts to `4M` after `x` was agreed.
///
/// No [`Pass`][Call::Pass] rule.
fn opener_third_agree(agreed: Suit) -> Rules {
    let strain = Strain::from(agreed);
    let rules = Rules::new()
        .rule(call(4, Strain::Notrump), 100, points(15..))
        .alert(slam::RKCB);
    if matches!(agreed, Suit::Hearts | Suit::Spades) {
        rules.rule(call(4, strain), 50, hcp(0..))
    } else {
        rules
            .rule(call(3, Strain::Notrump), 50, hcp(0..))
            .rule(call(5, strain), 30, hcp(0..))
    }
}

/// Opener's third-call table and RKCB tails after the second suit is agreed
pub(crate) fn second_suit_agreement_continuations() -> Package {
    Package {
        name: "two-over-one-second-suit-agreement",
        gate: second_suit_agreement,
        entries: || {
            let mut entries = Vec::new();
            for major in [Suit::Spades, Suit::Hearts] {
                for resp in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
                    if Strain::from(resp) >= Strain::from(major) {
                        continue;
                    }
                    let prefix = format!(
                        "P* {} - {} -",
                        call(1, Strain::from(major)),
                        call(2, Strain::from(resp)),
                    );
                    for rebid_call in distinct_calls(&opener_rebid(major, resp)) {
                        let Call::Bid(rebid_bid) = rebid_call else {
                            continue;
                        };
                        if rebid_bid.level != Level::new(2) {
                            continue;
                        }
                        let Ok(agreed) = Suit::try_from(rebid_bid.strain) else {
                            continue;
                        };
                        if agreed == major || agreed == resp {
                            continue;
                        }
                        let agreement = format!(
                            "{prefix} {rebid_call} - {} -",
                            call(3, Strain::from(agreed)),
                        );
                        entries.extend(rows_of(
                            Pattern::node(&agreement),
                            opener_third_agree(agreed),
                        ));
                        entries.extend(slam::rkcb_rows(&agreement, agreed));
                    }
                }
            }
            entries
        },
    }
}
