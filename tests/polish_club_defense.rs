//! Integration tests for the Strawberry Polish Club defensive book (M4.3)
//!
//! Spot-checks the direct and balancing seats over a one-of-a-suit opening, plus
//! the direct seat over their 1NT, weak two, and Multi 2♦.  The methods are
//! authored from the `Defense/` chapters of <https://polish.club>: NLTC-gauged
//! natural overcalls, takeout doubles, the Bailey cue (highest unbid plus
//! another), the Unusual 2NT (two lowest unbid), and Landy over their notrump.

use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};
use pons::bidding::polish_club::polish_club;
use pons::bidding::{Family, System};

fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The highest finite-logit call our side makes over the given auction
fn best(auction: &[Call], hand: &str) -> Call {
    let stance = polish_club().against(Family::NATURAL);
    let hand: Hand = hand.parse().expect("valid test hand");
    let logits = stance
        .classify(hand, RelativeVulnerability::NONE, auction)
        .expect("the defensive book covers this auction");
    (&logits.0)
        .into_iter()
        .filter(|(_, l)| l.is_finite())
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("some action")
}

// --- Direct seat over a one-of-a-suit opening ------------------------------

#[test]
fn natural_one_level_overcall() {
    // 5 spades, NLTC 8.0 (in the 6.0–8.5 band) → 1♠ over their 1♦.
    assert_eq!(
        best(&[bid(1, Strain::Diamonds)], "AQJ98.KQ3.52.J64"),
        bid(1, Strain::Spades)
    );
}

#[test]
fn takeout_double_short_in_their_suit() {
    // 17 HCP, a stiff heart, support for the others → takeout double of 1♥.
    assert_eq!(
        best(&[bid(1, Strain::Hearts)], "AKJ5.4.AQ75.K1063"),
        Call::Double
    );
}

#[test]
fn one_notrump_overcall() {
    // 16 balanced with a club stopper → 1NT over their 1♣.
    assert_eq!(
        best(&[bid(1, Strain::Clubs)], "AQ5.KJ4.K72.QJ32"),
        bid(1, Strain::Notrump)
    );
}

#[test]
fn bailey_cue_shows_highest_unbid_plus_another() {
    // 5-5 spades and hearts over 1♣: ♠ is the highest unbid, ♥ the other → cue 2♣.
    assert_eq!(
        best(&[bid(1, Strain::Clubs)], "KQ1098.AJ1098.5.4"),
        bid(2, Strain::Clubs)
    );
}

#[test]
fn unusual_notrump_shows_two_lowest_unbid() {
    // 5-5 in the two lowest unbid (♦ and ♥) over 1♣ → Unusual 2NT.
    assert_eq!(
        best(&[bid(1, Strain::Clubs)], "4.KJ1098.AQ1098.43"),
        bid(2, Strain::Notrump)
    );
}

// --- Balancing seat --------------------------------------------------------

#[test]
fn balancing_double_acts_on_less() {
    // (1♥) P P: a shapely 11-count short in hearts reopens with a takeout double.
    assert_eq!(
        best(
            &[bid(1, Strain::Hearts), Call::Pass, Call::Pass],
            "AQ54.3.KJ85.Q1064"
        ),
        Call::Double,
    );
}

// --- Over their 1NT, weak two, and Multi 2♦ --------------------------------

#[test]
fn landy_over_their_notrump() {
    // Both majors, 12 HCP → Landy 2♣ over their 1NT.
    assert_eq!(
        best(&[bid(1, Strain::Notrump)], "KJ54.QJ65.K3.Q43"),
        bid(2, Strain::Clubs)
    );
}

#[test]
fn takeout_double_of_weak_two() {
    // 14 HCP short in hearts → takeout double of their weak 2♥.
    assert_eq!(
        best(&[bid(2, Strain::Hearts)], "AQ54.3.KJ85.K1064"),
        Call::Double
    );
}

#[test]
fn double_over_their_multi() {
    // 14 HCP with a five-card major → takeout double of their Multi 2♦.
    assert_eq!(
        best(&[bid(2, Strain::Diamonds)], "AKJ54.Q43.5.K1063"),
        Call::Double
    );
}
