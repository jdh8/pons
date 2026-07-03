//! Opener's rebids (one round) and the forcing-1NT continuations

use super::{call, insert_uncontested};
use crate::bidding::constraint::{balanced, fifths, hcp, len, points, support};
use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};
use std::cell::Cell;

// ponytail: construction-time toggle, read during `register()`; set it before
// building the `Pair`.  A per-classify flag (like `set_fifths_companion`) would
// not work — the adjunct changes which *nodes exist*, baked once at build time.
std::thread_local! {
    /// Whether opener's rebid tables carry the **Meckstroth adjunct**: the
    /// invitational `3m` jumps (`1M – 1NT – 3m` and `1♥ – 1♠ – 3m`) and their
    /// responder continuations.  On by default.
    static MECKSTROTH: Cell<bool> = const { Cell::new(true) };
}

/// Enable or disable the Meckstroth adjunct in books built *after* this call
///
/// Read at book-construction time (during `register`); set it before building
/// the `Pair`.  The default is on.  Used by the `meckstroth-abc` A/B example to
/// build a baseline arm (off) and a treatment arm (on).
pub fn set_meckstroth_adjunct(on: bool) {
    MECKSTROTH.with(|cell| cell.set(on));
}

/// Whether the Meckstroth adjunct is currently enabled
fn meckstroth() -> bool {
    MECKSTROTH.with(Cell::get)
}

/// Whether a rebid is opener's invitational `3♣`/`3♦` jump (the Meckstroth `3m`)
fn is_invitational_minor_jump(rebid: Call) -> bool {
    rebid == call(3, Strain::Clubs) || rebid == call(3, Strain::Diamonds)
}

/// Append the Meckstroth-adjunct invitational minor jumps when enabled
///
/// `3♣`/`3♦` show 5+ cards in the minor and ≈15–17 points — the medium shapely
/// hand that otherwise underbids as a natural two-level minor.  The weight sits
/// above the natural minor (0.9) and the six-card-major rebid (1.0) but below
/// the strong 2NT (1.2), so disjointness is by strength: 18–19 balanced → 2NT;
/// 15–17 with a five-card minor → `3m`; a minimum → the natural two level.
fn with_invitational_minors(mut rules: Rules) -> Rules {
    if meckstroth() {
        for minor in [Suit::Clubs, Suit::Diamonds] {
            rules = rules.rule(
                Bid::new(3, Strain::from(minor)),
                1.05,
                len(minor, 5..) & points(15..=17),
            );
        }
    }
    rules
}

/// Opener's rebid after `1♥ – 1♠`: raise spades, rebid hearts, or show shape
///
/// Forcing on opener — there is no pass rule.
fn rebid_one_heart_one_spade() -> Rules {
    let mut rules = Rules::new()
        .rule(
            Bid::new(4, Strain::Spades),
            2.6,
            support(4..) & points(19..),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            2.2,
            support(4..) & points(16..=18),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            1.8,
            support(4..) & points(12..=15),
        )
        .rule(Bid::new(2, Strain::Hearts), 1.4, len(Suit::Hearts, 6..))
        .rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            fifths(18.0..20.0) & balanced(),
        );
    // Meckstroth adjunct: invitational 3♣/3♦ jumps with a five-card minor.
    rules = with_invitational_minors(rules);
    rules
        .rule(Bid::new(2, Strain::Clubs), 0.9, len(Suit::Clubs, 4..))
        .rule(Bid::new(2, Strain::Diamonds), 0.9, len(Suit::Diamonds, 4..))
        // Balanced minimum, and the guaranteed-legal fallback.
        .rule(Bid::new(1, Strain::Notrump), 0.5, fifths(12.0..15.0))
        .rule(Bid::new(1, Strain::Notrump), 0.2, hcp(0..))
}

/// Opener's rebid after `1M – 1NT` (the forcing notrump)
///
/// Forcing on opener.  A five-card-major rebid is the guaranteed-legal
/// fallback when nothing more descriptive fits — a basic simplification.
fn rebid_after_forcing_notrump(major: Suit) -> Rules {
    let trump = Strain::from(major);
    let mut rules = Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            fifths(18.0..20.0) & balanced(),
        )
        .rule(Bid::new(2, trump), 1.0, len(major, 6..));
    // Meckstroth adjunct: invitational 3♣/3♦ jumps with a five-card minor.
    rules = with_invitational_minors(rules);
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
/// fallback.  Under the up-the-line completion (`set_up_the_line`) opener
/// also shows four spades over a `1♥` response — without it the 4-4 spade
/// fit is lost to the 1NT rebid.
fn rebid_raise_major(responder_major: Suit, opener_minor: Suit) -> Rules {
    let m = Strain::from(responder_major);
    let mut rules = Rules::new()
        .rule(Bid::new(4, m), 2.6, support(4..) & points(19..))
        .rule(Bid::new(3, m), 2.2, support(4..) & points(16..=18))
        .rule(Bid::new(2, m), 1.8, support(4..) & points(12..=15))
        .rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            fifths(18.0..20.0) & balanced(),
        );
    // Up the line: four spades over a 1♥ response, ahead of the minor rebid
    // and the notrump fallbacks (a heart raise with four-card support still
    // wins on weight).
    if responder_major == Suit::Hearts && super::responses::up_the_line() {
        rules = rules.rule(Bid::new(1, Strain::Spades), 0.95, len(Suit::Spades, 4..));
    }
    rules
        .rule(
            Bid::new(2, Strain::from(opener_minor)),
            0.9,
            len(opener_minor, 5..),
        )
        .rule(
            Bid::new(1, Strain::Notrump),
            0.5,
            fifths(12.0..15.0) & balanced(),
        )
        .rule(Bid::new(1, Strain::Notrump), 0.2, hcp(0..))
}

