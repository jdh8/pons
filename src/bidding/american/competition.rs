//! The competitive package over our openings
//!
//! This module builds the [`Competitive`] book that covers contested auctions
//! after our one-level openings: direct-seat responses to their overcall,
//! system-on over their double, support doubles and redoubles for minor
//! openings, and opener's answer to partner's negative double of a two-level
//! minor overcall.

use super::super::constraint::{
    Cons, Constraint, hcp, len, min_level_is, points, stopper_in, support, they_bid,
};
use super::super::context::Context;
use super::super::fallback::{Fallback, FirstIs, OvercallAtMost, ReplaceNext, guard};
use super::super::{Competitive, Rules};
use super::notrump::{smolen_at_three, smolen_completion};
use super::{call, fallback_all_seats};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};
use std::cell::Cell;
use std::sync::Arc;

/// Which Lebensohl package the competitive book carries over our overcalled
/// `1NT` (Section 5)
///
/// Terminology: *Rubensohl* proper makes `2NT` an artificial **club** transfer;
/// the transfer styles here keep the weak `2NT` **relay**, which makes them
/// *Transfer Lebensohl*.
///
/// - `Off` — no Lebensohl node; responder falls to the instinct floor.
/// - `Plain` — weak `2NT` relay / sign-off vs strong direct `3NT` / forcing
///   3-level; matches BBA's 21GF. The prior default (+0.26 IMPs/divergent vs the
///   floor, 200k boards).
/// - `Transfer` — **the default.** Larry Cohen's *Transfer Lebensohl*: 3-level
///   bids transfer up the line *through* the adverse suit, the cue is Stayman, and
///   a transfer to a suit above theirs is INV+ so opener is driven to game (the
///   anti-stranding fix for the earlier transfer-Lebensohl attempt that stranded
///   game hands in partscores). Over `(2♥)`/`(2♠)`/`(2♣)` that is the whole story;
///   it measures **+0.46/+1.24 IMPs/divergent (none/both) vs plain Lebensohl**
///   (`lebensohl-ab`, 200k boards each), and +0.35/+0.05 vs the bare floor. Over
///   `(2♦)` it additionally frees `3♣` for game-forcing Stayman (Smolen after
///   opener's `3♦` denial), reshuffles the 3-level transfers to direct Jacoby
///   (`3♦`→♥, `3♥`→♠, `3♠`→♣ — the `3♠`→♣ leg a forced game-force, its completion
///   `4♣`), and adds Leaping Michaels `4♦` (both majors) / `4♣` (clubs + a major).
///   That `(2♦)` Smolen package is worth **+0.020/+0.024 IMPs/board,
///   +2.286/+2.822 IMPs/divergent (none/both)** over Cohen-pure-over-`(2♦)`
///   (`lebensohl-ab`, 200k filtered each), and it also wins after a takeout double
///   of a weak `2♦` (**+0.014/+0.019 IMPs/board, +1.77/+2.52 IMPs/divergent**,
///   `sohl-after-double-ab`) — which is why the advancer carries it too.
///
/// (True Rubensohl — `2NT` an artificial **club** transfer, low transfers two-way —
/// was implemented and measured worse than `Transfer` (DD `−0.017/−0.046`,
/// perfect-defense `+0.001/−0.023 IMPs/board` none/both) and removed: its only edge
/// was DD-blind right-siding, and jdh8 prefers the Smolen+LM-over-minors /
/// Cohen-over-majors split that `Transfer` carries. See `docs/ai-bidder/21gf-ledger.md`.)
///
/// (An earlier standard-low-Stayman + Smolen hybrid over *both* `(2♦)` and `(2♥)`
/// — no Jacoby reshuffle, no Leaping Michaels — measured DD `−1.31/−1.76 IMPs/div`
/// and was reverted. The narrowed `(2♦)`-only package that `Transfer` now carries
/// *wins*: the Jacoby reshuffle plus Leaping Michaels add genuine fit-finding (5-3
/// major games through Stayman+Smolen, 5-5 major games through Leaping Michaels)
/// that the perfect-defense measure credits — unlike the reverted hybrid, whose
/// only gain was DD-blind right-siding. See `docs/ai-bidder/21gf-ledger.md`.)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LebensohlStyle {
    /// Responder falls to the instinct floor (no Lebensohl node)
    Off,
    /// Plain Lebensohl (weak relay vs forcing 3-level) — the prior default
    Plain,
    /// Transfer Lebensohl (Larry Cohen's `2NT`-relay transfers) — the default;
    /// over `(2♦)` it adds `3♣`-Stayman + Smolen, Jacoby transfers
    /// (`3♦`→♥/`3♥`→♠/`3♠`→♣), and Leaping Michaels `4♣`/`4♦`
    Transfer,
}

thread_local! {
    /// Which Lebensohl package the competitive book carries (Section 5).
    static LEBENSOHL_STYLE: Cell<LebensohlStyle> = const { Cell::new(LebensohlStyle::Transfer) };
}

/// Select the Lebensohl package for books built *after* this call (thread-local,
/// read once at book-construction time)
pub fn set_lebensohl_style(style: LebensohlStyle) {
    LEBENSOHL_STYLE.with(|cell| cell.set(style));
}

/// Enable plain Lebensohl (`true` → [`LebensohlStyle::Plain`]) or disable it
/// (`false` → [`LebensohlStyle::Off`])
///
/// Back-compat shim over [`set_lebensohl_style`]; for Transfer Lebensohl call
/// that directly with [`LebensohlStyle::Transfer`].
pub fn set_lebensohl(on: bool) {
    set_lebensohl_style(if on {
        LebensohlStyle::Plain
    } else {
        LebensohlStyle::Off
    });
}

/// The currently selected Lebensohl package
fn lebensohl_style() -> LebensohlStyle {
    LEBENSOHL_STYLE.with(Cell::get)
}

thread_local! {
    /// Whether the Transfer-Lebensohl cue is split by stopper (see
    /// [`set_delayed_cue`]).
    static DELAYED_CUE: Cell<bool> = const { Cell::new(false) };
}

/// Enable the stopper-split cue for books built *after* this call (thread-local,
/// read once at book-construction time)
///
/// Larry Cohen's fast-denies / slow-shows, adapted to our Transfer Lebensohl:
/// the *direct* cue of their suit denies a stopper, while a *delayed* cue (relay
/// through `2NT`, then their suit) is Stayman *with* a stopper. It also denies a
/// 5-card unbid major (Smolen / Leaping Michaels handle those). Only the
/// single-unbid-major contexts — over `(2♥)` and `(2♠)` — are affected. Off by
/// default; gated behind this toggle for A/B measurement.
pub fn set_delayed_cue(on: bool) {
    DELAYED_CUE.with(|cell| cell.set(on));
}

/// Whether the stopper-split cue is enabled
pub(super) fn delayed_cue() -> bool {
    DELAYED_CUE.with(Cell::get)
}

