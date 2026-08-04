//! Paired fixed-corpus latency and Rust-allocation reference measurement.
//!
//! Pons and wrapped BBA see the identical 512 positions. This is not a bidding
//! A/B: calls are allowed to differ and no bridge result is scored.

use clap::{Parser, ValueEnum};
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use core::hint::black_box;
use pons::bidding::array::Logits;
use pons::bidding::benchmark::{
    classify_with_provenance_uncached, is_deterministic_instinct_floor,
};
use pons::bidding::{Stance, System, Table};
use pons::{american, american_instinct};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::time::{Duration, Instant};

#[path = "../../benches/support/mod.rs"]
mod support;
use support::{AllocationSnapshot, CountingAllocator, Position};

#[path = "../common/oracle.rs"]
#[allow(dead_code)]
mod oracle;
use oracle::{BbaOracle, DEFAULT_LIB, SYSTEM_2_OVER_1};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator::new();

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Engine {
    Both,
    Pons,
    Bba,
}

#[derive(Parser, Debug)]
#[command(about = "Fixed-corpus Pons/BBA latency reference (not a bidding A/B)")]
struct Args {
    #[arg(long, value_enum, default_value = "both")]
    engine: Engine,
    #[arg(long, default_value_t = 2)]
    warmups: usize,
    #[arg(long, default_value_t = 10)]
    repetitions: usize,
    #[arg(long, default_value = DEFAULT_LIB)]
    bba_lib: String,
    /// Parse and validate the corpus without running either engine.
    #[arg(long)]
    verify_only: bool,
    /// Omit the separate allocator pass (used when perf counts the process).
    #[arg(long)]
    skip_allocations: bool,
    /// Omit cached-vs-legacy stage-2 acceptance workloads (used by perf engine runs).
    #[arg(long, default_value_t = true)]
    skip_cache_acceptance: bool,
    /// Return success despite CV >2%; diagnostic only, never an acceptance run.
    #[arg(long)]
    allow_unstable: bool,
}

const DEAL_SEED: u64 = 1;
const DEALS: usize = 64;
// A single Pons traversal is only a few milliseconds on the acceptance CPUs,
// which makes scheduler interrupts dominate its ten-sample CV.  Treat one
// measured repetition as a fixed batch of corpus traversals; the reported
// unit and allocation pass remain per decision.
const ENGINE_SAMPLE_LOOPS: usize = 8;
const HOT_CACHED_OPERATIONS: usize = 2_048;
const HOT_LEGACY_OPERATIONS: usize = 64;
const WHOLE_CACHED_LOOPS: usize = 16;
const WHOLE_LEGACY_LOOPS: usize = 1;

#[derive(Clone, Copy)]
struct Legacy<'a>(&'a Stance);

impl System for Legacy<'_> {
    fn classify(
        &self,
        hand: contract_bridge::Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
    ) -> Option<Logits> {
        classify_with_provenance_uncached(self.0, hand, vul, auction).map(|(logits, _)| logits)
    }

    fn authored_at(&self, vul: RelativeVulnerability, auction: &[Call]) -> bool {
        self.0.authored_at(vul, auction)
    }
}

#[allow(dead_code)] // required by included `common::oracle`
fn seat_to_act(dealer: contract_bridge::Seat, len: usize) -> contract_bridge::Seat {
    contract_bridge::Seat::ALL[(dealer as usize + len) % 4]
}

fn classify_all(system: &dyn System, positions: &[Position], loops: usize) {
    for _ in 0..loops {
        for position in positions {
            black_box(
                system
                    .classify(
                        black_box(position.hand),
                        position.vul,
                        black_box(&position.auction),
                    )
                    .unwrap_or_else(|| {
                        panic!(
                            "engine failed to classify frozen corpus row {}",
                            position.id
                        )
                    }),
            );
        }
    }
}

fn timed(system: &dyn System, positions: &[Position], loops: usize) -> Duration {
    let start = Instant::now();
    classify_all(system, positions, loops);
    start.elapsed()
}

