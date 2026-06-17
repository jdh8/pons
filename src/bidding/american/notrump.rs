//! Notrump response structures for the 2/1 game-forcing system
//!
//! This module centralises every notrump continuation:
//!
//! - Responses to a **1NT** opening (Stayman 2♣, Jacoby transfers 2♦/2♥,
//!   notrump raises, and the quantitative 4NT invite).
//! - Responses to a **2NT-strength** notrump — both the direct 2NT opening
//!   (20–21 balanced) and opener's 2NT rebid after a 2♣ opening (22–24
//!   balanced): 3-level Stayman / transfers and the quantitative 4NT.
//! - Simple continuations after opener's **18–19 2NT rebid** over a one-level
//!   new-suit response.
//!
//! The public surface is [`register`], called once by
//! [`american`][super::american] during system assembly.

use super::{call, insert_uncontested};
use crate::bidding::constraint::{Cons, Constraint, balanced, described, hcp, len};
use crate::bidding::{Context, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Rank, Strain, Suit};

// ---------------------------------------------------------------------------
// 1NT response structure
// ---------------------------------------------------------------------------

/// Responses to our 1NT opening: Stayman, Jacoby transfers, and notrump raises
///
/// Stayman requires at least invitational values (8+ HCP) and a four-card
/// major; transfers apply at any strength; the quantitative 4NT invites
/// slam opposite 15–17 with a balanced 16–17 HCP hand and no four-card major.
#[must_use]
pub fn notrump_responses() -> Rules {
    Rules::new()
        // Jacoby transfers, any strength.
        .rule(Bid::new(2, Strain::Diamonds), 2.0, len(Suit::Hearts, 5..))
        .rule(Bid::new(2, Strain::Hearts), 2.0, len(Suit::Spades, 5..))
        // Stayman: a four-card major and at least invitational values.
        .rule(
            Bid::new(2, Strain::Clubs),
            1.5,
            (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4)) & hcp(8..),
        )
        // Quantitative 4NT slam invite (balanced, no four-card major).
        .rule(
            Bid::new(4, Strain::Notrump),
            1.2,
            hcp(16..=17) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        // Natural notrump raises (no five-card major — that would transfer).
        // Game-force with 9+, invite with a bare 8.  Forcing every 9 (rather
        // than inviting 8–9 and forcing 10+) is A/B-verified worth ≈+1 IMP per
        // divergent board vul none and ≈+3 vul both: opposite a 15–17 opener a 9
        // makes game often enough that the invitational stop loses more by missing
        // games (opener declining with a useful minimum) than it gains.  Deciding
        // the 9 by Fifths instead was measured *worse* — even quack-heavy 9s are
        // worth forcing, so selectivity just leaves games unbid.  3NT is
        // open-ended: a strong balanced hand bids game and leaves slam exploration
        // to a later pass.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(9..) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            1.0,
            hcp(8..=8) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(
            Call::Pass,
            0.0,
            hcp(..8) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
}

/// Opener's answer to Stayman: a four-card major, else 2♦
fn stayman_answers() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 1.0, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(2, Strain::Spades),
            1.0,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            0.5,
            len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
}

/// Complete a Jacoby transfer by bidding the anchor suit
fn complete_transfer(into: Suit) -> Rules {
    Rules::new().rule(Bid::new(2, Strain::from(into)), 1.0, hcp(0..))
}

/// The other major
const fn other_major(major: Suit) -> Suit {
    match major {
        Suit::Hearts => Suit::Spades,
        _ => Suit::Hearts,
    }
}

/// Opener holds a first- or second-round honour control (an ace or king) in `suit`
fn control_in(suit: Suit) -> Cons<impl Constraint + Clone> {
    // ponytail: A/K only — ignores shortness controls (void/singleton).  A full
    // cue scheme would add them, but a balanced 1NT opener rarely holds one.
    described(
        format!("control in {suit}"),
        move |hand: Hand, _: &Context<'_>| {
            let holding = hand[suit];
            holding.contains(Rank::A) || holding.contains(Rank::K)
        },
    )
}

