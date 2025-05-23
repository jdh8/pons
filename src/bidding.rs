use core::ops::{Deref, Index, IndexMut};
pub use dds_bridge::contract::*;
pub use dds_bridge::deal::{Hand, Holding, SmallSet};
pub use dds_bridge::solver::Vulnerability;
use thiserror::Error;

/// Types of illegal calls
///
/// The laws mentioned in the variants are from [The Laws of Duplicate Bridge
/// 2017][laws].
///
/// [laws]: http://www.worldbridge.org/wp-content/uploads/2017/03/2017LawsofDuplicateBridge-nohighlights.pdf
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum IllegalCall {
    /// Law 27: insufficient bid
    #[error("Law 27: insufficient bid")]
    InsufficientBid {
        /// The offending bid
        this: Bid,
        /// The last bid in the auction
        last: Option<Bid>,
    },

    /// Law 36: inadmissible doubles and redoubles
    #[error("Law 36: inadmissible doubles and redoubles")]
    InadmissibleDouble(Penalty),

    /// Law 38: bid of more than seven
    #[error("Law 38: bid of more than seven")]
    BidOfMoreThanSeven(Bid),

    /// Law 39: call after the final pass
    #[error("Law 39: call after the final pass")]
    AfterFinalPass,
}

/// A sequence of [`Call`]s
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Auction(Vec<Call>);

/// View the auction as a slice of calls
impl Deref for Auction {
    type Target = [Call];

    fn deref(&self) -> &[Call] {
        &self.0
    }
}

impl From<Auction> for Vec<Call> {
    fn from(auction: Auction) -> Self {
        auction.0
    }
}

