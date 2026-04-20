use criterion::{Criterion, criterion_group, criterion_main};
use dds_bridge::Seat;
use dds_bridge::solver::{Solver, Vulnerability};
use pons::deck::full_deal;
use pons::stats::{HistogramTable, average_ns_par};
use rand::SeedableRng;
use rand::rngs::SmallRng;
use std::hint::black_box;

/// Solve a small batch of random deals and collect into a histogram.
fn build_histogram(seed: u64, deals: usize) -> HistogramTable {
    let mut rng = SmallRng::seed_from_u64(seed);
    let solver = Solver::lock();
    (0..deals)
        .map(|_| solver.solve_deal(full_deal(&mut rng)))
        .collect()
}

fn bench_par_none_north(c: &mut Criterion) {
    let hist = build_histogram(0, 64);
    c.bench_function("average_ns_par_none_north", |b| {
        b.iter(|| {
            black_box(average_ns_par(
                black_box(hist),
                Vulnerability::NONE,
                Seat::North,
            ))
        });
    });
}

fn bench_par_all_east(c: &mut Criterion) {
    let hist = build_histogram(1, 64);
    c.bench_function("average_ns_par_all_east", |b| {
        b.iter(|| {
            black_box(average_ns_par(
                black_box(hist),
                Vulnerability::ALL,
                Seat::East,
            ))
        });
    });
}

criterion_group!(benches, bench_par_none_north, bench_par_all_east);
criterion_main!(benches);
