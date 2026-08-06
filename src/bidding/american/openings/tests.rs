use super::*;
use crate::bidding::context::Context;
use crate::bidding::trie::Classifier;
use contract_bridge::auction::RelativeVulnerability;

/// The highest-logit opening `rules` makes for a hand
fn opens(rules: &Rules, hand: &str) -> Call {
    let hand: Hand = hand.parse().expect("valid test hand");
    let context = Context::new(RelativeVulnerability::NONE, &[]);
    let logits = rules.classify(hand, &context);
    (&logits.0)
        .into_iter()
        .max_by(|(_, a): &(Call, &f32), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty")
}

#[test]
fn sub_ten_hcp_freaks_open_only_in_third_seat() {
    use crate::bidding::constraint::{PointScale, set_point_scale};
    // Calibrated to the rule-of-N+8 opt-out — the scale these example
    // hands' points assume (6-5 reads 12, not the point-count cap of 10).
    set_point_scale(PointScale::RuleOfNFloored);
    // ♠5 ♥JT952 ♦J ♣AK7652 — 9 HCP, 12 points (6-5).  Rule-of-N+8 alone
    // would walk it in the sound-opening front door; `hcp(10..)` bars it in
    // first seat, and no other rule takes it (points 12 is past the weak
    // two's band, and clubs never weak-two), so every logit goes dead and
    // the full book's floor passes.  Third seat opens it: 12 points clears
    // `points(11..)` and 9 HCP clears the legal `hcp(8..)`.
    let freak: Hand = "5.JT952.J.AK7652".parse().expect("valid test hand");
    let table = openings();

    let first = Context::new(RelativeVulnerability::NONE, &[]);
    assert!(
        (&table.classify(freak, &first).0)
            .into_iter()
            .all(|(_, logit)| logit.is_infinite()),
        "first seat rejects the sub-10-HCP freak (falls through to Pass)"
    );

    let third = Context::new(RelativeVulnerability::NONE, &[Call::Pass, Call::Pass]);
    let best = (&table.classify(freak, &third).0)
        .into_iter()
        .max_by(|(_, a): &(Call, &f32), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty");
    assert_eq!(
        best,
        Call::Bid(Bid::new(1, Strain::Hearts)),
        "third seat opens it light"
    );

    assert_eq!(
        opens(&table, "AT86.975.QJ4.AJ3"),
        Call::Bid(Bid::new(1, Strain::Clubs)),
        "a flat 12 still opens its better minor in first seat"
    );
    set_point_scale(PointScale::PointCount);
}

#[test]
fn wide_notrump_shape_gate() {
    let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
    let one_s = Call::Bid(Bid::new(1, Strain::Spades));
    let one_c = Call::Bid(Bid::new(1, Strain::Clubs));
    // 5422 / 6322, ~15–17 fifths, long suit a minor (joins the wide 1NT) or a
    // major (stays a suit); the long-minor 6322 also stays a suit.
    let five422_minor = "Q432.KQ.K2.AK432";
    let five422_major = "AK432.KQ.Q432.K2";
    let six322_minor = "Q2.K3.AQ4.KQ8765";
    let six322_major = "KQ8765.K3.AQ4.Q2";
    let balanced16 = "AQ32.K53.QJ4.A92";

    // Classic: only the balanced hand opens 1NT; the shapely ones open a suit.
    let narrow = openings_with(NotrumpShape::Balanced);
    assert_eq!(opens(&narrow, balanced16), one_nt);
    assert_eq!(opens(&narrow, five422_minor), one_c);
    assert_eq!(opens(&narrow, five422_major), one_s);
    assert_eq!(opens(&narrow, six322_minor), one_c);
    assert_eq!(opens(&narrow, six322_major), one_s);

    // Wide: the long-minor 5422 joins 1NT; majors and 6322 stay suits.
    let wide = openings_with(NotrumpShape::Wide);
    assert_eq!(opens(&wide, balanced16), one_nt);
    assert_eq!(opens(&wide, five422_minor), one_nt);
    assert_eq!(opens(&wide, five422_major), one_s);
    assert_eq!(opens(&wide, six322_minor), one_c);
    assert_eq!(opens(&wide, six322_major), one_s);

    // Wide6322 (default): the long-minor 6322 also joins 1NT; majors still stay suits.
    let wide6322 = openings_with(NotrumpShape::Wide6322);
    assert_eq!(opens(&wide6322, five422_minor), one_nt);
    assert_eq!(opens(&wide6322, five422_major), one_s);
    assert_eq!(opens(&wide6322, six322_minor), one_nt);
    assert_eq!(opens(&wide6322, six322_major), one_s);
}

#[test]
fn suppress_one_notrump_opens_a_minor() {
    let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
    let one_c = Call::Bid(Bid::new(1, Strain::Clubs));
    let balanced16 = "AQ32.K53.QJ4.A92"; // 4333, 16 HCP — a textbook 1NT opener

    // Default: opens 1NT.
    assert_eq!(opens(&openings(), balanced16), one_nt);

    // Suppressed: the same hand opens its minor — never 1NT, never Pass.
    set_open_one_notrump(false);
    let call = opens(&openings(), balanced16);
    set_open_one_notrump(true);
    assert_eq!(call, one_c);
}

#[test]
fn one_notrump_offshape_is_opt_in() {
    use contract_bridge::Seat;
    use contract_bridge::deck::full_deal;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    let baseline = openings();
    set_one_notrump_offshape(false);
    let explicit_off = openings();
    let context = Context::new(RelativeVulnerability::NONE, &[]);
    let mut rng = StdRng::seed_from_u64(0x1A70_FF51);
    for _ in 0..64 {
        let deal = full_deal(&mut rng);
        for hand in Seat::ALL.map(|seat| deal[seat]) {
            assert_eq!(
                baseline.classify(hand, &context).0,
                explicit_off.classify(hand, &context).0
            );
        }
    }

    let offshape = "AQ43.KQJ2.K532.J";
    let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
    assert_ne!(opens(&explicit_off, offshape), one_nt);
    set_one_notrump_offshape(true);
    assert_eq!(opens(&openings(), offshape), one_nt);
    set_one_notrump_offshape(false);
}

#[test]
fn weak_two_wild_is_opt_in() {
    use contract_bridge::Seat;
    use contract_bridge::deck::full_deal;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    let baseline = openings();
    set_weak_two_wild(false);
    let explicit_off = openings();
    let context = Context::new(RelativeVulnerability::NONE, &[]);
    let mut rng = StdRng::seed_from_u64(0x2EA4_71D2);
    for _ in 0..64 {
        let deal = full_deal(&mut rng);
        for hand in Seat::ALL.map(|seat| deal[seat]) {
            assert_eq!(
                baseline.classify(hand, &context).0,
                explicit_off.classify(hand, &context).0
            );
        }
    }

    let five_card = "KQJ98.743.52.842";
    let two_s = Call::Bid(Bid::new(2, Strain::Spades));
    assert_ne!(opens(&explicit_off, five_card), two_s);
    set_weak_two_wild(true);
    assert_eq!(opens(&openings(), five_card), two_s);
    set_weak_two_wild(false);
}

#[test]
fn sound_eleven_counts_open_one_of_a_suit() {
    use crate::bidding::constraint::{PointScale, set_point_scale};

    let one_s = Call::Bid(Bid::new(1, Strain::Spades));
    // 11 HCP, 5-2-4-2.  On the shipped raw-HCP+upgrade scale the wasted J9
    // doubleton voids the shape upgrade, leaving the hand at 11 points —
    // below the sole `points(12..=21)` opening — so it passes.
    let sound_11 = "AK986.J9.QJT6.64";
    assert_eq!(opens(&openings(), sound_11), Call::Pass);

    // On the rule-of-N+8 opt-out `points(12..)` *is* the Rule of 20 (11 + 9),
    // blind to the wasted J9, so the identity opens the same hand 1♠.
    set_point_scale(PointScale::RuleOfN);
    let call = opens(&openings(), sound_11);
    set_point_scale(PointScale::PointCount);
    assert_eq!(call, one_s);
}

#[test]
fn weak_two_hcp_band_gauges_raw_hcp() {
    let two_s = Call::Bid(Bid::new(2, Strain::Spades));
    // 9 HCP, 6-4-2-1: on the floored scale `points` = 9 + (10−8) = 11, so
    // the default `points(5..=10)` excludes it and — too weak for a 1-opener
    // — it passes.  Raw HCP 9 is a sound weak two the HCP band admits.
    let sound_nine = "KQ9832.KJ85.74.4";
    set_weak_two_hcp(None);
    assert_eq!(opens(&openings(), sound_nine), Call::Pass);
    set_weak_two_hcp(Some((5, 10)));
    assert_eq!(opens(&openings(), sound_nine), two_s);

    // A junky shapely light hand the shape-crediting default over-admits:
    // 4 HCP, 6-4-2-1 reads `points` = 4 + 2 = 6, so the default opens a 2♠
    // the raw-HCP band (4 < 5) correctly declines.
    let junk_four = "QJ9832.T985.74.J";
    set_weak_two_hcp(None);
    assert_eq!(opens(&openings(), junk_four), two_s);
    set_weak_two_hcp(Some((5, 10)));
    assert_eq!(opens(&openings(), junk_four), Call::Pass);

    set_weak_two_hcp(None);
}

#[test]
fn weak_two_eval_gauges_honor_location() {
    let two_s = Call::Bid(Bid::new(2, Strain::Spades));
    // Twin 6 HCP 6-3-2-2 hands: KQJ concentrated in the six-card suit
    // versus banished to the short suits.  Both read `points` 6, so the
    // shipped band opens both — the evaluator gauges tell them apart.
    let concentrated = "KQJ862.943.75.82";
    let scattered = "986432.94.KQ.J82";
    assert_eq!(opens(&openings(), concentrated), two_s);
    assert_eq!(opens(&openings(), scattered), two_s);

    // Discipline forms: prune the scattered hand, keep the concentrated
    // one (CCCC 8.10 vs 5.10; NLTC 9.5 vs 10.0 losers — probe-verified,
    // see `examples/probe-weak-two-eval.rs`).
    set_weak_two_eval(Some(WeakTwoEval::CcccFloor(7.0)));
    assert_eq!(opens(&openings(), concentrated), two_s);
    assert_eq!(opens(&openings(), scattered), Call::Pass);
    set_weak_two_eval(Some(WeakTwoEval::NltcCeil(9.5)));
    assert_eq!(opens(&openings(), concentrated), two_s);
    assert_eq!(opens(&openings(), scattered), Call::Pass);

    // Band (swap) forms replace `points` outright and win over the armed
    // HCP band.
    set_weak_two_hcp(Some((5, 10)));
    set_weak_two_eval(Some(WeakTwoEval::CcccBand(7.0, 13.0)));
    assert_eq!(opens(&openings(), concentrated), two_s);
    assert_eq!(opens(&openings(), scattered), Call::Pass);
    set_weak_two_eval(Some(WeakTwoEval::NltcBand(8.0, 9.5)));
    assert_eq!(opens(&openings(), concentrated), two_s);
    assert_eq!(opens(&openings(), scattered), Call::Pass);
    set_weak_two_hcp(None);

    // Byte-identical default restored.
    set_weak_two_eval(None);
    assert_eq!(opens(&openings(), scattered), two_s);
}

/// D1b: the box union behind each [`NotrumpShape`] variant accepts exactly
/// the shapes the legacy `balanced() | described(closure)` composite did,
/// exhaustively over the 560-shape length lattice.
#[test]
fn notrump_shape_boxes_match_closure() {
    use crate::bidding::constraint::for_each_shape;
    use contract_bridge::auction::RelativeVulnerability;

    let ctx = Context::new(RelativeVulnerability::NONE, &[]);
    for shape in [
        NotrumpShape::Balanced,
        NotrumpShape::Wide,
        NotrumpShape::Wide6322,
    ] {
        let gate = notrump_shape(shape);
        for_each_shape(|lengths, hand| {
            // The replaced composite, restated on lengths: balanced, or
            // the wide closure (5m422 / Wide6322's 6m322, long suit a
            // minor — ♣ and ♦ are indexes 0 and 1 in ASC order).
            let mut sorted = lengths;
            sorted.sort_unstable();
            let is_balanced = matches!(sorted, [3, 3, 3, 4] | [2, 3, 4, 4] | [2, 3, 3, 5]);
            let long = match (shape, sorted) {
                (NotrumpShape::Balanced, _) => 0,
                (_, [2, 2, 4, 5]) => 5,
                (NotrumpShape::Wide6322, [2, 2, 3, 6]) => 6,
                _ => 0,
            };
            let wide = long != 0 && (lengths[0] == long || lengths[1] == long);
            assert_eq!(
                gate.eval(hand, &ctx).is_finite(),
                is_balanced || wide,
                "{shape:?} disagrees at {lengths:?}",
            );
        });
    }
}

/// G0: the wide-minor 2NT shape is exactly `Wide6322` with the 5-card
/// majors removed — it drops the 5M(332) pan-handles (a 5-card major opens
/// one-of-a-major) and keeps the wide minors (5m422/6m322).
#[test]
fn two_notrump_wide_shape_drops_five_card_majors() {
    use crate::bidding::constraint::for_each_shape;
    use contract_bridge::auction::RelativeVulnerability;

    let ctx = Context::new(RelativeVulnerability::NONE, &[]);
    let wide6322 = notrump_shape(NotrumpShape::Wide6322);
    let g0 = two_notrump_wide_shape();
    for_each_shape(|lengths, hand| {
        // [C, D, H, S] in ASC order — majors are indexes 2 and 3.
        let majors_capped = lengths[2] <= 4 && lengths[3] <= 4;
        assert_eq!(
            g0.eval(hand, &ctx).is_finite(),
            wide6322.eval(hand, &ctx).is_finite() && majors_capped,
            "G0 shape disagrees at {lengths:?}",
        );
    });
}
