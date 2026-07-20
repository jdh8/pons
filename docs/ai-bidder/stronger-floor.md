# Stronger floor — the three-lever review (features / training / search)

> **⚠️ SUPERSEDED — the machinery this doc plans against was deleted.** The
> M1–M3 search and neural line (`american_search`, `american_neural*`,
> `search_floor.rs`, the v1/v2/v3/search nets and their weights) was removed in
> the variant tidy-up; only the BBA-distilled `NeuralFloorBba` survives. The
> *reasoning* here is kept for the record, but every code reference below is
> dangling — re-deriving the machinery is a prerequisite for any phase.

> **Status: review + decision, no code.** Answers three questions posed
> 2026-07-19, right after the floor swap (`american()` → the BBA-distilled net,
> deterministic system → `american_instinct()`; see
> [bba-gap-campaign.md B4](../bba-gap-campaign.md)). This doc is the *decision
> layer*: which of three levers actually moves the floor, grounded in the two BEN
> probes. Execution detail for the winning lever already lives in
> [`sound-search.md`](sound-search.md) (Milestone 8) — this doc does not restate
> it; it points there, and adds the one concrete work item the swap exposed
> (Phase 0 below).

The three questions:

1. Do we need more parameters/features like BEN? What differs between BEN's inputs and ours?
2. Do we need to train more?
3. Do we implement search, train the floor on the search, rebase search on the floor, and iterate?

## TL;DR — the verdict on each lever

| # | Lever | Verdict | Why (one line) |
| --- | --- | --- | --- |
| 1 | **More hand features** | **No** | Honor location past `(HCP, shape)` is a **0.4 % median** residual — our authoring vocabulary already *is* `(HCP, shape)`. |
| 1′ | **Auction-state memory** (the real feature gap) | **Yes, but horizon** | BEN's LSTM remembers the *sequence*; our MLP sees a flattened summary. This is M5.2 / M8.5, gated behind a sequence-model policy. |
| 2 | **Train more (of the same target)** | **No** | We hard-clone BBA; a cloned policy asymptotes to its teacher. More BBA rows → closer to BBA, never past it. |
| 2′ | **Re-distill *search* targets** | **Yes** | The sanctioned form of "train more": distil the improved search, not more oracle calls. This *is* expert iteration (M8.4). |
| 3 | **Search → distil → rebase → iterate** | **Yes — the plan** | Already the designed loop (M3 ran rounds 1–2; M8.4 is round 3). Make the search *sound* first, keep it *constructive-only*, and **re-base it on the shipped floor** (Phase 0 — new, below). |

**One sentence:** the measured pons↔BEN gap is **search over sampled worlds
(~half) + auction-state memory (the rest)**, *not* missing hand features — so the
stronger floor comes from making our built search sound and iterating it back
into the net (levers 2′ + 3), not from feeding the net a richer hand.

## The parameters: BEN vs pons, side by side

The comparison the review asked for. Sources: BEN v0.8.8.4 21GF, read from
`~/ben/src/{binary.py,botbidder.py,sample.py}`; pons from
[`features.rs`](../../src/bidding/features.rs), [`neural.rs`](../../src/bidding/neural.rs),
[`search_floor.rs`](../../src/bidding/search_floor.rs).

| Axis | pons (shipped `NeuralFloorBba`) | BEN (21GF) |
| --- | --- | --- |
| **Input width** | **88 f32** (`features_v3`) | **193 f32** per step |
| **Hand encoding** | **10 summary numbers**: 4 suit lengths + 4 per-suit HCP + total HCP + a shape scalar. *Disclosable-only* — no card detail. | **24-cell honor bitmap** `[A,K,Q,J,T,#small]`/suit (the real hand at honor granularity) **+** exact HCP + 4 shape scalars. |
| **Auction encoding** | **Flattened summary**: last bid, partner's last bid, strain bitmasks, penalty/passout flags **+** the disclosed `Inferences` min/max ranges (40 cells). No ordered call history. | **Full `4×40` seat-separated one-hot call history**, fed step-by-step. |
| **Memory over the auction** | **None** — a per-decision MLP (88→256→256→38). | **LSTM ×3 (128)** — recurrent auction-state memory. |
| **Hidden-hand inference** | **Authored** `Inferences::read` (sound ranges), fed as 40 input features. | **Learned Info-net** (same LSTM trunk) predicting 3 hidden hands' HCP + shape. |
| **Search at bid time** | Built but off by default (`american_search`, `search` feature): shortlist → sample → DD rollout → blend. | Info-net-biased dealing + soft NN-replay gate (0.70) + ~50 worlds + DD rollout, ranked by **mean IMP** with the net prior folded back (`adjust_NN` 60 contested / 200 undisturbed). |
| **Slam / keycard** | Authored book + `instinct()` rails. | Hands off to **BBA** at the table (not the net). |
| **Trained from** | **Hard behavioral cloning** of the BBA/EPBot 2/1 oracle — 40 k boards → 423 k rows, top-1 vs BBA **85.9 %**. | Supervised distillation of a rule engine (BBA-8730 per the arch doc — see caveat), **then inference-time search bolted on**. No self-play retrain. |

