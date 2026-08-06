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
| Constructive — ported | `openings.rs`, `weak_twos.rs`, `xyz.rs`, `nmf.rs`, guarded by `row_package_invariants` in `american/tests.rs`. |
| Constructive — **not** ported | Six American files + `dutch.rs`: 174 verb sites + 25 `install_rkcb`, batched and sequenced in [the port checklist](#port-checklist) below. |
| RKCB | A row **producer**: `slam::rkcb_rows(prefix, trump) -> Vec<Entry>`, with `install_rkcb` kept as a same-signature shim for the ~25 call sites in unported files. |
| Phase 1.5 (floating agreements) | **Cancelled, not deferred** — see below. |
| Phase 2 (cross-side assembly) | **Open**, restated below. Nothing exists: no `defense_vs`, no `competitive_vs`, no `Table::compose`. |
| Phase 3 (knob migration) | **Open**, restated below. Thread-locals untouched: 27 in `competition.rs`, 19 in `defense.rs`, 12 in `inference.rs`, 9 each in `notrump.rs` / `rebids.rs`. |

**Escape hatches: 7, and the convertible set is empty.** A `guarded` row carries
a hand-written `Guard` verbatim; a `classified` row a table computed at classify
time. Both are legal, both are opaque, and the variable-row grammar retired the
thirteen that were templates in disguise. What is left is not a backlog — each
survivor is a shape a template cannot spell:

| Kind | Sites | Why it cannot be a template |
| --- | --- | --- |
| Rebase carriers | 3 (`systems_on_over_double`, the doubled-Stayman runout, the 2NT lebensohl reroute) | A rebase re-points a whole *subtree*; `expand` emits leaf nodes. The wildcard tail is the guard's native shape. |
| Wildcard tails | 2 (free-bid answers `4d″`, `4d‴`) | The middle call is an unconstrained `Bid(_)`. Enumerating it costs 640 and several thousand columns respectively, nearly all unreachable, and every one would land on the rendered card. |
| Logit transplants | 2 (stolen Stayman, gladiator advance) | They read *another table's* logits at classify time. That is not a rule table, so `Rules` cannot express it. |

Plus the 1NT graft, permanently imperative for the same subtree reason.

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

### And the rule's one exception, paid for in C1

A re-spelling of exact rows is inert. A **`guarded` → exact conversion that
prunes arms is a bidding change**, even though nothing about the intended
semantics moved. The guard answered from a table built for every overcall; the
expansion builds a table per column, so arms that were dead in a given auction
disappear — which *tightens the projection*, which feeds the configured floor's
features, which moves calls **downstream** of the converted node. C1 moved 558
of 20 000 boards with zero divergence at the node itself.

So the gate for a conversion is three-part, and the render diff being non-empty
does not decide anything:

1. **Eval equivalence** — the retired wiring kept as a `#[cfg(test)]` oracle and
   probed over a superset of the guard's auction space (`Option<Logits>` equality
   catches over- *and* under-expansion). Note that a guarded table which
   *rejects* a hand is re-found massless on fall-through, while an exact node
   rejects to the floor; normalize massless to unanswered and say so.
2. **First divergence never at the converted node** — anything at the node is a
   translation bug; anything downstream is the reading channel above.
3. **A full [measurement.md](measurement.md) A/B** when arms were pruned. C6 and
   C7 pruned none and their dumps came back byte-identical; C6's A/B then
   measured 0 divergent boards in 819 200, which is what byte-identity predicts.

## Port checklist

Scoped 2026-08-06: the six American constructive files plus `dutch/`; phases
2/3 untouched. One batch = one commit = one inertness proof under the porting
rule above. Site counts are the reproducible greps —
`insert_uncontested|insert_all_seats|fallback_all_seats` per file, plus
`install_rkcb(` counted separately — the old status row's hand counts drifted
±2 against them.

**Conversion policy.** A site that resists transparent rows converts only when
the conversion prunes no arms (parts 1–2 of the conversion gate above); a
pruning conversion is *out of this campaign* — hatch it and mark it here as a
candidate for a later measured campaign. No batch is a bidding change. Today
the policy is dormant: every verb site in the tail is an exact
`insert_uncontested` node except the two backstops below.

**What the batches rest on** (verified at 7facdd3):

- The two `Undisturbed` game backstops (`game_force.rs:437`, `:476`) are
  wildcard tails; `classified(Pattern::guarded(…))` spells them — no new entry
  kind. **Hatch census 7 → 9 when G1 lands.**
- They fan `0..=2` leading passes, and `Pattern.fan` is only ever 0 or 3, so
  G0 adds a builder-level fan setter. No auction-string syntax grows.
- `defense.rs:5844` builds the 1NT-overcall graft through
  `notrump::register_one_nt(&mut Trie)`. The port keeps that signature; the
  body becomes `compile_into` over the 1NT packages.
- Five key sets derive from another table's `.rules()` (`rebids.rs:690`,
  `rebids.rs:1524`, `raises.rs:380`, `game_force.rs:378`, `:455`). Spell them
  as computed `entries` closures (the `nmf.rs:230` idiom), never as `expand`
  templates with a duplicated filter — the copy would drift from the
  knob-gated source table.
