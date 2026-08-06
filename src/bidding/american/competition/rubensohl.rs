//! Transfer Lebensohl (Rubensohl) — Larry Cohen's version
//!
//! Responder bids the next suit *up* through the adverse suit, so opener always
//! declares.  Includes the `(2♦)` Multi case, where `3♣` is game-forcing Stayman
//! with Smolen behind it, and the Leaping Michaels advances.
//! [`Competitive4333`] gates whether a flat 4333 takes the transfer.

use super::cue_raise::delayed_cue;
use super::lebensohl::{lebensohl_relay_shape, unbid_major};
use super::over_overcall::{author_direct_3nt, natural_floor_hcp, natural_floor_pts};
use super::penalty_double::responder_double;
use super::*;

/// How responder treats a flat 4-3-3-3 when our 1NT opening is overcalled.
///
/// The constructive 4333 rule (a flat hand plays 3NT, not the major fit, for want
/// of a ruffing value — see `notrump::flat_4333`) was unclear in competition: a
/// stopperless flat 4333 might *need* the 4-4 fit to escape a 3NT it cannot make.
/// A paired BBA A/B settled it — full [`Suppress`][Competitive4333::Suppress] of
/// the Transfer-Lebensohl cue-Stayman and the `3♣`-over-`(2♦)` Stayman beat both
/// `Allow` and the stopper-only middle on plain *and* PD double-dummy (960k boards
/// vul none, 63 fired: PD **+3.8 IMPs/fired**, +0.0002/board with the 95% CI
/// excluding 0; plain a wash-to-win at +1.3/fired).  Even the stopperless flat 4333
/// does better staying low than digging out a no-ruffing-value fit that gets
/// doubled.  **Default [`Suppress`][Competitive4333::Suppress]**; the other modes
/// stay for re-measurement (e.g. at vul both).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Competitive4333 {
    /// Cue-Stayman unchanged on a flat 4333 — the old behaviour / A/B baseline.
    Allow,
    /// Never cue-Stayman on a flat 4333; play 3NT (or a natural call) instead.
    Suppress,
    /// Suppress only a flat 4333 *with* a stopper in their suit (3NT is safe); a
    /// stopperless 4333 may still cue to dig out the 4-4 fit.
    SuppressWithStopper,
}

thread_local! {
    static COMPETITIVE_4333: Cell<Competitive4333> =
        const { Cell::new(Competitive4333::Suppress) };
}

/// Set how a flat 4-3-3-3 cue-Staymans when our 1NT is overcalled, for books
/// built *after* this call (thread-local; default [`Competitive4333::Suppress`]).
pub fn set_competitive_4333(mode: Competitive4333) {
    COMPETITIVE_4333.with(|cell| cell.set(mode));
}

/// The active [`Competitive4333`] mode
fn competitive_4333() -> Competitive4333 {
    COMPETITIVE_4333.with(Cell::get)
}

/// Gate ANDed into each competitive cue-Stayman rule: satisfied unless the active
/// [`Competitive4333`] mode diverts this flat 4-3-3-3 to 3NT.  Four suits all 3
/// or 4 cards long sum to 13 only as a 4-3-3-3, so that test *is* "flat 4333".
///
/// `gate` is true only in the 1NT-overcall context, where partner is a *balanced*
/// 1NT opener and a flat 4333 has no ruffing value anywhere.  When advancing a
/// takeout double (`gate = false`) partner is *short* in their suit, so the 4-4
/// fit keeps its ruffing value and the cue is never diverted — the curse does not
/// apply, and that A/B was never run.
fn competitive_4333_ok(over: Suit, gate: bool) -> Cons<impl Constraint + Clone> {
    let mode = if gate {
        competitive_4333()
    } else {
        Competitive4333::Allow
    };
    described(
        "not a flat 4-3-3-3 diverted to 3NT",
        move |hand: Hand, _: &Context<'_>| {
            let flat = Suit::ASC
                .into_iter()
                .all(|suit| (3..=4).contains(&hand[suit].len()));
            !match mode {
                Competitive4333::Allow => false,
                Competitive4333::Suppress => flat,
                Competitive4333::SuppressWithStopper => flat && has_stopper(hand[over]),
            }
        },
    )
}

