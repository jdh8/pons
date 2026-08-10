use super::*;
use contract_bridge::Hand;

fn calls(strs: &[&str]) -> Vec<Call> {
    strs.iter()
        .map(|c| c.parse().expect("valid test call"))
        .collect()
}

/// The winning rule's prose for `call` at this point of a North-dealt
/// auction, `""` when no authored rule serves it.
fn describe(auction: &[&str], hand: &str, call: &str) -> String {
    let stance = american(&pons::bidding::agreements::Agreements::default()).against();
    let auction = calls(auction);
    let hand: Hand = hand.parse().expect("valid test hand");
    let seat = Seat::ALL[auction.len() % 4];
    let rel = relative(AbsoluteVulnerability::NONE, seat);
    stance
        .explain_call(hand, rel, &auction, call.parse().expect("valid test call"))
        .and_then(|(_, rule)| rule)
        .map_or(String::new(), |rule| rule.description)
}

/// Opener's Ogust answer is gated on `suit_hcp` — the quality prose latch.
#[test]
fn ogust_answer_carries_quality_prose() {
    let desc = describe(&["2S", "P", "2NT", "P"], "KQJ982.843.75.62", "3D");
    assert!(quality_book(&desc), "quality prose not found in {desc:?}");
}

/// Responder's splinter pins the short suit to ≤1 in prose, and the
/// projected envelope caps it for partner's next look.
#[test]
fn splinter_shows_shortness() {
    let desc = describe(&["1S", "P"], "T984.AJ43.KQ42.7", "4C");
    assert!(
        shortness_book(&desc),
        "shortness prose not found in {desc:?}"
    );

    let auction = calls(&["1S", "P", "4C", "P"]);
    let rel = relative(AbsoluteVulnerability::NONE, Seat::North);
    // Envelope realization is a legacy-hull-walk property: the splinter's
    // shortness cap comes from the hand-written reader, which the envelope-union
    // regime's projection overlay does not yet carry (parked in
    // docs/dnf-migration.md — the cap is LOST knob-on, hull and boxes
    // both).  Pin the knob off for the realization assert.
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.decision.reading.envelope_union = false;
    let stance = american(&agreements).against();
    let inferences = stance.infer(rel, &auction);
    assert!(
        inferences
            .partner()
            .lengths
            .iter()
            .any(|range| range.max <= 1),
        "splinter did not cap any suit in partner's envelope"
    );
}

/// Responder's direct 3NT over their overcall of our 1NT (the lebensohl
/// node) is gated on `stopper_in`.
#[test]
fn direct_3nt_over_overcall_carries_stopper_prose() {
    let desc = describe(&["1NT", "2S"], "A542.84.KQ54.K93", "3NT");
    assert!(stopper_book(&desc), "stopper prose not found in {desc:?}");
}

/// The pure auction-shape latches: Ogust position, strong 2♣ opening, and
/// NT bid over an opponent's shown suit.
#[test]
fn structural_latches_fire_on_shape() {
    assert!(ogust_answer_position(&calls(&["2S", "P", "2NT", "P"])));
    assert!(!ogust_answer_position(&calls(&["1S", "P", "2NT", "P"])));

    let two_clubs: Call = "2C".parse().expect("valid test call");
    assert!(strong_two_clubs_opening(&calls(&["P"]), two_clubs));
    assert!(!strong_two_clubs_opening(&calls(&["1D", "P"]), two_clubs));

    let three_nt: Call = "3NT".parse().expect("valid test call");
    // North deals; South (same side as North) bids over East's 1♠.
    assert!(nt_after_their_suit(
        &calls(&["1H", "1S"]),
        three_nt,
        0,
        Seat::South
    ));
    assert!(!nt_after_their_suit(
        &calls(&["1H", "P"]),
        three_nt,
        0,
        Seat::South
    ));
}
