# Authored calls speak for themselves — handoff

**Written 2026-08-15**, out of the N2 forensic
([one-notrump-competitive.md](one-notrump-competitive.md) §N2).
Found at one node; the defect is the whole book's. This file states the
principle, maps where the code violates it, prices it, and lays out the
program to fix `pons` as a whole — with N2 as the local testbed every phase
is checked on first.

Prerequisites: [reading-drift-handoff.md](reading-drift-handoff.md) (the three
reading regimes; its step 4 is this campaign), [dnf-migration.md](dnf-migration.md)
(what happens when a *true* reading meets a frozen net — C1, C2, F1, F2b),
[reader-retirement.md](reader-retirement.md), [ai-bidder/sampled-projection.md](ai-bidder/sampled-projection.md).

## The principle (jdh8, 2026-08-15)

> A bid being natural does not mean that other people can magically know its
> strength and length ranges. "Alert" is not for all authored nodes. Authored
> nodes should speak for themselves whether alerted or not.

Three separate things are conflated today:

| concept | what it is for | who consumes it |
| --- | --- | --- |
| **alert** | disclosure — "this call is artificial" | `.bbsa` cards, `artificial_calls_are_alerted`, opponents' rights |
| **reading** | what the call *promises* — the union of its live rules | sampler, authored gates, instinct floor, the nets |
| **the natural walk** | a guess from auction *shape* when there is no rule | unauthored calls, undeclared opponents |

The engine uses the first as the switch for the second, and lets the third
override the second wherever the first is absent. Target state: **the reading
of an authored call is its rules, two-sided, always; the walk fills in only
where nothing is authored; the alert decides nothing about reading.**

## The evidence, in one probe

`probe-decision "Q93.K43.AKJT.Q42" "1NT 2♠ 2NT - 3♣ - 3♦ -"` — opener to act
after responder's weak Lebensohl relay and sign-off, both authored:

| what responder's calls say | rule | what opener reads |
| --- | --- | --- |
| `2NT` relay | `points(..=8) & (5+ suit not theirs) & hcp(6..)`, alerted | `hcp 6..37`, `points 6..37`, suits `0..13` |
| `3♦` sign-off | `min_level_is(3,♦) & len(♦,5..)`, natural | nothing added — ♦ `0..13` |
| decision | — | floor (`depth 0, fallback 0`) bids **`3NT` 1.400** over Pass 0 |

Anchor cost of that one node: opener bids `3NT` over the weak `3♦` on 16 of 18
boards, −52 plain / −125 PD. `PROBE_SCOPE=all` (`ReadingScope::All`) gives ♦
`5..13` back; the ceiling stays `..37` and **the floor still bids `3NT`**. The
natural `1NT 2♥ 2♠ -` reads as *nothing at all* under the default.

## Where the principle is violated — the code map

### 1. The scope gate: reading is switched by the alert

[projection.rs `authored_effect`](../src/bidding/inference/projection.rs):

```rust
let decode = match scope {
    ReadingScope::None    => false,
    ReadingScope::Alerted => alerted,   // shipped default
    ReadingScope::All     => true,
};
```

`ReadingProfile::scope` ([knobs.rs](../src/bidding/inference/knobs.rs)) is
`Alerted` by default. `All` is built (`bba-gen --ns-reading-scope all`,
`probe-reading-sound --ns-natural-reading`), *intersects* the rule with the
walk (does not set the suppression bit), and is **unmeasured** — deferred by
the reading-drift ledger to "its own campaign". This is it.

Size: `src/bidding/{american,dutch}` hold ~1,400 `.rule(` calls and ~350
`.alert(` tags. Roughly **a thousand authored rules project nothing** under
the default; each is a regime-2 row of the reading-drift table.

### 2. Strength projects floor-only — every made-bid ceiling is lost

