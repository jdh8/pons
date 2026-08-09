use super::super::tests::{best_call, bid_uvu, call};
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Hand, Strain};

#[test]
fn uvu_three_clubs_is_stayman() {
    // `1NT (2NT)`: their 2NT shows both minors; a 4-4 majors hand bids 3♣
    // (Stayman), a book node.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Notrump)];
    let (c, floored) = bid_uvu(&auction, "AQ32.KJ32.A2.432");
    assert_eq!(c, call(3, Strain::Clubs));
    assert!(!floored, "the cue must come from the book");
}

#[test]
fn uvu_three_diamonds_shows_hearts() {
    // 1NT (2NT): 5+♥ with ≤3♠ bids 3♦ (the heart cue).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Notrump)];
    let (c, _) = bid_uvu(&auction, "K3.KQ976.A32.432");
    assert_eq!(c, call(3, Strain::Diamonds));
}

#[test]
fn uvu_splinter_with_five_five() {
    // 1NT (2NT): 5-5 majors short a club → 4♣ splinter (FG+).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Notrump)];
    let (c, _) = bid_uvu(&auction, "AQ876.KJ987.32.A");
    assert_eq!(c, call(4, Strain::Clubs));
}

#[test]
fn uvu_penalty_double_on_values() {
    // 1NT (2NT): flat values, no 4-card major, no minor stopper → penalty X.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Notrump)];
    let (c, floored) = bid_uvu(&auction, "KJ2.AQ2.J532.532");
    assert_eq!(c, Call::Double);
    assert!(!floored, "the penalty X must come from the book");
}

#[test]
fn uvu_smolen_shows_the_five_card_spade() {
    // `1NT (2NT) 3♣ - 3♦ -`: opener's 3♦ denial lets responder bid Smolen 3♥
    // to show 5+ spades, with no heart promise.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Notrump),
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, floored) = bid_uvu(&auction, "AQ876.K32.A32.32");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "Smolen must come from the book");
}

#[test]
fn uvu_disabled_falls_to_floor() {
    // Disabled, 1NT (2NT) has no book node → instinct floor (the toggle works).
    super::uvu::set_uvu(false);
    let auction = [call(1, Strain::Notrump), call(2, Strain::Notrump)];
    let (_, floored) = best_call(&auction, "AQ32.KJ32.A2.432");
    super::uvu::set_uvu(true); // restore the default for sibling tests on this thread
    assert!(floored, "without the toggle the auction is unauthored");
}

/// `1NT (2NT) X`, opponents run to 3♣: responder with a club stack doubles
/// (the UvU penalty chase), and partner would leave it in.
///
/// Asserted against `american_instinct()`, not `american()`, because the
/// chase is a rule of the **deterministic** ladder gated on
/// `set_uvu_encircle` — and `with_floor` attaches `instinct()` to the
/// *constructive* book only, so on a contested auction like this one the
/// learned floor is the sole answer and the rule never runs. Through
/// `american()` this test asserted that the net happened to *agree* with the
/// ladder, which the v3 net did and the configured v4 net does not (it
/// passes, X 7.813 against P 8.544 — a 0.73-logit margin). Pinning an
/// individual net call is what `tests/common/mod.rs` exists to forbid; the
/// net is validated in aggregate by the `ab-*` harnesses.
#[test]
fn uvu_encircling_doubles_the_runout() {
    super::uvu::set_uvu(true);
    crate::bidding::instinct::set_uvu_encircle(true);
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Notrump),
        Call::Double,
        call(3, Strain::Clubs),
        Call::Pass,
        Call::Pass,
    ];
    let hand: Hand = "K54.84.732.KQJT9".parse().expect("valid test hand");
    let (logits, _) = crate::bidding::american::american_instinct(
        &crate::bidding::agreements::Agreements::current(),
    )
    .against()
    .classify_with_provenance(hand, RelativeVulnerability::NONE, &auction)
    .expect("a legal auction classifies");
    let c = (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty");
    crate::bidding::instinct::set_uvu_encircle(true); // the shipped default
    assert_eq!(c, Call::Double, "encircle the 3♣ runout with a club stack");
}
