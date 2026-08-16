use super::*;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Level, Strain};

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

fn hand(s: &str) -> Hand {
    s.parse().expect("valid test hand")
}

fn assert_feature_bits<const N: usize>(actual: [f32; N], reference: Vec<f32>) {
    assert_eq!(reference.len(), N);
    assert!(
        actual
            .iter()
            .zip(reference)
            .all(|(actual, reference)| actual.to_bits() == reference.to_bits())
    );
}

#[test]
fn fixed_evaluator_extractors_match_the_vec_reference_bit_for_bit() {
    let cards = hand("AQ32.K53.QJ4.A92");
    let auction = [
        bid(1, Strain::Clubs),
        Call::Pass,
        bid(1, Strain::Spades),
        Call::Double,
    ];
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    let inferences = Inferences::read(&context);

    let mut v2 = Vec::with_capacity(FEATURES_LEN_EVAL);
    push_eval_base(
        &mut v2,
        DecisionProfile::default().blind_inference,
        cards,
        &inferences,
    );
    assert_feature_bits(features_eval(cards, &inferences), v2.clone());

    let mut v3 = v2;
    v3.reserve_exact(CALLS_EVAL_V3 * LEN_CALL_EVAL_V3);
    for age in 1..=CALLS_EVAL_V3 {
        push_call_identity(
            &mut v3,
            auction.len().checked_sub(age).map(|index| auction[index]),
        );
    }
    assert_feature_bits(features_eval_v3(cards, &inferences, &auction), v3);

    let unseen = Unseen::new(cards);
    let mut v4 = Vec::with_capacity(FEATURES_LEN_EVAL_V4);
    push_hand_eval(&mut v4, cards);
    for who in [Relative::Lho, Relative::Partner, Relative::Rho] {
        push_points(
            &mut v4,
            shown(
                DecisionProfile::default().blind_inference,
                inferences.announced(who),
            ),
        );
        push_shape_gauss(
            &mut v4,
            &shape_of(
                &unseen,
                shown_boxes(
                    DecisionProfile::default().blind_inference,
                    inferences.announced_union(who),
                ),
            ),
        );
    }
    for age in 1..=CALLS_EVAL_V3 {
        push_call_identity(
            &mut v4,
            auction.len().checked_sub(age).map(|index| auction[index]),
        );
    }
    assert_feature_bits(features_eval_v4(cards, &inferences, &auction), v4);

    let mut shape = Vec::with_capacity(FEATURES_LEN_EVAL_SHAPE);
    push_hand_eval(&mut shape, cards);
    for who in [Relative::Lho, Relative::Partner, Relative::Rho] {
        push_inference(
            &mut shape,
            DecisionProfile::default().blind_inference,
            inferences.announced(who),
        );
        push_shape_dist(
            &mut shape,
            &shape_of(
                &unseen,
                shown_boxes(
                    DecisionProfile::default().blind_inference,
                    inferences.announced_union(who),
                ),
            ),
        );
    }
    for age in 1..=CALLS_EVAL_V3 {
        push_call_identity(
            &mut shape,
            auction.len().checked_sub(age).map(|index| auction[index]),
        );
    }
    assert_feature_bits(features_eval_shape(cards, &inferences, &auction), shape);

    let honours = UnseenHonours::new(cards);
    let mut points = Vec::with_capacity(FEATURES_LEN_EVAL_POINTS);
    push_hand_eval(&mut points, cards);
    for who in [Relative::Lho, Relative::Partner, Relative::Rho] {
        push_inference(
            &mut points,
            DecisionProfile::default().blind_inference,
            inferences.announced(who),
        );
        let boxes = shown_boxes(
            DecisionProfile::default().blind_inference,
            inferences.announced_union(who),
        );
        push_shape_gauss(&mut points, &shape_of(&unseen, boxes));
        push_hcp_ends(
            &mut points,
            shown(
                DecisionProfile::default().blind_inference,
                inferences.announced(who),
            ),
        );
        push_hcp_gauss(&mut points, &hcp_of(&honours, boxes));
    }
    for age in 1..=CALLS_EVAL_V3 {
        push_call_identity(
            &mut points,
            auction.len().checked_sub(age).map(|index| auction[index]),
        );
    }
    assert_feature_bits(features_eval_points(cards, &inferences, &auction), points);
}

fn empty_context() -> Context<'static> {
    Context::new(RelativeVulnerability::NONE, &[])
}

/// A length box, in `Suit::ASC` order: clubs, diamonds, hearts, spades.
fn lengths(bounds: [(u8, u8); 4]) -> Envelope {
    let mut envelope = Envelope::unknown();
    envelope.lengths = bounds.map(|(min, max)| super::super::inference::Range::new(min, max));
    envelope
}

fn shape_block(cards: &str, boxes: Option<&[Envelope]>) -> Vec<f32> {
    let mut out = Vec::new();
    push_shape_dist(&mut out, &shape_of(&Unseen::new(hand(cards)), boxes));
    assert_eq!(out.len(), LEN_SHAPE);
    out
}

/// The shipped [`LEN_SHAPE_GAUSS`] block, for the same reading.
fn gauss_block(cards: &str, boxes: Option<&[Envelope]>) -> Vec<f32> {
    let mut out = Vec::new();
    push_shape_gauss(&mut out, &shape_of(&Unseen::new(hand(cards)), boxes));
    assert_eq!(out.len(), LEN_SHAPE_GAUSS);
    out
}

/// `C(n, k)` in `f64` — the test's own arithmetic, independent of [`BINOM`].
fn choose(n: u32, k: u32) -> f64 {
    (0..k).fold(1.0, |acc, i| acc * f64::from(n - i) / f64::from(i + 1))
}

/// A hand with 1-3-4-5 in `Suit::ASC` order, so all four unseen counts differ.
const SPREAD_HAND: &str = "AKQ32.K532.QJ4.9";

#[test]
fn unconditional_shape_prior_is_hypergeometric() {
    // Unseen per suit, ASC: ♣12 ♦10 ♥9 ♠8, summing to 39.  A hidden seat
    // draws 13 of those, so `E[len_s] = 13 · n_s / 39 = n_s / 3` exactly.
    let block = shape_block(SPREAD_HAND, None);
    for (s, unseen) in [12.0, 10.0, 9.0, 8.0].into_iter().enumerate() {
        assert!(
            (f64::from(block[s]) - unseen / 3.0 / 13.0).abs() < 1e-6,
            "E[len_{s}] = {}",
            block[s]
        );
    }
    // Shows nothing, so it pins nothing.
    assert!(
        block[LEN_SHAPE - 1].abs() < 1e-6,
        "{}",
        block[LEN_SHAPE - 1]
    );
}

/// **The point of the encoding.**  `ReadingProfile::sum_closure` narrows every box to
/// what `Σ len = 13` already implies — it cannot reject a hand, so it is
/// information-free — yet it moves `push_inference`'s endpoints by multiple
/// σ of their own corpus spread.  The shape block must not move at all.
#[test]
fn shape_block_is_invariant_to_the_sum_closure() {
    // Two majors of 5+ leave at most 3 cards for each minor and at most 8
    // for each major.  Same set of hands, different bounding box.
    let open = lengths([(0, 13), (0, 13), (5, 13), (5, 13)]);
    let closed = lengths([(0, 3), (0, 3), (5, 8), (5, 8)]);

    let mut endpoints = (Vec::new(), Vec::new());
    push_inference(
        &mut endpoints.0,
        DecisionProfile::default().blind_inference,
        &open,
    );
    push_inference(
        &mut endpoints.1,
        DecisionProfile::default().blind_inference,
        &closed,
    );
    assert_ne!(endpoints.0, endpoints.1, "the closure moves the endpoints");

    for cards in [SPREAD_HAND, "AQ32.K53.QJ4.A92"] {
        assert_eq!(
            shape_block(cards, Some(&[open])),
            shape_block(cards, Some(&[closed])),
            "{cards}"
        );
    }
}