/// The single unbid major when `over` is itself a major (the other major)
///
/// `None` when `over` is a minor (then both majors are unbid) — the stopper-split
/// cue is only authored for the single-unbid-major contexts.
pub(super) fn unbid_major(over: Suit) -> Option<Suit> {
    match over {
        Suit::Hearts => Some(Suit::Spades),
        Suit::Spades => Some(Suit::Hearts),
        _ => None,
    }
}

/// The 2NT-relay shape over their `over` overcall: a 5+ suit (not their suit)
/// with 6+ HCP.
///
/// The 6-HCP floor is PD-distilled. A perfect-defense gate (relay only when
/// sampled double-dummy says our 3-level line out-scores defending) declines
/// nearly every sub-6 hand — pushing a near-bust to the 3 level loses on DD,
/// even with a 6-card suit — and this plain HCP floor recovers ~60–80% of that
/// gate's IMPs/board gain over relaying every 5-card suit (A/B, lebensohl-ab,
/// `--pd-relay`). Adverse-suit length/honors were *not* predictive; overall
/// weakness is the driver.
fn lebensohl_relay_shape(over: Suit) -> Cons<impl Constraint + Clone> {
    let five = |s: Suit| len(s, 5..);
    let any5 = match over {
        Suit::Clubs => five(Suit::Diamonds) | five(Suit::Hearts) | five(Suit::Spades),
        Suit::Diamonds => five(Suit::Clubs) | five(Suit::Hearts) | five(Suit::Spades),
        Suit::Hearts => five(Suit::Clubs) | five(Suit::Diamonds) | five(Suit::Spades),
        Suit::Spades => five(Suit::Clubs) | five(Suit::Diamonds) | five(Suit::Hearts),
    };
    any5 & hcp(6..)
}

// ---------------------------------------------------------------------------
// Section 1: direct-seat response to their overcall
// ---------------------------------------------------------------------------

/// Responder's action after our opening `o` and their overcall (≤ 2♠)
///
/// Covers cue-bid limit-plus raises, preemptive and competitive raises of
/// the opening suit, negative doubles, and weak jump shifts.
fn over_their_overcall(opening: Suit) -> Rules {
    let o = opening;
    let o_strain = Strain::from(o);

    let is_major = matches!(o, Suit::Hearts | Suit::Spades);
    let raise_min: usize = if is_major { 3 } else { 5 };
    let jump_min: usize = if is_major { 4 } else { 5 };

    let other_major = match o {
        Suit::Hearts => Suit::Spades,
        Suit::Spades => Suit::Hearts,
        _ => Suit::Hearts, // for minors, used only in negative double
    };

    let mut rules = Rules::new();

    // Cue-bid raises: for each suit t ≠ o, levels 2 and 3
    for t in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if t == o {
            continue;
        }
        let t_strain = Strain::from(t);
        for lvl in 2u8..=3 {
            rules = rules.rule(
                Bid::new(lvl, t_strain),
                2.0,
                they_bid(t_strain)
                    & min_level_is(lvl, t_strain)
                    & support(raise_min..)
                    & points(10..),
            );
        }
    }

    // Jump raise: preemptive (min_level=2 means we could bid 2o, so 3o is a jump)
    rules = rules.rule(
        Bid::new(3, o_strain),
        1.6,
        min_level_is(2, o_strain) & support(jump_min..) & points(..=9),
    );

    // Competitive raise: 3o when it's the minimum legal bid
    rules = rules.rule(
        Bid::new(3, o_strain),
        1.3,
        min_level_is(3, o_strain) & support(raise_min..) & points(6..=9),
    );

    // Single raise
    rules = rules.rule(
        Bid::new(2, o_strain),
        1.5,
        min_level_is(2, o_strain) & support(raise_min..) & points(6..=9),
    );

    // Negative double
    rules = if is_major {
        // Other major, 4+ cards, 8+ HCP
        rules.rule(Call::Double, 1.0, len(other_major, 4..) & hcp(8..))
    } else {
        // Both majors 4+, 8+ HCP
        rules.rule(
            Call::Double,
            1.0,
            len(Suit::Hearts, 4..) & len(Suit::Spades, 4..) & hcp(8..),
        )
    };

    // Weak jump shifts: for each suit x ≠ o, levels 2 and 3
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x == o {
            continue;
        }
        let x_strain = Strain::from(x);
        for lvl in 2u8..=3 {
            rules = rules.rule(
                Bid::new(lvl, x_strain),
                1.1,
                min_level_is(lvl - 1, x_strain) & len(x, 6..) & points(2..=5) & !they_bid(x_strain),
            );
        }
    }

    // Pass
    rules.rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Section 3: support doubles and redoubles
// ---------------------------------------------------------------------------

/// Opener's support double/redouble rules showing three-card support for major M
///
/// `Call::Double` with exactly 3 (support double); `2M` with 4+ (natural raise);
/// Pass as the catch-all.
fn support_rules(major: Suit) -> Rules {
    let m = Strain::from(major);
    Rules::new()
        .rule(Call::Double, 1.5, support(3..=3))
        .rule(Bid::new(2, m), 1.4, support(4..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Section 4: opener answers partner's negative double of a two-level minor
// ---------------------------------------------------------------------------

/// Opener's answer after `1M – (2m) – X – P` (partner doubled a minor overcall)
///
/// Shows four-card length in the other major or rebids the opening major on five.
/// No Pass rule — the double is forcing.
fn answer_neg_double_of_minor(opening_major: Suit) -> Rules {
    let m = Strain::from(opening_major);
    let other = if opening_major == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    let other_strain = Strain::from(other);
    Rules::new()
        .rule(Bid::new(2, other_strain), 1.0, len(other, 3..))
        .rule(Bid::new(2, m), 0.5, len(opening_major, 5..))
}

// ---------------------------------------------------------------------------
// Section 5: Lebensohl after our 1NT is overcalled
// ---------------------------------------------------------------------------

/// Responder's plain-Lebensohl actions after our `1NT` and a natural 2-level
/// overcall in `over`
///
/// A book node here *shadows* the instinct floor, so this table covers every
/// responder hand. The Lebensohl idea separates weak from strong: weak hands
/// relay through `2NT` to a `3♣` sign-off (or correct to a long suit), while game
/// hands bid a forcing 3-level suit or a to-play `3NT` directly — so a game is
/// never stranded in a partscore (the failure mode of the Rubensohl v1 attempt).
//
// ponytail: the cue (Stayman / stopper-ask, "slow shows / fast denies") is
// skipped — 4-4-major game hands bid 3NT. Author the cue + opener's reply if the
// A/B shows it matters.
//
// The Section-5 builders below are pure functions of `(over, hand)` — the auction
// prefix and the bidder's identity never enter — so `american/defense.rs` reuses
// them verbatim for "sohl after a takeout double" (advancing partner's takeout
// double of a weak two), where the opponents' suit is likewise at the two level.
pub(super) fn lebensohl_responder(over: Suit) -> Rules {
    let mut rules = Rules::new();

    // Forcing 3-level new suit: game-forcing, 5+ cards. A jump (when the 2-level
    // was available) or the cheapest 3-level bid (suit at/below the overcall) —
    // either way 3-of-a-suit over the interference is forcing. (All 3-level bids
    // clear a 2-level overcall, so no min-level gate is needed.)
    for s in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if s == over {
            continue;
        }
        let strain = Strain::from(s);
        rules = rules.rule(Bid::new(3, strain), 1.8, len(s, 5..) & points(10..));
    }

    // Direct cue of their suit = Stayman: game-forcing with a 4-card unbid major
    // (no 5-card suit to bid forcingly — those use the 3-level above). Answered by
    // [`cue_stayman_answer`]. Stopper-agnostic, mirroring Transfer's default cue,
    // so a 4-4 major fit is found even with a stopper. Weight sits between the
    // natural forcing 3-level (1.8) and direct 3NT (1.7): a known 5-card suit is
    // bid naturally, a bare 4-card major cues, else 3NT.
    let cue = Bid::new(3, Strain::from(over));
    rules = match unbid_major(over) {
        Some(major) => rules.rule(cue, 1.75, len(major, 4..) & points(10..)),
        None => rules.rule(
            cue,
            1.75,
            (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..)) & points(10..),
        ),
    };

    // Direct 3NT to play: game values with their suit stopped.
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        1.7,
        points(10..) & stopper_in(over),
    );

    // Penalty double of their overcall.
    rules = rules.rule(Call::Double, 1.55, len(over, 4..) & hcp(9..));

    // Natural new suit at the 2 level (above the overcall, below 2NT): weak,
    // competitive, to play.
    for s in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if s == over {
            continue;
        }
        let strain = Strain::from(s);
        rules = rules.rule(
            Bid::new(2, strain),
            1.5,
            min_level_is(2, strain) & len(s, 5..) & points(..=9),
        );
    }

    // 2NT = Lebensohl relay to 3♣: a weak hand with a long suit not biddable
    // naturally at the 2 level (long clubs, or a suit below the overcall) — sign
    // off in 3♣ or correct (see [`lebensohl_relay_rebid`]). The natural 2-level
    // outranks this relay, so above-the-overcall suits are still bid naturally;
    // balanced weak hands pass. See [`lebensohl_relay_shape`] for the 6+/good-5
    // shape and the PD-distilled 6-HCP floor on the 5-card arm.
    let long_suit = lebensohl_relay_shape(over);
    rules = rules.rule(Bid::new(2, Strain::Notrump), 1.4, points(..=9) & long_suit);

    // Pass — weak, nothing constructive to say.
    rules.rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener completes responder's Lebensohl `2NT` relay with the forced `3♣`
