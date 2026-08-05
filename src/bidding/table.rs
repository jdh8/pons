//! A full table: two systems in absolute seats
//!
//! [`Table`] seats one [`System`] as North/South and another as East/West,
//! fixes the dealer and the absolute vulnerability, and drives the auction:
//! it rotates the seat to act, converts the vulnerability to the side to act
//! (once per call, with [`relative`]), filters illegal calls, and bids a deal
//! out.
//!
//! A table deliberately does **not** implement [`System`]: that trait speaks
//! relative vulnerability and leaves seats to the caller, while a table owns
//! both.  For a dealer-relative, vulnerability-agnostic composition, use
//! [`System::vs`] instead.

use super::System;
use super::array::{CALL_VARIANTS, Logits, encode_call};
use super::book::{Pair, Stance};
use super::context::relative;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};

const ALL_CALLS_MASK: u64 = (1_u64 << CALL_VARIANTS) - 1;
const ALL_BIDS_MASK: u64 = ALL_CALLS_MASK & !0b111;

/// Incremental laws state for the side about to act.
#[derive(Clone, Copy, Debug)]
struct LegalCalls {
    mask: u64,
    len: usize,
    trailing_passes: usize,
    last_bid: Option<contract_bridge::Bid>,
    last_nonpass: Option<(usize, Call)>,
    ended: bool,
}

impl LegalCalls {
    const fn new() -> Self {
        let mut state = Self {
            mask: 0,
            len: 0,
            trailing_passes: 0,
            last_bid: None,
            last_nonpass: None,
            ended: false,
        };
        state.refresh_mask();
        state
    }

    fn from_auction(auction: &Auction) -> Self {
        let mut state = Self::new();
        for &call in auction.iter() {
            state.push(call);
        }
        debug_assert_eq!(state.ended, auction.has_ended());
        state
    }

    const fn allows(self, call: Call) -> bool {
        self.mask & (1_u64 << encode_call(call)) != 0
    }

    fn push(&mut self, call: Call) {
        let index = self.len;
        self.len += 1;
        if call == Call::Pass {
            self.trailing_passes += 1;
        } else {
            self.trailing_passes = 0;
            self.last_nonpass = Some((index, call));
            if let Call::Bid(bid) = call {
                self.last_bid = Some(bid);
            }
        }
        self.ended = self.len >= 4 && self.trailing_passes >= 3;
        self.refresh_mask();
    }

    const fn refresh_mask(&mut self) {
        if self.ended {
            self.mask = 0;
            return;
        }

        let bids = match self.last_bid {
            None => ALL_BIDS_MASK,
            Some(bid) => {
                let through_last = (1_u64 << (encode_call(Call::Bid(bid)) + 1)) - 1;
                ALL_BIDS_MASK & !through_last
            }
        };
        let mut mask = bids | 1_u64 << encode_call(Call::Pass);
        if let Some((index, call)) = self.last_nonpass
            && index % 2 != self.len % 2
        {
            match call {
                Call::Bid(_) => mask |= 1_u64 << encode_call(Call::Double),
                Call::Double => mask |= 1_u64 << encode_call(Call::Redouble),
                Call::Pass | Call::Redouble => {}
            }
        }
        self.mask = mask;
    }
}

fn select_with_legal_state(logits: Option<Logits>, legal: LegalCalls) -> Call {
    let Some(logits) = logits else {
        return Call::Pass;
    };
    let mut best: Option<(Call, f32)> = None;
    for (call, &logit) in logits.iter() {
        if !logit.is_finite() || !legal.allows(call) {
            continue;
        }
        if best.is_none_or(|(_, best_logit)| logit > best_logit) {
            best = Some((call, logit));
        }
    }
    best.map_or(Call::Pass, |(call, _)| call)
}

/// Production legal-call selection exposed to the benchmark adapter.
pub(crate) fn select_legal_call(logits: Option<Logits>, auction: &Auction) -> Call {
    select_with_legal_state(logits, LegalCalls::from_auction(auction))
}

/// Two systems seated at a table with a dealer and vulnerability
///
/// The type parameters are the North/South and East/West systems; see the
/// [module docs][self] for what a table does and does not do.
#[derive(Clone, Debug)]
pub struct Table<N, E> {
    north_south: N,
    east_west: E,
    dealer: Seat,
    vul: AbsoluteVulnerability,
}

impl<N: System, E: System> Table<N, E> {
    /// Seat two systems with a dealer and an absolute vulnerability
    #[must_use]
    pub const fn new(
        north_south: N,
        east_west: E,
        dealer: Seat,
        vul: AbsoluteVulnerability,
    ) -> Self {
        Self {
            north_south,
            east_west,
            dealer,
            vul,
        }
    }