impl Auction {
    /// Construct an empty auction
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Check if the auction is terminated (by 3 consecutive passes following
    /// a call)
    #[must_use]
    pub fn has_ended(&self) -> bool {
        self.len() >= 4 && self[self.len() - 3..] == [Call::Pass; 3]
    }

    /// Try doubling the last bid (dry run)
    fn try_double(&self) -> Result<(), IllegalCall> {
        let admissible = self
            .iter()
            .rev()
            .copied()
            .enumerate()
            .find(|&(_, call)| call != Call::Pass)
            .is_some_and(|(index, call)| index & 1 == 0 && matches!(call, Call::Bid(_)));

        if !admissible {
            return Err(IllegalCall::InadmissibleDouble(Penalty::Doubled));
        }
        Ok(())
    }

    /// Try redoubling the last double (dry run)
    fn try_redouble(&self) -> Result<(), IllegalCall> {
        let admissible = self
            .iter()
            .rev()
            .copied()
            .enumerate()
            .find(|&(_, call)| call != Call::Pass)
            .is_some_and(|(index, call)| index & 1 == 0 && call == Call::Double);

        if !admissible {
            return Err(IllegalCall::InadmissibleDouble(Penalty::Redoubled));
        }
        Ok(())
    }

    /// Try bidding a contract (dry run)
    fn try_bid(&self, bid: Bid) -> Result<(), IllegalCall> {
        if bid.level < 1 {
            return Err(IllegalCall::InsufficientBid {
                this: bid,
                last: None,
            });
        }

        if bid.level > 7 {
            return Err(IllegalCall::BidOfMoreThanSeven(bid));
        }

        let last = self.iter().rev().find_map(|&call| match call {
            Call::Bid(bid) => Some(bid),
            _ => None,
        });

        if last >= Some(bid) {
            return Err(IllegalCall::InsufficientBid { this: bid, last });
        }
        Ok(())
    }

    /// Add a call to the auction with checks
    ///
    /// # Errors
    /// [`IllegalCall`] if the call is forbidden by [The Laws of Duplicate
    /// Bridge][laws].
    ///
    /// [laws]: http://www.worldbridge.org/wp-content/uploads/2017/03/2017LawsofDuplicateBridge-nohighlights.pdf
    pub fn try_push(&mut self, call: Call) -> Result<(), IllegalCall> {
        if self.has_ended() {
            return Err(IllegalCall::AfterFinalPass);
        }

        match call {
            Call::Pass => (),
            Call::Double => self.try_double()?,
            Call::Redouble => self.try_redouble()?,
            Call::Bid(bid) => self.try_bid(bid)?,
        }

        self.0.push(call);
        Ok(())
    }

    /// Try adding calls to the auction
    ///
    /// # Errors
    /// If any call is illegal, an [`IllegalCall`] is returned.  Calls already
    /// added to the auction are kept.  If you want to roll back the auction,
    /// [`truncate`][Self::truncate] it to the previous length.
    pub fn try_extend(&mut self, iter: impl IntoIterator<Item = Call>) -> Result<(), IllegalCall> {
        let iter = iter.into_iter();

        if let Some(size) = iter.size_hint().1 {
            self.0.reserve(size);
        }

        for call in iter {
            self.try_push(call)?;
        }
        Ok(())
    }

    /// Pop the last call from the auction
    pub fn pop(&mut self) -> Option<Call> {
        self.0.pop()
    }

    /// Truncate the auction to the first `len` calls
    ///
    /// If `len` is greater or equal to the current length, this has no effect.
    pub fn truncate(&mut self, len: usize) {
        self.0.truncate(len);
    }

    /// Search the index of the declaring bid
    ///
    /// The first player of the declaring side who first bids the strain of
    /// the contract is the declarer.  This method locates the bid that makes
    /// the declarer.
    #[must_use]
    pub fn declarer(&self) -> Option<usize> {
        let (parity, strain) =
            self.iter()
                .copied()
                .enumerate()
                .rev()
                .find_map(|(index, call)| match call {
                    Call::Bid(bid) => Some((index & 1, bid.strain)),
                    _ => None,
                })?;

        self.iter()
            .skip(parity)
            .step_by(2)
            .position(|call| match call {
                Call::Bid(bid) => bid.strain == strain,
                _ => false,
            })
            .map(|position| position << 1 | parity)
    }
}

const fn hash_call(call: Call) -> usize {
    match call {
        Call::Pass => 0,
        Call::Double | Call::Redouble => 1,
        Call::Bid(bid) => bid.level as usize * 5 + bid.strain as usize - 3,
    }
}

/// Trie as a vulnerability-agnostic bidding system
///
/// A trie stores strategy for each covered auction without vulnerability.
/// For example, `[P, 1♠]` as an index stands for the 2nd-seat opening of 1♠.
/// The strategy there describes how the 3rd seat should react.
#[derive(Debug, Clone)]
pub struct Trie {
    children: [Option<Box<Trie>>; 37],
    strategy: Option<fn(Hand) -> Call>,
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
            children: [const { None }; 37],
            strategy: None,
        }
    }

    /// Get the strategy for the exact auction
    #[must_use]
    pub fn get(&self, auction: &[Call]) -> Option<fn(Hand) -> Call> {
        let mut node = self;

        for &call in auction {
            node = node.children[hash_call(call)].as_deref()?;
        }
        node.strategy
    }

    /// Insert a strategy into the trie
    pub fn insert(
        &mut self,
        auction: &[Call],
        strategy: fn(Hand) -> Call,
    ) -> Option<fn(Hand) -> Call> {
        let mut node = self;

        for &call in auction {
            node = node.children[hash_call(call)].get_or_insert_with(Box::default);
        }
        node.strategy.replace(strategy)
    }
}

impl Index<Vulnerability> for Trie {
    type Output = Self;

    fn index(&self, _: Vulnerability) -> &Self {
        self
    }
}

/// A bidding system aware of vulnerability
#[derive(Debug, Clone)]
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

/// Trait marking a bidding system
///
/// This trait is merely a marker since its supertraits already cover its usage.
/// Indexing with [`Vulnerability`] results in a [`Trie`] that handles auctions.
///
/// A bidding system generally assumes that dealer is north.  This default is
/// also reflected in [`dds_bridge::deal::Seat`].  We can rotate vulnerability
/// with [`Vulnerability::swap`].
///
/// ```
/// use dds_bridge::deal::Seat;
///
/// assert!(Seat::North as usize == 0);
/// ```
pub trait System: Index<Vulnerability, Output = Trie> {}

impl System for Trie {}
impl System for Forest {}
