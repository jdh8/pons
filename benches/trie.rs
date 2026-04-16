use criterion::{Criterion, criterion_group, criterion_main};
use dds_bridge::{Bid, Hand, Level, Strain};
use pons::bidding::array::Logits;
use pons::bidding::{Call, RelativeVulnerability, System, Trie};
use std::hint::black_box;

fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

fn just_pass() -> Logits {
    let mut logits = Logits::new();
    *logits.0.get_mut(Call::Pass) = 0.0;
    logits
}

/// Build a trie populated with a small grid of opening + response auctions.
fn populated_trie() -> Trie {
    let mut trie = Trie::new();
    trie.insert(&[], |_, _| just_pass());
    for level in 1..=3 {
        for strain in Strain::ASC {
            let opening = bid(level, strain);
            trie.insert(&[opening], |_, _| just_pass());
            trie.insert(&[opening, Call::Pass], |_, _| just_pass());
            for response_level in level..=4.min(level + 2) {
                for response_strain in Strain::ASC {
                    let response = bid(response_level, response_strain);
                    if matches!(response, Call::Bid(b) if b > Bid { level: Level::new(level), strain })
                    {
                        trie.insert(&[opening, Call::Pass, response], |_, _| just_pass());
                    }
                }
            }
        }
    }
    trie
}

fn bench_get(c: &mut Criterion) {
    let trie = populated_trie();
    let query = [bid(1, Strain::Hearts), Call::Pass, bid(2, Strain::Notrump)];
    c.bench_function("Trie::get_depth_3", |b| {
        b.iter(|| black_box(trie.get(black_box(&query))));
    });
}

fn bench_longest_prefix(c: &mut Criterion) {
    let trie = populated_trie();
    let query = [
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(7, Strain::Notrump), // diverges — forces full descent
    ];
    c.bench_function("Trie::longest_prefix_diverging", |b| {
        b.iter(|| black_box(trie.longest_prefix(black_box(&query))));
    });
}

fn bench_common_prefixes(c: &mut Criterion) {
    let trie = populated_trie();
    let query = [bid(1, Strain::Hearts), Call::Pass, bid(2, Strain::Notrump)];
    c.bench_function("Trie::common_prefixes_count", |b| {
        b.iter(|| black_box(trie.common_prefixes(black_box(&query)).count()));
    });
}

fn bench_iter(c: &mut Criterion) {
    let trie = populated_trie();
    c.bench_function("Trie::iter_count", |b| {
        b.iter(|| black_box(trie.iter().count()));
    });
}

fn bench_classify(c: &mut Criterion) {
    let trie = populated_trie();
    let auction = [bid(1, Strain::Hearts)];
    c.bench_function("System::classify_via_Trie", |b| {
        b.iter(|| {
            black_box(trie.classify(
                black_box(Hand::default()),
                RelativeVulnerability::NONE,
                black_box(&auction),
            ))
        });
    });
}

criterion_group!(
    benches,
    bench_get,
    bench_longest_prefix,
    bench_common_prefixes,
    bench_iter,
    bench_classify,
);
criterion_main!(benches);
