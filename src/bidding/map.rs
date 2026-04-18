//! Sparse map keyed by [`Call`]
//!
//! [`Map<T>`] is a thin wrapper around [`Array<Option<T>>`][Array] that
//! exposes a [`HashMap`][std::collections::HashMap]-like surface over the
//! finite set of legal bridge calls.  It performs no heap allocation: a [`Map`]
//! is a fixed-size array of optional values plus iterator adapters that skip
//! absent entries.

use super::{Array, Call, array};
use core::iter::{FilterMap, Flatten};

/// Fixed-size map whose keys are [`Call`]s
///
/// This type resembles [`std::collections::HashMap`] but needs no dynamic
/// allocation because calls form a small finite set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Map<T>(Array<Option<T>>);

/// Iterator over keys
pub type Keys<'a, T> =
    FilterMap<array::Iter<'a, Option<T>>, fn((Call, &Option<T>)) -> Option<Call>>;

/// Iterator over values by reference
pub type Values<'a, T> = Flatten<array::Values<'a, Option<T>>>;

/// Iterator over values by mutable reference
pub type ValuesMut<'a, T> = Flatten<array::ValuesMut<'a, Option<T>>>;

/// Iterator over moving values
pub type IntoValues<T> = Flatten<array::IntoValues<Option<T>>>;

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

    /// Insert a value for a call, replacing the existing one if any
    pub const fn insert(&mut self, call: Call, value: T) -> Option<T> {
        self.entry(call).replace(value)
    }

    /// Visit all key-value pairs in the table
    pub fn iter(&self) -> Iter<'_, T> {
        self.into_iter()
    }

    /// Visit all key-value pairs in the table with mutable access to the values
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.into_iter()
    }

    /// Visit all keys
    pub fn keys(&self) -> Keys<'_, T> {
        self.0
            .iter()
            .filter_map(|(call, entry)| entry.is_some().then_some(call))
    }

    /// Visit all values
    pub fn values(&self) -> Values<'_, T> {
        self.0.values().flatten()
    }

    /// Visit all values with mutable access
    pub fn values_mut(&mut self) -> ValuesMut<'_, T> {
        self.0.values_mut().flatten()
    }

    /// Consume all values
    pub fn into_values(self) -> IntoValues<T> {
        self.0.into_values().flatten()
    }

    /// Map all values as is and fill in missing values with a function
    pub fn unwrap_or_else(self, mut f: impl FnMut(Call) -> T) -> Array<T> {
        self.0.map(|call, entry| entry.unwrap_or_else(|| f(call)))
    }

    /// Map all values as is and fill in missing values with [`Default`]
    pub fn unwrap_or_default(self) -> Array<T>
    where
        T: Default,
    {
        self.unwrap_or_else(|_| T::default())
    }
}

impl<T> Default for Map<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Extend<(Call, T)> for Map<T> {
    fn extend<Iter: IntoIterator<Item = (Call, T)>>(&mut self, iter: Iter) {
        for (call, value) in iter {
            self.insert(call, value);
        }
    }
}

impl<'a, T: Copy> Extend<(&'a Call, &'a T)> for Map<T> {
    fn extend<Iter: IntoIterator<Item = (&'a Call, &'a T)>>(&mut self, iter: Iter) {
        for (call, value) in iter {
            self.insert(*call, *value);
        }
    }
}

impl<T> From<Array<T>> for Map<T> {
    fn from(array: Array<T>) -> Self {
        Self(array.map(|_, value| Some(value)))
    }
}

impl<T> TryInto<Array<T>> for Map<T> {
    type Error = Call;

    fn try_into(self) -> Result<Array<T>, Self::Error> {
        self.0.try_map(|call, entry| entry.ok_or(call))
    }
}

impl<T> FromIterator<(Call, T)> for Map<T> {
    fn from_iter<Iter: IntoIterator<Item = (Call, T)>>(iter: Iter) -> Self {
        let mut map = Self::new();
        map.extend(iter);
        map
    }
}

/// Iterator by reference
pub type Iter<'a, T> =
    FilterMap<array::Iter<'a, Option<T>>, fn((Call, &Option<T>)) -> Option<(Call, &T)>>;

/// Iterator by mutable reference
pub type IterMut<'a, T> =
    FilterMap<array::IterMut<'a, Option<T>>, fn((Call, &mut Option<T>)) -> Option<(Call, &mut T)>>;

/// Iterator by value
pub type IntoIter<T> =
    FilterMap<array::IntoIter<Option<T>>, fn((Call, Option<T>)) -> Option<(Call, T)>>;

impl<'a, T> IntoIterator for &'a Map<T> {
    type Item = (Call, &'a T);
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0
            .iter()
            .filter_map(|(call, entry)| entry.as_ref().map(|entry| (call, entry)))
    }
}

impl<'a, T> IntoIterator for &'a mut Map<T> {
    type Item = (Call, &'a mut T);
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0
            .iter_mut()
            .filter_map(|(call, entry)| entry.as_mut().map(|entry| (call, entry)))
    }
}

impl<T> IntoIterator for Map<T> {
    type Item = (Call, T);
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0
            .into_iter()
            .filter_map(|(call, entry)| entry.map(|entry| (call, entry)))
    }
}
