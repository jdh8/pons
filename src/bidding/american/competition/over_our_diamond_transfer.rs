//! Competition over our 2NT diamond transfer
//!
//! Opener's replies after the opponents double or overcall our 2NT diamond transfer
//! are authored under [`set_competition_over_diamond_transfer`].

use super::*;

thread_local! {
    /// Whether opener authors continuations after the opponents contest our 2NT
    /// diamond transfer (`1NT - 2NT (X)` or an overcall); **on by default**,
    /// with an off-switch for A/B measurement.  See
    /// [`set_competition_over_diamond_transfer`].
    static COMPETITION_OVER_DIAMOND_TRANSFER: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's replies after the opponents double or overcall our 2NT diamond
/// transfer (6+♦, or 5♦-4♣), for books built *after* this call (thread-local;
/// **on by default**).
///
/// Only the PUPPET scheme (the default) plays 2NT as the diamond transfer, so the
/// block no-ops under EUROPEAN (where 2NT is the balanced size-ask).  Their `(X)`
/// is lead-directing diamonds; the double frees `Pass` to be the catch-all
/// "no fit" call, which lets opener's `3♣` shed its uncontested
/// relay-denies-a-fit meaning and become **natural** (4+♣, finding responder's
/// 5♦-4♣ club fit): `3♦` = accept with a diamond fit (3+♦), `3♣` = no fit but
/// 4+♣, `XX` = maximum values without a fit (penalty-oriented), `Pass` = minimum
/// catch-all.  After a fit-showing `3♦`/`3♣` responder's rebids match the
/// uncontested tree (strip the `X` to a Pass); after `Pass`/`XX` (no fit)
/// responder always holds 5+♦ and signs off in `3♦`.  An overcall is handled
/// naturally: `3♣` leaves room to complete `3♦` with a fit (else `X` = penalty,
/// Pass = minimum); a higher overcall keeps `3NT` (max + stopper) / `X` (their
/// suit) / Pass.  **On by default** (off-switch `bba-gen
/// --no-ns-comp-over-diamond-transfer`): a paired A/B vs BBA over 1 000 000
/// `--filter-1nt` boards (410 fired, 0.04 %) measured a plain-DD **wash** (+0.24
/// IMPs/board it fires on, CI straddling 0) and a clear perfect-defense gain (+3.40
/// PD).  Unlike the 2♠ minor (which won on *both* scorers), the honest-DD signal is
/// a wash — but it never *loses* on plain DD, and the PD gain is real value the day
/// the opponents punish the floor's `X`-then-pull-to-`3NT` overreach, so it ships on.
pub fn set_competition_over_diamond_transfer(on: bool) {
    COMPETITION_OVER_DIAMOND_TRANSFER.with(|cell| cell.set(on));
}

/// Whether competition over our 2NT diamond transfer is currently authored
pub fn competition_over_diamond_transfer() -> bool {
    COMPETITION_OVER_DIAMOND_TRANSFER.with(Cell::get)
}

/// Opener's reply after the opponents double our 2NT diamond transfer
/// (`1NT - 2NT (X)`)
///
/// `Pass` now carries the "no diamond fit" message (the uncontested job of `3♣`),
/// so opener's `3♣` is freed to be natural 4+♣ (finding responder's 5♦-4♣ fit):
/// `3♦` = accept with 3+♦, `3♣` = no fit but 4+♣, `XX` = maximum values (no fit,
/// penalty-oriented), `Pass` = minimum catch-all.
fn diamond_doubled_opener() -> Rules {
    Rules::new()
        // Accept the transfer with a diamond fit — primary.
        .rule(Bid::new(3, Strain::Diamonds), 100, len(Suit::Diamonds, 3..))
        // No fit but real clubs: natural, lands responder's 5♦-4♣ in the club fit.
        .rule(
            Bid::new(3, Strain::Clubs),
            70,
            len(Suit::Diamonds, ..3) & len(Suit::Clubs, 4..),
        )
        // Maximum without a fit: redouble shows values (penalty-oriented).
        .rule(Call::Redouble, 60, hcp(17..))
        // Catch-all: minimum, no fit, no clubs.
        .rule(Call::Pass, 25, hcp(0..))
}

/// Responder's signoff after opener denied a diamond fit over our doubled 2NT
/// (`1NT - 2NT (X) - -` minimum, or `… XX -` maximum)
///
/// Responder always holds 5+♦ from the transfer, so pull to `3♦` rather than
/// languish in a doubled 2NT; Pass is a near-dead catch-all.
//
// ponytail: a strong responder bidding game over opener's XX is the rare soft
// spot left to the floor — refine only if an A/B says this branch leaks.
fn diamond_no_fit_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 80, len(Suit::Diamonds, 5..))
        .rule(Call::Pass, 10, hcp(0..))
}

