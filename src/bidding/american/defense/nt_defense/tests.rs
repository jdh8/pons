use super::super::tests::{best_call_with, call};
use crate::bidding::agreements::Agreements;
use crate::bidding::american::NotrumpDefense;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn always_pass_defense_passes_over_1nt() {
    // The always-pass baseline: a 15-count balanced hand that would normally make a
    // penalty double passes instead, and the Pass is a book node (not the floor)
    // so it shadows whatever the floor would have done over their 1NT.
    let over_1nt = [call(1, Strain::Notrump)];
    let mut agreements = Agreements::default();
    agreements.decision.reading.notrump_defense = NotrumpDefense::AlwaysPass;
    let (c, floored) = best_call_with(&agreements, &over_1nt, "AQ32.KQ3.K32.Q32");
    assert_eq!(c, Call::Pass);
    assert!(!floored, "the always-pass must come from the book node");
}

/// M1: the raw-HCP floor cuts the tail `points(8..)` admits through distribution
///
/// `96.A9532.Q7543.3` is the forensic's worst `2♦` overcaller — 6 HCP, 8 points
/// through 5-5 shape. It overcalls on the shipped default and passes at floor 8.
#[test]
fn natural_overcall_hcp_floor_cuts_the_shapely_tail() {
    let over_1nt = [call(1, Strain::Notrump)];
    let hand = "96.A9532.Q7543.3";
    let shipped = Agreements::default();
    assert_eq!(
        shipped.defense.natural_overcall_hcp_floor, 0,
        "inert default"
    );
    assert_eq!(
        best_call_with(&shipped, &over_1nt, hand).0,
        call(2, Strain::Hearts),
        "the shipped default overcalls on 6 HCP / 8 points"
    );
    let mut tight = Agreements::default();
    tight.defense.natural_overcall_hcp_floor = 8;
    assert_eq!(best_call_with(&tight, &over_1nt, hand).0, Call::Pass);
    // A 9-count with the same suit is above both candidate floors.
    for k in [8, 9] {
        let mut agreements = Agreements::default();
        agreements.defense.natural_overcall_hcp_floor = k;
        assert_eq!(
            best_call_with(&agreements, &over_1nt, "96.AQ932.KJ543.3").0,
            call(2, Strain::Hearts),
            "floor {k} keeps the 10-count"
        );
    }
}

/// M2: the advance node replaces the floor, and it has no notrump rung
///
/// The forensic's worst advance: 11 flat HCP with three-card support bid `2NT`
/// into a 15–17 opener and played it four down. Under the node it passes.
#[test]
fn natural_overcall_advance_never_bids_notrump() {
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    let mut agreements = Agreements::default();
    agreements.defense.natural_overcall_advance_enabled = true;
    let (c, floored) = best_call_with(&agreements, &auction, "J32.KJ4.T96.AQ95");
    assert_eq!(c, Call::Pass);
    assert!(!floored, "the advance must come from the book node");
    // The rungs that do exist: a constructive raise, a game raise of a major,
    // and the two-level misfit escape.
    assert_eq!(
        best_call_with(&agreements, &auction, "A32.KJ4.Q984.AQ9").0,
        call(3, Strain::Diamonds),
        "four-card support with 12+ raises"
    );
    assert_eq!(
        best_call_with(&agreements, &auction, "KQJ98.KJ4.4.AQ95").0,
        call(2, Strain::Spades),
        "a 5-card suit opposite a singleton escapes at the two level"
    );
    let hearts = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        Call::Pass,
    ];
    assert_eq!(
        best_call_with(&agreements, &hearts, "A32.Q984.KJ4.AQ9").0,
        call(4, Strain::Hearts),
        "four-card major support with 14+ bids game"
    );
}

/// Off, the advance node is absent and the floor still owns the seat.
#[test]
fn natural_overcall_advance_is_default_off() {
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    let agreements = Agreements::default();
    assert!(!agreements.defense.natural_overcall_advance_enabled);
    let (_, floored) = best_call_with(&agreements, &auction, "J32.KJ4.T96.AQ95");
    assert!(floored, "the shipped default leaves the seat to the floor");
}

/// The advance package's row invariants, evaluated with its gate open.
#[test]
fn natural_overcall_advance_package_invariants() {
    let mut agreements = Agreements::default();
    agreements.defense.natural_overcall_advance_enabled = true;
    crate::bidding::rows::assert_package_invariants(
        &agreements,
        &[super::natural_overcall_advance_package()],
    );
}
