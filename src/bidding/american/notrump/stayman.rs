//! Stayman — the `2♣` major ask, opener's answers, and both rebids
//!
//! The base convention: responder's `2♣`, opener's `2♦`/`2♥`/`2♠`, responder's
//! rebid whether or not a major came back, Smolen after the denial, and the
//! invitation-acceptance tables.  Garbage (drop-dead) Stayman rides it under
//! [`set_garbage_stayman`], the net-force seam under
//! [`set_stayman_net_force`].

use super::both_majors::fit_value;
use super::*;

/// Garbage (drop-dead) Stayman: a weak 2♣ intending to pass opener's answer
///
/// Two tiers, looser the weaker responder is (a broke 1NT rates to be a
/// disaster, so any ~7-card fit is an improvement): both tiers want a four-card
/// major, both majors playable (3+ for a ≥7-card fit on any major answer), and
/// short clubs; the broke tier accepts a thinner 2♦ landing (3+ diamonds), the
/// weak tier insists on 4+.  HCP bands are disjoint from the constructive 2♣
/// (`hcp(8..)`), so no hand matches two 2♣ rules.  Empty when off.
// ponytail: the 0-4/5-7 split and the 3-vs-4 diamond floor are tunable knobs —
// the A/B can tighten or loosen them.
pub(super) fn garbage_stayman_rule(agreements: &Agreements) -> Rules {
    if !agreements.build.notrump.garbage_stayman {
        return Rules::new();
    }
    Rules::new()
        // Broke (0-4): escape at almost any cost; accept a thin 2♦ landing.
        .rule(
            Bid::new(2, Strain::Clubs),
            150,
            (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..))
                & len(Suit::Hearts, 3..)
                & len(Suit::Spades, 3..)
                & len(Suit::Clubs, ..3)
                & len(Suit::Diamonds, 3..)
                & hcp(..5)
                & !flat_4333(),
        )
        .alert(STAYMAN)
        // Weak (5-7): insist on a safe 2♦ landing (4+ diamonds).
        .rule(
            Bid::new(2, Strain::Clubs),
            150,
            (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..))
                & len(Suit::Hearts, 3..)
                & len(Suit::Spades, 3..)
                & len(Suit::Clubs, ..3)
                & len(Suit::Diamonds, 4..)
                & hcp(5..8)
                & !flat_4333(),
        )
        .alert(STAYMAN)
}

/// Opener's answer to Stayman: a four-card major, else 2♦
///
/// `pub(super)` so the competitive book can reuse it as the always-mass catch-all
/// when authoring opener's penalty-pass over a `(2♣)` overcall (systems on).
pub(crate) fn stayman_answers() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 100, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(2, Strain::Spades),
            100,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            50,
            len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
}

/// Opener's Stayman answer at the *uncontested* `1NT - 2♣` node
///
/// Wraps [`stayman_answers`] with the opt-in max-showing overlays so the shared
/// `stayman_answers` (reused by the competitive book) stays untouched.  With both
/// toggles off this is byte-identical to `stayman_answers`.  A balanced 1NT with
/// a five-card major has ≤3 in the other major, so "both four-card majors" and
/// "five-card major" never overlap; the natural answers (weight 1.0) catch every
/// remaining case (single major, no major, a *minimum* five-card major).
pub(super) fn stayman_answers_uncontested(agreements: &Agreements) -> Rules {
    let mut rules = Rules::new();
    if agreements.build.notrump.stayman_both_majors {
        // Both four-card majors with a *maximum* (16-17, the invite-accepting
        // range): jump to 2NT.  Responder then names their own major (3♣ = hearts,
        // 3♦ = spades) so opener — the strong, concealed hand — declares it
        // (right-siding).  A minimum (15) bids 2♥ naturally, so 2NT only ever costs
        // a step on the maximum.
        let both = len(Suit::Hearts, 4..) & len(Suit::Spades, 4..);
        rules = rules
            .rule(Bid::new(2, Strain::Notrump), 110, both & hcp(16..))
            .alert(BOTH_MAJORS);
    }
    if agreements.build.notrump.stayman_5card_max {
        // Five-card major, maximum (16-17): jump.  Natural (names and shows its
        // own suit), so unalerted — alerting would make alert-reading suppress it.
        rules = rules
            .rule(
                Bid::new(3, Strain::Hearts),
                110,
                len(Suit::Hearts, 5..) & hcp(16..),
            )
            .rule(
                Bid::new(3, Strain::Spades),
                110,
                len(Suit::Spades, 5..) & hcp(16..),
            );
    }
    rules.chain(stayman_answers())
}

