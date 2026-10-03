use super::super::tests::{best_call_with, call};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

fn on() -> Agreements {
    Agreements::default()
}

fn auction(opening: Strain, overcall: Strain) -> [Call; 4] {
    [
        call(1, opening),
        call(1, overcall),
        Call::Double,
        Call::Pass,
    ]
}

#[test]
fn minimum_fit_stays_low() {
    // The anchor's trace hand: the floor bid 4♠ on it.
    let (c, floored) = best_call_with(
        &on(),
        &auction(Strain::Diamonds, Strain::Hearts),
        "JT75.AQT.KJ973.T",
    );
    assert!(
        c == call(1, Strain::Spades) || c == call(2, Strain::Spades),
        "{c}"
    );
    assert!(!floored);
}

#[test]
fn fit_is_raised_by_strength() {
    let a = auction(Strain::Clubs, Strain::Hearts);
    assert_eq!(
        best_call_with(&on(), &a, "KQ75.43.AK4.QJ62").0,
        call(3, Strain::Spades)
    );
    assert_eq!(
        best_call_with(&on(), &a, "AQ75.4.AK4.KQJ62").0,
        call(4, Strain::Spades)
    );
    // Over (1♠) the heart raise starts at the two level.
    let a = auction(Strain::Clubs, Strain::Spades);
    assert_eq!(
        best_call_with(&on(), &a, "43.KQ75.K4.QJ762").0,
        call(2, Strain::Hearts)
    );
}

#[test]
fn four_of_their_suit_does_not_convert() {
    let a = auction(Strain::Clubs, Strain::Hearts);
    let (c, floored) = best_call_with(&on(), &a, "K4.KJ97.A3.Q9752");
    assert_eq!(c, call(1, Strain::Notrump));
    assert!(!floored);
    // No stopper, no fit, no long minor: complete the major on three or
    // rebid the minor — never pass.
    let (c, _) = best_call_with(&on(), &a, "Q42.9765.AK.KJ32");
    assert_ne!(c, Call::Pass);
}

#[test]
fn off_leaves_the_answer_to_the_floor() {
    let mut off = Agreements::default();
    off.competition.modern_double_answer = false;
    let a = auction(Strain::Diamonds, Strain::Hearts);
    let (_, floored) = best_call_with(&off, &a, "JT75.AQT.KJ973.T");
    assert!(floored);
}