/// The suit a 3-level Transfer-Lebensohl bid in `bid_suit` shows, given the
/// opponents' 2-level overcall in `over`
///
/// The cheapest suit strictly above `bid_suit` that is *not* their suit — a
/// transfer *through* the adverse suit. `None` when `bid_suit` is their suit
/// (that bid is the Stayman cue, not a transfer) or no higher suit remains
/// (the lowest target, clubs, has no dedicated transfer — those rare hands use
/// the `2NT` relay or `3NT`).
pub(crate) fn transfer_target(bid_suit: Suit, over: Suit) -> Option<Suit> {
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
pub(crate) fn transfer_lebensohl_responder(over: Suit, gate_4333: bool) -> Rules {
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
                (Suit::Hearts, true) => rules
                    .rule(
                        cue,
                        170,
                        len(Suit::Spades, 4..)
                            & points(10..)
                            & !stopper_in(over)
                            & competitive_4333_ok(over, gate_4333),
                    )
                    .alert(LEBENSOHL_CUE),
                (Suit::Spades, true) => rules
                    .rule(
                        cue,
                        170,
                        len(Suit::Hearts, 4..)
                            & points(10..)
                            & !stopper_in(over)
                            & competitive_4333_ok(over, gate_4333),
                    )
                    .alert(LEBENSOHL_CUE),
                (Suit::Hearts, false) => rules
                    .rule(
                        cue,
                        170,
                        len(Suit::Spades, 4..)
                            & points(10..)
                            & competitive_4333_ok(over, gate_4333),
                    )
                    .alert(LEBENSOHL_CUE),
                (Suit::Spades, false) => rules
                    .rule(
                        cue,
                        170,
                        len(Suit::Hearts, 4..)
                            & points(10..)
                            & competitive_4333_ok(over, gate_4333),
                    )
                    .alert(LEBENSOHL_CUE),
                _ => rules
                    .rule(
                        cue,
                        170,
                        (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..))
                            & points(10..)
                            & competitive_4333_ok(over, gate_4333),
                    )
                    .alert(LEBENSOHL_CUE),
            };
        } else if let Some(target) = transfer_target(bid_suit, over) {
            // Transfer: show 5+ in the target, invitational or better. A major
            // target outranks the cue so a 5-card major is shown by the
            // transfer, not Stayman; a minor target is rare (long minor, no
            // stopper) and yields to Stayman / 3NT.
            let weight = if matches!(target, Suit::Hearts | Suit::Spades) {
                180
            } else {
                145
            };
            rules = rules
                .rule(Bid::new(3, strain), weight, len(target, 5..) & points(9..))
                .alert(LEBENSOHL_TRANSFER);
        } else if over != Suit::Clubs {
            // Top step (no suit above to transfer into): a *forced* game-force
            // transfer to clubs, 6+♣. Its completion lands at game, so 3♣ can
            // never be the contract — the only forcing long-club route (the
            // 2NT→3♣ relay is the *weak* one). Weight below 3NT's 1.5 so a 6♣
            // hand *with* a stopper picks 3NT; only no-stopper hands transfer.
            // (Over (2♣) clubs is their suit — there is no top-step transfer.)
            rules = rules
                .rule(
                    Bid::new(3, strain),
                    145,
                    len(Suit::Clubs, 6..) & points(10..),
                )
                .alert(LEBENSOHL_TRANSFER);
        }
    }

    // Direct 3NT to play: game values with their suit stopped, no major to show
    // (toggles: drop the stopper requirement, and/or trap-pass with 4+ in their
    // suit — long-in-their-suit defends better than it declares).
    rules = author_direct_3nt(rules, 150, over);

    // Stopper-split on: a GF hand with a stopper *and* exactly a 4-card unbid
    // major relays through 2NT to bid the cue *slowly* (Stayman with a stopper,
    // see [`lebensohl_relay_rebid`]) — outweighing direct 3NT (1.5) so the 4-4
    // major fit is still found. Denies a 5-card major (Smolen / Leaping Michaels).
    if let (true, Some(major)) = (delayed_cue(), unbid_major(over)) {
        rules = rules
            .rule(
                Bid::new(2, Strain::Notrump),
                160,
                points(10..) & stopper_in(over) & len(major, 4..) & len(major, ..5),
            )
            .alert(LEBENSOHL_RELAY);
    }

    // Responder's double of their overcall (penalty by default; see
    // [`DoubleStyle`]). Authoring it is also what kept the floor's penalty
    // doubles — the Rubensohl-v1 attempt lost them by shadowing with no double.
    rules = responder_double(rules, over);

    // Natural new suit at the 2 level (above the overcall, below 2NT): weak.
    for s in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if s == over {
            continue;
        }
        let strain = Strain::from(s);
        rules = rules.rule(
            Bid::new(2, strain),
            140,
            min_level_is(2, strain)
                & len(s, 5..)
                & points(..=8)
                & hcp(natural_floor_hcp()..)
                & points(natural_floor_pts()..),
        );
    }

    // 2NT = Lebensohl relay to 3♣: a weak long-suit hand (sign off or correct),
    // same shape as plain Lebensohl (see [`lebensohl_relay_shape`] — 6+ suit, or
    // a 5-carder with the PD-distilled 6-HCP floor, never their suit).
    let long_suit = lebensohl_relay_shape(over);
    rules = rules
        .rule(Bid::new(2, Strain::Notrump), 135, points(..=8) & long_suit)
        .alert(LEBENSOHL_RELAY);

    // Pass — weak, nothing constructive to say.
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Opener's reply after responder's Transfer-Lebensohl transfer to `target`
///
/// A transfer to a major is INV+, so opener is driven to **game**: `4M` with a
/// fit, else `3NT`. A transfer to a minor (rare — long minor, no stopper) is
/// completed at the 3 level, or `3NT` with a stopper; responder drives on.
pub(crate) fn transfer_completion(target: Suit, over: Suit) -> Rules {
    let t = Strain::from(target);
    let mut rules = Rules::new();
    if matches!(target, Suit::Hearts | Suit::Spades) {
        rules = rules.rule(Bid::new(4, t), 160, len(target, 3..)).rule(
            Bid::new(3, Strain::Notrump),
            140,
            len(target, ..3),
        );
    } else {
        // ponytail: minor-target 5m / slam exploration is left to the floor;
        // 3NT-or-complete covers the common game. Author it if the A/B shows
        // minor transfers matter.
        rules = rules
            .rule(Bid::new(3, Strain::Notrump), 150, stopper_in(over))
            .rule(Bid::new(3, t), 130, len(target, 3..));
    }
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Opener's reply to responder's Transfer-Lebensohl cue (Stayman, game-forcing)
///
/// Shows a 4-card unbid major at its cheapest legal level, else `3NT`.
pub(crate) fn cue_stayman_answer(over: Suit) -> Rules {
    let mut rules = Rules::new();
    for major in [Suit::Hearts, Suit::Spades] {
        if major == over {
            continue;
        }
        let m = Strain::from(major);
        rules = rules
            .rule(Bid::new(3, m), 160, len(major, 4..) & min_level_is(3, m))
            .rule(Bid::new(4, m), 150, len(major, 4..) & min_level_is(4, m));
    }
    // No 4-card unbid major → 3NT (always legal above the 3-level cue).
    rules.rule(Bid::new(3, Strain::Notrump), 130, hcp(0..))
}

/// Answerer's reply to the *direct* (no-stopper) cue under the stopper-split
///
/// The cuer denied a stopper in their suit, so 3NT needs *our* own stopper;
/// without it (and without a 4-card-major fit) we run to a minor-suit game
/// rather than a stopperless 3NT. A 4-card unbid major is shown first (the fit).
/// The trailing low-weight 3NT is a guaranteed-finite catch-all (it never wins
/// against the minors, but keeps the node from silently passing the game force).
pub(crate) fn cue_stayman_answer_no_stopper(over: Suit) -> Rules {
    let mut rules = Rules::new();
    for major in [Suit::Hearts, Suit::Spades] {
        if major == over {
            continue;
        }
        let m = Strain::from(major);
        rules = rules
            .rule(Bid::new(3, m), 160, len(major, 4..) & min_level_is(3, m))
            .rule(Bid::new(4, m), 150, len(major, 4..) & min_level_is(4, m));
    }
    // 3NT only with our own stopper (the cuer has none).
    rules = rules.rule(Bid::new(3, Strain::Notrump), 145, stopper_in(over));
    // No fit, no stopper → minor-suit game.
    for minor in [Suit::Clubs, Suit::Diamonds] {
        let m = Strain::from(minor);
        rules = rules.rule(Bid::new(4, m), 120, len(minor, 4..) & min_level_is(4, m));
    }
    // Guaranteed-finite catch-all (rare: no major, no stopper, no 4-card minor).
    rules.rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
}

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
pub(crate) fn transfer_stayman_2d_responder(gate_4333: bool) -> Rules {
    let mut rules = Rules::new();

    // 3♣ = Stayman: game-forcing with *exactly* a 4-card major. A single 5-card
    // major transfers instead; a 5-4 GF hand has its 4-card major here and so comes
    // to Stayman (for Smolen) — hence weight above the transfers, which it also fits.
    rules = rules
        .rule(
            Bid::new(3, Strain::Clubs),
            185,
            (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4))
                & points(10..)
                & competitive_4333_ok(Suit::Diamonds, gate_4333),
        )
        .alert(STAYMAN);

    // Direct Jacoby transfers above their suit (INV+, auto-driven to game).
    rules = rules
        .rule(
            Bid::new(3, Strain::Diamonds),
            180,
            len(Suit::Hearts, 5..) & points(9..),
        )
        .alert(LEBENSOHL_TRANSFER)
        .rule(
            Bid::new(3, Strain::Hearts),
            180,
            len(Suit::Spades, 5..) & points(9..),
        )
        .alert(LEBENSOHL_TRANSFER);

    // 3♠→clubs: a *forced* game-force with 6+ clubs (its completion is 4♣, so 3♣
    // can never be the contract). Weight below 3NT's, so a 6-club hand *with* a
    // diamond stopper picks 3NT; only the no-stopper hands transfer.
    rules = rules
        .rule(
            Bid::new(3, Strain::Spades),
            145,
            len(Suit::Clubs, 6..) & points(10..),
        )
        .alert(LEBENSOHL_TRANSFER);

    // Leaping Michaels: 5-5 game-forcing two-suiters.
    rules = rules
        .rule(
            Bid::new(4, Strain::Diamonds),
            200,
            len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & points(10..),
        )
        .alert(LEAPING_MICHAELS)
        .rule(
            Bid::new(4, Strain::Clubs),
            200,
            len(Suit::Clubs, 5..)
                & (len(Suit::Hearts, 5..) | len(Suit::Spades, 5..))
                & points(10..),
        )
        .alert(LEAPING_MICHAELS);

    // Weak / to-play outlets — identical to `transfer_lebensohl_responder(Diamonds)`.
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        150,
        points(10..) & stopper_in(Suit::Diamonds),
    );
    rules = responder_double(rules, Suit::Diamonds);
    for s in [Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(s);
        rules = rules.rule(
            Bid::new(2, strain),
            140,
            min_level_is(2, strain)
                & len(s, 5..)
                & points(..=8)
                & hcp(natural_floor_hcp()..)
                & points(natural_floor_pts()..),
        );
    }
    // Relay shape: 6+ suit, or a 5-carder with the PD-distilled 6-HCP floor,
    // never their diamonds (see [`lebensohl_relay_shape`]).
    let long_suit = lebensohl_relay_shape(Suit::Diamonds);
    rules = rules
        .rule(Bid::new(2, Strain::Notrump), 135, points(..=8) & long_suit)
        .alert(LEBENSOHL_RELAY);

    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer to `3♣` Stayman over `(2♦)`: a 4-card major, else `3♦`
