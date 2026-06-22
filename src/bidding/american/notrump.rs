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

use super::{call, insert_uncontested, slam};
use crate::bidding::constraint::{
    Cons, Constraint, balanced, described, hcp, len, points, stopper_in,
};
use crate::bidding::{Context, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Rank, Strain, Suit};

// ---------------------------------------------------------------------------
// 1NT response structure
// ---------------------------------------------------------------------------

/// Responses to our 1NT opening: Stayman, Jacoby transfers, Puppet Stayman, the
/// minor-suit transfers, and notrump raises
///
/// Stayman (2♣) needs invitational+ values and a four-card major; Jacoby
/// transfers (2♦/2♥) a five-card major, any strength.  Puppet Stayman (3♣) is a
/// game-forcing balanced hand with a three-card major, hunting opener's five-card
/// major.  The minor transfers cover diamonds (2NT: 6+♦ or 5♦4♣) and clubs (2♠:
/// 6+♣, or — relocated from the old natural 2NT — a balanced invitational eight).
/// The quantitative 4NT invites slam opposite a balanced 16–17 with no four-card
/// major.
#[must_use]
pub fn notrump_responses() -> Rules {
    Rules::new()
        // Jacoby transfers — any strength, except a game-forcing 5-4 in the
        // majors (the `hcp(..9)` arm denies it): that hand keeps off the transfer
        // and takes the 2♣ Stayman/Smolen route, which right-sides game to the
        // strong notrump.  A plain 5-3 still transfers.
        .rule(
            Bid::new(2, Strain::Diamonds),
            2.0,
            len(Suit::Hearts, 5..) & (len(Suit::Spades, ..4) | hcp(..9)),
        )
        .rule(
            Bid::new(2, Strain::Hearts),
            2.0,
            len(Suit::Spades, 5..) & (len(Suit::Hearts, ..4) | hcp(..9)),
        )
        // Both-majors 3♦: 5+/5+ in the majors, invitational+.  Outranks the
        // transfers (2.0) so a 5-5 INV+ hand shows both suits in one bid rather
        // than transferring and rebidding; weaker 5-5s (below the `points` floor)
        // still take the transfer route.  `points` (not `hcp`) so the 5-5 shape
        // upgrade counts — these are the unbalanced hands the gauge was built for.
        .rule(
            Bid::new(3, Strain::Diamonds),
            2.1,
            len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & points(8..),
        )
        // South African Texas at the four level — a 6-card major.  `4♣/4♦`
        // transfer to the major as the everyday *preemptive* to-play route:
        // jumping straight to game robs the opponents of the two-level a slow
        // Jacoby transfer would leave them to balance in.  A *direct* `4♥/4♠` is a
        // non-forcing slam try (opener passes a minimum, or launches RKCB with a
        // maximum — see [`slam_try_answer`]).  All four outrank the 2.0 Jacoby
        // transfers so the 6-card hand takes the four-level route; the `len(other
        // major, ..5)` guard keeps a 5-5+ two-suiter on the both-majors 3♦, and
        // the `hcp` split routes game-no-slam to the transfer and slam-invitational
        // (15–18) to the direct slam try.
        .rule(
            Bid::new(4, Strain::Clubs),
            2.5,
            len(Suit::Hearts, 6..) & len(Suit::Spades, ..5) & hcp(9..=14),
        )
        .rule(
            Bid::new(4, Strain::Diamonds),
            2.5,
            len(Suit::Spades, 6..) & len(Suit::Hearts, ..5) & hcp(9..=14),
        )
        .rule(
            Bid::new(4, Strain::Hearts),
            2.6,
            len(Suit::Hearts, 6..) & len(Suit::Spades, ..5) & hcp(15..=18),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            2.6,
            len(Suit::Spades, 6..) & len(Suit::Hearts, ..5) & hcp(15..=18),
        )
        // Puppet Stayman: game-forcing, balanced, with a three-card major.  Ranks
        // *above* Stayman so a 4-3 hand — holding both a four- and a three-card
        // major — takes the Puppet route, which catches opener's five-card major
        // in the three-card suit (plain Stayman would miss it).  `balanced()`
        // keeps Puppet off shapely hands, leaving them to the 2♠/2NT transfers;
        // a balanced no-four-card-major hand almost always has a three-card major
        // (2-2 majors need two doubletons), so this routes most balanced game
        // forces through 3♣ — the no-fit case just relays back to 3NT.
        .rule(
            Bid::new(3, Strain::Clubs),
            1.6,
            balanced()
                & hcp(9..=15)
                & (len(Suit::Hearts, 3..=3) | len(Suit::Spades, 3..=3))
                & len(Suit::Hearts, ..5)
                & len(Suit::Spades, ..5),
        )
        // Stayman: a four-card major and at least invitational values.
        .rule(
            Bid::new(2, Strain::Clubs),
            1.5,
            (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4)) & hcp(8..),
        )
        // Two-way 2♠: a six-card club one-suiter (weak signoff, or game-going via
        // a later splinter) OR a balanced invitational eight with no four-card
        // major.  The bare-8 invite relocated here when 2NT became the diamond
        // transfer; min→2NT and max→3NT reproduce the old natural-2NT outcomes.
        .rule(
            Bid::new(2, Strain::Spades),
            1.3,
            len(Suit::Clubs, 6..)
                | (hcp(8..=8) & balanced() & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4)),
        )
        // 2NT: transfer to diamonds — six diamonds, or a 5♦-4♣ minor two-suiter.
        .rule(
            Bid::new(2, Strain::Notrump),
            1.3,
            len(Suit::Diamonds, 6..) | (len(Suit::Diamonds, 5..) & len(Suit::Clubs, 4..)),
        )
        // Quantitative 4NT slam invite (balanced, no four-card major).
        .rule(
            Bid::new(4, Strain::Notrump),
            1.2,
            hcp(16..=17) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        // Natural 3NT game-force, 9+, no five-card major (those transfer).  A
        // balanced hand with a three-card major prefers Puppet (3♣ outranks), so
        // in practice this catches game forces lacking a three-card major and the
        // 18–19 too strong for the quantitative 4NT.  Forcing every 9 (rather than
        // inviting 8–9 and forcing 10+) is A/B-verified worth ≈+1 IMP per
        // divergent board vul none and ≈+3 vul both: opposite a 15–17 opener a 9
        // makes game often enough that the invitational stop loses more by missing
        // games than it gains.  Deciding the 9 by Fifths instead was measured
        // *worse* — even quack-heavy 9s are worth forcing.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(9..) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(
            Call::Pass,
            0.0,
            hcp(..8) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
}