thread_local! {
    /// Garbage (drop-dead) Stayman: a *weak* hand bids 2♣ to escape 1NT into a
    /// major (or diamond) partscore, intending to pass opener's answer.  **On by
    /// default** — a paired DD A/B vs BBA (205k boards, vul none) measured +0.51
    /// IMPs/fired plain (+0.0009/board, 95% CI excl 0) and +0.70 PD.  See
    /// [`set_garbage_stayman`].
    static GARBAGE_STAYMAN: Cell<bool> = const { Cell::new(true) };
}

/// Author garbage (drop-dead) Stayman for books built *after* this call
/// (thread-local; **on by default**).
///
/// A weak responder with short clubs and a four-card major bids 2♣ to escape a
/// likely-doomed 1NT, passing opener's 2♦/2♥/2♠.  Looser the weaker responder
/// is: broke hands accept a thinner 2♦ landing, since any ~7-card fit beats a
/// broke 1NT.
pub fn set_garbage_stayman(on: bool) {
    GARBAGE_STAYMAN.with(|cell| cell.set(on));
}

/// Whether garbage Stayman is currently authored (read by the inference engine
/// too, to widen the 2♣ point range it reads)
pub fn garbage_stayman() -> bool {
    GARBAGE_STAYMAN.with(Cell::get)
}

thread_local! {
    /// See [`set_stayman_net_force`].
    static STAYMAN_NET_FORCE: Cell<bool> = const { Cell::new(false) };
}

/// Price responder's Stayman-rebid invite/force seams with the evaluator net
/// instead of the point tests (thread-local; **off by default — measured a
/// loss**, kept for re-measurement)
///
/// The `probe-nt-invite-eval` screen (30 000 deals per class, seed 1784718391)
/// found the net's game make-probability is the first evaluator to out-rank
/// raw HCP at the 1NT invite/force boundary — but only on the Stayman class
/// (+0.030 ±0.017 IMPs/board vul none, +0.044 ±0.025 vul both, rising to
/// +0.048/+0.069 opposite exactly-15 openers); the balanced no-major seam
/// stays HCP (net ≈ 0, third evaluator family to fail there).  This knob
/// converts exactly the Stayman-rebid seams: with a fit the `4M`/`3M`/`3OM`
/// split, without one the `3NT`/`2NT` revert — each force arm becomes "the
/// net clears the game's IMP break-even at the live vulnerability", its
/// invite twin the declined half.  The 2♣ entry, Smolen, garbage/crawling
/// and the quantitative 4NT are untouched.
///
/// **The live A/B refuted it** (`ab-stayman-net-force --slice`, 200k sliced
/// boards per vul, seed 1784719896): vul none −0.022 plain DD / +0.003 PD,
/// vul both −0.021 plain / −0.027 PD.  The forensic split explains the
/// screen-vs-live reversal: at the *fit* seam the incumbent is not raw HCP
/// but [`fit_value`] — already an upgrade evaluator, and the net loses to it
/// on both scorers in both directions; at the *no-fit NT* seam the net's
/// flips are plain-DD-positive (matching the screen, which scored plain DD)
/// but PD-negative — the DD-trained net promotes 3NTs that die against
/// perfect defense, the decision table's "doubling artifact, don't ship" row.
/// A frequency-matched NT-seam-only gate re-scored under `single_dummy_leads`
/// is the remaining open refinement.
///
/// Unlike its construction-time neighbours, this is read at **classification
/// time** (like [`set_bilans_floor`][crate::bidding::instinct::set_bilans_floor]):
/// flip it on threads that classify through a [`Stance`][crate::bidding::Stance],
/// no book rebuild needed.
#[doc(hidden)]
pub fn set_stayman_net_force(on: bool) {
    STAYMAN_NET_FORCE.with(|cell| cell.set(on));
}

