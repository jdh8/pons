//! The wide, non-forcing `1♣` and its `1♦!` relay — the Watermelon Dutch
//! Doubleton plugin (<https://jdh8.github.io/watermelon-dutch/>)
//!
//! Gated on [`OpeningKnobs::wide_one_club`][crate::bidding::agreements::OpeningKnobs::wide_one_club]
//! (**default off**).  The opening table's half — both minors 11–23, the
//! narrowed strong `2♣!` — lives in `openings.rs`; this module is everything
//! responder and opener say afterwards, compiled *after* the base packages so
//! its exact nodes re-own american's:
//!
//! * **Responder's first call** ([`responses`]) — american's natural ladder
//!   (majors up the line, the notrump rungs, the weak jump shifts, the
//!   preemptive `3♣`) around one artificial gadget: `1♦!` is a catch-all
//!   **relay** holding every hand with no other call — too short in clubs to
//!   pass, 6–11 with no four-card major and no notrump bid, or 16+ balanced
//!   (`3NT` is capped at 15 to leave them the relay).  `2♣`/`2♦` are
//!   *natural* invite+/game-force in the minor with no four-card major
//!   (american routes these to an inverted raise and nothing; both are
//!   alerted for the strength they carry).
//! * **Opener's rebid after the relay** ([`opener_after_relay`]) — the book's
//!   seven-row ladder that lets a 2+♣ opening carry 21–23 without dropping
//!   the hand: cheap naturals at 11–17, `1NT`/`2M`/`2NT!` at 18–20, the
//!   artificial `2♦!` catch-all at 21–23.
//! * **Responder's second call** over opener's *minimum* rebids
//!   ([`after_relay_major`], [`after_relay_club`]) — the book leaves these to
//!   the host, but the host's nodes read `1♦` as natural diamonds, so a thin
//!   natural ladder is authored: weak signoffs, both minors, the inverted club
//!   raise, and the 16+ balanced `2NT`.  The tails after opener's rare `1NT`
//!   (18–20) and `2♦!` (21–23) stay american's — a known misread, deferred.
//! * **The natural minor two-bids** — opener's answers to the game-forcing
//!   `2♦` ([`opener_after_two_diamonds`]) and the invitational `2♣`
//!   ([`opener_after_two_clubs`]), and responder's game placement over each
//!   ([`responder_after_two_diamonds`], [`responder_after_two_clubs`]).  These
//!   were the Dutch system's Phase 2.2 increment, whose responder side flipped
//!   an opener-only loss into a measured win (`docs/archive/dutch-system.md`).

