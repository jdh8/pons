//! A full table: two systems in absolute seats
//!
//! [`Table`] seats one [`Bidder`] as North/South and another as East/West,
//! fixes the dealer and the absolute vulnerability, and drives the auction:
//! it rotates the seat to act, converts the vulnerability to the side to act
//! (once per call, with [`relative`]), filters illegal calls, and bids a deal
//! out.
//!
//! A table deliberately does **not** implement [`Bidder`]: that trait speaks
//! relative vulnerability and leaves seats to the caller, while a table owns
//! both.  For a dealer-relative, vulnerability-agnostic composition, use
//! [`Bidder::vs`] instead.

use super::Bidder;
use super::array::{CALL_VARIANTS, Logits, encode_call};
use super::book::{Partnership, System};
use super::context::relative;
use super::trie::Provenance;
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

/// The call a seat holding `logits` makes at `auction` — production's selector
///
/// Keeps only the finite logits that are legal here and takes the **first**
/// maximum, so a tie goes to the earlier call in [`Logits`] order.  `None`
/// logits, and a set with nothing finite and legal, both answer [`Call::Pass`].
///
/// Public because every probe that reports what the policy "would have bid"
/// has to agree with this function exactly; reimplementing it locally is how a
/// census silently drifts (`Iterator::max_by` breaks a tie the other way).
#[must_use]
pub fn select_legal_call(logits: Option<Logits>, auction: &Auction) -> Call {
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

impl<N: Bidder, E: Bidder> Table<N, E> {
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

    /// Whether the seat to act's *node* is authored rather than the floor
    ///
    /// The routing twin of [`Bidder::authored_at`]: same seat rotation, same
    /// absolute-to-relative vulnerability conversion.  This is a fact about
    /// the node, not about any hand: it names the classifier the auction
    /// resolves to.  A hand that classifier rejects (all `-∞` — a
    /// deliberately partial table) is served by the fallback chain instead,
    /// so [`classify`][Self::classify] may answer from a different
    /// classifier than this names.  A display choosing how to show a hand's
    /// logits — the **ladder** (rung logits in nats: a precedence, not odds)
    /// at an authored node, a policy whose masked softmax is a genuine
    /// probability at the floor — must read that off the provenance of the
    /// logits it shows, via
    /// [`classify_with_provenance`][Self::classify_with_provenance] and
    /// [`Provenance::is_authored`]; see `docs/ai-bidder/logit-calibration.md`
    /// for why the two scales must not be shown alike.
    #[must_use]
    pub fn authored_at(&self, auction: &[Call]) -> bool {
        let seat = self.seat_to_act(auction.len());
        let vul = relative(self.vul, seat);

        match seat {
            Seat::North | Seat::South => self.north_south.authored_at(vul, auction),
            Seat::East | Seat::West => self.east_west.authored_at(vul, auction),
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

impl Table<Partnership, Partnership> {
    /// Seat two systems, binding each into its [`Partnership`]
    ///
    /// This is the usual table assembly: each system is bound with
    /// [`bind`][System::bind].
    #[must_use]
    pub fn of_systems(ns: &System, ew: &System, dealer: Seat, vul: AbsoluteVulnerability) -> Self {
        Self::new(ns.bind(), ew.bind(), dealer, vul)
    }

    /// [`classify`][Self::classify] with the resolution [`Provenance`] —
    /// where the answer came from
    ///
    /// Same seat rotation and absolute-to-relative vulnerability conversion,
    /// delegating to
    /// [`Partnership::classify_with_provenance`][Partnership::classify_with_provenance].
    /// The provenance is the per-hand truth about which classifier answered:
    /// an authored node that gives `hand` no mass falls through to the
    /// fallback chain, possibly to the keyless floor, and only the provenance
    /// says which.  [`Provenance::is_authored`] turns it into the
    /// ladder-or-odds choice a display needs.
    #[must_use]
    pub fn classify_with_provenance(
        &self,
        hand: Hand,
        auction: &[Call],
    ) -> Option<(Logits, Provenance)> {
        let seat = self.seat_to_act(auction.len());
        let vul = relative(self.vul, seat);

        match seat {
            Seat::North | Seat::South => self
                .north_south
                .classify_with_provenance(hand, vul, auction),
            Seat::East | Seat::West => self.east_west.classify_with_provenance(hand, vul, auction),
        }
    }

    /// Read what `auction` has shown, from the seat about to act
    ///
    /// The routing twin of [`classify`][Self::classify]: same seat rotation,
    /// same absolute-to-relative vulnerability conversion, but it returns the
    /// shown ranges instead of the logits.  Goes through
    /// [`Partnership::infer`][Partnership::infer], **not** a bare
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
mod tests;
