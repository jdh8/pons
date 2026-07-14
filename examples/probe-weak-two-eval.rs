//! Calibrate CCCC / NLTC thresholds for the weak-two evaluator gauges
//! (`set_weak_two_eval`) before spending `.pdd` ledger rows on the A/B.
//!
//! No double dummy — random hands only, seconds to run.  Over every hand
//! holding a six-card suit in ♦/♥/♠ (the weak-two shape gate), print:
//!
//! - the CCCC and NLTC distributions, overall and conditional on the shipped
//!   strength band `points(5..=10)`;
//! - candidate **swap bands** cut at quantiles of the shipped population,
//!   with their fire rate (vs shipped), the shipped hands they keep, and the
//!   off-band hands they add — pick the matched-fire-rate band from here;
//! - candidate **discipline cuts** (CCCC floor / NLTC ceiling) with the
//!   fraction of shipped weak twos they prune.
//!
//! ```text
//! cargo run --release --example probe-weak-two-eval -- 1000000 0
//! ```
//! Args (positional, optional): deal `count` (default 1,000,000), `seed`
//! (default 0).

use contract_bridge::eval::{self, HandEvaluator};
use contract_bridge::{Hand, Seat, Suit};
use pons::bidding::constraint::point_count;
use rand::SeedableRng;
use rand::rngs::StdRng;

/// The three weak-two strains: a six-card suit in exactly one of these
fn weak_two_shape(hand: Hand) -> bool {
    [Suit::Diamonds, Suit::Hearts, Suit::Spades]
        .into_iter()
        .any(|suit| hand[suit].len() == 6)
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    let index = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[index]
}

fn print_quantiles(label: &str, sorted: &[f64]) {
    print!("{label:32}");
    for q in [0.01, 0.05, 0.10, 0.25, 0.50, 0.75, 0.90, 0.95, 0.99] {
        print!(" {:5.2}", quantile(sorted, q));
    }
    println!(" (n={})", sorted.len());
}

/// Fire rate, shipped kept, and non-shipped added for a candidate band.
/// `inclusive` matches the rule form: `cccc(lo..hi)` vs `nltc(lo..=hi)` —
/// NLTC's mass sits on half-integers, so the closed end matters.
fn band_report(
    name: &str,
    values: &[(f64, bool)],
    lo: f64,
    hi: f64,
    inclusive: bool,
    shipped: usize,
) {
    let hit = |v: f64| v >= lo && if inclusive { v <= hi } else { v < hi };
    let fired = values.iter().filter(|&&(v, _)| hit(v)).count();
    let kept = values.iter().filter(|&&(v, s)| s && hit(v)).count();
    let added = fired - kept;
    #[allow(clippy::cast_precision_loss)]
    let pct = |a: usize, b: usize| 100.0 * a as f64 / b as f64;
    println!(
        "{name:32} fires {fired:7} ({:6.2}% of shipped rate)  keeps {:5.1}% of shipped, adds {added:6}",
        pct(fired, shipped),
        pct(kept, shipped),
    );
}

fn main() {
    let mut argv = std::env::args().skip(1);
    let count: usize = argv
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000_000);
    let seed: u64 = argv.next().and_then(|s| s.parse().ok()).unwrap_or(0);

    // The twin hands from `weak_two_eval_gauges_honor_location` — the honor-
    // location separation the gauges are built on.
    for text in ["KQJ862.943.75.82", "986432.94.KQ.J82"] {
        let hand: Hand = text.parse().expect("valid probe hand");
        println!(
            "{text:20} points {:2}  CCCC {:5.2}  NLTC {:4.1}",
            point_count(hand),
            eval::cccc(hand),
            eval::NLTC.eval(hand),
        );
    }

    // (value, in-shipped-band) per weak-two-shaped hand, one entry per hand.
    let mut cccc: Vec<(f64, bool)> = Vec::new();
    let mut nltc: Vec<(f64, bool)> = Vec::new();
    let mut rng = StdRng::seed_from_u64(seed);
    for _ in 0..count {
        let deal = contract_bridge::deck::full_deal(&mut rng);
        for seat in [Seat::North, Seat::East, Seat::South, Seat::West] {
            let hand = deal[seat];
            if !weak_two_shape(hand) {
                continue;
            }
            let shipped = (5..=10).contains(&point_count(hand));
            cccc.push((eval::cccc(hand), shipped));
            nltc.push((eval::NLTC.eval(hand), shipped));
        }
    }
    let shipped = cccc.iter().filter(|&&(_, s)| s).count();
    #[allow(clippy::cast_precision_loss)]
    let rate = shipped as f64 / (4 * count) as f64;
    println!(
        "\n{} hands with a 6-card ♦/♥/♠ suit; shipped band fires {shipped} ({:.3}% of all hands)",
        cccc.len(),
        100.0 * rate,
    );

    println!(
        "\nquantiles                            1%    5%   10%   25%   50%   75%   90%   95%   99%"
    );
    for (name, values) in [("CCCC", &cccc), ("NLTC", &nltc)] {
        let mut all: Vec<f64> = values.iter().map(|&(v, _)| v).collect();
        let mut in_band: Vec<f64> = values
            .iter()
            .filter(|&&(_, s)| s)
            .map(|&(v, _)| v)
            .collect();
        all.sort_unstable_by(f64::total_cmp);
        in_band.sort_unstable_by(f64::total_cmp);
        print_quantiles(&format!("{name}, 6-card shape"), &all);
        print_quantiles(&format!("{name}, shipped points(5..=10)"), &in_band);

        // Swap-band candidates: quantile cuts of the shipped distribution.
        println!();
        let inclusive = name == "NLTC";
        for (qlo, qhi) in [(0.025, 0.975), (0.05, 0.95), (0.075, 0.925), (0.10, 0.90)] {
            let (lo, hi) = (quantile(&in_band, qlo), quantile(&in_band, qhi));
            let sep = if inclusive { "..=" } else { ".." };
            band_report(
                &format!("{name} band {lo:.2}{sep}{hi:.2} (q{qlo}-q{qhi})"),
                values,
                lo,
                hi,
                inclusive,
                shipped,
            );
        }

        // NLTC sits on a half-integer grid, so quantile cuts are coarse —
        // enumerate the plausible bands outright.
        if inclusive {
            println!();
            for lo in [6.5, 7.0, 7.5, 8.0] {
                for hi in [9.0, 9.5, 10.0] {
                    band_report(
                        &format!("{name} band {lo:.1}..={hi:.1}"),
                        values,
                        lo,
                        hi,
                        inclusive,
                        shipped,
                    );
                }
            }
        }

        // Discipline candidates: prune the junk tail of the shipped band.
        // Junk = low CCCC, but *high* NLTC (losers) — opposite tails.
        println!();
        for q in [0.10, 0.20, 0.25] {
            let (cut, kept) = if name == "CCCC" {
                (
                    quantile(&in_band, q),
                    format!("floor {:.2}", quantile(&in_band, q)),
                )
            } else {
                (
                    quantile(&in_band, 1.0 - q),
                    format!("ceil {:.2}", quantile(&in_band, 1.0 - q)),
                )
            };
            let pruned = in_band
                .iter()
                .filter(|&&v| if name == "CCCC" { v < cut } else { v > cut })
                .count();
            #[allow(clippy::cast_precision_loss)]
            let pct = 100.0 * pruned as f64 / shipped as f64;
            println!(
                "{:32} prunes {pct:5.1}% of shipped weak twos",
                format!("{name} {kept}")
            );
        }
        println!();
    }
}
