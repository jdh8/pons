use super::dutch;
use crate::bidding::System;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Strain};

/// The Dutch opening for a first-seat hand.
fn opens(hand: &str) -> Call {
    let stance = dutch(&crate::bidding::agreements::Agreements::default()).against();
    let hand = hand.parse().unwrap();
    let logits = stance
        .classify(hand, RelativeVulnerability::NONE, &[])
        .expect("an opening decision");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(call, _)| call)
        .unwrap()
}

fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// Both Dutch override packages preserve the declarative row invariants.
#[test]
fn row_package_invariants() {
    crate::bidding::rows::assert_package_invariants(
        &crate::bidding::agreements::Agreements::default(),
        &[super::openings::package(), super::responses::package()],
    );
}

/// The wide-1♣ opening partition (Phase 1): the load-bearing cases.
#[test]
fn opening_partition() {
    // The wide 1♣ hosts a strong balanced 23-count (american opens it 2♣).
    assert_eq!(opens("AKQ2.KQ3.KQ3.A32"), bid(1, Strain::Clubs));
    // Four-diamond hands open 1♣ — every one but the 4=4=4=1.
    assert_eq!(opens("KQ32.K32.KJ32.32"), bid(1, Strain::Clubs));
    // The singleton-club 4=4=4=1 is the one four-diamond hand that opens 1♦.
    assert_eq!(opens("KQ32.KQ32.Q432.2"), bid(1, Strain::Diamonds));
    // A real five-card diamond suit opens 1♦.
    assert_eq!(opens("A32.3.KQ432.K432"), bid(1, Strain::Diamonds));
    // 21–23 with a five-card major is the strong, artificial 2♣.
    assert_eq!(opens("AKQ32.AK3.AQ2.32"), bid(2, Strain::Clubs));
    // A balanced 16 opens 1NT, and — american's wide shape — so does a 5422
    // or 6322 with a long *minor* (was the wide 1♣ before the widening).
    assert_eq!(opens("AQ32.K53.QJ4.A92"), bid(1, Strain::Notrump));
    assert_eq!(opens("Q432.KQ.K2.AK432"), bid(1, Strain::Notrump)); // 5422, 5♣
    assert_eq!(opens("Q2.K3.AQ4.KQ8765"), bid(1, Strain::Notrump)); // 6322, 6♣
    // A 5422 with the five-card suit a *major* stays a suit opening (1♠).
    assert_eq!(opens("AK432.KQ.Q432.K2"), bid(1, Strain::Spades));
    // `points(12..)` gates the light end, and it is the Rule of 20 wherever
    // the two longest suits reach eight cards.  Flat 4-3-3-3 is the one
    // shape that falls short — the 1♣ node's extra doubleton term makes it
    // pay the thirteenth point the Rule of 20 asked for, so a flat 12-count
    // still passes and a flat 13-count opens.
    assert_eq!(opens("KJ32.K32.K32.Q32"), Call::Pass);
    assert_eq!(opens("KJ32.K32.KQ2.Q32"), bid(1, Strain::Clubs));
}

/// The Dutch call after an undisturbed `auction`.
fn responds(auction: &[Call], hand: &str) -> Call {
    let stance = dutch(&crate::bidding::agreements::Agreements::default()).against();
    let hand = hand.parse().unwrap();
    let logits = stance
        .classify(hand, RelativeVulnerability::NONE, auction)
        .expect("a decision");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(call, _)| call)
        .unwrap()
}

/// Responder's first call over the wide 1♣ (Phase 2.1).
#[test]
fn wide_1c_responses() {
    const P: Call = Call::Pass;
    let one_club = [bid(1, Strain::Clubs), P];
    // Weak, club-tolerant (3 HCP, 4♣): content to play 1♣.
    assert_eq!(responds(&one_club, "xxx.xxx.xxx.Kxxx"), P);
    // 5 HCP 4-4 majors, too weak for a 7+ major: the artificial relay.
    assert_eq!(
        responds(&one_club, "Kxxx.Qxxx.xxx.xx"),
        bid(1, Strain::Diamonds)
    );
    // 16 HCP, 5+♦, no four-card major: natural game force.
    assert_eq!(
        responds(&one_club, "Axx.Kx.AQxxx.Kxx"),
        bid(2, Strain::Diamonds)
    );
}

/// Opener's rebid after the `1♣ - 1♦` relay (Phase 2.1).
#[test]
fn opener_rebids() {
    const P: Call = Call::Pass;
    let relay = [bid(1, Strain::Clubs), P, bid(1, Strain::Diamonds), P];
    // 19 HCP balanced: the 18–20 notrump rebid.
    assert_eq!(
        responds(&relay, "AQx.KJx.KQx.Axxx"),
        bid(1, Strain::Notrump)
    );
    // 21 HCP, no 5-card major / 6-card minor / 5-5 minors: the artificial 2♦.
    assert_eq!(
        responds(&relay, "AKQ.x.AQxx.AQxxx"),
        bid(2, Strain::Diamonds)
    );
}

