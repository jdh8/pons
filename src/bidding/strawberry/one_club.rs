//! The Polish `1♣` (<https://polish.club/1C.html>): responses and opener's
//! rebids
//!
//! Authored so far, each table from the page named:
//!
//! * **Responder's first call** ([`responses`], `1C.md`) — the negative
//!   `1♦!`, positive majors from 7, the notrump ladder, the swapped
//!   game-forcing `2♣!`/`2♦!`, Reverse Flannery `2♥!`, the invitations — and
//!   the passed-hand table, where the game forces become invitations.
//! * **Opener's rebid** after `1♦!` ([`after_negative`], `1C/1D.md`), after a
//!   positive major ([`after_positive`], `1C/1M.md`), after `1NT`/`2NT`
//!   (`1C/1NT.md`), the minor game forces (`1C/2CD.md`) and Reverse Flannery
//!   (`1C/2H.md`).
//! * **Round two** where the page is a single table: responder over
//!   `1♣ - 1♦ - 1M`, `2♣` and `2♦!`, opener over `1♣ - 1♦ - 1M - 1NT`, and
//!   the reverse-445566 steps over Odwrotka (`1C/1M/2D.md`).
//! * **Checkback Gladiator** over `1♣ - 1♥ - 1♠`
//!   (`1C/1M/CheckbackGladiator.md`): the `2♣!` puppet with its weak and
//!   balanced-invitation variants, the game-forcing `2♦!`, and opener's
//!   rebid over `1NT`.
//! * **The strong rebids over `1♦!`**: the house 2NT structure under
//!   `2♦!` ([`two_notrump_rows`], one point lower than over the opening),
//!   standing in for the page's Muppet tree, and responder's placement over
//!   the 20–23 `2M`.
//!
//! Left to the floor, and listed in `docs/strawberry.md`: the shaped
//! invitations and slam tries of Checkback Gladiator, the relay answers over
//! `1♣ - 1NT - 2♦!`, the Muppet tree proper, the systems-on tail of the
//! 18–20 `1NT` rebid, opener's splinters and two-suited jumps, Namyats, and
//! everything contested.  An artificial ask whose answers are not authored
//! is withheld rather than bid: the hands it would take stay in the natural
//! rows.
//!
//! "FG" for opener opposite a positive is 17+ points, as the competitive
//! chapter states it; an invitation is 15–16.

use crate::bidding::agreements::Agreements;
use crate::bidding::american::notrump::two_notrump_rows;
use crate::bidding::constraint::{and, balanced, hcp, len, points};
use crate::bidding::rows::{Entry, Package, Pattern, rows_of};
use crate::bidding::{Alert, Rules};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// The negative `1♦`: 0–6 any, 7–11 with no major, or 16+ balanced
const NEGATIVE: Alert = Alert("strawberry:negative-diamond");
/// `2♣` = game force with diamonds, `2♦` = game force with clubs (swapped)
const SWAPPED_MINOR: Alert = Alert("strawberry:swapped-minor");
/// `2♥` = 4–5 hearts and exactly five spades, 6–9
const FLANNERY: Alert = Alert("strawberry:reverse-flannery");
/// `2♠` = invitational with six diamonds
const DIAMOND_INVITE: Alert = Alert("strawberry:diamond-invite");
/// A passed hand's `2♠` = invitational with both minors
const MINORS_INVITE: Alert = Alert("strawberry:minors-invite");
/// `3♦` = balanced game force, 12–15, no four-card major
const BALANCED_FORCE: Alert = Alert("strawberry:balanced-force");
/// Opener's `2♦` over the negative: 21+, artificial and forcing
const STRONG_REBID: Alert = Alert("strawberry:strong-rebid");
/// Opener's `2NT` over the negative: 18–20 with six clubs
const CLUB_REBID: Alert = Alert("strawberry:club-rebid");
/// A call that shows four of a major beside clubs: opener's `2♦` over
/// `1♣ - 1♦ - 1M - 1NT` (17–19), and the `3M` jump over `1♣ - 2♣`
const MAJOR_AND_CLUBS: Alert = Alert("strawberry:major-and-clubs");
/// Opener's `2NT` over `1♣ - 1♦ - 1M - 1NT`: 17–19, six of the major
const SIX_MAJOR: Alert = Alert("strawberry:six-major");
/// Responder's `2NT` over `1♣ - 1♦ - 2♣`: the constructive club raise
const MIXED_RAISE: Alert = Alert("strawberry:mixed-raise");
/// Responder's `3♦` over `1♣ - 1♦ - 2♣`: 16+ balanced
const BALANCED_SLAM_TRY: Alert = Alert("strawberry:balanced-slam-try");
/// Odwrotka: `2♦` over a positive major, a game force with three-card support
const ODWROTKA: Alert = Alert("strawberry:odwrotka");
/// One of responder's six steps over Odwrotka
const STEP: Alert = Alert("strawberry:odwrotka-step");
/// The jump in the other major over a positive: both minors, game forcing
const BOTH_MINORS: Alert = Alert("strawberry:both-minors");

/// Responder's `2♦` over `1♣ - 1♥ - 1♠`, the game-forcing checkback, and
/// opener's `3♥` answer to it (five spades and three hearts)
const CHECKBACK: Alert = Alert("strawberry:checkback");
/// Responder's `2♣` over `1♣ - 1♥ - 1♠`: the Gladiator puppet, a weak hand
/// with somewhere to play or the balanced invitation
const PUPPET: Alert = Alert("strawberry:checkback-puppet");
/// Opener's `2♦` over the puppet: 11–16, nothing about diamonds
const PUPPET_COMPLETION: Alert = Alert("strawberry:checkback-completion");

