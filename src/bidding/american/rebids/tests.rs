use super::*;
use crate::bidding::Bidder;
use contract_bridge::Hand;
use contract_bridge::auction::RelativeVulnerability;

/// The highest-logit call the trie makes for a hand at an auction
///
/// Shared with the per-agreement test modules below this one.
pub(super) fn best(trie: &Trie, auction: &[Call], hand: &str) -> Call {
    let hand: Hand = hand.parse().expect("valid test hand");
    let logits = trie
        .classify(hand, RelativeVulnerability::NONE, auction)
        .expect("trie covers this auction");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("logits array is never empty")
}

/// After `1♦ - 1♥`, a balanced 12–14 with a five-card diamond suit rebids
/// the natural `2♦` by default but `1NT` once `balanced_1nt_rebid` is
/// on — the only shape the knob moves (4333/4432 hold no five-card minor).
#[test]
fn balanced_1nt_rebid_knob_flips_2m_to_1nt() {
    let one_d_one_h = &[
        call(1, Strain::Diamonds),
        Call::Pass,
        call(1, Strain::Hearts),
        Call::Pass,
    ];
    // ♠KQ4 ♥Q3 ♦AK762 ♣853 — 3=2=5=3, 14 HCP, no four-card heart support.
    let hand = "KQ4.Q3.AK762.853";
    let build = |balanced_1nt_rebid: bool| {
        let mut agreements = crate::bidding::agreements::Agreements::default();
        agreements.rebid.balanced_1nt_rebid = balanced_1nt_rebid;
        let mut trie = Trie::new();
        crate::bidding::rows::compile_into(&mut trie, &agreements, &[remaining_rebid_bases()]);
        trie
    };

    assert_eq!(
        best(&build(false), one_d_one_h, hand),
        call(2, Strain::Diamonds)
    );

    // The shipped default.
    let on = build(true);
    assert_eq!(best(&on, one_d_one_h, hand), call(1, Strain::Notrump));
}

/// After `1m - 1♠`, a 5m-4♥ minimum rebids `2m` by default but `1NT` once
/// `unbalanced_1nt_rebid` is on, singleton spade or not; six of the minor
/// still rebids `2m`.
#[test]
fn unbalanced_1nt_rebid_knob_keeps_2m_for_six() {
    let auction = |minor| {
        [
            call(1, minor),
            Call::Pass,
            call(1, Strain::Spades),
            Call::Pass,
        ]
    };
    let build = |on: bool| {
        let mut agreements = crate::bidding::agreements::Agreements::default();
        agreements.rebid.unbalanced_1nt_rebid = on;
        let mut trie = Trie::new();
        crate::bidding::rows::compile_into(&mut trie, &agreements, &[remaining_rebid_bases()]);
        trie
    };
    let (off, on) = (build(false), build(true));
    let (d, c) = (auction(Strain::Diamonds), auction(Strain::Clubs));
    for (auction, hand, minor) in [
        (&d, "Q4.KJ86.AQ752.83", Strain::Diamonds), // 2=4=5=2, 12 HCP
        (&d, "4.KJ86.AQ752.K83", Strain::Diamonds), // 1=4=5=3, 13 HCP
        (&c, "Q4.KJ86.83.AQ752", Strain::Clubs),    // 2=4=2=5, 12 HCP
    ] {
        assert_eq!(best(&off, auction, hand), call(2, minor), "{hand}");
        assert_eq!(best(&on, auction, hand), call(1, Strain::Notrump), "{hand}");
    }
    // ♠4 ♥KJ86 ♦AQ7652 ♣83 — six diamonds keep the natural rebid.
    assert_eq!(best(&on, &d, "4.KJ86.AQ7652.83"), call(2, Strain::Diamonds));
}

/// After `1♦ - 1♠`, a 5♦-4♣ minimum rebids `2♦` by default but the new
/// lower suit `2♣` once `one_diamond_two_clubs` is on; six diamonds still
/// rebid `2♦`.
#[test]
fn one_diamond_two_clubs_knob_shows_the_second_suit() {
    let one_d_one_s = &[
        call(1, Strain::Diamonds),
        Call::Pass,
        call(1, Strain::Spades),
        Call::Pass,
    ];
    let build = |on: bool| {
        let mut agreements = crate::bidding::agreements::Agreements::default();
        agreements.rebid.one_diamond_two_clubs = on;
        let mut trie = Trie::new();
        crate::bidding::rows::compile_into(&mut trie, &agreements, &[remaining_rebid_bases()]);
        trie
    };
    let (off, on) = (build(false), build(true));
    // ♠83 ♥K4 ♦AQ762 ♣KJ84 — 2=2=5=4, 13 HCP.
    let five_four = "83.K4.AQ762.KJ84";
    assert_eq!(
        best(&off, one_d_one_s, five_four),
        call(2, Strain::Diamonds)
    );
    assert_eq!(best(&on, one_d_one_s, five_four), call(2, Strain::Clubs));
    // ♠8 ♥K4 ♦AQ7632 ♣KJ84 — 1=2=6=4.
    let six_four = "8.K4.AQ7632.KJ84";
    assert_eq!(best(&on, one_d_one_s, six_four), call(2, Strain::Diamonds));
}

