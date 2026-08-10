use super::*;
use crate::bidding::american::american_book;
use crate::bidding::inference::AuthoringStepCache;
use crate::bidding::trie::Provenance;
use contract_bridge::{Bid, Level, Strain};
use rand::{RngExt as _, SeedableRng as _};

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

fn assert_legal_mask(auction: &Auction, state: LegalCalls) {
    assert_eq!(state.mask, LegalCalls::from_auction(auction).mask);
    for (call, _) in Logits::new().iter() {
        assert_eq!(
            state.allows(call),
            auction.can_push(call).is_ok(),
            "legality differs for {call:?} after {auction:?}"
        );
    }
}

#[test]
fn incremental_legal_mask_matches_laws_on_random_prefixes() {
    for seed in 0..256 {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut auction = Auction::new();
        let mut state = LegalCalls::new();
        for _ in 0..64 {
            assert_legal_mask(&auction, state);
            if auction.has_ended() {
                break;
            }
            let legal: Vec<_> = Logits::new()
                .iter()
                .map(|(call, _)| call)
                .filter(|&call| auction.can_push(call).is_ok())
                .collect();
            let call = legal[rng.random_range(0..legal.len())];
            auction.push(call);
            state.push(call);
        }
    }
}

fn parse_fixture_call(text: &str) -> Call {
    match text {
        "P" => Call::Pass,
        "X" => Call::Double,
        "XX" => Call::Redouble,
        _ => {
            let bytes = text.as_bytes();
            assert_eq!(bytes.len(), 2, "invalid frozen-corpus call {text}");
            let level = bytes[0] - b'0';
            let strain = match bytes[1] {
                b'C' => Strain::Clubs,
                b'D' => Strain::Diamonds,
                b'H' => Strain::Hearts,
                b'S' => Strain::Spades,
                b'N' => Strain::Notrump,
                _ => panic!("invalid frozen-corpus strain in {text}"),
            };
            bid(level, strain)
        }
    }
}

#[test]
fn legal_mask_matches_laws_at_every_frozen_corpus_prefix() {
    const CORPUS: &str = include_str!("../../../benches/fixtures/bidding-performance.tsv");
    for line in CORPUS.lines().filter(|line| !line.starts_with('#')) {
        if line.trim().is_empty() {
            continue;
        }
        let auction_text = line
            .split('\t')
            .nth(6)
            .expect("frozen corpus has an auction column");
        let mut auction = Auction::new();
        let mut state = LegalCalls::new();
        assert_legal_mask(&auction, state);
        for call in auction_text
            .split_ascii_whitespace()
            .map(parse_fixture_call)
        {
            auction.push(call);
            state.push(call);
            assert_legal_mask(&auction, state);
        }
    }
}

#[test]
fn legal_selection_preserves_order_ties_and_filters_nonfinite_calls() {
    let mut auction = Auction::new();
    auction.push(bid(1, Strain::Clubs));
    let mut logits = Logits::new();
    logits[bid(1, Strain::Clubs)] = 100.0; // illegal insufficient bid
    logits[Call::Double] = 2.0;
    logits[bid(1, Strain::Diamonds)] = 2.0;
    logits[Call::Pass] = 1.0;
    assert_eq!(select_legal_call(Some(logits), &auction), Call::Double);

    logits[Call::Double] = f32::NAN;
    assert_eq!(
        select_legal_call(Some(logits), &auction),
        bid(1, Strain::Diamonds)
    );

    let ended: Auction = "- - - -".parse().expect("four passes are legal");
    assert_eq!(select_legal_call(Some(logits), &ended), Call::Pass);
    assert_eq!(select_legal_call(None, &auction), Call::Pass);

    let mut doubled = Auction::new();
    let mut doubled_state = LegalCalls::new();
    for call in [bid(1, Strain::Clubs), Call::Double] {
        doubled.push(call);
        doubled_state.push(call);
    }
    assert_legal_mask(&doubled, doubled_state);
    assert!(doubled_state.allows(Call::Redouble));
    doubled.push(Call::Redouble);
    doubled_state.push(Call::Redouble);
    assert_legal_mask(&doubled, doubled_state);
    assert!(!doubled_state.allows(Call::Double));
    assert_legal_mask(&ended, LegalCalls::from_auction(&ended));
    assert_eq!(LegalCalls::from_auction(&ended).mask, 0);
}