///
/// `3♥`/`3♠` shows a 4-card major (hearts first when both); `3♦` denies one,
/// leaving `3♥`/`3♠` free for responder's Smolen. `3♦` is the finite catch-all.
pub(crate) fn stayman_2d_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 160, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(3, Strain::Spades),
            155,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(Bid::new(3, Strain::Diamonds), 50, hcp(0..))
}

/// Responder's rebid after opener shows a 4-card major over `3♣` Stayman
///
/// Game-forcing already: raise the shown major to game with 4-card support (an
/// eight-card fit), else settle in `3NT` (the finite catch-all).
pub(crate) fn stayman_2d_fit_rebid(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 140, len(major, 4..))
        .rule(Bid::new(3, Strain::Notrump), 50, hcp(0..))
}

/// Opener's completion of the top-step→clubs transfer (a forced game-force)
///
/// Responder has 6+ clubs, no stopper in `over`, game values. Opener bids `3NT`
/// with a stopper of its own, else raises to `5♣` — `3♣` is unplayable below the
/// top step, so the auction must reach game. (`5♣` is the finite catch-all.)
//
// ponytail: minor-suit slam exploration is left to the floor; 3NT-or-5♣ covers
// the common game. Author a keycard ladder here only if the A/B shows it matters.
pub(crate) fn clubs_transfer_completion(over: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 140, stopper_in(over))
        .rule(Bid::new(5, Strain::Clubs), 50, hcp(0..))
}

