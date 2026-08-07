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
| Phase 2 (cross-side assembly) | **Both halves built and REFUTED** 2026-08-07, both default off. 2a — the net's card channel (`--declare-opponents`). A follow-up run the same day priced a **one-bit off-manifold** `theirs` perturbation at −0.01 plain / −0.02 PD per board; it was *meant* to be the in-distribution cell-B test and is not one, so the retrain reopen path stands and **cell B at scale is still owed**. 2b — the reader's (`Stance::with_opponents`, `--declare-their-book`). See below. The book-selection third leg (`defense_vs`, `competitive_vs`, `Table::compose`) is **not built and not wanted**: `defensive_vs` already existed, had zero production consumers, and went out with `Family`. |
| Phase 3 (knob migration) | **Open; its gate has not actually run** (the 2026-08-07 attempt perturbed frozen card rows — see Phase 2). Not on the critical path either way; the knob field table is the piece worth doing regardless. It does not depend on Phase 2 — restated below. Thread-locals untouched: **221 across 74 files** — the earlier figures here (27 `competition/`, 19 `defense/`, 12 `inference.rs`, 9 each `notrump/` / `rebids/`) were a stale undercount by ~3× (`defense/` is 53, `notrump/` 23). Reproduce with `rg -c --no-heading '^\s+static\s+[A-Z0-9_]+:' -g '*.rs' src` and the same over `'^pub fn set_'`; both total 221, so statics and public setters match 1:1. The by-agreement file split makes this *easier*: each agreement module's thread-locals are exactly the contents of its config struct. |

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

One consequence worth stating for scheduling: `with_floor` puts `instinct()` on
the constructive book regardless of the floor passed, so the configured floor
covers competitive + defensive only. The completed port was constructive, and
did not change floor attachment.

`instinct()` is itself a knob reader — `relocating_now()` picks the kickback
answer table over the plain one, `hcp()` captures `strength_dial()` — so the
ladder is a *third* artifact that has to be built under the same knob state as
the book and the card. It used to be a process-wide `LazyLock` in
`neural_floor`, frozen at the first forced classification anywhere, while
`with_floor` gave the constructive book a fresh one; the two floors of a single
`Pair` could therefore disagree. Since 2026-08-07 `with_floor` builds one
`Arc<Rules>` and hands it to both, and `ConfiguredFloorBba::new` takes it. The
scheduling point stands: **anything Phase 3 threads a config into must reach
`instinct()` as well as `american_book()` and `american_card()`** — three
readers, one knob state, and nothing but call-site discipline joining them until
the config struct exists.

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
- **2b — the reader.** `Context::their_system` pointed at a declared opponent
  instead of at ourselves. **Built and measured 2026-08-07 — REFUTED, default
  off.** Not baselined on 2a: the two channels are independent, and running
  them together would confound a near-diagonal net input with a reading.

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

### A one-bit `theirs` perturbation costs −0.01/−0.02 — but it is *not* cell B

> **Read the caveat before the table.** This run was designed as "cell B at full
> A/B scale" and **it is not that**. It perturbs one card row, but not one of the
> rows the corpus varies, so it measures the off-manifold penalty at 1 bit rather
> than the value of an honest declaration. Cell B at scale is still unrun.

Both sides are pons `american`, and the opponents really do play one agreement
differently (`--their-ns "--no-ns-garbage-stayman"`, then
`--no-ns-major-game-tries`), which moves exactly one card row. Three arms, their
book byte-identical across all three so their side reads us the same way and
every IMP is ours:

