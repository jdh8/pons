use super::super::call;
use super::super::tests::{best, best_on};
use super::*;
use crate::bidding::agreements::Agreements;
use crate::bidding::constraint::Constraint;

/// C2 pilot invariant: the native-[`EnvelopeUnion`] fit-split gate accepts exactly
/// the hands the composite it replaced accepted — per response suit,
/// opener's major, length floor, and gauge arm.
///
/// [`EnvelopeUnion`]: crate::bidding::inference::EnvelopeUnion
#[test]
fn jacoby_union_matches_composite() {
    use super::jacoby_box;
    use crate::bidding::constraint::{support, support_points};
    use crate::bidding::context::Context;
    use crate::bidding::verify;
    use contract_bridge::auction::{Call, RelativeVulnerability};
    use contract_bridge::{Bid, Strain, Suit};
    use rand::SeedableRng as _;
    use rand::rngs::StdRng;

    let mut rng = StdRng::seed_from_u64(0x2F18);
    for major in [Suit::Hearts, Suit::Spades] {
        let auction = [Call::Bid(Bid::new(1, Strain::from(major))), Call::Pass];
        let context = Context::new(RelativeVulnerability::NONE, &auction);
        let composite = support(4..) & support_points(major, 13..);
        let native = jacoby_box(major);
        let report = verify::compare(
            |hand| composite.eval(hand, &context).is_finite(),
            |hand| native.eval(hand, &context).is_finite(),
            &mut rng,
            4000,
        );
        assert!(
            report.agrees(),
            "jacoby box diverges over 1{major}: {:?}",
            report.disagreements,
        );
        assert!(
            report.reference_accepts > 0,
            "vacuous compare over 1{major}"
        );
    }
}

#[test]
fn major_responses_run_the_2_over_1_ladder() {
    let r = major_responses(Suit::Hearts, &Agreements::default());
    let a = [call(1, Strain::Hearts), Call::Pass];
    assert_eq!(best(&r, &a, "K2.KQ54.A964.Q92"), call(2, Strain::Notrump));
    assert_eq!(best(&r, &a, "Q32.J53.A964.Q92"), call(2, Strain::Hearts));
    assert_eq!(best(&r, &a, "A2.K3.Q543.KJ85"), call(2, Strain::Clubs));
}

#[test]
fn minor_response_three_notrump_reads_the_partial_table_floor_seam() {
    use crate::bidding::context::Context;
    use crate::bidding::trie::Classifier;
    use contract_bridge::Hand;
    use contract_bridge::auction::RelativeVulnerability;

    let rules = minor_responses(Suit::Diamonds, &Agreements::default());
    let auction = [call(1, Strain::Diamonds), Call::Pass];
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    let fallthrough: Hand = "T53.4.A74.AK6542".parse().expect("valid hand");
    assert!(!rules.classify(fallthrough, &context).has_mass());

    let reading = rules
        .rules()
        .iter()
        .find(|rule| rule.call() == call(3, Strain::Notrump))
        .expect("3NT rule")
        .project(&context);
    assert_eq!(reading.strength.hcp, Range::new(11, Range::FULL_POINTS.max));
    assert_eq!(reading.lengths[Suit::Hearts as usize].max, 3);
    assert_eq!(reading.lengths[Suit::Spades as usize].max, 3);
}

#[test]
fn choice_of_games_three_notrump() {
    let a = [call(1, Strain::Hearts), Call::Pass];
    let on = major_responses(Suit::Hearts, &Agreements::default());
    let mut without = Agreements::default();
    without.response.major_choice_of_games = false;
    let off = major_responses(Suit::Hearts, &without);

    // Flat (4333) with four trumps, 13 HCP: 3NT outranks Jacoby 2NT.
    assert_eq!(best(&on, &a, "K32.KQ54.A96.J92"), call(3, Strain::Notrump));
    assert_eq!(best(&off, &a, "K32.KQ54.A96.J92"), call(2, Strain::Notrump));
    // Flat (4333) with three trumps, 12 HCP: 3NT; off it is a forcing 1NT.
    assert_eq!(best(&on, &a, "K32.K54.A964.Q92"), call(3, Strain::Notrump));
    assert_eq!(best(&off, &a, "K32.K54.A964.Q92"), call(1, Strain::Notrump));
    // 4=3=3=3 over 1♥ keeps bidding 1♠ — the spade exclusion is load-bearing.
    assert_eq!(best(&on, &a, "KQ32.K54.A96.Q92"), call(1, Strain::Spades));
    assert_eq!(best(&off, &a, "KQ32.K54.A96.Q92"), call(1, Strain::Spades));
}

