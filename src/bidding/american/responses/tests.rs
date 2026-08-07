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
