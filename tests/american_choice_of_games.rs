//! Integration tests for the `1M - 3NT` choice-of-games response
//! (`ResponseKnobs::major_choice_of_games`, shipped default-on 2026-07-15)

mod common;
use common::*;
use pons::bidding::agreements::Agreements;

/// The opt-out 2/1 pair.
fn no_cog_stance() -> Stance {
    let mut agreements = Agreements::default();
    agreements.response.major_choice_of_games = false;
    american(&agreements).against()
}

/// A flat (4333) 13-count with three hearts offers 3NT over 1♥
#[test]
fn responder_offers_three_notrump() {
    let system = stance();
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Hearts), Call::Pass],
            "K32.K54.A964.K92",
        ),
        call(3, Strain::Notrump),
    );
}

/// Opted out, the same hand routes through the 2/1 in its four-card suit
#[test]
fn opt_out_routes_through_the_two_over_one() {
    let system = no_cog_stance();
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Hearts), Call::Pass],
            "K32.K54.A964.K92",
        ),
        call(2, Strain::Diamonds),
    );
}

/// A balanced minimum opener passes the choice-of-games 3NT — including
/// 5332, which the floor's ruffing-shortness correction would wrongly pull
#[test]
fn opener_passes_balanced() {
    let system = stance();
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Hearts),
                Call::Pass,
                call(3, Strain::Notrump),
                Call::Pass,
            ],
            "A5.AQJ54.Q54.T92",
        ),
        Call::Pass,
    );
}

/// An unbalanced opener corrects to the major game — the 5-3 fit ruffs
#[test]
fn opener_corrects_to_four_hearts_with_shape() {
    let system = stance();
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Hearts),
                Call::Pass,
                call(3, Strain::Notrump),
                Call::Pass,
            ],
            "5.AQJ542.KQ54.92",
        ),
        call(4, Strain::Hearts),
    );
}
