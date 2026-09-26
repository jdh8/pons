# The floor junk-action rail series

**Status (2026-09-26, anchor `c3bb94a7`).** Three rails shipped default-on in
one week — 3NT-pull, 2NT-double, unusual-4NT — cashing ≈ +0.026 plain /
+0.035 PD IMPs/board combined. Two priced candidates remain in the queue;
after they resolve, the census is stale by construction and must be re-run
before authoring anything else. Companion ranking of the other campaign
candidates: [next-steps.md](next-steps.md).

## What the series is

The 2026-09-25 priced decompose of the **shipping arm** (`american()`, v6
learned floor, snapshot `ab-results/anchor/2026-09-20-c3bb94a7/`) overturned
the old reading of the defensive-floor buckets: the floor's fault is junk
**action**, not silence. With our side silent and theirs having bid twice,
"we `X` / BBA passes" is −11.6k plain / −15.2k PD IMPs over 4,081 boards and
"we bid / BBA passes" −6.7k / −12.5k over 5,513 (worst rows all 4–9 HCP),
while every "we pass / BBA acts" slice is **PD-positive**. Pool at the census:
≈ −0.045 plain / −0.068 PD per board across
`Defensive / floor / round-1+2` and `Competitive / floor / round-2`.

A **rail** is a deterministic veto on the learned floor's candidate menu:

- One `InstinctProfile` field (bool, doc comment carries the ship numbers),
  a structural gate function in [instinct.rs](../src/bidding/instinct.rs)
  (`their_2nt_gate`, `their_3nt_gate`), and a mask in the
  [neural_floor.rs](../src/bidding/neural_floor.rs) rail ladder. The net
  keeps the judgement middle; the rail only removes calls. `Pass` always
  survives (the finite-catch-all invariant).
- Rails constrain the **learned floor only**. `instinct()` never sees them,
  so the anchor's decompose series stays comparable (bba-gap rule 7: rail
  work is visible on the shipping arm and the paired diff, never the
  instinct arm).
- **Parametric or nothing**: a gate is context predicates (their last bid
  class, our side silent, a suit-length condition) — never an exact auction.
  A candidate that needs an exact sequence is retrain fodder, not a rail
  (see *When to stop*).

Why the net can't learn this itself: `features_v6` has no alert column and
sets the we-bid-this-strain bit for artificial calls too, so a conventional
call and a natural one are the same input. The rail is the output-side patch;
the input-side fix is a retrain
([new-suit-veto.md](ai-bidder/new-suit-veto.md) has the full argument).

## How to find tasks — the priced census

