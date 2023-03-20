use core::ops::{Index, IndexMut};

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Strain {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
    Notrump,
}

#[derive(Clone, Copy, Debug)]
pub enum Seat {
    North,
    East,
    South,
    West,
}

#[derive(Clone, Copy, Debug)]
pub struct Card(u8);

impl Card {
    pub fn new(suit: Strain, rank: u8) -> Card {
        Card((suit as u8) << 4 | rank)
    }

    pub fn rank(&self) -> u8 {
        self.0 & 0xF
    }

    pub fn suit(&self) -> Strain {
        unsafe { core::mem::transmute(self.0 >> 4) }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Holding(u16);

impl Holding {
    pub fn bits(&self) -> u16 {
        self.0
    }

    pub fn len(&self) -> usize {
        self.0.count_ones() as usize
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn contains(&self, rank: u8) -> bool {
        self.0 & 1 << rank != 0
    }

    pub fn insert(&mut self, rank: u8) -> bool {
        let inserted = !self.contains(rank);
        self.0 |= 1 << rank;
        inserted
    }

    pub fn remove(&mut self, rank: u8) -> bool {
        let removed = self.contains(rank);
        self.0 &= !(1 << rank);
        removed
    }
}

#[derive(Clone, Copy, Debug)]
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

impl Hand {
    pub fn len(&self) -> usize {
        self.0.len() + self.1.len() + self.2.len() + self.3.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty() && self.1.is_empty() && self.2.is_empty() && self.3.is_empty()
    }

    pub fn contains(&self, card: Card) -> bool {
        self[card.suit()].contains(card.rank())
    }

    pub fn insert(&mut self, card: Card) -> bool {
        self[card.suit()].insert(card.rank())
    }

    pub fn remove(&mut self, card: Card) -> bool {
        self[card.suit()].remove(card.rank())
    }
}

#[derive(Clone, Copy, Debug)]
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
