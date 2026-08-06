//! Jacoby 2NT: opener's descriptive rebid and responder's slam try
//!
//! The game-forcing raise promising four-card support and 13+ HCP, so opener
//! can safely describe shape and strength at a high level.  Always on — this
//! agreement has no knob; the two *further* raise continuations
//! ([`super::game_try`], [`super::limit_raise`]) each have their own.

use super::*;

/// Shortness — opener's `3`-of-a-side-suit singleton/void show after Jacoby 2NT
const SHORTNESS: Alert = Alert("shortness");

/// Opener's rebid after `1M - 2NT -`: describe shape and strength
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
                220,
                len(side, 5..) & top_honors(side, 2..),
            );
        }
    }

    // 3-of-x for each side suit: singleton or void (shortness).
    for &side in &side_suits {
        rules = rules
            .rule(Bid::new(3, Strain::from(side)), 200, len(side, ..=1))
            .alert(SHORTNESS);
    }

    // No-shortness conjunct: none of the three side suits is short.
    let [a, b, c] = [side_suits[0], side_suits[1], side_suits[2]];
    let no_shortness = !len(a, ..=1) & !len(b, ..=1) & !len(c, ..=1);

    // 3M: 18+ points, no side shortness (big balanced-ish raise acceptance).
    // Opener's seat: the trump is the own five-card major, +5.
    rules = rules.rule(
        Bid::new(3, trump),
        150,
        support_points(major, 18..) & no_shortness.clone(),
    );

    // 3NT: 15–17 Fifths, no side shortness (medium, balanced).
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        140,
        fifths(15.0..18.0) & no_shortness,
    );

    // 4M: minimum opener, always applies (guaranteed legal).
    rules.rule(Bid::new(4, trump), 50, hcp(0..))
}

/// Responder's continuation after opener's Jacoby rebid
///
/// After a forcing rebid that is not the minimum 4M, responder can drive to
/// slam with 4NT (16+) or settle in game.  After the minimum 4M, slam needs
/// substantially more (18+).
fn responder_after_jacoby(major: Suit, opener_bid: Call) -> Rules {
    let four_major = call(4, Strain::from(major));
    let four_nt = call(4, Strain::Notrump);

    // Responder's seat: Jacoby promised four-card support, +4.
    if opener_bid == four_major {
        // Opener showed a minimum; slam needs extra values.
        Rules::new()
            .rule(four_nt, 100, support_points(major, 18..))
            .alert(slam::RKCB)
            .rule(Call::Pass, 0, hcp(0..))
    } else {
        // Opener showed something descriptive; slam is in range with 16+.
        Rules::new()
            .rule(four_nt, 100, support_points(major, 16..))
            .alert(slam::RKCB)
            .rule(four_major, 50, hcp(0..))
    }
}

/// Jacoby 2NT opener rebids, responder continuations and RKCB answer trees
pub(crate) fn jacoby_continuations() -> Package {
    Package {
        name: "jacoby-two-notrump-continuations",
        gate: || true,
        entries: || {
            let mut entries = Vec::new();
            for major in [Suit::Hearts, Suit::Spades] {
                let prefix = format!("P* {} - 2NT -", call(1, Strain::from(major)),);
                let rebids = jacoby_rebids(major);

                // Derive continuation keys from the live source table while
                // preserving first-declaration order.  The HashSet is only
                // the membership test; iterating the rules carries the order.
                let distinct: Vec<Call> = {
                    let mut seen = std::collections::HashSet::new();
                    rebids
                        .rules()
                        .iter()
                        .filter_map(|rule| seen.insert(rule.call()).then_some(rule.call()))
                        .collect()
                };

                entries.extend(rows_of(Pattern::node(&prefix), rebids));
                for opener_bid in distinct {
                    let response = format!("{prefix} {opener_bid} -");
                    entries.extend(rows_of(
                        Pattern::node(&response),
                        responder_after_jacoby(major, opener_bid),
                    ));
                    entries.extend(slam::rkcb_rows(&response, major));
                }
            }
            entries
        },
    }
}
