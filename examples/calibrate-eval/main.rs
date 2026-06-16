//! Calibrate hand evaluators against a precomputed double-dummy database.
//!
//! Reads the GIB-format `sol100000.txt` (West-first deal, then 20 hex digits of
//! the DD table: strains `NT,S,H,D,C`, declarers `E,N,W,S`, with E/W stored as
//! `13 - tricks`) — verified against the solver — so it costs **no** DD solving.
//!
//! For every partnership, with `s = eᴺ + eˢ` the combined evaluation and
//! `d = |eᴺ − eˢ|` the imbalance between the two hands, it fits:
//!
//!   M0  tricks = a + b·s            — the points/losers → tricks mapping
//!   M2  tricks = a + b·s + c·d      — additivity: does the *split* matter?
//!
//! (Linear split-invariance `bᴺ = bˢ` is trivially exact here — every evaluator
//! is itself a per-hand sum and random deals are N/S-symmetric — so the live
//! question is concentration: M2's `c ≈ 0` with `sd₂ ≈ sd₀` means the bare sum
//! is sufficient; `c ≠ 0` means a lopsided split plays differently than a
//! balanced one at equal total, i.e. the evaluator is not perfectly additive.)
//!
//! Usage: `cargo run --release --example calibrate-eval [path/to/sol100000.txt]`

use contract_bridge::eval::{self, HandEvaluator};
use contract_bridge::{Seat, Strain, Suit};
use nalgebra as na;

const DEFAULT_PATH: &str = "../ddss-sys/vendor/hands/sol100000.txt";

const NAMES: [&str; 7] = ["hcp", "fifths", "bumrap", "ltc", "nltc", "zar", "cccc"];
const EVALUATORS: [&dyn HandEvaluator<f64>; 7] = [
    &eval::SimpleEvaluator(eval::hcp::<f64>),
    &eval::FIFTHS,
    &eval::BUMRAP,
    &eval::SimpleEvaluator(eval::ltc::<f64>),
    &eval::NLTC,
    &eval::zar,
    &eval::cccc,
];

// GIB decode tables (see module docs / parse_GIB): strain slots then declarers.
const GIB_STRAINS: [Strain; 5] = [
    Strain::Notrump,
    Strain::Spades,
    Strain::Hearts,
    Strain::Diamonds,
    Strain::Clubs,
];
const GIB_SEATS: [Seat; 4] = [Seat::East, Seat::North, Seat::West, Seat::South];

const PAIRS: [(Seat, Seat); 2] = [(Seat::North, Seat::South), (Seat::East, Seat::West)];
const NT: usize = Strain::Notrump as usize;
const MIN_FIT: usize = 8; // suit context conditions on an 8+-card trump fit

/// `tbl[strain as usize][seat as usize]` = double-dummy tricks for that declarer.
fn decode_table(hex: &[u8]) -> [[u8; 4]; 5] {
    let mut tbl = [[0u8; 4]; 5];
    for (s, &strain) in GIB_STRAINS.iter().enumerate() {
        for (h, &seat) in GIB_SEATS.iter().enumerate() {
            let raw = (hex[4 * s + h] as char).to_digit(16).unwrap() as u8;
            let tricks = if matches!(seat, Seat::East | Seat::West) {
                13 - raw
            } else {
                raw
            };
            tbl[strain as usize][seat as usize] = tricks;
        }
    }
    tbl
}

/// Regression summary from the moment matrix `m = Σ vᵥᵀ`, `v = [1, s, d, y]`
/// where `s = eᴺ + eˢ`, `d = |eᴺ − eˢ|`.
struct Fit {
    n: f64,
    slope: f64,     // M0: tricks per unit of s
    intercept: f64, // M0
    sd0: f64,       // M0 residual std (tricks)
    r2: f64,        // M0
    conc: f64,      // M2 coefficient on d (concentration effect)
    sd2: f64,       // M2 residual std (tricks)
}

fn fit(m: &na::Matrix4<f64>) -> Fit {
    let n = m[(0, 0)];
    let sy = m[(0, 3)];
    let syy = m[(3, 3)];
    let sst = syy - sy * sy / n;

    // M0: regress y on [1, s]
    let (ss, sss, ssy) = (m[(0, 1)], m[(1, 1)], m[(1, 3)]);
    let det = n * sss - ss * ss;
    let slope = (n * ssy - ss * sy) / det;
    let intercept = (sy - slope * ss) / n;
    let sse0 = syy - intercept * sy - slope * ssy;
    let sd0 = (sse0 / (n - 2.0)).sqrt();

    // M2: regress y on [1, s, d] — its normal matrix is m's top-left 3×3
    let a = m.fixed_view::<3, 3>(0, 0).into_owned();
    let rhs = na::Vector3::new(m[(0, 3)], m[(1, 3)], m[(2, 3)]);
    let beta = a.lu().solve(&rhs).expect("singular normal equations");
    let sse2 = syy - beta.dot(&rhs);
    let sd2 = (sse2 / (n - 3.0)).sqrt();

    Fit {
        n,
        slope,
        intercept,
        sd0,
        r2: 1.0 - sse0 / sst,
        conc: beta[2],
        sd2,
    }
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_PATH.into());
    let text = std::fs::read_to_string(&path).expect("read sol file");

    // moment[eval][context], context 0 = NT, 1 = best suit fit
    let mut moment = [[na::Matrix4::<f64>::zeros(); 2]; 7];

    for line in text.lines().filter(|l| l.len() == 88) {
        let deal: contract_bridge::FullDeal =
            format!("W:{}", &line[0..67]).parse().expect("parse deal");
        let tbl = decode_table(&line.as_bytes()[68..88]);

        for (a, b) in PAIRS {
            let (ha, hb) = (deal[a], deal[b]);
            // NT target: best of the two declarers
            let y_nt = f64::from(tbl[NT][a as usize].max(tbl[NT][b as usize]));
            // longest combined suit = the trump fit
            let (mut best, mut best_len) = (Suit::Clubs, 0);
            for suit in Suit::ASC {
                let len = ha[suit].len() + hb[suit].len();
                if len > best_len {
                    (best, best_len) = (suit, len);
                }
            }
            let y_suit =
                f64::from(tbl[best as usize][a as usize].max(tbl[best as usize][b as usize]));

            for (i, f) in EVALUATORS.iter().enumerate() {
                let (en, es) = (f.eval(ha), f.eval(hb));
                let (s, d) = (en + es, (en - es).abs());
                let v = na::Vector4::new(1.0, s, d, y_nt);
                moment[i][0] += v * v.transpose();
                if best_len >= MIN_FIT {
                    let v = na::Vector4::new(1.0, s, d, y_suit);
                    moment[i][1] += v * v.transpose();
                }
            }
        }
    }

    for (ctx, label) in [(0, "NT (best declarer)"), (1, "best suit fit (8+ cards)")] {
        println!("\n=== context: {label} ===");
        println!(
            "{:<7} {:>9} {:>8} {:>8} {:>7} {:>6}   {:>8} {:>7}",
            "eval", "n", "slope", "intcpt", "sd0", "R^2", "conc(d)", "sd2"
        );
        for (i, name) in NAMES.iter().enumerate() {
            let f = fit(&moment[i][ctx]);
            println!(
                "{:<7} {:>9.0} {:>8.3} {:>8.2} {:>7.3} {:>6.3}   {:>8.3} {:>7.3}",
                name, f.n, f.slope, f.intercept, f.sd0, f.r2, f.conc, f.sd2
            );
        }
    }
}
