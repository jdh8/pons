//! Price the learned trick evaluator against the truth it amortizes.
//!
//! The net claims to replace the expensive half of a bilans-style floor: sample
//! auction-consistent layouts, solve each double-dummy, read off the trick
//! distribution. This harness runs *both* at the same held-out decision nodes
//! and reports how far apart they land.
//!
//! For each node it computes
//!
//! - **predicted** — [`trick_estimates`], one forward pass (µs);
//! - **empirical** — `--layouts` layouts from
//!   [`sample_layouts_replay`][pons::bidding::sampler::sample_layouts_replay]
//!   (the distribution the corpus was drawn from; `--bare` uses the plain range
//!   sampler instead), one `solve_deals` batch, then the sample moments.
//!
//! The headline is the **decision-band error**: the mean |Δ| in make
//! probability restricted to contracts the *net* puts inside [`BAND`] — the
//! span every real IMP bidding threshold lives in. Outside it the call does not
//! change, so precision there is not worth paying for. Selection is on the
//! **predicted** probability
//! on purpose: conditioning on the noisy empirical one would admit contracts
//! that landed in the band by sampling error and inflate the reported gap.
//!
//! Because the net is Gaussian and real trick counts are not, the harness also
//! reports the sampled mass below μ (50% iff symmetric) and the signed gap
//! between predicted and sampled spread.
//!
//! ```text
//! scripts/idle-run.sh cargo run --release --all-features --example eval-evaluator -- \
//!     --deals /nfs2/jdh8/pons/shard-....pdd --boards 200 --layouts 96 --seed $(date +%s)
//! ```
//!
//! The solver is a process-global lock and is used from the main thread only:
//! one batched solve per node, nodes walked sequentially. This is the only step
//! in the evaluator track that solves anything.

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Seat, Strain};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::bidding::context::relative;
use pons::bidding::evaluator::trick_estimates;
use pons::bidding::sampler::{sample_layouts, sample_layouts_replay};
use pons::bidding::{Family, Phase, Relative, Stance, System};
use pons::{american, dutch};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

/// Strains in the order the DD label lists them (GIB tail order).
const STRAINS: [Strain; 5] = [
    Strain::Notrump,
    Strain::Spades,
    Strain::Hearts,
    Strain::Diamonds,
    Strain::Clubs,
];

/// Declarers, relative to the actor.
const DECLARERS: [Relative; 4] = [
    Relative::Me,
    Relative::Lho,
    Relative::Partner,
    Relative::Rho,
];

/// The band where a make probability actually changes a call at IMPs.
///
/// Every threshold is a break-even of gained IMPs against lost ones, assuming
/// the alternative contract is cold. Bidding game over a partscore needs
/// 5/11 = 45.5% non-vulnerable and 6/16 = 37.5% vulnerable; a small slam over
/// game needs 11/22 and 13/26 — exactly 50% at both vulnerabilities, since the
/// slam bonus and the game bonus scale together; a grand over a small slam
/// needs 14/24 = 58.3% at the most demanding (majors, non-vulnerable) down to
/// 16/29 = 55.2% at the least (minors, vulnerable).
///
/// So the whole span is [0.375, 0.583], and this is that with a little margin.
/// Note the folk rule that a grand wants 2:1 odds (67%) is a safety margin for
/// not knowing the small slam is cold, not the break-even. At matchpoints every
/// one of these collapses to 50%: against a field in the lower contract the
/// higher one is a top if it makes and a bottom if it does not.
const BAND: std::ops::RangeInclusive<f64> = 0.35..=0.60;

#[derive(Parser)]
#[command(about = "Predicted trick mean/spread vs sampled double-dummy truth")]
struct Args {
    /// Held-out pre-solved deal database (a fleet shard, disjoint from training)
    #[arg(long)]
    deals: String,
    /// Skip this many deals before reading
    #[arg(long, default_value_t = 0)]
    skip: u64,
    /// Boards to walk
    #[arg(long, default_value_t = 200)]
    boards: usize,
    /// Layouts sampled and solved per decision node
    #[arg(long, default_value_t = 96)]
    layouts: usize,
    /// RNG seed
    #[arg(long, default_value_t = 0)]
    seed: u64,
    /// Book to walk with
    #[arg(long, default_value = "american")]
    system: String,
    /// Sample by range envelope alone, skipping the authoring-rule replay
    #[arg(long)]
    bare: bool,
}

