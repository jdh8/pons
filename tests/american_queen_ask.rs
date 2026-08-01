//! Integration tests for the queen relay (`set_queen_ask`): the whole
//! conversation played through the real [`Stance`], not a bare rule table.
//!
//! Per-node checks miss whole families, and a book node with finite mass
//! shadows the floor completely — so the questions these answer are "does the
//! relay survive the trie's fallback chain" and "does the knob's off state
//! leave the shipped auction alone", neither of which a unit test can see.
//!
//! The knob spans two regimes, and a harness has to arm **both**: rule presence
//! is gated at book-construction time, and the floor's recognizers read the
//! flag at classification time.  [`armed`] sets it and leaves it set for the
//! whole test body, which is what `set_kickback`'s docs ask of a harness.

mod common;
use common::*;
use pons::bidding::instinct::{set_king_zero_jump, set_queen_ask};

const P: Call = Call::Pass;

/// A stance built with the relay authored, with the flag left set so the
/// classification-time half is armed too.
fn armed() -> impl System {
    set_queen_ask(true);
    stance()
}

/// `1♠ P 3♠ P 4NT P 5♣ P` — spades agreed by the limit raise, 4NT asks, and
/// partner's 5♣ is one-or-four.  Three keycards of our own decodes to four
/// combined: one keycard missing, and the trump queen still an open question.
fn after_the_answer() -> Vec<Call> {
    vec![
        call(1, Strain::Spades),
        P,
        call(3, Strain::Spades),
        P,
        call(4, Strain::Notrump),
        P,
        call(5, Strain::Clubs),
        P,
    ]
}

/// ♠AKJ85 ♥AK2 ♦KJ2 ♣42 — three keycards (♠A, ♥A, ♠K), no trump queen.
const QUEENLESS_ASKER: &str = "AKJ85.AK2.KJ2.42";

/// Off the knob the shipped auction is untouched: four combined keycards bet
/// the small slam without ever asking about the queen.
#[test]
fn knob_off_bets_the_slam_blind() {
    set_queen_ask(false);
    let system = stance();
    assert_eq!(
        best_call(&system, &after_the_answer(), QUEENLESS_ASKER),
        call(6, Strain::Spades),
    );
}

/// On the knob the asker relays one step instead of guessing.
#[test]
fn relay_fires_through_the_stance() {
    let system = armed();
    assert_eq!(
        best_call(&system, &after_the_answer(), QUEENLESS_ASKER),
        call(5, Strain::Diamonds),
    );
}

/// Partner answers the relay, and the asker places the contract on the reply:
/// a denial leaves a keycard *and* the queen missing, so it stops at five.
#[test]
fn relay_denial_stops_at_five() {
    let system = armed();

    let mut auction = after_the_answer();
    auction.extend([call(5, Strain::Diamonds), P]);
    // ♠K74 ♥A653 ♦8432 ♣92 — three trumps opposite a shown five is eight, so
    // only the honour itself can answer, and it is missing.
    assert_eq!(
        best_call(&system, &auction, "K74.A653.8432.92"),
        call(5, Strain::Hearts),
    );

    auction.extend([call(5, Strain::Hearts), P]);
    assert_eq!(
        best_call(&system, &auction, QUEENLESS_ASKER),
        call(5, Strain::Spades),
    );
}

/// The queen shown brings the small slam.
#[test]
fn relay_queen_brings_the_slam() {
    let system = armed();

    let mut auction = after_the_answer();
    auction.extend([call(5, Strain::Diamonds), P]);
    assert_eq!(
        best_call(&system, &auction, "KQ4.A653.8432.92"),
        call(5, Strain::Spades),
    );

    auction.extend([call(5, Strain::Spades), P]);
    assert_eq!(
        best_call(&system, &auction, QUEENLESS_ASKER),
        call(6, Strain::Spades),
    );
}

/// A proven ten-card fit answers "queen" without the honour — the queen drops
/// or finesses either way once the side holds ten trumps.
#[test]
fn ten_card_fit_answers_the_queen() {
    let system = armed();

    let mut auction = after_the_answer();
    auction.extend([call(5, Strain::Diamonds), P]);
    // Five trumps opposite partner's shown five, and no queen in sight.
    assert_eq!(
        best_call(&system, &auction, "K7432.A65.843.92"),
        call(5, Strain::Spades),
    );
}

