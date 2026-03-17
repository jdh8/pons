use super::{Auction, Call, Filter, IllegalCall, Table, Vulnerability};
use core::ops::{Index, IndexMut};

/// Decision trie as a vulnerability-agnostic bidding system
///
/// A trie stores filter for each covered auction without vulnerability.
/// For example, `[P, 1♠]` as an index stands for the 2nd-seat opening of 1♠.
#[derive(Clone)]
pub struct Trie {
    children: Table<Box<Self>>,
    filter: Option<Filter>,
}

impl Default for Trie {
    fn default() -> Self {
        Self::new()
    }
}

impl Trie {
    /// Construct an empty trie
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self {
            children: Table::new(),
            filter: None,
        }
    }

    /// Get the sub-trie for the auction
    ///
    /// This method is not made public because auctions have context.
    #[must_use]
    fn subtrie(&self, auction: &[Call]) -> Option<&Self> {
        let mut node = self;

        for &call in auction {
            node = node.children.get(call)?;
        }
        Some(node)
    }

    /// Get the filter for the exact auction
    #[must_use]
    pub fn get(&self, auction: &[Call]) -> Option<&Filter> {
        self.subtrie(auction).and_then(|node| node.filter.as_ref())
    }

    /// Check if the query auction is a prefix in the trie
    #[must_use]
    pub fn is_prefix(&self, auction: &[Call]) -> bool {
        self.subtrie(auction).is_some()
    }

    /// Get the longest prefix of the auction that has a filter
    #[must_use]
    pub fn longest_prefix<'a>(&self, auction: &'a [Call]) -> Option<(&'a [Call], &Filter)> {
        let mut prefix = self.filter.as_ref().map(|f| (&[][..], f));
        let mut node = self;

        for (depth, &call) in auction.iter().enumerate() {
            node = match node.children.get(call) {
                Some(child) => child,
                None => break,
            };
            if let Some(f) = node.filter.as_ref() {
                prefix.replace((&auction[..=depth], f));
            }
        }
        prefix
    }

    /// Insert a filter into the trie
    pub fn insert(&mut self, auction: &[Call], f: Filter) -> Option<Filter> {
        let mut node = self;

        for &call in auction {
            node = node.children.entry(call).get_or_insert_with(Box::default);
        }
        node.filter.replace(f)
    }

    /// Depth first iteration over all filtered nodes
    #[must_use]
    pub fn iter(&'_ self) -> Suffixes<'_> {
        self.suffixes(Auction::new())
    }

    /// Depth first iteration over all suffixes of the auction
    #[must_use]
    pub fn suffixes(&'_ self, auction: Auction) -> Suffixes<'_> {
        Suffixes::new(self, auction)
    }

    /// Iterate over common prefixes of the auction
    #[must_use]
    #[inline]
    pub const fn common_prefixes(&'_ self, auction: Auction) -> CommonPrefixes<'_> {
        CommonPrefixes::new(self, auction)
    }
}

impl<'a> IntoIterator for &'a Trie {
    type Item = (Box<[Call]>, Result<&'a Filter, IllegalCall>);
    type IntoIter = Suffixes<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Copy)]
struct StackEntry<'a> {
    depth: usize,
    call: Call,
    node: &'a Trie,
}

fn collect_children(node: &'_ Trie, depth: usize) -> impl Iterator<Item = StackEntry<'_>> {
    node.children.iter().map(move |(call, child)| StackEntry {
        depth,
        call,
        node: child,
    })
}

/// Suffix iterator for a given auction
///
/// This is the return type of [`Trie::suffixes`].
#[derive(Clone)]
pub struct Suffixes<'a> {
    stack: Vec<StackEntry<'a>>,
    auction: Auction,
    separator: usize,
    value: Option<&'a Filter>,
}

impl<'a> Suffixes<'a> {
    /// Construct an empty iterator
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            stack: Vec::new(),
            auction: Auction::new(),
            separator: 0,
            value: None,
        }
    }

    /// Construct a suffix iterator for a trie and an auction
    #[must_use]
    pub fn new(trie: &'a Trie, auction: Auction) -> Self {
        let Some(node) = trie.subtrie(&auction) else {
            return Self::empty();
        };

        Self {
            stack: collect_children(node, 0).collect(),
            separator: auction.len(),
            value: node.filter.as_ref(),
            auction,
        }
    }
}

impl<'a> Iterator for Suffixes<'a> {
    type Item = (Box<[Call]>, Result<&'a Filter, IllegalCall>);

    fn next(&mut self) -> Option<Self::Item> {
        while self.value.is_none() {
            let entry = self.stack.pop()?;
            self.stack
                .extend(collect_children(entry.node, entry.depth + 1));
            self.value = entry.node.filter.as_ref();
            self.auction.truncate(self.separator + entry.depth);

            if let Err(e) = self.auction.force_push(entry.call) {
                return Some((self.auction[self.separator..].into(), Err(e)));
            }
        }

        Some((
            self.auction[self.separator..].into(),
            Ok(self.value.take()?),
        ))
    }
}

/// Common prefix iterator for a given auction
#[derive(Clone)]
pub struct CommonPrefixes<'a> {
    trie: &'a Trie,
    query: Auction,
    depth: usize,
    value: Option<&'a Filter>,
}

impl<'a> CommonPrefixes<'a> {
    /// Construct a common prefix iterator for a trie and an auction
    #[must_use]
    #[inline]
    pub const fn new(trie: &'a Trie, query: Auction) -> Self {
        Self {
            trie,
            query,
            depth: 0,
            value: trie.filter.as_ref(),
        }
    }
}

impl<'a> Iterator for CommonPrefixes<'a> {
    type Item = (Box<[Call]>, &'a Filter);

    fn next(&mut self) -> Option<Self::Item> {
        while self.value.is_none() {
            let &call = self.query.get(self.depth)?;
            self.trie = self.trie.children.get(call)?;
            self.value = self.trie.filter.as_ref();
            self.depth += 1;
        }

        Some((self.query[..self.depth].into(), self.value.take()?))
    }
}

impl Index<Vulnerability> for Trie {
    type Output = Self;

    fn index(&self, _: Vulnerability) -> &Self {
        self
    }
}

/// A bidding system aware of vulnerability
#[derive(Clone)]
pub struct Forest([Trie; 4]);

impl Index<Vulnerability> for Forest {
    type Output = Trie;

    fn index(&self, index: Vulnerability) -> &Trie {
        &self.0[usize::from(index.bits())]
    }
}

impl IndexMut<Vulnerability> for Forest {
    fn index_mut(&mut self, index: Vulnerability) -> &mut Trie {
        &mut self.0[usize::from(index.bits())]
    }
}
