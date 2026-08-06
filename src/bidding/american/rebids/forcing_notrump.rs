//! Responder's second call after the forcing `1NT`, and opener's acceptance
//!
//! One shared table covers every opener rebid that is *not* the `2NT` (whose
//! continuations live in `notrump.rs` or, under the adjunct, in
//! [`super::meckstroth`]), not a Meckstroth `3m` jump, and not an invitational
//! two-suiter.  Always on — this is the base structure the adjuncts overlay.

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
/// | 3M     | 1.5 | Three-card limit raise (10–12 HCP) |
/// | 2NT    | 1.2 | Natural notrump invite (11–12 HCP) |
/// | 2x≠M   | 1.1 | Six-card runout, weak (≤ 9 HCP); dead when illegal |
/// | 2M     | 1.0 | Preference to the major (7+ HCP, 2+ cards) |
/// | Pass   | 0.0 | Catch-all: the force was one round only |
fn responder_after_forcing_notrump(major: Suit) -> Rules {
    let trump = Strain::from(major);
    let mut rules = Rules::new()
        // Three-card limit raise — the standard 2/1 route: 1NT then 3M.
        .rule(Bid::new(3, trump), 150, len(major, 3..) & hcp(10..=12))
        // Natural notrump invite.
        .rule(Bid::new(2, Strain::Notrump), 120, hcp(11..=12))
        // Preference to opener's major.
        .rule(Bid::new(2, trump), 100, len(major, 2..) & hcp(7..))
        // Catch-all pass; the forcing 1NT is one round only.
        .rule(Call::Pass, 0, hcp(0..));

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
    rules
}

/// Responder's second call and opener's acceptance in the forcing-1NT structure
///
/// For each major and each distinct opener rebid that is NOT 2NT (the 18–19
/// balanced rebid's continuations live in the notrump module) and NOT a
/// Meckstroth `3m` jump (handled by
/// [`invitational_minor_continuations`](super::invitational_minor_continuations)),
/// authors responder's table at `[1M, 1NT, rebid]` and opener's acceptances at
/// `[1M, 1NT, rebid, 2NT]` and `[1M, 1NT, rebid, 3M]`.
pub(crate) fn forcing_notrump_continuations() -> Package {
    Package {
        name: "forcing-notrump-continuations",
        gate: || true,
        entries: || {
            let mut entries = Vec::new();
            for major in [Suit::Hearts, Suit::Spades] {
                // Collect distinct rebid calls that take the shared two-level
                // continuation: everything except the 2NT rebid, the `3m`
                // jumps and the two-suiter calls.  This must stay derived from
                // the knob-built source table rather than duplicating its
                // filters in a row template.
                let mut seen: Vec<Call> = Vec::new();
                for rule in rebid_after_forcing_notrump(major).rules() {
                    let rebid = rule.call();
                    if rebid != call(2, Strain::Notrump)
                        && !is_invitational_minor_jump(rebid)
                        && !is_forcing_nt_two_suiter(major, rebid)
                        && !seen.contains(&rebid)
                    {
                        seen.push(rebid);
                    }
                }

                for rebid in seen {
                    let prefix = format!(
                        "P* {} (P) 1NT (P) {rebid} (P)",
                        call(1, Strain::from(major)),
                    );
                    entries.extend(rows_of(
                        Pattern::node(&prefix),
                        responder_after_forcing_notrump(major),
                    ));
                    entries.extend(rows_of(
                        Pattern::node(&format!("{prefix} 2NT (P)")),
                        opener_accept_notrump_invite(),
                    ));
                    entries.extend(rows_of(
                        Pattern::node(&format!("{prefix} {} (P)", call(3, Strain::from(major)),)),
                        opener_accept_limit_raise(major),
                    ));
                }
            }
            entries
        },
    }
}
