# Sound search — closing the pons↔BEN gap through the sampler and the scorer

> **Status: design, no code.** The search *machinery* already exists and ships
> (M2.3 `american_search`, M3 distilled `american_neural_search`); this plan
> refines it. Nothing here starts until a phase is explicitly chosen. The
> concrete Phase 3 of [ben-gap-campaign.md](../ben-gap-campaign.md) and
> Milestone 8 of [plan.md](plan.md) both point here.

## The claim, and why it reframes the work

The instinct is right — **BEN's edge over pons is inference-time search over
sampled worlds** — but "plan to implement search" mis-states the starting line.
Search is *built*:

- **`american_search()` / `SearchFloor`** ([../../src/bidding/search_floor.rs:95](../../src/bidding/search_floor.rs)) — the live "net proposes, DD disposes" bidder: shortlist the net's top-`k=8`, sample layouts, price each candidate by double-dummy rollout, blend. Defaults 128 layouts, `k=8` ([search_floor.rs:104](../../src/bidding/search_floor.rs)).
- **`ev_all`** ([../../src/bidding/ev.rs:69](../../src/bidding/ev.rs)) — the evaluator: one shared DD solve per layout ([ev.rs:114](../../src/bidding/ev.rs)), roll each candidate to a contract, price under perfect-defense doubling (`ns_score_bid`, [ev.rs:138](../../src/bidding/ev.rs)).
- **`sampler.rs`** ([../../src/bidding/sampler.rs](../../src/bidding/sampler.rs)) — the constrained hidden-hand sampler that inverts `Inferences`.
- **M3** distilled that slow search into one fast forward pass, the champion floor `american_neural_search()` (beats deterministic `instinct()` +0.178 vul none / +1.716 vul both on PD; [plan.md M3.3](plan.md)).

Two independent probes located the gap **outside the hand features**:

