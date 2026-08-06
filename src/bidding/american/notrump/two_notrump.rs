//! The `2NT`-strength structures — the `2NT` opening, the `2♣` rebid, and `18–19`
//!
//! Three-level Stayman and transfers with the quantitative `4NT`, shared by the
//! direct `2NT` opening (20–21) and opener's `2NT` rebid after `2♣` (22–24);
//! plus the simple continuations after an 18–19 `2NT` rebid over a one-level
//! response.

use super::stayman::{smolen_at_three, smolen_completion};
use super::transfers::transfer_longer_major;
use super::*;

/// Responses to a 2NT-strength notrump (3-level Stayman/transfers, 4NT invite)
///
/// Used after both the direct 2NT opening (20–21 balanced) and opener's 2NT
/// rebid after 2♣ (22–24 balanced).
fn two_notrump_responses() -> Rules {
    // The longer-major discipline (see [`set_transfer_longer_major`]): a
    // two-suiter transfers to the longer major, equal lengths to hearts —
    // there is no both-majors bid or slam reroute at this level, so hearts
    // takes every tie.  Off, the old guards tie at 2.0 and the pick between
    // the transfers is arbitrary (a weak 6♠5♥ could transfer to hearts and
    // scramble — the M6.4 A/B caught exactly that board).
    let prefer_longer = transfer_longer_major();
    Rules::new()
        // 3-level Jacoby transfers.
        .rule(
            Bid::new(3, Strain::Diamonds),
            200,
            len(Suit::Hearts, 5..)
                & described(
                    "hearts not outnumbered (longer-major discipline)",
                    move |hand: Hand, _: &Context<'_>| {
                        !prefer_longer || hand[Suit::Hearts].len() >= hand[Suit::Spades].len()
                    },
                ),
        )
        .alert(JACOBY)
        .rule(
            Bid::new(3, Strain::Hearts),
            200,
            len(Suit::Spades, 5..)
                & described(
                    "spades longer (longer-major discipline)",
                    move |hand: Hand, _: &Context<'_>| {
                        !prefer_longer || hand[Suit::Spades].len() > hand[Suit::Hearts].len()
                    },
                ),
        )
        .alert(JACOBY)
        // 3-level Stayman: a four-card major and at least some values, but never a
        // flat 4-3-3-3 (it bids notrump directly, as over a 1NT opening).
        .rule(
            Bid::new(3, Strain::Clubs),
            150,
            (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4)) & hcp(5..) & !flat_4333(),
        )
        // Quantitative 4NT slam invite (balanced, no four-card major).
        .rule(
            Bid::new(4, Strain::Notrump),
            120,
            hcp(11..=12) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        // 3NT to play: game values, no major fit.
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            hcp(5..=10) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(Call::Pass, 0, hcp(..5))
}

/// Opener's answer to 3-level Stayman: a four-card major, else 3♦
fn stayman_answers_at_three() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(3, Strain::Spades),
            100,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            50,
            len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
}

/// Complete a 3-level transfer by bidding the anchor suit
fn complete_transfer_at_three(into: Suit) -> Rules {
    Rules::new().rule(Bid::new(3, Strain::from(into)), 100, hcp(0..))
}

/// Opener's answer to the quantitative 4NT: accept or decline the slam invite
///
/// `accept_hcp` is the minimum HCP to accept: 21 after a 2NT opening (20–21),
/// 24 after a 2♣–2x–2NT sequence (22–24).
pub(super) fn quantitative_answer(accept_hcp: u8) -> Rules {
    Rules::new()
        .rule(Bid::new(6, Strain::Notrump), 100, hcp(accept_hcp..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's call after opener's 18–19 2NT rebid
///
/// 6+ HCP bids 3NT; 12–13 makes a quantitative 4NT invite; fewer points pass.
fn after_rebid_two_notrump() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 120, hcp(12..=13))
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(6..))
        .rule(Call::Pass, 0, hcp(..6))
}

