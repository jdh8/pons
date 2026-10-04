//! probe-zar — Zar points against point counts, and what Zar owes for waste.
//!
//! A plain statistic over the pre-solved `.pdd` bank (no solver, no bidding).
//! Tricks are double-dummy from the better declarer of a pair.  Five tables,
//! one row per cell, the count last:
//!
//! - `suit` — every partnership whose longest combined suit has 8+ cards, with
//!   that suit as trumps (ties go to the suit taking more tricks):
//!   `eval value shape long7 major T tricks`;
//! - `nt` — every partnership, notrump tricks: `eval value shape fit majfit
//!   tricks` (`fit`: an 8+ card suit, `majfit`: an 8+ card major);
//! - `hand` — every single hand: `eval value shape own game off`, where `off`
//!   is the most tricks its side takes in any strain, `game` whether the side
//!   makes 3NT, 4M or 5m, and `own` whether the side holds the highest
//!   makeable contract of the deal;
//! - `open` — every single hand by the opening rules that admit it:
//!   `class shape own game off def` (`class` bits: 1 Zar ≥ 26, 2 HCP ≥ 12,
//!   4 Rule of 20, 8 `point_count` ≥ 12; `def` is the other side's `off`);
//! - `openhcp` — `class hcp`.
//!
//! `shape` is 0 balanced, 1 no singleton, 2 a singleton, 3 a void — of both
//! hands in the pair tables, of the one hand in the others.  The evaluators,
//! both hands summed in the pair tables: `hcp`; `pc` ([`point_count`]); `sp`
//! ([`support_point_count_in`]); `zar0` (Zar without its discount for short
//! honours); `zar` ([`zar`]); `fifths` (in half points); `r20` (HCP plus the
//! two longest suits); `zarw` and `zarx` ([`repaired`], in half points); `spc`
//! (`sp` plus Zar's controls, ace 2 and king 1).
//!
//! After the cells come the `moment` rows: per population (0 notrump tricks
//! with no 8-card fit, 1..=3 suit tricks with 8, 9, 10+ trumps) and pair shape,
//! the sums of products of the regressors in [`terms`].
//!
//! `scripts/zar-report.py` turns the table into `docs/zar.md`.
//!
//!   cargo build --release --example probe-zar
//!   RAYON_NUM_THREADS=8 nice -n 19 target/release/examples/probe-zar \
//!       /nfs2/jdh8/pons/22.pdd /nfs2/jdh8/pons/24.pdd \
//!       /nfs2/jdh8/pons/shard-*.pdd /mnt/hdd-data/jdh8/shards/shard-*.pdd > zar.tsv
use contract_bridge::eval::{fifths, hcp, zar};
use contract_bridge::{Hand, Holding, Rank, Seat, Strain, Suit};
use pons::bidding::constraint::{point_count, support_point_count_in};
use pons::pdd::{MAGIC, ROW_LEN, decode_row};
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::Read;

const TABLES: [&str; 5] = ["suit", "nt", "hand", "open", "openhcp"];
const EVALS: [&str; 10] = [
    "hcp", "pc", "sp", "zar0", "zar", "fifths", "r20", "zarw", "zarx", "spc",
];
const STRAINS: [Strain; 5] = [
    Strain::Clubs,
    Strain::Diamonds,
    Strain::Hearts,
    Strain::Spades,
    Strain::Notrump,
];
const HONOURS: [Rank; 5] = [Rank::A, Rank::K, Rank::Q, Rank::J, Rank::T];
// (table, then its fields as listed in the module doc, zero-padded)
type Key = [u8; 8];
/// Regressors in [`terms`], the constant included; the tricks follow them.
const TERMS: usize = 31;
/// Populations (4) by pair shapes (4).
const SETS: usize = 16;

struct Acc {
    cells: HashMap<Key, u64>,
    moments: Vec<u64>,
}

impl Default for Acc {
    fn default() -> Self {
        Self {
            cells: HashMap::new(),
            moments: vec![0; SETS * (TERMS + 1) * (TERMS + 1)],
        }
    }
}

fn lengths(h: Hand) -> [usize; 4] {
    let mut l = Suit::ASC.map(|s| h[s].len());
    l.sort_unstable();
    l
}

/// 0 balanced, 1 no singleton, 2 a singleton, 3 a void.
fn shape(l: [usize; 4]) -> u8 {
    match l {
        [0, ..] => 3,
        [1, ..] => 2,
        [3, 3, 3, 4] | [2, 3, 4, 4] | [2, 3, 3, 5] => 0,
        _ => 1,
    }
}

fn raw_hcp(h: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| hcp::<u8>(h[s])).sum()
}

/// The holdings [`zar`] discounts by a point: a singleton K, Q or J, and a
/// doubleton holding a Q or J.
fn discounted(x: Holding) -> bool {
    let [_, k, q, j, _] = HONOURS.map(|r| x.contains(r));
    match x.len() {
        1 => k || q || j,
        2 => q || j,
        _ => false,
    }
}

