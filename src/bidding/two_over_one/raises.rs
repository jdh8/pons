//! Continuations after the strong raises: Jacoby 2NT and splinters

use super::{call, insert_uncontested, slam};
use crate::bidding::constraint::{fifths, hcp, len, points, top_honors};
use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// Opener's rebid after `1M – (P) – 2NT – (P)`: describe shape and strength
///
/// Jacoby 2NT is a game-forcing raise promising four-card support and 13+ HCP,
/// so opener can safely describe at a high level.  This node is **forcing** —
/// there is no pass rule.
///
/// | Call | Meaning |
/// |---|---|
/// | 4♣/4♦ (below major) | Good five-card second suit (two of top three honors) |
/// | 3♣/3♦/3♥ (side suit shortness) | Singleton or void |
/// | 3M | 18+ balanced-ish acceptance (no side shortness) |
/// | 3NT | 15–17 balanced, no side shortness |
/// | 4M | Minimum opener (12–14) |
fn jacoby_rebids(major: Suit) -> Rules {
    let trump = Strain::from(major);
    let side_suits: Vec<Suit> = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
        .into_iter()
        .filter(|&s| s != major)
        .collect();

    let mut rules = Rules::new();

    // 4-of-x for each side suit x with Strain::from(x) < trump:
    // a good five-card second suit with two of the top three honors.
    for &side in &side_suits {
        if Strain::from(side) < trump {
            rules = rules.rule(
                Bid::new(4, Strain::from(side)),
                2.2,
                len(side, 5..) & top_honors(side, 2..),
            );
        }
    }

    // 3-of-x for each side suit: singleton or void (shortness).
    for &side in &side_suits {
        rules = rules.rule(Bid::new(3, Strain::from(side)), 2.0, len(side, ..=1));
    }

    // No-shortness conjunct: none of the three side suits is short.
    let [a, b, c] = [side_suits[0], side_suits[1], side_suits[2]];
    let no_shortness = !len(a, ..=1) & !len(b, ..=1) & !len(c, ..=1);

    // 3M: 18+ points, no side shortness (big balanced-ish raise acceptance).
    rules = rules.rule(Bid::new(3, trump), 1.5, points(18..) & no_shortness.clone());

    // 3NT: 15–17 Fifths, no side shortness (medium, balanced).
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        1.4,
        fifths(15.0..18.0) & no_shortness,
    );

    // 4M: minimum opener, always applies (guaranteed legal).
    rules.rule(Bid::new(4, trump), 0.5, hcp(0..))
}

/// Responder's continuation after opener's Jacoby rebid
///
/// After a forcing rebid that is not the minimum 4M, responder can drive to
/// slam with 4NT (16+) or settle in game.  After the minimum 4M, slam needs
/// substantially more (18+).
fn responder_after_jacoby(major: Suit, opener_bid: Call) -> Rules {
    let four_major = call(4, Strain::from(major));
    let four_nt = call(4, Strain::Notrump);

    if opener_bid == four_major {
        // Opener showed a minimum; slam needs extra values.
        Rules::new()
            .rule(four_nt, 1.0, points(18..))
            .rule(Call::Pass, 0.0, hcp(0..))
    } else {
        // Opener showed something descriptive; slam is in range with 16+.
        Rules::new()
            .rule(four_nt, 1.0, points(16..))
            .rule(four_major, 0.5, hcp(0..))
    }
}

/// Register the Jacoby 2NT opener-rebid and responder-continuation nodes
///
/// For each major M, inserts:
/// - opener's rebid at `[1M, 2NT]`, and
/// - for every distinct call R in that rebid table, responder's continuation
///   at `[1M, 2NT, R]`, followed by RKCB hooks.
pub(super) fn register(book: &mut Trie) {
    for major in [Suit::Hearts, Suit::Spades] {
        let our_calls = [call(1, Strain::from(major)), call(2, Strain::Notrump)];
        let rebids = jacoby_rebids(major);

        // Collect distinct bid calls before moving `rebids` into the trie.
        let distinct: Vec<Call> = {
            let mut seen = std::collections::HashSet::new();
            rebids
                .rules()
                .iter()
                .filter_map(|r| seen.insert(r.call()).then_some(r.call()))
                .collect()
        };

        insert_uncontested(book, &our_calls, rebids);

        // Responder's continuation after each of opener's rebids.
        for opener_bid in distinct {
            let resp = responder_after_jacoby(major, opener_bid);
            let resp_calls: [Call; 3] = [
                call(1, Strain::from(major)),
                call(2, Strain::Notrump),
                opener_bid,
            ];
            insert_uncontested(book, &resp_calls, resp);
            slam::install_rkcb(book, &resp_calls, major);
        }
    }
}
