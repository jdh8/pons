use core::iter::FusedIterator;
use dds_bridge::{Card, Deal, Hand, Rank, Seat, Suit};

use arrayvec::{ArrayVec, CapacityError};
use rand::prelude::SliceRandom as _;
use rand::{Rng, RngExt as _};
use thiserror::Error;

/// Error while generating deals
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The deck is already full and cannot accept more cards.
    #[error("{0}")]
    Capacity(CapacityError<Card>),
    /// The deal is not a valid subset of a bridge deal
    #[error("The deal is not a valid subset of a bridge deal")]
    Invalid,
}

impl From<CapacityError<Card>> for Error {
    fn from(value: CapacityError<Card>) -> Self {
        Self::Capacity(value)
    }
}

/// A subset of the standard 52-card deck
///
/// It requires shuffling to partially retrieve cards from the deck.  However,
/// it is deterministic to collect all cards.
#[derive(Debug, Clone)]
pub struct Deck {
    cards: ArrayVec<Card, 52>,
}

impl Deck {
    /// The maximum number of cards in a deck
    pub const CAPACITY: usize = 52;

    /// The standard 52-card deck
    #[must_use]
    pub fn standard_52() -> Self {
        let mut cards = ArrayVec::new();
        for i in 0..Self::CAPACITY {
            // SAFETY: `i` < `Self::CAPACITY` = 52, so `(i >> 2) as u8 + 2` is
            // in the range [2, 14], which is valid for `Rank`.
            #[allow(clippy::cast_possible_truncation)]
            let rank = Rank::new((i >> 2) as u8 + 2);
            cards.push(Card::new(Suit::ASC[i & 3], rank));
        }
        Self { cards }
    }

    /// Create a new empty deck
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cards: ArrayVec::new_const(),
        }
    }

    /// The number of cards currently in the deck
    #[must_use]
    pub const fn len(&self) -> usize {
        self.cards.len()
    }

    /// Whether the deck is empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Clear the deck, removing all the cards.
    pub fn clear(&mut self) {
        self.cards.clear();
    }

    /// Try pushing a card into the deck
    ///
    /// # Errors
    ///
    /// [`CapacityError`] if the deck already contains 52 cards.
    pub fn try_push(&mut self, card: Card) -> Result<(), CapacityError<Card>> {
        self.cards.try_push(card)
    }

    /// Try pushing cards in a hand into the deck
    ///
    /// # Errors
    ///
    /// [`CapacityError`] if the resulting deck would contain more than 52 cards.
    pub fn try_extend(&mut self, hand: Hand) -> Result<(), CapacityError<Card>> {
        for card in hand {
            self.try_push(card)?;
        }
        Ok(())
    }

    /// Drain the remaining cards in the deck into a hand.
    #[must_use]
    pub fn drain(&mut self) -> Hand {
        self.cards.drain(..).collect()
    }

    /// Randomly pick `n` cards from the deck and collect them into a hand.
    #[must_use]
    pub fn partial_shuffle(&mut self, rng: &mut (impl Rng + ?Sized), n: usize) -> Hand {
        self.cards.partial_shuffle(rng, n);
        self.cards.drain(self.cards.len() - n..).collect()
    }

    /// Randomly pop a card from the deck
    #[must_use]
    pub fn pop(&mut self, rng: &mut (impl Rng + ?Sized)) -> Option<Card> {
        match self.cards.len() {
            0..=1 => self.cards.pop(),
            len => self.cards.swap_pop(rng.random_range(0..len)),
        }
    }
}

impl Default for Deck {
    fn default() -> Self {
        Self::new()
    }
}

impl TryFrom<Hand> for Deck {
    type Error = CapacityError<Card>;

    fn try_from(hand: Hand) -> Result<Self, Self::Error> {
        let mut deck = Self::new();
        deck.try_extend(hand)?;
        Ok(deck)
    }
}

/// Shuffle and evenly deal 52 cards into 4 hands
pub fn full_deal(rng: &mut (impl Rng + ?Sized)) -> Deal {
    let mut deck = Deck::standard_52().cards;
    let (shuffled, rest) = deck.partial_shuffle(rng, 39);

    Deal::new(
        rest.iter().copied().collect(),
        shuffled[00..13].iter().copied().collect(),
        shuffled[13..26].iter().copied().collect(),
        shuffled[26..39].iter().copied().collect(),
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
        let mut deck = self.deck.clone();
        let mut deal = self.deal;
        let mut fill = |hand: &mut Hand| *hand |= deck.partial_shuffle(self.rng, 13 - hand.len());

        fill(&mut deal[self.shortest.lho()]);
        fill(&mut deal[self.shortest.partner()]);
        fill(&mut deal[self.shortest.rho()]);

        deal[self.shortest] |= deck.drain();
        Some(deal)
    }
}

impl<R: Rng + ?Sized> FusedIterator for FillDeals<'_, R> {}

/// Given a partial deal, return an iterator that fills in the remaining cards
/// randomly on each iteration.
///
/// # Errors
///
/// [`Error::Invalid`] if `deal` is invalid determined by
/// [`Deal::validate_and_collect`].
pub fn fill_deals<R: Rng + ?Sized>(
    rng: &mut R,
    deal: Deal,
) -> Result<FillDeals<'_, R>, Error> {
    Ok(FillDeals {
        rng,
        deal,
        deck: Deck::try_from(!deal.validate_and_collect().ok_or(Error::Invalid)?)?,

        #[allow(clippy::missing_panics_doc)]
        shortest: Seat::ALL
            .into_iter()
            .min_by_key(|&seat| deal[seat].len())
            .expect("Seat::ALL shall not be empty"),
    })
}
