//! XYZ — the two-way checkback after three one-level bids
//!
//! In effect on the ten uncontested auctions where our side made three bids
//! at the one level (`1x – 1y – 1z`, `z` a suit or notrump): responder's
//! **`2♣` puppets opener to `2♦`** — either a weak hand signing off in
//! diamonds (passes `2♦`) or any invitational hand (continues naturally) —
//! and **`2♦` is an artificial game force**, after which bidding is natural.
//! Direct two-level rebids are weak sign-offs.  The known cost: the natural
//! `2♣` sign-off becomes an orphan.
//!
//! Everything is gated on [`set_xyz`] — default **on**, shipped with
//! `set_up_the_line` (`ab-minor-continuations`, 300k boards: the pair is
//! plain +0.0382/+0.0559 IMPs/board NV/vul, PD +0.0289/+0.0407; XYZ alone is
//! plain +0.504/+0.795 per divergent, PD +0.332/+0.472 — a win on both
//! scorers).  With the knob off, `register` authors nothing.
//!
//! ponytail: pure puppet — opener never breaks the relay ("have a good
//! reason; most of the time accept" — the good reasons are rare enough to
//! skip).  Direct three-level jumps stay with the floor, and the contested
//! tails (they double `2♣`) rely on alert-reading: the relay's projection
//! carries no phantom club suit, so the floor defends sanely.

use super::{call, insert_uncontested};
use crate::bidding::constraint::{balanced, len, points};
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

    /// Whether opener judges the invitations that stop below game
    /// ([`accept_or_decline`]).  Default `true`; see
    /// [`set_xyz_invite_judgment`].
    static XYZ_INVITE_JUDGMENT: Cell<bool> = const { Cell::new(true) };
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
fn xyz() -> bool {
    XYZ.with(Cell::get)
}

/// Author opener's judgment of the invitations that stop below game
///
/// Read at book-construction time; default `true` (the shipped behavior).
/// The table is two rules — `points(14..)` bids the game, else `Pass` — with
/// no shape, fit or vulnerability term, the same signature as the retired 2/1
/// game backstop.  Off, it becomes an empty table, which is all-−∞ and so
/// falls through to `instinct()` by the documented escape hatch.
///
/// The most-*reached* candidate of the constructive book re-audit
/// (`probe-node-reach`: 0.114% on one key, and the table is registered once per
/// invite per prefix).  Only the crude `accept_or_decline` copies are gated;
/// the shaped acceptances (three-card support, the 5♠4♥ hand) always author.
pub fn set_xyz_invite_judgment(on: bool) {
    XYZ_INVITE_JUDGMENT.with(|cell| cell.set(on));
}

fn xyz_invite_judgment() -> bool {
    XYZ_INVITE_JUDGMENT.with(Cell::get)
}

/// Responder's rebid at `1x – 1y – 1z`: the XYZ round
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
            1.5,
            points(10..=12) | (len(Suit::Diamonds, 6..) & points(..=9)),
        )
        .alert(XYZ_RELAY)
        .rule(Bid::new(2, Strain::Diamonds), 1.4, points(13..))
        .alert(XYZ_FORCE);
    // Weak raise of opener's second-suit major.
    if let Some(second) = rebid.suit() {
        rules = rules.rule(Bid::new(2, rebid), 1.15, len(second, 4..) & points(6..=9));
    }
    // Weak rebid of responder's own major, to play.
    if response != Suit::Diamonds {
        rules = rules.rule(
            Bid::new(2, Strain::from(response)),
            1.1,
            len(response, 5..) & points(..=9),
        );
    }
    // The weak 5♠4♥ hand shows its second suit, to play.
    if response == Suit::Spades && rebid == Strain::Notrump {
        rules = rules.rule(
            Bid::new(2, Strain::Hearts),
            1.05,
            len(Suit::Hearts, 4..) & points(..=9),
        );
    }
    rules.rule(Call::Pass, 0.0, points(..=9))
}