/// Plain-bool read of the Stayman net-force knob (see [`set_stayman_net_force`])
pub(crate) fn stayman_net_force() -> bool {
    STAYMAN_NET_FORCE.with(Cell::get)
}

/// One side of a Stayman-rebid invite/force seam: knob-off exactly
/// `shape & points`; with [`set_stayman_net_force`] on, `shape` plus the net's
/// `want` verdict on `tricks` in `strain` replacing the point test
///
/// The call *reads* as `shape & points` either way ([`reads_as`]) — the net
/// arm is an opaque predicate, and letting it into the projection `Or` would
/// union the authored band out to a vacuous reading.
fn stayman_net_seam(
    shape: Cons<impl Constraint + Clone>,
    points: Cons<impl Constraint + Clone>,
    want: bool,
    strain: Strain,
    tricks: u8,
) -> Cons<impl Constraint + Clone> {
    let off = pred(|_: Hand, context: &Context<'_>| !context.decision_profile().stayman_net_force);
    let evaluated = shape.clone()
        & ((off & points.clone())
            | net_break_even_gate(|profile| profile.stayman_net_force, want, strain, tricks));
    reads_as(evaluated, shape & points)
}

/// Responder's rebid after opener answers Stayman with a major (`2♥`/`2♠`)
///
/// With a fit (four cards in opener's `major`): an invitational raise (`3M`) on
/// a flat eight, game (`4M`) on any upgrade past it (a ninth trump or working
/// shape — see [`fit_value`]), or — balanced, or slam-interested — the *other*
/// major (`3OM`) as an artificial slam try / choice of game.  Without a fit, the
/// auction reverts to notrump exactly as over a bare 1NT — invite `2NT`, game
/// `3NT`, and the quantitative `4NT` (16–17) — "ignore the 2♣ detour".
pub(super) fn stayman_major_rebid(major: Suit, agreements: &Agreements) -> Rules {
    let other = Strain::from(other_major(major));
    let strain = Strain::from(major);
    // Invitational-5-4 reroute: when on and opener showed *hearts*, a 5♠4♥ hand has
    // its own forcing `2♠` rebid (it Staymaned rather than transferring), so the
    // heart raises and the `3♠` slam-try are capped at four spades — routing that
    // hand to 2♠ and sharpening `3♠` into a balanced slam try that *denies* five
    // spades.  Off the flag (or over a 2♠ answer, where 5♥4♠ transfers and never
    // reaches here) the cap `len(♠,..14)` is a no-op.
    let reroute = agreements.build.notrump.invitational_5card_majors && major == Suit::Hearts;
    let spade_cap = if reroute {
        len(Suit::Spades, ..5)
    } else {
        len(Suit::Spades, ..14)
    };
    let mut rules = Rules::new();
    if reroute {
        // Forcing 5♠4♥, invitational through slam — opener picks ♥ (4-4) or ♠ (5-3)
        // and the level (see `answer_inv_5card_both`).  Responder's four hearts is
        // implied (it Staymaned, opener showed hearts), so `2♠` stays natural-spades
        // — flooring only its own strain keeps it unalerted (the artificial-alert
        // invariant); the spade-capped raises split off the ≤4-spade hands.
        rules = rules.rule(
            Bid::new(2, Strain::Spades),
            130,
            len(Suit::Spades, 5..) & hcp(8..),
        );
    }
    let rules = rules
        // Fit: artificial slam try / choice of game (balanced, or 16+); denies 5♠.
        .rule(
            Bid::new(3, other),
            140,
            stayman_net_seam(
                len(major, 4..) & (balanced() | hcp(16..)) & spade_cap.clone(),
                hcp(9..),
                true,
                strain,
                10,
            ),
        )
        .alert(SLAM_TRY)
        // Fit: sign off in the major game — any upgrade past a flat eight (a ninth
        // trump, or working shape) commits to game opposite the 15-17 opener.
        .rule(
            Bid::new(4, strain),
            130,
            stayman_net_seam(
                len(major, 4..) & spade_cap.clone(),
                described(
                    "game value for the fit",
                    move |hand: Hand, context: &Context<'_>| fit_value(context, hand, major) >= 9,
                ),
                true,
                strain,
                10,
            ),
        )
        // Fit: invitational raise — a flat eight, four-card fit, no upgrade (or,
        // net-priced, any fit hand whose game the net declines).
        .rule(
            Bid::new(3, strain),
            120,
            stayman_net_seam(
                len(major, 4..) & spade_cap.clone(),
                described(
                    "invitational value for the fit",
                    move |hand: Hand, context: &Context<'_>| fit_value(context, hand, major) == 8,
                ),
                false,
                strain,
                10,
            ),
        )
        // No fit: quantitative 4NT (as if the 2♣ detour never happened).
        .rule(
            Bid::new(4, Strain::Notrump),
            120,
            len(major, ..4) & hcp(16..=17),
        )
        // No fit: game / invitational notrump raise.  The net seams keep the
        // 2♣ entry's 8-HCP floor so a garbage/crawling weak hand never invites.
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            stayman_net_seam(
                len(major, ..4) & hcp(8..),
                hcp(9..),
                true,
                Strain::Notrump,
                9,
            ),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            100,
            stayman_net_seam(
                len(major, ..4) & hcp(8..),
                hcp(8..=8),
                false,
                Strain::Notrump,
                9,
            ),
        );
    // Stayman-then-minor slam try: a natural 5+ minor with slam values (14+) and no
    // fit for opener's major (capped at three, else responder raises or takes the
    // 3OM slam try).  Weight 1.25 outranks the no-fit `3NT`/`4NT` reverts so the
    // two-suiter shows its second suit instead of guessing notrump.  Empty off the
    // gate; the minor is real, so each rule floors only its own strain and stays
    // unalerted (the artificial-alert invariant).
    if agreements.build.notrump.stayman_minor_slam_try {
        rules
            .rule(
                Bid::new(3, Strain::Clubs),
                125,
                len(Suit::Clubs, 5..) & hcp(14..) & len(major, ..4),
            )
            .rule(
                Bid::new(3, Strain::Diamonds),
                125,
                len(Suit::Diamonds, 5..) & hcp(14..) & len(major, ..4),
            )
    } else {
        rules
    }
}

