//! Per-board scoring: final-contract extraction, signed NS scores, and IMPs

use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Bid, Contract, Penalty, Seat, Strain};
use ddss::{TrickCountRow, TrickCountTable};
use pons::scoring::{final_contract, imps, ns_score_bid, ns_score_contract};

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

fn auction(calls: impl IntoIterator<Item = Call>) -> Auction {
    let mut auction = Auction::new();
    auction.try_extend(calls).expect("test auction is legal");
    auction
}

#[test]
fn test_final_contract_doubled() {
    // (2♠) X passed out, dealer West: 2♠X played by West.
    let auction = auction([
        bid(2, Strain::Spades),
        Call::Double,
        Call::Pass,
        Call::Pass,
        Call::Pass,
    ]);
    assert_eq!(
        final_contract(&auction, Seat::West),
        Some((
            Contract::new(2, Strain::Spades, Penalty::Doubled),
            Seat::West
        ))
    );
}

#[test]
fn test_final_contract_bid_resets_penalty() {
    // 1♥ X 1♠ all pass: the double applies to 1♥, not to the final 1♠.
    let auction = auction([
        bid(1, Strain::Hearts),
        Call::Double,
        bid(1, Strain::Spades),
        Call::Pass,
        Call::Pass,
        Call::Pass,
    ]);
    assert_eq!(
        final_contract(&auction, Seat::North),
        Some((
            Contract::new(1, Strain::Spades, Penalty::Undoubled),
            Seat::South
        ))
    );
}

#[test]
fn test_final_contract_declarer_named_strain_first() {
    // North opens 1♥, South raises to 4♥: North declares.
    let auction = auction([
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(4, Strain::Hearts),
        Call::Pass,
        Call::Pass,
        Call::Pass,
    ]);
    assert_eq!(
        final_contract(&auction, Seat::North),
        Some((
            Contract::new(4, Strain::Hearts, Penalty::Undoubled),
            Seat::North
        ))
    );
}

#[test]
fn test_final_contract_pass_out() {
    let auction = auction([Call::Pass; 4]);
    assert_eq!(final_contract(&auction, Seat::North), None);
}

#[test]
fn test_ns_score_contract_signs_and_vulnerability() {
    // Every declarer takes 9 tricks in notrump.
    let row = TrickCountRow::new(9, 9, 9, 9);
    let table = TrickCountTable([row; 5]);
    let three_nt = Contract::new(3, Strain::Notrump, Penalty::Undoubled);

    // 3NT making by South: +400 nonvulnerable, +600 vulnerable.
    let by_south = Some((three_nt, Seat::South));
    assert_eq!(
        ns_score_contract(by_south, &table, AbsoluteVulnerability::NONE),
        400
    );
    assert_eq!(
        ns_score_contract(by_south, &table, AbsoluteVulnerability::NS),
        600
    );

    // The same contract by West flips the sign and reads EW vulnerability.
    let by_west = Some((three_nt, Seat::West));
    assert_eq!(
        ns_score_contract(by_west, &table, AbsoluteVulnerability::NS),
        -400
    );
    assert_eq!(
        ns_score_contract(by_west, &table, AbsoluteVulnerability::EW),
        -600
    );

    // A pass-out scores 0.
    assert_eq!(
        ns_score_contract(None, &table, AbsoluteVulnerability::ALL),
        0
    );
}

#[test]
fn test_ns_score_bid_perfect_defense_doubling() {
    let three_nt = Bid::new(3, Strain::Notrump);

    // Making (9 tricks): undoubled, identical to the plain-DD contract scorer.
    let makes = TrickCountTable([TrickCountRow::new(9, 9, 9, 9); 5]);
    assert_eq!(
        ns_score_bid(
            Some((three_nt, Seat::South)),
            &makes,
            AbsoluteVulnerability::NONE
        ),
        400
    );

    // Failing (7 tricks, down 2): scored *doubled* — −300, not the undoubled
    // −100 a plain-DD contract scorer would give.
    let fails = TrickCountTable([TrickCountRow::new(7, 7, 7, 7); 5]);
    assert_eq!(
        ns_score_bid(
            Some((three_nt, Seat::South)),
            &fails,
            AbsoluteVulnerability::NONE
        ),
        -300
    );
    let undoubled = Contract::new(3, Strain::Notrump, Penalty::Undoubled);
    assert_eq!(
        ns_score_contract(
            Some((undoubled, Seat::South)),
            &fails,
            AbsoluteVulnerability::NONE
        ),
        -100
    );

    // Pass-out scores 0.
    assert_eq!(ns_score_bid(None, &makes, AbsoluteVulnerability::ALL), 0);
}

#[test]
fn test_imps_scale() {
    assert_eq!(imps(0), 0);
    assert_eq!(imps(19), 0);
    assert_eq!(imps(20), 1);
    assert_eq!(imps(-20), -1);
    assert_eq!(imps(440), 10);
    assert_eq!(imps(-450), -10);
    assert_eq!(imps(4000), 24);
    assert_eq!(imps(-100_000), -24);
}
