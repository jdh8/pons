use super::*;
use crate::bidding::System;
use contract_bridge::auction::RelativeVulnerability;
use contract_bridge::{Hand, Strain};

/// Build a Trie with RKCB installed for the test auction
fn rkcb_trie() -> Trie {
    let mut trie = Trie::new();
    // Our calls: 1♠ – 2NT – 3♣ (the context before 4NT is asked;
    // install_rkcb appends the 4NT ask itself)
    let our_calls = [
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Bid(Bid::new(2, Strain::Notrump)),
        Call::Bid(Bid::new(3, Strain::Clubs)),
    ];
    install_rkcb(&mut trie, &our_calls, Suit::Spades);
    trie
}

/// The rows the producer emits hold the row invariants — alerts on every
/// artificial rung (totality is exact-node-exempt, and RKCB is all exact
/// nodes).  One package per trump: the minor lanes take the cramped-signoff
/// branch and stop before the king ask, so a majors-only probe would miss
/// half the tables.
#[test]
fn row_package_invariants() {
    use crate::bidding::rows::Package;

    const fn package(name: &'static str, entries: fn() -> Vec<Entry>) -> Package {
        Package {
            name,
            gate: || true,
            entries,
        }
    }

    crate::bidding::rows::assert_package_invariants(&[
        package("rkcb:♠", || rkcb_rows("P* 1♠ (P) 3♠ (P)", Suit::Spades)),
        package("rkcb:♥", || rkcb_rows("P* 1♥ (P) 3♥ (P)", Suit::Hearts)),
        package("rkcb:♦", || rkcb_rows("P* 1♦ (P) 3♦ (P)", Suit::Diamonds)),
        package("rkcb:♣", || rkcb_rows("P* 1♣ (P) 3♣ (P)", Suit::Clubs)),
    ]);
}

/// The best call made by the trie for the given hand at the given auction
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

// The raw table auction interleaves opposing passes after each of our calls.
// Opener (our side) is in seat 1 (no leading pass), so the auction is:
//   [1♠, P, 2NT, P, 3♣, P, 4NT, P]
const ANS_AUCTION: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
    Call::Bid(Bid::new(2, Strain::Notrump)),
    Call::Pass,
    Call::Bid(Bid::new(3, Strain::Clubs)),
    Call::Pass,
    Call::Bid(Bid::new(4, Strain::Notrump)),
    Call::Pass,
];

/// RKCB answers at [1♠, P, 2NT, P, 3♣, P, 4NT, P]
#[test]
fn answers_keycard_counts() {
    let trie = rkcb_trie();

    // KQ732.K53.Q42.92 — no aces, trump K → 1 keycard → 5♣
    assert_eq!(
        best(&trie, ANS_AUCTION, "KQ732.K53.Q42.92"),
        Call::Bid(Bid::new(5, Strain::Clubs)),
        "1 keycard → 5♣"
    );

    // QJ732.K53.Q42.Q2 — no aces, heart K is NOT a keycard → 0 keycards → 5♦
    assert_eq!(
        best(&trie, ANS_AUCTION, "QJ732.K53.Q42.Q2"),
        Call::Bid(Bid::new(5, Strain::Diamonds)),
        "0 keycards → 5♦"
    );

    // AK732.A53.842.92 — 2 aces + trump K = 3 keycards → 5♦
    assert_eq!(
        best(&trie, ANS_AUCTION, "AK732.A53.842.92"),
        Call::Bid(Bid::new(5, Strain::Diamonds)),
        "3 keycards → 5♦"
    );

    // AQ732.A53.842.92 — 2 aces + trump Q → 2 keycards with Q → 5♠
    assert_eq!(
        best(&trie, ANS_AUCTION, "AQ732.A53.842.92"),
        Call::Bid(Bid::new(5, Strain::Spades)),
        "2 keycards + trump Q → 5♠"
    );

    // A8732.A53.842.92 — 2 aces, no trump Q or K → 2 keycards, no Q → 5♥
    assert_eq!(
        best(&trie, ANS_AUCTION, "A8732.A53.842.92"),
        Call::Bid(Bid::new(5, Strain::Hearts)),
        "2 keycards, no trump Q → 5♥"
    );

    // AK732.A53.A42.A2 — 4 aces + trump K = 5 keycards, no Q → 5♥ (same step as 2)
    assert_eq!(
        best(&trie, ANS_AUCTION, "AK732.A53.A42.A2"),
        Call::Bid(Bid::new(5, Strain::Hearts)),
        "5 keycards, no trump Q → 5♥"
    );

    // AKQ32.A53.A42.A2 — 4 aces + trump K + trump Q = 5 keycards with Q → 5♠
    assert_eq!(
        best(&trie, ANS_AUCTION, "AKQ32.A53.A42.A2"),
        Call::Bid(Bid::new(5, Strain::Spades)),
        "5 keycards, with trump Q → 5♠"
    );
}

