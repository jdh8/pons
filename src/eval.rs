use core::iter::Sum;
use dds_bridge::{Hand, Holding, SmallSet as _};

/// Trait for hand evaluators
pub trait HandEvaluator<T> {
    /// Evaluate a hand
    #[must_use]
    fn eval(&self, hand: Hand) -> T;

    /// Evaluate a pair
    #[must_use]
    fn eval_pair(&self, pair: [Hand; 2]) -> T
    where
        T: core::ops::Add<Output = T>,
    {
        self.eval(pair[0]) + self.eval(pair[1])
    }
}

/// Functions are natural evaluators
impl<F: Fn(Hand) -> T, T> HandEvaluator<T> for F {
    fn eval(&self, hand: Hand) -> T {
        self(hand)
    }
}

/// Evaluator summing values of suit holdings
#[derive(Debug)]
pub struct SimpleEvaluator<T: Sum, F: Fn(Holding) -> T>(pub F);

impl<T: Sum, F: Fn(Holding) -> T> HandEvaluator<T> for SimpleEvaluator<T, F> {
    fn eval(&self, hand: Hand) -> T {
        hand.0.into_iter().map(&self.0).sum()
    }
}

impl<T: Sum, F: Clone + Fn(Holding) -> T> Clone for SimpleEvaluator<T, F> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Sum, F: Copy + Fn(Holding) -> T> Copy for SimpleEvaluator<T, F> {}

/// High card points
///
/// This function is the kernel of [`HCP`].
#[must_use]
pub fn hcp(holding: Holding) -> f64 {
    f64::from(
        4 * i32::from(holding.contains(14))
            + 3 * i32::from(holding.contains(13))
            + 2 * i32::from(holding.contains(12))
            + i32::from(holding.contains(11)),
    )
}

/// Short suit points
#[must_use]
// SAFETY: the integer to cast is in 0..=3, so the cast is safe.
#[allow(clippy::cast_precision_loss)]
pub fn shortness(holding: Holding) -> f64 {
    (3 - holding.len().min(3)) as f64
}

/// The [Fifths] evaluator for 3NT
///
/// This function is the kernel of [`FIFTHS`].
///
/// [Fifths]: https://bridge.thomasoandrews.com/valuations/cardvaluesfor3nt.html
#[must_use]
pub fn fifths(holding: Holding) -> f64 {
    f64::from(
        40 * i32::from(holding.contains(14))
            + 28 * i32::from(holding.contains(13))
            + 18 * i32::from(holding.contains(12))
            + 10 * i32::from(holding.contains(11))
            + 4 * i32::from(holding.contains(10)),
    ) / 10.0
}

/// The BUM-RAP evaluator
///
/// This function is the kernel of [`BUMRAP`].
#[must_use]
pub fn bumrap(holding: Holding) -> f64 {
    f64::from(
        18 * i32::from(holding.contains(14))
            + 12 * i32::from(holding.contains(13))
            + 6 * i32::from(holding.contains(12))
            + 3 * i32::from(holding.contains(11))
            + i32::from(holding.contains(10)),
    ) * 0.25
}

/// Plain old losing trick count
///
/// This function is the kernel of [`LTC`].
#[must_use]
pub fn ltc(holding: Holding) -> f64 {
    let len = holding.len();

    f64::from(
        i32::from(len >= 1 && !holding.contains(14))
            + i32::from(len >= 2 && !holding.contains(13))
            + i32::from(len >= 3 && !holding.contains(12)),
    )
}

/// New Losing Trick Count
///
/// This function is the kernel of [`NLTC`].
#[must_use]
pub fn nltc(holding: Holding) -> f64 {
    let len = holding.len();

    f64::from(
        3 * i32::from(len >= 1 && !holding.contains(14))
            + 2 * i32::from(len >= 2 && !holding.contains(13))
            + i32::from(len >= 3 && !holding.contains(12)),
    ) * 0.5
}

/// High card points
///
/// This is the well-known 4-3-2-1 point count by Milton Work.  Evaluation of
/// each suit is done by [`hcp`].
pub const HCP: SimpleEvaluator<f64, fn(Holding) -> f64> = SimpleEvaluator(hcp);

