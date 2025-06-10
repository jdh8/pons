use super::{Auction, Bid, Call, Filter, IllegalCall, Strain, Vulnerability};
use core::ops::{Index, IndexMut};

const fn encode_call(call: Call) -> usize {
    match call {
        Call::Pass => 0,
        Call::Double | Call::Redouble => 1,
        Call::Bid(bid) => bid.level as usize * 5 + bid.strain as usize - 3,
    }
}

const _: () = {
    let mut calls = [Call::Pass; 37];
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
    assert!(encode_call(Call::Redouble) == 1);

    let mut index = 2;

    while index < 37 {
        assert!(matches!(calls[index], Call::Bid(_)));
        index += 1;
    }
};

/// Decision trie as a vulnerability-agnostic bidding system
///
/// A trie stores filter for each covered auction without vulnerability.
/// For example, `[P, 1♠]` as an index stands for the 2nd-seat opening of 1♠.
#[derive(Clone)]
pub struct Trie {
    children: [Option<Box<Trie>>; 37],
    filter: Option<Filter>,
}

impl Default for Trie {
    #[inline]
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
            children: [const { None }; 37],
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
            node = node.children[encode_call(call)].as_deref()?;
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
            node = match node.children[encode_call(call)].as_deref() {
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
            node = node.children[encode_call(call)].get_or_insert_with(Box::default);
        }
        node.filter.replace(f)
    }

    /// Depth first iteration over all filtered nodes
    #[must_use]
    pub fn iter(&self) -> Suffixes {
        self.suffixes(Auction::new())
    }

    /// Depth first iteration over all suffixes of the auction
    #[must_use]
    pub fn suffixes(&self, auction: Auction) -> Suffixes {
        Suffixes::new(self, auction)
    }

    /// Iterate over common prefixes of the auction
    #[must_use]
    #[inline]
    pub const fn common_prefixes(&self, auction: Auction) -> CommonPrefixes {
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

const fn decode_call(index: usize) -> Option<Call> {
    match index {
        0 => Some(Call::Pass),
        1 => Some(Call::Double),
        2..=36 => {
            let code = index + 3;
            let (level, strain) = (code / 5, code % 5);

            Some(Call::Bid(super::Bid {
                // SAFETY: Maximum `level` is (36 + 3) / 5 which is within `u8`
                #[allow(clippy::cast_possible_truncation)]
                level: level as u8,
                strain: super::Strain::ASC[strain],
            }))
        }
        _ => None,
    }
}

const _: () = {
    let mut id = 0;

    while id < 37 {
        let call = decode_call(id).expect("Invalid call ID!");
        assert!(encode_call(call) == id);
        id += 1;
    }

    assert!(decode_call(37).is_none());
    assert!(decode_call(38).is_none());
    assert!(decode_call(39).is_none());
    assert!(decode_call(40).is_none());
};

#[derive(Clone, Copy)]
struct StackEntry<'a> {
    depth: usize,
    index: usize,
    node: &'a Trie,
}

fn collect_children(node: &Trie, depth: usize) -> impl Iterator<Item = StackEntry> {
    node.children
        .iter()
        .enumerate()
        .rev()
        .filter_map(move |(index, child)| {
            child.as_ref().map(|child| StackEntry {
                depth,
                index,
                node: child,
            })
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

            let call = decode_call(entry.index).expect("Invalid call index!");
            if let Err(e) = self.auction.force_push(call) {
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
            self.trie = self.trie.children[encode_call(call)].as_deref()?;
            self.value = self.trie.filter.as_ref();
            self.depth += 1;
        }

        Some((self.query[..self.depth].into(), self.value.take()?))
    }
}

impl Index<Vulnerability> for Trie {
    type Output = Self;

    #[inline]
    fn index(&self, _: Vulnerability) -> &Self {
        self
    }
}

/// A bidding system aware of vulnerability
#[derive(Clone)]
pub struct Forest([Trie; 4]);

impl Index<Vulnerability> for Forest {
    type Output = Trie;

    #[inline]
    fn index(&self, index: Vulnerability) -> &Trie {
        &self.0[usize::from(index.bits())]
    }
}

impl IndexMut<Vulnerability> for Forest {
    #[inline]
    fn index_mut(&mut self, index: Vulnerability) -> &mut Trie {
        &mut self.0[usize::from(index.bits())]
    }
}
