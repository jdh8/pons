# features_v9 — the shape distribution in place of the length hull

**Status (2026-10-24).** Built; the K = 8 retrain under ⊤ readings and its
four-cell gate against the shipped v6 floor are the first step of
[../shrinkage-plan.md](../shrinkage-plan.md) "Next: what lap 2 owes". Served
only through `--our-floor american-file` (no embedded net at this width);
`american()` still ships v6. Pipeline: `scripts/feature-retrain.sh`.

## 1. What v9 is

`features_v6` hands the net each seat's announced box as **hull endpoints**
(`min/13`, `max/13` per suit, `push_inference_v6`), and that encoding is not
invariant to information-preserving re-representation: `♥5..13` and `♥5..8`
admit the same hands yet the max column moves 1.00 → 0.62. Every
hull-tightening chop has therefore had to buy a retrain before it could be
judged on merit, and the shrinkage probe's κ-shrunk boxes are such a chop
served live, at endpoint values the net saw rarely, in exactly the disjunctive
positions (Or walls, rule competition) where one hull is the loosest summary
(the plan's "The encoding is the data structure that bit").

v9 = [v8](features-v8.md) with each seat's eight length endpoints swapped for
the `push_shape_dist` block the evaluator ablation built
([evaluator-net.md](evaluator-net.md)): the per-suit length distribution the
seat's announced **union** of boxes describes, conditioned on the cards the
deciding hand holds. A box that excludes no hands moves no column, and a
disjunction (a Multi's two majors) reaches the net as a bimodal marginal
instead of a `0..13` hull. Every other v8 column survives in place.

| block | start | len | from |
| --- | ---: | ---: | --- |
| hand | 0 | 10 | v6 |
| context | 10 | 36 | v6 |
| 4 seats × (shape 65, points 2, support 8) | 46 | 300 | shape: `LEN_SHAPE` (mean 4, sd 4, histogram 4 × 14, log-mass 1); points and support: v6 verbatim |
| vulnerability | 346 | 2 | v6 (was 118) |
| compact config | 348 | 56 | v6 |
| artificial | 404 | 12 | v8 |
| **total** | | **416** | |

Seat order is v6's (me, LHO, partner, RHO). The three hidden seats condition
on the 39 cards this hand does not hold, as `features_eval_v4` does; the
**own seat** conditions on the whole deck (`Unseen::whole_deck`, `C(13, k)`
per suit): its announced reading describes this hand to a table that sees
none of it, and the probe shrinks that reading too, so leaving the own seat
on endpoints would leave one hull channel open.

Two choices to flag. The block is the 65-column superset with the histogram,
not the 9-column Gaussian `features_eval_v4` ships: the evaluator ablation
priced the histogram at +0.0008 NLL over the Gaussian (barely past the seed
spread) on a mostly unimodal reading, while the policy floor's case is the
union, where mean and sd flatten two modes into one wide suit. The swap is
one line each (`push_shape_gauss`, `LEN_SHAPE_GAUSS`; 192 wide). And the
`points` endpoints stay endpoints, as in the evaluator: `points` couples to
shape only through `upgrade`, and the distributional twin on the strength
axis (`push_hcp_gauss`) is a separate, unmeasured bump.

Code: `features_v9` / `push_inference_v9` in `src/bidding/features.rs`;
`dump-teacher --feature-version 9` (requires `--configured`); `9` in the
trainer's supported versions; serving through `ConfiguredFloorV6::new_mean`,
whose blobs' length names `(input, hidden)` (`neural::blob_shape`) and so the
extractor. Tests: `features_v9_swaps_the_length_hull_for_the_shape_reading`,
`features_v9_top_reading_is_the_shape_prior`,
`v9_seat_block_is_invariant_to_the_sum_closure`, `splice_key_offsets_hold`,
`blob_shape_is_read_off_its_length`.

## 2. Labels: the transplant, keyed per version

The M32 labels ride [features-v8.md §2](features-v8.md)'s row-aligned splice
unchanged, with one fix the plan's review found (D2): `scripts/splice-labels.py`
keyed the reading-free key's vulnerability pair at the v6 offset (`118`), so a
v9 re-walk would have keyed on reading columns, matched nothing, and silently
kept every teacher one-hot. The offset now comes from each chunk's own
`feature_version` (`VUL_OFFSET = {6: 118, 8: 118, 9: 346}`, pinned by
`features::tests::splice_key_offsets_hold`; a version missing there is a
refusal), and the driver asserts the transplanted total across the 20 shards
at 424,070 ± 2% — the count the 2026-09-26 and lap-1 `--any-identity` splices
reached (lap 1: 422,746).

## 3. Pipeline

`NAME=v9 FEATURE_VERSION=9 scripts/feature-retrain.sh` (the A4 lap-1 driver,
checked in and parametrized): the fleet's 188 chunk windows re-walked at HEAD
through v9 by 20 `relabel-worker.sh` workers (`LAYOUTS=0`, no double dummy),
the splice with the count assertion, upload, eight draws of the shipped
recipe on dl02, **one trainer at a time** — a 416-wide corpus is 11.3 GB of
features and the trainer holds the stems and their concatenation together
while loading (≈ 2 × 13 GB peak) against ≈ 39 GB spare — then
`fold-constant-inputs.py` in scan mode, and the `NETS=` line for the runner.
The probed variant is the same script with
`NAME=v9-probed WALK_EXTRA="--ns-probe 100000 --ns-probe-vacuous"`.

The gate, pre-registered in the plan (review S1, S2): `ab-floor-file.sh` at
`VULS="none ns ew both"`, K = 8 v9 vs the shipped K = 8 v6 on paired deals,
**non-inferior on both scorers at every cell** (lower bound above −δ,
δ = 0.002) with the paired IMP intervals deciding and `ruler-tv.py` a
diagnostic read at the same deals and count. A win ships nothing by itself:
v9's job is to be the floor A4's four arms are re-run under.

## 4. Verdict

**Fails the gate (2026-10-24).** Against the shipped K = 8 v6, PD is a loss
at all four cells (−0.020 to −0.028 IMPs/board) and plain DD a loss at `ew`
(−0.019). Against a reseeded v6 K = 8 of the same recipe, seeds and trainer
(the control that removes the reseed cost), plain DD is flat in every cell
and PD a loss in every cell (−0.022 to −0.031): the shape-distribution
block, as the 65-column histogram, costs doubling-priced IMPs. The loss is
diffuse over the defensive lanes, with one hole at `1NT (2♥)` / `1NT (2♠)`.
Nets in `ab-results/v9/nets/` (not in the repo), A/B in
`ab-results/v9-floor`. The table and the reseed finding it produced are in
[shrinkage-plan.md](../shrinkage-plan.md) "Gate read, change 1". The code
stays as built: the feature version, the trainer support and the
file-loaded serving are what the Gaussian variant (192 wide) would reuse.

**A4's four arms inside v9 (2026-10-25)**: the v9 net barely reacts to the
probe's κ-shrunk boxes. The serving changes about an eighth as many boards
as under v6 (0.33% fired vs 2.6–2.9%). With the boxes in both training and
serving, they read flat. Nothing ships. The table is in
[shrinkage-plan.md](../shrinkage-plan.md) "A4's four arms inside v9".