/// Asker's continuation after 5♦ response
#[test]
fn asker_after_5d_response() {
    let trie = rkcb_trie();
    // Auction: [1♠, P, 2NT, P, 3♣, P, 4NT, P, 5♦, P]
    let auction: Vec<Call> = ANS_AUCTION
        .iter()
        .copied()
        .chain([Call::Bid(Bid::new(5, Strain::Diamonds)), Call::Pass])
        .collect();

    // KQ52.AK76.A72.93 — 3 keycards (A♥, A♦, K♠) → knows partner has 0 → sign off 5♠
    assert_eq!(
        best(&trie, &auction, "KQ52.AK76.A72.93"),
        Call::Bid(Bid::new(5, Strain::Spades)),
        "asker with 3 keycards after 5♦ → knows 0, sign off 5♠"
    );

    // Q852.AK76.K72.A3 — 2 keycards (A♥, A♣) → assumes partner has 3 → 6♠
    assert_eq!(
        best(&trie, &auction, "Q852.AK76.K72.A3"),
        Call::Bid(Bid::new(6, Strain::Spades)),
        "asker with 2 keycards after 5♦ → assumes 3, bid 6♠"
    );
}

/// King ask after 5♣ response (asker has 4 keycards)
#[test]
fn king_ask_after_5c() {
    let trie = rkcb_trie();
    // Auction: [1♠, P, 2NT, P, 3♣, P, 4NT, P, 5♣, P]
    let auction: Vec<Call> = ANS_AUCTION
        .iter()
        .copied()
        .chain([Call::Bid(Bid::new(5, Strain::Clubs)), Call::Pass])
        .collect();

    // AQJ2.A876.A72.A3 — 4 keycards, 19 HCP → all five combined and the
    // grand zone: 5NT king ask
    assert_eq!(
        best(&trie, &auction, "AQJ2.A876.A72.A3"),
        Call::Bid(Bid::new(5, Strain::Notrump)),
        "asker with 4 keycards and grand values after 5♣ → 5NT king ask"
    );
    // AQ52.A876.A72.A3 — the same four keycards on 18 HCP: short of the
    // grand zone the round is not spent, six of trumps ends it (RKCB is a
    // slam veto, not a slam seeker).
    assert_eq!(
        best(&trie, &auction, "AQ52.A876.A72.A3"),
        Call::Bid(Bid::new(6, Strain::Spades)),
        "asker with 4 keycards short of the grand zone → six, no king ask"
    );
}

/// King answer at the 5NT node
#[test]
fn king_answer_after_5nt() {
    let trie = rkcb_trie();
    // Auction: [1♠, P, 2NT, P, 3♣, P, 4NT, P, 5♣, P, 5NT, P]
    let auction: Vec<Call> = ANS_AUCTION
        .iter()
        .copied()
        .chain([
            Call::Bid(Bid::new(5, Strain::Clubs)),
            Call::Pass,
            Call::Bid(Bid::new(5, Strain::Notrump)),
            Call::Pass,
        ])
        .collect();

    // K9732.K53.942.92 — trump K (keycard) + K♥ (1 outside king) → 6♦
    assert_eq!(
        best(&trie, &auction, "K9732.K53.942.92"),
        Call::Bid(Bid::new(6, Strain::Diamonds)),
        "1 outside king → 6♦"
    );
}

