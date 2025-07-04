/// Helper module for [`Trie`]
pub mod trie;

use core::ops::{Deref, Index};
use dds_bridge::contract::{Bid, Call, Penalty, Strain};
use dds_bridge::deal::Hand;
use std::panic::RefUnwindSafe;
use std::sync::Arc;
use thiserror::Error;
pub use trie::Trie;

bitflags::bitflags! {
    /// Vulnerability of sides
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Vulnerability: u8 {
        /// We are vulnerable
        const WE = 1;
        /// Opponents are vulnerable
        const THEY = 2;
    }
}

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

    #[inline]
    fn deref(&self) -> &[Call] {
        &self.0
    }
}

impl From<Auction> for Vec<Call> {
    #[inline]
    fn from(auction: Auction) -> Self {
        auction.0
    }
}

impl Auction {
    /// Construct an empty auction
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Check if the auction is terminated (by 3 consecutive passes following
    /// a call)
    #[must_use]
    #[inline]
    pub fn has_ended(&self) -> bool {
        self.len() >= 4 && self[self.len() - 3..] == [Call::Pass; 3]
    }

    /// Test doubling the last bid
    fn can_double(&self) -> Result<(), IllegalCall> {
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

    /// Test redoubling the last double (dry run)
    fn can_redouble(&self) -> Result<(), IllegalCall> {
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

    /// Test bidding a contract (dry run)
    fn can_bid(&self, bid: Bid) -> Result<(), IllegalCall> {
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

    /// Test adding a call to the auction
    fn can_push(&self, call: Call) -> Result<(), IllegalCall> {
        if self.has_ended() {
            return Err(IllegalCall::AfterFinalPass);
        }

        match call {
            Call::Pass => Ok(()),
            Call::Double => self.can_double(),
            Call::Redouble => self.can_redouble(),
            Call::Bid(bid) => self.can_bid(bid),
        }
    }

    /// Add a call to the auction with checks
    ///
    /// # Errors
    ///
    /// [`IllegalCall`] if the call is forbidden by [The Laws of Duplicate
    /// Bridge][laws].
    ///
    /// [laws]: http://www.worldbridge.org/wp-content/uploads/2017/03/2017LawsofDuplicateBridge-nohighlights.pdf
    pub fn try_push(&mut self, call: Call) -> Result<(), IllegalCall> {
        self.can_push(call)?;
        self.0.push(call);
        Ok(())
    }

    /// Force adding a call to the auction
    ///
    /// 1. If [`Call::Double`] is inadmissible, this method tries to
    ///    redouble the last double.
    /// 2. Force pushing the original `call` despite of an error.
    ///
    /// # Errors
    ///
    /// [`IllegalCall`] if the call is forbidden by [The Laws of Duplicate
    /// Bridge][laws] after trying redoubling with [`Call::Double`].
    ///
    /// [laws]: http://www.worldbridge.org/wp-content/uploads/2017/03/2017LawsofDuplicateBridge-nohighlights.pdf
    pub fn force_push(&mut self, mut call: Call) -> Result<(), IllegalCall> {
        if call == Call::Double && self.can_redouble().is_ok() {
            call = Call::Redouble;
        }

        let report = self.can_push(call);
        self.0.push(call);
        report
    }

    /// Try adding calls to the auction
    ///
    /// # Errors
    ///
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
    #[inline]
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

/// Frequency of a call (`self.0` / [`u8::MAX`])
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frequency(pub u8);

/// Evaluate [`Frequency`] of a call, given a [`Hand`] and predefined position
#[derive(Clone)]
pub struct Filter(Arc<dyn Fn(Hand) -> Frequency + Send + Sync + RefUnwindSafe>);

impl Filter {
    /// Construct a filter from a callback
    #[inline]
    #[must_use]
    pub fn new(f: impl Fn(Hand) -> Frequency + Send + Sync + RefUnwindSafe + 'static) -> Self {
        Self(Arc::new(f))
    }
}

impl Deref for Filter {
    type Target = dyn Fn(Hand) -> Frequency + Send + Sync + RefUnwindSafe;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

/// Trait marking a bidding system
///
/// This trait is merely a marker since its supertraits already cover its usage.
/// Indexing with [`Vulnerability`] results in a [`Trie`] that handles auctions.
///
/// ```
/// use dds_bridge::deal::Seat;
///
/// assert!(Seat::North as usize == 0);
/// ```
pub trait System: Index<Vulnerability, Output = Trie> {}

impl System for Trie {}
impl System for trie::Forest {}
