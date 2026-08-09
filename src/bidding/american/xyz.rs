//! XYZ — the two-way checkback after three one-level bids
//!
//! In effect on the ten uncontested auctions where our side made three bids
//! at the one level (`1x - 1y - 1z`, `z` a suit or notrump): responder's
//! **`2♣` puppets opener to `2♦`** — either a weak hand signing off in
//! diamonds (passes `2♦`) or any invitational hand (continues naturally) —
//! and **`2♦` is an artificial game force**, after which bidding is natural.
//! Direct two-level rebids are weak sign-offs.  The known cost: the natural
//! `2♣` sign-off becomes an orphan.
//!
//! Everything is gated on [`set_xyz`] — default **on**, shipped with
//! `up_the_line` (`ab-minor-continuations`, 300k boards: the pair is
//! plain +0.0382/+0.0559 IMPs/board NV/vul, PD +0.0289/+0.0407; XYZ alone is
//! plain +0.504/+0.795 per divergent, PD +0.332/+0.472 — a win on both
//! scorers).  With the knob off, `register` authors nothing.
//!
//! ponytail: pure puppet — opener never breaks the relay ("have a good
//! reason; most of the time accept" — the good reasons are rare enough to
//! skip).  Direct three-level jumps stay with the floor, and the contested
//! tails (they double `2♣`) rely on alert-reading: the relay's projection
//! carries no phantom club suit, so the floor defends sanely.

use super::call;
use crate::bidding::agreements::{Agreements, RebidKnobs};
use crate::bidding::constraint::{balanced, len, points};
use crate::bidding::rows::{Entry, Package, Pattern, compile_into, rows_of};
use crate::bidding::{Alert, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};
use std::cell::Cell;

/// XYZ `2♣` — puppet to `2♦`: weak with diamonds, or any invitation
const XYZ_RELAY: Alert = Alert("xyz-relay");
/// XYZ `2♦` — artificial game force
const XYZ_FORCE: Alert = Alert("xyz-game-force");
/// Opener's forced `2♦` completing the puppet — says nothing about diamonds
const XYZ_COMPLETION: Alert = Alert("xyz-completion");

std::thread_local! {
    /// Whether the XYZ structure is authored.  Default `true` (see the
    /// module doc for the measured verdict).
    static XYZ: Cell<bool> = const { Cell::new(true) };
}

/// Author XYZ for books built *after* this call (default `true`; off-switch
/// `--no-ns-xyz` in `bba-gen`)
///
/// Read at book-construction time; set it before building the [`Pair`]
/// (`register` authors the whole tree or nothing).
///
/// [`Pair`]: crate::bidding::Pair
pub fn set_xyz(on: bool) {
    XYZ.with(|cell| cell.set(on));
}

/// Whether XYZ is currently authored
pub(crate) fn xyz() -> bool {
    XYZ.with(Cell::get)
}

/// Responder's rebid at `1x - 1y - 1z`: the XYZ round
///
/// | Call | Wt | Meaning |
/// |------|----|---------|
/// | 2♣   | 1.5 | Relay to 2♦ — any invite (10–12), or weak with 6+ diamonds |
/// | 2♦   | 1.4 | Artificial game force (13+) |
/// | 2z (z a major) | 1.15 | Weak raise of opener's second suit (4 trumps) |
/// | 2y (y a major) | 1.1 | Weak rebid, 5+ suit, to play |
/// | 2♥ (y = ♠, z = NT) | 1.05 | Weak second suit, to play |
/// | Pass | 0.0 | Weak, nothing to say |
fn xyz_responder(response: Suit, rebid: Strain) -> Rules {
    let mut rules = Rules::new()
        .rule(
            Bid::new(2, Strain::Clubs),
            150,
            points(10..=12) | (len(Suit::Diamonds, 6..) & points(..=9)),
        )
        .alert(XYZ_RELAY)
        .rule(Bid::new(2, Strain::Diamonds), 140, points(13..))
        .alert(XYZ_FORCE);
    // Weak raise of opener's second-suit major.
    if let Some(second) = rebid.suit() {
        rules = rules.rule(Bid::new(2, rebid), 115, len(second, 4..) & points(6..=9));
    }
    // Weak rebid of responder's own major, to play.
    if response != Suit::Diamonds {
        rules = rules.rule(
            Bid::new(2, Strain::from(response)),
            110,
            len(response, 5..) & points(..=9),
        );
    }
    // The weak 5♠4♥ hand shows its second suit, to play.
    if response == Suit::Spades && rebid == Strain::Notrump {
        rules = rules.rule(
            Bid::new(2, Strain::Hearts),
            105,
            len(Suit::Hearts, 4..) & points(..=9),
        );
    }
    rules.rule(Call::Pass, 0, points(..=9))
}

