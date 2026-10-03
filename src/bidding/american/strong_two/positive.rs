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
//!   over a suit positive falls through to the floor (over their double,
//!   where it cannot, `3NT`).
//! - **Responder over opener's major**: `4M` with three-card support, over
//!   which opener asks; without it `6NT` on 9+ HCP and `3NT` below that.
//! - **Opener over that `3NT`**: `6NT` on 23+, `4M` on a six-card major, else
//!   pass.
//!
//! [`RebidKnobs::strong_two_grand`][crate::bidding::agreements::RebidKnobs::strong_two_grand]
//! (default on) adds the **grand rung**: every keycard ask here bids seven on
//! two of the three side kings (`slam::grand_rkcb_rows`), and the same ladder
//! answers the floor's `4NT` over a minor positive — the ask stays the
//! floor's, since its level judgement is at par, but its own minor ladder
//! stops in six short of 37 combined points.
//!
//! [`RebidKnobs::strong_two_positive_notrump`][crate::bidding::agreements::RebidKnobs::strong_two_positive_notrump]
//! (default on) is the one place the count is authored: over the
//! balanced `2NT` positive, a hand with no five-card major bids `3NT` on up to
//! 24 HCP, `6NT` on 25–29 and `7NT` on 30+, and responder raises the `3NT` to
//! six on 9+ and to seven on 14+, the `6NT` to seven on 11+.  The floor's own
//! `6NT` there is the evaluator net's, bid on 22 opposite the positive's 7.
//!
//! Not authored: opener's minors, minor-suit raises, responder's second suit,
//! and the contested tails (every call here but the `4NT` ask is natural, so
//! the floor's `2♣` game force carries them).

use super::super::slam::{RKCB, grand_rkcb_rows, rkcb_rows};
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
///
/// `count` (`strong_two_positive_notrump`) makes the table over the `2NT`
/// positive total with the notrump count, doubled or not.
// ponytail: `3NT` whatever the strength; delete the doubled node once
// `Trie::resolve_floored` skips a rejected classifier through a rebase.
fn opener_rebid(positive: Bid, doubled: bool, count: bool) -> Rules {
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
    if count && positive.strain == Strain::Notrump {
        rules
            .rule(Bid::new(7, Strain::Notrump), 100, hcp(30..))
            .rule(Bid::new(6, Strain::Notrump), 100, hcp(25..=29))
            .rule(Bid::new(3, Strain::Notrump), 100, hcp(..=24))
    } else if doubled {
        rules.rule(Bid::new(3, Strain::Notrump), 10, hcp(0..))
    } else {
        rules
    }
}

/// Responder after opener's counted `3NT` over the `2NT` positive: opener
/// holds at most 24, and double-dummy six pays from 9 opposite, seven from 14
fn responder_after_counted_game() -> Rules {
    Rules::new()
        .rule(Bid::new(7, Strain::Notrump), 100, hcp(14..))
        .rule(Bid::new(6, Strain::Notrump), 100, hcp(9..=13))
        .rule(Call::Pass, 0, hcp(..=8))
}

/// Responder after opener's counted `6NT` (25–29): seven on 11+
fn responder_after_counted_slam() -> Rules {
    Rules::new()
        .rule(Bid::new(7, Strain::Notrump), 100, hcp(11..))
        .rule(Call::Pass, 0, hcp(..=10))
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
/// double negative, which keeps its own subtree.  `grand` is
/// `strong_two_grand`: the grand rung on every ask, and the ladder below the
/// floor's `4NT` over a minor positive.  `count` is
/// `strong_two_positive_notrump`: the notrump count over the `2NT` positive.
pub(super) fn entries(waiting: bool, grand: bool, count: bool) -> Vec<Entry> {
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
        let doubled = format!("P* 2♣ (X) {positive} -");
        table(&rebid, opener_rebid(positive, false, count));
        table(&doubled, opener_rebid(positive, true, count));
        if count && positive.strain == Strain::Notrump {
            for prefix in [&rebid, &doubled] {
                table(&format!("{prefix} 3NT -"), responder_after_counted_game());
                table(&format!("{prefix} 6NT -"), responder_after_counted_slam());
            }
        }

        // Opener's RKCB for a major positive — and, on the grand rung, the
        // floor's for a minor one.
        if let Ok(suit) = Suit::try_from(positive.strain)
            && (grand || matches!(suit, Suit::Hearts | Suit::Spades))
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

    let rkcb = if grand { grand_rkcb_rows } else { rkcb_rows };
    for (prefix, trump) in asks {
        entries.extend(rkcb(&prefix, trump));
    }
    entries
}
