//! A rule whose call is illegal at its node is dropped when the book is built,
//! so it can no longer shadow the floor into a pass (`Rules::legal_after`).

mod common;
use common::*;

const P: Call = Call::Pass;

fn default_partnership() -> Partnership {
    american(&pons::bidding::agreements::Agreements::default()).bind()
}

/// `1♠ - 2♥ - 3♥ -`: the six-heart `3♥` rebid used to keep the node from the
/// floor, and responder passed a game force.
#[test]
fn forcing_raise_is_not_passed() {
    let auction = [
        call(1, Strain::Spades),
        P,
        call(2, Strain::Hearts),
        P,
        call(3, Strain::Hearts),
        P,
    ];
    assert_eq!(
        best_call(&default_partnership(), &auction, "4.QJT872.Q85.AK2"),
        call(4, Strain::Hearts)
    );
}

/// `1♠ - 3♠ - 4NT - 5♠ -`: the answer is the contract, so the sign-off is an
/// explicit pass rather than an illegal `5♠`.
#[test]
fn slam_sign_off_still_passes() {
    let auction = [
        call(1, Strain::Spades),
        P,
        call(3, Strain::Spades),
        P,
        call(4, Strain::Notrump),
        P,
        call(5, Strain::Spades),
        P,
    ];
    assert_eq!(
        best_call(&default_partnership(), &auction, "KQJ94.KQ5.KQ4.Q2"),
        P
    );
}
