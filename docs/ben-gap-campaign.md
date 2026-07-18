# The BEN gap campaign — closing pons↔BEN, with BBA as the exploit guard

**Status: first anchor MEASURED (2026-07-17) — pons is
−1.906 plain / −1.860 PD IMPs/board behind BEN Tier S** (20k boards at
`119675f`; trail below). Phase 0 is complete: the EPBot-vs-BEN calibration
exit gate PASSED (plain DD −0.568 pooled from EPBot's side vs BBA's
published −0.38 DD / −0.51 SD; details in
[ben-gen-design.md](ben-gen-design.md), validation step 4). Phase 1's
remaining items: Tier-F gap on the anchor seeds, then the 102.4k Tier-F
decompose. The
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

The full source-verified mechanism — input encoding, net shapes, the
decision pipeline, how competition resolves, and Tier F vs Tier S — is
mapped in [ben-architecture.md](ben-architecture.md). Summary below.

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

## BEN's declared system (source-extracted)

BEN's *policy* lives entirely in Keras weights — there is no symbolic book
in its source to extract. What the source does ship is BEN's **declared**
system: `BBA/CC/BEN-21GF.bbsa`, a 258-line BBA convention card that BEN
itself consults at runtime (`consult_bba = True`: a score nudge plus RKCB /
keycard answers). We vendor it byte-identical as
[vendor/ben/BEN-21GF.bbsa](../vendor/ben/BEN-21GF.bbsa) (the `.bbsa` format
has no comment syntax, so provenance lives here: lorserker/ben v0.8.8.4,
sha256 `28e2d15f5f2761355b5d01c47b5c738e155533d212dd2953895c82da6584717a`).

The card is stock BBA 2/1 (`System type = 0`) with exactly **10 toggle
lines** changed — 7 treatments:

| Treatment | Stock BBA 2/1 | BEN-21GF |
| --- | --- | --- |
| Keycard responses | Blackwood 0314 | **Blackwood 1430** |
| Checkback after 1x‑1y‑1NT | New Minor Forcing | **Two‑Way NMF** (2♣ invitational relay, 2♦ GF) |
| Major-raise structure | Shape Bergen | **Strength Lawrence** |
| Leaping Michaels (4m over their weak two = 5‑5 strong) | off | **on** |
| 1NT‑3♥/3♠ splinter (short major, minor-oriented GF) | off | **on** |
| Gerber | any 4♣ per card | **NT openings only** |
| Extended Stayman continuations | on | **off** |

Provenance and its limits:

- BEN's training data is GIB-bid hands, so its *learned* book is
  GIB‑2/1‑shaped; the card above is its *declared* system. The two mostly
  agree — kin to `american()` either way.
- **Weights-vs-card caveat**: EPBot loaded with BEN's card is BEN's
  *skeleton*, not BEN — EPBot measures ≈0.35 IMPs/bd behind BEN, and that
  edge lives in the weights (search over sampled worlds), not the card.
- Rule-level query surface: `~/ben/src/bba/BBA.py` wraps EPBot with the BEN
  card loaded and exposes `interpret_bid`, `get_info_meaning`,
  `get_info_min/max_length`, `get_info_strength` — a queryable book with
  meanings, for spot-checking what BEN's declared system says a call shows.

Harness hook: `bba-gen --our-card/--their-card <file.bbsa>` loads a full
card (system id from its `System type` header; explicit `--*-conv` singles
still apply on top).

### The Info-net probe (weights-side extraction)

The card is BEN's *declared* book; its Info net (auction → predicted HCP +
shape of the three hidden hands) is a queryable slice of the *learned* one —
usable as a reference for our `Inferences::read`. Pipeline (one forward pass
per row, no sampling/DD — minutes on CPU):

1. `cargo run --features serde --example probe-ben-info` — self-plays our
   default system, emits one jsonl row per (board, auction prefix): actor's
   hand, our reading of the three hidden seats, and ground truth (HCP,
   upgraded `point_count`, suit lengths).
2. `scripts/ben-info-dump.py` (run with `~/ben/.venv/bin/python`) — appends
   BEN's Info-net prediction per hidden seat. Deterministic; batched.
3. `scripts/ben-info-compare.py` — ranks (a) truth violations = the actual
   hand breaks our shown band (self-play is honest, so every one is a
   reading bug), (b) BEN outside our band, (c) vagueness = we show nothing
   where BEN commits.

Reading the reports: **truth + BEN against us** = our reading bug;
**truth with us, BEN against** = BEN misreading *our* conventions through
its GIB/BBA-shaped prior (disclosure asymmetry — exploit-guard material,
not a bug); vagueness is inflated by BEN conditioning on the actor's own
hand (residual-strength inference ours deliberately leaves to the sampler),
so trust auction-level aggregates, not single rows.

