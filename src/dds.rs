use crate::contract::Strain;
use crate::deal::{Deal, Seat};
use bitflags::bitflags;
use core::fmt;

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct DDTableDeal {
    cards: [[u32; 4]; 4],
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct DDTableDeals {
    no_of_tables: i32,
    deals: [DDTableDeal; 200],
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct DDTableResults {
    res_table: [[i32; 4]; 5],
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct DDTablesRes {
    no_of_tables: i32,
    results: [DDTableResults; 200],
}

impl Default for DDTablesRes {
    fn default() -> Self {
        Self {
            no_of_tables: 0,
            results: [Default::default(); 200],
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct ParResults {
    par_score: [[i8; 16]; 2],
    par_contracts_string: [[i8; 128]; 2],
}

impl Default for ParResults {
    fn default() -> Self {
        Self {
            par_score: [[0; 16]; 2],
            par_contracts_string: [[0; 128]; 2],
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct AllParResults {
    pub presults: [ParResults; 40],
}

impl Default for AllParResults {
    fn default() -> Self {
        Self {
            presults: [Default::default(); 40],
        }
    }
}

#[link(name = "dds")]
extern "C" {
    fn CalcAllTables(
        dealsp: &DDTableDeals,
        mode: i32,
        trump_filter: *const i32,
        resp: &mut DDTablesRes,
        presp: *mut AllParResults) -> i32;
} 

impl From<Deal> for DDTableDeal {
    fn from(deal: Deal) -> Self {
        Self {cards: [
            [
                deal[Seat::North][Strain::Spades].bits().into(),
                deal[Seat::North][Strain::Hearts].bits().into(),
                deal[Seat::North][Strain::Diamonds].bits().into(),
                deal[Seat::North][Strain::Clubs].bits().into(),
            ],
            [
                deal[Seat::East][Strain::Spades].bits().into(),
                deal[Seat::East][Strain::Hearts].bits().into(),
                deal[Seat::East][Strain::Diamonds].bits().into(),
                deal[Seat::East][Strain::Clubs].bits().into(),
            ],
            [
                deal[Seat::South][Strain::Spades].bits().into(),
                deal[Seat::South][Strain::Hearts].bits().into(),
                deal[Seat::South][Strain::Diamonds].bits().into(),
                deal[Seat::South][Strain::Clubs].bits().into(),
            ],
            [
                deal[Seat::West][Strain::Spades].bits().into(),
                deal[Seat::West][Strain::Hearts].bits().into(),
                deal[Seat::West][Strain::Diamonds].bits().into(),
                deal[Seat::West][Strain::Clubs].bits().into(),
            ],
        ]}
    }
}

impl From<&[Deal]> for DDTableDeals {
    fn from(slice: &[Deal]) -> Self {
        let mut pack = Self {
            no_of_tables: slice.len() as i32,
            deals: [Default::default(); 200],
        };
        core::iter::zip(&mut pack.deals, slice).for_each(|(y, x)| *y = (*x).into());
        pack
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TricksPerStrain(u16);

impl TricksPerStrain {
    pub fn new(n: u8, e: u8, s: u8, w: u8) -> Self {
        Self(((n as u16) << (4 * Seat::North as u8) |
              (e as u16) << (4 * Seat::East  as u8) |
              (s as u16) << (4 * Seat::South as u8) |
              (w as u16) << (4 * Seat::West  as u8)).into())
    }

    pub fn at(&self, seat: Seat) -> u8 {
        (self.0 >> (4 * seat as u8) & 0xF) as u8
    }
}

impl fmt::Display for TricksPerStrain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:X}{:X}{:X}{:X}",
            self.at(Seat::North),
            self.at(Seat::East),
            self.at(Seat::South),
            self.at(Seat::West))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TricksTable([TricksPerStrain; 5]);

impl core::ops::Index<Strain> for TricksTable {
    type Output = TricksPerStrain;

    fn index(&self, strain: Strain) -> &TricksPerStrain {
        &self.0[strain as usize]
    }
}

fn make_row(row: [i32; 4]) -> TricksPerStrain {
    TricksPerStrain::new(row[0] as u8, row[1] as u8, row[2] as u8, row[3] as u8)
}

impl From<&DDTableResults> for TricksTable {
    fn from(table: &DDTableResults) -> Self {
        Self([
            make_row(table.res_table[Strain::Spades as usize]),
            make_row(table.res_table[Strain::Hearts as usize]),
            make_row(table.res_table[Strain::Diamonds as usize]),
            make_row(table.res_table[Strain::Clubs as usize]),
            make_row(table.res_table[Strain::Notrump as usize]),
        ])
    }
}

impl fmt::Display for TricksTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{}{}{}", self.0[0], self.0[1], self.0[2], self.0[3], self.0[4])
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct StrainFlags : u8 {
        const CLUBS = 0x01;
        const DIAMONDS = 0x02;
        const HEARTS = 0x04;
        const SPADES = 0x08;
        const NOTRUMP = 0x10;
    }
}

unsafe fn solve_segment(deals: &[Deal], filter: [i32; 5]) -> DDTablesRes {
    let mut res = DDTablesRes::default();
    CalcAllTables(&deals.into(), -1, &filter[0], &mut res, core::ptr::null_mut());
    res
}

pub fn solve(deals: &[Deal], flags: StrainFlags) -> Vec<TricksTable> {
    let filter = [
        !flags.contains(StrainFlags::SPADES) as i32,
        !flags.contains(StrainFlags::HEARTS) as i32,
        !flags.contains(StrainFlags::DIAMONDS) as i32,
        !flags.contains(StrainFlags::CLUBS) as i32,
        !flags.contains(StrainFlags::NOTRUMP) as i32,
    ];

    let seglen = (200 / flags.bits().count_ones()) as usize;
    let (q, r) = (deals.len() / seglen, deals.len() % seglen);
    let mut tables = Vec::new();

    for i in 0..q {
        let res = unsafe { solve_segment(&deals[i * seglen .. (i + 1) * seglen], filter) };
        tables.extend(res.results[..seglen].iter().map(TricksTable::from));
    }

    if r > 0 {
        let res = unsafe { solve_segment(&deals[q * seglen ..], filter) };
        tables.extend(res.results[..r].iter().map(TricksTable::from));
    }

    tables
}