/// Opener's answer to Stayman: a four-card major, else 2♦
///
/// `pub(super)` so the competitive book can reuse it as the always-mass catch-all
/// when authoring opener's penalty-pass over a `(2♣)` overcall (systems on).
pub(super) fn stayman_answers() -> Rules {
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

/// Complete a four-level Texas transfer by bidding game in the anchor major
///
/// `4♣ → 4♥`, `4♦ → 4♠`.  Responder showed 6+ with game-no-slam values, so
/// opener simply names the game and declares.
fn complete_texas(into: Suit) -> Rules {
    Rules::new().rule(Bid::new(4, Strain::from(into)), 1.0, hcp(0..))
}

/// Opener's answer to a direct four-of-a-major slam try (`1NT–4♥/4♠`)
///
/// Non-forcing: a **maximum** (17) accepts by launching RKCB (`4NT`); a minimum
/// signs off by passing the major game.  The 1430 ladder ([`slam`]) then exchanges
/// keycards and places `6M`, or `5M` when the partnership is missing two.
fn slam_try_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.0, hcp(17..))
        .rule(Call::Pass, 0.0, hcp(..17))
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
            len(Suit::Hearts, 4..=4) & len(Suit::Spades, 5..) & hcp(9..),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            1.4,
            len(Suit::Spades, 4..=4) & len(Suit::Hearts, 5..) & hcp(9..),
        )
        .rule(Bid::new(4, Strain::Notrump), 1.2, hcp(16..=17))
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(9..))
        .rule(Bid::new(2, Strain::Notrump), 1.0, hcp(8..=8))
}

