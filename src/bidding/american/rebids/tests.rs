use super::*;
use crate::bidding::System;
use contract_bridge::Hand;
use contract_bridge::auction::RelativeVulnerability;

/// Build a Trie with the major-rebid-tails adjunct on but
/// fourth-suit-forcing off, then restore both knobs to their (on)
/// defaults (mirrors `slam::tests::rkcb_trie`).
fn tails_trie() -> Trie {
    set_major_rebid_tails(true);
    set_fourth_suit_forcing(false);
    let mut trie = Trie::new();
    register_major_rebid_tails(&mut trie);
    set_fourth_suit_forcing(true);
    trie
}

/// Build a Trie with both the major-rebid-tails and fourth-suit-forcing
/// knobs on (the shipped defaults).
fn fsf_trie() -> Trie {
    set_major_rebid_tails(true);
    set_fourth_suit_forcing(true);
    let mut trie = Trie::new();
    register_major_rebid_tails(&mut trie);
    trie
}

/// Build the full rebid Trie with the opener extras ladder on (the shipped
/// default).
fn ladder_trie() -> Trie {
    set_opener_extras_ladder(true);
    let mut trie = Trie::new();
    register(&mut trie);
    trie
}

/// The raw table auction `[1♦, P, 1♠, P]` (opener to rebid).
const AFTER_1D_1S: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Diamonds)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
];

#[test]
fn opener_extras_ladder_shows_strength() {
    let trie = ladder_trie();
    let b = |hand| best(&trie, AFTER_1D_1S, hand);
    // Self-sufficient 6+ diamonds, 16 HCP → jump-rebid 3♦.
    assert_eq!(
        b("653.K3.AKQT854.A"),
        Call::Bid(Bid::new(3, Strain::Diamonds))
    );
    // 5♦ 4♥, 18 HCP → jump-shift 3♥ (game-forcing two-suiter).
    assert_eq!(
        b("T64.AJ86.AKQ95.A"),
        Call::Bid(Bid::new(3, Strain::Hearts))
    );
    // 5-5 in the minors, 18 HCP → jump-shift 3♣.
    assert_eq!(b("K.62.AQJ94.AKJ85"), Call::Bid(Bid::new(3, Strain::Clubs)));
    // A dead minimum still takes the natural 2♦ rebid.
    assert_eq!(
        b("K54.Q3.KJ8542.32"),
        Call::Bid(Bid::new(2, Strain::Diamonds))
    );
}

#[test]
fn opener_extras_ladder_reverts_when_off() {
    set_opener_extras_ladder(false);
    let mut trie = Trie::new();
    register(&mut trie);
    set_opener_extras_ladder(true);
    // Knob off: the 16-count monster reverts to the minimum 2♦ rebid.
    assert_eq!(
        best(&trie, AFTER_1D_1S, "653.K3.AKQT854.A"),
        Call::Bid(Bid::new(2, Strain::Diamonds))
    );
}

/// Build the full rebid Trie with opener's major jump-rebid rung on (the
/// shipped default).
fn major_jump_trie() -> Trie {
    set_opener_major_jump_rebid(true);
    let mut trie = Trie::new();
    register(&mut trie);
    trie
}

/// The raw table auction `[1♥, P, 1♠, P]` (opener to rebid).
const AFTER_1H_1S: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Hearts)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
];

/// The raw table auction `[1♠, P, 1NT, P]` (opener rebids over forcing 1NT).
const AFTER_1S_1NT: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Notrump)),
    Call::Pass,
];

#[test]
fn opener_major_jump_rebid_shows_strength() {
    let trie = major_jump_trie();
    // 6+ hearts, 16 HCP, no spade fit → jump-rebid 3♥.
    assert_eq!(
        best(&trie, AFTER_1H_1S, "3.AKQJ72.KQ5.J54"),
        Call::Bid(Bid::new(3, Strain::Hearts))
    );
    // A minimum 6-card heart hand still takes the natural 2♥.
    assert_eq!(
        best(&trie, AFTER_1H_1S, "A2.KQ9872.Q43.J5"),
        Call::Bid(Bid::new(2, Strain::Hearts))
    );
    // The forcing-1NT node carries the same rung: 1♠ – 1NT – 3♠.
    assert_eq!(
        best(&trie, AFTER_1S_1NT, "AKQJ72.3.KQ5.J54"),
        Call::Bid(Bid::new(3, Strain::Spades))
    );
}

