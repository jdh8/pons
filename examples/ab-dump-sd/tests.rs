//! The disclosure manipulation must reach the blind leader
//! (docs/research-method-review.md §4).  Before 2026-10-16 an East–West
//! leader read our calls through the plain partnership, which decodes the
//! opponents' calls at `ReadingScope::Alerted` only: the unalerted natural
//! `(1NT) 2♥` overcall fell to the natural walk, so `--on-ns-overcall 20:37`
//! reproduced the `9:14` numbers to the last IMP.

use super::*;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{FullDeal, Hand, Suit};
use pons::bidding::agreements::Agreements;
use pons::bidding::{Envelope, Inferences, Relative};
use pons::single_dummy::single_dummy_leads;

fn auction(text: &str) -> Auction {
    let mut auction = Auction::new();
    for token in text.split_whitespace() {
        let call: Call = token.parse().expect("a call");
        auction.try_push(call).expect("a legal call");
    }
    auction
}

fn band(lo: u8, hi: u8) -> Agreements {
    let mut agreements = Agreements::default();
    agreements.decision.reading.natural_overcall_points = (lo, hi);
    agreements
}

fn reader(agreements: &Agreements, legacy: bool) -> TableReader {
    let partnership = american(agreements).bind();
    if legacy {
        TableReader::legacy(partnership)
    } else {
        TableReader::new(partnership)
    }
}

/// The blind leader's inferences on a complete auction
fn leader_inferences(text: &str, dealer: Seat, reader: &TableReader) -> Inferences {
    reader
        .lead_inputs(&auction(text), dealer, AbsoluteVulnerability::NONE)
        .expect("a contract")
        .2
}

fn shown(text: &str, dealer: Seat, reader: &TableReader, who: Relative) -> Envelope {
    *leader_inferences(text, dealer, reader).get(who)
}

/// We declare `(1NT) 2♥`: West, theirs, leads and reads our overcall (RHO).
const WE_DECLARE: &str = "1NT 2H P P P";
/// They declare `2♠`: North, ours, leads and reads partner's overcall.
const THEY_DECLARE: &str = "1NT 2H 2S P P P";

#[test]
fn east_west_leader_reads_the_overcall_band() {
    let sane = shown(
        WE_DECLARE,
        Seat::East,
        &reader(&band(9, 14), false),
        Relative::Rho,
    );
    let absurd = shown(
        WE_DECLARE,
        Seat::East,
        &reader(&band(20, 37), false),
        Relative::Rho,
    );
    assert_eq!(
        (sane.strength.points.min, sane.strength.points.max),
        (9, 14)
    );
    assert_eq!(absurd.strength.points.min, 20);
    assert_eq!(sane.length(Suit::Hearts).min, 5);
    assert_eq!(absurd.length(Suit::Hearts).min, 5);
}

/// The defect itself, pinned so `--legacy-leader-reading` keeps reproducing
/// the old numbers: the legacy East–West leader cannot tell the bands apart.
#[test]
fn legacy_east_west_leader_is_blind_to_the_band() {
    let sane = shown(
        WE_DECLARE,
        Seat::East,
        &reader(&band(9, 14), true),
        Relative::Rho,
    );
    let absurd = shown(
        WE_DECLARE,
        Seat::East,
        &reader(&band(20, 37), true),
        Relative::Rho,
    );
    assert_eq!(sane, absurd);
    assert_eq!(
        sane.strength.points.max, 37,
        "the natural walk caps nothing"
    );
}

/// A North–South leader is ours and reads partner's call at the full scope
/// already; the fix leaves that reading alone.
#[test]
fn north_south_leader_is_unchanged() {
    for (lo, hi) in [(9, 14), (20, 37)] {
        let agreements = band(lo, hi);
        let new = shown(
            THEY_DECLARE,
            Seat::East,
            &reader(&agreements, false),
            Relative::Partner,
        );
        let old = shown(
            THEY_DECLARE,
            Seat::East,
            &reader(&agreements, true),
            Relative::Partner,
        );
        assert_eq!(new, old);
        assert_eq!(new.strength.points.min, lo);
    }
}

/// The negative-double shape is *alerted*, so even the legacy reader decodes
/// it from either side: `measurement.md`'s second inert flag
/// (`--on-ns-negative-double-shape`) does not reproduce on this reader.
#[test]
fn negative_double_shape_reaches_both_leaders() {
    let shape = |shape| {
        let mut agreements = Agreements::default();
        agreements.competition.negative_double_shape = shape;
        agreements
    };
    // `1♣ (1♥) X` by South, North dealing: we declare `1♠` (East leads, LHO
    // doubled) or they declare `2♦` (North leads, partner doubled).
    for (text, who) in [
        ("1C 1H X P 1S P P P", Relative::Lho),
        ("1C 1H X 2D P P P", Relative::Partner),
    ] {
        for legacy in [true, false] {
            let modern = shown(
                text,
                Seat::North,
                &reader(&shape(NegativeDoubleShape::Modern), legacy),
                who,
            );
            let both = shown(
                text,
                Seat::North,
                &reader(&shape(NegativeDoubleShape::BothMajors), legacy),
                who,
            );
            assert_eq!(
                modern.length(Suit::Spades).max,
                4,
                "{text}: exactly four spades"
            );
            assert_eq!(both.length(Suit::Hearts).min, 4, "{text}: both majors");
        }
    }
}