/// Opener completes Smolen by bidding game in responder's shown five-card major
pub(super) fn smolen_completion(five_card: Suit) -> Rules {
    let strain = Strain::from(five_card);
    Rules::new()
        // Eight-card fit: bid game in the long major so opener declares.
        .rule(Bid::new(4, strain), 1.0, len(five_card, 3..))
        // No fit: notrump game.
        .rule(Bid::new(3, Strain::Notrump), 0.5, len(five_card, ..3))
}

/// Smolen at the three level: responder's jump after opener denies a major
/// (`…3♣–3♦`).  Game is already forced, so no strength gate.
pub(super) fn smolen_at_three() -> Rules {
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
// Puppet Stayman (1NT–3♣)
// ---------------------------------------------------------------------------

/// Opener's answer to Puppet Stayman: a five-card major, else 3♦ to deny
///
/// Puppet is balanced and game-forcing, so opener always cooperates — name a
/// five-card major (`3♥`/`3♠`), otherwise bid `3♦`, denying five but possibly
/// holding a four-card major for the Smolen-style 4-4 hunt below.
fn puppet_answers() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 1.0, len(Suit::Hearts, 5..))
        .rule(
            Bid::new(3, Strain::Spades),
            1.0,
            len(Suit::Spades, 5..) & len(Suit::Hearts, ..5),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            0.5,
            len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
}

/// Responder's rebid after opener names a five-card major over Puppet
///
/// Three-card support is an eight-card fit — bid game in the major so opener
/// declares; otherwise opener's major was responder's short one, so settle in
/// 3NT.  Puppet hands are balanced, so there is no splinter slam-try here (that
/// tool lives in the shapely 2♠ club structure).
fn puppet_major_rebid(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 1.0, len(major, 3..))
        .rule(Bid::new(3, Strain::Notrump), 0.5, len(major, ..3))
}

/// Responder's rebid after opener denies a five-card major (`1NT–3♣–3♦`)
///
/// Smolen-style: a four-card major (so responder is 4-3) bids the *shorter*
/// three-card major to show four in the longer, right-siding game to opener.
/// With no four-card major (3-3, or three and a short major) there is no 4-4 to
/// find — settle in 3NT.
fn puppet_deny_rebid() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Hearts),
            1.0,
            len(Suit::Spades, 4..=4) & len(Suit::Hearts, 3..=3),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            1.0,
            len(Suit::Hearts, 4..=4) & len(Suit::Spades, 3..=3),
        )
        .rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..))
}

/// Opener completes the Puppet 4-4 hunt: game in responder's shown major, or 3NT
///
/// Responder's short-major bid named four cards in `shown_major`; raise to game
/// with four-card support, else 3NT.
fn puppet_smolen_completion(shown_major: Suit) -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::from(shown_major)),
            1.0,
            len(shown_major, 4..),
        )
        .rule(Bid::new(3, Strain::Notrump), 0.5, len(shown_major, ..4))
}

// ---------------------------------------------------------------------------
// Both-majors 3♦ (1NT–3♦ = 5+/5+ majors, invitational+)
// ---------------------------------------------------------------------------

/// Opener's answer to the both-majors 3♦: pick the strain by strength
///
/// With a maximum (17) jump to the eight-card major game, or 3NT when 2-2 in the
/// majors leaves only a seven-card fit.  A minimum (15–16) signs off in three of
/// the better major — spades whenever holding three, else hearts — leaving
/// responder to pass an invitation or raise with game values.  Authored, not
/// floored: the keyless floor misreads 3♦ as natural diamonds and forces game.
//
// ponytail: "better major" is spades-with-three, else hearts — it finds an
// eight-card fit when one exists but prefers spades on a tie (e.g. 3♠ on 3-4
// majors).  Good enough; refine only if the A/B asks for it.
fn five_five_major_answer() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Spades),
            1.2,
            hcp(17..) & len(Suit::Spades, 3..),
        )
        .rule(
            Bid::new(4, Strain::Hearts),
            1.2,
            hcp(17..) & len(Suit::Spades, ..3) & len(Suit::Hearts, 3..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            1.2,
            hcp(17..) & len(Suit::Spades, ..3) & len(Suit::Hearts, ..3),
        )
        .rule(Bid::new(3, Strain::Spades), 1.0, len(Suit::Spades, 3..))
        .rule(Bid::new(3, Strain::Hearts), 1.0, len(Suit::Spades, ..3))
}

