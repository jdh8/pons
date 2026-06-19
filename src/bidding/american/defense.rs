//! Defensive actions for the 2/1 system: overcalls, advances, and doubles
//!
//! This module covers everything our side does when the opponents open the
//! auction: simple overcalls, the 1NT overcall, takeout doubles, the
//! Michaels cue-bid, the Unusual 2NT, advances of all of these, advancing
//! partner's takeout double, responsive doubles when partner has made a
//! takeout double and they raise, and defense to a weak-two opening (takeout
//! double, a natural 2NT overcall, and natural suit overcalls).

use super::super::constraint::{
    balanced, described, hcp, len, min_level_is, points, short_in_their_suits,
    stopper_in_their_suits, top_honors,
};
use super::super::context::Context;
use super::super::{Defensive, Rules};
use super::competition::{
    LebensohlStyle, complete_lebensohl_relay, cue_stayman_answer, lebensohl_relay_rebid,
    lebensohl_responder, transfer_completion, transfer_lebensohl_responder, transfer_target,
};
use super::{call, insert_all_seats};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};
use std::cell::Cell;

// ---------------------------------------------------------------------------
// Sohl after a takeout double (advancing partner's takeout double of a weak two)
// ---------------------------------------------------------------------------

thread_local! {
    /// Which sohl package the advancer carries after partner's takeout double of
    /// a weak two (`(2X)–X–(P)`); see [`set_advance_sohl_style`].
    static ADVANCE_SOHL: Cell<LebensohlStyle> = const { Cell::new(LebensohlStyle::Off) };
}

/// Select the sohl package the **advancer** carries after partner's takeout
/// double of a weak two, for books built *after* this call (thread-local, read
/// once at book-construction time)
///
/// Reuses [`LebensohlStyle`]: `Off` (the **default**) keeps the flat
/// [`advance_double`] ladder; `Plain` adds the weak `2NT` relay vs a forcing
/// 3-level suit; `Transfer` adds Larry Cohen's transfers-through + cue-Stayman
/// (the best variant in the A/B, but only DD-neutral vs the floor — so it stays
/// opt-in, not the default; see `docs/ai-bidder/21gf-ledger.md`). The geometry
/// matches Lebensohl after our overcalled `1NT` (the opponents' suit is at the
/// two level in both), so the Section-5 builders are reused verbatim under the
/// `(2X)–X–(P)` prefix.
pub fn set_advance_sohl_style(style: LebensohlStyle) {
    ADVANCE_SOHL.with(|cell| cell.set(style));
}

/// The currently selected advance-of-double sohl package
fn advance_sohl_style() -> LebensohlStyle {
    ADVANCE_SOHL.with(Cell::get)
}

thread_local! {
    /// Whether Leaping Michaels (4♣/4♦ strong two-suiters over their weak two)
    /// is active; see [`set_leaping_michaels`].
    static LEAPING_MICHAELS: Cell<bool> = const { Cell::new(true) };
}

/// Toggle Leaping Michaels for books built *after* this call (thread-local, read
/// once at book-construction time)
///
/// Over their weak two, a jump to `4♣`/`4♦` names a 5-5 two-suiter with
/// game-forcing values: over a major it is a minor plus the *other* major; over
/// `2♦` the `4♦` cue shows both majors and `4♣` shows clubs plus a major.  **On by
/// default** — the authored advances make it a clear DD win (+1.090/+1.452
/// IMPs/board, none/both), and the inference reader lets the live-search bidder
/// price the advance (and reach slam) on top; see `docs/ai-bidder/21gf-ledger.md`.
/// Turn it off to recover the pre-Leaping-Michaels weak-two defense.
pub fn set_leaping_michaels(on: bool) {
    LEAPING_MICHAELS.with(|cell| cell.set(on));
}

/// Whether Leaping Michaels is currently enabled
///
/// Crate-visible so the inference reader can condition partner's hand on the
/// two-suiter when the search bidder samples (see `inference::leaping_michaels_reading`).
pub(crate) fn leaping_michaels_enabled() -> bool {
    LEAPING_MICHAELS.with(Cell::get)
}

// ---------------------------------------------------------------------------
// Direct overcalls and doubles
// ---------------------------------------------------------------------------

