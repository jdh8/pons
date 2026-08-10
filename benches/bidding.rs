use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use core::cell::Cell;
use core::hint::black_box;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use pons::bidding::Rules;
use pons::bidding::array::Logits;
use pons::bidding::benchmark::{
    ActiveEvaluatorFeatures, active_evaluator_features, active_evaluator_forward,
    classify_instinct_scoped, classify_instinct_uncached, classify_with_provenance_uncached,
    is_deterministic_instinct_floor, select_legal_call,
};
use pons::bidding::evaluator::trick_estimates_with_auction;
use pons::bidding::inference::Inferences;
use pons::bidding::{Pair, Stance, System, Table, instinct};
use pons::{american, american_instinct};
use rand::SeedableRng;
use rand::rngs::StdRng;

#[path = "support/mod.rs"]
mod support;
use support::{Category, CountingAllocator, DepthBin, Position};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator::new();

const DEAL_SEED: u64 = 1;
const DEALS: usize = 64;

#[derive(Clone, Copy)]
struct Legacy<'a>(&'a Stance);

impl System for Legacy<'_> {
    fn classify(
        &self,
        hand: contract_bridge::Hand,
        vul: contract_bridge::auction::RelativeVulnerability,
        auction: &[Call],
    ) -> Option<Logits> {
        classify_with_provenance_uncached(self.0, hand, vul, auction).map(|(logits, _)| logits)
    }

    fn authored_at(
        &self,
        vul: contract_bridge::auction::RelativeVulnerability,
        auction: &[Call],
    ) -> bool {
        self.0.authored_at(vul, auction)
    }
}

fn as_auction(calls: &[Call]) -> Auction {
    let mut auction = Auction::new();
    auction
        .try_extend(calls.iter().copied())
        .expect("validated corpus auction");
    auction
}

fn fixed_deals() -> Vec<FullDeal> {
    (0..DEALS)
        .map(|index| full_deal(&mut StdRng::seed_from_u64(DEAL_SEED + index as u64)))
        .collect()
}

fn same_logits(one: &Logits, two: &Logits) -> bool {
    one.iter()
        .zip(two.iter())
        .all(|((one_call, one), (two_call, two))| {
            one_call == two_call && one.to_bits() == two.to_bits()
        })
}

fn validate_categories(stance: &Stance, deterministic: &Stance, positions: &[Position]) {
    for position in positions {
        let (logits, provenance) = stance
            .classify_with_provenance(position.hand, position.vul, &position.auction)
            .unwrap_or_else(|| panic!("corpus position {} is uncovered", position.id));
        match position.category {
            Category::AuthoredShallow | Category::AuthoredDeep => assert!(
                provenance.depth > 0 && provenance.fallback.is_none(),
                "position {} is labelled authored but resolves to the root floor: {provenance:?}",
                position.id
            ),
            Category::NeuralFloor | Category::ConstructiveFloor | Category::ForcedInstinctFloor => {
                assert!(
                    provenance.depth == 0 && provenance.fallback.is_some(),
                    "position {} is labelled floor but has {provenance:?}",
                    position.id
                )
            }
            Category::RkcbSlamTail => {
                assert!(
                    support::is_rkcb_historical_decode(&position.auction),
                    "position {} is not an asker decoding partner's RKCB answer",
                    position.id
                );
                assert!(
                    provenance.depth == 0 && provenance.fallback.is_some(),
                    "position {} bypasses the floor RKCB decoder: {provenance:?}",
                    position.id
                );
            }
            Category::SystemsOn => assert!(
                provenance.rebases > 0,
                "position {} is labelled systems-on but has no rebase: {provenance:?}",
                position.id
            ),
            Category::Fallback => assert!(
                provenance.depth > 0 && provenance.fallback.is_some() && provenance.rebases == 0,
                "position {} is not a direct non-root fallback: {provenance:?}",
                position.id
            ),
            Category::Representative => {}
        }
        if position.category == Category::ConstructiveFloor {
            assert_eq!(
                pons::bidding::book::Phase::of(&position.auction),
                pons::bidding::book::Phase::Constructive
            );
        }
        if position.category == Category::ForcedInstinctFloor {
            let instinct = deterministic
                .classify(position.hand, position.vul, &position.auction)
                .expect("deterministic floor covers category position");
            assert!(
                same_logits(&logits, &instinct),
                "position {} should exercise deterministic floor delegation",
                position.id
            );
        }
        if position.category == Category::NeuralFloor {
            let instinct = deterministic
                .classify(position.hand, position.vul, &position.auction)
                .expect("deterministic floor covers neural position");
            assert!(
                !same_logits(&logits, &instinct),
                "position {} does not distinguish neural judgement from instinct",
                position.id
            );
        }
    }
}