pub(super) fn complete_lebensohl_relay() -> Rules {
    Rules::new().rule(Bid::new(3, Strain::Clubs), 1.0, hcp(0..))
}

/// Responder's rebid after the `2NT` relay is completed at `3♣`
///
/// Pass to play clubs, or correct to the six-card suit (still a weak sign-off).
pub(super) fn lebensohl_relay_rebid(over: Suit) -> Rules {
    let mut rules = Rules::new();
    for s in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if s == over {
            continue;
        }
        let strain = Strain::from(s);
        rules = rules.rule(
            Bid::new(3, strain),
            1.0,
            min_level_is(3, strain) & len(s, 5..),
        );
    }
    // Stopper-split on: the *delayed* cue of their suit — Stayman with a stopper,
    // game-forcing, exactly a 4-card unbid major (denies 5). Answered by
    // [`cue_stayman_answer`] (the stopper is guaranteed, so 3NT is safe).
    if let (true, Some(major)) = (delayed_cue(), unbid_major(over)) {
        rules = rules.rule(
            Bid::new(3, Strain::from(over)),
            1.5,
            points(10..) & stopper_in(over) & len(major, 4..) & len(major, ..5),
        );
    }
    rules.rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Section 5b: Transfer Lebensohl (Rubensohl) — Larry Cohen's version
// ---------------------------------------------------------------------------

/// The suit a 3-level Transfer-Lebensohl bid in `bid_suit` shows, given the
/// opponents' 2-level overcall in `over`
///
/// The cheapest suit strictly above `bid_suit` that is *not* their suit — a
/// transfer *through* the adverse suit. `None` when `bid_suit` is their suit
/// (that bid is the Stayman cue, not a transfer) or no higher suit remains
/// (the lowest target, clubs, has no dedicated transfer — those rare hands use
/// the `2NT` relay or `3NT`).
pub(super) fn transfer_target(bid_suit: Suit, over: Suit) -> Option<Suit> {
    if bid_suit == over {
        return None; // the cue = Stayman, not a transfer
    }
    [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
        .into_iter()
        .find(|&s| (s as u8) > (bid_suit as u8) && s != over)
}

/// Responder's Transfer-Lebensohl actions after our `1NT` and a natural 2-level
/// overcall in `over`
///
/// Weak hands keep the plain-Lebensohl outlets (natural 2-level, `2NT` relay to
/// `3♣`, penalty double). Invitational-or-better hands transfer at the 3 level:
/// each non-cue suit bid transfers to the next suit up *through* the adverse
/// suit, and the cue (their suit) is Stayman. Because a weak hand always has a
/// natural 2-level call, a 3-level transfer to a suit above theirs is INV+ — so
/// opener is driven to game (see [`transfer_completion`]) and a game is never
/// stranded in a partscore (the Rubensohl-v1 failure).
pub(super) fn transfer_lebensohl_responder(over: Suit) -> Rules {
    let mut rules = Rules::new();

    // 3-level transfers (INV+, 5+ in the target) and the cue (Stayman, GF).
    for bid_suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(bid_suit);
        if bid_suit == over {
            // Cue = Stayman: game values with a 4-card unbid major. (The arms
            // differ in constraint type, so each returns the updated `Rules`.)
            // With the stopper-split on, the *direct* cue denies a stopper —
            // stopper hands relay through 2NT to the delayed cue (the broadened
            // 2NT below + [`lebensohl_relay_rebid`]).
            let cue = Bid::new(3, strain);
            let split = delayed_cue() && unbid_major(over).is_some();
            rules = match (over, split) {
                (Suit::Hearts, true) => rules.rule(
                    cue,
                    1.7,
                    len(Suit::Spades, 4..) & points(10..) & !stopper_in(over),
                ),
                (Suit::Spades, true) => rules.rule(
                    cue,
                    1.7,
                    len(Suit::Hearts, 4..) & points(10..) & !stopper_in(over),
                ),
                (Suit::Hearts, false) => {
                    rules.rule(cue, 1.7, len(Suit::Spades, 4..) & points(10..))
                }
                (Suit::Spades, false) => {
                    rules.rule(cue, 1.7, len(Suit::Hearts, 4..) & points(10..))
                }
                _ => rules.rule(
                    cue,
                    1.7,
                    (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..)) & points(10..),
                ),
            };
        } else if let Some(target) = transfer_target(bid_suit, over) {
            // Transfer: show 5+ in the target, invitational or better. A major
            // target outranks the cue so a 5-card major is shown by the
            // transfer, not Stayman; a minor target is rare (long minor, no
            // stopper) and yields to Stayman / 3NT.
            let weight = if matches!(target, Suit::Hearts | Suit::Spades) {
                1.8
            } else {
                1.45
            };
            rules = rules.rule(Bid::new(3, strain), weight, len(target, 5..) & points(9..));
        } else if over != Suit::Clubs {
            // Top step (no suit above to transfer into): a *forced* game-force
            // transfer to clubs, 6+♣. Its completion lands at game, so 3♣ can
            // never be the contract — the only forcing long-club route (the
            // 2NT→3♣ relay is the *weak* one). Weight below 3NT's 1.5 so a 6♣
            // hand *with* a stopper picks 3NT; only no-stopper hands transfer.
            // (Over (2♣) clubs is their suit — there is no top-step transfer.)
            rules = rules.rule(
                Bid::new(3, strain),
                1.45,
                len(Suit::Clubs, 6..) & points(10..),
            );
        }
    }

    // Direct 3NT to play: game values with their suit stopped, no major to show.
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        1.5,
        points(10..) & stopper_in(over),
    );

    // Stopper-split on: a GF hand with a stopper *and* exactly a 4-card unbid
    // major relays through 2NT to bid the cue *slowly* (Stayman with a stopper,
    // see [`lebensohl_relay_rebid`]) — outweighing direct 3NT (1.5) so the 4-4
    // major fit is still found. Denies a 5-card major (Smolen / Leaping Michaels).
    if let (true, Some(major)) = (delayed_cue(), unbid_major(over)) {
        rules = rules.rule(
            Bid::new(2, Strain::Notrump),
            1.6,
            points(10..) & stopper_in(over) & len(major, 4..) & len(major, ..5),
        );
    }

    // Penalty double of their overcall (kept — the Rubensohl-v1 attempt lost the
    // floor's penalty doubles by shadowing them with no double of its own).
    rules = rules.rule(Call::Double, 1.55, len(over, 4..) & hcp(9..));

    // Natural new suit at the 2 level (above the overcall, below 2NT): weak.
    for s in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if s == over {
            continue;
        }
        let strain = Strain::from(s);
        rules = rules.rule(
            Bid::new(2, strain),
            1.4,
            min_level_is(2, strain) & len(s, 5..) & points(..=8),
        );
    }

    // 2NT = Lebensohl relay to 3♣: a weak long-suit hand (sign off or correct),
    // same shape as plain Lebensohl (see [`lebensohl_relay_shape`] — 6+ suit, or
    // a 5-carder with the PD-distilled 6-HCP floor, never their suit).
    let long_suit = lebensohl_relay_shape(over);
    rules = rules.rule(Bid::new(2, Strain::Notrump), 1.35, points(..=8) & long_suit);

    // Pass — weak, nothing constructive to say.
    rules.rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's reply after responder's Transfer-Lebensohl transfer to `target`
