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
//! - `zar` — Zar points ([`zar`]).
//!
//! and keyed by what the report conditions on: the pair's `shape` (0 both
//! balanced, 1 no singleton, 2 a singleton, 3 a void), `long7` (a 7+ card
//! suit), `major`, combined trumps `T` (8..11+), the shorter trump holding `m`
//! (0..4+), and three trump honours NLTC cannot see — `q` (the queen in a
//! holding of two or fewer), `k` (a singleton king), `j` (the jack).
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

const EVALS: [&str; 4] = ["nltc", "sp", "hcp", "zar"];
// (eval, value, shape, long7, major, T, m, q, k, j, tricks)
type Key = [u8; 11];
type Cells = HashMap<Key, u64>;

fn lengths(h: Hand) -> [usize; 4] {
    let mut l = Suit::ASC.map(|s| h[s].len());
    l.sort_unstable();
    l
}

fn balanced(h: Hand) -> bool {
    matches!(lengths(h), [3, 3, 3, 4] | [2, 3, 4, 4] | [2, 3, 3, 5])
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn scan(rows: &[u8], acc: &mut Cells) {
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
            let values = [
                (2.0 * (NLTC.eval(ha) + NLTC.eval(hb))) as u8,
                support_point_count_in(ha, trump) + support_point_count_in(hb, trump),
                suit_hcp(ha) + suit_hcp(hb),
                zar::<u8>(ha) + zar::<u8>(hb),
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

fn add(mut x: Cells, y: Cells) -> Cells {
    for (k, n) in y {
        *x.entry(k).or_default() += n;
    }
    x
}

fn main() {
    let chunk = 1 << 20; // rows
    let mut total = Cells::new();
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
                let mut a = Cells::new();
                scan(c, &mut a);
                a
            })
            .reduce(Cells::new, add);
        total = add(total, part);
        eprintln!("{f}: {} deals", bytes.len() / ROW_LEN);
    }
    let mut cells: Vec<_> = total.into_iter().collect();
    cells.sort_unstable();
    println!("eval\tvalue\tshape\tlong7\tmajor\tT\tm\tq\tk\tj\ttricks\tn");
    for (k, n) in cells {
        let fields: Vec<String> = k[1..].iter().map(u8::to_string).collect();
        println!("{}\t{}\t{n}", EVALS[usize::from(k[0])], fields.join("\t"));
    }
}
