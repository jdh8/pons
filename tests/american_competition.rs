//! Integration tests for the competitive package of the 2/1 game-forcing system

use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};
use pons::american;
use pons::bidding::array::Logits;
use pons::bidding::{Stance, System, Tag};

const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The 2/1 pair bound against natural opponents
fn stance() -> Stance {
    american().against(Tag::NATURAL)
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
