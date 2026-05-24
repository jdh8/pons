# Pons

[![Build Status](https://github.com/jdh8/pons/actions/workflows/rust.yml/badge.svg)](https://github.com/jdh8/pons)
[![Crates.io](https://img.shields.io/crates/v/pons)](https://crates.io/crates/pons)
[![Docs.rs](https://docs.rs/pons/badge.svg)](https://docs.rs/pons)

This library provides tools for analyzing and simulating hands in the card
game contract bridge.  It is named after [an anatomical part of the
brainstem][pons] and also "bridge" in Latin.

[pons]: https://en.wikipedia.org/wiki/Pons

## Modules

- [`bidding`](https://docs.rs/pons/latest/pons/bidding/) — [`Trie`]-based representation of a bidding system. Auction primitives ([`Call`], [`Auction`], etc.) live in the [`contract-bridge`](https://crates.io/crates/contract-bridge) crate.
- [`stats`](https://docs.rs/pons/latest/pons/stats/) — numerically stable accumulators and double-dummy par scoring over histograms.

Card sets and shuffling live in [`contract-bridge::deck`](https://docs.rs/contract-bridge/latest/contract_bridge/deck/); hand-evaluation kernels in [`contract-bridge::eval`](https://docs.rs/contract-bridge/latest/contract_bridge/eval/).

## Feature flags

- `serde` — derive `Serialize`/`Deserialize` for the library's value types. Off by default.

## Quick start

Deal 10 random hands and evaluate the North hand with several point counts:

```rust
use contract_bridge::Seat;
use contract_bridge::deck::full_deal;
use contract_bridge::eval::{self, HandEvaluator};

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

Estimate NS par from random fill-in deals (requires `ddss`'s solver,
linked via `ddss-sys` in `dev-dependencies`; see
[`examples/average-ns-par`](examples/average-ns-par/main.rs)):

```rust
use contract_bridge::deck;
use contract_bridge::{Builder, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver, Vulnerability};
use pons::stats;
# let north_hand: Hand = "T9762.AT54.JT75.".parse().unwrap();
# let south_hand: Hand = "A.KQ962.A86.Q642".parse().unwrap();

let cards = Builder::new()
    .north(north_hand)
    .south(south_hand)
    .build_partial()
    .expect("north and south hands are disjoint and ≤13 each");
let solutions = Solver::lock().solve_deals(
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

- `check-nltc` and `check-zar` — validate hand-evaluation methods against double-dummy results.
- `average-ns-par` — Monte-Carlo NS par score for a partial deal.
- `defend-2sx-or-3nt` — compare expected NS score from defending 2♠× vs declaring 3NT.

Run any of them with `cargo run --example <name>`.

Two examples that don't need pons live one level down the stack:
[`generate-deals`](https://github.com/jdh8/contract-bridge/tree/main/examples/generate-deals)
in `contract-bridge` and
[`notrump-tricks`](https://github.com/jdh8/ddss/tree/main/examples/notrump-tricks)
in `ddss` (with a [parallel
copy](https://github.com/jdh8/dds-bridge/tree/main/examples/notrump-tricks)
in `dds-bridge`).

[`Call`]: https://docs.rs/contract-bridge/latest/contract_bridge/auction/enum.Call.html
[`Auction`]: https://docs.rs/contract-bridge/latest/contract_bridge/auction/struct.Auction.html
[`Trie`]: https://docs.rs/pons/latest/pons/bidding/trie/struct.Trie.html
