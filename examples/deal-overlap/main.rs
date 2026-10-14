//! Count the deals two or more runs share, before pooling them or reading one
//! as the other's independent replication (#6).
//!
//! A source is a `bba-gen` dump (a file, or a directory of `shard-*.json`), or
//! `seeded:BASE:COUNT` for a harness on `common::seeded_deals`.  A deal is its
//! four hands, so the same hands under a different dealer or vulnerability
//! still count as shared.  Prints every pair's intersection and exits 1 when
//! any pair shares a deal. Sources containing no deals are rejected.
//!
//! ```text
//! cargo run --release --features serde --example deal-overlap -- \
//!   ab-results/run1/off-none ab-results/run2/off-none seeded:1790874659000:600000
//! ```

use contract_bridge::FullDeal;
use std::collections::HashSet;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;

fn deals(source: &str) -> HashSet<FullDeal> {
    let deals: HashSet<FullDeal> = if let Some(spec) = source.strip_prefix("seeded:") {
        let (base, count) = spec
            .split_once(':')
            .unwrap_or_else(|| panic!("expected seeded:BASE:COUNT, got {source}"));
        let parse = |s: &str| {
            s.parse()
                .unwrap_or_else(|e| panic!("bad number {s:?} in {source}: {e}"))
        };
        common::seeded_deals(parse(base), parse(count) as usize)
            .into_iter()
            .collect()
    } else {
        common::load_dump(source)
            .boards
            .into_iter()
            .map(|b| b.deal)
            .collect()
    };
    assert!(!deals.is_empty(), "no deals in {source}");
    deals
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let sources: Vec<String> = std::env::args().skip(1).collect();
    assert!(
        sources.len() >= 2,
        "usage: deal-overlap SOURCE SOURCE [SOURCE...]"
    );
    let sets: Vec<_> = sources.iter().map(|s| deals(s)).collect();
    let mut overlap = false;
    for i in 0..sets.len() {
        for j in i + 1..sets.len() {
            let shared = sets[i].intersection(&sets[j]).count();
            overlap |= shared > 0;
            println!(
                "{shared} shared ({:.2}% of {}, {:.2}% of {}): {} ∩ {}",
                100.0 * shared as f64 / sets[i].len() as f64,
                sets[i].len(),
                100.0 * shared as f64 / sets[j].len() as f64,
                sets[j].len(),
                sources[i],
                sources[j],
            );
        }
    }
    std::process::exit(i32::from(overlap));
}

#[cfg(test)]
mod tests;