///
/// A transfer to a major is INV+, so opener is driven to **game**: `4M` with a
/// fit, else `3NT`. A transfer to a minor (rare — long minor, no stopper) is
/// completed at the 3 level, or `3NT` with a stopper; responder drives on.
pub(super) fn transfer_completion(target: Suit, over: Suit) -> Rules {
    let t = Strain::from(target);
    let mut rules = Rules::new();
    if matches!(target, Suit::Hearts | Suit::Spades) {
        rules = rules.rule(Bid::new(4, t), 1.6, len(target, 3..)).rule(
            Bid::new(3, Strain::Notrump),
            1.4,
            len(target, ..3),
        );
    } else {
        // ponytail: minor-target 5m / slam exploration is left to the floor;
        // 3NT-or-complete covers the common game. Author it if the A/B shows
        // minor transfers matter.
        rules = rules
            .rule(Bid::new(3, Strain::Notrump), 1.5, stopper_in(over))
            .rule(Bid::new(3, t), 1.3, len(target, 3..));
    }
    rules.rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's reply to responder's Transfer-Lebensohl cue (Stayman, game-forcing)
///
/// Shows a 4-card unbid major at its cheapest legal level, else `3NT`.
pub(super) fn cue_stayman_answer(over: Suit) -> Rules {
    let mut rules = Rules::new();
    for major in [Suit::Hearts, Suit::Spades] {
        if major == over {
            continue;
        }
        let m = Strain::from(major);
        rules = rules
            .rule(Bid::new(3, m), 1.6, len(major, 4..) & min_level_is(3, m))
            .rule(Bid::new(4, m), 1.5, len(major, 4..) & min_level_is(4, m));
    }
    // No 4-card unbid major → 3NT (always legal above the 3-level cue).
    rules.rule(Bid::new(3, Strain::Notrump), 1.3, hcp(0..))
}

/// Answerer's reply to the *direct* (no-stopper) cue under the stopper-split
///
/// The cuer denied a stopper in their suit, so 3NT needs *our* own stopper;
/// without it (and without a 4-card-major fit) we run to a minor-suit game
/// rather than a stopperless 3NT. A 4-card unbid major is shown first (the fit).
/// The trailing low-weight 3NT is a guaranteed-finite catch-all (it never wins
/// against the minors, but keeps the node from silently passing the game force).
pub(super) fn cue_stayman_answer_no_stopper(over: Suit) -> Rules {
    let mut rules = Rules::new();
    for major in [Suit::Hearts, Suit::Spades] {
        if major == over {
            continue;
        }
        let m = Strain::from(major);
        rules = rules
            .rule(Bid::new(3, m), 1.6, len(major, 4..) & min_level_is(3, m))
            .rule(Bid::new(4, m), 1.5, len(major, 4..) & min_level_is(4, m));
    }
    // 3NT only with our own stopper (the cuer has none).
    rules = rules.rule(Bid::new(3, Strain::Notrump), 1.45, stopper_in(over));
    // No fit, no stopper → minor-suit game.
    for minor in [Suit::Clubs, Suit::Diamonds] {
        let m = Strain::from(minor);
        rules = rules.rule(Bid::new(4, m), 1.2, len(minor, 4..) & min_level_is(4, m));
    }
    // Guaranteed-finite catch-all (rare: no major, no stopper, no 4-card minor).
    rules.rule(Bid::new(3, Strain::Notrump), 1.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Section 5c: Transfer over (2♦) — 3♣-Stayman + Smolen, Jacoby transfers
// (3♦→♥, 3♥→♠, 3♠→♣), and Leaping Michaels 4♣/4♦
// ---------------------------------------------------------------------------

/// Responder's action after our `1NT` and a `(2♦)` overcall, the `(2♦)`-only
/// Smolen leg of the [`LebensohlStyle::Transfer`] package
///
/// `2♦` leaves `3♣` free below the cue, so Stayman moves there (with Smolen after
/// opener's `3♦` denial) and the transfers shift down to direct Jacoby: `3♦`→♥,
/// `3♥`→♠, `3♠`→♣. The major transfers are INV+ and auto-driven to game by
/// [`transfer_completion`]; the `3♠`→♣ leg is a *forced* game-force (its completion
/// is `4♣`, so `3♣` is unplayable). Leaping Michaels `4♦` (both majors) and `4♣`
/// (clubs + a major) show 5-5 game-forcing two-suiters — partner opened `1NT`, so
/// `points(10..)` (≈ 8 HCP after the 5-5 upgrade) already forces game. The weak
/// outlets (natural 2-level, `2NT` relay, penalty double, direct `3NT`) match
/// `Transfer` so the A/B isolates the constructive change.
pub(super) fn transfer_stayman_2d_responder() -> Rules {
    let mut rules = Rules::new();

    // 3♣ = Stayman: game-forcing with *exactly* a 4-card major. A single 5-card
    // major transfers instead; a 5-4 GF hand has its 4-card major here and so comes
    // to Stayman (for Smolen) — hence weight above the transfers, which it also fits.
    rules = rules.rule(
        Bid::new(3, Strain::Clubs),
        1.85,
        (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4)) & points(10..),
    );

    // Direct Jacoby transfers above their suit (INV+, auto-driven to game).
    rules = rules
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.8,
            len(Suit::Hearts, 5..) & points(9..),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            1.8,
            len(Suit::Spades, 5..) & points(9..),
        );

    // 3♠→clubs: a *forced* game-force with 6+ clubs (its completion is 4♣, so 3♣
    // can never be the contract). Weight below 3NT's, so a 6-club hand *with* a
    // diamond stopper picks 3NT; only the no-stopper hands transfer.
    rules = rules.rule(
        Bid::new(3, Strain::Spades),
        1.45,
        len(Suit::Clubs, 6..) & points(10..),
    );

    // Leaping Michaels: 5-5 game-forcing two-suiters.
    rules = rules
        .rule(
            Bid::new(4, Strain::Diamonds),
            2.0,
            len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & points(10..),
        )
        .rule(
            Bid::new(4, Strain::Clubs),
            2.0,
            len(Suit::Clubs, 5..)
                & (len(Suit::Hearts, 5..) | len(Suit::Spades, 5..))
                & points(10..),
        );

    // Weak / to-play outlets — identical to `transfer_lebensohl_responder(Diamonds)`.
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        1.5,
        points(10..) & stopper_in(Suit::Diamonds),
    );
    rules = rules.rule(Call::Double, 1.55, len(Suit::Diamonds, 4..) & hcp(9..));
    for s in [Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(s);
        rules = rules.rule(
            Bid::new(2, strain),
            1.4,
            min_level_is(2, strain) & len(s, 5..) & points(..=8),
        );
    }
    // Relay shape: 6+ suit, or a 5-carder with the PD-distilled 6-HCP floor,
    // never their diamonds (see [`lebensohl_relay_shape`]).
    let long_suit = lebensohl_relay_shape(Suit::Diamonds);
    rules = rules.rule(Bid::new(2, Strain::Notrump), 1.35, points(..=8) & long_suit);

    rules.rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's answer to `3♣` Stayman over `(2♦)`: a 4-card major, else `3♦`
///
/// `3♥`/`3♠` shows a 4-card major (hearts first when both); `3♦` denies one,
/// leaving `3♥`/`3♠` free for responder's Smolen. `3♦` is the finite catch-all.
pub(super) fn stayman_2d_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 1.6, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(3, Strain::Spades),
            1.55,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(Bid::new(3, Strain::Diamonds), 0.5, hcp(0..))
}