// -----------------------------------------------------------------------
// Minor-suit keycard (plain 4NT)
// -----------------------------------------------------------------------

/// A trie with minor RKCB installed below `[1m, 2m, 4NT]`
fn minor_trie(trump: Suit) -> Trie {
    let mut trie = Trie::new();
    let m = Strain::from(trump);
    let our_calls = [Call::Bid(Bid::new(1, m)), Call::Bid(Bid::new(2, m))];
    install_rkcb(&mut trie, &our_calls, trump);
    trie
}

/// The answer node auction `[1m, P, 2m, P, 4NT, P]`
fn minor_ans_auction(trump: Suit) -> Vec<Call> {
    let m = Strain::from(trump);
    vec![
        Call::Bid(Bid::new(1, m)),
        Call::Pass,
        Call::Bid(Bid::new(2, m)),
        Call::Pass,
        Call::Bid(Bid::new(4, Strain::Notrump)),
        Call::Pass,
    ]
}

/// `minor_ans_auction` extended by one keycard answer `+ [answer, P]`
fn after_minor_answer(trump: Suit, answer: Bid) -> Vec<Call> {
    let mut a = minor_ans_auction(trump);
    a.push(Call::Bid(answer));
    a.push(Call::Pass);
    a
}

/// The generic answer table still fires for a minor trump (clubs).
#[test]
fn minor_answers_keycard_counts() {
    let trie = minor_trie(Suit::Clubs);
    let auction = minor_ans_auction(Suit::Clubs);

    // A654.832.K65.987 — A♠ only (K is diamonds) → 1 keycard → 5♣
    assert_eq!(
        best(&trie, &auction, "A654.832.K65.987"),
        Call::Bid(Bid::new(5, Strain::Clubs)),
        "1 keycard → 5♣"
    );
    // Q654.832.K65.J87 — no aces, no K♣ → 0 keycards → 5♦
    assert_eq!(
        best(&trie, &auction, "Q654.832.K65.J87"),
        Call::Bid(Bid::new(5, Strain::Diamonds)),
        "0 keycards → 5♦"
    );
    // A654.A32.65.J987 — A♠ A♥, clubs J987 (no K/Q) → 2 keycards no Q → 5♥
    assert_eq!(
        best(&trie, &auction, "A654.A32.65.J987"),
        Call::Bid(Bid::new(5, Strain::Hearts)),
        "2 keycards, no trump Q → 5♥"
    );
    // A654.A32.65.Q987 — A♠ A♥ + Q♣ → 2 keycards with Q → 5♠
    assert_eq!(
        best(&trie, &auction, "A654.A32.65.Q987"),
        Call::Bid(Bid::new(5, Strain::Spades)),
        "2 keycards with trump Q → 5♠"
    );
}

/// Clubs after a 5♣ answer: 3+ keycards → 6♣; otherwise Pass to play 5♣.
#[test]
fn clubs_after_5c_signoff_is_pass() {
    let trie = minor_trie(Suit::Clubs);
    let auction = after_minor_answer(Suit::Clubs, Bid::new(5, Strain::Clubs));

    // A654.A32.65.KQ87 — A♠ A♥ + K♣ → 3 keycards → 6♣
    assert_eq!(
        best(&trie, &auction, "A654.A32.65.KQ87"),
        Call::Bid(Bid::new(6, Strain::Clubs)),
        "asker 3 keycards after 5♣ → 6♣"
    );
    // A654.832.K65.987 — 1 keycard → off two → Pass to play partner's 5♣
    assert_eq!(
        best(&trie, &auction, "A654.832.K65.987"),
        Call::Pass,
        "asker ≤2 keycards after 5♣ → Pass (play 5♣)"
    );
}

