use crate::bidding::Rules;
use crate::bidding::constraint::{balanced, hcp, stopper_in, support_points};
use crate::bidding::rows::{Entry, Package, Pattern, expand, rows_of};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// Opener's forcing rebid after an inverted minor raise
fn opener_after_inverted_raise(minor: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Notrump), 100, hcp(12..=14) & balanced())
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(18..=19))
        .rule(
            Bid::new(2, Strain::Hearts),
            80,
            stopper_in(Suit::Hearts) & hcp(15..),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            80,
            stopper_in(Suit::Spades) & hcp(15..),
        )
        .rule(Bid::new(3, Strain::from(minor)), 50, hcp(0..))
}

/// Responder's third call after opener rebids `2NT` over an inverted raise
fn responder_after_inverted_raise_two_notrump(minor: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(13..))
        .rule(Bid::new(3, Strain::from(minor)), 50, hcp(0..))
}

/// Responder's third call after opener's 18–19 jump to `3NT`
///
/// With slam values (~32+ combined and 5+-card support), launch minor RKCB;
/// otherwise play the cold 3NT.
fn responder_after_inverted_raise_three_notrump(minor: Suit) -> Rules {
    Rules::new()
        // Responder's seat: the inverted raise promised 5+ trumps, +5.
        .rule(
            Bid::new(4, Strain::Notrump),
            100,
            support_points(minor, 14..),
        )
        .alert(super::super::slam::RKCB)
        .rule(Call::Pass, 50, hcp(0..))
}

/// Responder's third call after opener shows a major-suit stopper
fn responder_after_inverted_raise_major(minor: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(13..))
        .rule(Bid::new(2, Strain::Notrump), 80, hcp(10..=12))
        .rule(Bid::new(3, Strain::from(minor)), 50, hcp(0..))
}

/// Opener places the contract after responder's `2NT` continuation
fn opener_after_inverted_raise_two_notrump() -> Rules {
    Rules::new().rule(Bid::new(3, Strain::Notrump), 50, hcp(0..))
}

pub(super) fn inverted_minor_rows() -> Vec<Entry> {
    let mut entries = Vec::new();
    entries.extend(expand(
        "P* 1m - 2m -",
        |_| true,
        |b| opener_after_inverted_raise(b.suit('m')),
    ));
    entries.extend(expand(
        "P* 1m - 2m - 2NT -",
        |_| true,
        |b| responder_after_inverted_raise_two_notrump(b.suit('m')),
    ));
    entries.extend(expand(
        "P* 1m - 2m - 2M -",
        |_| true,
        |b| responder_after_inverted_raise_major(b.suit('m')),
    ));
    entries.extend(expand(
        "P* 1m - 2m - 2M - 2NT -",
        |_| true,
        |_| opener_after_inverted_raise_two_notrump(),
    ));
    entries
}

/// Minor-suit RKCB asks and answers after an inverted raise and `3NT` rebid
pub(crate) fn minor_keycard_continuations() -> Package {
    Package {
        name: "inverted-minor-keycard",
        gate: super::super::slam::minor_keycard,
        entries: || {
            let mut entries = Vec::new();
            for minor in [Suit::Clubs, Suit::Diamonds] {
                let prefix = format!(
                    "P* {} - {} - 3NT -",
                    super::super::call(1, Strain::from(minor)),
                    super::super::call(2, Strain::from(minor)),
                );
                entries.extend(rows_of(
                    Pattern::node(&prefix),
                    responder_after_inverted_raise_three_notrump(minor),
                ));
                entries.extend(super::super::slam::rkcb_rows(&prefix, minor));
            }
            entries
        },
    }
}