- Every port adds its `package()` to `row_package_invariants`
  (`american/tests.rs:161`). The list is hand-edited; a port not listed there is
  not gated.
- `smoke-default` and `render-book` hardwire the shipped `american()`, so a
  dutch port is unprovable until D0 builds the dutch twins. And `dutch_book()`
  overwrites american nodes by re-insert — legal across `compile_into` calls,
  a `group()` panic within one package — so dutch is its own package list,
  compiled after american's.
- The 25 `install_rkcb` sites become inline `rkcb_rows(prefix, trump)`
  entries batch by batch; T1 deletes the shim when the last one goes.

### Batches

| id | target | sites | gates to re-render under | notes |
| --- | --- | --- | --- | --- |
| N1 | `notrump.rs` 3096–3227: base nodes + Stayman continuations | ≈23 | `stayman_cue_continuation`, `stayman_minor_slam_try` | |
| N2 | 3228–3429: crawling, invitational 5-4, six-card invite + transfer slam tries + GF majors | ≈24 | `crawling_stayman`, `invitational_5card_majors`, the seven gates at 3300–3416 | compound gates become named fns |
| N3 | 3430–3534: max overlays, 3♣ Puppet/European, both-majors 3♦, NT splinter | ≈20 | `stayman_both_majors`, `stayman_5card_max`, `puppet`, `nt_splinter` | |
| N4 | 3535–3612: Texas, 2NT response, 2♠ response | ≈23 | `texas_slam_drive`, `puppet` (minor structures) | |
| N5 | 3614–3716: `register_two_nt_and_rebids` | 10 | — | two loops over literal prefixes |
| R1 | `rebids.rs`, 7 register fns | 34 + 4 rkcb production; 1 test fixture | `meckstroth`, `meckstroth && MECKSTROTH_MINOR_JUMPS`, `forcing_nt_two_suiter`, `balanced_1nt_rebid`, `opener_extras_ladder`, `opener_major_jump_rebid`, `major_rebid_tails`, `major_rebid_tails && fourth_suit_forcing`, `nt_invite_hcp`, `responses::up_the_line` | may split the Meckstroth ladder out; 2 table-derived key sets |
| S1 | `strong_two.rs` | 15 + 4 rkcb | `slam::minor_keycard` | line 258's doc comment claims a 0–2 fan but `insert_uncontested` fans 0–3: port verbatim at 3, fix the comment |
| P1 | `responses.rs` | 9 + 2 rkcb | splinter / inverted-minor gates | hoist the inline `Rules` at 823–939 to named builders first |
| G0 | `rows.rs`: builder-level fan = 2 | — | — | the grammar grows with its consumer, G1 |
| G1 | `game_force.rs` | 8 + 2 rkcb | `game_backstop_enabled` | the two backstops become hatches; 2 table-derived key sets |
| Z1 | `raises.rs` | 7 + 3 rkcb | game-try + limit-raise gates | 1 table-derived key set |
| T1 | retire `install_rkcb` (`slam.rs:826`); flip the status rows above | — | — | after Z1 |
| D0 | dutch inertness harness: dutch twins of the smoke dump and the render | — | — | keep the rayon/thread-local discipline `smoke-default`'s doc states |
| D1 | `dutch.rs` `dutch_book()` | 10 | — | own package list, compiled after american's |

Sequence: N1→N5, R1, S1, P1, G0, G1, Z1, T1, D0, D1. Batch boundaries inside
`notrump.rs` and R1's split may flex at execution; the counts keep drift
visible. The `notrump.rs` per-batch counts include that batch's rkcb sites;
the per-file anchors are exact (90+10, 34+4 production plus 1 test fixture,
15+4, 9+2, 8+2, 7+3, dutch 10+0).

### The batch recipe

1. Hoist inline `Rules` and compound gates to named fns — `Package`'s fields
   are bare `fn` pointers and capture nothing.
2. Spell the entries: `expand` where the domain is static, computed `entries`
   closures for table-derived key sets, `rkcb_rows` inline for keycard tails.
   **Table builders stay untouched** — that is what made the `weak_twos.rs`
   port (e7621f2) inert.
3. `package()` plus a one-line `register()` calling `compile_into`;
   `notrump.rs` keeps `register_one_nt(&mut Trie)` for the graft.
4. List the package(s) in `row_package_invariants`.
5. `cargo fmt`, `cargo test --all-features`, `cargo +nightly clippy
   --all-targets --all-features -- -D warnings`, `RUSTDOCFLAGS="-D warnings"
   cargo doc --no-deps --all-features`.
6. The porting rule above: sha-compare `smoke-default` (20 000 boards, seed 1)
   and `render-book` stdout against the parent commit, both unchanged; then
   re-render under every gate in the batch's column (throwaway `tmp-rows-*`
   example, deleted after blessing). Never straddle a behavioural commit.
7. CHANGELOG entry carrying the shas; propose the commit message.

**Stops.** A weight tie surfaced by the invariant probe stops the batch —
resolution touches the reading and may not be inert; escalate rather than
nudge. Any dump delta is a translation bug: fix it, never re-bless.

Delegation: N1–N5, R1, S1, P1, Z1, D1 are mechanical re-spellings with this
recipe as the spec; G0, G1, T1, and D0 stay in the main loop.
