use super::super::tests::{best_call, call};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn opener_answers_cue_raise_instead_of_passing() {
    // 1♠ (2♣) 3♣ - (cue-raise = limit-plus spade raise): opener must not
    // leave the cuebid in. The screenshot deal's East (♠QT743 ♥KQ7 ♦832 ♣A9,
    // 11 HCP — a minimum) declines by signing off in 3♠, from the book.
    let auction = [
        call(1, Strain::Spades),
        call(2, Strain::Clubs),
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (c, floored) = best_call(&auction, "QT743.KQ7.832.A9");
    assert_eq!(c, call(3, Strain::Spades));
    assert!(
        !floored,
        "opener's answer must come from the cue-raise book"
    );
}

#[test]
fn opener_answers_minor_cue_raise() {
    // 1♦ (2♣) 3♣ - (cue-raise = limit-plus diamond raise).
    // Minimum, no club stopper (12 HCP, ♣Q doubleton) → sign off 3♦.
    let auction = [
        call(1, Strain::Diamonds),
        call(2, Strain::Clubs),
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (c, floored) = best_call(&auction, "K43.Q43.AJ632.Q5");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the minor sign-off must come from the book");
    // Values + a club stopper (17 HCP, ♣Kx) → accept the best game, 3NT.
    let (c, floored) = best_call(&auction, "A54.Q43.AKJ32.K5");
    assert_eq!(c, call(3, Strain::Notrump));
    assert!(!floored, "the 3NT accept must come from the book");
}

#[test]
fn minor_cue_raise_decline_jumps_when_3m_is_below_the_cue() {
    // 1♣ (2♦) 3♦ - (cue-raise = limit-plus club raise): 3♣ now sits
    // *below* the cue and is illegal, so a minimum opener must decline in 4♣,
    // not pass the cuebid out. Guards the 4m fallback rung.
    let auction = [
        call(1, Strain::Clubs),
        call(2, Strain::Diamonds),
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, floored) = best_call(&auction, "A32.K43.43.KQ432");
    assert_eq!(c, call(4, Strain::Clubs));
    assert!(!floored, "the 4♣ decline must come from the book");
}