/// Our action over their one-of-a-suit opening
///
/// One decision: a natural overcall (five-card suit), a takeout double, a
/// 15–18 1NT overcall, or pass.  Strong hands (17+) double first regardless
/// of shape, planning to bid again — otherwise an opening-strength hand with
/// length in the opponents' suit would be stuck.
///
/// Two-suited overcalls are also available:
/// - **Michaels cue-bid** (2 of their suit, 8+ HCP, 5-5): over a minor,
///   both majors; over a major, the other major and an unspecified minor.
/// - **Unusual 2NT** (8+ HCP, 5-5 in the two lowest unbid suits): over 1♣
///   shows diamonds and hearts; over 1♦ shows clubs and hearts; over a major
///   shows both minors.
#[must_use]
pub fn defense_to_suit(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let t = theirs.suit().expect("their opening is always a suit bid");

    let mut rules = Rules::new()
        .rule(
            Bid::new(1, Strain::Notrump),
            1.5,
            hcp(15..=18) & balanced() & stopper_in_their_suits(),
        )
        .rule(Call::Double, 1.3, hcp(12..) & short_in_their_suits())
        .rule(Call::Double, 1.2, points(17..))
        .rule(Call::Pass, 0.0, hcp(0..));

    // Natural overcalls: five-card suit, 8–16 points.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain != theirs {
            let level = if strain > theirs { 1 } else { 2 };
            let weight = if level == 1 { 1.4 } else { 1.0 };
            rules = rules.rule(
                Bid::new(level, strain),
                weight,
                len(suit, 5..) & points(8..=16),
            );
        }
    }

    // Michaels cue-bid: 2 of their suit, 5-5, 8+ HCP.
    rules = match t {
        // t minor → both majors
        Suit::Clubs | Suit::Diamonds => rules.rule(
            Bid::new(2, theirs),
            2.0,
            len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & points(8..),
        ),
        // t = ♥ → spades + a minor
        Suit::Hearts => rules.rule(
            Bid::new(2, theirs),
            2.0,
            len(Suit::Spades, 5..)
                & (len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..))
                & points(8..),
        ),
        // t = ♠ → hearts + a minor
        Suit::Spades => rules.rule(
            Bid::new(2, theirs),
            2.0,
            len(Suit::Hearts, 5..)
                & (len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..))
                & points(8..),
        ),
    };

    // Unusual 2NT: 5-5 in the two lowest unbid suits, 8+ HCP.
    match t {
        Suit::Clubs => rules.rule(
            Bid::new(2, Strain::Notrump),
            1.9,
            len(Suit::Diamonds, 5..) & len(Suit::Hearts, 5..) & points(8..),
        ),
        Suit::Diamonds => rules.rule(
            Bid::new(2, Strain::Notrump),
            1.9,
            len(Suit::Clubs, 5..) & len(Suit::Hearts, 5..) & points(8..),
        ),
        Suit::Hearts | Suit::Spades => rules.rule(
            Bid::new(2, Strain::Notrump),
            1.9,
            len(Suit::Clubs, 5..) & len(Suit::Diamonds, 5..) & points(8..),
        ),
    }
}

/// Our action over their weak-two opening
///
/// A weak two steals a level of room, so the toolkit is leaner than over a
/// one-bid: a takeout double (the workhorse), a natural 2NT overcall (15–18
/// with a stopper), and natural suit overcalls at the cheapest legal level.
/// Strong hands (17+) still double first, planning to bid again.
///
/// Overcall levels are derived from `their_opening`, so the suits higher than
/// theirs sit at the opening level and the lower ones one rung up — over 2♥, a
/// spade overcall is 2♠ but a club overcall is 3♣.
#[must_use]
pub fn defense_to_weak_two(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let level = their_opening.level.get();

    let mut rules = Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            1.5,
            hcp(15..=18) & balanced() & stopper_in_their_suits(),
        )
        .rule(Call::Double, 1.3, hcp(12..) & short_in_their_suits())
        .rule(Call::Double, 1.2, points(17..))
        .rule(Call::Pass, 0.0, hcp(0..));

    // Natural overcalls: five-card suit, 10–16 points, at the cheapest legal level.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain != theirs {
            let overcall_level = if strain > theirs { level } else { level + 1 };
            rules = rules.rule(
                Bid::new(overcall_level, strain),
                1.0,
                len(suit, 5..) & points(10..=16),
            );
        }
    }

    // Leaping Michaels: a jump to 4♣/4♦ showing a 5-5 two-suiter with
    // game-forcing values.  These are all 4-level jumps, so they never collide
    // with the natural overcalls above (which sit at the 2/3 level), and 4♦ over
    // 2♦ is a cue the natural loop skips.
    if leaping_michaels_enabled() {
        let t = theirs.suit().expect("weak two is a suit bid");
        let gf = points(14..);
        match t {
            // Over a major: a minor plus the OTHER major.
            Suit::Hearts | Suit::Spades => {
                let other = if t == Suit::Hearts {
                    Suit::Spades
                } else {
                    Suit::Hearts
                };
                for minor in [Suit::Clubs, Suit::Diamonds] {
                    rules = rules.rule(
                        Bid::new(4, Strain::from(minor)),
                        2.0,
                        len(minor, 5..) & len(other, 5..) & gf.clone(),
                    );
                }
            }
            // Over 2♦: 4♣ = clubs + a major; 4♦ (cue) = both majors.  Advancer's
            // continuation (incl. the 4♣ major-ask) is authored in
            // `leaping_michaels_advances`.
            Suit::Diamonds => {
                rules = rules
                    .rule(
                        Bid::new(4, Strain::Clubs),
                        2.0,
                        len(Suit::Clubs, 5..)
                            & (len(Suit::Hearts, 5..) | len(Suit::Spades, 5..))
                            & gf.clone(),
                    )
                    .rule(
                        Bid::new(4, Strain::Diamonds),
                        2.0,
                        len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & gf.clone(),
                    );
            }
            Suit::Clubs => {} // no weak 2♣ in our system
        }
    }
    rules
}

