use super::super::tests::{best_call, bid_xfer, call};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

// --- Competition over our Jacoby transfers (Side A) + defense to theirs (B) ---

#[test]
fn transfer_super_accept_uncontested() {
    // 1NT - 2♦ -: four hearts + a maximum → 3♥ (jump super-accept).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, floored) = bid_xfer(&auction, "A2.KQ32.KQ32.K32");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "the super-accept must come from the book");
}

#[test]
fn transfer_doubled_opener_completes_with_support() {
    // 1NT - 2♦ (X): three hearts, not a maximum → 2♥ (complete the transfer).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Double,
    ];
    let (c, floored) = bid_xfer(&auction, "KQ2.K32.KQ32.Q32");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the completion must come from the book");
}

#[test]
fn transfer_doubled_opener_super_accepts() {
    // 1NT - 2♦ (X): four hearts + a maximum → 3♥ (the double does not suppress it).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Double,
    ];
    let (c, _) = bid_xfer(&auction, "A2.KQ32.KQ32.K32");
    assert_eq!(c, call(3, Strain::Hearts));
}

#[test]
fn transfer_doubled_opener_passes_with_doubleton() {
    // 1NT - 2♦ (X): only a doubleton heart → Pass (declines; responder re-asks).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Double,
    ];
    let (c, floored) = bid_xfer(&auction, "KQ32.K2.KQ32.Q32");
    assert_eq!(c, Call::Pass);
    assert!(!floored, "the declining pass must come from the book");
}

#[test]
fn transfer_doubled_opener_redoubles_with_the_transfer_suit() {
    // 1NT - 2♦ (X): the doubled diamonds are opener's own (5 to AKQ) → XX.
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Double,
    ];
    let (c, _) = bid_xfer(&auction, "Q43.K2.AKQ32.Q32");
    assert_eq!(c, Call::Redouble);
}

#[test]
fn transfer_doubled_reask_is_forcing() {
    // 1NT - 2♦ (X) - -: responder re-asks with XX (still holds five hearts).
    let reask = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Double,
        Call::Pass,
        Call::Pass,
    ];
    let (c, floored) = bid_xfer(&reask, "K2.QJ432.K32.432");
    assert_eq!(c, Call::Redouble);
    assert!(!floored, "the re-ask must come from the book");
    // … XX -: opener is forced to complete (no Pass) → 2♥.
    let answer = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Double,
        Call::Pass,
        Call::Pass,
        Call::Redouble,
        Call::Pass,
    ];
    let (c, floored) = bid_xfer(&answer, "AQ32.K32.KQ2.432");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the forced completion must come from the book");
}

#[test]
fn transfer_overcalled_opener_super_accepts() {
    // 1NT - 2♦ (2♠): four-card heart fit → 3♥ (cheapest level above their 2♠).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        call(2, Strain::Spades),
    ];
    let (c, floored) = bid_xfer(&auction, "K2.KQ32.AQ32.K32");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "the natural super-accept must come from the book");
}

#[test]
fn defense_to_their_transfer_doubles_the_bid_suit() {
    // After `(1NT) - (2♦)`, their 2♦ transfers to hearts; our fourth-hand X is
    // lead-directing in diamonds, the bid suit.
    crate::bidding::american::set_transfer_defense(true);
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
    ];
    let (c, floored) = best_call(&auction, "K2.A32.KQ1054.432");
    crate::bidding::american::set_transfer_defense(false); // restore default
    assert_eq!(c, Call::Double);
    assert!(
        !floored,
        "the lead-directing X must come from the defense book"
    );
}

#[test]
fn defense_to_their_transfer_cues_michaels() {
    // After `(1NT) - (2♦)`, their 2♦ transfers to hearts; 5 spades and 5
    // diamonds cue 2♥ to show the other major plus a minor.
    crate::bidding::american::set_transfer_defense(true);
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
    ];
    let (c, floored) = best_call(&auction, "AQ1054.3.KJ1054.32");
    crate::bidding::american::set_transfer_defense(false); // restore default
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the Michaels cue must come from the defense book");
}
