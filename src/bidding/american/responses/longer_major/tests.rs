use super::{hearts_first, hearts_take_first, spades_take_first};
use crate::bidding::constraint::Constraint;

#[test]
fn major_selectors_partition_every_holding() {
    // `hearts_first` is the exact complement of `spades_first`, so on any
    // pair of major lengths exactly one of the two selectors fires — the
    // guard that a future edit cannot let them overlap or leave a gap.
    for spades in 0..=13 {
        for hearts in 0..=13 - spades {
            assert_ne!(
                spades_take_first(spades, hearts),
                hearts_take_first(spades, hearts),
                "selectors overlap or gap at {spades}♠ {hearts}♥",
            );
        }
    }
    // Direction: 4-4 up the line to hearts, 5-5 high to spades, 5♠4♥ longer.
    assert!(hearts_take_first(4, 4));
    assert!(spades_take_first(5, 5));
    assert!(spades_take_first(5, 4));
}

#[test]
fn hearts_first_renders_positively() {
    // The point of the change: the 1♥ selector reads as positive prose, not
    // a negated `spades_first` ("not (…)").
    assert_eq!(
        hearts_first().describe().to_string(),
        "hearts longer than spades, or equal below five",
    );
}
