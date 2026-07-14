//! Calibrate hand evaluators against a precomputed double-dummy database.
//!
//! Reads a DD database in either format (GIB text like `sol100000.txt`, or
//! binary `.pdd`; see [`pons::pdd::load`]) — verified against the solver — so
//! it costs **no** DD solving.
//!
//! Hands taking fewer than 6 tricks are dropped — they are never bid. For the
//! survivors, with `s = eᴺ + eˢ` the combined evaluation and `d = |eᴺ − eˢ|` the
//! imbalance between the two hands, each evaluator/context gets:
//!
//!   * the forward mapping `tricks = a + b·s` (slope, intercept, residual σ, R²)
//!     plus a concentration term `c·d`: `c ≈ 0` means the bare sum is sufficient,
//!     i.e. the evaluator is additive (a lopsided split plays like a balanced one
//!     at equal total);
//!   * per trick zone — partscore (6–8), game (9–11), slam (12–13) — the mean ± sd
//!     of `s`, i.e. the combined strength each zone actually requires.
//!
//! Note the forward slope `b = E[tricks | s]` is the trick yield of a point; do
//! not invert the zone Σ-means to get it (that inverse is steeper by ≈ 1/R²).
//!
//! Usage: `cargo run --release --example eval-calibrate [path/to/sol100000.txt]`

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

const PAIRS: [(Seat, Seat); 2] = [(Seat::North, Seat::South), (Seat::East, Seat::West)];
const MIN_FIT: usize = 8; // suit context conditions on an 8+-card trump fit

/// Regression summary from the moment matrix `m = Σ vᵥᵀ`, `v = [1, s, d, y]`
/// where `s = eᴺ + eˢ`, `d = |eᴺ − eˢ|`.
struct Fit {
    n: f64,
    slope: f64,     // M0: tricks per unit of s
    intercept: f64, // M0
    sd0: f64,       // M0 residual std (tricks)
    r2: f64,        // M0
    conc: f64,      // M2 coefficient on d (concentration effect)
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

    Fit {
        n,
        slope,
        intercept,
        sd0,
        r2: 1.0 - sse0 / sst,
        conc: beta[2],
    }
}

/// Trick zone (only hands worth bidding: 6+ tricks). `< 6` is dropped entirely.
fn zone(tricks: u8) -> Option<usize> {
    match tricks {
        6..=8 => Some(0),   // partscore
        9..=11 => Some(1),  // game
        12..=13 => Some(2), // slam
        _ => None,
    }
}
const ZONES: [&str; 3] = ["part(6-8)", "game(9-11)", "slam(12-13)"];