const MAJORS: [Suit; 2] = [Suit::Hearts, Suit::Spades];
const MINORS: [Suit; 2] = [Suit::Diamonds, Suit::Clubs];

fn bid(level: u8, strain: impl Into<Strain>) -> Bid {
    Bid::new(level, strain.into())
}

/// The rows both response tables share: majors, the notrump ladder, Reverse
/// Flannery
///
/// A positive major is 7+ points and four cards: the longer first, hearts on
/// 4-4, spades on 5-5.  `1NT` is 8–10 and `2NT` 10–11, both without a major
/// or a six-card minor; a 10-count takes `1NT`.
fn shared(rules: Rules) -> Rules {
    let no_major = || and(MAJORS, ..4);
    rules
        .rule(
            bid(2, Suit::Hearts),
            240,
            points(6..=9) & len(Suit::Spades, 5..=5) & len(Suit::Hearts, 4..=5),
        )
        .alert(FLANNERY)
        .rule(
            bid(1, Strain::Notrump),
            175,
            hcp(8..=10) & and(MAJORS, 2..=3) & and(MINORS, 2..=5),
        )
        .rule(
            bid(2, Strain::Notrump),
            170,
            hcp(10..=11) & and(MAJORS, 2..=3) & and(MINORS, 2..=5),
        )
        .rule(
            bid(1, Suit::Spades),
            165,
            points(7..)
                & ((len(Suit::Spades, 5..=5) & len(Suit::Hearts, ..6)) | len(Suit::Spades, 6..)),
        )
        .rule(
            bid(1, Suit::Hearts),
            160,
            points(7..) & len(Suit::Hearts, 4..),
        )
        .rule(
            bid(1, Suit::Spades),
            155,
            points(7..) & len(Suit::Spades, 4..),
        )
        .rule(
            bid(3, Suit::Clubs),
            180,
            points(9..=11) & len(Suit::Clubs, 6..) & no_major(),
        )
}

/// Responder's first call by an unpassed hand
///
/// The negative `1♦!` is the catch-all, stated by its three hand types so the
/// reading is theirs: the table is total without a vacuous rule.  A game
/// force is 12+ points; with a five-card minor it bids the *other* minor,
/// without one it is the balanced `3♦!` or `3NT`.
fn responses() -> Rules {
    let no_major = || and(MAJORS, ..4);
    let mut rules = Rules::new();
    for major in MAJORS {
        rules = rules
            .rule(bid(4, major), 260, len(major, 8..) & hcp(..=10))
            .rule(bid(3, major), 250, len(major, 7..) & hcp(3..=6));
    }
    shared(rules)
        // ponytail: diamonds first on 5-5 or 5♦6♣; split by length if the
        // minor game forces ever measure.
        .rule(
            bid(2, Suit::Clubs),
            205,
            points(12..) & len(Suit::Diamonds, 5..) & and(MAJORS, ..5),
        )
        .alert(SWAPPED_MINOR)
        .rule(
            bid(2, Suit::Diamonds),
            200,
            points(12..) & len(Suit::Clubs, 5..) & and(MAJORS, ..5),
        )
        .alert(SWAPPED_MINOR)
        .rule(
            bid(3, Strain::Notrump),
            190,
            hcp(12..=15) & and(MAJORS, 3..=3) & and(MINORS, 3..=4),
        )
        .rule(
            bid(3, Suit::Diamonds),
            185,
            hcp(12..=15) & and(MAJORS, 2..=3) & and(MINORS, 3..=4),
        )
        .alert(BALANCED_FORCE)
        .rule(
            bid(2, Suit::Spades),
            180,
            points(9..=11) & len(Suit::Diamonds, 6..) & no_major(),
        )
        .alert(DIAMOND_INVITE)
        .rule(
            bid(1, Suit::Diamonds),
            50,
            points(..=6)
                | (points(..=11) & no_major())
                | (hcp(16..) & and(MAJORS, 2..=3) & and(MINORS, 3..=4)),
        )
        .alert(NEGATIVE)
}

/// Responder's first call by a passed hand
///
/// The game forces become invitations and the preempts strong invitations;
/// the negative `1♦!` takes every hand with no major and nothing better.
fn passed_responses() -> Rules {
    let no_major = || and(MAJORS, ..4);
    let mut rules = Rules::new();
    for suit in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        rules = rules.rule(bid(3, suit), 250, points(10..=11) & len(suit, 6..));
    }
    shared(rules)
        .rule(
            bid(2, Suit::Spades),
            205,
            points(9..=11)
                & and(MINORS, 4..)
                & (len(Suit::Diamonds, 5..) | len(Suit::Clubs, 5..))
                & no_major(),
        )
        .alert(MINORS_INVITE)
        .rule(
            bid(2, Suit::Clubs),
            200,
            points(9..=11) & len(Suit::Diamonds, 5..) & no_major(),
        )
        .alert(SWAPPED_MINOR)
        .rule(
            bid(2, Suit::Diamonds),
            195,
            points(9..=11) & len(Suit::Clubs, 5..) & no_major(),
        )
        .alert(SWAPPED_MINOR)
        .rule(bid(1, Suit::Diamonds), 50, points(..=6) | no_major())
        .alert(NEGATIVE)
}