/// Opener's reply to Leaping Michaels `4♦` (both majors, 5-5 game-forcing)
///
/// Bid game in the better major fit, preferring the nine-card fit (4-card
/// support) and breaking ties toward spades. `4♥` is the finite catch-all.
pub(crate) fn lm_2d_both_majors_advance() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Spades), 160, len(Suit::Spades, 4..))
        .rule(Bid::new(4, Strain::Hearts), 155, len(Suit::Hearts, 4..))
        .rule(Bid::new(4, Strain::Spades), 150, len(Suit::Spades, 3..))
        .rule(Bid::new(4, Strain::Hearts), 100, hcp(0..))
}

/// Opener's reply to Leaping Michaels `4♣` (clubs + an unknown 5+ major)
///
/// `4♦` asks which major; responder names it in [`lm_2d_clubs_major`].
//
// ponytail: opener always relays — the major usually outplays 5♣, and opener's
// final placement (pass the major / correct to 5♣) is left to the floor. Add a
// direct 5♣ sign-off only if the A/B shows the relay costs.
pub(crate) fn lm_2d_clubs_ask() -> Rules {
    Rules::new().rule(Bid::new(4, Strain::Diamonds), 140, hcp(0..))
}

/// Responder names the 5+ major behind a `4♣` Leaping Michaels, over the `4♦` ask
pub(crate) fn lm_2d_clubs_major() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Hearts), 150, len(Suit::Hearts, 5..))
        .rule(Bid::new(4, Strain::Spades), 150, len(Suit::Spades, 5..))
        .rule(Bid::new(5, Strain::Clubs), 50, hcp(0..))
}

#[cfg(test)]
mod tests;
