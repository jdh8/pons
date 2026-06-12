# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Broken intra-doc links in `bidding::two_over_one`: replaced the unresolvable
  `[`slam`]` reference (a private module) with plain backtick notation, and
  qualified `[`Pair::against`]` with its full crate path so rustdoc can resolve
  it from `competition`.

### Changed

- `two_over_one()` attaches the instinct floor (see `bidding::instinct` under
  *Added*) to its competitive and defensive books as a root `Always`
  fallback, so the bound stance never falls off the book in a contested
  auction. Auctions that previously classified as `None` — and so were passed
  by drivers, including passing partner's takeout double on a worthless hand
  — now get a natural answer: their three-level preempts, jump overcalls past
  the negative-double range, and deep competitive continuations among them.
  Authored rules are unaffected: resolution reaches the root fallback last.
  The standalone `competition()` and `defensive()` builders stay floor-less.
- The `defend-2sx-or-3nt` example is now a flavor-comparison harness for the
  `(2♠) X (P)` defend-vs-declare decision. West's weak-two opening still comes
  from the real `two_over_one` system, while North's takeout double and South's
  Pass-vs-3NT advance are swept across alternative *flavors* — Shape / Support /
  Sound doubles and Defense / Balanced / Offense responses — each written as a
  crisp constraint in the `bidding::constraint` vocabulary. It reports
  per-double-flavor population stats and per-response-policy regret against a
  double-dummy oracle.

### Added

- `bidding::instinct`: the instinct bidder, a keyless floor for off-book
  auctions. `instinct()` is one context-driven `Rules` ladder that answers
  *every* auction with a sane natural action: penalty pass only with a trump
  stack, raises of partner's suit with three-card support and rising strength
  per level, notrump and five-card-suit overcalls when we have not bid,
  takeout doubles of their low suit bids, and pass. Partner's *live takeout
  double* (the auction ends `… (bid) X (Pass)` with their suit bid at the
  three level or below doubled) is recognized mechanically and never passed
  without a trump stack — the advance ladder guarantees an action down to a
  cheapest-notrump escape. Every instinct call is natural, so the floor stays
  coherent when both partners land on it; strength-showing cue-bids are
  deliberately excluded until both sides of the convention can be authored.
  Floor activations are observable as `Provenance { depth: 0, fallback:
  Some(_), .. }` from `Trie::resolve` — in simulation, the most-hit auctions
  are the next nodes worth authoring properly.
- `bidding::constraint`: `they_bid(strain)` and `short_in_their_suits()`
  (takeout shape: at most three cards in each suit the opponents have bid),
  promoted from private helpers in the 2/1 books.
- `bidding::two_over_one::defense_to_weak_two` and `advance_double`: defense to
  the opponents' weak twos and advancing partner's takeout double, filling the
  one gap the `defend-2sx-or-3nt` example needed. The defensive book now
  answers a `(2♦/2♥/2♠)` opening with a takeout double, a natural 15–18 2NT
  overcall, and cheapest-level suit overcalls; `advance_double` answers
  `(opening) X (P)` for advancer with a penalty pass on a trump stack, a
  major-suit game jump, 3NT with a stopper, cheapest-level new suits, and a
  lebensohl-style escape to the cheapest notrump. Bid levels are derived from
  the opening, so the one advancer builder serves both one-bids and weak twos
  (it is now also registered after `(1x) X (P)`).
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
- `bidding::book`: role-aware pair books, split three ways by the `Phase` of
  the auction. A `Constructive` book covers the strictly uncontested auctions
  (openings keyed per seat by their leading passes, and the continuations
  while the opponents only pass), a `Competitive` book covers the auctions
  where we open and they intervene (negative doubles, "system on" rebases),
  and a `Defensive` book covers the auctions where they open (overcalls,
  doubles, defense). All three wrap and deref to `Trie`; each adds a *gated*
  `System` impl that answers only for its phase. `Phase::of(&[Call])` is the
  single routing point that assumes a standard pass — forcing/strong-pass
  openings stay out of scope (use a bare `Trie`).
