//! Dutch responses to the wide 1♣ opening — the load-bearing convention (Phase 2.1)
//!
//! Two nodes diverge hard from american and are authored here; every deeper
//! continuation is reused from `bare_american` for now (Phase 2.2 authors the
//! `1♣-1♦-1M/1NT/2♣/2♦` trees).  See `docs/dutch-system.md` for the full spec.
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

use crate::bidding::constraint::{balanced, hcp, len, points};
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