/// Responder's second call after opener's minimum relay rebid (Phase 2.2).
#[test]
fn relay_deep_continuations() {
    const P: Call = Call::Pass;
    let c = bid(1, Strain::Clubs);
    let d = bid(1, Strain::Diamonds);
    // After `1♣ - 1♦ - 1♥`: the 5♠/4♥ two-suiter (8 pts) is Reverse Flannery — a
    // raise to 2♥, not a natural spade bid.
    let after_1h = [c, P, d, P, bid(1, Strain::Hearts), P];
    assert_eq!(
        responds(&after_1h, "KQxxx.Kxxx.xx.xx"),
        bid(2, Strain::Hearts)
    );
    // Both minors (5-4), 10 pts: 2♠ (the other major, repurposed).
    assert_eq!(
        responds(&after_1h, "x.xx.KQxxx.AJxx"),
        bid(2, Strain::Spades)
    );
    // After `1♣ - 1♦ - 1♠`: the same two-suiter raises spades (2♠); both-minors is 2♥.
    let after_1s = [c, P, d, P, bid(1, Strain::Spades), P];
    assert_eq!(
        responds(&after_1s, "KQxxx.Kxxx.xx.xx"),
        bid(2, Strain::Spades)
    );
    assert_eq!(
        responds(&after_1s, "x.xx.KQxxx.AJxx"),
        bid(2, Strain::Hearts)
    );
    // After `1♣ - 1♦ - 2♣`: the two-suiter shows as 2♥; an invitational club raise is 2♠.
    let after_2c = [c, P, d, P, bid(2, Strain::Clubs), P];
    assert_eq!(
        responds(&after_2c, "KQxxx.Kxxx.xx.xx"),
        bid(2, Strain::Hearts)
    );
    assert_eq!(
        responds(&after_2c, "Qxx.x.Qxxx.AQxx"),
        bid(2, Strain::Spades)
    );
}

/// Opener's rebid after responder's game-forcing 2♦ (Phase 2.2 increment 2).
#[test]
fn opener_rebids_after_two_diamonds() {
    const P: Call = Call::Pass;
    let a = [bid(1, Strain::Clubs), P, bid(2, Strain::Diamonds), P];
    // Four-card diamond support — raise the known nine-card fit.
    assert_eq!(responds(&a, "Axx.Kx.KJxx.Qxx"), bid(3, Strain::Diamonds));
    // Five clubs, short diamonds — the real second suit.
    assert_eq!(responds(&a, "Ax.Kx.xxx.AQxxx"), bid(3, Strain::Clubs));
    // Balanced 16, both majors stopped — to play.
    assert_eq!(responds(&a, "AQx.KQx.Qxx.Kxxx"), bid(3, Strain::Notrump));
    // Heart stopper only — shown up the line toward 3NT.
    assert_eq!(responds(&a, "xxx.AQx.Kxx.Kxxx"), bid(2, Strain::Hearts));
    // Spade stopper only — the other up-the-line stopper show.
    assert_eq!(responds(&a, "AQx.xxx.Kxx.Kxxx"), bid(2, Strain::Spades));
    // Minimum, no major stopper — the notrump catch-all (never Pass).
    assert_eq!(responds(&a, "xxx.xxx.KQx.AQxx"), bid(2, Strain::Notrump));
}

/// Opener's rebid after responder's invitational-or-better 2♣ (Phase 2.2 inc.2).
#[test]
fn opener_rebids_after_two_clubs() {
    const P: Call = Call::Pass;
    let a = [bid(1, Strain::Clubs), P, bid(2, Strain::Clubs), P];
    // Balanced 16, both majors stopped — accept to game.
    assert_eq!(responds(&a, "AQx.KQx.Qxx.Kxxx"), bid(3, Strain::Notrump));
    // 18 balanced but a major unstopped — a maximum still forces game.
    assert_eq!(responds(&a, "Axx.xxx.AKx.AKxx"), bid(3, Strain::Notrump));
    // Balanced 13, only two clubs — the non-forcing 2NT decline.
    assert_eq!(responds(&a, "AQx.KQx.Qxxx.xx"), bid(2, Strain::Notrump));
    // Minimum with club support — the non-forcing 3♣ decline.
    assert_eq!(responds(&a, "AQx.Kxx.xx.KJxx"), bid(3, Strain::Clubs));
}

/// Responder's authored continuation after opener's rebid (Phase 2.2 inc.2,
/// the redo).  The opener-only version leaned on the floor and measured a
/// loss — the floor dropped the game force and blasted slam.  These lock in
/// the fix: the force is honoured (never passed short of game) and every
/// branch caps at the right game.
#[test]
fn responder_continues_after_opener_rebid() {
    const P: Call = Call::Pass;
    let c = bid(1, Strain::Clubs);
    let d2 = bid(2, Strain::Diamonds);
    let c2 = bid(2, Strain::Clubs);
    let gf = "AQx.Kx.KQxxx.xx"; // a legal game-forcing 2♦ responder
    // GF 2♦: over any descriptive rebid (3♦ support / 2♥ stopper / 2NT),
    // responder names the game — 3NT — and never passes the force.
    for rebid in [
        bid(3, Strain::Diamonds),
        bid(2, Strain::Hearts),
        bid(2, Strain::Notrump),
    ] {
        let auction = [c, P, d2, P, rebid, P];
        assert_eq!(responds(&auction, gf), bid(3, Strain::Notrump));
    }
    // GF 2♦: over opener's own 3NT (balanced 15+ to play), responder passes.
    let gf_3nt = [c, P, d2, P, bid(3, Strain::Notrump), P];
    assert_eq!(responds(&gf_3nt, gf), P);
    // Invite+ 2♣, opener declines 3♣: the game-forcing end drives 3NT …
    let inv_3c = [c, P, c2, P, bid(3, Strain::Clubs), P];
    assert_eq!(
        responds(&inv_3c, "AQx.Kx.Kx.KQxxx"),
        bid(3, Strain::Notrump)
    );
    // … a minimum invite passes the decline.
    assert_eq!(responds(&inv_3c, "Jxx.Qx.Qx.KQxxx"), P);
    // Invite+ 2♣, opener accepts 3NT: responder passes the game.
    let inv_3nt = [c, P, c2, P, bid(3, Strain::Notrump), P];
    assert_eq!(responds(&inv_3nt, "Jxx.Qx.Qx.KQxxx"), P);
}