#[test]
fn a_shown_void_reads_as_its_exact_mass() {
    // ♠ 0..=0 against the 8 unseen spades: the seat draws all 13 from the
    // other 31 unseen cards.
    let block = shape_block(
        SPREAD_HAND,
        Some(&[lengths([(0, 13), (0, 13), (0, 13), (0, 0)])]),
    );
    assert!(block[3].abs() < 1e-6, "E[len_♠] = {}", block[3]);
    assert!(block[7].abs() < 1e-6, "sd[len_♠] = {}", block[7]);
    // Spades are suit 3, so its 14-bin marginal starts at 8 + 3·14 = 50.
    assert!((block[50] - 1.0).abs() < 1e-6, "P(♠ = 0) = {}", block[50]);

    let all = choose(39, 13);
    let want = -(choose(31, 13) / all).ln() / all.ln();
    assert!(
        (f64::from(block[LEN_SHAPE - 1]) - want).abs() < 1e-6,
        "pinned = {} want {want}",
        block[LEN_SHAPE - 1]
    );
}

/// An agreement can over-claim against a hand that holds the cards it wants.
/// Dividing by zero mass is not an option; read it as nothing shown.
#[test]
fn an_unsatisfiable_reading_falls_back_to_nothing_shown() {
    // Only 8 spades are unseen, so "9+ spades" admits no shape at all.
    let impossible = lengths([(0, 13), (0, 13), (0, 13), (9, 13)]);
    assert_eq!(
        shape_block(SPREAD_HAND, Some(&[impossible])),
        shape_block(SPREAD_HAND, None)
    );
}

/// A strength box, leaving lengths unknown.
fn strength(hcp: (u8, u8), points: (u8, u8)) -> Envelope {
    use super::super::inference::Range;
    let mut envelope = Envelope::unknown();
    envelope.strength.hcp = Range::new(hcp.0, hcp.1);
    envelope.strength.points = Range::new(points.0, points.1);
    envelope
}

fn hcp_block(cards: &str, boxes: Option<&[Envelope]>) -> Vec<f32> {
    let mut out = Vec::new();
    push_hcp_gauss(&mut out, &hcp_of(&UnseenHonours::new(hand(cards)), boxes));
    assert_eq!(out.len(), LEN_HCP_GAUSS);
    out
}

/// `E[hcp]` of one hidden seat, undoing the block's ÷37.
fn mean_hcp(cards: &str, boxes: Option<&[Envelope]>) -> f64 {
    f64::from(hcp_block(cards, boxes)[0]) * 37.0
}

/// The kernel's arithmetic, against a closed form it cannot fake: the three
/// hidden seats split what this hand does not hold, so an unconstrained
/// seat averages a third of the missing HCP.
#[test]
fn unconditional_hcp_prior_is_hypergeometric() {
    // SPREAD_HAND holds ♠AKQ ♥K ♦QJ = 15 HCP, so 25 are unseen.
    let block = hcp_block(SPREAD_HAND, None);
    assert!(
        (mean_hcp(SPREAD_HAND, None) - 25.0 / 3.0).abs() < 1e-4,
        "E[hcp] = {}",
        mean_hcp(SPREAD_HAND, None)
    );
    // Shows nothing, so it pins nothing.
    assert!(block[2].abs() < 1e-6, "pinned = {}", block[2]);
    // The walk must cover the whole prior, not a sub-lattice of it: a
    // 13-card draw from 39 has σ[hcp] ≈ 4, and 0 would mean it collapsed.
    let sd = f64::from(block[1]) * HCP_SPREAD_SCALE;
    assert!((3.0..5.0).contains(&sd), "sd[hcp] = {sd}");
}

/// **The point of the encoding.**  The endpoints of `11..=26` say the
/// midpoint is 18.5; the truncated prior says the seat averages barely over
/// 13, because 25 unseen HCP rarely land three-quarters in one hand.
#[test]
fn a_wide_band_is_nothing_like_its_midpoint() {
    let wide = strength((11, 26), (11, 26));
    let mean = mean_hcp(SPREAD_HAND, Some(&[wide]));
    assert!((11.0..=26.0).contains(&mean), "E[hcp] = {mean}");
    assert!(
        mean < 15.0,
        "E[hcp] = {mean}, nowhere near the midpoint 18.5"
    );
}

/// The band reads **both** strength axes.  A 1NT box carries a crisp
/// `hcp 15..=17` beside a slacked `points 15..=19`; dropping either leg
/// widens the support, so the mean must sit inside the tighter one.
#[test]
fn the_strength_band_intersects_hcp_with_points() {
    let notrump = strength((15, 17), (15, 19));
    let mean = mean_hcp(SPREAD_HAND, Some(&[notrump]));
    assert!((15.0..=17.0).contains(&mean), "E[hcp] = {mean}");

    // The `points` leg is load-bearing too: `upgrade >= 0` caps raw HCP at
    // `points.max`, so widening it alone moves the reading.
    let looser = strength((15, 17), (15, 16));
    assert_ne!(
        hcp_block(SPREAD_HAND, Some(&[notrump])),
        hcp_block(SPREAD_HAND, Some(&[looser]))
    );
}

/// The union of boxes is an OR of bands weighted by *prior mass*, not a
/// hull — and this is the case the endpoints cannot represent at all.
///
/// "6-9 or 20-23" hulls to `6..=23`, which hands the net a band whose bulk
/// is the 10-19 middle the reading explicitly excludes.  The kernel instead
/// reads the two humps it was given, and weights them: against 25 unseen
/// HCP the strong alternative is nearly impossible, so the reading sits on
/// the weak hump — below where the hull's own truncated mean lands.
#[test]
fn disjoint_strength_boxes_do_not_read_as_their_hull() {
    let weak = strength((6, 9), (6, 9));
    let strong = strength((20, 23), (20, 23));
    let split = mean_hcp(SPREAD_HAND, Some(&[weak, strong]));
    let hull = mean_hcp(SPREAD_HAND, Some(&[strength((6, 23), (6, 23))]));
    assert!((6.0..=23.0).contains(&split), "E[hcp] = {split}");
    assert!(split < 9.0, "E[hcp] = {split}, off the weak hump");
    assert!(
        hull > split + 0.5,
        "split {split} vs hull {hull}: the union collapsed to its span"
    );
}

/// An agreement can over-claim on strength as well as on shape.
#[test]
fn an_unsatisfiable_strength_reading_falls_back_to_nothing_shown() {
    // 25 HCP are unseen, so "30+" admits no split at all.
    let impossible = strength((30, 37), (30, 37));
    assert_eq!(
        hcp_block(SPREAD_HAND, Some(&[impossible])),
        hcp_block(SPREAD_HAND, None)
    );
}

/// The superset carries the shipped vector verbatim, so the control arm of
/// the ablation is reproducible from the same corpus.
#[test]
fn shape_superset_embeds_the_shipped_vector() {
    assert_eq!(LEN_SHAPE, 65);
    assert_eq!(FEATURES_LEN_EVAL_SHAPE, 289);

    let auction = [
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Double,
    ];
    let ctx = Context::new(RelativeVulnerability::ALL, &auction);
    let inferences = Inferences::read(&ctx);
    let cards = hand(SPREAD_HAND);
    let wide = features_eval_shape(cards, &inferences, &auction);
    let shipped = features_eval_v3(cards, &inferences, &auction);

    assert_eq!(wide.len(), FEATURES_LEN_EVAL_SHAPE);
    assert_eq!(wide[..LEN_HAND_EVAL], shipped[..LEN_HAND_EVAL]);
    for seat in 0..3 {
        let from = LEN_HAND_EVAL + seat * LEN_SEAT_SHAPE;
        let was = LEN_HAND_EVAL + seat * LEN_INFERENCE;
        assert_eq!(
            wide[from..from + LEN_INFERENCE],
            shipped[was..was + LEN_INFERENCE],
            "seat {seat}"
        );
    }
    let tail = LEN_HAND_EVAL + 3 * LEN_SEAT_SHAPE;
    assert_eq!(wide[tail..], shipped[FEATURES_LEN_EVAL..]);
    for (i, &v) in wide.iter().enumerate() {
        assert!(
            v.is_finite() && (-1.0..=1.5).contains(&v),
            "shape[{i}] = {v}"
        );
    }
}

