# The BEN gap campaign — closing pons↔BEN, with BBA as the exploit guard

**Status: Phase 0 in progress (2026-07-16) — harness built and validated
(steps 1–3 of the [ben-gen validation plan](ben-gen-design.md)); the
EPBot-vs-BEN calibration (step 4, the Phase 0 exit gate) is running.** The
[survey](open-source-bidder-survey.md) refuted "pons is the strongest
open-source bidder": BEN (GPL-3.0, code + weights in-repo) beats EPBot by
0.35–0.38 IMPs/deal DD in BBA's own cross-engine tables, and pons trails
that same EPBot by ≈1.7–1.9. Chained estimate: **pons ≈2.1 IMPs/board behind
BEN**. This campaign re-aims the north star from BBA at BEN — the measured
open-source front-runner among human-system bidders — and demotes BBA from
target to **reference**: a cheap, precise, independent yardstick whose job
is to catch us exploiting BEN's quirks instead of getting better at bridge.

The harness engineering lives in [ben-gen-design.md](ben-gen-design.md).
Ship rules stay in [measurement.md](measurement.md); the BBA anchor runbook
and its bucket history stay in [bba-gap-campaign.md](bba-gap-campaign.md) —
that loop is not retired, it is re-subordinated.

## Know the enemy

BEN is instructive, not just strong. Its current nets are **trained on deals
bid by BBA 8730** (model names carry the BBA build), it vendors BBA and can
consult it during bidding (`consult_bba`, a score nudge + RKCB answers), and
on top of that distilled policy it runs threshold-gated Monte-Carlo
sampling + double-dummy rollouts before committing to a call. The student
beat the teacher by adding **search over sampled worlds** to a policy
distilled from the teacher. That is the same architecture pons is
independently building toward (constrained sampling, DD search at leaves,
`american_neural_v3` distillation) — so the gap decomposition below is also
a referendum on which of our roadmap items to accelerate.

Consequences:

- Where BBA and BEN agree, the old campaign's verdicts transfer.
- Where BEN diverges from BBA, the delta is (mostly) *search-based
  judgment*: thin games, competitive decisions, slam accuracy priced by DD
  over samples. Expect new buckets to implicate judgment (game/slam COGs,
  competitive pricing) more than convention coverage.
- BEN's system (BEN-21GF card) is GIB-flavored 2/1 GF — close kin to
  `american()`. Disclosure asymmetry is mild and symmetric: our floor reads
  BEN through `Family::NATURAL` (as it reads BBA), BEN models opponents
  through its own nets. This mirrors BBA's published protocol, where
  external engines bid their own systems.

## The reference pair

| Engine | Role | Cost | What it's for |
| --- | --- | --- | --- |
| **BEN v0.8.8.4** (BEN-21GF, stock "Tier S" / policy-only "Tier F") | **Target.** The campaign metric is pons-vs-BEN IMPs/board. | Slow (HTTP + NN + DD rollouts): 20k-board anchor ≈ overnight; 102.4k Tier-F A/B arm ≈ 5–6 h | Headline anchor (Tier S); per-fix primary A/B (Tier F); decompose |
| **BBA/EPBot** (vendored FFI) | **Guard + microscope.** | Fast: 409.6k-board arm in ~1–2 h | Exploit guard on every ship; big-sample forensics; the existing anchor series continues as a secondary metric |

**The dual-reference ship rule** (extends, never overrides,
[measurement.md](measurement.md)'s decision table — all iron rules stand):

1. **Primary**: fresh-seed A/B **vs BEN Tier F**, both brackets (plain DD +
   PD; sd-lead where the treatment is competitive/lead-shaped). Verdict from
   the standard decision table.
2. **Guard**: same-seed A/B **vs BBA**, plain DD must not regress outside
   CI. A vs-BEN win that loses plain DD vs BBA is presumed **exploitation**
   (policy-net artifact, sampler blind spot) until forensics on the worst
   divergent boards proves otherwise. PD-only wobbles on the guard don't
   block (the vul-PD doubling artifact is a known false alarm) — plain does.
3. The existing self-play/advertised exploitation guard for sd verdicts
   stays as-is.

Rationale: BBA is rule-based and independent of BEN's training lineage in
its *decisions* (even though BEN trained on its output) — a genuine bridge
improvement should win or wash against both; a quirk exploit shows up as a
one-reference win. This is the anti-overfitting discipline the user set for
the campaign, and it is cheap: the guard costs 1–2 h of FFI time per ship.

**Tier-transfer caveat**: Tier F (policy-only) is weaker than Tier S (the
engine BBA measured). Per-fix verdicts read at Tier F; the periodic Tier-S
anchor is the truth. If shipped Tier-F wins stop moving the Tier-S anchor,
stop and re-examine the tier gap (Phase 1 measures it once, same seeds).

## Phases

### Phase 0 — harness (`ben-gen`) + validation

Build per [ben-gen-design.md](ben-gen-design.md). Exit criteria: the five
validation steps pass, most importantly the **EPBot-vs-BEN calibration** —
our harness reruns BBA's own published match (Table 1: EPBot −0.51 SD /
−0.38 DD vs BEN-21GF) with zero pons code in the loop. Sign + rough
magnitude agreement validates the harness *and* bounds our vendored EPBot's
vintage. Estimated effort: ~2–3 days code + 2–3 nights compute.

