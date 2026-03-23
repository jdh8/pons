use super::{Bid, Call, Strain};
use core::iter::{Enumerate, FusedIterator};
use core::ops::{Index, IndexMut};

/// Number of possible calls
const CALL_VARIANTS: usize = 3 + 7 * 5;

#[inline]
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

#[inline]
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

#[test]
#[should_panic(expected = "Invalid call ID!")]
fn test_decode_call_invalid() {
    decode_call(CALL_VARIANTS);
}

/// [`Array`] maps each call to a value.
///
/// Like a mathematical function, every potentially valid call maps to a
/// corresponding value.  This type can be viewed as a 'dense' version of
/// [`super::Map`], which is more efficient but less flexible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Array<T>([T; CALL_VARIANTS]);

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

    /// Visit all key-value pairs in the table
    pub fn iter(&self) -> impl FusedIterator<Item = (Call, &T)> + DoubleEndedIterator {
        self.0
            .iter()
            .enumerate()
            .map(|(index, entry)| (decode_call(index), entry))
    }

    /// Visit all key-value pairs in the table with mutable access to the values
    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (Call, &mut T)> + DoubleEndedIterator {
        self.0
            .iter_mut()
            .enumerate()
            .map(|(index, entry)| (decode_call(index), entry))
    }
}

impl<T> Array<Option<T>> {
    /// New array with all `None` values
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

impl<T: Default> Default for Array<T> {
    fn default() -> Self {
        Self::from_fn(|_| T::default())
    }
}

impl<T> IntoIterator for Array<T> {
    type Item = (Call, T);
    type IntoIter = core::iter::Map<
        Enumerate<core::array::IntoIter<T, CALL_VARIANTS>>,
        fn((usize, T)) -> (Call, T),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.0
            .into_iter()
            .enumerate()
            .map(|(index, entry)| (decode_call(index), entry))
    }
}