#[test]
fn two_over_one_fit_leg_and_gates() {
    use crate::bidding::constraint::PointScale;
    // Calibrated to the rule-of-N+8 opt-out — the scale the Points13 arm's
    // example hand assumes (the 6-4 reads 13, not the point-count 12).
    let a = [call(1, Strain::Hearts), Call::Pass];
    // Arms are relative to the legacy gate; the shipped default is
    // fit + Points13 (the `fit` arm below).
    let mut base = Agreements::default();
    base.decision.reading.point_scale = PointScale::RuleOfNFloored;
    let arm = |fit: bool, gate: TwoOverOneGate| {
        let mut agreements = base;
        agreements.response.two_over_one_fit = fit;
        agreements.response.two_over_one_gate = gate;
        major_responses(Suit::Hearts, &agreements)
    };
    let baseline = arm(false, TwoOverOneGate::Points13);
    let fit = arm(true, TwoOverOneGate::Points13);
    let hcp13 = arm(false, TwoOverOneGate::Hcp13);
    let hcp12 = arm(false, TwoOverOneGate::Hcp12);

    // Fit leg: exactly three trumps, 11 HCP + spade singleton reads 13
    // support points — a 2/1 preparing the heart raise; off, a 1NT.
    assert_eq!(
        best_on(&fit, &a, "7.K54.A964.KJ932", base.decision),
        call(2, Strain::Clubs)
    );
    assert_eq!(
        best_on(&baseline, &a, "7.K54.A964.KJ932", base.decision),
        call(1, Strain::Notrump)
    );
    // Hcp13 demotes a shaped 12 (6-4 reads 13 points) back to 1NT.
    assert_eq!(
        best_on(&baseline, &a, "32.Q4.AKJ964.Q93", base.decision),
        call(2, Strain::Diamonds)
    );
    assert_eq!(
        best_on(&hcp13, &a, "32.Q4.AKJ964.Q93", base.decision),
        call(1, Strain::Notrump)
    );
    // Hcp12 admits a no-fit flat 12 the shipped gate leaves in 1NT.
    assert_eq!(
        best_on(&baseline, &a, "K32.54.A964.KQ92", base.decision),
        call(1, Strain::Notrump)
    );
    assert_eq!(
        best_on(&hcp12, &a, "K32.54.A964.KQ92", base.decision),
        call(2, Strain::Clubs)
    );
}

#[test]
fn two_over_one_natural_lengths_and_light_major() {
    let a = [call(1, Strain::Spades), Call::Pass];
    // The major discount subtracts from an `Hcp*` floor (`hcp_floor -
    // discount`); the shipped `Points13` gate hardcodes `points(13..)` and
    // ignores it, so pin the raw-HCP gate this knob was designed against.
    let mut base = Agreements::default();
    base.response.two_over_one_gate = TwoOverOneGate::Hcp13;
    let arm = |natural_lengths: bool, discount: bool| {
        let mut agreements = base;
        agreements.response.two_over_one_natural_lengths = natural_lengths;
        agreements.response.two_over_one_major_discount = discount;
        major_responses(Suit::Spades, &agreements)
    };
    let nat = arm(true, false);
    let nat_light = arm(true, true);
    let baseline = arm(false, false);

    // 1♠ - 2♣ is the catch-all and may be three: a 2=4=4=3 game force bids
    // the cheaper club (weight 1.1) once three qualifies; on the uniform
    // 4+ floor it must show its four-card diamond instead.
    assert_eq!(
        best(&baseline, &a, "AK.KJ54.KJ54.432"),
        call(2, Strain::Diamonds)
    );
    assert_eq!(best(&nat, &a, "AK.KJ54.KJ54.432"), call(2, Strain::Clubs));

    // 1♠ - 2♥ promises five, and the discount lets a 12-HCP five-carder with
    // no spade fit force game; without it the no-fit floor is 13 and the
    // hand makes a forcing 1NT.
    assert_eq!(best(&nat, &a, "Q2.KQJ54.K32.J43"), call(1, Strain::Notrump));
    assert_eq!(
        best(&nat_light, &a, "Q2.KQJ54.K32.J43"),
        call(2, Strain::Hearts)
    );
}
