//! Transfer Lebensohl (Rubensohl) — Larry Cohen's version
//!
//! Responder bids the next suit *up* through the adverse suit, so opener always
//! declares.  Includes the `(2♦)` Multi case, where `3♣` is game-forcing Stayman
//! with Smolen behind it, and the Leaping Michaels advances.
//! [`Competitive4333`] gates whether a flat 4333 takes the transfer.

use super::lebensohl::{author_direct_3nt, natural_floor_hcp, natural_floor_pts};
use super::lebensohl::{lebensohl_relay_shape, unbid_major};
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

/// Responder's stopper ask after a disclosed Multi has corrected to spades.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MultiStopperAsk {
    /// Shipped default: no `3♠` ask.
    Off,
    /// Opener names its longest side suit; responder searches for the game.
    FitSearch,
    /// Opener immediately places the contract in `4♥` or five of a minor.
    OpenerPlaces,
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
fn competitive_4333_ok(
    over: Suit,
    gate: bool,
    agreements: &Agreements,
) -> Cons<impl Constraint + Clone + use<>> {
    let mode = if gate {
        agreements.competition.competitive_4333
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
pub(crate) fn transfer_lebensohl_responder(
    over: Suit,
    gate_4333: bool,
    agreements: &Agreements,
) -> Rules {
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
            let split = agreements.competition.delayed_cue && unbid_major(over).is_some();
            rules = match (over, split) {
                (Suit::Hearts, true) => rules
                    .rule(
                        cue,
                        170,
                        len(Suit::Spades, 4..)
                            & points(10..)
                            & !stopper_in(over)
                            & competitive_4333_ok(over, gate_4333, agreements),
                    )
                    .alert(LEBENSOHL_CUE),
                (Suit::Spades, true) => rules
                    .rule(
                        cue,
                        170,
                        len(Suit::Hearts, 4..)
                            & points(10..)
                            & !stopper_in(over)
                            & competitive_4333_ok(over, gate_4333, agreements),
                    )
                    .alert(LEBENSOHL_CUE),
                (Suit::Hearts, false) => rules
                    .rule(
                        cue,
                        170,
                        len(Suit::Spades, 4..)
                            & points(10..)
                            & competitive_4333_ok(over, gate_4333, agreements),
                    )
                    .alert(LEBENSOHL_CUE),
                (Suit::Spades, false) => rules
                    .rule(
                        cue,
                        170,
                        len(Suit::Hearts, 4..)
                            & points(10..)
                            & competitive_4333_ok(over, gate_4333, agreements),
                    )
                    .alert(LEBENSOHL_CUE),
                _ => rules
                    .rule(
                        cue,
                        170,
                        (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..))
                            & points(10..)
                            & competitive_4333_ok(over, gate_4333, agreements),
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
    rules = author_direct_3nt(rules, 150, over, agreements);

    // Stopper-split on: a GF hand with a stopper *and* exactly a 4-card unbid
    // major relays through 2NT to bid the cue *slowly* (Stayman with a stopper,
    // see [`lebensohl_relay_rebid`]) — outweighing direct 3NT (1.5) so the 4-4
    // major fit is still found. Denies a 5-card major (Smolen / Leaping Michaels).
    if let (true, Some(major)) = (agreements.competition.delayed_cue, unbid_major(over)) {
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
    rules = responder_double(rules, over, agreements);

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
                & hcp(natural_floor_hcp(agreements)..)
                & points(natural_floor_pts(agreements)..),
        );
    }

    // 2NT = Lebensohl relay to 3♣: a weak long-suit hand (sign off or correct),
    // same shape as plain Lebensohl (see [`lebensohl_relay_shape`] — any 5-card
    // suit but theirs, with the PD-distilled 6-HCP floor).
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
pub(crate) fn transfer_completion(target: Suit, over: Suit, agreements: &Agreements) -> Rules {
    let completion_alerts = agreements.decision.reading.completion_alerts;
    let t = Strain::from(target);
    let mut rules = Rules::new();
    if matches!(target, Suit::Hearts | Suit::Spades) {
        rules = rules
            .rule(Bid::new(4, t), 160, len(target, 3..))
            .alert_if(completion_alerts, COMPLETION)
            .rule(Bid::new(3, Strain::Notrump), 140, len(target, ..3))
            .alert_if(completion_alerts, COMPLETION);
    } else {
        // ponytail: minor-target 5m / slam exploration is left to the floor;
        // 3NT-or-complete covers the common game. Author it if the A/B shows
        // minor transfers matter.
        rules = rules
            .rule(Bid::new(3, Strain::Notrump), 150, stopper_in(over))
            .alert_if(completion_alerts, COMPLETION)
            .rule(Bid::new(3, t), 130, len(target, 3..))
            .alert_if(completion_alerts, COMPLETION);
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

/// The constructive half of responder's `(2♦)` table, shared by the natural
/// and Multi legs: `3♣` Stayman, the direct Jacoby transfers, the forced
/// `3♠`→♣ game-force, and Leaping Michaels
///
/// `2♦` leaves `3♣` free below the cue, so Stayman moves there (with Smolen after
/// opener's `3♦` denial) and the transfers shift down to direct Jacoby: `3♦`→♥,
/// `3♥`→♠, `3♠`→♣. The major transfers are INV+ and auto-driven to game by
/// [`transfer_completion`]; the `3♠`→♣ leg is a *forced* game-force (its completion
/// is `4♣`, so `3♣` is unplayable). Leaping Michaels `4♦` (both majors) and `4♣`
/// (clubs + a major) show 5-5 game-forcing two-suiters — partner opened `1NT`, so
/// `points(10..)` (≈ 8 HCP after the 5-5 upgrade) already forces game.
fn stayman_2d_constructive(mut rules: Rules, gate_4333: bool, agreements: &Agreements) -> Rules {
    // 3♣ = Stayman: game-forcing with *exactly* a 4-card major. A single 5-card
    // major transfers instead; a 5-4 GF hand has its 4-card major here and so comes
    // to Stayman (for Smolen) — hence weight above the transfers, which it also fits.
    rules = rules
        .rule(
            Bid::new(3, Strain::Clubs),
            185,
            (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4))
                & points(10..)
                & competitive_4333_ok(Suit::Diamonds, gate_4333, agreements),
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

    leaping_michaels_2d(rules)
}

/// Leaping Michaels over their `(2♦)`: `4♦` both majors, `4♣` clubs plus an
/// unknown major, both 5-5 game-forcing
///
/// Its own function because three responder tables carry it verbatim — the
/// natural leg, the N4 Multi leg (both via [`stayman_2d_constructive`]) and
/// the Kokish–Kraft variant ([`kokish_kraft_responder`]), which shares no
/// other constructive rung.  Partner opened `1NT`, so `points(10..)` (≈ 8 HCP
/// after the 5-5 upgrade) already forces game.
fn leaping_michaels_2d(rules: Rules) -> Rules {
    rules
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
        .alert(LEAPING_MICHAELS)
}

/// The weak natural major escapes at the two level, shared by both `(2♦)` legs
///
/// `weak_len` is the Multi lane's floorless rung
/// ([`CompetitionKnobs::multi_weak_escape`]): a suit that long is itself
/// evidence their Multi is the *other* major, so it may act with no HCP floor.
/// `None` — always, on the natural leg — keeps the shipped single rung, so the
/// default system stays byte-identical.
fn natural_major_escapes(mut rules: Rules, agreements: &Agreements, weak_len: Option<u8>) -> Rules {
    for s in [Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(s);
        let floored = len(s, 5..)
            & hcp(natural_floor_hcp(agreements)..)
            & points(natural_floor_pts(agreements)..);
        let head = min_level_is(2, strain) & points(..=8);
        rules = match weak_len {
            None => rules.rule(Bid::new(2, strain), 140, head & floored),
            Some(n) => rules.rule(
                Bid::new(2, strain),
                140,
                head & (floored | len(s, usize::from(n)..)),
            ),
        };
    }
    rules
}

/// Responder's action after our `1NT` and a `(2♦)` overcall, the `(2♦)`-only
/// Smolen leg of the [`LebensohlStyle::Transfer`] package
///
/// The constructive calls are [`stayman_2d_constructive`]; the weak outlets
/// (natural 2-level, `2NT` relay, penalty double, direct `3NT`) match
/// `Transfer` so the A/B isolates the constructive change.  Every gate here
/// keys on *diamonds* as their suit — the Multi leg
/// ([`multi_2d_responder`]) is the same table with those gates re-keyed.
pub(crate) fn transfer_stayman_2d_responder(gate_4333: bool, agreements: &Agreements) -> Rules {
    let mut rules = stayman_2d_constructive(Rules::new(), gate_4333, agreements);

    // Weak / to-play outlets — identical to `transfer_lebensohl_responder(Diamonds)`.
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        150,
        points(10..) & stopper_in(Suit::Diamonds),
    );
    // The double.  Armed, `two_diamond_double` replaces the cooperative
    // [`DoubleStyle`] gate with a real diamond penalty double — the only channel
    // responder has for a diamond suit here, since `3♦` is the heart transfer.
    // Same weight, so nothing else in the table re-ranks.
    //
    // Alerted: `project_authored` decodes alerted calls only, and measured on
    // `probe-call-reading "1N (2D) X -"` the unalerted rule read as `points 8..`
    // with every suit ⊤ — the length and quality never reached opener.  The alert
    // is what turns the gate into a reading.
    rules = match agreements.competition.two_diamond_double {
        Some((min_len, min_suit_hcp, floor)) => rules
            .rule(
                Call::Double,
                155,
                len(Suit::Diamonds, min_len..)
                    & suit_hcp(Suit::Diamonds, min_suit_hcp..)
                    & hcp(floor..),
            )
            .alert(TWO_DIAMOND_PENALTY),
        None => responder_double(rules, Suit::Diamonds, agreements),
    };
    rules = natural_major_escapes(rules, agreements, None);
    // Relay shape: any 5-card suit but their diamonds, with the PD-distilled
    // 6-HCP floor (see [`lebensohl_relay_shape`]).
    let long_suit = lebensohl_relay_shape(Suit::Diamonds);
    rules = rules
        .rule(Bid::new(2, Strain::Notrump), 135, points(..=8) & long_suit)
        .alert(LEBENSOHL_RELAY);

    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Responder's action after our `1NT` and their `(2♦)` **Multi** — one
/// unknown six-card major (`their.two_diamonds_multi`, campaign package N4)
///
/// [`stayman_2d_constructive`] verbatim — the fits it hunts are in the major
/// they do *not* hold — with every diamond-keyed gate re-keyed, because their
/// `2♦` names a suit nobody holds:
///
/// - `X` = invitational-plus values (`hcp 8+`), no diamond claim: the waiting
///   call.  They will name the major (their advancer answers pass-or-correct;
///   over the double they sit 43% of the time), and opener sits or doubles by
///   what they show — see [`multi_penalty_answer`].  BBA's own counter doubles
///   at 5+ (41% of hands); 8+ is the invitational floor opposite 15-17.
/// - `3NT` = game values with **both majors** stopped — the blast that needs no
///   more information.  A game hand with a major open doubles and places after
///   they name the suit ([`multi_responder_rebid`]).  v2/v3 blasted
///   unconditionally: plain DD liked it, perfect defense did not (−3.7/−4.3 a
///   board on the ex-doublers) — the stopperless-3NT DD fragility.
/// - The `2NT` relay adds diamonds to its shape ([`multi_relay_shape`]) and a
///   natural `3♦` sign-off after opener's `3♣` ([`multi_relay_rebid`]).
/// - `two_diamond_double` is ignored — a diamond penalty double of a Multi is
///   the gate N4b measured null.
pub(crate) fn multi_2d_responder(gate_4333: bool, agreements: &Agreements) -> Rules {
    let mut rules = stayman_2d_constructive(Rules::new(), gate_4333, agreements);
    rules = rules
        // v4: back to the both-majors gate for the *direct* blast — the v2/v3
        // unconditional 3NT was the package's one PD-negative rung (−3.7/−4.3
        // per board vs perfect defense on the ex-doublers, +1.6 plain: the
        // DD-fragile stopperless game).  A game hand with a major open doubles
        // and places once they name the suit ([`multi_responder_rebid`]) —
        // the v1 idea, now with the second call authored instead of floored.
        .rule(
            Bid::new(3, Strain::Notrump),
            150,
            points(10..) & stopper_in(Suit::Hearts) & stopper_in(Suit::Spades),
        );
    let weak_len = agreements.competition.multi_weak_escape;
    rules = natural_major_escapes(rules, agreements, weak_len);
    let relay = Bid::new(2, Strain::Notrump);
    rules = match weak_len {
        None => rules.rule(relay, 135, points(..=8) & multi_relay_shape()),
        Some(n) => rules.rule(
            relay,
            135,
            points(..=8) & (multi_relay_shape() | multi_any_len(usize::from(n))),
        ),
    };
    rules = rules
        .alert(LEBENSOHL_RELAY)
        // v6 (BBA mimic): BBA's `hcp 5–17` values double, the 41% workhorse
        // of its counter — 6 is where its Pass bucket (median 5) ends.  Below
        // the natural 2M (140) and the relay (135) so a weak hand with a 5+
        // suit still escapes or relays; every placing call above it places.
        // What the double does next is [`multi_responder_rebid`].
        .rule(Call::Double, 130, hcp(6..))
        .alert(MULTI_VALUES);
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// The `2NT`-relay shape over their Multi: a 5+ suit in *any* suit with the
/// same PD-distilled 6-HCP floor as [`lebensohl_relay_shape`] — diamonds are
/// ours to sign off in, since their `2♦` never held them
///
/// This is the only outlet a long *minor* or diamond suit has — the natural
/// escape is majors-only — so under `multi_weak_escape` the caller unions
/// [`multi_any_len`] onto it and a 6-card club suit below the floor gets a
/// call instead of a pass.
fn multi_relay_shape() -> Cons<impl Constraint + Clone> {
    multi_any_len(5) & hcp(6..)
}

/// A suit — any suit — at least `n` long
fn multi_any_len(n: usize) -> Cons<impl Constraint + Clone> {
    len(Suit::Clubs, n..)
        | len(Suit::Diamonds, n..)
        | len(Suit::Hearts, n..)
        | len(Suit::Spades, n..)
}

/// Responder's rebid after `1NT (2♦ Multi) 2NT - 3♣ -`: pass with clubs, or a
/// natural sign-off in a 5+ suit — `3♦` included, the one rung the natural
/// leg's [`lebensohl_relay_rebid`] cannot have.  Opener then passes
/// ([`multi_signoff_pass`]): the first A/B left that seat to the floor, which
/// raised the weak `3♦` to `3NT` on 45 of 52 relay boards.
pub(crate) fn multi_relay_rebid() -> Rules {
    let mut rules = Rules::new();
    for s in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(s);
        rules = rules.rule(
            Bid::new(3, strain),
            100,
            min_level_is(3, strain) & len(s, 5..),
        );
    }
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Opener passes responder's weak relay sign-off — a total table on purpose:
/// responder showed at most eight, and the floor's alternative was `3NT`
pub(crate) fn multi_signoff_pass() -> Rules {
    Rules::new().rule(Call::Pass, 0, hcp(0..))
}

/// Responder's second call once their pass-or-correct has resolved the major
/// (`1NT (2♦) X (2♥) - -`, `… X (2♥) - (2♠)`, `… X (2♠) - -`, `… X (2♥) X (2♠)`)
///
/// `major` is their resolved suit; `ran` is the `X (2♥) - (2♠)` shape — the
/// weak advancer's pass-or-correct corrected to spades.  v7: BBA's own
/// second-turn structure (probed at `counter-d-x2h`, `counter-d-x2h2s`,
/// `counter-d-x2s`) **minus the rungs perfect defense refused** when v6
/// mimicked it whole (docs/one-notrump-competitive.md §N4 v6):
///
/// - `4NT` — quantitative, `hcp 16+` (BBA 16–21).
/// - `2♠` (hearts resolved only) — five spades, `hcp 6–8`, to play.
/// - `X` — **takeout showing the other major**: BBA's X here is exactly four
///   of the other major and 1–2 of theirs, `hcp 6–17`, its label "reopening
///   double"; the one BBA rung that measured positive on *both* scorers
///   (v6 vs v4: +2.4 plain / +1.6 PD per fired NV).  In the `ran` shape it is
///   spade length instead (BBA: 3–5, median 4) — penalty.
/// - `3NT` — game values **with a stopper in the resolved major**, v4's gate.
///   BBA's `3NT` is the bare `hcp 9–15`; v6 played it and perfect defense
///   refused it at every seat (−2.5 to −4.6 per fired), the same DD-declarer
///   artifact v2/v3 measured.
/// - Pass — the rest.  BBA's `2NT` invite (8–9, −3.0/−6.9 PD per invite),
///   its `3♠` try (−2.3/−3.6) and its natural `3m` (7–8, a wash) were in v6
///   and are not here; the 8-9 hand sells out, as v5 already established.
pub(crate) fn multi_responder_rebid(major: Suit, ran: bool, stopper_ask: MultiStopperAsk) -> Rules {
    let other = if major == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    let mut rules = Rules::new().rule(Bid::new(4, Strain::Notrump), 160, hcp(16..));
    if ran && stopper_ask != MultiStopperAsk::Off {
        rules = rules
            .rule(
                Bid::new(3, Strain::Spades),
                158,
                points(10..=12) & len(Suit::Spades, ..=3) & !stopper_in(Suit::Spades),
            )
            .alert(MULTI_STOPPER_ASK);
    }
    if !ran && major == Suit::Hearts {
        // Above the takeout double: BBA's X is exactly four spades, its 2♠
        // five weak ones.
        rules = rules.rule(
            Bid::new(2, Strain::Spades),
            156,
            len(Suit::Spades, 5..) & hcp(..=8),
        );
    }
    if ran {
        rules = rules
            .rule(Call::Double, 155, len(Suit::Spades, 4..) & hcp(7..))
            .alert(MULTI_PENALTY);
    } else {
        rules = rules
            .rule(Call::Double, 155, len(other, 4..) & len(major, ..=2))
            .alert(MULTI_TAKEOUT);
    }
    rules
        .rule(
            Bid::new(3, Strain::Notrump),
            150,
            points(10..) & stopper_in(major),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer to the Multi `3♠` stopper ask.
///
/// A stopper bids `3NT`.  Otherwise [`FitSearch`][MultiStopperAsk::FitSearch]
/// names the longest side suit at the four level, while
/// [`OpenerPlaces`][MultiStopperAsk::OpenerPlaces] places the minor games
/// immediately.  [`longest_unbid`] is a partition, so it is also the finite
/// fallback when opener's four spades leave only three-card side suits.
pub(crate) fn multi_stopper_answer(mode: MultiStopperAsk) -> Rules {
    let mut rules = Rules::new().rule(Bid::new(3, Strain::Notrump), 160, stopper_in(Suit::Spades));
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        let level = if mode == MultiStopperAsk::OpenerPlaces
            && matches!(suit, Suit::Clubs | Suit::Diamonds)
        {
            5
        } else {
            4
        };
        rules = rules.rule(
            Bid::new(level, Strain::from(suit)),
            140,
            longest_unbid(suit, Suit::Spades),
        );
    }
    rules
}

/// Responder continues the fit-search after opener names a minor.
pub(crate) fn multi_fit_search_rebid(shown: Suit) -> Rules {
    match shown {
        Suit::Clubs => Rules::new()
            .rule(Bid::new(5, Strain::Clubs), 160, len(Suit::Clubs, 4..))
            .rule(
                Bid::new(4, Strain::Hearts),
                150,
                len(Suit::Hearts, 4..) & at_least_as_long(Suit::Hearts, Suit::Diamonds),
            )
            .rule(
                Bid::new(4, Strain::Diamonds),
                150,
                len(Suit::Diamonds, 4..) & longer_suit(Suit::Diamonds, Suit::Hearts),
            )
            .rule(
                Bid::new(4, Strain::Hearts),
                100,
                at_least_as_long(Suit::Hearts, Suit::Diamonds),
            )
            .rule(
                Bid::new(4, Strain::Diamonds),
                100,
                longer_suit(Suit::Diamonds, Suit::Hearts),
            ),
        Suit::Diamonds => Rules::new()
            .rule(Bid::new(5, Strain::Diamonds), 160, len(Suit::Diamonds, 4..))
            .rule(
                Bid::new(4, Strain::Hearts),
                150,
                len(Suit::Hearts, 4..) & at_least_as_long(Suit::Hearts, Suit::Clubs),
            )
            .rule(
                Bid::new(5, Strain::Clubs),
                150,
                len(Suit::Clubs, 4..) & longer_suit(Suit::Clubs, Suit::Hearts),
            )
            .rule(
                Bid::new(4, Strain::Hearts),
                100,
                at_least_as_long(Suit::Hearts, Suit::Clubs),
            )
            .rule(
                Bid::new(5, Strain::Clubs),
                100,
                longer_suit(Suit::Clubs, Suit::Hearts),
            ),
        _ => unreachable!("the fit-search names only a minor below 4♥"),
    }
}

/// Place the sole unfinished fit-search branch, `4♣–4♦`.
pub(crate) fn multi_fit_search_place() -> Rules {
    Rules::new()
        .rule(Bid::new(5, Strain::Diamonds), 150, len(Suit::Diamonds, 4..))
        .rule(Bid::new(5, Strain::Clubs), 100, hcp(0..))
}

/// Opener's action when the opponents raise the stopper ask to `4♠`.
pub(crate) fn multi_stopper_over_four_spades() -> Rules {
    Rules::new()
        .rule(
            Call::Double,
            150,
            stopper_in(Suit::Spades) | len(Suit::Spades, 4..),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's forcing continuation after opener passes their `4♠` raise.
pub(crate) fn multi_stopper_forcing_rebid() -> Rules {
    let mut rules = Rules::new();
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        rules = rules.rule(
            Bid::new(5, Strain::from(suit)),
            140,
            longest_unbid(suit, Suit::Spades),
        );
    }
    rules
}

/// Opener's answer to responder's takeout double of the resolved major
/// (`1NT (2♦) X (2♥) - - X -`, `… X (2♠) - - X -`)
///
/// BBA's own answers here are opaque (`2NT` 34% even holding four of the
/// other major, `3m` with four, a penalty pass with 4+ of theirs), so this is
/// the bridge answer to a double that showed four of the other major and
/// shortness in theirs: sit with four of their suit, bid the 4-4 fit, else
/// the longer four-card minor, else `2NT`.  Total.
pub(crate) fn multi_takeout_answer(major: Suit) -> Rules {
    let other = if major == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    let level = if other == Suit::Spades { 2 } else { 3 };
    Rules::new()
        .rule(Call::Pass, 150, len(major, 4..))
        .rule(Bid::new(level, Strain::from(other)), 140, len(other, 4..))
        .rule(Bid::new(3, Strain::Clubs), 130, len(Suit::Clubs, 4..))
        .rule(Bid::new(3, Strain::Diamonds), 130, len(Suit::Diamonds, 4..))
        .rule(Bid::new(2, Strain::Notrump), 100, hcp(0..))
}

/// Opener's answer to responder's quantitative `4NT` (16+ opposite 15–17):
/// `6NT` from the top, else pass.  Total.
pub(crate) fn multi_quant_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(6, Strain::Notrump), 140, hcp(17..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener over responder's Multi values double when the advancer passes it
/// (`1NT (2♦) X -`) — a seat BBA's advancer never gives us (0.0% at
/// `advance-x`), kept for other opponents.  BBA's opener shows a four-card
/// major, else cues `3♦`; the cue is replaced by a pass — the double was
/// values and 2♦x with 23+ is a fine spot.  Total.
pub(crate) fn multi_pass_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 141, len(Suit::Hearts, 4..))
        .rule(Bid::new(2, Strain::Spades), 140, len(Suit::Spades, 4..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer to the advancer's pass-or-correct `2♥`/`2♠` after
/// responder's Multi values double (`1NT (2♦) X (2M) ?`)
///
/// `X` with four-plus trumps — nominally penalty; if the overcaller's major
/// is the other one they correct, and partner has been told where our trumps
/// are.  Everything else passes: their suit is not yet known, so opener's own
/// bids here are phantoms, and responder — who showed 8+ — speaks next with
/// the auction resolved.  Deliberately a total table (the wait is the call).
pub(crate) fn multi_penalty_answer(major: Suit) -> Rules {
    Rules::new()
        .rule(Call::Double, 150, len(major, 4..))
        .alert(MULTI_PENALTY)
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's balancing seat after responder *passes* their Multi and the
/// advancer names a major (`1NT (2♦) - (2M) ?`)
///
/// The lane's one seat with no book node at all — 57% of the `(2♦)` bucket
/// (253 bd, −426 plain, −199 PD on the `1e9a47e2` arms) — where the floor,
/// reading their `2♦` as diamonds, sells out at the two level.
///
/// This is [`multi_penalty_answer`] one branch over with the gate raised from
/// four trumps to five, and the raise is what the anchor itself does.  Probed
/// at the seat (`probe-bba-constraints --mode custom --seat 0 --calls
/// "1NT 2♦ - 2♥" --filter-call 1NT`, 4000 hands/vul) BBA passes **94.2%** over
/// `(2♥)` and **92.7%** over `(2♠)`, and its only action is
/// `hcp(15..=17) & len(M, 5..) & balanced()` — a trump-length **penalty**
/// double of the suit they named, not the delayed takeout double of Multi
/// theory, and it never bids a natural suit in this seat at all.
///
/// Partner passed rather than doubling, so opener is short of the values half
/// of the structure and only trump length can act; five cards behind a
/// pass-or-correct is the whole case.  Total.
pub(crate) fn multi_balance_double(major: Suit) -> Rules {
    Rules::new()
        .rule(Call::Double, 150, len(major, 5..))
        .alert(MULTI_PENALTY)
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's completion of the `3♠`→♣ game-force over the Multi
///
/// The natural leg's [`clubs_transfer_completion`] picks `3NT` on a *diamond*
/// stopper — meaningless here, and opener cannot know which major to hold — so
/// it is `3NT` outright: responder's six clubs are the source of tricks either
/// way, and `5♣` on a 6-2 fit is the worse guess.
pub(crate) fn multi_clubs_transfer_completion(agreements: &Agreements) -> Rules {
    let completion_alerts = agreements.decision.reading.completion_alerts;
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 140, hcp(0..))
        .alert_if(completion_alerts, COMPLETION)
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

/// Opener's completion of a top-step minor transfer (a forced game-force)
///
/// Responder has 6+ cards in `target`, no stopper in `over`, and game values.
/// Opener bids `3NT` with a stopper of its own, else raises to five of the minor.
/// The three-level completion is below the top step, so the auction must reach
/// game; five of the minor is the finite catch-all.
//
// ponytail: minor-suit slam exploration is left to the floor; 3NT-or-5m covers
// the common game. Author a keycard ladder here only if the A/B shows it matters.
pub(crate) fn minor_transfer_completion(
    target: Suit,
    over: Suit,
    agreements: &Agreements,
) -> Rules {
    let completion_alerts = agreements.decision.reading.completion_alerts;
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 140, stopper_in(over))
        .alert_if(completion_alerts, COMPLETION)
        .rule(Bid::new(5, Strain::from(target)), 50, hcp(0..))
        .alert_if(completion_alerts, COMPLETION)
}

pub(crate) fn clubs_transfer_completion(over: Suit, agreements: &Agreements) -> Rules {
    minor_transfer_completion(Suit::Clubs, over, agreements)
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

// ---------------------------------------------------------------------------
// Kokish–Kraft — the opt-in whole-table counter-defense to their `(2♦)` Multi
// ---------------------------------------------------------------------------

/// Responder's Kokish–Kraft table over their `(2♦)` Multi
/// (`competition.multi_kokish_kraft`, campaign package N4-KK)
///
/// The Eric Kokish–Beverly Kraft notes carry a table for exactly this object —
/// `2♦` as one unknown six-card major — and it is the most complete published
/// package for it (`docs/ai-bidder/multi-landy-2d-counter-defense-research.md`
/// §1).  Registered *instead of* [`multi_2d_responder`]'s subtree, never over
/// it: the two disagree on `2NT`, `3♣` and `3♠`, so an overlay would leave the
/// shipped rows shadowing these.
///
/// What changes against the shipped v7 lane:
///
/// - **`X` is invitational-plus values with no shape promise** (`hcp 8+`), not
///   v6's BBA-mimic `hcp 6+`.  K–K's doubler-rebid table has a natural
///   invitational `2NT` in it, so majorless doubles are part of the method; the
///   6–7 band that v6 doubled on now takes the designed **neutral pass**, whose
///   own delayed table is [`kokish_kraft_delayed`].
/// - **`2NT` and `3♣` are floorless minor transfers** (→♣ and →♦, six-card
///   suit, *no* point floor).  They replace the weak `2NT` relay, which dies
///   structurally, and are two-way: a weak hand preempts their unknown major
///   and passes the completion, a game hand rebids ([`kokish_kraft_transfer_rebid`]).
///   K–K makes them invitational-or-better; the floor is dropped here because
///   this lane's own census prices a passed-out long minor at −3.92 plain /
///   −4.84 PD a board (see [`CompetitionKnobs::multi_weak_escape`][crate::bidding::agreements::CompetitionKnobs::multi_weak_escape]).
/// - **`3♠` is both minors, game-forcing** at 5-4 or better, not the shipped
///   forced `3♠`→♣ game force (which the `2NT` transfer now carries).
/// - **`3NT` claims no stopper** (the source's gamble on a long suit), where
///   v4–v7 wanted both majors stopped.
/// - **`3♦`/`3♥` (transfers to ♥/♠), the weak `2♥`/`2♠` escapes and Leaping
///   Michaels `4♣`/`4♦` are unchanged**, and `4♥`/`4♠` are the *uncontested*
///   direct slam-try tier copied under the overcall.
///
/// Ordering constraints the weights encode: the minor transfers outrank `3NT`,
/// `3♠`, the natural `2M` and `X` (a long-minor hand transfers, it never
/// blasts); `2M` outranks `X` (a weak five-card major escapes); Leaping
/// Michaels outranks the transfers.
pub(crate) fn kokish_kraft_responder(agreements: &Agreements) -> Rules {
    let mut rules = leaping_michaels_2d(Rules::new());
    // The uncontested `1NT - 4M` tier verbatim: a six-card major with
    // slam-invitational values, opener passing a minimum or launching RKCB
    // ([`slam_try_answer`]).  Deliberately *not* South African Texas — our
    // `4♣`/`4♦` are Leaping Michaels here.
    //
    // Which is also why the band is thin: [`direct_4m_max`] is `15` under the
    // shipped `notrump.texas_slam_drive`, because uncontested a 17+ six-card
    // major takes Texas at `4♣`/`4♦` and drives its own keycard ask.  That
    // route does not exist in this lane, so the 16+ hand falls back on the
    // `3♦`/`3♥` transfer and reaches `4M` with its slam try floored — exactly
    // as the shipped v7 lane already routes it, so nothing regresses, but
    // widening to `15..=18` here is a real candidate and a behaviour change.
    // Recorded in docs/one-notrump-competitive.md §N4-KK; it wants its own arm.
    let slam_try_max = direct_4m_max(agreements);
    for (major, other) in [(Suit::Hearts, Suit::Spades), (Suit::Spades, Suit::Hearts)] {
        rules = rules.rule(
            Bid::new(4, Strain::from(major)),
            260,
            len(major, 6..) & len(other, ..5) & hcp(15..=slam_try_max),
        );
    }
    // The major transfers, unchanged from the shipped leg: INV+, auto-driven
    // to game by [`transfer_completion`].
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
    // The floorless minor transfers.  `3♣`→♦ above `2NT`→♣ so a hand long in
    // both takes the higher-ranking suit at the cheaper relative cost.
    rules = rules
        .rule(Bid::new(3, Strain::Clubs), 178, len(Suit::Diamonds, 6..))
        .alert(KK_MINOR_TRANSFER)
        .rule(Bid::new(2, Strain::Notrump), 176, len(Suit::Clubs, 6..))
        .alert(KK_MINOR_TRANSFER);
    // `3♠` both minors GF at 5-4+, then the direct `3NT` blast.
    //
    // Two ordering repairs against the design sketch, both forced by the same
    // thing — a *bare* `points(10..)` `3NT` contains every other constructive
    // gate in the table, so whatever sits below it is unreachable:
    //
    // 1. `3♠` outranks `3NT` (the sketch had it the other way): the
    //    both-minors gate implies `points(10..)`, so a higher `3NT` would make
    //    `3♠` dead code rather than a rare rung.  The source agrees on the
    //    merits — its `3NT` is the last-resort gamble, the shape calls come
    //    first — and the sketch's stated ordering constraints say nothing
    //    about this pair.
    // 2. **`3NT` keeps a stopper gate.**  The sketch dropped it "per source"
    //    (K–K's `3NT` promises no stopper information).  Measured bare, the
    //    rule confines the values double to `points 8..9` — `probe-call-reading
    //    --ns-multi-kokish-kraft "1N (2D) X -"` reads exactly that — because
    //    every 10+ hand blasts instead, which contradicts the same source's
    //    "`X` = invitational **or better**" and re-runs at maximum frequency
    //    the stopperless blast perfect defense priced at −3.7/−4.3 a board in
    //    N4 v2/v3.  Ranking `X` above a bare `3NT` does not fix it either: the
    //    survivors are then `hcp ≤ 7` hands with distributional points and no
    //    transfer — a 7-count 4-4-4-1 blasting 3NT — which is worse than dead.
    //    So the gate stays, unchanged from v4–v7, and dropping it is a
    //    one-line sub-arm if the user wants K–K's letter measured.
    rules = rules
        .rule(
            Bid::new(3, Strain::Spades),
            152,
            len(Suit::Clubs, 4..)
                & len(Suit::Diamonds, 4..)
                & (len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..))
                & points(10..),
        )
        .alert(KK_MINORS)
        .rule(
            Bid::new(3, Strain::Notrump),
            150,
            points(10..) & stopper_in(Suit::Hearts) & stopper_in(Suit::Spades),
        );
    // The weak natural escapes, shared with the shipped leg (and with its
    // floorless `multi_weak_escape` rung, which composes).
    rules = natural_major_escapes(rules, agreements, agreements.competition.multi_weak_escape);
    rules
        .rule(Call::Double, 130, hcp(8..))
        .alert(KK_VALUES)
        .rule(Call::Pass, 0, hcp(0..))
}

/// The Kokish–Kraft doubler's rebid once their pass-or-correct has resolved the
/// major (`1NT (2♦) X (2♥) - -` and its three siblings)
///
/// [`multi_responder_rebid`]'s fork.  The one structural difference is the
/// **repeated double**: K–K (like every other exact-object source surveyed)
/// separates the two delayed doubles — after an initial `X` a second double is
/// cooperative *penalty*, while after an initial *pass* it is takeout
/// ([`kokish_kraft_delayed`]).  So this arm reverts v7's takeout `X` to v4's
/// trump-length penalty double, inside this variant only.  The `ran` shape
/// (their weak advancer corrected `2♥` to `2♠`) was already penalty and is
/// unchanged.
///
/// `2NT` is K–K's natural invitational rebid, new here: v6 played BBA's own
/// 8–9 invite and perfect defense priced it at −3.0/−6.9 per invite, so it
/// rides this arm rather than the default lane.  It carries the same
/// `stopper_in(major)` its delayed twin does ([`kokish_kraft_delayed`]) —
/// natural notrump opposite a *known* six-card suit means a guard in it, and
/// without the gate an 8-count and a 16-count reach `3NT` with the opponents'
/// long major unstopped in both hands.  The weak `2♠` signoff of v7 dies — a
/// weak five-card spade hand bids `2♠` directly instead of doubling.
pub(crate) fn kokish_kraft_doubler_rebid(major: Suit, ran: bool) -> Rules {
    let mut rules = Rules::new().rule(Bid::new(4, Strain::Notrump), 160, hcp(16..));
    rules = if ran {
        rules.rule(Call::Double, 155, len(Suit::Spades, 4..) & hcp(7..))
    } else {
        rules.rule(Call::Double, 155, len(major, 4..))
    }
    .alert(MULTI_PENALTY);
    rules
        .rule(
            Bid::new(3, Strain::Notrump),
            150,
            points(10..) & stopper_in(major),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            145,
            hcp(8..=9) & stopper_in(major),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer to the doubler's natural invitational `2NT`: accept the
/// game from the top of the range, else pass.  Total.
pub(crate) fn kokish_kraft_invite_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 140, hcp(16..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's **delayed** table after the neutral pass, once their
/// pass-or-correct has named the major (`1NT (2♦) - (2♥) - -` and siblings)
///
/// The other half of K–K's split-double theory: having passed the Multi rather
/// than doubling it, responder's double here is **takeout** — four of the other
/// major, at most a doubleton in theirs — and opener answers it with the
/// shipped [`multi_takeout_answer`].  `2NT` is natural and competitive with
/// their suit stopped; `3♣`/`3♦` are competitive six-card suits, kept from the
/// source though the traffic is thin now that a long minor transfers at once.
///
/// This seat is where the shipped lane's `hcp 6+` doubles go under K–K's `hcp
/// 8+` gate: the 6–7 band passes first and speaks here, with the major known.
///
/// **Two rungs are dead in self-play, and they are kept anyway.** Responder
/// reached this seat by *passing*, and under K–K a weak six-card minor does not
/// pass — it transfers, floorlessly, at `2NT`/`3♣`. So the source's competitive
/// `3♣`/`3♦` can only ever fire opposite a partner who is not bidding this
/// table (a human, or a hand that passed for a reason the book does not model),
/// and every A/B will show them at zero. For the same reason the first-turn
/// pass denies `hcp 8+`, so the natural `2NT` is really `hcp == 7`, not the
/// `7..=9` the rule spells. Both are consequences of making the minor
/// transfers floorless where K–K makes them invitational-plus; the reversible
/// alternative is a floor on the transfers, which is a different arm.  The
/// rungs stay because they cost nothing, they are the source's, and deleting
/// them would silently hand those seats to the floor.
pub(crate) fn kokish_kraft_delayed(major: Suit) -> Rules {
    let other = if major == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    Rules::new()
        .rule(
            Call::Double,
            150,
            len(other, 4..) & len(major, ..=2) & hcp(6..),
        )
        .alert(MULTI_TAKEOUT)
        .rule(
            Bid::new(2, Strain::Notrump),
            140,
            hcp(7..=9) & stopper_in(major),
        )
        .rule(
            Bid::new(3, Strain::Clubs),
            130,
            len(Suit::Clubs, 6..) & points(..=9),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            129,
            len(Suit::Diamonds, 6..) & points(..=9),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's forced completion of a Kokish–Kraft minor transfer — `2NT`→`3♣`,
/// `3♣`→`3♦`
///
/// Unconditional, doubled or not: the transfer carries no point floor, so
/// opener has nothing to decide and the whole message is "partner declares".
pub(crate) fn kokish_kraft_minor_completion(minor: Suit, agreements: &Agreements) -> Rules {
    let completion_alerts = agreements.decision.reading.completion_alerts;
    Rules::new()
        .rule(Bid::new(3, Strain::from(minor)), 100, hcp(0..))
        .alert_if(completion_alerts, COMPLETION)
}

/// Responder's rebid over the completed minor transfer
///
/// Pass is the sign-off — the whole point of the floorless rung.  Above it sit
/// the source's two-suiter steps (after `3♣`: `3♦` = +♥, `3♥` = +♠, `3♠` = +♦;
/// after `3♦`: `3♥` = +♠, `3♠` = +♥) and `3NT` as the plain six-card-minor
/// choice of games.  Every rung above Pass is game-forcing, so the transfer is
/// two-way and opener's completion never strands one.
///
/// Under [`CompetitionKnobs::multi_minor_slam_try`][crate::bidding::agreements::CompetitionKnobs::multi_minor_slam_try]
/// a `4m` slam try sits between the lowest two-suiter step and `3NT` — the
/// rung [`landy_bba_transfer_rebid`][crate::bidding::american::competition::lebensohl::landy_bba_transfer_rebid]
/// already carries one lane over, whose own floor is `13`.  Without it the
/// ladder ends at `3NT` and a 21-count that transferred has nowhere left to say
/// so.  There is no room under `3NT`, so the rung buys its information by
/// giving `3NT` up — which is the trade the arm prices, and why the floor is
/// the knob's payload rather than a constant.
///
/// Opener's answer **is** authored ([`kokish_kraft_slam_answer`]), against the
/// N1 slam-exploration doctrine that leaves a `4m` suit contract to the floor.
/// Probed at this seat with the rung live, the floor's whole vocabulary is
/// `{6NT, 4♥, Pass}` — `4♥` being a bid in the suit their Multi showed — and it
/// can never keycard here, since `instinct`'s `4NT` ask is gated on
/// `Context::undisturbed` and this lane is disturbed by construction.
pub(crate) fn kokish_kraft_transfer_rebid(minor: Suit, agreements: &Agreements) -> Rules {
    let mut rules = Rules::new();
    let mut weight = 160;
    for (second, step) in kokish_kraft_second_suits(minor) {
        rules = rules
            .rule(Bid::new(3, *step), weight, len(*second, 4..) & points(10..))
            .alert(KK_TWO_SUITER);
        weight -= 4;
    }
    if let Some(floor) = agreements.competition.multi_minor_slam_try {
        rules = rules.rule(
            Bid::new(4, Strain::from(minor)),
            151,
            points(floor..) & len(minor, 6..),
        );
    }
    rules
        .rule(Bid::new(3, Strain::Notrump), 150, points(10..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// The `(second suit, step)` pairs a completed transfer to `minor` can show
///
/// The source's own assignment, which is *not* next-suit-up: after the club
/// transfer the three steps are ♥, ♠, ♦; after the diamond transfer the two
/// steps are ♠ then ♥.  One table so the responder's rebid and opener's answer
/// cannot drift apart.
pub(crate) fn kokish_kraft_second_suits(minor: Suit) -> &'static [(Suit, Strain)] {
    match minor {
        Suit::Clubs => &[
            (Suit::Hearts, Strain::Diamonds),
            (Suit::Spades, Strain::Hearts),
            (Suit::Diamonds, Strain::Spades),
        ],
        _ => &[
            (Suit::Spades, Strain::Hearts),
            (Suit::Hearts, Strain::Spades),
        ],
    }
}

/// Opener's answer to a two-suiter rebid behind a Kokish–Kraft minor transfer
///
/// Responder is 6-4 and game-forcing.  Bid the major game on four-card support
/// (a ten-card fit's worth of trumps between the two hands is not on offer
/// anywhere else), else `3NT` — the same call [`multi_clubs_transfer_completion`]
/// settled on in this lane, and for the same reason: opener cannot know which
/// major is theirs, and five of a minor on a 6-2 is the worse guess.
//
// ponytail: no minor-suit slam ladder and no 5m rung. Author one only if the
// A/B shows the 3NT-or-4M pair costs.
pub(crate) fn kokish_kraft_two_suiter_answer(second: Suit) -> Rules {
    let mut rules = Rules::new();
    if matches!(second, Suit::Hearts | Suit::Spades) {
        rules = rules.rule(Bid::new(4, Strain::from(second)), 160, len(second, 4..));
    }
    rules.rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
}

/// Responder, after their advancer competes over a Kokish–Kraft minor transfer
/// and opener sits
///
/// The transfer was floorless, so opener's guarded sit is the only safe answer
/// and the values — if responder has any — must act here instead.  `3NT` with
/// game values and their now-named major stopped; **`X` on `hcp 10+` without
/// one**, which is what keeps a 25-plus-point pair from selling out to a
/// three-level partscore in a suit we know is only six long; otherwise the
/// preempt has done its job and Pass takes the plus.
///
/// `hcp`, not `points`, on the double alone: responder's shown suit is six
/// long, so `points` counts length this hand cannot cash on defence, and the
/// `3NT` rung above is the one that wants the length term.
///
/// Opener has already passed, so the double stands as penalty
/// ([`multi_signoff_pass`] answers it).  Unalerted: a double is penalty by
/// default in this book, and it claims no shape — so the natural walk's reading
/// is the true one and there is nothing an alert would add.
///
/// Under [`CompetitionKnobs::multi_minor_slam_try`][crate::bidding::agreements::CompetitionKnobs::multi_minor_slam_try]
/// a third call joins them: `4m` on shortness in their now-named major, ranked
/// between the two.  That is the hand the two-call table has no home for — 10+
/// with a singleton or void in their suit, which wants to play our minor and
/// should not be doubling with a void.  It is [`kokish_kraft_transfer_rebid`]'s
/// rung one round later and one level below the `5m` the residue first
/// proposed: eleven tricks become ten, and the floor still carries on from a
/// suit contract.
pub(crate) fn kokish_kraft_transfer_overcalled(
    minor: Suit,
    major: Suit,
    agreements: &Agreements,
) -> Rules {
    let mut rules = Rules::new().rule(
        Bid::new(3, Strain::Notrump),
        150,
        points(10..) & stopper_in(major),
    );
    if agreements.competition.multi_minor_slam_try.is_some() {
        rules = rules.rule(
            Bid::new(4, Strain::from(minor)),
            145,
            len(major, ..=1) & len(minor, 6..) & points(10..),
        );
    }
    rules
        .rule(Call::Double, 140, hcp(10..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer to the Kokish–Kraft `4m` slam try
/// ([`CompetitionKnobs::multi_minor_slam_try`][crate::bidding::agreements::CompetitionKnobs::multi_minor_slam_try])
///
/// Responder holds six-plus of the minor and enough not to want `3NT`; opener
/// opened a balanced 15-17, so it holds two-plus and the fit is assured.  A
/// **maximum asks** — `4NT` is RKCB for the minor, `american::slam::rkcb_rows`
/// supplying the whole ladder including the cramped minor sign-offs — and
/// anything else declines to `5m`.  Total, so the seat is never the floor's.
///
/// The `hcp(16..)` gate is deliberately a **constant across both arms** of the
/// knob: the arms differ in responder's floor and nowhere else, or the A/B
/// prices two changes at once.
///
/// Why authored at all, when N1's doctrine leaves a `4m` suit contract to the
/// floor to cue-bid on from: measured, it does not.  At this seat the floor
/// offers `{6NT, 4♥, Pass}` and takes `4♥` — their Multi's own suit — on a
/// minimum, and it cannot reach for keycard at all, `instinct`'s `4NT` ask
/// being gated on `Context::undisturbed`.
pub(crate) fn kokish_kraft_slam_answer(minor: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 160, hcp(16..))
        .rule(Bid::new(5, Strain::from(minor)), 100, hcp(0..))
}

/// Opener's answer to the Kokish–Kraft `3♠` (both minors, game-forcing)
///
/// The unknown major cuts both ways, so `3NT` needs **both** majors stopped —
/// that double stopper is exactly the information the call was made to buy.
/// Without it, four of the better minor and responder raises to game
/// ([`kokish_kraft_minors_place`]).
pub(crate) fn kokish_kraft_minors_answer() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Notrump),
            160,
            stopper_in(Suit::Hearts) & stopper_in(Suit::Spades),
        )
        .rule(
            Bid::new(4, Strain::Diamonds),
            150,
            at_least_as_long(Suit::Diamonds, Suit::Clubs),
        )
        .rule(Bid::new(4, Strain::Clubs), 100, hcp(0..))
}

/// Responder completes the both-minors game force in opener's chosen minor.
/// Total — responder holds four-plus in each, so opener's pick is playable.
pub(crate) fn kokish_kraft_minors_place(minor: Suit) -> Rules {
    Rules::new().rule(Bid::new(5, Strain::from(minor)), 100, hcp(0..))
}

/// Opener when they compete over the Kokish–Kraft `3♠` both-minors game force
///
/// Responder promised game values with both minors, so 25+ combined points sit
/// behind a call in a suit responder is by definition short in: double.
///
/// **Total on purpose, one rule.** An earlier draft wrote `X` on `hcp(15..)`
/// with a `Pass` catch-all under it, which is a lie twice over: this seat
/// belongs to the hand that opened `1NT`, so `hcp(15..)` is vacuous and the
/// `Pass` could never fire.  A rule nothing reaches is worse than no rule —
/// it reads as a decision that was made.
//
// ponytail: no 5m rung. Opener cannot know which minor responder is longer in
// and we are already at the four level; if the A/B shows the doubles costing,
// the fix is a length-gated pull, not a Pass.
pub(crate) fn kokish_kraft_minors_overcalled() -> Rules {
    Rules::new().rule(Call::Double, 100, hcp(0..))
}

#[cfg(test)]
mod tests;