/// Opener's rebid after `1♣ - 1♦!`
///
/// The strong hands name themselves; everything else bids a four-card major
/// up the line, clubs, or — the weak notrump with no major — its three-card
/// major.  The two-suited game forces (`3♥!`, `3♠!`, `3NT!`) are not
/// authored: those hands rebid `2♦!` or `2M`.
// ponytail: `1M` stretches to 20 for the 4M-5♣ and 4=4=1=4 hands the book
// leaves between 19 and the 21+ `2♦!`.
fn after_negative() -> Rules {
    let mut rules = Rules::new();
    for minor in MINORS {
        rules = rules.rule(bid(3, minor), 300, points(21..=23) & len(minor, 6..));
    }
    rules
        .rule(
            bid(2, Suit::Spades),
            295,
            points(20..=23) & len(Suit::Spades, 5..),
        )
        .rule(
            bid(2, Suit::Hearts),
            290,
            points(20..=23) & len(Suit::Hearts, 5..),
        )
        .rule(bid(2, Suit::Diamonds), 280, points(21..))
        .alert(STRONG_REBID)
        .rule(
            bid(2, Strain::Notrump),
            270,
            points(18..=20) & len(Suit::Clubs, 6..),
        )
        .alert(CLUB_REBID)
        .rule(
            bid(1, Strain::Notrump),
            260,
            hcp(18..=20) & and(MAJORS, 2..=5) & and(MINORS, 2..=6),
        )
        .rule(
            bid(2, Suit::Clubs),
            200,
            points(..=17) & len(Suit::Clubs, 5..) & and(MAJORS, ..4),
        )
        .rule(
            bid(1, Suit::Spades),
            155,
            points(..=20) & len(Suit::Spades, 5..) & len(Suit::Hearts, ..=5),
        )
        .rule(
            bid(1, Suit::Hearts),
            150,
            points(..=20) & len(Suit::Hearts, 4..),
        )
        .rule(
            bid(1, Suit::Spades),
            145,
            points(..=20) & len(Suit::Spades, 4..),
        )
        .rule(
            bid(1, Suit::Hearts),
            100,
            hcp(..=14) & len(Suit::Hearts, 3..=3) & len(Suit::Spades, ..4),
        )
        .rule(
            bid(1, Suit::Spades),
            95,
            hcp(..=14) & len(Suit::Spades, 3..=3) & len(Suit::Hearts, ..3),
        )
}

