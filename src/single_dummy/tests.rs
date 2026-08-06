use super::*;
use contract_bridge::{Bid, Level, Strain};
use rand::SeedableRng;
use rand::rngs::StdRng;

const fn four_spades() -> Bid {
    Bid {
        level: Level::new(4),
        strain: Strain::Spades,
    }
}

/// North + South hold every spade, ace, king, and top honor: no defender can
/// ever win a trick, so North takes all thirteen in spades in *every* layout.
/// The spade game is therefore a double-dummy lock and the defender who holds
/// no trump can never make it.
fn unbeatable_spade_fit() -> (Hand, Hand) {
    // North: all top spades + AK of every side suit.
    let north: Hand = "AKQJT98.AK.AK.AK".parse().expect("valid test hand");
    // South: the rest of the spades + QJ(T) of the side suits.
    let south: Hand = "765432.QJT.QJ.QJ".parse().expect("valid test hand");
    (north, south)
}

/// The lock makes on every deal: `make_probability` is exactly 1 and the mean
/// trick count is a full thirteen, regardless of how the defenders' cards lie.
#[test]
fn unbeatable_fit_always_makes() {
    let (north, south) = unbeatable_spade_fit();
    let mut rng = StdRng::seed_from_u64(1);
    let hist = single_dummy(north, south, Seat::North, &mut rng, 16);

    assert_eq!(hist.expected_tricks(Seat::North, Strain::Spades), 13.0);
    assert_eq!(hist.make_probability(Seat::North, four_spades()), 1.0);
    // A defender holding no trump can never bring home a spade game.
    assert_eq!(hist.make_probability(Seat::East, four_spades()), 0.0);
}

/// Same seed and inputs reproduce the same histogram exactly (the solver never
/// samples its own RNG).
#[test]
fn deterministic_given_a_seed() {
    let (north, south) = unbeatable_spade_fit();
    let mut rng_a = StdRng::seed_from_u64(7);
    let a = single_dummy(north, south, Seat::North, &mut rng_a, 12);
    let mut rng_b = StdRng::seed_from_u64(7);
    let b = single_dummy(north, south, Seat::North, &mut rng_b, 12);
    assert_eq!(a, b);
}

/// With no layouts the histogram is empty and the readers report `NaN`.
#[test]
fn empty_is_no_signal() {
    let (north, south) = unbeatable_spade_fit();
    let mut rng = StdRng::seed_from_u64(0);
    let hist = single_dummy(north, south, Seat::North, &mut rng, 0);
    assert!(hist.expected_tricks(Seat::North, Strain::Spades).is_nan());
    assert!(hist.make_probability(Seat::North, four_spades()).is_nan());
}

/// The unbeatable-fit deal with the remaining cards split between the
/// defenders — the actual layout the lead scorer plays out.
fn unbeatable_deal() -> FullDeal {
    let (north, south) = unbeatable_spade_fit();
    let east: Hand = ".987654.T9876.T9".parse().expect("valid test hand");
    let west: Hand = ".32.5432.8765432".parse().expect("valid test hand");
    let mut builder = Builder::new();
    builder[Seat::North] = north;
    builder[Seat::South] = south;
    builder[Seat::East] = east;
    builder[Seat::West] = west;
    builder.build_full().expect("52 disjoint cards")
}

/// A silent reading — nothing shown by anyone.
fn no_inferences() -> Inferences {
    use crate::bidding::Context;
    use contract_bridge::auction::RelativeVulnerability;
    Inferences::read(&Context::new(RelativeVulnerability::NONE, &[]))
}

/// Against the lock no lead matters: declarer takes all thirteen whatever
/// East chooses, and the chosen card really is East's.
#[test]
fn lead_cannot_beat_the_lock() {
    let deal = unbeatable_deal();
    let mut rng = StdRng::seed_from_u64(3);
    let (lead, tricks) = single_dummy_lead_tricks(
        &deal,
        Strain::Spades,
        Seat::North,
        &no_inferences(),
        &mut rng,
        8,
    );
    assert!(deal[Seat::East][lead.suit].contains(lead.rank));
    assert_eq!(u8::from(tricks), 13);
}

