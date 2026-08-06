use super::super::tests::{bid_transfer, bid_transfer_dbl, call};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn transfer_smolen_three_clubs_is_stayman() {
    // 1NT (2♦): a 4-4 majors game-force bids 3♣ Stayman (a book node).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer(&auction, "AQ32.KJ32.A2.432");
    assert_eq!(c, call(3, Strain::Clubs));
    assert!(!floored, "Stayman must come from the book");
}

#[test]
fn transfer_smolen_opener_answers_stayman() {
    // 1NT (2♦) 3♣: opener shows a 4-card major (3♥ here).
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (c, floored) = bid_transfer(&auction, "K2.AQ54.A32.Q432");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "the Stayman answer must come from the book");
}

#[test]
fn transfer_smolen_three_diamonds_is_the_heart_transfer() {
    // The reshuffle: 1NT (2♦) 3♦ shows hearts (the freed cue slot), a book node.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer(&auction, "K3.KQ976.A32.432");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the heart transfer must come from the book");

    // Opener auto-drives the INV+ transfer to game with a fit.
    let opener = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, _) = bid_transfer(&opener, "AQ5.A432.KQ4.J32");
    assert_eq!(c, call(4, Strain::Hearts));
}

#[test]
fn transfer_smolen_routes_five_four_to_stayman_not_a_transfer() {
    // A 5♠4♥ game-force must bid 3♣ Stayman (1.85), not the 3♥ spade transfer
    // (1.8) — else Smolen could never show the 5-4.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, _) = bid_transfer(&auction, "AKJ54.Q432.K2.32");
    assert_eq!(c, call(3, Strain::Clubs));
}

#[test]
fn transfer_smolen_jumps_smolen_after_the_denial() {
    // `1NT (2♦) 3♣ - 3♦ -` (no major): responder bids Smolen 3♥ to show 5 spades.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, floored) = bid_transfer(&auction, "AKJ54.Q432.K2.32");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "Smolen must come from the book");

    // Opener completes in the five-card spade game.
    let mut full = auction.to_vec();
    full.push(call(3, Strain::Hearts));
    full.push(Call::Pass);
    let (c, _) = bid_transfer(&full, "Q32.A65.AQ43.K32");
    assert_eq!(c, call(4, Strain::Spades));
}

#[test]
fn transfer_smolen_leaping_michaels_both_majors() {
    // 1NT (2♦) 4♦ = both majors 5-5, game-forcing.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer(&auction, "KQ954.AJ876.2.32");
    assert_eq!(c, call(4, Strain::Diamonds));
    assert!(!floored, "Leaping Michaels must come from the book");

    // Opener bids game in the better major (4♠ on three-card support).
    let opener = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(4, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, _) = bid_transfer(&opener, "A32.K43.AQ32.Q42");
    assert_eq!(c, call(4, Strain::Spades));
}

#[test]
fn transfer_smolen_keeps_cohen_over_a_major_overcall() {
    // Over (2♥), Transfer is plain Cohen: 5 spades transfers through
    // hearts — 3♦ shows spades (the Smolen reshuffle is (2♦)-only).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
    let (c, floored) = bid_transfer(&auction, "AKQ65.43.K32.J32");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the Cohen transfer must come from the book");
}

#[test]
fn transfer_lebensohl_shows_spades_through_their_hearts() {
    // 1NT (2♥); responder, 5 spades and game values, transfers *through*
    // hearts: 3♦ shows spades (not diamonds), a book node.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
    let (c, floored) = bid_transfer(&auction, "AKQ65.43.K32.J32");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the transfer must come from the book");
}

#[test]
fn transfer_lebensohl_opener_bids_game_not_a_partscore() {
    // After 1NT (2♥) 3♦ (transfer to spades), opener with a fit must bid
    // the spade *game*, never a 3♠ partscore (the Rubensohl-v1 failure).
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, _) = bid_transfer(&auction, "AK5.KQ52.A43.432");
    assert_eq!(c, call(4, Strain::Spades));
}

#[test]
fn transfer_lebensohl_cue_is_stayman() {
    // 1NT (2♥) 3♥ is the cue = Stayman; opener answers a 4-card major.
    // (Over (2♦) the cue slot is freed for the Smolen 3♣-Stayman instead.)
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let (c, floored) = bid_transfer(&auction, "AQ32.K43.A32.K32");
    assert_eq!(c, call(3, Strain::Spades));
    assert!(!floored, "the Stayman answer must come from the book");
}