/// Clubs after a 5♦/5♥/5♠ answer: no room — always 6♣, never Pass or 5♣.
#[test]
fn clubs_no_room_always_six() {
    let trie = minor_trie(Suit::Clubs);
    for answer in [
        Bid::new(5, Strain::Diamonds),
        Bid::new(5, Strain::Hearts),
        Bid::new(5, Strain::Spades),
    ] {
        let auction = after_minor_answer(Suit::Clubs, answer);
        for hand in ["A654.A32.65.KQ87", "Q654.Q32.Q65.Q98"] {
            let call = best(&trie, &auction, hand);
            assert_eq!(
                call,
                Call::Bid(Bid::new(6, Strain::Clubs)),
                "clubs after {answer:?}, hand {hand}: must be 6♣ (no room to stop)"
            );
        }
    }
}

/// Diamonds after a 5♣ answer: 3+ keycards → 6♦; otherwise sign off in 5♦.
#[test]
fn diamonds_after_5c_signoff_is_5d() {
    let trie = minor_trie(Suit::Diamonds);
    let auction = after_minor_answer(Suit::Diamonds, Bid::new(5, Strain::Clubs));

    // A654.A32.K65.987 — A♠ A♥ + K♦ → 3 keycards → 6♦
    assert_eq!(
        best(&trie, &auction, "A654.A32.K65.987"),
        Call::Bid(Bid::new(6, Strain::Diamonds)),
        "asker 3 keycards after 5♣ → 6♦"
    );
    // A654.832.J65.987 — A♠ only, no K♦ → 1 keycard → 5♦ signoff (legal over 5♣)
    assert_eq!(
        best(&trie, &auction, "A654.832.J65.987"),
        Call::Bid(Bid::new(5, Strain::Diamonds)),
        "asker ≤2 keycards after 5♣ → 5♦ signoff"
    );
}

/// Diamonds after a 5♦ answer: 3+ keycards (knows partner 0) → Pass; 2 → 6♦.
#[test]
fn diamonds_after_5d_signoff_is_pass() {
    let trie = minor_trie(Suit::Diamonds);
    let auction = after_minor_answer(Suit::Diamonds, Bid::new(5, Strain::Diamonds));

    // A654.A32.K65.987 — 3 keycards → knows partner 0 → Pass to play 5♦
    assert_eq!(
        best(&trie, &auction, "A654.A32.K65.987"),
        Call::Pass,
        "asker 3 keycards after 5♦ → Pass (play 5♦)"
    );
    // A654.A32.J65.987 — A♠ A♥, no K♦ → 2 keycards → assumes partner 3 → 6♦
    assert_eq!(
        best(&trie, &auction, "A654.A32.J65.987"),
        Call::Bid(Bid::new(6, Strain::Diamonds)),
        "asker 2 keycards after 5♦ → 6♦"
    );
}

/// The asker never bids 5NT for a minor (the king ask is major-only).
#[test]
fn minors_never_bid_5nt() {
    for trump in [Suit::Clubs, Suit::Diamonds] {
        let trie = minor_trie(trump);
        for answer in [
            Bid::new(5, Strain::Clubs),
            Bid::new(5, Strain::Diamonds),
            Bid::new(5, Strain::Hearts),
            Bid::new(5, Strain::Spades),
        ] {
            let auction = after_minor_answer(trump, answer);
            for hand in ["A654.A32.AK5.AQ8", "Q654.Q32.Q65.Q98"] {
                assert_ne!(
                    best(&trie, &auction, hand),
                    Call::Bid(Bid::new(5, Strain::Notrump)),
                    "{trump:?} after {answer:?}, hand {hand}: must never bid 5NT"
                );
            }
        }
    }
}

/// The 5NT king-ask node is never installed for a minor trump.
#[test]
fn minor_king_ask_node_absent() {
    let trie = minor_trie(Suit::Clubs);
    // [1♣, P, 2♣, P, 4NT, P, 5♣, P, 5NT, P] — the major king-ask path
    let mut auction = after_minor_answer(Suit::Clubs, Bid::new(5, Strain::Clubs));
    auction.push(Call::Bid(Bid::new(5, Strain::Notrump)));
    auction.push(Call::Pass);
    let hand: Hand = "A654.A32.65.KQ87".parse().unwrap();
    assert!(
        trie.classify(hand, RelativeVulnerability::NONE, &auction)
            .is_none(),
        "no king-answer table should exist for a minor trump"
    );
}

