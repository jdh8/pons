//! Rubens transfers in the rich advance of partner's takeout double
//!
//! The jump-cue transfer and its continuations are gated by [`set_advance_rubens`].

use super::*;

thread_local! {
    /// Whether the **jump-cue Rubens transfer** layer sits on top of the rich
    /// advance — a jump-cue transfer to a 5+ unbid major; see
    /// [`set_advance_rubens`].  No effect unless [`RICH_ADVANCE_DOUBLE`] is on.
    static ADVANCE_RUBENS: Cell<bool> = const { Cell::new(false) };
}

/// Toggle the **jump-cue Rubens transfer** layer on top of the rich advance for
/// books built *after* this call (thread-local, read at book-construction time)
///
/// **Off by default**, and a no-op unless [`set_rich_advance_double`] is also on.
/// When on, the advancer's jump-cue (and, over `(1♠)`, a natural `3♥`) becomes a
/// **transfer to a 5+ unbid major** (invitational-or-better) — the doubler
/// completes and *declares*, right-siding the strong hand.  Right-siding is
/// invisible to double-dummy (the trick count is the same whoever declares), so
/// its value shows up under the single-dummy lead scorer, not the DD A/B; this
/// knob (`bba-gen --ns-advance-rubens`) exists to confirm no DD *regression* and
/// as an sd-lead re-measure candidate.  See `docs/ai-bidder/21gf-ledger.md`.
pub fn set_advance_rubens(on: bool) {
    ADVANCE_RUBENS.with(|cell| cell.set(on));
}

/// Whether the jump-cue Rubens transfer layer is currently authored
pub fn advance_rubens_enabled() -> bool {
    ADVANCE_RUBENS.with(Cell::get)
}

/// The advancer's jump-cue major transfers over a one-of-`theirs` opening:
/// `(transfer bid, the 5+ unbid major it shows)`.  A transfer is the rank
/// immediately below its target major, at the three level.  Over `(1♠)` the sole
/// unbid major (hearts, `3♥`) is below the jump-cue (`3♠`), so it is shown by the
/// natural invitational `3♥` jump in [`advance_double_rich`] instead and is not
/// returned here.
pub(super) fn advance_major_transfers(theirs: Strain) -> Vec<(Bid, Suit)> {
    if theirs == Strain::Spades {
        return Vec::new();
    }
    let mut out = Vec::new();
    for target in [Suit::Hearts, Suit::Spades] {
        if Strain::from(target) == theirs {
            continue;
        }
        let below = match target {
            Suit::Hearts => Suit::Diamonds,
            Suit::Spades => Suit::Hearts,
            _ => unreachable!("only hearts and spades are majors"),
        };
        out.push((Bid::new(3, Strain::from(below)), target));
    }
    out
}

/// Doubler's completion of the advancer's Rubens transfer
/// (`(1t) X - transfer { - | (X) } ?`, gated by [`set_advance_rubens`])
///
/// The transfer promised a 5+ `target` major; the doubler bids it (declaring —
/// the right-siding point), jumping to game (`4M`) with a maximum and support.
/// The completion is a finite catch-all so the artificial transfer is never
/// passed out.  Both bids are natural (`target`), so neither is alerted.
fn complete_advance_transfer(target: Suit) -> Rules {
    let strain = Strain::from(target);
    Rules::new()
        // Super-accept: maximum with support jumps to game.
        .rule(Bid::new(4, strain), 130, len(target, 4..) & points(15..))
        // Complete the transfer (always) — never pass the artificial call.
        .rule(Bid::new(3, strain), 100, hcp(0..))
}

/// Advancer's rebid after the doubler completed the transfer
/// (`(1t) X - transfer { - | (X) } 3M - ?`)
///
/// The transfer was invitational-or-better; opposite the doubler's minimum
/// completion a game-forcing advancer (12+) raises to game, an invitational one
/// (10–11) rests in the three-level partscore.
fn advance_transfer_rebid(target: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(target)), 100, hcp(12..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Rubens-transfer continuation rows for one opening suit
pub(super) fn advance_rubens_rows(
    base: &str,
    theirs: Strain,
    agreements: &Agreements,
) -> Vec<Entry> {
    let mut entries = Vec::new();
    if agreements.build.defense.advance_rubens_enabled {
        for (bid, target) in advance_major_transfers(theirs) {
            let completion = Bid::new(3, Strain::from(target));
            for rho in ["-", "(X)"] {
                let after_xfer = format!("{base} {bid} {rho}");
                entries.extend(rows_of(
                    Pattern::node(&after_xfer),
                    complete_advance_transfer(target),
                ));
                entries.extend(rows_of(
                    Pattern::node(&format!("{after_xfer} {completion} -")),
                    advance_transfer_rebid(target),
                ));
            }
        }
    }
    entries
}

#[cfg(test)]
mod tests;
