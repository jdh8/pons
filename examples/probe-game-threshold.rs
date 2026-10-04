//! probe-game-threshold — the suit-game point threshold per exact trump length.
//!
//! A plain statistic over the pre-solved `.pdd` bank (no solver, no bidding):
//! every partnership with an 8+ card fit in a major (or, with `--minor`, a
//! minor) is one case, bucketed by trump length (8..12+) and a point total on
//! three scales that never include trump length:
//!
//! - `A` — both hands on support points ([`support_point_count_in`]);
//! - `B` — the shorter-trump hand on support points, the longer on
//!   [`point_count`] (equal length counts both ways at half weight);
//! - `C` — raw HCP.
//!
//! Skipped: 8-card fits where one hand is (4333) and the other balanced — such
//! a pair plays 3NT.  Tricks are double-dummy with the better declarer of the
//! pair.  Per (scale, fit, points) cell it prints the doubled weight `w` (so a
//! half weight stays integral), the game-make count, and the sum and sum of
//! squares of IMPs, undoubled, not vul (`n`) and vul (`v`):
//!
//! - `g1` — game over one level lower (4M over 3M, 5m over 4m);
//! - `g2` — game over the alternative (4M over 2M, 5m over 3NT, each strain
//!   with its own better declarer).
//!
//! `scripts/game-threshold-report.py` turns the table into the break-even
//! points of `docs/major-game-threshold.md` and `docs/minor-game-threshold.md`.
//!
//!   cargo build --release --example probe-game-threshold
//!   RAYON_NUM_THREADS=8 nice -n 19 target/release/examples/probe-game-threshold \
//!       [--minor] /nfs2/jdh8/pons/22.pdd /nfs2/jdh8/pons/24.pdd \
//!       /nfs2/jdh8/pons/shard-*.pdd /mnt/hdd-data/jdh8/shards/shard-*.pdd > byfit.tsv
use clap::Parser;
use contract_bridge::eval::hcp;
use contract_bridge::{Bid, Contract, Hand, Penalty, Seat, Strain, Suit};
use pons::bidding::constraint::{point_count, support_point_count_in};
use pons::pdd::{MAGIC, ROW_LEN, decode_row};
use pons::scoring::imps;
use rayon::prelude::*;
use std::io::Read;

#[derive(Parser)]
struct Args {
    /// Minor fits (5m over 4m / 3NT) instead of majors (4M over 3M / 2M).
    #[arg(long)]
    minor: bool,
    /// `.pdd` deal files.
    files: Vec<String>,
}

const PMAX: usize = 48;
const FITS: usize = 5; // 8, 9, 10, 11, 12+
const N: usize = 3 * FITS * PMAX;
// w, make, then (sum, sumsq) for g1 nv, g1 vul, g2 nv, g2 vul
type Cell = [i64; 10];

fn idx(v: usize, fit: usize, p: usize) -> usize {
    (v * FITS + fit.min(12) - 8) * PMAX + p.min(PMAX - 1)
}

fn score(level: u8, s: Strain, tricks: u8, vul: bool) -> i64 {
    let contract = Contract {
        bid: Bid::new(level, s),
        penalty: Penalty::Undoubled,
    };
    i64::from(contract.score(tricks, vul))
}

fn suit_hcp(h: Hand) -> usize {
    Suit::ASC.iter().map(|&s| hcp::<usize>(h[s])).sum()
}

fn lengths(h: Hand) -> [usize; 4] {
    let mut l = Suit::ASC.map(|s| h[s].len());
    l.sort_unstable();
    l
}

fn flat(h: Hand) -> bool {
    lengths(h) == [3, 3, 3, 4]
}

fn balanced(h: Hand) -> bool {
    matches!(lengths(h), [3, 3, 3, 4] | [2, 3, 4, 4] | [2, 3, 3, 5])
}

fn scan(minor: bool, rows: &[u8], acc: &mut [Cell]) {
    let (suits, game) = if minor {
        ([Suit::Clubs, Suit::Diamonds], 5)
    } else {
        ([Suit::Hearts, Suit::Spades], 4)
    };
    for row in rows.as_chunks::<ROW_LEN>().0 {
        let (deal, table) = decode_row(row).expect("bad row");
        for (a, b) in [(Seat::North, Seat::South), (Seat::East, Seat::West)] {
            let (ha, hb) = (deal[a], deal[b]);
            let best = |s: Strain| u8::from(table[s].get(a)).max(u8::from(table[s].get(b)));
            for suit in suits {
                let (la, lb) = (ha[suit].len(), hb[suit].len());
                let fit = la + lb;
                if fit < 8 || fit == 8 && (flat(ha) && balanced(hb) || flat(hb) && balanced(ha)) {
                    continue;
                }
                let s = Strain::from(suit);
                // ponytail: best declarer of the pair (DD-optimistic for wrong-siding)
                let tricks = best(s);
                let sp = |h| usize::from(support_point_count_in(h, suit));
                let pc = |h| usize::from(point_count(h));
                let mut entries =
                    vec![(0, sp(ha) + sp(hb), 2), (2, suit_hcp(ha) + suit_hcp(hb), 2)];
                if la < lb {
                    entries.push((1, sp(ha) + pc(hb), 2));
                } else if lb < la {
                    entries.push((1, sp(hb) + pc(ha), 2));
                } else {
                    entries.push((1, sp(ha) + pc(hb), 1));
                    entries.push((1, sp(hb) + pc(ha), 1));
                }
                let mut gain = [0i64; 4];
                for (k, vul) in [(0, false), (1, true)] {
                    let g = score(game, s, tricks, vul);
                    gain[k] = imps(g - score(game - 1, s, tricks, vul));
                    gain[k + 2] = imps(
                        g - if minor {
                            score(3, Strain::Notrump, best(Strain::Notrump), vul)
                        } else {
                            score(2, s, tricks, vul)
                        },
                    );
                }
                for (v, p, w) in entries {
                    let e = &mut acc[idx(v, fit, p)];
                    e[0] += w;
                    e[1] += w * i64::from(tricks >= game + 6);
                    for k in 0..4 {
                        e[2 + 2 * k] += w * gain[k];
                        e[3 + 2 * k] += w * gain[k] * gain[k];
                    }
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
    let args = Args::parse();
    let chunk = 1 << 20; // rows
    let mut total = vec![[0i64; 10]; N];
    for f in &args.files {
        let mut file = std::fs::File::open(f).unwrap();
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
                scan(args.minor, c, &mut a);
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
    // Raw doubled-weight sums; the report script divides.
    println!("var\tfit\tP\tw\tmake\tg1n\tq1n\tg1v\tq1v\tg2n\tq2n\tg2v\tq2v");
    for v in 0..3 {
        for f in 0..FITS {
            for p in 0..PMAX {
                let e = total[(v * FITS + f) * PMAX + p];
                if e[0] == 0 {
                    continue;
                }
                let fit = ["8", "9", "10", "11", "12+"][f];
                let cells: Vec<String> = e.iter().map(i64::to_string).collect();
                println!("{}\t{fit}\t{p}\t{}", &"ABC"[v..v + 1], cells.join("\t"));
            }
        }
    }
}
