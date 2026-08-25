//! Integration tests for the competitive package of the 2/1 game-forcing system

mod common;
use common::*;

// ---------------------------------------------------------------------------
// Section 1: direct-seat response to their overcall, 1♥ (2♣)
// ---------------------------------------------------------------------------

#[test]
fn test_cue_bid_limit_raise() {
    // 1♥ (2♣) ?: 12 HCP, four hearts → 3♣ (cue bid = limit-plus raise)
    let system = partnership();
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Hearts), call(2, Strain::Clubs)],
            "K32.KQ54.A964.32"
        ),
        call(3, Strain::Clubs),
    );
}

#[test]
fn test_preemptive_jump_raise() {
    // 1♥ (2♣) ?: 6 HCP, four hearts → 3♥ (preemptive jump raise)
    let system = partnership();
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Hearts), call(2, Strain::Clubs)],
            "832.KJ75.Q9642.2"
        ),
        call(3, Strain::Hearts),
    );
}

#[test]
fn test_competitive_single_raise() {
    // 1♥ (2♣) ?: 8 HCP, three hearts → 2♥ (single raise)
    let system = partnership();
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Hearts), call(2, Strain::Clubs)],
            "832.KJ7.Q9642.Q2"
        ),
        call(2, Strain::Hearts),
    );
}

#[test]
fn test_negative_double_over_overcall() {
    // 1♥ (2♣) ?: 10 HCP, four spades → Double (negative double)
    let system = partnership();
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Hearts), call(2, Strain::Clubs)],
            "KQ32.J5.A964.982"
        ),
        Call::Double,
    );
}

// ---------------------------------------------------------------------------
// Section 3: support doubles and redoubles after 1♦ - 1♠ (2♣/X) ?
// ---------------------------------------------------------------------------

#[test]
fn test_support_double() {
    // 1♦ - 1♠ (2♣): 13 HCP, exactly 3 spades → Double (support double)
    let system = partnership();
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Diamonds),
                Call::Pass,
                call(1, Strain::Spades),
                call(2, Strain::Clubs),
            ],
            "K32.AQ5.A9642.32"
        ),
        Call::Double,
    );
}

#[test]
fn test_support_raise() {
    // 1♦ - 1♠ (2♣): 13 HCP, four spades → 2♠ (natural raise)
    let system = partnership();
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Diamonds),
                Call::Pass,
                call(1, Strain::Spades),
                call(2, Strain::Clubs),
            ],
            "K432.AQ5.A9642.2"
        ),
        call(2, Strain::Spades),
    );
}

#[test]
fn test_support_redouble() {
    // 1♦ - 1♠ (X): 13 HCP, exactly 3 spades → Redouble (support redouble)
    let system = partnership();
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Diamonds),
                Call::Pass,
                call(1, Strain::Spades),
                Call::Double,
            ],
            "K32.AQ5.A9642.32"
        ),
        Call::Redouble,
    );
}

// ---------------------------------------------------------------------------
// Section 4: opener answers partner's negative double of a minor overcall
// ---------------------------------------------------------------------------

#[test]
fn test_answer_negative_double_bids_other_major() {
    // `1♥ (2♣) X -`: 12 HCP, four spades → 2♠ (answering the negative double)
    let system = partnership();
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Hearts),
                call(2, Strain::Clubs),
                Call::Double,
                Call::Pass,
            ],
            "KQ32.AQJ54.94.32"
        ),
        call(2, Strain::Spades),
    );
}

#[test]
fn competitive_4333_knob_gates_the_cue_stayman() {
    // 1NT (2♥): a flat 4-3-3-3 with four spades and game values cues 3♥ (Stayman)
    // to dig out the 4-4 spade fit.  The competitive-4333 knob governs whether that
    // flat hand still cues, or is diverted to 3NT (the constructive 4333 rule).  The
    // field is read at book construction, so each arm builds its own partnership.
    use pons::bidding::american::Competitive4333;
    let arm = |school| {
        let mut agreements = pons::bidding::agreements::Agreements::default();
        agreements.competition.competitive_4333 = school;
        american(&agreements).bind()
    };
    let auction = &[call(1, Strain::Notrump), call(2, Strain::Hearts)];
    let cue = call(3, Strain::Hearts);
    // Flat 4333, four spades, game values.  The no-stopper hand cannot bid 3NT
    // (their hearts unguarded), so its cue is unambiguous; the stopper hand can.
    let no_stopper = "KQJ5.432.KQ3.Q43"; // 13 HCP, ♥432 unguarded
    let with_stopper = "KQJ5.K32.Q43.J43"; // 12 HCP, ♥K32 a stopper

    assert_eq!(
        best_call(&arm(Competitive4333::Allow), auction, no_stopper),
        cue,
        "Allow: a flat 4333 cues as usual"
    );

    assert_ne!(
        best_call(&arm(Competitive4333::Suppress), auction, no_stopper),
        cue,
        "Suppress: a flat 4333 never cues"
    );

    let with_stopper_arm = arm(Competitive4333::SuppressWithStopper);
    assert_ne!(
        best_call(&with_stopper_arm, auction, with_stopper),
        cue,
        "SuppressWithStopper: a flat 4333 *with* a stopper is diverted to 3NT"
    );
    assert_eq!(
        best_call(&with_stopper_arm, auction, no_stopper),
        cue,
        "SuppressWithStopper: a stopperless flat 4333 still cues to find the fit"
    );
}

