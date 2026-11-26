//! The strong jump shift `1♥ - 2♠`
//! (`response.heart_spade_strong_jump_shift`), through the whole stance.

mod common;
use common::*;

const P: Call = Call::Pass;

fn partnership(floor: Option<u8>) -> Partnership {
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.response.heart_spade_strong_jump_shift = floor;
    american(&agreements).bind()
}

/// North opens, the opponents pass throughout; the calls of North-South.
fn uncontested(system: &Partnership, north: &str, south: &str) -> Vec<Call> {
    let mut auction = Vec::new();
    for turn in 0.. {
        let hand = if turn % 2 == 0 { north } else { south };
        let ours = best_call(system, &auction, hand);
        auction.extend([ours, P]);
        if ours == P && turn > 0 {
            break;
        }
        assert!(turn < 12, "runaway auction: {auction:?}");
    }
    auction.into_iter().step_by(2).collect()
}

/// The anchor's worst board: BBA reaches `7♣`, the `1♠` response stopped in
/// `3NT`.
#[test]
fn the_club_fit_reaches_slam() {
    let (north, south) = ("73.AQ763.K.AQJ96", "AQJ85.K2.A2.K853");
    let calls = uncontested(&partnership(Some(16)), north, south);
    assert_eq!(
        calls[..5],
        [
            call(1, Strain::Hearts),
            call(2, Strain::Spades),
            call(3, Strain::Clubs),
            call(4, Strain::Clubs),
            call(4, Strain::Notrump),
        ]
    );
    let last = calls[calls.len() - 2];
    assert!(
        [call(6, Strain::Clubs), call(7, Strain::Clubs)].contains(&last),
        "{calls:?}"
    );
    // Off, the response is `1♠`.
    let off = uncontested(&partnership(None), north, south);
    assert_eq!(off[1], call(1, Strain::Spades));
}

/// Two minimums stop in game: the floor's seat over `3NT` reads the jump
/// shift as natural and passes.
#[test]
fn a_misfit_minimum_stops_in_three_notrump() {
    let calls = uncontested(
        &partnership(Some(16)),
        "73.AQ763.KJ4.Q96",
        "AQJ85.K2.A52.K85",
    );
    assert_eq!(
        calls,
        [
            call(1, Strain::Hearts),
            call(2, Strain::Spades),
            call(2, Strain::Notrump),
            call(3, Strain::Notrump),
            P,
        ]
    );
}
