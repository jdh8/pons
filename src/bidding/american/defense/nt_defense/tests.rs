use super::super::tests::{best_call, call};
use crate::bidding::american::{NotrumpDefense, set_notrump_defense};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn always_pass_defense_passes_over_1nt() {
    // The always-pass baseline: a 15-count balanced hand that would normally make a
    // penalty double passes instead, and the Pass is a book node (not the floor)
    // so it shadows whatever the floor would have done over their 1NT.
    let over_1nt = [call(1, Strain::Notrump)];
    set_notrump_defense(NotrumpDefense::AlwaysPass);
    let (c, floored) = best_call(&over_1nt, "AQ32.KQ3.K32.Q32");
    set_notrump_defense(NotrumpDefense::Natural);
    assert_eq!(c, Call::Pass);
    assert!(!floored, "the always-pass must come from the book node");
}
