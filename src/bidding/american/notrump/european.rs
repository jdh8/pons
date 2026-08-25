//! The European minor scheme — `2♠` clubs, `2NT` invite, `3♣` diamonds
//!
//! Opt-in via [`notrump_minors`][field@crate::bidding::inference::ReadingProfile::notrump_minors]
//! set to [`EUROPEAN`][super::EUROPEAN]:
//! BBA's Atlantic style, and the standard Polish Club / WJ treatment.  Replaces
//! [`super::minor_transfers`] and [`super::puppet_stayman`] wholesale — the two schemes
//! wire the same keys under complementary gates.

use super::minor_transfers::diamond_transfer_game;
use super::size_ask::{SizeAskEight, size_ask_eight_class};
use super::*;

/// European minor-suit responses to 1NT (opt-in via
/// [`notrump_minors`][field@crate::bidding::inference::ReadingProfile::notrump_minors])
///
/// `2♠` = transfer to clubs (a six-card one-suiter, weak-to-game).  `2NT` = a
/// balanced invitational eight with no four-card major — the size ask, opener
/// accepting game with a maximum.  `3♣` = transfer to diamonds, the same shape
/// one suit over: a **six-card** one-suiter, weak-to-game.  There is no Puppet
/// Stayman: a game-forcing balanced hand with only a three-card major bids 3NT
/// (the standard continental treatment).
///
/// Both transfers are pinned to EPBot's measured buckets — `2♠` clubs 6–7, `3♣`
/// diamonds 6–7, hard min/max on 40k probe hands
/// ([bba-1nt-minors.md](../../../../docs/ai-bidder/bba-1nt-minors.md)).  This is
/// an **opponent model**, not a system we play: fidelity to EPBot is the
/// acceptance test, so the tables track the probe even where a soundness
/// argument would author something else.
pub(super) fn european_minors(agreements: &Agreements) -> Rules {
    // 2NT = the bare-8 size ask (no four-card major), gated on `size_ask_eight`:
    // `Shipped` excludes the flat 4-3-3-3 (it passes), `Invite` size-asks the whole
    // class, `Pass` drops the 2NT size ask entirely.
    let size_ask = match agreements.notrump.size_ask_eight {
        SizeAskEight::Shipped => Rules::new()
            .rule(
                Bid::new(2, Strain::Notrump),
                130,
                hcp(8..=8)
                    & balanced()
                    & len(Suit::Hearts, ..4)
                    & len(Suit::Spades, ..4)
                    & !flat_4333(),
            )
            .alert(EUROPEAN),
        SizeAskEight::Invite => Rules::new()
            .rule(Bid::new(2, Strain::Notrump), 130, size_ask_eight_class())
            .alert(EUROPEAN),
        SizeAskEight::Pass => Rules::new(),
    };
    Rules::new()
        .rule(Bid::new(2, Strain::Spades), 130, len(Suit::Clubs, 6..))
        .alert(EUROPEAN)
        .chain(size_ask)
        .rule(Bid::new(3, Strain::Clubs), 130, len(Suit::Diamonds, 6..))
        .alert(EUROPEAN)
}

//
// ponytail: opener always completes the 2♠/3♣ transfers — no super-accept.
// Measured, not assumed: `probe-bba-constraints --mode nt-3c` returns a single
// `3♦` bucket at 100.0% (n=2968, 15–17 HCP, 98% balanced).

/// Opener completes the European club transfer: `3♣` (the 2♠ bidder has clubs)
fn european_two_spade_answer(agreements: &Agreements) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Clubs), 0, hcp(0..))
        .alert_if(agreements.decision.reading.completion_alerts, COMPLETION)
}

