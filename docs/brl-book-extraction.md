# Extracting books from brl — probe report

**Status: probe complete; campaign CLOSED (2026-07-17). Verdict: extraction
is mechanically proven; brl's learned system is only *partially* ruly — the
≈90% root-fidelity gate for a faithful full-tree book is NOT met (root ≈58%
fitted / ≈84% ceiling), so Phase C as a rule-book replacement is a no-go in
the current DSL vocabulary. Disclosure mining, however, works and already
produced the first published description of brl's conventions. jdh8's
classification seals it: brl plays a strong-pass-family system — a **HUM**
under WBF systems policy (openings routinely weaker than Pass; Pass leaning
on values), which humans may play only in the few most prestigious events
that admit HUMs. Not digging deeper.**

brl facts and environment pins: [vendor/brl/PIN.md](../vendor/brl/PIN.md);
survey context: [open-source-bidder-survey.md](open-source-bidder-survey.md).
Harness: [`vendor/brl/dump_selfplay.py`](../vendor/brl/dump_selfplay.py)
(greedy self-play corpus, brl's own eval protocol) →
[`examples/probe-brl-book`](../examples/probe-brl-book/main.rs) (per-node
vul-flip test, box fitting under `Rules` max-weight semantics, fidelity +
expressiveness-ceiling metrics).

Corpus: 200k uniform deals × 4 vul combos = **800k boards, 10.02M decisions**
(`~/brl/corpus/brl-selfplay-200k.jsonl` + sidecar; ~10 min wall on shared
CPU, no DD anywhere). Validation: hand-layout round-trip at dump time;
legality + argmax-consistency asserts at ingest; and 40 boards (~500
decisions) rebuilt from the dumped PBN through pgx's own parsing logic
reproduce every call — load-bearing, because brl violates human priors hard
enough that "looks wrong" is not evidence of a wiring bug.

## What brl actually plays

The learned system is radically non-human. Highlights (all merged-vul unless
noted; per-call boxes and recalls in the probe output):

- **Openings are anti-monotone in HCP.** Dealer opening rate by band
  0–3/4–7/8–11/12–15/16+: **99.8% / 97.8% / 83.9% / 51.4% / 67.8%**. Dealer
  Pass (21.7% of boards) is an informative, good-hand-leaning call
  (its box: `hcp(7..=18)` flat-ish); near-yarboroughs *always* open.
- **1C is a 43.9% catch-all** (`hcp(1..=12)`, any shape, recall 91%) — closer
  to "I have a weak-ish hand" than to clubs. 1D (13.6%) is genuinely
  multi-way (single-box recall 3.6% despite an 85–89% ceiling: at least
  two disjoint hand types share the call).
- **It invented relays.** Over 1C–(X), partner redoubles 82.7% of the time
  (`XX` box is near-shapeless, recall 98%) — a forced relay; the auction
  `1C X XX` alone carries 13% of all boards. Over 1C–(P), 1D is a 67.5%
  waiting/relay response. Over a strong 2C opening (see below), 2D is a
  64.5% negative/waiting reply (`hcp(1..=10)`).
- **It reinvented Stayman/transfers.** Over its (rare, 2.1%) 1NT opening —
  which is *strong-NT-shaped*, `hcp(13..=18)` semi-balanced — the responses
  are: 2C 25.9% general inquiry (`hcp(7..=16)`, recall 94%); **2D = hearts**
  (`len(H, 4..=7)`, recall 91%); **2H = spades** (`len(S, 5..=7)`, recall
  97%); 2S weak (`hcp(1..=8)`, recall 81%).
- **Sane, extractable fragments exist**: 2C opening = strong `hcp(15..=24)`
  (recall 80%); 3C opening = natural club preempt `len(C, 6..=8)` (recall
  60%); over our 1C, opponent's X = "values with spades" (`hcp(7..=20) &
  len(S, 3..=6)`, recall 90%); over 1D–(P), responder bids majors up the
  line (1H recall 86%, 1S 69%).
- **Vulnerability conditions everything.** Paired vul-flip (same deal,
  flipped vul bits — exact, since the policy is deterministic): root 16.6%
  of openings change; over 1C the LHO's action changes 23.6% (ns~ew 41.4%!);
  even quiet nodes run 4–9%. No human system re-shapes its *constructive
  openings* by vulnerability at this magnitude.

## The numbers

Per major node: box-fidelity (held-out, vul-stratified fit), the
expressiveness ceiling (exact `(hcp, 4×len)` tuple majority, merged and
vul-in-tuple), fidelity on brl-confident rows (top-1 prob ≥ 0.9), and median
entropy. Split is by deal (all four vul variants on one side); fit-split
self-fidelity ≈ holdout everywhere (no overfitting — the fitter is
bias-limited).

| Node | mass | boxes | ceiling | ceiling+vul | confident | ent. median | vul-flip |
| --- | --- | --- | --- | --- | --- | --- | --- |
| (root) | 100% | 57.8% | 81.0% | 83.9% | 78.9% | 0.57 | 16.6% |
| `1C` | 43.9% | 69.0% | 79.6% | 86.1% | 82.1% | 0.31 | 23.6% |
| `P` | 21.7% | 54.9% | 78.6% | 82.8% | 69.6% | 0.64 | 19.1% |
| `1C X` | 20.1% | 83.4% | 91.9% | 92.9% | 95.2% | 0.07 | 7.4% |
| `1D` | 13.6% | 58.4% | 84.9% | 89.0% | 58.3% | 0.09 | 16.0% |
| `1C P` | 12.8% | 68.0% | 89.4% | 89.8% | 83.5% | 0.25 | 5.5% |
| `P 1C` | 9.6% | 51.6% | 79.1% | 83.4% | 56.3% | 0.39 | 21.3% |
| `1D P` | 6.8% | 68.1% | 84.3% | 84.4% | 81.8% | 0.20 | 7.9% |
| `2C` | 4.0% | 84.0% | 90.9% | — | 91.8% | 0.05 | 10.3% |
| `2C P` | 3.3% | 76.5% | 88.0% | — | 82.0% | 0.00 | 4.1% |
| `1NT` | 2.1% | 86.7% | 92.9% | — | 91.8% | 0.00 | 6.9% |
| `1NT P` | 1.7% | 75.6% | 87.3% | — | 84.5% | 0.02 | 5.3% |

Confident-row share: 38.5% of root decisions have top-1 prob ≥ 0.9, rising
to 56.9% at depth 1 and 65.3% at depth 2 — brl mixes hard exactly where the
catch-all calls live (its top-2 at the root are routinely 0.48/0.46-style
near-ties).

## Attribution — why fidelity stops where it does

1. **brl's own mixing** is the biggest single bite at the root. On the
   confident slice, single boxes score 78.9% ≈ the 81.0% merged ceiling —
   where brl is decisive, even the crude fitter nearly saturates the
   vocabulary. No deterministic book can (or should) reproduce near-tie
   coin-flips; for *playing strength* this slice of "infidelity" is likely
   cheap, but that exchange rate is unmeasured (see the fork below).
2. **Vulnerability** is real but secondary once in-vocabulary: the vul-aware
   ceiling buys +2.9 at the root, +6.5 at `1C`. A faithful book must be
   vul-stratified (the DSL's `vulnerable()`/`they_vulnerable()` suffice).
3. **The single-box-per-call fitter** leaves 15–30 points below the ceiling
   at multi-way nodes. Extreme case, the `1D` node: X-vs-Pass fidelity 15.4%
   non-Pass vs an 89.0% vul-aware ceiling — brl's X there is a union of
   disjoint hand types no single box can hold. The DSL supports `|`
   (union-of-boxes); that escalation has clear headroom and was deliberately
   deferred (ponytail ladder).
4. **Vocabulary** caps everything at 83–93% by node: even exact
   (hcp, shape, vul) tuples cannot say more. The remainder is honor
   placement and finer texture (the DSL's `top_honors` could recover some;
   the tuple ceiling with honors was not computed — cardinality).

Contrast with the pons precedent: `american_neural_v3` distilled *our own
rule-based book* at 95.3% top-1 from disclosable features — a rule system is
its own sufficient statistic. brl is an RL policy with genuinely fuzzy
boundaries; 57.8%-at-the-root is a statement about brl, not about the
machinery.

## Verdict and the Phase C fork

- **Gate (≈90% pooled root + healthy non-Pass): NOT met.** A full-tree
  extracted book that *replaces* the net faithfully is out of reach in the
  current vocabulary (root ceiling ≈84%).
- **Disclosure mining: works, keep.** The probe already yields alert-ready
  descriptions of the system's crisp skeleton (relays, strong 2C, transfers,
  preempt shapes) — exactly what a defense/disclosure would need if brl were
  ever seated as an opponent reference.
- **If "pons plays brl" is ever wanted**, the honest route is the in-crate
  net port (4×1024 forward pass + pgx obs encoder + fixture parity, per the
  campaign plan's Phase C notes), with the extracted book as documentation,
  not the decision engine.
- **The one open empirical question** worth a cheap future pilot: the
  fidelity→IMPs exchange rate — an extracted book (union-of-boxes fitter,
  vul-stratified) vs the raw net in a DD duplicate. Mixing-driven
  infidelity may be nearly free in IMPs; boundary-driven infidelity is not.
  Not started; it needs the net port (or a Python-side book player) and DD
  budget, and it queues behind the BEN campaign.

## Relation to the BEN campaign

None operationally: no BBA/BEN processes, no DD, no `src/` changes; the
corpus and probe are CPU-side artifacts. brl remains what the survey said —
a yardstick above BEN on the public ladder, now with its system partially
documented. The BEN campaign's priorities are unchanged.

## Reproduction

```sh
# corpus (~10 min shared-CPU; seed recorded in the sidecar)
cd ~/brl && nice -n 19 .venv/bin/python \
  /path/to/pons/vendor/brl/dump_selfplay.py \
  --deals 200000 --seed <fresh> --out corpus/brl-selfplay-200k.jsonl

# probe (debug build is fine; ~4 min)
cargo run --example probe-brl-book -- \
  --corpus ~/brl/corpus/brl-selfplay-200k.jsonl --out probe-full.md
```

This run: seed 1784227601 (sidecar `brl-selfplay-200k.jsonl.sidecar.json`),
brl `fdd958ff`, weights sha256 `63bff43e…`, pgx 1.4.0, jax 0.4.23.