/// The shipped vector's layout, and the one property it exists for: the
/// sum closure moves the endpoints it replaces and must not move it.
#[test]
fn eval_v4_is_invariant_where_the_hull_is_not() {
    assert_eq!(LEN_SEAT_V4, 11);
    assert_eq!(FEATURES_LEN_EVAL_V4, 97);

    let open = lengths([(0, 13), (0, 13), (5, 13), (5, 13)]);
    let closed = lengths([(0, 3), (0, 3), (5, 8), (5, 8)]);
    for cards in [SPREAD_HAND, "AQ32.K53.QJ4.A92"] {
        assert_eq!(
            gauss_block(cards, Some(&[open])),
            gauss_block(cards, Some(&[closed])),
            "{cards}"
        );
    }
    // …and it is not invariant to everything, or it would be reading nothing.
    assert_ne!(
        gauss_block(SPREAD_HAND, Some(&[open])),
        gauss_block(SPREAD_HAND, None)
    );
}

/// v4 is v3's hand and calls with each seat's eight length endpoints swapped
/// for the shape reading — every column traceable to a shipped one.
#[test]
fn eval_v4_swaps_the_length_hull_for_the_shape_reading() {
    let auction = [
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Double,
    ];
    let ctx = Context::new(RelativeVulnerability::ALL, &auction);
    let inferences = Inferences::read(&ctx);
    let cards = hand(SPREAD_HAND);
    let v4 = features_eval_v4(cards, &inferences, &auction);
    let v3 = features_eval_v3(cards, &inferences, &auction);
    let wide = features_eval_shape(cards, &inferences, &auction);

    assert_eq!(v4.len(), FEATURES_LEN_EVAL_V4);
    assert_eq!(v4[..LEN_HAND_EVAL], v3[..LEN_HAND_EVAL]);
    for seat in 0..3 {
        // The `points` endpoints survive the swap verbatim…
        let from = LEN_HAND_EVAL + seat * LEN_SEAT_V4;
        let was = LEN_HAND_EVAL + seat * LEN_INFERENCE + 8;
        assert_eq!(
            v4[from..from + LEN_POINTS],
            v3[was..was + LEN_POINTS],
            "seat {seat} points"
        );
        // …and the shape reading is the superset's own summary and mass.
        let wide_seat = LEN_HAND_EVAL + seat * LEN_SEAT_SHAPE + LEN_INFERENCE;
        assert_eq!(
            v4[from + LEN_POINTS..from + LEN_SEAT_V4 - 1],
            wide[wide_seat..wide_seat + 8],
            "seat {seat} moments"
        );
        assert_eq!(
            v4[from + LEN_SEAT_V4 - 1],
            wide[wide_seat + LEN_SHAPE - 1],
            "seat {seat} mass"
        );
    }
    let tail = LEN_HAND_EVAL + 3 * LEN_SEAT_V4;
    assert_eq!(v4[tail..], v3[FEATURES_LEN_EVAL..]);
}

/// The strength superset carries [`features_eval_v4`] verbatim, so the
/// ablation's control arm is the shipped vector reproduced from the same
/// corpus — the whole reason a superset exists.
#[test]
fn points_superset_embeds_the_shipped_vector() {
    assert_eq!(LEN_SEAT_POINTS, 24);
    assert_eq!(FEATURES_LEN_EVAL_POINTS, 136);

    let auction = [
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Double,
    ];
    let ctx = Context::new(RelativeVulnerability::ALL, &auction);
    let inferences = Inferences::read(&ctx);
    let cards = hand(SPREAD_HAND);
    let wide = features_eval_points(cards, &inferences, &auction);
    let v4 = features_eval_v4(cards, &inferences, &auction);

    assert_eq!(wide.len(), FEATURES_LEN_EVAL_POINTS);
    assert_eq!(wide[..LEN_HAND_EVAL], v4[..LEN_HAND_EVAL]);
    for seat in 0..3 {
        // v4's seat block is the superset's `points` endpoints and shape
        // reading, contiguous — offsets 8..19 of the 24.
        let from = LEN_HAND_EVAL + seat * LEN_SEAT_POINTS + 8;
        let was = LEN_HAND_EVAL + seat * LEN_SEAT_V4;
        assert_eq!(
            wide[from..from + LEN_SEAT_V4],
            v4[was..was + LEN_SEAT_V4],
            "seat {seat}"
        );
    }
    let tail = LEN_HAND_EVAL + 3 * LEN_SEAT_POINTS;
    assert_eq!(wide[tail..], v4[LEN_HAND_EVAL + 3 * LEN_SEAT_V4..]);
    for (i, &v) in wide.iter().enumerate() {
        assert!(
            v.is_finite() && (-1.0..=1.5).contains(&v),
            "points[{i}] = {v}"
        );
    }
}

/// The two halves of the block must agree: each suit's 14 bins are a
/// probability distribution, and the `E`/`sd` summary beside them is its
/// first two moments.  Cheap, and it is what catches an offset slip.
#[test]
fn the_marginal_and_its_summary_agree() {
    let block = shape_block(
        SPREAD_HAND,
        Some(&[lengths([(0, 13), (0, 3), (5, 13), (5, 13)])]),
    );
    for s in 0..4 {
        let bins: Vec<f64> = block[8 + s * 14..8 + (s + 1) * 14]
            .iter()
            .map(|&p| f64::from(p))
            .collect();
        let total: f64 = bins.iter().sum();
        assert!((total - 1.0).abs() < 1e-5, "suit {s} mass {total}");

        let mean: f64 = bins.iter().enumerate().map(|(k, p)| k as f64 * p).sum();
        let var: f64 = bins
            .iter()
            .enumerate()
            .map(|(k, p)| (k as f64 - mean).powi(2) * p)
            .sum();
        assert!(
            (f64::from(block[s]) - mean / 13.0).abs() < 1e-5,
            "suit {s} E: {} vs {}",
            block[s],
            mean / 13.0
        );
        assert!(
            (f64::from(block[4 + s]) - var.sqrt() / SPREAD_SCALE).abs() < 1e-5,
            "suit {s} sd: {} vs {}",
            block[4 + s],
            var.sqrt() / SPREAD_SCALE
        );
    }
}

/// The negative control has to reach the shape block too, or it stops
/// bounding the reading channel: blind, every seat must read as the bare
/// hypergeometric prior over shapes.
#[test]
fn blind_inference_blanks_the_shape_block() {
    let auction = [bid(1, Strain::Spades), Call::Pass, bid(2, Strain::Clubs)];
    let ctx = Context::new(RelativeVulnerability::NONE, &auction);
    let inferences = Inferences::read(&ctx);
    let cards = hand(SPREAD_HAND);

    let seeing = features_eval_shape_on(false, cards, &inferences, &auction);
    let blind = features_eval_shape_on(true, cards, &inferences, &auction);

    assert_ne!(seeing, blind, "an opened auction shows something");
    let prior = shape_block(SPREAD_HAND, None);
    for seat in 0..3 {
        let from = LEN_HAND_EVAL + seat * LEN_SEAT_SHAPE + LEN_INFERENCE;
        assert_eq!(blind[from..from + LEN_SHAPE], prior[..], "seat {seat}");
    }
}

#[test]
fn block_offsets_are_consistent() {
    assert_eq!(LEN_HAND_V3, 10);
    assert_eq!(OFFSET_CONTEXT, LEN_HAND_V3);
    assert_eq!(LEN_CONTEXT, 36);
    assert_eq!(OFFSET_INFERENCES, OFFSET_CONTEXT + LEN_CONTEXT);
    assert_eq!(LEN_INFERENCES, 40);
    assert_eq!(OFFSET_VUL, OFFSET_INFERENCES + LEN_INFERENCES);
    assert_eq!(LEN_VUL, 2);
    assert_eq!(OFFSET_VUL + LEN_VUL, FEATURES_LEN_V3);
}