// ---------------------------------------------------------------------------
// Section: opener's competitive long-suit rebid
// ---------------------------------------------------------------------------

#[test]
fn competitive_rebid_reaches_the_missed_game() {
    // Dealer West, 1♦ (1♥) - (2♥): West holds a self-sufficient AKJT984 and by
    // default can only make a takeout double it does not have the shape for.
    // With the competitive rebid on, West shows the suit — and the *existing*
    // raise ladder then carries East (14 opposite a shown 6+) to the cold
    // diamond game (5♦ makes 11 tricks double-dummy). Both sides through the
    // real partnership: the fix is opener's rebid alone, responder was never broken.
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.instinct.competitive_rebid = true;
    let system = american(&agreements).bind();

    let after_raise = [
        call(1, Strain::Diamonds),
        call(1, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Hearts),
    ];
    assert_eq!(
        best_call(&system, &after_raise, "765.A.AKJT984.63"),
        call(3, Strain::Diamonds),
        "opener rebids the seven-card suit instead of doubling"
    );

    let after_rebid = [
        call(1, Strain::Diamonds),
        call(1, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Hearts),
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    // The raise's *level* is the evaluator's, and it has moved with each
    // measured regime: the legacy hull read carried to the 5♦ game, the F2b
    // hull-only twin stopped in 4♦, and the shipped v3 calls-tail twin
    // (2026-07-27, `win | win`) carries to 5♦ again — the game this very
    // fixture calls cold (11 tricks double-dummy).  Phase 3 briefly dropped it
    // back to 4♦: the substituted `1♦` opening stopped recording diamonds in
    // the lane's bid-history, so its own `3♦` rebid read as a *first* showing
    // (♦4+) instead of a rebid (♦6+).  All pins deliberate; what this test
    // guards is that the rebid puts the raise ladder in motion at all.
    assert_eq!(
        best_call(&system, &after_rebid, "AKQ.T95.Q73.QJ95"),
        call(5, Strain::Diamonds),
        "responder raises the shown suit"
    );
    let mut legacy_agreements = pons::bidding::agreements::Agreements::default();
    legacy_agreements.decision.reading.envelope_union = false;
    let legacy_system = american(&legacy_agreements).bind();
    assert_eq!(
        best_call(&legacy_system, &after_rebid, "AKQ.T95.Q73.QJ95"),
        call(5, Strain::Diamonds),
        "legacy hull read raises to the diamond game"
    );
}

// ---------------------------------------------------------------------------
// Section 12b: systems-on over their double of our splinter
// ---------------------------------------------------------------------------

#[test]
fn doubled_splinter_runs_systems_on() {
    // Anchor board 2448 (Constructive/book/round-1 bucket #4 tail): opener holds
    // 16 HCP with four aces and five spades. After the `1♠ - 4♣ (X)` splinter,
    // the knob off the double reroutes opener to the competitive book and the
    // floor decides. The shipped v6 floor passes this exact off-arm hand.
    // Systems-on (the default) rebases the double onto the undisturbed
    // splinter tree, so opener keycards instead of passing the game force.
    let auction = [
        call(1, Strain::Spades),
        Call::Pass,
        call(4, Strain::Clubs),
        Call::Double,
    ];
    let hand = "A9543.AT75.A2.A4";

    let arm = |systems_on| {
        let mut agreements = pons::bidding::agreements::Agreements::default();
        agreements.competition.splinter_doubled = systems_on;
        american(&agreements).bind()
    };
    let off = best_call(&arm(false), &auction, hand);
    let on = best_call(&arm(true), &auction, hand);

    assert_eq!(
        off,
        Call::Pass,
        "the shipped v6 floor passes this exact off-arm continuation"
    );
    assert_eq!(
        on,
        call(4, Strain::Notrump),
        "systems-on drives Keycard Blackwood, never passing the game force"
    );
}

#[test]
fn lebensohl_signoff_is_not_a_game_force() {
    // `1NT (2♠) 2NT - 3♣ - 3♦ -` — partner relayed with at most nine points and
    // signed off in the minor.  `opener_forced_past_invitation` reads the force
    // off auction *shape*, so it cannot tell this from a game-forcing
    // three-level bid: the flag makes the floor take its deterministic rail and
    // blast `3NT` on a bare minimum.  That is the N2 node — 16 of 18 boards,
    // −52 plain / −125 PD (docs/one-notrump-competitive.md §N2).
    //
    // The knob has to be exercised through the *book*, not the bare instinct
    // harness: partner's `points ≤ 8` comes from the alerted `2NT` relay's
    // projection, and a context with no authored overlay reads `0..37`.
    let signoff = [
        call(1, Strain::Notrump),
        call(2, Strain::Spades),
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    // A 15-count minimum holding the spade stopper the `3NT` rule gates on.
    // Even facing partner's *ceiling* this is 23 — two short of game.
    let minimum = "Q93.K43.AKJT.Q42";

    let arm = |ceiling_read| {
        let mut agreements = pons::bidding::agreements::Agreements::default();
        agreements.decision.instinct.forcing_ceiling_read = ceiling_read;
        american(&agreements).bind()
    };

    assert_eq!(
        best_call(&arm(false), &signoff, minimum),
        call(3, Strain::Notrump),
        "off, the shape-only force blasts game over a sign-off"
    );
    // On, assert only that the blast is gone: which call replaces it is the
    // floor's business and moves with every retrain.
    assert_ne!(
        best_call(&arm(true), &signoff, minimum),
        call(3, Strain::Notrump),
        "a partner capped below ten has not forced us to game"
    );

    // The control: partner's *direct* three-level suit promises `points(10..)`,
    // so the knob must leave that force alone.  Same seat, same hand.
    let forcing = [
        call(1, Strain::Notrump),
        call(2, Strain::Spades),
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&arm(true), &forcing, minimum),
        best_call(&arm(false), &forcing, minimum),
        "the ceiling read is inert where partner is unlimited"
    );
}

/// A slam-zone bid in competition is a control bid, not a rebid — so the
/// keycard ask keys on the eight-card fit, not the suit somebody merely named.
///
/// The Phase 3 A′′ run's worst board (docs/authored-reading-handoff.md):
/// `- 1♣ (1♠) 2♥ - 3♥ - 3♠ - 4♣ - 4NT - 5♠ -`, both vulnerable.  Reading
/// opener's floor `4♣` as a six-card club rebid made `keycard_trump` prefer
/// ♣ (3 + 6) to ♥ (5 + 3), so responder bid `6♣` doubled in a seven-card fit
/// instead of `6♥` in an eight-card fit holding AKQ: −18 IMPs.
#[test]
fn test_slam_zone_control_bid_does_not_hijack_the_trump_suit() {
    let system = partnership();
    // ♠A9 ♥A9874 ♦AJ8 ♣K84, opposite the 1♣ opener's ♠Q842 ♥KQ6 ♦93 ♣AJ73.
    let responder = "A9.A9874.AJ8.K84";
    let auction = [
        Call::Pass,
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        call(2, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
        call(4, Strain::Clubs),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
        call(5, Strain::Spades),
        Call::Pass,
    ];
    // Which call replaces it is the floor's business; only the phantom strain
    // is this test's business.
    assert_ne!(
        best_call(&system, &auction, responder),
        call(6, Strain::Clubs),
        "a 4♣ control bid opposite an agreed heart fit is not a club suit"
    );
}

// ---------------------------------------------------------------------------
// Section: the Kokish–Kraft counter to their (2♦) Multi (opt-in, N4-KK)
// ---------------------------------------------------------------------------

/// Walk a whole `1NT (2♦)` auction on the Kokish–Kraft arm, asserting **our**
/// call at every seat we own
///
/// The lane had no full-auction coverage at all; each unit test below
/// `competition::rubensohl` pins one node in isolation, which cannot catch a
/// key that never gets reached because the seat before it took a different
/// branch.  `theirs` supplies the opponents' scripted calls in order;
/// `expected` is our call at each of our turns, alternating opener/responder
/// from the `1NT` opening.
fn walk_kokish_kraft(opener: &str, responder: &str, theirs: &[Call], expected: &[Call]) {
    walk_kokish_kraft_with(None, opener, responder, theirs, expected)
}

/// [`walk_kokish_kraft`] with the `4m` slam try armed at a `points` floor
fn walk_kokish_kraft_with(
    slam: Option<u8>,
    opener: &str,
    responder: &str,
    theirs: &[Call],
    expected: &[Call],
) {
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.decision.their.two_diamonds_multi = true;
    agreements.competition.multi_kokish_kraft = true;
    agreements.competition.multi_minor_slam_try = slam;
    let system = american(&agreements).bind();

    let mut auction = vec![call(1, Strain::Notrump)];
    let mut theirs = theirs.iter();
    for (turn, want) in expected.iter().enumerate() {
        auction.push(*theirs.next().expect("a scripted opposing call"));
        let hand = if turn % 2 == 0 { responder } else { opener };
        let got = best_call(&system, &auction, hand);
        assert_eq!(
            got, *want,
            "turn {turn} of {auction:?}: {hand} should call {want}"
        );
        auction.push(got);
    }
}

#[test]
fn kokish_kraft_transfers_a_long_minor_and_finds_the_major_game() {
    // 1NT (2♦) 2NT - 3♣ - 3♦ - 4♥: the floorless club transfer is two-way, so
    // responder completes the picture with the source's clubs-plus-hearts step
    // and opener bids the game in the ten-card fit.  Nothing in this auction
    // exists on the shipped v7 lane, where 2NT is the weak relay.
    walk_kokish_kraft(
        "AQ2.KJ32.AQ3.432", // 16 balanced, four hearts
        "3.AQ32.32.AKQJ32", // 1-4-2-6, a solid six-card club suit
        &[
            call(2, Strain::Diamonds),
            Call::Pass,
            Call::Pass,
            Call::Pass,
        ],
        &[
            call(2, Strain::Notrump),  // responder: transfer to clubs
            call(3, Strain::Clubs),    // opener: the forced completion
            call(3, Strain::Diamonds), // responder: clubs + hearts, game-forcing
            call(4, Strain::Hearts),   // opener: the 4-4 heart game
        ],
    );
}

#[test]
fn kokish_kraft_minor_slam_try_reaches_the_minor_slam() {
    // 1NT (2♦) 2NT - 3♣ - 4♣ - 4NT - 5♦ - 6♣: the whole point of the rung.
    // On the shipped table this auction ends in `3NT` — the ladder has nothing
    // above it — and 33 combined points with a solid six-card suit stay there.
    // Every seat here is book: the try, opener's ask, the keycard answer and
    // the placement.
    walk_kokish_kraft_with(
        Some(13),
        "AQ32.KQ4.A43.J32", // 16 balanced, three small clubs
        "32.K42.A2.AKQJ32", // 17, a solid six-card club suit and two aces
        &[
            call(2, Strain::Diamonds),
            Call::Pass,
            Call::Pass,
            Call::Pass,
            Call::Pass,
            Call::Pass,
        ],
        &[
            call(2, Strain::Notrump),  // responder: transfer to clubs
            call(3, Strain::Clubs),    // opener: the forced completion
            call(4, Strain::Clubs),    // responder: the slam try, giving 3NT up
            call(4, Strain::Notrump),  // opener: a maximum asks keycard
            call(5, Strain::Diamonds), // responder: three keycards
            call(6, Strain::Clubs),    // opener: places the slam
        ],
    );
}

#[test]
fn kokish_kraft_doubles_then_penalizes_their_resolved_major() {
    // 1NT (2♦) X (2♥) - - X: the values double waits for the pass-or-correct
    // to name the suit, and the *repeated* double is penalty under K–K's
    // delayed-double split — where shipped v7 makes it takeout.  Opener, having
    // sat the first time on three trumps, sits again.
    walk_kokish_kraft(
        "AQ2.J32.AQ32.K32", // 16 balanced, only three hearts: it waits
        "432.KQ98.A32.Q32", // 11, four good hearts behind the overcaller
        &[
            call(2, Strain::Diamonds),
            call(2, Strain::Hearts),
            Call::Pass,
            Call::Pass,
        ],
        &[
            Call::Double, // responder: values, no shape promise
            Call::Pass,   // opener: three trumps is not a trump stack
            Call::Double, // responder: penalty, on the now-known suit
            Call::Pass,   // opener: sits for the penalty, it is not takeout
        ],
    );
}