use super::responses::{INVERTED_MINOR, WEAK_JUMP_SHIFT, with_major_selection};
use crate::bidding::agreements::Agreements;
use crate::bidding::constraint::{balanced, hcp, len, points, stopper_in, support, support_points};
use crate::bidding::rows::{Entry, Package, Pattern, compile_into, rows_of};
use crate::bidding::{Alert, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// The artificial `1♦` relay — the wide-1♣ catch-all response
const RELAY: Alert = Alert("wide-1c:relay");
/// Natural but game-forcing `2♦` (5+♦) — alerted so partner reads the force
const GAME_FORCE: Alert = Alert("wide-1c:game-force");
/// Natural invitational-or-better `2♣` (5+♣) — alerted so partner reads
/// invite+, not american's inverted minor raise
const INVITE_PLUS: Alert = Alert("wide-1c:invite-plus");
/// Opener's artificial 21–23 catch-all rebid after the relay
const STRONG_REBID: Alert = Alert("wide-1c:strong-rebid");
/// Opener's `2NT!` rebid after the relay: 18–20 with six-plus clubs
const CLUB_REBID_2NT: Alert = Alert("wide-1c:2nt-clubs");
/// Opener's `2M` rebid after the relay: 18–20, four-plus in the major **and**
/// five-plus clubs — it floors the unnamed club suit, so it is artificial by
/// the house rule and decoded by rule projection (like american's reverse)
const RELAY_REVERSE: Alert = Alert("wide-1c:reverse");
/// Both minors (5+/4+), invitational — the "other major" repurposed after
/// the relay, since responder denied a four-card major on round one
const BOTH_MINORS: Alert = Alert("wide-1c:both-minors");
/// An invitational club raise (9–11, 4+♣) after opener's `2♣`, shown by the
/// artificial `2♠` (inverted: the cheaper call is the stronger raise)
const CLUB_RAISE_INV: Alert = Alert("wide-1c:club-raise-inv");
/// Responder's 16+ balanced after the relay — a meaning inversion vs
/// american's invite, alerted so projection discloses the slam-going strength
const STRONG_BALANCED: Alert = Alert("wide-1c:16-balanced");

/// Responder's first call over the wide `1♣`
///
/// American's ladder with the book's three changes: `1♦!` relays instead of
/// showing diamonds, `2♣`/`2♦` are natural minor bids (invite+ / game force)
/// instead of an inverted raise and nothing, and `3NT` is capped at 15 so the
/// 16+ balanced hand relays.  A weak hand content to play `1♣` passes; a weak
/// hand with fewer than four clubs relays out.
fn responses(agreements: &Agreements) -> Rules {
    let no_major = || len(Suit::Hearts, ..4) & len(Suit::Spades, ..4);
    let mut rules = with_major_selection(Rules::new(), agreements);
    rules = rules
        // 2♦ game-forcing, natural diamonds — alerted for the force.
        .rule(
            Bid::new(2, Strain::Diamonds),
            130,
            len(Suit::Diamonds, 5..) & points(13..) & no_major(),
        )
        .alert(GAME_FORCE)
        // 2♣ invitational-or-better, natural clubs.
        .rule(
            Bid::new(2, Strain::Clubs),
            125,
            len(Suit::Clubs, 5..) & points(11..) & no_major(),
        )
        .alert(INVITE_PLUS)
        // The notrump ladder without a four-card major; 3NT capped at 15 so a
        // 16+ balanced hand takes the relay and rightsides the notrump.
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            hcp(13..=15) & balanced() & no_major(),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            100,
            hcp(11..=12) & balanced() & no_major(),
        )
        .rule(Bid::new(1, Strain::Notrump), 50, hcp(6..=10) & no_major())
        // Preemptive club raise, as american plays it.
        .rule(
            Bid::new(3, Strain::Clubs),
            110,
            support(5..) & support_points(Suit::Clubs, ..=9),
        )
        .alert(INVERTED_MINOR)
        // 1♦! relay — everything the naturals above did not take: six-plus
        // with nothing to say, or too short in clubs to pass.
        .rule(
            Bid::new(1, Strain::Diamonds),
            30,
            hcp(6..) | len(Suit::Clubs, ..4),
        )
        .alert(RELAY)
        // Weak with club tolerance — content to play 1♣.
        .rule(Call::Pass, 0, hcp(..6));
    // Weak jump shifts, as american plays them.
    for x in [Suit::Hearts, Suit::Spades] {
        rules = rules
            .rule(
                Bid::new(2, Strain::from(x)),
                100,
                len(x, 6..) & points(2..=5),
            )
            .alert(WEAK_JUMP_SHIFT);
    }
    rules
}