/// Responder's decision over opener's minimum 3-level signoff
///
/// Opener showed 15–16 by signing off in `major`; responder raises to game with
/// the upper half of the invitational+ range and otherwise passes.  Needed
/// because the floor forces responder to game off the 3♦ opening and so could
/// not pass the invitation.  `points` again — responder is the 5-5 hand.
fn five_five_min_rebid(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 1.0, points(10..))
        .rule(Call::Pass, 0.9, points(..10))
}

// ---------------------------------------------------------------------------
// Minor-suit transfers (1NT–2NT diamonds, 1NT–2♠ clubs/invite)
// ---------------------------------------------------------------------------

/// Opener passes a weak responder retreat
///
/// Authored only to override the keyless floor, which reads a three-level suit
/// response to our 1NT as game-forcing and would otherwise refuse to pass.
fn pass_out() -> Rules {
    Rules::new().rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's reply to the 2NT diamond transfer: complete to 3♦ with a fit, else 3♣
///
/// Three-card diamond support is an assured eight-card fit — complete the
/// transfer.  Short diamonds bid `3♣` instead, pass-or-correct, letting a 5♦4♣
/// responder pick the better minor.
fn diamond_transfer_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 1.0, len(Suit::Diamonds, 3..))
        .rule(Bid::new(3, Strain::Clubs), 0.5, len(Suit::Diamonds, ..3))
}

/// Responder's rebid after opener completes the diamond transfer (`…2NT–3♦`)
///
/// Game values bid 3NT — a long suit bids game on fewer points (`threshold` ≈ 8,
/// below the 9 a balanced hand needs).  Otherwise pass the diamond partscore.
fn diamond_transfer_game(threshold: u8) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(threshold..))
        .rule(Call::Pass, 0.0, hcp(..threshold))
}

/// Responder's rebid after opener's pass-or-correct `3♣` (`…2NT–3♣`, short ♦)
///
/// Game values bid 3NT; a six-card diamond suit retreats to `3♦` (a 6-2 fit beats
/// the possible club misfit); otherwise (5♦4♣) pass and sit for opener's clubs.
fn diamond_transfer_correct(threshold: u8) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(threshold..))
        .rule(
            Bid::new(3, Strain::Diamonds),
            0.5,
            len(Suit::Diamonds, 6..) & hcp(..threshold),
        )
        .rule(Call::Pass, 0.0, len(Suit::Diamonds, ..6) & hcp(..threshold))
}

/// A six-card club one-suiter short in `short` with game values — a splinter shape
fn club_splinter(short: Suit, threshold: u8) -> Cons<impl Constraint + Clone> {
    len(Suit::Clubs, 6..) & hcp(threshold..) & len(short, ..2)
}

/// A six-card club hand with game values and no singleton — game-going, slamless
fn club_no_shortness(threshold: u8) -> Cons<impl Constraint + Clone> {
    len(Suit::Clubs, 6..)
        & hcp(threshold..)
        & len(Suit::Diamonds, 2..)
        & len(Suit::Hearts, 2..)
        & len(Suit::Spades, 2..)
}

/// Opener's reply to the two-way 2♠: `3♣` with a maximum, `2NT` with a minimum
///
/// Showing strength lets responder pass-or-correct safely: the weak club hand
/// lands in `3♣` either way, the balanced invite plays `2NT` (min) or `3NT`
/// (max), and a game-going club hand splinters.
fn two_spade_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Clubs), 1.0, hcp(17..))
        .rule(Bid::new(2, Strain::Notrump), 0.9, hcp(0..))
}

/// Responder's pass-or-correct after opener's minimum `2NT` over the two-way 2♠
fn two_spade_over_min() -> Rules {
    Rules::new()
        // Balanced invite: opener is minimum, settle in 2NT.
        .rule(Call::Pass, 0.0, hcp(8..=8) & balanced())
        // Weak club one-suiter: correct to the club partscore.
        .rule(
            Bid::new(3, Strain::Clubs),
            0.8,
            len(Suit::Clubs, 6..) & hcp(..8),
        )
        // Game-going clubs with a singleton: splinter so opener picks 3NT or 5♣.
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.0,
            club_splinter(Suit::Diamonds, 8),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            1.0,
            club_splinter(Suit::Hearts, 8),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            1.0,
            club_splinter(Suit::Spades, 8),
        )
        // Game-going clubs without a singleton: 3NT.
        .rule(Bid::new(3, Strain::Notrump), 0.9, club_no_shortness(8))
}

