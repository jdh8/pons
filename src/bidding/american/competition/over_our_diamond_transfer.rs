//! Competition over our 2NT diamond transfer
//!
//! Opener's replies after the opponents double or overcall our 2NT diamond transfer
//! are authored under `agreements.competition.competition_over_diamond_transfer`.
//! Its sub-knob `diamond_transfer_responder_captain` (default on since 2026-10-15)
//! removes opener's `3NT` rung over a high overcall and authors responder's
//! answer to opener's penalty double there and opener's pass of the `4♦` pull —
//! see [`diamond_overcalled_captain`], [`diamond_pulls_penalty`] and
//! [`diamond_pull_accepted`].

use super::*;

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
        .penalty()
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
        .penalty()
        .rule(Call::Pass, 20, hcp(0..))
}

/// Opener's reply over a high overcall with responder captain
/// (`diamond_transfer_responder_captain`): [`diamond_overcalled_high`] without
/// its `3NT` rung
///
/// The transfer is uncapped downwards, so opener cannot place `3NT`; the hand
/// that knows is responder, who bids on over the pass with values.  `X` = 4+ in
/// their suit (penalty), else Pass.
fn diamond_overcalled_captain(over: Suit) -> Rules {
    Rules::new()
        .rule(Call::Double, 60, len(over, 4..))
        .penalty()
        .rule(Call::Pass, 20, hcp(0..))
}

/// Responder's answer to opener's penalty double of their high overcall
/// (`1NT - 2NT (3♥) X -`), under `diamond_transfer_responder_captain`
///
/// Opener showed four of their suit opposite responder's known diamonds, so
/// defending is the default; the one pull is the shape that cannot defend —
/// six diamonds and at most one card in their suit — which plays `4♦`.  Over
/// their `3♦` cue that shape cannot exist, so the lane is pass-only.
/// Unauthored, this seat belongs to the floor, which pulled weak diamond hands
/// to `3NT` on every worst board of the 2026-10-15 re-measure.
fn diamond_pulls_penalty(over: Suit) -> Rules {
    let rules = if over == Suit::Diamonds {
        Rules::new()
    } else {
        Rules::new().rule(
            Bid::new(4, Strain::Diamonds),
            80,
            len(Suit::Diamonds, 6..) & len(over, ..2),
        )
    };
    rules.rule(Call::Pass, 20, hcp(0..))
}

/// Opener's answer to responder's `4♦` pull of the penalty double
/// (`1NT - 2NT (3♥) X - 4♦ -`), under `diamond_transfer_responder_captain`
///
/// The pull is a signoff by the captain: six diamonds and at most one card in
/// their suit, which opener's four there cannot improve.  Pass.  Unauthored,
/// this seat belonged to the floor, which bid `4♠` on all 44 such auctions of
/// the 2026-10-15 three-arm A/B; responder then passed it or ran to `5♦`.
fn diamond_pull_accepted() -> Rules {
    Rules::new().rule(Call::Pass, 20, hcp(0..))
}

/// Competition over our own `2NT` diamond transfer as a row package
/// (`agreements.competition.competition_over_diamond_transfer`, default on)
///
/// Opener's replies after they double `1NT - 2NT (X)` or overcall it.  Only
/// the PUPPET scheme plays `2NT` as the diamond transfer, so the package
/// no-ops under EUROPEAN.
pub(super) fn competition_over_diamond_transfer_package() -> Package {
    Package {
        name: "competition-over-diamond-transfer",
        gate: |agreements| {
            agreements.competition.competition_over_diamond_transfer
                && agreements.decision.reading.notrump_minors == PUPPET
        },
        entries: |agreements| {
            const TWO_NT: &str = "P* 1NT - 2NT";
            let captain = agreements.competition.diamond_transfer_responder_captain;
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
            // natural — or, with responder captain, `X`/Pass, responder's
            // answer to the `X` and opener's pass of the `4♦` pull.
            entries.extend(rows_of(
                Pattern::after(TWO_NT, "(3♣)"),
                diamond_overcalled_low(),
            ));
            for (over, suit) in [
                ("(3♦)", Suit::Diamonds),
                ("(3♥)", Suit::Hearts),
                ("(3♠)", Suit::Spades),
            ] {
                if captain {
                    entries.extend(rows_of(
                        Pattern::after(TWO_NT, over),
                        diamond_overcalled_captain(suit),
                    ));
                    entries.extend(rows_of(
                        Pattern::after(TWO_NT, &format!("{over} X -")),
                        diamond_pulls_penalty(suit),
                    ));
                    if suit != Suit::Diamonds {
                        entries.extend(rows_of(
                            Pattern::after(TWO_NT, &format!("{over} X - 4♦ -")),
                            diamond_pull_accepted(),
                        ));
                    }
                } else {
                    entries.extend(rows_of(
                        Pattern::after(TWO_NT, over),
                        diamond_overcalled_high(suit),
                    ));
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
