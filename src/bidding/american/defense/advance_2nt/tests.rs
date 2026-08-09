use super::super::tests::{best_call_with, call};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

/// The doubler answers the advancer's invitational `2NT` naturally — declines
/// with a minimum, accepts to play with a balanced maximum, or shows a 5-card
/// major game-forcing — instead of the floor passing a game.
#[test]
fn doubler_accepts_or_declines_the_2nt_invite() {
    // (1♠) X - 2NT - ? — doubler acts over the invitational 2NT; the
    // unbid major is hearts.
    let invite = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let mut arm = Agreements::current();
    arm.defense.rich_advance_double_enabled = true;
    arm.defense.advance_2nt_continuation_enabled = true;

    // Maximum with a 5-card heart suit: accept by showing it, game-forcing.
    let max_major = "x.AKQxx.Kxx.AQxx"; // 18 HCP, 5♥
    let (accept, _) = best_call_with(&arm, &invite, max_major);
    // Balanced maximum, no 5-card major: 3NT to play.
    let max_flat = "KQx.AJx.Qxx.KQxx"; // 17 HCP, balanced
    let (notrump, _) = best_call_with(&arm, &invite, max_flat);
    // Minimum takeout double: decline the limited invite, pass 2NT.
    let minimum = "KQxx.Qxx.xx.KQxx"; // 12 HCP, minimum
    let (decline, _) = best_call_with(&arm, &invite, minimum);

    // Advancer places game over the doubler's forcing 3♥: raise with support.
    let after_major = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let with_fit = "Axx.Kxx.Qxx.QJxx"; // 12 HCP, 3♥ support, spade stopper
    let (raise, _) = best_call_with(&arm, &after_major, with_fit);

    assert_eq!(
        accept,
        call(3, Strain::Hearts),
        "maximum shows a 5-card major, game-forcing"
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
