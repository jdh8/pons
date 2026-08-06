use super::super::tests::{best_call, call};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn major_support_double_shows_three_spades() {
    super::support_double::set_major_support_double(true);
    // `1♥ - 1♠ (2♣)`: opener with exactly three spades doubles.
    let auction = [
        call(1, Strain::Hearts),
        Call::Pass,
        call(1, Strain::Spades),
        call(2, Strain::Clubs),
    ];
    let (support, floored) = best_call(&auction, "K32.AQ542.A95.32");
    assert_eq!(support, Call::Double, "exactly three = support double");
    assert!(!floored, "an authored node, not the floor");
    super::support_double::set_major_support_double(true);
}
