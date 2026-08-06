use super::{hearts_first, hearts_take_first, spades_take_first};
use crate::bidding::constraint::Constraint;

#[test]
fn major_selectors_partition_every_holding() {
    // `hearts_first` is the exact complement of `spades_first`, so on any
    // pair of major lengths exactly one of the two selectors fires — the
    // guard that a future edit cannot let them overlap or leave a gap.
    for spades in 0..=13 {
        for hearts in 0..=13 - spades {
            assert_ne!(
                spades_take_first(spades, hearts),
                hearts_take_first(spades, hearts),
                "selectors overlap or gap at {spades}♠ {hearts}♥",
            );
        }
    }
    // Direction: 4-4 up the line to hearts, 5-5 high to spades, 5♠4♥ longer.
    assert!(hearts_take_first(4, 4));
    assert!(spades_take_first(5, 5));
    assert!(spades_take_first(5, 4));
}

#[test]
fn hearts_first_renders_positively() {
    // The point of the change: the 1♥ selector reads as positive prose, not
    // a negated `spades_first` ("not (…)").
    assert_eq!(
        hearts_first().describe().to_string(),
        "hearts longer than spades, or equal below five",
    );
}

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
