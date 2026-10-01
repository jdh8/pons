//! The `2NT`-strength structures — the `2NT` opening, the `2♣` rebid, and `18–19`
//!
//! Three-level Stayman and transfers with the quantitative `4NT`, shared by the
//! direct `2NT` opening (20–21) and opener's `2NT` rebid after `2♣` (22–24);
//! plus the simple continuations after an 18–19 `2NT` rebid over a one-level
//! response — and, under `notrump.rebid_checkback`, BBA's structure over the
//! four `1m - 1M - 2NT` lanes: three of the new minor as an alerted checkback
//! (five of the major, 8+), the forcing six-card `3M`, and `3♠` over `1♥` as
//! four spades.

use super::stayman::{smolen_at_three, smolen_completion};
use super::*;
use crate::bidding::rows::Entry;

/// Responses to a 2NT-strength notrump (3-level Stayman/transfers, 4NT invite)
///
/// Used after both the direct 2NT opening (20–21 balanced) and opener's 2NT
/// rebid after 2♣ (22–24 balanced).  `shift` lowers every HCP band by that
/// much: 0 over the opening, 2 over the `2♣` rebid under
/// `notrump.strong_two_notrump_floors`.
fn two_notrump_responses(agreements: &Agreements, shift: u8) -> Rules {
    // The longer-major discipline (see `notrump.transfer_longer_major`): a
    // two-suiter transfers to the longer major, equal lengths to hearts —
    // there is no both-majors bid or slam reroute at this level, so hearts
    // takes every tie.  Off, the old guards tie at 2.0 and the pick between
    // the transfers is arbitrary (a weak 6♠5♥ could transfer to hearts and
    // scramble — the M6.4 A/B caught exactly that board).
    let prefer_longer = agreements.notrump.transfer_longer_major;
    let rules = Rules::new()
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
            (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4)) & hcp(5 - shift..) & !flat_4333(),
        )
        // Quantitative 4NT slam invite (balanced, no four-card major).
        .rule(
            Bid::new(4, Strain::Notrump),
            120,
            hcp(11 - shift..=12 - shift) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        // 3NT to play: game values, no major fit.
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            hcp(5 - shift..=10 - shift) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(Call::Pass, 0, hcp(..5 - shift));
    // 13–16 only: the table's 17+ hole is deliberate, the floor bids the
    // grand there (the A/B's two −13 boards were its 7NT overridden).
    if agreements.notrump.quantitative_six_notrump {
        rules.rule(
            Bid::new(6, Strain::Notrump),
            120,
            hcp(13 - shift..=16 - shift) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
    } else {
        rules
    }
}

/// The direct `6NT` above a quantitative `4NT` (`notrump.quantitative_six_notrump`)
fn six_notrump(
    rules: Rules,
    agreements: &Agreements,
    floor: u8,
    shape: Cons<impl Constraint + 'static>,
) -> Rules {
    if agreements.notrump.quantitative_six_notrump {
        rules.rule(Bid::new(6, Strain::Notrump), 120, hcp(floor..) & shape)
    } else {
        rules
    }
}

/// Responder's quantitative pair once 2NT-strength Stayman finds no major
/// (`2NT - 3♣ - 3♦`): the same `4NT` / `6NT` the direct responses carry,
/// lowered by the same `shift`
fn quantitative_after_stayman_denial(agreements: &Agreements, shift: u8) -> Rules {
    if agreements.notrump.quantitative_six_notrump {
        Rules::new()
            .rule(
                Bid::new(4, Strain::Notrump),
                120,
                hcp(11 - shift..=12 - shift),
            )
            .rule(Bid::new(6, Strain::Notrump), 120, hcp(13 - shift..))
    } else {
        Rules::new()
    }
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
/// 24 after a 2♣ - 2x - 2NT sequence (22–24).
pub(super) fn quantitative_answer(accept_hcp: u8) -> Rules {
    Rules::new()
        .rule(Bid::new(6, Strain::Notrump), 100, hcp(accept_hcp..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's call after opener's 18–19 2NT rebid
///
/// 6+ HCP bids 3NT; 12–13 makes a quantitative 4NT invite; fewer points pass;
/// 14+ bids `6NT` under `notrump.quantitative_six_notrump`.  On a minor–major
/// `lane` under `notrump.rebid_checkback`, the checkback and forcing majors
/// of [`rebid_checkback_responder`] sit above all of these.
fn after_rebid_two_notrump(agreements: &Agreements, lane: Option<(Suit, Suit)>) -> Rules {
    let rules = Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 120, hcp(12..=13))
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(6..))
        .rule(Call::Pass, 0, hcp(..6));
    let rules = six_notrump(rules, agreements, 14, hcp(0..));
    match lane {
        Some((opening, major)) if agreements.notrump.rebid_checkback => {
            rebid_checkback_responder(rules, opening, major)
        }
        _ => rules,
    }
}

/// Three of the new minor over the 18–19 `2NT` rebid: exactly five of the
/// major, 8+ — asks for three-card support or the other four-card major
const REBID_CHECKBACK: Alert = Alert("rebid-checkback");

/// The minor opener did *not* open — the checkback bids three of it
fn new_minor(opening: Suit) -> Suit {
    match opening {
        Suit::Clubs => Suit::Diamonds,
        _ => Suit::Clubs,
    }
}

/// Responder's checkback and forcing majors over `1m - 1M - 2NT`
/// (`notrump.rebid_checkback`), above the notrump ladder
///
/// | Call | Wt | Meaning |
/// |------|----|---------|
/// | 3M | 1.50 | Six-plus, forcing — opener is balanced, so always raised |
/// | 3(new minor)! | 1.40 | Checkback: exactly five of the major, 8+ |
/// | 3♠ (over 1♥) | 1.35 | Four spades, at most four hearts, 7+ — `2NT` did not deny them |
///
/// A 14+ hand with a five-card major checks back before it bids `6NT`; the
/// slam calls come after opener's answer.  Weak six-carders use `points`, so
/// a 5-count with a good suit still reaches the 6-2 game.
fn rebid_checkback_responder(rules: Rules, opening: Suit, major: Suit) -> Rules {
    let rules = rules
        .rule(
            Bid::new(3, Strain::from(major)),
            150,
            len(major, 6..) & points(6..),
        )
        .rule(
            Bid::new(3, Strain::from(new_minor(opening))),
            140,
            len(major, 5..=5) & hcp(8..),
        )
        .alert(REBID_CHECKBACK);
    if major == Suit::Hearts {
        rules.rule(
            Bid::new(3, Strain::Spades),
            135,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..=4) & hcp(7..),
        )
    } else {
        rules
    }
}

/// Opener's answer to the checkback: three-card support, the other four-card
/// major, else `3NT` — all natural, and the table never passes the force
fn rebid_checkback_answers(major: Suit) -> Rules {
    let other = other_major(major);
    Rules::new()
        .rule(Bid::new(3, Strain::from(major)), 130, len(major, 3..))
        .rule(
            Bid::new(3, Strain::from(other)),
            125,
            len(other, 4..) & len(major, ..=2),
        )
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
}

/// Responder places the contract once opener shows three-card support: the
/// major game, or RKCB with 14+ (32+ combined and a 5-3 fit)
fn after_checkback_fit(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 120, hcp(14..))
        .alert(slam::RKCB)
        .rule(Bid::new(4, Strain::from(major)), 100, hcp(0..))
}

/// Responder after opener shows the other four-card major: raise it with
/// four, else the notrump ladder (`4NT` 12–13, `6NT` 14+, `3NT`)
fn after_checkback_other_major(other: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(other)), 120, len(other, 4..))
        .rule(Bid::new(4, Strain::Notrump), 110, hcp(12..=13))
        .rule(Bid::new(6, Strain::Notrump), 110, hcp(14..))
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
}