/// Opener completes the puppet: `2♦`, always
fn xyz_completion() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Diamonds), 0, points(0..))
        .alert(XYZ_COMPLETION)
}

/// Responder's continuation after `2♣ - 2♦`: pass out the sign-off, or invite
///
/// Every bid here is invitational (10–12); pass is the weak-diamond sign-off
/// the relay promised.
fn xyz_after_relay(opening: Suit, response: Suit, rebid: Strain) -> Rules {
    let mut rules = Rules::new();
    // Invitational raise of opener's second-suit major — fit first.
    if let Some(second) = rebid.suit() {
        rules = rules.rule(Bid::new(2, rebid), 130, len(second, 4..) & points(10..=12));
    }
    // Invitational rebid of responder's own major (5+).
    if response != Suit::Diamonds {
        rules = rules.rule(
            Bid::new(2, Strain::from(response)),
            120,
            len(response, 5..) & points(10..=12),
        );
    }
    // The invitational 5♠4♥ hand shows its second suit.
    if response == Suit::Spades && rebid == Strain::Notrump {
        rules = rules.rule(
            Bid::new(2, Strain::Hearts),
            110,
            len(Suit::Hearts, 4..) & points(10..=12),
        );
    }
    // Minor-suit invites: support for opener's minor, or a long suit of our own.
    for minor in [Suit::Clubs, Suit::Diamonds] {
        let long = if minor == opening { 5 } else { 6 };
        rules = rules.rule(
            Bid::new(3, Strain::from(minor)),
            100,
            len(minor, long..) & points(10..=12),
        );
    }
    rules
        // Balanced invite, and the finite catch-all for every 10+ hand.
        .rule(Bid::new(2, Strain::Notrump), 20, points(10..))
        // The weak sign-off: the relay promised diamonds.
        .rule(Call::Pass, 0, points(..=9))
}

/// Opener accepts (14+) or declines an invitation reached through the relay
///
/// Empty when [`RebidKnobs::xyz_invite_judgment`] is off: an all-−∞ table is the
/// documented fall-through, so the node lands on the floor without the
/// registration sites needing to know.
fn accept_or_decline(game: Bid, knobs: &RebidKnobs) -> Rules {
    if !knobs.xyz_invite_judgment {
        return Rules::new();
    }
    Rules::new()
        .rule(game, 100, points(14..))
        .rule(Call::Pass, 0, points(0..))
}

/// Opener's answer to the `2♦` game force: natural, cheapest useful feature
///
/// Three-card support for responder's major first, then a concealed second
/// major, then shape; `2NT` is the balanced-minimum catch-all.
fn xyz_gf_answers(opening: Suit, response: Suit, rebid: Strain) -> Rules {
    let mut rules = Rules::new();
    // Three-card support for responder's major.
    if response != Suit::Diamonds {
        rules = rules.rule(Bid::new(2, Strain::from(response)), 130, len(response, 3..));
    }
    // A concealed four-card spade suit (the 1♥ rebid was bid up the line).
    if rebid == Strain::Hearts {
        rules = rules.rule(Bid::new(2, Strain::Spades), 120, len(Suit::Spades, 4..));
    }
    // Opener's five-card heart suit after 1♥ - 1♠ - 1NT.
    if opening == Suit::Hearts {
        rules = rules.rule(Bid::new(2, Strain::Hearts), 120, len(Suit::Hearts, 6..));
    }
    // A four-card diamond raise after a 1♦ response.
    if response == Suit::Diamonds {
        rules = rules.rule(Bid::new(3, Strain::Diamonds), 110, len(Suit::Diamonds, 4..));
    }
    // A six-card minor rebids its suit.
    if opening != Suit::Hearts {
        rules = rules.rule(Bid::new(3, Strain::from(opening)), 80, len(opening, 6..));
    }
    rules
        .rule(
            Bid::new(2, Strain::Notrump),
            100,
            points(12..=14) & balanced(),
        )
        // Guaranteed-legal catch-all — the force may not be passed.
        .rule(Bid::new(2, Strain::Notrump), 10, points(0..))
}

