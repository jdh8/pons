//! probe-notrump-threshold — the 3NT point threshold, and what length and
//! flatness are worth on top of it.
//!
//! A plain statistic over the pre-solved `.pdd` bank (no solver, no bidding);
//! one case per partnership.  Key: scale v (`H` HCP, `F` Fifths, `P` pons
//! [`point_count`]), length points X = Σ max(0, len−4) over both hands' suits
//! (cap 5), F = number of 4333 hands (0..2), M = 8+ card major fit, and the
//! combined count P2 in half points.  Per cell: n, P(9+ tricks), and
//! (sum, sumsq) of IMPs for 3NT over 2NT / over 1NT, NV and vul.  Tricks are
//! double-dummy with the better declarer of the pair.
//!
//! `scripts/notrump-threshold-report.py` turns the table into
//! `docs/notrump-game-threshold.md`.
//!
//!   cargo build --release --example probe-notrump-threshold
//!   RAYON_NUM_THREADS=8 nice -n 19 target/release/examples/probe-notrump-threshold \
//!       /nfs2/jdh8/pons/22.pdd /nfs2/jdh8/pons/24.pdd /nfs2/jdh8/pons/shard-*.pdd \
//!       /mnt/hdd-data/jdh8/shards/shard-*.pdd > notrump.tsv
use contract_bridge::eval::{fifths, hcp};
use contract_bridge::{Bid, Contract, Hand, Penalty, Seat, Strain, Suit};
use pons::bidding::constraint::point_count;
use pons::pdd::{MAGIC, ROW_LEN, decode_row};
use pons::scoring::imps;
use rayon::prelude::*;
use std::io::Read;

const PMAX: usize = 96; // half points
const XS: usize = 6;
const N: usize = 3 * XS * 3 * 2 * PMAX;
type Cell = [i64; 10];
fn idx(v: usize, x: usize, f: usize, m: usize, p: usize) -> usize {
    (((v * XS + x.min(XS - 1)) * 3 + f) * 2 + m) * PMAX + p.min(PMAX - 1)
}

fn score(level: u8, tricks: u8, vul: bool) -> i64 {
    let bid = Bid::new(level, Strain::Notrump);
    i64::from(
        Contract {
            bid,
            penalty: Penalty::Undoubled,
        }
        .score(tricks, vul),
    )
}

fn raw_hcp(h: Hand) -> usize {
    Suit::ASC.iter().map(|&s| hcp::<usize>(h[s])).sum()
}

/// Fifths in tenths of a point (exact integers)
fn fifths10(h: Hand) -> usize {
    Suit::ASC
        .iter()
        .map(|&s| (fifths(h[s]) * 10.0).round() as usize)
        .sum()
}

fn length_points(h: Hand) -> usize {
    Suit::ASC
        .iter()
        .map(|&s| h[s].len().saturating_sub(4))
        .sum()
}

fn flat(h: Hand) -> bool {
    Suit::ASC.iter().all(|&s| h[s].len() <= 4)
        && Suit::ASC.iter().filter(|&&s| h[s].len() == 3).count() == 3
}

fn scan(rows: &[u8], acc: &mut [Cell]) {
    for row in rows.as_chunks::<ROW_LEN>().0 {
        let (deal, table) = decode_row(row).expect("bad row");
        for (a, b) in [(Seat::North, Seat::South), (Seat::East, Seat::West)] {
            let (ha, hb) = (deal[a], deal[b]);
            // ponytail: best declarer of the pair (DD-optimistic for wrong-siding)
            let nt = table[Strain::Notrump];
            let tricks = u8::from(nt.get(a)).max(u8::from(nt.get(b)));
            let x = length_points(ha) + length_points(hb);
            let f = usize::from(flat(ha)) + usize::from(flat(hb));
            let m = usize::from(
                [Suit::Hearts, Suit::Spades]
                    .iter()
                    .any(|&s| ha[s].len() + hb[s].len() >= 8),
            );
            let fifths_half = (fifths10(ha) + fifths10(hb) + 2) / 5; // nearest half point
            let ps = [
                2 * (raw_hcp(ha) + raw_hcp(hb)),
                fifths_half,
                2 * usize::from(point_count(ha) + point_count(hb)),
            ];
            let mut gain = [0i64; 4];
            for (k, vul) in [(0, false), (1, true)] {
                let g = score(3, tricks, vul);
                gain[k] = imps(g - score(2, tricks, vul));
                gain[k + 2] = imps(g - score(1, tricks, vul));
            }
            for (v, &p) in ps.iter().enumerate() {
                let e = &mut acc[idx(v, x, f, m, p)];
                e[0] += 1;
                e[1] += i64::from(tricks >= 9);
                for k in 0..4 {
                    e[2 + 2 * k] += gain[k];
                    e[3 + 2 * k] += gain[k] * gain[k];
                }
            }
        }
    }
}

fn add(x: &mut [Cell], y: &[Cell]) {
    for (p, q) in x.iter_mut().zip(y) {
        for k in 0..10 {
            p[k] += q[k];
        }
    }
}

fn main() {
    let chunk = 1 << 20; // rows
    let mut total = vec![[0i64; 10]; N];
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
            .map(|c| {
                let mut a = vec![[0i64; 10]; N];
                scan(c, &mut a);
                a
            })
            .reduce(
                || vec![[0i64; 10]; N],
                |mut x, y| {
                    add(&mut x, &y);
                    x
                },
            );
        add(&mut total, &part);
        eprintln!("{f}: {} deals", bytes.len() / ROW_LEN);
    }
    println!("var\tX\tF\tM\tP2\tn\tmake\ts2n\tq2n\ts2v\tq2v\ts1n\tq1n\ts1v\tq1v");
    for v in 0..3 {
        for x in 0..XS {
            for f in 0..3 {
                for m in 0..2 {
                    for p in 0..PMAX {
                        let e = total[idx(v, x, f, m, p)];
                        if e[0] == 0 {
                            continue;
                        }
                        let cells: Vec<String> = e.iter().map(i64::to_string).collect();
                        println!(
                            "{}\t{x}\t{f}\t{m}\t{p}\t{}",
                            &"HFP"[v..v + 1],
                            cells.join("\t")
                        );
                    }
                }
            }
        }
    }
}
