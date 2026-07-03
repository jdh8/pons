//! Responses to one-level suit openings in the 2/1 game-forcing system

use super::super::Alert;
use super::super::Rules;
use super::super::Trie;
use super::super::constraint::{
    Cons, Constraint, balanced, described, hcp, len, points, stopper_in, support,
};
use crate::bidding::context::Context;
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};
use std::cell::Cell;

std::thread_local! {
    /// Whether minor-opening responses pick the **longer major** (equal
    /// lengths: 4-4 up the line to `1♥`, 5-5+ higher-first to `1♠`) instead of
    /// unconditional hearts-first.  Default `false` — measured a null
    /// (`ab-minor-continuations`, 2M boards: plain-DD wash, PD −0.12/−0.22
    /// per divergent NV/vul; and −0.003..−0.005 IMPs/board *marginal* on top
    /// of the shipped xyz + up-the-line package).
    static LONGER_MAJOR_RESPONSE: Cell<bool> = const { Cell::new(false) };
    /// Whether the natural minor-opening tree is completed **up the line**:
    /// the `1♣ – 1♦` response, opener's `1♠` rebid over `1m – 1♥`, and
    /// opener's natural `2♣` rebid after `1♣ – 1♦`.  Default `true`, shipped
    /// **jointly with XYZ** (`ab-minor-continuations`, 300k boards, with
    /// `set_xyz`: plain +0.0382/+0.0559 IMPs/board NV/vul, PD
    /// +0.0289/+0.0407).  Alone it is a measured **loss** (plain
    /// −0.91/−1.28 per divergent) — the 1♦ response reroutes hands into
    /// auctions only the XYZ round continues; don't enable it with XYZ off.
    static UP_THE_LINE: Cell<bool> = const { Cell::new(true) };
}

/// Author the longer-major response discipline for books built *after* this
/// call (default `false`)
///
/// On: a response to `1♣`/`1♦` names the longer major — `1♠` on 5♠4♥ or any
/// 5-5+, `1♥` up the line only on 4-4 — so partner can infer "spades are not
/// longer than hearts" from `1♥`.  The M6.4 control-bid classifier reads the
/// same discipline at classify time (`classify_high_bid` in `inference.rs`):
/// the response rule, the rebid structure, and the classifier move together
/// (see `docs/bidding-theorems.md`).  Off (the default): the historic
/// unconditional hearts-first pair — measured no worse than longest-first
/// once `set_up_the_line`'s 1♠ rebid recovers the concealed spade fits, and
/// longest-first costs a level on the heart fits.
pub fn set_longer_major_response(on: bool) {
    LONGER_MAJOR_RESPONSE.with(|cell| cell.set(on));
}

/// Whether the longer-major response discipline is active (also read by the
/// inference engine at classify time)
pub(crate) fn longer_major_response() -> bool {
    LONGER_MAJOR_RESPONSE.with(Cell::get)
}

/// Author the up-the-line completion of the natural minor tree for books
/// built *after* this call (default `true`; off-switch `--no-ns-up-the-line`
/// in `bba-gen`)
///
/// On: responder bids `1♦` over `1♣` on four-plus diamonds without a
/// four-card major (off, those hands squeeze into the notrump ladder or fall
/// to the floor), opener rebids `1♠` over `1m – 1♥` on four spades (off, the
/// 4-4 spade fit is lost to a 1NT rebid), and opener rebids a natural `2♣`
/// after `1♣ – 1♦` on six-plus clubs (off, a misdescribed 1NT catch-all).
///
/// Shipped **jointly with [`set_xyz`][super::set_xyz]**: the 1♦ response only
/// pays once responder's second round has the XYZ machinery (alone it
/// measured plain −0.91/−1.28 per divergent).
pub fn set_up_the_line(on: bool) {
    UP_THE_LINE.with(|cell| cell.set(on));
}

/// Whether the up-the-line completion is currently authored
pub(crate) fn up_the_line() -> bool {
    UP_THE_LINE.with(Cell::get)
}

/// Spades take the first response: strictly longer, or equal length five-plus
///
/// The longer-major discipline's selector — 5-5 responds `1♠` planning to
/// show hearts next; 4-4 responds `1♥` up the line.
fn spades_first() -> Cons<impl Constraint + Clone> {
    described(
        "spades longer than hearts, or equal five-plus",
        |hand: Hand, _: &Context<'_>| {
            let spades = hand[Suit::Spades].len();
            let hearts = hand[Suit::Hearts].len();
            spades > hearts || (spades == hearts && spades >= 5)
        },
    )
}