/// `passed_hand_major_pass`: opener passes a passed hand's `1M` response on a
/// balanced minimum with exactly three-card support — and only there.
#[test]
fn passed_hand_major_pass_knob_passes_the_balanced_minimum() {
    let build = |ceiling: Option<u8>| {
        let mut agreements = crate::bidding::agreements::Agreements::default();
        agreements.rebid.passed_hand_major_pass = ceiling;
        let mut trie = Trie::new();
        crate::bidding::rows::compile_into(
            &mut trie,
            &agreements,
            &[one_heart_one_spade_rebid(), remaining_rebid_bases()],
        );
        trie
    };
    let (off, on) = (build(None), build(Some(13)));
    let lane = |passes: usize, opening: Strain, major: Strain| {
        let mut auction = vec![Call::Pass; passes];
        auction.extend([call(1, opening), Call::Pass, call(1, major), Call::Pass]);
        auction
    };
    let notrump = call(1, Strain::Notrump);
    // ♠K84 ♥Q3 ♦A762 ♣KJ53 — 3=2=4=4, 13 HCP.
    let minimum = "K84.Q3.A762.KJ53";
    for passes in 0..=3 {
        let auction = lane(passes, Strain::Clubs, Strain::Spades);
        assert_eq!(best(&off, &auction, minimum), notrump);
        // Partner has passed only behind two or three leading passes.
        let expected = if passes >= 2 { Call::Pass } else { notrump };
        assert_eq!(best(&on, &auction, minimum), expected, "{passes} passes");
    }
    let third = lane(2, Strain::Clubs, Strain::Spades);
    // ♠K84 ♥K3 ♦A762 ♣KJ53 — 14 HCP is over the ceiling, until it is raised.
    assert_eq!(best(&on, &third, "K84.K3.A762.KJ53"), notrump);
    assert_eq!(
        best(&build(Some(14)), &third, "K84.K3.A762.KJ53"),
        Call::Pass
    );
    // A doubleton still rebids 1NT, four-card support still raises, and an
    // unbalanced hand keeps its natural rebid.
    assert_eq!(best(&on, &third, "K8.Q43.A762.KJ53"), notrump);
    assert_eq!(
        best(&on, &third, "K842.Q3.A76.KJ53"),
        call(2, Strain::Spades)
    );
    assert_eq!(
        best(&on, &third, "K84.3.A762.KJ853"),
        call(2, Strain::Clubs)
    );
    // Four spades over 1♥ still go up the line.
    let over_one_heart = lane(3, Strain::Diamonds, Strain::Hearts);
    assert_eq!(
        best(&on, &over_one_heart, "KJ53.Q84.A762.K3"),
        call(1, Strain::Spades)
    );
    assert_eq!(best(&on, &over_one_heart, "K53.Q84.A762.KJ3"), Call::Pass);
    // 1♥ - 1♠ on a 3=5=3=2.
    let majors = lane(2, Strain::Hearts, Strain::Spades);
    assert_eq!(best(&off, &majors, "K84.KJ853.A76.Q3"), notrump);
    assert_eq!(best(&on, &majors, "K84.KJ853.A76.Q3"), Call::Pass);
}

