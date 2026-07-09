//! Shared harness for the `american_*` integration tests.
//!
//! Each test file pulls it in with `mod common;` then `use common::*;`, which
//! brings the three helpers (`call`, `stance`, `best_call`) and the auction
//! types the assertions name into scope.  A file that needs a variant — the
//! legality-filtered `best_call` for contested auctions, the knob-setting
//! `stance` — defines its own locally, which shadows the glob import.
//!
//! `dead_code`/`unused_imports` are allowed because every test crate compiles
//! this module in full yet uses only the subset it names.

#![allow(dead_code, unused_imports)]

pub use contract_bridge::auction::{Call, RelativeVulnerability};
pub use contract_bridge::{Bid, Hand, Strain};
pub use pons::american;
pub use pons::bidding::array::Logits;
pub use pons::bidding::{Family, Stance, System};

/// Shorthand for a bid call at `level`/`strain`.
pub const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The 2/1 pair bound against natural opponents.
pub fn stance() -> Stance {
    american().against(Family::NATURAL)
}

/// The single highest-logit call the system assigns the hand for the auction.
pub fn best_call(system: &impl System, auction: &[Call], hand: &str) -> Call {
    let hand: Hand = hand.parse().expect("valid test hand");
    let logits: Logits = system
        .classify(hand, RelativeVulnerability::NONE, auction)
        .expect("system covers this auction");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty")
}