/// Opener completes the puppet: `2♦`, always
fn xyz_completion() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Diamonds), 0.0, points(0..))
        .alert(XYZ_COMPLETION)
}

/// Responder's continuation after `2♣ – 2♦`: pass out the sign-off, or invite
///
/// Every bid here is invitational (10–12); pass is the weak-diamond sign-off
/// the relay promised.
fn xyz_after_relay(opening: Suit, response: Suit, rebid: Strain) -> Rules {
    let mut rules = Rules::new();
    // Invitational raise of opener's second-suit major — fit first.
    if let Some(second) = rebid.suit() {
        rules = rules.rule(Bid::new(2, rebid), 1.3, len(second, 4..) & points(10..=12));
    }
    // Invitational rebid of responder's own major (5+).
    if response != Suit::Diamonds {
        rules = rules.rule(
            Bid::new(2, Strain::from(response)),
            1.2,
            len(response, 5..) & points(10..=12),
        );
    }
    // The invitational 5♠4♥ hand shows its second suit.
    if response == Suit::Spades && rebid == Strain::Notrump {
        rules = rules.rule(
            Bid::new(2, Strain::Hearts),
            1.1,
            len(Suit::Hearts, 4..) & points(10..=12),
        );
    }
    // Minor-suit invites: support for opener's minor, or a long suit of our own.
    for minor in [Suit::Clubs, Suit::Diamonds] {
        let long = if minor == opening { 5 } else { 6 };
        rules = rules.rule(
            Bid::new(3, Strain::from(minor)),
            1.0,
            len(minor, long..) & points(10..=12),
        );
    }
    rules
        // Balanced invite, and the finite catch-all for every 10+ hand.
        .rule(Bid::new(2, Strain::Notrump), 0.2, points(10..))
        // The weak sign-off: the relay promised diamonds.
        .rule(Call::Pass, 0.0, points(..=9))
}

/// Opener accepts (14+) or declines an invitation reached through the relay
///
/// Empty when [`set_xyz_invite_judgment`] is off: an all-−∞ table is the
/// documented fall-through, so the node lands on the floor without the
/// registration sites needing to know.
fn accept_or_decline(game: Bid) -> Rules {
    if !xyz_invite_judgment() {
        return Rules::new();
    }
    Rules::new()
        .rule(game, 1.0, points(14..))
        .rule(Call::Pass, 0.0, points(0..))
}

/// Opener's answer to the `2♦` game force: natural, cheapest useful feature
///
/// Three-card support for responder's major first, then a concealed second
/// major, then shape; `2NT` is the balanced-minimum catch-all.
fn xyz_gf_answers(opening: Suit, response: Suit, rebid: Strain) -> Rules {
    let mut rules = Rules::new();
    // Three-card support for responder's major.
    if response != Suit::Diamonds {
        rules = rules.rule(Bid::new(2, Strain::from(response)), 1.3, len(response, 3..));
    }
    // A concealed four-card spade suit (the 1♥ rebid was bid up the line).
    if rebid == Strain::Hearts {
        rules = rules.rule(Bid::new(2, Strain::Spades), 1.2, len(Suit::Spades, 4..));
    }
    // Opener's five-card heart suit after 1♥ – 1♠ – 1NT.
    if opening == Suit::Hearts {
        rules = rules.rule(Bid::new(2, Strain::Hearts), 1.2, len(Suit::Hearts, 6..));
    }
    // A four-card diamond raise after a 1♦ response.
    if response == Suit::Diamonds {
        rules = rules.rule(Bid::new(3, Strain::Diamonds), 1.1, len(Suit::Diamonds, 4..));
    }
    // A six-card minor rebids its suit.
    if opening != Suit::Hearts {
        rules = rules.rule(Bid::new(3, Strain::from(opening)), 0.8, len(opening, 6..));
    }
    rules
        .rule(
            Bid::new(2, Strain::Notrump),
            1.0,
            points(12..=14) & balanced(),
        )
        // Guaranteed-legal catch-all — the force may not be passed.
        .rule(Bid::new(2, Strain::Notrump), 0.1, points(0..))
}