/// Responder's rebid after opener answers Stayman with a major (`2♥`/`2♠`)
///
/// With a fit (four cards in opener's `major`): an invitational raise (`3M`),
/// game (`4M`), or — balanced, or slam-interested — the *other* major (`3OM`) as
/// an artificial slam try / choice of game.  Without a fit, the auction reverts
/// to notrump exactly as over a bare 1NT — invite `2NT`, game `3NT`, and the
/// quantitative `4NT` (16–17) — "ignore the 2♣ detour".
fn stayman_major_rebid(major: Suit) -> Rules {
    let other = Strain::from(other_major(major));
    let strain = Strain::from(major);
    Rules::new()
        // Fit: artificial slam try / choice of game (balanced, or 16+).
        .rule(
            Bid::new(3, other),
            1.4,
            len(major, 4..) & hcp(9..) & (balanced() | hcp(16..)),
        )
        // Fit: sign off in the major game.
        .rule(Bid::new(4, strain), 1.3, len(major, 4..) & hcp(9..))
        // Fit: invitational raise.
        .rule(Bid::new(3, strain), 1.2, len(major, 4..) & hcp(8..=8))
        // No fit: quantitative 4NT (as if the 2♣ detour never happened).
        .rule(
            Bid::new(4, Strain::Notrump),
            1.2,
            len(major, ..4) & hcp(16..=17),
        )
        // No fit: game / invitational notrump raise.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            len(major, ..4) & hcp(9..),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            1.0,
            len(major, ..4) & hcp(8..=8),
        )
}

/// A flat 4-3-3-3 — the one balanced shape with no doubleton
fn flat_4333() -> Cons<impl Constraint + Clone> {
    balanced()
        & len(Suit::Clubs, 3..)
        & len(Suit::Diamonds, 3..)
        & len(Suit::Hearts, 3..)
        & len(Suit::Spades, 3..)
}

/// Opener's reply to responder's `3OM` slam try / choice of game
///
/// A flat `(4333)` chooses notrump (`3NT`); a maximum (17) cue-bids the cheapest
/// honour control to cooperate; otherwise opener signs off in the major game.
fn stayman_slam_try_answer(major: Suit) -> Rules {
    let mut rules = Rules::new().rule(Bid::new(3, Strain::Notrump), 1.4, flat_4333());
    // Cheapest control cue with a maximum: each suit ranking below the major.
    let mut weight = 1.3;
    for cue in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        if Strain::from(cue) < Strain::from(major) {
            rules = rules.rule(
                Bid::new(4, Strain::from(cue)),
                weight,
                hcp(17..) & control_in(cue),
            );
            weight -= 0.05;
        }
    }
    // Minimum, or a maximum without a cheap control: sign off in game.
    rules.rule(Bid::new(4, Strain::from(major)), 1.0, hcp(0..))
}

/// Responder's rebid after opener denies a major (`1NT–2♣–2♦`)
///
/// Smolen: jump in the four-card major to show *five* in the other, game-forcing,
/// so the strong notrump hand declares.  Lacking 5–4, revert to notrump as if the
/// 2♣ detour never happened — invite `2NT`, game `3NT`, quantitative `4NT`.
fn stayman_no_major_rebid() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Hearts),
            1.4,
            len(Suit::Hearts, 4..=4) & len(Suit::Spades, 5..) & hcp(10..),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            1.4,
            len(Suit::Spades, 4..=4) & len(Suit::Hearts, 5..) & hcp(10..),
        )
        .rule(Bid::new(4, Strain::Notrump), 1.2, hcp(16..=17))
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(9..))
        .rule(Bid::new(2, Strain::Notrump), 1.0, hcp(8..=8))
}

/// Opener completes Smolen by bidding game in responder's shown five-card major
fn smolen_completion(five_card: Suit) -> Rules {
    let strain = Strain::from(five_card);
    Rules::new()
        // Eight-card fit: bid game in the long major so opener declares.
        .rule(Bid::new(4, strain), 1.0, len(five_card, 3..))
        // No fit: notrump game.
        .rule(Bid::new(3, Strain::Notrump), 0.5, len(five_card, ..3))
}

