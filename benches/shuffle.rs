use criterion::{Criterion, criterion_group, criterion_main};
use dds_bridge::{Builder, Seat};
use pons::deck::{Deck, fill_deals, full_deal};
use rand::SeedableRng;
use rand::rngs::SmallRng;
use std::hint::black_box;

fn bench_draw_13(c: &mut Criterion) {
    c.bench_function("Deck::draw_13_from_full", |b| {
        let mut rng = SmallRng::seed_from_u64(0);
        b.iter(|| {
            let mut deck = Deck::ALL;
            black_box(deck.draw(&mut rng, 13))
        });
    });
}

fn bench_full_deal(c: &mut Criterion) {
    c.bench_function("full_deal", |b| {
        let mut rng = SmallRng::seed_from_u64(0);
        b.iter(|| black_box(full_deal(&mut rng)));
    });
}

fn bench_fill_deals_known_north(c: &mut Criterion) {
    let mut builder = Builder::default();
    let mut rng = SmallRng::seed_from_u64(42);
    let mut deck = Deck::ALL;
    builder[Seat::North] = deck.draw(&mut rng, 13);
    let subset = builder.build_partial().expect("known partial deal is valid");

    c.bench_function("fill_deals_with_known_north_x100", |b| {
        let mut rng = SmallRng::seed_from_u64(0);
        b.iter(|| {
            for d in fill_deals(&mut rng, subset).take(100) {
                black_box(d);
            }
        });
    });
}

fn bench_full_deal_x100(c: &mut Criterion) {
    c.bench_function("full_deal_x100", |b| {
        let mut rng = SmallRng::seed_from_u64(0);
        b.iter(|| {
            for _ in 0..100 {
                black_box(full_deal(&mut rng));
            }
        });
    });
}

criterion_group!(
    benches,
    bench_draw_13,
    bench_full_deal,
    bench_full_deal_x100,
    bench_fill_deals_known_north,
);
criterion_main!(benches);