/// Opener's rebid after the `1♣ - 1♦!` relay — the book's seven rows
///
/// Responder holds at most three cards in each major, so clubs come before a
/// four-card major at 11–17: `2♣` on five-plus, else a four-card major or a
/// balanced minimum's three-card major (a minimum may **not** rebid `1NT` —
/// that is the 18–20 slot).  At 18–20: `1NT` with no five-card major, no
/// six-card minor and no void, `2M` a four-card major with five-plus clubs,
/// `2NT!` six-plus clubs; `3♣` is 15–17 with six-plus clubs.  At 21–23 the
/// artificial `2♦!` catch-all.  The relay is forcing, so the guaranteed-legal
/// fallback is american's `1NT`, never a pass.
fn opener_after_relay() -> Rules {
    Rules::new()
        // 18–20 with six-plus clubs.
        .rule(
            Bid::new(2, Strain::Notrump),
            135,
            hcp(18..=20) & len(Suit::Clubs, 6..),
        )
        .alert(CLUB_REBID_2NT)
        // 18–20 reverse into a four-card major with five-plus clubs, up the line.
        .rule(
            Bid::new(2, Strain::Hearts),
            130,
            hcp(18..=20) & len(Suit::Hearts, 4..) & len(Suit::Clubs, 5..),
        )
        .alert(RELAY_REVERSE)
        .rule(
            Bid::new(2, Strain::Spades),
            125,
            hcp(18..=20) & len(Suit::Spades, 4..) & len(Suit::Hearts, ..4) & len(Suit::Clubs, 5..),
        )
        .alert(RELAY_REVERSE)
        // 18–20, no five-card major, no six-card minor, no void.
        .rule(
            Bid::new(1, Strain::Notrump),
            120,
            hcp(18..=20)
                & len(Suit::Spades, 1..=4)
                & len(Suit::Hearts, 1..=4)
                & len(Suit::Diamonds, 1..=4)
                & len(Suit::Clubs, 2..=5),
        )
        // 15–17 with six-plus clubs.
        .rule(
            Bid::new(3, Strain::Clubs),
            112,
            hcp(15..=17) & len(Suit::Clubs, 6..),
        )
        // 11–17 with five-plus clubs.
        .rule(
            Bid::new(2, Strain::Clubs),
            110,
            hcp(11..=17) & len(Suit::Clubs, 5..),
        )
        // 21–23, any shape — the artificial catch-all.
        .rule(Bid::new(2, Strain::Diamonds), 105, hcp(21..=23))
        .alert(STRONG_REBID)
        // 11–17: a four-card major, up the line …
        .rule(
            Bid::new(1, Strain::Hearts),
            100,
            hcp(11..=17) & len(Suit::Hearts, 4..),
        )
        .rule(
            Bid::new(1, Strain::Spades),
            98,
            hcp(11..=17) & len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        // … or a balanced minimum's three-card major.
        .rule(
            Bid::new(1, Strain::Hearts),
            96,
            hcp(11..=17) & balanced() & len(Suit::Hearts, 3..=3),
        )
        .rule(
            Bid::new(1, Strain::Spades),
            94,
            hcp(11..=17) & balanced() & len(Suit::Spades, 3..=3) & len(Suit::Hearts, ..3),
        )
        // Guaranteed-legal fallback over a forcing relay (american's idiom).
        .rule(Bid::new(1, Strain::Notrump), 20, hcp(0..))
}

/// Responder's second call after `1♣ - 1♦! - 1M` (opener 11–17, 3+ in the major)
///
/// A natural ladder around two gadgets.  The **other major** (`2OM!`) is
/// repurposed to both minors (5+/4+, invite): a natural major here is
/// impossible, since a four-card major bid up the line on round one.  `2NT`
/// is the 16+ balanced hand that relayed past `3NT`'s cap, rightsiding the
/// notrump.  Everything else is natural — weak `1♠`/`1NT`, natural `2♣`/`2♦`,
/// the `3♣` shape jump — over a `Pass` catch-all.
fn after_relay_major(opener: Suit) -> Rules {
    let other = if opener == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    // Both minors: 4-4 with at least one five-bagger.
    let both_minors = || {
        len(Suit::Clubs, 4..)
            & len(Suit::Diamonds, 4..)
            & (len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..))
    };
    let mut rules = Rules::new()
        // 2OM! = both minors 5+/4+, 9–11 invite.
        .rule(
            Bid::new(2, Strain::from(other)),
            145,
            both_minors() & points(9..=11),
        )
        .alert(BOTH_MINORS)
        // 2NT = 16+ balanced, game-forcing (rightsides the notrump).
        .rule(Bid::new(2, Strain::Notrump), 140, hcp(16..) & balanced())
        .alert(STRONG_BALANCED)
        // 3♣ = 6–9, 6+♣ — the shapely jump.
        .rule(
            Bid::new(3, Strain::Clubs),
            130,
            len(Suit::Clubs, 6..) & points(6..=9),
        )
        // 2♦ = 5–9, 6+♦.
        .rule(
            Bid::new(2, Strain::Diamonds),
            125,
            len(Suit::Diamonds, 6..) & points(5..=9),
        )
        // 2♣ = 0–9, 5+♣.
        .rule(
            Bid::new(2, Strain::Clubs),
            115,
            len(Suit::Clubs, 5..) & points(0..=9),
        );
    // 1♠ = 0–5, 4+♠ — only after 1♥ (a six-count bid it on round one).
    if opener == Suit::Hearts {
        rules = rules.rule(
            Bid::new(1, Strain::Spades),
            120,
            len(Suit::Spades, 4..) & points(0..=5),
        );
    }
    rules
        // 1NT = natural, weak balanced (usually 5–7).
        .rule(Bid::new(1, Strain::Notrump), 100, hcp(5..=7))
        // Finite catch-all — a weak hand content to pass opener's minimum.
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's second call after `1♣ - 1♦! - 2♣` (opener 11–17, 5+♣)
///
/// Club support splits inverted: the artificial `2♠!` is the **invitational**
/// raise (9–11), the natural `3♣` the **minimum** one (7–9).  `2♦` is natural,
/// `2NT` the 16+ balanced rightside, `Pass` the catch-all.
fn after_relay_club() -> Rules {
    Rules::new()
        // 2♠! = 9–11, 4+♣ — the invitational club raise (inverted: cheaper = stronger).
        .rule(
            Bid::new(2, Strain::Spades),
            145,
            len(Suit::Clubs, 4..) & points(9..=11),
        )
        .alert(CLUB_RAISE_INV)
        // 2NT = 16+ balanced, game-forcing.
        .rule(Bid::new(2, Strain::Notrump), 140, hcp(16..) & balanced())
        .alert(STRONG_BALANCED)
        // 2♦ = 7–9, 5+♦.
        .rule(
            Bid::new(2, Strain::Diamonds),
            125,
            len(Suit::Diamonds, 5..) & points(7..=9),
        )
        // 3♣ = 7–9, 4+♣ — the natural minimum raise.
        .rule(
            Bid::new(3, Strain::Clubs),
            110,
            len(Suit::Clubs, 4..) & points(7..=9),
        )
        // Finite catch-all.
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's rebid after `1♣ - 2♦` (responder game-forcing, 5+♦, no four-card major)
///
/// A game force with **no major fit possible** — opener denied a five-card
/// major, responder a four-card one — so the live questions are the strain
/// (diamonds / clubs / notrump) and slam.  Opener raises responder's diamonds
/// (a known nine-card fit, the best news), introduces a real club suit, shows
/// a single major stopper up the line toward 3NT, or bids notrump by strength.
/// Forcing, so the catch-all is a bid (`2NT`), never Pass.
// ponytail: caps at game; add RKCB on the 3♦ diamond-fit branch if an A/B
// shows the slam tail losing.
fn opener_after_two_diamonds() -> Rules {
    Rules::new()
        // 3♦ — four-card diamond support: a known nine-card fit, the best news.
        .rule(Bid::new(3, Strain::Diamonds), 145, len(Suit::Diamonds, 4..))
        // 3♣ — a real five-card club suit, no diamond support (minor two-suiter).
        .rule(
            Bid::new(3, Strain::Clubs),
            135,
            len(Suit::Clubs, 5..) & len(Suit::Diamonds, ..4),
        )
        // 3NT — balanced extras, both majors stopped, to play.
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            balanced() & hcp(15..) & stopper_in(Suit::Hearts) & stopper_in(Suit::Spades),
        )
        // 2♥ / 2♠ — a single major stopper, shown up the line toward 3NT (a
        // both-stopped hand is excluded and falls to the notrump catch-all).
        .rule(
            Bid::new(2, Strain::Hearts),
            100,
            stopper_in(Suit::Hearts) & !stopper_in(Suit::Spades),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            95,
            stopper_in(Suit::Spades) & !stopper_in(Suit::Hearts),
        )
        // Finite catch-all — a minimum, or both-major stoppers without extras:
        // bid notrump and let responder place the game (never Pass).
        .rule(Bid::new(2, Strain::Notrump), 50, hcp(0..))
}

/// Opener's rebid after `1♣ - 2♣` (responder invitational-or-better, 5+♣, no major)
///
/// Same no-major-fit world as the game-forcing `2♦`, but `2♣` is only
/// **invite+**, so opener must be able to stop.  Opener accepts to game with a
/// maximum (`3NT` — balanced-and-stopped, or forced by 17+ opposite the
/// invite's 11+), otherwise declines non-forcing: `3♣` raises responder's
/// known suit, `2NT` is the balanced-minimum catch-all.
// ponytail: no game-try rung; add 2♥/2♠ help-suit tries (with an authored
// responder node to read them) if an A/B shows thin invited games missed.
fn opener_after_two_clubs() -> Rules {
    Rules::new()
        // 3NT — accept to game: balanced maximum, both majors stopped.
        .rule(
            Bid::new(3, Strain::Notrump),
            130,
            balanced() & hcp(14..) & stopper_in(Suit::Hearts) & stopper_in(Suit::Spades),
        )
        // 3NT — accept to game: a 17+ maximum forces even stopper-shy, since
        // opposite the 11+ invite the partnership holds 28+.
        .rule(Bid::new(3, Strain::Notrump), 110, hcp(17..))
        // 3♣ — decline: club support, non-forcing (capped at 16 so a maximum
        // can never leave this in).
        .rule(
            Bid::new(3, Strain::Clubs),
            100,
            len(Suit::Clubs, 3..) & hcp(..=16),
        )
        // 2NT — decline / finite catch-all: balanced minimum, non-forcing.
        .rule(Bid::new(2, Strain::Notrump), 90, hcp(0..))
}

/// Responder's continuation after `1♣ - 2♦` (game force), keyed on opener's rebid
///
/// Honour the force without blasting slam blind: over every descriptive rebid
/// responder names the game (`3NT`); over opener's own `3NT` (a balanced 15+
/// to play) responder passes.  Natural throughout, so no alert.
// ponytail: flat 3NT.  Refine to 5m / RKCB on the diamond-fit branch only if
// an A/B says the game cap leaks slams.
fn responder_after_two_diamonds(opener_rebid: Bid) -> Rules {
    if opener_rebid == Bid::new(3, Strain::Notrump) {
        return Rules::new().rule(Call::Pass, 0, hcp(0..));
    }
    Rules::new().rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
}

/// Responder's continuation after `1♣ - 2♣` (invite+), keyed on opener's rebid
///
/// Over opener's `3NT` responder passes.  Over a non-forcing decline (`3♣` or
/// `2NT`) responder passes with the pure invite and drives `3NT` with the
/// game-forcing end of invite+ (12+ opposite opener's up-to-16 decline).
fn responder_after_two_clubs(opener_rebid: Bid) -> Rules {
    if opener_rebid == Bid::new(3, Strain::Notrump) {
        return Rules::new().rule(Call::Pass, 0, hcp(0..));
    }
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, points(12..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// The wide-1♣ structure as one gated row package
pub(super) fn package() -> Package {
    Package {
        name: "wide-one-club",
        gate: |agreements| agreements.opening.wide_one_club,
        entries: |agreements| {
            let mut entries: Vec<Entry> = rows_of(Pattern::node("P* 1♣ -"), responses(agreements));
            entries.extend(rows_of(Pattern::node("P* 1♣ - 1♦ -"), opener_after_relay()));
            entries.extend(rows_of(
                Pattern::node("P* 1♣ - 1♦ - 1♥ -"),
                after_relay_major(Suit::Hearts),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1♣ - 1♦ - 1♠ -"),
                after_relay_major(Suit::Spades),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1♣ - 1♦ - 2♣ -"),
                after_relay_club(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1♣ - 2♦ -"),
                opener_after_two_diamonds(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1♣ - 2♣ -"),
                opener_after_two_clubs(),
            ));
            for (pattern, rebid) in [
                ("P* 1♣ - 2♦ - 3♦ -", Bid::new(3, Strain::Diamonds)),
                ("P* 1♣ - 2♦ - 3♣ -", Bid::new(3, Strain::Clubs)),
                ("P* 1♣ - 2♦ - 3NT -", Bid::new(3, Strain::Notrump)),
                ("P* 1♣ - 2♦ - 2♥ -", Bid::new(2, Strain::Hearts)),
                ("P* 1♣ - 2♦ - 2♠ -", Bid::new(2, Strain::Spades)),
                ("P* 1♣ - 2♦ - 2NT -", Bid::new(2, Strain::Notrump)),
            ] {
                entries.extend(rows_of(
                    Pattern::node(pattern),
                    responder_after_two_diamonds(rebid),
                ));
            }
            for (pattern, rebid) in [
                ("P* 1♣ - 2♣ - 3NT -", Bid::new(3, Strain::Notrump)),
                ("P* 1♣ - 2♣ - 3♣ -", Bid::new(3, Strain::Clubs)),
                ("P* 1♣ - 2♣ - 2NT -", Bid::new(2, Strain::Notrump)),
            ] {
                entries.extend(rows_of(
                    Pattern::node(pattern),
                    responder_after_two_clubs(rebid),
                ));
            }
            entries
        },
    }
}

/// Register the wide-1♣ overlay (a no-op unless the knob is on)
pub(super) fn register(book: &mut Trie, agreements: &Agreements) {
    compile_into(book, agreements, &[package()]);
}

#[cfg(test)]
mod tests;
