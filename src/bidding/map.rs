use super::Call;
use super::array::Array;
use core::iter::FilterMap;

/// Fixed-size map whose keys are [`Call`]s
///
/// This type resembles [`std::collections::HashMap`] but needs no dynamic
/// allocation because calls form a small finite set.
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
    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }

    /// Visit all key-value pairs in the table with mutable access to the values
    pub fn iter_mut(&mut self) -> <&mut Self as IntoIterator>::IntoIter {
        self.into_iter()
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

    /// Unwrap all values, panicking if any value is missing
    ///
    /// # Panics
    ///
    /// Panics if any call has no value.  The panic message includes the first
    /// call with no value.
    pub fn unwrap(self) -> Array<T> {
        self.unwrap_or_else(|call| panic!("missing value for {call:?}"))
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

impl<T> FromIterator<(Call, T)> for Map<T> {
    fn from_iter<Iter: IntoIterator<Item = (Call, T)>>(iter: Iter) -> Self {
        let mut map = Self::new();
        map.extend(iter);
        map
    }
}

impl<'a, T> IntoIterator for &'a Map<T> {
    type Item = (Call, &'a T);
    type IntoIter = FilterMap<
        <&'a Array<Option<T>> as IntoIterator>::IntoIter,
        fn((Call, &Option<T>)) -> Option<(Call, &T)>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.0
            .iter()
            .filter_map(|(call, entry)| entry.as_ref().map(|entry| (call, entry)))
    }
}

impl<'a, T> IntoIterator for &'a mut Map<T> {
    type Item = (Call, &'a mut T);
    type IntoIter = FilterMap<
        <&'a mut Array<Option<T>> as IntoIterator>::IntoIter,
        fn((Call, &mut Option<T>)) -> Option<(Call, &mut T)>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.0
            .iter_mut()
            .filter_map(|(call, entry)| entry.as_mut().map(|entry| (call, entry)))
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
