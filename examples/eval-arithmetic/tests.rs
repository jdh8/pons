use super::*;

/// The design row must read the corpus layout the sidecar documents: own
/// hand in `[0, 34)`, three range blocks after it, labels from `FEATURES`.
/// A silent off-by-one here would fit clean-looking nonsense, so pin the
/// arithmetic against a row built by hand.
#[test]
fn design_reads_the_documented_layout() {
    let mut row = [0f32; ROW_LEN];
    // Own hand: 5=♣, 3=♦, 4=♥, 1=♠ with ♠A, ♥KQ, and 12 HCP overall.
    for (suit, len) in [5.0, 3.0, 4.0, 1.0].into_iter().enumerate() {
        row[suit * SUIT_BITS] = len / 13.0;
    }
    row[3 * SUIT_BITS + 3] = 1.0; // ♠A
    row[2 * SUIT_BITS + 4] = 1.0; // ♥K
    row[2 * SUIT_BITS + 5] = 1.0; // ♥Q
    row[2 * SUIT_BITS + 2] = 5.0 / 10.0; // ♥ suit HCP = K+Q
    row[3 * SUIT_BITS + 2] = 4.0 / 10.0; // ♠ suit HCP = A
    row[32] = 12.0 / 40.0;
    // Partner (seat 1 of the range blocks): 11–14 points, ♥ 4–5.
    let partner = RANGES + SEAT_RANGE;
    row[partner + 12] = 11.0 / 37.0;
    row[partner + 13] = 14.0 / 37.0;
    row[partner + 2 * 3] = 4.0 / 13.0;
    row[partner + 2 * 3 + 1] = 5.0 / 13.0;

    // Target 2*4 + 0 = ♥ played by `me`.
    let v = design(&row, 8);
    assert!((v[0] - 1.0).abs() < 1e-9);
    assert!((v[1] - (12.0 + 12.5)).abs() < 1e-6, "pair HCP: {}", v[1]);
    assert!((v[2] - (4.0 + 4.5)).abs() < 1e-6, "♥ fit: {}", v[2]);
    assert!((v[3] - 1.0).abs() < 1e-9, "aces: {}", v[3]);
    assert!((v[4] - 1.0).abs() < 1e-9, "kings: {}", v[4]);
    assert!((v[5] - 5.0).abs() < 1e-6, "♥ honour strength: {}", v[5]);

    // Same deal, played by LHO: the pairs swap, and the fit is now the
    // opponents' — both unshown here, so zero.
    let w = design(&row, 9);
    assert!((w[1] - 0.0).abs() < 1e-9, "their HCP: {}", w[1]);
    assert!((w[6] - (12.0 + 12.5)).abs() < 1e-6, "our HCP: {}", w[6]);
}

/// Nested rungs are the reason one Gram serves the whole ladder: solving
/// the `k × k` prefix must be the same as fitting that rung alone.
#[test]
fn rungs_are_nested_prefixes() {
    assert!(RUNGS.windows(2).all(|w| w[0].1 < w[1].1));
    assert_eq!(RUNGS.last().expect("non-empty ladder").1, K);
}