fn allocated(system: &dyn System, positions: &[Position]) -> AllocationSnapshot {
    ALLOCATOR.reset();
    classify_all(system, positions, 1);
    ALLOCATOR.snapshot()
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn sample_sd(values: &[f64], center: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    (values
        .iter()
        .map(|value| (value - center).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64)
        .sqrt()
}

fn percentile_median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

fn summarize_samples(name: &str, samples: &[f64], unit: &str) -> f64 {
    let average = mean(samples);
    let cv = if average == 0.0 {
        0.0
    } else {
        sample_sd(samples, average) / average
    };
    println!(
        "{name}: median={:.3} {unit} mean={average:.3} {unit} CV={:.2}%",
        percentile_median(samples.to_vec()),
        100.0 * cv,
    );
    if cv > 0.02 {
        eprintln!(
            "warning: {name} CV {:.2}% exceeds the 2% stability gate; rerun on a quieter core",
            100.0 * cv
        );
    }
    cv
}

fn summarize_engine(
    name: &str,
    samples: &[f64],
    allocations: Option<AllocationSnapshot>,
    positions: usize,
) -> f64 {
    let cv = summarize_samples(name, samples, "us/decision");
    if let Some(allocations) = allocations {
        println!(
            "{name}: rust_allocations={:.3}/decision rust_requested_bytes={:.1}/decision (native allocations not observed)",
            allocations.allocations as f64 / positions as f64,
            allocations.bytes as f64 / positions as f64,
        );
    } else {
        println!("{name}: Rust allocation pass skipped");
    }
    cv
}

fn t95(df: usize) -> f64 {
    // Two-sided 95% Student-t critical values for the small repeat counts this
    // harness is designed for; normal limit thereafter.
    const TABLE: [f64; 30] = [
        12.706, 4.303, 3.182, 2.776, 2.571, 2.447, 2.365, 2.306, 2.262, 2.228, 2.201, 2.179, 2.160,
        2.145, 2.131, 2.120, 2.110, 2.101, 2.093, 2.086, 2.080, 2.074, 2.069, 2.064, 2.060, 2.056,
        2.052, 2.048, 2.045, 2.042,
    ];
    TABLE.get(df.saturating_sub(1)).copied().unwrap_or_else(|| {
        // Cornish-Fisher expansion about the normal critical value.  Unlike a
        // direct jump to 1.96 after df=30, this stays conservative for the
        // user-selectable repeat counts immediately above the table.
        let df = df as f64;
        let z: f64 = 1.959_963_984_540_054;
        z + (z.powi(3) + z) / (4.0 * df)
            + (5.0 * z.powi(5) + 16.0 * z.powi(3) + 3.0 * z) / (96.0 * df.powi(2))
    })
}

#[derive(Clone, Copy, Debug)]
struct RatioInterval {
    estimate: f64,
    median: f64,
    lower: f64,
    upper: f64,
}

fn paired_ratio(name: &str, numerator: &[f64], denominator: &[f64]) -> RatioInterval {
    assert_eq!(numerator.len(), denominator.len());
    assert!(numerator.len() >= 2);
    let logs: Vec<_> = numerator
        .iter()
        .zip(denominator)
        .map(|(numerator, denominator)| (numerator / denominator).ln())
        .collect();
    let center = mean(&logs);
    let se = sample_sd(&logs, center) / (logs.len() as f64).sqrt();
    let margin = t95(logs.len().saturating_sub(1)) * se;
    let interval = RatioInterval {
        estimate: center.exp(),
        median: percentile_median(
            numerator
                .iter()
                .zip(denominator)
                .map(|(numerator, denominator)| numerator / denominator)
                .collect(),
        ),
        lower: (center - margin).exp(),
        upper: (center + margin).exp(),
    };
    println!(
        "paired {name} ratio: estimate={:.4} median={:.4} 95% CI=[{:.4}, {:.4}] (log-ratio t interval, n={})",
        interval.estimate,
        interval.median,
        interval.lower,
        interval.upper,
        logs.len(),
    );
    interval
}

fn micros_per_decision(duration: Duration, count: usize) -> f64 {
    duration.as_secs_f64() * 1_000_000.0 / count as f64
}

fn fixed_deals() -> Vec<FullDeal> {
    (0..DEALS)
        .map(|index| full_deal(&mut StdRng::seed_from_u64(DEAL_SEED + index as u64)))
        .collect()
}

fn timed_hot_cached(stance: &Stance, positions: &[&Position], loops: usize) -> f64 {
    let start = Instant::now();
    for _ in 0..loops {
        for position in positions {
            black_box(stance.classify_with_provenance(
                black_box(position.hand),
                position.vul,
                black_box(&position.auction),
            ));
        }
    }
    micros_per_decision(start.elapsed(), loops * positions.len())
}

fn timed_hot_legacy(stance: &Stance, positions: &[&Position], loops: usize) -> f64 {
    let start = Instant::now();
    for _ in 0..loops {
        for position in positions {
            black_box(classify_with_provenance_uncached(
                stance,
                black_box(position.hand),
                position.vul,
                black_box(&position.auction),
            ));
        }
    }
    micros_per_decision(start.elapsed(), loops * positions.len())
}

fn timed_whole_cached(tables: &[Table<&Stance, &Stance>], deals: &[FullDeal], loops: usize) -> f64 {
    let start = Instant::now();
    for _ in 0..loops {
        for (table, deal) in tables.iter().zip(deals) {
            black_box(table.bid_out(black_box(deal)));
        }
    }
    micros_per_decision(start.elapsed(), loops * deals.len())
}

fn timed_whole_legacy(
    tables: &[Table<Legacy<'_>, Legacy<'_>>],
    deals: &[FullDeal],
    loops: usize,
) -> f64 {
    let start = Instant::now();
    for _ in 0..loops {
        for (table, deal) in tables.iter().zip(deals) {
            black_box(table.bid_out(black_box(deal)));
        }
    }
    micros_per_decision(start.elapsed(), loops * deals.len())
}

#[derive(Debug)]
struct CacheAcceptance {
    unstable: bool,
    hot_ratio: RatioInterval,
    whole_ratio: RatioInterval,
}

fn run_cache_acceptance(
    stance: &Stance,
    positions: &[Position],
    warmups: usize,
    repetitions: usize,
) -> anyhow::Result<CacheAcceptance> {
    let deterministic = american_instinct().against();
    let hot: Vec<_> = positions
        .iter()
        .filter(|position| {
            is_deterministic_instinct_floor(
                stance,
                &deterministic,
                position.hand,
                position.vul,
                &position.auction,
            )
        })
        .collect();
    anyhow::ensure!(
        !hot.is_empty(),
        "corpus has no deterministic instinct-delegating floor positions"
    );
    let hot_cached_loops = HOT_CACHED_OPERATIONS.div_ceil(hot.len());
    let hot_legacy_loops = HOT_LEGACY_OPERATIONS.div_ceil(hot.len());
    let deals = fixed_deals();
    let vulnerabilities = [
        AbsoluteVulnerability::NONE,
        AbsoluteVulnerability::NS,
        AbsoluteVulnerability::EW,
        AbsoluteVulnerability::ALL,
    ];
    let tables: Vec<_> = (0..deals.len())
        .map(|index| {
            Table::new(
                stance,
                stance,
                Seat::ALL[index % 4],
                vulnerabilities[index / 4 % 4],
            )
        })
        .collect();
    let legacy_tables: Vec<_> = (0..deals.len())
        .map(|index| {
            Table::new(
                Legacy(stance),
                Legacy(stance),
                Seat::ALL[index % 4],
                vulnerabilities[index / 4 % 4],
            )
        })
        .collect();

    // Keep timing honest even if the parity test is not run beside this
    // executable: both arms must perform the same decisions and finish each
    // fixed deal on the same auction before their durations are comparable.
    for position in &hot {
        let cached = stance
            .classify_with_provenance(position.hand, position.vul, &position.auction)
            .expect("the shipped stance is total");
        let legacy = classify_with_provenance_uncached(
            stance,
            position.hand,
            position.vul,
            &position.auction,
        )
        .expect("the legacy stance is total");
        anyhow::ensure!(
            cached.1 == legacy.1
                && cached.0.iter().zip(legacy.0.iter()).all(
                    |((cached_call, cached), (legacy_call, legacy))| {
                        cached_call == legacy_call && cached.to_bits() == legacy.to_bits()
                    }
                ),
            "cached/legacy hot-position parity failed at corpus row {}",
            position.id
        );
    }
    for (index, ((cached, legacy), deal)) in
        tables.iter().zip(&legacy_tables).zip(&deals).enumerate()
    {
        anyhow::ensure!(
            cached.bid_out(deal) == legacy.bid_out(deal),
            "cached/legacy whole-deal parity failed at fixed deal {index}"
        );
    }

    println!(
        "cache acceptance workload: {} deterministic instinct floor positions; {} deals; seed=1; dealer=i%4; vul=(i/4)%4",
        hot.len(),
        deals.len()
    );
    for _ in 0..warmups {
        black_box(timed_hot_cached(stance, &hot, 1));
        black_box(timed_hot_legacy(stance, &hot, 1));
        black_box(timed_whole_cached(&tables, &deals, 1));
        black_box(timed_whole_legacy(&legacy_tables, &deals, 1));
    }

    let mut hot_cached = Vec::with_capacity(repetitions);
    let mut hot_legacy = Vec::with_capacity(repetitions);
    let mut whole_cached = Vec::with_capacity(repetitions);
    let mut whole_legacy = Vec::with_capacity(repetitions);
    for repetition in 0..repetitions {
        if repetition % 2 == 0 {
            hot_cached.push(timed_hot_cached(stance, &hot, hot_cached_loops));
            hot_legacy.push(timed_hot_legacy(stance, &hot, hot_legacy_loops));
            whole_cached.push(timed_whole_cached(&tables, &deals, WHOLE_CACHED_LOOPS));
            whole_legacy.push(timed_whole_legacy(
                &legacy_tables,
                &deals,
                WHOLE_LEGACY_LOOPS,
            ));
        } else {
            hot_legacy.push(timed_hot_legacy(stance, &hot, hot_legacy_loops));
            hot_cached.push(timed_hot_cached(stance, &hot, hot_cached_loops));
            whole_legacy.push(timed_whole_legacy(
                &legacy_tables,
                &deals,
                WHOLE_LEGACY_LOOPS,
            ));
            whole_cached.push(timed_whole_cached(&tables, &deals, WHOLE_CACHED_LOOPS));
        }
    }

    let mut unstable = false;
    unstable |= summarize_samples("hot instinct cached", &hot_cached, "us/decision") > 0.02;
    unstable |= summarize_samples("hot instinct legacy", &hot_legacy, "us/decision") > 0.02;
    let hot_ratio = paired_ratio("cached/legacy hot instinct", &hot_cached, &hot_legacy);
    unstable |= summarize_samples("whole deal cached", &whole_cached, "us/deal") > 0.02;
    unstable |= summarize_samples("whole deal legacy", &whole_legacy, "us/deal") > 0.02;
    let whole_ratio = paired_ratio("cached/legacy whole deal", &whole_cached, &whole_legacy);
    Ok(CacheAcceptance {
        unstable,
        hot_ratio,
        whole_ratio,
    })
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    anyhow::ensure!(args.repetitions > 0, "--repetitions must be positive");
    if matches!(args.engine, Engine::Both)
        || (!args.skip_cache_acceptance && matches!(args.engine, Engine::Both | Engine::Pons))
    {
        anyhow::ensure!(
            args.repetitions >= 2,
            "paired 95% intervals require at least two repetitions"
        );
    }
    let positions = support::parse_corpus().map_err(anyhow::Error::msg)?;
    println!(
        "validated {} positions: 64 Pons + 64 BBA per depth 2/4/8/12",
        positions.len()
    );
    if args.verify_only {
        return Ok(());
    }
    println!(
        "engine timing batch: {ENGINE_SAMPLE_LOOPS} corpus traversals per warmup/measured repetition"
    );

    let use_pons = matches!(args.engine, Engine::Both | Engine::Pons);
    let use_bba = matches!(args.engine, Engine::Both | Engine::Bba);
    let stance: Option<Stance> = use_pons.then(|| american().against());
    // EPBot is loaded and driven only from this main thread.
    let bba = use_bba
        .then(|| BbaOracle::load(&args.bba_lib, SYSTEM_2_OVER_1, Vec::new()))
        .transpose()?;

    for _ in 0..args.warmups {
        if let Some(stance) = &stance {
            classify_all(stance, &positions, ENGINE_SAMPLE_LOOPS);
        }
        if let Some(bba) = &bba {
            classify_all(bba, &positions, ENGINE_SAMPLE_LOOPS);
        }
    }

    let pons_alloc = stance
        .as_ref()
        .filter(|_| !args.skip_allocations)
        .map(|system| allocated(system, &positions));
    let bba_alloc = bba
        .as_ref()
        .filter(|_| !args.skip_allocations)
        .map(|system| allocated(system, &positions));
    let mut pons_samples = Vec::with_capacity(args.repetitions);
    let mut bba_samples = Vec::with_capacity(args.repetitions);
    for repetition in 0..args.repetitions {
        // Alternate order to keep monotonic thermal/frequency drift out of the
        // paired log-ratio estimate.
        if repetition % 2 == 0 {
            if let Some(system) = &stance {
                pons_samples.push(micros_per_decision(
                    timed(system, &positions, ENGINE_SAMPLE_LOOPS),
                    positions.len() * ENGINE_SAMPLE_LOOPS,
                ));
            }
            if let Some(system) = &bba {
                bba_samples.push(micros_per_decision(
                    timed(system, &positions, ENGINE_SAMPLE_LOOPS),
                    positions.len() * ENGINE_SAMPLE_LOOPS,
                ));
            }
        } else {
            if let Some(system) = &bba {
                bba_samples.push(micros_per_decision(
                    timed(system, &positions, ENGINE_SAMPLE_LOOPS),
                    positions.len() * ENGINE_SAMPLE_LOOPS,
                ));
            }
            if let Some(system) = &stance {
                pons_samples.push(micros_per_decision(
                    timed(system, &positions, ENGINE_SAMPLE_LOOPS),
                    positions.len() * ENGINE_SAMPLE_LOOPS,
                ));
            }
        }
    }

    let mut unstable = false;
    if use_pons {
        unstable |= summarize_engine("Pons", &pons_samples, pons_alloc, positions.len()) > 0.02;
    }
    if use_bba {
        unstable |= summarize_engine(
            "BBA wrapper-inclusive",
            &bba_samples,
            bba_alloc,
            positions.len(),
        ) > 0.02;
    }
    let pons_bba =
        (use_pons && use_bba).then(|| paired_ratio("Pons/BBA", &pons_samples, &bba_samples));
    let cache_acceptance = if !args.skip_cache_acceptance {
        stance
            .as_ref()
            .map(|stance| run_cache_acceptance(stance, &positions, args.warmups, args.repetitions))
            .transpose()?
    } else {
        None
    };
    if let Some(cache_acceptance) = &cache_acceptance {
        unstable |= cache_acceptance.unstable;
        anyhow::ensure!(
            cache_acceptance.hot_ratio.upper <= 0.25,
            "hot instinct cache gate failed: cached/legacy upper 95% bound {:.4} exceeds 0.25 (not demonstrably 4x faster)",
            cache_acceptance.hot_ratio.upper
        );
        anyhow::ensure!(
            cache_acceptance.whole_ratio.upper <= 0.75,
            "whole-deal cache gate failed: cached/legacy upper 95% bound {:.4} exceeds 0.75 (not demonstrably 25% faster)",
            cache_acceptance.whole_ratio.upper
        );
    }
    if let Some(ratio) = pons_bba {
        anyhow::ensure!(
            ratio.upper < 1.0,
            "Pons/BBA gate failed: upper 95% ratio bound {:.4} is not below 1.0",
            ratio.upper
        );
    }
    anyhow::ensure!(
        !unstable || args.allow_unstable,
        "CV stability gate failed (>2%); rerun on a quieter core or use --allow-unstable for diagnostics only"
    );
    Ok(())
}
