use super::*;
use contract_bridge::{Seat, Suit};

#[test]
fn enrich_thresholds_parse_as_hcp_then_fit() {
    assert_eq!(parse_enrich("28:9"), Ok((28, 9)));
    assert!(parse_enrich("28").is_err(), "no separator");
    assert!(parse_enrich("28:x").is_err(), "fit is not a number");
}

/// The raw-hand test must ignore spades: a spade ask is 4NT under either
/// card, so a spade fit carries no configuration signal and accepting on
/// one would spend the whole enriched slice on deals that cannot diverge.
#[test]
fn the_raw_hand_test_ignores_spades() {
    // North holds all thirteen spades and all his side's points; the best
    // *non*-spade fit at the table is E-W's nine (diamonds, and clubs).
    let deal: FullDeal = "N:AKQJT98765432... .98765.T987.T987 \
                          .AKQJT.AKQJ.AKQJ .432.65432.65432"
        .parse()
        .expect("a PBN deal, North first");
    assert_eq!(deal[Seat::North][Suit::Spades].len(), 13, "a 13-card fit");
    assert_eq!(
        slam_ish(&deal),
        (40, 9),
        "all forty points to N-S, and the fit is E-W's nine — not the spade thirteen",
    );
}