/// Opener's rebid after `1♣ – 1♦`
///
/// Under the up-the-line completion (`set_up_the_line`) a six-plus club suit
/// rebids a natural `2♣` — without it those hands land in the misdescribed
/// 1NT catch-all.
fn rebid_one_club_one_diamond() -> Rules {
    let mut rules = Rules::new()
        .rule(Bid::new(1, Strain::Hearts), 1.3, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(1, Strain::Spades),
            1.3,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.5,
            support(4..) & points(16..=18),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.2,
            support(4..) & points(12..=15),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            1.1,
            fifths(18.0..20.0) & balanced(),
        );
    if super::responses::up_the_line() {
        rules = rules.rule(Bid::new(2, Strain::Clubs), 0.9, len(Suit::Clubs, 6..));
    }
    rules
        .rule(
            Bid::new(1, Strain::Notrump),
            0.5,
            fifths(12.0..15.0) & balanced(),
        )
        .rule(Bid::new(1, Strain::Notrump), 0.2, hcp(0..))
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

/// Responder's call over opener's invitational `3m` jump (Meckstroth adjunct)
///
/// Opener has shown 5+ of the minor and ≈15–17 points.  Responder accepts game
/// with a maximum forcing-1NT (or `1♠`) hand and declines to a preference in
/// opener's five-card major with a minimum.  The `len(major, ..)` guards keep
/// the major-preference rules dead when responder is short, so one table serves
/// both the forcing-1NT auctions and `1♥ – 1♠` (where responder's holding in
/// opener's major is unknown).
///
/// | Call   | Wt  | Meaning |
/// |--------|-----|---------|
/// | 4M     | 1.4 | Accept: 5-3 major game (3+ support, 10+ points) |
/// | 3NT    | 1.2 | Accept: notrump game, no major fit (10+ points) |
/// | 3M     | 1.0 | Decline: preference to opener's major (2+ cards, minimum) |
/// | Pass   | 0.0 | Decline: minimum, short in the major — pass the invite |
fn responder_after_invitational_minor(major: Suit) -> Rules {
    let trump = Strain::from(major);
    Rules::new()
        // Accept to the 5-3 major game.
        .rule(Bid::new(4, trump), 1.4, len(major, 3..) & points(10..))
        // Accept to notrump game with no major fit.
        .rule(Bid::new(3, Strain::Notrump), 1.2, points(10..))
        // Decline: preference to opener's five-card major.
        .rule(Bid::new(3, trump), 1.0, len(major, 2..) & points(..10))
        // Catch-all: minimum, short in the major — pass the invitation.
        // ponytail: a 5m minor game is folded into 3NT; add an explicit 5m raise
        // if the A/B shows it matters.
        .rule(Call::Pass, 0.0, points(0..))
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
/// Accept with 14+ points (bid game in the major), decline with a pass.
fn opener_accept_limit_raise(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 1.0, points(14..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Register responder's second call and opener's acceptance in the
/// forcing-1NT structure
///
/// For each major and each distinct opener rebid that is NOT 2NT (the 18–19
/// balanced rebid's continuations live in the notrump module) and NOT a
/// Meckstroth `3m` jump (handled by `register_invitational_minor_continuations`),
/// inserts responder's table at `[1M, 1NT, rebid]` and opener's acceptances at
/// `[1M, 1NT, rebid, 2NT]` and `[1M, 1NT, rebid, 3M]`.
fn register_forcing_notrump_continuations(book: &mut Trie) {
    for major in [Suit::Hearts, Suit::Spades] {
        let one_major = call(1, Strain::from(major));
        let one_nt = call(1, Strain::Notrump);

        // Collect distinct rebid calls that take the shared two-level
        // continuation: everything except the 2NT rebid and the `3m` jumps.
        let mut seen: Vec<Call> = Vec::new();
        for rule in rebid_after_forcing_notrump(major).rules() {
            let rebid = rule.call();
            if rebid != call(2, Strain::Notrump)
                && !is_invitational_minor_jump(rebid)
                && !seen.contains(&rebid)
            {
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

    register_invitational_minor_continuations(book);
}

/// Register responder's call over opener's invitational `3m` (Meckstroth adjunct)
///
/// Covers both the forcing-1NT auctions (`1M – 1NT – 3m`) and the `1♥ – 1♠`
/// auction (`1♥ – 1♠ – 3m`, where opener's major is hearts).  A no-op when the
/// adjunct is disabled — opener's tables then carry no `3m` jump to continue.
fn register_invitational_minor_continuations(book: &mut Trie) {
    if !meckstroth() {
        return;
    }
    let three_minors = [call(3, Strain::Clubs), call(3, Strain::Diamonds)];

    // Forcing 1NT: 1M – 1NT – 3m, responder's major support unknown.
    for major in [Suit::Hearts, Suit::Spades] {
        let prefix = [call(1, Strain::from(major)), call(1, Strain::Notrump)];
        for three_m in three_minors {
            insert_uncontested(
                book,
                &[prefix[0], prefix[1], three_m],
                responder_after_invitational_minor(major),
            );
        }
    }

    // 1♥ – 1♠ – 3m: opener's major is hearts, responder has shown 4+ spades.
    let one_heart = call(1, Strain::Hearts);
    let one_spade = call(1, Strain::Spades);
    for three_m in three_minors {
        insert_uncontested(
            book,
            &[one_heart, one_spade, three_m],
            responder_after_invitational_minor(Suit::Hearts),
        );
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
