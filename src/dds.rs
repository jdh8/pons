// Keep our external names consistent with DDS
#![allow(non_snake_case)]

use crate::deal::{Deal, Seat, Strain};
use bitflags::bitflags;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ddTableDeal {
    pub cards: [[u32; 4]; 4],
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ddTableDeals {
    pub noOfTables: i32,
    pub deals: [ddTableDeal; 200],
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ddTableResults {
    pub resTable: [[i32; 4]; 5],
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ddTablesRes {
    pub noOfTables: i32,
    pub results: [ddTableResults; 200],
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct parResults {
    pub parScore: [[i8; 16]; 2],
    pub parContractsString: [[i8; 128]; 2],
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct allParResults {
    pub presults: [parResults; 40],
}

#[link(name = "dds")]
extern "C" {
    pub fn CalcAllTables(
        dealsp: &ddTableDeals,
        mode: i32,
        trumpFilter: *const i32,
        resp: &mut ddTablesRes,
        presp: *mut allParResults) -> i32;
} 

impl From<Deal> for ddTableDeal {
    fn from(deal: Deal) -> Self {
        Self{cards: [
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

impl From<&[Deal]> for ddTableDeals {
    fn from(slice: &[Deal]) -> Self {
        let mut pack = Self {
            noOfTables: slice.len() as i32,
            deals: unsafe { core::mem::zeroed() },
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

impl From<ddTableResults> for TricksTable {
    fn from(table: ddTableResults) -> Self {
        Self([
            make_row(table.resTable[Strain::Spades as usize]),
            make_row(table.resTable[Strain::Hearts as usize]),
            make_row(table.resTable[Strain::Diamonds as usize]),
            make_row(table.resTable[Strain::Clubs as usize]),
            make_row(table.resTable[Strain::Notrump as usize]),
        ])
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

unsafe fn solve_segment(deals: &[Deal], filter: [i32; 5]) -> ddTablesRes {
    let mut res: ddTablesRes = core::mem::zeroed();
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
    let q = deals.len() / seglen;
    let r = deals.len() % seglen;
    let mut tables = Vec::new();

    for i in 0..q {
        let res = unsafe { solve_segment(&deals[i * seglen .. (i + 1) * seglen], filter) };
        tables.extend(res.results.map(TricksTable::from));
    }

    if r > 0 {
        let res = unsafe { solve_segment(&deals[q * seglen ..], filter) };
        tables.extend(res.results.map(TricksTable::from));
    }

    tables
}
