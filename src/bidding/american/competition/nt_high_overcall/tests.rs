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

/// Opener answers the takeout double in the **longer** major.
///
/// Both majors' rows sit at one weight, so before the `at_least_as_long` guard
/// the call encoding — not the hand — decided, and a 4♥-5♠ answered `3♥`.
#[test]
fn the_double_answer_picks_the_longer_major() {
    let auction = [
        call(1, Strain::Notrump),
        call(3, Strain::Clubs),
        Call::Double,
        Call::Pass,
    ];
    let (five_four, floored) = best_call_with(&arm(), &auction, "AQJ54.KQ98.K3.42");
    assert_eq!(
        five_four,
        call(3, Strain::Spades),
        "five spades outrank four hearts"
    );
    assert!(!floored, "an authored node, not the floor");
    let (four_five, _) = best_call_with(&arm(), &auction, "KQ98.AQJ54.K3.42");
    assert_eq!(
        four_five,
        call(3, Strain::Hearts),
        "and the mirror image answers hearts"
    );
    // The jumped rung ties the same way, and is guarded the same way.
    let (maximum, _) = best_call_with(&arm(), &auction, "AQJ54.KQ98.A3.42");
    assert_eq!(
        maximum,
        call(4, Strain::Spades),
        "a maximum jumps in the longer major"
    );
    let (four_four, _) = best_call_with(&arm(), &auction, "KQ98.AQJ5.K43.42");
    assert_eq!(
        four_four,
        call(3, Strain::Hearts),
        "a genuine 4-4 still fires both rows, and still answers hearts"
    );
}

/// `nt_high_overcall_x_major_at_four` (**shipped default-on**): under their
/// `(3♠)` the shown major's cheapest level is four, and the pre-ship ladder had
/// no rung there.
///
/// The hand is the census board — `1NT (3♠) X -` with `3NT` three down while
/// hearts were worth nine tricks (`docs/one-notrump-competitive.md` §N3).
#[test]
fn the_four_level_major_rung_bids_the_fit() {
    let auction = [
        call(1, Strain::Notrump),
        call(3, Strain::Spades),
        Call::Double,
        Call::Pass,
    ];
    let hand = "K5.QJ84.A92.KQ92";
    let (rung, floored) = best_call_with(&arm(), &auction, hand);
    assert_eq!(
        rung,
        call(4, Strain::Hearts),
        "the shipped rung bids the 4-4 fit the double promised"
    );
    assert!(!floored, "an authored node, not the floor");
    let mut off = arm();
    off.competition.nt_high_overcall_x_major_at_four = false;
    assert_eq!(
        best_call_with(&off, &auction, hand).0,
        call(3, Strain::Notrump),
        "the pre-ship arm punts to 3NT on one stopper"
    );
    // Their `(3♥)` already has a cheap rung, so the knob is inert there.
    let over_hearts = [
        call(1, Strain::Notrump),
        call(3, Strain::Hearts),
        Call::Double,
        Call::Pass,
    ];
    let spades = "QJ84.K5.A92.KQ92";
    assert_eq!(
        best_call_with(&arm(), &over_hearts, spades).0,
        best_call_with(&off, &over_hearts, spades).0,
        "the knob touches only the arm with no three-level rung"
    );
}

/// `nt_high_overcall_x_leave_in` (**default on**): opener converts the takeout
/// double to penalty on **length** in their seven-card suit, and on nothing
/// else.
///
/// The v1 gate was the opposite polarity (`top_honors(over, ..=1)`, i.e. pass
/// on any no-major hand) and was measured and refuted; `thin` below is the hand
/// that arm passed and this one does not.  The three-card half of the v2
/// candidate is [`the_leave_in_three_is_its_own_knob`], measured separately and
/// refuted in its own right.
#[test]
fn the_leave_in_defends_on_length() {
    let auction = [
        call(1, Strain::Notrump),
        call(3, Strain::Clubs),
        Call::Double,
        Call::Pass,
    ];
    let mut off = arm();
    off.competition.nt_high_overcall_x_leave_in = false;
    // Three to one honor in their suit: no trump trick, so `3NT` either way.
    // The refuted v1 gate passed exactly this hand.
    let thin = "AQ3.J72.AQJ4.K92";
    for (agreements, why) in [
        (
            arm(),
            "the shipped gate punts to 3NT: Kxx is not a trump holding",
        ),
        (off, "and so does the arm with the gate switched off"),
    ] {
        assert_eq!(
            best_call_with(&agreements, &auction, thin).0,
            call(3, Strain::Notrump),
            "{why}"
        );
    }
    // Four cards in a suit they have shown seven of.
    let long = "A3.J72.AQJ4.K952";
    assert_eq!(
        best_call_with(&off, &auction, long).0,
        call(3, Strain::Notrump),
        "switched off, opener still punts"
    );
    let (left_in, floored) = best_call_with(&arm(), &auction, long);
    assert_eq!(
        left_in,
        Call::Pass,
        "the shipped gate defends on the length"
    );
    assert!(!floored, "an authored node, not the floor");
    // A four-card major outranks the leave-in, on a hand the gate does accept.
    let fit = "AQ32.J72.K4.KQ95";
    assert_eq!(
        best_call_with(&arm(), &auction, fit).0,
        call(3, Strain::Spades),
        "a fit is still bid"
    );
}

/// `nt_high_overcall_x_leave_in_three` adds the three-card holding headed by
/// two of the top three, and is inert on its own.
///
/// The two disjuncts are separate knobs because the re-slice of v1's dumps
/// (`docs/one-notrump-competitive.md` §N3, "Round 6") had the honor trend
/// running *against* this half at every measured step, while the length half
/// was the best cell in the table.  Round 7 measured them as separate arms and
/// confirmed it: the length half ships, this one is CI-clear negative on
/// sd-lead at both vulnerabilities (−2.44 / −2.99 IMPs per fired) and stays off.
#[test]
fn the_leave_in_three_is_its_own_knob() {
    let auction = [
        call(1, Strain::Notrump),
        call(3, Strain::Clubs),
        Call::Double,
        Call::Pass,
    ];
    // `KQx` of their suit, no four-card major, only three clubs.
    let solid = "A32.J72.AJT4.KQ9";
    let length_only = arm(); // the shipped default: length gate on, extension off
    assert_eq!(
        best_call_with(&length_only, &auction, solid).0,
        call(3, Strain::Notrump),
        "the shipped length gate leaves the three-card holding to 3NT"
    );
    let mut both = length_only;
    both.competition.nt_high_overcall_x_leave_in_three = true;
    let (left_in, floored) = best_call_with(&both, &auction, solid);
    assert_eq!(left_in, Call::Pass, "KQx behind seven of them defends");
    assert!(!floored, "an authored node, not the floor");
    // The extension is inert without the gate it rides on.
    let mut three_only = arm();
    three_only.competition.nt_high_overcall_x_leave_in = false;
    three_only.competition.nt_high_overcall_x_leave_in_three = true;
    assert_eq!(
        best_call_with(&three_only, &auction, solid).0,
        call(3, Strain::Notrump),
        "the extension does nothing without the leave-in itself"
    );
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
