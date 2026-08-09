use super::super::tests::{best_call_with, call};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn weak_two_doubled_gets_business_redouble_and_systems_on() {
    let mut arm = Agreements::current();
    arm.competition.weak_two_competition = true;
    // `2♠ (X)`: 17-count with a singleton spade — no Ogust fit — redoubles.
    let auction = [call(2, Strain::Spades), Call::Double];
    let (xx, floored) = best_call_with(&arm, &auction, "A.K654.A964.KQ32");
    assert_eq!(xx, Call::Redouble, "business redouble on values");
    assert!(!floored, "an authored node, not the floor");
    // A 3-card raise stays preemptive (RONF rides through unchanged).
    let (raise, _) = best_call_with(&arm, &auction, "954.Q542.964.432");
    assert_eq!(raise, call(3, Strain::Spades), "the raise stays preemptive");
    // Deeper continuations are systems-on: opener answers Ogust through
    // the rebase exactly as if undisturbed (min points, good suit → 3♦).
    let ogust = [
        call(2, Strain::Hearts),
        Call::Double,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let (answer, _) = best_call_with(&arm, &ogust, "54.KQ9654.96.432");
    assert_eq!(answer, call(3, Strain::Diamonds), "Ogust survives their X");
}

#[test]
fn weak_two_overcalled_double_is_values_and_ogust_survives() {
    let mut arm = Agreements::current();
    arm.competition.weak_two_competition = true;
    // `2♥ (2♠)`: a 12-count doubles for penalty-leaning values.
    let auction = [call(2, Strain::Hearts), call(2, Strain::Spades)];
    let (double, floored) = best_call_with(&arm, &auction, "KJ54.Q5.A964.Q32");
    assert_eq!(double, Call::Double, "values double");
    assert!(!floored, "an authored node, not the floor");
    // A 16-count with a doubleton heart still asks Ogust...
    let (ask, _) = best_call_with(&arm, &auction, "AK54.Q5.A964.K32");
    assert_eq!(ask, call(2, Strain::Notrump), "Ogust survives the overcall");
    // ...and opener's five-rung answer arrives through the targeted rebase.
    let answered = [
        call(2, Strain::Hearts),
        call(2, Strain::Spades),
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let (answer, _) = best_call_with(&arm, &answered, "54.KQ9654.96.432");
    assert_eq!(answer, call(3, Strain::Diamonds), "min points, good suit");
}

#[test]
fn strong_two_contested_stays_strong() {
    let mut arm = Agreements::current();
    arm.competition.strong_two_competition = true;
    // `2♣ (X)`: systems on — a bust still gives the 2♥ double negative.
    let doubled = [call(2, Strain::Clubs), Call::Double];
    let (negative, floored) = best_call_with(&arm, &doubled, "9542.Q54.964.432");
    assert_eq!(negative, call(2, Strain::Hearts), "systems on over their X");
    assert!(!floored, "the rebase resolves to the authored tree");
    // `2♣ (2♠)`: a positive with good hearts bids them naturally (3♥ —
    // the 2-level is gone); a values hand without a suit doubles; a bust
    // passes and waits.
    let overcalled = [call(2, Strain::Clubs), call(2, Strain::Spades)];
    let (positive, _) = best_call_with(&arm, &overcalled, "54.AQ542.964.Q32");
    assert_eq!(positive, call(3, Strain::Hearts), "natural positive");
    let (waiting, _) = best_call_with(&arm, &overcalled, "954.Q542.964.432");
    assert_eq!(waiting, Call::Pass, "the waiting pass");
    // ...backed by opener's forced reopening: 24 balanced with a spade
    // stopper rebids 2NT rather than selling out.
    let reopen = [
        call(2, Strain::Clubs),
        call(2, Strain::Spades),
        Call::Pass,
        Call::Pass,
    ];
    let (rebid, _) = best_call_with(&arm, &reopen, "AQ2.AKQ5.KQ54.A2");
    assert_eq!(rebid, call(2, Strain::Notrump), "opener never sells out");
}
