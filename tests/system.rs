use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Level, Strain};
use pons::bidding::array::Logits;
use pons::bidding::trie::classifier;
use pons::bidding::{Competitive, Constructive, Defensive, Family, Pair, System};

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
fn test_stance_routes_by_phase() {
    let one_s = bid(1, Strain::Spades);
    let two_h = bid(2, Strain::Hearts);

    let mut constructive = Constructive::new();
    constructive.insert(&[], classifier(|_, _| marker_logits(1.0)));
    constructive.insert(&[one_s, Call::Pass], classifier(|_, _| marker_logits(2.0)));

    let mut competitive = Competitive::new();
    competitive.insert(&[one_s, two_h], classifier(|_, _| marker_logits(4.0)));

    let mut defensive = Defensive::new();
    defensive.insert(&[one_s], classifier(|_, _| marker_logits(3.0)));

    let stance =
        Pair::new(Family::NATURAL, constructive, competitive, defensive).against(Family::NATURAL);

    // Nobody has opened: the opening decision is constructive.
    assert_eq!(classify_marker(&stance, &[]), Some(1.0));
    // We opened 1♠ and they only passed: still constructive.
    assert_eq!(classify_marker(&stance, &[one_s, Call::Pass]), Some(2.0));
    // We opened 1♠ and they overcalled 2♥: competitive.
    assert_eq!(classify_marker(&stance, &[one_s, two_h]), Some(4.0));
    // They opened 1♠: the defensive book answers.
    assert_eq!(classify_marker(&stance, &[one_s]), Some(3.0));
}

#[test]
fn test_books_gate_on_phase() {
    let one_s = bid(1, Strain::Spades);
    let two_h = bid(2, Strain::Hearts);

    // The constructive book holds the keys, but [1♠] is a they-opened auction
    // and [1♠, 2♥] is competitive, so its gate keeps it silent on both.
    let mut constructive = Constructive::new();
    constructive.insert(&[one_s], classifier(|_, _| marker_logits(1.0)));
    constructive.insert(&[one_s, two_h], classifier(|_, _| marker_logits(1.5)));
    assert_eq!(classify_marker(&constructive, &[one_s]), None);
    assert_eq!(classify_marker(&constructive, &[one_s, two_h]), None);

    // The competitive book answers only when we opened and they intervened.
    let mut competitive = Competitive::new();
    competitive.insert(&[], classifier(|_, _| marker_logits(2.0)));
    competitive.insert(&[one_s], classifier(|_, _| marker_logits(2.5)));
    competitive.insert(&[one_s, two_h], classifier(|_, _| marker_logits(3.0)));
    assert_eq!(classify_marker(&competitive, &[]), None);
    assert_eq!(classify_marker(&competitive, &[one_s]), None);
    assert_eq!(classify_marker(&competitive, &[one_s, two_h]), Some(3.0));

    // The defensive book holds the opening node, but [] is constructive, so
    // its gate keeps it silent.
    let mut defensive = Defensive::new();
    defensive.insert(&[], classifier(|_, _| marker_logits(4.0)));
    assert_eq!(classify_marker(&defensive, &[]), None);
}

#[test]
fn test_family_override_selects_book() {
    let one_c = bid(1, Strain::Clubs);

    let mut natural_defense = Defensive::new();
    natural_defense.insert(&[one_c], classifier(|_, _| marker_logits(1.0)));

    let mut special_defense = Defensive::new();
    special_defense.insert(&[one_c], classifier(|_, _| marker_logits(2.0)));

    let pair = Pair::new(
        Family::NATURAL,
        Constructive::new(),
        Competitive::new(),
        natural_defense,
    )
    .defensive_vs(Family::WEAK_NOTRUMP, special_defense);

    assert_eq!(
        classify_marker(&pair.against(Family::NATURAL), &[one_c]),
        Some(1.0)
    );
    assert_eq!(
        classify_marker(&pair.against(Family::WEAK_NOTRUMP), &[one_c]),
        Some(2.0)
    );
    // A family with no override gets the default defense.
    assert_eq!(
        classify_marker(&pair.against(Family::STRONG_CLUB), &[one_c]),
        Some(1.0)
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