- `bidding::book`: `Pair` and `Stance` — a pair's authored system and its
  bound form. A `Pair` assembles the three books with a `Family` identity (an
  open `&'static str` newtype with stock constants such as `NATURAL` and
  `POLISH_CLUB`; downstream systems mint their own) plus optional
  per-opponent-family overrides (`competitive_vs`, `defensive_vs`), because
  what the competitive and defensive books mean depends on the opponents'
  system. Binding happens once at table assembly: `pair.against(them)`
  selects the books for that opposing family and returns a `Stance`, the
  `System` that classifies by `Phase`. `against` merges a structural clone of
  the constructive trie into the bound competitive trie (classifiers stay
  `Arc`-shared), so a competitive rebase such as "system on over their
  double" lands in the uncontested core; constructive-phase queries use the
  unmerged constructive trie, so no competitive fallback can leak into
  undisturbed auctions. Seat-dependent systems (e.g. transfer openings in
  1st/2nd seat only) need no extra machinery — each seat already has its own
  subtree under its leading-pass prefix — and vulnerability-dependent
  agreements are authored with the `vulnerable()`/`they_vulnerable()`
  constraints.
- `bidding::table`: `Table`, the absolute-seat table driver. It seats two
  systems as North/South and East/West with a dealer and an
  `AbsoluteVulnerability`, rotates the seat to act, converts the
  vulnerability once per call, filters illegal calls with the contract-bridge
  law (`Auction::can_push`), and bids a deal out (`classify`, `next_call`,
  `bid_out`). `Table::of_pairs(ns, ew, dealer, vul)` binds two `Pair`s
  against each other's families and seats them. `Versus` remains the lazy,
  dealer-relative composer; `Table` is the full-board driver and deliberately
  not a `System` (the trait speaks relative vulnerability).
- `Trie::merge`: structural union for assembling a system from separately
  authored fragments (uncontested core + competitive packages). On collision
  `self` keeps its classifier and the keys are reported back; fallback lists
  concatenate with `self`'s first; `Arc`s are reused.
- `classifier`, `guard`, and `rewriter`: identity functions giving plain
  closures the higher-ranked `&Context`/`&[Call]` signature the compiler
  cannot generalize on its own.
- `bidding::two_over_one`: the first concrete, reusable system —
  `two_over_one()` builds a `Pair` (family `NATURAL`) for basic Two-over-One
  Game Forcing (re-exported as `pons::two_over_one`). It covers the uncontested openings
  (strong 15–17 / 20–21 notrumps, strong artificial 2♣, five-card majors,
  better-minor 1♣/1♦, weak twos, preempts, a lighter 3rd/4th-seat major), the
  first response to every one-level opening (the 2/1 game force, the forcing
  1NT, single/limit/Jacoby-2NT/weak-jump raises, 1♠ over 1♥, four-card majors
  up the line over a minor), the 1NT structure (Stayman and Jacoby transfers
  with their opener completions), one round of opener's rebids (after a
  one-level new suit and the forcing 1NT), negative doubles with "system on
  over their double", and a defensive book (natural overcalls, takeout
  doubles, the 15–18 1NT overcall, simple advances, and a penalty-oriented
  defense to their 1NT). The public sub-builders `openings`, `major_responses`,
  `minor_responses`, `notrump_responses`, and `defense_to_suit` return `Rules`
  for reuse and testing, and `competition()` returns the `Competitive` book.
  Authored entirely from the existing vocabulary; no new infrastructure. A
  `two-over-one` example (`cargo run --example two-over-one`) bids out random
  boards end to end with both sides playing the system, seated via
  `Table::of_pairs`.
