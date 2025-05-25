use super::{Auction, Call, Strategy, Trie};

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

#[derive(Debug, Clone)]
pub struct SuffixIter<'a> {
    stack: Vec<(usize, usize, &'a Trie)>,
    auction: Auction,
    separator: usize,
    value: Option<Strategy>,
}

impl<'a> SuffixIter<'a> {
    pub const fn empty() -> Self {
        Self {
            stack: Vec::new(),
            auction: Auction::new(),
            separator: 0,
            value: None,
        }
    }

    pub fn new(subtrie: Option<&'a Trie>, auction: &[Call]) -> Self {
        let Some(subtrie) = subtrie else {
            return Self::empty();
        };

        Self {
            stack: Vec::new(),
            auction: Auction(auction.to_vec()),
            separator: auction.len(),
            value: subtrie.strategy,
        }
    }
}