fn cursor<'a>(positions: &'a [&'a Position]) -> impl FnMut() -> &'a Position {
    let index = Cell::new(0_usize);
    move || {
        let current = index.get();
        index.set((current + 1) % positions.len());
        positions[current]
    }
}

#[allow(clippy::too_many_arguments)]
fn allocation_report(
    stance: &Stance,
    positions: &[Position],
    contexts: &[pons::bidding::Context<'_>],
    inferences: &[Inferences],
    features: &[ActiveEvaluatorFeatures],
    ladder: &Rules,
    pair: &Pair,
    logits: &[Logits],
    auctions: &[Auction],
    deals: &[FullDeal],
    tables: &[Table<&Stance, &Stance>],
    legacy_tables: &[Table<Legacy<'_>, Legacy<'_>>],
    hot_instinct: &[&Position],
) {
    fn measured(name: &str, operations: usize, run: impl FnOnce()) {
        ALLOCATOR.reset();
        run();
        let snapshot = ALLOCATOR.snapshot();
        eprintln!(
            "alloc/{name}: {:.3} allocations/op, {:.1} requested bytes/op ({} ops)",
            snapshot.allocations as f64 / operations as f64,
            snapshot.bytes as f64 / operations as f64,
            operations
        );
    }
    measured("inferences", positions.len(), || {
        for context in contexts {
            black_box(Inferences::read(black_box(context)));
        }
    });
    measured("evaluator-features", positions.len(), || {
        for (position, inference) in positions.iter().zip(inferences) {
            black_box(active_evaluator_features(
                black_box(position.hand),
                black_box(inference),
                black_box(&position.auction),
            ));
        }
    });
    measured("evaluator-forward", positions.len(), || {
        for feature in features {
            black_box(active_evaluator_forward(black_box(feature)));
        }
    });
    measured("evaluator-complete", positions.len(), || {
        for (position, inference) in positions.iter().zip(inferences) {
            black_box(trick_estimates_with_auction(
                black_box(position.hand),
                black_box(inference),
                black_box(&position.auction),
            ));
        }
    });
    measured("instinct-scoped", positions.len(), || {
        for position in positions {
            black_box(classify_instinct_scoped(
                stance,
                ladder,
                black_box(position.hand),
                position.vul,
                black_box(&position.auction),
            ));
        }
    });
    measured("instinct-legacy", positions.len(), || {
        for position in positions {
            black_box(classify_instinct_uncached(
                stance,
                ladder,
                black_box(position.hand),
                position.vul,
                black_box(&position.auction),
            ));
        }
    });
    measured("full-classification", positions.len(), || {
        for position in positions {
            black_box(stance.classify_with_provenance(
                black_box(position.hand),
                position.vul,
                black_box(&position.auction),
            ));
        }
    });
    measured("hot-instinct-cached", hot_instinct.len(), || {
        for position in hot_instinct {
            black_box(stance.classify_with_provenance(
                black_box(position.hand),
                position.vul,
                black_box(&position.auction),
            ));
        }
    });
    measured("hot-instinct-legacy", hot_instinct.len(), || {
        for position in hot_instinct {
            black_box(classify_with_provenance_uncached(
                stance,
                black_box(position.hand),
                position.vul,
                black_box(&position.auction),
            ));
        }
    });
    measured("legal-selection", positions.len(), || {
        for (logits, auction) in logits.iter().zip(auctions) {
            black_box(select_legal_call(black_box(*logits), black_box(auction)));
        }
    });
    measured("whole-deal", deals.len(), || {
        for (table, deal) in tables.iter().zip(deals) {
            black_box(table.bid_out(black_box(deal)));
        }
    });
    measured("whole-deal-legacy", deals.len(), || {
        for (table, deal) in legacy_tables.iter().zip(deals) {
            black_box(table.bid_out(black_box(deal)));
        }
    });
    measured("stance-construction", 1, || {
        black_box(pair.against());
    });
}

fn bidding(c: &mut Criterion) {
    let positions = support::parse_corpus().expect("valid frozen bidding corpus");
    let pair = american(&pons::bidding::agreements::Agreements::default());
    let stance = pair.against();
    let deterministic =
        american_instinct(&pons::bidding::agreements::Agreements::default()).against();
    validate_categories(&stance, &deterministic, &positions);
    let hot_instinct: Vec<_> = positions
        .iter()
        .filter(|position| {
            is_deterministic_instinct_floor(
                &stance,
                &deterministic,
                position.hand,
                position.vul,
                &position.auction,
            )
        })
        .collect();
    assert!(
        !hot_instinct.is_empty(),
        "corpus has no deterministic instinct-delegating floor positions"
    );
    eprintln!(
        "deterministic instinct-delegating floor positions: {}",
        hot_instinct.len()
    );
    let contexts: Vec<_> = positions
        .iter()
        .map(|position| stance.prefixed_context(position.vul, &position.auction))
        .collect();
    let inferences: Vec<_> = contexts.iter().map(Inferences::read).collect();
    let features: Vec<_> = positions
        .iter()
        .zip(&inferences)
        .map(|(position, inference)| {
            active_evaluator_features(position.hand, inference, &position.auction)
        })
        .collect();
    let logits: Vec<_> = positions
        .iter()
        .map(|position| {
            stance
                .classify(position.hand, position.vul, &position.auction)
                .expect("floor is total")
        })
        .collect();
    let auctions: Vec<_> = positions
        .iter()
        .map(|position| as_auction(&position.auction))
        .collect();
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
                &stance,
                &stance,
                Seat::ALL[index % 4],
                vulnerabilities[index / 4 % 4],
            )
        })
        .collect();
    let legacy_tables: Vec<_> = (0..deals.len())
        .map(|index| {
            Table::new(
                Legacy(&stance),
                Legacy(&stance),
                Seat::ALL[index % 4],
                vulnerabilities[index / 4 % 4],
            )
        })
        .collect();

    // Force every lazy artifact used below before either allocation or timing.
    eprintln!("active evaluator variant: {}", features[0].variant());
    black_box(active_evaluator_forward(&features[0]));
    black_box(trick_estimates_with_auction(
        positions[0].hand,
        &inferences[0],
        &positions[0].auction,
    ));
    let ladder = instinct(&pons::bidding::agreements::Agreements::default());
    black_box(classify_instinct_scoped(
        &stance,
        &ladder,
        positions[0].hand,
        positions[0].vul,
        &positions[0].auction,
    ));
    black_box(stance.classify_with_provenance(
        positions[0].hand,
        positions[0].vul,
        &positions[0].auction,
    ));

    allocation_report(
        &stance,
        &positions,
        &contexts,
        &inferences,
        &features,
        &ladder,
        &pair,
        &logits,
        &auctions,
        &deals,
        &tables,
        &legacy_tables,
        &hot_instinct,
    );

    let mut inference_group = c.benchmark_group("inferences");
    for bin in DepthBin::ALL {
        let selected: Vec<_> = positions
            .iter()
            .zip(&contexts)
            .filter(|(position, _)| position.depth_bin == bin)
            .collect();
        let index = Cell::new(0_usize);
        inference_group.bench_function(BenchmarkId::from_parameter(bin.as_str()), |b| {
            b.iter(|| {
                let current = index.get();
                index.set((current + 1) % selected.len());
                black_box(Inferences::read(black_box(selected[current].1)))
            });
        });
    }
    inference_group.finish();

    let refs: Vec<_> = positions.iter().collect();
    let mut next = cursor(&refs);
    c.bench_function("evaluator/features", |b| {
        b.iter(|| {
            let position = next();
            let inference = &inferences[position.id as usize];
            black_box(active_evaluator_features(
                black_box(position.hand),
                black_box(inference),
                black_box(&position.auction),
            ))
        });
    });
    let feature_index = Cell::new(0_usize);
    c.bench_function("evaluator/forward", |b| {
        b.iter(|| {
            let index = feature_index.get();
            feature_index.set((index + 1) % features.len());
            black_box(active_evaluator_forward(black_box(&features[index])))
        });
    });
    let evaluator_index = Cell::new(0_usize);
    c.bench_function("evaluator/complete", |b| {
        b.iter(|| {
            let index = evaluator_index.get();
            evaluator_index.set((index + 1) % positions.len());
            let position = &positions[index];
            black_box(trick_estimates_with_auction(
                black_box(position.hand),
                black_box(&inferences[index]),
                black_box(&position.auction),
            ))
        });
    });
    let mut instinct_group = c.benchmark_group("acceptance/instinct-component");
    let scoped_instinct_index = Cell::new(0_usize);
    instinct_group.bench_function("scoped", |b| {
        b.iter(|| {
            let index = scoped_instinct_index.get();
            scoped_instinct_index.set((index + 1) % positions.len());
            black_box(classify_instinct_scoped(
                &stance,
                &ladder,
                black_box(positions[index].hand),
                positions[index].vul,
                black_box(&positions[index].auction),
            ))
        });
    });
    let legacy_instinct_index = Cell::new(0_usize);
    instinct_group.bench_function("legacy", |b| {
        b.iter(|| {
            let index = legacy_instinct_index.get();
            legacy_instinct_index.set((index + 1) % positions.len());
            black_box(classify_instinct_uncached(
                &stance,
                &ladder,
                black_box(positions[index].hand),
                positions[index].vul,
                black_box(&positions[index].auction),
            ))
        });
    });
    instinct_group.finish();
    let classify_index = Cell::new(0_usize);
    c.bench_function("stance/full-classification", |b| {
        b.iter(|| {
            let index = classify_index.get();
            classify_index.set((index + 1) % positions.len());
            let position = &positions[index];
            black_box(stance.classify_with_provenance(
                black_box(position.hand),
                position.vul,
                black_box(&position.auction),
            ))
        });
    });
    let mut categories = c.benchmark_group("stance/classification-by-category");
    for category in [
        Category::AuthoredShallow,
        Category::AuthoredDeep,
        Category::NeuralFloor,
        Category::ConstructiveFloor,
        Category::ForcedInstinctFloor,
        Category::RkcbSlamTail,
        Category::SystemsOn,
        Category::Fallback,
    ] {
        let selected: Vec<_> = positions
            .iter()
            .filter(|position| position.category == category)
            .collect();
        let index = Cell::new(0_usize);
        categories.bench_function(BenchmarkId::from_parameter(category.as_str()), |b| {
            b.iter(|| {
                let current = index.get();
                index.set((current + 1) % selected.len());
                let position = selected[current];
                black_box(stance.classify_with_provenance(
                    black_box(position.hand),
                    position.vul,
                    black_box(&position.auction),
                ))
            });
        });
    }
    categories.finish();
    let mut hot_group = c.benchmark_group("acceptance/hot-instinct-floor");
    let cached_hot_index = Cell::new(0_usize);
    hot_group.bench_function("cached", |b| {
        b.iter(|| {
            let index = cached_hot_index.get();
            cached_hot_index.set((index + 1) % hot_instinct.len());
            let position = hot_instinct[index];
            black_box(stance.classify_with_provenance(
                black_box(position.hand),
                position.vul,
                black_box(&position.auction),
            ))
        });
    });
    let legacy_hot_index = Cell::new(0_usize);
    hot_group.bench_function("legacy", |b| {
        b.iter(|| {
            let index = legacy_hot_index.get();
            legacy_hot_index.set((index + 1) % hot_instinct.len());
            let position = hot_instinct[index];
            black_box(classify_with_provenance_uncached(
                &stance,
                black_box(position.hand),
                position.vul,
                black_box(&position.auction),
            ))
        });
    });
    hot_group.finish();
    let legal_index = Cell::new(0_usize);
    c.bench_function("table/legal-call-selection", |b| {
        b.iter(|| {
            let index = legal_index.get();
            legal_index.set((index + 1) % positions.len());
            black_box(select_legal_call(
                black_box(logits[index]),
                black_box(&auctions[index]),
            ))
        });
    });

    let cached_deal_index = Cell::new(0_usize);
    let mut whole = c.benchmark_group("acceptance/whole-deal");
    whole.throughput(Throughput::Elements(1));
    whole.bench_function("cached", |b| {
        b.iter(|| {
            let index = cached_deal_index.get();
            cached_deal_index.set((index + 1) % deals.len());
            black_box(tables[index].bid_out(black_box(&deals[index])))
        });
    });
    let legacy_deal_index = Cell::new(0_usize);
    whole.bench_function("legacy", |b| {
        b.iter(|| {
            let index = legacy_deal_index.get();
            legacy_deal_index.set((index + 1) % deals.len());
            black_box(legacy_tables[index].bid_out(black_box(&deals[index])))
        });
    });
    whole.finish();

    c.bench_function("stance/Pair::against", |b| {
        b.iter(|| black_box(pair.against()));
    });
}

criterion_group!(benches, bidding);
criterion_main!(benches);
