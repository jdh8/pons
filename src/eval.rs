use core::iter::Sum;
use dds_bridge::deal::{Hand, Holding, SmallSet as _};

/// Trait for hand evaluators
///
/// This trait might be replaced with [`Fn`] in the future.
pub trait HandEvaluator<T> {
    /// Evaluate a hand
    fn call(&self, hand: Hand) -> T;
}

/// Evaluator summing values of suit holdings
pub struct SimpleEvaluator<T: Sum, F: Fn(Holding) -> T>(F);

impl<T: Sum, F: Fn(Holding) -> T> HandEvaluator<T> for SimpleEvaluator<T, F> {
    fn call(&self, hand: Hand) -> T {
        hand.0.into_iter().map(&self.0).sum()
    }
}

/// High card points
///
/// This function is the kernel of [`HCP`].
#[must_use]
pub fn hcp(holding: Holding) -> i32 {
    4 * i32::from(holding.contains(14))
        + 3 * i32::from(holding.contains(13))
        + 2 * i32::from(holding.contains(12))
        + i32::from(holding.contains(11))
}

/// The [Fifths] evaluator for 3NT, &times; 10
///
/// This function is the kernel of [`DECI_FIFTHS`].
///
/// [Fifths]: https://bridge.thomasoandrews.com/valuations/cardvaluesfor3nt.html
#[must_use]
pub fn deci_fifths(holding: Holding) -> i32 {
    40 * i32::from(holding.contains(14))
        + 28 * i32::from(holding.contains(13))
        + 18 * i32::from(holding.contains(12))
        + 10 * i32::from(holding.contains(11))
        + 4 * i32::from(holding.contains(10))
}

/// Plain old losing trick count
///
/// This function is the kernel of [`LTC`].
#[must_use]
pub fn ltc(holding: Holding) -> i32 {
    let len = holding.len();
    
    i32::from(len >= 1 && !holding.contains(14))
        + i32::from(len >= 2 && !holding.contains(13))
        + i32::from(len >= 3 && !holding.contains(12))
}

/// New Losing Trick Count &times; 2
///
/// This function is the kernel of [`HALF_NLTC`].
#[must_use]
pub fn half_nltc(holding: Holding) -> i32 {
    let len = holding.len();
    
    3 * i32::from(len >= 1 && !holding.contains(14))
        + 2 * i32::from(len >= 2 && !holding.contains(13))
        + i32::from(len >= 3 && !holding.contains(12))
}

/// High card points
///
/// This is the well-known 4-3-2-1 point count by Milton Work.  Evaluation of
/// each suit is done by [`hcp`].
pub const HCP: SimpleEvaluator<i32, fn(Holding) -> i32> = SimpleEvaluator(hcp);

/// The [Fifths] evaluator for 3NT, &times; 10
///
/// This is 10 &times; Thomas Andrews's computed point count for 3NT.  We make
/// the result an integer to improve interoperability.  This evaluator calls
/// [`deci_fifths`] for each suit.
///
/// [Fifths]: https://bridge.thomasoandrews.com/valuations/cardvaluesfor3nt.html
pub const DECI_FIFTHS: SimpleEvaluator<i32, fn(Holding) -> i32> = SimpleEvaluator(deci_fifths);

/// Plain old losing trick count
pub const LTC: SimpleEvaluator<i32, fn(Holding) -> i32> = SimpleEvaluator(ltc);

/// New Losing Trick Count &times; 2
///
/// [NLTC](https://en.wikipedia.org/wiki/Losing-Trick_Count#New_Losing-Trick_Count_(NLTC))
/// is a variant of losing trick count that gives different weights to missing
/// honors.  A missing A/K/Q is worth 1.5/1.0/0.5 tricks respectively.
///
/// This evaluator counts half losers to make the result an integer. This
/// evaluator calls [`half_nltc`] for each suit.
pub const HALF_NLTC: SimpleEvaluator<i32, fn(Holding) -> i32> = SimpleEvaluator(half_nltc);

/// Test point counts with four kings
#[test]
#[allow(clippy::unusual_byte_groupings)]
fn test_four_kings() {
    const KXXX: Holding = Holding::from_bits(0b01000_0000_0111_00);
    const KXX: Holding = Holding::from_bits(0b01000_0000_0011_00);
    const HAND: Hand = Hand([KXXX, KXX, KXX, KXX]);
    assert_eq!(HCP.call(HAND), 12);
    assert_eq!(DECI_FIFTHS.call(HAND), 28 * 4);
}