/// After `1♠ - 1NT`, a 5-3-3-2 minimum rebids `2♠` by default but its
/// three-card minor once `forcing_nt_three_card_minor` is on — clubs with 3-3.
#[test]
fn forcing_nt_three_card_minor_knob_bids_the_minor() {
    let one_s_one_nt = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(1, Strain::Notrump),
        Call::Pass,
    ];
    let build = |on: bool| {
        let mut agreements = crate::bidding::agreements::Agreements::default();
        agreements.decision.reading.forcing_nt_three_card_minor = on;
        let mut trie = Trie::new();
        crate::bidding::rows::compile_into(&mut trie, &agreements, &[remaining_rebid_bases()]);
        trie
    };
    let (off, on) = (build(false), build(true));
    // ♠AQ874 ♥K6 ♦Q73 ♣J42 — 5=2=3=3, 12 HCP.
    let three_three = "AQ874.K6.Q73.J42";
    assert_eq!(
        best(&off, one_s_one_nt, three_three),
        call(2, Strain::Spades)
    );
    assert_eq!(best(&on, one_s_one_nt, three_three), call(2, Strain::Clubs));
    // ♠AQ874 ♥K62 ♦Q73 ♣J4 — 5=3=3=2: the three-card diamonds.
    let diamonds = "AQ874.K62.Q73.J4";
    assert_eq!(best(&on, one_s_one_nt, diamonds), call(2, Strain::Diamonds));
    // ♠AQ874 ♥K6 ♦Q732 ♣J4 — a four-card minor keeps its natural rebid.
    let four = "AQ874.K6.Q732.J4";
    assert_eq!(best(&on, one_s_one_nt, four), call(2, Strain::Diamonds));
}

