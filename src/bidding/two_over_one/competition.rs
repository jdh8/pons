//! Competition over our opening

use super::{call, fallback_all_seats};
use crate::bidding::Competitive;
use crate::bidding::Rules;
use crate::bidding::constraint::{hcp, len};
use crate::bidding::fallback::{Fallback, FirstIs, OvercallAtMost, ReplaceNext};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};
use std::sync::Arc;

/// Negative double of an overcall of our major opening, showing the other major
fn negative_doubles(opening_major: Suit) -> Rules {
    let other = if opening_major == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    Rules::new()
        .rule(Call::Double, 1.0, len(other, 4..) & hcp(8..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// The competitive package over our openings: negative doubles and system-on
///
/// Standalone, the system-on rebase has nothing to land on; bind through
/// [`Pair::against`][crate::bidding::Pair::against] (as
/// [`two_over_one`][super::two_over_one] is meant to be used) so it resolves
/// into the uncontested core.
#[must_use]
pub fn competition() -> Competitive {
    let mut book = Competitive::new();

    // Over our major openings: negative doubles and system-on.
    for major in [Suit::Hearts, Suit::Spades] {
        let opening = call(1, Strain::from(major));
        fallback_all_seats(
            &mut book,
            &[opening],
            2,
            Arc::new(OvercallAtMost(Bid::new(2, Strain::Spades))),
            Fallback::classify(negative_doubles(major)),
        );
        fallback_all_seats(
            &mut book,
            &[opening],
            2,
            Arc::new(FirstIs(Call::Double)),
            Fallback::rebase(ReplaceNext(Call::Pass)),
        );
    }
    book
}
