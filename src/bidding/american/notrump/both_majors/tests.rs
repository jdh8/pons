use super::super::tests::{P, best, bid};
use contract_bridge::Strain;

#[test]
fn both_majors_relay_game_placement() {
    // 1NT - 2♣ - 2NT - 3♣ (responder names hearts) - 3♥: responder
    // places game on `point_count + extra trumps + a fit in the other major`.
    let relay = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Notrump),
        P,
        bid(3, Strain::Clubs),
        P,
        bid(3, Strain::Hearts),
        P,
    ];

    // Double 4-4 fit: a flat 7 reaches game (7 + 0 + 1 = 8) — the second major
    // fit is knowable because opener showed both majors.
    assert_eq!(best(&relay, "KQ54.J932.654.J2"), bid(4, Strain::Hearts));
    // Single 8-card fit, 8 HCP: the pre-accepted invite bids game (8 + 0 + 0).
    assert_eq!(best(&relay, "K32.A654.J432.32"), bid(4, Strain::Hearts));
    // Below the authored `fit_value >= 8` gate the floor's fit-sum (default 31,
    // a measured default-on win) takes over, counting the full trump length
    // opposite opener's 16-point max: a 6-count with a nine-card fit
    // (6 + 16 + 5 + 4 = 31) and a 7-count with an eight-card fit
    // (7 + 16 + 4 + 4 = 31) both clear it and bid game.
    assert_eq!(best(&relay, "Q32.KJ954.762.32"), bid(4, Strain::Hearts));
    assert_eq!(best(&relay, "K32.QJ54.J432.32"), bid(4, Strain::Hearts));
}