#[test]
fn opener_major_jump_rebid_reverts_when_off() {
    // Knob off: the 16-count 6-heart hand reverts to the minimum 2♥ rebid.
    set_opener_major_jump_rebid(false);
    let mut trie = Trie::new();
    register(&mut trie);
    set_opener_major_jump_rebid(true);
    assert_eq!(
        best(&trie, AFTER_1H_1S, "3.AKQJ72.KQ5.J54"),
        Call::Bid(Bid::new(2, Strain::Hearts))
    );
}

/// `[1♥, P, 1♠, P, 3♥, P]` — responder to act over opener's jump-rebid.
const AFTER_1H_1S_3H: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Hearts)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
    Call::Bid(Bid::new(3, Strain::Hearts)),
    Call::Pass,
];

/// `[1♠, P, 1NT, P, 3♠, P]` — responder to act over opener's jump-rebid.
const AFTER_1S_1NT_3S: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Notrump)),
    Call::Pass,
    Call::Bid(Bid::new(3, Strain::Spades)),
    Call::Pass,
];

#[test]
fn responder_accepts_major_jump_rebid() {
    let trie = major_jump_trie();
    // Fit (3 hearts) + values → raise to game 4♥.
    assert_eq!(
        best(&trie, AFTER_1H_1S_3H, "KQ85.K76.542.J43"),
        Call::Bid(Bid::new(4, Strain::Hearts))
    );
    // No heart fit + values → notrump game 3NT.
    assert_eq!(
        best(&trie, AFTER_1H_1S_3H, "KQ85.6.KJ43.Q642"),
        Call::Bid(Bid::new(3, Strain::Notrump))
    );
    // Minimum → pass the invitational jump, play 3♥.
    assert_eq!(best(&trie, AFTER_1H_1S_3H, "Q985.42.J8532.K4"), Call::Pass);
    // Forcing-1NT node: a doubleton spade fit (8 cards) + values → 4♠.
    assert_eq!(
        best(&trie, AFTER_1S_1NT_3S, "87.KQ86.KJ43.T92"),
        Call::Bid(Bid::new(4, Strain::Spades))
    );
}

/// The highest-logit call the trie makes for a hand at an auction
fn best(trie: &Trie, auction: &[Call], hand: &str) -> Call {
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

/// The raw table auction `[1♥, P, 1♠, P, 2♠, P]` (opener in seat 1).
const AFTER_2S: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Hearts)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
    Call::Bid(Bid::new(2, Strain::Spades)),
    Call::Pass,
];

/// The raw table auction `[1♥, P, 1♠, P, 2♥, P]`.
const AFTER_2H: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Hearts)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
    Call::Bid(Bid::new(2, Strain::Hearts)),
    Call::Pass,
];

/// The raw table auction `[1♥, P, 1♠, P, 2♥, P, 2NT, P]`.
const AFTER_2H_2NT: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Hearts)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
    Call::Bid(Bid::new(2, Strain::Hearts)),
    Call::Pass,
    Call::Bid(Bid::new(2, Strain::Notrump)),
    Call::Pass,
];

/// The raw table auction `[1♥, P, 1♠, P, 2♣, P]`.
const AFTER_2C: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Hearts)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
    Call::Bid(Bid::new(2, Strain::Clubs)),
    Call::Pass,
];

/// The raw table auction `[1♥, P, 1♠, P, 2♣, P, 2♦, P]`
/// (fourth-suit-forcing).
const AFTER_2C_2D: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Hearts)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
    Call::Bid(Bid::new(2, Strain::Clubs)),
    Call::Pass,
    Call::Bid(Bid::new(2, Strain::Diamonds)),
    Call::Pass,
];

/// The raw table auction `[1♥, P, 1♠, P, 2♣, P, 2♦, P, 2♠, P]`.
const AFTER_2C_2D_2S: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Hearts)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
    Call::Bid(Bid::new(2, Strain::Clubs)),
    Call::Pass,
    Call::Bid(Bid::new(2, Strain::Diamonds)),
    Call::Pass,
    Call::Bid(Bid::new(2, Strain::Spades)),
    Call::Pass,
];

