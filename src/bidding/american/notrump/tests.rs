use crate::american;
use crate::bidding::Bidder;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Strain};

pub(super) const P: Call = Call::Pass;

pub(super) fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The highest-logit call `american()` assigns the hand at the auction
pub(super) fn best(auction: &[Call], hand: &str) -> Call {
    best_with(
        &crate::bidding::agreements::Agreements::default(),
        auction,
        hand,
    )
}

/// [`best`], but under an explicit set of agreements
pub(super) fn best_with(
    agreements: &crate::bidding::agreements::Agreements,
    auction: &[Call],
    hand: &str,
) -> Call {
    let hand = hand.parse().expect("valid test hand");
    let logits = american(agreements)
        .bind()
        .classify(hand, RelativeVulnerability::NONE, auction)
        .expect("a decision");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("the logits array is never empty")
}

/// Over a natural (2♣) overcall of our 1NT we play *systems on*, not
/// Lebensohl: 2♣ steals no room, so responder keeps the uncontested Jacoby
/// transfers, shows the stolen 2♣ Stayman with a Double, and opener answers in
/// the uncontested tree (the systems-on rebase in `competition.rs`). There is
/// no natural 2♦ escape — 2♦ is a transfer.
#[test]
fn systems_on_over_two_clubs() {
    use contract_bridge::auction::Auction;
    // The highest-logit *legal* call (what the real bidder picks; the bare
    // `best` helper ignores legality, so it can't drop the now-illegal 2♣).
    let best_legal = |auction: &[Call], hand: &str| -> Call {
        let hand = hand.parse().expect("valid test hand");
        let logits = american(&crate::bidding::agreements::Agreements::default())
            .bind()
            .classify(hand, RelativeVulnerability::NONE, auction)
            .expect("a decision");
        let mut played = Auction::new();
        for &c in auction {
            played.push(c);
        }
        let mut scored: Vec<_> = (&logits.0)
            .into_iter()
            .filter(|(_, l)| l.is_finite())
            .collect();
        scored.sort_by(|x, y| y.1.partial_cmp(x.1).expect("no NaN"));
        scored
            .into_iter()
            .map(|(c, _)| c)
            .find(|&c| played.can_push(c).is_ok())
            .unwrap_or(Call::Pass)
    };

    let over_2c = [bid(1, Strain::Notrump), bid(2, Strain::Clubs)];
    // 5 hearts → 2♦ transfer; 5 spades → 2♥ transfer (systems on, not natural).
    assert_eq!(
        best_legal(&over_2c, "2.KJ876.5432.432"),
        bid(2, Strain::Diamonds)
    );
    assert_eq!(
        best_legal(&over_2c, "KJ876.2.5432.432"),
        bid(2, Strain::Hearts)
    );
    // 4-4 majors, invitational: the stolen 2♣ Stayman is shown by Double.
    assert_eq!(best_legal(&over_2c, "KJ32.KQ43.432.43"), Call::Double);

    // Opener completes the transfer: 1NT (2♣) 2♦ - → 2♥, via the rebase.
    let over_xfer = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Clubs),
        bid(2, Strain::Diamonds),
        P,
    ];
    assert_eq!(
        best_legal(&over_xfer, "KQ3.A53.KQ54.K92"),
        bid(2, Strain::Hearts)
    );

    // Opener answers the stolen Stayman: 1NT (2♣) X - → 2♥ with four hearts.
    let over_dbl = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Clubs),
        Call::Double,
        P,
    ];
    assert_eq!(
        best_legal(&over_dbl, "AQ3.KJ54.KQ4.92"),
        bid(2, Strain::Hearts)
    );
}

