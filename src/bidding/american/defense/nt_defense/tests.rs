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
/// through 5-5 shape. The shipped floor of 8 passes it; `0` restores the overcall.
#[test]
fn natural_overcall_hcp_floor_cuts_the_shapely_tail() {
    let over_1nt = [call(1, Strain::Notrump)];
    let hand = "96.A9532.Q7543.3";
    let shipped = Agreements::default();
    assert_eq!(
        shipped.defense.natural_overcall_hcp_floor, 8,
        "shipped default-on 2026-08-23"
    );
    assert_eq!(
        best_call_with(&shipped, &over_1nt, hand).0,
        Call::Pass,
        "the shipped floor rejects 6 HCP even at 8 points"
    );
    let mut loose = Agreements::default();
    loose.defense.natural_overcall_hcp_floor = 0;
    assert_eq!(
        best_call_with(&loose, &over_1nt, hand).0,
        call(2, Strain::Hearts),
        "0 restores the untightened overcall"
    );
    // A 10-count with the same suit is above both candidate floors.
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

/// Shipped on, the book owns the seat; off, it returns to the instinct floor.
#[test]
fn natural_overcall_advance_ships_on_and_reverts() {
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    let hand = "J32.KJ4.T96.AQ95";
    let shipped = Agreements::default();
    assert!(
        shipped.defense.natural_overcall_advance_enabled,
        "shipped default-on 2026-08-23"
    );
    let (c, floored) = best_call_with(&shipped, &auction, hand);
    assert_eq!(c, Call::Pass);
    assert!(!floored, "the shipped default answers from the book node");
    let mut off = Agreements::default();
    off.defense.natural_overcall_advance_enabled = false;
    let (_, floored) = best_call_with(&off, &auction, hand);
    assert!(floored, "off returns the seat to the instinct floor");
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