/// West's hand in the sampled-world checks: nothing, so a 20+ overcall fits
/// beside a 15–17 notrump.
const WEST: &str = "8642.32.9643.854";

fn south_hcp_in_worlds(reader: &TableReader) -> Vec<u8> {
    let hand: Hand = WEST.parse().expect("a hand");
    let inferences = leader_inferences(WE_DECLARE, Seat::East, reader);
    let mut rng = StdRng::seed_from_u64(20261016);
    pons::bidding::sample_layouts(hand, Seat::West, &inferences, &mut rng, 64)
        .iter()
        .map(|world: &FullDeal| common::hand_hcp(world[Seat::South]))
        .collect()
}

/// The band reaches the leader's *sampler*, not only its envelope.
#[test]
fn absurd_band_moves_the_sampled_worlds() {
    let sane = south_hcp_in_worlds(&reader(&band(9, 14), false));
    let absurd = south_hcp_in_worlds(&reader(&band(20, 37), false));
    assert!(!sane.is_empty() && !absurd.is_empty());
    assert!(sane.iter().all(|&hcp| hcp <= 14), "{sane:?}");
    assert!(absurd.iter().all(|&hcp| hcp >= 18), "{absurd:?}");
    let legacy_sane = south_hcp_in_worlds(&reader(&band(9, 14), true));
    let legacy_absurd = south_hcp_in_worlds(&reader(&band(20, 37), true));
    assert_eq!(
        legacy_sane, legacy_absurd,
        "the legacy sampler never saw the band"
    );
}

/// The leads themselves: `(1NT) 2♥` deals (East 15–17 balanced, South a
/// 9–14 overcall with 5+ hearts), led by West under each band.
fn west_leads(reader: &TableReader, deals: &[FullDeal]) -> Vec<contract_bridge::Card> {
    let inferences = leader_inferences(WE_DECLARE, Seat::East, reader);
    let questions: Vec<_> = deals
        .iter()
        .map(|&deal| LeadQuestion {
            deal,
            strain: contract_bridge::Strain::Hearts,
            declarer: Seat::South,
            inferences: inferences.clone(),
        })
        .collect();
    let mut rng = StdRng::seed_from_u64(1);
    single_dummy_leads(&questions, &mut rng, 16)
        .into_iter()
        .map(|(lead, _)| lead)
        .collect()
}

/// The band changes the blind lead, and the legacy reader never did.  On 300
/// such deals (seed 7) the disclosed reader moved 195 leads and 48 trick
/// counts between `9:14` and `20:37`; the legacy reader moved none.
#[test]
fn absurd_band_moves_the_lead() {
    let mut deal_rng = StdRng::seed_from_u64(7);
    let mut deals = Vec::new();
    while deals.len() < 8 {
        let deal = contract_bridge::deck::full_deal(&mut deal_rng);
        let (east, south) = (deal[Seat::East], deal[Seat::South]);
        let balanced = Suit::ASC.iter().all(|&suit| east[suit].len() >= 2)
            && Suit::ASC
                .iter()
                .filter(|&&suit| east[suit].len() == 2)
                .count()
                <= 1;
        if balanced
            && (15..=17).contains(&common::hand_hcp(east))
            && (9..=14).contains(&common::hand_hcp(south))
            && south[Suit::Hearts].len() >= 5
        {
            deals.push(deal);
        }
    }
    let leads = |lo, hi, legacy| west_leads(&reader(&band(lo, hi), legacy), &deals);
    assert_eq!(leads(9, 14, true), leads(20, 37, true));
    assert_ne!(leads(9, 14, false), leads(20, 37, false));
}

/// The European-minors declaration (read off each arm's dump) reaches the
/// North–South leader:
/// over their `(1NT) - (2NT) - (3NT)` South reads West's `2NT` as our Puppet
/// diamond transfer by default, and as the European invitation declared
#[test]
fn european_minors_reach_the_north_south_leader() {
    const INVITE: &str = "1NT P 2NT P 3NT P P P";
    let agreements = Agreements::default();
    let symmetric = shown(
        INVITE,
        Seat::East,
        &reader(&agreements, false),
        Relative::Lho,
    );
    let declared = shown(
        INVITE,
        Seat::East,
        &TableReader::new(common::european_minors_opponents(
            american(&agreements).bind(),
            &agreements,
        )),
        Relative::Lho,
    );
    assert!(symmetric.length(Suit::Diamonds).min >= 5, "{symmetric:?}");
    assert!(declared.length(Suit::Diamonds).min < 5, "{declared:?}");
}
