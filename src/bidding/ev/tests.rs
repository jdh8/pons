use super::*;
use crate::american;
use contract_bridge::auction::RelativeVulnerability;
use contract_bridge::{Bid, Level, Strain};
use rand::SeedableRng;
use rand::rngs::StdRng;

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

/// A balanced 20-count (4-3-3-3, AKQ2/KQ2/KJ2/Q32): strong enough that 3NT
/// is a sound game and 7NT is a hopeless grand, so the EV ranking between
/// them is unambiguous.
fn balanced_twenty() -> Hand {
    "AKQ2.KQ2.KJ2.Q32".parse().expect("valid test hand")
}

/// The deterministic continuation policy used throughout these tests.
fn deterministic() -> impl System {
    american(&crate::bidding::agreements::Agreements::current()).against()
}

/// Sanity: the evaluator prefers the obviously-right call.  As dealer with a
/// flat 20-count, a sound game (3NT) must out-value a hopeless grand (7NT),
/// and the grand must price out clearly negative (it goes down off the top).
#[test]
fn prefers_game_over_hopeless_grand() {
    let policy = deterministic();
    let context = Context::new(RelativeVulnerability::NONE, &[]);
    let mut rng = StdRng::seed_from_u64(20);
    let evs = ev_all(
        balanced_twenty(),
        Seat::North,
        AbsoluteVulnerability::NONE,
        &context,
        &[bid(3, Strain::Notrump), bid(7, Strain::Notrump)],
        &policy,
        &mut rng,
        48,
    );

    assert!(
        evs[0] > evs[1],
        "3NT ({}) should beat 7NT ({})",
        evs[0],
        evs[1]
    );
    assert!(
        evs[1] < 0.0,
        "7NT off the top should be negative, got {}",
        evs[1]
    );
}

/// Determinism: the model never samples its own RNG, so the same seed and
/// inputs reproduce the same EVs exactly (invariant §0.5).
#[test]
fn deterministic_given_a_seed() {
    let policy = deterministic();
    let context = Context::new(RelativeVulnerability::NONE, &[]);
    let calls = [bid(3, Strain::Notrump), Call::Pass];

    let mut rng_a = StdRng::seed_from_u64(7);
    let a = ev_all(
        balanced_twenty(),
        Seat::North,
        AbsoluteVulnerability::NONE,
        &context,
        &calls,
        &policy,
        &mut rng_a,
        24,
    );
    let mut rng_b = StdRng::seed_from_u64(7);
    let b = ev_all(
        balanced_twenty(),
        Seat::North,
        AbsoluteVulnerability::NONE,
        &context,
        &calls,
        &policy,
        &mut rng_b,
        24,
    );
    assert_eq!(a, b);
}

/// An infeasible auction samples no layout, so every EV is `NaN` — the
/// "no signal" contract, not a panic.  North hoards nine hearts while RHO's
/// 1H opening demands five, leaving only four in the deck.
#[test]
fn infeasible_auction_is_no_signal() {
    let policy = deterministic();
    // dealer_of(North, 1) == West, so the lone prior call (1H) is West's,
    // and West is North's RHO — exactly the seat the opening constrains.
    let auction = [bid(1, Strain::Hearts)];
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    let hoard: Hand = "32.AKQJT9876.2.2".parse().expect("valid test hand");
    let mut rng = StdRng::seed_from_u64(5);

    let evs = ev_all(
        hoard,
        Seat::North,
        AbsoluteVulnerability::NONE,
        &context,
        &[Call::Pass, bid(2, Strain::Hearts)],
        &policy,
        &mut rng,
        8,
    );
    assert!(
        evs.iter().all(|ev| ev.is_nan()),
        "no layout means no signal"
    );
}

/// An illegal candidate carries no signal even when other candidates do.
#[test]
fn illegal_candidate_is_nan() {
    let policy = deterministic();
    // RHO (West) opened 1H; North is to act.  1C is below 1H — illegal.
    let auction = [bid(1, Strain::Hearts)];
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    let mut rng = StdRng::seed_from_u64(3);

    let evs = ev_all(
        balanced_twenty(),
        Seat::North,
        AbsoluteVulnerability::NONE,
        &context,
        &[bid(1, Strain::Clubs), Call::Pass],
        &policy,
        &mut rng,
        8,
    );
    assert!(evs[0].is_nan(), "1C over 1H is illegal");
    assert!(evs[1].is_finite(), "Pass is legal and should score");
}

/// Requesting no candidates returns nothing.
#[test]
fn empty_candidates_is_empty() {
    let policy = deterministic();
    let context = Context::new(RelativeVulnerability::NONE, &[]);
    let mut rng = StdRng::seed_from_u64(0);
    assert!(
        ev_all(
            balanced_twenty(),
            Seat::North,
            AbsoluteVulnerability::NONE,
            &context,
            &[],
            &policy,
            &mut rng,
            8,
        )
        .is_empty()
    );
}

/// Phase 1b: the search sampler replays the authored policy by *default* — a
/// sampled world must fall in range *and* reproduce the authored calls.  This
/// guards the flip (`RULE_ACCEPT` in `inference`); revert it and this fails.
#[test]
fn rule_replay_is_the_default() {
    assert!(
        crate::bidding::inference::rule_accept_enabled(),
        "the search EV samples its rollout worlds by rule-replay by default"
    );
}