### Phase 1 — first BEN anchor + decompose

- **Headline**: Tier S, 20k boards (8×1,250 × {vul none, both}), fresh
  `SEED_BASE`, persisted as a new anchor series (`ab-results/ben-anchor/`),
  scored plain + PD (+ sd spot-checks). This number replaces the survey's
  chained ≈2.1 estimate. Do **not** expect it to equal the chain — different
  deal stream and protocol; sign and ballpark are the check.
- **Tier gap, once**: same 20k seeds at Tier F. The delta (expected: Tier F
  weaker, i.e. our number less negative) calibrates how to read all future
  Tier-F A/Bs.
- **Decompose**: Tier F, 102.4k boards, through `bba-decompose` unchanged
  (`explain_call` attribution + replay verification are ours-side and
  engine-agnostic). Output: the ranked bucket table for BEN, side by side
  with the BBA anchor's. The interesting column is the *difference*: buckets
  where BEN outbids us but BBA doesn't are the search-judgment frontier;
  buckets shared with BBA are the already-understood book/floor work with a
  new price tag.

### Phase 2 — the loop

Same steady-state loop as the BBA campaign, re-aimed:

```text
BEN anchor report → worst bucket → trace boards → fix (book / floor / search)
→ Tier-F A/B vs BEN + BBA plain-DD guard → ship by decision table
→ re-anchor Tier S (periodic, not per-fix) → next bucket
```

Anticipated fix vocabulary, in order of prior:

1. **Book/floor fixes shared with the BBA buckets** — the mined-to-residual
   BBA table (defensive round-1 wall, constructive opening/rebids, RKCB
   accuracy) re-prices against a stronger reference; some "residuals" may be
   first-order vs BEN.
2. **DD-search-at-leaves** — BEN's edge is DD-priced judgment; our
   counterweight already exists in embryo (leaf DD search, sd-declarer
   playout, M6.4 floor slam tools). Buckets that scream "judgment, not
   convention" route here rather than to node authoring.
3. **Learned-floor round 3+** (Pillar B of the BBA campaign) — a BEN-shaped
   routing gate (does the floor candidate win *vs BEN*, not just vs BBA)
   slots into the existing acceptance protocol.

Per-treatment history goes to the existing ledgers; this doc gets the anchor
headline trail, as bba-gap-campaign.md does today.

### Phase 3 — horizon (explicitly deferred)

If Phase 1 shows the gap dominated by search-judgment buckets, the strategic
answer converges with the AI-bidder roadmap: distil + search (BEN's own
recipe, which pons has all the parts for). Also deferred: a Rust Blue Chip
table manager (unlocks WBridge5/GIB/BBA-as-member under one harness — the
full Table 1 yardstick set), and BEN-disclosure-aware inference. None of
these starts before the first anchor says the cheap fixes are exhausted.

## Operations

- Compute realities: see the throughput table in
  [ben-gen-design.md](ben-gen-design.md). Tier-S anchors are overnight jobs;
  Tier-F A/Bs are afternoon jobs; BBA guards are hours. All heavy runs under
  `scripts/idle-run.sh` conventions, arms sequential, BEN servers
  nice/SCHED_IDLE ([shared-machine-data-gen.md](shared-machine-data-gen.md)
  governs — the servers are the real load).
- **Never** restart/upgrade the BEN servers mid-experiment (the no-rebuild
  rule's analog); one `SEED_BASE` per experiment shared across arms; the
  anchor series keeps its persistent seed (same sanctioned exception as the
  BBA series).
- Version discipline: BEN pinned at v0.8.8.4 + config hash recorded in
  `gen_args`. BEN is actively developed; re-pinning is a deliberate
  campaign decision (it moves the target), taken at most per-milestone and
  re-anchored immediately.
- GPL boundary: BEN runs as a separate process over HTTP. Never link, embed,
  or vendor BEN code/weights into pons.

## Success criteria

1. **Phase 0/1 (near-term)**: the calibration reproduces BBA's Table 1 in
   sign and ballpark; a measured pons-vs-BEN anchor exists with CI, retiring
   the chained estimate; a ranked BEN bucket table exists next to the BBA
   one.
2. **Campaign metric**: pooled plain/PD IMPs/board vs BEN Tier S, tracked in
   this doc per re-anchor exactly as bba-gap-campaign.md tracks its metric.
3. **The claim**: "pons is the strongest open-source bidder" is re-evaluated
   when the anchor reaches parity vs BEN — with the honest footnote that brl
   (+1.24 vs WBridge5, non-human system) sits above BEN on the public
   ladder, and a Blue Chip TM would let us measure against the rest of
   Table 1 directly.

## Open questions at design time

1. All five ben-gen validation unknowns (tag dialect, RSS, 21GF config
   contents, Linux BBA-consult path, EPBot vintage) — resolved in Phase 0.
2. Does the BBA bucket ranking transfer to BEN, or does search-judgment
   dominate? — Phase 1's decompose answers this; it decides whether Phase 2
   leans book-authoring or search.
3. How big is the Tier F↔S gap, and is it stable enough to trust Tier-F
   ship verdicts? — Phase 1 measures it once, re-checked whenever Tier-F
   ships stop moving the Tier-S anchor.
4. Where does the measured anchor land vs the chained ≈2.1? Large
   disagreement (beyond protocol noise) would itself be a finding about the
   survey's transitivity caveats.
