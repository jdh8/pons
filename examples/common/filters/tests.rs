use super::*;

fn hand(text: &str) -> Hand {
    text.parse().unwrap()
}

/// The coverage hole `--wide-1nt` closes: a 15-HCP 5m422 the default
/// `Wide6322` opens 1NT passes the wide scan and fails the strict one.
#[test]
fn wide_candidate_admits_the_semi_balanced_opener() {
    let five_four_two_two = hand("K2.AJ.KQ32.Q5432");
    assert_eq!(hand_hcp(five_four_two_two), 15);
    assert!(is_wide_1nt_opener(five_four_two_two));
    assert!(!is_1nt_opener(five_four_two_two));

    let flat = hand("AKQ2.K32.K32.432");
    assert!(is_1nt_opener(flat) && is_wide_1nt_opener(flat));

    // Neither admits a singleton or a hand outside 15–17.
    let singleton = hand("AK32.K32.KQ432.2");
    assert!(!is_1nt_opener(singleton) && !is_wide_1nt_opener(singleton));
    let eighteen = hand("AKQ2.K32.KQ2.J32");
    assert_eq!(hand_hcp(eighteen), 18);
    assert!(!is_1nt_opener(eighteen) && !is_wide_1nt_opener(eighteen));
}

/// `--filter-landy` pairs the candidate with its **LHO**: the same Landy hand
/// at the candidate's RHO is rejected.
#[test]
fn landy_scan_pairs_the_candidate_with_its_lho() {
    // North is the only 1NT candidate; East (North's LHO) is 5-4 in the majors.
    let lho: FullDeal = "N:AKQ2.K32.K32.432 JT987.AQJT.A5.65 654.987.QJT9.KQJ 3.654.8764.AT987"
        .parse()
        .unwrap();
    // The same hands with East and West swapped: the Landy hand is now RHO.
    let rho: FullDeal = "N:AKQ2.K32.K32.432 3.654.8764.AT987 654.987.QJT9.KQJ JT987.AQJT.A5.65"
        .parse()
        .unwrap();
    for wide in [false, true] {
        assert!(Filter::OneNt.accepts(&lho, wide) && Filter::OneNt.accepts(&rho, wide));
        assert!(Filter::Landy.accepts(&lho, wide));
        assert!(!Filter::Landy.accepts(&rho, wide));
    }
}

/// `--filter-spade-raise` pairs the four-spade opener with its **partner**,
/// and a five-spade opener (who opens `1♠`) is no raiser.
#[test]
fn spade_raise_scan_pairs_the_raiser_with_its_partner() {
    // North raises (AK32 spades, 15 HCP, a singleton); South holds QJ54.
    let partner: FullDeal = "N:AK32.KQ32.AJ432. T98.AJT9.KQ5.T98 QJ54.654.87.KQJ2 76.87.T96.A76543"
        .parse()
        .unwrap();
    assert!(Filter::SpadeRaise.accepts(&partner, false));
    // East and South swapped: the four spades now sit at North's LHO.
    let lho: FullDeal = "N:AK32.KQ32.AJ432. QJ54.654.87.KQJ2 T98.AJT9.KQ5.T98 76.87.T96.A76543"
        .parse()
        .unwrap();
    assert!(!Filter::SpadeRaise.accepts(&lho, false));

    assert!(is_spade_raiser(hand("AK32.KQ32.AJ432.")));
    assert!(!is_spade_raiser(hand("AK432.KQ3.AJ43.2")));
    // The strict 1NT opener opens 1NT, not a suit.
    assert!(!is_spade_raiser(hand("AKQ2.K32.K32.432")));
}

/// `--filter-heart-raise` pairs the four-heart opener with its **partner**,
/// and a five-card major (which opens `1M`) is no raiser.
#[test]
fn heart_raise_scan_pairs_the_raiser_with_its_partner() {
    let partner: FullDeal = "N:KQ32.AK32.AJ432. AJT9.T98.KQ5.T98 654.QJ54.87.KQJ2 87.76.T96.A76543"
        .parse()
        .unwrap();
    assert!(Filter::HeartRaise.accepts(&partner, false));
    // East and South swapped: the four hearts now sit at North's LHO.
    let lho: FullDeal = "N:KQ32.AK32.AJ432. 654.QJ54.87.KQJ2 AJT9.T98.KQ5.T98 87.76.T96.A76543"
        .parse()
        .unwrap();
    assert!(!Filter::HeartRaise.accepts(&lho, false));

    assert!(is_heart_raiser(hand("KQ32.AK32.AJ432.")));
    assert!(!is_heart_raiser(hand("KQ3.AK432.AJ43.2")));
    assert!(!is_heart_raiser(hand("AJ432.AK32.K43.2")));
    assert!(!is_heart_raiser(hand("K32.AKQ2.K32.432")));
}