First 1000-board run (2026-07-17, NV, seed 1784259000): real reading bugs —
preemptive `1C (P) 3C` jump raise read as 10+ limit (3/3), cue/two-suiter
calls read as their *natural* suit (phantom-suit class: `(P 2D) 4D` on a
void, `(P 1C) 2C` Michaels on a club void, `P P 1H (2D) 3D` cue-raise read
as 4+ diamonds 4/4), opener's `1D (P) 1S (P) 2D` rebid shown 6+ but bid on
5 (5/8), and `1S (1N) X` read as 15+ on a 9-count. BEN-misreads-us: our
natural 2D/2C over 1NT (BEN's prior says Multi/Landy), our South-African
Texas 4D. Vagueness: **passes narrow nothing** in our reading — BEN reads a
passed hand at ~6.3 mean HCP.

#### Fix ledger

- **Phantom suits — FIXED 2026-07-17** (probe re-run, same seed: every
  phantom-length bucket drained; `--sound-reading` runs the fixed reading).
  Mechanism: our own pair's conventions decode via alert + projection (clean
  — the cue-bidder's *partner* was never flagged), but **opponents'** calls
  read through the natural walk, which had no cue concept. Two knobs in
  `inference.rs`, both **SHIPPED default-on** (length-soundness 2026-07-18
  by its dual-reference A/B; cue-reading 2026-07-18 as bid-inert reading
  soundness — see the inertness-probe entry):
  - `set_cue_reading` — a bid of a suit only the opponents have naturally
    shown is a cue, never a holding; records Michaels/Leaping Michaels over
    a minor opening (both majors 5-5) and the non-jump cue-raise (3+
    support, 10+ points, mirroring the Rubens floors).
  - `set_length_soundness` — opener's immediate 2-level rebid of the opened
    suit reads 5+ not 6+ (the floor routinely rebids a good five, 5/8 +
    3/4); an agreed-suit re-raise adds no length (`1M-2M-3M` game tries read
    a phantom sixth); a doubler's later jump is never a weak six-card jump
    (made on three, 2/2).
- **Table-wide alert reading — BUILT 2026-07-17** (jdh8: "Alerting in bridge
  is for opponents. We should make it available to the other 3 players on
  the table, not just the partner."). The projection pass decoded an alerted
  call only when the *reader's own* book authored it, so the opponents'
  alerted conventions (their splinter, their checkback, their Michaels) fell
  to the natural walk. `set_table_alert_reading` (**SHIPPED default-on 2026-07-18** as bid-inert
  reading soundness — see the inertness-probe entry; `--no-ns-table-alert-reading`
  is the off-switch)
  resolves each opponent call in *their* phase-routed book — the stance
  models them as playing our own books, exact in self-play, an approximation
  vs BBA/BEN — under their at-the-time context, and decodes it when the rule
  alerts it; their natural calls keep the walk. Same-seed probe: seat-suit
  length violations **552 → 324 (−41%), 228 drained, 0 introduced**.
- **Pass reading — BUILT 2026-07-17** (jdh8: "The general reading of a pass
  is to exclude all the other calls" — negative inference; each pass carries
  −log p bits, few but never zero, on the most frequent call in bridge). In
  a well-authored table the complement is the Pass rule's own gate (the
  opening table passes on `points(..12)` *because* the bids cover 12+), so
  `set_pass_reading` (**SHIPPED default-on 2026-07-18** as bid-inert reading
  soundness — see the inertness-probe entry; `--no-ns-pass-reading` is the
  off-switch) decodes each pass off the
  union of its table's Pass gates, both bounds (`Constraint::project_band` —
  the ceilings `project` drops return here, an `hcp` ceiling widened by the
  scale's max upgrade), resolved in the trie of the pass's *own turn*
  (slice-relative `trie_for`); own side always, opponents under table-wide
  disclosure. Falls out: no-open ≤ 11 points, silent responder ≤ 10,
  their-suit direct seat ≤ 17 HCP (`defense_to_suit`'s pass gate authored as
  the strong tier's byte-identical complement), 1NT signoff ≤ 13 with no
  six-card major; trap-pass advances (trivial gates) and floor passes
  correctly read nothing. Replay sampling un-short-circuits Pass too
  (`set_rule_accept`): the sample-level exact complement rejects
  would-have-opened/preempted candidates — the disjunctive half the
  envelope can't hold. Same-seed probe (BEN annotations grafted from
  `probe-nv-ben.jsonl`, 9,130/9,160 rows; the on-arm auction stream is
  row-identical to `probe-nv-table.jsonl`): truth violations **97 → 97
  points / 324 → 324 lengths, 0 introduced** (compare script now checks
  ceilings — floor-only before, so a cap-only change was unfalsifiable);
  full-band hidden seats **15,101 → 7,279 (−52 %)**; acted-seat vagueness
  deviation **24,417 → 9,740 (−60 %**, vs −3 % from the three prior knobs
  combined) — the `[P]`, `[P P]`, `[1x P]` passer buckets all drain
  (arm: `probe-nv-pass.jsonl`). Remaining pass-family vagueness: *unacted*
  seats (deal conservation — joint, envelope-inexpressible; the layout
  sampler applies it when dealing full deals) and the deferred gates —
  `[1N P]` their-1NT direct seat, advance/balancing seats, later-round
  passes — author those tables' gates the same way once their complements
  are checked.
- **Reading-knob bid-inertness probe — MEASURED 2026-07-17** (same-seed
  bba-gen divergence vs the off arm, 6400 boards/knob at seed 1784294370
  plus 211,200 board-pairs for pass from the guard cells): `cue_reading`
  **0** divergent, `table_alert_reading` **0**, `pass_reading` **1/211,200**
  (a deep contested floor decision, 3NT↔4♠), `length_soundness` **23/6400
  (0.36%)** — and the all-four composite's divergence is entirely
  length-soundness's. Consequence for the queued A/B: three of the four
  knobs are reading/instrument-side — their plain/PD IMPs verdict is a
  **wash by construction** (the guard cells for pass are the on-disk
  witness, `ab-results/reading-knobs/2026-07-17/`), so their ship gate is
  probe soundness (0 new violations, vagueness −60%) plus the surfaces that
  consume readings: sd-lead pricing, search-mode sampling, disclosure.
  Only **length-soundness** has a priceable bidding delta — its
  dual-reference A/B (off arm shared, `scripts/reading-knobs-ab.sh length`)
  is the one that runs to a numeric verdict.
- **Length-soundness A/B MEASURED + SHIPPED default-on 2026-07-18**
  (SEED_BASE 1784294370, sha 74d783d arms; a mid-experiment lib rebuild by a
  parallel session was certified bid-inert — 0/6400 off-flag re-bid drift).
  Plain-wash + PD-win on **both** references: guard vs BBA (204.8k
  boards/cell) plain +0.0010/+0.0006 wash, PD **+0.0022 ±0.0011 /
  +0.0023 ±0.0016** clear wins; primary vs BEN Tier F (51.2k/cell) plain
  +0.0008/−0.0001 wash, PD +0.0020 ±0.0022 / +0.0015 ±0.0030 directionally
  consistent; +0.4–1.1 IMPs/fired. `--no-ns-length-soundness` is the
  off-switch. The same run banked the **first Tier-F gap** (fresh seeds):
  plain −0.879 (none) / −1.092 (both), PD −1.122 / −1.519, divergence 71% —
  vs the Tier-S anchor −1.640/−2.172 plain: **BEN's search ≈ 0.8–1.1
  IMPs/board** at this distance. Ops: Tier-F throughput measured 0.27 s/bid
  under an 8-instance fleet (≈2× the uncontended design figure — the
  per-instance bid lock makes instance count the throughput knob); fleet
  scaled to 32 instances (~1 GB RSS each, Tier-F arenas grow ~30 KB/board)
  after the run's port discovery completed.
- **Still open, by ranked margin**: (1) preemptive minor jump raises
  (`1C (P) 3C`, `P P (1D) P 3D`) read as 10+ limit while the floor bids
  them on 3–6 — decide the raise's meaning, then align floor and reader;
  (2) the XYZ complex after `1m-1M-1S/1NT` **over-claims in the projection
  itself** (12 violation rows survive table-wide decode — same-pair and
  defender views now share the one buggy projection; fixing it fixes both) —
  audit the XYZ rules' alerts/projection; (3) `1S (1N) X` shows 15+ on the
  wrong seat (1/1, attribution suspect); (4) ~~passes narrow nothing~~ —
  **BUILT**, see the pass-reading entry above; what remains of the pass
  family is the deferred gates (their-1NT direct seat, advances,
  later-round) and the unacted-seat conservation class, which is a sampler
  property, not a reading bug.

### The bitmap-ablation probe (does honor *location* matter?)

The Info-net probe reads the *auction*; this one reads the *hand feature*. BEN's
bidder input (`model_version 3`, `n_cards_bidding 24`, 193 floats/timestep) feeds
total HCP and all four suit lengths as **explicit named scalars** — and they are
exact linear read-outs of the 24-cell hand bitmap (`get_hcp` = 4/3/2/1 dot
product, `get_shape` = per-suit sum), fed only "so the net won't build neurons
for this." So the answer to *"can BEN's hand parameters distill to strength +
each suit's length?"* is **yes, by construction** — the only residual the bitmap
carries beyond `(HCP, shape)` is **which suit each honor sits in** (honor
location; spot cards ≤9 collapse into one counter, so `KJ986 ≡ KJ432` and that
blindness is un-probeable). `scripts/ben-bitmap-ablation.py` (run with
`~/ben/.venv/bin/python`, `--selftest` for the no-TF packer/KL check) holds the
auction fixed, swaps the true hand for canonical hands of **identical HCP and
identical four lengths** with honors repacked into the long suits vs the short
suits, and measures how far the raw policy softmax moves — `pred_fun_seq`, no
`/bid` search (Tier-S DD override is a separate question). Positive control: a
random legal hand of different HCP/shape (the hand-sensitivity normalizer).

Run 2026-07-17 (`bidder BEN-21GF-8730`, NV corpus reused from the Info-net probe,
8,995/9,152 rows scored, 157 extreme-shape pack skips logged;
`ab-results/ben-info-probe/2026-07-17/bitmap-ablation.json`, seed 1784309099):
**honor location is mostly irrelevant, decisive in a thin bridge-meaningful
tail.** Median `KL_honor / KL_rand` = **0.4%** (of everything BEN reads from the
hand at a juncture, honor placement beyond HCP+shape is 0.4% at the median);
sensitivity control healthy (`KL_rand` median 1.08, p95 16.4 — the net reads the
hand hard). Material-flip rate (argmax changes *and* `KL_honor > 0.1` nats,
i.e. not a near-tie tie-break) = **3.4%**, concentrated exactly where a bridge
player expects: **opening decisions** (auction-length 1: 5.7%; strain not-yet-set:
7.9%) and **light hands** (8–11 HCP: 4.6%), near-zero for strong/constructive
hands (16+ HCP: 2%, 20+: 0%). The top tail is two clean patterns:
- **Preempt suit quality** — 5–6 HCP, `2♥`/`2♠` weak-two/preempt: honors packed
  *into* the 6-card suit keep the preempt (`KL≈0`), the same HCP scattered *out*
  of it → **PASS** (`KL 3.6–4.9`). BEN preempts only with a good suit.
- **Slam-zone control placement** — 8-to-14-call NT auctions where honor
  relocation flips `5♣↔5♥↔5♦` (`KL 4–5`): which suit to cue / where to place it.

Consequence for the floor: our authoring vocabulary (`points`,
`support_points`, `fit_sum`, `hcp`, length constraints) **is** `(HCP, shape)` and
so is a near-sufficient statistic of the *hand* for BEN's policy — the −1.9 IMP
gap lives in **search over sampled worlds + auction-state memory**, not a missing
hand feature. The one structural blind spot worth a feature is **suit quality /
honor concentration**, and only for **preempt discipline and slam cue placement**
— not a global floor term. (Ties into the `length_soundness` A/B: length alone,
without honor location, is what our reader currently trades in.)

## The reference pair

| Engine | Role | Cost | What it's for |
| --- | --- | --- | --- |
| **BEN v0.8.8.4** (BEN-21GF, stock "Tier S" / policy-only "Tier F") | **Target.** The campaign metric is pons-vs-BEN IMPs/board. | Slow (HTTP + NN + DD rollouts): 20k-board anchor ≈ overnight; 102.4k Tier-F A/B arm ≈ 5–6 h | Headline anchor (Tier S); per-fix primary A/B (Tier F); decompose |
| **BBA/EPBot** (vendored FFI) | **Guard + microscope.** | Fast: 409.6k-board arm in ~1–2 h | Exploit guard on every ship; big-sample forensics; the existing anchor series continues as a secondary metric |

**The ship rule — inverted 2026-07-18** (jdh8: *BEN is too slow for
development* — measured 0.27 s/bid; a 102.4k Tier-F arm-pair costs ~16 h
where the same A/B vs BBA is ~15 min end-to-end, generation ~90 s per
204.8k cell plus fired-boards-only diffs. Extends, never overrides,
[measurement.md](measurement.md)'s decision table — all iron rules stand):

1. **Per-fix gate**: fresh-seed A/B **vs BBA**, both brackets (plain DD +
   PD; sd-lead where the treatment is competitive/lead-shaped). Verdict
   from the standard decision table. This is the development loop.
2. **Per-batch validation**: a **vs-BEN Tier-F** A/B per milestone/batch of
   shipped fixes, arms **sized to the fired rate** (a 0.2%-firing knob
   needs 100k+ boards; a 1–5%-firing convention change reads fine at
   25.6k), plus the **periodic Tier-S anchor** as the truth metric. A batch
   that wins vs BBA but regresses vs BEN triggers forensics on the worst
   divergent boards before the next batch ships.
3. The existing self-play/advertised exploitation guard for sd verdicts
   stays as-is.

Rationale: the original primary/guard split guarded against tuning to
BEN's quirks — but gating per-fix on BBA makes BEN-overfitting impossible
by construction, and the per-batch BEN run catches the converse
(BBA-overfitting) at the same cadence the old chained-anchor discipline
did. BBA is rule-based and independent of BEN's training lineage in its
*decisions* (even though BEN trained on its output) — a genuine bridge
improvement wins or washes against both. Fleet ops: 32 Tier-F instances
(ports 8085+, ~1 GB RSS each, arenas grow ~30 KB/board; each instance ≈
one busy thread behind its bid lock, so instance count is the throughput
knob) make a sized Tier-F arm a ~1–4 h batch job.

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
recipe, which pons has all the parts for). **The concrete plan now exists:**
[ai-bidder/sound-search.md](ai-bidder/sound-search.md) (= AI-bidder
Milestone 8) — refine the *built* search (`american_search` /
`american_neural_search`) rather than build it: fix the sampler
(uniform-reject → importance-weighted, reading-tightened), fix the scorer
(single-dummy slam pricing in the offline teacher), re-distill; ordered by
leverage with a do-not-retry list. It stays gated behind Phase 1's decompose,
which decides whether the gap is search-judgment (→ Milestone 8) or shared
book/floor buckets. Also deferred: a Rust Blue Chip
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
- BEN-vs-EPBot exploit-guard runs seat the guard with
  `--their-card vendor/ben/BEN-21GF.bbsa`, so EPBot plays BEN's *declared*
  system (see the source-extracted section above) rather than stock BBA
  defaults.
- Version discipline: BEN pinned at v0.8.8.4 + config hash recorded in
  `gen_args`. BEN is actively developed; re-pinning is a deliberate
  campaign decision (it moves the target), taken at most per-milestone and
  re-anchored immediately.
- GPL boundary: BEN runs as a separate process over HTTP. Never link, embed,
  or vendor BEN code/weights into pons.

## Anchor trail (the campaign metric)

Tier S, 20k boards (8×1,250 × {none, both}), persistent
`SEED_BASE=1784237746`, series `ab-results/ben-anchor/`. Pooled = both arms.

| date | pons | plain | PD | notes |
| --- | --- | --- | --- | --- |
| 2026-07-17 | `119675f` | **−1.906** (none −1.640 [−1.736, −1.545], both −2.172 [−2.293, −2.050]) | **−1.860** (none −1.510, both −2.209) | First anchor; retires the chained ≈2.1. Divergence 71%/70% (vs 49%/46% for EPBot-vs-BEN). Reading knobs at committed defaults (off). |

**Tier-F gap** (one-time calibration, fresh seeds 1784294370, 102.4k/arm,
sha 74d783d, `ab-results/reading-knobs/2026-07-17/`): plain **−0.879**
(none) / **−1.092** (both), PD −1.122 / −1.519, divergence 71%. Vs the
Tier-S rows above: **search is worth ≈0.8–1.1 IMPs/board to BEN** at this
distance. Read Tier-F A/B deltas as directional; the Tier-S anchor stays
the truth metric (different seed bases, so cross-tier comparison carries
±0.05-ish deal-mix noise on top of the CIs).

Cross-checks: the same-era BBA anchor reads −1.68/−1.73 plain, so BEN
measures ≈0.2 harder than BBA — same sign, smaller than the naive chain
(pons−BBA −1.68 plus EPBot−BEN −0.568 ≈ −2.25) predicts; IMP transitivity
is nonlinear across deal streams, which the design doc anticipated. The
vul-both arm is ~0.5 worse than vul-none, the same skew the BBA series
shows.

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
4. ~~Where does the measured anchor land vs the chained ≈2.1?~~ —
   ANSWERED (first anchor): −1.906 plain pooled, slightly inside the chain
   (≈2.1 from the survey; ≈2.25 chaining our own calibration onto the BBA
   anchor). Sign and ballpark agree; the shortfall is ordinary IMP
   nonlinearity across protocols, not a transitivity anomaly.