// -----------------------------------------------------------------------
// The queen relay
// -----------------------------------------------------------------------

/// A spade-trump book with the relay authored.  Named for what the tests
/// below are asking of it; the relay is unconditional, so it is the
/// ordinary RKCB trie.
fn relay_trie() -> Trie {
    rkcb_trie()
}

/// A spade book reached through a **limit raise**, so partner is shown for
/// only three trumps and the fit is eight — the one length where the trump
/// queen still decides between five and six ([`QUEEN_FIT`] is ten).  The
/// Jacoby-2NT book above promises four-plus opposite a five-card major, so
/// its fit is nine and the relay is correctly dead there.
///
/// [`QUEEN_FIT`]: crate::bidding::instinct::QUEEN_FIT
fn eight_card_relay_trie() -> Trie {
    let mut trie = Trie::new();
    let our_calls = [
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Bid(Bid::new(3, Strain::Spades)),
    ];
    install_rkcb(&mut trie, &our_calls, Suit::Spades);
    trie
}

/// `[1♠, P, 3♠, P, 4NT, P]` — the limit-raise ask node
const LIMIT_ANS_AUCTION: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
    Call::Bid(Bid::new(3, Strain::Spades)),
    Call::Pass,
    Call::Bid(Bid::new(4, Strain::Notrump)),
    Call::Pass,
];

/// A nine-card fit answers the queen question by itself, so the relay never
/// starts — Jacoby 2NT promises four-plus opposite five.
#[test]
fn nine_card_fit_needs_no_relay() {
    let trie = relay_trie();
    let mut auction = ANS_AUCTION.to_vec();
    auction.extend([Call::Bid(Bid::new(5, Strain::Clubs)), Call::Pass]);
    assert_eq!(
        best(&trie, &auction, "AKJ8.AK2.KJ32.42"),
        Call::Bid(Bid::new(6, Strain::Spades)),
        "four trumps opposite a shown five is nine: bid six, do not ask"
    );
}