/// The buff jump: nine trumps is not a queen, but partner is about to pass five
/// without ever learning the fit is a card longer than promised.  Six of trumps
/// is the answer that neither claims the honour nor gets passed — and the asker
/// leaves it alone.
#[test]
fn ninth_trump_jumps_to_six() {
    let system = armed();

    let mut auction = after_the_answer();
    auction.extend([call(5, Strain::Diamonds), P]);
    // ♠K743 ♥A653 ♦843 ♣92 — four trumps opposite a shown five is nine.
    assert_eq!(
        best_call(&system, &auction, "K743.A653.843.92"),
        call(6, Strain::Spades),
    );

    auction.extend([call(6, Strain::Spades), P]);
    assert_eq!(
        best_call(&system, &auction, QUEENLESS_ASKER),
        P,
        "the jump places the contract: the asker has nothing left to say"
    );
}

/// A side-suit void rides the same jump — a trick the ladder has no rung for.
#[test]
fn a_void_jumps_to_six() {
    let system = armed();

    let mut auction = after_the_answer();
    auction.extend([call(5, Strain::Diamonds), P]);
    // ♠K74 ♥A6532 ♦8432 ♣— — an eight-card fit, no queen, but a club void.
    assert_eq!(
        best_call(&system, &auction, "K74.A6532.8432."),
        call(6, Strain::Spades),
    );
}

/// `set_king_zero_jump`: the queen-shown answerer with no side king places the
/// contract in six of trumps rather than answering on the bottom rung — and the
/// asker holding two side kings of its own still bids seven over it, because
/// the jump is a placement and not a barrier.
#[test]
fn zero_king_jump_still_leaves_seven_live() {
    set_king_zero_jump(true);
    let system = armed();

    // 1♠ P 3♠ P 4NT P 5♠ P — 5♠ is two-or-five *with* the queen, so the relay
    // is dead and the king ask is the asker's next move.
    let mut auction = vec![
        call(1, Strain::Spades),
        P,
        call(3, Strain::Spades),
        P,
        call(4, Strain::Notrump),
        P,
        call(5, Strain::Diamonds),
        P,
    ];
    // Over a none-or-three the ladder is: 5♥ relays, 5♠ denies, 5NT shows,
    // 6♣ asks for kings — and the zero-king answer is 6♠, the permuted top.
    auction.extend([call(5, Strain::Hearts), P, call(5, Strain::Notrump), P]);
    auction.extend([call(6, Strain::Clubs), P]);

    // ♠KQ43 ♥8653 ♦843 ♣92 — the queen, and not one side king.
    assert_eq!(
        best_call(&system, &auction, "KQ43.8653.843.92"),
        call(6, Strain::Spades),
        "queen and no side king: place it in six"
    );

    auction.extend([call(6, Strain::Spades), P]);
    // ♠AJ85 ♥AK2 ♦AK2 ♣42 — five keycards between the hands, the queen shown,
    // and two side kings in the asker's own hand: the grand gate is met.
    assert_eq!(
        best_call(&system, &auction, "AJ85.AK2.AK2.42"),
        call(7, Strain::Spades),
        "two side kings opposite zero: seven is still live over the jump"
    );
    set_king_zero_jump(false);
}

/// Seven is explored only when the values are already there: all five keycards
/// and the queen on a 15-count still stops in six, because RKCB is a slam veto
/// and not a slam seeker.
#[test]
fn king_ask_needs_the_grand_values() {
    let system = armed();

    let mut auction = after_the_answer();
    auction.extend([call(5, Strain::Diamonds), P, call(5, Strain::Spades), P]);
    // ♠AK985 ♥A32 ♦A42 ♣32 — four keycards, so all five are on the table.
    assert_eq!(
        best_call(&system, &auction, "AK985.A32.A42.32"),
        call(6, Strain::Spades),
    );
    // The same shape at 21 HCP has the values to spend the round.
    assert_eq!(
        best_call(&system, &auction, "AK985.AK2.AK2.32"),
        call(5, Strain::Notrump),
    );
}
