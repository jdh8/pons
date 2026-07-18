//! Dutch responses to the wide 1♣ opening — the load-bearing convention (Phase 2)
//!
//! The nodes that diverge hard from american are authored here; the rare 18–20
//! `1NT` and 21–23 `2♦!` relay rebids' continuations are still `bare_american`
//! (their strength self-discloses to the floor via projection).  See
//! `docs/dutch-system.md` for the full spec.
//!
//! * **Responder's first call** ([`one_club_responses`]) — a natural ladder
//!   around one artificial gadget: `1♦!` is a catch-all **relay**, holding
//!   every hand the naturals don't take (constructive values, or too short in
//!   clubs to pass, or a strong 16+ with no descriptive bid).  `2♣`/`2♦` are
//!   *natural* invite+/game-force in the minor (american routes these to an
//!   inverted raise and a weak jump shift — Dutch overwrites both).
//! * **Opener's rebid after the relay** ([`opener_rebids_after_relay`]) — the
//!   clarification ladder that lets the wide 1♣ carry 21–23 without dropping
//!   the hand: cheap natural rebids for 11–17, jumps/reverses for 18–20, and an
//!   artificial `2♦!` catch-all for 21–23.
//! * **Responder's second call** over opener's *minimum* rebids (Phase 2.2) —
//!   [`relay_responses_after_major`] (`1♣-1♦-1M`) and [`relay_responses_after_club`]
//!   (`1♣-1♦-2♣`): natural ladders around Reverse Flannery, a both-minors
//!   repurposing of the "other major", and inverted club raises.

use crate::bidding::constraint::{balanced, hcp, len, points, stopper_in};
use crate::bidding::{Alert, Rules};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// The artificial `1♦` relay — the wide-1♣ catch-all response
const RELAY: Alert = Alert("dutch-1c:relay");
/// Natural but game-forcing `2♦` (5+♦) — alerted so partner reads the force,
/// not american's weak jump shift.
const GAME_FORCE: Alert = Alert("dutch-1c:game-force");
/// Natural invitational-or-better `2♣` (5+♣) — alerted so partner reads
/// invite+, not american's inverted minor raise.
const INVITE_PLUS: Alert = Alert("dutch-1c:invite-plus");
/// A weak jump shift — a single jump showing a weak six-card suit
const WEAK_JUMP: Alert = Alert("dutch-1c:weak-jump");
/// A shapely invitational minor jump — six-plus cards, 9–11
const MINOR_INVITE: Alert = Alert("dutch-1c:minor-invite");
/// A preemptive major jump — seven-plus cards, 3–6
const MAJOR_PREEMPT: Alert = Alert("dutch-1c:major-preempt");
/// Opener's artificial 21–23 catch-all rebid after the relay (`2♦`, generic)
const STRONG_REBID: Alert = Alert("dutch-1c:strong-rebid");
/// Reverse Flannery — the 7–9, exactly-5♠, 4–5♥ two-suiter, shown late through
/// the relay (a raise of opener's major, or `2♥` over opener's `2♣`)
const REVERSE_FLANNERY: Alert = Alert("dutch-1c:reverse-flannery");
/// Both minors (5+/4+), invitational — the "other major" repurposed, since a
/// major fit that mattered was already found on the relay
const BOTH_MINORS: Alert = Alert("dutch-1c:both-minors");
/// An invitational club raise (9–11, 4+♣) after opener's `2♣`, shown by the
/// artificial `2♠` (inverted: the cheaper call is the stronger raise)
const CLUB_RAISE_INV: Alert = Alert("dutch-1c:club-raise-inv");
/// Responder's 16+ balanced — a meaning inversion vs american's invite, alerted
/// so projection discloses the slam-going strength (rightsides the notrump)
const STRONG_BALANCED: Alert = Alert("dutch-1c:16-balanced");

