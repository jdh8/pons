//! Per-board scoring: final-contract extraction, signed NS scores, and IMPs

use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Bid, Contract, Penalty, Seat, Strain};
use ddss::{TrickCountRow, TrickCountTable};
use pons::scoring::{
    final_contract, imps, ns_score_bid, ns_score_contract, ns_score_pd_tricks, ns_score_tricks,
};

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
    // `(2♠) X - - -`, dealer West: 2♠X played by West.
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
    // `1♥ (X) 1♠ - - -`: the double applies to 1♥, not to the final 1♠.
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
    let three_nt = Contract::new(3, Strain::Notrump, Penalty::Undoubled);

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
    assert_eq!(
        ns_score_contract(
            Some((three_nt, Seat::South)),
            &fails,
            AbsoluteVulnerability::NONE
        ),
        -100
    );

    // Pass-out scores 0.
    assert_eq!(ns_score_bid(None, &makes, AbsoluteVulnerability::ALL), 0);
}

#[test]
fn test_ns_score_bid_ignores_table_double() {
    // The double-dummy-bidding rule: the penalty is re-derived from the outcome,
    // so a real `X`/`XX` on the table never reaches the score.
    let bare = Contract::new(2, Strain::Hearts, Penalty::Undoubled);
    let doubled = Contract::new(2, Strain::Hearts, Penalty::Doubled);
    let redoubled = Contract::new(2, Strain::Hearts, Penalty::Redoubled);
    let vul = AbsoluteVulnerability::NONE;

    // Making (8 tricks): 2♥X made is +470 at the table, but a double-dummy
    // opponent would not have doubled — scored as the undoubled +110.
    let makes = TrickCountTable([TrickCountRow::new(8, 8, 8, 8); 5]);
    for contract in [bare, doubled, redoubled] {
        assert_eq!(
            ns_score_bid(Some((contract, Seat::South)), &makes, vul),
            110
        );
    }
    assert_eq!(
        ns_score_contract(Some((doubled, Seat::South)), &makes, vul),
        470
    );

    // Failing (7 tricks, down 1): doubled −100 whatever the table said — the
    // undoubled −50 is floored and the redoubled −200 is cut back.
    let fails = TrickCountTable([TrickCountRow::new(7, 7, 7, 7); 5]);
    for contract in [bare, doubled, redoubled] {
        assert_eq!(
            ns_score_bid(Some((contract, Seat::South)), &fails, vul),
            -100
        );
    }
    // An EW declarer flips the sign.
    assert_eq!(ns_score_bid(Some((bare, Seat::West)), &fails, vul), 100);
}

#[test]
fn test_ns_score_pd_tricks_doubles_only_failures() {
    // The whole SD-PD bracket rests on one comparison — `tricks < 6 + level`.
    // A `<`/`<=` slip would double every *exactly made* game, and no harness
    // would panic: they would silently print a different number.
    let four_spades = Contract {
        bid: Bid::new(4, Strain::Spades),
        penalty: Penalty::Undoubled,
    };
    let vul = AbsoluteVulnerability::NONE;

    // Exactly making is undoubled: 420, not a doubled make.
    assert_eq!(ns_score_pd_tricks(four_spades, Seat::North, 10, vul), 420);
    assert_eq!(
        ns_score_pd_tricks(four_spades, Seat::North, 10, vul),
        ns_score_tricks(four_spades, Seat::North, 10, vul),
    );

    // One down is scored doubled: −100, not the undoubled −50.
    assert_eq!(ns_score_pd_tricks(four_spades, Seat::North, 9, vul), -100);

    // An EW declarer flips the sign, as in `ns_score_tricks`.
    assert_eq!(ns_score_pd_tricks(four_spades, Seat::West, 9, vul), 100);

    // The table penalty is ignored either way: a doubled make is priced as the
    // undoubled 420, a redoubled failure as the merely doubled −100.
    let doubled = Contract {
        bid: Bid::new(4, Strain::Spades),
        penalty: Penalty::Doubled,
    };
    let redoubled = Contract {
        bid: Bid::new(4, Strain::Spades),
        penalty: Penalty::Redoubled,
    };
    assert_eq!(ns_score_pd_tricks(doubled, Seat::North, 10, vul), 420);
    assert_eq!(ns_score_pd_tricks(redoubled, Seat::North, 9, vul), -100);
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
