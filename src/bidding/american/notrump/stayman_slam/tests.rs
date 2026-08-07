use super::super::tests::{P, best, bid};
use contract_bridge::Strain;

#[test]
pub fn stayman_minor_slam_try() {
    use crate::bidding::american::set_stayman_minor_slam_try;
    set_stayman_minor_slam_try(true);

    // Responder: 4♠ 5♣, ≤3 hearts, 14 HCP — a slam-oriented two-suiter that
    // Staymaned, found no heart fit, and shows its longer minor.
    let after_2h = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Hearts),
        P,
    ];
    assert_eq!(best(&after_2h, "AJ54.32.32.AKQ32"), bid(3, Strain::Clubs));

    // Opener over the 3♣ slam try.
    let after_3c = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(3, Strain::Clubs),
        P,
    ];
    // Fit (4♣) + maximum (16): cooperate by raising the minor.
    assert_eq!(best(&after_3c, "A2.AQJ2.K32.Q543"), bid(4, Strain::Clubs));
    // No club fit (3♣): sign off in 3NT even with a maximum.
    assert_eq!(best(&after_3c, "A2.AQJ2.K432.Q54"), bid(3, Strain::Notrump));

    // Responder keycards over opener's minor raise (1430 RKCB).
    let after_4c = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(3, Strain::Clubs),
        P,
        bid(4, Strain::Clubs),
        P,
    ];
    assert_eq!(best(&after_4c, "AJ54.32.32.AKQ32"), bid(4, Strain::Notrump));

    // Off the gate the sequence is unauthored — responder does not bid 3♣.
    set_stayman_minor_slam_try(false);
    assert_ne!(best(&after_2h, "AJ54.32.32.AKQ32"), bid(3, Strain::Clubs));
    set_stayman_minor_slam_try(true);
}
