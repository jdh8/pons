//! Major-suit strain after a natural positive to the strong `2♣`
//!
//! Gated by
//! [`RebidKnobs::strong_two_positive`][crate::bidding::agreements::RebidKnobs::strong_two_positive].
//! Off, opener's rebid over `2♣ - 2NT`, `2♣ - 2M` and `2♣ - 3m` is the
//! floor's, which counts points and jumps to `6NT`.  Its *level* is close to
//! right — double-dummy, slam pays from about 31 combined HCP — so these
//! tables leave every notrump decision with it and author only the major
//! fit and its keycards:
//!
//! - **Opener's rebid**: `4NT` RKCB with three-card support for a major
//!   positive; else a five-card major at the cheapest level.  Any other hand
//!   falls through to the floor (over their double, where it cannot, `3NT`).
//! - **Responder over opener's major**: `4M` with three-card support, over
//!   which opener asks; without it `6NT` on 9+ HCP and `3NT` below that.
//! - **Opener over that `3NT`**: `6NT` on 23+, `4M` on a six-card major, else
//!   pass.
//!
//! Not authored: opener's minors, minor-suit raises (the minor RKCB tables
//! have no grand rung), responder's second suit, and the contested tails
//! (every call here but the `4NT` ask is natural, so the floor's `2♣` game
//! force carries them).

use super::super::slam::{RKCB, rkcb_rows};
use crate::bidding::Rules;
use crate::bidding::constraint::{hcp, len, support};
use crate::bidding::rows::{Entry, Pattern, rows_of};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

#[cfg(test)]
mod tests;

/// Opener's major rebids over `positive`: the cheapest bid in each major
/// other than responder's, below `3NT`
fn new_majors(positive: Bid) -> impl Iterator<Item = (Suit, Bid)> {
    [Suit::Hearts, Suit::Spades]
        .into_iter()
        .filter_map(move |suit| {
            let strain = Strain::from(suit);
            let bid = (2..=3)
                .map(|level| Bid::new(level, strain))
                .find(|&bid| bid > positive)?;
            (strain != positive.strain).then_some((suit, bid))
        })
}

/// Opener's rebid after the positive (at `2♣ - positive -`)
///
/// No catch-all on purpose while the opponents are silent: a hand with
/// neither the fit nor a five-card major falls through to the floor.
///
/// `doubled` is the same table at `2♣ (X) positive -`, made total with a
/// `3NT` catch-all (the contested floor's own usual call).  Left to the
/// `2♣ (X)` systems-on rebase, a rejected hand would *not* fall through — the
/// fall-through pass resolves the rebase again and lands on the same table —
/// and opener would pass a game force.
// ponytail: `3NT` whatever the strength; delete the doubled node once
// `Trie::resolve_floored` skips a rejected classifier through a rebase.
fn opener_rebid(positive: Bid, doubled: bool) -> Rules {
    let mut rules = Rules::new();
    if matches!(positive.strain, Strain::Hearts | Strain::Spades) {
        rules = rules
            .rule(Bid::new(4, Strain::Notrump), 170, support(3..))
            .alert(RKCB);
    }
    for (suit, bid) in new_majors(positive) {
        // Spades before hearts on 5-5, as over the waiting 2♦.
        let weight = if suit == Suit::Spades { 155 } else { 150 };
        rules = rules.rule(bid, weight, len(suit, 5..));
    }
    if doubled {
        rules.rule(Bid::new(3, Strain::Notrump), 10, hcp(0..))
    } else {
        rules
    }
}

/// Responder after opener's major (at `2♣ - positive - major -`)
fn responder_after_major(suit: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, suit.into()), 150, support(3..))
        .rule(Bid::new(6, Strain::Notrump), 120, hcp(9..))
        .rule(Bid::new(3, Strain::Notrump), 50, hcp(0..))
}

/// Opener after responder's `3NT` over his major
fn opener_after_denial(suit: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(6, Strain::Notrump), 120, hcp(23..))
        .rule(Bid::new(4, suit.into()), 100, len(suit, 6..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener over the raise of his major: always the `4NT` RKCB
fn ask() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, hcp(0..))
        .alert(RKCB)
}

/// Every row below the natural positives
///
/// `waiting` says whether `2♥` is a positive (`strong_two_waiting`) or the
/// double negative, which keeps its own subtree.
pub(super) fn entries(waiting: bool) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut table = |node: &str, rules: Rules| {
        entries.extend(rows_of(Pattern::node(node), rules));
    };
    let mut asks = Vec::new();

    for positive in [
        Bid::new(2, Strain::Hearts),
        Bid::new(2, Strain::Spades),
        Bid::new(2, Strain::Notrump),
        Bid::new(3, Strain::Clubs),
        Bid::new(3, Strain::Diamonds),
    ] {
        if positive == Bid::new(2, Strain::Hearts) && !waiting {
            continue;
        }
        let rebid = format!("P* 2♣ - {positive} -");
        table(&rebid, opener_rebid(positive, false));
        table(
            &format!("P* 2♣ (X) {positive} -"),
            opener_rebid(positive, true),
        );

        // Opener's RKCB for a major positive.
        if let Ok(suit) = Suit::try_from(positive.strain)
            && matches!(suit, Suit::Hearts | Suit::Spades)
        {
            asks.push((rebid.clone(), suit));
        }

        for (suit, bid) in new_majors(positive) {
            let shown = format!("{rebid} {bid} -");
            table(&shown, responder_after_major(suit));
            table(&format!("{shown} 3NT -"), opener_after_denial(suit));
            let raise = format!("{shown} {} -", Bid::new(4, suit.into()));
            table(&raise, ask());
            asks.push((raise, suit));
        }
    }

    for (prefix, trump) in asks {
        entries.extend(rkcb_rows(&prefix, trump));
    }
    entries
}