/// Responder's rebid after opener shows a 4-card major over `3♣` Stayman
///
/// Game-forcing already: raise the shown major to game with 4-card support (an
/// eight-card fit), else settle in `3NT` (the finite catch-all).
pub(super) fn stayman_2d_fit_rebid(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 1.4, len(major, 4..))
        .rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..))
}

/// Opener's completion of the top-step→clubs transfer (a forced game-force)
///
/// Responder has 6+ clubs, no stopper in `over`, game values. Opener bids `3NT`
/// with a stopper of its own, else raises to `5♣` — `3♣` is unplayable below the
/// top step, so the auction must reach game. (`5♣` is the finite catch-all.)
//
// ponytail: minor-suit slam exploration is left to the floor; 3NT-or-5♣ covers
// the common game. Author a keycard ladder here only if the A/B shows it matters.
pub(super) fn clubs_transfer_completion(over: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.4, stopper_in(over))
        .rule(Bid::new(5, Strain::Clubs), 0.5, hcp(0..))
}

/// Opener's reply to Leaping Michaels `4♦` (both majors, 5-5 game-forcing)
///
/// Bid game in the better major fit, preferring the nine-card fit (4-card
/// support) and breaking ties toward spades. `4♥` is the finite catch-all.
pub(super) fn lm_2d_both_majors_advance() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Spades), 1.6, len(Suit::Spades, 4..))
        .rule(Bid::new(4, Strain::Hearts), 1.55, len(Suit::Hearts, 4..))
        .rule(Bid::new(4, Strain::Spades), 1.5, len(Suit::Spades, 3..))
        .rule(Bid::new(4, Strain::Hearts), 1.0, hcp(0..))
}

/// Opener's reply to Leaping Michaels `4♣` (clubs + an unknown 5+ major)
///
/// `4♦` asks which major; responder names it in [`lm_2d_clubs_major`].
//
// ponytail: opener always relays — the major usually outplays 5♣, and opener's
// final placement (pass the major / correct to 5♣) is left to the floor. Add a
// direct 5♣ sign-off only if the A/B shows the relay costs.
pub(super) fn lm_2d_clubs_ask() -> Rules {
    Rules::new().rule(Bid::new(4, Strain::Diamonds), 1.4, hcp(0..))
}

/// Responder names the 5+ major behind a `4♣` Leaping Michaels, over the `4♦` ask
pub(super) fn lm_2d_clubs_major() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Hearts), 1.5, len(Suit::Hearts, 5..))
        .rule(Bid::new(4, Strain::Spades), 1.5, len(Suit::Spades, 5..))
        .rule(Bid::new(5, Strain::Clubs), 0.5, hcp(0..))
}

