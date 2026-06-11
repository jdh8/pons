# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `bidding::context`: `Context`, the mechanical auction context passed to
  classifiers and constraints — vulnerability (relative to the side to act),
  the raw table auction, and facts derived from it (bid strains per side,
  partner's last bid, the contract to beat, doubling state, passed-hand and
  seat facts, `min_level`). Also `context::relative(AbsoluteVulnerability,
  Seat)`, the only vulnerability conversion in the crate: drivers convert
  once per `classify` call, and systems pass the relative value through
  unchanged.
- `bidding::constraint`: a composable constraint vocabulary for authoring
  rules. A `Constraint` maps `(Hand, &Context)` to a logit contribution;
  crisp predicates return `0.0`/`-∞`. Primitives: `hcp`, `len`, `balanced`,
  `nltc_at_most`, the context-relative `support`, `stopper_in_their_suits`,
  `passed_hand`, `undisturbed`, `vulnerable`, `they_vulnerable`, `nth_seat`,
  and the `pred` escape hatch. Constraints compose with `&` (sum, AND for
  crisp), `|` (max, OR), and `!` (crisp flip) on the `Cons` wrapper.
- `bidding::rules`: `Rules`, an ordered rule list acting as a `Classifier`.
  Each `Rule` ties a call to a constraint with a weight (soft priority); a
  call's logit is the **max** of `weight + constraint` over its rules.
  `Rules::explain` reports the winning rule per call — "why did you bid
  that".
- `bidding::fallback`: guarded fallbacks generalizing the trie over
  competitive auctions. `Trie::fallback_at` attaches ordered `(Guard,
  Fallback)` entries to a node; `Trie::resolve` answers from the exact book
  first, then walks up from the deepest reachable node taking the first
  admitted fallback, reporting a `Provenance` (depth, entry index, rebase
  count). `Fallback::Rebase` rewrites the auction and re-resolves (at most
  `REBASE_LIMIT` times) — "system on over their double" is
  `FirstIs(Call::Double)` + `ReplaceNext(Call::Pass)` instead of a copied
  subtree. Stock guards: `Always`, `Undisturbed`, `FirstIs`,
  `OvercallAtMost`.
- `bidding::compose`: lazy `System` combinators. `a.vs(b)` composes a table
  where `a`'s partnership is the dealer's side, dispatching purely by
  auction-length parity; the opposing slot is also where an approximate
  opponent model goes. `a.or_else(b)` layers `a` over a fallback system,
  falling through on `None` or logits without probability mass. A blanket
  `impl System for &S` lets `(&a).vs(&a)` work without cloning.
- `Trie::merge` and `Forest::merge`: structural union for assembling a
  system from separately authored fragments (uncontested core + competitive
  packages). On collision `self` keeps its classifier and the keys are
  reported back; fallback lists concatenate with `self`'s first; `Arc`s are
  reused.
- `Forest::insert` and `Forest::fallback_at` take a `SeatClasses` mask
  (`UNPASSED | PASSED`) and share one `Arc` across both books when both are
  selected.
- `classifier`, `guard`, and `rewriter`: identity functions giving plain
  closures the higher-ranked `&Context`/`&[Call]` signature the compiler
  cannot generalize on its own.

### Changed

- Reduced Clippy noise across bidding internals and tests: several small
  closure-coercion/context helpers are now `const fn`, builder-style
  constraint constructors gained `#[must_use]`, doc-code link formatting was
  cleaned up, and float assertions in tests were refactored to robust helper
  predicates instead of direct float equality.

- **Breaking:** `bidding::trie::Classifier::classify` now takes
  `(Hand, &Context)` instead of
  `(Hand, RelativeVulnerability, CommonPrefixes)`. The context carries the
  vulnerability and (optionally) the common prefixes. Closure classifiers
  change from `|hand, vul| …` to `classifier(|hand, context| …)`.
- **Breaking:** `Forest` is redesigned from the vulnerability-indexed
  `[Trie; 4]` into a *partnership system* `{ unpassed, passed }` keyed by
  pass-stripped auctions. `Forest::from_fn` and the
  `Index<RelativeVulnerability>` impls are removed; vulnerability
  conditions move into constraints (`vulnerable()` / `they_vulnerable()`).
  With `k` leading passes, the opener's side is "passed" iff `k ≥ 2`, the
  defenders' iff `k ≥ 1`; stripping makes 1st/2nd-seat books (and 3rd/4th)
  literally share nodes and normalizes parity: even stripped depth = the
  opening side acts.
