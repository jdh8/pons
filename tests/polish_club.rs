//! Integration tests for the Strawberry Polish Club port (AI-bidder M4.3)
//!
//! The textbook opening fixtures are the milestone's hard gate; they reuse the
//! exact hands curated against BBA's WJ reference in the `bba-wj-reference`
//! example (S.2).  The corpus guard pins the 0-opaque invariant the description
//! corpus depends on, and the reach-game check confirms the floored constructive
//! book never strands a strong uncontested auction below game.

use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};
use pons::bidding::polish_club::{bare_polish_club, polish_club};
use pons::bidding::trie::Trie;
use pons::bidding::{Family, System};

fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The highest finite-logit call the assembled system makes for a hand
fn opens(hand: &str) -> Call {
    let stance = polish_club().against(Family::NATURAL);
    let hand: Hand = hand.parse().expect("valid test hand");
    let logits = stance
        .classify(hand, RelativeVulnerability::NONE, &[])
        .expect("an opening decision");
    (&logits.0)
        .into_iter()
        .filter(|(_, l)| l.is_finite())
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("some opening")
}

#[test]
fn textbook_openings_are_correct() {
    // The five system-defining hard assertions (BBA WJ is ground truth here).
    assert_eq!(opens("AQ5.KJ4.KQ72.K43"), bid(1, Strain::Clubs)); // strong balanced
    assert_eq!(opens("43.K43.Q82.AKJ95"), bid(1, Strain::Clubs)); // clubs
    assert_eq!(opens("KJ4.AQ5.Q872.K32"), bid(1, Strain::Notrump)); // 15–17 1NT
    assert_eq!(opens("K3.AQ952.KJ3.842"), bid(1, Strain::Hearts)); // five-card major
    assert_eq!(opens("AQ952.K3.KJ3.842"), bid(1, Strain::Spades)); // five-card major

    // The rest of the curated set, now authored.
    assert_eq!(opens("AQ5.AKJ.KQ72.Q43"), bid(1, Strain::Clubs)); // strong balanced 21
    assert_eq!(opens("K3.842.AQJ95.KJ3"), bid(1, Strain::Diamonds)); // natural diamond
    assert_eq!(opens("KQJ976.43.852.42"), bid(2, Strain::Diamonds)); // Multi (weak 6♠)
}

/// Every authored rule renders a non-opaque meaning (the corpus invariant).
#[test]
fn books_are_zero_opaque() {
    let pair = bare_polish_club();
    let books: [&Trie; 3] = [&pair.constructive.0, &pair.competitive.0, &pair.defensive.0];
    for trie in books {
        for (auction, classifier) in trie.iter() {
            let Some(rules) = classifier.as_rules() else {
                continue;
            };
            for rule in rules.rules() {
                let prose = rule.describe().to_string();
                assert!(
                    !prose.contains("(opaque condition)"),
                    "opaque rule for {:?} at {auction:?}: {prose}",
                    rule.call(),
                );
            }
        }
    }
}

/// Play out an uncontested auction from a 1NT opening; opponents always pass.
fn play_uncontested(opener: &str, responder: &str) -> Vec<Call> {
    let stance = polish_club().against(Family::NATURAL);
    let oh: Hand = opener.parse().expect("valid opener hand");
    let rh: Hand = responder.parse().expect("valid responder hand");

    // Seat 0 opened 1NT; seat 1 (an opponent) passed.
    let mut auction = vec![bid(1, Strain::Notrump), Call::Pass];
    loop {
        let n = auction.len();
        if n >= 4 && auction[n - 3..].iter().all(|&c| c == Call::Pass) {
            break;
        }
        assert!(n <= 48, "auction did not terminate: {auction:?}");
        let next = match n % 4 {
            seat @ (0 | 2) => {
                let hand = if seat == 0 { oh } else { rh };
                match stance.classify(hand, RelativeVulnerability::NONE, &auction) {
                    None => Call::Pass,
                    Some(logits) => (&logits.0)
                        .into_iter()
                        .filter(|(_, l)| l.is_finite())
                        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("not NaN"))
                        .map(|(c, _)| c)
                        .unwrap_or(Call::Pass),
                }
            }
            _ => Call::Pass,
        };
        auction.push(next);
    }
    auction
}

fn final_bid(auction: &[Call]) -> Bid {
    auction
        .iter()
        .rev()
        .find_map(|c| match c {
            Call::Bid(b) => Some(*b),
            _ => None,
        })
        .expect("some contract was reached")
}

/// A strong (game-going) responder opposite a 1NT opening reaches game, never
/// stranding in the floored constructive book.  The 15–17 1NT reuses the
/// verified 2/1 notrump responses, so its game-forcing sequences carry over.
#[test]
fn strong_one_notrump_auctions_reach_game() {
    let opener = "AQ32.KJ5.KQ4.Q92"; // a flat 17-count, opens 1NT
    let responders = [
        "KQ542.A42.J3.832", // 5+ spades, game values → 4♠ via transfer
        "K92.Q73.AQ54.Q32", // balanced game values, no major → 3NT
        "73.AKQ842.K64.53", // 6+ hearts, game values → 4♥
    ];
    for rh in responders {
        let auction = play_uncontested(opener, rh);
        let contract = final_bid(&auction);
        let reached_game = contract.level.get() >= 4
            || (contract.level.get() == 3 && contract.strain == Strain::Notrump);
        assert!(
            reached_game,
            "responder {rh} stranded below game: {auction:?} (final {contract:?})"
        );
    }
}