/// Responder after opener denies a fit with `3NT`: the quantitative pair, or pass
fn after_checkback_denial() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 120, hcp(12..=13))
        .rule(Bid::new(6, Strain::Notrump), 120, hcp(14..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder after the forcing `3M` is raised: RKCB with 12+ (a 6-2 fit
/// opposite 18–19), else the game stands
fn after_six_card_raise() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 120, hcp(12..))
        .alert(slam::RKCB)
        .rule(Call::Pass, 0, hcp(0..))
}

/// Every node below responder's checkback, forcing `3M` and (over `1♥`) `3♠`
///
/// `two_nt_rebid` is the row prefix ending in opener's `2NT` and the pass
/// (`P* 1♦ - 1♠ - 2NT -`).  The contested tails (they double the checkback)
/// are left to reading: the ask projects the major and no minor, so the floor
/// holds no phantom suit — the XYZ precedent.
fn rebid_checkback_rows(two_nt_rebid: &str, opening: Suit, major: Suit) -> Vec<Entry> {
    let trump = Strain::from(major);
    let other = other_major(major);
    let node = |tail: &str| Pattern::node(&format!("{two_nt_rebid} {tail}"));
    let four_nt = call(4, Strain::Notrump);

    // The checkback and opener's three answers.
    let ask = format!("{} -", call(3, Strain::from(new_minor(opening))));
    let mut entries = rows_of(node(&ask), rebid_checkback_answers(major));
    let fit = format!("{ask} {} -", call(3, trump));
    entries.extend(rows_of(node(&fit), after_checkback_fit(major)));
    entries.extend(slam::rkcb_rows(&format!("{two_nt_rebid} {fit}"), major));
    entries.extend(slam::rkcb_answerer_rows(
        &format!("{two_nt_rebid} {fit}"),
        major,
    ));
    let shown = format!("{ask} {} -", call(3, Strain::from(other)));
    entries.extend(rows_of(node(&shown), after_checkback_other_major(other)));
    entries.extend(rows_of(
        node(&format!("{shown} {four_nt} -")),
        accept_quantitative_nineteen(),
    ));
    let denied = format!("{ask} {} -", call(3, Strain::Notrump));
    entries.extend(rows_of(node(&denied), after_checkback_denial()));
    entries.extend(rows_of(
        node(&format!("{denied} {four_nt} -")),
        accept_quantitative_nineteen(),
    ));

    // The forcing 3M: opener raises, responder keycards with 12+.
    let three_m = format!("{} -", call(3, trump));
    entries.extend(rows_of(
        node(&three_m),
        Rules::new().rule(Bid::new(4, trump), 100, hcp(0..)),
    ));
    let raised = format!("{three_m} {} -", call(4, trump));
    entries.extend(rows_of(node(&raised), after_six_card_raise()));
    entries.extend(slam::rkcb_rows(&format!("{two_nt_rebid} {raised}"), major));
    entries.extend(slam::rkcb_answerer_rows(
        &format!("{two_nt_rebid} {raised}"),
        major,
    ));

    // Over 1♥, responder's natural 3♠: opener raises with four, else 3NT.
    if major == Suit::Hearts {
        entries.extend(rows_of(
            node(&format!("{} -", call(3, Strain::Spades))),
            Rules::new()
                .rule(Bid::new(4, Strain::Spades), 120, len(Suit::Spades, 4..))
                .rule(Bid::new(3, Strain::Notrump), 100, hcp(0..)),
        ));
    }
    entries
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
        gate: |_| true,
        entries: |agreements| {
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
                // `2♣ - 2♥` is a positive, not the bust, under `strong_two_waiting`.
                if agreements.decision.strong_two_waiting
                    && base.get(1) == Some(&call(2, Strain::Hearts))
                {
                    continue;
                }
                let shift = if base.len() > 1 && agreements.notrump.strong_two_notrump_floors {
                    2
                } else {
                    0
                };
                let prefix = core::iter::once("P*".to_owned())
                    .chain(base.iter().map(|call| format!("{call} -")))
                    .collect::<Vec<_>>()
                    .join(" ");

                // Responses to the 2NT bid.
                entries.extend(rows_of(
                    Pattern::node(&prefix),
                    two_notrump_responses(agreements, shift),
                ));

                // Stayman answers and transfer completions at the three level.
                let extend = |tail: Call| format!("{prefix} {tail} -");
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
                let extend2 = |a: Call, b: Call| format!("{prefix} {a} - {b} -");
                let extend3 = |a: Call, b: Call, c: Call| format!("{prefix} {a} - {b} - {c} -");
                let (three_c, three_d) = (call(3, Strain::Clubs), call(3, Strain::Diamonds));
                let (three_h, three_s) = (call(3, Strain::Hearts), call(3, Strain::Spades));
                entries.extend(rows_of(
                    Pattern::node(&extend2(three_c, three_d)),
                    smolen_at_three(),
                ));
                entries.extend(rows_of(
                    Pattern::node(&extend2(three_c, three_d)),
                    quantitative_after_stayman_denial(agreements, shift),
                ));
                entries.extend(rows_of(
                    Pattern::node(&extend3(three_c, three_d, four_nt)),
                    quantitative_answer(*accept_hcp),
                ));
                entries.extend(rows_of(
                    Pattern::node(&extend3(three_c, three_d, three_h)),
                    smolen_completion(Suit::Spades, agreements),
                ));
                entries.extend(rows_of(
                    Pattern::node(&extend3(three_c, three_d, three_s)),
                    smolen_completion(Suit::Hearts, agreements),
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
        gate: |_| true,
        entries: |agreements| {
            let one_nt = call(1, Strain::Notrump);
            let two_nt = call(2, Strain::Notrump);
            let four_nt = call(4, Strain::Notrump);
            // The minor–major lanes carry the checkback (`notrump.rebid_checkback`).
            let lane = |opening: Suit, major: Suit| {
                (
                    [call(1, Strain::from(opening)), call(1, Strain::from(major))],
                    Some((opening, major)),
                )
            };
            let rebid_prefixes = [
                ([call(1, Strain::Hearts), call(1, Strain::Spades)], None),
                ([call(1, Strain::Clubs), call(1, Strain::Diamonds)], None),
                lane(Suit::Clubs, Suit::Hearts),
                lane(Suit::Clubs, Suit::Spades),
                lane(Suit::Diamonds, Suit::Hearts),
                lane(Suit::Diamonds, Suit::Spades),
                ([call(1, Strain::Hearts), one_nt], None),
                ([call(1, Strain::Spades), one_nt], None),
            ];
            let mut entries = Vec::new();

            for (prefix, lane) in &rebid_prefixes {
                let prefix = core::iter::once("P*".to_owned())
                    .chain(prefix.iter().map(|call| format!("{call} -")))
                    .collect::<Vec<_>>()
                    .join(" ");

                // Responder's action over opener's 2NT rebid.
                let two_nt_rebid = format!("{prefix} {two_nt} -");
                entries.extend(rows_of(
                    Pattern::node(&two_nt_rebid),
                    after_rebid_two_notrump(agreements, *lane),
                ));

                // Opener's reply to the quantitative 4NT raise.
                let quantitative_raise = format!("{two_nt_rebid} {four_nt} -");
                entries.extend(rows_of(
                    Pattern::node(&quantitative_raise),
                    accept_quantitative_nineteen(),
                ));

                if let Some((opening, major)) = *lane
                    && agreements.notrump.rebid_checkback
                {
                    entries.extend(rebid_checkback_rows(&two_nt_rebid, opening, major));
                }
            }

            entries
        },
    }
}
