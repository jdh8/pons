use super::super::register;
use super::super::tests::best;
use super::*;
use crate::bidding::Trie;
use crate::bidding::agreements::Agreements;

/// The rebid Trie with the natural jump shifts live (Meckstroth off).
fn jump_shift_trie() -> Trie {
    let mut agreements = Agreements::default();
    agreements.rebid.meckstroth_adjunct = false;
    agreements.rebid.forcing_nt_jump_shifts = true;
    let mut trie = Trie::new();
    register(&mut trie, &agreements);
    trie
}

const P: Call = Call::Pass;
const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

const AFTER_1S_1NT: &[Call] = &[bid(1, Strain::Spades), P, bid(1, Strain::Notrump), P];
const AFTER_1H_1NT: &[Call] = &[bid(1, Strain::Hearts), P, bid(1, Strain::Notrump), P];

#[test]
fn opener_picks_the_rung_by_shape() {
    let trie = jump_shift_trie();
    // 5♠4♣ 19 HCP: the natural 3♣ jump shift.
    assert_eq!(
        best(&trie, AFTER_1S_1NT, "AKJ98.A3.K2.AQ76"),
        bid(3, Strain::Clubs)
    );
    // 5-5 majors 18+: the higher suit first — 3♥ is the jump shift, not the
    // 15–17 two-suiter.
    assert_eq!(
        best(&trie, AFTER_1S_1NT, "AKJ98.AKQ76.3.K2"),
        bid(3, Strain::Hearts)
    );
    // 6♠ 18 HCP no side suit: the artificial 3NT.
    assert_eq!(
        best(&trie, AFTER_1S_1NT, "AKQJ98.A3.K42.Q2"),
        bid(3, Strain::Notrump)
    );
    // 5-5 majors 15–17: 2♥ (the displaced two-suiter), then 3♥ over the
    // preference; responder accepts on the two-suiter's own table.
    assert_eq!(
        best(&trie, AFTER_1S_1NT, "AKJ98.AQ976.3.32"),
        bid(2, Strain::Hearts)
    );
    let pref: Vec<Call> = [
        AFTER_1S_1NT,
        &[bid(2, Strain::Hearts), P, bid(2, Strain::Spades), P],
    ]
    .concat();
    assert_eq!(
        best(&trie, &pref, "AKJ98.AQ976.3.32"),
        bid(3, Strain::Hearts)
    );
    let invite: Vec<Call> = [&pref[..], &[bid(3, Strain::Hearts), P]].concat();
    assert_eq!(
        best(&trie, &invite, "Q32.K54.9652.K43"),
        bid(4, Strain::Spades)
    );
    // Over the 2NT invite the 5-5 bids 4♥; a 5-4 maximum still takes 3NT.
    let nt_invite: Vec<Call> = [
        AFTER_1S_1NT,
        &[bid(2, Strain::Hearts), P, bid(2, Strain::Notrump), P],
    ]
    .concat();
    assert_eq!(
        best(&trie, &nt_invite, "AKJ98.AQ976.3.32"),
        bid(4, Strain::Hearts)
    );
    assert_eq!(
        best(&trie, &nt_invite, "AKJ98.AQ97.K3.32"),
        bid(3, Strain::Notrump)
    );
    // 6♠4♦ 18+: a side suit prefers the jump shift.
    assert_eq!(
        best(&trie, AFTER_1S_1NT, "AKQJ98.3.AK42.Q2"),
        bid(3, Strain::Diamonds)
    );
    // 6♠ with 16–17: the 3♠ jump-rebid keeps its band.
    assert_eq!(
        best(&trie, AFTER_1S_1NT, "AKQJ98.3.K42.Q32"),
        bid(3, Strain::Spades)
    );
    // 5332 18 HCP: the natural 2NT.
    assert_eq!(
        best(&trie, AFTER_1S_1NT, "AKJ98.A32.KQ2.K2"),
        bid(2, Strain::Notrump)
    );
    // 5332 21 HCP: 2NT is uncapped.
    assert_eq!(
        best(&trie, AFTER_1S_1NT, "AKQ98.AK2.KQ2.K2"),
        bid(2, Strain::Notrump)
    );
    // Over 1♥ the 3♠ jump shift exists (responder's 1NT denied four spades).
    assert_eq!(
        best(&trie, AFTER_1H_1NT, "AKJ8.AKJ98.3.A76"),
        bid(3, Strain::Spades)
    );
    // A 5♥4♠ 15–17 still takes the two-suiter reverse.
    assert_eq!(
        best(&trie, AFTER_1H_1NT, "AKJ8.AKJ98.3.876"),
        bid(2, Strain::Spades)
    );
}

