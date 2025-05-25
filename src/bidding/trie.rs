use super::{Auction, Call, IllegalCall, Penalty, Strategy, Trie};

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

/// [`Auction::try_push`] but coerces [`Call::Double`] to [`Call::Redouble`]
///
/// # Errors
/// When coercing double to redouble cannot fix the auction.
fn force_push(auction: &mut Auction, call: Call) {
    match auction.try_push(call) {
        Ok(()) => {}
        Err(IllegalCall::InadmissibleDouble(Penalty::Doubled)) => auction
            .try_push(Call::Redouble)
            .expect("Redouble should be admissible"),
        _ => panic!("Cannot fix the auction with a redouble"),
    }
}

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
    pub fn new(trie: &'a Trie, auction: &[Call]) -> Self {
        let Some(node) = trie.subtrie(auction) else {
            return Self::empty();
        };

        Self {
            stack: collect_children(node, 0).collect(),
            auction: Auction(auction.to_vec()),
            separator: auction.len(),
            value: node.strategy,
        }
    }
}

impl Iterator for SuffixIter<'_> {
    type Item = (Box<[Call]>, Strategy);

    fn next(&mut self) -> Option<Self::Item> {
        while self.value.is_none() {
            let entry = self.stack.pop()?;
            self.stack
                .extend(collect_children(entry.node, entry.depth + 1));
            self.value = entry.node.strategy;
            self.auction.0.drain(self.separator + entry.depth..);

            let call = decode_call(entry.index).expect("Invalid call index!");
            force_push(&mut self.auction, call);
        }

        let value = self.value.take().expect("The loop above ensures a value");
        Some((self.auction[self.separator..].into(), value))
    }
}
