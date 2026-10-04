//! probe-nltc — NLTC against point counts, and what the trump suit owes it.
//!
//! A plain statistic over the pre-solved `.pdd` bank (no solver, no bidding):
//! every partnership whose longest combined suit has 8+ cards is one case,
//! with that suit as trumps (ties go to the suit taking more tricks) and
//! double-dummy tricks from the better declarer of the pair.  Each case is
//! counted once per evaluator, both hands summed:
//!
//! - `nltc` — [`NLTC`] in half losers;
//! - `sp` — support points ([`support_point_count_in`]);
//! - `hcp` — raw HCP;
//! - `zar` — Zar points ([`zar`]);
//! - `cover` — the losers of the hand with longer trumps (declarer; on a tie
//!   the one with fewer losers) that the other hand (dummy) does not cover,
//!   both hands seen ([`uncovered`]), in half losers;
//! - `blind` — declarer's losers less the cover cards dummy counts without
//!   seeing them ([`cover`]), in half losers plus [`BLIND_BASE`].
//!
//! and keyed by what the report conditions on: the pair's `shape` (0 both
//! balanced, 1 no singleton, 2 a singleton, 3 a void), `long7` (a 7+ card
//! suit), `major`, combined trumps `T` (8..11+), the shorter trump holding `m`
//! (0..4+), and three trump honours NLTC cannot see — `q` (the queen in a
//! holding of two or fewer), `k` (a singleton king), `j` (the jack).
//!
//! After the cells come the `moment` rows: the sums of products of the cover-card
//! regressors ([`terms`]), from which the report fits what each dummy feature
//! is worth to declarer's count.
//!
//! `scripts/nltc-report.py` turns the table into `docs/nltc.md`.
//!
//!   cargo build --release --example probe-nltc
//!   RAYON_NUM_THREADS=8 nice -n 19 target/release/examples/probe-nltc \
//!       /nfs2/jdh8/pons/22.pdd /nfs2/jdh8/pons/24.pdd \
//!       /nfs2/jdh8/pons/shard-*.pdd /mnt/hdd-data/jdh8/shards/shard-*.pdd > nltc.tsv
use contract_bridge::eval::{HandEvaluator, NLTC, hcp, zar};
use contract_bridge::{Hand, Rank, Seat, Strain, Suit};
use pons::bidding::constraint::support_point_count_in;
use pons::pdd::{MAGIC, ROW_LEN, decode_row};
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::Read;

const EVALS: [&str; 6] = ["nltc", "sp", "hcp", "zar", "cover", "blind"];
/// Keeps `blind` non-negative: dummy can count more covers than declarer has losers.
const BLIND_BASE: u8 = 40;
// (eval, value, shape, long7, major, T, m, q, k, j, tricks)
type Key = [u8; 11];
type Cells = HashMap<Key, u64>;
/// Regressors in [`terms`], the constant included; the tricks follow them.
const TERMS: usize = 22;
type Moments = [[u64; TERMS + 1]; TERMS + 1];
type Acc = (Cells, Moments);

/// NLTC charges a missing ace, king, queen to rounds 1, 2, 3 of a suit, at
/// 3, 2, 1 half losers.
const HONOURS: [(Rank, usize, u8); 3] = [(Rank::A, 1, 3), (Rank::K, 2, 2), (Rank::Q, 3, 1)];

/// Declarer's NLTC in half losers after dummy's actual cover cards: an honour
/// dummy holds covers the loser declarer counted for it, and a side suit dummy
/// is shorter in loses its last rounds to ruffs.
fn uncovered(decl: Hand, dummy: Hand, trump: Suit) -> u8 {
    // ponytail: ruffs go to suits in order and drawing trumps costs none
    let mut ruffs = dummy[trump].len();
    Suit::ASC
        .into_iter()
        .map(|s| {
            let (d, h) = (decl[s], dummy[s]);
            let mut rounds = d.len().min(3);
            if s != trump {
                let ruffed = rounds.saturating_sub(h.len()).min(ruffs);
                ruffs -= ruffed;
                rounds -= ruffed;
            }
            HONOURS
                .iter()
                .filter(|&&(r, round, _)| round <= rounds && !d.contains(r) && !h.contains(r))
                .map(|&(_, _, w)| w)
                .sum::<u8>()
        })
        .sum()
}

/// Dummy's cover cards in half losers, the fit of [`terms`] in round numbers:
/// ace 3, king 2 (a singleton side king 1), queen 1; a side void, singleton,
/// doubleton 3, 2, 1 with four trumps and 2, 1, 0 with three.
fn cover(dummy: Hand, trump: Suit) -> u8 {
    let ruffers = dummy[trump].len().clamp(2, 4) - 2;
    Suit::ASC
        .into_iter()
        .map(|s| {
            let h = dummy[s];
            let [a, k, q] = [Rank::A, Rank::K, Rank::Q].map(|r| u8::from(h.contains(r)));
            let bare_king = u8::from(s != trump && h.len() == 1) * k;
            // by dummy's trumps (two or fewer, three, four or more), then the suit's length
            let ruff = match (ruffers, h.len()) {
                _ if s == trump => 0,
                (2, 0) => 3,
                (2, 1) | (1, 0) => 2,
                (2, 2) | (1, 1) => 1,
                _ => 0,
            };
            3 * a + 2 * k - bare_king + q + ruff
        })
        .sum()
}

