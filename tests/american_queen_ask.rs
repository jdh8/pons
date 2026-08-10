//! Integration tests for the queen relay: the whole
//! conversation played through the real [`Partnership`], not a bare rule table.
//!
//! Per-node checks miss whole families, and a book node with finite mass
//! shadows the floor completely — so the questions these answer are "does the
//! relay survive the trie's fallback chain" and "does the full system reach the
//! relay rungs in practice", neither of which a unit test can see.

mod common;
use common::*;

const P: Call = Call::Pass;

/// A partnership built with the shipped default system (the queen relay is always on).
fn armed() -> impl Bidder {
    partnership()
}

/// `1♠ - 3♠ - 4NT - 5♣ -` — spades agreed by the limit raise, 4NT asks, and
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

#[test]
fn relay_fires_through_the_partnership() {
    let system = armed();
    assert_eq!(
        best_call(&system, &after_the_answer(), QUEENLESS_ASKER),
        call(5, Strain::Diamonds),
    );
}

/// Partner answers the relay, and the asker places the contract on the reply.
/// The merged ladder puts the flat denial on **five of trump itself**, so the
/// answer and the signoff are the same call and the asker has nothing to add.
#[test]
fn relay_denial_stops_at_five() {
    let system = armed();

    let mut auction = after_the_answer();
    auction.extend([call(5, Strain::Diamonds), P]);
    // ♠K74 ♥A653 ♦8432 ♣92 — three trumps opposite a shown five is eight, so
    // only the honour itself can answer, and it is missing.
    assert_eq!(
        best_call(&system, &auction, "K74.A653.8432.92"),
        call(5, Strain::Spades),
    );

    auction.extend([call(5, Strain::Spades), P]);
    assert_eq!(
        best_call(&system, &auction, QUEENLESS_ASKER),
        P,
        "the denial is already the contract"
    );
}

/// The queen shown brings the small slam.
#[test]
fn relay_queen_brings_the_slam() {
    let system = armed();

    let mut auction = after_the_answer();
    auction.extend([call(5, Strain::Diamonds), P]);
    // The queen and not one side king — 5NT, the top of the merged ladder.
    assert_eq!(
        best_call(&system, &auction, "KQ4.A653.8432.92"),
        call(5, Strain::Notrump),
    );

    auction.extend([call(5, Strain::Notrump), P]);
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
        call(5, Strain::Notrump),
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

/// The merged reply names a king, so the asker holding one of its own has the
/// two the grand gate wants without spending another round.
#[test]
fn one_king_each_bids_the_grand_in_one_round() {
    let system = armed();

    let mut auction = after_the_answer();
    // 5♦ asks; 5♥ is the cheapest king rung — the queen plus the ♥K, and no
    // side king cheaper than it (there is none).
    auction.extend([call(5, Strain::Diamonds), P]);
    // ♠Q43 ♥K653 ♦8432 ♣92 — the queen and exactly the ♥K.
    assert_eq!(
        best_call(&system, &auction, "Q43.K653.8432.92"),
        call(5, Strain::Hearts),
    );

    auction.extend([call(5, Strain::Hearts), P]);
    // ♠AKJ85 ♥AK2 ♦A42 ♣32 — four keycards (all five between the hands), the
    // ♥K of its own opposite partner's, and the values to want seven.
    assert_eq!(
        best_call(&system, &auction, "AKJ85.AK2.A42.32"),
        call(7, Strain::Spades),
        "two side kings shown in one round: bid it",
    );
}

/// The second relay: with no side king of its own the asker cannot count two
/// off the reply alone, so it asks once more — one step above partner's reply,
/// which is the room a relocated ask buys twice.
#[test]
fn second_relay_finds_the_second_king() {
    let system = armed();

    let mut auction = after_the_answer();
    auction.extend([call(5, Strain::Diamonds), P, call(5, Strain::Hearts), P]);
    // ♠AKQJ8 ♥A32 ♦AQ2 ♣32 — four keycards, twenty points, and not one side
    // king: the second king is the whole question.
    assert_eq!(
        best_call(&system, &auction, "AKQJ8.A32.AQ2.32"),
        call(5, Strain::Spades),
        "no side king of our own: relay again",
    );

    auction.extend([call(5, Strain::Spades), P]);
    // ♠Q43 ♥K653 ♦K843 ♣92 — the ♥K it already showed, and the ♦K as well.
    assert_eq!(
        best_call(&system, &auction, "Q43.K653.K843.92"),
        call(5, Strain::Notrump),
        "a second side king: say so on the cheap rung",
    );
    // ♠Q43 ♥K653 ♦8432 ♣92 — only the king it showed; six of trumps ends it.
    assert_eq!(
        best_call(&system, &auction, "Q43.K653.8432.92"),
        call(6, Strain::Spades),
    );

    let mut grand = auction.clone();
    grand.extend([call(5, Strain::Notrump), P]);
    assert_eq!(
        best_call(&system, &grand, "AKQJ8.A32.AQ2.32"),
        call(7, Strain::Spades),
    );

    auction.extend([call(6, Strain::Spades), P]);
    assert_eq!(
        best_call(&system, &auction, "AKQJ8.A32.AQ2.32"),
        P,
        "one king short of the grand, and six is already the contract",
    );
}

/// Seven is explored only when the values are already there: all five keycards
/// and the queen on a 17-count still stops in six, because RKCB is a slam veto
/// and not a slam seeker.
#[test]
fn king_ask_needs_the_grand_values() {
    let system = armed();

    let mut auction = after_the_answer();
    auction.extend([call(5, Strain::Diamonds), P, call(5, Strain::Hearts), P]);
    // ♠AKQ85 ♥A32 ♦A42 ♣32 — four keycards and no side king, but seventeen
    // points is not a grand.
    assert_eq!(
        best_call(&system, &auction, "AKQ85.A32.A42.32"),
        call(6, Strain::Spades),
    );
}

/// The none-or-three lane: four keycards of our own is four **combined** —
/// partner answered none — so the denial stops at five and a king-showing
/// reply stops at six.  The missing keycard vetoes both the six over a denial
/// the one-or-four decode would bid and the seven its grand rule would try.
#[test]
fn none_or_three_decodes_the_total_through_the_partnership() {
    let system = armed();

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
    // ♠AKJ42 ♥A32 ♦AK5 ♣43 — four keycards, 19 HCP, no trump queen.
    assert_eq!(
        best_call(&system, &auction, "AKJ42.A32.AK5.43"),
        call(5, Strain::Hearts),
        "partner showed none: one keycard is missing, ask the queen",
    );

    let mut denied = auction.clone();
    denied.extend([call(5, Strain::Hearts), P, call(5, Strain::Spades), P]);
    assert_eq!(
        best_call(&system, &denied, "AKJ42.A32.AK5.43"),
        P,
        "queen denied on four combined: the denial is already the contract",
    );

    auction.extend([call(5, Strain::Hearts), P, call(6, Strain::Hearts), P]);
    assert_eq!(
        best_call(&system, &auction, "AKJ42.A32.AK5.43"),
        call(6, Strain::Spades),
        "one keycard is missing: six, never seven",
    );
}