/// Opener converts the stolen-Stayman Double to penalty with good clubs, and
/// *only* in the contested context — uncontested forcing Stayman never passes.
#[test]
fn penalty_pass_over_two_clubs() {
    let arm = |spec| {
        let mut agreements = crate::bidding::agreements::Agreements::default();
        agreements.competition.penalty_pass = spec;
        agreements
    };

    // 16 HCP, 5332 with AK-fifth of clubs (5 clubs, 7 club HCP), no 4-card major.
    let opener = "A2.K3.Q42.AK432";
    let over_dbl = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Clubs),
        Call::Double,
        P,
    ];
    let uncontested_stayman = [bid(1, Strain::Notrump), P, bid(2, Strain::Clubs), P];

    // With the penalty pass enabled (the shipped default), opener sits to defend
    // 2♣ doubled.
    let on = arm(Some((4, 4, true)));
    assert_eq!(best_with(&on, &over_dbl, opener), Call::Pass);
    // Context-specific: the same hand still answers forcing Stayman (2♦) in the
    // *uncontested* auction — the conversion must not leak onto that shared node.
    assert_eq!(
        best_with(&on, &uncontested_stayman, opener),
        bid(2, Strain::Diamonds)
    );

    // With it off, opener can never convert: answers Stayman 2♦.
    let off = arm(None);
    assert_eq!(best_with(&off, &over_dbl, opener), bid(2, Strain::Diamonds));
}

#[test]
fn notrump_responses_transfer_and_stayman() {
    let a = [bid(1, Strain::Notrump), Call::Pass];
    assert_eq!(best(&a, "KJ542.Q32.K43.92"), bid(2, Strain::Hearts));
    // Four-four in the majors takes Stayman; a 4-3 hand would Puppet (3♣).
    assert_eq!(best(&a, "KJ54.KQ32.43.Q92"), bid(2, Strain::Clubs));
}

#[test]
fn minor_transfer_slam_try_is_authored_end_to_end() {
    let completed = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Spades),
        P,
        bid(3, Strain::Clubs),
        P,
    ];
    let responder = "32.K42.A2.AKQJ32";
    assert_eq!(best(&completed, responder), bid(4, Strain::Clubs));

    let mut shown = completed.to_vec();
    shown.extend([bid(4, Strain::Clubs), P]);
    let partnership = american(&crate::bidding::agreements::Agreements::default()).bind();
    let read = crate::bidding::Inferences::read(
        &partnership.prefixed_context(RelativeVulnerability::NONE, &shown),
    );
    let partner = read.get(crate::bidding::inference::Relative::Partner);
    assert!(partner.length(contract_bridge::Suit::Clubs).min >= 6);
    assert!(partner.strength.points.min >= 13);

    for tail in [P, Call::Double] {
        let mut asked = completed.to_vec();
        asked.extend([bid(4, Strain::Clubs), tail]);
        assert_eq!(best(&asked, "AQ32.KQ54.A4.K32"), bid(4, Strain::Notrump));
        asked.extend([bid(4, Strain::Notrump), P]);
        assert_eq!(best(&asked, responder), bid(5, Strain::Diamonds));
    }

    let mut off = crate::bidding::agreements::Agreements::default();
    off.notrump.minor_transfer_slam_try = None;
    assert_eq!(
        best_with(&off, &completed, responder),
        bid(3, Strain::Notrump)
    );
}

#[test]
fn supported_five_diamonds_can_slam_try() {
    let supported = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Notrump),
        P,
        bid(3, Strain::Diamonds),
        P,
    ];
    let responder = "K2.A3.AKQ32.AQ32";
    let mut narrow = crate::bidding::agreements::Agreements::default();
    narrow.notrump.minor_transfer_slam_fit = false;
    assert_eq!(
        best_with(&narrow, &supported, responder),
        bid(3, Strain::Notrump)
    );

    let mut wide = narrow;
    wide.notrump.minor_transfer_slam_fit = true;
    assert_eq!(
        best_with(&wide, &supported, responder),
        bid(4, Strain::Diamonds)
    );

    let mut shown = supported.to_vec();
    shown.extend([bid(4, Strain::Diamonds), P]);
    let partnership = american(&wide).bind();
    let read = crate::bidding::Inferences::read(
        &partnership.prefixed_context(RelativeVulnerability::NONE, &shown),
    );
    assert!(read.admits(
        crate::bidding::inference::Relative::Partner,
        responder.parse().expect("a hand")
    ));

    let denied = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Notrump),
        P,
        bid(3, Strain::Clubs),
        P,
    ];
    assert_eq!(
        best_with(&wide, &denied, responder),
        bid(3, Strain::Notrump)
    );

    assert_eq!(
        best_with(&wide, &shown, "AQ32.KJ54.A43.Q2"),
        bid(4, Strain::Notrump)
    );
}
