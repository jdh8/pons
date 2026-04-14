use core::iter::FusedIterator;
use dds_bridge::{Card, Deal, Hand, Seat};
use rand::{Rng, RngExt as _};
use thiserror::Error;

/// The deal is not a valid subset of a bridge deal
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("The deal is not a valid subset of a bridge deal")]
pub struct InvalidDeal;

/// A subset of the standard 52-card deck
///
/// This is a set of unique cards backed by [`Hand`].  Duplicates are
/// structurally impossible.  It requires shuffling to partially retrieve
/// cards from the deck.  However, it is deterministic to collect all cards.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Deck(Hand);

impl Deck {
    /// The standard 52-card deck
    pub const ALL: Self = Self(Hand::ALL);

    /// An empty deck
    pub const EMPTY: Self = Self(Hand::EMPTY);

    /// The number of cards currently in the deck
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the deck is empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clear the deck, removing all the cards.
    pub const fn clear(&mut self) {
        self.0 = Hand::EMPTY;
    }

    /// Insert a card into the deck
    ///
    /// Returns `true` if the card was newly inserted, `false` if it was
    /// already present.
    pub fn insert(&mut self, card: Card) -> bool {
        self.0.insert(card)
    }

    /// Take the remaining cards in the deck into a hand.
    #[must_use]
    pub const fn take(&mut self) -> Hand {
        core::mem::replace(&mut self.0, Hand::EMPTY)
    }

    /// Randomly draw `n` cards from the deck and collect them into a hand.
    ///
    /// If `n >= self.len()`, all remaining cards are drawn without shuffling.
    #[must_use]
    pub fn draw(&mut self, rng: &mut (impl Rng + ?Sized), n: usize) -> Hand {
        let len = self.0.len();
        if n >= len {
            return self.take();
        }

        let mut hand = Hand::EMPTY;
        for i in 0..n {
            // Clear random number of lowest set bits so the new lowest is our pick.
            let bits = (0..rng.random_range(..len - i))
                .fold(self.0.to_bits(), |bits, _| bits & (bits - 1));
            // Isolate the lowest set bit and move it from deck to hand.
            let selected = Hand::from_bits_retain(bits & bits.wrapping_neg());
            hand |= selected;
            self.0 ^= selected;
        }
        hand
    }

    /// Randomly pop a card from the deck
    #[must_use]
    pub fn pop(&mut self, rng: &mut (impl Rng + ?Sized)) -> Option<Card> {
        self.draw(rng, 1).into_iter().next()
    }
}

impl From<Hand> for Deck {
    fn from(hand: Hand) -> Self {
        Self(hand)
    }
}

/// Shuffle and evenly deal 52 cards into 4 hands
#[must_use]
pub fn full_deal(rng: &mut (impl Rng + ?Sized)) -> Deal {
    let mut deck = Deck::ALL;

    Deal::new(
        deck.draw(rng, 13),
        deck.draw(rng, 13),
        deck.draw(rng, 13),
        deck.take(),
    )
}

/// An infinite iterator that fills undealt cards randomly into a partial deal.
///
/// Created by [`fill_deals`].
#[derive(Debug)]
pub struct FillDeals<'a, R: Rng + ?Sized> {
    rng: &'a mut R,
    deal: Deal,
    deck: Deck,

    /// The seat with the fewest cards in the initial deal.  This seat will be
    /// filled last to save entropy.
    shortest: Seat,
}

impl<R: Rng + ?Sized> Iterator for FillDeals<'_, R> {
    type Item = Deal;

    fn next(&mut self) -> Option<Deal> {
        let mut deck = self.deck;
        let mut deal = self.deal;
        let mut fill = |hand: &mut Hand| *hand |= deck.draw(self.rng, 13 - hand.len());

        fill(&mut deal[self.shortest.lho()]);
        fill(&mut deal[self.shortest.partner()]);
        fill(&mut deal[self.shortest.rho()]);

        deal[self.shortest] |= deck.take();
        Some(deal)
    }
}

impl<R: Rng + ?Sized> FusedIterator for FillDeals<'_, R> {}

/// Given a partial deal, return an iterator that fills in the remaining cards
/// randomly on each iteration.
///
/// # Errors
///
/// [`InvalidDeal`] if `deal` is invalid determined by
/// [`Deal::validate_and_collect`].
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub fn fill_deals<R: Rng + ?Sized>(
    rng: &mut R,
    deal: Deal,
) -> Result<FillDeals<'_, R>, InvalidDeal> {
    Ok(FillDeals {
        rng,
        deal,
        deck: Deck::from(!deal.validate_and_collect().ok_or(InvalidDeal)?),

        #[allow(clippy::missing_panics_doc)]
        shortest: Seat::ALL
            .into_iter()
            .min_by_key(|&seat| deal[seat].len())
            .expect("Seat::ALL shall not be empty"),
    })
}