/// Our action over their 1NT opening: penalty double or natural two-level overcall
pub fn defense_to_notrump() -> Rules {
    let mut rules = Rules::new()
        .rule(Call::Double, 1.3, hcp(15..) & balanced())
        .rule(Call::Pass, 0.0, hcp(0..));
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        rules = rules.rule(
            Bid::new(2, Strain::from(suit)),
            1.0,
            len(suit, 5..) & points(8..=14),
        );
    }
    rules
}

// ---------------------------------------------------------------------------
// Advances
// ---------------------------------------------------------------------------

/// Advancer's action after partner's takeout double, RHO passing: `(opening) X (P)`
///
/// Partner doubled for takeout and asked us to pick.  In priority order:
///
/// - **pass for penalty** with a trump stack (four-plus of their suit, two top
///   honors) — converting the takeout double into penalties;
/// - **jump to a major-suit game** with four-plus cards and opening values;
/// - **bid 3NT** with a stopper in their suit and game-going values;
/// - **bid a new suit** at the cheapest legal level with four-plus cards;
/// - **escape to the cheapest notrump** as a weak catch-all — no fit, no
///   stopper, nothing better to say (lebensohl in spirit);
/// - **pass** as the final fallback.
///
/// Suit and notrump levels are derived from `their_opening`, so the one builder
/// answers over a one-bid (advances at the one and two levels) and over a weak
/// two (advances at the two and three levels) alike.
#[must_use]
pub fn advance_double(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let t = theirs.suit().expect("their opening is always a suit bid");
    let level = their_opening.level.get();

    let mut rules = Rules::new()
        // Convert for penalty: a trump stack sits for the double.
        .rule(Call::Pass, 1.5, len(t, 4..) & top_honors(t, 2..) & hcp(6..))
        // 3NT to play: a stopper in their suit and game values.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.3,
            hcp(13..) & stopper_in_their_suits(),
        )
        // Weak escape to the cheapest notrump: no fit, no stopper, no stack.
        .rule(Bid::new(level, Strain::Notrump), 0.3, hcp(0..))
        // Final fallback.
        .rule(Call::Pass, 0.0, hcp(0..));

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain == theirs {
            continue;
        }
        let bid_level = if strain > theirs { level } else { level + 1 };
        // Natural advance at the cheapest legal level.
        rules = rules.rule(Bid::new(bid_level, strain), 1.0, len(suit, 4..));
        // Major-suit game jump with support and opening values.
        if matches!(suit, Suit::Hearts | Suit::Spades) {
            rules = rules.rule(Bid::new(4, strain), 1.4, len(suit, 4..) & points(11..));
        }
    }
    rules
}

