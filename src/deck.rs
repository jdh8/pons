use core::fmt;
use core::iter::FusedIterator;
use core::str::FromStr;
use dds_bridge::{Builder, Card, FullDeal, Hand, PartialDeal, Seat};
use rand::{Rng, RngExt as _};

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
    #[inline]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the deck is empty
    #[must_use]
    #[inline]
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
    #[inline]
    pub const fn take(&mut self) -> Hand {
        core::mem::replace(&mut self.0, Hand::EMPTY)
    }

    /// Randomly draw `n` cards from the deck and collect them into a hand.
    ///
    /// If `n >= self.len()`, all remaining cards are drawn without shuffling.
    ///
    /// On each iteration, pick a uniform `k` in `0..remaining`, then strip the
    /// `k` lowest set bits from `self.0`.  The new lowest set bit is the `k`-th
    /// smallest card, which is moved from the deck to the hand.  This performs
    /// `n` selections without materializing the card set.
    #[must_use]
    pub fn draw(&mut self, rng: &mut (impl Rng + ?Sized), n: usize) -> Hand {
        let len = self.0.len();
        if n >= len {
            return self.take();
        }

        let mut hand = Hand::EMPTY;
        for i in 0..n {
            let bits = (0..rng.random_range(..len - i))
                .fold(self.0.to_bits(), |bits, _| bits & (bits - 1));
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

impl fmt::Display for Deck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Deck {
    type Err = <Hand as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Hand>().map(Self)
    }
}

/// Shuffle and evenly deal 52 cards into 4 hands
#[must_use]
pub fn full_deal(rng: &mut (impl Rng + ?Sized)) -> FullDeal {
    let mut deck = Deck::ALL;

    // SAFETY: the panic condition is guaranteed not to occur by construction,
    // since the deck has 52 cards and each hand receives exactly 13 cards.
    #[allow(clippy::missing_panics_doc)]
    Builder::new()
        .north(deck.draw(rng, 13))
        .east(deck.draw(rng, 13))
        .south(deck.draw(rng, 13))
        .west(deck.take())
        .build_full()
        .expect("each hand receives exactly 13 cards by construction")
}

/// An infinite iterator that fills undealt cards randomly into a partial deal.
///
/// Created by [`fill_deals`].
#[derive(Debug)]
pub struct FillDeals<'a, R: Rng + ?Sized> {
    rng: &'a mut R,
    builder: Builder,
    deck: Deck,

    /// The seat with the fewest cards in the initial deal.  This seat will be
    /// filled last to save entropy.
    shortest: Seat,
}

impl<R: Rng + ?Sized> Iterator for FillDeals<'_, R> {
    type Item = FullDeal;

    fn next(&mut self) -> Option<FullDeal> {
        let mut deck = self.deck;
        let mut builder = self.builder;
        let mut fill = |hand: &mut Hand| *hand |= deck.draw(self.rng, 13 - hand.len());

        fill(&mut builder[self.shortest.lho()]);
        fill(&mut builder[self.shortest.partner()]);
        fill(&mut builder[self.shortest.rho()]);

        builder[self.shortest] |= deck.take();
        Some(
            builder
                .build_full()
                .expect("each seat holds exactly 13 cards after filling"),
        )
    }
}

impl<R: Rng + ?Sized> FusedIterator for FillDeals<'_, R> {}

/// Return an iterator that completes `subset` with random cards on each iteration.
///
/// Every iteration deals the cards missing from `subset` to the seats that need
/// them, yielding a fresh [`FullDeal`].  Because the input is a [`PartialDeal`],
/// its invariants guarantee that every iteration produces a valid deal.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub fn fill_deals<R: Rng + ?Sized>(rng: &mut R, subset: PartialDeal) -> FillDeals<'_, R> {
    let builder = Builder::from(subset);
    FillDeals {
        rng,
        builder,
        deck: Deck::from(!subset.collected()),

        #[allow(clippy::missing_panics_doc)]
        shortest: Seat::ALL
            .into_iter()
            .min_by_key(|&seat| builder[seat].len())
            .expect("Seat::ALL shall not be empty"),
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::Deck;
    use core::str::FromStr;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

    impl Serialize for Deck {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.collect_str(self)
        }
    }

    impl<'de> Deserialize<'de> for Deck {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let s = <&str>::deserialize(d)?;
            Self::from_str(s).map_err(de::Error::custom)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(deck: Deck) {
        assert_eq!(deck.to_string().parse::<Deck>().unwrap(), deck);
    }

    #[test]
    fn full_and_empty_roundtrip() {
        roundtrip(Deck::ALL);
        roundtrip(Deck::EMPTY);
    }

    #[test]
    fn partial_deck_roundtrip() {
        let mut deck = Deck::EMPTY;
        for s in ["♠A", "♥K", "♦Q", "♣J", "♠2"] {
            deck.insert(s.parse().unwrap());
        }
        roundtrip(deck);
    }

    #[test]
    fn parses_from_hand_notation() {
        let deck: Deck = "AKQJ.T98.765.432".parse().unwrap();
        assert_eq!(deck.len(), 13);
        assert_eq!(deck.to_string(), "AKQJ.T98.765.432");
    }
}
