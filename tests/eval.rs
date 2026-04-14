use approx::assert_ulps_eq;
use dds_bridge::{Hand, Holding};
use pons::eval::{
    BUMRAP, BUMRAP_PLUS, FIFTHS, HandEvaluator as _, NLTC, SimpleEvaluator, hcp, hcp_plus, ltc,
    shortness, zar,
};

/// Test point counts with four kings
#[test]
#[allow(clippy::unusual_byte_groupings)]
fn test_four_kings() {
    const KXXX: Holding = Holding::from_bits_truncate(0b01000_0000_0111_00);
    const KXX: Holding = Holding::from_bits_truncate(0b01000_0000_0011_00);
    const HAND: Hand = Hand::new(KXXX, KXX, KXX, KXX);

    assert_eq!(SimpleEvaluator(hcp::<u8>).eval(HAND), 12);
    assert_ulps_eq!(FIFTHS.eval(HAND), 2.8 * 4.0);
    assert_ulps_eq!(BUMRAP.eval(HAND), 12.0);

    assert_eq!(SimpleEvaluator(ltc::<u8>).eval(HAND), 8);
    assert_ulps_eq!(NLTC.eval(HAND), 8.0);
    assert_eq!(zar::<u8>(HAND), 24);
}

/// Test a random hand from Cuebids: KJ53.K84.43.KT85
/// <https://cuebids.com/session/deal/yrBmPu9P4O20qzclHpX1>
#[test]
#[allow(clippy::unusual_byte_groupings)]
fn test_random_from_cuebids() {
    const KJ53: Holding = Holding::from_bits_truncate(0b01010_0000_1010_00);
    const K84: Holding = Holding::from_bits_truncate(0b01000_0100_0100_00);
    const XX: Holding = Holding::from_bits_truncate(0b00000_0000_0110_00);
    const KT85: Holding = Holding::from_bits_truncate(0b01001_0100_1000_00);
    const HAND: Hand = Hand::new(KT85, XX, K84, KJ53);

    assert_eq!(SimpleEvaluator(hcp::<u8>).eval(HAND), 10);
    assert_eq!(SimpleEvaluator(hcp_plus::<u8>).eval(HAND), 11);
    assert_ulps_eq!(FIFTHS.eval(HAND), 9.8);
    assert_ulps_eq!(BUMRAP.eval(HAND), 10.0);
    assert_ulps_eq!(BUMRAP_PLUS.eval(HAND), 11.0);

    assert_eq!(SimpleEvaluator(ltc::<u8>).eval(HAND), 8);
    assert_ulps_eq!(NLTC.eval(HAND), 8.5);
    assert_eq!(zar::<u8>(HAND), 23);
}

/// Test zar evaluator with shortness penalties (waste deductions)
#[test]
#[allow(clippy::unusual_byte_groupings)]
fn test_zar_waste() {
    // A9876.K.QJ.T5432 in S.H.D.C order
    // Hand::new takes (clubs, diamonds, hearts, spades)
    const A9876: Holding = Holding::from_bits_truncate(0b10000_1100_1100_00);
    const K: Holding = Holding::from_bits_truncate(0b01000_0000_0000_00);
    const QJ: Holding = Holding::from_bits_truncate(0b00110_0000_0000_00);
    const T5432: Holding = Holding::from_bits_truncate(0b00001_0000_1111_00);
    const HAND: Hand = Hand::new(T5432, QJ, K, A9876);

    // Spades A9876 (5 cards): A=6, no waste -> 6
    // Hearts K (1 card): K=4, waste (singleton K) -> 3
    // Diamonds QJ (2 cards): Q=2 + J=1 = 3, waste (Q/J in doubleton) -> 2
    // Clubs T5432 (5 cards): no honors -> 0
    // honors = 6 + 3 + 2 + 0 = 11
    // lengths sorted: [1, 2, 5, 5]
    // sum = 5 + 5 = 10, diff = 5 - 1 = 4
    // zar = 11 + 10 + 4 = 25
    assert_eq!(zar::<u8>(HAND), 25);
}

/// Test zar evaluator with a flat hand of aces
#[test]
#[allow(clippy::unusual_byte_groupings)]
fn test_zar_aces() {
    // AXXX.AXX.AXX.AXX where X is low cards
    const AXXX: Holding = Holding::from_bits_truncate(0b10000_0000_0111_00);
    const AXX: Holding = Holding::from_bits_truncate(0b10000_0000_0011_00);
    const HAND: Hand = Hand::new(AXXX, AXX, AXX, AXX);

    // Each ace: 6 points, no waste -> 24 honors
    // lengths sorted: [3, 3, 3, 4]
    // sum = 3 + 4 = 7, diff = 4 - 3 = 1
    // zar = 24 + 7 + 1 = 32
    assert_eq!(zar::<u8>(HAND), 32);
}

/// Test eval constants with an empty hand
#[test]
fn test_empty_hand() {
    let hand = Hand::default();
    assert_eq!(SimpleEvaluator(hcp::<u8>).eval(hand), 0);
    assert_eq!(SimpleEvaluator(shortness::<u8>).eval(hand), 12);
    assert_eq!(SimpleEvaluator(hcp_plus::<u8>).eval(hand), 12);
    assert_ulps_eq!(FIFTHS.eval(hand), 0.0);
    assert_ulps_eq!(BUMRAP.eval(hand), 0.0);
    assert_ulps_eq!(BUMRAP_PLUS.eval(hand), 12.0);
    assert_eq!(SimpleEvaluator(ltc::<u8>).eval(hand), 0);
    assert_ulps_eq!(NLTC.eval(hand), 0.0);
    assert_eq!(zar::<u8>(hand), 0);
}

/// Test eval_pair sums both hands
#[test]
#[allow(clippy::unusual_byte_groupings)]
fn test_eval_pair() {
    const KXXX: Holding = Holding::from_bits_truncate(0b01000_0000_0111_00);
    const KXX: Holding = Holding::from_bits_truncate(0b01000_0000_0011_00);
    const HAND: Hand = Hand::new(KXXX, KXX, KXX, KXX);

    assert_eq!(SimpleEvaluator(hcp::<u8>).eval_pair([HAND, HAND]), 24);
}

/// Test BUMRAP_PLUS intermediate values
#[test]
#[allow(clippy::unusual_byte_groupings)]
fn test_bumrap_plus_shortness() {
    // Void: shortness = 3, bumrap = 0 -> max(3, 0, 2) = 3
    assert_ulps_eq!(BUMRAP_PLUS.0(Holding::EMPTY), 3.0);

    // Singleton ace: shortness = 2, bumrap = 4.5 -> max(4.5, 2, 5.5) = 5.5
    const A: Holding = Holding::from_bits_truncate(0b10000_0000_0000_00);
    assert_ulps_eq!(BUMRAP_PLUS.0(A), 5.5);
}