/// Smolen at the three level: responder's jump after opener denies a major
/// (`…3♣–3♦`).  Game is already forced, so no strength gate.
fn smolen_at_three() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Hearts),
            1.4,
            len(Suit::Hearts, 4..=4) & len(Suit::Spades, 5..),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            1.4,
            len(Suit::Spades, 4..=4) & len(Suit::Hearts, 5..),
        )
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(0..))
}

/// Opener accepts a no-fit (2NT) Stayman invitation with a maximum, else passes
///
/// Responder invited with a bare 8, so a 1NT opener needs its 17 for game (8+17
/// = 25).  Authored rather than left to the floor: the keyless floor reads a
/// three-level suit response over our 1NT as forcing and so cannot *decline* an
/// invitational raise.
fn accept_invitation(game: Bid) -> Rules {
    Rules::new()
        .rule(game, 1.0, hcp(17..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's acceptance of an invitational major raise
///
/// With a maximum, bid the major game — but choose 3NT on a flat 4-3-3-3, where
/// notrump rates to play as well as the eight-card fit.  A minimum passes.
fn accept_major_invitation(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.1, hcp(17..) & flat_4333())
        .rule(Bid::new(4, Strain::from(major)), 1.0, hcp(17..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// 2NT-strength response structure (2NT opening and 2♣–2x–2NT rebid)
// ---------------------------------------------------------------------------

/// Responses to a 2NT-strength notrump (3-level Stayman/transfers, 4NT invite)
///
/// Used after both the direct 2NT opening (20–21 balanced) and opener's 2NT
/// rebid after 2♣ (22–24 balanced).
fn two_notrump_responses() -> Rules {
    Rules::new()
        // 3-level Jacoby transfers.
        .rule(Bid::new(3, Strain::Diamonds), 2.0, len(Suit::Hearts, 5..))
        .rule(Bid::new(3, Strain::Hearts), 2.0, len(Suit::Spades, 5..))
        // 3-level Stayman: a four-card major and at least some values.
        .rule(
            Bid::new(3, Strain::Clubs),
            1.5,
            (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4)) & hcp(5..),
        )
        // Quantitative 4NT slam invite (balanced, no four-card major).
        .rule(
            Bid::new(4, Strain::Notrump),
            1.2,
            hcp(11..=12) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        // 3NT to play: game values, no major fit.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(5..=10) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(Call::Pass, 0.0, hcp(..5))
}

/// Opener's answer to 3-level Stayman: a four-card major, else 3♦
fn stayman_answers_at_three() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 1.0, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(3, Strain::Spades),
            1.0,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            0.5,
            len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
}

/// Complete a 3-level transfer by bidding the anchor suit
fn complete_transfer_at_three(into: Suit) -> Rules {
    Rules::new().rule(Bid::new(3, Strain::from(into)), 1.0, hcp(0..))
}

/// Opener's answer to the quantitative 4NT: accept or decline the slam invite
///
/// `accept_hcp` is the minimum HCP to accept: 21 after a 2NT opening (20–21),
/// 24 after a 2♣–2x–2NT sequence (22–24).
fn quantitative_answer(accept_hcp: u8) -> Rules {
    Rules::new()
        .rule(Bid::new(6, Strain::Notrump), 1.0, hcp(accept_hcp..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Simple continuations after an 18–19 2NT rebid
// ---------------------------------------------------------------------------

/// Responder's call after opener's 18–19 2NT rebid
///
/// 6+ HCP bids 3NT; 12–13 makes a quantitative 4NT invite; fewer points pass.
fn after_rebid_two_notrump() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.2, hcp(12..=13))
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(6..))
        .rule(Call::Pass, 0.0, hcp(..6))
}

/// Opener's reply to the quantitative raise opposite the 18–19 rebid
///
/// Accept (6NT) with a maximum 19 HCP, decline (pass) otherwise.
fn accept_quantitative_nineteen() -> Rules {
    Rules::new()
        .rule(Bid::new(6, Strain::Notrump), 1.0, hcp(19..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register all notrump continuations into the constructive book
///
/// Registers the 1NT structure (Stayman, transfers, 4NT quantitative), the
/// 2NT-strength structure (3-level Stayman/transfers, 4NT invite) under three
/// base prefixes (direct 2NT opening and the two 2♣–2x–2NT auctions), and
/// simple responses after opener's 18–19 2NT rebid.
pub(super) fn register(book: &mut Trie) {
    register_one_nt(book);
    register_two_nt_and_rebids(book);
}

/// Register the standard 1NT-opening response structure
///
/// Stayman 2♣, Jacoby transfers 2♦/2♥, notrump raises, and the quantitative
/// 4NT invite — the baseline 2/1 treatment.  Factored from the
/// 2NT-strength/18–19-rebid block ([`register_two_nt_and_rebids`]) so an
/// alternative 1NT scheme could replace just this part.
pub(super) fn register_one_nt(book: &mut Trie) {
    let one_nt = call(1, Strain::Notrump);
    let four_nt = call(4, Strain::Notrump);

    let two_c = call(2, Strain::Clubs);
    let two_d = call(2, Strain::Diamonds);
    let two_h = call(2, Strain::Hearts);
    let two_s = call(2, Strain::Spades);
    let three_h = call(3, Strain::Hearts);
    let three_s = call(3, Strain::Spades);

    insert_uncontested(book, &[one_nt], notrump_responses());
    // Stayman answers and transfer completions.
    insert_uncontested(book, &[one_nt, two_c], stayman_answers());
    insert_uncontested(book, &[one_nt, two_d], complete_transfer(Suit::Hearts));
    insert_uncontested(book, &[one_nt, two_h], complete_transfer(Suit::Spades));
    // Quantitative 4NT answer.
    insert_uncontested(book, &[one_nt, four_nt], quantitative_answer(17));

    // --- Stayman continuations ------------------------------------------------
    //
    // Responder's rebid after opener shows a major, and opener's reply to the
    // artificial 3OM slam try.
    insert_uncontested(
        book,
        &[one_nt, two_c, two_h],
        stayman_major_rebid(Suit::Hearts),
    );
    insert_uncontested(
        book,
        &[one_nt, two_c, two_s],
        stayman_major_rebid(Suit::Spades),
    );
    insert_uncontested(
        book,
        &[one_nt, two_c, two_h, three_s],
        stayman_slam_try_answer(Suit::Hearts),
    );
    insert_uncontested(
        book,
        &[one_nt, two_c, two_s, three_h],
        stayman_slam_try_answer(Suit::Spades),
    );
    // Responder's rebid after opener denies a major (Smolen, else revert to NT),
    // and opener's Smolen completion (game in responder's five-card major).
    insert_uncontested(book, &[one_nt, two_c, two_d], stayman_no_major_rebid());
    insert_uncontested(
        book,
        &[one_nt, two_c, two_d, three_h],
        smolen_completion(Suit::Spades),
    );
    insert_uncontested(
        book,
        &[one_nt, two_c, two_d, three_s],
        smolen_completion(Suit::Hearts),
    );
    // Opener accepts or declines responder's invitation (major raise, or the
    // no-fit 2NT) — authored, since the floor reads the three-level raise as
    // forcing and could not decline.
    let two_nt = call(2, Strain::Notrump);
    insert_uncontested(
        book,
        &[one_nt, two_c, two_h, three_h],
        accept_major_invitation(Suit::Hearts),
    );
    insert_uncontested(
        book,
        &[one_nt, two_c, two_s, three_s],
        accept_major_invitation(Suit::Spades),
    );
    for major_answer in [two_h, two_s, two_d] {
        insert_uncontested(
            book,
            &[one_nt, two_c, major_answer, two_nt],
            accept_invitation(Bid::new(3, Strain::Notrump)),
        );
    }
    // Opener's quantitative accept after a no-fit revert to 4NT.
    insert_uncontested(
        book,
        &[one_nt, two_c, two_h, four_nt],
        quantitative_answer(17),
    );
    insert_uncontested(
        book,
        &[one_nt, two_c, two_s, four_nt],
        quantitative_answer(17),
    );
    insert_uncontested(
        book,
        &[one_nt, two_c, two_d, four_nt],
        quantitative_answer(17),
    );
}

/// Register the 2NT-strength structure and the 18–19 2NT-rebid continuations
///
/// The half of the notrump book that an alternative 1NT-opening scheme would
/// keep unchanged — only [`register_one_nt`] varies.
pub(super) fn register_two_nt_and_rebids(book: &mut Trie) {
    let one_nt = call(1, Strain::Notrump);
    let two_nt = call(2, Strain::Notrump);
    let four_nt = call(4, Strain::Notrump);

    // --- 2NT-strength structure ----------------------------------------------
    //
    // Three base prefixes (our calls only; passes are interleaved by
    // `insert_uncontested`):
    //   1. [2NT]                  → direct 2NT opening (20–21), accept_hcp = 21
    //   2. [2♣, 2♦, 2NT]         → 2♣–2♦–2NT sequence (22–24), accept_hcp = 24
    //   3. [2♣, 2♥, 2NT]         → 2♣–2♥–2NT sequence (22–24), accept_hcp = 24

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

    for (base, accept_hcp) in bases {
        // Responses to the 2NT bid.
        insert_uncontested(book, base, two_notrump_responses());

        // Stayman answers and transfer completions at the three level.
        let extend = |tail: Call| -> Vec<Call> {
            base.iter().copied().chain(core::iter::once(tail)).collect()
        };
        insert_uncontested(
            book,
            &extend(call(3, Strain::Clubs)),
            stayman_answers_at_three(),
        );
        insert_uncontested(
            book,
            &extend(call(3, Strain::Diamonds)),
            complete_transfer_at_three(Suit::Hearts),
        );
        insert_uncontested(
            book,
            &extend(call(3, Strain::Hearts)),
            complete_transfer_at_three(Suit::Spades),
        );

        // Quantitative 4NT answer.
        insert_uncontested(book, &extend(four_nt), quantitative_answer(*accept_hcp));

        // Smolen after 3♣ Stayman when opener denies a major (3♦): responder
        // jumps to show 5–4 in the majors, opener completes to game in the long
        // one.  Mirrors the 1NT-level structure one level up.
        let extend2 =
            |a: Call, b: Call| -> Vec<Call> { base.iter().copied().chain([a, b]).collect() };
        let extend3 = |a: Call, b: Call, c: Call| -> Vec<Call> {
            base.iter().copied().chain([a, b, c]).collect()
        };
        let (three_c, three_d) = (call(3, Strain::Clubs), call(3, Strain::Diamonds));
        let (three_h, three_s) = (call(3, Strain::Hearts), call(3, Strain::Spades));
        insert_uncontested(book, &extend2(three_c, three_d), smolen_at_three());
        insert_uncontested(
            book,
            &extend3(three_c, three_d, three_h),
            smolen_completion(Suit::Spades),
        );
        insert_uncontested(
            book,
            &extend3(three_c, three_d, three_s),
            smolen_completion(Suit::Hearts),
        );
    }

    // --- 18–19 2NT rebid continuations --------------------------------------
    //
    // The auctions where opener's existing rebid table carries 2NT = 18–19.
    // Each prefix is [opener's opening, responder's first call] — our side's
    // two calls that precede the rebid.

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

    for prefix in rebid_prefixes {
        // Responder's action over opener's 2NT rebid.
        let mut our = prefix.to_vec();
        our.push(two_nt);
        insert_uncontested(book, &our, after_rebid_two_notrump());

        // Opener's reply to the quantitative 4NT raise.
        our.push(four_nt);
        insert_uncontested(book, &our, accept_quantitative_nineteen());
    }
}