/// Insert the advancer's actions after partner's takeout double of weak-two
/// `opening` (in `suit`), honoring the selected [`set_advance_sohl_style`]
///
/// `Off` keeps the flat [`advance_double`] ladder.  `Plain`/`Transfer` shadow it
/// with the reused Section-5 sohl builders under the `[2X, X, P]` prefix — the
/// weak `2NT` relay (and, for `Transfer`, the transfers-through + cue-Stayman) —
/// plus the doubler's continuations (relay completion, the rebid after `3♣`, and
/// the transfer / cue answers).  A forcing 3-level suit (`Plain`) or a
/// constructive advance is driven on by the instinct floor, which already
/// handles forced-to-game auctions.
fn insert_advance_of_double(d: &mut Defensive, suit: Suit, opening: Bid, style: LebensohlStyle) {
    let dbl_p = [Call::Bid(opening), Call::Double, Call::Pass];
    if style == LebensohlStyle::Off {
        insert_all_seats(d, &dbl_p, 3, advance_double(opening));
        return;
    }

    // Advancer's first action shadows the floor (the builders end in a 0.0 Pass,
    // which covers the weak and penalty-pass hands).
    let advancer = if style == LebensohlStyle::Transfer {
        transfer_lebensohl_responder(suit)
    } else {
        lebensohl_responder(suit)
    };
    insert_all_seats(d, &dbl_p, 3, advancer);

    // Doubler completes the 2NT relay with a forced 3♣; advancer then signs off.
    let two_nt = call(2, Strain::Notrump);
    let three_clubs = call(3, Strain::Clubs);
    insert_all_seats(
        d,
        &[
            Call::Bid(opening),
            Call::Double,
            Call::Pass,
            two_nt,
            Call::Pass,
        ],
        3,
        complete_lebensohl_relay(),
    );
    insert_all_seats(
        d,
        &[
            Call::Bid(opening),
            Call::Double,
            Call::Pass,
            two_nt,
            Call::Pass,
            three_clubs,
            Call::Pass,
        ],
        3,
        lebensohl_relay_rebid(suit),
    );

    // Transfer style: the doubler answers each 3-level transfer / cue.
    if style == LebensohlStyle::Transfer {
        for bid_suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let resp = call(3, Strain::from(bid_suit));
            let reply = if bid_suit == suit {
                cue_stayman_answer(suit)
            } else if let Some(target) = transfer_target(bid_suit, suit) {
                transfer_completion(target, suit)
            } else {
                continue; // the lowest suit has no transfer target — floored
            };
            insert_all_seats(
                d,
                &[
                    Call::Bid(opening),
                    Call::Double,
                    Call::Pass,
                    resp,
                    Call::Pass,
                ],
                3,
                reply,
            );
        }
    }
}

/// Advancer's response to partner's Michaels cue-bid over their opening `t`
fn michaels_advances(t: Suit) -> Rules {
    match t {
        // Partner shows both majors: prefer the longer one.
        Suit::Clubs | Suit::Diamonds => {
            let hearts_longer = described(
                "♥ at least as long as ♠",
                |hand: Hand, _: &Context<'_>| hand[Suit::Hearts].len() >= hand[Suit::Spades].len(),
            );
            let spades_longer = described("♠ longer than ♥", |hand: Hand, _: &Context<'_>| {
                hand[Suit::Spades].len() > hand[Suit::Hearts].len()
            });
            Rules::new()
                .rule(
                    Bid::new(4, Strain::Hearts),
                    1.3,
                    points(10..) & len(Suit::Hearts, 3..) & hearts_longer.clone(),
                )
                .rule(
                    Bid::new(4, Strain::Spades),
                    1.3,
                    points(10..) & len(Suit::Spades, 3..) & spades_longer.clone(),
                )
                .rule(Bid::new(2, Strain::Hearts), 1.0, hearts_longer)
                .rule(Bid::new(2, Strain::Spades), 1.0, spades_longer)
        }
        // Partner shows spades + a minor: bid spades.
        Suit::Hearts => Rules::new()
            .rule(
                Bid::new(4, Strain::Spades),
                1.3,
                points(10..) & len(Suit::Spades, 3..),
            )
            .rule(Bid::new(2, Strain::Spades), 0.5, hcp(0..)),
        // Partner shows hearts + a minor: bid hearts.
        Suit::Spades => Rules::new()
            .rule(
                Bid::new(4, Strain::Hearts),
                1.3,
                points(10..) & len(Suit::Hearts, 3..),
            )
            .rule(Bid::new(3, Strain::Hearts), 0.5, hcp(0..)),
    }
}

