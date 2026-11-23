//! probe-drury-slam — after Reverse Drury, when does opener's keycard ask pay?
//!
//! A plain statistic over the pre-solved `.pdd` bank (no solver, no bidding).
//! For each partnership and each major, the population of
//! `- - 1M - 2♣! -` with opener on 19+ support points:
//!
//! - opener: 5+ in the major, `support_point_count_in` 19+;
//! - responder: 3+ in the major, support points 10+, `point_count` ≤ 11 (a
//!   passed hand), and no splinter (4+ trumps with a short suit).
//!
//! One row per cell `osp shape keys queen bare tricks n`, with tricks
//! double-dummy in the major from the better declarer of the pair.  `osp` is
//! opener's support points (capped at 24), `shape` opener's (0 balanced,
//! 1 no singleton, 2 a singleton, 3 a void), `keys` the keycards the pair
//! misses (aces + trump king, capped at 2), `queen` whether the pair holds
//! the trump queen, `bare` whether opener holds a side suit of 2+ cards with
//! neither ace nor king (what opener sees of an open suit).
//!
//!   cargo build --release --example probe-drury-slam
//!   RAYON_NUM_THREADS=8 nice -n 19 target/release/examples/probe-drury-slam \
//!       /nfs2/jdh8/pons/22.pdd /nfs2/jdh8/pons/24.pdd /nfs2/jdh8/pons/shard-*.pdd > drury.tsv
use contract_bridge::{Hand, Rank, Seat, Strain, Suit};
use pons::bidding::constraint::{point_count, support_point_count_in};
use pons::pdd::{MAGIC, ROW_LEN, decode_row};
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::Read;

type Key = [u8; 6];

fn shape(h: Hand) -> u8 {
    let mut l = Suit::ASC.map(|s| h[s].len());
    l.sort_unstable();
    match l {
        [0, ..] => 3,
        [1, ..] => 2,
        [3, 3, 3, 4] | [2, 3, 4, 4] | [2, 3, 3, 5] => 0,
        _ => 1,
    }
}

fn scan(rows: &[u8]) -> HashMap<Key, u64> {
    let mut cells = HashMap::new();
    for row in rows.as_chunks::<ROW_LEN>().0 {
        let (deal, table) = decode_row(row).expect("bad row");
        for (o, r) in [
            (Seat::North, Seat::South),
            (Seat::South, Seat::North),
            (Seat::East, Seat::West),
            (Seat::West, Seat::East),
        ] {
            let (ho, hr) = (deal[o], deal[r]);
            for major in [Suit::Hearts, Suit::Spades] {
                let osp = support_point_count_in(ho, major);
                if ho[major].len() < 5 || osp < 19 {
                    continue;
                }
                let short = Suit::ASC.iter().any(|&s| s != major && hr[s].len() <= 1);
                if hr[major].len() < 3
                    || support_point_count_in(hr, major) < 10
                    || point_count(hr) > 11
                    || (hr[major].len() >= 4 && short)
                {
                    continue;
                }
                let holds = |s: Suit, k: Rank| ho[s].contains(k) || hr[s].contains(k);
                let aces = Suit::ASC.iter().filter(|&&s| !holds(s, Rank::A)).count();
                let keys = (aces + usize::from(!holds(major, Rank::K))).min(2) as u8;
                let queen = u8::from(holds(major, Rank::Q));
                let bare = u8::from(Suit::ASC.iter().any(|&s| {
                    s != major
                        && ho[s].len() >= 2
                        && !ho[s].contains(Rank::A)
                        && !ho[s].contains(Rank::K)
                }));
                let s = Strain::from(major);
                let tricks = u8::from(table[s].get(o)).max(u8::from(table[s].get(r)));
                let key = [osp.min(24), shape(ho), keys, queen, bare, tricks];
                *cells.entry(key).or_default() += 1;
            }
        }
    }
    cells
}

fn main() {
    let chunk = 1 << 20; // rows
    let mut total: HashMap<Key, u64> = HashMap::new();
    for f in std::env::args().skip(1) {
        let mut file = std::fs::File::open(&f).unwrap();
        let mut magic = [0u8; 8];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(magic, MAGIC, "{f}");
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes.len() % ROW_LEN, 0, "{f}");
        let part = bytes
            .par_chunks(chunk * ROW_LEN)
            .map(scan)
            .reduce(HashMap::new, |mut a, b| {
                for (k, n) in b {
                    *a.entry(k).or_default() += n;
                }
                a
            });
        for (k, n) in part {
            *total.entry(k).or_default() += n;
        }
        eprintln!("{f}: {} deals", bytes.len() / ROW_LEN);
    }
    let mut cells: Vec<_> = total.into_iter().collect();
    cells.sort_unstable();
    println!("osp\tshape\tkeys\tqueen\tbare\ttricks\tn");
    for (k, n) in cells {
        let fields: Vec<String> = k.iter().map(u8::to_string).collect();
        println!("{}\t{n}", fields.join("\t"));
    }
}
