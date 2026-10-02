use super::*;
use crate::american;
use crate::bidding::agreements::Agreements;
use contract_bridge::{Bid, Level, Strain};

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

/// Opener's turn after `1♠ - 2♠ -`: the seam Phase 3b measured
const SEAM: [Call; 4] = [
    bid(1, Strain::Spades),
    Call::Pass,
    bid(2, Strain::Spades),
    Call::Pass,
];

fn partnership() -> Partnership {
    american(&Agreements::default()).bind()
}

/// A lookahead cheap enough for a unit test, gated on [`SEAM`] alone
fn lookahead() -> Lookahead<impl Fn(&[Call]) -> bool> {
    let mut lookahead = Lookahead::new(partnership(), |auction: &[Call]| auction == SEAM);
    lookahead.layouts = 8;
    lookahead
}

/// A sound invitation: 16 HCP, six spades
fn inviter() -> Hand {
    "AKJ854.A2.KJ3.42".parse().expect("valid test hand")
}

/// The seam's table offers the whole ladder — pass, the invitation, game —
/// whatever the hand holds; that is what makes a rung a candidate.
#[test]
fn rungs_span_the_ladder() {
    let rungs = partnership().rungs(inviter(), RelativeVulnerability::NONE, &SEAM);
    for call in [Call::Pass, bid(3, Strain::Spades), bid(4, Strain::Spades)] {
        assert!(rungs.contains(&call), "{call} missing from {rungs:?}");
    }
    // The keyless floor has no rungs to offer.
    let floor = [bid(1, Strain::Spades), bid(7, Strain::Clubs)];
    assert!(
        partnership()
            .rungs(inviter(), RelativeVulnerability::NONE, &floor)
            .is_empty()
    );
}

/// Off the gate the wrapper is the partnership, logit for logit.
#[test]
fn ungated_is_the_partnership() {
    let auction = &SEAM[..2];
    let hand: Hand = "92.KQ3.A8752.J63".parse().expect("valid test hand");
    assert_eq!(
        lookahead().classify(hand, RelativeVulnerability::NONE, auction),
        partnership().classify(hand, RelativeVulnerability::NONE, auction),
    );
}

/// At the gate the answer is one of the table's legal rungs, and the same one
/// every time (invariant §0.5: classify stays pure though the rollout samples).
#[test]
fn gated_pick_is_a_rung_and_deterministic() {
    let lookahead = lookahead();
    let mut auction = Auction::new();
    auction.try_extend(SEAM).expect("the seam is legal");
    let call = |_| {
        let logits = lookahead.classify(inviter(), RelativeVulnerability::NONE, &SEAM);
        select_legal_call(logits, &auction)
    };
    let first = call(());
    assert_eq!(first, call(()));
    assert!(
        partnership()
            .rungs(inviter(), RelativeVulnerability::NONE, &SEAM)
            .contains(&first),
        "{first} is not a rung"
    );
}

/// A margin no swing can clear leaves the book's call standing.
#[test]
fn unreachable_margin_keeps_the_book() {
    let mut lookahead = lookahead();
    lookahead.margin = 24.0;
    assert_eq!(
        lookahead.classify(inviter(), RelativeVulnerability::NONE, &SEAM),
        partnership().classify(inviter(), RelativeVulnerability::NONE, &SEAM),
    );
}
