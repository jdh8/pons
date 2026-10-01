use crate::american;
use crate::bidding::Bidder;
use crate::bidding::agreements::Agreements;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Strain};

const P: Call = Call::Pass;

fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The highest-logit call `american()` assigns the hand at the auction
fn best_with(agreements: &Agreements, auction: &[Call], hand: &str) -> Call {
    let logits = american(agreements)
        .bind()
        .classify(
            hand.parse().expect("valid test hand"),
            RelativeVulnerability::NONE,
            auction,
        )
        .expect("a decision");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("the logits array is never empty")
}

fn waiting() -> Agreements {
    Agreements::default()
}

/// The pre-2026-10-01 double negative (`strong_two_waiting` off)
fn double_negative() -> Agreements {
    let mut agreements = Agreements::default();
    agreements.decision.strong_two_waiting = false;
    agreements
}

/// Off, a bust bids the `2♥` double negative; on, it waits with `2♦`, and
/// `2♥` is the heart positive.
#[test]
fn waiting_replaces_the_double_negative() {
    let (off, on) = (double_negative(), waiting());
    let opened = [bid(2, Strain::Clubs), P];
    let bust = "8632.J9842.96.42";
    assert_eq!(best_with(&off, &opened, bust), bid(2, Strain::Hearts));
    assert_eq!(best_with(&on, &opened, bust), bid(2, Strain::Diamonds));
    let positive = "J3.AKT42.T432.32";
    assert_eq!(best_with(&off, &opened, positive), bid(2, Strain::Diamonds));
    assert_eq!(best_with(&on, &opened, positive), bid(2, Strain::Hearts));
}

/// On, a 0–3 responder passes opener's `2M` without three-card support,
/// jumps to game with it, and passes a minor rebid; 4+ keeps the old calls.
#[test]
fn a_bust_passes_or_jumps_over_the_suit_rebid() {
    let on = waiting();
    let (c, d, h, s, nt) = (
        Strain::Clubs,
        Strain::Diamonds,
        Strain::Hearts,
        Strain::Spades,
        Strain::Notrump,
    );
    let hearts = [bid(2, c), P, bid(2, d), P, bid(2, h), P];
    assert_eq!(best_with(&on, &hearts, "8632.J9.9642.432"), P);
    assert_eq!(best_with(&on, &hearts, "8632.J94.962.432"), bid(4, h));
    assert_eq!(best_with(&on, &hearts, "8632.K94.Q62.432"), bid(3, h));
    assert_eq!(best_with(&on, &hearts, "Q632.K9.Q962.432"), bid(2, nt));
    let spades = [bid(2, c), P, bid(2, d), P, bid(2, s), P];
    assert_eq!(best_with(&on, &spades, "J96.8632.9642.43"), bid(4, s));
    let clubs = [bid(2, c), P, bid(2, d), P, bid(3, c), P];
    assert_eq!(best_with(&on, &clubs, "J632.9842.962.43"), P);
}

/// On, `2♣ - 2♦` no longer forces game: a bust that transferred over the
/// `2NT` rebid passes the completion instead of the floor bidding game.
#[test]
fn waiting_lifts_the_floor_game_force() {
    let (c, d, h, nt) = (
        Strain::Clubs,
        Strain::Diamonds,
        Strain::Hearts,
        Strain::Notrump,
    );
    let completed = [
        bid(2, c),
        P,
        bid(2, d),
        P,
        bid(2, nt),
        P,
        bid(3, d),
        P,
        bid(3, h),
        P,
    ];
    assert_eq!(best_with(&waiting(), &completed, "832.J9842.962.43"), P);
}

/// On, opener never passes the `2♦` wait (the catch-all covers a `2♣` opened
/// on points), and opener's floor stays forced over their interference.
#[test]
fn opener_stays_forced_after_the_wait() {
    let on = waiting();
    let (c, d, s) = (Strain::Clubs, Strain::Diamonds, Strain::Spades);
    let waited = [bid(2, c), P, bid(2, d), P];
    assert_ne!(best_with(&on, &waited, "AKQJ.KQ84.9.AQ96"), P);
    let overcalled = [bid(2, c), P, bid(2, d), bid(2, s)];
    assert_ne!(best_with(&on, &overcalled, "KJ73.AK2.AK43.AT"), P);
}

/// On, opener does not sell out when they overcall responder's transfer over
/// the `2NT` rebid (`- 2♣ - 2♦ - 2NT - 3♦ (3♥)`).
#[test]
fn opener_competes_over_the_contested_transfer() {
    let (c, d, h, nt) = (
        Strain::Clubs,
        Strain::Diamonds,
        Strain::Hearts,
        Strain::Notrump,
    );
    let auction = [
        P,
        bid(2, c),
        P,
        bid(2, d),
        P,
        bid(2, nt),
        P,
        bid(3, d),
        bid(3, h),
    ];
    assert_ne!(best_with(&waiting(), &auction, "A.AK9.AJ75.AQ975"), P);
}

/// Their `2♣` still decodes as the house structure: the knob is ours alone
/// (`common::mirror_agreements`), so a defender's call over their
/// `2♣ - 2♦ - 2NT - 3♦` does not move with it.
#[test]
fn waiting_does_not_reach_their_two_clubs() {
    let (c, d, nt) = (Strain::Clubs, Strain::Diamonds, Strain::Notrump);
    let auction = [P, bid(2, c), P, bid(2, d), P, bid(2, nt), P, bid(3, d)];
    let hand = "T975.QT3.43.KJ83".parse().expect("valid test hand");
    let classify = |agreements: &Agreements| {
        american(agreements)
            .bind()
            .classify(hand, RelativeVulnerability::ALL, &auction)
            .expect("a decision")
    };
    assert_eq!(classify(&waiting()), classify(&double_negative()));
}
