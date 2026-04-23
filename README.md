# Pons

[![Build Status](https://github.com/jdh8/pons/actions/workflows/rust.yml/badge.svg)](https://github.com/jdh8/pons)
[![Crates.io](https://img.shields.io/crates/v/pons)](https://crates.io/crates/pons)
[![Docs.rs](https://docs.rs/pons/badge.svg)](https://docs.rs/pons)

This library provides tools for analyzing and simulating hands in the card
game contract bridge.  It is named after [an anatomical part of the
brainstem][pons] and also "bridge" in Latin.

[pons]: https://en.wikipedia.org/wiki/Pons

## Modules

- [`bidding`](https://docs.rs/pons/latest/pons/bidding/) — [`Call`], [`Auction`], and a [`Trie`]-based representation of a bidding system.
- [`deck`](https://docs.rs/pons/latest/pons/deck/) — card sets, shuffling, and iterators that fill in partial deals.
- [`eval`](https://docs.rs/pons/latest/pons/eval/) — hand-evaluation kernels (HCP, shortness, LTC, NLTC, BUM-RAP, Fifths, Zar).
- [`stats`](https://docs.rs/pons/latest/pons/stats/) — numerically stable accumulators and double-dummy par scoring over histograms.

## Feature flags

- `serde` — derive `Serialize`/`Deserialize` for the library's value types. Off by default.

## Quick start

Deal 10 random hands and evaluate the North hand with several point counts:

```rust
use pons::{full_deal, eval};
use pons::eval::HandEvaluator;
use dds_bridge::Seat;

let mut rng = rand::rng();
for _ in 0..10 {
    let deal = full_deal(&mut rng);
    let north = deal[Seat::North];

    let hcp: u8 = eval::SimpleEvaluator(eval::hcp).eval(north);
    let nltc: f64 = eval::NLTC.eval(north);
    let zar: u8 = eval::zar(north);

    println!("{}  HCP={hcp}  NLTC={nltc}  Zar={zar}", deal.display(Seat::North));
}
```

Estimate NS par from random fill-in deals (requires `dds-bridge`'s solver,
linked via `dds-bridge-sys` in `dev-dependencies`; see
[`examples/average-ns-par`](examples/average-ns-par/main.rs)):

```rust
use pons::{deck, stats};
use dds_bridge::{Builder, Hand, Seat};
use dds_bridge::solver::{self, NonEmptyStrainFlags, Vulnerability};
# let north_hand: Hand = "T9762.AT54.JT75.".parse().unwrap();
# let south_hand: Hand = "A.KQ962.A86.Q642".parse().unwrap();

let cards = Builder::new()
    .north(north_hand)
    .south(south_hand)
    .build_partial()
    .expect("north and south hands are disjoint and ≤13 each");
let solutions = solver::Solver::lock().solve_deals(
    &deck::fill_deals(&mut rand::rng(), cards).take(90).collect::<Vec<_>>(),
    NonEmptyStrainFlags::ALL,
);
let par = stats::average_ns_par(
    solutions.into_iter().collect(),
    Vulnerability::NONE,
    Seat::North,
);
```

## Examples

The [`examples/`](examples/) directory has runnable programs:

- `generate-deals` — stream random deals to stdout.
- `notrump-tricks` — average tricks taken in notrump per hand feature.
- `check-nltc` and `check-zar` — validate hand-evaluation methods against double-dummy results.
- `average-ns-par` — Monte-Carlo NS par score for a partial deal.

Run any of them with `cargo run --example <name>`.

## MSRV

Pons currently requires Rust **1.93**. The CI matrix builds and tests on the
MSRV toolchain on Ubuntu, macOS, and Windows.

[`Call`]: https://docs.rs/pons/latest/pons/bidding/enum.Call.html
[`Auction`]: https://docs.rs/pons/latest/pons/bidding/struct.Auction.html
[`Trie`]: https://docs.rs/pons/latest/pons/bidding/trie/struct.Trie.html
