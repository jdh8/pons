//! Responder's second call after the forcing `1NT`, and opener's acceptance
//!
//! One shared table covers every opener rebid that is *not* the `2NT` (whose
//! continuations live in `notrump.rs` or, under the adjunct, in
//! [`super::meckstroth`]), not a Meckstroth `3m` jump, and not an invitational
//! two-suiter.  Always on — this is the base structure the adjuncts overlay.

use super::jump_shifts::is_forcing_nt_jump_shift;
use super::meckstroth::is_invitational_minor_jump;
use super::two_suiter::is_forcing_nt_two_suiter;
use super::*;

/// Responder's options after opener's rebid in the forcing-1NT structure
///
/// One shared table covers every opener rebid; rules for calls that are
/// illegal in a particular sequence simply go dead.  The table in priority
/// order:
///
/// | Call   | Wt  | Meaning |
/// |--------|-----|---------|
/// | 3♥     | 1.55 | Four-card raise of opener's `2♥` over `1♠` (10–12 HCP), knob `forcing_notrump_heart_raise` |
/// | 3M     | 1.5 | Three-card limit raise (10–12 HCP); two cards over opener's own `2M`, knob `forcing_notrump_doubleton_raise` |
/// | 3♥     | 1.3 | Six-card invite over `1♠` (10–12 HCP), knob `forcing_notrump_suit_invite`; not over `2♥` |
/// | 2NT    | 1.2 | Natural notrump invite (11–12 HCP) |
/// | 2x≠M   | 1.1 | Six-card runout, weak (≤ 9 HCP); dead when illegal |
/// | 2M     | 1.0 | Preference to the major (7+ HCP, 2+ cards); under the heart raise, not with four hearts over `2♥` |
/// | Pass   | 0.0 | Catch-all: the force was one round only |
fn responder_after_forcing_notrump(
    major: Suit,
    rebid: Call,
    suit_invite: bool,
    doubleton_raise: bool,
    heart_raise: bool,
) -> Rules {
    let trump = Strain::from(major);
    // Opener's own `2M` rebid shows six, so a doubleton is an eight-card fit.
    let support = if doubleton_raise && rebid == call(2, trump) {
        2
    } else {
        3
    };
    let mut rules = Rules::new()
        // Limit raise — the standard 2/1 route: 1NT then 3M.
        .rule(
            Bid::new(3, trump),
            150,
            len(major, support..) & hcp(10..=12),
        )
        // Natural notrump invite.
        .rule(Bid::new(2, Strain::Notrump), 120, hcp(11..=12))
        // Catch-all pass; the forcing 1NT is one round only.
        .rule(Call::Pass, 0, hcp(0..));

    // Raise opener's `2♥` over `1♠`: a 4-4 fit outranks the spade preference,
    // so four hearts invite on 10–12 or pass below that.
    if heart_raise && major == Suit::Spades && rebid == call(2, Strain::Hearts) {
        rules = rules
            .rule(
                Bid::new(3, Strain::Hearts),
                155,
                len(Suit::Hearts, 4..) & hcp(10..=12),
            )
            .rule(
                Bid::new(2, trump),
                100,
                len(major, 2..) & len(Suit::Hearts, ..=3) & hcp(7..),
            );
    } else {
        // Preference to opener's major.
        rules = rules.rule(Bid::new(2, trump), 100, len(major, 2..) & hcp(7..));
    }

    // Six-card runouts into a side suit (dead when the call is illegal in
    // the current auction).
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit != major {
            rules = rules.rule(
                Bid::new(2, Strain::from(suit)),
                110,
                len(suit, 6..) & hcp(..=9),
            );
        }
    }
    // The invitational jump in hearts over `1♠` (over opener's own `2♥` it
    // would be a raise, not this hand).  The minor invites lost (CHANGELOG):
    // opener passes `3m` where `2NT` found `3NT`.
    if suit_invite && is_heart_invite(major, rebid) {
        rules = rules.rule(
            Bid::new(3, Strain::Hearts),
            130,
            len(Suit::Hearts, 6..) & hcp(10..=12),
        );
    }
    rules
}

