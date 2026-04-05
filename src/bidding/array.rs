use super::{Bid, Call, Strain};
use core::iter::Enumerate;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeInclusive};

/// Number of possible calls
const CALL_VARIANTS: usize = 3 + 7 * 5;

/// Hash/encode calls into indices for array storage
const fn encode_call(call: Call) -> usize {
    match call {
        Call::Pass => 0,
        Call::Double => 1,
        Call::Redouble => 2,
        Call::Bid(bid) => 3 + (bid.level - 1) as usize * 5 + bid.strain as usize,
    }
}

const _: () = {
    let mut calls = [Call::Pass; CALL_VARIANTS];
    let mut level = 1;
    let mut strain = 0;

    while level <= 7 {
        while strain <= 4 {
            let bid = Bid {
                level,
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
                // SAFETY: Maximum `level` is within `u8`
                #[allow(clippy::cast_possible_truncation)]
                level: level as u8,
                strain: super::Strain::ASC[strain],
            })
        }
        _ => panic!("Invalid call ID!"),
    }
}

const _: () = {
    let mut id = 0;

    while id < CALL_VARIANTS {
        let call = decode_call(id);
        assert!(encode_call(call) == id);
        id += 1;
    }
};

/// All calls in order of their encoding
pub const KEYS: [Call; CALL_VARIANTS] = {
    let mut calls = [Call::Pass; CALL_VARIANTS];
    let mut index = 0;

    while index < CALL_VARIANTS {
        calls[index] = decode_call(index);
        index += 1;
    }
    calls
};

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

    /// Map all values with a function
    pub fn map<U>(self, mut f: impl FnMut(Call, T) -> U) -> Array<U> {
        let mut result = [const { MaybeUninit::uninit() }; CALL_VARIANTS];

        for (index, value) in self.0.into_iter().enumerate() {
            result[index] = MaybeUninit::new(f(decode_call(index), value));
        }

        Array(unsafe { core::mem::transmute_copy(&result) })
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

impl<T> Index<Range<Call>> for Array<T> {
    type Output = [T];

    fn index(&self, range: Range<Call>) -> &Self::Output {
        let start = encode_call(range.start);
        let end = encode_call(range.end);
        &self.0[start..end]
    }
}

impl<T> IndexMut<Range<Call>> for Array<T> {
    fn index_mut(&mut self, range: Range<Call>) -> &mut Self::Output {
        let start = encode_call(range.start);
        let end = encode_call(range.end);
        &mut self.0[start..end]
    }
}

impl<T> Index<RangeFrom<Call>> for Array<T> {
    type Output = [T];

    fn index(&self, range: RangeFrom<Call>) -> &Self::Output {
        let start = encode_call(range.start);
        &self.0[start..]
    }
}

impl<T> IndexMut<RangeFrom<Call>> for Array<T> {
    fn index_mut(&mut self, range: RangeFrom<Call>) -> &mut Self::Output {
        let start = encode_call(range.start);
        &mut self.0[start..]
    }
}

impl<T> Index<RangeInclusive<Call>> for Array<T> {
    type Output = [T];

    fn index(&self, range: RangeInclusive<Call>) -> &Self::Output {
        let start = encode_call(*range.start());
        let end = encode_call(*range.end());
        &self.0[start..=end]
    }
}

impl<T> IndexMut<RangeInclusive<Call>> for Array<T> {
    fn index_mut(&mut self, range: RangeInclusive<Call>) -> &mut Self::Output {
        let start = encode_call(*range.start());
        let end = encode_call(*range.end());
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

    /// Convert to an array of odds
    ///
    /// The maximum value is set to one for numerical stability.
    #[must_use]
    pub fn to_odds(self) -> Array<f32> {
        let max = self.into_values().fold(f32::NEG_INFINITY, f32::max);
        Array(core::array::from_fn(|i| (self.0.0[i] - max).exp()))
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
