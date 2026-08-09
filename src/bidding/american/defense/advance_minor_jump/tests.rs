use super::super::tests::{best_call_with, call};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

/// The invitational minor jump (`agreements.defense.advance_minor_jump_enabled`): a three-level
/// minor jump shows a 5+ one-suiter (10–12) denying a 4-card unbid major.
/// With a 4-card major the advancer cues to find the fit; with a stopper it
/// prefers notrump (the jump ranks below the notrump ladder).
#[test]
fn advance_minor_jump_shows_invitational_one_suiter() {
    // (1♥) X - ? — advancer to act; the unbid major is spades.
    let auction = [call(1, Strain::Hearts), Call::Double, Call::Pass];
    let mut arm = Agreements::current();
    arm.defense.rich_advance_double_enabled = true;
    arm.defense.advance_minor_jump_enabled = true;
    let mut without_jump = arm;
    without_jump.defense.advance_minor_jump_enabled = false;

    // 11 HCP, 5 diamonds, 3 spades (no 4-card major), no heart stopper: an
    // invitational one-suiter → jump 3♦.
    let one_suiter = "xxx.xx.AQJxx.KJx";
    let (jump, _) = best_call_with(&arm, &auction, one_suiter);
    // 11 HCP, 4 spades + 5 diamonds, no heart stopper: a 4-card unbid major →
    // cue 2♥ to find the fit, not the minor jump.
    let with_major = "KQxx.xx.AQxxx.xx";
    let (cued, _) = best_call_with(&arm, &auction, with_major);
    // 11 HCP, 5 diamonds, heart stopper, balanced: a minor needs eleven
    // tricks, so it prefers 2NT — the jump ranks below the notrump ladder.
    let stopped = "Qxx.KJx.AJxxx.xx";
    let (notrump, _) = best_call_with(&arm, &auction, stopped);

    // Knob off: the same one-suiter has no minor jump, so it cues instead of
    // jumping.
    let (off, _) = best_call_with(&without_jump, &auction, one_suiter);

    assert_eq!(
        jump,
        call(3, Strain::Diamonds),
        "invitational 5+ minor, no major → 3♦ jump"
    );
    assert_eq!(
        cued,
        call(2, Strain::Hearts),
        "a 4-card unbid major cues, not the minor jump"
    );
    assert_eq!(
        notrump,
        call(2, Strain::Notrump),
        "a stopper prefers notrump over the minor jump"
    );
    assert_ne!(off, call(3, Strain::Diamonds), "knob off → no minor jump");
}

/// The doubler's accept/decline of the minor jump, and the advancer's
/// placement over the doubler's forcing new suit: the limited invite lets the
/// doubler pass (unlike the forcing cue), accept a game-forcing 5+ suit, or
/// bid `3NT` to play — then the advancer raises the shown suit with support.
#[test]
fn doubler_accepts_or_declines_the_minor_jump() {
    // (1♠) X - 3♦ - ? — doubler acts over the invitational 3♦ jump; the
    // unbid major is hearts.
    let jump = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let mut arm = Agreements::current();
    arm.defense.rich_advance_double_enabled = true;
    arm.defense.advance_minor_jump_enabled = true;

    // Maximum with a 5-card heart suit: accept by showing it, game-forcing.
    let max_major = "x.AKQxx.Kxx.AQxx"; // 18 HCP, 5♥, no spade stopper
    let (accept, _) = best_call_with(&arm, &jump, max_major);
    // Balanced maximum with a spade stopper, no 5-card suit: 3NT to play.
    let max_flat = "KQx.AJx.Qxx.KQxx"; // 17 HCP, spade stopper
    let (notrump, _) = best_call_with(&arm, &jump, max_flat);
    // Minimum takeout double: decline the limited invitation.
    let minimum = "Kxxx.Qxx.xx.Kxxx"; // ~8 HCP, minimum
    let (decline, _) = best_call_with(&arm, &jump, minimum);

    // Advancer places game over the doubler's forcing 3♥: raise with support.
    let after_major = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let with_fit = "xx.Kxx.AQJxx.xxx"; // 10 HCP, 5♦, 3♥ support (a valid 3♦ jump)
    let (raise, _) = best_call_with(&arm, &after_major, with_fit);

    assert_eq!(
        accept,
        call(3, Strain::Hearts),
        "maximum shows a new 5+ suit, game-forcing"
    );
    assert_eq!(
        notrump,
        call(3, Strain::Notrump),
        "balanced maximum accepts 3NT to play"
    );
    assert_eq!(decline, Call::Pass, "minimum declines the limited invite");
    assert_eq!(
        raise,
        call(4, Strain::Hearts),
        "advancer raises the doubler's shown major to game"
    );
}

/// The doubler's stopper-ask cue over the minor jump: with game values but no
/// stopper (and no biddable side suit) the doubler cues their suit; the
/// advancer bids `3NT` with a stopper (right-sided) or signs off in the minor
/// game.
#[test]
fn doubler_stopper_ask_over_the_minor_jump() {
    // (1♠) X - 3♣ - ? — doubler acts over the invitational 3♣ jump.
    let jump = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let mut arm = Agreements::current();
    arm.defense.rich_advance_double_enabled = true;
    arm.defense.advance_minor_jump_enabled = true;

    // 18 HCP, no spade stopper, no 5-card side suit: cue 3♠ to ask.
    let ask = "xxx.AKx.AQx.KQxx";
    let (cue, _) = best_call_with(&arm, &jump, ask);

    // (1♠) X - 3♣ - 3♠ - ? — advancer answers the stopper-ask.
    let after_ask = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ];
    // A spade stopper: right-side the notrump game.
    let stopped = "Kx.xxx.xxx.AQJxx";
    let (notrump, _) = best_call_with(&arm, &after_ask, stopped);
    // No stopper anywhere: play the minor game.
    let no_stop = "xx.xxx.xxx.AKJxx";
    let (minor, _) = best_call_with(&arm, &after_ask, no_stop);

    assert_eq!(
        cue,
        call(3, Strain::Spades),
        "game values without a stopper cue the stopper-ask"
    );
    assert_eq!(
        notrump,
        call(3, Strain::Notrump),
        "the advancer right-sides 3NT with a stopper"
    );
    assert_eq!(
        minor,
        call(5, Strain::Clubs),
        "without a stopper the advancer signs off in the minor game"
    );
}