/// Register the XYZ tree under one `1x – 1y – 1z` prefix
fn register_prefix(book: &mut Trie, opening: Suit, response: Suit, rebid: Strain) {
    let prefix = [
        call(1, Strain::from(opening)),
        call(1, Strain::from(response)),
        call(1, rebid),
    ];
    let two_c = call(2, Strain::Clubs);
    let two_d = call(2, Strain::Diamonds);

    // Responder's XYZ round, the forced completion, and the game force.
    insert_uncontested(book, &prefix, xyz_responder(response, rebid));
    insert_uncontested(
        book,
        &[prefix[0], prefix[1], prefix[2], two_c],
        xyz_completion(),
    );
    insert_uncontested(
        book,
        &[prefix[0], prefix[1], prefix[2], two_d],
        xyz_gf_answers(opening, response, rebid),
    );

    // The invitational round after the relay, and opener's acceptances.
    let relay = [prefix[0], prefix[1], prefix[2], two_c, two_d];
    insert_uncontested(book, &relay, xyz_after_relay(opening, response, rebid));

    let mut accept = |invite: Call, table: Rules| {
        let key = [relay[0], relay[1], relay[2], relay[3], relay[4], invite];
        insert_uncontested(book, &key, table);
    };
    if rebid.suit().is_some() {
        // Raise of opener's second-suit major → game in it.
        accept(call(2, rebid), accept_or_decline(Bid::new(4, rebid)));
    }
    if response != Suit::Diamonds {
        // Responder's own-major invite → game with a third trump, else 3NT.
        let major = Strain::from(response);
        accept(
            call(2, major),
            Rules::new()
                .rule(Bid::new(4, major), 1.2, len(response, 3..) & points(14..))
                .rule(Bid::new(3, Strain::Notrump), 1.0, points(14..))
                .rule(Call::Pass, 0.0, points(0..)),
        );
    }
    if response == Suit::Spades && rebid == Strain::Notrump {
        // The 5♠4♥ invite: raise either major with a fit, else 3NT.
        accept(
            call(2, Strain::Hearts),
            Rules::new()
                .rule(
                    Bid::new(4, Strain::Spades),
                    1.3,
                    len(Suit::Spades, 3..) & points(14..),
                )
                .rule(
                    Bid::new(4, Strain::Hearts),
                    1.2,
                    len(Suit::Hearts, 4..) & points(14..),
                )
                .rule(Bid::new(3, Strain::Notrump), 1.0, points(14..))
                .rule(Call::Pass, 0.0, points(0..)),
        );
    }
    for minor in [Suit::Clubs, Suit::Diamonds] {
        accept(
            call(3, Strain::from(minor)),
            accept_or_decline(Bid::new(3, Strain::Notrump)),
        );
    }
    accept(
        call(2, Strain::Notrump),
        accept_or_decline(Bid::new(3, Strain::Notrump)),
    );
}

/// Register the XYZ structure on all ten one-level prefixes (no-op when off)
///
/// On the four `1m – 1M – 1NT` slots, [New Minor Forcing](super::nmf) overrides
/// XYZ when its knob is on (default off) — the two conventions are mutually
/// exclusive on that node, so at most one is authored there.
pub(super) fn register(book: &mut Trie) {
    let nmf = super::nmf::new_minor_forcing();
    if !xyz() && !nmf {
        return;
    }
    for opening in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        for response in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            if Strain::from(response) <= Strain::from(opening) {
                continue;
            }
            if xyz() {
                for higher in Suit::ASC {
                    if Strain::from(higher) > Strain::from(response) {
                        register_prefix(book, opening, response, Strain::from(higher));
                    }
                }
            }
            // The 1NT rebid: NMF claims the four minor-opening/major-response
            // slots when on, otherwise XYZ (when on) as before.
            if nmf && super::nmf::is_nmf_slot(opening, response) {
                super::nmf::register_prefix(book, opening, response);
            } else if xyz() {
                register_prefix(book, opening, response, Strain::Notrump);
            }
        }
    }
}