/// The competitive package over our openings: cue-bid raises, preemptive raises,
/// negative doubles for all four openings, support doubles/redoubles, and
/// opener's answers to negative doubles of minor overcalls
///
/// Standalone, the system-on rebase has nothing to land on; bind through
/// [`Pair::against`][crate::bidding::Pair::against] (as [`american`][super::american] is meant to be
/// used) so it resolves into the uncontested core.
#[must_use]
pub fn competition() -> Competitive {
    let mut book = Competitive::new();

    // Section 1 & 2: over all four openings, attach direct-seat response rules
    // and system-on over their double.
    for opening in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let opening_call = call(1, Strain::from(opening));
        fallback_all_seats(
            &mut book,
            &[opening_call],
            3,
            Arc::new(OvercallAtMost(Bid::new(2, Strain::Spades))),
            Fallback::classify(over_their_overcall(opening)),
        );
        fallback_all_seats(
            &mut book,
            &[opening_call],
            3,
            Arc::new(FirstIs(Call::Double)),
            Fallback::rebase(ReplaceNext(Call::Pass)),
        );
    }

    // Section 3: support doubles and redoubles for each (minor, major) pair.
    for minor in [Suit::Clubs, Suit::Diamonds] {
        for major in [Suit::Hearts, Suit::Spades] {
            let suffix = [
                call(1, Strain::from(minor)),
                Call::Pass,
                call(1, Strain::from(major)),
            ];
            let just_below = if major == Suit::Hearts {
                Bid::new(2, Strain::Diamonds)
            } else {
                Bid::new(2, Strain::Hearts)
            };

            // Support double: they overcall at most `just_below`
            fallback_all_seats(
                &mut book,
                &suffix,
                3,
                Arc::new(OvercallAtMost(just_below)),
                Fallback::classify(support_rules(major)),
            );

            // Support redouble: they doubled
            fallback_all_seats(
                &mut book,
                &suffix,
                3,
                Arc::new(guard(|_: &Context<'_>, suffix: &[Call]| {
                    suffix == [Call::Double]
                })),
                Fallback::classify({
                    let m = Strain::from(major);
                    Rules::new()
                        .rule(Call::Redouble, 1.5, support(3..=3))
                        .rule(Bid::new(2, m), 1.4, support(4..))
                        .rule(Call::Pass, 0.0, hcp(0..))
                }),
            );
        }
    }

    // Section 4: opener answers partner's negative double of a two-level minor.
    // Suffix is [1M]; guard checks that suffix is [2m, X, P].
    for major in [Suit::Hearts, Suit::Spades] {
        fallback_all_seats(
            &mut book,
            &[call(1, Strain::from(major))],
            3,
            Arc::new(guard(|_: &Context<'_>, suffix: &[Call]| {
                matches!(
                    suffix,
                    [Call::Bid(b), Call::Double, Call::Pass]
                        if b.level.get() == 2
                            && (b.strain == Strain::Clubs || b.strain == Strain::Diamonds)
                )
            })),
            Fallback::classify(answer_neg_double_of_minor(major)),
        );
    }

    // Section 5: Lebensohl after our 1NT is overcalled at the 2 level. Purely
    // additive — nothing else lands at [1NT] in the competitive book. Plain or
    // Transfer Lebensohl per [`LebensohlStyle`]; both keep the weak 2NT relay.
    let style = lebensohl_style();
    if style != LebensohlStyle::Off {
        let one_nt = call(1, Strain::Notrump);
        let two_nt = call(2, Strain::Notrump);
        let three_clubs = call(3, Strain::Clubs);
        for over in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let overcall = call(2, Strain::from(over));

            // Responder's first action: the uncovered suffix is exactly their overcall.
            let responder = match style {
                LebensohlStyle::Transfer if over == Suit::Diamonds => {
                    transfer_stayman_2d_responder()
                }
                LebensohlStyle::Transfer => transfer_lebensohl_responder(over),
                _ => lebensohl_responder(over),
            };
            fallback_all_seats(
                &mut book,
                &[one_nt],
                3,
                Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                    suffix == [overcall]
                })),
                Fallback::classify(responder),
            );

            // Opener completes the 2NT relay with 3♣: suffix is [overcall, 2NT, P].
            fallback_all_seats(
                &mut book,
                &[one_nt],
                3,
                Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                    suffix == [overcall, two_nt, Call::Pass]
                })),
                Fallback::classify(complete_lebensohl_relay()),
            );

            // Responder's rebid after 3♣ (the weak relay sign-off): suffix is
            // [overcall, 2NT, P, 3♣, P].
            fallback_all_seats(
                &mut book,
                &[one_nt],
                3,
                Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                    suffix == [overcall, two_nt, Call::Pass, three_clubs, Call::Pass]
                })),
                Fallback::classify(lebensohl_relay_rebid(over)),
            );

            // Plain style: opener's reply to the direct cue (Stayman). Suffix is
            // [overcall, 3X, P] where 3X is the cue of their suit. (Transfer wires
            // its cue reply in the block below.)
            if style == LebensohlStyle::Plain {
                let cue = call(3, Strain::from(over));
                fallback_all_seats(
                    &mut book,
                    &[one_nt],
                    3,
                    Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                        suffix == [overcall, cue, Call::Pass]
                    })),
                    Fallback::classify(cue_stayman_answer(over)),
                );
            }

            // Transfer style: opener's reply to each 3-level transfer / cue.
            // Suffix is [overcall, 3X, P] where 3X is responder's transfer or cue.
            // Over (2♦) the Smolen block below owns the 3-level replies, so this
            // covers (2♥)/(2♠)/(2♣) only.
            if style == LebensohlStyle::Transfer && over != Suit::Diamonds {
                for bid_suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                    let resp = call(3, Strain::from(bid_suit));
                    let reply = if bid_suit == over {
                        cue_stayman_answer(over)
                    } else if let Some(target) = transfer_target(bid_suit, over) {
                        transfer_completion(target, over)
                    } else if over != Suit::Clubs {
                        clubs_transfer_completion(over) // top step → clubs (forced GF)
                    } else {
                        continue; // over (2♣): clubs is their suit — floored
                    };
                    fallback_all_seats(
                        &mut book,
                        &[one_nt],
                        3,
                        Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                            suffix == [overcall, resp, Call::Pass]
                        })),
                        Fallback::classify(reply),
                    );
                }
            }

            // Recognize a delayed cue (2NT relay, then their suit) over (2♥)/(2♠):
            // Stayman with a stopper, answered like the direct cue but with 3NT
            // safe. Always wired so a human partner who plays it gets a sensible
            // reply, even though the bot only *bids* it under `set_delayed_cue`.
            if style == LebensohlStyle::Transfer && unbid_major(over).is_some() {
                let cue = call(3, Strain::from(over));
                fallback_all_seats(
                    &mut book,
                    &[one_nt],
                    3,
                    Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                        suffix
                            == [
                                overcall,
                                two_nt,
                                Call::Pass,
                                three_clubs,
                                Call::Pass,
                                cue,
                                Call::Pass,
                            ]
                    })),
                    Fallback::classify(cue_stayman_answer(over)),
                );
            }

            // Section 5c: Transfer over (2♦) — 3♣-Stayman + Smolen, the Jacoby
            // transfers (3♦→♥, 3♥→♠, 3♠→♣), and Leaping Michaels 4♣/4♦.
            // (The 2♥/2♠/2♣ branches reuse the Transfer completions above.)
            if style == LebensohlStyle::Transfer && over == Suit::Diamonds {
                let p = Call::Pass;
                let c3 = call(3, Strain::Clubs);
                let d3 = call(3, Strain::Diamonds);
                let h3 = call(3, Strain::Hearts);
                let s3 = call(3, Strain::Spades);
                let c4 = call(4, Strain::Clubs);
                let d4 = call(4, Strain::Diamonds);
                let nodes: Vec<(Vec<Call>, Rules)> = vec![
                    // 3♣ Stayman, opener's answer; then Smolen after the 3♦ denial.
                    (vec![overcall, c3, p], stayman_2d_answer()),
                    (vec![overcall, c3, p, d3, p], smolen_at_three()),
                    (
                        vec![overcall, c3, p, d3, p, h3, p],
                        smolen_completion(Suit::Spades),
                    ),
                    (
                        vec![overcall, c3, p, d3, p, s3, p],
                        smolen_completion(Suit::Hearts),
                    ),
                    // Opener showed a 4-card major over Stayman; responder places.
                    (
                        vec![overcall, c3, p, h3, p],
                        stayman_2d_fit_rebid(Suit::Hearts),
                    ),
                    (
                        vec![overcall, c3, p, s3, p],
                        stayman_2d_fit_rebid(Suit::Spades),
                    ),
                    // Jacoby transfers: 3♦→♥, 3♥→♠ (auto-driven), 3♠→♣ (forced GF).
                    (
                        vec![overcall, d3, p],
                        transfer_completion(Suit::Hearts, over),
                    ),
                    (
                        vec![overcall, h3, p],
                        transfer_completion(Suit::Spades, over),
                    ),
                    (vec![overcall, s3, p], clubs_transfer_completion(over)),
                    // Leaping Michaels: 4♦ both majors, 4♣ clubs + a major (ask).
                    (vec![overcall, d4, p], lm_2d_both_majors_advance()),
                    (vec![overcall, c4, p], lm_2d_clubs_ask()),
                    (vec![overcall, c4, p, d4, p], lm_2d_clubs_major()),
                ];
                for (suffix, rules) in nodes {
                    fallback_all_seats(
                        &mut book,
                        &[one_nt],
                        3,
                        Arc::new(guard(move |_: &Context<'_>, s: &[Call]| {
                            s == suffix.as_slice()
                        })),
                        Fallback::classify(rules),
                    );
                }
            }
        }
    }

    book
}

