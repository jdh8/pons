use super::{Bid, Call, Strain};
use core::convert::Infallible;
use core::iter::Enumerate;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeInclusive};
use dds_bridge::Level;

/// Number of possible calls
const CALL_VARIANTS: usize = 3 + 7 * 5;

/// Hash/encode calls into indices for array storage
const fn encode_call(call: Call) -> usize {
    match call {
        Call::Pass => 0,
        Call::Double => 1,
        Call::Redouble => 2,
        Call::Bid(bid) => encode_bid(bid),
    }
}

/// Encode a bid into its index in the array
const fn encode_bid(bid: Bid) -> usize {
    3 + (bid.level.get() as usize - 1) * 5 + bid.strain as usize
}

const _: () = {
    let mut calls = [Call::Pass; CALL_VARIANTS];
    let mut level: u8 = 1;
    let mut strain = 0;

    while level <= 7 {
        while strain <= 4 {
            let bid = Bid {
                level: Level::new(level),
                strain: Strain::ASC[strain],
            };
            calls[encode_call(Call::Bid(bid))] = Call::Bid(bid);
            strain += 1;
        }
        strain = 0;
        level += 1;
    }

    assert!(encode_call(Call::Pass) == 0);
    assert!(encode_call(Call::Double) == 1);
    assert!(encode_call(Call::Redouble) == 2);

    let mut index = 3;

    while index < CALL_VARIANTS {
        assert!(matches!(calls[index], Call::Bid(_)));
        index += 1;
    }
};

/// Decode indices back to calls
const fn decode_call(index: usize) -> Call {
    match index {
        0 => Call::Pass,
        1 => Call::Double,
        2 => Call::Redouble,
        3..=37 => {
            let code = index - 3 + 5;
            let (level, strain) = (code / 5, code % 5);

            Call::Bid(super::Bid {
                #[allow(clippy::cast_possible_truncation)]
                level: Level::new(level as u8),
                strain: super::Strain::ASC[strain],
            })
        }
        _ => panic!("Invalid call ID!"),
    }
}

/// Compile-time assertion that `encode_call` and `decode_call` are inverses
const _: () = {
    let mut id = 0;

    while id < CALL_VARIANTS {
        let call = decode_call(id);
        assert!(encode_call(call) == id);
        id += 1;
    }
};

#[test]
fn test_encode_special_calls() {
    assert_eq!(encode_call(Call::Pass), 0);
    assert_eq!(encode_call(Call::Double), 1);
    assert_eq!(encode_call(Call::Redouble), 2);
}

#[test]
#[should_panic(expected = "Invalid call ID!")]
fn test_decode_call_invalid() {
    decode_call(CALL_VARIANTS);
}

/// Fixed-size array indexed by [`Call`]s
///
/// Like a mathematical function, every potentially valid call maps to a
/// corresponding value.  This type serves as the underlying hash table for
/// [`super::Map`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Array<T>([T; CALL_VARIANTS]);

/// Iterator over values by reference
pub type Values<'a, T> = core::slice::Iter<'a, T>;

/// Iterator over values by mutable reference
pub type ValuesMut<'a, T> = core::slice::IterMut<'a, T>;

/// Iterator over moving values
pub type IntoValues<T> = core::array::IntoIter<T, CALL_VARIANTS>;

impl<T> Array<T> {
    /// Create a new array from a function
    #[must_use]
    pub fn from_fn(mut f: impl FnMut(Call) -> T) -> Self {
        Self(core::array::from_fn(|index| f(decode_call(index))))
    }

    /// Get the value corresponding to a call
    #[must_use]
    pub const fn get(&self, call: Call) -> &T {
        &self.0[encode_call(call)]
    }

    /// Get the mutable reference to the value corresponding to a call
    #[must_use]
    pub const fn get_mut(&mut self, call: Call) -> &mut T {
        &mut self.0[encode_call(call)]
    }

    /// Borrow all values
    pub const fn each_ref(&self) -> Array<&T> {
        Array(self.0.each_ref())
    }

    /// Mutably borrow all values
    pub const fn each_mut(&mut self) -> Array<&mut T> {
        Array(self.0.each_mut())
    }

    /// Visit all key-value pairs in the table
    pub fn iter(&self) -> Iter<'_, T> {
        self.into_iter()
    }

    /// Visit all key-value pairs in the table with mutable access to the values
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.into_iter()
    }

    /// Fallible [`map`][Self::map] that fails fast
    ///
    /// # Errors
    ///
    /// Returns the first error produced by the mapping function, if any.
    pub fn try_map<U, E>(self, mut f: impl FnMut(Call, T) -> Result<U, E>) -> Result<Array<U>, E> {
        let mut result = [const { MaybeUninit::uninit() }; CALL_VARIANTS];

        for (index, value) in self.0.into_iter().enumerate() {
            match f(decode_call(index), value) {
                Ok(u) => result[index] = MaybeUninit::new(u),
                Err(e) => {
                    // Drop the already-initialized entries before returning.
                    // SAFETY: `result[..index]` are initialized by previous iterations.
                    unsafe { result[..index].assume_init_drop() };
                    return Err(e);
                }
            }
        }

        // SAFETY: All entries in `result` are initialized by the loop above,
        // and `Array` is a transparent wrapper around an array.
        Ok(Array(unsafe { core::mem::transmute_copy(&result) }))
    }

    /// Map all values with a function
    #[allow(clippy::missing_panics_doc)]
    pub fn map<U>(self, mut f: impl FnMut(Call, T) -> U) -> Array<U> {
        self.try_map::<_, Infallible>(|call, value| Ok(f(call, value)))
            .unwrap()
    }

    /// Visit all values
    pub fn values(&self) -> Values<'_, T> {
        self.0.iter()
    }

    /// Visit all values with mutable access
    pub fn values_mut(&mut self) -> ValuesMut<'_, T> {
        self.0.iter_mut()
    }

    /// Consume all values
    pub fn into_values(self) -> IntoValues<T> {
        self.0.into_iter()
    }
}