/// Opener's reply after the opponents overcall our 2NT diamond transfer at `3♣`
/// (the one overcall that leaves the `3♦` completion legal)
fn diamond_overcalled_low() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 100, len(Suit::Diamonds, 3..))
        .rule(Call::Double, 60, len(Suit::Clubs, 4..))
        .rule(Call::Pass, 20, hcp(0..))
}

/// Opener's reply after the opponents overcall our 2NT diamond transfer above `3♣`
/// (`3♦` cue / `3♥` / `3♠` — the `3♦` completion is gone)
///
/// `3NT` = maximum with a stopper in their suit (to play), `X` = length in their
/// suit (penalty), else Pass and leave responder captain.
fn diamond_overcalled_high(over: Suit) -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            hcp(17..) & stopper_in(over),
        )
        .rule(Call::Double, 60, len(over, 4..))
        .rule(Call::Pass, 20, hcp(0..))
}

/// Competition over our own `2NT` diamond transfer as a row package
/// ([`set_competition_over_diamond_transfer`], default on)
///
/// Opener's replies after they double `1NT - 2NT (X)` or overcall it.  Only
/// the PUPPET scheme plays `2NT` as the diamond transfer, so the package
/// no-ops under EUROPEAN.
pub(super) fn competition_over_diamond_transfer_package() -> Package {
    Package {
        name: "competition-over-diamond-transfer",
        gate: |agreements| {
            agreements
                .build
                .competition
                .competition_over_diamond_transfer
                && agreements.decision.reading.notrump_minors() == PUPPET
        },
        entries: |_| {
            const TWO_NT: &str = "P* 1NT - 2NT";
            // Our 2NT doubled: opener's 3♦-fit / 3♣-clubs / XX-values / Pass
            // reply, then the systems-on rebase off his fit-showing bid.
            let mut entries = rows_of(Pattern::after(TWO_NT, "(X)"), diamond_doubled_opener());
            entries.push(systems_on_over_double(TWO_NT, "3♦"));
            // Opener denied a fit (Pass = min, XX = max values); responder
            // signs off in 3♦ (always 5+♦).
            for deny in ["(X) - -", "(X) XX -"] {
                entries.extend(rows_of(
                    Pattern::after(TWO_NT, deny),
                    diamond_no_fit_rebid(),
                ));
            }

            // Our 2NT overcalled.  `3♣` leaves the `3♦` completion legal; a
            // higher overcall (`3♦` cue / `3♥` / `3♠`) keeps `3NT`/`X`/Pass
            // natural.
            for (over, rules) in [
                ("(3♣)", diamond_overcalled_low()),
                ("(3♦)", diamond_overcalled_high(Suit::Diamonds)),
                ("(3♥)", diamond_overcalled_high(Suit::Hearts)),
                ("(3♠)", diamond_overcalled_high(Suit::Spades)),
            ] {
                entries.extend(rows_of(Pattern::after(TWO_NT, over), rules));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