- `bidding::two_over_one`: the system is now a complete 2/1 card rather than
  a basic slice. New in this pass, each in its own submodule:
  - **2/1 game-force continuations** through the slam-try level: opener's
    rebids after every two-level response (jump rebid, raise, six-card
    rebid, 12–14/18–19 2NT, new suits up the line), responder's rebids
    (trump-setting 3M, second-suit raises, 3NT), opener's 4NT hook once
    trump is set, and an `Undisturbed`-guarded *game backstop* so no
    uncovered game-forcing continuation dies below game.
  - **Forcing 1NT continuations**: the three-card limit raise (1NT then
    3M), 2NT invites, weak six-card runouts, preference, and opener's
    accept/decline.
  - **Jacoby 2NT rebids**: side-suit shortness at the three level, a good
    five-card second suit at the four level, 3M (18+) / 3NT (15–17) with no
    shortness, 4M minimum; responder drives with 4NT.
  - **Splinters** (double jump = four trumps, 10–13, singleton/void),
    **inverted minors**, and **weak jump shifts** (6-card suit, 2–5 HCP),
    with opener continuations.
  - **The strong 2♣ structure**: 2♦ waiting, 2♥ double negative (0–3),
    natural five-card positives and the 2NT balanced positive; opener's
    natural rebids (2NT = 22–24, 3NT = 25–27) and responder continuations.
  - **The 2NT machinery** at every strength: three-level Stayman and Jacoby
    transfers over the 2NT opening *and* the 2♣–2x–2NT rebids ("system
    on"), plus quantitative 4NT raises over 1NT, 2NT, and the 18–19 2NT
    rebid, with opener's graded accept/decline.
  - **Weak-two continuations**: Ogust 2NT (min/max × bad/good suit, 3NT =
    solid), RONF preemptive raises, forcing new suits with opener's reply.
  - **RKCB 1430** below every major-suit trump agreement (after Jacoby
    rebids, splinters, game-force trump setting, and the 2♣ major raise):
    the 1430 answers, asker continuations with a documented 0/3–1/4
    ambiguity policy, the 5NT king ask, and grand-slam decisions.
  - **Fuller competition**: cue-bid (limit-plus) raises, preemptive jump
    raises, competitive raises, negative doubles over all four openings
    (system on over their double everywhere), weak jump shifts in
    competition, support doubles/redoubles (exactly three-card support),
    and opener's forced answer to a negative double.
  - **Two-suited defense**: Michaels cue-bids and the unusual 2NT with
    longer-suit advances and game jumps, plus responsive doubles after
    partner's takeout double and their raise.

  Still left for later passes: lebensohl and reopening actions in deeper
  competitive auctions, responder's natural rebids after `1m–1M–2m`, and
  minor-suit keycard.
- `bidding::constraint`: four new public primitives — `top_honors(suit,
  range)` (count of A/K/Q for suit quality), `stopper_in(suit)`,
  `partner_suit_is(suit)` (pins partner's last bid suit, where `support`
  only grades it), and `min_level_is(level, strain)` (the legality anchor
  for rules whose call sits at a dynamic level, such as cue bids).

### Changed

- **Breaking** (within the unreleased 2/1 card): raise meanings moved to the
  modern defaults. `1m–2m` is now the strong inverted raise (10+, forcing)
  and `1m–3m` the weak preemptive one; direct `1M–3M` limit raises promise
  four trumps, with the three-card limit raise routed through the forcing
  1NT.

- Reduced Clippy noise across bidding internals and tests: several small
  closure-coercion/context helpers are now `const fn`, builder-style
  constraint constructors gained `#[must_use]`, doc-code link formatting was
  cleaned up, and float assertions in tests were refactored to robust helper
  predicates instead of direct float equality.

- **Breaking:** `Partnership` is replaced by `Pair` + `Stance`, and book
  routing is now three-way via `Phase::of`. `Constructive` answers *only*
  strictly undisturbed auctions; competition over our openings moves out of
  the constructive trie into the new `Competitive` book (`two_over_one`'s
  negative doubles and system-on now live in its `competition()` builder).
  Assemble the three books with `Pair::new(family, constructive, competitive,
  defensive)` and bind once against the opponents' family —
  `pair.against(them)` returns the `Stance` that implements `System`; a
  `Pair` itself is authoring material, not a `System`. `two_over_one()` now
  returns the `Pair`.
- pons now requires `contract-bridge` 0.1.2 for the newly public
  `Auction::can_push`, the dry-run legality check behind `Table::next_call`.
  Until 0.1.2 is published, `Cargo.toml` carries a `[patch.crates-io]` entry
  pointing at the sibling checkout; drop it at release.

- **Breaking:** `bidding::trie::Classifier::classify` now takes
  `(Hand, &Context)` instead of
  `(Hand, RelativeVulnerability, CommonPrefixes)`. The context carries the
  vulnerability and (optionally) the common prefixes. Closure classifiers
  change from `|hand, vul| …` to `classifier(|hand, context| …)`.
- **Breaking:** the vulnerability-indexed `Forest` (`[Trie; 4]`, with `from_fn`
  and the `Index<RelativeVulnerability>` impls) and its `SeatClasses` mask are
  removed in favor of `bidding::book`. Vulnerability conditions live in
  constraints (`vulnerable()` / `they_vulnerable()`), seats are explicit leading
  passes in the book key, and seat-dependent strength is a constraint
  (`nth_seat` / `passed_hand`). Author a pair's notes with the role-aware
  books assembled into a `Pair` (see above) instead of a bare `Trie`, which
  stays as the low-level table-model escape hatch (and the way to express
  forcing passes).
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

### Fixed

- Seat-fan coverage gaps: responses and continuations now answer after
  4th-seat openings (leading-pass fan extended to three passes), and the
  defensive book answers when their opening arrives after leading passes —
  previously `[P, 1♦]` and kin were silently off-book.

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
