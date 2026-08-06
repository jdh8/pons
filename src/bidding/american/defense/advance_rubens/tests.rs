use super::super::tests::{best_call, call};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

/// The Rubens layer: a 5+ unbid major transfers via the rank below it, and
/// the doubler completes by declaring that major — over every opening where
/// the transfer is a genuine jump-cue (`1♣`/`1♦`/`1♥`).
#[test]
fn rubens_transfer_completes_into_the_major() {
    super::advance_rich::set_rich_advance_double(true);
    super::advance_rubens::set_advance_rubens(true);

    // Advancer with 5 spades, 10 HCP: transfer via 3♥ (the rank below spades)
    // over each opening; the doubler completes to spades and declares.
    let advancer = "KQJ42.xx.KJx.xxx"; // 5 spades, 10 HCP
    for open in [Strain::Clubs, Strain::Diamonds, Strain::Hearts] {
        let start = [call(1, open), Call::Double, Call::Pass];
        let (xfer, _) = best_call(&start, advancer);
        assert_eq!(
            xfer,
            call(3, Strain::Hearts),
            "5-spade INV+ transfers via 3♥ over (1{open:?})"
        );
        let after = [
            call(1, open),
            Call::Double,
            Call::Pass,
            call(3, Strain::Hearts),
            Call::Pass,
        ];
        let (complete, floored) = best_call(&after, "AKx.xxx.Axxx.xxx");
        assert_eq!(
            complete,
            call(3, Strain::Spades),
            "doubler completes the transfer into spades over (1{open:?})"
        );
        assert!(
            !floored,
            "the completion must come from the book, not the floor"
        );
    }

    super::advance_rubens::set_advance_rubens(false);
    super::advance_rich::set_rich_advance_double(true); // restore default
}
