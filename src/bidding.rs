/// [`Call`]-indexed array
pub mod array;
/// [`Call`]-keyed hash map
pub mod map;
/// [`Trie`] as a bidding system
pub mod trie;

pub use array::Array;
pub use map::Map;
pub use trie::Trie;

use core::borrow::Borrow;
use core::ops::Deref;
use dds_bridge::{Bid, Hand, Penalty, Strain};
use thiserror::Error;

/// Any legal announcement in the bidding stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Call {
    /// A call indicating no wish to change the contract
    Pass,
    /// A call increasing penalties and bonuses for the contract
    Double,
    /// A call doubling the score to the previous double
    Redouble,
    /// A call proposing a contract
    Bid(Bid),
}

impl From<Bid> for Call {
    fn from(bid: Bid) -> Self {
        Self::Bid(bid)
    }
}

bitflags::bitflags! {
    /// Vulnerability of sides
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct RelativeVulnerability: u8 {
        /// We are vulnerable
        const WE = 1;
        /// Opponents are vulnerable
        const THEY = 2;
    }
}

impl RelativeVulnerability {
    /// No player is vulnerable
    pub const NONE: Self = Self::empty();
    /// All players are vulnerable
    pub const ALL: Self = Self::all();
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

impl AsRef<[Call]> for Auction {
    fn as_ref(&self) -> &[Call] {
        self
    }
}

impl Borrow<[Call]> for Auction {
    fn borrow(&self) -> &[Call] {
        self
    }
}

impl From<Auction> for Vec<Call> {
    fn from(auction: Auction) -> Self {
        auction.0
    }
}

impl IntoIterator for Auction {
    type Item = Call;
    type IntoIter = std::vec::IntoIter<Call>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Auction {
    type Item = &'a Call;
    type IntoIter = core::slice::Iter<'a, Call>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
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

    /// Add a call to the auction
    ///
    /// # Panics
    ///
    /// Panics if the call is illegal.
    pub fn push(&mut self, call: Call) {
        self.try_push(call).unwrap();
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
    pub fn pop(&mut self) -> Option<Call> {
        self.0.pop()
    }

    /// Truncate the auction to the first `len` calls
    ///
    /// If `len` is greater or equal to the current length, this has no effect.
    pub fn truncate(&mut self, len: usize) {
        self.0.truncate(len);
    }

    /// Find the position of the declaring bid in the call sequence
    ///
    /// The declarer is the first player on the declaring side to have bid the
    /// strain of the final contract.  This method returns the index of that
    /// bid in `self`, so `self[index]` is the declaring bid.
    ///
    /// The index also encodes the relative seat: `index % 2 == 0` is the
    /// dealer's side, and `index % 2 == 1` is the other side.  To obtain the
    /// absolute [`dds_bridge::Seat`], add the dealer's seat offset modulo 4.
    ///
    /// Returns [`None`] if the auction has no bid (passed out).
    ///
    /// # Examples
    ///
    /// ```
    /// use pons::bidding::{Auction, Call};
    /// use dds_bridge::{Bid, Level, Strain};
    ///
    /// // 1♥ by opener (index 1), raised to 4♥ — declarer bid 1♥ at index 1
    /// let mut auction = Auction::new();
    /// let one_heart = Call::Bid(Bid { level: Level::new(1), strain: Strain::Hearts });
    /// let four_hearts = Call::Bid(Bid { level: Level::new(4), strain: Strain::Hearts });
    /// auction.try_push(Call::Pass).unwrap();  // index 0 (dealer)
    /// auction.try_push(one_heart).unwrap();   // index 1 (declarer)
    /// auction.try_push(Call::Pass).unwrap();  // index 2
    /// auction.try_push(four_hearts).unwrap(); // index 3 (dummy)
    /// auction.try_push(Call::Pass).unwrap();
    /// auction.try_push(Call::Pass).unwrap();
    /// auction.try_push(Call::Pass).unwrap();
    /// assert_eq!(auction.declarer(), Some(1));
    /// ```
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

/// Trait for a bidding system
///
/// A bidding system tries classifying a hand into logits for each call given
/// vulnerability and the auction.
pub trait System {
    /// Classify a hand into logits for each call
    fn classify(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: Auction,
    ) -> Option<array::Logits>;
}

impl System for Trie {
    fn classify(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: Auction,
    ) -> Option<array::Logits> {
        self.get(&auction)
            .map(|f| f.classify(hand, vul, self.common_prefixes(auction)))
    }
}

impl System for trie::Forest {
    fn classify(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: Auction,
    ) -> Option<array::Logits> {
        self[vul].classify(hand, vul, auction)
    }
}