- `System for Trie` resolves through fallbacks (`Trie::resolve`) instead of
  exact lookup only, so a trie with fallbacks now answers auctions outside
  its book. `get`, `longest_prefix`, `common_prefixes`, and `suffixes` are
  unchanged. The `System` docs pin the vulnerability convention: `vul` is
  relative to the side to act, and composite systems pass it through
  unchanged.

- **Breaking:** `stats::average_ns_par`'s vulnerability parameter is now
  `contract_bridge::AbsoluteVulnerability` instead of `ddss::Vulnerability`.
  `AbsoluteVulnerability` is a new NS/EW bit set in `contract-bridge` 0.1.1 (now
  the minimum required version) that mirrors the existing `RelativeVulnerability`
  for symmetry. The four values map one-to-one — replace
  `ddss::Vulnerability::{NONE, NS, EW, ALL}` with the same constants on
  `contract_bridge::AbsoluteVulnerability`. The double-dummy solver is unchanged
  (still `ddss`).

## [0.8.0] — 2026-05-24

### Changed

- **Breaking:** Replace the `dds-bridge` dependency with `ddss` (a
  performance-oriented DDS fork) and the `dds-bridge-sys` dev-dependency
  with `ddss-sys`. Most public types are structurally compatible — `Par`,
  `ParContract`, `TrickCountTable`, `TrickCountRow`, and `Vulnerability` all
  live at the same paths under `ddss::*` — so downstream callers usually
  only need to swap the crate name in imports. Two shape changes:
  - `dds_bridge::Solver::default()` → `ddss::Solver::lock()`. The new
    handle holds a reentrant lock, so its solve methods take `&self` (drop
    the `mut`) and the type is `!Send`.
  - The free `dds_bridge::solve_deals(&deals)` is now a method that takes
    a non-empty strain selector: `Solver::lock().solve_deals(&deals,
    NonEmptyStrainFlags::ALL)` reproduces the old all-strains behavior.
  `calculate_par` remains a free function with the same signature and can
  be called with or without a held `Solver` (it acquires the global ddss
  lock internally; the lock is reentrant per thread).
- **Breaking:** Auction primitives (`Call`, `Auction`, `IllegalCall`,
  `RelativeVulnerability`, and their parse errors), the entire `eval`
  module (`HandEvaluator`, `SimpleEvaluator`, `hcp`, `shortness`,
  `fifths`, `bumrap`, `ltc`, `nltc`, `zar`, `hcp_plus`, `FIFTHS`,
  `BUMRAP`, `BUMRAP_PLUS`, `NLTC`), and the entire `deck` module
  (`Deck`, `full_deal`, `FillDeals`, `fill_deals`) move into the new
  `contract-bridge` crate. Update imports such as
  `use pons::bidding::Call;` → `use contract_bridge::auction::Call;`,
  `use pons::eval::hcp;` → `use contract_bridge::eval::hcp;`, and
  `use pons::deck::full_deal;` →
  `use contract_bridge::deck::full_deal;`.
- **Breaking:** `pons` no longer re-exports bridge data types
  (`Hand`, `Strain`, `Bid`, `Seat`, etc.) — these live in the new
  `contract-bridge` crate, not `dds-bridge`. Replace
  `use dds_bridge::Hand;` with `use contract_bridge::Hand;`.
- Track `dds-bridge`'s flattening of the `solver` module to the crate
  root: `dds_bridge::solver::*` imports become `dds_bridge::*` (e.g.
  `dds_bridge::solver::Vulnerability` → `dds_bridge::Vulnerability`).
- Relocated tests that exercised only lower-crate APIs out of `pons`,
  so failures point at the crate they actually cover. `tests/eval.rs`,
  `tests/deck.rs`, `tests/proptest_roundtrip.rs`, and `tests/solver.rs`
  are removed; the auction block in `tests/bidding.rs` and the
  contract-bridge/ddss serde tests in `tests/serde.rs` are removed in
  place, leaving only `Array`/`Map`/`Logits` tests and pons stats serde
  respectively. The moved tests now live in `contract-bridge` (auction,
  deck, eval, proptest, serde) and `ddss`/`dds-bridge` (large-batch
  solver). No behavior or public-API change in pons.