Two structural differences carry signal; the rest is the same `(HCP, shape)`
statistic in different clothes:

- **Honor granularity** (BEN's 24-cell bitmap vs our 10 summary numbers) — but
  see Lever 1: measured at 0.4 %.
- **Auction memory** (BEN's LSTM over the call sequence vs our flattened
  summary) — the one representational gap with real mass, Lever 1′.

## Lever 1 — features: no for the hand, horizon for the auction

**The ML point (grounded):** a feature only helps if the *target* depends on it.
Two probes measured whether BEN's target depends on what we don't feed:

- **Bitmap-ablation probe** ([ben-gap-campaign.md](../ben-gap-campaign.md)):
  zero out honor *location* in BEN's own input and its policy barely moves —
  **0.4 % median KL**, material only in a thin tail (weak-two suit quality,
  slam-zone control placement). Our authoring vocabulary (`points`,
  `support_points`, `fit_sum`, `hcp`, length constraints) **is** `(HCP, shape)`,
  so it is a near-sufficient statistic of the hand for the policy. **Feeding the
  net the 13 cards buys ~nothing.**
- **Tier-F ruliness probe**: BEN's policy is **95 %+ rule-expressible**
  (exact-tuple ceilings 91–100 % at every high-mass node, holding into contested
  nodes). A richer hand feature can't recover a gap that isn't in the hand.

**So the only sanctioned new *hand* feature** is suit-quality / honor-
concentration, and only scoped to **preempt discipline + slam cue placement** —
not a global floor term. Low priority; do it as a targeted convention/eval, not a
representation overhaul.

**The real feature gap is Lever 1′ — auction-state memory.** Our MLP flattens the
auction into last-bid + ranges; BEN's LSTM remembers the *shape* of the whole
sequence. That is genuine capacity we lack. But it is the **horizon** item:
it needs the sequence-model policy (plan.md **M5.2**), folded into M8 as **M8.5**.
Defer until the cheaper search levers are spent — the probes put roughly half the
non-search residual here, and it costs a new net architecture.

## Lever 2 — training: no for more clones, yes for search targets

**The ML point:** we train by **hard behavioral cloning** — the BBA oracle
exposes only its chosen call, so the target is one-hot and the loss drives the
student toward BBA's argmax ([bba-gap-campaign.md B4](../bba-gap-campaign.md)).
A cloned policy's ceiling **is its teacher.** We are at 85.9 % top-1 vs BBA on
40 k boards; more boards raise fidelity toward BBA and **stop there**. Since BBA
is our *exploit guard*, not our north star, cloning it harder can't reach BEN.

**The form of "train more" that pays is re-distilling *better* targets** — targets
from our own improved search (sound sampler + honest scorer), not more oracle
calls. That is exactly **M8.4** ("regenerate `dump-search` → retrain in
`trainer/` → ship weights, accept only gains"), and it is the engine of Lever 3.
So: **don't scale the BBA clone; scale the search-distillation loop.**

(Cheap knob worth noting, not doing: the shipped BBA net used 40 k boards /
300 epochs / weight-decay 0; the disclosable-american net used ~1 M rows. Room to
grow the clone exists — it just asymptotes to the wrong target.)

## Lever 3 — search + expert iteration: yes, this is the plan

**The ML point (the one that matters):** double-dummy search over sampled worlds
is a **policy-improvement operator** — it turns the fast policy's shortlist into a
cardplay-grounded better decision, which you then **distil back** into the fast
net; repeat, and the student passes the teacher. This is the AlphaZero-shaped
loop, and it is BEN's own recipe (though BEN ran it *once* — supervised then
fixed search; pons's iterated distil-then-search is, if anything, more than BEN
does). We already ran rounds 1–2 (M3 → the `american_neural_search` champion);
**M8.4 is round 3.**

The loop, and where each step lives:

```
       ┌────────────────────────────────────────────────────┐
       │  0. re-base the live search on the shipped floor    │  ← Phase 0 (new)
       ▼                                                     │
  fast floor  ──prior/shortlist──►  live DD search  ──distil──┘
 (classify_bba)                    (american_search)   M8.4
       ▲                                 │
       │                                 ├─ sound sampler (M8.1: rule-replay
       │                                 │   already default-on; add weighted
       └── round N+1 net ◄── retrain ◄───┤   dealing) 
                            trainer/     ├─ honest slam scorer (M8.2: sd-declarer,
                                         │   offline teacher only)
                                         └─ constructive leaves only (M8.3);
                                            competitive = obstruction wall
```

**The two hard constraints** (both paid for — see
[`sound-search.md`](sound-search.md#the-three-walls)):

1. **Constructive reach only.** DD + perfect defense can't price *obstruction*
   (a preempt/sac's value is the opponents' errors). Pricing competitive leaves
   is why M7.0 died at **−2.96 IMPs/bd** (the `7♣xx` grand). Search's home is the
   constructive slam/game boundary; competition stays with the authored floor.
   *Do-not-retry, even with a single-dummy declarer* (the defense still plays
   double-dummy, so obstruction stays invisible).
2. **Make the search *sound* before distilling it.** Distilling a biased search
   bakes the bias into the net. M8.1 (tight sampled worlds) and M8.2 (honest slam
   scorer) come before M8.4 (re-distill).

Everything in Lever 3 except Phase 0 is already specified in
[`sound-search.md`](sound-search.md). **Do not re-plan it here.**

## Phase 0 — re-base the live search on the shipped floor (new, from the swap)

The 2026-07-19 swap made `classify_bba` / `features_v3` the shipped floor, but the
**live search still runs on the pre-swap nets**:

- **Shortlist prior:** [search_floor.rs:126-127](../../src/bidding/search_floor.rs#L126-L127)
  — `neural::classify` over 160-dim `features::features`, i.e. the **v1** net.
- **Rollout continuation:** [search_floor.rs:76-77](../../src/bidding/search_floor.rs#L76-L77)
  — `american_neural_search()`, the **M3.2** net (its comment even documents the
  old round-2 iteration intent).

So "put live DD search on top of the stronger prior" — the follow-on named in
[bba-gap-campaign.md](../bba-gap-campaign.md) — is not yet literally true: the
search sits on a prior and a continuation that predate the floor it's meant to
strengthen. This is a decision the pre-swap M8 plan doesn't capture, and it must
be settled **before** M8.4 (the re-distill needs a defined prior + continuation):

- **The continuation** rolls out all four seats — it should be our *best fast
  policy*. Options: repoint to `classify_bba` (the shipped floor), or keep the
  M3.2 net (it was itself search-distilled, so plausibly a better *rollout*
  policy than a fresh BBA clone). An A/B on a fixed divergent set decides.
- **The prior/shortlist** only has to *propose* candidates for the search to
  price; a weaker prior costs recall in the shortlist, not final accuracy.
  Repointing to `classify_bba` / `features_v3` is the obvious default; measure it.
- **Cleanest resolution:** fold Phase 0 *into* M8.4 — the re-distill mints a fresh
  net (sound-search targets, `features_v3`) that becomes **both** the prior and
  the continuation in round 4, retiring the v1/M3 wiring in one step. Phase 0 then
  reduces to "don't A/B the old wiring in isolation; let the first M8.4 round
  define the search's nets." That is the lazy-correct path unless an interim
  measurement is wanted.

*Do-not:* treat this as a mechanical repoint and ship it unmeasured — the search's
prior/continuation are load-bearing for what it distils, so any change is an A/B
under the dual-reference rule (vs BEN Tier-F primary + BBA plain-DD guard), like
every other M8 step.

## Sequencing recommendation

Cheapest-first, leverage-per-cost (mirrors [`sound-search.md`](sound-search.md#start-here-for-the-first-coding-session)):

1. **M8.1c importance-weighted dealing** — the last open sampler item (rule-replay
   already shipped; see the sync note below). Tightens every downstream EV. Cheap.
2. **M8.2 the `SD_EVAL` offline slam scorer** — the highest-value *correctness*
   win, and a lever to *beat* BEN (whose bid-time scorer is DD-optimistic too).
3. **M8.4 round-3 re-distill**, absorbing **Phase 0** (the fresh net becomes the
   search's prior + continuation). A/B vs the prior champion, accept only gains.
4. **Horizon:** M8.3 constructive leaves (gate-checked against M6.4 first), then
   Lever 1′ / M8.5 auction-state memory (needs M5.2's sequence-model policy).

Levers 1 (hand features) and 2 (more clones) are **closed** — don't spend on them.

## Caveats / open flags

- **BEN's teacher is inconsistently documented.** `ben-architecture.md` reads the
  `-8730-` weights as **BBA-8730**; `ben-gap-campaign.md:81` says BEN trained on
  **GIB-bid hands**. Both may be true across BEN's lineage (upstream GIB/BBO data,
  this 21GF model re-distilled on BBA output), but the docs don't reconcile it and
  I can't confirm the corpus from the inference-only vendored tree. Flagged, not
  resolved — it doesn't change any verdict here (search, not the teacher, is the
  gap).
- **The −1.906 / −1.860 BEN anchor predates the swap** — it measured the old
  deterministic floor. The refresh folds into the next periodic Tier-S anchor
  (already noted in [ben-gap-campaign.md](../ben-gap-campaign.md)).

## Linked plans (execution detail — not restated here)

- [`sound-search.md`](sound-search.md) — Milestone 8, the full 5-phase design for making the built search sound. **The execution home for Lever 3.**
- [`plan.md`](plan.md) — M5.2/M5.3 (auction memory, Lever 1′), M8 (search).
- [`../ben-gap-campaign.md`](../ben-gap-campaign.md) — the gap measurement, the two probes, the dual-reference ship rule.
- [`../ben-architecture.md`](../ben-architecture.md) — how BEN bids (the parameter/search source).
- [`../measurement.md`](../measurement.md) — the A/B playbook every phase runs under.
