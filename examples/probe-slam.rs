//! probe-slam — which count decides a slam, in a suit and at notrump.
//!
//! A plain statistic over the pre-solved `.pdd` bank (no solver, no bidding).
//! Tricks are double-dummy from the better declarer of a pair, both hands
//! summed on every count.  Two tables, one row per cell, the cases last:
//!
//! - `suit` — every partnership whose longest combined suit has 8+ cards, with
//!   that suit as trumps (ties go to the suit taking more tricks):
//!   `eval value shape major T keys open tricks`;
//! - `nt` — every partnership, notrump tricks: `eval value pop keys open
//!   tricks` (`pop`: 0 both hands balanced and no 8-card major fit, 1 both
//!   balanced with one, 2 unbalanced without one, 3 the rest).
//!
//! `shape` is 0 both balanced, 1 no singleton, 2 a singleton, 3 a void.
//! `keys` is the keycards missing, capped at 2: the aces, and in a suit the
//! trump king.  `open` is whether some suit has two top losers: neither the
//! ace nor the king, and at notrump or in trumps nothing more; in a side suit
//! also two or more cards in each hand.
//!
//! The counts are [`suit_values`] and [`notrump_values`]; the last three of
//! each are [`SUIT_COUNTS`] and [`NOTRUMP_COUNTS`], weights on the regressors.
//!
//! After the cells come the `moment` rows, the sums of products of the
//! regressors in [`suit_terms`] (`smoment`, per zone of support points +
//! trumps and trump length) and [`notrump_terms`] (`nmoment`, per zone of HCP
//! and `pop`).
//!
//! `scripts/slam-report.py` turns the table into `docs/suit-slam.md` and
//! `docs/notrump-slam.md`.
//!
//!   cargo build --release --example probe-slam
//!   RAYON_NUM_THREADS=8 nice -n 19 target/release/examples/probe-slam \
//!       /nfs2/jdh8/pons/22.pdd /nfs2/jdh8/pons/24.pdd \
//!       /nfs2/jdh8/pons/shard-*.pdd /mnt/hdd-data/jdh8/shards/shard-*.pdd > slam.tsv
use contract_bridge::eval::{HandEvaluator, NLTC, bumrap, fifths, hcp, zar};
use contract_bridge::{Hand, Rank, Seat, Strain, Suit};
use pons::bidding::constraint::{point_count, support_point_count_in};
use pons::pdd::{MAGIC, ROW_LEN, decode_row};
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::Read;

const SUIT_EVALS: [&str; 9] = [
    "hcp", "pc", "sp", "spc", "zar", "nltc", "aq", "slam", "seen",
];
const NOTRUMP_EVALS: [&str; 9] = [
    "hcp", "fifths", "bumrap", "hc", "pc", "zar", "hl", "fl", "nts",
];
const HONOURS: [Rank; 5] = [Rank::A, Rank::K, Rank::Q, Rank::J, Rank::T];
// (table, then its fields as listed in the module doc, zero-padded)
type Key = [u16; 9];
/// Regressors in [`suit_terms`], the constant included; the [`TARGETS`] follow.
const SUIT_TERMS: usize = 34;
/// Regressors in [`notrump_terms`], the constant included; the [`TARGETS`] follow.
const NOTRUMP_TERMS: usize = 24;
/// The tricks, whether they are 12 or more, and whether they are 13.
const TARGETS: usize = 3;
/// Zones (4) by trump lengths (3), and zones (4) by `pop` (4).
const SUIT_SETS: usize = 12;
const NOTRUMP_SETS: usize = 16;

struct Acc {
    cells: HashMap<Key, u64>,
    suit: Vec<u64>,
    notrump: Vec<u64>,
}

