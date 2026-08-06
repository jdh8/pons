use super::super::tests::{P, best, bid};
use contract_bridge::Strain;

/// The opt-in six-card-major game invite: just below the Texas blast floor,
/// responder transfers and jumps to `3M`; opener accepts game or passes `3M`
/// on `point_count + trump length`.
#[test]
fn sixcard_major_invite() {
    use crate::bidding::american::set_sixcard_invite_floor;
    use crate::bidding::constraint::set_support_points;

    // This exercises the invite *mechanism* (transfer → 3M invite → accept
    // ladder), whose hands are calibrated to legacy `point_count` arithmetic
    // in the comments below.  The shipped `support_points` scale reads these
    // shaped six-card hands ~1 hotter, tipping some across the blast/accept
    // boundaries — that shift is measured by the A/B and `test_support_points`,
    // so pin the legacy scale here to test the ladder in isolation.
    set_support_points(false);

    let one_nt = [bid(1, Strain::Notrump), P];
    // 6 hearts, ♥KQ + ♠J = 6 HCP, 6-3-2-2: point_count 7 (+1 unbalanced),
    // point_count + length = 13 — one below the blast floor (14), so it invites.
    let inv = "J43.KQ8765.32.32";
    // 6 hearts, ♥KQ only = 5 HCP, point_count 6, sum 12 — too weak to invite.
    let weak = "543.KQ8765.32.32";

    // Turned off (floor 14 == blast floor): the invite hand transfers and the
    // floor handles the rebid — no authored 3♥ invite.
    set_sixcard_invite_floor(14);
    let after_transfer = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Diamonds),
        P,
        bid(2, Strain::Hearts),
        P,
    ];
    assert_ne!(best(&after_transfer, inv), bid(3, Strain::Hearts));

    // On by default (floor 13): the invite hand transfers (2♦) then jumps to 3♥;
    // the weak hand stays out of the invite.
    set_sixcard_invite_floor(13);
    assert_eq!(best(&one_nt, inv), bid(2, Strain::Diamonds));
    assert_eq!(best(&after_transfer, inv), bid(3, Strain::Hearts));
    assert_ne!(best(&after_transfer, weak), bid(3, Strain::Hearts));

    // Opener over 1NT–2♦–2♥–3♥: accept (4♥) on point_count + trump length ≥ 18,
    // else pass.  16 with a doubleton (16+2) accepts; a flat 15 with a doubleton
    // (15+2 = 17) passes; a 15 with three-card support (15+3) accepts.
    let over_invite = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Diamonds),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(3, Strain::Hearts),
        P,
    ];
    assert_eq!(
        best(&over_invite, "AK5.32.AQ74.K963"),
        bid(4, Strain::Hearts)
    ); // 16, ♥xx
    assert_eq!(best(&over_invite, "AK5.32.AQ74.Q963"), P); // 15, ♥xx
    assert_eq!(
        best(&over_invite, "AK52.432.AQ74.Q9"),
        bid(4, Strain::Hearts)
    ); // 15, ♥xxx (4-3-4-2 — a flat 4333 would read 14 and rightly pass)

    // Spade side: 6 spades, ♠KQ + ♥J = 6 HCP transfers (2♥) then jumps to 3♠.
    let spade_inv = "KQ8765.J43.32.32";
    assert_eq!(best(&one_nt, spade_inv), bid(2, Strain::Hearts));
    let after_spade = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(2, Strain::Spades),
        P,
    ];
    assert_eq!(best(&after_spade, spade_inv), bid(3, Strain::Spades));

    set_sixcard_invite_floor(13); // restore the default (on)
    set_support_points(true); // restore the shipped default
}