/// Responder's rebid after opener denies a major (`1NT - 2♣ - 2♦`)
///
/// Smolen: jump in the four-card major to show *five* in the other, game-forcing,
/// so the strong notrump hand declares.  Lacking 5–4, revert to notrump as if the
/// 2♣ detour never happened — invite `2NT`, game `3NT`, quantitative `4NT`.
pub(super) fn stayman_no_major_rebid(agreements: &Agreements) -> Rules {
    let rules = Rules::new()
        .rule(
            Bid::new(3, Strain::Hearts),
            140,
            len(Suit::Hearts, 4..=4) & len(Suit::Spades, 5..) & hcp(9..),
        )
        .alert(SMOLEN)
        .rule(
            Bid::new(3, Strain::Spades),
            140,
            len(Suit::Spades, 4..=4) & len(Suit::Hearts, 5..) & hcp(9..),
        )
        .alert(SMOLEN)
        .rule(Bid::new(4, Strain::Notrump), 120, hcp(16..=17))
        // The notrump revert seams are net-priced under `set_stayman_net_force`
        // (Smolen and the quantitative 4NT outrank them by weight either way).
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            stayman_net_seam(hcp(8..), hcp(9..), true, Strain::Notrump, 9),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            100,
            stayman_net_seam(hcp(8..), hcp(8..=8), false, Strain::Notrump, 9),
        );
    let rules = if agreements.build.notrump.crawling_stayman {
        // Crawling Stayman: 4-4 majors short in diamonds (a bare 2♥, weak) — both
        // majors, pass-or-correct (see `answer_crawling_stayman`).  Gated by the
        // diamond shortness (≤1) that brought it here: garbage hands have 3+
        // diamonds and pass 2♦ instead.  Responder's four spades is implied by the
        // crawling 2♣, so 2♥ floors only hearts and stays unalerted natural (like
        // the 2♠ sibling).  Disjoint from every rule above (all need hcp ≥8).
        rules.rule(
            Bid::new(2, Strain::Hearts),
            140,
            len(Suit::Hearts, 4..) & len(Suit::Diamonds, ..=1) & hcp(..8),
        )
    } else {
        rules
    };
    let rules = if agreements.build.notrump.invitational_5card_majors {
        // 5♠4♥, non-forcing invitational: opener denied hearts, so name the
        // five-card spade suit (natural, outranks the 2NT fallback).  Opener passes
        // a minimum or raises to game (see `answer_inv_5card_spades`).  A 5♠4♥
        // game-force jumped Smolen `3♥` above.  Responder's four hearts is implied
        // (it Staymaned), so `2♠` floors only spades and stays unalerted natural.
        rules.rule(
            Bid::new(2, Strain::Spades),
            110,
            len(Suit::Spades, 5..) & hcp(8..=8),
        )
    } else {
        rules
    };
    // Stayman-then-minor slam try, opener having denied a major: a natural 5+ minor
    // with slam values (14+).  No major cap — the 2♦ answer already denied the fit.
    // Weight 1.25 outranks the `3NT`/`4NT` reverts; the minor is real, so it floors
    // only its own strain and stays unalerted.  Empty off the gate.  (Smolen owns
    // `3♥`/`3♠`, so `3♣`/`3♦` are free here.)
    if agreements.build.notrump.stayman_minor_slam_try {
        rules
            .rule(
                Bid::new(3, Strain::Clubs),
                125,
                len(Suit::Clubs, 5..) & hcp(14..),
            )
            .rule(
                Bid::new(3, Strain::Diamonds),
                125,
                len(Suit::Diamonds, 5..) & hcp(14..),
            )
    } else {
        rules
    }
}