#[test]
fn length_is_correct_for_contested_auction() {
    let auction = [
        bid(1, Strain::Hearts),
        bid(1, Strain::Spades),
        bid(2, Strain::Hearts),
    ];
    let ctx = Context::new(RelativeVulnerability::WE, &auction);
    let f = features_v3(hand("AQ32.K53.QJ4.A92"), &ctx);
    assert_eq!(f.len(), FEATURES_LEN_V3);
}

#[test]
fn v3_length_and_range() {
    // v3 is 88 floats: a 10-value restrictive hand block + the 78-value
    // shared context/inferences/vul tail.
    assert_eq!(FEATURES_LEN_V3, 88);
    let auction = [
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Double,
    ];
    for ctx in [
        empty_context(),
        Context::new(RelativeVulnerability::ALL, &auction),
    ] {
        let f = features_v3(hand("AKQ32.K532.QJ4.9"), &ctx);
        assert_eq!(f.len(), FEATURES_LEN_V3);
        for (i, &v) in f.iter().enumerate() {
            assert!(v.is_finite() && (0.0..=1.5).contains(&v), "v3[{i}] = {v}");
        }
    }
}

/// The negative control blanks the whole inference block and nothing else.
///
/// Knob-off an opened auction shows *something* — this is what every reading
/// generator competes to sharpen; knob-on all four seats read
/// `Envelope::unknown`, the `[0, 1]` pattern, and the rest of the vector is
/// untouched.
#[test]
fn blind_inference_blanks_only_the_reading_block() {
    let auction = [bid(1, Strain::Spades), Call::Pass, bid(2, Strain::Clubs)];
    let ctx = Context::new(RelativeVulnerability::NONE, &auction);
    let hand = hand("AKQ32.K532.QJ4.9");

    let seen = features_v3(hand, &ctx);
    let profile = DecisionProfile {
        blind_inference: true,
        ..Default::default()
    };
    let blind_ctx = Context::new(RelativeVulnerability::NONE, &auction).with_profile(profile);
    let blind = features_v3(hand, &blind_ctx);

    let block = OFFSET_INFERENCES..OFFSET_INFERENCES + LEN_INFERENCES;
    assert_ne!(
        seen[block.clone()],
        blind[block.clone()],
        "nothing was shown"
    );
    assert_eq!(
        blind[block.clone()],
        [0.0, 1.0].repeat(LEN_INFERENCES / 2),
        "blind is not the unknown pattern"
    );
    assert_eq!(seen[..block.start], blind[..block.start]);
    assert_eq!(seen[block.end..], blind[block.end..]);
}

#[test]
fn empty_auction_known_values() {
    let ctx = empty_context();
    let f = features_v3(hand("AKQ32.K532.QJ4.9"), &ctx);

    // Context layout: 5 our_strains + 5 their_strains + 7 last_bid + 7 partner
    // + 3 penalty + 1 undisturbed + 1 passed + 1 partner_passed + 1 leading
    // + 4 seat + 1 we_opened = 36.
    // Seat one-hot: auction.len() = 0, so index 0 is set.
    let seat_one_hot_start = OFFSET_CONTEXT + 5 + 5 + 7 + 7 + 3 + 1 + 1 + 1 + 1;
    assert_eq!(f[seat_one_hot_start], 1.0, "seat index 0 should be 1.0");
    assert_eq!(f[seat_one_hot_start + 1], 0.0);
    assert_eq!(f[seat_one_hot_start + 2], 0.0);
    assert_eq!(f[seat_one_hot_start + 3], 0.0);

    // Vulnerability: both 0.0 (NONE)
    assert_eq!(f[OFFSET_VUL], 0.0, "WE vul should be 0.0");
    assert_eq!(f[OFFSET_VUL + 1], 0.0, "THEY vul should be 0.0");

    // contract-to-beat present bit = 0.0
    let last_bid_start = OFFSET_CONTEXT + 5 + 5;
    assert_eq!(f[last_bid_start], 0.0, "contract-to-beat present bit");

    // undisturbed = 1.0 for empty auction
    let undisturbed_offset = OFFSET_CONTEXT + 5 + 5 + 7 + 7 + 3;
    assert_eq!(f[undisturbed_offset], 1.0, "undisturbed should be 1.0");
}

#[test]
fn disclosable_hand_block_for_known_hand() {
    // "AKQ32.K532.QJ4.9" — Suit::ASC order is clubs, diamonds, hearts, spades.
    let f = features_v3(hand("AKQ32.K532.QJ4.9"), &empty_context());

    // Clubs: singleton 9, no HCP.
    assert!((f[0] - 1.0 / 13.0).abs() < 1e-6, "clubs len/13");
    assert_eq!(f[1], 0.0, "clubs suit_hcp");
    // Diamonds: QJ4 = 3 cards, 3 HCP.
    assert!((f[2] - 3.0 / 13.0).abs() < 1e-6, "diamonds len/13");
    assert!((f[3] - 3.0 / 10.0).abs() < 1e-6, "diamonds suit_hcp");
    // Hearts: K532 = 4 cards, 3 HCP.
    assert!((f[4] - 4.0 / 13.0).abs() < 1e-6, "hearts len/13");
    assert!((f[5] - 3.0 / 10.0).abs() < 1e-6, "hearts suit_hcp");
    // Spades: AKQ32 = 5 cards, 9 HCP.
    assert!((f[6] - 5.0 / 13.0).abs() < 1e-6, "spades len/13");
    assert!((f[7] - 9.0 / 10.0).abs() < 1e-6, "spades suit_hcp");
    // Global: 15 HCP, then the fuzzy shape upgrade scaled by 2.
    assert!((f[8] - 15.0 / 40.0).abs() < 1e-6, "hcp/40");
    assert!((0.0..=1.0).contains(&f[9]), "shape/2 in range");
}

#[test]
fn vulnerability_bits() {
    let h = hand("AQ32.K53.QJ4.A92");
    let ctx_we = Context::new(RelativeVulnerability::WE, &[]);
    let f = features_v3(h, &ctx_we);
    assert_eq!(f[OFFSET_VUL], 1.0, "WE vul bit");
    assert_eq!(f[OFFSET_VUL + 1], 0.0, "THEY vul bit");

    let ctx_all = Context::new(RelativeVulnerability::ALL, &[]);
    let f2 = features_v3(h, &ctx_all);
    assert_eq!(f2[OFFSET_VUL], 1.0);
    assert_eq!(f2[OFFSET_VUL + 1], 1.0);
}

#[test]
fn we_opened_bit() {
    let h = hand("AQ32.K53.QJ4.A92");
    let we_opened_offset = OFFSET_CONTEXT + 35; // last value in context block

    // Empty auction: no opener → 0.0
    let f0 = features_v3(h, &empty_context());
    assert_eq!(f0[we_opened_offset], 0.0, "no opener → 0.0");

    // After `(1♠)`: auction.len()=1, opening_index=0, (1-0)%2=1 ≠ 0 → they opened
    let auction_they = [bid(1, Strain::Spades)];
    let ctx_they = Context::new(RelativeVulnerability::NONE, &auction_they);
    let f1 = features_v3(h, &ctx_they);
    assert_eq!(f1[we_opened_offset], 0.0, "they opened (RHO opened)");

    // After `1♠ -`: auction.len()=2, opening_index=0, (2-0)%2=0 → we opened
    let auction_we = [bid(1, Strain::Spades), Call::Pass];
    let ctx_we = Context::new(RelativeVulnerability::NONE, &auction_we);
    let f2 = features_v3(h, &ctx_we);
    assert_eq!(f2[we_opened_offset], 1.0, "we opened (partner opened)");
}

/// Nothing shown is `[0, 1]` per value pair — the `Envelope::unknown`
/// encoding.  Zeros would be a *different*, out-of-distribution hand.
const UNKNOWN_BLOCK: [f32; LEN_INFERENCE] = [0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0];

