use super::super::tests::{bid, call};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn lebensohl_forcing_three_level_is_a_book_node() {
    // 1NT–(2♦); responder 5 spades, game values, no diamond stopper →
    // forcing 3♠ (a jump), not a partscore.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid(&auction, "KQT95.A43.32.J32");
    assert_eq!(c, call(3, Strain::Spades));
    assert!(!floored, "the forcing 3-level bid must come from the book");
}

#[test]
fn lebensohl_weak_long_suit_relays_then_completes() {
    // Weak hand (6 HCP), 6 clubs, over 2♦ → 2NT relay; opener forced to 3♣.
    let responder = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid(&responder, "J2.43.32.KQ9876");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the Lebensohl relay must come from the book");

    let opener = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let (completion, _) = bid(&opener, "AQ32.KQ5.AQ4.A32");
    assert_eq!(completion, call(3, Strain::Clubs));
}

#[test]
fn lebensohl_weak_bids_natural_two_level() {
    // A weak hand with 5 hearts bids natural 2♥ (below 2NT), to play.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid(&auction, "K2.QJ976.432.432");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the natural 2-level bid must come from the book");
}

#[test]
fn lebensohl_cue_is_stayman() {
    // 1NT–(2♥): a game-force with 4 spades and no 5-card suit cues 3♥ = Stayman
    // (it cannot bid a forcing 3-level suit, and the cue outranks direct 3NT).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
    let (c, floored) = bid(&auction, "AQ32.K43.A32.K32");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "the cue must come from the book");

    // Opener answers Stayman with the 4-card spade fit.
    let opener = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let (a, floored) = bid(&opener, "KJ54.A32.K43.Q32");
    assert_eq!(a, call(3, Strain::Spades));
    assert!(!floored, "the Stayman answer must come from the book");
}

#[test]
fn lebensohl_five_card_suit_relays_then_signs_off_at_the_three_level() {
    // Weak hand, a 5-card heart suit it cannot show at the 2 level (below
    // their 2♠): relay 2NT, then correct 3♣→3♥ as a 3-level sign-off.
    let responder = [call(1, Strain::Notrump), call(2, Strain::Spades)];
    let (c, floored) = bid(&responder, "32.KQJ32.432.432");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the relay must come from the book");

    let after_3c = [
        call(1, Strain::Notrump),
        call(2, Strain::Spades),
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (c, floored) = bid(&after_3c, "32.KQJ32.432.432");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "the 3-level sign-off must come from the book");
}

#[test]
fn lebensohl_maximum_raises_weak_signoff_to_game() {
    // 1NT–(2♠)–2NT–P–3♣–P–3♥–P: responder's weak heart sign-off. A maximum
    // (17) opener with three-card support stretches to 4♥; a minimum passes.
    let after_signoff = [
        call(1, Strain::Notrump),
        call(2, Strain::Spades),
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    // 4-3-4-2, 17: a flat 4-3-3-3 17-count would read 16 on the shipped
    // rule-of-N+8 scale and rightly decline the stretch.
    let (c, floored) = bid(&after_signoff, "AK32.K43.A432.K3");
    assert_eq!(c, call(4, Strain::Hearts));
    assert!(!floored, "the game raise must come from the book");

    let (c, _) = bid(&after_signoff, "AK32.K43.KQ3.432");
    assert_eq!(c, Call::Pass, "a minimum passes the weak sign-off");
}
