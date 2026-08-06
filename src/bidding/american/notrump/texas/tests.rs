use super::super::tests::{P, best, bid};
use contract_bridge::Strain;

/// The revised South African Texas with the slam-drive reroute (default on):
/// a 16+ six-card major Texas-transfers (4♣/4♦) and drives its own RKCB, while
/// the bare-15 cusp keeps the opener-decides direct 4♥; end to end through
/// `american()`.
#[test]
fn south_african_texas_slam_try() {
    let one_nt = [bid(1, Strain::Notrump), P];

    // Responder, 6 hearts: a 16-count (slam) and a 10-count (game) both take the
    // 4♣ Texas transfer; only the bare-15 invitational cusp keeps the direct 4♥.
    assert_eq!(best(&one_nt, "42.AKJ872.KQ4.K2"), bid(4, Strain::Clubs));
    assert_eq!(best(&one_nt, "42.AKJ872.Q43.32"), bid(4, Strain::Clubs));
    assert_eq!(best(&one_nt, "42.AKJ872.KQ4.Q2"), bid(4, Strain::Hearts));

    // Opener over the bare-15 direct invite (1NT - 4♥ -): a maximum (17) launches
    // RKCB, a minimum (15) signs off by passing the major game.
    let over_try = [bid(1, Strain::Notrump), P, bid(4, Strain::Hearts), P];
    assert_eq!(best(&over_try, "KQ3.K53.AQ54.K92"), bid(4, Strain::Notrump));
    assert_eq!(best(&over_try, "KQ3.K53.KQ54.Q92"), P);

    // Opener completes the 4♣ transfer (1NT - 4♣ -) → 4♥.
    let over_transfer = [bid(1, Strain::Notrump), P, bid(4, Strain::Clubs), P];
    assert_eq!(
        best(&over_transfer, "KQ3.K53.KQ54.Q92"),
        bid(4, Strain::Hearts)
    );

    // Responder's drive over the completion (1NT - 4♣ - 4♥ -): the 16-count
    // keycards (4NT), the 10-count passes the game.
    let over_completion = [
        bid(1, Strain::Notrump),
        P,
        bid(4, Strain::Clubs),
        P,
        bid(4, Strain::Hearts),
        P,
    ];
    assert_eq!(
        best(&over_completion, "42.AKJ872.KQ4.K2"),
        bid(4, Strain::Notrump)
    );
    assert_eq!(best(&over_completion, "42.AKJ872.Q43.32"), P);

    // RKCB is wired on the drive: opener answers keycards over responder's 4NT
    // (♥K + ♦A = 2 keycards, no ♥Q → 5♥), proving the ladder is rooted here.
    let over_ask = [
        bid(1, Strain::Notrump),
        P,
        bid(4, Strain::Clubs),
        P,
        bid(4, Strain::Hearts),
        P,
        bid(4, Strain::Notrump),
        P,
    ];
    assert_eq!(best(&over_ask, "KQ3.K53.AQ54.K92"), bid(5, Strain::Hearts));
}