/// Responder's first call over the wide `1♣`
///
/// A natural ladder — four-card majors up the line (7+; six-counts fall to the
/// relay), `2♣`/`2♦` natural invite+/game-force in the minor, the notrump
/// ladder without a four-card major, weak jump shifts (exactly six cards) and
/// shapely minor invites / major preempts — resting on the `1♦!` relay and a
/// `Pass` catch-all.  The relay soaks up everything with constructive values or
/// too short in clubs to pass; the weakest club-tolerant hands pass 1♣.
pub(super) fn one_club_responses() -> Rules {
    let no_major = || len(Suit::Hearts, ..4) & len(Suit::Spades, ..4);
    Rules::new()
        // Natural four-card majors, up the line (7+; six-counts take the relay).
        .rule(
            Bid::new(1, Strain::Hearts),
            1.5,
            len(Suit::Hearts, 4..) & points(7..),
        )
        .rule(
            Bid::new(1, Strain::Spades),
            1.4,
            len(Suit::Spades, 4..) & points(7..) & len(Suit::Hearts, ..4),
        )
        // 2♦ game-forcing, natural diamonds — alerted for the force.
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.3,
            len(Suit::Diamonds, 5..) & points(13..) & no_major(),
        )
        .alert(GAME_FORCE)
        // 2♣ invitational-or-better, natural clubs.
        .rule(
            Bid::new(2, Strain::Clubs),
            1.2,
            len(Suit::Clubs, 5..) & points(11..) & no_major(),
        )
        .alert(INVITE_PLUS)
        // Balanced invite / to-play, no four-card major.
        .rule(
            Bid::new(2, Strain::Notrump),
            1.05,
            hcp(10..=11) & balanced() & no_major(),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(12..=15) & balanced() & no_major(),
        )
        // Weak jump shifts — exactly six cards (seven-plus preempts a level higher).
        .rule(
            Bid::new(2, Strain::Hearts),
            1.0,
            len(Suit::Hearts, 6..=6) & points(0..=6),
        )
        .alert(WEAK_JUMP)
        .rule(
            Bid::new(2, Strain::Spades),
            1.0,
            len(Suit::Spades, 6..=6) & points(0..=6),
        )
        .alert(WEAK_JUMP)
        // 1NT natural, 8–10, no four-card major.
        .rule(Bid::new(1, Strain::Notrump), 0.9, hcp(8..=10) & no_major())
        // Shapely invitational minor jumps — six-plus cards, 9–11.
        .rule(
            Bid::new(3, Strain::Clubs),
            0.85,
            len(Suit::Clubs, 6..) & points(9..=11) & no_major(),
        )
        .alert(MINOR_INVITE)
        .rule(
            Bid::new(3, Strain::Diamonds),
            0.85,
            len(Suit::Diamonds, 6..) & points(9..=11) & no_major(),
        )
        .alert(MINOR_INVITE)
        // Preemptive major jumps — seven-plus cards, 3–6.
        .rule(
            Bid::new(3, Strain::Hearts),
            0.8,
            len(Suit::Hearts, 7..) & points(3..=6),
        )
        .alert(MAJOR_PREEMPT)
        .rule(
            Bid::new(3, Strain::Spades),
            0.8,
            len(Suit::Spades, 7..) & points(3..=6),
        )
        .alert(MAJOR_PREEMPT)
        // 1♦ artificial relay — constructive values, or too short in clubs to
        // pass: everything the naturals above didn't take.
        .rule(
            Bid::new(1, Strain::Diamonds),
            0.3,
            hcp(5..) | len(Suit::Clubs, ..3),
        )
        .alert(RELAY)
        // Weak with club tolerance — content to play 1♣.
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's rebid after the `1♣-1♦` relay
///
/// The clarification ladder that pays for the wide 1♣.  Minimum (11–17) hands
/// rebid cheaply — a three-card major, or five-plus clubs; medium (18–20) hands
/// jump or reverse (`1NT` balanced, `2M` four-card major, `3♣` six-plus clubs);
/// maximum (21–23) hands take the artificial `2♦!` catch-all (the source
/// system's `2NT!` 5-5-minor rebid is unreachable in pons — every 5+♦ hand
/// opens `1♦`).  A minimum balanced hand may **not** rebid 1NT (that is the
/// 18–20 slot) — it shows a three-card major or five clubs instead.  `2♣` spans
/// 11–20 so no five-club hand without a jump falls through; opener resolves the
/// exact band on the next round (Phase 2.2).
pub(super) fn opener_rebids_after_relay() -> Rules {
    // Note: the source system's `2NT!` rebid (21–23, 5+♦ 5+♣) is unreachable in
    // pons — every 5+♦ hand (5-5 minors included) opens `1♦`, so it never
    // reaches `1♣-1♦`.  Dropped rather than ship dead code; all 21–23 hands
    // that arrive here take the `2♦!` catch-all below.
    Rules::new()
        // 18–20 with six-plus clubs.
        .rule(
            Bid::new(3, Strain::Clubs),
            1.35,
            hcp(18..=20) & len(Suit::Clubs, 6..),
        )
        // 18–20 reverse into a four-card major, up the line.
        .rule(
            Bid::new(2, Strain::Hearts),
            1.3,
            hcp(18..=20) & len(Suit::Hearts, 4..),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            1.25,
            hcp(18..=20) & len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        // 18–20 balanced — a minimum balanced hand may NOT rebid 1NT (below).
        .rule(Bid::new(1, Strain::Notrump), 1.2, hcp(18..=20) & balanced())
        // 11–20 with five-plus clubs (band resolved on opener's next round).
        .rule(
            Bid::new(2, Strain::Clubs),
            1.1,
            hcp(11..=20) & len(Suit::Clubs, 5..),
        )
        // 21–23 no specific shape — artificial catch-all (diamond reversals dropped).
        .rule(Bid::new(2, Strain::Diamonds), 1.05, hcp(21..=23))
        .alert(STRONG_REBID)
        // 11–17 minimum — a three-card major, up the line.
        .rule(
            Bid::new(1, Strain::Hearts),
            1.0,
            hcp(11..=17) & len(Suit::Hearts, 3..),
        )
        .rule(
            Bid::new(1, Strain::Spades),
            0.95,
            hcp(11..=17) & len(Suit::Spades, 3..) & len(Suit::Hearts, ..3),
        )
        // Finite catch-all (opener is always 11–23; guards impossible hands).
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Responder's second call after `1♣-1♦-1M` (opener minimum, 11–17, 3+ in the
/// major)
///
/// A natural ladder around two artificial gadgets.  The **raise** of opener's
/// major (`2M!`) is Reverse Flannery — exactly the 7–9 / 5=♠ / 4–5♥ two-suiter
/// that took the relay to dodge the `1♣-1♠-2♣` rebid squeeze — so an ordinary
/// invitational raiser (who would have raised or bid on round one) never arrives
/// and needs no call.  The **other major** (`2OM!`) is repurposed to both minors
/// (5+/4+, invite): a natural major here is impossible (real four-card majors bid
/// up the line on round one), so the bid is free.  Everything else is natural —
/// weak `1♠`/`1NT`, natural `2♣`/`2♦`, the `3♣` shape jump — over the strong
/// `2NT` (16+ balanced, rightsiding the notrump) and a `Pass` catch-all.
pub(super) fn relay_responses_after_major(opener: Suit) -> Rules {
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
        // 2M! raise = Reverse Flannery: exactly 5♠, 4–5♥, 7–9.
        .rule(
            Bid::new(2, Strain::from(opener)),
            1.5,
            len(Suit::Spades, 5..=5) & len(Suit::Hearts, 4..=5) & points(7..=9),
        )
        .alert(REVERSE_FLANNERY)
        // 2OM! = both minors 5+/4+, 9–11 invite.
        .rule(
            Bid::new(2, Strain::from(other)),
            1.45,
            both_minors() & points(9..=11),
        )
        .alert(BOTH_MINORS)
        // 2NT = 16+ balanced, game-forcing (rightsides the notrump).
        .rule(Bid::new(2, Strain::Notrump), 1.4, hcp(16..) & balanced())
        .alert(STRONG_BALANCED)
        // 3♣ = 6–9, 6+♣ — the shapely jump.
        .rule(
            Bid::new(3, Strain::Clubs),
            1.3,
            len(Suit::Clubs, 6..) & points(6..=9),
        )
        // 2♦ = 5–9, 6+♦.
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.25,
            len(Suit::Diamonds, 6..) & points(5..=9),
        )
        // 2♣ = 0–9, 5+♣.
        .rule(
            Bid::new(2, Strain::Clubs),
            1.15,
            len(Suit::Clubs, 5..) & points(0..=9),
        );
    // 1♠ = 0–6, 4+♠ — only after 1♥ (over 1♠ the call is unavailable).
    if opener == Suit::Hearts {
        rules = rules.rule(
            Bid::new(1, Strain::Spades),
            1.2,
            len(Suit::Spades, 4..) & points(0..=6),
        );
    }
    rules
        // 1NT = natural, weak balanced (usually 5–7).
        .rule(Bid::new(1, Strain::Notrump), 1.0, hcp(5..=7))
        // Finite catch-all — a weak hand content to pass opener's minimum.
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Responder's second call after `1♣-1♦-2♣` (opener minimum, 11–17, 5+♣)
///
/// The earlier relay frees the low bids to turn conventional.  `2♥!` is the same
/// Reverse Flannery two-suiter (a natural heart suit is impossible — it bids up
/// the line on round one).  Club support splits inverted: the artificial `2♠!`
/// is the **invitational** raise (9–11), the natural `3♣` the **minimum** one
/// (7–9).  `2♦` is natural, `2NT` the 16+ balanced rightside, `Pass` the
/// catch-all.
pub(super) fn relay_responses_after_club() -> Rules {
    Rules::new()
        // 2♥! = Reverse Flannery: exactly 5♠, 4–5♥, 7–9.
        .rule(
            Bid::new(2, Strain::Hearts),
            1.5,
            len(Suit::Spades, 5..=5) & len(Suit::Hearts, 4..=5) & points(7..=9),
        )
        .alert(REVERSE_FLANNERY)
        // 2♠! = 9–11, 4+♣ — the invitational club raise (inverted: cheaper = stronger).
        .rule(
            Bid::new(2, Strain::Spades),
            1.45,
            len(Suit::Clubs, 4..) & points(9..=11),
        )
        .alert(CLUB_RAISE_INV)
        // 2NT = 16+ balanced, game-forcing.
        .rule(Bid::new(2, Strain::Notrump), 1.4, hcp(16..) & balanced())
        .alert(STRONG_BALANCED)
        // 2♦ = 7–9, 5+♦.
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.25,
            len(Suit::Diamonds, 5..) & points(7..=9),
        )
        // 3♣ = 7–9, 4+♣ — the natural minimum raise.
        .rule(
            Bid::new(3, Strain::Clubs),
            1.1,
            len(Suit::Clubs, 4..) & points(7..=9),
        )
        // Finite catch-all.
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's rebid after `1♣-2♦` (responder game-forcing, 5+♦, no four-card major)
///
/// A game force with **no major fit possible** — opener denied a five-card major
/// by opening 1♣, responder denied a four-card major — so the only live questions
/// are the strain (diamonds / clubs / notrump) and slam.  Opener borrows the
/// inverted-minor ladder (american's `1♦-2♦` continuation): raise responder's
/// diamonds (a known nine-card fit, the best news — and the wide 1♣ hosts most
/// four-diamond hands), introduce a real club suit, show a single major stopper
/// up the line toward 3NT, or bid notrump by strength.  Forcing, so the catch-all
/// is a bid (2NT), never Pass.
///
/// Slam beyond 3NT / 5m is deferred to a later increment: a live book node
/// shadows the floor's M6.4 RKCB here, so this increment lands the *game* and
/// leaves keycard exploration to a follow-up that reuses `american::slam`.
// ponytail: caps at game; add RKCB reuse (widen `slam::install_rkcb` to pub(crate))
// when the slam tail measures worth the cross-module coupling.
pub(super) fn opener_rebids_after_two_diamonds() -> Rules {
    Rules::new()
        // 3♦ — four-card diamond support: a known nine-card fit, the best news.
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.45,
            len(Suit::Diamonds, 4..),
        )
        // 3♣ — a real five-card club suit, no diamond support (minor two-suiter).
        .rule(
            Bid::new(3, Strain::Clubs),
            1.35,
            len(Suit::Clubs, 5..) & len(Suit::Diamonds, ..4),
        )
        // 3NT — balanced extras, both majors stopped, to play.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.2,
            balanced() & hcp(15..) & stopper_in(Suit::Hearts) & stopper_in(Suit::Spades),
        )
        // 2♥ / 2♠ — a single major stopper, shown up the line toward 3NT (a
        // both-stopped hand is excluded and falls to the notrump catch-all).
        .rule(
            Bid::new(2, Strain::Hearts),
            1.0,
            stopper_in(Suit::Hearts) & !stopper_in(Suit::Spades),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            0.95,
            stopper_in(Suit::Spades) & !stopper_in(Suit::Hearts),
        )
        // Finite catch-all — a minimum, or both-major stoppers without extras:
        // bid notrump and let responder place the game (never Pass; opener is 11–23).
        .rule(Bid::new(2, Strain::Notrump), 0.5, hcp(0..))
}

