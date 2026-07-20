//! Integration tests for the instinct floor under the 2/1 contested books
//!
//! These auctions have no authored rules — before the floor, the system
//! returned [`None`] and drivers passed by default.

mod common;
use common::*;

// --- The headline disaster, fixed -------------------------------------------

/// (3♣) X (P): the defensive book has no 3♣ entry, but the floor still
/// advances partner's takeout double instead of passing it out
#[test]
fn test_advance_of_takeout_double_over_preempt() {
    let system = stance();
    assert_eq!(
        best_call(
            &system,
            &[call(3, Strain::Clubs), Call::Double, Call::Pass],
            "92.J8532.9742.92",
        ),
        call(3, Strain::Hearts),
    );
}

// --- Their three-level preempts ----------------------------------------------

/// (3♦) direct seat: takeout double on shape with opening values
#[test]
fn test_takeout_double_of_preempt() {
    let system = stance();
    assert_eq!(
        best_call(&system, &[call(3, Strain::Diamonds)], "KQ32.AJ53.2.A942"),
        Call::Double,
    );
}

/// (3♦) direct seat: nothing to say with a weak hand
#[test]
fn test_pass_of_preempt_without_values() {
    let system = stance();
    assert_eq!(
        best_call(&system, &[call(3, Strain::Diamonds)], "Q5432.J53.942.92"),
        Call::Pass,
    );
}

// --- Competitive auctions past the authored package --------------------------

/// 1♥ (3♣): their jump overcall is past the negative-double package
/// (`OvercallAtMost(2♠)`), but responder still raises with a limit hand
///
/// With the bilans floor default-on the raise goes to game rather than the
/// limit 3♥ — the third competitive auction to move up a level when the floor
/// shipped (see `docs/ai-bidder/evaluator-net.md`).
#[test]
fn test_raise_over_jump_overcall() {
    let system = stance();
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Hearts), call(3, Strain::Clubs)],
            "Q32.K53.A964.Q92",
        ),
        call(4, Strain::Hearts),
    );
}

/// A deep contested continuation no book authors still gets an answer
#[test]
fn test_deep_contested_auction_is_covered() {
    let system = stance();
    let auction = [
        call(1, Strain::Hearts),
        call(2, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
        Call::Pass,
    ];
    let hand: Hand = "92.AQJ53.KQ42.96".parse().expect("valid test hand");
    assert!(
        system
            .classify(hand, RelativeVulnerability::NONE, &auction)
            .is_some()
    );
}

// --- Precedence ---------------------------------------------------------------

// --- Rubens advances: the floor owns advancing a simple overcall ------------

/// (1♣) 1♠ (P): advancing with our own five-card diamond suit and a good 9
/// (10+ upgraded points) transfers — 2♣ shows diamonds.  Reaches the floor now
/// that the book authors no `advances`.
#[test]
fn test_rubens_new_suit_transfer_through_system() {
    let system = stance();
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Clubs), call(1, Strain::Spades), Call::Pass],
            "2.K32.KQT54.J432",
        ),
        call(2, Strain::Clubs),
    );
}

/// (1♣) 1♠ (P): a limit raise of spades goes through the transfer into their
/// suit — 2♥, not a direct 2♠.
#[test]
fn test_rubens_limit_raise_through_system() {
    let system = stance();
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Clubs), call(1, Strain::Spades), Call::Pass],
            "K54.K32.K43.Q432",
        ),
        call(2, Strain::Hearts),
    );
}

/// (1♦) 1♠ (P): a weak six-card raise jumps preemptively to game.
#[test]
fn test_rubens_preemptive_raise_through_system() {
    let system = stance();
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Diamonds),
                call(1, Strain::Spades),
                Call::Pass
            ],
            "KJ7532.432.2.432",
        ),
        call(4, Strain::Spades),
    );
}

/// (1♠) 2♣ (P): a two-level overcall has no room for the ladder, so the cue
/// (2♠) is the limit-plus club raise.
#[test]
fn test_rubens_cue_raise_through_system() {
    let system = stance();
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Spades), call(2, Strain::Clubs), Call::Pass],
            "432.K32.K2.KQJ54",
        ),
        call(2, Strain::Spades),
    );
}

/// The floor never shadows an authored rule: the direct overcall node still
/// answers from the defensive book
#[test]
fn test_authored_rules_still_win() {
    let system = stance();
    // A light five-card major overcalls 1♠ over (1♦) — the authored
    // `defense_to_suit` answer, not an instinct one.
    assert_eq!(
        best_call(&system, &[call(1, Strain::Diamonds)], "AQJ32.853.42.K92"),
        call(1, Strain::Spades),
    );
}