const VULS: [AbsoluteVulnerability; 4] = [
    AbsoluteVulnerability::NONE,
    AbsoluteVulnerability::NS,
    AbsoluteVulnerability::EW,
    AbsoluteVulnerability::ALL,
];

/// Running mean of an error series.
#[derive(Default, Clone, Copy)]
struct Mean {
    sum: f64,
    n: u64,
}

impl Mean {
    fn push(&mut self, x: f64) {
        self.sum += x;
        self.n += 1;
    }

    fn get(self) -> f64 {
        if self.n == 0 {
            f64::NAN
        } else {
            self.sum / self.n as f64
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let stance: Stance = match args.system.as_str() {
        "american" => american().against(Family::NATURAL),
        "dutch" => dutch().against(Family::NATURAL),
        other => anyhow::bail!("--system must be american|dutch, got {other:?}"),
    };
    let deals = pons::pdd::load_slice(&args.deals, args.skip, args.boards)?;
    eprintln!(
        "eval-evaluator: {} boards, {} layouts/node, {} sampler",
        deals.len(),
        args.layouts,
        if args.bare { "range-only" } else { "replay" }
    );

    let mut rng = StdRng::seed_from_u64(args.seed);
    let (mut mean_err, mut sd_err) = (Mean::default(), Mean::default());
    let (mut band_err, mut all_err) = (Mean::default(), Mean::default());
    let (mut band_err_contested, mut noise_floor) = (Mean::default(), Mean::default());
    let (mut pred_sd, mut emp_sd, mut emp_below) =
        (Mean::default(), Mean::default(), Mean::default());
    let (mut nodes, mut starved) = (0u64, 0u64);

    for (board, (deal, _)) in deals.iter().enumerate() {
        // Every DD solve here is a fresh sampled layout, so a long run has no
        // other progress signal — without this the only output is the header
        // and, hours later, the table.
        if board > 0 && board % 50 == 0 {
            eprintln!("  ... {board}/{} boards, {nodes} nodes priced", deals.len());
        }
        let dealer = rng.random_range(0..4usize);
        let vul = VULS[rng.random_range(0..4usize)];
        let mut auction = Auction::new();
        while !auction.has_ended() {
            let seat = Seat::ALL[(dealer + auction.len()) % 4];
            let hand = deal[seat];
            let rel = relative(vul, seat);
            let Some(mut logits) = stance.classify(hand, rel, &auction) else {
                auction.push(Call::Pass);
                continue;
            };
            for (call, slot) in logits.iter_mut() {
                if auction.can_push(call).is_err() {
                    *slot = f32::NEG_INFINITY;
                }
            }

            let inferences = stance.infer(rel, &auction);
            let sampled = if args.bare {
                sample_layouts(hand, seat, &inferences, &mut rng, args.layouts)
            } else {
                sample_layouts_replay(
                    hand,
                    seat,
                    &stance,
                    rel,
                    &auction,
                    &inferences,
                    &mut rng,
                    args.layouts,
                )
            };
            // A short draw is a weak signal, not an error — but moments off a
            // handful of layouts are noise, so require a usable sample.
            if sampled.len() * 4 < args.layouts {
                starved += 1;
            } else {
                // One batched solve per node, on the main thread: the solver is
                // a process-global lock and must never meet rayon.
                let tables = Solver::lock().solve_deals(&sampled, NonEmptyStrainFlags::ALL);
                let predicted = trick_estimates(hand, &inferences);
                let contested = Phase::of(&auction) != Phase::Constructive;

                for strain in STRAINS {
                    for who in DECLARERS {
                        let truth = empirical(&tables, seat, strain, who);
                        let (t_mean, t_sd) = moments(&truth);
                        let p = predicted.get(strain, who);
                        mean_err.push((f64::from(p.mean) - t_mean).abs());
                        sd_err.push((f64::from(p.sd) - t_sd).abs());
                        // Signed, not absolute: the net conditions on the
                        // *ranges*, the sampler on the actual calls. Pooling
                        // auctions that read out alike should cost width, and
                        // this is where that price shows up.
                        pred_sd.push(f64::from(p.sd));
                        emp_sd.push(t_sd);
                        // Skew diagnostic: for a symmetric law half the mass
                        // sits below the mean. Double-dummy trick counts on a
                        // good fit are left-skewed and walled at 13, and this is
                        // how far the Gaussian's core assumption is off.
                        emp_below.push(
                            truth.iter().filter(|&&t| f64::from(t) < t_mean).count() as f64
                                / truth.len() as f64,
                        );

                        // Make probability at every level the contract could be
                        // played at: 7 tricks (1-level) through 13 (grand).
                        for tricks in 7..=13u8 {
                            let want = f64::from(tricks);
                            let empirical_p =
                                truth.iter().filter(|&&t| f64::from(t) >= want).count() as f64
                                    / truth.len() as f64;
                            let predicted_p = f64::from(p.p_at_least(tricks));
                            let delta = (predicted_p - empirical_p).abs();
                            all_err.push(delta);
                            // Select on the *prediction*, not the sample:
                            // conditioning on a noisy `empirical_p` landing in
                            // the band would pull in contracts that got there by
                            // sampling error, inflating the reported gap.
                            if BAND.contains(&predicted_p) {
                                band_err.push(delta);
                                // The truth is itself a sample: `empirical_p` is
                                // a binomial mean over `truth.len()` layouts, so
                                // even a perfect net would miss it by about
                                // √(2/π)·SE. Report that floor beside the gap.
                                noise_floor.push(
                                    (2.0 / std::f64::consts::PI).sqrt()
                                        * (empirical_p * (1.0 - empirical_p) / truth.len() as f64)
                                            .sqrt(),
                                );
                                if contested {
                                    band_err_contested.push(delta);
                                }
                            }
                        }
                    }
                }
                nodes += 1;
            }
            auction.push(argmax_legal(&logits));
        }
    }

    println!("nodes priced          {nodes}  (starved samples skipped: {starved})");
    println!(
        "moment MAE (tricks)   mean {:.3}  sd {:.3}",
        mean_err.get(),
        sd_err.get()
    );
    println!(
        "spread (tricks)       predicted sd {:.3}  sampled sd {:.3}  → net is {:+.3} wide",
        pred_sd.get(),
        emp_sd.get(),
        pred_sd.get() - emp_sd.get(),
    );
    println!(
        "sampled mass below µ  {:.1}%   (50% iff symmetric — the Gaussian's blind spot)",
        100.0 * emp_below.get(),
    );
    println!("P(make) MAE           all levels {:.4}", all_err.get());
    println!(
        "P(make) MAE in band   {:.4}   ({} contracts predicted 35–60%; contested {:.4})",
        band_err.get(),
        band_err.n,
        band_err_contested.get(),
    );
    println!(
        "  sampling-noise floor {:.4}  → net's own error ≈ {:.4}",
        noise_floor.get(),
        // Deconvolve in *quadrature*, not linearly. The measured gap is the net's
        // error against a noisy estimate of truth; the two are independent, so
        // their squares add. Subtracting linearly understates the net by ~45% at
        // these magnitudes. Both terms are MAEs of roughly Gaussian errors, and
        // MAE = √(2/π)·σ for a Gaussian, so the √(2/π) factors cancel and the
        // MAEs compose in quadrature exactly as the σ's do.
        (band_err.get().powi(2) - noise_floor.get().powi(2))
            .max(0.0)
            .sqrt(),
    );
    Ok(())
}

/// The sampled trick counts for one (strain, declarer), declarer relative.
fn empirical(tables: &[TrickCountTable], seat: Seat, strain: Strain, who: Relative) -> Vec<u8> {
    let declarer = match who {
        Relative::Me => seat,
        Relative::Lho => seat.lho(),
        Relative::Partner => seat.partner(),
        Relative::Rho => seat.rho(),
    };
    tables
        .iter()
        .map(|table| table[strain].get(declarer).get())
        .collect()
}

/// Sample mean and (Bessel-corrected) standard deviation — the two statistics
/// the net's heads are fit to estimate.
fn moments(sample: &[u8]) -> (f64, f64) {
    let n = sample.len() as f64;
    let mean = sample.iter().map(|&t| f64::from(t)).sum::<f64>() / n;
    let var = sample
        .iter()
        .map(|&t| (f64::from(t) - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0).max(1.0);
    (mean, var.sqrt())
}

/// The highest-logit finite (hence legal) call, defaulting to a pass.
fn argmax_legal(logits: &pons::bidding::array::Logits) -> Call {
    logits
        .iter()
        .filter(|(_, l)| l.is_finite())
        .max_by(|a, b| a.1.partial_cmp(b.1).expect("logits are never NaN"))
        .map_or(Call::Pass, |(call, _)| call)
}
