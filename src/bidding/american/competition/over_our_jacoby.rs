//! Competition over our Jacoby transfer
//!
//! Opener's replies after the opponents double or overcall our Jacoby transfer are
//! authored under `agreements.competition.competition_over_transfer`.

use super::*;

/// Opener's reply after the opponents double our Jacoby transfer
/// (`1NT - 2♦/2♥ (X)`)
///
/// The transfer is still a command, but the `(X)` buys opener a meaningful pass:
/// **complete** (bid `major`) with three-card support, **jump super-accept**
/// (`3-major`) with four and a maximum, **Pass** with a doubleton (declines —
/// responder re-asks below), or `XX` when the doubled transfer suit (`bid`) is
/// opener's own and it wants to defend.
fn transfer_doubled_opener(major: Suit, bid: Suit, agreements: &Agreements) -> Rules {
    let strain = Strain::from(major);
    let mut rules = Rules::new();
    if agreements.notrump.transfer_super_accept {
        rules = rules.rule(Bid::new(3, strain), 150, len(major, 4..) & hcp(17..));
    }
    rules
        .rule(Bid::new(2, strain), 100, len(major, 3..))
        .rule(Call::Redouble, 60, len(bid, 5..) & suit_hcp(bid, 5..))
        .rule(Call::Pass, 25, len(major, ..3))
}

/// Responder's re-ask after opener passed our doubled transfer
/// (`1NT - 2♦/2♥ (X) - -`)
///
/// Opener's pass declined the transfer; responder still holds the five-card
/// major, so `XX` insists opener complete (forcing — opener answers with
/// [`complete_transfer`], no Pass).  An owning Pass is the catch-all.
fn transfer_pass_reask(major: Suit) -> Rules {
    Rules::new()
        .rule(Call::Redouble, 100, len(major, 5..))
        .alert(TRANSFER_REDOUBLE)
        .rule(Call::Pass, 10, hcp(0..))
}

/// Opener's reply after the opponents overcall our Jacoby transfer
/// (`1NT - 2♦/2♥ (overcall)`)
///
/// Super-accept the `major` at the cheapest level above their `over_suit` with
/// four-card support; else `X` shows length in their suit (cards); else Pass.
/// Responder stays captain.
fn transfer_overcalled_opener(major: Suit, over_suit: Suit, over_level: u8) -> Rules {
    let strain = Strain::from(major);
    let lvl = if strain > Strain::from(over_suit) {
        over_level
    } else {
        over_level + 1
    };
    Rules::new()
        .rule(
            Bid::new(lvl, strain),
            100,
            min_level_is(lvl, strain) & len(major, 4..),
        )
        .rule(Call::Double, 60, len(over_suit, 4..))
        .rule(Call::Pass, 20, hcp(0..))
}

/// Competition over our own Jacoby transfers as a row package
/// (`agreements.competition.competition_over_transfer`, default off)
///
/// Opener's replies after they double `1NT - 2♦/2♥ (X)` or overcall it.
/// Keyed at the `1NT - 2♦` / `1NT - 2♥` nodes — distinct trie paths
/// from the Transfer-Lebensohl `1NT (2♦/2♥)` block, where theirs sits at
/// depth 1.
pub(super) fn competition_over_transfer_package() -> Package {
    Package {
        name: "competition-over-transfer",
        gate: |agreements| agreements.competition.competition_over_transfer,
        entries: |agreements| {
            let mut entries = Vec::new();
            for (resp, major) in [(Suit::Diamonds, Suit::Hearts), (Suit::Hearts, Suit::Spades)] {
                let key = format!("P* 1NT - 2{}", Strain::from(resp));
                let completion = format!("2{}", Strain::from(major));

                // Our transfer doubled: opener's reply, then the systems-on
                // rebase off his completion or super-accept.
                entries.extend(rows_of(
                    Pattern::after(&key, "(X)"),
                    transfer_doubled_opener(major, resp, agreements),
                ));
                entries.push(systems_on_over_double(&key, &completion));
                // Opener passed to decline; responder re-asks, and opener's
                // forced completion has no Pass rule so he cannot sit.
                entries.extend(rows_of(
                    Pattern::after(&key, "(X) - -"),
                    transfer_pass_reask(major),
                ));
                entries.extend(rows_of(
                    Pattern::after(&key, "(X) - - XX -"),
                    complete_transfer(major, agreements),
                ));

                // Our transfer overcalled.  Opener's natural reply.
                let overcalls: &[(Suit, u8)] = match resp {
                    Suit::Diamonds => &[(Suit::Spades, 2), (Suit::Clubs, 3), (Suit::Diamonds, 3)],
                    _ => &[(Suit::Clubs, 3), (Suit::Diamonds, 3)],
                };
                for &(over_suit, over_level) in overcalls {
                    entries.extend(rows_of(
                        Pattern::after(&key, &format!("({over_level}{})", Strain::from(over_suit))),
                        transfer_overcalled_opener(major, over_suit, over_level),
                    ));
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