/// Advancer's response to partner's Leaping Michaels jump over their weak two
///
/// `theirs` is the suit they opened; `lm` is the suit of the jump (Clubs or
/// Diamonds).  The overcall is game-forcing, so every advance reaches game.
/// - Over a **major**, the jump names `lm` plus the *other* major: bid that
///   major game with a fit, else the `lm` minor game.
/// - Over **2♦**, the `4♦` *cue* shows both majors → pick the longer; the `4♣`
///   jump shows clubs + an unknown major → `5♣` with a club fit and no major,
///   else `4♥` pass-or-correct (see [`leaping_michaels_2d_4c_rebid`]).
fn leaping_michaels_advances(theirs: Suit, lm: Suit) -> Rules {
    match theirs {
        // Over a major: lm + the OTHER major, both known.
        Suit::Hearts | Suit::Spades => {
            let major = if theirs == Suit::Hearts {
                Suit::Spades
            } else {
                Suit::Hearts
            };
            // Prefer the major game even on a doubleton (a 7-card fit) — it
            // scores well and needs only ten tricks; retreat to the 5m game only
            // on a genuine major misfit (≤1), where DD has to make eleven.
            Rules::new()
                .rule(Bid::new(4, Strain::from(major)), 1.3, len(major, 2..))
                .rule(Bid::new(5, Strain::from(lm)), 1.2, len(major, 0..=1))
        }
        // Over 2♦.
        Suit::Diamonds => match lm {
            // 4♦ cue = both majors: pick the longer (both forced to game).
            Suit::Diamonds => {
                let hearts_longer =
                    described("♥ at least as long as ♠", |h: Hand, _: &Context<'_>| {
                        h[Suit::Hearts].len() >= h[Suit::Spades].len()
                    });
                let spades_longer = described("♠ longer than ♥", |h: Hand, _: &Context<'_>| {
                    h[Suit::Spades].len() > h[Suit::Hearts].len()
                });
                Rules::new()
                    .rule(Bid::new(4, Strain::Hearts), 1.3, hearts_longer)
                    .rule(Bid::new(4, Strain::Spades), 1.3, spades_longer)
            }
            // 4♣ = clubs + a major: 5♣ with a club fit and no major, else 4♥
            // pass-or-correct (partner names their major).
            Suit::Clubs => Rules::new()
                .rule(
                    Bid::new(5, Strain::Clubs),
                    1.2,
                    len(Suit::Clubs, 3..) & len(Suit::Hearts, 0..=2) & len(Suit::Spades, 0..=2),
                )
                .rule(Bid::new(4, Strain::Hearts), 1.3, hcp(0..)),
            _ => unreachable!("a Leaping Michaels jump is clubs or diamonds"),
        },
        Suit::Clubs => unreachable!("there is no weak 2♣ opening"),
    }
}

/// Overcaller's rebid after `(2♦)–4♣–(P)–4♥–(P)`: pass-or-correct to their major
///
/// `4♣` over `2♦` showed clubs + a major; advancer's `4♥` is pass-or-correct, so
/// the overcaller passes with hearts or corrects to `4♠` with spades.
fn leaping_michaels_2d_4c_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Spades), 1.3, len(Suit::Spades, 5..))
        .rule(Call::Pass, 1.0, hcp(0..))
}

/// The two suits shown by an Unusual 2NT over their opening `t`
///
/// Returns `(a, b)` where `a < b` (lower suit first).
const fn unusual_suits(t: Suit) -> (Suit, Suit) {
    match t {
        Suit::Clubs => (Suit::Diamonds, Suit::Hearts),
        Suit::Diamonds => (Suit::Clubs, Suit::Hearts),
        Suit::Hearts | Suit::Spades => (Suit::Clubs, Suit::Diamonds),
    }
}

/// Advancer's response to partner's Unusual 2NT over their opening `t`
fn unusual_nt_advances(t: Suit) -> Rules {
    let (a, b) = unusual_suits(t);
    let a_longer = described(
        format!("{a} at least as long as {b}"),
        move |hand: Hand, _: &Context<'_>| hand[a].len() >= hand[b].len(),
    );
    let b_longer = described(
        format!("{b} longer than {a}"),
        move |hand: Hand, _: &Context<'_>| hand[b].len() > hand[a].len(),
    );
    Rules::new()
        .rule(Bid::new(3, Strain::from(a)), 1.0, a_longer)
        .rule(Bid::new(3, Strain::from(b)), 1.0, b_longer)
}

// ---------------------------------------------------------------------------
// Responsive doubles
// ---------------------------------------------------------------------------