/// Against the lock even a fallible declarer takes all thirteen: every
/// line wins in every world, so the playout cannot lose a trick.
#[test]
fn playout_cannot_misplay_the_lock() {
    let deal = unbeatable_deal();
    let inferences = no_inferences();
    let mut rng = StdRng::seed_from_u64(5);
    let (lead, lead_tricks, tricks) = single_dummy_declarer_tricks(
        &deal,
        Strain::Spades,
        Seat::North,
        &inferences,
        &inferences,
        &mut rng,
        8,
        8,
    );
    assert!(deal[Seat::East][lead.suit].contains(lead.rank));
    assert_eq!(u8::from(lead_tricks), 13);
    assert_eq!(u8::from(tricks), 13);
}

/// A grand slam hinging on a two-way trump-queen guess: North-South hold
/// every side winner, and the spade suit (AJT9 opposite K876, missing
/// Q5432) picks up West's ♠Q54 double-dummy by finessing through West —
/// but a declarer who cannot see the queen must guess.
fn two_way_guess_deal() -> FullDeal {
    let mut builder = Builder::new();
    builder[Seat::North] = "AJT9.AKQ.AKQ2.AK".parse().expect("valid test hand");
    builder[Seat::South] = "K876.JT9.JT9.QJ4".parse().expect("valid test hand");
    builder[Seat::West] = "Q54.8765.876.T98".parse().expect("valid test hand");
    builder[Seat::East] = "32.432.543.76532".parse().expect("valid test hand");
    builder.build_full().expect("52 disjoint cards")
}

/// Double-dummy the guess deal is a cold grand (the finesse is always
/// "found"), but the single-dummy playout must guess blind: over many
/// seeds it sometimes misguesses (no peeking at the actual layout) and
/// sometimes guesses right — and never loses more than the guess.
#[test]
fn playout_guesses_where_double_dummy_peeks() {
    let deal = two_way_guess_deal();
    // Fixture validity: DD says North makes 7♠ on the actual layout.
    let table = Solver::lock(None).solve_deal(deal);
    assert_eq!(u8::from(table[Strain::Spades].get(Seat::North)), 13);

    let inferences = no_inferences();
    let results: Vec<u8> = (0..12)
        .map(|seed| {
            let mut rng = StdRng::seed_from_u64(seed);
            let (_, lead_tricks, tricks) = single_dummy_declarer_tricks(
                &deal,
                Strain::Spades,
                Seat::North,
                &inferences,
                &inferences,
                &mut rng,
                8,
                8,
            );
            // The lead endpoint plays double-dummy after the lead, so it
            // still picks up the queen every time — the whole gap between
            // the two endpoints on this deal *is* the third-eye finesse.
            assert_eq!(u8::from(lead_tricks), 13);
            u8::from(tricks)
        })
        .collect();
    assert!(
        results.iter().all(|&tricks| (11..=13).contains(&tricks)),
        "only the guess (and rare mean-max risk) may cost tricks: {results:?}"
    );
    assert!(
        results.iter().any(|&tricks| tricks < 13),
        "a blind declarer must sometimes misguess: {results:?}"
    );
    assert!(
        results.contains(&13),
        "a blind declarer must sometimes guess right: {results:?}"
    );
}

/// Same seed and inputs reproduce the same playout exactly.
#[test]
fn playout_is_deterministic() {
    let deal = two_way_guess_deal();
    let inferences = no_inferences();
    let mut rng_a = StdRng::seed_from_u64(17);
    let a = single_dummy_declarer_tricks(
        &deal,
        Strain::Spades,
        Seat::North,
        &inferences,
        &inferences,
        &mut rng_a,
        6,
        6,
    );
    let mut rng_b = StdRng::seed_from_u64(17);
    let b = single_dummy_declarer_tricks(
        &deal,
        Strain::Spades,
        Seat::North,
        &inferences,
        &inferences,
        &mut rng_b,
        6,
        6,
    );
    assert_eq!(a, b);
}

/// Same seed and inputs reproduce the same lead and trick count.
#[test]
fn lead_choice_is_deterministic() {
    let deal = unbeatable_deal();
    let inferences = no_inferences();
    let mut rng_a = StdRng::seed_from_u64(11);
    let a = single_dummy_lead_tricks(
        &deal,
        Strain::Spades,
        Seat::North,
        &inferences,
        &mut rng_a,
        6,
    );
    let mut rng_b = StdRng::seed_from_u64(11);
    let b = single_dummy_lead_tricks(
        &deal,
        Strain::Spades,
        Seat::North,
        &inferences,
        &mut rng_b,
        6,
    );
    assert_eq!(a, b);
}
