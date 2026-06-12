//! Opener's rebids (one round) and the forcing-1NT continuations

use super::{call, insert_uncontested};
use crate::bidding::constraint::{balanced, hcp, len, support};
use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// Opener's rebid after `1♥ – 1♠`: raise spades, rebid hearts, or show shape
///
/// Forcing on opener — there is no pass rule.
fn rebid_one_heart_one_spade() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Spades), 2.6, support(4..) & hcp(19..))
        .rule(
            Bid::new(3, Strain::Spades),
            2.2,
            support(4..) & hcp(16..=18),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            1.8,
            support(4..) & hcp(12..=15),
        )
        .rule(Bid::new(2, Strain::Hearts), 1.4, len(Suit::Hearts, 6..))
        .rule(Bid::new(2, Strain::Notrump), 1.2, hcp(18..=19) & balanced())
        .rule(Bid::new(2, Strain::Clubs), 0.9, len(Suit::Clubs, 4..))
        .rule(Bid::new(2, Strain::Diamonds), 0.9, len(Suit::Diamonds, 4..))
        // Balanced minimum, and the guaranteed-legal fallback.
        .rule(Bid::new(1, Strain::Notrump), 0.5, hcp(12..=14))
        .rule(Bid::new(1, Strain::Notrump), 0.2, hcp(12..))
}

/// Opener's rebid after `1M – 1NT` (the forcing notrump)
///
/// Forcing on opener.  A five-card-major rebid is the guaranteed-legal
/// fallback when nothing more descriptive fits — a basic simplification.
fn rebid_after_forcing_notrump(major: Suit) -> Rules {
    let trump = Strain::from(major);
    let mut rules = Rules::new()
        .rule(Bid::new(2, Strain::Notrump), 1.2, hcp(18..=19) & balanced())
        .rule(Bid::new(2, trump), 1.0, len(major, 6..));
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        if Strain::from(suit) < trump {
            rules = rules.rule(Bid::new(2, Strain::from(suit)), 0.9, len(suit, 4..));
        }
    }
    // Opener always holds at least five of the major, so this always applies.
    rules.rule(Bid::new(2, trump), 0.3, len(major, 5..))
}

/// Opener's rebid raising responder's new major after a minor opening
///
/// Used at `1m – 1M`.  Forcing on opener; a 1NT rebid is the guaranteed-legal
/// fallback.
fn rebid_raise_major(responder_major: Suit, opener_minor: Suit) -> Rules {
    let m = Strain::from(responder_major);
    Rules::new()
        .rule(Bid::new(4, m), 2.6, support(4..) & hcp(19..))
        .rule(Bid::new(3, m), 2.2, support(4..) & hcp(16..=18))
        .rule(Bid::new(2, m), 1.8, support(4..) & hcp(12..=15))
        .rule(Bid::new(2, Strain::Notrump), 1.2, hcp(18..=19) & balanced())
        .rule(
            Bid::new(2, Strain::from(opener_minor)),
            0.9,
            len(opener_minor, 5..),
        )
        .rule(Bid::new(1, Strain::Notrump), 0.5, hcp(12..=14) & balanced())
        .rule(Bid::new(1, Strain::Notrump), 0.2, hcp(12..))
}

/// Opener's rebid after `1♣ – 1♦`
fn rebid_one_club_one_diamond() -> Rules {
    Rules::new()
        .rule(Bid::new(1, Strain::Hearts), 1.3, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(1, Strain::Spades),
            1.3,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.5,
            support(4..) & hcp(16..=18),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.2,
            support(4..) & hcp(12..=15),
        )
        .rule(Bid::new(2, Strain::Notrump), 1.1, hcp(18..=19) & balanced())
        .rule(Bid::new(1, Strain::Notrump), 0.5, hcp(12..=14) & balanced())
        .rule(Bid::new(1, Strain::Notrump), 0.2, hcp(12..))
}

// ---------------------------------------------------------------------------
// Responder's second call after the forcing 1NT
// ---------------------------------------------------------------------------