/// The XYZ tree under one `1x - 1y - 1z` prefix
fn rows_for_prefix(opening: Suit, response: Suit, rebid: Strain, knobs: &RebidKnobs) -> Vec<Entry> {
    let prefix = format!(
        "P* {} - {} - {} -",
        call(1, Strain::from(opening)),
        call(1, Strain::from(response)),
        call(1, rebid),
    );

    let mut entries = Vec::new();

    // Responder's XYZ round, the forced completion, and the game force.
    entries.extend(rows_of(
        Pattern::node(&prefix),
        xyz_responder(response, rebid),
    ));
    entries.extend(rows_of(
        Pattern::node(&format!("{prefix} 2♣ -")),
        xyz_completion(),
    ));
    entries.extend(rows_of(
        Pattern::node(&format!("{prefix} 2♦ -")),
        xyz_gf_answers(opening, response, rebid),
    ));

    // The invitational round after the relay, and opener's acceptances.
    let relay = format!("{prefix} 2♣ - 2♦ -");
    entries.extend(rows_of(
        Pattern::node(&relay),
        xyz_after_relay(opening, response, rebid),
    ));

    let mut accept = |invite: Call, table: Rules| {
        entries.extend(rows_of(
            Pattern::node(&format!("{relay} {invite} -")),
            table,
        ));
    };
    if rebid.suit().is_some() {
        // Raise of opener's second-suit major → game in it.
        accept(call(2, rebid), accept_or_decline(Bid::new(4, rebid), knobs));
    }
    if response != Suit::Diamonds {
        // Responder's own-major invite → game with a third trump, else 3NT.
        let major = Strain::from(response);
        accept(
            call(2, major),
            Rules::new()
                .rule(Bid::new(4, major), 120, len(response, 3..) & points(14..))
                .rule(Bid::new(3, Strain::Notrump), 100, points(14..))
                .rule(Call::Pass, 0, points(0..)),
        );
    }
    if response == Suit::Spades && rebid == Strain::Notrump {
        // The 5♠4♥ invite: raise either major with a fit, else 3NT.
        accept(
            call(2, Strain::Hearts),
            Rules::new()
                .rule(
                    Bid::new(4, Strain::Spades),
                    130,
                    len(Suit::Spades, 3..) & points(14..),
                )
                .rule(
                    Bid::new(4, Strain::Hearts),
                    120,
                    len(Suit::Hearts, 4..) & points(14..),
                )
                .rule(Bid::new(3, Strain::Notrump), 100, points(14..))
                .rule(Call::Pass, 0, points(0..)),
        );
    }
    for minor in [Suit::Clubs, Suit::Diamonds] {
        accept(
            call(3, Strain::from(minor)),
            accept_or_decline(Bid::new(3, Strain::Notrump), knobs),
        );
    }
    accept(
        call(2, Strain::Notrump),
        accept_or_decline(Bid::new(3, Strain::Notrump), knobs),
    );

    entries
}

/// The XYZ structure on all ten one-level prefixes (no-op when off)
///
/// On the four `1m - 1M - 1NT` slots, [New Minor Forcing](super::nmf) overrides
/// XYZ when its knob is on (default off) — the two conventions are mutually
/// exclusive on that node, so this package yields those slots and
/// [`nmf::package`][super::nmf::package] writes them instead.
pub(super) fn package() -> Package {
    Package {
        name: "xyz",
        gate: |a| a.decision.reading.xyz(),
        entries: |agreements| {
            let knobs = &agreements.rebid;
            let nmf = knobs.new_minor_forcing;
            let mut entries = Vec::new();
            for opening in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
                for response in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                    if Strain::from(response) <= Strain::from(opening) {
                        continue;
                    }
                    for higher in Suit::ASC {
                        if Strain::from(higher) > Strain::from(response) {
                            entries.extend(rows_for_prefix(
                                opening,
                                response,
                                Strain::from(higher),
                                knobs,
                            ));
                        }
                    }
                    // The 1NT rebid: NMF claims the four minor-opening/
                    // major-response slots when on, otherwise XYZ as before.
                    if !(nmf && super::nmf::is_nmf_slot(opening, response)) {
                        entries.extend(rows_for_prefix(opening, response, Strain::Notrump, knobs));
                    }
                }
            }
            entries
        },
    }
}

/// Register both checkback conventions; each package is a no-op when its knob
/// is off, and their keys are disjoint by construction
pub(super) fn register(book: &mut Trie, agreements: &Agreements) {
    compile_into(book, agreements, &[package(), super::nmf::package()]);
}
