use crate::contract::Strain;
use rand::prelude::SliceRandom;
use core::fmt;
use core::ops::{BitAnd, BitOr, BitXor, Index, IndexMut, Not, Sub};

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Seat {
    North,
    East,
    South,
    West,
}

#[derive(Clone, Copy, Debug)]
pub struct Card {
    pub suit: Strain,
    pub rank: u8,
}

impl Card {
    pub const fn new(suit: Strain, rank: u8) -> Self {
        Self { suit, rank }
    }
}

pub trait SmallSet<T> {
    fn empty() -> Self;
    fn all() -> Self;
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool where Self: Sized + PartialEq<Self> {
        self == &Self::empty()
    }

    fn contains(&self, value: T) -> bool;
    fn insert(&mut self, value: T) -> bool;
    fn remove(&mut self, value: T) -> bool;
    fn toggle(&mut self, value: T) -> bool;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Holding(u16);

impl SmallSet<u8> for Holding {
    fn empty() -> Self {
        Self(0)
    }

    fn all() -> Self {
        Self(Self::ALL)
    }

    fn len(&self) -> usize {
        self.0.count_ones() as usize
    }

    fn contains(&self, rank: u8) -> bool {
        self.0 & 1 << rank != 0
    }

    fn insert(&mut self, rank: u8) -> bool {
        let insertion = 1 << rank & Self::ALL;
        let inserted = insertion & !self.0 != 0;
        self.0 |= insertion;
        inserted
    }

    fn remove(&mut self, rank: u8) -> bool {
        let removed = self.contains(rank);
        self.0 &= !(1 << rank);
        removed
    }

    fn toggle(&mut self, rank: u8) -> bool {
        self.0 ^= 1 << rank & Self::ALL;
        self.contains(rank)
    }
}

impl Holding {
    const ALL: u16 = 0x7FFC;

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn from_bits(bits: u16) -> Self {
        Self(bits & Self::ALL)
    }
}

impl BitAnd for Holding {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl BitOr for Holding {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitXor for Holding {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self {
        Self(self.0 ^ rhs.0)
    }
}

impl Not for Holding {
    type Output = Self;

    fn not(self) -> Self {
        Self::from_bits(!self.0)
    }
}

impl Sub for Holding {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0 & !rhs.0)
    }
}

impl fmt::Display for Holding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for rank in (2..15).rev() {
            if self.contains(rank) {
                use fmt::Write;
                f.write_char(b"23456789TJQKA"[rank as usize - 2] as char)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Hand(Holding, Holding, Holding, Holding);

impl Index<Strain> for Hand {
    type Output = Holding;

    fn index(&self, suit: Strain) -> &Holding {
        match suit {
            Strain::Clubs => &self.0,
            Strain::Diamonds => &self.1,
            Strain::Hearts => &self.2,
            Strain::Spades => &self.3,
            Strain::Notrump => panic!("Notrump is not a suit"),
        }
    }
}

impl IndexMut<Strain> for Hand {
    fn index_mut(&mut self, suit: Strain) -> &mut Holding {
        match suit {
            Strain::Clubs => &mut self.0,
            Strain::Diamonds => &mut self.1,
            Strain::Hearts => &mut self.2,
            Strain::Spades => &mut self.3,
            Strain::Notrump => panic!("Notrump is not a suit"),
        }
    }
}

impl SmallSet<Card> for Hand {
    fn empty() -> Self {
        Self::default()
    }

    fn all() -> Self {
        Self(Holding::all(), Holding::all(), Holding::all(), Holding::all())
    }

    fn len(&self) -> usize {
        self.0.len() + self.1.len() + self.2.len() + self.3.len()
    }

    fn contains(&self, card: Card) -> bool {
        self[card.suit].contains(card.rank)
    }

    fn insert(&mut self, card: Card) -> bool {
        self[card.suit].insert(card.rank)
    }

    fn remove(&mut self, card: Card) -> bool {
        self[card.suit].remove(card.rank)
    }

    fn toggle(&mut self, card: Card) -> bool {
        self[card.suit].toggle(card.rank)
    }
}

impl fmt::Display for Hand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}.{}",
            self[Strain::Spades],
            self[Strain::Hearts],
            self[Strain::Diamonds],
            self[Strain::Clubs])
    }
}

impl BitAnd for Hand {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0, self.1 & rhs.1, self.2 & rhs.2, self.3 & rhs.3)
    }
}

impl BitOr for Hand {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0, self.1 | rhs.1, self.2 | rhs.2, self.3 | rhs.3)
    }
}

impl BitXor for Hand {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self {
        Self(self.0 ^ rhs.0, self.1 ^ rhs.1, self.2 ^ rhs.2, self.3 ^ rhs.3)
    }
}

impl Not for Hand {
    type Output = Self;

    fn not(self) -> Self {
        Self(!self.0, !self.1, !self.2, !self.3)
    }
}

impl Sub for Hand {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0, self.1 - rhs.1, self.2 - rhs.2, self.3 - rhs.3)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Deal([Hand; 4]);

impl Index<Seat> for Deal {
    type Output = Hand;

    fn index(&self, seat: Seat) -> &Hand {
        &self.0[seat as usize]
    }
}

impl IndexMut<Seat> for Deal {
    fn index_mut(&mut self, seat: Seat) -> &mut Hand {
        &mut self.0[seat as usize]
    }
}

impl fmt::Display for Deal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "N:{} {} {} {}",
            self[Seat::North],
            self[Seat::East],
            self[Seat::South],
            self[Seat::West])
    }
}

#[derive(Clone, Debug, Default)]
pub struct Deck {
    pub cards: Vec<Card>,
}

impl Deck {
    pub fn standard_52() -> Self {
        let suits = [Strain::Clubs, Strain::Diamonds, Strain::Hearts, Strain::Spades];
        let product = suits.iter().flat_map(|x| core::iter::repeat(x).zip(2..15));
        Self { cards: product.map(|(suit, rank)| Card::new(*suit, rank)).collect() }
    }

    pub fn deal(&self) -> Deal {
        let mut deal = Deal::default();

        for (index, card) in self.cards.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            deal[unsafe { core::mem::transmute((index & 0x3) as u8) }].insert(*card);
        }

        deal
    }

    pub fn shuffle(&mut self) {
        self.cards.shuffle(&mut rand::thread_rng());
    }
}

pub fn shuffled_standard_52_deck() -> Deck {
    let mut deck = Deck::standard_52();
    deck.shuffle();
    deck
}