impl<T> Array<Option<T>> {
    /// New array with all [`None`] values
    #[must_use]
    pub const fn new() -> Self {
        Self([const { None }; CALL_VARIANTS])
    }
}

impl<T: Clone> Array<T> {
    /// Create a new array with all entries set to the same value
    #[must_use]
    pub fn repeat(value: T) -> Self {
        Self(core::array::repeat(value))
    }
}

impl<T> Index<Call> for Array<T> {
    type Output = T;

    fn index(&self, call: Call) -> &Self::Output {
        self.get(call)
    }
}

impl<T> IndexMut<Call> for Array<T> {
    fn index_mut(&mut self, call: Call) -> &mut Self::Output {
        self.get_mut(call)
    }
}

impl<T> Index<RangeFull> for Array<T> {
    type Output = [T];

    fn index(&self, _: RangeFull) -> &Self::Output {
        &self.0
    }
}

impl<T> IndexMut<RangeFull> for Array<T> {
    fn index_mut(&mut self, _: RangeFull) -> &mut Self::Output {
        &mut self.0
    }
}

impl<T> Index<Range<Bid>> for Array<T> {
    type Output = [T];

    fn index(&self, range: Range<Bid>) -> &Self::Output {
        let start = encode_bid(range.start);
        let end = encode_bid(range.end);
        &self.0[start..end]
    }
}

impl<T> IndexMut<Range<Bid>> for Array<T> {
    fn index_mut(&mut self, range: Range<Bid>) -> &mut Self::Output {
        let start = encode_bid(range.start);
        let end = encode_bid(range.end);
        &mut self.0[start..end]
    }
}

impl<T> Index<RangeFrom<Bid>> for Array<T> {
    type Output = [T];

    fn index(&self, range: RangeFrom<Bid>) -> &Self::Output {
        let start = encode_bid(range.start);
        &self.0[start..]
    }
}

impl<T> IndexMut<RangeFrom<Bid>> for Array<T> {
    fn index_mut(&mut self, range: RangeFrom<Bid>) -> &mut Self::Output {
        let start = encode_bid(range.start);
        &mut self.0[start..]
    }
}

impl<T> Index<RangeInclusive<Bid>> for Array<T> {
    type Output = [T];

    fn index(&self, range: RangeInclusive<Bid>) -> &Self::Output {
        let start = encode_bid(*range.start());
        let end = encode_bid(*range.end());
        &self.0[start..=end]
    }
}

impl<T> IndexMut<RangeInclusive<Bid>> for Array<T> {
    fn index_mut(&mut self, range: RangeInclusive<Bid>) -> &mut Self::Output {
        let start = encode_bid(*range.start());
        let end = encode_bid(*range.end());
        &mut self.0[start..=end]
    }
}

impl<T: Default> Default for Array<T> {
    fn default() -> Self {
        Self::from_fn(|_| T::default())
    }
}

/// Iterator by reference
pub type Iter<'a, T> = core::iter::Map<Enumerate<Values<'a, T>>, fn((usize, &T)) -> (Call, &T)>;

/// Iterator by mutable reference
pub type IterMut<'a, T> =
    core::iter::Map<Enumerate<ValuesMut<'a, T>>, fn((usize, &mut T)) -> (Call, &mut T)>;

/// Iterator by value
pub type IntoIter<T> = core::iter::Map<Enumerate<IntoValues<T>>, fn((usize, T)) -> (Call, T)>;

impl<'a, T> IntoIterator for &'a Array<T> {
    type Item = (Call, &'a T);
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.values()
            .enumerate()
            .map(|(index, entry)| (decode_call(index), entry))
    }
}

impl<'a, T> IntoIterator for &'a mut Array<T> {
    type Item = (Call, &'a mut T);
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.values_mut()
            .enumerate()
            .map(|(index, entry)| (decode_call(index), entry))
    }
}

impl<T> IntoIterator for Array<T> {
    type Item = (Call, T);
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_values()
            .enumerate()
            .map(|(index, entry)| (decode_call(index), entry))
    }
}

/// Logits for all calls
///
/// Logits are log odds.  Their additive nature allows linear operations.  This
/// property is useful for machine learning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Logits(pub Array<f32>);

impl Logits {
    /// Initialize all logits to negative infinity
    ///
    /// A logit of negative infinity corresponds to a probability of zero.  This
    /// means that all calls are initially considered impossible until evidence
    /// suggests otherwise.
    #[must_use]
    pub const fn new() -> Self {
        Self(Array([f32::NEG_INFINITY; CALL_VARIANTS]))
    }

    /// Compute the softmax (normalized probability distribution)
    ///
    /// The maximum logit is subtracted before exponentiation for numerical stability.
    ///
    /// Returns [`None`] if all logits are [`f32::NEG_INFINITY`] (the default),
    /// meaning no call has any probability mass.
    #[must_use]
    pub fn softmax(self) -> Option<Array<f32>> {
        let max = self.into_values().fold(f32::NEG_INFINITY, f32::max);

        (max > f32::NEG_INFINITY).then(|| {
            let exp: [_; CALL_VARIANTS] = core::array::from_fn(|i| (self.0.0[i] - max).exp());
            let sum: f32 = exp.iter().sum();
            Array(core::array::from_fn(|i| exp[i] / sum))
        })
    }
}

impl Default for Logits {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Logits {
    type Target = Array<f32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Logits {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