/// The raw table auction `[1♥, P, 1♠, P, 2♣, P, 2♦, P, 2NT, P]`.
const AFTER_2C_2D_2NT: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Hearts)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
    Call::Bid(Bid::new(2, Strain::Clubs)),
    Call::Pass,
    Call::Bid(Bid::new(2, Strain::Diamonds)),
    Call::Pass,
    Call::Bid(Bid::new(2, Strain::Notrump)),
    Call::Pass,
];

/// The off state: `register` inserts nothing with the knob off.
#[test]
fn off_state_inserts_nothing() {
    set_major_rebid_tails(false);
    let mut trie = Trie::new();
    register_major_rebid_tails(&mut trie);
    set_major_rebid_tails(true); // restore the shipped default
    let hand: Hand = "K432.AQ5.432.Q32".parse().expect("valid test hand");
    assert!(
        trie.classify(hand, RelativeVulnerability::NONE, AFTER_2S)
            .is_none(),
        "the adjunct must insert zero nodes while off"
    );
}

/// C1: responder's second call after opener's `2♠` raise picks by points.
#[test]
fn spade_raise_responder_picks_by_points() {
    let trie = tails_trie();

    // A432.KQ5.K54.J32 — 13 points, balanced (4333) -> accept to game.
    assert_eq!(
        best(&trie, AFTER_2S, "A432.KQ5.K54.J32"),
        Call::Bid(Bid::new(4, Strain::Spades)),
        "13 points -> 4♠"
    );
    // A432.K542.Q54.J3 — 10 points (4-4-3-2) -> invitational raise.  A
    // flat 4333 10-count reads 9 on the shipped scale and passes.
    assert_eq!(
        best(&trie, AFTER_2S, "A432.K542.Q54.J3"),
        Call::Bid(Bid::new(3, Strain::Spades)),
        "10 points -> 3♠"
    );
    // A432.432.Q54.J32 — 7 points, balanced -> pass.
    assert_eq!(
        best(&trie, AFTER_2S, "A432.432.Q54.J32"),
        Call::Pass,
        "7 points -> pass"
    );
}

/// C4: responder's second call after opener's `2♥` rebid prefers the fit.
#[test]
fn heart_rebid_responder_prefers_the_fit() {
    let trie = tails_trie();

    // K987.AQ.9876.Q32 — 11 points, 2 hearts -> invite in hearts, beats 2NT.
    assert_eq!(
        best(&trie, AFTER_2H, "K987.AQ.9876.Q32"),
        Call::Bid(Bid::new(3, Strain::Hearts)),
        "11 points, 2 hearts -> 3♥ beats 2NT"
    );
    // AJ98.K.7654.QJ87 — 11 points, singleton heart -> the notrump invite.
    assert_eq!(
        best(&trie, AFTER_2H, "AJ98.K.7654.QJ87"),
        Call::Bid(Bid::new(2, Strain::Notrump)),
        "11 points, 1 heart -> 2NT"
    );
}

/// C6: opener's call over responder's `2NT` invite goes by raw HCP.
#[test]
fn opener_answers_the_heart_invite_by_hcp() {
    let trie = tails_trie();

    // AK32.KQ32.A32.32 — 16 HCP -> accept with 3NT.
    assert_eq!(
        best(&trie, AFTER_2H_2NT, "AK32.KQ32.A32.32"),
        Call::Bid(Bid::new(3, Strain::Notrump)),
        "16 HCP -> 3NT"
    );
    // K432.KQ32.Q32.Q3 — 12 HCP -> decline with the 3♥ retreat.
    assert_eq!(
        best(&trie, AFTER_2H_2NT, "K432.KQ32.Q32.Q3"),
        Call::Bid(Bid::new(3, Strain::Hearts)),
        "12 HCP -> 3♥ retreat"
    );
}