    /// The seat acting after `len` calls
    #[must_use]
    pub const fn seat_to_act(&self, len: usize) -> Seat {
        Seat::ALL[(self.dealer as usize + len) % 4]
    }

    /// Classify a hand for the seat to act
    ///
    /// Routes to the side of [`seat_to_act`][Self::seat_to_act] and converts
    /// the table's absolute vulnerability to that side's perspective.
    #[must_use]
    pub fn classify(&self, hand: Hand, auction: &[Call]) -> Option<Logits> {
        let seat = self.seat_to_act(auction.len());
        let vul = relative(self.vul, seat);

        match seat {
            Seat::North | Seat::South => self.north_south.classify(hand, vul, auction),
            Seat::East | Seat::West => self.east_west.classify(hand, vul, auction),
        }
    }

    /// The highest-logit *legal* call, defaulting to a pass
    ///
    /// An auction the system does not cover — or covers only with illegal
    /// calls — resolves to a pass, so the bidding always terminates.
    // ponytail: the `partial_cmp` expect cannot fire — the preceding
    // `is_finite` filter leaves only non-NaN logits to compare.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn next_call(&self, hand: Hand, auction: &Auction) -> Call {
        select_legal_call(self.classify(hand, auction), auction)
    }

    /// Continue a seeded auction until it ends
    ///
    /// Call `i` of the seed is attributed to
    /// [`seat_to_act(i)`][Self::seat_to_act], i.e. the seed is positioned
    /// from the dealer.  A seed that has already ended is returned unchanged.
    /// [`bid_out`][Self::bid_out] is this with an empty seed.
    #[must_use]
    pub fn bid_out_from(&self, deal: &FullDeal, mut auction: Auction) -> Auction {
        let mut north_south_state = self.north_south.new_deal_state();
        let mut east_west_state = self.east_west.new_deal_state();
        let mut legal = LegalCalls::from_auction(&auction);
        while !auction.has_ended() {
            let seat = self.seat_to_act(auction.len());
            let vul = relative(self.vul, seat);
            let logits = match seat {
                Seat::North | Seat::South => self.north_south.classify_in_deal(
                    deal[seat],
                    vul,
                    &auction,
                    north_south_state.as_deref_mut(),
                ),
                Seat::East | Seat::West => self.east_west.classify_in_deal(
                    deal[seat],
                    vul,
                    &auction,
                    east_west_state.as_deref_mut(),
                ),
            };
            let call = select_with_legal_state(logits, legal);
            auction.push(call);
            legal.push(call);
        }
        auction
    }

    /// Bid out a deal from the dealer until the auction ends
    #[must_use]
    pub fn bid_out(&self, deal: &FullDeal) -> Auction {
        self.bid_out_from(deal, Auction::new())
    }
}

impl Table<Stance, Stance> {
    /// Seat two pairs, binding each into its [`Stance`]
    ///
    /// This is the usual table assembly: each pair is bound with
    /// [`against`][Pair::against].
    #[must_use]
    pub fn of_pairs(ns: &Pair, ew: &Pair, dealer: Seat, vul: AbsoluteVulnerability) -> Self {
        Self::new(ns.against(), ew.against(), dealer, vul)
    }

    /// Read what `auction` has shown, from the seat about to act
    ///
    /// The routing twin of [`classify`][Self::classify]: same seat rotation,
    /// same absolute-to-relative vulnerability conversion, but it returns the
    /// shown ranges instead of the logits.  Goes through
    /// [`Stance::infer`][Stance::infer], **not** a bare
    /// [`Inferences::read`][super::Inferences::read] — a keyless context
    /// silently skips every projection-based reading and hands back a vacuous
    /// `0..=37`.  Consumers wanting what the bidder actually sees must enter
    /// here.
    #[must_use]
    pub fn infer(&self, auction: &[Call]) -> super::Inferences {
        let seat = self.seat_to_act(auction.len());
        let vul = relative(self.vul, seat);

        match seat {
            Seat::North | Seat::South => self.north_south.infer(vul, auction),
            Seat::East | Seat::West => self.east_west.infer(vul, auction),
        }
    }
}

#[cfg(test)]
mod tests {
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
        const CORPUS: &str = include_str!("../../benches/fixtures/bidding-performance.tsv");
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
        let stance = american_book().against();
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
                stance.clone(),
                stance.clone(),
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
                let expected = stance.classify_with_provenance(hand, vul, &auction);
                let actual = stance.classify_with_step_cache_provenance(
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
}