/// High card points plus useful shortness
///
/// For each suit, we count max([HCP], shortness, HCP + shortness &minus; 1).
/// This method avoids double counting of short honors.  This evaluator is
/// particularly useful for suit contracts.
pub const HCP_PLUS: SimpleEvaluator<f64, fn(Holding) -> f64> =
    SimpleEvaluator(|x| hcp(x).max(shortness(x)));

/// The [Fifths] evaluator for 3NT
///
/// This is Thomas Andrews's computed point count for 3NT.  This evaluator calls
/// [`fifths`] for each suit.
///
/// [Fifths]: https://bridge.thomasoandrews.com/valuations/cardvaluesfor3nt.html
pub const FIFTHS: SimpleEvaluator<f64, fn(Holding) -> f64> = SimpleEvaluator(fifths);

/// The BUM-RAP evaluator
///
/// This is the BUM-RAP point count (4.5-3-1.5-0.75-0.25).  This evaluator calls
/// [`bumrap`] for each suit.
pub const BUMRAP: SimpleEvaluator<f64, fn(Holding) -> f64> = SimpleEvaluator(bumrap);

/// BUM-RAP with shortness
///
/// For each suit, we count max([BUM-RAP][BUMRAP], shortness, BUM-RAP +
/// shortness &minus; 1).  This method avoids double counting of short honors.
/// This evaluator is particularly useful for suit contracts.
pub const BUMRAP_PLUS: SimpleEvaluator<f64, fn(Holding) -> f64> =
    SimpleEvaluator(|x| bumrap(x).max(shortness(x)));

/// Plain old losing trick count
pub const LTC: SimpleEvaluator<f64, fn(Holding) -> f64> = SimpleEvaluator(ltc);

/// New Losing Trick Count
///
/// [NLTC](https://en.wikipedia.org/wiki/Losing-Trick_Count#New_Losing-Trick_Count_(NLTC))
/// is a variant of losing trick count that gives different weights to missing
/// honors.  A missing A/K/Q is worth 1.5/1.0/0.5 tricks respectively.
///
/// This evaluator calls [`nltc`] for each suit.
pub const NLTC: SimpleEvaluator<f64, fn(Holding) -> f64> = SimpleEvaluator(nltc);

/// Test point counts with four kings
#[test]
#[allow(clippy::unusual_byte_groupings)]
fn test_four_kings() {
    use approx::assert_ulps_eq;

    const KXXX: Holding = Holding::from_bits(0b01000_0000_0111_00);
    const KXX: Holding = Holding::from_bits(0b01000_0000_0011_00);
    const HAND: Hand = Hand([KXXX, KXX, KXX, KXX]);

    assert_ulps_eq!(HCP.eval(HAND), 12.0);
    assert_ulps_eq!(FIFTHS.eval(HAND), 2.8 * 4.0);
    assert_ulps_eq!(BUMRAP.eval(HAND), 12.0);
    assert_ulps_eq!(LTC.eval(HAND), 8.0);
    assert_ulps_eq!(NLTC.eval(HAND), 8.0);
}

/// Test a random hand from Cuebids: KJ53.K84.43.KT85
/// <https://cuebids.com/session/deal/yrBmPu9P4O20qzclHpX1>
#[test]
#[allow(clippy::unusual_byte_groupings)]
fn test_random_from_cuebids() {
    use approx::assert_ulps_eq;

    const KJ53: Holding = Holding::from_bits(0b01010_0000_1010_00);
    const K84: Holding  = Holding::from_bits(0b01000_0100_0100_00);
    const XX: Holding   = Holding::from_bits(0b00000_0000_0110_00);
    const KT85: Holding = Holding::from_bits(0b01001_0100_1000_00);
    const HAND: Hand = Hand([KT85, XX, K84, KJ53]);

    assert_ulps_eq!(HCP.eval(HAND), 10.0);
    assert_ulps_eq!(HCP_PLUS.eval(HAND), 11.0);
    assert_ulps_eq!(FIFTHS.eval(HAND), 9.8);
    assert_ulps_eq!(BUMRAP.eval(HAND), 10.0);
    assert_ulps_eq!(BUMRAP_PLUS.eval(HAND), 11.0);
    assert_ulps_eq!(LTC.eval(HAND), 8.0);
    assert_ulps_eq!(NLTC.eval(HAND), 8.5);
}
