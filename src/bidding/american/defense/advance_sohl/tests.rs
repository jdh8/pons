use super::super::tests::{best_call, call};
use crate::bidding::american::{LebensohlStyle, set_advance_sohl_style};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

/// Best call with the advance-of-double sohl forced to `style` (independent of
/// any other test on this thread having changed it)
fn advance(style: LebensohlStyle, auction: &[Call], hand: &str) -> (Call, bool) {
    set_advance_sohl_style(style);
    best_call(auction, hand)
}

/// `(2♦) X -` — partner doubled their weak two, advancer to act
fn over_2d() -> [Call; 3] {
    [call(2, Strain::Diamonds), Call::Double, Call::Pass]
}

#[test]
fn off_keeps_the_flat_advance_no_relay() {
    // Default Off: a weak six-club hand bids the natural 3♣ (advance_double),
    // not the 2NT relay — the toggle gates the new structure.
    let (c, _) = advance(LebensohlStyle::Off, &over_2d(), "32.43.32.KQ9876");
    assert_eq!(c, call(3, Strain::Clubs));
}

#[test]
fn plain_weak_long_suit_relays_then_completes() {
    // Plain: weak hand (6 HCP), six clubs → 2NT relay; doubler forced to 3♣.
    let (c, floored) = advance(LebensohlStyle::Plain, &over_2d(), "J2.43.32.KQ9876");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the relay must come from the book");

    let relayed = [
        call(2, Strain::Diamonds),
        Call::Double,
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let (completion, _) = advance(LebensohlStyle::Plain, &relayed, "AKJ2.KQ52.4.A532");
    assert_eq!(completion, call(3, Strain::Clubs));
}

#[test]
fn plain_forcing_three_level_is_a_book_node() {
    // Plain: five spades and game values → forcing 3♠ (a jump over 2♦),
    // never a weak partscore.
    let (c, floored) = advance(LebensohlStyle::Plain, &over_2d(), "KQT95.A43.32.J32");
    assert_eq!(c, call(3, Strain::Spades));
    assert!(!floored, "the forcing 3-level bid must come from the book");
}

#[test]
fn transfer_shows_spades_through_their_hearts() {
    // Transfer: over (2♥), five spades and game values transfer *through*
    // hearts — 3♦ shows spades (not diamonds), a book node.
    let over_2h = [call(2, Strain::Hearts), Call::Double, Call::Pass];
    let (c, floored) = advance(LebensohlStyle::Transfer, &over_2h, "AKQ65.43.K32.J32");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the transfer must come from the book");
}

#[test]
fn transfer_doubler_bids_game_not_partscore() {
    // After (2♥) X - 3♦ (transfer to spades), the doubler with a fit bids
    // the spade *game*, never a 3♠ partscore.
    let auction = [
        call(2, Strain::Hearts),
        Call::Double,
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, _) = advance(LebensohlStyle::Transfer, &auction, "AK52.4.A432.K432");
    assert_eq!(c, call(4, Strain::Spades));
}

#[test]
fn transfer_cue_is_stayman() {
    // (2♥) X - 3♥ is the cue = Stayman; the doubler shows a 4-card major.
    // (Over (2♦) the cue slot is freed for the Smolen 3♣-Stayman instead.)
    let auction = [
        call(2, Strain::Hearts),
        Call::Double,
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let (c, floored) = advance(LebensohlStyle::Transfer, &auction, "AQ32.K32.4.KJ432");
    assert_eq!(c, call(3, Strain::Spades));
    assert!(!floored, "the Stayman answer must come from the book");
}

#[test]
fn penalty_pass_sits_for_the_double() {
    // A trump stack in their suit (five spades over 2♠) has no constructive
    // call — the book's terminal Pass leaves the takeout double in for
    // penalty, exactly as the flat ladder would.
    let over_2s = [call(2, Strain::Spades), Call::Double, Call::Pass];
    let (c, floored) = advance(LebensohlStyle::Plain, &over_2s, "KQJ95.J32.432.32");
    assert_eq!(c, Call::Pass);
    assert!(!floored, "the sign-off Pass must come from the book node");
}

#[test]
fn transfer_over_2d_is_three_club_stayman() {
    // (2♦) X -: Transfer's (2♦)-only Smolen leg bids 3♣-Stayman for a 4-4
    // majors GF advancer, a book node (over (2♥)/(2♠) it is plain Cohen, whose
    // 3♣ is not Stayman).
    let (c, floored) = advance(LebensohlStyle::Transfer, &over_2d(), "AQ32.KJ32.A2.432");
    assert_eq!(c, call(3, Strain::Clubs));
    assert!(!floored, "the Stayman bid must come from the book");
}

#[test]
fn transfer_over_2h_is_plain_cohen() {
    // Over (2♥) Transfer is plain Cohen: a 5-spade GF transfers *through*
    // hearts — 3♦ shows spades, a book node (the diamond Smolen leg only
    // fires over (2♦)).
    let over_2h = [call(2, Strain::Hearts), Call::Double, Call::Pass];
    let (c, floored) = advance(LebensohlStyle::Transfer, &over_2h, "AKQ65.43.K32.J32");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the transfer must come from the book");
}
