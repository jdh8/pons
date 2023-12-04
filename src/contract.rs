#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd)]
#[repr(u8)]
pub enum Strain {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
    Notrump,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd)]
pub struct Bid {
    pub level: u8,
    pub strain: Strain,
}

impl Bid {
    pub const fn new(level: u8, strain: Strain) -> Self {
        Self { level, strain }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Call {
    Pass,
    Double,
    Redouble,
    Bid(Bid),
}

impl From<Bid> for Call {
    fn from(bid: Bid) -> Self {
        Self::Bid(bid)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Penalty {
    Passed,
    Doubled,
    Redoubled,
}

#[derive(Clone, Copy, Debug)]
pub struct Contract {
    pub bid: Bid,
    pub penalty: Penalty,
}

impl From<Bid> for Contract {
    fn from(bid: Bid) -> Self {
        Self {
            bid,
            penalty: Penalty::Passed,
        }
    }
}

const fn compute_doubled_penalty(undertricks: i32, vulnerable: bool) -> i32 {
    match undertricks + vulnerable as i32 {
        1 => 100,
        2 => if vulnerable { 200 } else { 300 },
        many => 300 * many - 400,
    }
}

impl Contract {
    pub const fn new(level: u8, strain: Strain, penalty: Penalty) -> Self {
        Self { bid: Bid::new(level, strain), penalty }
    }

    // Base score for making this contract
    // https://en.wikipedia.org/wiki/Bridge_scoring#Contract_points
    pub fn points(&self) -> i32 {
        let level = i32::from(self.bid.level);
        let per_trick = if self.bid.strain >= Strain::Hearts { 30 } else { 20 };
        let notrump = if self.bid.strain == Strain::Notrump { 10 } else { 0 };
        (per_trick * level + notrump) << (self.penalty as u8)
    }

    pub fn score(&self, tricks: u8, vulnerable: bool) -> i32 {
        let overtricks = i32::from(tricks) - i32::from(self.bid.level) - 6;

        if overtricks >= 0 {
            let base = self.points();
            let game = if base < 100 { 50 } else if vulnerable { 500 } else { 300 };
            let doubled = self.penalty as i32 * 50;

            let slam = match self.bid.level {
                6 => if vulnerable { 750 } else { 500 },
                7 => if vulnerable { 1500 } else { 1000 },
                _ => 0,
            };

            let per_trick = match self.penalty {
                Penalty::Passed => if self.bid.strain >= Strain::Hearts { 30 } else { 20 },
                penalty => penalty as i32 * if vulnerable { 200 } else { 100 },
            };
            
            base + game + slam + doubled + overtricks * per_trick
        }
        else {
            match self.penalty {
                Penalty::Passed => overtricks * if vulnerable { 100 } else { 50 },
                penalty => penalty as i32 * -compute_doubled_penalty(-overtricks, vulnerable),
            }
        }
    }
}
