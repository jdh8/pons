use super::super::tests::{P, best_with, bid};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

/// Crawling Stayman: 4-4 majors *short in diamonds* (4414/4405) Stayman and,
/// over opener's 2♦ denial, crawl to 2♥ — opener passes (heart fit), corrects
/// to 2♠ (spade fit), or flees to 3♣ (no major fit, a 5-card-minor 1NT).
#[test]
fn crawling_stayman_escape() {
    let one_nt = [bid(1, Strain::Notrump), P];
    // 4414, a weak 5-count (♠QJ + ♥Q): garbage cannot escape it (one diamond).
    let h4414 = "QJ32.Q1043.4.T543";
    // 4405, a weak 5-count, void diamonds.
    let h4405 = "QJ32.Q1043..T9432";

    let on = Agreements::default();

    // Both short-diamond 4-4 hands bid 2♣ (crawling), unlike garbage Stayman.
    assert_eq!(best_with(&on, &one_nt, h4414), bid(2, Strain::Clubs));
    assert_eq!(best_with(&on, &one_nt, h4405), bid(2, Strain::Clubs));

    // Over opener's 2♦ denial, crawl to 2♥ (both majors, pass-or-correct).
    let two_d = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Diamonds),
        P,
    ];
    assert_eq!(best_with(&on, &two_d, h4414), bid(2, Strain::Hearts));
    assert_eq!(best_with(&on, &two_d, h4405), bid(2, Strain::Hearts));

    // Opener's reply to the crawl (1NT - 2♣ - 2♦ - 2♥): three hearts pass the 4-3
    // fit; two hearts/three spades correct to 2♠; short in both majors with a
    // five-card minor flee to 3♣ (an 8-9 card club fit — responder is short
    // diamonds, hence long clubs).
    let crawl = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Diamonds),
        P,
        bid(2, Strain::Hearts),
        P,
    ];
    assert_eq!(best_with(&on, &crawl, "A32.K43.KQ32.A52"), P); // 3-3 majors → pass 2♥
    assert_eq!(
        best_with(&on, &crawl, "K43.A2.KQ32.A432"),
        bid(2, Strain::Spades)
    ); // 3-2 → 2♠
    assert_eq!(
        best_with(&on, &crawl, "K2.A2.KJ43.AJ432"),
        bid(3, Strain::Clubs)
    ); // 2-2-4-5 → 3♣

    // Doubled tail (1NT - 2♣ - 2♦ (X) 2♥) is systems-on via the competition rebase:
    // responder still crawls to 2♥, and opener still corrects (2♠ shown here).
    let two_d_doubled = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Diamonds),
        Call::Double,
    ];
    assert_eq!(
        best_with(&on, &two_d_doubled, h4414),
        bid(2, Strain::Hearts)
    );
    let crawl_doubled = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Diamonds),
        Call::Double,
        bid(2, Strain::Hearts),
        P,
    ];
    assert_eq!(
        best_with(&on, &crawl_doubled, "K43.A2.KQ32.A432"),
        bid(2, Strain::Spades)
    );

    // With crawling off, the weak short-diamond 4-4 has no escape and passes.
    let mut off = on;
    off.decision.reading.crawling_stayman = false;
    assert_eq!(best_with(&off, &one_nt, h4414), P);
}
