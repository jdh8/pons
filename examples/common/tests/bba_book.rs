//! The `bba` book on a three-node fixture cut from the 2026-10-29 walk at
//! `8fb078de` with `--card none` (`bba-book-fixture.jsonl`: the root, `1♠`
//! and `1♠ -`, every other expanded child marked a dead end) — the pinned
//! node of docs/bba-system.md Gate 1.

use super::super::bba_book::{BbaBook, Chooser, Mask, Node, is_calculated};
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain, Suit};
use pons::bidding::agreements::Agreements;
use pons::bidding::{Bidder, Partnership};
use std::collections::HashMap;

const S1: Call = Call::Bid(Bid::new(1, Strain::Spades));
const NT1: Call = Call::Bid(Bid::new(1, Strain::Notrump));
const S2: Call = Call::Bid(Bid::new(2, Strain::Spades));
const P: Call = Call::Pass;

/// `KJ2.J654.Q87.T93`: an 8-count with three spades, which BBA's `1♠ -` node
/// admits for both the simple raise and the forcing `1NT`
fn eight_count() -> Hand {
    "KJ2.J654.Q87.T93".parse().unwrap()
}

fn fixture() -> BbaBook {
    let nodes: HashMap<String, Node> = include_str!("bba-book-fixture.jsonl")
        .lines()
        .map(|line| serde_json::from_str::<Node>(line).unwrap())
        .map(|node| (node.k.clone(), node))
        .collect();
    BbaBook::compile(&nodes).unwrap()
}

fn bound(chooser: Chooser) -> Partnership {
    fixture()
        .system(&Agreements::default(), chooser, Mask::default())
        .bind()
}

#[test]
fn the_fold_reads_the_pinned_node() {
    let book = fixture();
    assert_eq!(book.len(), 3);
    let node = book.get(&[S1, P]).unwrap();
    let raise = node.iter().find(|entry| entry.call == S2).unwrap();
    assert_eq!(raise.band.hcp, [7, 9]);
    assert_eq!(raise.band.lengths[3], [3, 13], "3+♠ after the raise");
    assert_eq!(raise.band.gloss(), "7-9 3+♠");
    assert!(
        is_calculated(raise.label),
        "BBA's simple raise is its floor's"
    );
    assert!(!raise.alert);
    let forcing = node.iter().find(|entry| entry.call == NT1).unwrap();
    assert_eq!(forcing.band.hcp, [5, 12]);
    assert_eq!(forcing.band.lengths[3], [0, 3]);
    assert!(forcing.alert, "BBA alerts its forcing 1NT");
    // The opener's own band carried into the node: `1♠` was 11-20 with 5+♠,
    // which is the opener's state, not the responder's.
    let opening = book.get(&[]).unwrap();
    let one_spade = opening.iter().find(|entry| entry.call == S1).unwrap();
    assert_eq!(one_spade.band.gloss(), "11-20 5+♠");
    // Off the walk.
    assert!(book.get(&[S1, P, S2]).is_none());
}

