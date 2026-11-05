use super::super::tests::{best_call_with, call};
use crate::bidding::Relative;
use crate::bidding::agreements::Agreements;
use crate::bidding::american::american;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Hand, Strain};
use rand::SeedableRng as _;

const P: Call = Call::Pass;
const X: Call = Call::Double;
const XX: Call = Call::Redouble;
const NT: Call = call(1, Strain::Notrump);
const S1: Call = call(1, Strain::Spades);
const D1: Call = call(1, Strain::Diamonds);
const C2: Call = call(2, Strain::Clubs);
const D2: Call = call(2, Strain::Diamonds);
const H2: Call = call(2, Strain::Hearts);
const S2: Call = call(2, Strain::Spades);

/// `(1♠) 1NT (X)` — the advancer to act
const OVER_SPADE: [Call; 3] = [S1, NT, X];
/// `(1♦) 1NT (X)` — the advancer to act
const OVER_DIAMOND: [Call; 3] = [D1, NT, X];

fn on() -> Agreements {
    let mut agreements = Agreements::default();
    agreements.defense.nt_overcall_doubled_runout = true;
    agreements
}

/// The call the knob-on book makes, asserting it came from the book
#[track_caller]
fn book(auction: &[Call], hand: &str) -> Call {
    let (call, floored) = best_call_with(&on(), auction, hand);
    assert!(!floored, "{hand} at {auction:?} must come from the book");
    call
}

#[test]
fn weak_advancer_runs_to_its_long_suit() {
    assert_eq!(book(&OVER_SPADE, "J6.T8643.Q62.T97"), H2);
    assert_eq!(book(&OVER_SPADE, "J64.T86432.Q6.97"), H2);
    assert_eq!(book(&OVER_SPADE, "7.J8.Q9642.J8632"), D2);
    assert_eq!(book(&OVER_SPADE, "7.J8.Q964.J86532"), C2);
    // Five of their suit is not a runout.
    assert_eq!(book(&OVER_SPADE, "J8643.T6.Q62.T97"), P);
    // Over a minor every other suit is open.
    assert_eq!(book(&OVER_DIAMOND, "J86432.T6.Q6.T97"), S2);
}

#[test]
fn weak_four_four_minor_bust_bids_the_longer_minor() {
    assert_eq!(book(&OVER_SPADE, "J86.T6.Q962.T974"), D2);
    assert_eq!(book(&OVER_SPADE, "J86.T6.Q96.JT974"), C2);
    // Over their diamonds the bust runs to clubs.
    assert_eq!(book(&OVER_DIAMOND, "J86.T6.Q962.T974"), C2);
    // A five-card major in the bust is the escape instead.
    assert_eq!(book(&OVER_SPADE, "6.T8643.Q962.T97"), H2);
}

#[test]
fn values_redouble_and_a_flat_bust_sits() {
    // The three hands the BBA-relabel net took to a phantom 3♦
    // (docs/reseed-gap-handoff.md §5).
    for hand in ["J.T86432.QJ62.AQ", "AK72.JT873.A7.T9", "K43.K6532.AJT4.2"] {
        assert_eq!(book(&OVER_SPADE, hand), XX, "{hand}");
    }
    // Seven HCP redoubles, six sits.
    assert_eq!(book(&OVER_SPADE, "K6.J843.QJ2.T974"), XX);
    assert_eq!(book(&OVER_SPADE, "K6.T843.QJ2.T974"), P);
}

#[test]
fn overcaller_sits_the_escape_and_balances_with_a_suit() {
    let run = [S1, NT, X, H2, P];
    assert_eq!(book(&run, "AQ5.K7.KQ65.AJ93"), P);
    let doubled = [S1, NT, X, H2, X];
    assert_eq!(book(&doubled, "AQ5.K7.KQ65.AJ93"), P);
    let balancing = [S1, NT, X, P, P];
    assert_eq!(book(&balancing, "AQ5.K7.KQ652.AJ9"), D2);
    assert_eq!(book(&balancing, "AQ5.K75.KQ6.AJ93"), P);
    // Their double of the balancing run is sat too.
    assert_eq!(book(&[S1, NT, X, P, P, D2, X], "J64.T86.J6.T9752"), P);
}

/// Every advancer call at `(1♠) 1NT (X)` comes from the book and its reading
/// admits the hand that made it — the leak `readings_admit_the_bidder`
/// exempted while the node was the floor's.
#[test]
fn readings_admit_the_advancer() {
    let partnership = american(&on()).bind();
    let mut rng = rand::rngs::StdRng::seed_from_u64(0x1A7D);
    let hands: Vec<Hand> = ["J.T86432.QJ62.AQ", "AK72.JT873.A7.T9", "K43.K6532.AJT4.2"]
        .iter()
        .map(|text| text.parse().expect("a hand"))
        .chain(crate::bidding::verify::random_hands(&mut rng).take(64))
        .collect();
    for hand in hands {
        let made = book(&OVER_SPADE, &hand.to_string());
        let read = [S1, NT, X, made, P];
        let inferences = partnership.infer(RelativeVulnerability::NONE, &read);
        assert!(
            inferences.admits(Relative::Partner, hand),
            "{hand} bid {made:?} at (1♠) 1NT (X) but the reading excludes it"
        );
    }
}
