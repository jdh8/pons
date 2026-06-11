//! Mechanical context of an auction
//!
//! [`Context`] packages everything a [`Classifier`][super::trie::Classifier]
//! or a [`Constraint`][super::constraint::Constraint] may consult besides the
//! hand itself: vulnerability, the raw table auction, and facts derived from
//! it (who bid which strains, the contract to beat, passed-hand status).
//!
//! All facts here are *mechanical*: they follow from the laws of the game
//! alone.  System interpretation (such as forcing status) deliberately does
//! not live here — it belongs to classifiers, which know their system.

use super::trie::CommonPrefixes;
use contract_bridge::auction::{AbsoluteVulnerability, Call, RelativeVulnerability};
use contract_bridge::{Bid, Level, Penalty, Seat, Strain, Suit};

/// Convert absolute vulnerability to the perspective of a seat
///
/// This is the only vulnerability conversion in the crate: drivers call it
/// once per [`classify`][super::System::classify] call, and systems pass the
/// relative value through unchanged.
#[must_use]
pub fn relative(vul: AbsoluteVulnerability, seat: Seat) -> RelativeVulnerability {
    let (we, they) = match seat {
        Seat::North | Seat::South => (AbsoluteVulnerability::NS, AbsoluteVulnerability::EW),
        Seat::East | Seat::West => (AbsoluteVulnerability::EW, AbsoluteVulnerability::NS),
    };
    let mut relative = RelativeVulnerability::NONE;
    relative.set(RelativeVulnerability::WE, vul.contains(we));
    relative.set(RelativeVulnerability::THEY, vul.contains(they));
    relative
}

/// Mechanical facts about an auction from the perspective of the side to act
///
/// A context is computed once per classification from the raw table auction
/// (all four players' calls).  "We" always refers to the partnership of the
/// player about to call, and the vulnerability is relative to that side.
#[derive(Clone, Debug)]
pub struct Context<'a> {
    vul: RelativeVulnerability,
    auction: &'a [Call],
    our_strains: u8,
    their_strains: u8,
    partner_last_bid: Option<Bid>,
    last_bid: Option<Bid>,
    penalty: Penalty,
    undisturbed: bool,
    passed_hand: bool,
    partner_passed_hand: bool,
    opening_index: Option<usize>,
    prefixes: Option<CommonPrefixes<'a, 'a>>,
}

impl<'a> Context<'a> {
    /// Compute the context of an auction
    ///
    /// `vul` must be relative to the side to act, and `auction` must be the
    /// raw table auction including all passes.
    #[must_use]
    pub fn new(vul: RelativeVulnerability, auction: &'a [Call]) -> Self {
        let len = auction.len();
        let mut context = Self {
            vul,
            auction,
            our_strains: 0,
            their_strains: 0,
            partner_last_bid: None,
            last_bid: None,
            penalty: Penalty::Undoubled,
            undisturbed: true,
            passed_hand: len >= 4 && matches!(auction[len % 4], Call::Pass),
            partner_passed_hand: len >= 2 && matches!(auction[(len - 2) % 4], Call::Pass),
            opening_index: auction.iter().position(|&call| call != Call::Pass),
            prefixes: None,
        };

        for (index, &call) in auction.iter().enumerate() {
            let ours = (len - index).is_multiple_of(2);

            match call {
                Call::Pass => {}
                Call::Double => {
                    context.penalty = Penalty::Doubled;
                    context.undisturbed &= ours;
                }
                Call::Redouble => {
                    context.penalty = Penalty::Redoubled;
                    context.undisturbed &= ours;
                }
                Call::Bid(bid) => {
                    context.last_bid = Some(bid);
                    context.penalty = Penalty::Undoubled;
                    context.undisturbed &= ours;

                    if ours {
                        context.our_strains |= 1 << bid.strain as u8;
                        if (len - index) % 4 == 2 {
                            context.partner_last_bid = Some(bid);
                        }
                    } else {
                        context.their_strains |= 1 << bid.strain as u8;
                    }
                }
            }
        }
        context
    }

