# The Dutch system — campaign ledger

Dutch is a **natural 2/1 built around a wide, non-forcing 1♣** — a "lawyer's
Polish Club" that naturalises the Polish 1♣: Polish constructiveness, but
natural and less restricted. It is a **champion candidate**. We copy
[`american()`](../src/bidding/american.rs) and apply the Dutch diff one
measurable phase at a time; Dutch promotes to the shipped default only if it
measures stronger, at which point `american()` demotes to the ablation baseline
(the WBridge5-ships-a-modded-French model).

Read [docs/bidding-architecture.md](bidding-architecture.md) first (the
book/floor/inference layer cake) and [docs/measurement.md](measurement.md) (no
bidding change ships without an A/B).

## Target system

Openings:

| Call | Meaning |
| --- | --- |
| **1♣** | NF, 2+♣, **≤4♦** (no 5+♦), no 5-card major, 11–23 HCP, Rule of 20 — the wide catch-all; soaks up every 4-diamond hand except the 4441, plus all strong hands lacking the 2♣ shape |
| **1♦** | **5+♦, or exactly 4=4=4=1** (singleton club) — no other 4-diamond hand; no 5-card major; 11–23 HCP, Rule of 20. So 1♦ ⟹ 4+♦, never 3. Do **not** reuse american's `prefers_diamonds` (it opens 1♦ on 3♦ and on 4-diamond non-4441 hands). Encoded as `no5cM & (len(♦,5..) \| len(♣,..=1))`. |
| **1M** | 5+ cards, 10–20 HCP, Rule of 20 |
| **2♣!** | strong artificial: 21–23 with a 5-card major or 6-card minor, or any 24+ |
| **2♦** | Multi |
| **2♥ / 2♠** | Muiderberg |
| **2NT** | UNT (both minors) |
| 3-level | preempts as american (strength TBR) |

Responses to the wide 1♣ (the load-bearing convention):

| Call | Meaning |
| --- | --- |
| **1♣–P** | 0–5, 4–5♣ |
| **1♣–1♦!** | artificial catch-all relay (R), may be weak |
| **1♣–2♣** | INV+, 5+♣, no 4-card major |
| **1♣–2♦** | FG, 5+♦, no 4-card major |

## Decisions

1. **Layout = reuse + override.** `dutch()` is a thin factory that reuses
   american's shared module `register()`s (1NT, slam, XYZ, raises, most rebids,
   most competition/defense) and swaps in Dutch modules only for the divergent
   parts. No fork of the ~19k-line tree.
2. **Family = `Family::NATURAL`.** Opponents defend Dutch as natural for now.
3. **2-level openings = lift the shapes, replace weak twos.** Multi/Muiderberg/UNT
   replace american's weak-two 2♦/2♥/2♠ and strong 2NT; strong balanced 20–23
   moves into the wide 1♣. Reuse the existing `woolsey_multi` /
   `woolsey_muiderberg` / `unusual_2nt` shape builders (lifted out of
   1NT-defense gating), re-banded to weak-two strength. (In this codebase
   "Woolsey" is a *defense to their 1NT*, not opening preempts — only the shape
   builders are reused.)

## The central risk — reader/floor are american-tuned

`inference.rs` (the reader) and `instinct.rs` (the floor) are tied to american's
meanings. Two mechanisms:

- **Rule projection** (`set_alert_reading`, default-on) makes every alerted,
  DSL-authored artificial call self-decode by replaying its own `project` fold.
  So Dutch's strong 2♣, Multi 2♦, Muiderberg 2M, UNT 2NT, and the 1♣–1♦ relay
  decode **for free** once authored with projecting combinators + `.alert(...)`.
- The **hardcoded american reading** decodes the natural, unalerted openings.
  Dutch's natural openings differ by range/shape (1♣ 11–23 2+♣; 1♦ 5+/4414; 1M
  10–20), so they get a **soft misread** (biased HCP band), not a phantom-suit
  disaster. Strategy: rely on projection for the artificial calls, accept soft
  natural misreads, **measure the cost**, fix only the worst offenders (Phase 4).

`instinct()` is keyless and only fires off-book, so **completeness of authored
Dutch nodes** (both sides of every convention) is what keeps the floor from
bidding an artificial opening as natural. Dutch keeps american's 15–17 1NT, so
the floor's transfer-completion still holds.

## Phase ledger

| Phase | Scope | Status |
| --- | --- | --- |
| 0 | Scaffold `dutch()`, re-export, 0.000 baseline | **DONE** |
| 1 | Dutch openings: wide 1♣, 1♦ 5+/4441, 1M 10–20, strong 2♣ | **DONE** (code; A/B pending) |
| 2 | Wide-1♣ responses: 1♦ relay, 2♣ INV+, 2♦ FG + opener's rebids | pending |
| 3 | 2-level openings (Multi/Muiderberg/UNT) + strong-2♣ tree | pending |
| 4 | Reader/floor reconciliation + divergent-opening competitive book | pending |
| 5 | Iterate to champion vs BBA/BEN; promote if it wins | pending |

Each phase gates on a paired-seed A/B via `examples/bba-gen` (dutch arm vs
american arm), dual-scored (`ns_score_pd` + `ns_score_contract`), fresh
`SEED_BASE`, run sequentially under `scripts/idle-run.sh`. Preemptive phases
(3) are read knowing DD is blind to obstruction — lean on the sd-lead / PD
bracket, not plain DD alone.

### Phase 1 notes

`bare_dutch()` takes a full `bare_american()` pair and **overwrites only the
opening node** (`Trie::insert_arc` replaces the classifier at the opening key)
with `dutch::openings::dutch_openings()`; every american continuation is reused
verbatim. Widening was minimal: `insert_uncontested` and `with_instinct_floor`
in `american.rs` → `pub(in crate::bidding)`. Openings live in
`src/bidding/dutch/openings.rs`.

Design choices to validate in the A/B (each defensible from the spec, cheap to
flip):

- **Rule of 20 is a hard gate** (`hcp(band) & rule_of_20()`), so a flat
  sub-R20 minimum passes out — e.g. a 4-3-3-3 twelve-count (12+4+3 = 19).
- **1NT is balanced-only 15–17** (american's wide-6322 5422/6322 shapes open a
  minor instead). A deliberate Dutch choice; wide6322 is a later within-Dutch
  A/B if wanted.
- **No 3rd/4th-seat light (9-count) major openers** — american has them; the
  Dutch spec caps majors at a Rule-of-20 10-count. Watch the passed-out-seat
  boards.
- **Strong balanced 20–21 still opens 2NT** (Phase 1 placeholder); only 22–23
  balanced reaches the wide 1♣ until Phase 3 turns 2NT into UNT.

Guards: `dutch_artificial_calls_are_alerted` (inference.rs) walks the Dutch
constructive book; `dutch::tests::opening_partition` pins the six load-bearing
opening cases (incl. the wide 1♣ hosting a 23-count). `dutch()` is re-exported
as `pons::dutch`.

### Phase 0 notes

Scaffolded `dutch()` as a sibling factory (re-exported `pons::dutch`). The
reuse + override seam opened in Phase 1; widening american internals is done
only as each part diverges.
