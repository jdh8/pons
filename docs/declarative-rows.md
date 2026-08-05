# The declarative rows layer — campaign ledger

Book assembly as **data**: a `Package` (name, knob gate, entries), a list of
`Entry` rows each tying an auction `Pattern` to one rule or one rebase, and one
fold — `rows::compile_into` — lowering them onto the existing `Trie` through the
existing verbs. The mega-functions that spelled wiring imperatively become
package lists.

**The grammar and its lowering table live in `src/bidding/rows.rs`'s module
doc.** Do not restate them here; if this file and that one disagree, that one is
right. **The per-batch log is `CHANGELOG.md`** (newest first, the campaign spans
roughly the "RKCB is a row producer" entry back to "Book assembly is becoming
declarative"). This file is the map: what is ported, what is not, and what the
two open phases are.

## Status

| Area | State |
| --- | --- |
| Contested — `competition()` | **Done.** Nothing but `compile_into` over 21 packages; zero hand-rolled wiring. |
| Contested — `defensive()` | **Done bar one site.** 22 packages, plus the 1NT-overcall systems-on graft, which is *permanently* imperative: `compile_into` writes rows, not a whole subtree. |
| Constructive — ported | `openings.rs`, `weak_twos.rs`, `xyz.rs`, `nmf.rs`, guarded by `row_package_invariants` in `american.rs`. |
| Constructive — **not** ported | `notrump.rs` (4,734 lines, ~92 imperative sites), `rebids.rs` (36), `strong_two.rs` (17), `responses.rs` / `game_force.rs` (9 each), `raises.rs` (8). All of `dutch/`. |
| RKCB | A row **producer**: `slam::rkcb_rows(prefix, trump) -> Vec<Entry>`, with `install_rkcb` kept as a same-signature shim for the ~25 call sites in unported files. |
| Phase 1.5 (floating agreements) | **Cancelled, not deferred** — see below. |
| Phase 2 (cross-side assembly) | **Open**, restated below. Nothing exists: no `defense_vs`, no `competitive_vs`, no `Table::compose`. |
| Phase 3 (knob migration) | **Open**, restated below. Thread-locals untouched: 27 in `competition.rs`, 19 in `defense.rs`, 12 in `inference.rs`, 9 each in `notrump.rs` / `rebids.rs`. |

**Escape hatches still open: ~20** `guarded(` / `classified(` sites (19 in
`competition.rs`, 1 in `defense.rs`), plus the 1NT graft. A `guarded` row carries
a hand-written `Guard` verbatim; a `classified` row a table computed at classify
time. Both are legal, both are opaque.

### Why 1.5 was cancelled

The floating-addendum design (`FloatGuard`: semantic "wherever trumps are
agreed" agreements lowered to root fallbacks) is **not owed.** The instinct floor
already implements it as `keycard_trump` / `keycard_ask_bid` under
`set_floor_rkcb`, default-on since M6.4 and disclosed via `Alert("floor:rkcb")`.
Building it in the book would shadow a *measured* floor package with
hand-authored rows. DOPI/ROPI/DEPO and the Kickback ladder are likewise floor
code, not book wiring.

## Why the layer is load-bearing

Not cosmetics. The bidding-engine recovery (stages 1–6, archived at
[docs/archive/bidding-performance-handoff.md](archive/bidding-performance-handoff.md))
**compiles row grammar.** Stage 3 keeps a `Pattern`/`Row` authoring ledger alive
through trie mutation, graft, merge and floor, and consumes it at
`Pair::against()`; stages 4–5 turn exact patterns into direct transitions and
first-call guards into dispatch.

Everything that is *not* a row falls to the legacy slow path — the ~20 opaque
guards, the 1NT graft, and every unported constructive file. So **the fast
path's coverage is this campaign's port coverage.** `notrump.rs` is the largest
single block of remaining slow path.

## The floor coupling (read before touching knobs)

The floor's **attachment point is unchanged**: `common::with_floor` writes a root
`Always` fallback into the mutable competitive and defensive tries, `Pair::against()`
finalizes around it, and the classifier stays opaque to `CompiledRuleRegistry`
and `AuthoringDecoder`.

The floor's **kind** changed when the configured net became the default.
`NeuralFloorBba` was a unit struct — knob-independent, order-independent.
`ConfiguredFloorBba(Config)` carries a convention card built by `american_card()`
from the *same* thread-locals that a `Package`'s `gate` and `entries` read at
`compile_into`. So `american()` is two knob readers producing one artifact,
joined only by being called in one expression — which its doc comment asserts and
the test `the_default_floor_reads_the_live_card` pins. `american_with_config`
cannot even do that much: a card claiming an agreement the rules do not play is a
misdisclosure to the net, and nothing checks it.

One consequence worth stating for scheduling: `with_floor` hardwires `instinct()`
on the constructive book regardless of the floor passed, so the configured floor
covers competitive + defensive only — and the entire remaining port tail is
constructive. The floor work and the port work do not block each other.

## Phase 2 — cross-side assembly (open)

The original framing was "invent a cross-side channel". That is wrong now: two
already exist, and **both default to *the opponents play us***.

| Channel | Consumer | Default |
| --- | --- | --- |
| `Context::their_system: Option<&Stance>` | the reading layer, `project_authored` | `Stance::prefixed_context` attaches self |
| `Context::config: Option<&Config>` | the net, `features_v4` | `Config::symmetric` |
| book selection | — | **does not exist; this is Phase 2** |

`Table::of_pairs` already seats two independently-built `Pair`s, so a mixed table
is expressible today; `examples/ab-kickback`'s `build()` is a working prototype of
the floor-side half.

So the goal is **one declared-opponent value feeding all three channels**, so
they cannot disagree. Key it on the opponents' `Card` — row-level via
`Card::row`, structural rather than an identity label, so the `Family`-deletion
doctrine holds at build time too. Do *not* build machinery to project their 1♣
opening rules; it duplicates a value the floor already consumes.

**This is a bidding change, and its A/B must split**, because a mixed table moves
the net's inputs and the book rows at once:

- **2a — floor only.** Mixed `Config`, books unchanged. Zero new code, measurable
  now.
- **2b — books.** `defense_vs` / `competitive_vs` on top, baselined on 2a.

Full [measurement.md](measurement.md) discipline, both DD brackets. First
consumer: the american-vs-dutch mixed table.

## Phase 3 — knob migration (open)

Per-book config structs replacing thread-locals, constructed `::from_ambient()`
at first so every call site and `bba-gen --ns-*` switch keeps working; harnesses
migrate one at a time; thread-locals retire last. Byte-identity at every stage.

**Terminal step: `Card::from(&config)`.** When the thread-locals go,
`american_card()` has no source. Migrate `the_default_floor_reads_the_live_card`
alongside it. Skipping this freezes every card at its default, and the only
symptom is that every future card-row A/B measures zero.

## The one porting rule

A port is **inert until proven inert**: seeded `examples/smoke-default` auction
dump and `render-book` output both diff empty across the port commit and its
parent. Knob defaults are not an argument — re-render under the knobs the ported
package gates. Diffs that straddle a behavioural commit (a floor swap, a shipped
convention) prove nothing; rebase the port off it.
