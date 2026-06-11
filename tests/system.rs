use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Level, Strain};
use pons::bidding::System;
use pons::bidding::array::Logits;
use pons::bidding::trie::{Forest, classifier};

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

const fn marker_logits(value: f32) -> Logits {
    let mut logits = Logits::new();
    *logits.0.get_mut(Call::Pass) = value;
    logits
}

fn classify_marker(system: &impl System, auction: &[Call]) -> Option<f32> {
    system
        .classify(Hand::default(), RelativeVulnerability::NONE, auction)
        .map(|logits| *logits.0.get(Call::Pass))
}

#[test]
fn test_forest_shares_first_and_second_seat() {
    let mut forest = Forest::new();
    forest
        .unpassed
        .insert(&[], classifier(|_, _| marker_logits(1.0)));
    forest
        .passed
        .insert(&[], classifier(|_, _| marker_logits(2.0)));

    let passes = [Call::Pass; 3];
    // 1st and 2nd seat openings share the unpassed root node,
    // 3rd and 4th the passed one.
    assert_eq!(classify_marker(&forest, &passes[..0]), Some(1.0));
    assert_eq!(classify_marker(&forest, &passes[..1]), Some(1.0));
    assert_eq!(classify_marker(&forest, &passes[..2]), Some(2.0));
    assert_eq!(classify_marker(&forest, &passes[..3]), Some(2.0));
}

#[test]
fn test_forest_defense_classes() {
    let mut forest = Forest::new();
    let one_s = bid(1, Strain::Spades);
    forest
        .unpassed
        .insert(&[one_s], classifier(|_, _| marker_logits(3.0)));
    forest
        .passed
        .insert(&[one_s], classifier(|_, _| marker_logits(4.0)));
    forest
        .unpassed
        .insert(&[one_s, Call::Pass], classifier(|_, _| marker_logits(5.0)));

    // Direct seat: they open 1♠ and our side has not passed.
    assert_eq!(classify_marker(&forest, &[one_s]), Some(3.0));
    // Our dealer passed before their 2nd-seat 1♠: we defend as a passed side.
    assert_eq!(classify_marker(&forest, &[Call::Pass, one_s]), Some(4.0));
    // The 2nd-seat opener's own side stays unpassed for its continuations,
    // and the stripped key [1♠, P] is shared with the 1st-seat book.
    assert_eq!(
        classify_marker(&forest, &[Call::Pass, one_s, Call::Pass]),
        Some(5.0),
    );
}

#[test]
fn test_forest_third_seat_continuation() {
    let mut forest = Forest::new();
    let one_h = bid(1, Strain::Hearts);
    forest
        .passed
        .insert(&[one_h, Call::Pass], classifier(|_, _| marker_logits(6.0)));

    // P-P-1♥-P: the 3rd-seat opener's side responds from the passed book.
    assert_eq!(
        classify_marker(&forest, &[Call::Pass, Call::Pass, one_h, Call::Pass]),
        Some(6.0),
    );
}