#[derive(Clone, Copy, Debug, Default)]
struct DealCacheCoverage {
    decisions: usize,
    successful_prepares: usize,
    appended_steps: usize,
}

impl DealCacheCoverage {
    fn assert_substantial(self) {
        assert!(
            self.successful_prepares * 2 >= self.decisions,
            "deal cache served fewer than half the decisions: {self:?}",
        );
        assert!(
            self.appended_steps * 2 >= self.decisions,
            "deal cache processed too little auction history: {self:?}",
        );
    }
}

fn assert_same_classification(
    board: usize,
    auction: &[Call],
    expected: Option<(Logits, Provenance)>,
    actual: Option<(Logits, Provenance)>,
) {
    match (expected, actual) {
        (None, None) => {}
        (Some((expected, expected_provenance)), Some((actual, actual_provenance))) => {
            assert_eq!(
                actual_provenance, expected_provenance,
                "deal-cache provenance divergence on board {board} at {auction:?}",
            );
            for ((expected_call, expected), (actual_call, actual)) in
                expected.iter().zip(actual.iter())
            {
                assert_eq!(actual_call, expected_call);
                assert_eq!(
                    actual.to_bits(),
                    expected.to_bits(),
                    "deal-cache logit divergence on board {board} at {auction:?}, call {expected_call:?}",
                );
            }
        }
        (expected, actual) => panic!(
            "deal-cache classification presence divergence on board {board} at {auction:?}: reference={}, cached={}",
            expected.is_some(),
            actual.is_some(),
        ),
    }
}

fn assert_deal_cache_parity(count: usize, seed: u64) -> DealCacheCoverage {
    let partnership = american_book(&crate::bidding::agreements::Agreements::default()).bind();
    let vulnerabilities = [
        AbsoluteVulnerability::NONE,
        AbsoluteVulnerability::NS,
        AbsoluteVulnerability::EW,
        AbsoluteVulnerability::ALL,
    ];
    let mut coverage = DealCacheCoverage::default();
    for board in 0..count {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed.wrapping_add(board as u64));
        let deal = contract_bridge::deck::full_deal(&mut rng);
        let table = Table::new(
            partnership.clone(),
            partnership.clone(),
            Seat::ALL[board % 4],
            vulnerabilities[board % 4],
        );
        let mut caches = [AuthoringStepCache::new(), AuthoringStepCache::new()];
        let mut auction = Auction::new();
        let mut legal = LegalCalls::new();
        while !auction.has_ended() {
            assert_legal_mask(&auction, legal);
            coverage.decisions += 1;
            let seat = table.seat_to_act(auction.len());
            let vul = relative(table.vul, seat);
            let hand = deal[seat];
            let side = usize::from(matches!(seat, Seat::East | Seat::West));
            let expected = partnership.classify_with_provenance(hand, vul, &auction);
            let actual = partnership.classify_with_step_cache_provenance(
                hand,
                vul,
                &auction,
                &mut caches[side],
            );
            assert_same_classification(board, &auction, expected, actual);
            let call = select_legal_call(actual.map(|(logits, _)| logits), &auction);
            auction.push(call);
            legal.push(call);
        }
        for cache in &caches {
            let (successful_prepares, appended_steps) = cache.coverage();
            coverage.successful_prepares += successful_prepares;
            coverage.appended_steps += appended_steps;
        }
    }
    coverage
}

#[test]
fn deal_cache_preserves_whole_auction() {
    assert_deal_cache_parity(128, 0x5EED_CA5E).assert_substantial();
}

#[test]
#[ignore = "release acceptance sweep over 20,000 seeded deals"]
fn deal_cache_preserves_twenty_thousand_auctions() {
    assert_deal_cache_parity(20_000, 1).assert_substantial();
}
