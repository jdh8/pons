use super::{Auction, Call, IllegalCall, Strategy, Trie};

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
        assert!(super::encode_call(call) == id);
        id += 1;
    }

    assert!(decode_call(37).is_none());
    assert!(decode_call(38).is_none());
    assert!(decode_call(39).is_none());
    assert!(decode_call(40).is_none());
};

#[derive(Debug, Clone, Copy)]
struct SuffixStackEntry<'a> {
    depth: usize,
    index: usize,
    node: &'a Trie,
}

fn collect_children(node: &Trie, depth: usize) -> impl Iterator<Item = SuffixStackEntry> {
    node.children
        .iter()
        .enumerate()
        .rev()
        .filter_map(move |(index, child)| {
            child.as_ref().map(|child| SuffixStackEntry {
                depth,
                index,
                node: child,
            })
        })
}

/// Suffix iterator for a given auction
///
/// This is the return type of [`Trie::suffix_iter`].
#[derive(Debug, Clone)]
pub struct SuffixIter<'a> {
    stack: Vec<SuffixStackEntry<'a>>,
    auction: Auction,
    separator: usize,
    value: Option<Strategy>,
}

impl<'a> SuffixIter<'a> {
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
            value: node.strategy,
            auction,
        }
    }
}

impl Iterator for SuffixIter<'_> {
    type Item = (Box<[Call]>, Result<Strategy, IllegalCall>);

    fn next(&mut self) -> Option<Self::Item> {
        while self.value.is_none() {
            let entry = self.stack.pop()?;
            self.stack
                .extend(collect_children(entry.node, entry.depth + 1));
            self.value = entry.node.strategy;
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
#[derive(Debug, Clone)]
pub struct CommonPrefixIter<'a> {
    trie: &'a Trie,
    query: Auction,
    depth: usize,
    value: Option<Strategy>,
}

impl<'a> CommonPrefixIter<'a> {
    /// Construct a common prefix iterator for a trie and an auction
    #[must_use]
    pub fn new(trie: &'a Trie, query: Auction) -> Self {
        Self {
            trie,
            query,
            depth: 0,
            value: trie.strategy,
        }
    }
}

impl Iterator for CommonPrefixIter<'_> {
    type Item = (Box<[Call]>, Strategy);

    fn next(&mut self) -> Option<Self::Item> {
        while self.value.is_none() {
            let &call = self.query.get(self.depth)?;
            self.trie = self.trie.children[super::encode_call(call)].as_deref()?;
            self.value = self.trie.strategy;
            self.depth += 1;
        }

        Some((self.query[..self.depth].into(), self.value.take()?))
    }
}