/// Jacoby 2NT — the game-forcing major raise with four-card support
const JACOBY_2NT: Alert = Alert("jacoby-2nt");
/// Splinter — a double jump in a new suit showing a singleton or void
const SPLINTER: Alert = Alert("splinter");
/// Weak jump shift — a single jump showing a weak six-card suit
const WEAK_JUMP_SHIFT: Alert = Alert("weak-jump-shift");
/// Inverted minor raise — forcing `2m`, preemptive `3m`
const INVERTED_MINOR: Alert = Alert("inverted-minor");
/// 2/1 game force — a new suit at the two level, game forcing
const GAME_FORCE: Alert = Alert("game-force");

/// Responses to our `1♥`/`1♠` opening
///
/// The 2/1 core: a new suit at the two level is game forcing
/// (`hcp(13..)`), the forcing 1NT is the catch-all below it, raises are
/// graded by strength (single / limit / Jacoby 2NT / weak jump to game), and
/// over 1♥ a four-card spade suit takes the one level.  Splinters (double jump
/// in a new suit) and weak jump shifts round out the response set.
#[must_use]
pub fn major_responses(major: Suit) -> Rules {
    let trump = Strain::from(major);
    let mut rules = Rules::new()
        // Jacoby 2NT: game-forcing raise with four-card support.
        .rule(
            Bid::new(2, Strain::Notrump),
            3.0,
            support(4..) & points(13..),
        )
        .alert(JACOBY_2NT)
        // Limit raise: four-card support, 10–12 points.
        .rule(Bid::new(3, trump), 2.0, support(4..) & points(10..=12))
        // Weak jump to game: lots of trumps, few points.
        .rule(Bid::new(4, trump), 1.6, support(5..) & points(..6))
        // Single raise.
        .rule(Bid::new(2, trump), 1.5, support(3..) & points(6..=9))
        // Forcing 1NT: the catch-all when nothing more descriptive fits.
        .rule(Bid::new(1, Strain::Notrump), 0.5, hcp(6..=12))
        .rule(Call::Pass, 0.0, hcp(..6));

    // 1♠ over 1♥: a new suit at the one level, preferred to a single raise.
    if major == Suit::Hearts {
        rules = rules.rule(
            Bid::new(1, Strain::Spades),
            1.7,
            len(Suit::Spades, 4..) & points(6..) & !support(4..),
        );
    }

    // Splinters: double jump in a new suit — four-card support, 10–13 HCP,
    // singleton or void in the splinter suit.
    let splinter_suits: &[Suit] = if major == Suit::Hearts {
        &[Suit::Spades, Suit::Clubs, Suit::Diamonds]
    } else {
        &[Suit::Clubs, Suit::Diamonds, Suit::Hearts]
    };

    for &x in splinter_suits {
        let (level, strain) = splinter_bid(major, x);
        rules = rules
            .rule(
                Bid::new(level, strain),
                2.8,
                support(4..) & points(10..=13) & len(x, ..=1),
            )
            .alert(SPLINTER);
    }

    // Weak jump shifts: single jump in a new suit — 6-card suit, 2–5 HCP.
    let wjs_suits: &[Suit] = if major == Suit::Hearts {
        &[Suit::Spades, Suit::Clubs, Suit::Diamonds]
    } else {
        &[Suit::Clubs, Suit::Diamonds, Suit::Hearts]
    };

    for &x in wjs_suits {
        let (level, strain) = wjs_bid(major, x);
        rules = rules
            .rule(Bid::new(level, strain), 1.0, len(x, 6..) & points(2..=5))
            .alert(WEAK_JUMP_SHIFT);
    }

    // 2/1 game-forcing new suits: cheaper suits, ranked up the line.
    let mut weight = 1.1;
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        if Strain::from(suit) < trump {
            rules = rules
                .rule(
                    Bid::new(2, Strain::from(suit)),
                    weight,
                    len(suit, 4..) & points(13..) & !support(4..),
                )
                .alert(GAME_FORCE);
            weight -= 0.05;
        }
    }
    rules
}

/// The splinter bid for major `m` with void/singleton in `x`
///
/// A splinter is the lowest double-jump bid in a new suit.
fn splinter_bid(major: Suit, x: Suit) -> (u8, Strain) {
    // 1♥ splinters: 3♠ (one above 2♠), 4♣, 4♦
    // 1♠ splinters: 4♣, 4♦, 4♥
    let major_strain = Strain::from(major);
    let x_strain = Strain::from(x);

    if x_strain > major_strain {
        // Over 1♥, spades is a double jump at 3 level (3♠ skips 2♠)
        (3, x_strain)
    } else {
        // Below the major, level 4
        (4, x_strain)
    }
}

