use super::{Call, Hand, Map, RelativeVulnerability};
use core::iter::FusedIterator;
use core::ops::{Index, IndexMut};
use std::sync::Arc;

/// Trait for a function that classifies a hand into logits for each call
pub trait Classifier: Send + Sync {
    /// Classify a hand with the given context into logits
    fn classify(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        prefixes: CommonPrefixes<'_, '_>,
    ) -> super::array::Logits;
}

impl<F> Classifier for F
where
    F: Fn(Hand, RelativeVulnerability) -> super::array::Logits + Send + Sync,
{
    fn classify(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        _: CommonPrefixes<'_, '_>,
    ) -> super::array::Logits {
        self(hand, vul)
    }
}

/// Decision trie as a vulnerability-agnostic bidding system
///
/// A trie stores a [`Classifier`] for each covered auction without
/// vulnerability.  For example, `[P, 1♠]` as an index stands for the 2nd-seat
/// opening of 1♠.
#[derive(Clone)]
pub struct Trie {
    children: Map<Box<Self>>,
    classify: Option<Arc<dyn Classifier>>,
}

impl Default for Trie {
    fn default() -> Self {
        Self::new()
    }
}

impl Trie {
    /// Construct an empty trie
    #[must_use]
    pub const fn new() -> Self {
        Self {
            children: Map::new(),
            classify: None,
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

    /// Get the [`Classifier`] for the exact auction
    #[must_use]
    pub fn get(&self, auction: &[Call]) -> Option<&dyn Classifier> {
        self.subtrie(auction)
            .and_then(|node| node.classify.as_deref())
    }

    /// Check if the query auction is a prefix in the trie
    #[must_use]
    pub fn is_prefix(&self, auction: &[Call]) -> bool {
        self.subtrie(auction).is_some()
    }

    /// Get the longest prefix of the auction that has a [`Classifier`]
    #[must_use]
    pub fn longest_prefix<'a>(&self, auction: &'a [Call]) -> Option<(&'a [Call], &dyn Classifier)> {
        let mut prefix = self.classify.as_deref().map(|f| (&[][..], f));
        let mut node = self;

        for (depth, &call) in auction.iter().enumerate() {
            node = match node.children.get(call) {
                Some(child) => child,
                None => break,
            };
            if let Some(f) = node.classify.as_deref() {
                prefix.replace((&auction[..=depth], f));
            }
        }
        prefix
    }

    /// Insert a [`Classifier`] into the trie
    pub fn insert(
        &mut self,
        auction: &[Call],
        f: impl Classifier + 'static,
    ) -> Option<Arc<dyn Classifier>> {
        let mut node = self;

        for &call in auction {
            node = node.children.entry(call).get_or_insert_with(Box::default);
        }
        node.classify.replace(Arc::new(f))
    }

    /// Depth first iteration over all nodes with a [`Classifier`]
    #[must_use]
    pub fn iter(&'_ self) -> Suffixes<'_> {
        self.suffixes(&[])
    }

    /// Depth first iteration over all suffixes to the auction
    #[must_use]
    pub fn suffixes(&self, auction: &[Call]) -> Suffixes<'_> {
        Suffixes::new(self, auction)
    }

    /// Iterate over common prefixes of the auction
    #[must_use]
    pub fn common_prefixes<'q>(&self, query: &'q [Call]) -> CommonPrefixes<'_, 'q> {
        CommonPrefixes::new(self, query)
    }
}

impl<'a> IntoIterator for &'a Trie {
    type Item = (Box<[Call]>, &'a dyn Classifier);
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
    auction: Vec<Call>,
    separator: usize,
    value: Option<&'a dyn Classifier>,
}

impl<'a> Suffixes<'a> {
    /// Construct an empty iterator
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            stack: Vec::new(),
            auction: Vec::new(),
            separator: 0,
            value: None,
        }
    }

    /// Construct a suffix iterator for a trie and an auction
    #[must_use]
    pub fn new(trie: &'a Trie, auction: &[Call]) -> Self {
        let Some(node) = trie.subtrie(auction) else {
            return Self::empty();
        };

        Self {
            stack: collect_children(node, 0).collect(),
            separator: auction.len(),
            value: node.classify.as_deref(),
            auction: auction.to_vec(),
        }
    }
}

impl<'a> Iterator for Suffixes<'a> {
    type Item = (Box<[Call]>, &'a dyn Classifier);

    fn next(&mut self) -> Option<Self::Item> {
        while self.value.is_none() {
            let entry = self.stack.pop()?;
            self.stack
                .extend(collect_children(entry.node, entry.depth + 1));
            self.value = entry.node.classify.as_deref();
            self.auction.truncate(self.separator + entry.depth);
            self.auction.push(entry.call);
        }

        Some((self.auction[self.separator..].into(), self.value.take()?))
    }
}

impl FusedIterator for Suffixes<'_> {}

/// Common prefix iterator for a given auction
#[derive(Clone)]
pub struct CommonPrefixes<'trie, 'q> {
    trie: &'trie Trie,
    query: &'q [Call],
    depth: usize,
    value: Option<&'trie dyn Classifier>,
}

impl<'trie, 'q> CommonPrefixes<'trie, 'q> {
    /// Construct a common prefix iterator for a trie and an auction
    #[must_use]
    pub fn new(trie: &'trie Trie, query: &'q [Call]) -> Self {
        Self {
            trie,
            query,
            depth: 0,
            value: trie.classify.as_deref(),
        }
    }
}

impl<'trie, 'q> Iterator for CommonPrefixes<'trie, 'q> {
    type Item = (&'q [Call], &'trie dyn Classifier);

    fn next(&mut self) -> Option<Self::Item> {
        while self.value.is_none() {
            let &call = self.query.get(self.depth)?;
            self.trie = self.trie.children.get(call)?;
            self.value = self.trie.classify.as_deref();
            self.depth += 1;
        }

        Some((&self.query[..self.depth], self.value.take()?))
    }
}

impl FusedIterator for CommonPrefixes<'_, '_> {}

/// A bidding system aware of vulnerability
#[derive(Clone)]
pub struct Forest([Trie; 4]);

impl Forest {
    /// Construct a forest with empty tries
    #[must_use]
    pub const fn new() -> Self {
        Self([Trie::new(), Trie::new(), Trie::new(), Trie::new()])
    }

    /// Construct a forest from a function mapping each vulnerability to a trie
    #[must_use]
    pub fn from_fn(mut f: impl FnMut(RelativeVulnerability) -> Trie) -> Self {
        Self([
            f(RelativeVulnerability::NONE),
            f(RelativeVulnerability::WE),
            f(RelativeVulnerability::THEY),
            f(RelativeVulnerability::ALL),
        ])
    }
}

impl Default for Forest {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<RelativeVulnerability> for Forest {
    type Output = Trie;

    fn index(&self, index: RelativeVulnerability) -> &Trie {
        &self.0[usize::from(index.bits())]
    }
}

impl IndexMut<RelativeVulnerability> for Forest {
    fn index_mut(&mut self, index: RelativeVulnerability) -> &mut Trie {
        &mut self.0[usize::from(index.bits())]
    }
}
