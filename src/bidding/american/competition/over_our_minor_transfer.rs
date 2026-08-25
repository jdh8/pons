//! Competition over our two-way 2♠ minor response
//!
//! Opener's replies after the opponents double or overcall our two-way 2♠ minor
//! response are authored under `agreements.competition.competition_over_minor_transfer`.

use super::*;

/// Opener's coded reply after the opponents double our two-way 2♠
/// (`1NT - 2♠ (X)`)
///
/// Their `X` is lead-directing spades, so opener answers the size-ask *and* shows
/// a spade stopper in one call: `2NT`/`3♣` keep their uncontested min/max meaning
/// and promise a stopper (responder then plays the rebased systems-on tree), while
/// `Pass`/`XX` deny a stopper for the minimum/maximum respectively (responder signs
/// off in clubs below).
fn minor_doubled_opener() -> Rules {
    Rules::new()
        // Maximum + spade stopper: the uncontested `3♣` max answer.
        .rule(
            Bid::new(3, Strain::Clubs),
            100,
            hcp(17..) & stopper_in(Suit::Spades),
        )
        // Minimum + spade stopper: the uncontested `2NT` min answer.
        .rule(Bid::new(2, Strain::Notrump), 90, stopper_in(Suit::Spades))
        // Maximum, no stopper: `XX`.
        .rule(Call::Redouble, 80, hcp(17..))
        // Minimum, no stopper: `Pass`.
        .rule(Call::Pass, 25, hcp(0..))
}

/// Responder's placement after opener denied a spade stopper over our doubled 2♠
/// (`1NT - 2♠ (X) - -` minimum, or `… XX -` maximum)
///
/// Opener has shown min/max but no stopper, so notrump is off; the six-card club
/// hand signs off in `3♣`.  Pass is the catch-all — the balanced-invite hand has no
/// safe spot and defends the doubled 2♠ (rare; the convention is opt-in).
//
// ponytail: the invite hand passing 2♠-doubled is the known soft spot; refine only
// if an A/B says the no-stopper branch leaks.
fn minor_no_stopper_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Clubs), 80, len(Suit::Clubs, 6..))
        .rule(Call::Pass, 10, hcp(0..))
}

/// Opener's reply after the opponents overcall our two-way 2♠ at `2NT` or `3♣` —
/// the bids that steal opener's size-ask steps (`1NT - 2♠ (2NT/3♣)`)
///
/// Keep the min/max + stopper signal alive in the room that remains: `3NT` =
/// maximum with a spade stopper (to play), `X` = maximum without one (penalty /
/// values), `Pass` = minimum.
fn minor_overcalled_high() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            hcp(17..) & stopper_in(Suit::Spades),
        )
        .rule(Call::Double, 70, hcp(17..))
        .penalty()
        .rule(Call::Pass, 20, hcp(0..))
}

/// Opener's systems-off reply after the opponents overcall our two-way 2♠ above
/// `3♣` (`1NT - 2♠ (3♦/3♥/3♠)`)
///
/// Their suit is too high to keep the size-ask, so opener falls back to natural
/// competition: `X` shows length in their suit (cards), else Pass and leave
/// responder captain.
fn minor_overcalled_low(over: Suit) -> Rules {
    Rules::new()
        .rule(Call::Double, 60, len(over, 4..))
        .penalty()
        .rule(Call::Pass, 20, hcp(0..))
}

/// Competition over our own two-way `2♠` minor response as a row package
/// (`agreements.competition.competition_over_minor_transfer`, default on)
///
/// Opener's replies after they double `1NT - 2♠ (X)` or overcall it.  Only
/// the PUPPET `2♠` (clubs *or* the balanced size-ask) has a min/max answer to
/// protect, so the package no-ops under the EUROPEAN pure-transfer scheme.
pub(super) fn competition_over_minor_transfer_package() -> Package {
    Package {
        name: "competition-over-minor-transfer",
        gate: |agreements| {
            agreements.competition.competition_over_minor_transfer
                && agreements.decision.reading.notrump_minors == PUPPET
        },
        entries: |_| {
            const TWO_SPADE: &str = "P* 1NT - 2♠";
            // A.1 — our 2♠ doubled.  Opener's coded min/max + stopper reply,
            // then the systems-on rebase off his 2NT/3♣ stopper-bid (the
            // `two_spade_over_min`/`max` machinery).
            let mut entries = rows_of(Pattern::after(TWO_SPADE, "(X)"), minor_doubled_opener());
            entries.push(systems_on_over_double(TWO_SPADE, "2NT"));
            // Opener denied a stopper (Pass = min, XX = max); responder signs
            // off in clubs.
            for deny in ["(X) - -", "(X) XX -"] {
                entries.extend(rows_of(
                    Pattern::after(TWO_SPADE, deny),
                    minor_no_stopper_rebid(),
                ));
            }

            // A.2 — our 2♠ overcalled.  `2NT`/`3♣` steal the size-ask steps, so
            // opener keeps the min/max + stopper signal; a higher overcall
            // (`3♦/3♥/3♠`) is systems-off.
            for (over, rules) in [
                ("(2NT)", minor_overcalled_high()),
                ("(3♣)", minor_overcalled_high()),
                ("(3♦)", minor_overcalled_low(Suit::Diamonds)),
                ("(3♥)", minor_overcalled_low(Suit::Hearts)),
                ("(3♠)", minor_overcalled_low(Suit::Spades)),
            ] {
                entries.extend(rows_of(Pattern::after(TWO_SPADE, over), rules));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
