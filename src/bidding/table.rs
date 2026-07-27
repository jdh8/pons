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
use super::array::Logits;
use super::book::{Pair, Stance};
use super::context::relative;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};

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
        let Some(logits) = self.classify(hand, auction) else {
            return Call::Pass;
        };

        let mut scored: Vec<(Call, f32)> = logits
            .iter()
            .map(|(call, &logit)| (call, logit))
            .filter(|&(_, logit)| logit.is_finite())
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));

        scored
            .into_iter()
            .map(|(call, _)| call)
            .find(|&call| auction.can_push(call).is_ok())
            .unwrap_or(Call::Pass)
    }

    /// Continue a seeded auction until it ends
    ///
    /// Call `i` of the seed is attributed to
    /// [`seat_to_act(i)`][Self::seat_to_act], i.e. the seed is positioned
    /// from the dealer.  A seed that has already ended is returned unchanged.
    /// [`bid_out`][Self::bid_out] is this with an empty seed.
    #[must_use]
    pub fn bid_out_from(&self, deal: &FullDeal, mut auction: Auction) -> Auction {
        while !auction.has_ended() {
            let seat = self.seat_to_act(auction.len());
            auction.push(self.next_call(deal[seat], &auction));
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