[constraint.rs](../src/bidding/constraint.rs) `Points::project` /
`Hcp::project` write `floor..=37` ("floor only, matching every hand-written
reader; the ceiling returns in `project_band`, widened by
`hcp_ceiling_slack`"). `project_band` — the two-sided one — is used **only for
the pass reading** (`project_pass`).

The rationale is stale. The rule evaluates `point_count(hand) ≤ 8` on the same
deterministic scale the envelope's `strength.points` records, so the ceiling is
**exact on its own axis**; the other axis follows with the known slack
(`hcp ≤ points ≤ hcp + upgrade_ceiling`), which `hcp_band` /
`upgrade_ceiling` already compute. The comment predates the two-axis
`Strength`. Fuzzy strength (`fuzzy_fifths`) is off; when on, widen by the fuzz
exactly as `hcp_ceiling_slack` does — a knob, not a reason to drop the bound.

Size: **244 explicit ceilings** (`points(..=N)`, `hcp(..=N)`, `lo..=hi`
bands) in authored rules, 169 of them two-sided bands. Every one of them reads
as `..37` on a made bid, alerted or not. Every weak sign-off, every limited
raise authored as a band, every "to play" call reads *unlimited* to the floor.
This is the **binding** defect at N2 (`All` alone leaves the `3NT` in place)
and the one the reading-drift handoff filed as "the other three known leaks,
first bullet".

### 3. The walk overrides, and the projection only intersects

[read.rs](../src/bidding/inference/read.rs): after a 1NT opening the walk
*blankets* every suit bid on the opening side except a lane's first
three-level call (`nt_blanket`, `over_one_notrump`, `stayman_artificial`,
`nt_splinter_artificial`, `nt_structure_artificial` — one predicate per
convention that ever contradicted it). Right for the uncontested transfer
structure; wrong for the sohl sign-off, and it is why cause 1 alone cannot
save the sign-off's length. Under `All` the rule is *intersected with* the
walk's guess so the walk's bookkeeping (natural-suit lanes, agreed fits, cue
detection) survives — which is why "wrong walk ∧ sound rule = **empty box**"
(reading-drift step 4's warning). The bookkeeping is derived from the bid's
*face*, not from what the bid promised.

### 4. Who consumes the reading — why this is a bidding change

- **The nets.** [features.rs `push_inference`](../src/bidding/features.rs):
  per hidden seat, exactly `4 × (len.min, len.max) + (points.min, points.max)`.
  Ceilings and lengths flow straight into the v5 policy floor and the
  evaluator (`trick_estimates`). Both were fit on Alerted-scope, floor-only
  readings. `net_points` is the existing precedent for keeping a frozen net
  on-distribution while the reader changes underneath it.
- **The sampler.** `Inferences::admits` tests lengths and `points` — a
  ceiling that arrives *removes* hands the sampler was dealing outside the
  stated band (C2's 249 rejections). Unlike C1 this is information-carrying,
  so a retrain can be earned on it.
- **Authored gates** read `.min` (`support_floor`, `fit_sum_game`,
  `keycard_trump`) and, once ceilings exist, will read `.max`
  (`combined_hcp`, the NT-slam milestones).
- **The instinct floor** consults the auction's interpretation
  (`set_inference_aware`) — the deterministic floor moves too.

So per reading-drift's second mechanism: **every phase below is an A/B, read
from the decision table, never a soundness argument.**

## Target design

1. **Reading = the rules, always.** For every authored call — ours, and theirs
   when a book is declared — the reading is the union of the call's live
   rules under the bidder's at-the-time context (`authored_effect` already
   does this; drop the `Alerted` gate). Sound by construction: with a finite
   catch-all at every table (iron rule) a hand makes call C only through a
   C-rule it satisfies, so the union is a superset of the truth — loose,
   never wrong. Alerts keep their disclosure job untouched.
2. **Two-sided strength.** Forward projection of `points`/`hcp` becomes the
   band (`project_band`'s arithmetic), exact on the rule's own axis, slacked
   on the other; fuzz-widened when a fuzzy gauge is on. Passes already do
   this — made bids catch up.
3. **Substitute, don't intersect.** An authored call *owns* its reading and
   sets the walk's suppression bit; the walk's lane bookkeeping is derived
   from the projected envelope (a suit projected `4+`/`5+` counts as bid
   naturally; a projected fit is an agreed fit) instead of from the bid's
   face. Then `nt_blanket` and its per-convention exceptions become dead code
   for authored calls, and the walk shrinks to genuinely unauthored auctions
   and undeclared opponents.
4. **Negative inference** (later). Argmax means "made C" also means "no
   higher-weighted rule at that node held": fold `project_complement` of the
   higher rules into C's reading. Exact for argmax; it is what turns "passed
   the relay rather than transferring" into `< 9 points` with no reader.
5. **Frozen nets get a compat view, then a retrain.** Separate the *reading*
   (truth, served to the sampler, gates and instinct floor) from the
   *encoding* served to nets that were fit on the old reading:
   `features::legacy_view` reconstructs the training-time hull (Alerted-scope,
   floor-only) at every node — the `net_points` fold generalised — so phases
   1-3 can be measured with the nets held on-distribution. Then
   `features_v6` = the honest reading, regen + retrain by the F2b recipe
   (`dump-evaluator`, `dump-teacher` with the new profile; held-out gate;
   knob-matched twin selected per call), flip, and delete the view.

## The soundness gate

Everything ships through the behavioural sweep, not the static one:

- `readings_admit_the_bidder` ([inference/tests.rs](../src/bidding/inference/tests.rs))
  — replay the *bidder* over seeded hands at a node, require partner's reading
  to admit the hand, both regimes. Rows land with the repair that greens them.
- `probe-reading-sound` — the ambient exclusion rate (partner **2.36%** at
  last run; "lebensohl continuations" was already on its named-but-unrepaired
  list). Re-run per phase; a phase that *raises* it has a walk defect it just
  exposed (empty boxes are the diagnostic), and that defect is cleared before
  the A/B, not after.

## Program

Each phase: one knob (or two-binary), `bba-gen --filter-1nt` on N2 first
(the local testbed — decompose with `probe-1nt-interference --bucket 2♠
--responses 8`, the `2NT`/`3♦` rows and the opener-`3NT` count are the
target), then the whole-book anchor (`scripts/anchor.sh` /
`scripts/ab-reading-drift.sh`'s two-binary protocol), plain + PD, both vuls,
verdict from [measurement.md](measurement.md)'s table. Non-loss ships a
soundness correction; a loss traces its worst boards before any conclusion.

| # | Phase | Mechanism | Knob / protocol | Gate | Status |
| --- | --- | --- | --- | --- | --- |
| 0 | **N2a** — opener passes the relay's minor sign-off | book node (`{relay} 3♦ -` → `Pass`, a `landy_signoff_answer` twin) shadows the floor; independent of every reading phase | knobless; measure on `--filter-1nt` | standard | queued — cheapest, fixes 16/18 regardless |
| 1 | **Strength ceilings** | `Points::project` / `Hcp::project` → band | `reading.strength_ceilings` (default off until measured); `bba-gen --ns-strength-ceilings`; **two arms**: raw (nets see the new `.max`) and `legacy_view` (nets held) | admits sweep + `probe-reading-sound` unchanged-or-better; A/B | **first** — binding at N2; C2's re-open trigger ("a two-sided forward projection") is met by it |
| 2 | **`ReadingScope::All`** as default | built; drop the alert gate | `--ns-reading-scope all`; same two-arm protocol | clear the empty-box worklist *before* measuring (`probe-reading-sound --ns-natural-reading`, bucketed) | queued behind 1 |
| 3 | **Substitute, don't intersect** | authored calls set suppression; walk bookkeeping from the projection; retire `nt_blanket` & co. for authored calls | two-binary (a refactor, not a knob) | byte-identity where projection ⊇ walk is *not* expected — this moves readings by design; A/B | queued behind 2 |
| 4 | Negative inference | fold `project_complement` of higher rules | knob | admits sweep (must stay green — it tightens) | later |
| 5 | **features_v6 + retrain** | honest reading in, `legacy_view` and `net_points` fold out | F2b recipe (dump, held-out gate, twin, flip) | held-out NLL/MAE ≥ shipped; A/B of the flip | after 1-3 land |

Why this order: 1 before 2 because at N2 the missing ceiling, not the missing
length, is what the floor spends; 2 before 3 because 3 is the cleanup that 2
makes possible (once every authored call projects, the walk's exceptions have
nothing left to protect); 5 last because it is the only phase that cannot be
undone with a knob.

## The N2 testbed — exact recipe

```bash
# the node, before/after any phase
cargo run --release --example probe-decision -- "Q93.K43.AKJT.Q42" "1NT 2♠ 2NT - 3♣ - 3♦ -" both
PROBE_SCOPE=all cargo run --release --example probe-decision -- "Q93.K43.AKJT.Q42" "1NT 2♠ 2NT - 3♣ - 3♦ -" both

# the lane, off an anchor arm (seconds, deal-keyed DD cache)
cargo run --release --features serde --example probe-1nt-interference -- \
    ab-results/anchor/<arm>/american-none --dd-cache ab-results/anchor/dd-cache.json \
    --bucket "2♠" --responses 8            # our-response / BBA-response / why-passed tables
cargo run --release --features serde --example probe-1nt-interference -- \
    ab-results/anchor/<arm>/american-none --dd-cache ab-results/anchor/dd-cache.json \
    --bucket "2♠" --show 40 --next "2NT"   # the relay boards; count "3♦ - 3NT"

# the isolated A/B for a phase, N2-enriched
scripts/idle-run.sh scripts/bba-gen-parallel.sh ab-results/<exp>/<arm> 19200 -v none --filter-1nt <knob flag>
```

Success at N2, per phase: opener's `3NT` over `2NT - 3♣ - 3♦` gone (0/18);
the `2NT` relay row no longer the lane's worst per board; `1NT 2♥ 2♠ -` reads
♠ `5..13` with a ceiling.

## Traps recorded elsewhere that apply here

- **A true reading can lose 4/4** (dnf-migration C1: hull *endpoints* moved
  without mass; nets non-invariant to the encoding + gates reading `.min`
  against thresholds tuned without it). Ceilings *do* move mass, so they are
  earnable — but the frozen nets still see displaced endpoints. That is what
  the `legacy_view` arm is for: it separates "the reading is right" from "the
  net was fit on the wrong one" in one run.
- **Wrong-seat trap** (dnf-migration F2b′): project under the bidder's
  at-the-time context; context-relative legs (`support(..)`, `min_level_is`)
  re-target if projected under the reader's context. `authored_effect` already
  does this; keep it when substituting for the walk.
- **⊤ is sometimes the only sound projection**: negations, `pred` net gates,
  RKCB answers, `Fallback::classify` logit rewrites (Gladiator's stolen relay).
  Substitution must fall back to the walk for a call whose projection is ⊤ on
  every axis, or the walk's bookkeeping loses a genuinely natural bid.
- **A reading knob is a bidding knob** (reading-drift §2): the instinct floor
  is inference-aware; every phase moves calls at every floor node.
- **Alerts and `.bbsa` cards** are untouched by this campaign by construction
  — reading no longer depends on them. Do not "fix" a reading by alerting a
  natural call (the completion-alert precedent was for a *puppet*, which is
  artificial).
- **Train/serve skew** (dnf-migration F1 side finding): the policy net serves
  on prefixed features and was trained on bare-context ones. Phase 5 is the
  place to close it, not before.
- **The strip** (`systems_on_overcall_strip`) is a claim that two structures
  coincide; phase 3 must keep it scoped per RHO call exactly as
  `gladiator_keeps_the_strip_where_it_has_no_structure` pins.

## Decisions for jdh8

1. Phase 1 knob name and default: `reading.strength_ceilings`, off until
   measured — or ship-on-non-loss like the reading-drift batch (they are
   soundness corrections)?
2. `legacy_view` as a first-class arm (recommended: it is the only way to read
   a loss as "net" vs "reading"), or go straight to raw and retrain on a loss?
3. Phase 3's walk-bookkeeping-from-projection: refactor in place, or a
   parallel `walk_v2` behind the two-binary protocol until it wins?
4. Whether N2a lands now (one node, own A/B, no reading dependency) or waits
   to be the *control* that shows phases 1-2 fix it without a node.

## Ledger

| date | item | status |
| --- | --- | --- |
| 2026-08-15 | N2 forensic; principle stated; this handoff | written |
