use core::iter::Sum;
use dds_bridge::deal::{Hand, Holding, SmallSet as _};

pub trait HandEvaluator<T> {
    fn call(&self, hand: Hand) -> T;
}

pub struct SimpleEvaluator<T: Sum, F: Fn(Holding) -> T>(F);

impl<T: Sum, F: Fn(Holding) -> T> HandEvaluator<T> for SimpleEvaluator<T, F> {
    fn call(&self, hand: Hand) -> T {
        hand.0.into_iter().map(&self.0).sum()
    }
}

fn hcp(holding: Holding) -> i32 {
    4 * i32::from(holding.contains(14))
        + 3 * i32::from(holding.contains(13))
        + 2 * i32::from(holding.contains(12))
        + i32::from(holding.contains(11))
}

fn deci_fifths(holding: Holding) -> i32 {
    40 * i32::from(holding.contains(14))
        + 28 * i32::from(holding.contains(13))
        + 18 * i32::from(holding.contains(12))
        + 10 * i32::from(holding.contains(11))
        + 4 * i32::from(holding.contains(10))
}

pub const HCP: SimpleEvaluator<i32, fn(Holding) -> i32> = SimpleEvaluator(hcp);
pub const DECI_FIFTHS: SimpleEvaluator<i32, fn(Holding) -> i32> = SimpleEvaluator(deci_fifths);

#[test]
#[allow(clippy::unusual_byte_groupings)]
fn test_four_kings() {
    const KXXX: Holding = Holding::from_bits(0b01000_0000_0111_00);
    const KXX: Holding = Holding::from_bits(0b01000_0000_0011_00);
    const HAND: Hand = Hand([KXXX, KXX, KXX, KXX]);
    assert_eq!(HCP.call(HAND), 12);
    assert_eq!(DECI_FIFTHS.call(HAND), 28 * 4);
}
