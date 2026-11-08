use super::super::tests::{best_call_with, call};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

fn on() -> Agreements {
    Agreements::default()
}

fn off() -> Agreements {
    let mut agreements = Agreements::default();
    agreements.competition.other_major_takeout_double = false;
    agreements
}

const H: Strain = Strain::Hearts;
const S: Strain = Strain::Spades;
const C: Strain = Strain::Clubs;
const D: Strain = Strain::Diamonds;
const NT: Strain = Strain::Notrump;

fn x_answer(opening: Strain, level: u8, overcall: Strain) -> [Call; 4] {
    [
        call(1, opening),
        call(level, overcall),
        Call::Double,
        Call::Pass,
    ]
}

#[test]
fn the_double_is_takeout_for_the_minors() {
    let a = [call(1, H), call(1, S)];
    assert_eq!(
        best_call_with(&on(), &a, "K43.82.KJ43.Q763").0,
        Call::Double
    );
    // Four of their suit may still double with the shape...
    assert_eq!(
        best_call_with(&on(), &a, "KJ43.82.KJ4.Q763").0,
        Call::Double
    );
    // ...but the trump stack does not.
    assert_ne!(
        best_call_with(&on(), &a, "KQT863.2.K43.KT7").0,
        Call::Double
    );
    // Off, the stack doubles as shipped.
    assert_eq!(
        best_call_with(&off(), &a, "KQT863.2.K43.KT7").0,
        Call::Double
    );
}

#[test]
fn the_two_level_floor_is_ten() {
    let a = [call(1, S), call(2, H)];
    assert_ne!(
        best_call_with(&on(), &a, "82.K43.KJ43.Q763").0,
        Call::Double
    );
    assert_eq!(
        best_call_with(&on(), &a, "82.K43.KJ43.A763").0,
        Call::Double
    );
    let a = [call(1, H), call(2, S)];
    assert_eq!(
        best_call_with(&on(), &a, "K43.82.KJ43.A763").0,
        Call::Double
    );
}

#[test]
fn opener_answers_at_every_slot() {
    let answer = |a: &[Call], hand| {
        let (c, floored) = best_call_with(&on(), a, hand);
        assert!(!floored, "{hand}");
        c
    };
    let a = x_answer(H, 1, S);
    // The trace hand: the floor jumped to 4♥ on a 5-2.
    assert_eq!(answer(&a, "A5.KJ654.AT5.983"), call(1, NT));
    // No four-card minor, no stopper: the finite catch-all.
    assert_eq!(answer(&a, "95.KJ654.AQ5.A83"), call(2, H));
    assert_eq!(answer(&a, "A5.KJ654.AT54.98"), call(2, D));
    assert_eq!(answer(&a, "A5.KJ654.A5.T984"), call(2, C));
    assert_eq!(answer(&a, "K54.KJ654.AQ5.98"), call(1, NT));
    assert_eq!(answer(&a, "A5.AKJ654.AT5.98"), call(3, H));
    assert_eq!(answer(&a, "A5.AKJ54.AK54.98"), call(3, D));
    let a = x_answer(S, 2, H);
    assert_eq!(answer(&a, "QT874.7.KQ63.AT5"), call(3, D));
    assert_eq!(answer(&a, "KQT87.A7.K63.T54"), call(2, NT));
    assert_eq!(answer(&a, "KQT87.72.KQ3.Q54"), call(2, S));
    let a = x_answer(H, 2, S);
    assert_eq!(answer(&a, "95.KJ654.AQ5.A83"), call(3, H));
    assert_eq!(answer(&a, "K54.KJ654.AQ5.98"), call(2, NT));
}

#[test]
fn opener_never_passes_for_penalty() {
    let (c, _) = best_call_with(&on(), &x_answer(H, 1, S), "KJ54.KJ654.A5.98");
    assert_ne!(c, Call::Pass);
}

#[test]
fn doubler_reaches_game_opposite_the_notrump_rebid() {
    let a = [
        call(1, H),
        call(1, S),
        Call::Double,
        Call::Pass,
        call(1, NT),
        Call::Pass,
    ];
    let answer = |hand| {
        let (c, floored) = best_call_with(&on(), &a, hand);
        assert!(!floored, "{hand}");
        c
    };
    assert_eq!(answer("K43.82.KJ43.AQ63"), call(3, NT));
    assert_eq!(answer("K43.82.KJ43.Q763"), Call::Pass);
}

#[test]
fn interference_keeps_opener_in_the_book() {
    let a = [call(1, H), call(1, S), Call::Double, call(2, S)];
    let (c, floored) = best_call_with(&on(), &a, "A5.KJ9654.AT5.98");
    assert_eq!(c, call(3, H));
    assert!(!floored);
    let (c, floored) = best_call_with(&on(), &a, "A5.KJ65.AT54.983");
    assert_eq!(c, Call::Pass);
    assert!(!floored);
    let a = [call(1, H), call(1, S), Call::Double, Call::Redouble];
    let (c, floored) = best_call_with(&on(), &a, "95.KJ654.AQ5.A83");
    assert_eq!(c, call(2, H));
    assert!(!floored);
}

#[test]
fn off_leaves_the_answer_to_the_floor() {
    let (_, floored) = best_call_with(&off(), &x_answer(H, 1, S), "A5.KJ654.AT5.983");
    assert!(floored);
}
