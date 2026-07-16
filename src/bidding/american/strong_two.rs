//! Strong 2♣ opening structure for the 2/1 game-forcing system
//!
//! The strong, artificial `2♣` opening promises 22+ HCP and is game
//! forcing.  This module registers the full response tree into the
//! constructive book:
//!
//! - **Responses** to 2♣: 2♦ waiting, 2♥ double negative (0–3 HCP),
//!   and natural positives with a good five-card suit.
//! - **Opener's rebid** after 2♦ waiting or the 2♥ double negative.
//! - **Responder continuations** after each of opener's suit rebids.
//! - **Opener's decision** after the major or minor raise, including a
//!   hook for Roman Key Card Blackwood.
//!
//! Every node in this auction is forcing unless it carries a
//! [`Call::Pass`] rule; see the module-level note in
//! [`american`][super] on *forcing by omission*.

use super::super::constraint::{balanced, fifths, hcp, len, points, support, top_honors};
use super::super::{Alert, Rules, Trie};
use super::{call, insert_uncontested};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

// ---------------------------------------------------------------------------
// Response tables
// ---------------------------------------------------------------------------

/// The two artificial responses to 2♣ (the natural positives are not alerted)
const WAITING: Alert = Alert("strong-2c:waiting");
const DOUBLE_NEGATIVE: Alert = Alert("strong-2c:negative");

/// Responses to the 2♣ opening (at `&[2♣]`)
///
/// The auction is forcing — there is no [`Call::Pass`] rule.  2♦ is
/// the waiting bid (a catch-all for weaker hands); 2♥ is the double
/// negative showing 0–3 HCP; the remaining options are natural positives
/// with a good five-card suit.
fn responses() -> Rules {
    Rules::new()
        // 2♥: double negative — 0–3 HCP.
        .rule(Bid::new(2, Strain::Hearts), 2.0, hcp(0..=3))
        .alert(DOUBLE_NEGATIVE)
        // 2♠: natural positive — five spades to two of the top three honors.
        .rule(
            Bid::new(2, Strain::Spades),
            1.5,
            len(Suit::Spades, 5..) & top_honors(Suit::Spades, 2..) & points(8..),
        )
        // 3♣: natural positive — five clubs to two top honors.
        .rule(
            Bid::new(3, Strain::Clubs),
            1.4,
            len(Suit::Clubs, 5..) & top_honors(Suit::Clubs, 2..) & points(8..),
        )
        // 3♦: natural positive — five diamonds to two top honors.
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.4,
            len(Suit::Diamonds, 5..) & top_honors(Suit::Diamonds, 2..) & points(8..),
        )
        // 2NT: balanced positive — 8+ HCP, balanced shape.
        .rule(Bid::new(2, Strain::Notrump), 1.3, hcp(8..) & balanced())
        // 2♦: waiting catch-all — 4+ HCP (not strong enough for a positive).
        .rule(Bid::new(2, Strain::Diamonds), 0.5, hcp(4..))
        .alert(WAITING)
}

/// Opener's rebid after `2♣–(P)–2♦–(P)` (at `&[2♣, 2♦]`)
///
/// Forcing — no [`Call::Pass`] rule.  Opener describes shape and Fifths
/// range; 2NT is used for a 22–24 balanced minimum and 3NT for 25–27.
/// A 2NT fallback catches any 22+ hand that has no natural rebid.
fn opener_rebid_after_waiting() -> Rules {
    Rules::new()
        // 2♠: five or more spades.
        .rule(Bid::new(2, Strain::Spades), 1.55, len(Suit::Spades, 5..))
        // 2♥: five or more hearts.
        .rule(Bid::new(2, Strain::Hearts), 1.5, len(Suit::Hearts, 5..))
        // 2NT: balanced 22–24.
        .rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            fifths(22.0..25.0) & balanced(),
        )
        // 3NT: balanced 25–27.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.2,
            fifths(25.0..28.0) & balanced(),
        )
        // 3♣: five or more clubs.
        .rule(Bid::new(3, Strain::Clubs), 1.0, len(Suit::Clubs, 5..))
        // 3♦: five or more diamonds.
        .rule(Bid::new(3, Strain::Diamonds), 1.0, len(Suit::Diamonds, 5..))
        // 2NT fallback: guaranteed legal for any 22+ hand.
        .rule(Bid::new(2, Strain::Notrump), 0.2, fifths(22.0..))
}