#[test]
fn eval_layout_and_unknown_pattern() {
    assert_eq!(LEN_HAND_EVAL, 24);
    assert_eq!(FEATURES_LEN_EVAL, 54);
    let h = hand("AKQ32.K532.QJ4.9");
    let ctx = empty_context();
    let f = features_eval(h, &Inferences::read(&ctx));
    assert_eq!(f.len(), FEATURES_LEN_EVAL);

    // The hand block no longer *repeats* `features_v3`'s summary — it
    // **recovers** it: `len = #spots + ΣA..T` and `suit_hcp = 4A+3K+2Q+J`.
    // That identity is what makes the honour block strictly more
    // informative, so the first layer can still represent everything the
    // 10-float summary carried.  Both sides divide rather than multiply
    // out, and 8 is a power of two, so the compare is exact.
    let v3 = features_v3(h, &ctx);
    for (i, block) in f[..LEN_HAND_EVAL]
        .as_chunks::<{ LEN_HAND_EVAL / 4 }>()
        .0
        .iter()
        .enumerate()
    {
        let (spots, honours) = (block[0] * 8.0, &block[1..]);
        assert_eq!(v3[2 * i], (spots + honours.iter().sum::<f32>()) / 13.0);
        let hcp = 4.0 * honours[0] + 3.0 * honours[1] + 2.0 * honours[2] + honours[3];
        assert_eq!(v3[2 * i + 1], hcp / 10.0);
    }

    // No auction: all three hidden seats read as unknown.
    for start in [24, 34, 44] {
        assert_eq!(
            f[start..start + LEN_INFERENCE],
            UNKNOWN_BLOCK,
            "seat block at {start} should be unknown"
        );
    }
}