/// The weak jump shift bid for major `m` into suit `x`
///
/// A WJS is a single jump into a new suit below the major.
fn wjs_bid(major: Suit, x: Suit) -> (u8, Strain) {
    let major_strain = Strain::from(major);
    let x_strain = Strain::from(x);

    if x_strain > major_strain {
        // Over 1♥, 2♠ (one jump over 1♠)
        (2, x_strain)
    } else {
        // Below or equal to major: 3-level jump
        (3, x_strain)
    }
}

/// Responses to our `1♣`/`1♦` opening
///
/// Four-card majors up the line, a 2/1 game force (`1♦–2♣`), the notrump
/// ladder when no major fits, and inverted minor raises promising five-card
/// support (strong 2-of-minor forcing, weak preemptive 3-of-minor).
#[must_use]
pub fn minor_responses(minor: Suit) -> Rules {
    let trump = Strain::from(minor);
    let mut rules = Rules::new();
    // Major selection between 4+ majors, per the longer-major knob.
    rules = if longer_major_response() {
        // Longer-major discipline (`set_longer_major_response`): the response
        // names the longer major — 1♠ on 5♠4♥/6♠5♥ or any 5-5+, 1♥ up the
        // line only on 4-4 — so 1♥ denies longer spades and the M6.4
        // control-bid classifier can read `1♣–1♥–2♣–4♠` as a control bid.
        rules
            .rule(
                Bid::new(1, Strain::Spades),
                1.5,
                len(Suit::Spades, 4..) & points(6..) & spades_first(),
            )
            .rule(
                Bid::new(1, Strain::Hearts),
                1.4,
                len(Suit::Hearts, 4..) & points(6..) & !spades_first(),
            )
    } else {
        // Default pair — unconditional hearts-first: any four-plus hearts
        // responds 1♥ even with longer spades (5♠4♥, 6♠5♥), so partner can
        // only infer "1♠ denies four hearts", never the converse, and the
        // M6.4 classifier must read a later jump into the suit *above* the
        // response as natural to play (the first M6.4 A/B round assumed
        // longest-first here and lost 6 IMPs per fired board).  The
        // longest-first arm above was measured as the prescribed trio
        // (response + rebids + classifier) and came back a null — hearts-first
        // stays the default; see `set_longer_major_response` and
        // `docs/bidding-theorems.md`.
        rules
            .rule(
                Bid::new(1, Strain::Hearts),
                1.5,
                len(Suit::Hearts, 4..) & points(6..),
            )
            .rule(
                Bid::new(1, Strain::Spades),
                1.4,
                len(Suit::Spades, 4..) & points(6..) & len(Suit::Hearts, ..4),
            )
    };
    // Up-the-line completion (`set_up_the_line`): a natural 1♦ over 1♣ on
    // four-plus diamonds without a four-card major.  Weight 1.2 sits below
    // the majors (1.5/1.4) and the inverted raise (1.25), above the notrump
    // ladder (1.0) — so diamond hands stop mislabeling themselves as
    // balanced notrump responses or falling to the floor.
    if minor == Suit::Clubs && up_the_line() {
        rules = rules.rule(
            Bid::new(1, Strain::Diamonds),
            1.2,
            len(Suit::Diamonds, 4..)
                & points(6..)
                & len(Suit::Hearts, ..4)
                & len(Suit::Spades, ..4),
        );
    }
    rules = rules
        // Notrump ladder without a four-card major (3NT open-ended for game-plus).
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(13..) & balanced() & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            1.0,
            hcp(11..=12) & balanced() & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        .rule(
            Bid::new(1, Strain::Notrump),
            0.5,
            hcp(6..=10) & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        // Inverted minor raises (five-card support required since opener may hold only three).
        // Strong raise: forcing one round — no majors, 10+ points.
        .rule(
            Bid::new(2, trump),
            1.25,
            support(5..) & points(10..) & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        .alert(INVERTED_MINOR)
        // Weak preemptive raise.
        .rule(Bid::new(3, trump), 1.1, support(5..) & points(..=9))
        .alert(INVERTED_MINOR)
        .rule(Call::Pass, 0.0, hcp(..6));

    // Weak jump shifts: 2♥ and 2♠ over either minor.
    for x in [Suit::Hearts, Suit::Spades] {
        rules = rules
            .rule(
                Bid::new(2, Strain::from(x)),
                1.0,
                len(x, 6..) & points(2..=5),
            )
            .alert(WEAK_JUMP_SHIFT);
    }

    // 2/1 game force: 1♦–2♣ (clubs are cheaper than diamonds).
    if minor == Suit::Diamonds {
        rules = rules
            .rule(
                Bid::new(2, Strain::Clubs),
                1.3,
                len(Suit::Clubs, 4..)
                    & points(13..)
                    & len(Suit::Hearts, ..4)
                    & len(Suit::Spades, ..4),
            )
            .alert(GAME_FORCE);
    }
    rules
}

/// Register the first responses and their response-level continuations
///
/// Inserts the response tables to every one-of-a-suit opening, then opener's
/// rebids after splinters and inverted raises.
pub(super) fn register(book: &mut Trie) {
    // --- First responses to every one-of-a-suit opening ---
    for major in [Suit::Hearts, Suit::Spades] {
        super::insert_uncontested(
            book,
            &[super::call(1, Strain::from(major))],
            major_responses(major),
        );
    }
    for minor in [Suit::Clubs, Suit::Diamonds] {
        super::insert_uncontested(
            book,
            &[super::call(1, Strain::from(minor))],
            minor_responses(minor),
        );
    }

    // --- Splinter continuations (opener's rebid after a splinter) ---
    for major in [Suit::Hearts, Suit::Spades] {
        let m_strain = Strain::from(major);
        let splinter_suits: &[Suit] = if major == Suit::Hearts {
            &[Suit::Spades, Suit::Clubs, Suit::Diamonds]
        } else {
            &[Suit::Clubs, Suit::Diamonds, Suit::Hearts]
        };

        for &x in splinter_suits {
            let (level, strain) = splinter_bid(major, x);
            let splinter = super::call(level, strain);
            let our_calls = &[super::call(1, m_strain), splinter];

            let after_splinter = Rules::new()
                .rule(Bid::new(4, Strain::Notrump), 1.0, points(16..))
                .alert(super::slam::RKCB)
                .rule(Bid::new(4, m_strain), 0.5, hcp(0..));

            super::insert_uncontested(book, our_calls, after_splinter);
            super::slam::install_rkcb(book, our_calls, major);
        }
    }

    // --- Inverted minor raise continuations (opener's rebid) ---
    for minor in [Suit::Clubs, Suit::Diamonds] {
        let m_strain = Strain::from(minor);
        let our_calls = &[super::call(1, m_strain), super::call(2, m_strain)];

        // Opener's rebid after the inverted raise: no Pass (forcing).
        let after_inv_raise = Rules::new()
            .rule(Bid::new(2, Strain::Notrump), 1.0, hcp(12..=14) & balanced())
            .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(18..=19))
            .rule(
                Bid::new(2, Strain::Hearts),
                0.8,
                stopper_in(Suit::Hearts) & hcp(15..),
            )
            .rule(
                Bid::new(2, Strain::Spades),
                0.8,
                stopper_in(Suit::Spades) & hcp(15..),
            )
            .rule(Bid::new(3, m_strain), 0.5, hcp(0..));

        super::insert_uncontested(book, our_calls, after_inv_raise);

        // Responder's third call after opener bids 2NT.
        let after_2nt = Rules::new()
            .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(13..))
            .rule(Bid::new(3, m_strain), 0.5, hcp(0..));

        let our_calls_2nt = &[
            super::call(1, m_strain),
            super::call(2, m_strain),
            super::call(2, Strain::Notrump),
        ];
        super::insert_uncontested(book, our_calls_2nt, after_2nt);

        // Responder's third call after opener's 18–19 jump to 3NT: with slam
        // values (~32+ combined and 5+-card support) launch minor RKCB; else
        // play the cold 3NT.  4NT is keycard here by construction — install_rkcb
        // registers the answers below this node.
        let after_3nt = Rules::new()
            .rule(Bid::new(4, Strain::Notrump), 1.0, points(14..))
            .alert(super::slam::RKCB)
            .rule(Call::Pass, 0.5, hcp(0..));
        let our_calls_3nt = &[
            super::call(1, m_strain),
            super::call(2, m_strain),
            super::call(3, Strain::Notrump),
        ];
        super::insert_uncontested(book, our_calls_3nt, after_3nt);
        super::slam::install_rkcb(book, our_calls_3nt, minor);

        // Responder's third call after opener bids 2♥ or 2♠.
        for major in [Suit::Hearts, Suit::Spades] {
            let major_strain = Strain::from(major);
            let after_major = Rules::new()
                .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(13..))
                .rule(Bid::new(2, Strain::Notrump), 0.8, hcp(10..=12))
                .rule(Bid::new(3, m_strain), 0.5, hcp(0..));

            let our_calls_major = &[
                super::call(1, m_strain),
                super::call(2, m_strain),
                super::call(2, major_strain),
            ];
            super::insert_uncontested(book, our_calls_major, after_major);

            // Fourth call: after [1m, 2m, 2M, 2NT].
            let after_2nt_4th = Rules::new().rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..));

            let our_calls_2nt_4th = &[
                super::call(1, m_strain),
                super::call(2, m_strain),
                super::call(2, major_strain),
                super::call(2, Strain::Notrump),
            ];
            super::insert_uncontested(book, our_calls_2nt_4th, after_2nt_4th);
        }
    }
}