/// Responder's rebid after opener completes the European club transfer (`…2♠ - 3♣`)
///
/// A weak six-card club one-suiter passes the partscore; game values bid 3NT.
/// The exact twin of the diamond lane's [`diamond_transfer_game`]`(8, false)`,
/// and for the same measured reason.
///
// ponytail: no splinter arm.  `--mode nt-2s-3c` (400k hands, 9929 reaching the
// node) shows **no `3♦`/`3♥`/`3♠` bucket at all** — indeed no three-level call
// but `3NT`.  EPBot shows shortness here only as a *void*, and only at `4♠`
// (spades 0–0, 2.1%), `5♥` (hearts 0–0, 1.6%) and `5♦` (diamonds 0–0, 0.4%);
// its `4♦`/`4♥` are control cues (1–4 / 1–3 cards in the bid suit) and `4NT` is
// keycard.  The Pass/`3NT` split below sits on EPBot's exact boundary and covers
// the node's two biggest buckets (47.7% at 0–7 HCP, 19.8% at 8–15); the void
// shows, the cues, `4NT` and the `5♣` signoff (6.6%) are unmodelled.  The
// splinter rungs this replaces were inherited from Puppet's two-way `2♠` on no
// evidence — the same copy-paste `420d891` removed from the diamond lane.  See
// docs/ai-bidder/bba-1nt-minors.md.
fn european_two_spade_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 90, hcp(8..))
        .rule(Call::Pass, 0, hcp(..8))
}

/// Opener's reply to the European 2NT invite: `3NT` with a maximum, else pass
///
/// The 2NT bidder is a balanced eight; opposite a 17 (`25` combined) opener accepts
/// game, otherwise passes and plays 2NT — reproducing the natural-2NT outcome.
fn european_two_nt_answer(agreements: &Agreements) -> Rules {
    let floor = agreements.notrump.size_ask_accept_floor;
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(floor..))
        .rule(Call::Pass, 0, hcp(..floor))
}

/// Opener completes the European diamond transfer: `3♦`
fn european_three_club_answer(agreements: &Agreements) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 0, hcp(0..))
        .alert_if(agreements.decision.reading.completion_alerts, COMPLETION)
}

/// European 1NT - 3♣ diamond transfer and responder's game decision
pub(crate) fn european_three_club() -> Package {
    Package {
        name: "european-three-club",
        gate: |agreements| european_scheme(agreements),
        entries: |agreements| {
            let mut entries = rows_of(
                Pattern::node("P* 1NT - 3♣ -"),
                european_three_club_answer(agreements),
            );
            // ponytail: no splinter arm here, and the Puppet lane's `3♥`/`3♠`
            // rungs would be the wrong ones anyway.  `--mode nt-3c-3d` (400k
            // hands, 10260 reaching the node) shows **no `3♥`/`3♠` bucket at
            // all**: EPBot shows shortness only as a *void*, and only at `4♠`
            // (spades 0–0, 2.3%), `5♥` (hearts 0–0, 1.6%) and `5♣` (clubs 0–0,
            // 0.4%) — its `4♥`/`4♣` are control cues (1–3 / 1–4 in the bid
            // suit).  The Pass/`3NT` split below covers the node's two biggest
            // buckets (46.4% at 0–7 HCP, 19.7% at 8–14); the void shows and the
            // `5♦` signoff (8.5%) are unmodelled.  See
            // docs/ai-bidder/bba-1nt-minors.md.
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 3♣ - 3♦ -"),
                // `None`: this arm is an *opponent model* (see the module doc), so our
                // `4m` slam try must not leak into it.
                diamond_transfer_game(8, false, None),
            ));
            entries
        },
    }
}

/// European balanced invitation through 1NT - 2NT
pub(crate) fn european_two_notrump() -> Package {
    Package {
        name: "european-two-notrump",
        gate: |agreements| european_scheme(agreements),
        entries: |agreements| {
            rows_of(
                Pattern::node("P* 1NT - 2NT -"),
                european_two_nt_answer(agreements),
            )
        },
    }
}

/// European 1NT - 2♠ club transfer and club-splinter continuations
pub(crate) fn european_two_spade() -> Package {
    Package {
        name: "european-two-spade",
        gate: |agreements| european_scheme(agreements),
        entries: |agreements| {
            let mut entries = rows_of(
                Pattern::node("P* 1NT - 2♠ -"),
                european_two_spade_answer(agreements),
            );
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♠ - 3♣ -"),
                european_two_spade_rebid(),
            ));
            entries
        },
    }
}