impl Default for Acc {
    fn default() -> Self {
        Self {
            cells: HashMap::new(),
            suit: vec![0; SUIT_SETS * (SUIT_TERMS + TARGETS) * (SUIT_TERMS + TARGETS)],
            notrump: vec![0; NOTRUMP_SETS * (NOTRUMP_TERMS + TARGETS) * (NOTRUMP_TERMS + TARGETS)],
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

/// Zar's controls: ace 2, king 1.
fn controls(h: Hand) -> u8 {
    Suit::ASC
        .iter()
        .map(|&s| 2 * u8::from(h[s].contains(Rank::A)) + u8::from(h[s].contains(Rank::K)))
        .sum()
}

/// Slam counts built on the regressors of [`suit_terms`], in eighths of a
/// point, an ace being 4 points.
///
/// - `aq`: each honour half the one above (ace 4, king 2, queen 1, jack ½)
///   and a side void, singleton, doubleton 3, 1½, ½.
/// - `slam`: the fit of the chance of 12 tricks with the hands apart.
/// - `seen`: the same fit with partner's shortness known.
#[rustfmt::skip]
const SUIT_COUNTS: [[i16; SUIT_TERMS - 1]; 3] = [
    [32, 16, 8, 4, 0,  32, 16, 8, 4, 0,  24, 12, 4,  24, 12, 4,  24, 12, 4,
     0, 0, 0,  0, 0,  0, 0, 0, 0,  0, 0, 0, 0,  0],
    [32, 15, 7, 3, 1,  34, 20, 10, 5, 2,  23, 11, 3,  22, 11, 3,  17, 8, 2,
     -6, -3, -2,  -2, -1,  0, 0, 0, 0,  0, 0, 0, 0,  4],
    [32, 17, 7, 3, 1,  30, 18, 9, 4, 2,  44, 18, 4,  41, 18, 4,  34, 15, 4,
     -11, -4, -1,  -2, -1,  -23, -13, -5, -2,  -2, -10, -2, 0,  1],
];

/// Notrump slam counts built on the regressors of [`notrump_terms`], in
/// twentieths of a point.
///
/// - `hl`: HCP, and ⅔ and ⅓ of a point for the first two length points.
/// - `fl`: Fifths, and the same for length.
/// - `nts`: the fit of the chance of 12 tricks in the slam zone.
#[rustfmt::skip]
const NOTRUMP_COUNTS: [[i16; NOTRUMP_TERMS - 1]; 3] = [
    [80, 60, 40, 20, 0, 0,  13, 6, 0, 0,  0,  0, 0, 0,  0, 0, 0,  0, 0, 0,  0, 0, 0],
    [80, 56, 36, 20, 8, 0,  13, 6, 0, 0,  0,  0, 0, 0,  0, 0, 0,  0, 0, 0,  0, 0, 0],
    [80, 54, 32, 17, 7, 3,  13, 6, 8, 8,  -4,  0, 0, 0,  -8, -8, -8,  -3, -3, -5,  0, 0, 0],
];

/// A count of weights on the regressors after the constant.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
fn weigh(weights: &[i16], z: &[u64]) -> u16 {
    let sum: i64 = weights
        .iter()
        .zip(&z[1..])
        .map(|(&w, &x)| i64::from(w) * x as i64)
        .sum();
    sum.max(0) as u16
}

/// The counts of the `suit` table, in the order of [`SUIT_EVALS`]: raw HCP,
/// `point_count`, support points, support points plus controls, Zar, NLTC in
/// half losers, and [`SUIT_COUNTS`].
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn suit_values(ha: Hand, hb: Hand, trump: Suit, z: &[u64]) -> [u16; 9] {
    let sp = support_point_count_in(ha, trump) + support_point_count_in(hb, trump);
    let [aq, slam, seen] = SUIT_COUNTS.map(|w| weigh(&w, z));
    [
        (raw_hcp(ha) + raw_hcp(hb)).into(),
        (point_count(ha) + point_count(hb)).into(),
        sp.into(),
        (sp + controls(ha) + controls(hb)).into(),
        (zar::<u8>(ha) + zar::<u8>(hb)).into(),
        (2.0 * (NLTC.eval(ha) + NLTC.eval(hb))) as u16,
        aq,
        slam,
        seen,
    ]
}

/// The counts of the `nt` table, in the order of [`NOTRUMP_EVALS`]: raw HCP,
/// Fifths in half points, BUM-RAP in quarter points, HCP plus controls,
/// `point_count`, Zar, and [`NOTRUMP_COUNTS`].
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn notrump_values(ha: Hand, hb: Hand, z: &[u64]) -> [u16; 9] {
    let sum = |f: fn(contract_bridge::Holding) -> f64| {
        Suit::ASC.iter().map(|&s| f(ha[s]) + f(hb[s])).sum::<f64>()
    };
    let points = raw_hcp(ha) + raw_hcp(hb);
    let [hl, fl, nts] = NOTRUMP_COUNTS.map(|w| weigh(&w, z));
    [
        points.into(),
        (2.0 * sum(fifths)).round() as u16,
        (4.0 * sum(bumrap)).round() as u16,
        (points + controls(ha) + controls(hb)).into(),
        (point_count(ha) + point_count(hb)).into(),
        (zar::<u8>(ha) + zar::<u8>(hb)).into(),
        hl,
        fl,
        nts,
    ]
}

fn targets(z: &mut [u64], tricks: u8) {
    z.copy_from_slice(&[tricks.into(), (tricks >= 12).into(), (tricks == 13).into()]);
}

/// The suit regressors and the targets.  The order is `SUIT_TERMS` in
/// `scripts/slam-report.py`.
fn suit_terms(ha: Hand, hb: Hand, trump: Suit, tricks: u8) -> [u64; SUIT_TERMS + TARGETS] {
    let mut z = [0; SUIT_TERMS + TARGETS];
    z[0] = 1;
    for (h, p) in [(ha, hb), (hb, ha)] {
        // this hand's trumps: four or more, three, two or fewer
        let ruffers = 4 - h[trump].len().clamp(2, 4);
        for s in Suit::ASC {
            let x = h[s];
            let hon = HONOURS.map(|r| u64::from(x.contains(r)));
            let base = if s == trump { 6 } else { 1 };
            for (i, n) in hon.into_iter().enumerate() {
                z[base + i] += n;
            }
            if s == trump {
                continue;
            }
            let [a, k, q, j, _] = hon;
            if x.len() < 3 {
                z[11 + 3 * ruffers + x.len()] += 1;
            }
            match x.len() {
                1 => {
                    z[20] += k;
                    z[21] += q;
                    z[22] += j;
                }
                2 if q + j > 0 => z[if a + k > 0 { 24 } else { 23 }] += 1,
                _ => {}
            }
            // honours facing partner's void (25..) or singleton (29..)
            let short = p[s].len();
            if short < 2 {
                for (i, n) in hon.into_iter().take(4).enumerate() {
                    z[25 + 4 * short + i] += n;
                }
            }
        }
    }
    let second = Suit::ASC
        .into_iter()
        .filter(|&s| s != trump)
        .map(|s| ha[s].len() + hb[s].len())
        .max();
    z[33] = u64::from(second >= Some(8));
    targets(&mut z[SUIT_TERMS..], tricks);
    z
}

/// The notrump regressors and the targets.  The order is `NOTRUMP_TERMS` in
/// `scripts/slam-report.py`.
fn notrump_terms(ha: Hand, hb: Hand, tricks: u8) -> [u64; NOTRUMP_TERMS + TARGETS] {
    let mut z = [0; NOTRUMP_TERMS + TARGETS];
    z[0] = 1;
    let (mut long, mut fit) = (0, 0);
    for (h, p) in [(ha, hb), (hb, ha)] {
        z[11] += u64::from(lengths(h) == [3, 3, 3, 4]);
        for s in Suit::ASC {
            let x = h[s];
            let hon = HONOURS.map(|r| u64::from(x.contains(r)));
            for (i, n) in hon.into_iter().enumerate() {
                z[1 + i] += n;
            }
            z[6] += u64::from(x.contains(Rank::new(9)));
            long += x.len().saturating_sub(4);
            fit = fit.max(x.len() + p[s].len());
            let [a, k, q, j, _] = hon;
            match x.len() {
                1 => {
                    z[15] += k;
                    z[16] += q;
                    z[17] += j;
                }
                2 if a + k == 2 => z[20] += 1,
                2 if q + j > 0 => z[if a + k > 0 { 19 } else { 18 }] += 1,
                _ => {}
            }
            if p[s].len() < 2 {
                z[21] += k;
                z[22] += q;
                z[23] += j;
            }
        }
    }
    // length points (cards beyond four): at least 1, 2, 3, 4
    for i in 1..=4 {
        z[6 + i] = u64::from(long >= i);
    }
    // the longest combined suit: at least 8, 9, 10 cards
    for i in 0..3 {
        z[12 + i] = u64::from(fit >= 8 + i);
    }
    targets(&mut z[NOTRUMP_TERMS..], tricks);
    z
}

fn push(moments: &mut [u64], set: usize, z: &[u64]) {
    let width = z.len();
    let start = set * width * width;
    for (j, x) in z.iter().enumerate() {
        for (k, y) in z.iter().enumerate() {
            moments[start + j * width + k] += x * y;
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn scan(rows: &[u8], acc: &mut Acc) {
    for row in rows.as_chunks::<ROW_LEN>().0 {
        let (deal, table) = decode_row(row).expect("bad row");
        for (a, b) in [(Seat::North, Seat::South), (Seat::East, Seat::West)] {
            let (ha, hb) = (deal[a], deal[b]);
            // ponytail: best declarer of the pair (DD-optimistic for wrong-siding)
            let best = |s: Strain| u8::from(table[s].get(a)).max(u8::from(table[s].get(b)));
            let fit = |s: Suit| ha[s].len() + hb[s].len();
            let trump = Suit::ASC
                .into_iter()
                .max_by_key(|&s| (fit(s), best(s.into())))
                .unwrap();
            let (t, ts, tn) = (fit(trump), best(trump.into()), best(Strain::Notrump));
            let (sa, sb) = (shape(lengths(ha)), shape(lengths(hb)));
            let holds = |s: Suit, r: Rank| ha[s].contains(r) || hb[s].contains(r);
            let aces = Suit::ASC.iter().filter(|&&s| !holds(s, Rank::A)).count();
            let bare = |s: Suit| !holds(s, Rank::A) && !holds(s, Rank::K);
            let points = raw_hcp(ha) + raw_hcp(hb);

            let majfit = fit(Suit::Hearts).max(fit(Suit::Spades)) >= 8;
            let pop = 2 * u16::from(sa.max(sb) > 0) + u16::from(majfit);
            let keys = aces.min(2) as u16;
            let open = u16::from(Suit::ASC.into_iter().any(bare));
            let z = notrump_terms(ha, hb, tn);
            for (e, v) in notrump_values(ha, hb, &z).into_iter().enumerate() {
                let key = [1, e as u16, v, pop, keys, open, tn.into(), 0, 0];
                *acc.cells.entry(key).or_default() += 1;
            }
            let zone = usize::from(points.clamp(24, 39) / 4 - 6);
            push(&mut acc.notrump, 4 * zone + usize::from(pop), &z);

            if t < 8 {
                continue;
            }
            let z = suit_terms(ha, hb, trump, ts);
            let values = suit_values(ha, hb, trump, &z);
            let major = u16::from(matches!(trump, Suit::Hearts | Suit::Spades));
            let keys = (aces + usize::from(!holds(trump, Rank::K))).min(2) as u16;
            let two = |s: Suit| s == trump || ha[s].len().min(hb[s].len()) >= 2;
            let open = u16::from(Suit::ASC.into_iter().any(|s| bare(s) && two(s)));
            let (shape, t) = (sa.max(sb).into(), t.min(11));
            for (e, v) in values.into_iter().enumerate() {
                let key = [
                    0,
                    e as u16,
                    v,
                    shape,
                    major,
                    t as u16,
                    keys,
                    open,
                    ts.into(),
                ];
                *acc.cells.entry(key).or_default() += 1;
            }
            // support points + trumps: under 34, 34 to 37, 38 to 41, 42 and over
            let gate = usize::from(values[2]) + t;
            let set = 3 * ((gate.clamp(30, 45) + 2) / 4 - 8) + t.min(10) - 8;
            push(&mut acc.suit, set, &z);
        }
    }
}

fn add(mut x: Acc, y: Acc) -> Acc {
    for (k, n) in y.cells {
        *x.cells.entry(k).or_default() += n;
    }
    for (m, n) in x.suit.iter_mut().zip(y.suit) {
        *m += n;
    }
    for (m, n) in x.notrump.iter_mut().zip(y.notrump) {
        *m += n;
    }
    x
}

fn print_moments(name: &str, moments: &[u64], width: usize) {
    for (i, m) in moments.iter().enumerate() {
        let (set, cell) = (i / (width * width), i % (width * width));
        println!("{name}\t{set}\t{}\t{}\t{m}", cell / width, cell % width);
    }
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
        let (table, evals) = if k[0] == 0 {
            ("suit", &SUIT_EVALS[..])
        } else {
            ("nt", &NOTRUMP_EVALS[..])
        };
        let fields: Vec<String> = k[2..].iter().map(u16::to_string).collect();
        println!(
            "{table}\t{}\t{}\t{n}",
            evals[usize::from(k[1])],
            fields.join("\t")
        );
    }
    print_moments("smoment", &total.suit, SUIT_TERMS + TARGETS);
    print_moments("nmoment", &total.notrump, NOTRUMP_TERMS + TARGETS);
}