#[test]
fn the_band_admits_the_eight_count_for_two_calls() {
    let book = fixture();
    let admitted = book
        .admissible(&[S1, P], eight_count(), Mask::default())
        .unwrap();
    assert_eq!(admitted, vec![NT1, S2]);
    // A 10-count with three spades is only the forcing `1NT` (the 7-9 raise
    // is a point short, the 10-12 raises want four); widening by one HCP
    // (trap 4) lets the raise back in.
    let ten_count: Hand = "KJ2.KJ54.Q87.T93".parse().unwrap();
    let no_gloss = Mask {
        no_ceiling_floor: true,
        ..Mask::default()
    };
    assert_eq!(
        book.admissible(&[S1, P], ten_count, no_gloss).unwrap(),
        vec![NT1]
    );
    let widened = Mask {
        hcp_slack: 1,
        ..Mask::default()
    };
    assert_eq!(
        book.admissible(
            &[S1, P],
            ten_count,
            Mask {
                no_ceiling_floor: true,
                ..widened
            }
        )
        .unwrap(),
        vec![NT1, S2]
    );
    assert!(
        book.admissible(&[S1, P, S2], eight_count(), Mask::default())
            .is_none()
    );
    // Above the ceiling BBA's floor glosses `6♠` as 9+ with 3+♠, which admits
    // the ten-count; the ceiling knob drops the gloss.
    assert_eq!(
        book.admissible(&[S1, P], ten_count, Mask::default())
            .unwrap(),
        vec![NT1, Call::Bid(Bid::new(6, Strain::Spades))]
    );
    assert_eq!(
        book.admissible(&[S1, P], ten_count, no_gloss).unwrap(),
        vec![NT1]
    );
    // `5NT` as an opening is `Not defined`: no rule, no reading, never admitted.
    let opening = book.get(&[]).unwrap();
    let five_nt = opening
        .iter()
        .find(|entry| entry.call == Call::Bid(Bid::new(5, Strain::Notrump)))
        .unwrap();
    assert_eq!(five_nt.label, "Not defined");
    assert!(!five_nt.in_book(Mask::default()));
}

#[test]
fn the_system_masks_the_floor_to_the_band() {
    let partnership = bound(Chooser::Net);
    let (logits, provenance) = partnership
        .classify_with_provenance(eight_count(), RelativeVulnerability::NONE, &[S1, P])
        .unwrap();
    assert!(
        provenance.is_authored(),
        "a walked node answers as a book node"
    );
    let finite: Vec<Call> = logits
        .iter()
        .filter(|(_, logit)| logit.is_finite())
        .map(|(call, _)| call)
        .collect();
    assert_eq!(
        finite,
        vec![NT1, S2],
        "the choice is the floor's, inside the band"
    );
    // Off the walk the floor answers unmasked.
    let (_, provenance) = partnership
        .classify_with_provenance(eight_count(), RelativeVulnerability::NONE, &[S1, P, S2, P])
        .unwrap();
    assert!(!provenance.is_authored());
}

#[test]
fn partner_reads_the_band_and_the_alert() {
    let partnership = bound(Chooser::Net);
    // Opener to act after the raise: partner is the responder.
    let inferences = partnership.infer(RelativeVulnerability::NONE, &[S1, P, S2, P]);
    let partner = inferences.partner();
    assert!(
        partner.lengths[Suit::Spades as usize].min >= 3,
        "{partner:?}"
    );
    assert!(
        partner.strength.hcp.min >= 7 && partner.strength.hcp.max <= 9,
        "{partner:?}"
    );
    // And the opponents' passes are read off BBA's book too: the passer over
    // `1♠` holds at most 14.
    let lho = inferences.lho();
    assert!(lho.strength.hcp.max <= 14, "{lho:?}");
    let (_, rule) = partnership
        .explain_call(eight_count(), RelativeVulnerability::NONE, &[S1, P], NT1)
        .unwrap();
    assert_eq!(rule.unwrap().alert, Some("forcing 1NT"));
    let (_, rule) = partnership
        .explain_call(eight_count(), RelativeVulnerability::NONE, &[S1, P], S2)
        .unwrap();
    assert_eq!(rule.unwrap().alert, None);
}

#[test]
fn the_chooser_names_are_the_floor_names() {
    assert_eq!(Chooser::parse("bba"), Some(Chooser::Net));
    assert_eq!(Chooser::parse("bba-ladder"), Some(Chooser::Partition));
    assert_eq!(Chooser::parse("american"), None);
    assert_eq!(Chooser::Net.name(), "bba");
    // The partition chooser builds too.
    let partnership = bound(Chooser::Partition);
    assert!(
        partnership
            .classify(eight_count(), RelativeVulnerability::NONE, &[S1, P])
            .is_some()
    );
}
