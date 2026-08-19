use super::super::tests::{best_call_with, call};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

/// The package's own arm.
fn arm() -> Agreements {
    let mut agreements = Agreements::default();
    agreements.competition.nt_high_overcall_responses = true;
    agreements
}

/// The census hand the plan named: `- 1NT 3♣ ?` on `KQJT742` spades used to be
/// a floor call opener read as nothing, and the board had `6♠` cold.
#[test]
fn the_forcing_three_level_suit_is_authored() {
    let auction = [call(1, Strain::Notrump), call(3, Strain::Clubs)];
    let (bid, floored) = best_call_with(&arm(), &auction, "KQJT742.A5.K3.42");
    assert_eq!(bid, call(3, Strain::Spades), "natural, game-forcing");
    assert!(!floored, "an authored node, not the floor");
    // Opener raises the force to game on three-card support.
    let answer = [
        call(1, Strain::Notrump),
        call(3, Strain::Clubs),
        call(3, Strain::Spades),
        Call::Pass,
    ];
    let (raise, _) = best_call_with(&arm(), &answer, "A95.AJ98.AJ9.J98");
    assert_eq!(raise, call(4, Strain::Spades), "three-card support raises");
}

/// The double is the 4-4 major finder, and its `points(8..)` floor is the
/// census repair — the floor doubled on 6–7 and opener drove to a bad game.
#[test]
fn the_double_finds_the_four_four_major() {
    let auction = [call(1, Strain::Notrump), call(3, Strain::Hearts)];
    let (x, floored) = best_call_with(&arm(), &auction, "QJ86.42.K842.Q95");
    assert_eq!(x, Call::Double, "four spades and values");
    assert!(!floored, "an authored node, not the floor");
    let (pass, _) = best_call_with(&arm(), &auction, "QJ86.42.8642.J95");
    assert_eq!(pass, Call::Pass, "six HCP is below the double's floor");
    // Opener shows the four-card major at its cheapest level.
    let answer = [
        call(1, Strain::Notrump),
        call(3, Strain::Hearts),
        Call::Double,
        Call::Pass,
    ];
    let (major, _) = best_call_with(&arm(), &answer, "AK54.Q5.AJ96.Q73");
    assert_eq!(major, call(3, Strain::Spades), "four spades answer 3♠");
    let (jump, _) = best_call_with(&arm(), &answer, "AK54.Q5.AQ96.Q73");
    assert_eq!(jump, call(4, Strain::Spades), "a maximum jumps to game");
}

/// A six-card major with no three-level slot plays game; a five-card minor
/// below their suit never bypasses `3NT`.
#[test]
fn the_four_level_rungs_are_priced_under_three_notrump() {
    let over_spades = [call(1, Strain::Notrump), call(3, Strain::Spades)];
    let (game, _) = best_call_with(&arm(), &over_spades, "5.KQJ842.K93.T74");
    assert_eq!(game, call(4, Strain::Hearts), "six hearts play game");
    // A five-card club suit with game values bids 3NT, not 4♣ — with a stopper
    // under either setting, and without one under the shipped default, which
    // leans on partner's 1NT for the stop.
    let (notrump, _) = best_call_with(&arm(), &over_spades, "A93.K4.Q82.KJ964");
    assert_eq!(notrump, call(3, Strain::Notrump), "the stopper plays 3NT");
    let (blind, _) = best_call_with(&arm(), &over_spades, "943.KQ.Q82.AKJ96");
    assert_eq!(
        blind,
        call(3, Strain::Notrump),
        "no stopper still plays 3NT"
    );
    // Gate the direct 3NT (the pre-flip arm) and the minor is the fallback.
    let mut gated = arm();
    gated.competition.nt_high_overcall_3nt_stopper = true;
    let (minor, _) = best_call_with(&gated, &over_spades, "943.KQ.Q82.AKJ96");
    assert_eq!(
        minor,
        call(4, Strain::Clubs),
        "gated: no stopper, no major → 4♣"
    );
}

/// The `(3♣)` transfer variant: `3♦` shows hearts, and opener completes at
/// game because the transfer is invitational-plus.
#[test]
fn the_three_club_transfers_are_authored() {
    let mut agreements = arm();
    agreements.competition.nt_3c_transfers = true;
    let auction = [call(1, Strain::Notrump), call(3, Strain::Clubs)];
    let (transfer, floored) = best_call_with(&agreements, &auction, "K5.KJ982.Q943.42");
    assert_eq!(transfer, call(3, Strain::Diamonds), "transfer to hearts");
    assert!(!floored, "an authored node, not the floor");
    let completed = [
        call(1, Strain::Notrump),
        call(3, Strain::Clubs),
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (game, _) = best_call_with(&agreements, &completed, "A95.AQ4.AJ92.J98");
    assert_eq!(game, call(4, Strain::Hearts), "INV+ is driven to game");
    // Their double steals no room — the completion is unchanged.
    let doubled = [
        call(1, Strain::Notrump),
        call(3, Strain::Clubs),
        call(3, Strain::Diamonds),
        Call::Double,
    ];
    let (still, _) = best_call_with(&agreements, &doubled, "A95.AQ4.AJ92.J98");
    assert_eq!(
        still,
        call(4, Strain::Hearts),
        "the doubled transfer completes"
    );

    // The top step exactly mirrors 1NT (2♦) 3♠, with the minors swapped:
    // six-card game force, then 3NT with a stopper or five of the minor.
    let (diamonds, floored) = best_call_with(&agreements, &auction, "K5.43.AKQJ86.432");
    assert_eq!(diamonds, call(3, Strain::Spades), "transfer to diamonds");
    assert!(!floored, "the diamond transfer must come from the book");
    let (stopped, _) = best_call_with(&agreements, &auction, "K5.43.AKQJ86.K32");
    assert_eq!(stopped, call(3, Strain::Notrump), "club stopper → 3NT");
    let (five, _) = best_call_with(&agreements, &auction, "K5.43.AKQJ8.5432");
    assert_eq!(
        five,
        call(3, Strain::Notrump),
        "five diamonds is not enough"
    );
    let diamond_transfer = [
        call(1, Strain::Notrump),
        call(3, Strain::Clubs),
        call(3, Strain::Spades),
        Call::Pass,
    ];
    let (notrump, _) = best_call_with(&agreements, &diamond_transfer, "A432.KQ5.A32.K32");
    assert_eq!(notrump, call(3, Strain::Notrump), "club stopper → 3NT");
    let (game, _) = best_call_with(&agreements, &diamond_transfer, "A432.KQ5.AK3.432");
    assert_eq!(game, call(5, Strain::Diamonds), "no stopper → 5♦");
}
