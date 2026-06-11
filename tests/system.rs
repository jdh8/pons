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

/// System always answering with a marker value
struct Constant(f32);

impl System for Constant {
    fn classify(&self, _: Hand, _: RelativeVulnerability, _: &[Call]) -> Option<Logits> {
        Some(marker_logits(self.0))
    }
}

/// System echoing the vulnerability it receives
struct VulProbe;

impl System for VulProbe {
    fn classify(&self, _: Hand, vul: RelativeVulnerability, _: &[Call]) -> Option<Logits> {
        Some(marker_logits(f32::from(vul.bits())))
    }
}

/// System with no answer at all
struct Silent;

impl System for Silent {
    fn classify(&self, _: Hand, _: RelativeVulnerability, _: &[Call]) -> Option<Logits> {
        None
    }
}

/// System answering with logits carrying no probability mass
struct NoMass;

impl System for NoMass {
    fn classify(&self, _: Hand, _: RelativeVulnerability, _: &[Call]) -> Option<Logits> {
        Some(Logits::new())
    }
}

fn classify_marker(system: &impl System, auction: &[Call]) -> Option<f32> {
    system
        .classify(Hand::default(), RelativeVulnerability::NONE, auction)
        .map(|logits| *logits.0.get(Call::Pass))
}

fn assert_marker_eq(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= f32::EPSILON);
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

#[test]
fn test_versus_dispatches_by_parity() {
    let table = Constant(1.0).vs(Constant(2.0));
    let passes = [Call::Pass; 3];

    assert_eq!(classify_marker(&table, &passes[..0]), Some(1.0));
    assert_eq!(classify_marker(&table, &passes[..1]), Some(2.0));
    assert_eq!(classify_marker(&table, &passes[..2]), Some(1.0));
    assert_eq!(classify_marker(&table, &passes[..3]), Some(2.0));
}

#[test]
fn test_versus_passes_vulnerability_through() {
    let table = VulProbe.vs(VulProbe);
    let passes = [Call::Pass; 2];

    for vul in [
        RelativeVulnerability::NONE,
        RelativeVulnerability::WE,
        RelativeVulnerability::THEY,
        RelativeVulnerability::ALL,
    ] {
        for len in 0..=2 {
            let logits = table
                .classify(Hand::default(), vul, &passes[..len])
                .expect("probe always answers");
            // Vulnerability is relative to the side to act: no flipping.
            assert_marker_eq(*logits.0.get(Call::Pass), f32::from(vul.bits()));
        }
    }
}

#[test]
fn test_versus_borrows_without_cloning() {
    let system = Constant(1.0);
    let table = (&system).vs(&system);
    assert_eq!(classify_marker(&table, &[]), Some(1.0));
    assert_eq!(classify_marker(&table, &[Call::Pass]), Some(1.0));
}

#[test]
fn test_or_else_falls_through_on_none() {
    let layered = Silent.or_else(Constant(9.0));
    assert_eq!(classify_marker(&layered, &[]), Some(9.0));
}

#[test]
fn test_or_else_falls_through_on_no_mass() {
    let layered = NoMass.or_else(Constant(9.0));
    assert_eq!(classify_marker(&layered, &[]), Some(9.0));
}

#[test]
fn test_or_else_prefers_first_answer_with_mass() {
    let layered = Constant(1.0).or_else(Constant(9.0));
    assert_eq!(classify_marker(&layered, &[]), Some(1.0));
}