/// Advancer's action when partner made a takeout double and they raised `t` to `raise_lvl`
///
/// Responsive double: both suits of the rank opposite the opened suit (minor/major).
/// Natural bids at the minimum legal level (2–3) for suits other than `t`, 5-card, 8+ HCP.
fn responsive_doubles(t: Suit, _raise_lvl: u8) -> Rules {
    // Responsive double shows the two unbid suits of the same rank (minor or major).
    let mut rules = if matches!(t, Suit::Hearts | Suit::Spades) {
        // t major → both minors
        Rules::new().rule(
            Call::Double,
            1.5,
            len(Suit::Clubs, 4..) & len(Suit::Diamonds, 4..) & points(8..),
        )
    } else {
        // t minor → both majors
        Rules::new().rule(
            Call::Double,
            1.5,
            len(Suit::Hearts, 4..) & len(Suit::Spades, 4..) & points(8..),
        )
    };

    rules = rules.rule(Call::Pass, 0.0, hcp(0..));

    // Natural bids for suits ≠ t at levels 2 and 3.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == t {
            continue;
        }
        let strain = Strain::from(suit);
        for bid_lvl in 2u8..=3 {
            rules = rules.rule(
                Bid::new(bid_lvl, strain),
                1.0,
                min_level_is(bid_lvl, strain) & len(suit, 5..) & points(8..),
            );
        }
    }
    rules
}

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

/// Build the defensive book: all our actions when the opponents open
///
/// Seat-fanned with `insert_all_seats(…, 3, …)` so every seat is covered.
/// Keys for a defensive auction are the raw table auction starting from their
/// opening, e.g. `[1♦, 2♦, Pass]` means they opened 1♦, we cue-bid 2♦
/// (Michaels), opener's side passed, and we are the advancer.
#[must_use]
pub fn defensive() -> Defensive {
    let mut d = Defensive::new();
    let advance_sohl = advance_sohl_style();

    // Over each one-of-a-suit opening: overcalls, double, 1NT, Michaels, Unusual.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let theirs = Strain::from(suit);
        let opening = Bid::new(1, theirs);
        insert_all_seats(&mut d, &[Call::Bid(opening)], 3, defense_to_suit(opening));

        // Advancing partner's takeout double: [1t, X, P] — advancer to act.
        insert_all_seats(
            &mut d,
            &[Call::Bid(opening), Call::Double, Call::Pass],
            3,
            advance_double(opening),
        );

        // Advances of a natural overcall ([1t, overcall, Pass]) are left to the
        // instinct floor's Rubens transfers — the programmatic floor expresses
        // the transfer band for every (opening, overcall) pair in one place,
        // where a per-suit authored table cannot.

        // Advances of Michaels: [1t, 2t, Pass] — advancer to act.
        let michaels_bid = call(2, theirs);
        insert_all_seats(
            &mut d,
            &[Call::Bid(opening), michaels_bid, Call::Pass],
            3,
            michaels_advances(suit),
        );

        // Advances of Unusual 2NT: [1t, 2NT, Pass] — advancer to act.
        let unusual_bid = call(2, Strain::Notrump);
        insert_all_seats(
            &mut d,
            &[Call::Bid(opening), unusual_bid, Call::Pass],
            3,
            unusual_nt_advances(suit),
        );

        // Responsive doubles: partner doubled for takeout, they raised to lvl.
        for raise_lvl in [2u8, 3] {
            let raise = call(raise_lvl, theirs);
            insert_all_seats(
                &mut d,
                &[Call::Bid(opening), Call::Double, raise],
                3,
                responsive_doubles(suit, raise_lvl),
            );
        }
    }

    // Over each weak-two opening: takeout double, natural overcalls, 2NT, and
    // advancing partner's takeout double.  Clubs is omitted — a 2♣ opening is
    // the strong artificial bid, not a weak two.
    for suit in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let theirs = Strain::from(suit);
        let opening = Bid::new(2, theirs);
        insert_all_seats(
            &mut d,
            &[Call::Bid(opening)],
            3,
            defense_to_weak_two(opening),
        );

        // Advancing partner's takeout double: [2t, X, P] — advancer to act.
        // Plain/Transfer sohl per `set_advance_sohl_style` (default Off keeps the
        // flat `advance_double` ladder).
        insert_advance_of_double(&mut d, suit, opening, advance_sohl);

        // Advances of Leaping Michaels: [2t, 4m, P] — advancer to act.  The jump
        // is below game, so the advancer is forced on (a fit major game, else the
        // 5m minor game — never a passed 4m partscore).
        if leaping_michaels_enabled() {
            for lm in [Suit::Clubs, Suit::Diamonds] {
                insert_all_seats(
                    &mut d,
                    &[Call::Bid(opening), call(4, Strain::from(lm)), Call::Pass],
                    3,
                    leaping_michaels_advances(suit, lm),
                );
            }
            // Over 2♦, 4♣ shows clubs + an unknown major; advancer's 4♥ is
            // pass-or-correct, so the overcaller names their major in rebid.
            if suit == Suit::Diamonds {
                insert_all_seats(
                    &mut d,
                    &[
                        Call::Bid(opening),
                        call(4, Strain::Clubs),
                        Call::Pass,
                        call(4, Strain::Hearts),
                        Call::Pass,
                    ],
                    3,
                    leaping_michaels_2d_4c_rebid(),
                );
            }
        }
    }

    insert_all_seats(&mut d, &[call(1, Strain::Notrump)], 3, defense_to_notrump());
    d
}

