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
declarative"). This file is the map: what is ported and what the two open
phases are.

## Status

| Area | State |
| --- | --- |
| Contested — `competition()` | **Done.** Nothing but `compile_into` over 21 packages; zero hand-rolled wiring. |
| Contested — `defensive()` | **Done bar one site.** 22 packages, plus the 1NT-overcall systems-on graft, which is *permanently* imperative: `compile_into` writes rows, not a whole subtree. |
| Constructive — ported | The American `weak_twos.rs`, `xyz.rs`, `nmf.rs`, `strong_two.rs`, and the `openings`, `notrump`, `rebids`, `raises`, `responses`, `game_force` and `slam` module trees, plus both Dutch override packages. American and Dutch each have a `row_package_invariants` test. |
| RKCB | A row **producer**: `slam::rkcb_rows(prefix, trump) -> Vec<Entry>`. All 25 production callers and the three test fixtures use it directly; `install_rkcb` is retired. |
| Phase 1.5 (floating agreements) | **Cancelled, not deferred** — see below. |
| Phase 2 (cross-side assembly) | **2a built and REFUTED** 2026-08-07 (`--declare-opponents`, default off — the net's card block is near-diagonal, see below). **2b blocked** on the same retrain: no `defense_vs`, no `competitive_vs`, no `Table::compose`. |
| Phase 3 (knob migration) | **Open**, restated below. Thread-locals untouched: 27 across `competition/`, 19 across `defense/`, 12 in `inference.rs`, 9 each across `notrump/` / `rebids/`. The by-agreement file split makes this *easier*: each agreement module's thread-locals are exactly the contents of its config struct. |

**Escape hatches: 9, and the convertible set is empty.** A `guarded` row carries
a hand-written `Guard` verbatim; a `classified` row a table computed at classify
time. Both are legal, both are opaque, and the variable-row grammar retired the
thirteen that were templates in disguise. What is left is not a backlog — each
survivor is a shape a template cannot spell:

| Kind | Sites | Why it cannot be a template |
| --- | --- | --- |
| Rebase carriers | 3 (`systems_on_over_double`, the doubled-Stayman runout, the 2NT lebensohl reroute) | A rebase re-points a whole *subtree*; `expand` emits leaf nodes. The wildcard tail is the guard's native shape. |
| Wildcard tails | 4 (free-bid answers `4d″`, `4d‴`; two 2/1 game backstops) | The free-bid middle is an unconstrained `Bid(_)`; the game backstops cover any undisturbed tail. Enumerating them produces hundreds or thousands of mostly unreachable columns and lands every one on the rendered card. |
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
guards and the 1NT graft. So **the fast path's coverage is this campaign's port
coverage.** All constructive book assembly, including the Dutch overrides, is
now declarative.

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
covers competitive + defensive only. The completed port was constructive, and
did not change floor attachment.

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

- **2a — floor only.** Mixed `Config`, books unchanged. **Built and measured
  2026-08-07 — REFUTED at the current net; the knob stays, default off.**
- **2b — books.** `defense_vs` / `competitive_vs` on top, baselined on 2a.
  **Blocked**, see below.

### 2a measured: the card block is a near-diagonal input

`bba-gen --declare-opponents` (default off) replaces `Config::symmetric` with
`Config::new(&our_card, &theirs)`, where `theirs` is read off the opponents
themselves — `BbaOracle::card()` for an EPBot seat, `floor_card(name)` for a
pons `--their-floor`. Books are untouched; only the net's input moves. 2000
boards/arm/vul, seed 424242, paired:

| cell | `theirs` differs from ours | fired | plain DD | perfect defense |
| --- | --- | --- | --- | --- |
| **A** — vs BBA's real 2/1 card, vul none | **43 of 135 rows** | 31.4% | **−0.7015** ±0.1622 | **−1.1615** ±0.1950 |
| **A** — vul both | | 26.8% | **−0.7410** ±0.1923 | **−1.1075** ±0.2262 |
| **B** — american vs dutch, vul none | 1 row + the system one-hot | 1.4% | −0.0085 ±0.0345 | −0.0070 ±0.0418 |
| **B** — vul both | | 1.2% | +0.0155 ±0.0390 | +0.0240 ±0.0467 |

Cell A loses about **the whole BBA gap**, four-sigma, at both vuls and under
both brackets. It is not a plumbing bug: the card reads back 135 rows, all in
`{0, 1}`, and its 43 disagreements with `american_card()` reproduce
`probe-bba-conventions`' independent diff exactly.

The cause is corpus coverage. `docs/ai-bidder/configured-net.md` §"The cells"
draws every v4 cell from `{American, Dutch} × {kickback off, on}` — **four
pons-generated cards, pairwise differing in at most one row plus the base-system
one-hot.** Cell B is one of those trained cells and washes; cell A is a
20×-wider `theirs` block than anything in training, and the net degrades off the
manifold. So `theirs` is not the general "who is across the table" input its
name suggests — it is a near-diagonal one, and truthfulness is out of
distribution.

**Reopen after a retrain whose corpus carries BBA's own card as a `theirs`
cell**, not before; the harness is already in place and the re-measure is two
`--declare-opponents` arms.

But price the ceiling before paying for one. Cell B is a pure *label* test —
the opponents were a pons Dutch book in **both** arms, and only the declared
`theirs` moved, across the widest in-distribution variation that exists (the
corpus has exactly two base systems). It moved 1.2–1.4% of boards and scored a
wash. So within its trained range the card block is nearly inert; the cell-A
blowup is unregularized weights off-manifold, not a feature the net leans on.
A retrain would be buying a channel whose only honest measurement is zero.
The cheap next step is therefore **cell B at full A/B scale**, which needs no
retrain and no new corpus: if a truthful, in-distribution `theirs` is still a
wash at ~200k bd/arm/vul, the retrain is dead and so is 2a.

**This does not block 2b's reader half.** `their_system` does not touch the
card block — it feeds the deterministic reader, whose `Inferences` the net
consumes through a block that varies enormously in training, so a different
inference is not off-manifold the way a novel card bitvector is. What is
blocked is 2b *as the doc specs it above* — "one declared-opponent value
feeding all three channels" — because the third channel is the config.

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

Scoped 2026-08-06 and completed 2026-08-07: the tail was six American
constructive files plus `dutch.rs`; phases 2/3 remain untouched. N1–N5, R1,
S1, P1, G0, G1, Z1, T1, D0, and D1 are complete. One batch = one commit = one
inertness proof under the porting rule above. The original site counts came
from `insert_uncontested|insert_all_seats|fallback_all_seats` per file, plus
`install_rkcb(` counted separately.

**Conversion policy.** A site that resists transparent rows converts only when
the conversion prunes no arms (parts 1–2 of the conversion gate above); a
pruning conversion is *out of this campaign* — hatch it and mark it here as a
candidate for a later measured campaign. No batch was a bidding change. D1's
ten Dutch verb sites were exact `insert_uncontested` nodes. G1's two non-exact
backstops landed as hatches.

**What the batches rest on** (verified at 7facdd3):

- The two `Undisturbed` game backstops are wildcard tails;
  `classified(Pattern::guarded(…))` now spells them — no new entry kind.
  **The hatch census is 9.**
- They fan `0..=2` leading passes. Before G0, `Pattern.fan` was only ever 0 or
  3, so G0 added a builder-level fan setter. No auction-string syntax grew.
- `defense.rs:270` builds the 1NT-overcall graft through
  `notrump::register_one_nt(&mut Trie)`. Its signature remains load-bearing;
  its body is now `compile_into` over the 1NT packages.
- Five key sets derive from another table's `.rules()`. All five are now
  computed `entries` closures in `rebids/`, `game_force.rs`, and `raises.rs`,
  never `expand` templates with duplicated filters that could drift from their
  knob-gated source tables.
- Every American port adds its package(s) to `row_package_invariants` in
  `american/tests.rs`; Dutch has the same invariant over its own two-package
  list in `dutch/tests.rs`. The lists are hand-edited; an unlisted port is not
  gated.
- `smoke-dutch` and `render-dutch-book` now provide D1's deterministic parent
  proof over the shipped `dutch()` and public floorless `dutch_book()`.
  `dutch_book()` overwrites american nodes by re-insert — legal across
  `compile_into` calls, a `group()` panic within one package — so dutch is its
  own package list, compiled after american's.
- All 25 original production `install_rkcb` sites now inline
  `rkcb_rows(prefix, trump)`. The three test fixtures compile the same producer
  through `compile_entries`; the shim is gone.

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
| G1 | **Done** | `game_force.rs` | 8 + 2 rkcb | `opener_third_enabled`, `second_suit_agreement`, `game_backstop_enabled` | four packages; two backstop hatches; two table-derived key sets |
| Z1 | **Done** | `raises.rs` | 7 + 3 rkcb | game-try + limit-raise gates | three packages; one table-derived key set |
| T1 | **Done** | retire `install_rkcb` (`slam.rs`) | — | — | three test fixtures use the row producer directly |
| D0 | **Done** | Dutch inertness harness | — | — | deterministic, non-vacuous Dutch twins of smoke and render |
| D1 | **Done** | `dutch.rs::dutch_book()` | 10 | — | two-package list compiled after American's; 8 replacements + 9 additions |

Completed: N1→N5, R1, S1, P1, G0, G1, Z1, T1, D0, D1. Phase 1 of the rows
campaign is complete. The exact-node imperative helpers and `install_rkcb`
have no callers and no longer exist.
All file locations in the table are historical. Commit `1c0ef51` split
`rebids`, `notrump`, `competition` and `defense` into agreement modules, and a
follow-up campaign finished the job for `raises`, `over_our_notrump_calls`
(which is gone, promoted to four flat `competition/` siblings),
`weak_two_defense`, `game_force`, `advance_rich`, `slam`, `responses` and
`openings`. A file named here as `<name>.rs` is now usually a `<name>/` module
tree; the parent keeps its path, so every reference above still resolves, but a
*line* anchor into one of them does not.

That campaign is the precondition for Phase 3 below: each agreement module's
thread-locals are now exactly the contents of its config struct. It also grew
the knob sweep from 31 arms to 67 — the extra arms are almost all
default-**off** packages, whose rows no unarmed dump can see at all, plus one
per variant of the three multi-arm enums (`TwoOverOneGate`, `WeakTwoEval`,
`NotrumpShape`) that a lifted combinator `match`es on.

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
6. Sha-compare the batch's smoke and book render against its parent. The
   American batches through T1 also used the reusable `tmp-rows-port` sweep:
   every translated gate had a non-vacuous arm, all arm outputs were pairwise
   distinct, and matching before/after outputs byte-diffed empty. T1 retired
   that harness after its final 31-arm proof. D0 supplied the analogous Dutch
   dumps for D1. Never straddle a behavioural commit.
7. CHANGELOG entry carrying the shas; propose the commit message.

**Stops.** A weight tie surfaced by the invariant probe stops the batch —
resolution touches the reading and may not be inert; escalate rather than
nudge. Any dump delta is a translation bug: fix it, never re-bless.

The port campaign is complete. Phase 2 cross-side assembly and Phase 3 knob
migration remain open above.