#[test]
fn eval_v3_call_tail_is_most_recent_first() {
    assert_eq!(FEATURES_LEN_EVAL_V3, 94);
    let auction = [bid(1, Strain::Spades), Call::Pass, bid(2, Strain::Clubs)];
    let ctx = Context::new(RelativeVulnerability::NONE, &auction);
    let f = features_eval_v3(hand("AQ32.K53.QJ4.A92"), &Inferences::read(&ctx), &auction);
    assert_eq!(f.len(), FEATURES_LEN_EVAL_V3);
    // Head is exactly the v2 vector.
    assert_eq!(
        f[..FEATURES_LEN_EVAL],
        features_eval(hand("AQ32.K53.QJ4.A92"), &Inferences::read(&ctx))[..]
    );
    // Slot 0 (latest): 2♣ — present, level 2/7, ♣ one-hot first in ASC.
    let slot0 = &f[54..64];
    assert_eq!(slot0[..7], [1.0, 2.0 / 7.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
    assert_eq!(slot0[7..], [0.0, 0.0, 0.0]);
    // Slot 1: pass — no bid, pass bit set.
    let slot1 = &f[64..74];
    assert_eq!(slot1[..7], [0.0; 7]);
    assert_eq!(slot1[7..], [1.0, 0.0, 0.0]);
    // Slot 2: 1♠ — present, level 1/7, ♠ is fourth in ASC.
    let slot2 = &f[74..84];
    assert_eq!(slot2[..7], [1.0, 1.0 / 7.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    // Slot 3: beyond the auction — all zeros, unlike any real call.
    assert_eq!(f[84..94], [0.0; 10]);
}

#[test]
fn eval_seat_blocks_are_actor_relative() {
    // A 1♠ opening one call ago is RHO's: only the last block moves.
    let auction = [bid(1, Strain::Spades)];
    let ctx = Context::new(RelativeVulnerability::NONE, &auction);
    let f = features_eval(hand("AQ32.K53.QJ4.A92"), &Inferences::read(&ctx));

    assert_eq!(f[24..34], UNKNOWN_BLOCK, "LHO has not called");
    assert_eq!(f[34..44], UNKNOWN_BLOCK, "partner has not called");
    // RHO: 5+ spades (block offset 6 = spades min, `Suit::ASC` order) and a
    // non-zero point floor.
    assert!(f[50] >= 5.0 / 13.0, "RHO spade floor: {}", f[50]);
    assert!(f[52] > 0.0, "RHO point floor: {}", f[52]);
}

#[test]
fn penalty_one_hot() {
    let h = hand("AQ32.K53.QJ4.A92");
    let penalty_offset = OFFSET_CONTEXT + 5 + 5 + 7 + 7;

    // Undoubled (default)
    let f0 = features_v3(h, &empty_context());
    assert_eq!(f0[penalty_offset], 1.0, "undoubled");
    assert_eq!(f0[penalty_offset + 1], 0.0);
    assert_eq!(f0[penalty_offset + 2], 0.0);

    // Doubled
    let auction_x = [bid(1, Strain::Spades), Call::Double];
    let ctx_x = Context::new(RelativeVulnerability::NONE, &auction_x);
    let f1 = features_v3(h, &ctx_x);
    assert_eq!(f1[penalty_offset], 0.0);
    assert_eq!(f1[penalty_offset + 1], 1.0, "doubled");
    assert_eq!(f1[penalty_offset + 2], 0.0);

    // Redoubled
    let auction_xx = [bid(1, Strain::Spades), Call::Double, Call::Redouble];
    let ctx_xx = Context::new(RelativeVulnerability::NONE, &auction_xx);
    let f2 = features_v3(h, &ctx_xx);
    assert_eq!(f2[penalty_offset], 0.0);
    assert_eq!(f2[penalty_offset + 1], 0.0);
    assert_eq!(f2[penalty_offset + 2], 1.0, "redoubled");
}

// ── The configured extractor ────────────────────────────────────────────

/// `LEN_CARD` must equal what a card actually renders
///
/// A row added to `SCHEMA` or `PONS_SCHEMA` shifts every feature after the
/// card blocks, silently misaligning an artifact against its extractor.
/// This is the tripwire; the cost of ignoring it is a worse bidder with no
/// other symptom.
#[test]
fn card_block_is_the_whole_card() {
    assert_eq!(
        crate::bidding::card::american_card(&crate::bidding::agreements::Agreements::default())
            .rows
            .len(),
        LEN_CARD_ROWS
    );
    assert_eq!(
        crate::bidding::card::dutch_card(&crate::bidding::agreements::Agreements::default())
            .rows
            .len(),
        LEN_CARD_ROWS
    );
    assert_eq!(LEN_CARD, LEN_SYSTEM + LEN_CARD_ROWS);
    assert_eq!(FEATURES_LEN_V4, FEATURES_LEN_V3 + 2 * LEN_CARD);
    assert_eq!(FEATURES_LEN_V4, 368);
}

/// The base system must reach the vector, not just the rows
///
/// `dutch_card` differs from `american_card` by its header (2/1 → WJ) plus a
/// single row, and the header is the only channel for the wide non-forcing
/// 1♣.  Encoding rows alone would leave a WJ opponent nearly
/// indistinguishable from a 2/1 one.
#[test]
fn the_base_system_is_encoded() {
    let american = Config::symmetric(&crate::bidding::card::american_card(
        &crate::bidding::agreements::Agreements::default(),
    ));
    let dutch = Config::symmetric(&crate::bidding::card::dutch_card(
        &crate::bidding::agreements::Agreements::default(),
    ));

    assert_eq!(american.ours[..LEN_SYSTEM], [1.0, 0.0, 0.0, 0.0, 0.0]);
    assert_eq!(dutch.ours[..LEN_SYSTEM], [0.0, 0.0, 1.0, 0.0, 0.0]);
    assert_ne!(american, dutch);

    // The header, plus the one row the two systems disagree on.
    let differing = american
        .ours
        .iter()
        .zip(&dutch.ours)
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        differing, 3,
        "two one-hot slots plus `1D opening with 5 cards`"
    );
}

/// Each side is encoded independently — the opponents may play anything
///
/// The opposition is ourselves, BBA, BEN or another engine entirely, so the
/// two blocks must be able to disagree, including on the base system.
#[test]
fn the_two_sides_are_independent() {
    let mixed = Config::new(
        &crate::bidding::card::american_card(&crate::bidding::agreements::Agreements::default()),
        &crate::bidding::card::dutch_card(&crate::bidding::agreements::Agreements::default()),
    );
    assert_eq!(mixed.ours[..LEN_SYSTEM], [1.0, 0.0, 0.0, 0.0, 0.0]);
    assert_eq!(mixed.theirs[..LEN_SYSTEM], [0.0, 0.0, 1.0, 0.0, 0.0]);
    assert_ne!(mixed.ours, mixed.theirs);
}

/// v4 is v3 with two card blocks appended — the v3 prefix is untouched
#[test]
fn features_v4_extends_v3_in_place() {
    let hand = hand("AQ32.K53.QJ4.A92");
    let config = Config::symmetric(&crate::bidding::card::american_card(
        &crate::bidding::agreements::Agreements::default(),
    ));
    let auction = [bid(1, Strain::Spades)];
    let context = Context::new(RelativeVulnerability::NONE, &auction).with_config(&config);

    let v3 = features_v3(hand, &context);
    let v4 = features_v4(hand, &context);

    assert_eq!(v4.len(), FEATURES_LEN_V4);
    assert_eq!(v4[..FEATURES_LEN_V3], v3[..], "the v3 prefix must not move");
    assert!(v4.iter().all(|value| value.is_finite()));
    assert!(
        v4[OFFSET_OUR_CARD..].iter().all(|v| *v == 0.0 || *v == 1.0),
        "card rows are boolean"
    );
    // Symmetric config: the two blocks agree.
    assert_eq!(
        v4[OFFSET_OUR_CARD..OFFSET_THEIR_CARD],
        v4[OFFSET_THEIR_CARD..]
    );
}

/// The point of the whole design: a knob moves the features
///
/// If flipping a convention left the vector unchanged, one net could never
/// serve both regimes and the arms would still differ by their weights —
/// which is the confound `docs/ai-bidder/configured-net.md` exists to kill.
#[test]
fn a_convention_knob_moves_the_card_block() {
    use crate::bidding::instinct::RkcbVariant;

    let plain_agreements = crate::bidding::agreements::Agreements::default();
    let plain = Config::symmetric(&crate::bidding::card::american_card(&plain_agreements));
    let mut relocated_agreements = plain_agreements;
    relocated_agreements.decision.reading.rkcb_variant = RkcbVariant::Kickback;
    let relocated = Config::symmetric(&crate::bidding::card::american_card(&relocated_agreements));
    assert_ne!(
        plain, relocated,
        "`Kickback 1430` rides `ReadingProfile::rkcb_variant`, so the config block must differ"
    );

    // Exactly one row moves, and the two sides move together.
    let differing = plain
        .ours
        .iter()
        .zip(&relocated.ours)
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(differing, 1, "only the kickback row should move");
    assert_eq!(plain.theirs, plain.ours, "symmetric config");
}

/// The train/serve skew, measured rather than argued
///
/// `dump-teacher` extracts from a bare [`Context::new`], which carries no
/// trie prefixes, so `Inferences::read` skips `project_authored` entirely.
/// Serving does not: `ConfiguredFloorBba::classify` gets the trie context. This
/// pins **how much** of the vector that costs, so a corpus is never dumped
/// through the wrong extractor by accident.
///
/// One auction is not the scale of the effect: this one moves 3 of the 40
/// inference floats, while dumping 200 bank deals both ways moves **all 40,
/// on 75% of rows**. Quote the corpus figure, not this one.
///
/// Documented in `docs/ai-bidder/configured-net.md` and, as the original
/// finding, in `docs/dnf-migration.md` F1. If a change makes these agree,
/// this test fails and the skew note should come out of both docs.
#[test]
fn bare_and_prefixed_contexts_disagree() {
    let partnership =
        crate::bidding::american(&crate::bidding::agreements::Agreements::default()).bind();
    // An artificial call whose meaning lives in its authoring rule: the
    // Jacoby 2NT game-forcing raise.  A bare context cannot project it.
    let auction = [
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Notrump),
        Call::Pass,
    ];
    let hand = hand("AQ32.K53.QJ4.A92");

    let bare = features_v3(hand, &Context::new(RelativeVulnerability::NONE, &auction));
    let served = features_v3(
        hand,
        &partnership.prefixed_context(RelativeVulnerability::NONE, &auction),
    );

    let moved: Vec<usize> = (0..FEATURES_LEN_V3)
        .filter(|index| bare[*index] != served[*index])
        .collect();
    eprintln!(
        "bare-vs-prefixed: {} of {LEN_INFERENCES} inference floats move",
        moved.len()
    );
    assert!(
        !moved.is_empty(),
        "if these now agree the skew is gone — delete the warnings in \
         configured-net.md and dnf-migration.md rather than this test"
    );
    // The hand and vulnerability blocks describe the actor, not the reading,
    // so only the inference block may move.
    assert!(
        moved
            .iter()
            .all(|index| (OFFSET_INFERENCES..OFFSET_INFERENCES + LEN_INFERENCES).contains(index)),
        "only the inference block should differ, got {moved:?}"
    );
}

/// An unattached config encodes as zeros rather than panicking in release
#[test]
fn features_v4_without_a_config_is_zero_padded() {
    let hand = hand("AQ32.K53.QJ4.A92");
    let context = empty_context();
    // The debug assert in `features_v4` fires on a missing config, so reach
    // past it: this pins the *release* shape, that the vector is still the
    // right width rather than short.
    let mut out = features_v3(hand, &context);
    out.resize(FEATURES_LEN_V4, 0.0);
    assert_eq!(out.len(), FEATURES_LEN_V4);
    assert!(out[OFFSET_OUR_CARD..].iter().all(|value| *value == 0.0));
}

// ── The compact-config extractor ────────────────────────────────────────

/// `LEN_COMPACT` and the per-slot layout are the artifact/extractor contract
///
/// The same tripwire as `card_block_is_the_whole_card`, one block over: a
/// slot added, moved, or re-defaulted silently misaligns a v5 artifact
/// against its extractor with no symptom other than worse bidding.  The
/// expected vector is the shipped knob defaults, hardcoded slot by slot.
#[test]
fn compact_layout_is_pinned() {
    assert_eq!(LEN_COMPACT, 28);
    assert_eq!(FEATURES_LEN_V5, 144);
    assert_eq!(OFFSET_OUR_COMPACT, FEATURES_LEN_V3);
    assert_eq!(OFFSET_THEIR_COMPACT, OFFSET_OUR_COMPACT + LEN_COMPACT);

    #[rustfmt::skip]
    let expected = [
        0.0, //  0: dutch — `capture(false)`
        0.0, //  1: relocating — `RkcbVariant::Plain`, kickback opt-in
        1.0, //  2: garbage_stayman — default on
        0.0, //  3: new_minor_forcing — default off (XYZ shadows it)
        1.0, //  4: xyz — default on
        0.0, //  5: transfer_super_accept — default off
        1.0, //  6: fourth_suit_forcing — default on
        1.0, //  7: jordan_truscott — default on
        1.0, //  8: leaping_michaels — default on
        1.0, //  9: responsive_takeout — default on
        1.0, // 10: major_support_double — default on
        1.0, // 11: nt_splinter — default on
        0.0, // 12: one_notrump_offshape — default off
        0.0, 0.0, 1.0, // 13..16: NotrumpShape — Wide6322, the shipped default
        1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // 16..23: NotrumpDefense — Natural
        0.0, 0.0, 1.0, // 23..26: LebensohlStyle — Transfer
        0.0, // 26: minors_european — Puppet scheme by default
        0.0, // 27: landy — `landy` off
    ];
    assert_eq!(
        ConventionCard::capture(&crate::bidding::agreements::Agreements::default(), false).encode(),
        expected
    );
}

/// Each enum block is a one-hot: exactly one slot set, whatever the variant
#[test]
fn one_hot_blocks_are_exclusive() {
    let check = |agreements: ConventionCard| {
        let encoded = agreements.encode();
        for block in [13..16, 16..23, 23..26] {
            let sum: f32 = encoded[block.clone()].iter().sum();
            assert_eq!(sum, 1.0, "block {block:?} of {agreements:?}");
        }
    };
    let mut agreements =
        ConventionCard::capture(&crate::bidding::agreements::Agreements::default(), false);
    check(agreements);
    for shape in [
        NotrumpShape::Balanced,
        NotrumpShape::Wide,
        NotrumpShape::Wide6322,
    ] {
        for defense in [
            NotrumpDefense::Natural,
            NotrumpDefense::DirectDont,
            NotrumpDefense::Meckwell,
            NotrumpDefense::Woolsey,
            NotrumpDefense::DirectLandy,
            NotrumpDefense::AlwaysPass,
            NotrumpDefense::Off,
        ] {
            for lebensohl in [
                LebensohlStyle::Off,
                LebensohlStyle::Plain,
                LebensohlStyle::Transfer,
            ] {
                agreements.shape = shape;
                agreements.defense = defense;
                agreements.lebensohl = lebensohl;
                check(agreements);
            }
        }
    }
}

/// The projection and the live capture agree on our own default cards
///
/// `from_card` reads row names; `capture` reads the knobs those rows are
/// generated from.  A disagreement at the shipped defaults means a wrong
/// row-name mapping — which would feed a v5 net a system nobody plays.  The
/// Dutch system also pins the system-header path (`dutch` rides `Card::system`,
/// not a row).
#[test]
fn projection_agrees_with_capture_at_defaults() {
    assert_eq!(
        ConventionCard::from_card(&crate::bidding::card::american_card(
            &crate::bidding::agreements::Agreements::default()
        )),
        ConventionCard::capture(&crate::bidding::agreements::Agreements::default(), false)
    );
    assert_eq!(
        ConventionCard::from_card(&crate::bidding::card::dutch_card(
            &crate::bidding::agreements::Agreements::default()
        )),
        ConventionCard::capture(&crate::bidding::agreements::Agreements::default(), true)
    );
}

/// v5 is v3 with two compact blocks appended — the v3 prefix is untouched
#[test]
fn features_v5_appends_both_blocks() {
    let hand = hand("AQ32.K53.QJ4.A92");
    let ours = ConventionCard::capture(&crate::bidding::agreements::Agreements::default(), false);
    let theirs = ConventionCard::from_card(&crate::bidding::card::dutch_card(
        &crate::bidding::agreements::Agreements::default(),
    ));
    let compact = CompactConfig::new(&ours, &theirs);
    let auction = [bid(1, Strain::Spades)];
    let context = Context::new(RelativeVulnerability::NONE, &auction).with_compact(&compact);

    let v3 = features_v3(hand, &context);
    let v5 = features_v5(hand, &context);

    assert_eq!(v5.len(), FEATURES_LEN_V5);
    assert_eq!(v5[..FEATURES_LEN_V3], v3[..], "the v3 prefix must not move");
    assert!(v5.iter().all(|value| value.is_finite()));
    assert_eq!(
        v5[OFFSET_OUR_COMPACT..OFFSET_THEIR_COMPACT],
        ours.encode(),
        "our block is our encoding, verbatim"
    );
    assert_eq!(
        v5[OFFSET_THEIR_COMPACT..],
        theirs.encode(),
        "their block is their encoding, verbatim"
    );
    // A mixed table: the `dutch` slot separates the two sides.
    assert_ne!(
        v5[OFFSET_OUR_COMPACT..OFFSET_THEIR_COMPACT],
        v5[OFFSET_THEIR_COMPACT..]
    );
}

/// Every axis moves its own slots, and only the **trained** ones reach the net
///
/// The two-sided contract the per-axis knob A/Bs stand on, and neither half is
/// covered elsewhere:
///
/// 1. `compact_layout_is_pinned` and `projection_agrees_with_capture_at_defaults`
///    both test only **at the shipped defaults**, so a crossed getter inside
///    [`ConventionCard::capture`] between two knobs that share a default — say
///    `garbage_stayman`/`xyz` (both on) or `new_minor_forcing`/
///    `transfer_super_accept`/`one_notrump_offshape` (all off) — passes every
///    existing test while feeding the net a regime nobody plays.  Its only
///    symptom would be that the axis A/B measures nothing.
/// 2. `folded_compact_columns_are_exactly_zero` pins the fold in the *weights*.
///    This is the same claim where measurement actually needs it: flipping a
///    frozen axis moves no logit at all, so its A/B prices the book alone,
///    rather than the book plus a random init-draw vector (the ≈ −0.015
///    IMPs/board/bit tax, `docs/ai-bidder/card-manifold.md`).
///
/// The order inside the loop is load-bearing: **arm, capture, restore, and only
/// then classify.**  [`ConventionCard`] is `Copy`, so both arms can be captured up
/// front and run under one ambient world — which makes the `features_v3` prefix
/// bit-identical by construction.  Without that, `garbage_stayman`,
/// `nt_splinter`, `notrump_defense`, `landy` and `notrump_minors` reach
/// the vector a second way, through the classify-time inference walk, and two of
/// those are frozen axes whose inertness assertion would become
/// auction-dependent.
#[test]
fn each_compact_axis_moves_its_slots_and_only_live_ones_move_the_net() {
    use crate::bidding::Rules;
    use crate::bidding::agreements::Agreements;
    use crate::bidding::instinct::{RkcbVariant, forced};
    use crate::bidding::neural_floor::ConfiguredFloorV5;
    use crate::bidding::trie::Classifier;
    use std::sync::Arc;

    // The trained columns of the shipped `american_bba_v5` blob, one side.
    // Deliberately restated rather than shared with
    // `neural::tests::folded_compact_columns_are_exactly_zero`: that one pins
    // the weights, this one pins the behaviour, and a thaw that updates only
    // one of them must fail both.
    const LIVE: [usize; 13] = [0, 1, 3, 4, 7, 13, 15, 16, 19, 23, 25, 26, 27];

    let seat = hand("92.K53.AQJ42.962");
    let auction = [
        bid(1, Strain::Hearts),
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Spades),
        Call::Pass,
    ];
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    assert!(
        !forced(&context),
        "the net must answer here — a forced auction would pass every row vacuously"
    );

    // One empty ladder for every arm. `instinct()` reads the pinned relocation
    // field at build time, so a per-arm ladder would move the `relocating` row for two
    // reasons at once; empty is safe because the auction is not forced, and the
    // assert above keeps it that way.
    let ladder = Arc::new(Rules::new());
    let logits = |agreements: &ConventionCard| {
        ConfiguredFloorV5::new(CompactConfig::symmetric(agreements), Arc::clone(&ladder))
            .classify(seat, &context)
            .iter()
            .map(|(_, logit)| logit.to_bits())
            .collect::<Vec<u32>>()
    };

    let base = ConventionCard::capture(&crate::bidding::agreements::Agreements::default(), false);
    let base_slots = base.encode();
    let baseline = logits(&base);

    let check = |name: &str, flipped: ConventionCard, expect: &[usize]| {
        let moved: Vec<usize> = (0..LEN_COMPACT)
            .filter(|&slot| flipped.encode()[slot] != base_slots[slot])
            .collect();
        assert_eq!(moved, expect, "{name}: wrong slots moved");

        let after = logits(&flipped);
        if expect.iter().any(|slot| LIVE.contains(slot)) {
            assert_ne!(baseline, after, "{name}: a trained slot must move the net");
        } else {
            assert_eq!(baseline, after, "{name}: a folded slot must be inert");
        }
    };

    // `dutch` selects a book, not a knob, so it is the one axis with nothing to
    // arm — `capture` takes it as a parameter.
    check(
        "dutch",
        ConventionCard::capture(&crate::bidding::agreements::Agreements::default(), true),
        &[0],
    );

    // The build-time axes arm the captured value directly.
    let mut offshape = crate::bidding::agreements::Agreements::default();
    offshape.opening.one_notrump_offshape = true;
    check(
        "one_notrump_offshape",
        ConventionCard::capture(&offshape, false),
        &[12],
    );
    let mut shape = crate::bidding::agreements::Agreements::default();
    shape.opening.notrump_shape = NotrumpShape::Balanced;
    check("shape", ConventionCard::capture(&shape, false), &[13, 15]);
    let mut nmf = crate::bidding::agreements::Agreements::default();
    nmf.rebid.new_minor_forcing = true;
    check(
        "new_minor_forcing",
        ConventionCard::capture(&nmf, false),
        &[3],
    );
    let mut fsf = crate::bidding::agreements::Agreements::default();
    fsf.rebid.fourth_suit_forcing = false;
    check(
        "fourth_suit_forcing",
        ConventionCard::capture(&fsf, false),
        &[6],
    );
    let mut super_accept = crate::bidding::agreements::Agreements::default();
    super_accept.notrump.transfer_super_accept = true;
    check(
        "transfer_super_accept",
        ConventionCard::capture(&super_accept, false),
        &[5],
    );
    let mut jordan = crate::bidding::agreements::Agreements::default();
    jordan.competition.jordan_truscott = false;
    check(
        "jordan_truscott",
        ConventionCard::capture(&jordan, false),
        &[7],
    );
    let mut support_double = crate::bidding::agreements::Agreements::default();
    support_double.competition.major_support_double = false;
    check(
        "major_support_double",
        ConventionCard::capture(&support_double, false),
        &[10],
    );
    let mut lebensohl = crate::bidding::agreements::Agreements::default();
    lebensohl.competition.lebensohl_style = LebensohlStyle::Off;
    check(
        "lebensohl",
        ConventionCard::capture(&lebensohl, false),
        &[23, 25],
    );
    let mut leaping = crate::bidding::agreements::Agreements::default();
    leaping.defense.leaping_michaels_enabled = false;
    check(
        "leaping_michaels",
        ConventionCard::capture(&leaping, false),
        &[8],
    );
    let mut responsive = crate::bidding::agreements::Agreements::default();
    responsive.defense.responsive_takeout_enabled = false;
    check(
        "responsive_takeout",
        ConventionCard::capture(&responsive, false),
        &[9],
    );

    // Enum targets leave a trained lane deliberately: `Wide` (14), `Plain` (24)
    // and the five non-{Natural, Woolsey} defenses are folded, and `Wide` is
    // additionally unreachable through `from_card`.  These are the poles
    // `examples/dump-teacher`'s axis shards rotate.
    /// Name, arm the flip, and the slots it must move.
    type Axis = (&'static str, fn(&mut Agreements), &'static [usize]);
    let rows: &[Axis] = &[
        (
            "relocating",
            |a| a.decision.reading.rkcb_variant = RkcbVariant::Kickback,
            &[1],
        ),
        (
            "garbage_stayman",
            |a| a.decision.reading.garbage_stayman = false,
            &[2],
        ),
        ("xyz", |a| a.decision.reading.xyz = false, &[4]),
        (
            "nt_splinter",
            |a| a.decision.reading.nt_splinter = false,
            &[11],
        ),
        (
            "defense",
            |a| a.decision.reading.notrump_defense = NotrumpDefense::Woolsey,
            &[16, 19],
        ),
        (
            "minors_european",
            |a| a.decision.reading.notrump_minors = EUROPEAN,
            &[26],
        ),
        (
            "landy",
            |a| {
                a.decision.reading.landy = true;
                a.decision.reading.convention_points = (8, 14);
            },
            &[27],
        ),
    ];

    for (name, arm, expect) in rows {
        let mut agreements = Agreements::default();
        arm(&mut agreements);
        let flipped = ConventionCard::capture(&agreements, false);
        check(name, flipped, expect);
    }
}

/// An unattached compact config encodes as zeros rather than panicking in release
#[test]
fn features_v5_without_a_compact_config_is_zero_padded() {
    let hand = hand("AQ32.K53.QJ4.A92");
    let context = empty_context();
    // The debug assert in `features_v5` fires on a missing compact config, so
    // reach past it: this pins the *release* shape, that the vector is still
    // the right width rather than short.
    let mut out = features_v3(hand, &context);
    out.resize(FEATURES_LEN_V5, 0.0);
    assert_eq!(out.len(), FEATURES_LEN_V5);
    assert!(out[OFFSET_OUR_COMPACT..].iter().all(|value| *value == 0.0));
}

/// `legacy_view` is byte-exact: the nets see the pre-ceilings vector even
/// while the sampler, the gates and the floor see the tightened one.
///
/// That exactness is the whole point of the hedge — if the held view drifted
/// from a genuine ceilings-off read, its A/B arm would price a third thing
/// rather than isolating "the reading was wrong" from "the nets are stale".
/// It is not free: `net_inferences` reads the auction a second time, off the
/// uncompiled projection path, because a compiled plan serves only the
/// profile it was compiled under.
#[test]
fn legacy_view_reproduces_the_pre_ceilings_feature_vector() {
    use crate::bidding::features::{CompactConfig, ConventionCard, features_v5};
    use crate::bidding::inference::ReadingScope;
    // `1NT (2♠) 2NT (P)` — over their Muiderberg, our lebensohl relay is
    // gated `points(..=8)`, the ceiling N2 found missing.
    let auction = [
        Call::Bid(Bid {
            level: Level::new(1),
            strain: Strain::Notrump,
        }),
        Call::Bid(Bid {
            level: Level::new(2),
            strain: Strain::Spades,
        }),
        Call::Bid(Bid {
            level: Level::new(2),
            strain: Strain::Notrump,
        }),
        Call::Pass,
    ];
    let cards = hand("AQ32.K53.QJ4.A92");
    let vector = |ceilings: bool, legacy_view: bool| {
        let mut agreements = crate::bidding::agreements::Agreements::default();
        // Isolate the Phase 1 ceiling hedge in its historical reading scope.
        agreements.decision.reading.scope = ReadingScope::Alerted;
        agreements.decision.reading.strength_ceilings = ceilings;
        agreements.decision.legacy_view = legacy_view;
        let compact = CompactConfig::symmetric(&ConventionCard::capture(&agreements, true));
        let partnership = crate::american(&agreements).bind();
        let context = partnership
            .prefixed_context(RelativeVulnerability::NONE, &auction)
            .with_compact(&compact)
            .with_decision_cache(cards);
        features_v5(cards, &context)
    };

    let shipped = vector(false, false);
    assert_eq!(
        vector(true, true),
        shipped,
        "the held view must reproduce the shipped vector bit for bit"
    );
    assert_ne!(
        vector(true, false),
        shipped,
        "with the view off the ceilings must reach the nets — otherwise the \
         two A/B arms measure the same thing"
    );
    // The view is a *net* redirection only: the reading everything else
    // consumes still carries the ceiling.
    assert_ne!(vector(false, true), vector(true, false));
}

/// Phase 2 extends `legacy_view` across the reading-scope flip: frozen v5 nets
/// keep the alerted-only representation while direct consumers see every
/// authored natural call.
#[test]
fn legacy_view_reproduces_the_pre_all_feature_vector() {
    use crate::bidding::features::{CompactConfig, ConventionCard, features_v5};
    use crate::bidding::inference::ReadingScope;

    let auction = [
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Bid(Bid::new(1, Strain::Notrump)),
        Call::Pass,
        Call::Bid(Bid::new(3, Strain::Diamonds)),
        Call::Pass,
    ];
    let cards = hand("AQ32.K53.QJ4.A92");
    let vector = |scope: ReadingScope, legacy_view: bool| {
        let mut agreements = crate::bidding::agreements::Agreements::default();
        agreements.decision.reading.nt_overcall_gladiator = true;
        agreements.decision.reading.scope = scope;
        agreements.decision.reading.strength_ceilings = false;
        agreements.decision.reading.upgrade_closure = false;
        agreements.decision.legacy_view = legacy_view;
        let compact = CompactConfig::symmetric(&ConventionCard::capture(&agreements, true));
        let partnership = crate::american(&agreements).bind();
        let context = partnership
            .prefixed_context(RelativeVulnerability::NONE, &auction)
            .with_compact(&compact)
            .with_decision_cache(cards);
        features_v5(cards, &context)
    };

    let trained = vector(ReadingScope::Alerted, false);
    assert_eq!(vector(ReadingScope::All, true), trained);
    assert_ne!(vector(ReadingScope::All, false), trained);
}