/// Responder's options after opener's rebid in the forcing-1NT structure
///
/// One shared table covers every opener rebid; rules for calls that are
/// illegal in a particular sequence simply go dead.  The table in priority
/// order:
///
/// | Call   | Wt  | Meaning |
/// |--------|-----|---------|
/// | 3M     | 1.5 | Three-card limit raise (10–12 HCP) |
/// | 2NT    | 1.2 | Natural notrump invite (11–12 HCP) |
/// | 2x≠M   | 1.1 | Six-card runout, weak (≤ 9 HCP); dead when illegal |
/// | 2M     | 1.0 | Preference to the major (7+ HCP, 2+ cards) |
/// | Pass   | 0.0 | Catch-all: the force was one round only |
fn responder_after_forcing_notrump(major: Suit) -> Rules {
    let trump = Strain::from(major);
    let mut rules = Rules::new()
        // Three-card limit raise — the standard 2/1 route: 1NT then 3M.
        .rule(Bid::new(3, trump), 1.5, len(major, 3..) & hcp(10..=12))
        // Natural notrump invite.
        .rule(Bid::new(2, Strain::Notrump), 1.2, hcp(11..=12))
        // Preference to opener's major.
        .rule(Bid::new(2, trump), 1.0, len(major, 2..) & hcp(7..))
        // Catch-all pass; the forcing 1NT is one round only.
        .rule(Call::Pass, 0.0, hcp(0..));

    // Six-card runouts into a side suit (dead when the call is illegal in
    // the current auction).
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit != major {
            rules = rules.rule(
                Bid::new(2, Strain::from(suit)),
                1.1,
                len(suit, 6..) & hcp(..=9),
            );
        }
    }
    rules
}

/// Opener accepts or declines responder's 2NT notrump invite
///
/// Accept with 14+ HCP (bid 3NT), decline with a pass.
fn opener_accept_notrump_invite() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(14..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener accepts or declines responder's 3M limit raise
///
/// Accept with 14+ HCP (bid game in the major), decline with a pass.
fn opener_accept_limit_raise(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 1.0, hcp(14..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Register responder's second call and opener's acceptance in the
/// forcing-1NT structure
///
/// For each major and each distinct opener rebid that is NOT 2NT (the 18–19
/// balanced rebid's continuations live in the notrump module), inserts
/// responder's table at `[1M, 1NT, rebid]` and opener's acceptances at
/// `[1M, 1NT, rebid, 2NT]` and `[1M, 1NT, rebid, 3M]`.
fn register_forcing_notrump_continuations(book: &mut Trie) {
    for major in [Suit::Hearts, Suit::Spades] {
        let one_major = call(1, Strain::from(major));
        let one_nt = call(1, Strain::Notrump);

        // Collect distinct non-2NT rebid calls from opener's table.
        let mut seen: Vec<Call> = Vec::new();
        for rule in rebid_after_forcing_notrump(major).rules() {
            let rebid = rule.call();
            if rebid != call(2, Strain::Notrump) && !seen.contains(&rebid) {
                seen.push(rebid);
            }
        }

        for rebid in seen {
            insert_uncontested(
                book,
                &[one_major, one_nt, rebid],
                responder_after_forcing_notrump(major),
            );
            insert_uncontested(
                book,
                &[one_major, one_nt, rebid, call(2, Strain::Notrump)],
                opener_accept_notrump_invite(),
            );
            insert_uncontested(
                book,
                &[one_major, one_nt, rebid, call(3, Strain::from(major))],
                opener_accept_limit_raise(major),
            );
        }
    }
}

/// Register opener's rebids after a one-level new suit and the forcing 1NT
pub(super) fn register(book: &mut Trie) {
    register_forcing_notrump_continuations(book);
    insert_uncontested(
        book,
        &[call(1, Strain::Hearts), call(1, Strain::Spades)],
        rebid_one_heart_one_spade(),
    );
    for major in [Suit::Hearts, Suit::Spades] {
        insert_uncontested(
            book,
            &[call(1, Strain::from(major)), call(1, Strain::Notrump)],
            rebid_after_forcing_notrump(major),
        );
    }
    insert_uncontested(
        book,
        &[call(1, Strain::Clubs), call(1, Strain::Diamonds)],
        rebid_one_club_one_diamond(),
    );
    for minor in [Suit::Clubs, Suit::Diamonds] {
        for responder_major in [Suit::Hearts, Suit::Spades] {
            insert_uncontested(
                book,
                &[
                    call(1, Strain::from(minor)),
                    call(1, Strain::from(responder_major)),
                ],
                rebid_raise_major(responder_major, minor),
            );
        }
    }
}