/// After `1♦ - 1♠ - 2♣`, a weak responder with diamonds at least as long as
/// clubs gives preference to `2♦`; longer clubs reject to the floor.
#[test]
fn one_diamond_two_clubs_preference_returns_to_diamonds() {
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.rebid.one_diamond_two_clubs = true;
    let mut trie = Trie::new();
    crate::bidding::rows::compile_into(
        &mut trie,
        &agreements,
        &[one_diamond_two_clubs_preference()],
    );
    let auction = &[
        call(1, Strain::Diamonds),
        Call::Pass,
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    // ♠KJ643 ♥Q852 ♦74 ♣96 — 5=4=2=2, 6 HCP.
    assert_eq!(
        best(&trie, auction, "KJ643.Q852.74.96"),
        call(2, Strain::Diamonds)
    );
    // ♠KJ643 ♥Q852 ♦7 ♣964 — longer clubs: no book mass, the floor's call.
    let hand: Hand = "KJ643.Q852.7.964".parse().unwrap();
    assert!(
        trie.classify(hand, RelativeVulnerability::NONE, auction)
            .is_none_or(|l| { (&l.0).into_iter().all(|(_, &w)| w == f32::NEG_INFINITY) })
    );
}

/// `forcing_notrump_suit_invite`: after `1♠ - 1NT - 2♣`, 10–12 with six hearts
/// jumps to `3♥` (it passed before), and opener bids `4♥` on a doubleton with
/// 14+, `3NT` on a singleton with 15+, and passes the minimum.
#[test]
fn forcing_notrump_suit_invite_jumps_in_the_six_card_suit() {
    let a = |calls: &[Call]| {
        let mut auction = vec![call(1, Strain::Spades), Call::Pass];
        for &c in calls {
            auction.extend([c, Call::Pass]);
        }
        auction
    };
    let build = |on: bool| {
        let mut agreements = crate::bidding::agreements::Agreements::default();
        agreements.rebid.forcing_notrump_suit_invite = on;
        // Isolate the invite: over `2♥` the heart raise would bid `3♥` too.
        agreements.rebid.forcing_notrump_heart_raise = false;
        let mut trie = Trie::new();
        crate::bidding::rows::compile_into(
            &mut trie,
            &agreements,
            &[forcing_notrump_continuations()],
        );
        trie
    };
    let (off, on) = (build(false), build(true));
    let nt = call(1, Strain::Notrump);
    let two_clubs = a(&[nt, call(2, Strain::Clubs)]);
    // ♠8 ♥K96543 ♦AQ98 ♣J5 — 10 HCP, six hearts.
    let invite = "8.K96543.AQ98.J5";
    assert_eq!(best(&off, &two_clubs, invite), Call::Pass);
    assert_eq!(best(&on, &two_clubs, invite), call(3, Strain::Hearts));
    // A six-card minor stays without an invite (it measured a loss).
    let minor = "8.K9.AQ9854.J53";
    assert_ne!(best(&on, &two_clubs, minor), call(3, Strain::Diamonds));
    // Over opener's own `2♥` the jump is a raise, not this hand.
    let two_hearts = a(&[nt, call(2, Strain::Hearts)]);
    assert_ne!(best(&on, &two_hearts, invite), call(3, Strain::Hearts));

    let three_hearts = a(&[nt, call(2, Strain::Clubs), call(3, Strain::Hearts)]);
    for (hand, answer) in [
        ("AKJ84.Q2.K3.QJ54", call(4, Strain::Hearts)), // doubleton, 16
        ("AQJ84.2.KQ3.KJ54", call(3, Strain::Notrump)), // singleton, 16
        ("AJ984.2.K53.AQ54", Call::Pass),              // singleton, 12
    ] {
        assert_eq!(best(&on, &three_hearts, hand), answer, "{hand}");
    }
}

/// `forcing_notrump_doubleton_raise`: after `1♠ - 1NT - 2♠`, 10–12 with a
/// doubleton raises to `3♠` (the `2NT` invite before); over `2♣` the same hand
/// still invites in notrump, and a 10-count with a doubleton raises too.
#[test]
fn forcing_notrump_doubleton_raise_over_the_six_card_rebid() {
    let a = |calls: &[Call]| {
        let mut auction = vec![call(1, Strain::Spades), Call::Pass];
        for &c in calls {
            auction.extend([c, Call::Pass]);
        }
        auction
    };
    let build = |on: bool| {
        let mut agreements = crate::bidding::agreements::Agreements::default();
        agreements.rebid.forcing_notrump_doubleton_raise = on;
        let mut trie = Trie::new();
        crate::bidding::rows::compile_into(
            &mut trie,
            &agreements,
            &[forcing_notrump_continuations()],
        );
        trie
    };
    let (off, on) = (build(false), build(true));
    let nt = call(1, Strain::Notrump);
    let two_spades = a(&[nt, call(2, Strain::Spades)]);
    // ♠97 ♥QJ52 ♦A8753 ♣A7 — 11 HCP, two spades.
    let hand = "97.QJ52.A8753.A7";
    assert_eq!(best(&off, &two_spades, hand), call(2, Strain::Notrump));
    assert_eq!(best(&on, &two_spades, hand), call(3, Strain::Spades));
    // ♠T6 ♥KT65 ♦Q32 ♣AJ86 — 10 HCP: no invite before, raises now.
    let ten = "T6.KT65.Q32.AJ86";
    assert_ne!(best(&off, &two_spades, ten), call(3, Strain::Spades));
    assert_eq!(best(&on, &two_spades, ten), call(3, Strain::Spades));
    // Over `2♣` a doubleton is no fit: still the notrump invite.
    let two_clubs = a(&[nt, call(2, Strain::Clubs)]);
    assert_eq!(best(&on, &two_clubs, hand), call(2, Strain::Notrump));
}

/// `forcing_notrump_heart_raise`: after `1♠ - 1NT - 2♥`, 10–12 with four hearts
/// raises to `3♥` (passed or bid `2NT` before), a weak doubleton spade with four
/// hearts passes rather than prefer `2♠`, and opener accepts on 14+ points.
#[test]
fn forcing_notrump_heart_raise_over_two_hearts() {
    let a = |calls: &[Call]| {
        let mut auction = vec![call(1, Strain::Spades), Call::Pass];
        for &c in calls {
            auction.extend([c, Call::Pass]);
        }
        auction
    };
    let build = |on: bool| {
        let mut agreements = crate::bidding::agreements::Agreements::default();
        agreements.rebid.forcing_notrump_heart_raise = on;
        let mut trie = Trie::new();
        crate::bidding::rows::compile_into(
            &mut trie,
            &agreements,
            &[forcing_notrump_continuations()],
        );
        trie
    };
    let (off, on) = (build(false), build(true));
    let nt = call(1, Strain::Notrump);
    let two_hearts = a(&[nt, call(2, Strain::Hearts)]);
    // ♠7 ♥KJ54 ♦A8753 ♣Q62 — 10 HCP, four hearts.
    let ten = "7.KJ54.A8753.Q62";
    assert_eq!(best(&off, &two_hearts, ten), Call::Pass);
    assert_eq!(best(&on, &two_hearts, ten), call(3, Strain::Hearts));
    // ♠97 ♥QJ54 ♦K875 ♣Q62 — 8 HCP: pass, not the false preference.
    let weak = "97.QJ54.K875.Q62";
    assert_eq!(best(&off, &two_hearts, weak), call(2, Strain::Spades));
    assert_eq!(best(&on, &two_hearts, weak), Call::Pass);
    // Opener ♠AKJ75 ♥AQ63 ♦K4 ♣82 (17 HCP) accepts.
    let raised = a(&[nt, call(2, Strain::Hearts), call(3, Strain::Hearts)]);
    assert_eq!(
        best(&on, &raised, "AKJ75.AQ63.K4.82"),
        call(4, Strain::Hearts)
    );
}