/// C7: responder's second call after opener's `2♣` rebid picks by weight.
#[test]
fn minor_rebid_responder_picks_by_weight() {
    let trie = tails_trie();

    // K432.AQ5.432.Q32 — 11 points, 3 hearts -> the jump preference
    // outranks the 2NT invite (both are live at 10-12 points).
    assert_eq!(
        best(&trie, AFTER_2C, "K432.AQ5.432.Q32"),
        Call::Bid(Bid::new(3, Strain::Hearts)),
        "11 points, 3 hearts -> 3♥"
    );
    // K432.98.76.AQJ54 — 11 points (10 HCP + unbalanced upgrade), 5 clubs,
    // only 2 hearts (no 3-heart holding) -> raise opener's minor.
    assert_eq!(
        best(&trie, AFTER_2C, "K432.98.76.AQJ54"),
        Call::Bid(Bid::new(3, Strain::Clubs)),
        "11 points, 5 clubs, no 3-heart holding -> 3♣"
    );
    // K432.Q3.K432.432 — 8 points, balanced, 2 hearts -> simple preference.
    assert_eq!(
        best(&trie, AFTER_2C, "K432.Q3.K432.432"),
        Call::Bid(Bid::new(2, Strain::Hearts)),
        "8 points, 2 hearts -> 2♥"
    );
    // K98765.4.Q32.J32 — 6 HCP (7 points), 6 spades, singleton heart ->
    // too weak for any invite; the weak spade rebid is the only live call
    // besides pass.
    assert_eq!(
        best(&trie, AFTER_2C, "K98765.4.Q32.J32"),
        Call::Bid(Bid::new(2, Strain::Spades)),
        "weak hand, 6 spades -> 2♠"
    );
    // AK32.Q54.K54.Q32 — 14 points, balanced, no fit found -> the game route.
    assert_eq!(
        best(&trie, AFTER_2C, "AK32.Q54.K54.Q32"),
        Call::Bid(Bid::new(3, Strain::Notrump)),
        "14 points, no fit -> 3NT"
    );
}

/// D0: fourth-suit-forcing fires at 12+ points; below that floor the
/// existing jump-preference table is unchanged.
#[test]
fn fourth_suit_forcing_fires_at_twelve_points() {
    let trie = fsf_trie();

    // AK32.Q54.K54.Q32 — 14 points, no fit found -> 2♦ fourth-suit-forcing
    // beats the old 3NT game route (weight 2.0 vs 0.9).
    assert_eq!(
        best(&trie, AFTER_2C, "AK32.Q54.K54.Q32"),
        Call::Bid(Bid::new(2, Strain::Diamonds)),
        "14 points -> 2♦ fourth-suit-forcing"
    );
    // K432.AQ5.432.Q32 — 11 points, 3 hearts -> below the 12-point FSF
    // floor, so the jump preference to 3♥ still wins.
    assert_eq!(
        best(&trie, AFTER_2C, "K432.AQ5.432.Q32"),
        Call::Bid(Bid::new(3, Strain::Hearts)),
        "11 points, 3 hearts -> 3♥ (below the FSF floor)"
    );
}

/// D1: opener's answer to the fourth-suit-forcing game force picks by
/// weight; the guaranteed-legal `2♥` catches every remaining hand.
#[test]
fn fourth_suit_forcing_opener_answers_by_weight() {
    let trie = fsf_trie();

    // KQ4.AJ76.A32.987 — 3 spades and a diamond stopper (the ace) ->
    // the delayed raise (1.4) beats the notrump answer (1.2).
    assert_eq!(
        best(&trie, AFTER_2C_2D, "KQ4.AJ76.A32.987"),
        Call::Bid(Bid::new(2, Strain::Spades)),
        "3 spades + diamond stopper -> 2♠ beats 2NT"
    );
    // 98.K8765.432.876 — no 3-card spade support, no 6th heart, no
    // diamond stopper, no 5-card club suit -> the guaranteed-legal
    // catch-all.
    assert_eq!(
        best(&trie, AFTER_2C_2D, "98.K8765.432.876"),
        Call::Bid(Bid::new(2, Strain::Hearts)),
        "none of the above -> 2♥ catch-all"
    );
}

/// D2: responder places the contract at game over opener's answer.
#[test]
fn fourth_suit_forcing_responder_places_the_contract() {
    let trie = fsf_trie();

    // AKJ87.654.32.432 — 5 spades after opener's 2♠ answer -> 4♠.
    assert_eq!(
        best(&trie, AFTER_2C_2D_2S, "AKJ87.654.32.432"),
        Call::Bid(Bid::new(4, Strain::Spades)),
        "5 spades after 2♠ -> 4♠"
    );
    // Q432.KJ8.654.J32 — 3 hearts after opener's 2NT answer -> 4♥.
    assert_eq!(
        best(&trie, AFTER_2C_2D_2NT, "Q432.KJ8.654.J32"),
        Call::Bid(Bid::new(4, Strain::Hearts)),
        "3 hearts after 2NT -> 4♥"
    );
    // Q432.K8.6543.J32 — neither a spade fit nor 3 hearts -> 3NT.
    assert_eq!(
        best(&trie, AFTER_2C_2D_2NT, "Q432.K8.6543.J32"),
        Call::Bid(Bid::new(3, Strain::Notrump)),
        "neither fit -> 3NT"
    );
}