/// Opener's rebid after `1♣-2♣` (responder invitational-or-better, 5+♣, no major)
///
/// Same no-major-fit world as the game-forcing `2♦`, but 2♣ is only **invite+**,
/// so opener must be able to stop.  Opener accepts to game with a maximum (jump
/// to `3NT` — balanced-and-stopped, or forced by 17+ opposite the invite's 11+),
/// otherwise declines non-forcing: `3♣` raises responder's known suit, `2NT` the
/// balanced-minimum catch-all.  Responder then places the contract off the
/// **floor** — it passes a dead minimum, drives a game force to 3NT, and (over
/// `3♣`) corrects to the club partscore; measured to do so correctly, so no
/// authored responder node is needed and the floor's slam machinery stays live.
///
/// The help-suit game try (opener's `2♥`/`2♠` showing a single stopper + extras)
/// is dropped: the floor misreads the artificial try as a natural suit and
/// under-accepts.  A cheap accept/decline lands the same games without it.
// ponytail: no game-try rung; add 2♥/2♠ help-suit tries (with an authored
// responder node to read them) if the A/B shows thin invited games being missed.
pub(super) fn opener_rebids_after_two_clubs() -> Rules {
    Rules::new()
        // 3NT — accept to game: balanced maximum, both majors stopped.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.3,
            balanced() & hcp(14..) & stopper_in(Suit::Hearts) & stopper_in(Suit::Spades),
        )
        // 3NT — accept to game: a 17+ maximum forces even stopper-shy, since
        // opposite the 11+ invite the partnership holds 28+ (no minimum rebid may
        // be passed out).
        .rule(Bid::new(3, Strain::Notrump), 1.1, hcp(17..))
        // 3♣ — decline: minimum-or-invitational club support, non-forcing (capped
        // at 16 so a maximum can never leave this in).
        .rule(
            Bid::new(3, Strain::Clubs),
            1.0,
            len(Suit::Clubs, 3..) & hcp(..=16),
        )
        // 2NT — decline / finite catch-all: balanced minimum, non-forcing.
        .rule(Bid::new(2, Strain::Notrump), 0.9, hcp(0..))
}
