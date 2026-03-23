use super::{Bid, Call, Strain};
use core::iter::{Enumerate, FilterMap, FusedIterator};

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

/// Bidding table is a map from calls to custom data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Table<T>([Option<T>; CALL_VARIANTS]);

impl<T> Table<T> {
    /// Create a new bidding table with all entries set to `None`
    #[must_use]
    pub const fn new() -> Self {
        Self([const { None }; CALL_VARIANTS])
    }

    /// Get the value corresponding to a call
    #[must_use]
    pub const fn get(&self, call: Call) -> Option<&T> {
        self.0[encode_call(call)].as_ref()
    }

    /// Get the mutable entry for in-place manipulation
    #[must_use]
    pub const fn entry(&mut self, call: Call) -> &mut Option<T> {
        &mut self.0[encode_call(call)]
    }

    /// Get the mutable reference to the value
    #[must_use]
    pub const fn get_mut(&mut self, call: Call) -> Option<&mut T> {
        self.entry(call).as_mut()
    }

    /// Insert a value for a call, replacing the existing one if any
    pub const fn insert(&mut self, call: Call, value: T) -> Option<T> {
        self.entry(call).replace(value)
    }

    /// Visit all key-value pairs in the table
    pub fn iter(&self) -> impl FusedIterator<Item = (Call, &T)> + DoubleEndedIterator {
        self.0
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| entry.as_ref().map(|entry| (decode_call(index), entry)))
    }

    /// Visit all key-value pairs in the table with mutable access to the values
    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (Call, &mut T)> + DoubleEndedIterator {
        self.0
            .iter_mut()
            .enumerate()
            .filter_map(|(index, entry)| entry.as_mut().map(|entry| (decode_call(index), entry)))
    }
}

impl<T> Default for Table<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntoIterator for Table<T> {
    type Item = (Call, T);
    type IntoIter = FilterMap<
        Enumerate<core::array::IntoIter<Option<T>, CALL_VARIANTS>>,
        fn((usize, Option<T>)) -> Option<(Call, T)>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.0
            .into_iter()
            .enumerate()
            .filter_map(|(index, entry)| entry.map(|entry| (decode_call(index), entry)))
    }
}
