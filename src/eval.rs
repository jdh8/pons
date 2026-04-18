use core::cmp::Ord;
use core::iter::Sum;
use dds_bridge::{Hand, Holding, Rank, Suit};

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
pub struct SimpleEvaluator<T: Sum, F: Fn(Holding) -> T>(
    /// The per-suit kernel: invoked once per holding, its results are summed.
    pub F,
);

impl<T: Sum, F: Fn(Holding) -> T> HandEvaluator<T> for SimpleEvaluator<T, F> {
    fn eval(&self, hand: Hand) -> T {
        Suit::ASC.into_iter().map(|s| (self.0)(hand[s])).sum()
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
/// This is the well-known 4-3-2-1 point count by Milton Work.
#[must_use]
pub fn hcp<T: From<u8>>(holding: Holding) -> T {
    T::from(
        4 * u8::from(holding.contains(Rank::A))
            + 3 * u8::from(holding.contains(Rank::K))
            + 2 * u8::from(holding.contains(Rank::Q))
            + u8::from(holding.contains(Rank::J)),
    )
}

/// Short suit points
#[must_use]
// SAFETY: the integer to cast is in 0..=3, so the cast is safe.
#[allow(clippy::cast_possible_truncation)]
pub fn shortness<T: From<u8>>(holding: Holding) -> T {
    T::from(3 - holding.len().min(3) as u8)
}

/// The [Fifths] evaluator for 3NT
///
/// This function is the kernel of [`FIFTHS`].
///
/// [Fifths]: https://bridge.thomasoandrews.com/valuations/cardvaluesfor3nt.html
#[must_use]
pub fn fifths(holding: Holding) -> f64 {
    f64::from(
        40 * i32::from(holding.contains(Rank::A))
            + 28 * i32::from(holding.contains(Rank::K))
            + 18 * i32::from(holding.contains(Rank::Q))
            + 10 * i32::from(holding.contains(Rank::J))
            + 4 * i32::from(holding.contains(Rank::T)),
    ) / 10.0
}

/// The BUM-RAP evaluator
///
/// This function is the kernel of [`BUMRAP`].
#[must_use]
pub fn bumrap(holding: Holding) -> f64 {
    f64::from(
        18 * i32::from(holding.contains(Rank::A))
            + 12 * i32::from(holding.contains(Rank::K))
            + 6 * i32::from(holding.contains(Rank::Q))
            + 3 * i32::from(holding.contains(Rank::J))
            + i32::from(holding.contains(Rank::T)),
    ) * 0.25
}

/// Plain old losing trick count
#[must_use]
pub fn ltc<T: From<u8>>(holding: Holding) -> T {
    let len = holding.len();

    T::from(
        u8::from(len >= 1 && !holding.contains(Rank::A))
            + u8::from(len >= 2 && !holding.contains(Rank::K))
            + u8::from(len >= 3 && !holding.contains(Rank::Q)),
    )
}

/// New Losing Trick Count
///
/// This function is the kernel of [`NLTC`].
#[must_use]
pub fn nltc(holding: Holding) -> f64 {
    let len = holding.len();

    f64::from(
        3 * i32::from(len >= 1 && !holding.contains(Rank::A))
            + 2 * i32::from(len >= 2 && !holding.contains(Rank::K))
            + i32::from(len >= 3 && !holding.contains(Rank::Q)),
    ) * 0.5
}

/// High card points plus useful shortness
///
/// For each suit, we count max([HCP][hcp], shortness, HCP + shortness &minus; 1).
/// This method avoids double counting of short honors.  This evaluator is
/// particularly useful for suit contracts.
#[must_use]
pub fn hcp_plus<T: From<u8>>(holding: Holding) -> T {
    let count: u8 = hcp(holding);
    let short: u8 = shortness(holding);

    T::from(if count > 0 && short > 0 {
        count + short - 1
    } else {
        count.max(short)
    })
}

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
pub const BUMRAP_PLUS: SimpleEvaluator<f64, fn(Holding) -> f64> = SimpleEvaluator(|x| {
    let b: f64 = bumrap(x);
    let s: f64 = shortness(x);
    b.max(s).max(b + s - 1.0)
});

/// New Losing Trick Count
///
/// [NLTC](https://en.wikipedia.org/wiki/Losing-Trick_Count#New_Losing-Trick_Count_(NLTC))
/// is a variant of losing trick count that gives different weights to missing
/// honors.  A missing A/K/Q is worth 1.5/1.0/0.5 tricks respectively.
///
/// This evaluator calls [`nltc`] for each suit.
pub const NLTC: SimpleEvaluator<f64, fn(Holding) -> f64> = SimpleEvaluator(nltc);

/// [Zar points][zar], an evaluation by by Zar Petkov
///
/// [zar]: https://en.wikipedia.org/wiki/Zar_Points
pub fn zar<T: From<u8>>(hand: Hand) -> T {
    let holdings = Suit::ASC.map(|s| hand[s]);
    let mut lengths = holdings.map(Holding::len);
    lengths.sort_unstable();

    // SAFETY: the lengths are at most 13, so the cast is safe.
    #[allow(clippy::cast_possible_truncation)]
    let sum = (lengths[3] + lengths[2]) as u8;

    // SAFETY: `lengths` is already sorted, so the result is non-negative.
    #[allow(clippy::cast_possible_truncation)]
    let diff = (lengths[3] - lengths[0]) as u8;

    let honors: u8 = holdings
        .into_iter()
        .map(|holding| {
            let [a, k, q, j] = [Rank::A, Rank::K, Rank::Q, Rank::J].map(|r| holding.contains(r));
            let count = 6 * u8::from(a) + 4 * u8::from(k) + 2 * u8::from(q) + u8::from(j);
            let waste = match holding.len() {
                1 => k || q || j,
                2 => q || j,
                _ => false,
            };
            count - u8::from(waste)
        })
        .sum();

    T::from(honors + sum + diff)
}