/// Opener's rebid after `2♣–(P)–2♥–(P)` (at `&[2♣, 2♥]`)
///
/// Forcing — no [`Call::Pass`] rule.  Identical shape logic to
/// [`opener_rebid_after_waiting`], except hearts must be rebid at the
/// three level (2♥ is already occupied by the double-negative response).
fn opener_rebid_after_negative() -> Rules {
    Rules::new()
        // 2♠: five or more spades.
        .rule(Bid::new(2, Strain::Spades), 1.55, len(Suit::Spades, 5..))
        // 3♥: five or more hearts (2♥ is taken by the double negative).
        .rule(Bid::new(3, Strain::Hearts), 1.5, len(Suit::Hearts, 5..))
        // 2NT: balanced 22–24.
        .rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            fifths(22.0..25.0) & balanced(),
        )
        // 3NT: balanced 25–27.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.2,
            fifths(25.0..28.0) & balanced(),
        )
        // 3♣: five or more clubs.
        .rule(Bid::new(3, Strain::Clubs), 1.0, len(Suit::Clubs, 5..))
        // 3♦: five or more diamonds.
        .rule(Bid::new(3, Strain::Diamonds), 1.0, len(Suit::Diamonds, 5..))
        // 2NT fallback: guaranteed legal for any 22+ hand.
        .rule(Bid::new(2, Strain::Notrump), 0.2, fifths(22.0..))
}

// ---------------------------------------------------------------------------
// Responder continuations after a suit rebid (waiting sequence)
// ---------------------------------------------------------------------------

/// Responder after `2♣–(P)–2♦–(P)–2♥–(P)` (at `&[2♣, 2♦, 2♥]`)
///
/// Raise hearts with three-card support; retreat to 2NT otherwise.
fn resp_after_waiting_hearts() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 1.5, support(3..))
        .rule(Bid::new(2, Strain::Notrump), 0.5, hcp(0..))
}

/// Responder after `2♣–(P)–2♦–(P)–2♠–(P)` (at `&[2♣, 2♦, 2♠]`)
///
/// Raise spades with three-card support; retreat to 2NT otherwise.
fn resp_after_waiting_spades() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Spades), 1.5, support(3..))
        .rule(Bid::new(2, Strain::Notrump), 0.5, hcp(0..))
}

/// Responder after `2♣–(P)–2♦–(P)–3♣–(P)` (at `&[2♣, 2♦, 3♣]`)
///
/// Raise clubs with four-card support and values; bid 3NT otherwise.
fn resp_after_waiting_clubs() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Clubs), 1.2, support(4..) & points(4..))
        .rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..))
}

/// Responder after `2♣–(P)–2♦–(P)–3♦–(P)` (at `&[2♣, 2♦, 3♦]`)
///
/// Raise diamonds with four-card support and values; bid 3NT otherwise.
fn resp_after_waiting_diamonds() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Diamonds),
            1.2,
            support(4..) & points(4..),
        )
        .rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Responder continuations after a suit rebid (double-negative sequence)
// ---------------------------------------------------------------------------

/// Responder after `2♣–(P)–2♥–(P)–R–(P)` for suit `R` after the double negative
///
/// With four-card support, raise to the cheapest level of opener's suit.
/// Without support, pass.
fn resp_after_negative_suit(raise: Bid) -> Rules {
    Rules::new()
        .rule(raise, 1.0, support(4..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Opener after a major raise
// ---------------------------------------------------------------------------

/// Opener after `2♣–(P)–2♦–(P)–2♥–(P)–3♥–(P)` (at `&[2♣, 2♦, 2♥, 3♥]`)
///
/// With 28+ HCP, launch RKCB (4NT); otherwise sign off in 4♥.
fn opener_after_hearts_raise() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.0, hcp(28..))
        .alert(super::slam::RKCB)
        .rule(Bid::new(4, Strain::Hearts), 0.5, hcp(0..))
}

/// Opener after `2♣–(P)–2♦–(P)–2♠–(P)–3♠–(P)` (at `&[2♣, 2♦, 2♠, 3♠]`)
///
/// With 28+ HCP, launch RKCB (4NT); otherwise sign off in 4♠.
fn opener_after_spades_raise() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.0, hcp(28..))
        .alert(super::slam::RKCB)
        .rule(Bid::new(4, Strain::Spades), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Opener after a minor raise
// ---------------------------------------------------------------------------

/// Opener after `2♣–(P)–2♦–(P)–3♣–(P)–4♣–(P)` (at `&[2♣, 2♦, 3♣, 4♣]`)
///
/// With 28+ HCP, launch minor RKCB (4NT); otherwise sign off in 5♣.  With the
/// minor keycard off ([`super::slam::set_minor_keycard`]), the pre-keycard
/// blind jump to 6♣ on 27+ instead.
fn opener_after_clubs_raise() -> Rules {
    if !super::slam::minor_keycard() {
        return Rules::new()
            .rule(Bid::new(6, Strain::Clubs), 1.0, hcp(27..))
            .rule(Bid::new(5, Strain::Clubs), 0.5, hcp(0..));
    }
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.0, hcp(28..))
        .alert(super::slam::RKCB)
        .rule(Bid::new(5, Strain::Clubs), 0.5, hcp(0..))
}