| arm | what our net is told they play |
| --- | --- |
| `symmetric` | our own card (today's default) |
| `wrong` | a card differing on the *other* experiment's row |
| `truth` | the row they actually differ on |

`wrong` is the arm the two-arm 2a design could not spend: without it, a `truth`
result cannot separate "reading them **correctly** helps" from "reading them as
**anything other than ourselves** helps". 204 800 bd/arm/vul, both vuls, fresh
seed per experiment (1786103953 / 1786103961), `scripts/ab-declared-agreement.sh`,
dual scoring, ~4.4% of boards divergent — the channel is live, not inert.

| experiment | arm | vul | plain DD | perfect defense |
| --- | --- | --- | --- | --- |
| garbage stayman | truth | none | −0.0003 ±0.0051 | **−0.0252** ±0.0065 |
| | truth | both | −0.0062 ±0.0062 | **−0.0328** ±0.0076 |
| | wrong | none | **−0.0115** ±0.0051 | **−0.0119** ±0.0065 |
| | wrong | both | **−0.0106** ±0.0062 | **−0.0120** ±0.0077 |
| major game tries | truth | none | **−0.0124** ±0.0052 | **−0.0125** ±0.0065 |
| | truth | both | **−0.0105** ±0.0064 | **−0.0130** ±0.0078 |
| | wrong | none | +0.0028 ±0.0052 | **−0.0206** ±0.0065 |
| | wrong | both | **+0.0070** ±0.0063 | **−0.0195** ±0.0077 |

Fourteen of sixteen cells are negative and twelve are CI-clear negative. Every
`truth` cell is ≤ 0 on plain and CI-clear negative under PD. The two positives
are one arm's plain half, whose PD half is −0.020. `truth` does not consistently
beat `wrong` — it wins on plain and loses under PD in the garbage experiment,
loses on both in the game-tries one.

#### Why this is not the in-distribution test it was built to be

**Neither row varies in the corpus.** `dump-teacher` arms exactly one knob
(`set_rkcb_variant`) and takes `--system american|dutch`, so across all eight
ordered cells the card moves in **three coordinates only**: the base-system
one-hot, `1D opening with 5 cards`, and `Kickback 1430`. `diff
cards/American.bbsa cards/Dutch.bbsa` is two lines. `Garbage Stayman` and
`Two way game tries` are frozen at `1` in every training card on both sides.

So flipping one of them is cell A's failure mode at one bit instead of
forty-three, not a walk along the manifold. Their weights sit near
initialisation, and the measurement says what unregularized weights predict:
*perturbing a frozen `theirs` coordinate costs ≈ −0.01 plain / −0.02 PD per
board.* That the `wrong` control loses the same amount is the same statement —
it also flips a frozen coordinate — so the two arms carry no information the net
could rank, and their near-tie is **not** evidence that declaring is worthless.

This is therefore **evidence for the corpus-coverage explanation, not against
it**, and it prices what a retrain would be buying back. The reopen path in the
section above stands.

**Second defect, found the same day and independent of the manifold problem:**
`"Two way game tries"` on our card is `major_game_tries()`, but that BBA row
names an *artificial* relay scheme and ours is a natural long-suit try plus the
`3M` re-raise — a gap the 21GF ledger already lists (row 124, "add (Batch 3)").
So the game-tries experiment's `truth` arm declared a row that was never true
for either side. That experiment is void on its own terms; fix the row before
reusing it.

**What is still owed is the original prescription:** cell B — American vs Dutch
— at ~200k bd/arm/vul. `1D opening with 5 cards` is a genuinely trained
coordinate, so that arm alone tests an honest, in-distribution declaration. The
harness for it now exists (`--their-floor dutch --declare-opponents`), and 0c
made the Dutch seat's own config truthful, which it was not before.

*Repro:* `PER_SHARD=6400 scripts/idle-run.sh scripts/ab-declared-agreement.sh
ab-results/declared-agreement`.

### 2b measured: an honest reading is worth about zero

The reader's channel was a genuine `Context` change, because `their_system` was
doing two jobs. Five of its eight consumers resolve the *reader's own* prior
calls (the `decoded_own` walk, the `systems_on_overcall_strip` re-key, the
compiled-rule lookups); one resolves the opponents' alerted calls; two —
the pass walk and the probed overlay — do both, per index parity. So the field
split into `own_system` (always ours) and `their_system` (declared, else ours),
one builder `Context::with_system(ours)` setting both from
[`Stance::opponents`]. `Stance::with_opponents(them)` is the only way to make
them differ; the incremental reader (`AuthoringStepCache::prepare`) declines to
serve a declared opponent and falls back to the full walk, which routes by
side already.

Measured with `scripts/ab-declared-book.sh`: our american floor against a pons
dutch book (`--their-floor dutch`) in both arms, `--declare-their-book` the
only difference, so the dutch side reads us identically either way and every
IMP is ours. 204800 boards/arm/vul, seed 1786091527:

| vul | fired | plain DD | perfect defense |
| --- | --- | --- | --- |
| none | 1.91% | **+0.0038** ±0.0035 | −0.0019 ±0.0043 |
| both | 1.75% | **−0.0050** ±0.0043 | **−0.0120** ±0.0053 |
| pooled | | −0.0006 | **−0.0070** |

Plain DD a wash, perfect defense a loss — the decision table's "do not ship"
row, and the mirror image of the shippable pattern. It is not a coverage
artifact the way 2a was: a Dutch opponent is exactly the cell the reader was
built for, and the reading it produces is *correct*.

The mechanism reads clean off the divergent boards. Dutch's 1♣ is wide and
non-forcing; read as our own strong 1♣ we stay out, read honestly we come in
(`- - 1♣ - 1♥ 2NT` where the control arm passes). Competing more on a truthful
picture is right at equal non-vulnerability — the one cell that gains — and
wrong at vulnerable, where the extra auctions run past what the books author.
So the honest summary is that **the reading was never the binding constraint**:
we were guessing their system wrong and it cost about nothing, because the
calls the misreading affects are ones where our own continuations are thin.

Kept as an opt-in constructor per the house rule. The default system is
byte-identical — `--their-floor american --declare-their-book` reproduces the
no-flag dump board for board, which is the inertness gate for this channel.

## Phase 3 — knob migration (open)

Per-book config structs replacing thread-locals, constructed `::from_ambient()`
at first so every call site and `bba-gen --ns-*` switch keeps working; harnesses
migrate one at a time; thread-locals retire last. Byte-identity at every stage.

**Terminal step: `Card::from(&config)`.** When the thread-locals go,
`american_card()` has no source. Migrate `the_default_floor_reads_the_live_card`
alongside it. Skipping this freezes every card at its default, and the only
symptom is that every future card-row A/B measures zero.

### It does not depend on Phase 2

The whole Phase-2 surface is knob-free — `Config` / `Config::new` /
`Config::symmetric` / `encode_card` ([features.rs](../src/bidding/features.rs)),
`Card::row` / `foreign_card` ([card.rs](../src/bidding/card.rs)),
`BbaOracle::card()`, `ConfiguredFloorBba`, `Stance::with_opponents`, and
`Context::{own_system, their_system, config}`. Exactly one function in that path
reads a thread-local, `american_row()`, and that is Phase 3's own terminal step.
Phase 3 can start whenever.

### Two justifications that do not survive contact

Recorded so nobody re-derives them:

1. **Phase 3 cannot widen Phase 2a's cell B.** `dutch_book()` sets no knobs at
   all (`rg 'set_' src/bidding/dutch.rs src/bidding/dutch/` is empty), so a
   `DutchConfig::from_ambient()` would be byte-identical to the American one.
   The single-row american/dutch card diff is *structural* and pinned by
   `card/tests.rs`'s `dutch_differs_from_american_in_the_diamond_opening`. The
   real limit is that the Dutch book has authored exactly one schema-nameable
   divergence; authoring Multi 2♦ and the Polish two-suiters (the Dutch
   campaign's own Phase 3) moves four rows for zero knob work.
2. **Phase 3 does not unblock the mixed-knob cell either.** `ab-kickback`'s
   `build(arm, opponent)` already hands the net two independently-carded arms
   through a set → snapshot → restore transaction, and `Config::new` is public
   and validates nothing. What blocked `bba-gen` was a CLI guard, since
   narrowed — see the `--their-ns` / `--declare-as` entry in the changelog.

So the payoff is **structural, not measured**: no ambient state, a total and
cheap card read, generated CLI/UI/doc surfaces that cannot drift, and the
thread-local hazard class gone. The hazards are real and have cost twice —
the `LADDER` freeze above, and the off-shape card rows that silently lied.

### The gate has not run yet

Because the payoff is structural, the migration was gated on the one thing that
could still have made it a *bidding* win: the declaration channel it exists to
serve. The run built for that gate (above) turned out to perturb frozen card
coordinates rather than trained ones, so **it does not decide the gate either
way** — it prices the off-manifold penalty at one bit and leaves the channel's
value unmeasured. The gate is cell B at scale, and it is still owed.

Either way the migration is not urgent, because nothing about it is on the
critical path of a measured win. What is worth doing on its own terms,
independent of the gate:

- **the knob field table** — one row per knob generating the config structs,
  `bba-gen`'s clap surface, `web`'s `SETTINGS` and this repo's option index, so
  the four cannot drift. That drift is a live defect class with its own CI job
  (`.github/workflows/rust.yml`'s `web`) and its own abort mode (stale `--ns-*`
  flags killing scripts). It needs no threading and no measurement.
- **the hazard repairs**, which are cheap and independent — the two above were
  found and fixed while auditing for this campaign.

The full call-site migration (three config structs threaded to ~1500 sites,
`set_*` deleted, `Card::from(&config)`) is **not scheduled**. Reopen it if the
declaration channel is ever revived, or if the ambient state causes a third
defect.

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