/// Fourth-suit-forcing rides the major-rebid-tails adjunct: with tails
/// off, turning FSF on still inserts nothing (the whole adjunct — not
/// just FSF's own nodes — is gated by `major_rebid_tails()` first).
#[test]
fn fourth_suit_forcing_without_tails_inserts_nothing() {
    set_major_rebid_tails(false);
    set_fourth_suit_forcing(true);
    let mut trie = Trie::new();
    register_major_rebid_tails(&mut trie);
    set_major_rebid_tails(true); // restore the shipped defaults

    let hand: Hand = "K432.AQ5.432.Q32".parse().expect("valid test hand");
    assert!(
        trie.classify(hand, RelativeVulnerability::NONE, AFTER_2C)
            .is_none(),
        "fourth-suit-forcing must not register without the tails adjunct"
    );
}

#[test]
fn nt_invite_hcp_gauges_the_no_fit_rung() {
    // 1♥ – 1♠ – 2♦ (the remnant report's 2NT-invite seam): a 9-HCP
    // six-spade hand reads 10 points and invites 2NT by default — a
    // notrump invite priced in ruffs it will never take.  HCP-gauged it
    // takes the weak 2♠ rebid instead; a flat-ish 10-count invites on
    // either gauge.
    let after_2d: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Diamonds)),
        Call::Pass,
    ];
    let shaped = "KT8642.7.QJ4.QJ3"; // 9 HCP, 10 points
    let flat = "AT86.97.QJ42.QJ3"; // 10 HCP, 10 points
    let two_nt = Call::Bid(Bid::new(2, Strain::Notrump));

    let default_trie = fsf_trie();
    assert_eq!(
        best(&default_trie, after_2d, shaped),
        Call::Bid(Bid::new(2, Strain::Spades)),
        "default (HCP-gauged): the shaped 9 takes the weak rebid"
    );
    assert_eq!(
        best(&default_trie, after_2d, flat),
        two_nt,
        "a real 10-count still invites"
    );

    set_nt_invite_hcp(false);
    let legacy_trie = fsf_trie();
    set_nt_invite_hcp(true);
    assert_eq!(
        best(&legacy_trie, after_2d, shaped),
        two_nt,
        "the points gauge (off arm) invites the shaped 9"
    );
    assert_eq!(best(&legacy_trie, after_2d, flat), two_nt);
}

/// The fourth-suit-forcing `2♦` rule carries the alert.
#[test]
fn fourth_suit_forcing_rule_is_alerted() {
    set_fourth_suit_forcing(true);
    let rules = responder_after_minor_rebid(Suit::Clubs);

    let fsf_rule = rules
        .rules()
        .iter()
        .find(|r| r.call() == Call::Bid(Bid::new(2, Strain::Diamonds)))
        .expect("the fourth-suit-forcing rule is present");
    assert!(
        fsf_rule.alert().is_some(),
        "fourth-suit-forcing must carry an alert"
    );
}

/// After `1♦ – 1♥`, a balanced 12–14 with a five-card diamond suit rebids
/// the natural `2♦` by default but `1NT` once `set_balanced_1nt_rebid` is
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
    let build = || {
        let mut trie = Trie::new();
        insert_uncontested(
            &mut trie,
            &[call(1, Strain::Diamonds), call(1, Strain::Hearts)],
            rebid_raise_major(Suit::Hearts, Suit::Diamonds),
        );
        trie
    };

    set_balanced_1nt_rebid(false);
    assert_eq!(best(&build(), one_d_one_h, hand), call(2, Strain::Diamonds));

    set_balanced_1nt_rebid(true); // the shipped default
    let on = build();
    assert_eq!(best(&on, one_d_one_h, hand), call(1, Strain::Notrump));
}