#[test]
fn transfer_lebensohl_keeps_the_penalty_double() {
    // Length and values in their suit, no game bid of our own: with the
    // `Penalty` style on, double from the book — Rubensohl v1 lost this by
    // shadowing the floor. (The default is now `Optional` (2-3 cards), which
    // would route this 4-card-diamond hand elsewhere; see
    // [`takeout_authored_double`].)
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer_dbl(
        super::penalty_double::DoubleStyle::Penalty,
        &auction,
        "K2.K43.J932.Q432",
    );
    assert_eq!(c, Call::Double);
    assert!(!floored, "the penalty double must come from the book");
}

#[test]
fn transfer_lebensohl_weak_bids_natural_two_level() {
    // Weak 5-card heart hand still bids natural 2♥ (transfers are INV+).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer(&auction, "K2.QJ976.432.432");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the natural 2-level bid must come from the book");
}

#[test]
fn transfer_lebensohl_top_step_is_a_clubs_transfer() {
    // The top step (no suit above to transfer into) is a forced game-force
    // transfer to clubs: 6+♣, game values, no stopper in their suit. The same
    // 10-HCP hand bids it over every overcall — 3♠ over (2♦)/(2♥), 3♥ over
    // (2♠) — a book node, never the natural floor. Tested under `Penalty`: the
    // default `Takeout` (≤3 in their suit, 1.55) outranks the clubs transfer
    // (1.45) and would hijack this short-suit hand into a takeout double — a
    // known weight interaction; the structural node is checked here in
    // isolation.
    let hand = "32.543.32.AKQJ86";
    for (over, top) in [
        (Strain::Diamonds, Strain::Spades),
        (Strain::Hearts, Strain::Spades),
        (Strain::Spades, Strain::Hearts),
    ] {
        let auction = [call(1, Strain::Notrump), call(2, over)];
        let (c, floored) =
            bid_transfer_dbl(super::penalty_double::DoubleStyle::Penalty, &auction, hand);
        assert_eq!(c, call(3, top), "top step → clubs over (2{over:?})");
        assert!(!floored, "the clubs transfer must come from the book");
    }
}

#[test]
fn transfer_lebensohl_traps_a_too_good_stopper() {
    // Over 1NT (2♥) with game values, a *too-good* heart stopper (♥AQ86, 6
    // HCP in their suit) traps: pass and wait for opener's reopening takeout
    // double, then convert. A merely *adequate* stopper (♥A964, 4 HCP) is a
    // source of tricks and still declares 3NT. (Trap pass on by default.)
    // The trap is a takeout-style mechanism — under the default Penalty style
    // this 4-card-heart hand doubles for penalty directly — so it is pinned to
    // Takeout here; the 3NT line (1.7) outranks any double, so it is style-free.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
    let (trap, _) = bid_transfer_dbl(
        super::penalty_double::DoubleStyle::Takeout,
        &auction,
        "K32.AQ86.KJ5.J32",
    );
    assert_eq!(
        trap,
        Call::Pass,
        "a too-good stopper (6 HCP in hearts) traps"
    );
    // Also pinned to Takeout: under Penalty default this 4-card-heart hand
    // prefers the penalty double (1.55) to the relay's direct 3NT (1.5) — four
    // trumps behind declarer beat one fragile stopper, which is sound.
    let (bid, _) = bid_transfer_dbl(
        super::penalty_double::DoubleStyle::Takeout,
        &auction,
        "K32.A964.KJ5.Q32",
    );
    assert_eq!(
        bid,
        call(3, Strain::Notrump),
        "an adequate stopper (4 HCP in hearts) still bids 3NT"
    );
}

#[test]
fn transfer_lebensohl_top_step_opener_completes_at_game() {
    // After 1NT (2♥) 3♠ (transfer to clubs, forced GF): opener bids 3NT with
    // a heart stopper, else raises to 5♣ — 3♣ is unplayable, so it reaches game.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        call(3, Strain::Spades),
        Call::Pass,
    ];
    let (c, floored) = bid_transfer(&auction, "A432.KQ5.A32.432");
    assert_eq!(c, call(3, Strain::Notrump), "stopper → 3NT");
    assert!(!floored, "the completion must come from the book");

    let (c, _) = bid_transfer(&auction, "A432.543.AKQ.432");
    assert_eq!(c, call(5, Strain::Clubs), "no stopper → 5♣");
}