#[cfg(test)]
mod tests {
    use crate::bidding::Family;
    use crate::bidding::american::american;
    use contract_bridge::auction::{Call, RelativeVulnerability};
    use contract_bridge::{Bid, Hand, Strain};

    const fn call(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid::new(level, strain))
    }

    /// `american()`'s best call for a hand in an auction, and whether the instinct
    /// floor (not a book node) produced it
    fn best_call(auction: &[Call], hand: &str) -> (Call, bool) {
        let hand: Hand = hand.parse().expect("valid test hand");
        let (logits, prov) = american()
            .against(Family::NATURAL)
            .classify_with_provenance(hand, RelativeVulnerability::NONE, auction)
            .expect("a legal auction classifies");
        let best = (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty");
        (best, prov.depth == 0 && prov.fallback.is_some())
    }

    /// As [`best_call`], with plain Lebensohl forced on (independent of any other
    /// test on this thread having changed the style)
    fn bid(auction: &[Call], hand: &str) -> (Call, bool) {
        super::set_lebensohl_style(super::LebensohlStyle::Plain);
        best_call(auction, hand)
    }

    /// As [`best_call`], with Transfer Lebensohl forced on
    fn bid_transfer(auction: &[Call], hand: &str) -> (Call, bool) {
        super::set_lebensohl_style(super::LebensohlStyle::Transfer);
        best_call(auction, hand)
    }

    #[test]
    fn transfer_smolen_three_clubs_is_stayman() {
        // 1NT–(2♦): a 4-4 majors game-force bids 3♣ Stayman (a book node).
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) = bid_transfer(&auction, "AQ32.KJ32.A2.432");
        assert_eq!(c, call(3, Strain::Clubs));
        assert!(!floored, "Stayman must come from the book");
    }

    #[test]
    fn transfer_smolen_opener_answers_stayman() {
        // 1NT–(2♦)–3♣: opener shows a 4-card major (3♥ here).
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            call(3, Strain::Clubs),
            Call::Pass,
        ];
        let (c, floored) = bid_transfer(&auction, "K2.AQ54.A32.Q432");
        assert_eq!(c, call(3, Strain::Hearts));
        assert!(!floored, "the Stayman answer must come from the book");
    }

    #[test]
    fn transfer_smolen_three_diamonds_is_the_heart_transfer() {
        // The reshuffle: 1NT–(2♦)–3♦ shows hearts (the freed cue slot), a book node.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) = bid_transfer(&auction, "K3.KQ976.A32.432");
        assert_eq!(c, call(3, Strain::Diamonds));
        assert!(!floored, "the heart transfer must come from the book");

        // Opener auto-drives the INV+ transfer to game with a fit.
        let opener = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            call(3, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, _) = bid_transfer(&opener, "AQ5.A432.KQ4.J32");
        assert_eq!(c, call(4, Strain::Hearts));
    }

    #[test]
    fn transfer_smolen_routes_five_four_to_stayman_not_a_transfer() {
        // A 5♠4♥ game-force must bid 3♣ Stayman (1.85), not the 3♥ spade transfer
        // (1.8) — else Smolen could never show the 5-4.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, _) = bid_transfer(&auction, "AKJ54.Q432.K2.32");
        assert_eq!(c, call(3, Strain::Clubs));
    }

    #[test]
    fn transfer_smolen_jumps_smolen_after_the_denial() {
        // 1NT–(2♦)–3♣–P–3♦(no major)–P: responder bids Smolen 3♥ to show 5 spades.
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            call(3, Strain::Clubs),
            Call::Pass,
            call(3, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, floored) = bid_transfer(&auction, "AKJ54.Q432.K2.32");
        assert_eq!(c, call(3, Strain::Hearts));
        assert!(!floored, "Smolen must come from the book");

        // Opener completes in the five-card spade game.
        let mut full = auction.to_vec();
        full.push(call(3, Strain::Hearts));
        full.push(Call::Pass);
        let (c, _) = bid_transfer(&full, "Q32.A65.AQ43.K32");
        assert_eq!(c, call(4, Strain::Spades));
    }

    #[test]
    fn transfer_smolen_leaping_michaels_both_majors() {
        // 1NT–(2♦)–4♦ = both majors 5-5, game-forcing.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) = bid_transfer(&auction, "KQ954.AJ876.2.32");
        assert_eq!(c, call(4, Strain::Diamonds));
        assert!(!floored, "Leaping Michaels must come from the book");

        // Opener bids game in the better major (4♠ on three-card support).
        let opener = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            call(4, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, _) = bid_transfer(&opener, "A32.K43.AQ32.Q42");
        assert_eq!(c, call(4, Strain::Spades));
    }

    #[test]
    fn transfer_smolen_keeps_cohen_over_a_major_overcall() {
        // Over (2♥), Transfer is plain Cohen: 5 spades transfers through
        // hearts — 3♦ shows spades (the Smolen reshuffle is (2♦)-only).
        let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
        let (c, floored) = bid_transfer(&auction, "AKQ65.43.K32.J32");
        assert_eq!(c, call(3, Strain::Diamonds));
        assert!(!floored, "the Cohen transfer must come from the book");
    }

    #[test]
    fn lebensohl_forcing_three_level_is_a_book_node() {
        // 1NT–(2♦); responder 5 spades, game values, no diamond stopper →
        // forcing 3♠ (a jump), not a partscore.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) = bid(&auction, "KQT95.A43.32.J32");
        assert_eq!(c, call(3, Strain::Spades));
        assert!(!floored, "the forcing 3-level bid must come from the book");
    }

    #[test]
    fn lebensohl_weak_long_suit_relays_then_completes() {
        // Weak hand (6 HCP), 6 clubs, over 2♦ → 2NT relay; opener forced to 3♣.
        let responder = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) = bid(&responder, "J2.43.32.KQ9876");
        assert_eq!(c, call(2, Strain::Notrump));
        assert!(!floored, "the Lebensohl relay must come from the book");

        let opener = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        let (completion, _) = bid(&opener, "AQ32.KQ5.AQ4.A32");
        assert_eq!(completion, call(3, Strain::Clubs));
    }

    #[test]
    fn lebensohl_weak_bids_natural_two_level() {
        // A weak hand with 5 hearts bids natural 2♥ (below 2NT), to play.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) = bid(&auction, "K2.QJ976.432.432");
        assert_eq!(c, call(2, Strain::Hearts));
        assert!(!floored, "the natural 2-level bid must come from the book");
    }

    #[test]
    fn lebensohl_cue_is_stayman() {
        // 1NT–(2♥): a game-force with 4 spades and no 5-card suit cues 3♥ = Stayman
        // (it cannot bid a forcing 3-level suit, and the cue outranks direct 3NT).
        let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
        let (c, floored) = bid(&auction, "AQ32.K43.A32.K32");
        assert_eq!(c, call(3, Strain::Hearts));
        assert!(!floored, "the cue must come from the book");

        // Opener answers Stayman with the 4-card spade fit.
        let opener = [
            call(1, Strain::Notrump),
            call(2, Strain::Hearts),
            call(3, Strain::Hearts),
            Call::Pass,
        ];
        let (a, floored) = bid(&opener, "KJ54.A32.K43.Q32");
        assert_eq!(a, call(3, Strain::Spades));
        assert!(!floored, "the Stayman answer must come from the book");
    }

    #[test]
    fn lebensohl_five_card_suit_relays_then_signs_off_at_the_three_level() {
        // Weak hand, a 5-card heart suit it cannot show at the 2 level (below
        // their 2♠): relay 2NT, then correct 3♣→3♥ as a 3-level sign-off.
        let responder = [call(1, Strain::Notrump), call(2, Strain::Spades)];
        let (c, floored) = bid(&responder, "32.KQJ32.432.432");
        assert_eq!(c, call(2, Strain::Notrump));
        assert!(!floored, "the relay must come from the book");

        let after_3c = [
            call(1, Strain::Notrump),
            call(2, Strain::Spades),
            call(2, Strain::Notrump),
            Call::Pass,
            call(3, Strain::Clubs),
            Call::Pass,
        ];
        let (c, floored) = bid(&after_3c, "32.KQJ32.432.432");
        assert_eq!(c, call(3, Strain::Hearts));
        assert!(!floored, "the 3-level sign-off must come from the book");
    }

    #[test]
    fn transfer_lebensohl_shows_spades_through_their_hearts() {
        // 1NT–(2♥); responder, 5 spades and game values, transfers *through*
        // hearts: 3♦ shows spades (not diamonds), a book node.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
        let (c, floored) = bid_transfer(&auction, "AKQ65.43.K32.J32");
        assert_eq!(c, call(3, Strain::Diamonds));
        assert!(!floored, "the transfer must come from the book");
    }

    #[test]
    fn transfer_lebensohl_opener_bids_game_not_a_partscore() {
        // After 1NT–(2♥)–3♦ (transfer to spades), opener with a fit must bid
        // the spade *game*, never a 3♠ partscore (the Rubensohl-v1 failure).
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Hearts),
            call(3, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, _) = bid_transfer(&auction, "AK5.KQ52.A43.432");
        assert_eq!(c, call(4, Strain::Spades));
    }

    #[test]
    fn transfer_lebensohl_cue_is_stayman() {
        // 1NT–(2♥)–3♥ is the cue = Stayman; opener answers a 4-card major.
        // (Over (2♦) the cue slot is freed for the Smolen 3♣-Stayman instead.)
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Hearts),
            call(3, Strain::Hearts),
            Call::Pass,
        ];
        let (c, floored) = bid_transfer(&auction, "AQ32.K43.A32.K32");
        assert_eq!(c, call(3, Strain::Spades));
        assert!(!floored, "the Stayman answer must come from the book");
    }

    #[test]
    fn transfer_lebensohl_keeps_the_penalty_double() {
        // Length and values in their suit, no game bid of our own: penalty
        // double, from the book — Rubensohl v1 lost this by shadowing the floor.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) = bid_transfer(&auction, "K2.K43.J932.Q432");
        assert_eq!(c, Call::Double);
        assert!(!floored, "the penalty double must come from the book");
    }

    #[test]
    fn transfer_lebensohl_weak_bids_natural_two_level() {
        // Weak 5-card heart hand still bids natural 2♥ (transfers are INV+).
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) = bid_transfer(&auction, "K2.QJ976.432.432");
        assert_eq!(c, call(2, Strain::Hearts));
        assert!(!floored, "the natural 2-level bid must come from the book");
    }

    #[test]
    fn transfer_lebensohl_top_step_is_a_clubs_transfer() {
        // The top step (no suit above to transfer into) is a forced game-force
        // transfer to clubs: 6+♣, game values, no stopper in their suit. The same
        // 10-HCP hand bids it over every overcall — 3♠ over (2♦)/(2♥), 3♥ over
        // (2♠) — a book node, never the natural floor.
        let hand = "32.543.32.AKQJ86";
        for (over, top) in [
            (Strain::Diamonds, Strain::Spades),
            (Strain::Hearts, Strain::Spades),
            (Strain::Spades, Strain::Hearts),
        ] {
            let auction = [call(1, Strain::Notrump), call(2, over)];
            let (c, floored) = bid_transfer(&auction, hand);
            assert_eq!(c, call(3, top), "top step → clubs over (2{over:?})");
            assert!(!floored, "the clubs transfer must come from the book");
        }
        // Plain Transfer (Cohen) gets it over (2♦) too — previously floored.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) = bid_transfer(&auction, hand);
        assert_eq!(c, call(3, Strain::Spades));
        assert!(!floored);
    }

    #[test]
    fn transfer_lebensohl_top_step_opener_completes_at_game() {
        // After 1NT–(2♥)–3♠ (transfer to clubs, forced GF): opener bids 3NT with
        // a heart stopper, else raises to 5♣ — 3♣ is unplayable, so it reaches game.
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Hearts),
            call(3, Strain::Spades),
            Call::Pass,
        ];
        let (c, floored) = bid_transfer(&auction, "A432.KQ5.A32.432");
        assert_eq!(c, call(3, Strain::Notrump), "stopper → 3NT");
        assert!(!floored, "the completion must come from the book");

        let (c, _) = bid_transfer(&auction, "A432.543.AKQ.432");
        assert_eq!(c, call(5, Strain::Clubs), "no stopper → 5♣");
    }
}
