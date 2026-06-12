//! Slam machinery: Roman Keycard Blackwood 1430

use crate::bidding::Trie;
use contract_bridge::Suit;
use contract_bridge::auction::Call;

/// Install RKCB 1430 below an agreed trump suit
///
/// `our_calls` is the undisturbed sequence of our side's calls so far (the
/// same form [`uncontested`][super::uncontested] takes); the 4NT ask, its
/// answers, and the 5NT king ask are inserted below it.  Major-suit trumps
/// only — minor-suit keycard needs signoff space this table does not model.
#[allow(dead_code)] // wired up by the game-force, Jacoby, and splinter chunks
pub(super) fn install_rkcb(_book: &mut Trie, _our_calls: &[Call], _trump: Suit) {}