    /// Attach the common prefixes of the auction in the queried [`Trie`]
    ///
    /// [`Trie`]: super::Trie
    #[must_use]
    pub fn with_prefixes(mut self, prefixes: CommonPrefixes<'a, 'a>) -> Self {
        self.prefixes = Some(prefixes);
        self
    }

    /// Vulnerability relative to the side to act
    #[must_use]
    pub const fn vul(self: &Context<'a>) -> RelativeVulnerability {
        self.vul
    }

    /// The raw table auction
    #[must_use]
    pub const fn auction(&self) -> &'a [Call] {
        self.auction
    }

    /// Whether our side has bid the strain
    #[must_use]
    pub const fn we_bid(&self, strain: Strain) -> bool {
        self.our_strains & (1 << strain as u8) != 0
    }

    /// Whether the opponents have bid the strain
    #[must_use]
    pub const fn they_bid(&self, strain: Strain) -> bool {
        self.their_strains & (1 << strain as u8) != 0
    }

    /// Iterate over the suits the opponents have bid
    pub fn their_suits(&self) -> impl Iterator<Item = Suit> + use<> {
        let strains = self.their_strains;
        Suit::ASC
            .into_iter()
            .filter(move |&suit| strains & (1 << Strain::from(suit) as u8) != 0)
    }

    /// The last bid made by partner, if any
    #[must_use]
    pub const fn partner_last_bid(&self) -> Option<Bid> {
        self.partner_last_bid
    }

    /// The suit of partner's last bid, if it was a suit bid
    #[must_use]
    pub fn partner_last_suit(&self) -> Option<Suit> {
        self.partner_last_bid.and_then(|bid| bid.strain.suit())
    }

    /// The highest bid so far — the contract to beat
    #[must_use]
    pub const fn last_bid(&self) -> Option<Bid> {
        self.last_bid
    }

    /// Doubling state of the last bid
    #[must_use]
    pub const fn penalty(&self) -> Penalty {
        self.penalty
    }

    /// Whether the opponents have made nothing but passes
    #[must_use]
    pub const fn undisturbed(&self) -> bool {
        self.undisturbed
    }

    /// Whether the player to act passed on their first turn
    #[must_use]
    pub const fn passed_hand(&self) -> bool {
        self.passed_hand
    }

    /// Whether partner passed on their first turn
    #[must_use]
    pub const fn partner_passed_hand(&self) -> bool {
        self.partner_passed_hand
    }

    /// Number of passes before the first non-pass call
    #[must_use]
    pub fn leading_passes(&self) -> usize {
        self.opening_index.unwrap_or(self.auction.len())
    }

    /// The seat number (1–4) about to make the first non-pass call
    ///
    /// Returns [`None`] once anyone has acted, or when the auction has been
    /// passed out.
    #[must_use]
    pub fn seat_to_open(&self) -> Option<u8> {
        // SAFETY: the auction length is at most 3 here, so the cast is safe.
        #[allow(clippy::cast_possible_truncation)]
        (self.opening_index.is_none() && self.auction.len() < 4)
            .then(|| self.auction.len() as u8 + 1)
    }

    /// The seat number (1–4) of the first non-pass call, if any
    #[must_use]
    pub fn opener_seat(&self) -> Option<u8> {
        // SAFETY: at most 3 passes may precede the opening, so the cast is safe.
        #[allow(clippy::cast_possible_truncation)]
        self.opening_index.map(|index| index as u8 + 1)
    }

    /// The cheapest level at which the strain can legally be bid
    ///
    /// Returns [`None`] when no bid in the strain is available anymore.
    #[must_use]
    pub fn min_level(&self, strain: Strain) -> Option<Level> {
        match self.last_bid {
            None => Some(Level::new(1)),
            Some(last) if strain > last.strain => Some(last.level),
            Some(last) => Level::try_new(last.level.get() + 1).ok(),
        }
    }

    /// Common prefixes of the auction in the queried [`Trie`], if attached
    ///
    /// [`Trie`]: super::Trie
    #[must_use]
    pub const fn prefixes(&self) -> Option<&CommonPrefixes<'a, 'a>> {
        self.prefixes.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid {
            level: Level::new(level),
            strain,
        })
    }

    #[test]
    fn test_relative_vulnerability() {
        assert_eq!(
            relative(AbsoluteVulnerability::NS, Seat::North),
            RelativeVulnerability::WE,
        );
        assert_eq!(
            relative(AbsoluteVulnerability::NS, Seat::East),
            RelativeVulnerability::THEY,
        );
        assert_eq!(
            relative(AbsoluteVulnerability::ALL, Seat::West),
            RelativeVulnerability::ALL,
        );
        assert_eq!(
            relative(AbsoluteVulnerability::NONE, Seat::South),
            RelativeVulnerability::NONE,
        );
    }

    #[test]
    fn test_contested_auction_facts() {
        // We opened 1♠, LHO passed, partner bid 2♣, RHO doubled; we act next.
        let auction = [
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Double,
        ];
        let context = Context::new(RelativeVulnerability::NONE, &auction);

        assert!(context.we_bid(Strain::Spades));
        assert!(context.we_bid(Strain::Clubs));
        assert!(!context.they_bid(Strain::Spades));
        assert_eq!(context.partner_last_bid(), Some(Bid::new(2, Strain::Clubs)));
        assert_eq!(context.partner_last_suit(), Some(Suit::Clubs));
        assert_eq!(context.last_bid(), Some(Bid::new(2, Strain::Clubs)));
        assert_eq!(context.penalty(), Penalty::Doubled);
        assert!(!context.undisturbed());
        assert!(!context.passed_hand());
        assert!(!context.partner_passed_hand());
        assert_eq!(context.opener_seat(), Some(1));
    }

    #[test]
    fn test_their_suits_and_min_level() {
        // They opened 1♥ and raised to 2♥ over partner's 1♠ overcall.
        let auction = [
            bid(1, Strain::Hearts),
            bid(1, Strain::Spades),
            bid(2, Strain::Hearts),
        ];
        let context = Context::new(RelativeVulnerability::NONE, &auction);

        assert_eq!(context.their_suits().collect::<Vec<_>>(), [Suit::Hearts]);
        assert!(context.we_bid(Strain::Spades));
        assert_eq!(context.min_level(Strain::Hearts), Some(Level::new(3)));
        assert_eq!(context.min_level(Strain::Spades), Some(Level::new(2)));
        assert_eq!(context.min_level(Strain::Clubs), Some(Level::new(3)));
    }

    #[test]
    fn test_min_level_exhausted() {
        let auction = [bid(7, Strain::Notrump)];
        let context = Context::new(RelativeVulnerability::NONE, &auction);
        assert_eq!(context.min_level(Strain::Spades), None);
        assert_eq!(context.min_level(Strain::Notrump), None);
    }

    #[test]
    fn test_passed_hands() {
        // We passed, they overcalled 1♠ over partner's 1♥; we act next.
        let auction = [
            Call::Pass,
            Call::Pass,
            bid(1, Strain::Hearts),
            bid(1, Strain::Spades),
        ];
        let context = Context::new(RelativeVulnerability::NONE, &auction);

        assert!(context.passed_hand());
        assert!(!context.partner_passed_hand());
        assert_eq!(context.leading_passes(), 2);
        assert_eq!(context.opener_seat(), Some(3));
        assert_eq!(context.seat_to_open(), None);
    }

    #[test]
    fn test_seat_to_open() {
        let passes = [Call::Pass; 4];

        for len in 0..=3 {
            let context = Context::new(RelativeVulnerability::NONE, &passes[..len]);
            // SAFETY: `len` is at most 3, so the cast is safe.
            #[allow(clippy::cast_possible_truncation)]
            let seat = len as u8 + 1;
            assert_eq!(context.seat_to_open(), Some(seat));
        }

        let passed_out = Context::new(RelativeVulnerability::NONE, &passes);
        assert_eq!(passed_out.seat_to_open(), None);
    }
}