/// The cover-card regressors and the tricks: what dummy can count without
/// seeing declarer's hand.  The order is `TERMS` in `scripts/nltc-report.py`.
fn terms(decl: Hand, dummy: Hand, trump: Suit, losers: u8, tricks: u8) -> [u64; TERMS + 1] {
    let mut z = [0; TERMS + 1];
    z[0] = 1;
    z[1] = losers.into();
    // dummy's trumps: two or fewer, three, four or more
    let ruffers = dummy[trump].len().clamp(2, 4) - 2;
    for s in Suit::ASC {
        let h = dummy[s];
        let [a, k, q] = [Rank::A, Rank::K, Rank::Q].map(|r| u64::from(h.contains(r)));
        if s == trump {
            [z[7], z[8], z[9]] = [a, k, q];
            z[10] = u64::from(h.contains(Rank::J) || decl[s].contains(Rank::J));
            let fit = h.len() + decl[s].len();
            [z[20], z[21]] = [u64::from(fit >= 9), u64::from(fit >= 10)];
            continue;
        }
        z[2] += a;
        z[if h.len() >= 2 { 3 } else { 5 }] += k;
        z[if h.len() >= 3 { 4 } else { 6 }] += q;
        if h.len() < 3 {
            z[11 + 3 * ruffers + h.len()] += 1;
        }
    }
    z[TERMS] = tricks.into();
    z
}

fn lengths(h: Hand) -> [usize; 4] {
    let mut l = Suit::ASC.map(|s| h[s].len());
    l.sort_unstable();
    l
}

fn balanced(h: Hand) -> bool {
    matches!(lengths(h), [3, 3, 3, 4] | [2, 3, 4, 4] | [2, 3, 3, 5])
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn scan(rows: &[u8], (acc, moments): &mut Acc) {
    for row in rows.as_chunks::<ROW_LEN>().0 {
        let (deal, table) = decode_row(row).expect("bad row");
        for (a, b) in [(Seat::North, Seat::South), (Seat::East, Seat::West)] {
            let (ha, hb) = (deal[a], deal[b]);
            // ponytail: best declarer of the pair (DD-optimistic for wrong-siding)
            let best = |s: Suit| {
                let row = table[Strain::from(s)];
                u8::from(row.get(a)).max(u8::from(row.get(b)))
            };
            let trump = Suit::ASC
                .into_iter()
                .max_by_key(|&s| (ha[s].len() + hb[s].len(), best(s)))
                .unwrap();
            let (ta, tb) = (ha[trump], hb[trump]);
            if ta.len() + tb.len() < 8 {
                continue;
            }
            let (la, lb) = (lengths(ha), lengths(hb));
            let shape = match la[0].min(lb[0]) {
                0 => 3,
                1 => 2,
                _ => u8::from(!(balanced(ha) && balanced(hb))),
            };
            let short_queen = |t: contract_bridge::Holding| t.len() <= 2 && t.contains(Rank::Q);
            let bare_king = |t: contract_bridge::Holding| t.len() == 1 && t.contains(Rank::K);
            let suit_hcp = |h: Hand| Suit::ASC.iter().map(|&s| hcp::<u8>(h[s])).sum::<u8>();
            let (na, nb) = ((2.0 * NLTC.eval(ha)) as u8, (2.0 * NLTC.eval(hb)) as u8);
            let (decl, dummy, losers) = if (tb.len(), na) > (ta.len(), nb) {
                (hb, ha, nb)
            } else {
                (ha, hb, na)
            };
            let z = terms(decl, dummy, trump, losers, best(trump));
            for (row, x) in moments.iter_mut().zip(z) {
                for (m, y) in row.iter_mut().zip(z) {
                    *m += x * y;
                }
            }
            let values = [
                na + nb,
                support_point_count_in(ha, trump) + support_point_count_in(hb, trump),
                suit_hcp(ha) + suit_hcp(hb),
                zar::<u8>(ha) + zar::<u8>(hb),
                uncovered(decl, dummy, trump),
                BLIND_BASE + losers - cover(dummy, trump),
            ];
            for (e, v) in values.into_iter().enumerate() {
                let key = [
                    e as u8,
                    v,
                    shape,
                    u8::from(la[3].max(lb[3]) >= 7),
                    u8::from(matches!(trump, Suit::Hearts | Suit::Spades)),
                    (ta.len() + tb.len()).min(11) as u8,
                    ta.len().min(tb.len()).min(4) as u8,
                    u8::from(short_queen(ta) || short_queen(tb)),
                    u8::from(bare_king(ta) || bare_king(tb)),
                    u8::from(ta.contains(Rank::J) || tb.contains(Rank::J)),
                    best(trump),
                ];
                *acc.entry(key).or_default() += 1;
            }
        }
    }
}

fn add(mut x: Acc, y: Acc) -> Acc {
    for (k, n) in y.0 {
        *x.0.entry(k).or_default() += n;
    }
    for (row, other) in x.1.iter_mut().zip(y.1) {
        for (m, n) in row.iter_mut().zip(other) {
            *m += n;
        }
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
    let (cells, moments) = total;
    let mut cells: Vec<_> = cells.into_iter().collect();
    cells.sort_unstable();
    println!("eval\tvalue\tshape\tlong7\tmajor\tT\tm\tq\tk\tj\ttricks\tn");
    for (k, n) in cells {
        let fields: Vec<String> = k[1..].iter().map(u8::to_string).collect();
        println!("{}\t{}\t{n}", EVALS[usize::from(k[0])], fields.join("\t"));
    }
    for (i, row) in moments.iter().enumerate() {
        for (j, m) in row.iter().enumerate() {
            println!("moment\t{i}\t{j}\t{m}");
        }
    }
}
