use core::ops::Deref;
pub use dds_bridge::contract::*;
pub use dds_bridge::deal::{Hand, Holding, SmallSet};

/// A sequence of [`Call`]s
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Auction {
    /// The sequence of [`Call`]s
    calls: Vec<Call>,

    /// The proposed contract
    contract: Option<Contract>,

    /// The index of [`Self::contract`] in [`Self::calls`]
    ///
    /// If the contract is being (re)doubled, the index updates to the
    /// (re)double.
    index: usize,
}

/// View the auction as a slice of calls
impl Deref for Auction {
    type Target = [Call];

    fn deref(&self) -> &[Call] {
        &self.calls
    }
}

impl Auction {
    /// Construct an empty auction
    #[must_use]
    pub const fn new() -> Self {
        Self {
            calls: Vec::new(),
            contract: None,
            index: 0,
        }
    }

    /// Check if the auction is terminated (by 3 consecutive passes following
    /// a call)
    #[must_use]
    pub fn has_ended(&self) -> bool {
        self.len() >= self.index + 4
    }

    /// Add a call to the auction
    pub fn push(&mut self, call: Call) -> bool {
        if self.has_ended() {
            return false;
        }

        match call {
            Call::Pass => (),

            Call::Double => {
                let Some(contract) = self.contract else {
                    return false;
                };
                if contract.penalty != Penalty::None {
                    return false;
                }
                if (self.index ^ self.len()) & 1 == 0 {
                    return false;
                }

                self.contract = Some(Contract {
                    bid: contract.bid,
                    penalty: Penalty::Doubled,
                });
                self.index = self.len();
            }

            Call::Redouble => {
                let Some(contract) = self.contract else {
                    return false;
                };
                if contract.penalty != Penalty::Doubled {
                    return false;
                }
                if (self.index ^ self.len()) & 1 == 0 {
                    return false;
                }

                self.contract = Some(Contract {
                    bid: contract.bid,
                    penalty: Penalty::Redoubled,
                });
                self.index = self.len();
            }

            Call::Bid(bid) => {
                // Invalid bid
                if bid.level < 1 || bid.level > 7 {
                    return false;
                }

                // Insufficient bid
                if self.contract.is_some_and(|contract| bid <= contract.bid) {
                    return false;
                }

                self.contract = Some(Contract {
                    bid,
                    penalty: Penalty::None,
                });
                self.index = self.len();
            }
        }

        self.calls.push(call);
        true
    }

    /// Search the index of the declaring bid
    ///
    /// The first player of the declaring side who first bids the strain of
    /// the contract is the declarer.  This method locates the bid that makes
    /// the declarer.
    #[must_use]
    pub fn declarer(&self) -> Option<usize> {
        self.contract.and_then(|contract| {
            let strain = contract.bid.strain;
            let parity = self.index & 1 ^ usize::from(contract.penalty == Penalty::Doubled);

            self.iter()
                .skip(parity)
                .step_by(2)
                .position(|call| match call {
                    Call::Bid(bid) => bid.strain == strain,
                    _ => false,
                })
                .map(|position| position << 1 | parity)
        })
    }
}