/// Opener's reply to the quantitative raise opposite the 18–19 rebid
///
/// Accept (6NT) with a maximum 19 HCP, decline (pass) otherwise.
fn accept_quantitative_nineteen() -> Rules {
    Rules::new()
        .rule(Bid::new(6, Strain::Notrump), 100, hcp(19..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responses and continuations shared by the three 2NT-strength sequences
pub(crate) fn two_notrump_structure() -> Package {
    Package {
        name: "two-notrump-structure",
        gate: || true,
        entries: || {
            let two_nt = call(2, Strain::Notrump);
            let four_nt = call(4, Strain::Notrump);
            let bases: &[(&[Call], u8)] = &[
                (&[two_nt], 21),
                (
                    &[call(2, Strain::Clubs), call(2, Strain::Diamonds), two_nt],
                    24,
                ),
                (
                    &[call(2, Strain::Clubs), call(2, Strain::Hearts), two_nt],
                    24,
                ),
            ];
            let mut entries = Vec::new();

            for (base, accept_hcp) in bases {
                let prefix = core::iter::once("P*".to_owned())
                    .chain(base.iter().map(|call| format!("{call} (P)")))
                    .collect::<Vec<_>>()
                    .join(" ");

                // Responses to the 2NT bid.
                entries.extend(rows_of(Pattern::node(&prefix), two_notrump_responses()));

                // Stayman answers and transfer completions at the three level.
                let extend = |tail: Call| format!("{prefix} {tail} (P)");
                entries.extend(rows_of(
                    Pattern::node(&extend(call(3, Strain::Clubs))),
                    stayman_answers_at_three(),
                ));
                entries.extend(rows_of(
                    Pattern::node(&extend(call(3, Strain::Diamonds))),
                    complete_transfer_at_three(Suit::Hearts),
                ));
                entries.extend(rows_of(
                    Pattern::node(&extend(call(3, Strain::Hearts))),
                    complete_transfer_at_three(Suit::Spades),
                ));

                // Quantitative 4NT answer.
                entries.extend(rows_of(
                    Pattern::node(&extend(four_nt)),
                    quantitative_answer(*accept_hcp),
                ));

                // Smolen after 3♣ Stayman when opener denies a major (3♦):
                // responder jumps to show 5–4 in the majors, opener completes
                // to game in the long one.
                let extend2 = |a: Call, b: Call| format!("{prefix} {a} (P) {b} (P)");
                let extend3 =
                    |a: Call, b: Call, c: Call| format!("{prefix} {a} (P) {b} (P) {c} (P)");
                let (three_c, three_d) = (call(3, Strain::Clubs), call(3, Strain::Diamonds));
                let (three_h, three_s) = (call(3, Strain::Hearts), call(3, Strain::Spades));
                entries.extend(rows_of(
                    Pattern::node(&extend2(three_c, three_d)),
                    smolen_at_three(),
                ));
                entries.extend(rows_of(
                    Pattern::node(&extend3(three_c, three_d, three_h)),
                    smolen_completion(Suit::Spades),
                ));
                entries.extend(rows_of(
                    Pattern::node(&extend3(three_c, three_d, three_s)),
                    smolen_completion(Suit::Hearts),
                ));
            }

            entries
        },
    }
}

/// Continuations after opener's 18–19 2NT rebid
pub(crate) fn two_notrump_rebids() -> Package {
    Package {
        name: "two-notrump-rebids",
        gate: || true,
        entries: || {
            let one_nt = call(1, Strain::Notrump);
            let two_nt = call(2, Strain::Notrump);
            let four_nt = call(4, Strain::Notrump);
            let rebid_prefixes: &[&[Call]] = &[
                &[call(1, Strain::Hearts), call(1, Strain::Spades)],
                &[call(1, Strain::Clubs), call(1, Strain::Diamonds)],
                &[call(1, Strain::Clubs), call(1, Strain::Hearts)],
                &[call(1, Strain::Clubs), call(1, Strain::Spades)],
                &[call(1, Strain::Diamonds), call(1, Strain::Hearts)],
                &[call(1, Strain::Diamonds), call(1, Strain::Spades)],
                &[call(1, Strain::Hearts), one_nt],
                &[call(1, Strain::Spades), one_nt],
            ];
            let mut entries = Vec::new();

            for prefix in rebid_prefixes {
                let prefix = core::iter::once("P*".to_owned())
                    .chain(prefix.iter().map(|call| format!("{call} (P)")))
                    .collect::<Vec<_>>()
                    .join(" ");

                // Responder's action over opener's 2NT rebid.
                let two_nt_rebid = format!("{prefix} {two_nt} (P)");
                entries.extend(rows_of(
                    Pattern::node(&two_nt_rebid),
                    after_rebid_two_notrump(),
                ));

                // Opener's reply to the quantitative 4NT raise.
                let quantitative_raise = format!("{two_nt_rebid} {four_nt} (P)");
                entries.extend(rows_of(
                    Pattern::node(&quantitative_raise),
                    accept_quantitative_nineteen(),
                ));
            }

            entries
        },
    }
}