#[cfg(test)]
mod tests {
    use crate::bidding::Family;
    use crate::bidding::american::{
        LebensohlStyle, american, set_advance_sohl_style, set_leaping_michaels,
    };
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

    /// Best call with the advance-of-double sohl forced to `style` (independent of
    /// any other test on this thread having changed it)
    fn advance(style: LebensohlStyle, auction: &[Call], hand: &str) -> (Call, bool) {
        set_advance_sohl_style(style);
        best_call(auction, hand)
    }

    /// `(2♦)–X–(P)` — partner doubled their weak two, advancer to act
    fn over_2d() -> [Call; 3] {
        [call(2, Strain::Diamonds), Call::Double, Call::Pass]
    }

    #[test]
    fn off_keeps_the_flat_advance_no_relay() {
        // Default Off: a weak six-club hand bids the natural 3♣ (advance_double),
        // not the 2NT relay — the toggle gates the new structure.
        let (c, _) = advance(LebensohlStyle::Off, &over_2d(), "32.43.32.KQ9876");
        assert_eq!(c, call(3, Strain::Clubs));
    }

    #[test]
    fn plain_weak_long_suit_relays_then_completes() {
        // Plain: weak hand, six clubs → 2NT relay; the doubler is forced to 3♣.
        let (c, floored) = advance(LebensohlStyle::Plain, &over_2d(), "32.43.32.KQ9876");
        assert_eq!(c, call(2, Strain::Notrump));
        assert!(!floored, "the relay must come from the book");

        let relayed = [
            call(2, Strain::Diamonds),
            Call::Double,
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        let (completion, _) = advance(LebensohlStyle::Plain, &relayed, "AKJ2.KQ52.4.A532");
        assert_eq!(completion, call(3, Strain::Clubs));
    }

    #[test]
    fn plain_forcing_three_level_is_a_book_node() {
        // Plain: five spades and game values → forcing 3♠ (a jump over 2♦),
        // never a weak partscore.
        let (c, floored) = advance(LebensohlStyle::Plain, &over_2d(), "KQT95.A43.32.J32");
        assert_eq!(c, call(3, Strain::Spades));
        assert!(!floored, "the forcing 3-level bid must come from the book");
    }

    #[test]
    fn transfer_shows_spades_through_their_hearts() {
        // Transfer: over (2♥), five spades and game values transfer *through*
        // hearts — 3♦ shows spades (not diamonds), a book node.
        let over_2h = [call(2, Strain::Hearts), Call::Double, Call::Pass];
        let (c, floored) = advance(LebensohlStyle::Transfer, &over_2h, "AKQ65.43.K32.J32");
        assert_eq!(c, call(3, Strain::Diamonds));
        assert!(!floored, "the transfer must come from the book");
    }

    #[test]
    fn transfer_doubler_bids_game_not_partscore() {
        // After (2♥)–X–(P)–3♦ (transfer to spades), the doubler with a fit bids
        // the spade *game*, never a 3♠ partscore.
        let auction = [
            call(2, Strain::Hearts),
            Call::Double,
            Call::Pass,
            call(3, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, _) = advance(LebensohlStyle::Transfer, &auction, "AK52.4.A432.K432");
        assert_eq!(c, call(4, Strain::Spades));
    }

    #[test]
    fn transfer_cue_is_stayman() {
        // (2♦)–X–(P)–3♦ is the cue = Stayman; the doubler shows a 4-card major.
        let auction = [
            call(2, Strain::Diamonds),
            Call::Double,
            Call::Pass,
            call(3, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, floored) = advance(LebensohlStyle::Transfer, &auction, "AQ32.K32.4.KJ432");
        assert_eq!(c, call(3, Strain::Spades));
        assert!(!floored, "the Stayman answer must come from the book");
    }

    #[test]
    fn penalty_pass_sits_for_the_double() {
        // A trump stack in their suit (five spades over 2♠) has no constructive
        // call — the book's terminal Pass leaves the takeout double in for
        // penalty, exactly as the flat ladder would.
        let over_2s = [call(2, Strain::Spades), Call::Double, Call::Pass];
        let (c, floored) = advance(LebensohlStyle::Plain, &over_2s, "KQJ95.J32.432.32");
        assert_eq!(c, Call::Pass);
        assert!(!floored, "the sign-off Pass must come from the book node");
    }

    /// Best call with Leaping Michaels forced to `on` (and the sohl toggles reset,
    /// independent of any other test on this thread)
    fn leaping(on: bool, auction: &[Call], hand: &str) -> (Call, bool) {
        set_advance_sohl_style(LebensohlStyle::Off);
        set_leaping_michaels(on);
        best_call(auction, hand)
    }

    #[test]
    fn leaping_michaels_minor_plus_other_major_over_a_major() {
        // Over (2♥): 5-5 clubs+spades, game values → 4♣; 5-5 diamonds+spades → 4♦.
        let over_2h = [call(2, Strain::Hearts)];
        let (c, floored) = leaping(true, &over_2h, "AKQ65.4.32.KQJ76");
        assert_eq!(c, call(4, Strain::Clubs));
        assert!(!floored, "Leaping Michaels must come from the book node");

        let (d, _) = leaping(true, &over_2h, "AKQ65.4.KQJ76.32");
        assert_eq!(d, call(4, Strain::Diamonds));
    }

    #[test]
    fn leaping_michaels_cue_shows_both_majors_over_2d() {
        // Over (2♦): 5-5 in the majors → 4♦ (the cue), both majors.
        let over_2d = [call(2, Strain::Diamonds)];
        let (c, floored) = leaping(true, &over_2d, "AKQ65.KQJ76.4.32");
        assert_eq!(c, call(4, Strain::Diamonds));
        assert!(!floored, "Leaping Michaels must come from the book node");
    }

    #[test]
    fn leaping_michaels_advancer_picks_the_major_game() {
        // (2♥)–4♣–(P): partner shows clubs + spades. With spade support the
        // advancer bids the 4♠ game; with none, the 5♣ minor game (never pass 4♣).
        let auction = [call(2, Strain::Hearts), call(4, Strain::Clubs), Call::Pass];
        let (fit, floored) = leaping(true, &auction, "KQ7.32.J865.A432");
        assert_eq!(fit, call(4, Strain::Spades));
        assert!(!floored, "the advance must come from the book node");

        // A doubleton (7-card fit) still takes the 4♠ game — it scores well and
        // needs only ten tricks.
        let (thin, _) = leaping(true, &auction, "K7.QJ32.8654.A32");
        assert_eq!(thin, call(4, Strain::Spades));

        // A genuine major misfit (≤1) retreats to the 5♣ game, not a passed 4♣.
        let (no_fit, _) = leaping(true, &auction, "2.QJ32.J8654.KQ4");
        assert_eq!(no_fit, call(5, Strain::Clubs));
    }

    #[test]
    fn leaping_michaels_advancer_picks_longer_major_over_2d_cue() {
        // (2♦)–4♦–(P): the cue shows both majors; advancer picks the longer.
        let auction = [
            call(2, Strain::Diamonds),
            call(4, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, floored) = leaping(true, &auction, "AQ32.K8.654.9432");
        assert_eq!(c, call(4, Strain::Spades));
        assert!(!floored, "the advance must come from the book node");
    }

    #[test]
    fn leaping_michaels_2d_4c_pass_or_correct() {
        // (2♦)–4♣–(P): clubs + an unknown major → 4♥ pass-or-correct, then the
        // overcaller with spades corrects to 4♠.
        let advance = [
            call(2, Strain::Diamonds),
            call(4, Strain::Clubs),
            Call::Pass,
        ];
        let (relay, _) = leaping(true, &advance, "K32.A87.9654.J32");
        assert_eq!(relay, call(4, Strain::Hearts));

        let rebid = [
            call(2, Strain::Diamonds),
            call(4, Strain::Clubs),
            Call::Pass,
            call(4, Strain::Hearts),
            Call::Pass,
        ];
        let (correct, _) = leaping(true, &rebid, "AKQ65.4.32.KQJ76");
        assert_eq!(correct, call(4, Strain::Spades));
    }

    #[test]
    fn leaping_michaels_silent_when_disabled() {
        // Turned off: the same club-spade two-suiter never jumps to 4♣ (the
        // escape hatch back to the pre-Leaping-Michaels weak-two defense).
        let over_2h = [call(2, Strain::Hearts)];
        let (c, _) = leaping(false, &over_2h, "AKQ65.4.32.KQJ76");
        assert_ne!(c, call(4, Strain::Clubs));
    }
}