/// Responder's action after opener's maximum `3♣` over the two-way 2♠
fn two_spade_over_max() -> Rules {
    Rules::new()
        // Weak club one-suiter: pass the club partscore.
        .rule(Call::Pass, 0.0, len(Suit::Clubs, 6..) & hcp(..8))
        // Game-going clubs with a singleton: splinter.
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.0,
            club_splinter(Suit::Diamonds, 8),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            1.0,
            club_splinter(Suit::Hearts, 8),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            1.0,
            club_splinter(Suit::Spades, 8),
        )
        // Balanced invite (opener maximum → accept game) or game clubs without a
        // singleton: 3NT.
        .rule(
            Bid::new(3, Strain::Notrump),
            0.9,
            (hcp(8..=8) & balanced()) | club_no_shortness(8),
        )
}

/// Opener picks the game over responder's club splinter: 3NT with the short suit
/// stopped, else 5♣
fn pick_game_over_club_splinter(short: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.0, stopper_in(short))
        .rule(Bid::new(5, Strain::Clubs), 0.9, hcp(0..))
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

    // --- Puppet Stayman (1NT–3♣) ----------------------------------------------
    //
    // Opener shows a five-card major (3♥/3♠) or denies with 3♦; responder raises
    // a 5-3 fit, or — Smolen-style after 3♦ — bids the shorter major to find a
    // 4-4 with opener declaring.
    let three_c = call(3, Strain::Clubs);
    let three_d = call(3, Strain::Diamonds);
    insert_uncontested(book, &[one_nt, three_c], puppet_answers());
    insert_uncontested(
        book,
        &[one_nt, three_c, three_h],
        puppet_major_rebid(Suit::Hearts),
    );
    insert_uncontested(
        book,
        &[one_nt, three_c, three_s],
        puppet_major_rebid(Suit::Spades),
    );
    insert_uncontested(book, &[one_nt, three_c, three_d], puppet_deny_rebid());
    // Responder's shorter-major bid named four cards in the *other* major.
    insert_uncontested(
        book,
        &[one_nt, three_c, three_d, three_h],
        puppet_smolen_completion(Suit::Spades),
    );
    insert_uncontested(
        book,
        &[one_nt, three_c, three_d, three_s],
        puppet_smolen_completion(Suit::Hearts),
    );

    // --- Both-majors 3♦ (1NT–3♦) ----------------------------------------------
    //
    // Opener signs off in 3M with a minimum or jumps to game (4M / 3NT) with a
    // maximum; over a minimum signoff responder passes an invitation or raises.
    insert_uncontested(book, &[one_nt, three_d], five_five_major_answer());
    insert_uncontested(
        book,
        &[one_nt, three_d, three_h],
        five_five_min_rebid(Suit::Hearts),
    );
    insert_uncontested(
        book,
        &[one_nt, three_d, three_s],
        five_five_min_rebid(Suit::Spades),
    );

    // --- South African Texas (1NT–4♣/4♦ transfers, 1NT–4♥/4♠ slam tries) ------
    //
    // To-play transfers: opener names the game in the anchor major and declares.
    // Slam tries: opener passes (minimum) or launches RKCB (maximum); the 1430
    // ladder in `slam` registers the keycard exchange and the slam placement.
    let four_c = call(4, Strain::Clubs);
    let four_d = call(4, Strain::Diamonds);
    let four_h = call(4, Strain::Hearts);
    let four_s = call(4, Strain::Spades);
    insert_uncontested(book, &[one_nt, four_c], complete_texas(Suit::Hearts));
    insert_uncontested(book, &[one_nt, four_d], complete_texas(Suit::Spades));
    insert_uncontested(book, &[one_nt, four_h], slam_try_answer());
    insert_uncontested(book, &[one_nt, four_s], slam_try_answer());
    slam::install_rkcb(book, &[one_nt, four_h], Suit::Hearts);
    slam::install_rkcb(book, &[one_nt, four_s], Suit::Spades);

    // --- Diamond transfer (1NT–2NT) -------------------------------------------
    insert_uncontested(book, &[one_nt, two_nt], diamond_transfer_answer());
    insert_uncontested(book, &[one_nt, two_nt, three_d], diamond_transfer_game(8));
    insert_uncontested(
        book,
        &[one_nt, two_nt, three_c],
        diamond_transfer_correct(8),
    );
    // Weak retreat to 3♦ over opener's pass-or-correct 3♣ — opener must pass.
    insert_uncontested(book, &[one_nt, two_nt, three_c, three_d], pass_out());

    // --- Two-way 2♠ (clubs or balanced invite) --------------------------------
    insert_uncontested(book, &[one_nt, two_s], two_spade_answer());
    insert_uncontested(book, &[one_nt, two_s, two_nt], two_spade_over_min());
    insert_uncontested(book, &[one_nt, two_s, three_c], two_spade_over_max());
    // Weak-club correction to 3♣ over opener's minimum 2NT — opener must pass.
    insert_uncontested(book, &[one_nt, two_s, two_nt, three_c], pass_out());
    // Opener picks 3NT/5♣ over the game-going club splinter (3♦/3♥/3♠), after
    // either the minimum (2NT) or maximum (3♣) reply.
    for opener_reply in [two_nt, three_c] {
        for short in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            insert_uncontested(
                book,
                &[one_nt, two_s, opener_reply, call(3, Strain::from(short))],
                pick_game_over_club_splinter(short),
            );
        }
    }
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