/// The regressors and the tricks.  `trump` is `None` at notrump, where every
/// suit is a side suit.  The order is `TERMS` in `scripts/zar-report.py`.
fn terms(ha: Hand, hb: Hand, trump: Option<Suit>, tricks: u8) -> [u64; TERMS + 1] {
    let mut z = [0; TERMS + 1];
    z[0] = 1;
    for (h, p) in [(ha, hb), (hb, ha)] {
        let l = lengths(h);
        z[6] += l[3] as u64;
        z[7] += l[2] as u64;
        z[8] += l[0] as u64;
        for s in Suit::ASC {
            let x = h[s];
            let hon = HONOURS.map(|r| u64::from(x.contains(r)));
            for (i, n) in hon.into_iter().enumerate() {
                z[1 + i] += n;
            }
            if Some(s) == trump {
                for (i, n) in hon.into_iter().enumerate() {
                    z[23 + i] += n;
                }
                z[14] += u64::from(discounted(x));
                continue;
            }
            let [a, k, q, j, _] = hon;
            match x.len() {
                1 => {
                    z[9] += k;
                    z[10] += q;
                    z[11] += j;
                }
                2 if q + j > 0 => z[if a + k > 0 { 13 } else { 12 }] += 1,
                _ => {}
            }
            // honours opposite partner's void (15..) or singleton (19..)
            let short = p[s].len();
            if short < 2 {
                for (i, n) in hon.into_iter().take(4).enumerate() {
                    z[15 + 4 * short + i] += n;
                }
            }
        }
    }
    // the shorter trump holding has 2, 1, 0 cards
    if let Some(m) = trump
        .map(|t| ha[t].len().min(hb[t].len()))
        .filter(|&m| m < 3)
    {
        z[30 - m] = 1;
    }
    z[TERMS] = tricks.into();
    z
}

/// Zar with the terms of the report in round numbers, in half Zar points.
///
/// `zarw` repairs the waste: a singleton side king loses 2½ points, not 1;
/// facing partner's void a side ace loses 2½, a king 2 and a queen ½; facing
/// partner's singleton a king loses 2 and a queen 1.
///
/// `zarx` adds the trump terms: 2 for the 9th trump and ½ for the 10th; less
/// 2½, 4½ or 6 when the shorter trump holding is a doubleton, singleton or
/// void; no discount for a short trump honour; and with eight trumps a point
/// for each of the trump queen, jack and ten.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn repaired(ha: Hand, hb: Hand, trump: Suit) -> [u8; 2] {
    let (ta, tb) = (ha[trump], hb[trump]);
    let fit = ta.len() + tb.len();
    let mut w = 2 * i32::from(zar::<u8>(ha) + zar::<u8>(hb));
    let mut x = match fit {
        ..=8 => 0,
        9 => 4,
        _ => 5,
    };
    x -= [12, 9, 5].get(ta.len().min(tb.len())).copied().unwrap_or(0);
    for t in [ta, tb] {
        x += 2 * i32::from(discounted(t));
        if fit == 8 {
            x += 2 * HONOURS[2..].iter().filter(|&&r| t.contains(r)).count() as i32;
        }
    }
    for (h, p) in [(ha, hb), (hb, ha)] {
        for s in Suit::ASC.into_iter().filter(|&s| s != trump) {
            let [a, k, q, _, _] = HONOURS.map(|r| i32::from(h[s].contains(r)));
            w -= 3 * i32::from(h[s].len() == 1) * k;
            w -= match p[s].len() {
                0 => 5 * a + 4 * k + q,
                1 => 4 * k + 2 * q,
                _ => 0,
            };
        }
    }
    [w.max(0) as u8, (w + x).max(0) as u8]
}