/// Whether responder's `3♥` after `1M - 1NT - rebid` is the six-card invite:
/// over `1♠`, unless opener rebid hearts
fn is_heart_invite(major: Suit, rebid: Call) -> bool {
    major == Suit::Spades && rebid != call(2, Strain::Hearts)
}

/// Opener's answer to responder's six-card heart invite
///
/// Game on two-card support and 14+ points, `3NT` on 15+ HCP, else a pass.
fn opener_accept_heart_invite() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Hearts),
            110,
            len(Suit::Hearts, 2..) & points(14..),
        )
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(15..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's second call and opener's acceptance in the forcing-1NT structure
///
/// For each major and each distinct opener rebid that is NOT 2NT (the 18–19
/// balanced rebid's continuations live in the notrump module) and NOT a
/// Meckstroth `3m` jump (handled by
/// [`invitational_minor_continuations`](super::invitational_minor_continuations)),
/// nor a two-suiter or jump-shift rung (each has its own package), authors responder's table at `1M - 1NT - rebid -` and opener's acceptances at
/// `1M - 1NT - rebid - 2NT -` and `1M - 1NT - rebid - 3M -` (and
/// `1♠ - 1NT - 2♥ - 3♥ -` under the heart raise).
pub(crate) fn forcing_notrump_continuations() -> Package {
    Package {
        name: "forcing-notrump-continuations",
        gate: |_| true,
        entries: |agreements| {
            let mut entries = Vec::new();
            for major in [Suit::Hearts, Suit::Spades] {
                // Collect distinct rebid calls that take the shared two-level
                // continuation: everything except the 2NT rebid, the `3m`
                // jumps and the two-suiter calls.  This must stay derived from
                // the knob-built source table rather than duplicating its
                // filters in a row template.
                let mut seen: Vec<Call> = Vec::new();
                for rule in rebid_after_forcing_notrump(major, agreements).rules() {
                    let rebid = rule.call();
                    if rebid != call(2, Strain::Notrump)
                        && !is_invitational_minor_jump(rebid)
                        && !is_forcing_nt_two_suiter(major, rebid)
                        && !is_forcing_nt_jump_shift(major, rebid)
                        && !seen.contains(&rebid)
                    {
                        seen.push(rebid);
                    }
                }

                for rebid in seen {
                    let prefix = format!("P* {} - 1NT - {rebid} -", call(1, Strain::from(major)),);
                    let suit_invite = agreements.rebid.forcing_notrump_suit_invite;
                    entries.extend(rows_of(
                        Pattern::node(&prefix),
                        responder_after_forcing_notrump(
                            major,
                            rebid,
                            suit_invite,
                            agreements.rebid.forcing_notrump_doubleton_raise,
                            agreements.rebid.forcing_notrump_heart_raise,
                        ),
                    ));
                    if agreements.rebid.forcing_notrump_heart_raise
                        && major == Suit::Spades
                        && rebid == call(2, Strain::Hearts)
                    {
                        entries.extend(rows_of(
                            Pattern::node(&format!("{prefix} 3♥ -")),
                            opener_accept_limit_raise(Suit::Hearts),
                        ));
                    }
                    if suit_invite && is_heart_invite(major, rebid) {
                        entries.extend(rows_of(
                            Pattern::node(&format!("{prefix} 3♥ -")),
                            opener_accept_heart_invite(),
                        ));
                    }
                    entries.extend(rows_of(
                        Pattern::node(&format!("{prefix} 2NT -")),
                        opener_accept_notrump_invite(),
                    ));
                    entries.extend(rows_of(
                        Pattern::node(&format!("{prefix} {} -", call(3, Strain::from(major)),)),
                        opener_accept_limit_raise(major),
                    ));
                }
            }
            entries
        },
    }
}