/// Sufficient statistics for `tricks` vs combined evaluation `Σ` within a zone.
#[derive(Clone, Copy, Default)]
struct Stat {
    n: f64,
    sum: f64,    // Σ s
    sum2: f64,   // Σ s²
    tricks: f64, // Σ y
    sy: f64,     // Σ s·y
    y2: f64,     // Σ y²
}
impl Stat {
    fn push(&mut self, sigma: f64, tricks: f64) {
        self.n += 1.0;
        self.sum += sigma;
        self.sum2 += sigma * sigma;
        self.tricks += tricks;
        self.sy += sigma * tricks;
        self.y2 += tricks * tricks;
    }
    fn mean(&self) -> f64 {
        self.sum / self.n
    }
    fn sd(&self) -> f64 {
        (self.sum2 / self.n - self.mean().powi(2)).max(0.0).sqrt()
    }
    /// `tricks ≈ slope·Σ + intercept`, with R², from this zone's hands.
    fn line(&self) -> (f64, f64, f64) {
        let det = self.n * self.sum2 - self.sum * self.sum;
        let slope = (self.n * self.sy - self.sum * self.tricks) / det;
        let intercept = (self.tricks - slope * self.sum) / self.n;
        let sse = self.y2 - intercept * self.tricks - slope * self.sy;
        let sst = self.y2 - self.tricks * self.tricks / self.n;
        (slope, intercept, 1.0 - sse / sst)
    }
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_PATH.into());
    let deals = pons::pdd::load(&path).expect("read sol file");

    // moment[eval][context] over biddable hands (6+ tricks); context 0 = NT, 1 = fit
    let mut moment = [[na::Matrix4::<f64>::zeros(); 2]; 7];
    // stat[eval][context][zone]
    let mut stat = [[[Stat::default(); 3]; 2]; 7];

    for (deal, table) in deals {
        for (a, b) in PAIRS {
            let (ha, hb) = (deal[a], deal[b]);
            // NT target: best of the two declarers
            let nt = table[Strain::Notrump];
            let y_nt = nt.get(a).get().max(nt.get(b).get());
            // longest combined suit = the trump fit
            let (mut best, mut best_len) = (Suit::Clubs, 0);
            for suit in Suit::ASC {
                let len = ha[suit].len() + hb[suit].len();
                if len > best_len {
                    (best, best_len) = (suit, len);
                }
            }
            let suit = table[Strain::from(best)];
            let y_suit = suit.get(a).get().max(suit.get(b).get());

            // (context, tricks, included?)
            let contexts = [(0, y_nt, true), (1, y_suit, best_len >= MIN_FIT)];
            for (i, f) in EVALUATORS.iter().enumerate() {
                let (en, es) = (f.eval(ha), f.eval(hb));
                let (s, d) = (en + es, (en - es).abs());
                for (ctx, y, ok) in contexts {
                    let Some(z) = zone(y) else { continue };
                    if !ok {
                        continue;
                    }
                    let yf = f64::from(y);
                    let v = na::Vector4::new(1.0, s, d, yf);
                    moment[i][ctx] += v * v.transpose();
                    stat[i][ctx][z].push(s, yf);
                }
            }
        }
    }

    for (ctx, label) in [(0, "NT (best declarer)"), (1, "best suit fit (8+ cards)")] {
        println!("\n=== context: {label} — biddable hands (6+ tricks) ===");
        println!(
            "{:<7} {:>9} {:>8} {:>8} {:>7} {:>6}   {:>8}",
            "eval", "n", "slope", "intcpt", "sd0", "R^2", "conc(d)"
        );
        for (i, name) in NAMES.iter().enumerate() {
            let f = fit(&moment[i][ctx]);
            println!(
                "{:<7} {:>9.0} {:>8.3} {:>8.2} {:>7.3} {:>6.3}   {:>8.3}",
                name, f.n, f.slope, f.intercept, f.sd0, f.r2, f.conc
            );
        }

        let z = stat[0][ctx];
        println!(
            "  zone n / mean tricks:  {} {:.0}/{:.1}   {} {:.0}/{:.1}   {} {:.0}/{:.1}",
            ZONES[0],
            z[0].n,
            z[0].tricks / z[0].n,
            ZONES[1],
            z[1].n,
            z[1].tricks / z[1].n,
            ZONES[2],
            z[2].n,
            z[2].tricks / z[2].n,
        );
        println!("  combined Σ (mean±sd) per zone — the strength a zone requires:");
        println!(
            "{:<7} {:>14} {:>14} {:>14}",
            "eval", ZONES[0], ZONES[1], ZONES[2]
        );
        for (i, name) in NAMES.iter().enumerate() {
            let s = stat[i][ctx];
            println!(
                "{:<7} {:>9.1}±{:<4.1} {:>9.1}±{:<4.1} {:>9.1}±{:<4.1}",
                name,
                s[0].mean(),
                s[0].sd(),
                s[1].mean(),
                s[1].sd(),
                s[2].mean(),
                s[2].sd(),
            );
        }

        println!("  linear form within each zone — tricks = b·Σ + a  (R²):");
        println!(
            "{:<7} {:>18} {:>18} {:>18}",
            "eval", ZONES[0], ZONES[1], ZONES[2]
        );
        for (i, name) in NAMES.iter().enumerate() {
            let cell = |z: usize| {
                let (b, a, r2) = stat[i][ctx][z].line();
                format!("{b:+.3}Σ{a:+.1} ({r2:.2})")
            };
            println!(
                "{:<7} {:>18} {:>18} {:>18}",
                name,
                cell(0),
                cell(1),
                cell(2)
            );
        }
    }
}