- Dev-dependencies pruned: `approx` and `ddss-sys` are no longer used
  by anything in `pons` and are removed from `Cargo.toml`.

### Removed

- `pons::deck` and `pons::eval` modules (moved to `contract-bridge`).
- The crate-root re-exports `Deck`, `full_deal`, `HandEvaluator`,
  `Auction`, `Call` (moved to `contract-bridge`).
- The `generate-deals` and `notrump-tricks` examples. They no longer
  depended on anything in `pons` and now live with the crates they
  actually need: `generate-deals` in
  [`contract-bridge`](https://github.com/jdh8/contract-bridge/tree/main/examples/generate-deals)
  and `notrump-tricks` in
  [`ddss`](https://github.com/jdh8/ddss/tree/main/examples/notrump-tricks)
  (with a [parallel
  copy](https://github.com/jdh8/dds-bridge/tree/main/examples/notrump-tricks)
  in `dds-bridge`).

### Fixed

- README's `average_ns_par` doctest no longer overflows the stack on
  Windows. The fix is in `ddss` 0.1.2 (now the minimum required
  version): the batch solver's FFI packs are allocated directly on the
  heap via `Box::new_zeroed`, instead of routing through a stack
  temporary as `Box::default()` does at opt-level 0.

### Internal

- Set `[profile.dev.package."*"]` to `opt-level = 2`, so dependencies —
  most notably `ddss-sys`'s C++ DDS engine via `cc` — are optimized in
  dev builds. Pons's own Rust stays at opt-level 0 so any future
  stack-temp-class bug in this crate's own code still surfaces under
  `cargo test`. Big speedup for the `average_ns_par` doctest and
  `tests/par.rs`.

## [0.7.0] — 2026-05-20

### Changed

- **Breaking:** Bump `dds-bridge` to **0.19** and `dds-bridge-sys` to **3.0**
  (the latter is a dev-dependency used only by `tests/solver.rs`). The
  underlying DDS C++ library moves to v3.0.0 with PascalCase struct names
  and snake_case fields; `pons`'s own safe API is unaffected, but downstream
  users who pin to older versions of these dependencies should also bump
  them in lockstep. See the `dds-bridge-sys` v3.0.0 and `dds-bridge` v0.19.0
  changelogs for the rename map.

### Added
- New `defend-2sx-or-3nt` example: compares the expected NS score from
  defending 2♠× vs declaring 3NT after the auction `(2♠) X (P)`. The
  bidding system is a single `Trie` with three classifiers — West's
  weak-two opening at `[]`, North's takeout double at `[2♠]`, and South's
  natural call at `[2♠, X, P]` (which may be Pass, 3NT, or an
  out-of-scope call such as a 3-level new suit, jump in hearts, or
  Lebensohl 2NT). South's classifier is used only as an eligibility
  filter: deals are rejection-sampled so only those where West opens 2♠,
  North doubles, *and* South naturally faces a P-or-3NT decision are
  kept and double-dummy solved. Each accepted deal is scored under three
  strategies — always defend 2♠×, always declare 3NT, and a per-deal
  oracle that picks the higher of the two — giving an upper bound on
  what any policy keyed on South's hand could achieve. Scoring uses
  `dds_bridge::Contract::score`. Accepts an optional `--south` for
  hand-specific analysis (errors if the hand falls out of scope) or
  randomizes all four seats when omitted.

## [0.6.1] — 2026-04-25

### Changed
- Updated `dds-bridge` dependency to 0.18
- `full_deal` now returns `FullDeal` (was `Deal`)
- `fill_deals` now takes a pre-validated `PartialDeal`; no longer returns `Result`
- Track `dds-bridge`'s trick-count rename: `solver::TricksTable` → `solver::TrickCountTable` in `stats::HistogramTable`'s `FromIterator` impl and in the `check-zar` / `check-nltc` examples. Pure rename on the consumer side.
- The `serde` feature now also pulls in `serde_with` (optional dep).

### Internal
- Replaced the last hand-written `serde_impl` submodule (on `Deck`) with `serde_with::SerializeDisplay` / `DeserializeFromStr` derives. No change to the serialized form.
- Replaced non-const `.unwrap()` in tests and the `Auction::declarer` doctest with `?` propagation. Tests with a single fallible error type return `Result<(), E>`; tests mixing error types or unwrapping `Option` return `anyhow::Result<()>`.
- Moved inline `mod tests` blocks in `bidding.rs` and `deck.rs` into dedicated `bidding/tests.rs` and `deck/tests.rs` files. No behavior change.

## [0.6.0] — 2026-04-19

### Added
- Optional `serde` feature for serialization/deserialization support
- `Display` and `FromStr` implementations for `Deck` and bidding types
- `Classifier` promoted to a trait (was a plain `fn` in 0.5.0)
- Constructors for `Forest`
- `FusedIterator` implementation for `Trie` iterators
- `Debug` on `Trie` and iterator types
- Slicing API for `Auction`; `Index<Range<Bid>>` and bid-range indexing on `Array`
- `Logits::softmax` (replaces `to_odds`); returns `None` when all logits are `-∞`
- `fill_deals` helper
- Criterion benchmarks for shuffle, trie, and parallel solving
- proptest-based roundtrip and histogram invariant tests

### Changed
- `System::classify` now takes a slice
- `Auction::push` is panicking; confusing `force_push` removed
- `Deck` rejects duplicate cards
- `RelativeVulnerability` renamed from previous type
- Converters borrow instead of consuming
- Public fields replaced with getters
- Error types marked `#[non_exhaustive]`
- `average_ns_par` return type improved; redundant count parameter removed
- Random deal generation moved to `dds-bridge`; local `solver` module renamed to `random`
- Deterministic stats moved to `mod stats`
- MSRV pinned to 1.93
- Updated `dds-bridge` dependency to 0.16

### Fixed
- Memory leak in `Array::try_map`
- `hcp_plus` calculation

### Internal
- Added `#[inline]` to trivial getters on `Copy` types
- Aligned `HistogramRow::count` to take `self` (non-breaking: `HistogramRow: Copy`)
- Deduplicated `Map::get_mut`
- Bidding context lives with the stored classifier; shared API between systems and classifiers
- Hardened GitHub workflow; CI enforces `fmt`, `clippy`, and doc warnings
- Expanded README; documented the `map` module

## [0.5.0] — 2026-03-25

### Added
- `Array<T>` modeling `Call -> T`, with `Array`-like and full iterator API
- `Map` with iteration over keys, values, and entries; separated iteration for arrays
- `Logits` module (under `mod array`); `Logits::to_odds`
- Abstract bidding table supporting multiple calls per node
- Classifier concept (as a plain `fn`) replacing the filter-based approach
- Own `bidding::Vulnerability` type
- Absolute `bidding::Frequency` for easier filtering
- Different indices for X (double) and XX (redouble)

### Changed
- Edition updated to Rust 2024
- Magic number 38 replaced with a named constant

## [0.3.1] — 2025-05-31

### Fixed
- `Strategy` now requires `RefUnwindSafe` so `Trie` stays `UnwindSafe`

### Internal
- Inlined small functions for optimization

## [0.3.0] — 2025-05-30

### Added
- Core bridge data structures: `Card`, `Suit`, `Hand`, `Deck`, `Holding`
- `SmallSet` trait for `Holding` and `Hand`
- DDS (double-dummy solver) integration via `dds-bridge`
- Contract scoring
- Bitset operators for `Holding` and `Hand`
- Basic CLI to solve random deals
- Hand evaluation (LTC, NLTC, BUM-RAP, Zar points)
- `Auction` with `push`, `pop`, and `truncate`
- `Trie` for bidding strategies, with depth-first iteration, suffix and prefix iterators
- Statistics utilities for evaluators; histograms

[0.8.0]: https://github.com/jdh8/pons/compare/0.7.0...0.8.0
[0.7.0]: https://github.com/jdh8/pons/compare/0.6.1...0.7.0
[0.6.1]: https://github.com/jdh8/pons/compare/0.6.0...0.6.1
[0.6.0]: https://github.com/jdh8/pons/releases/tag/0.6.0
[0.5.0]: https://github.com/jdh8/pons/releases/tag/0.5.0
[0.3.1]: https://github.com/jdh8/pons/releases/tag/0.3.1
[0.3.0]: https://github.com/jdh8/pons/releases/tag/0.3.0
