use super::Call;
use super::array::Array;
use core::iter::{FilterMap, FusedIterator};

/// Bidding table optionally maps calls to values.
///
/// This type resembles [`std::collections::HashMap`] but needs no dynamic
/// allocation because calls form a small finite set.  A mathematical relation
/// also models this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Map<T>(Array<Option<T>>);

impl<T> Map<T> {
    /// Create a new bidding table with all entries set to `None`
    #[must_use]
    pub const fn new() -> Self {
        Self(Array::new())
    }

    /// Get the value corresponding to a call
    #[must_use]
    pub const fn get(&self, call: Call) -> Option<&T> {
        self.0.get(call).as_ref()
    }

    /// Get the mutable entry for in-place manipulation
    #[must_use]
    pub const fn entry(&mut self, call: Call) -> &mut Option<T> {
        self.0.get_mut(call)
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
            .filter_map(|(call, entry)| entry.as_ref().map(|entry| (call, entry)))
    }

    /// Visit all key-value pairs in the table with mutable access to the values
    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (Call, &mut T)> + DoubleEndedIterator {
        self.0
            .iter_mut()
            .filter_map(|(call, entry)| entry.as_mut().map(|entry| (call, entry)))
    }
}

impl<T> Default for Map<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntoIterator for Map<T> {
    type Item = (Call, T);
    type IntoIter = FilterMap<
        <Array<Option<T>> as IntoIterator>::IntoIter,
        fn((Call, Option<T>)) -> Option<(Call, T)>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.0
            .into_iter()
            .filter_map(|(call, entry)| entry.map(|entry| (call, entry)))
    }
}

impl<T> From<Array<T>> for Map<T> {
    fn from(array: Array<T>) -> Self {
        Self(array.map(|_, value| Some(value)))
    }
}