#[test]
fn responder_places_over_the_jump_shift() {
    let trie = jump_shift_trie();
    let after_3c: Vec<Call> = [AFTER_1S_1NT, &[bid(3, Strain::Clubs), P]].concat();
    let then = |prefix: &[Call], c: Call| -> Vec<Call> { [prefix, &[c, P]].concat() };
    // Three spades, 10+: the 3♠ slam-try; opener asks keycards on 20+.
    assert_eq!(
        best(&trie, &after_3c, "Q32.K54.A65.Q432"),
        bid(3, Strain::Spades)
    );
    let after_try = then(&after_3c, bid(3, Strain::Spades));
    assert_eq!(
        best(&trie, &after_try, "AKJ98.A3.Q2.KJ76"),
        bid(4, Strain::Spades)
    );
    assert_eq!(
        best(&trie, &after_try, "AKJ98.A3.AK.AQ76"),
        bid(4, Strain::Notrump)
    );
    // Three spades, minimum: the 5-3 game.
    assert_eq!(
        best(&trie, &after_3c, "Q32.J54.965.J432"),
        bid(4, Strain::Spades)
    );
    // Five hearts: natural 3♥; opener raises with three.
    assert_eq!(
        best(&trie, &after_3c, "Q3.J8542.965.J43"),
        bid(3, Strain::Hearts)
    );
    let after_3h_resp = then(&after_3c, bid(3, Strain::Hearts));
    assert_eq!(
        best(&trie, &after_3h_resp, "AKJ98.A63.K.AQ76"),
        bid(4, Strain::Hearts)
    );
    // Singleton: 3NT; opener pulls to 4♠ on a sixth spade, else passes.
    assert_eq!(
        best(&trie, &after_3c, "3.J432.9652.J432"),
        bid(3, Strain::Notrump)
    );
    let after_3nt = then(&after_3c, bid(3, Strain::Notrump));
    assert_eq!(
        best(&trie, &after_3nt, "AKJ987.A.K2.AQ76"),
        bid(4, Strain::Spades)
    );
    assert_eq!(best(&trie, &after_3nt, "AKJ98.A3.K2.AQ76"), P);
    // The natural 2NT carries the same game-forcing round.
    let after_2nt = then(AFTER_1S_1NT, bid(2, Strain::Notrump));
    assert_eq!(
        best(&trie, &after_2nt, "Q32.K54.A65.Q432"),
        bid(3, Strain::Spades)
    );
    assert_eq!(
        best(&trie, &after_2nt, "Q3.J8542.965.J43"),
        bid(3, Strain::Hearts)
    );
    assert_eq!(
        best(&trie, &after_2nt, "3.J432.9652.J432"),
        bid(3, Strain::Notrump)
    );
    // 1♠ - 1NT - 3♥: the 4-4 heart game.
    let after_3h: Vec<Call> = [AFTER_1S_1NT, &[bid(3, Strain::Hearts), P]].concat();
    assert_eq!(
        best(&trie, &after_3h, "3.J542.9652.J432"),
        bid(4, Strain::Hearts)
    );
}

#[test]
fn heart_fit_slam_try_over_the_spade_jump_shift_is_four_clubs() {
    let trie = jump_shift_trie();
    let after_3s: Vec<Call> = [AFTER_1H_1NT, &[bid(3, Strain::Spades), P]].concat();
    assert_eq!(
        best(&trie, &after_3s, "32.K54.A65.KJ32"),
        bid(4, Strain::Clubs)
    );
    let after_4c: Vec<Call> = [&after_3s[..], &[bid(4, Strain::Clubs), P]].concat();
    assert_eq!(
        best(&trie, &after_4c, "AKJ8.AKJ98.A.A76"),
        bid(4, Strain::Notrump)
    );
    assert_eq!(
        best(&trie, &after_4c, "AQJ8.AKJ98.3.976"),
        bid(4, Strain::Hearts)
    );
}

#[test]
fn responder_places_over_the_long_major_3nt() {
    let trie = jump_shift_trie();
    let after_3nt: Vec<Call> = [AFTER_1S_1NT, &[bid(3, Strain::Notrump), P]].concat();
    // Singleton spade, six hearts: play 4♥.
    assert_eq!(
        best(&trie, &after_3nt, "3.KJ8542.965.J43"),
        bid(4, Strain::Hearts)
    );
    // Over the 4♠ correction opener asks keycards only on 21+.
    let corrected: Vec<Call> = [&after_3nt[..], &[bid(4, Strain::Spades), P]].concat();
    assert_eq!(
        best(&trie, &corrected, "AKQJ98.A3.AK2.K2"),
        bid(4, Strain::Notrump)
    );
    assert_eq!(best(&trie, &corrected, "AKQJ98.A3.K42.Q2"), P);
    assert_eq!(
        best(&trie, &after_3nt, "Q3.K54.A65.Q432"),
        bid(4, Strain::Notrump)
    );
    assert_eq!(
        best(&trie, &after_3nt, "Q3.J54.9652.J432"),
        bid(4, Strain::Spades)
    );
    assert_eq!(best(&trie, &after_3nt, "3.J542.9652.J432"), P);
}

#[test]
fn knob_is_inert_under_meckstroth_and_off_by_default() {
    // Meckstroth on (the default) wins the seam whatever the jump-shift knob says.
    let mut agreements = Agreements::default();
    agreements.rebid.forcing_nt_jump_shifts = true;
    let mut trie = Trie::new();
    register(&mut trie, &agreements);
    assert_eq!(
        best(&trie, AFTER_1S_1NT, "AKJ98.A3.K2.AQ76"),
        bid(2, Strain::Notrump)
    );
    assert!(!forcing_nt_jump_shifts_on(&Agreements::default().rebid));
}

#[test]
fn debug_dump() {
    use crate::bidding::Bidder;
    use contract_bridge::Hand;
    use contract_bridge::auction::RelativeVulnerability;
    let trie = jump_shift_trie();
    for (auction, hand) in [
        (
            [AFTER_1H_1NT, &[bid(3, Strain::Spades), P]].concat(),
            "32.K54.A65.KJ32",
        ),
        (
            [
                AFTER_1S_1NT,
                &[
                    bid(2, Strain::Hearts),
                    P,
                    bid(2, Strain::Spades),
                    P,
                    bid(3, Strain::Hearts),
                    P,
                ],
            ]
            .concat(),
            "Q32.K54.9652.K43",
        ),
    ] {
        let h: Hand = hand.parse().unwrap();
        let logits = trie
            .classify(h, RelativeVulnerability::NONE, &auction)
            .unwrap();
        let finite: Vec<_> = (&logits.0)
            .into_iter()
            .filter(|(_, l)| l.is_finite())
            .collect();
        eprintln!(
            "{hand} points={} -> {finite:?}",
            crate::bidding::constraint::point_count(h)
        );
    }
}