/// Opener after `2♣–(P)–2♦–(P)–3♦–(P)–4♦–(P)` (at `&[2♣, 2♦, 3♦, 4♦]`)
///
/// With 28+ HCP, launch minor RKCB (4NT); otherwise sign off in 5♦.  With the
/// minor keycard off ([`super::slam::set_minor_keycard`]), the pre-keycard
/// blind jump to 6♦ on 27+ instead.
fn opener_after_diamonds_raise() -> Rules {
    if !super::slam::minor_keycard() {
        return Rules::new()
            .rule(Bid::new(6, Strain::Diamonds), 1.0, hcp(27..))
            .rule(Bid::new(5, Strain::Diamonds), 0.5, hcp(0..));
    }
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.0, hcp(28..))
        .alert(super::slam::RKCB)
        .rule(Bid::new(5, Strain::Diamonds), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register all strong 2♣ continuations into the constructive book
///
/// Called once from [`american`][super::american] to attach the
/// full strong-two structure.  Every table is inserted via
/// [`insert_uncontested`], which fans 0–2 leading passes so the same
/// logic fires regardless of which seat held the 2♣ opening.
pub(super) fn register(book: &mut Trie) {
    let c2 = call(2, Strain::Clubs);
    let d2 = call(2, Strain::Diamonds);
    let h2 = call(2, Strain::Hearts);
    let s2 = call(2, Strain::Spades);
    let c3 = call(3, Strain::Clubs);
    let d3 = call(3, Strain::Diamonds);
    let h3 = call(3, Strain::Hearts);
    let s3 = call(3, Strain::Spades);
    let h4 = call(4, Strain::Hearts);
    let s4 = call(4, Strain::Spades);
    let c4 = call(4, Strain::Clubs);
    let d4 = call(4, Strain::Diamonds);

    // Responses to 2♣ (forcing).
    insert_uncontested(book, &[c2], responses());

    // Opener's rebid after the waiting 2♦.
    insert_uncontested(book, &[c2, d2], opener_rebid_after_waiting());

    // Opener's rebid after the double-negative 2♥.
    insert_uncontested(book, &[c2, h2], opener_rebid_after_negative());

    // Responder continuations after opener's suit rebid (waiting sequence).
    insert_uncontested(book, &[c2, d2, h2], resp_after_waiting_hearts());
    insert_uncontested(book, &[c2, d2, s2], resp_after_waiting_spades());
    insert_uncontested(book, &[c2, d2, c3], resp_after_waiting_clubs());
    insert_uncontested(book, &[c2, d2, d3], resp_after_waiting_diamonds());

    // Responder continuations after opener's suit rebid (double-negative sequence).
    // Raise to the cheapest level; pass without support.
    insert_uncontested(
        book,
        &[c2, h2, s2],
        resp_after_negative_suit(Bid::new(3, Strain::Spades)),
    );
    insert_uncontested(
        book,
        &[c2, h2, c3],
        resp_after_negative_suit(Bid::new(4, Strain::Clubs)),
    );
    insert_uncontested(
        book,
        &[c2, h2, d3],
        resp_after_negative_suit(Bid::new(4, Strain::Diamonds)),
    );
    insert_uncontested(
        book,
        &[c2, h2, h3],
        resp_after_negative_suit(Bid::new(4, Strain::Hearts)),
    );

    // Opener after responder's major raise (waiting sequence).
    insert_uncontested(book, &[c2, d2, h2, h3], opener_after_hearts_raise());
    super::slam::install_rkcb(
        book,
        &[
            call(2, Strain::Clubs),
            call(2, Strain::Diamonds),
            call(2, Strain::Hearts),
            call(3, Strain::Hearts),
        ],
        Suit::Hearts,
    );

    insert_uncontested(book, &[c2, d2, s2, s3], opener_after_spades_raise());
    super::slam::install_rkcb(
        book,
        &[
            call(2, Strain::Clubs),
            call(2, Strain::Diamonds),
            call(2, Strain::Spades),
            call(3, Strain::Spades),
        ],
        Suit::Spades,
    );

    // Opener after responder's minor raise (waiting sequence).  The 4NT ask
    // and its answer ladder ride the minor-keycard knob together — a lone ask
    // with no authored answers would strand partner on the floor.
    insert_uncontested(book, &[c2, d2, c3, c4], opener_after_clubs_raise());
    if super::slam::minor_keycard() {
        super::slam::install_rkcb(
            book,
            &[
                call(2, Strain::Clubs),
                call(2, Strain::Diamonds),
                call(3, Strain::Clubs),
                call(4, Strain::Clubs),
            ],
            Suit::Clubs,
        );
    }

    insert_uncontested(book, &[c2, d2, d3, d4], opener_after_diamonds_raise());
    if super::slam::minor_keycard() {
        super::slam::install_rkcb(
            book,
            &[
                call(2, Strain::Clubs),
                call(2, Strain::Diamonds),
                call(3, Strain::Diamonds),
                call(4, Strain::Diamonds),
            ],
            Suit::Diamonds,
        );
    }

    // Suppress unused-variable warnings for variables used only in some branches.
    let _ = (h4, s4);
}
