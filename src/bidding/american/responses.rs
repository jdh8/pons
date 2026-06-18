//! Responses to one-level suit openings in the 2/1 game-forcing system

use super::super::Rules;
use super::super::Trie;
use super::super::constraint::{balanced, hcp, len, points, stopper_in, support};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

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
        rules = rules.rule(
            Bid::new(level, strain),
            2.8,
            support(4..) & points(10..=13) & len(x, ..=1),
        );
    }

    // Weak jump shifts: single jump in a new suit — 6-card suit, 2–5 HCP.
    let wjs_suits: &[Suit] = if major == Suit::Hearts {
        &[Suit::Spades, Suit::Clubs, Suit::Diamonds]
    } else {
        &[Suit::Clubs, Suit::Diamonds, Suit::Hearts]
    };

    for &x in wjs_suits {
        let (level, strain) = wjs_bid(major, x);
        rules = rules.rule(Bid::new(level, strain), 1.0, len(x, 6..) & points(2..=5));
    }

    // 2/1 game-forcing new suits: cheaper suits, ranked up the line.
    let mut weight = 1.1;
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        if Strain::from(suit) < trump {
            rules = rules.rule(
                Bid::new(2, Strain::from(suit)),
                weight,
                len(suit, 4..) & points(13..) & !support(4..),
            );
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
    let mut rules = Rules::new()
        // Four-card majors up the line (hearts before spades).
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
        // Weak preemptive raise.
        .rule(Bid::new(3, trump), 1.1, support(5..) & points(..=9))
        .rule(Call::Pass, 0.0, hcp(..6));

    // Weak jump shifts: 2♥ and 2♠ over either minor.
    for x in [Suit::Hearts, Suit::Spades] {
        rules = rules.rule(
            Bid::new(2, Strain::from(x)),
            1.0,
            len(x, 6..) & points(2..=5),
        );
    }

    // 2/1 game force: 1♦–2♣ (clubs are cheaper than diamonds).
    if minor == Suit::Diamonds {
        rules = rules.rule(
            Bid::new(2, Strain::Clubs),
            1.3,
            len(Suit::Clubs, 4..) & points(13..) & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        );
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
