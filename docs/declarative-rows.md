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
| Constructive — ported | `openings.rs`, `weak_twos.rs`, `xyz.rs`, `nmf.rs`, the `notrump` and `rebids` module trees, `strong_two.rs`, and `responses.rs`, guarded by `row_package_invariants` in `american/tests.rs`. |
| Constructive — **not** ported | Two American files (`game_force.rs`, `raises.rs`) plus `dutch.rs`: 25 verb sites + 5 production `install_rkcb` calls, sequenced below. |
| RKCB | A row **producer**: `slam::rkcb_rows(prefix, trump) -> Vec<Entry>`. The same-signature `install_rkcb` shim remains for five production sites; three test-only callers in `slam/tests.rs` must also move before T1 deletes it. |
| Phase 1.5 (floating agreements) | **Cancelled, not deferred** — see below. |
| Phase 2 (cross-side assembly) | **Open**, restated below. Nothing exists: no `defense_vs`, no `competitive_vs`, no `Table::compose`. |
| Phase 3 (knob migration) | **Open**, restated below. Thread-locals untouched: 27 across `competition/`, 19 across `defense/`, 12 in `inference.rs`, 9 each across `notrump/` / `rebids/`. The by-agreement file split makes this *easier*: each agreement module's thread-locals are exactly the contents of its config struct. |

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

Everything that is *not* a row falls to the legacy slow path — the opaque
guards, the 1NT graft, and every unported constructive file. So **the fast
path's coverage is this campaign's port coverage.** The remaining imperative
constructive wiring is confined to `game_force.rs`, `raises.rs`, and `dutch.rs`.

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

Scoped 2026-08-06: the original tail was six American constructive files plus
`dutch.rs`; phases 2/3 remain untouched. N1–N5, R1, S1, P1, and G0 are
complete. The remaining sequence is G1, Z1, T1, D0, D1. One batch = one
commit = one inertness proof under the porting rule above. Site counts are the
reproducible greps — `insert_uncontested|insert_all_seats|fallback_all_seats`
per file, plus
`install_rkcb(` counted separately.

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
- `defense.rs:270` builds the 1NT-overcall graft through
  `notrump::register_one_nt(&mut Trie)`. Its signature remains load-bearing;
  its body is now `compile_into` over the 1NT packages.
- Five key sets derive from another table's `.rules()`
  (`rebids/forcing_notrump.rs:72`, `rebids/major_tails.rs:426`, `raises.rs:380`,
  `game_force.rs:378`, `:455`). Spell them
  as computed `entries` closures (the `nmf.rs:230` idiom), never as `expand`
  templates with a duplicated filter — the copy would drift from the
  knob-gated source table.
- Every port adds its package(s) to `row_package_invariants`
  (`american/tests.rs:161`). The list is hand-edited; a port not listed there is
  not gated.
- `smoke-default` and `render-book` hardwire the shipped `american()`, so a
  dutch port is unprovable until D0 builds the dutch twins. And `dutch_book()`
  overwrites american nodes by re-insert — legal across `compile_into` calls,
  a `group()` panic within one package — so dutch is its own package list,
  compiled after american's.
- Of the original 25 production `install_rkcb` sites, 20 now inline
  `rkcb_rows(prefix, trump)` and five remain in G1/Z1. Three additional
  test-only calls in `slam/tests.rs` are outside those anchors; T1 converts
  them before deleting the shim.

### Batches