#[allow(clippy::cast_possible_truncation)]
fn scan(rows: &[u8], acc: &mut Acc) {
    let mut count = |key: Key| *acc.cells.entry(key).or_default() += 1;
    for row in rows.as_chunks::<ROW_LEN>().0 {
        let (deal, table) = decode_row(row).expect("bad row");
        let sides = [(Seat::North, Seat::South), (Seat::East, Seat::West)];
        // ponytail: best declarer of the pair (DD-optimistic for wrong-siding)
        let tricks = sides.map(|(a, b)| {
            STRAINS.map(|s| u8::from(table[s].get(a)).max(u8::from(table[s].get(b))))
        });
        // the highest makeable contract: most tricks, then the higher strain
        let top = tricks.map(|t| (0..5).map(|i| (t[i], i)).max().unwrap());
        let off = top.map(|(t, _)| t);
        let game = tricks.map(|t| t[4] >= 9 || t[2].max(t[3]) >= 10 || t[0].max(t[1]) >= 11);

        for (i, (a, b)) in sides.into_iter().enumerate() {
            let own = u8::from(off[i] >= 7 && top[i] > top[1 - i]);
            let (ha, hb) = (deal[a], deal[b]);
            let (la, lb) = (lengths(ha), lengths(hb));
            let fit = |s: Suit| ha[s].len() + hb[s].len();
            let trump = Suit::ASC
                .into_iter()
                .max_by_key(|&s| (fit(s), tricks[i][s as usize]))
                .unwrap();
            let (t, ts, tn) = (fit(trump), tricks[i][trump as usize], tricks[i][4]);
            let shp = shape(la).max(shape(lb));
            let zars = zar::<u8>(ha) + zar::<u8>(hb);
            let waste = Suit::ASC
                .iter()
                .map(|&s| u8::from(discounted(ha[s])) + u8::from(discounted(hb[s])))
                .sum::<u8>();
            let fifths2 = Suit::ASC
                .iter()
                .map(|&s| fifths(ha[s]) + fifths(hb[s]))
                .sum::<f64>();
            let [zarw, zarx] = repaired(ha, hb, trump);
            let sp = support_point_count_in(ha, trump) + support_point_count_in(hb, trump);
            let controls = Suit::ASC
                .iter()
                .flat_map(|&s| [ha[s], hb[s]])
                .map(|x| 2 * u8::from(x.contains(Rank::A)) + u8::from(x.contains(Rank::K)))
                .sum::<u8>();
            let values = [
                raw_hcp(ha) + raw_hcp(hb),
                point_count(ha) + point_count(hb),
                sp,
                zars + waste,
                zars,
                (2.0 * fifths2).round() as u8,
                0,
                zarw,
                zarx,
                sp + controls,
            ];
            let majfit = fit(Suit::Hearts).max(fit(Suit::Spades)) >= 8;
            for e in [0, 1, 3, 4, 5] {
                let (fit, majfit) = (u8::from(t >= 8), u8::from(majfit));
                count([1, e, values[usize::from(e)], shp, fit, majfit, tn, 0]);
            }
            if t >= 8 {
                let long7 = u8::from(la[3].max(lb[3]) >= 7);
                let major = u8::from(matches!(trump, Suit::Hearts | Suit::Spades));
                for e in [0, 1, 2, 3, 4, 7, 8, 9] {
                    let v = values[usize::from(e)];
                    count([0, e, v, shp, long7, major, t.min(11) as u8, ts]);
                }
            }
            let (set, z) = if t >= 8 {
                (t.min(10) - 7, terms(ha, hb, Some(trump), ts))
            } else {
                (0, terms(ha, hb, None, tn))
            };
            let width = TERMS + 1;
            let start = (4 * set + usize::from(shp)) * width * width;
            for (j, x) in z.into_iter().enumerate() {
                for (k, y) in z.into_iter().enumerate() {
                    acc.moments[start + j * width + k] += x * y;
                }
            }

            for h in [ha, hb] {
                let l = lengths(h);
                let (shp, points, pc) = (shape(l), raw_hcp(h), point_count(h));
                let r20 = points + (l[2] + l[3]) as u8;
                let zars = zar::<u8>(h);
                let waste = Suit::ASC
                    .iter()
                    .map(|&s| u8::from(discounted(h[s])))
                    .sum::<u8>();
                let (game, def) = (u8::from(game[i]), off[1 - i]);
                for (e, v) in [(0, points), (1, pc), (3, zars + waste), (4, zars), (6, r20)] {
                    count([2, e, v, shp, own, game, off[i], 0]);
                }
                let class = u8::from(zars >= 26)
                    | u8::from(points >= 12) << 1
                    | u8::from(r20 >= 20) << 2
                    | u8::from(pc >= 12) << 3;
                count([3, class, shp, own, game, off[i], def, 0]);
                count([4, class, points, 0, 0, 0, 0, 0]);
            }
        }
    }
}

fn add(mut x: Acc, y: Acc) -> Acc {
    for (k, n) in y.cells {
        *x.cells.entry(k).or_default() += n;
    }
    for (m, n) in x.moments.iter_mut().zip(y.moments) {
        *m += n;
    }
    x
}

fn main() {
    let chunk = 1 << 20; // rows
    let mut total = Acc::default();
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
                let mut a = Acc::default();
                scan(c, &mut a);
                a
            })
            .reduce(Acc::default, add);
        total = add(total, part);
        eprintln!("{f}: {} deals", bytes.len() / ROW_LEN);
    }
    let mut cells: Vec<_> = total.cells.into_iter().collect();
    cells.sort_unstable();
    for (k, n) in cells {
        let table = usize::from(k[0]);
        let mut fields: Vec<String> = k[1..].iter().map(u8::to_string).collect();
        if table < 3 {
            fields[0] = EVALS[usize::from(k[1])].to_owned();
        }
        println!("{}\t{}\t{n}", TABLES[table], fields.join("\t"));
    }
    let width = TERMS + 1;
    for (i, m) in total.moments.iter().enumerate() {
        let (set, cell) = (i / (width * width), i % (width * width));
        println!("moment\t{set}\t{}\t{}\t{m}", cell / width, cell % width);
    }
}