- **The bitmap-ablation probe** ([ben-gap-campaign.md](../ben-gap-campaign.md#the-bitmap-ablation-probe-does-honor-location-matter)): BEN's policy net reads the hand as `(HCP, shape)` — honor location is a 0.4% median residual. Our authoring vocabulary (`points`, `support_points`, `fit_sum`, `hcp`, length constraints) **is** `(HCP, shape)`, so it is a near-sufficient statistic of the hand. No missing feature.
- **The Info-net probe** ([ben-gap-campaign.md](../ben-gap-campaign.md#the-info-net-probe-weights-side-extraction)): where BEN diverges from us, the delta is search-based judgment and *auction-state reading*, not hand features.

**Conclusion: the −1.9 IMP gap lives in `search over sampled worlds + auction-state memory`.** So the work is not to build search — it is to make the search we have *sound*: fix the **sampler**, fix the **scorer**, and (horizon) enrich the **auction-state memory**. This doc plans all three.

## The map you already own

| Axis | pons today | BEN (v0.8.8.4, source-grounded) | Gap → phase |
| --- | --- | --- | --- |
| **Hidden-hand sampler** | uniform deal + **hard reject** on `Inferences` ranges (`sample_layouts`); optional rule-replay (`sample_layouts_replay`, `set_rule_accept` default **off**) | **Info-net-biased dealing** (honors dealt toward predicted hcp/shape means) + **soft NN-replay gate** (keep worlds whose replay prob clears 0.70, keep-best fallback) | Phase 1 |
| **Rollout scorer** | double-dummy, perfect-defense doubling, **raw points**, own distilled net as continuation | double-dummy (DDS), **IMP** conversion, **BBA** as continuation policy | Phase 2 (sd for slam), Phase 5 (IMP ranking) |
| **Net prior** | strict **shortlist only**; always searches non-forced auctions | shortlist **and** folded back into EV (`adjust_NN·insta_score`); search gated on ≥2 candidates over a low threshold | Phase 5 |
| **Slam / forcing machinery** | authored book + `instinct()` forced rails | consults BBA at the table for RKCB/keycard/forcing | (out of scope — GPL) |

The two axes that pay are the sampler (a real BEN gap) and the scorer (an
*absolute* correctness win, since BEN's scorer is DD-optimistic too — see the
slam-optimism wall).

## The three walls

### 1. The obstruction wall (scorer) — competitive stays off-limits

DD and perfect-defense assume both sides see all 52 cards, so the value of a
preempt / weak jump / sacrifice — *making the opponents guess* — is invisible,
while its overbid cost is fully counted ([measurement.md, Known biases](../measurement.md#known-biases)).

**This is why "search at every leaf" (M7.0) is dead.** M7.0 wrapped *every*
authored leaf, competitive included, with DD leaf-pricing; it regressed
**−2.96 IMPs/board**, all three worst boards a redoubled `7♣xx` grand ([plan.md M7.0](plan.md)). The attribution has a nuance the plan must not get wrong:

- **Proximate cause**: undecoded competitive leaves widen partner's range → the
  sampler deals `7♣` makeable on biased worlds → search picks it → it dies at
  the real table. (A sampler problem — Phase 1 territory.)
- **Final, authoritative cause** (the 2026-07-02 demotion banner): the
  **obstruction wall**. "More decoding narrows ranges but cannot lift a
  scoring-harness limitation." Even a *perfect* decode cannot make DD price
  competitive/sacrifice judgment soundly, because the value lives in the
  opponents' errors, which no perfect-defense scorer models.

**Do-not-retry: DD-pricing competitive leaves.** And note the trap — a
single-dummy *declarer* playout does **not** rescue this: it models declarer's
guesses, but the defense still plays double-dummy on the actual deal, so the
obstruction value (opponents mis-bidding / mis-defending) stays invisible.
Competitive obstruction is a single-dummy-*lead* / real-table question, not a
search-scorer one. **Search's home is constructive reach, not competition.**

### 2. The sampler wall (biased EV) — the fixable one

The live search is only as good as the worlds it samples. Two weaknesses vs BEN:

- **Loose reading → biased worlds.** `Inferences::read` produces ranges; a
  convention we don't decode widens partner's range (sound-by-construction —
  never a crash — but biased). The `7♣xx` grand is the signature failure.
- **Uniform dealing → high variance.** `sample_layouts` deals the other 39
  cards *uniformly* and rejects out-of-range draws ([sampler.rs:88](../../src/bidding/sampler.rs)). BEN instead **biases the deal** toward the Info-net's predicted hcp/shape means, so its ~50–200 kept worlds concentrate where partner actually is. Same sound ranges, far less variance and edge-bias.

Both are attackable **without any BEN code** — the reading is ours, and
importance-weighted dealing is driven by *our* `Inferences`. Phase 1.

### 3. The slam-optimism wall (scorer) — the place to *beat* BEN

Every DD-play scorer lets declarer pick every two-way queen and drop every
offside stiff king, so at the slam boundary it is **optimistic for the arm
bidding more slams** ([measurement.md, slam-optimism wall](../measurement.md#known-biases)). `ev_all` scores DD, so the search over-values slam reach — and so does the
distilled floor it teaches.

Crucially, **BEN's bid-time scorer is also double-dummy** (`estimator=none`,
`double_dummy_calculator=True`) — so BEN wears the same slam optimism. A pons
teacher scored by the **single-dummy declarer playout** (`single_dummy_playout`,
[../../src/single_dummy.rs:506](../../src/single_dummy.rs)) — declarer guesses over sampled worlds, defense DD on the actual deal — would learn *realistic* slam discipline. This is not gap-closing; it is a lever to **exceed** BEN on slam accuracy. Phase 2.

## The plan

Each phase: **deliverable / measure / deps / seam / do-not**. Phases are
ordered by leverage-per-cost, not dependency; 1 is cheapest and already moving.

### Phase 1 — Sampler soundness (cheapest, GPL-clean, in flight)

Make the sampled worlds tight and realistic, so every downstream EV (live search
*and* re-distilled floor) is less biased.

- **1a — Land the reading knobs.** The four in-flight `Inferences::read`
  tighteners (`set_length_soundness`, `set_cue_reading`, `set_table_alert_reading`,
  `set_pass_reading`) move to their measured defaults. Three are reading-side
  bid-inert washes (ship gate = probe soundness + disclosure/sd/search surfaces
  that consume readings); `length_soundness` is the one with a priceable bidding
  delta and a live dual-reference A/B ([ben-gap-campaign.md fix ledger](../ben-gap-campaign.md#fix-ledger)). *Deliverable:* tighter ranges default-on where measured. *Deps:* none (running).
- **1b — Rule-replay sampling default for search.** Promote
  `sample_layouts_replay` (`set_rule_accept`, [sampler.rs:120](../../src/bidding/sampler.rs)) from opt-in to the search sampler's default: a drawn world must not only fall in-range but *replay* the authored policy within `MARGIN=3` nats — pons's analog of BEN's soft NN-replay gate. *Measure:* EV-bias / variance on a fixed divergent set; then the Phase-4 re-distill A/B. *Deps:* 1a.
- **1c — Importance-weighted dealing.** Bias `fill_deals` toward the reading's
  center (deal honors toward the predicted hcp/shape, not uniformly), the
  GPL-clean analog of BEN's Info-net-biased dealing — driven by *our*
  `Inferences`, no BEN weights. *Deliverable:* a weighted dealer behind a knob;
  same sound ranges, lower variance at fixed layout budget. *Measure:* variance
  reduction at equal `n`; downstream A/B. *Deps:* none (independent of 1a/1b).

*Do-not:* never loosen a reading to feed the sampler — when it starves, draw
more deals (a deal is ~0.3µs; a DD solve dwarfs it), never widen the envelope
([bidding-architecture.md, Samplers](../bidding-architecture.md#samplers-samplerrs)).

### Phase 2 — Scorer soundness for slam (single-dummy in the *offline* teacher)

Make the search's slam pricing honest by scoring rollouts with the single-dummy
declarer playout — **offline only**, because the cost is prohibitive live.

- **Deliverable:** an `SD_EVAL` thread-local (default off, idiom of
  `LENGTH_SOUNDNESS` at [inference.rs](../../src/bidding/inference.rs)) that swaps `ev_all`'s per-candidate trick source from the shared DD table to `single_dummy_declarer_tricks` ([single_dummy.rs:564](../../src/single_dummy.rs)); consumed by the `dump-search` teacher, **not** the live `SearchFloor`.
- **Seam:** [ev.rs:132-141](../../src/bidding/ev.rs) — the scoring line `ns_score_bid(result, tricks, vul)` at [ev.rs:138](../../src/bidding/ev.rs). The swap needs (a) leader-view **and** declarer-view inferences per reached contract, computed with `Stance::infer` on the rolled-out prefix — the exact idiom already in [examples/common/mod.rs:213-249](../../examples/common/mod.rs) (`sd_declarer_ns_score`) — but `ev_all` is generic over `System` while `infer` lives on `Stance` ([book.rs](../../src/bidding/book.rs)), so it needs `ev_all` narrowed to `&Stance` or a new `System::infer`, not a bare scoring-line swap; (b) a perfect-defense-doubling variant of `ns_score_tricks` ([scoring.rs:190](../../src/scoring.rs)) — double when `sd_tricks < 6 + level`, so a failing sacrifice still prices doubled (do **not** copy the example's plain `ns_score_tricks`, which drops the doubling ev_all's semantics require).
- **Measure:** a constructive slam-boundary A/B, scored plain + PD **and** the
  sd-declarer bracket (`ab-dump-sd --sd-declarer` / `ab-slam-entry --sd`), with
  the Pavlicek Δlogit shave on DD-making slams ([measurement.md slam-boundary addendum](../measurement.md#the-decision-table)).
- **Deps:** Phase 1 (tight worlds make the playout meaningful).
- **Do-not:** wire the playout into the **live** `SearchFloor` — the "one solve
  shared across candidates" invariant ([ev.rs:114](../../src/bidding/ev.rs)) cannot survive per-contract playout; live cost blows up ~10³–10⁴× (the scorer-seam grounding). Offline batch data-gen absorbs it; the live floor stays the fast distilled net. Do **not** use sd-declarer to justify competitive leaf-pricing (wall 1).

### Phase 3 — Extend search to sound *constructive* leaves (M7.2, gated)

The one still-live branch of the demoted M7: DD-price authored **constructive**
leaves at the slam boundary — the Leaping Michaels template (+2.8 IMPs/board for
search over the rule floor once the *whole* sequence was authored,
[plan.md M7 preamble](plan.md); shipped as a +1.01/board authored convention in
[21gf-ledger.md](21gf-ledger.md)).

- **Gate first:** check whether M6.4's authored slam machinery (floor RKCB,
  minor keycard, `set_transfer_slam_try`, `set_texas_slam_drive`, Stayman
  slam-deficit fixes) *already* reaches the target slams. If yes, the window is
  closed — skip. This gate is why M7.2 is "optional single A/B," not a milestone.
- **Deliverable (if the gate opens):** wrap constructive book leaves (cap the
  authored bid at game, decode the convention via M6's reading, let `ev_all`
  price the slam zone) behind a knob. *Never competitive leaves* (wall 1).
- **Measure:** constructive-abc A/B/C, slam-boundary scope, plain + PD +
  sd-declarer. It must clear its **own** A/B — DD-pricing authored candidates is
  a different experiment from raw-net-on-constructive (which lost 0.8 IMPs/board;
  the floor partition is *re-opened*, not overturned).
- **Deps:** Phase 1 (sound constructive decode), Phase 2 (honest slam scorer).
  **Not** M7.0's dead competitive wiring, despite plan.md's stale `Deps:` line.

### Phase 4 — Re-distill the improved search into the fast floor (M3 round 3)

Bake Phases 1–3 into one forward pass, exactly as M3 rounds 1–2 did.

- **Loop:** regenerate `dump-search` (with the tight sampler + sd-scored teacher
  + wider sound-leaf coverage) → retrain in the off-crate `trainer/` candle
  workspace → ship weights back via `include_bytes!` under the feature version →
  A/B the new floor **vs the prior champion, accept only gains** ([plan.md M3.2](plan.md)).
- **Guard (dual-reference):** the new floor's A/B runs **vs BEN Tier-F**
  (primary) *and* **vs BBA plain-DD** (exploit guard). A vs-BEN win that regresses
  plain-DD vs BBA is presumed exploitation until forensics clears it
  ([ben-gap-campaign.md, dual-reference ship rule](../ben-gap-campaign.md#the-reference-pair)).
- **Deps:** any of Phases 1–3 landing a measured improvement to the teacher.
- **GPL:** the teacher is *our* search over *our* nets; the trainer is off-crate;
  no BEN code or weights enter the loop.

### Phase 5 — Auction-state memory + EV mechanics (horizon)

The probes named *two* legs — "sampled worlds **+ auction-state memory**."
Phases 1–4 are the worlds; this leg is the memory, deferred behind M5.2.

- **5a — Sequence-model policy** (M5.2): move Component B from the summary-feature
  MLP to a small transformer over the call sequence, so the net remembers the
  auction's *shape*, not a flattened summary. Deps: M1 baseline.
- **5b — EV in IMP space** (cheap): BEN ranks candidates by **expected IMP**
  (`use_real_imp_or_mp_bidding`), pons averages **raw points** — and raw-points
  vs IMP diverge under matchpoint-frequency effects ([measurement.md](../measurement.md#known-biases)). Trial IMP-space candidate ranking in `SearchFloor`. Cheap `ev_all`/`blend` experiment.
- **5c — Prior fold-back + confidence gate** (cheap): BEN folds the net prior
  back into the EV (`adjust_NN·insta_score`) and skips search when only one
  candidate clears its threshold; pons keeps the net a strict shortlist and
  always searches non-forced auctions. Trial (a) blending the prior logit into
  the EV band, (b) a confidence gate to skip search when the net is sure (a free
  latency win). Cheap `SearchFloor` tuning.

## Measurement discipline (non-negotiable)

Every phase measures per [measurement.md](../measurement.md) and the BEN
campaign's dual-reference rule — **no bidding change ships on analysis alone**:

1. **Primary: vs BEN Tier-F**, both brackets (plain DD + perfect defense; sd-lead
   where lead-shaped). Verdict from the [decision table](../measurement.md#the-decision-table).
2. **Guard: vs BBA plain-DD** (same seeds) — a one-reference win is presumed
   exploitation until the worst divergent boards clear it.
3. **Slam changes** (Phases 2/3): plain + PD **and** the sd-declarer playout, with
   the Pavlicek shave. Plain-DD-only slam "wins" are suspect until the playout
   confirms the sign.
4. **Operational** ([shared-machine-data-gen.md](../shared-machine-data-gen.md)):
   `SEED_BASE=$(date +%s)` once per experiment, shared across arms; arms
   **sequential** via `scripts/idle-run.sh`; **never rebuild** a binary or restart
   a BEN server mid-run.

## GPL boundary (BEN is GPL-3.0)

BEN is an **analysis oracle and A/B opponent**, run as a separate process, never
linked/vendored/distilled into shipped weights. Every idea here is our own
re-implementation of a *technique* (importance-weighted dealing, replay-gated
sampling, single-dummy scoring), driven by *our* `Inferences` and *our* nets.
Copying BEN's weights or behavioral-cloning its calls into shipped weights is
the line we do not cross.

## Do-not-retry (consolidated)

- **DD-pricing competitive leaves** (M7.0) — obstruction wall, unfixable by decode
  or by single-dummy declarer. Search's home is constructive reach.
- **A standalone "decode sweep" workstream** (ex-M7.1) — it lives under M6;
  extend M6's `Inferences::read` arms, don't resurrect an M7 container.
- **Single-dummy declarer playout in the *live* bidder** — cost blowup; offline
  teacher only.
- **M7.3 continuation-policy fidelity** (book+floor rollout) — parked; reintroduces
  the recursion the net was distilled to avoid; only relevant if leaf-pricing
  ever shows measurable EV, which competitively it will not.
- **`american_search_book` as a permanent twin** — it was the M7 treatment arm;
  on a constructive win it folds into `american_search` as a default-on knob and
  the `_book` name is deleted. `american()` / `instinct()` never move.

## Start here (for the first coding session)

**Phase 1c or 1b** is the cheapest first real code (Phase 1a is already running
as A/Bs). Recommended order:

1. **Phase 1b** — flip `set_rule_accept` to default-on for the search sampler and
   A/B it (smallest diff; the machinery exists). Confirms replay-gating tightens
   worlds before adding the weighted dealer.
2. **Phase 2** — the `SD_EVAL` offline scorer (the highest-value *correctness*
   win; the scoring swap at [ev.rs:138](../../src/bidding/ev.rs) plus two-view inference plumbing — mirrors `sd_declarer_ns_score`, but `ev_all` must first gain `Stance` access, being generic over `System` today).
3. **Phase 4** — re-distill and measure vs BEN Tier-F + BBA guard.

Phases 3 and 5 wait on the gate-check (does M6 already reach the slams?) and on
M5.2 respectively.