| id | state | target | sites | gates to re-render under | notes |
| --- | --- | --- | --- | --- | --- |
| N1 | **Done** | historical `notrump.rs` base, Stayman, crawling | 22 + 2 rkcb | `stayman_cue_continuation`, `stayman_minor_slam_try`, `crawling_stayman` | |
| N2 | **Done** | historical `notrump.rs` transfer suite + invitational majors | 19 + 4 rkcb | invitational, transfer-GF/slam-try, six-card-invite gates | compound gates became named fns |
| N3 | **Done** | max overlays, 3♣ Puppet/European, both-majors 3♦, NT splinter | 20 | `stayman_both_majors`, `stayman_5card_max`, Puppet/European, `nt_splinter` | |
| N4 | **Done** | Texas, 2NT response, 2♠ response | 19 + 4 rkcb | `texas_slam_drive`, Puppet/European | |
| N5 | **Done** | `register_two_nt_and_rebids` | 10 | — | two computed-entry packages |
| R1 | **Done** | historical `rebids.rs`, seven register fns | 34 + 4 rkcb production; 1 test fixture | the ten rebid sweep arms | nine packages; two table-derived key sets |
| S1 | **Done** | `strong_two.rs` | 15 + 4 rkcb | `slam::minor_keycard` | ported at fan 3; corrected the stale 0–2 comment |
| P1 | **Done** | `responses.rs` | 9 + 2 rkcb | `major_choice_of_games`, `slam::minor_keycard` | three packages; seven inline tables hoisted |
| G0 | **Done** | `rows.rs`: builder-level fan = 2 | — | — | `Pattern::with_fan(2)`; no string syntax added |
| G1 | **Next** | `game_force.rs` | 8 + 2 rkcb | `game_backstop_enabled` | two backstop hatches; two table-derived key sets |
| Z1 | Open | `raises.rs` | 7 + 3 rkcb | game-try + limit-raise gates | one table-derived key set |
| T1 | Open | retire `install_rkcb` (`slam.rs`) | — | — | also convert its three test-only callers |
| D0 | Open | Dutch inertness harness | — | — | Dutch twins of smoke and render |
| D1 | Open | `dutch.rs::dutch_book()` | 10 | — | own package list, compiled after American's |

Completed: N1→N5, R1, S1, P1, G0. Remaining sequence:
G1→Z1→T1→D0→D1. The remaining anchors are exact: 8+2,
7+3, and Dutch 10+0 — 25 verb sites and five production RKCB calls in total.
Completed notrump/rebid locations in the table are historical; commit `1c0ef51`
subsequently split them into agreement modules.

### The batch recipe

1. Hoist inline `Rules` and compound gates to named fns — `Package`'s fields
   are bare `fn` pointers and capture nothing.
2. Spell the entries: `expand` where the domain is static, computed `entries`
   closures for table-derived key sets, `rkcb_rows` inline for keycard tails.
   **Table builders stay untouched** — that is what made the `weak_twos.rs`
   port (e7621f2) inert.
3. Package constructors plus a one-line `register()` calling `compile_into`;
   `notrump.rs` keeps `register_one_nt(&mut Trie)` for the graft.
4. List the package(s) in `row_package_invariants`.
5. `cargo fmt`, `cargo test --all-features`, `cargo +nightly clippy
   --all-targets --all-features -- -D warnings`, `RUSTDOCFLAGS="-D warnings"
   cargo doc --no-deps --all-features`.
6. Sha-compare `smoke-default` (20 000 boards, seed 1) and `render-book`
   against the parent commit. Extend the reusable `examples/tmp-rows-port`
   harness so every translated gate is exercised by at least one non-vacuous
   arm; its no-argument output is the arm list. Capture every arm before and
   after the batch, require all arms to have pairwise-distinct outputs, and
   byte-diff matching outputs. Keep this harness through T1; do not create a
   per-batch temporary example. D0 supplies the analogous Dutch dumps. Never
   straddle a behavioural commit.
7. CHANGELOG entry carrying the shas; propose the commit message.

**Stops.** A weight tie surfaced by the invariant probe stops the batch —
resolution touches the reading and may not be inert; escalate rather than
nudge. Any dump delta is a translation bug: fix it, never re-bless.

Delegation: Z1 and D1 remain mechanical re-spellings with this recipe as the
spec; G0, G1, T1, and D0 stay in the main loop.