/// Responder's second call after `1♣ - 1♦ - 1M`
fn after_negative_major(major: Suit) -> Rules {
    let mut rules = Rules::new()
        .rule(bid(2, Strain::Notrump), 200, hcp(16..) & balanced())
        .rule(bid(3, major), 150, points(4..=6) & len(major, 5..))
        .rule(
            bid(2, major),
            140,
            points(4..=6) & (len(major, 4..) | (len(major, 3..=3) & !balanced())),
        );
    for (minor, weight) in [(Suit::Diamonds, 135), (Suit::Clubs, 130)] {
        rules = rules.rule(bid(2, minor), weight, points(7..=11) & len(minor, 5..));
    }
    rules
        .rule(bid(1, Strain::Notrump), 100, hcp(5..=11))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's third call after `1♣ - 1♦ - 1M - 1NT`
fn after_negative_major_notrump(major: Suit) -> Rules {
    Rules::new()
        .rule(bid(2, Strain::Notrump), 150, points(17..) & len(major, 6..))
        .alert(SIX_MAJOR)
        .rule(bid(2, major), 140, points(17..) & len(major, 5..=5))
        .rule(
            bid(2, Suit::Diamonds),
            130,
            points(17..) & len(major, 4..=4) & len(Suit::Clubs, 4..),
        )
        .alert(MAJOR_AND_CLUBS)
        .rule(
            bid(2, Suit::Clubs),
            100,
            points(..=16) & len(Suit::Clubs, 5..),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's second call after `1♣ - 1♦ - 2♣`
fn after_negative_clubs() -> Rules {
    let mut rules = Rules::new()
        .rule(bid(3, Suit::Diamonds), 200, hcp(16..) & balanced())
        .alert(BALANCED_SLAM_TRY)
        .rule(
            bid(3, Suit::Clubs),
            150,
            points(10..=11) & len(Suit::Clubs, 4..),
        )
        .rule(
            bid(2, Strain::Notrump),
            140,
            points(7..=9) & len(Suit::Clubs, 4..),
        )
        .alert(MIXED_RAISE)
        .rule(
            bid(2, Suit::Diamonds),
            130,
            points(7..=9) & len(Suit::Diamonds, 5..),
        );
    for (major, weight) in [(Suit::Spades, 125), (Suit::Hearts, 120)] {
        rules = rules.rule(bid(2, major), weight, points(..=6) & len(major, 5..));
    }
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Responder's second call after `1♣ - 1♦ - 2♦!`: the weak sign-offs
///
/// A hand worth game opposite 21 (4+ HCP) bids "as if partner opened a
/// natural 2NT": the house 2NT structure shares this node
/// ([`two_notrump_rows`] in [`package`]).  These rows outrank its transfers,
/// which take any strength, and cover every hand its pass would: a bust
/// never passes the artificial rebid.
fn after_strong_rebid() -> Rules {
    Rules::new()
        .rule(
            bid(2, Suit::Hearts),
            310,
            hcp(..=3) & len(Suit::Hearts, 4..),
        )
        .rule(
            bid(2, Suit::Spades),
            305,
            hcp(..=3) & len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(bid(2, Strain::Notrump), 300, hcp(..=3) & and(MAJORS, ..4))
}

/// Responder's second call after `1♣ - 1♦ - 2M` (20–23, five cards)
///
/// The page gives no table.  Game with a fit or with values, to play: left
/// to the floor, opener read the raise as unlimited and bid six over it.  A
/// hand too good for these rows has none and bids on with the floor.
fn after_strong_major(major: Suit) -> Rules {
    Rules::new()
        .rule(bid(4, major), 150, points(4..=9) & len(major, 3..))
        .rule(bid(3, Strain::Notrump), 140, hcp(4..=9) & len(major, ..3))
        .rule(Call::Pass, 0, hcp(..=3))
}

/// Opener's rebid after a positive `1♣ - 1M`
///
/// Odwrotka `2♦!` takes every game force with three-card support, so the
/// splinters and the solid-club `3NT` are not authored.  The weak notrump
/// rebids `1NT` over `1♠` even with four hearts: checkback finds them.
fn after_positive(major: Suit) -> Rules {
    let mut rules = Rules::new()
        .rule(bid(2, Suit::Diamonds), 250, points(17..) & len(major, 3..))
        .alert(ODWROTKA)
        // The jump in the other major (a jump shift over 1♠, since 2♥ is
        // natural there): both minors.
        .rule(
            if major == Suit::Hearts {
                bid(2, Suit::Spades)
            } else {
                bid(3, Suit::Hearts)
            },
            240,
            points(17..) & and(MINORS, 5..) & len(major, ..3),
        )
        .alert(BOTH_MINORS);
    rules = if major == Suit::Hearts {
        // 1♠ is natural and forcing, at any strength.
        rules
            .rule(
                bid(1, Suit::Spades),
                235,
                len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
            )
            .rule(
                bid(2, Strain::Notrump),
                220,
                points(17..)
                    & len(Suit::Spades, 2..=3)
                    & len(Suit::Hearts, 1..=2)
                    & and(MINORS, 3..=5),
            )
    } else {
        rules
            .rule(
                bid(2, Suit::Hearts),
                235,
                points(17..) & len(Suit::Hearts, 5..) & len(Suit::Spades, ..3),
            )
            .rule(
                bid(2, Strain::Notrump),
                220,
                points(17..)
                    & len(Suit::Spades, ..=2)
                    & len(Suit::Hearts, 2..=4)
                    & and(MINORS, 2..=5),
            )
    };
    for minor in MINORS {
        rules = rules.rule(
            bid(3, minor),
            230,
            points(17..) & len(minor, 6..) & len(major, ..3),
        );
    }
    rules
        .rule(bid(3, major), 190, points(15..=16) & len(major, 4..))
        .rule(bid(2, major), 180, points(..=14) & len(major, 4..))
        // Unbalanced clubs with no fit.  Over 1♥ four spades bid 1♠ above;
        // over 1♠ a four-card heart suit is no reason to hide the clubs.
        .rule(
            bid(2, Suit::Clubs),
            150,
            points(..=17) & len(Suit::Clubs, 5..) & len(major, ..4) & !balanced(),
        )
        .rule(
            bid(1, Strain::Notrump),
            140,
            hcp(12..=14) & len(major, 2..=3),
        )
}

/// Responder's step over Odwrotka: the major's length and a strength band
/// (reverse 445566 — the stronger step first, so the weak hand never bids
/// notrump)
fn odwrotka_steps(major: Suit) -> Rules {
    Rules::new()
        .rule(bid(2, Suit::Hearts), 100, points(10..) & len(major, 4..=4))
        .alert(STEP)
        .rule(bid(2, Suit::Spades), 100, points(..=9) & len(major, 4..=4))
        .alert(STEP)
        .rule(
            bid(2, Strain::Notrump),
            100,
            points(10..) & len(major, 5..=5),
        )
        .alert(STEP)
        .rule(bid(3, Suit::Clubs), 100, points(..=9) & len(major, 5..=5))
        .alert(STEP)
        .rule(bid(3, Suit::Diamonds), 100, points(10..) & len(major, 6..))
        .alert(STEP)
        .rule(bid(3, major), 100, points(..=9) & len(major, 6..))
        .alert(STEP)
}

/// Opener's placement over an Odwrotka step
// ponytail: game only.  The slam tries (and the book's recommended strong-2♥
// relay structure) are owed; add them when the steps measure.
fn after_odwrotka_step(major: Suit, four_cards: bool) -> Rules {
    // Capped at 19: a stronger opener has no row here and bids on with the
    // floor.
    let game = bid(4, major);
    if four_cards {
        Rules::new()
            .rule(game, 100, points(..=19) & len(major, 4..))
            .rule(bid(3, Strain::Notrump), 50, points(..=19) & len(major, ..4))
    } else {
        Rules::new().rule(game, 100, points(..=19))
    }
}

/// Opener's rebid after `1♣ - 1NT` (8–10 balanced, no major)
///
/// The artificial `2♦!` relay waits for its answers; the natural rows carry
/// its hands.
fn after_notrump() -> Rules {
    let mut rules = Rules::new();
    for (major, weight) in [(Suit::Spades, 205), (Suit::Hearts, 200)] {
        rules = rules.rule(bid(2, major), weight, points(17..) & len(major, 5..));
    }
    for (minor, weight) in [(Suit::Diamonds, 195), (Suit::Clubs, 190)] {
        rules = rules.rule(bid(3, minor), weight, points(18..) & len(minor, 6..));
    }
    rules
        .rule(bid(3, Strain::Notrump), 150, hcp(16..))
        .rule(bid(2, Strain::Notrump), 140, hcp(14..=15))
        .rule(
            bid(2, Suit::Clubs),
            130,
            points(..=14) & len(Suit::Clubs, 6..),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's rebid after `1♣ - 2NT` (10–11 balanced, no major)
fn after_jump_notrump() -> Rules {
    let mut rules = Rules::new();
    for (major, weight) in [(Suit::Spades, 205), (Suit::Hearts, 200)] {
        rules = rules.rule(bid(3, major), weight, points(17..) & len(major, 5..));
    }
    rules
        .rule(bid(3, Strain::Notrump), 150, hcp(14..))
        .rule(
            bid(3, Suit::Clubs),
            130,
            points(..=13) & len(Suit::Clubs, 6..),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's rebid after a swapped minor game force
///
/// `shown` is responder's suit: diamonds over `2♣!`, clubs over `2♦!`.  A
/// minimum and an 18+ hand share the cheap natural rebids, the 15–17 hands
/// jump (`3NT` without a fit); `2M` is 17+ with five.  Opener's splinters
/// over `2♦!` are not authored.
fn after_swapped_minor(shown: Suit) -> Rules {
    let extremes = || points(..=14) | points(18..);
    let mut rules = Rules::new();
    for (major, weight) in [(Suit::Spades, 225), (Suit::Hearts, 220)] {
        rules = rules.rule(bid(2, major), weight, points(17..) & len(major, 5..));
    }
    if shown == Suit::Diamonds {
        for (major, weight) in [(Suit::Spades, 215), (Suit::Hearts, 210)] {
            rules = rules
                .rule(
                    bid(3, major),
                    weight,
                    points(15..=17) & len(major, 4..) & len(Suit::Clubs, 5..),
                )
                .alert(MAJOR_AND_CLUBS);
        }
        rules
            .rule(
                bid(3, Suit::Diamonds),
                200,
                points(15..=17) & len(Suit::Diamonds, 3..),
            )
            .rule(
                bid(3, Strain::Notrump),
                190,
                points(15..=17) & len(Suit::Diamonds, ..3),
            )
            .rule(
                bid(3, Suit::Clubs),
                180,
                extremes() & len(Suit::Clubs, 6..) & len(Suit::Diamonds, ..3),
            )
            .rule(
                bid(2, Suit::Diamonds),
                170,
                extremes() & len(Suit::Diamonds, 3..),
            )
            .rule(
                bid(2, Strain::Notrump),
                160,
                extremes() & len(Suit::Diamonds, ..3),
            )
    } else {
        rules
            .rule(bid(3, Strain::Notrump), 190, points(15..=17))
            .rule(bid(3, Suit::Clubs), 180, extremes() & len(Suit::Clubs, 4..))
            .rule(
                bid(2, Strain::Notrump),
                160,
                extremes() & len(Suit::Clubs, ..4),
            )
    }
}

/// Responder's game placement over opener's rebid of a swapped minor force
// ponytail: flat game, as the wide-1♣ package first shipped it.  Minor-suit
// games and the slam tries are owed.
fn place_game(opener_rebid: Bid) -> Rules {
    if opener_rebid == bid(3, Strain::Notrump) {
        return Rules::new().rule(Call::Pass, 0, hcp(0..));
    }
    // Capped at 15: a stronger hand has no row here and bids on with the
    // floor, and opener reads the game as a limited hand.
    let limited = || points(..=15);
    let rules = Rules::new();
    let rules = match (opener_rebid.level.get(), opener_rebid.strain) {
        // A five-card major at the two level: raise on three.
        (2, Strain::Hearts) => rules.rule(
            bid(4, Suit::Hearts),
            100,
            limited() & len(Suit::Hearts, 3..),
        ),
        (2, Strain::Spades) => rules.rule(
            bid(4, Suit::Spades),
            100,
            limited() & len(Suit::Spades, 3..),
        ),
        // A four-card major at the three level: raise on four.
        (3, Strain::Hearts) => rules.rule(
            bid(4, Suit::Hearts),
            100,
            limited() & len(Suit::Hearts, 4..),
        ),
        (3, Strain::Spades) => rules.rule(
            bid(4, Suit::Spades),
            100,
            limited() & len(Suit::Spades, 4..),
        ),
        _ => rules,
    };
    rules.rule(bid(3, Strain::Notrump), 50, limited())
}

/// Opener's rebid after Reverse Flannery `1♣ - 2♥!` (6–9, five spades, 4–5
/// hearts)
// ponytail: game is bid directly; the book's `2NT!` lebensohl and its `3♣!`
// and `3♦!` game-forcing asks are owed with their answers.
fn after_flannery() -> Rules {
    Rules::new()
        .rule(
            bid(4, Suit::Spades),
            220,
            points(17..) & len(Suit::Spades, 3..),
        )
        .rule(
            bid(4, Suit::Hearts),
            215,
            points(17..) & len(Suit::Hearts, 4..) & len(Suit::Spades, ..3),
        )
        .rule(
            bid(3, Strain::Notrump),
            210,
            hcp(18..) & len(Suit::Spades, ..3) & len(Suit::Hearts, ..4),
        )
        .rule(
            bid(3, Suit::Spades),
            150,
            points(15..=16) & len(Suit::Spades, 3..),
        )
        .rule(
            bid(3, Suit::Hearts),
            145,
            points(15..=16) & len(Suit::Hearts, 4..) & len(Suit::Spades, ..3),
        )
        .rule(
            Call::Pass,
            110,
            len(Suit::Hearts, 4..) | (len(Suit::Hearts, 3..=3) & len(Suit::Spades, ..2)),
        )
        .rule(bid(2, Suit::Spades), 100, hcp(0..))
}

/// Opener's rebid after `1♣ - 3♦!` (balanced, 12–15, no major)
///
/// The page gives no table: notrump by weight of points.
fn after_balanced_force() -> Rules {
    Rules::new()
        .rule(bid(6, Strain::Notrump), 120, hcp(20..))
        .rule(bid(4, Strain::Notrump), 110, hcp(18..=19))
        .rule(bid(3, Strain::Notrump), 50, hcp(0..))
}

/// A two-row table: `accept` under `when`, else the sign-off
///
/// The shape of every closing table below.  Each answers an artificial call
/// whose page gives no continuation, or whose continuation is owed, so that
/// the call is never passed: natural, to play, and no further.
fn close(
    accept: Bid,
    when: impl crate::bidding::constraint::Constraint + 'static,
    sign_off: Bid,
) -> Rules {
    Rules::new()
        .rule(accept, 100, when)
        .rule(sign_off, 50, hcp(0..))
}

/// Opener over the artificial invitations and the balanced game force
///
/// `1♣ - 2♠!` (six diamonds) and a passed hand's `2♣!`, `2♦!` and `2♠!`:
/// `3NT` on 15+, else the suit shown.  A passed hand's `3♦` is natural and
/// invitational: `3NT` on 14+, else pass.
// ponytail: no stopper check and no minor-suit game; the pages give none.
fn closing_entries() -> Vec<Entry> {
    let node = |key: &str, rules: Rules| rows_of(Pattern::node(key), rules);
    let game = bid(3, Strain::Notrump);
    let mut entries = node(
        "P* 1♣ - 2♠ -",
        close(game, hcp(15..), bid(3, Suit::Diamonds)),
    )
    .into_iter()
    .collect::<Vec<Entry>>();
    // The key above fans over every seat; the passed-hand `2♠!` is both
    // minors, where diamonds are still the cheaper preference.
    for seat in ["- -", "- - -"] {
        entries.extend(node(
            &format!("{seat} 1♣ - 3♦ -"),
            Rules::new()
                .rule(game, 100, hcp(14..))
                .rule(Call::Pass, 0, hcp(0..)),
        ));
        entries.extend(node(
            &format!("{seat} 1♣ - 2♣ -"),
            close(game, hcp(15..), bid(2, Suit::Diamonds)),
        ));
        entries.extend(node(
            &format!("{seat} 1♣ - 2♦ -"),
            close(game, hcp(15..), bid(3, Suit::Clubs)),
        ));
    }
    // Responder over the both-minors game force: notrump.
    entries.extend(node(
        "P* 1♣ - 1♥ - 2♠ -",
        Rules::new().rule(game, 50, hcp(0..)),
    ));
    entries.extend(node(
        "P* 1♣ - 1♠ - 3♥ -",
        Rules::new().rule(game, 50, hcp(0..)),
    ));
    // Opener over the mixed raise and the balanced slam try after
    // `1♣ - 1♦ - 2♣`.
    entries.extend(node(
        "P* 1♣ - 1♦ - 2♣ - 2NT -",
        close(game, points(16..), bid(3, Suit::Clubs)),
    ));
    entries.extend(node(
        "P* 1♣ - 1♦ - 2♣ - 3♦ -",
        Rules::new().rule(game, 50, hcp(0..)),
    ));
    for major in MAJORS {
        let m = Call::Bid(bid(1, major));
        // Responder over opener's 17–19 rebids after `1♣ - 1♦ - 1M - 1NT`:
        // game on 7+, else the major.
        entries.extend(node(
            &format!("P* 1♣ - 1♦ - {m} - 1NT - 2♦ -"),
            close(game, hcp(7..), bid(2, major)),
        ));
        entries.extend(node(
            &format!("P* 1♣ - 1♦ - {m} - 1NT - 2NT -"),
            close(bid(4, major), hcp(6..), bid(3, major)),
        ));
    }
    entries
}

/// Responder's second call after `1♣ - 1♥ - 1♠` (forcing)
///
/// The natural rows of the page and both checkbacks.  The `2♣!` puppet
/// carries the minimum with four spades, long hearts or long diamonds, and
/// the balanced invitation; `1NT` is what is left of the minimum.
// ponytail: the puppet's shaped invitations (`3♣`, `3♦`, `3♥`, `3♠` over the
// completion) and its 15–17 `3NT` are not authored; neither are the direct
// slam tries.  A minimum passes the completion only on six diamonds.
fn after_one_spade() -> Rules {
    let minimum = || points(..=10);
    Rules::new()
        .rule(
            bid(3, Suit::Spades),
            160,
            points(13..) & len(Suit::Spades, 4..),
        )
        .rule(
            bid(3, Suit::Hearts),
            155,
            points(13..) & len(Suit::Hearts, 6..),
        )
        .rule(bid(2, Suit::Diamonds), 150, points(13..))
        .alert(CHECKBACK)
        .rule(
            bid(2, Suit::Spades),
            140,
            points(11..=12) & len(Suit::Spades, 4..),
        )
        .rule(
            bid(2, Suit::Hearts),
            135,
            points(11..=12) & len(Suit::Hearts, 5..),
        )
        .rule(
            bid(2, Suit::Clubs),
            130,
            points(11..=12)
                | (minimum() & len(Suit::Spades, 4..))
                | (minimum() & len(Suit::Hearts, 6..))
                | (minimum() & len(Suit::Hearts, 5..=5) & !balanced())
                | (minimum() & len(Suit::Diamonds, 6..)),
        )
        .alert(PUPPET)
        .rule(bid(1, Strain::Notrump), 100, points(..=12))
}

/// Opener's answer to the puppet `1♣ - 1♥ - 1♠ - 2♣!`
///
/// 11–16 completes; a game force says so, with five spades or without.
// ponytail: two game-forcing answers for the page's seven, and no `2♥`
// invitation on three hearts.
fn after_puppet() -> Rules {
    Rules::new()
        .rule(
            bid(2, Suit::Spades),
            160,
            points(17..) & len(Suit::Spades, 5..),
        )
        .rule(bid(2, Strain::Notrump), 150, points(17..))
        .rule(bid(2, Suit::Diamonds), 100, points(..=16))
        .alert(PUPPET_COMPLETION)
}

/// Responder's hand type over the completion `1♣ - 1♥ - 1♠ - 2♣ - 2♦`
///
/// The four variants of the puppet, and nothing else: no other hand reaches
/// this node.
fn after_completion() -> Rules {
    let minimum = || points(..=10);
    Rules::new()
        .rule(
            bid(2, Suit::Spades),
            120,
            minimum() & len(Suit::Spades, 4..),
        )
        .rule(
            bid(2, Suit::Hearts),
            110,
            minimum() & len(Suit::Hearts, 5..),
        )
        .rule(Call::Pass, 105, minimum() & len(Suit::Diamonds, 6..))
        .rule(bid(2, Strain::Notrump), 100, points(11..=12))
}

/// Responder's game over a game-forcing answer to the puppet, or over
/// opener's forcing `2♠` after `1NT`: spades on a fit, hearts on six, else
/// notrump
///
/// `fit` is the spade length that makes eight: three facing five, four
/// facing four.
fn place_after_puppet(fit: usize) -> Rules {
    Rules::new()
        .rule(bid(4, Suit::Spades), 100, len(Suit::Spades, fit..))
        .rule(bid(4, Suit::Hearts), 90, len(Suit::Hearts, 6..))
        .rule(bid(3, Strain::Notrump), 50, hcp(0..))
}

/// Opener's third call after `1♣ - 1♥ - 1♠ - 1NT` (a minimum, at most three
/// spades)
// ponytail: `3NT` stands for the page's forcing `2NT` and its minor-suit
// jumps; the `2♦!` fourth suit and the `2♥` invitation are not authored.
fn after_one_spade_notrump() -> Rules {
    Rules::new()
        .rule(
            bid(2, Suit::Spades),
            160,
            points(17..) & len(Suit::Spades, 5..),
        )
        .rule(bid(3, Strain::Notrump), 150, points(17..))
        .rule(
            bid(2, Suit::Clubs),
            110,
            points(..=16) & len(Suit::Clubs, 5..) & !balanced(),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer to the game-forcing checkback `1♣ - 1♥ - 1♠ - 2♦!`
fn after_checkback() -> Rules {
    Rules::new()
        .rule(
            bid(3, Suit::Hearts),
            160,
            points(17..) & len(Suit::Spades, 5..) & len(Suit::Hearts, 3..=3),
        )
        .alert(CHECKBACK)
        .rule(
            bid(2, Suit::Spades),
            155,
            points(17..) & len(Suit::Spades, 5..) & len(Suit::Hearts, ..3),
        )
        .rule(
            bid(3, Suit::Diamonds),
            150,
            points(18..) & len(Suit::Diamonds, 5..) & len(Suit::Hearts, ..3),
        )
        .rule(
            bid(3, Suit::Clubs),
            145,
            points(15..) & len(Suit::Clubs, 5..) & len(Suit::Hearts, ..3),
        )
        .rule(bid(2, Suit::Hearts), 140, len(Suit::Hearts, 3..=3))
        .rule(bid(2, Strain::Notrump), 100, hcp(0..))
}

/// Responder's game over opener's answer to the checkback
// ponytail: game only, as elsewhere.
fn place_after_checkback(answer: Bid) -> Rules {
    let limited = || points(..=15);
    let rules = Rules::new();
    let rules = match (answer.level.get(), answer.strain) {
        // Three hearts opposite: play them on five.
        (_, Strain::Hearts) => rules.rule(
            bid(4, Suit::Hearts),
            100,
            limited() & len(Suit::Hearts, 5..),
        ),
        // Five spades opposite: play them on three.
        (2, Strain::Spades) => rules.rule(
            bid(4, Suit::Spades),
            100,
            limited() & len(Suit::Spades, 3..),
        ),
        _ => rules,
    };
    rules.rule(bid(3, Strain::Notrump), 50, limited())
}

/// The Polish `1♣` structure as one row package
pub(super) fn package() -> Package {
    Package {
        name: "strawberry-one-club",
        gate: |_| true,
        entries: |agreements: &Agreements| {
            let node = |key: &str, rules: Rules| rows_of(Pattern::node(key), rules);
            let pass = || Rules::new().rule(Call::Pass, 0, hcp(0..));
            let mut entries: Vec<Entry> =
                rows_of(Pattern::node("P* 1♣ -").with_fan(1), responses());
            for seat in ["- -", "- - -"] {
                entries.extend(node(&format!("{seat} 1♣ -"), passed_responses()));
            }
            entries.extend(node("P* 1♣ - 1♦ -", after_negative()));
            entries.extend(node("P* 1♣ - 1♦ - 2♣ -", after_negative_clubs()));
            entries.extend(node("P* 1♣ - 1♦ - 2♦ -", after_strong_rebid()));
            // 21–23 where the house opening is 20–21: every band one point
            // lower, and the quantitative `4NT` accepted on the upper two.
            entries.extend(two_notrump_rows(agreements, "P* 1♣ - 1♦ - 2♦ -", 1, 22));
            // The balanced game force, by an unpassed hand only: a passed
            // hand's `3♦` is the natural invitation `closing_entries` answers.
            entries.extend(rows_of(
                Pattern::node("P* 1♣ - 3♦ -").with_fan(1),
                after_balanced_force(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1♣ - 3♦ - 4NT -").with_fan(1),
                Rules::new()
                    .rule(bid(6, Strain::Notrump), 100, hcp(14..))
                    .rule(Call::Pass, 0, hcp(0..)),
            ));
            entries.extend(node("P* 1♣ - 1NT -", after_notrump()));
            entries.extend(node("P* 1♣ - 2NT -", after_jump_notrump()));
            entries.extend(node("P* 1♣ - 2♥ -", after_flannery()));
            entries.extend(closing_entries());
            entries.extend(node("P* 1♣ - 1♥ - 1♠ -", after_one_spade()));
            let answers = after_checkback();
            for rule in answers.rules() {
                if let Call::Bid(answer) = rule.call() {
                    let key = format!("P* 1♣ - 1♥ - 1♠ - 2♦ - {} -", rule.call());
                    entries.extend(node(&key, place_after_checkback(answer)));
                }
            }
            entries.extend(node("P* 1♣ - 1♥ - 1♠ - 2♦ -", answers));
            let puppet = "P* 1♣ - 1♥ - 1♠ - 2♣ -";
            entries.extend(node(puppet, after_puppet()));
            entries.extend(node(&format!("{puppet} 2♦ -"), after_completion()));
            entries.extend(node(&format!("{puppet} 2♦ - 2♥ -"), pass()));
            entries.extend(node(&format!("{puppet} 2♦ - 2♠ -"), pass()));
            entries.extend(node(
                &format!("{puppet} 2♦ - 2NT -"),
                Rules::new()
                    .rule(bid(3, Strain::Notrump), 100, hcp(14..))
                    .rule(Call::Pass, 0, hcp(0..)),
            ));
            entries.extend(node(&format!("{puppet} 2♠ -"), place_after_puppet(3)));
            entries.extend(node(&format!("{puppet} 2NT -"), place_after_puppet(4)));
            let notrump = "P* 1♣ - 1♥ - 1♠ - 1NT -";
            entries.extend(node(notrump, after_one_spade_notrump()));
            entries.extend(node(&format!("{notrump} 2♠ -"), place_after_puppet(3)));
            for major in MAJORS {
                let m = Call::Bid(bid(1, major));
                let strong = format!("P* 1♣ - 1♦ - {} -", Call::Bid(bid(2, major)));
                let game = Call::Bid(bid(4, major));
                entries.extend(node(&strong, after_strong_major(major)));
                entries.extend(node(&format!("{strong} {game} -"), pass()));
                entries.extend(node(
                    &format!("{strong} 3NT -"),
                    Rules::new().rule(bid(4, major), 100, len(major, 6..)).rule(
                        Call::Pass,
                        0,
                        hcp(0..),
                    ),
                ));
                entries.extend(node(
                    &format!("P* 1♣ - 1♦ - {m} -"),
                    after_negative_major(major),
                ));
                entries.extend(node(
                    &format!("P* 1♣ - 1♦ - {m} - 1NT -"),
                    after_negative_major_notrump(major),
                ));
                entries.extend(node(&format!("P* 1♣ - {m} -"), after_positive(major)));
                entries.extend(node(&format!("P* 1♣ - {m} - 2♦ -"), odwrotka_steps(major)));
                for (step, four_cards) in [
                    (bid(2, Suit::Hearts), true),
                    (bid(2, Suit::Spades), true),
                    (bid(2, Strain::Notrump), false),
                    (bid(3, Suit::Clubs), false),
                    (bid(3, Suit::Diamonds), false),
                    (bid(3, major), false),
                ] {
                    let step = Call::Bid(step);
                    entries.extend(node(
                        &format!("P* 1♣ - {m} - 2♦ - {step} -"),
                        after_odwrotka_step(major, four_cards),
                    ));
                }
            }
            for (force, shown) in [("2♣", Suit::Diamonds), ("2♦", Suit::Clubs)] {
                let rebids = after_swapped_minor(shown);
                // The seat fan stops at one leading pass: a passed hand's
                // `2♣`/`2♦` is an invitation, and opener's rebid over it is
                // the floor's.
                for rule in rebids.rules() {
                    if let Call::Bid(rebid) = rule.call() {
                        let key = format!("P* 1♣ - {force} - {} -", rule.call());
                        entries.extend(rows_of(Pattern::node(&key).with_fan(1), place_game(rebid)));
                    }
                }
                entries.extend(rows_of(
                    Pattern::node(&format!("P* 1♣ - {force} -")).with_fan(1),
                    rebids,
                ));
            }
            entries
        },
    }
}