/// The relay itself: the queenless asker asks, partner answers on the two
/// rungs above, and the asker places the contract on the reply.
#[test]
fn relay_asks_answers_and_places() {
    let trie = eight_card_relay_trie();
    let mut auction = LIMIT_ANS_AUCTION.to_vec();
    auction.extend([Call::Bid(Bid::new(5, Strain::Clubs)), Call::Pass]);

    // Three keycards, no trump queen → 5♦ relays instead of guessing.
    assert_eq!(
        best(&trie, &auction, "AKJ8.AK2.KJ32.42"),
        Call::Bid(Bid::new(5, Strain::Diamonds)),
        "queenless: ask the queen"
    );
    // The same count holding it → the relay is dead, bid the slam.
    assert_eq!(
        best(&trie, &auction, "AKQ8.AK2.KJ32.42"),
        Call::Bid(Bid::new(6, Strain::Spades)),
        "our own queen settles it: no relay"
    );
    // Four keycards decodes to all five combined, so six is bid whatever
    // the queen does.  Without the values to look at seven neither reply
    // is worth a round — no queen relay, and no 5NT king ask either.
    assert_eq!(
        best(&trie, &auction, "AK98.A32.A432.32"),
        Call::Bid(Bid::new(6, Strain::Spades)),
        "all five keycards, no grand values: bid the slam, ask nothing"
    );

    // Partner replies in one round: 5♠ denies flat, 6♠ denies with a buff,
    // 5♥/6♣/6♦ show the queen *and* the cheapest side king, 5NT shows the
    // queen with none.  Three trumps opposite the opener's shown five is
    // eight, the one length where only the honour itself can answer.
    auction.extend([Call::Bid(Bid::new(5, Strain::Diamonds)), Call::Pass]);
    assert_eq!(
        best(&trie, &auction, "K74.A653.8432.92"),
        Call::Bid(Bid::new(5, Strain::Spades)),
        "no trump queen → five of trump, which is the signoff too"
    );
    assert_eq!(
        best(&trie, &auction, "KQ4.A653.8432.92"),
        Call::Bid(Bid::new(5, Strain::Notrump)),
        "trump queen, no side king → 5NT"
    );
    assert_eq!(
        best(&trie, &auction, "KQ4.K653.8432.92"),
        Call::Bid(Bid::new(5, Strain::Hearts)),
        "trump queen and the ♥ king → the cheapest king rung"
    );
    assert_eq!(
        best(&trie, &auction, "KQ4.6532.K843.92"),
        Call::Bid(Bid::new(6, Strain::Diamonds)),
        "the ♦ king with no cheaper one → the rung above, skipping denies"
    );
    // A fifth trump opposite the opener's shown five is ten, and ten runs
    // the suit without the honour — the one length that may claim it.
    assert_eq!(
        best(&trie, &auction, "K7432.A65.843.92"),
        Call::Bid(Bid::new(5, Strain::Notrump)),
        "the tenth trump stands in for the queen"
    );
    // Nine is the in-between: not a queen, but far too good to let partner
    // pass five over a denial.  Jump to six and say so.
    assert_eq!(
        best(&trie, &auction, "K743.A653.843.92"),
        Call::Bid(Bid::new(6, Strain::Spades)),
        "the ninth trump is a buff, not a queen: bid six"
    );
    // The same nine-card fit holding the honour still shows it — the buff
    // jump is for hands that have nothing to show, not a substitute for
    // the rung above.
    assert_eq!(
        best(&trie, &auction, "KQ43.A653.843.92"),
        Call::Bid(Bid::new(5, Strain::Notrump)),
        "queen in hand: a show rung, not the jump"
    );
    // A void rides the same jump: worth a trick the ladder cannot show,
    // and partner is about to pass five without ever hearing about it.
    assert_eq!(
        best(&trie, &auction, "K74.A6532.8432."),
        Call::Bid(Bid::new(6, Strain::Spades)),
        "an eight-card fit with a void: still worth six"
    );

    // The asker places it.  Three keycards is four combined: a denied
    // queen leaves a keycard *and* the queen out, so stop at five.
    let mut denied = auction.clone();
    denied.extend([Call::Bid(Bid::new(5, Strain::Spades)), Call::Pass]);
    assert_eq!(
        best(&trie, &denied, "AKJ8.AK2.KJ32.42"),
        Call::Pass,
        "queen denied on four keycards: the denial is already the contract"
    );
    let mut shown = auction;
    shown.extend([Call::Bid(Bid::new(5, Strain::Notrump)), Call::Pass]);
    assert_eq!(
        best(&trie, &shown, "AKJ8.AK2.KJ32.42"),
        Call::Bid(Bid::new(6, Strain::Spades)),
        "queen shown on four keycards: bid the slam"
    );
}

