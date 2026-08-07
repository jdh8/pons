use crate::bidding::constraint::Constraint;

#[test]
fn fit_split_union_matches_composite() {
    use super::{fit_split_boxes, gauge_floor};
    use crate::bidding::constraint::{Cons, hcp, len, points, support, support_points};
    use crate::bidding::context::Context;
    use crate::bidding::inference::Strength;
    use crate::bidding::verify;
    use contract_bridge::auction::{Call, RelativeVulnerability};
    use contract_bridge::{Bid, Strain, Suit};
    use rand::SeedableRng as _;
    use rand::rngs::StdRng;

    fn check(
        context: &Context<'_>,
        rng: &mut StdRng,
        suit: Suit,
        min_len: usize,
        major: Suit,
        no_fit: Cons<impl Constraint + Clone>,
        floor: impl FnOnce(&mut Strength),
    ) {
        let composite = len(suit, min_len..)
            & !support(4..)
            & (no_fit | (support(3..) & support_points(major, 13..)));
        let native = fit_split_boxes(suit, min_len, major, floor);
        let report = verify::compare(
            |hand| composite.eval(hand, context).is_finite(),
            |hand| native.eval(hand, context).is_finite(),
            rng,
            4000,
        );
        assert!(
            report.agrees(),
            "fit-split diverges: 2{suit} over 1{major}, min_len {min_len}: {:?}",
            report.disagreements,
        );
        assert!(
            report.reference_accepts > 0,
            "vacuous compare: 2{suit} over 1{major}, min_len {min_len}",
        );
    }

    let mut rng = StdRng::seed_from_u64(0x2F17);
    for major in [Suit::Hearts, Suit::Spades] {
        let auction = [Call::Bid(Bid::new(1, Strain::from(major))), Call::Pass];
        let context = Context::new(RelativeVulnerability::NONE, &auction);
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
            if Strain::from(suit) >= Strain::from(major) {
                continue;
            }
            for min_len in [3, 4, 5] {
                let ctx = &context;
                let rng = &mut rng;
                check(
                    ctx,
                    rng,
                    suit,
                    min_len,
                    major,
                    points(13..),
                    gauge_floor(|s| &mut s.points, 13),
                );
                check(
                    ctx,
                    rng,
                    suit,
                    min_len,
                    major,
                    points(12..),
                    gauge_floor(|s| &mut s.points, 12),
                );
                check(
                    ctx,
                    rng,
                    suit,
                    min_len,
                    major,
                    hcp(13..),
                    gauge_floor(|s| &mut s.hcp, 13),
                );
                check(
                    ctx,
                    rng,
                    suit,
                    min_len,
                    major,
                    hcp(12..),
                    gauge_floor(|s| &mut s.hcp, 12),
                );
            }
        }
    }
}
