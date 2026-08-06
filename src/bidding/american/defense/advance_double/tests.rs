use super::super::tests::{best_call, call};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

/// The forced rung's priority is the **cheapest bid**, not the highest
/// rank: with no 4-card suit outside theirs, the advance keeps the auction
/// as low as possible ([`cheapest_forced`]).
#[test]
fn forced_three_card_advance_bids_the_cheapest() {
    let over_1s = [call(1, Strain::Spades), Call::Double, Call::Pass];
    let over_1c = [call(1, Strain::Clubs), Call::Double, Call::Pass];
    super::advance_rich::set_rich_advance_double(true);
    super::set_longest_first_advance(true);

    // Broke with four small of their spades and 3-3-3 outside: no sit (no
    // top honors), no 4-card rung — the forced rung bids 2♣, not 2♥.
    let (forced, floored) = best_call(&over_1s, "5432.432.432.432");
    assert_eq!(forced, call(2, Strain::Clubs), "forced → cheapest 2♣");
    assert!(!floored, "the forced advance is a book node, not the floor");

    // Over (1♣) every advance sits at the one level, so cheapest means
    // lowest-ranking: 1♦, not 1♠.
    let (forced, _) = best_call(&over_1c, "432.432.432.5432");
    assert_eq!(forced, call(1, Strain::Diamonds), "forced → cheapest 1♦");
}