/// Seven is explored only when the values are there, and bid on two of the
/// three side kings — RKCB is a slam veto, not a slam seeker.  The merged
/// reply names one of them, so the second relay is spent only when the
/// asker holds none of its own.
#[test]
fn relay_king_ask_needs_the_grand_values() {
    let trie = eight_card_relay_trie();
    let mut shown = LIMIT_ANS_AUCTION.to_vec();
    // 5♥ shows the trump queen and the ♥ king, denying nothing cheaper.
    shown.extend([
        Call::Bid(Bid::new(5, Strain::Clubs)),
        Call::Pass,
        Call::Bid(Bid::new(5, Strain::Diamonds)),
        Call::Pass,
        Call::Bid(Bid::new(5, Strain::Hearts)),
        Call::Pass,
    ]);
    // ♠AK98 ♥A32 ♦A432 ♣32 — four keycards, so all five are on the table
    // and the queen is shown, but 15 HCP is not a grand-zone hand: six.
    assert_eq!(
        best(&trie, &shown, "AK98.A32.A432.32"),
        Call::Bid(Bid::new(6, Strain::Spades)),
        "all five keycards and the queen, no grand values: six, never a second relay"
    );
    // ♠AK98 ♥AK2 ♦AK32 ♣32 — 21 HCP and a side king of its own opposite
    // partner's: two are already shown, so bid seven without asking again.
    assert_eq!(
        best(&trie, &shown, "AK98.AK2.AK32.32"),
        Call::Bid(Bid::new(7, Strain::Spades)),
        "one king each, shown in a single round: grand"
    );
    // ♠AKQJ ♥A32 ♦AQ32 ♣32 — 20 HCP, four keycards, and not one side king:
    // the second king is the whole question, so relay again at 5♠.
    assert_eq!(
        best(&trie, &shown, "AKQJ.A32.AQ32.32"),
        Call::Bid(Bid::new(5, Strain::Spades)),
        "grand values but no side king of our own: ask for a second"
    );

    let mut asked = shown;
    asked.extend([Call::Bid(Bid::new(5, Strain::Spades)), Call::Pass]);
    assert_eq!(
        best(&trie, &asked, "Q743.K65.K42.92"),
        Call::Bid(Bid::new(5, Strain::Notrump)),
        "a second side king → the cheap rung"
    );
    assert_eq!(
        best(&trie, &asked, "Q743.K65.842.92"),
        Call::Bid(Bid::new(6, Strain::Spades)),
        "only the king already shown → six of trumps ends it"
    );

    let mut answered = asked.clone();
    answered.extend([Call::Bid(Bid::new(5, Strain::Notrump)), Call::Pass]);
    assert_eq!(
        best(&trie, &answered, "AKQJ.A32.AQ32.32"),
        Call::Bid(Bid::new(7, Strain::Spades)),
        "two of the three side kings between the hands: grand"
    );
    let mut stopped = asked;
    stopped.extend([Call::Bid(Bid::new(6, Strain::Spades)), Call::Pass]);
    assert_eq!(
        best(&trie, &stopped, "AKQJ.A32.AQ32.32"),
        Call::Pass,
        "only partner's king: six is already the contract"
    );
}

/// The none-or-three lane decodes its own counts: four keycards of our own
/// is four **combined** — partner answered none — so a denial stops at
/// five and no grand is ever touched, while the grand explorer there holds
/// two (reading partner for three) and finds seven over a king-showing
/// reply.  The one-or-four lane's decode must not leak across.
#[test]
fn the_none_or_three_lane_decodes_the_total() {
    let trie = eight_card_relay_trie();
    let mut auction = LIMIT_ANS_AUCTION.to_vec();
    auction.extend([Call::Bid(Bid::new(5, Strain::Diamonds)), Call::Pass]);

    // ♠AKJ42 ♥A32 ♦AK5 ♣43 — four keycards, 19 HCP: partner showed none,
    // so one is missing and the queen decides five against six — relay.
    assert_eq!(
        best(&trie, &auction, "AKJ42.A32.AK5.43"),
        Call::Bid(Bid::new(5, Strain::Hearts)),
        "four keycards over none-or-three: one is missing, ask the queen"
    );

    // Queen denied flat: a keycard *and* the queen are out, so the denial
    // is already the contract.
    let mut denied = auction.clone();
    denied.extend([
        Call::Bid(Bid::new(5, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(5, Strain::Spades)),
        Call::Pass,
    ]);
    assert_eq!(
        best(&trie, &denied, "AKJ42.A32.AK5.43"),
        Call::Pass,
        "queen denied on four combined: stop at five"
    );

    // Queen and the ♥ king shown (6♥ in this lane): still four combined,
    // so 19 HCP and a side king of our own must not tempt a grand missing
    // a keycard.
    let mut shown = auction;
    shown.extend([
        Call::Bid(Bid::new(5, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(6, Strain::Hearts)),
        Call::Pass,
    ]);
    assert_eq!(
        best(&trie, &shown, "AKJ42.A32.AK5.43"),
        Call::Bid(Bid::new(6, Strain::Spades)),
        "one keycard out: six, never seven"
    );

    // ♠KJ942 ♥AQJ ♦KQJ ♣QJ — two keycards on twenty points reads partner
    // for three: all five are on the table, and partner's ♥K opposite our
    // ♦K is the second side king — seven.
    assert_eq!(
        best(&trie, &shown, "KJ942.AQJ.KQJ.QJ"),
        Call::Bid(Bid::new(7, Strain::Spades)),
        "two keycards reading three: the reply names the second king"
    );
}