#[cfg(test)]
mod tests {
    use crate::american;
    use crate::bidding::{Family, System};
    use contract_bridge::auction::{Call, RelativeVulnerability};
    use contract_bridge::{Bid, Strain};

    const P: Call = Call::Pass;

    fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid::new(level, strain))
    }

    /// The highest-logit call `american()` assigns the hand at the auction
    fn best(auction: &[Call], hand: &str) -> Call {
        let hand = hand.parse().expect("valid test hand");
        let logits = american()
            .against(Family::NATURAL)
            .classify(hand, RelativeVulnerability::NONE, auction)
            .expect("a decision");
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("the logits array is never empty")
    }

    /// The revised South African Texas: 4♣/4♦ to-play transfers and the 4♥/4♠
    /// non-forcing slam try wired into RKCB, end to end through `american()`.
    #[test]
    fn south_african_texas_slam_try() {
        let one_nt = [bid(1, Strain::Notrump), P];

        // Responder, 6 hearts: a 16-count makes the direct 4♥ slam try; a 10-count
        // takes the 4♣ to-play transfer.
        assert_eq!(best(&one_nt, "42.AKJ872.KQ4.K2"), bid(4, Strain::Hearts));
        assert_eq!(best(&one_nt, "42.AKJ872.Q43.32"), bid(4, Strain::Clubs));

        // Opener over the slam try (1NT–P–4♥–P): a maximum (17) launches RKCB, a
        // minimum (15) signs off by passing the major game.
        let over_try = [bid(1, Strain::Notrump), P, bid(4, Strain::Hearts), P];
        assert_eq!(best(&over_try, "KQ3.K53.AQ54.K92"), bid(4, Strain::Notrump));
        assert_eq!(best(&over_try, "KQ3.K53.KQ54.Q92"), P);

        // Opener completes the 4♣ to-play transfer (1NT–P–4♣–P) → 4♥.
        let over_transfer = [bid(1, Strain::Notrump), P, bid(4, Strain::Clubs), P];
        assert_eq!(
            best(&over_transfer, "KQ3.K53.KQ54.Q92"),
            bid(4, Strain::Hearts)
        );

        // RKCB is wired: responder answers keycards over 4NT (A♥+K♥ = 2, no ♥Q → 5♥),
        // then the asker with 3 keycards places the small slam.
        let over_ask = [
            bid(1, Strain::Notrump),
            P,
            bid(4, Strain::Hearts),
            P,
            bid(4, Strain::Notrump),
            P,
        ];
        assert_eq!(best(&over_ask, "42.AKJ872.KQ4.K2"), bid(5, Strain::Hearts));
        let over_answer = [
            bid(1, Strain::Notrump),
            P,
            bid(4, Strain::Hearts),
            P,
            bid(4, Strain::Notrump),
            P,
            bid(5, Strain::Hearts),
            P,
        ];
        assert_eq!(
            best(&over_answer, "KQ3.AK3.AQ54.J92"),
            bid(6, Strain::Hearts)
        );
    }

    /// Over a natural (2♣) overcall of our 1NT we play *systems on*, not
    /// Lebensohl: 2♣ steals no room, so responder keeps the uncontested Jacoby
    /// transfers, shows the stolen 2♣ Stayman with a Double, and opener answers in
    /// the uncontested tree (the systems-on rebase in `competition.rs`). There is
    /// no natural 2♦ escape — 2♦ is a transfer.
    #[test]
    fn systems_on_over_two_clubs() {
        use contract_bridge::auction::Auction;
        // The highest-logit *legal* call (what the real bidder picks; the bare
        // `best` helper ignores legality, so it can't drop the now-illegal 2♣).
        let best_legal = |auction: &[Call], hand: &str| -> Call {
            let hand = hand.parse().expect("valid test hand");
            let logits = american()
                .against(Family::NATURAL)
                .classify(hand, RelativeVulnerability::NONE, auction)
                .expect("a decision");
            let mut played = Auction::new();
            for &c in auction {
                played.push(c);
            }
            let mut scored: Vec<_> = (&logits.0)
                .into_iter()
                .filter(|(_, l)| l.is_finite())
                .collect();
            scored.sort_by(|x, y| y.1.partial_cmp(x.1).expect("no NaN"));
            scored
                .into_iter()
                .map(|(c, _)| c)
                .find(|&c| played.can_push(c).is_ok())
                .unwrap_or(Call::Pass)
        };

        let over_2c = [bid(1, Strain::Notrump), bid(2, Strain::Clubs)];
        // 5 hearts → 2♦ transfer; 5 spades → 2♥ transfer (systems on, not natural).
        assert_eq!(
            best_legal(&over_2c, "2.KJ876.5432.432"),
            bid(2, Strain::Diamonds)
        );
        assert_eq!(
            best_legal(&over_2c, "KJ876.2.5432.432"),
            bid(2, Strain::Hearts)
        );
        // 4-4 majors, invitational: the stolen 2♣ Stayman is shown by Double.
        assert_eq!(best_legal(&over_2c, "KJ32.KQ43.432.43"), Call::Double);

        // Opener completes the transfer: 1NT–(2♣)–2♦–(P) → 2♥, via the rebase.
        let over_xfer = [
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            bid(2, Strain::Diamonds),
            P,
        ];
        assert_eq!(
            best_legal(&over_xfer, "KQ3.A53.KQ54.K92"),
            bid(2, Strain::Hearts)
        );

        // Opener answers the stolen Stayman: 1NT–(2♣)–X–(P) → 2♥ with four hearts.
        let over_dbl = [
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Double,
            P,
        ];
        assert_eq!(
            best_legal(&over_dbl, "AQ3.KJ54.KQ4.92"),
            bid(2, Strain::Hearts)
        );
    }

    /// Opener converts the stolen-Stayman Double to penalty with good clubs, and
    /// *only* in the contested context — uncontested forcing Stayman never passes.
    #[test]
    fn penalty_pass_over_two_clubs() {
        use crate::bidding::american::set_penalty_pass;

        // 16 HCP, 5332 with AK-fifth of clubs (5 clubs, 7 club HCP), no 4-card major.
        let opener = "A2.K3.Q42.AK432";
        let over_dbl = [
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Double,
            P,
        ];
        let uncontested_stayman = [bid(1, Strain::Notrump), P, bid(2, Strain::Clubs), P];

        // With the penalty pass enabled, opener sits to defend 2♣ doubled.
        set_penalty_pass(Some((4, 4, true)));
        assert_eq!(best(&over_dbl, opener), Call::Pass);
        // Context-specific: the same hand still answers forcing Stayman (2♦) in the
        // *uncontested* auction — the conversion must not leak onto that shared node.
        assert_eq!(best(&uncontested_stayman, opener), bid(2, Strain::Diamonds));

        // With it off (the default), opener can never convert: answers Stayman 2♦.
        set_penalty_pass(None);
        assert_eq!(best(&over_dbl, opener), bid(2, Strain::Diamonds));
    }
}
