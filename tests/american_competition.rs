//! Integration tests for the competitive package of the 2/1 game-forcing system

use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};
use pons::american;
use pons::bidding::array::Logits;
use pons::bidding::{Family, Stance, System};

const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The 2/1 pair bound against natural opponents
fn stance() -> Stance {
    american().against(Family::NATURAL)
}

/// The single highest-logit call the system assigns the hand for the auction
fn best_call(system: &impl System, auction: &[Call], hand: &str) -> Call {
    let hand: Hand = hand.parse().expect("valid test hand");
    let logits: Logits = system
        .classify(hand, RelativeVulnerability::NONE, auction)
        .expect("system covers this auction");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty")
}

// ---------------------------------------------------------------------------
// Section 1: direct-seat response to their overcall (1♥ – 2♣)
// ---------------------------------------------------------------------------

#[test]
fn test_cue_bid_limit_raise() {
    // 1♥ – (2♣) – ?: 12 HCP, four hearts → 3♣ (cue bid = limit-plus raise)
    let system = stance();
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
    // 1♥ – (2♣) – ?: 6 HCP, four hearts → 3♥ (preemptive jump raise)
    let system = stance();
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
    // 1♥ – (2♣) – ?: 8 HCP, three hearts → 2♥ (single raise)
    let system = stance();
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
    // 1♥ – (2♣) – ?: 10 HCP, four spades → Double (negative double)
    let system = stance();
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
// Section 3: support doubles and redoubles (1♦ – P – 1♠ – ?)
// ---------------------------------------------------------------------------

#[test]
fn test_support_double() {
    // 1♦ – P – 1♠ – (2♣): 13 HCP, exactly 3 spades → Double (support double)
    let system = stance();
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
    // 1♦ – P – 1♠ – (2♣): 13 HCP, four spades → 2♠ (natural raise)
    let system = stance();
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
    // 1♦ – P – 1♠ – (X): 13 HCP, exactly 3 spades → Redouble (support redouble)
    let system = stance();
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
    // 1♥ – (2♣) – X – P: 12 HCP, four spades → 2♠ (answering the negative double)
    let system = stance();
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

// ---------------------------------------------------------------------------
// Section 5: the (2♦)-as-Multi counter-defense toggle (set_defense_to_2d_multi)
// ---------------------------------------------------------------------------

#[test]
fn test_multi_2d_double_is_values() {
    // 1NT – (2♦) – ?: 9 HCP, no five-card suit, four diamonds. Default (off) reads
    // 2♦ as natural diamonds; the default Optional double needs 2-3 of them, so a
    // four-diamond hand cannot fire and responder does not double. With the Multi
    // counter-defense on, 2♦ shows an unknown major and this values hand takes the
    // workhorse double. The toggle is read at book construction, so set it before
    // building each stance. (Four diamonds, not three: under the default Optional
    // style — 2-3 cards — a three-diamond hand would optional-double in *both* arms,
    // erasing the contrast.)
    let auction = &[call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let hand = "KJ4.Q73.J762.Q53";

    pons::bidding::american::set_defense_to_2d_multi(false);
    let off = best_call(&stance(), auction, hand);

    pons::bidding::american::set_defense_to_2d_multi(true);
    let on = best_call(&stance(), auction, hand);

    // Restore the default so the toggle never leaks to another test on this thread.
    pons::bidding::american::set_defense_to_2d_multi(false);

    assert_eq!(
        on,
        Call::Double,
        "Multi counter-defense doubles with values"
    );
    assert_ne!(off, Call::Double, "the natural-diamond default does not");
}

#[test]
fn competitive_4333_knob_gates_the_cue_stayman() {
    // 1NT-(2♥): a flat 4-3-3-3 with four spades and game values cues 3♥ (Stayman)
    // to dig out the 4-4 spade fit.  The competitive-4333 knob governs whether that
    // flat hand still cues, or is diverted to 3NT (the constructive 4333 rule).  The
    // toggle is read at book construction, so set it before building each stance,
    // then restore the default so it never leaks to another test on this thread.
    use pons::bidding::american::{Competitive4333, set_competitive_4333};
    let auction = &[call(1, Strain::Notrump), call(2, Strain::Hearts)];
    let cue = call(3, Strain::Hearts);
    // Flat 4333, four spades, game values.  The no-stopper hand cannot bid 3NT
    // (their hearts unguarded), so its cue is unambiguous; the stopper hand can.
    let no_stopper = "KQJ5.432.KQ3.Q43"; // 13 HCP, ♥432 unguarded
    let with_stopper = "KQJ5.K32.Q43.J43"; // 12 HCP, ♥K32 a stopper

    set_competitive_4333(Competitive4333::Allow);
    assert_eq!(
        best_call(&stance(), auction, no_stopper),
        cue,
        "Allow: a flat 4333 cues as usual"
    );

    set_competitive_4333(Competitive4333::Suppress);
    assert_ne!(
        best_call(&stance(), auction, no_stopper),
        cue,
        "Suppress: a flat 4333 never cues"
    );

    set_competitive_4333(Competitive4333::SuppressWithStopper);
    assert_ne!(
        best_call(&stance(), auction, with_stopper),
        cue,
        "SuppressWithStopper: a flat 4333 *with* a stopper is diverted to 3NT"
    );
    assert_eq!(
        best_call(&stance(), auction, no_stopper),
        cue,
        "SuppressWithStopper: a stopperless flat 4333 still cues to find the fit"
    );

    set_competitive_4333(Competitive4333::Suppress); // restore the default
}

// ---------------------------------------------------------------------------
// Section: opener's competitive long-suit rebid (`set_competitive_rebid`)
// ---------------------------------------------------------------------------

#[test]
fn competitive_rebid_reaches_the_missed_game() {
    // Dealer West, 1♦ (1♥) P (2♥): West holds a self-sufficient AKJT984 and by
    // default can only make a takeout double it does not have the shape for.
    // With the competitive rebid on, West shows the suit — and the *existing*
    // raise ladder then carries East (14 opposite a shown 6+) to the cold
    // diamond game (5♦ makes 11 tricks double-dummy). Both sides through the
    // real stance: the fix is opener's rebid alone, responder was never broken.
    use pons::bidding::instinct::set_competitive_rebid;
    set_competitive_rebid(true);
    // Isolate the competitive rebid: with Rule-of-20 light openings on (the
    // default) responder reads the 1♦ opener as 10+ and no longer drives the
    // thin 5♦, a separately-measured trade-off of opening lighter — not this
    // feature's concern, so pin the classic 12+ opener here.
    pons::bidding::american::set_rule_of_20(false);
    let system = stance();

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
    assert_eq!(
        best_call(&system, &after_rebid, "AKQ.T95.Q73.QJ95"),
        call(5, Strain::Diamonds),
        "responder raises the shown suit to the diamond game"
    );

    set_competitive_rebid(false); // restore the default
}

// ---------------------------------------------------------------------------
// Section 12b: systems-on over their double of our splinter
// ---------------------------------------------------------------------------

#[test]
fn doubled_splinter_runs_systems_on() {
    use pons::bidding::american::set_splinter_doubled;
    // Anchor board 2448 (Constructive/book/round-1 bucket #4 tail): opener holds
    // 16 HCP with four aces and five spades. 1♠ – (P) – 4♣ (splinter) – (X): with
    // the knob off the double reroutes opener to the competitive book, where it
    // fell to the floor and *passed* the doubled game force. Systems-on (the
    // shipped default) rebases the double back onto the undisturbed splinter tree,
    // so opener keycards toward the slam the field bids — identical to the call it
    // makes when the splinter is not doubled.
    let auction = [
        call(1, Strain::Spades),
        Call::Pass,
        call(4, Strain::Clubs),
        Call::Double,
    ];
    let hand = "A9543.AT75.A2.A4";

    set_splinter_doubled(false);
    let off = best_call(&stance(), &auction, hand);

    set_splinter_doubled(true);
    let on = best_call(&stance(), &auction, hand);

    set_splinter_doubled(true); // restore the shipped default (on)

    assert_eq!(
        off,
        Call::Pass,
        "the off arm leaves the doubled splinter in"
    );
    assert_eq!(
        on,
        call(4, Strain::Notrump),
        "systems-on drives Keycard Blackwood, never passing the game force"
    );
}
