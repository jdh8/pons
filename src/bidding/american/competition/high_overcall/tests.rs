use super::super::tests::{best_call, call};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn high_overcalls_get_a_structure() {
    super::high_overcall::set_high_overcall_responses(true);
    // [1♠, (3♦)]: 4 hearts + 12 HCP make the 3-level negative double; a
    // diamond stopper + 16 bids 3NT instead.
    let auction = [call(1, Strain::Spades), call(3, Strain::Diamonds)];
    let (neg, floored) = best_call(&auction, "K5.KQ54.965.A432");
    assert_eq!(neg, Call::Double, "the 3-level negative double");
    assert!(!floored, "an authored node, not the floor");
    let (game, _) = best_call(&auction, "K5.KQ54.A65.A432");
    assert_eq!(game, call(3, Strain::Notrump), "3NT with a stopper");
    // Opener answers the forcing double with the unbid major.
    let answer = [
        call(1, Strain::Spades),
        call(3, Strain::Diamonds),
        Call::Double,
        Call::Pass,
    ];
    let (major, _) = best_call(&answer, "AQ542.KJ54.96.32");
    assert_eq!(major, call(3, Strain::Hearts), "four hearts answer 3♥");
    super::high_overcall::set_high_overcall_responses(false);
}