/// Opener completes Smolen by bidding game in responder's shown five-card major
pub(crate) fn smolen_completion(five_card: Suit) -> Rules {
    let strain = Strain::from(five_card);
    Rules::new()
        // Eight-card fit: bid game in the long major so opener declares.
        .rule(Bid::new(4, strain), 100, len(five_card, 3..))
        // No fit: notrump game.
        .rule(Bid::new(3, Strain::Notrump), 50, len(five_card, ..3))
}

/// Smolen at the three level: responder's jump after opener denies a major
/// (`…3♣ - 3♦`).  Game is already forced, so no strength gate.
pub(crate) fn smolen_at_three() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Hearts),
            140,
            len(Suit::Hearts, 4..=4) & len(Suit::Spades, 5..),
        )
        .alert(SMOLEN)
        .rule(
            Bid::new(3, Strain::Spades),
            140,
            len(Suit::Spades, 4..=4) & len(Suit::Hearts, 5..),
        )
        .alert(SMOLEN)
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
}

/// Opener accepts a no-fit (2NT) Stayman invitation with a maximum, else passes
///
/// Responder invited with a bare 8, so a 1NT opener needs its 17 for game (8+17
/// = 25).  Authored rather than left to the floor: the keyless floor reads a
/// three-level suit response over our 1NT as forcing and so cannot *decline* an
/// invitational raise.
pub(super) fn accept_invitation(game: Bid) -> Rules {
    Rules::new()
        .rule(game, 100, hcp(17..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's acceptance of an invitational major raise
///
/// With a maximum, bid the major game — but choose 3NT on a flat 4-3-3-3, where
/// notrump rates to play as well as the eight-card fit.  A minimum passes.
pub(super) fn accept_major_invitation(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 110, hcp(17..) & flat_4333())
        .rule(Bid::new(4, Strain::from(major)), 100, hcp(17..))
        .rule(Call::Pass, 0, hcp(0..))
}

#[cfg(test)]
mod tests;
