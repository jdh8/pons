//! Continuations after the strong raises
//!
//! This module is the **index**: the shared vocabulary and `register`.  Each
//! raise agreement lives in its own submodule.
//!
//! | Module | Agreement | Knob |
//! | --- | --- | --- |
//! | [`jacoby`] | opener's descriptive rebid after `1M - 2NT`, and responder's slam try | always on |
//! | [`game_try`] | long-suit and general game tries after `1M - 2M` | [`set_major_game_tries`] |
//! | [`limit_raise`] | opener's acceptance ladder after `1M - 3M` | [`set_limit_raise_acceptance`] |
//!
//! Both knobbed agreements ship default-on, measured on a silenced-opponent
//! A/B (200k boards/vul, plain-DD + perfect-defense both winning):
//! +0.042/+0.065 IMPs/board NV/vul for the game tries, and +0.002/+0.002 for
//! limit-raise acceptance — the whole of that win being the keycard ask at
//! +4.4/+5.2 IMPs/divergent.

use super::{call, slam};
use crate::bidding::agreements::Agreements;
use crate::bidding::constraint::{fifths, hcp, len, support_points, top_honors};
use crate::bidding::rows::{Package, Pattern, compile_into, rows_of};
use crate::bidding::{Alert, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};
use std::cell::Cell;

mod game_try;
mod jacoby;
mod limit_raise;

pub use game_try::set_major_game_tries;
pub use limit_raise::set_limit_raise_acceptance;

// The packages, re-exported so `american::tests::row_package_invariants` and
// `register` below name them at one path.
pub(super) use game_try::major_game_try_continuations;
// The two raise cells, reached by `responses::capture` — the raise auctions
// are responses to our opening, so they share `Build::response`.
pub(crate) use game_try::major_game_tries;
pub(super) use jacoby::jacoby_continuations;
pub(super) use limit_raise::limit_raise_acceptance_continuations;

/// Register all strong-raise continuations into the constructive book
pub(super) fn register(book: &mut Trie, agreements: &Agreements) {
    compile_into(
        book,
        agreements,
        &[
            jacoby_continuations(),
            major_game_try_continuations(),
            limit_raise_acceptance_continuations(),
        ],
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
pub use limit_raise::limit_raise_acceptance;
