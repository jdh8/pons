use super::super::notrump::flat_4333;
use crate::bidding::constraint::{balanced, hcp, len, points, support};
use crate::bidding::rows::{Package, expand};
use crate::bidding::{Alert, Rules};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};
use std::cell::Cell;

std::thread_local! {
    /// Whether `1M - 3NT` is authored as a **choice of games**: 3-4 card
    /// support, exactly (4333), 12-15 HCP — responder offers 3NT, opener
    /// passes balanced and corrects to `4M` with shape.  Default `true` —
    /// **shipped default-on 2026-07-15** (isolated: plain +0.0006/+0.0011
    /// NV/vul, PD +0.0005/+0.0010, all CIs clear; perfectly additive atop
    /// the 2/1 fit-split, full-package numbers on that knob).
    static MAJOR_CHOICE_OF_GAMES: Cell<bool> = const { Cell::new(true) };
}

/// Author the `1M - 3NT` choice-of-games response for books built after this
/// call (default `true`; off-switch `--no-ns-major-choice-of-games` in
/// `bba-gen`)
///
/// On: `3NT` over `1♥`/`1♠` shows 3-4 card support, exactly (4333) and 12-15
/// HCP (over `1♥` it also denies four spades — that hand bids `1♠` first).
/// Opener passes with a balanced hand and corrects to `4M` with shape; the
/// alerted reading pins responder's three-card support so later floor
/// decisions know the fit.  Off: the flat hand routes through its lone
/// four-card suit as a 2/1 (or Jacoby 2NT / limit raise with four trumps).
pub fn set_major_choice_of_games(on: bool) {
    MAJOR_CHOICE_OF_GAMES.with(|cell| cell.set(on));
}

/// Whether the choice-of-games 3NT is currently authored
fn major_choice_of_games() -> bool {
    MAJOR_CHOICE_OF_GAMES.with(Cell::get)
}

/// Choice of games — `1M - 3NT` with 3-4 card support, (4333), 12-15 HCP
const CHOICE_OF_GAMES: Alert = Alert("choice-of-games-3nt");

pub(super) fn with_choice_of_games(rules: Rules, major: Suit) -> Rules {
    let mut rules = rules;
    // Choice-of-games 3NT (`set_major_choice_of_games`): exactly (4333) with
    // 3-4 card support, 12-15 HCP — offer 3NT and let opener choose (the
    // curse of (4333): the flat hand often plays better in notrump).  On 4333
    // `points` reads raw HCP under the floored scale, so the band is HCP.
    // Weight 3.2 outranks Jacoby 2NT (3.0) and the limit raise (2.0) so flat
    // four-trump hands prefer it; over 1♥ the spade exclusion is load-bearing
    // — without it 3.2 would steal 4=3=3=3 from the 1♠ response (1.7).
    if major_choice_of_games() {
        let cog = support(3..=4) & flat_4333() & points(12..=15);
        rules = if major == Suit::Hearts {
            rules.rule(
                Bid::new(3, Strain::Notrump),
                320,
                cog & len(Suit::Spades, ..4),
            )
        } else {
            rules.rule(Bid::new(3, Strain::Notrump), 320, cog)
        }
        .alert(CHOICE_OF_GAMES);
    }
    rules
}

/// Opener's choice after responder's choice-of-games `3NT`
///
/// Correct to `4M` with an unbalanced hand (the alerted reading pins 3+
/// support, so the 5-3 fit is known); pass balanced — including 5332, which
/// the floor's ruffing-shortness correction would wrongly pull.
fn opener_after_choice_of_games(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 100, !balanced())
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's continuations after responder offers a choice of games
pub(crate) fn choice_of_games_continuations() -> Package {
    Package {
        name: "major-choice-of-games-continuations",
        gate: major_choice_of_games,
        entries: || {
            expand(
                "P* 1M - 3NT -",
                |_| true,
                |b| opener_after_choice_of_games(b.suit('M')),
            )
        },
    }
}