1. **Snapshot.** Use the latest anchor (`ab-results/anchor/<date>-<sha>/`);
   if stale, re-anchor per the [bba-gap runbook](bba-gap-campaign.md#runbook).
2. **Decompose the shipping arm**: `bba-decompose --our-floor american` over
   both vulnerability dirs. Replay must read **100.00% / 0 mismatches** or
   the buckets are invalid (bba-gap rule 1).
3. **Slice `boards.jsonl`.** Rows carry `provenance`, `phase`, `family`,
   `our_call`, `their_call`, `points` (HCP), `swing_plain`, `swing_pd`,
   `hand`, and the `(seed, board)` join key back to the shard dumps (for the
   auction and the our-side-silent test). Filter `provenance == "floor"`,
   keep rows where **we act and the reference passes**, group by auction
   prefix × our call class, sum both swings, rank by **PD** (the honest
   scorer for junk action — the losses are doubling-shaped).
   Scratch scripts in the scratchpad are fine; the verdict rows land here,
   the scripts don't.
4. **Qualify a row** before it becomes a task:
   - the reference passes on (essentially) every board in the slice;
   - the hands share a structural signature (a low HCP band, a short suit)
     expressible as a parametric gate;
   - total ≥ ~0.5k PD IMPs over the 409.6k-board census — below that even
     the isolated 204.8k/vul A/B is resolving noise.
5. **Re-census after every ship.** Shipped rails mask rows (the 3NT-pull
   rail hid most of the `1♣ - 1♠ - 2NT - 3NT` bucket before the 4NT arm was
   priced separately); a pre-ship table over-states what remains.

## Task queue

Priced at `c3bb94a7`, in order:

| # | slice | boards | plain | PD | state |
| --- | --- | ---: | ---: | ---: | --- |
| R4 | fourth-round junk bid after `1NT - 2♣ …` | 113 | −0.6k | −1.1k | open — trace, then author |
| R5 | junk `2♠` cue over `1♠ - 2♦` | 446 | −0.4k | −0.7k | open — competitive-floor row; check the cue's PDI/tag context before gating |
| R6 | re-run the census (post-R4/R5, or immediately if either refutes) | — | — | — | mandatory before any further authoring |

Done (ledger; full cells in each `InstinctProfile` doc comment and the
CHANGELOG):

| rail | knob | shipped | plain | PD | fired | seed |
| --- | --- | --- | --- | --- | ---: | --- |
| 3NT suit-pull veto | `their_3nt_pull_veto` | 2026-09-25 | +0.0081/+0.0102 | +0.0152/+0.0175 | 0.12–0.13% | 1790283934 |
| 2NT-double veto | `their_2nt_double_veto` | 2026-09-25 `bf7d7cce` | +0.0143/+0.0177 | +0.0159/+0.0199 | 0.35% | 1790318091 |
| unusual-4NT veto | `their_3nt_unusual_veto` | 2026-09-25 `56ecccdd` | +0.0035/+0.0045 | +0.0041/+0.0053 | 0.03–0.04% | 1790324543 |
| *(predecessor)* new-suit veto | `new_suit_veto` | refuted in aggregate, **off** | — | — | — | see [new-suit-veto.md](ai-bidder/new-suit-veto.md) |

Every shipped rail was a win in **every** cell with single-dummy alike — the
vein's hit rate so far is 3/3 on the narrow gates and 0/1 on the broad one.
Narrow beats broad here.

## Per-rail runbook

1. **Trace.** Join the slice's rows back to the dumps, confirm the reference
   passes throughout, read the HCP band and a handful of worst boards. If
   the reference sometimes acts too, the row is judgement, not junk — skip.
2. **Author, default off.** Profile field + gate + mask + test in
   [neural_floor/tests.rs](../src/bidding/neural_floor/tests.rs) + `bba-gen`
   flag + runner cloned from
   [ab-2nt-double-veto.sh](../scripts/ab-2nt-double-veto.sh) (the gate reads
   `theirs` when the rail only fires on their openings). Watch the clap trap:
   the `bba-gen` default must equal the crate default
   (`default_args_arm_the_shipped_system` pins it, bba-gap rule 1).
3. **Measure.** 204,800 bd/vul, fresh `SEED_BASE`, `scripts/idle-run.sh`,
   arms sequential, no rebuilds in flight. Verdict from
   `diff.on.vs.plain.{none,both}.{plain,pd}.txt` + `sd.*` per the
   [measurement.md](measurement.md) decision table.
4. **Ship on a win.** Flip `InstinctProfile::default()`, invert the flag to a
   `--no-ns-*` control, cells into the doc comment + CHANGELOG, row here and
   in [bba-gap-campaign.md](bba-gap-campaign.md) open-work item 4. Full gate
   list including `cd web && cargo test` — the knob crosses the public API.
5. **Refuted or wash?** Knob stays, default off (house rule), row updated
   here with the cells. Two in a row → *When to stop*, criterion 2.

## When to stop

The series ends the first time any of these holds:

1. **The census dries.** No qualifying slice ≥ ~0.5k PD remains. The two
   queued rows are already near this floor; expect the census, not the
   authoring, to end the series.
2. **Two consecutive wash/loss verdicts.** The hit rate has collapsed —
   re-census instead of authoring a third; if the fresh census ranks nothing
   new, stop.
3. **The next candidate isn't parametric.** An exact-auction gate is a node
   per sequence wearing a rail costume. Record the row as retrain evidence
   and stop rather than author it.
4. **The residual flips to silence.** Rails only delete actions. When the
   remaining floor loss is "we pass / reference acts" (PD-positive today) or
   wall-bound obstruction, this series has no material — that's the floor
   backlog and the retrain, not a rail.
5. **A matched retrain lands.** Every rail is scaffolding over a v6 input
   blindness. *(2026-09-26: the retrain is built — `features_v8`, M32 labels
   transplanted, no relabel, [ai-bidder/features-v8.md](ai-bidder/features-v8.md) —
   measured **suspect** (plain win, PD erases it), stays opt-in; the rails stand and the net's parameters stay fixed for R4/R5. Its worst boards name a new rail candidate: our silent side acting over their 4NT.)* At the next matched policy/evaluator retrain, re-run each
   shipped rail's A/B against the new net (the runners make this cheap); a
   rail whose win vanished gets its default flipped off. Rails must not
   accumulate past the net that needed them.

Stopping is a verdict, not a failure: the series was designed to cash a
bounded pool (−0.068 PD/board at `c3bb94a7`), and the three ships plus the
queue account for the large rows already.
