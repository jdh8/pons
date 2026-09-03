# Session 4 handoff — does the relabelling program have a production payoff?

**Written 2026-09-04**, out of the review of session 3 (`533717c1`). Session 3
built the rollout harness and used it to refute the target rule it was meant to
enable; §6 of [logit-calibration.md](logit-calibration.md) then queued a
session 4 that spends **~50 box-days of double dummy** relabelling a corpus.
This file argues that number should not be spent yet, names the two doubts that
come first, and specifies the ~2-hour experiment that resolves them. **Refined
2026-09-04** against the floor shell's code: the population to flip to is the
**net-served** slice, not "unauthored", and the proposal is the shell's own logits.

Prerequisites: [logit-calibration.md](logit-calibration.md) §4, §4a and §6 (the
harness, the opponent arm, the ledger), [../measurement.md](../measurement.md)
(the decision table and DD's blind spots),
[../bidding-architecture.md](../bidding-architecture.md) (the shadowing
invariant this file leans on).

## Where session 3 left it

Settled, and not worth reopening:

* **The in-sample target rule must not be retrained on.** At `M = 32` its
  +0.2499/+0.5434 DD/PD shrinks to +0.0362/+0.2446 held out; the both-scorer
  relabel rate falls 20.98% → 11.01%. A bigger margin does not debias reusing
  the selection layouts.
* **`M` and independent validation are separate constraints.** At `M = 8` the
  held-out scorers disagree in sign; at `M = 128` both are positive and the
  curse has collapsed to ~0.06.
* **The rollout opponent does not matter** (§4a, added by the review).
  Self-play and BBA give overlapping held-out intervals at 1.46× wall clock.
  *Do not spend another arm on this.*

Still open when the review started: the production label gate and its `M`, the
vulnerability axis, and — added by the review — a corpus-fed mode, because the
probe displaces *our book's argmax on our self-play auctions* while §4's rule
displaces *BBA's one-hot on the BBA corpus*.

## The two doubts that come before any of that

### D1 — every decision in the census is one production never asks the net about

[`probe-rollout-label.rs:312`](../../examples/probe-rollout-label.rs) gates the
harvest on `provenance.is_authored()` — the book answered this hand. But
`OrElse::classify` ([`compose.rs:74-79`](../../src/bidding/compose.rs)) takes
the book's logits whenever they have mass and only *then* falls through to the
floor. So at **100%** of the harvested population, production bids the book's
call and the net is never evaluated.

Relabelling those decisions changes the net's weights, but the production
policy at those nodes is invariant to it. The payoff would have to arrive
entirely through **generalisation to unauthored nodes** — the ones the floor
actually serves — and the census measures that at zero decisions.

This is not necessarily wrong. It is coherent if the plan is to delete those
book nodes and hand the positions to the floor (the iron rule's "to give the
floor a position, delete the node"). But no such deletion is planned or
measured, so as it stands the +0.105 IMPs/decision at `M = 128` is **not a
production number**, and 50 box-days would be bought against it.

### D2 — Pass as the plurality relabel target is DD's signature, not a finding

[../measurement.md](../measurement.md)'s iron rule: DD is blind to obstruction
and concealment. An oracle that cannot price what a bid conceals will
systematically prefer passing. §4's headline node table has Pass as the
plurality target among its top swaps (56, ahead of 1NT's 36), and PD runs about
3× plain DD throughout.

The doc correctly marks that breakdown as exploratory and not held-out
validated. The risk it names is concrete: the thing 50 box-days buys could be a
corpus taught to pass more often by a scorer that cannot see the alternative.
That is the shape of a losing A/B, and it is cheaper to rule out than to
discover after the retrain.

## Do this first: flip the gate (~2 hours of box time)

Re-run the `M`-series against the population the net actually serves. One
contained change to one example plus one visibility widening in `src/`; no
bidding moves.

### The population is narrower than "not authored"

`!provenance.is_authored()` is necessary but not sufficient. The floor the
root fallback resolves to is **three floors**
([`common.rs:63-91`](../../src/bidding/common.rs), `with_floors`):

| where the unauthored decision lands | who answers | net evaluated in production? |
| --- | --- | --- |
| constructive book (`Phase::Constructive`) | the `instinct()` ladder | **no** |
| contested book, `forced(context)` true | the same ladder, as a rail ([`neural_floor.rs:145`](../../src/bidding/neural_floor.rs)) | **no** |
| contested book, not forced | `classify_bba_v6` → `mask_illegal` → `competitive_gate` → `new_suit_gate` | **yes** |

Only the third row is D1's population. Harvesting the other two would repeat
D1's mistake one layer down: relabel a decision, retrain the net, and production
still bids the ladder. So the harvest predicate is

```text
!provenance.is_authored()
  && Phase::of(&auction) != Phase::Constructive
  && !forced(&context)
  && admissible.len() > 1
```

`Phase::of` is already `pub` (`pons::bidding::book::Phase`). `instinct::forced`
is `pub(crate)` ([`instinct.rs:4284`](../../src/bidding/instinct.rs)) and must
be widened to `pub` — the same move session 3 made for `select_legal_call`, a
visibility change with a byte-identical census, not a bidding change. The
example also needs the `Context` the shell saw; `Partnership::prefixed_context`
already builds it (the present probe uses it for the features).

### The proposal is the shell's logits, not the raw net's

At a net-served node the `Logits` that `classify_with_provenance` returns **are**
the production distribution: the net's logits after the legality mask and both
gates. Use them as the proposal directly, instead of calling `classify_bba_v6`
again. Three things follow:

1. **The own call needs no change.** `select_legal_call(Some(logits), &auction)`
   is the shell's argmax, which is what production bids.
2. **The candidate set is the shell's top-`k` ∪ {own}.** A call the gates veto
   is `-∞` and can never be a candidate — correct, because a retrained net's
   preference for it could not reach production through the same gates either.
   `PASS_DEMOTION` is inside this proposal too, so the census prices the shell
   as shipped.
3. **The epsilon gate disappears and `flat` nearly does.** After the mask the
   legal mass is 1, so nothing is `thin`; with `k = 3` and at least two finite
   calls the set always has an alternative, so `flat` counts only gate-starved
   nodes. Report both as 0 (or the small count) rather than deleting the
   columns, so the two censuses stay comparable.

The raw-net-versus-shell question (are the gates costing IMPs?) is a *separate*
arm with its own retrain-free payoff; it is not this experiment and should not
be folded into it.

### Vulnerability is one flag, not a phase

The probe's `Shared.vul` is hard-coded to `NONE`
([`probe-rollout-label.rs:488`](../../examples/probe-rollout-label.rs)). The
net-served population is *contested by construction*, and the competitive book
and `competitive_gate` both read vulnerability, so the axis matters more here
than it did for the authored census. Add `--vul {none,both}` (the measurement
playbook's two cells) and run both. It is a two-line change; do not defer it to
the session-4 build.

### Size the run before spending it

The authored census had 1.60 choice-bearing decisions per deal and reached
±0.037 DD at `M = 128` on 631 decisions. The net-served density is unknown and
probably lower (contested, unauthored, unforced). Run the `M = 8` row first
(~90 s at 400 deals) and read `rolled out N` off the header; scale `-c` so the
`M = 128` rows carry **≥ 600 decisions**. If that needs more than ~1,500 deals,
the population is thin enough that the whole program is small, which is itself
an answer — record the density.

### Add the D2 check to the report

The node table's "top swap" column is in-sample. Add one held-out line per
scorer: the share of **Pass** among held-out relabel targets versus its share
among own calls at the same decisions. Read it as: Pass over-represented in the
PD-only relabels but not in the both-scorer relabels is the doubling-artifact
signature; Pass over-represented in both is the DD blind spot and a reason to
distrust the sign regardless of the interval.

### Cost and schedule

Per row the shape is §4's (`7.3 ms × 2M × decisions`, DD ≥ 99% of clock).
Sequence: `M = 8` sizing at `none` → `M = 128` at `none` → `M = 128` at `both`
→ `M = 32` at `none` only if the two `M = 128` rows disagree. Budget two hours
on the box; `scripts/idle-run.sh`, one arm at a time, one `SEED_BASE`.

### Read it like this

| held-out result at `M = 128`, both vulnerabilities | verdict |
| --- | --- |
| positive on **both** scorers, intervals excluding zero, D2 line clean | the net's own population carries the signal — session 4 earns its double dummy, restricted to the net-served slice (below) |
| PD positive, plain DD indistinguishable from zero | the same ambiguity as the authored population; a doubling artifact until a single-dummy read exists. Do **not** spend the corpus pass |
| positive on one vulnerability only | the signal is in the competitive book's vulnerability axis, not the calibration; hand it to the competitive-book campaign as a floor-gap report and close session 4 |
| negative or zero on both | the shell's argmax is already the best of its own top-3. The relabelling program is priced on nodes production does not use — park it behind a book-deletion plan and close session 4 |

Whatever it returns, record it as §4b of [logit-calibration.md](logit-calibration.md)
and add a ledger row. A negative here is as valuable as a positive: it is the
difference between spending 50 box-days and not.

## If it survives: the session-4 build, in order

1. **Corpus-fed mode, on the net-served slice only.** Harvest decisions from
   the shipped `dump-teacher` corpus with **BBA's label** as the paired
   baseline. Tag every corpus decision with the *same three-way predicate*
   (our `authored_at`, `Phase`, `forced`) and report the payoff per slice, but
   **spend the relabel budget on the net-served slice first**. The 50 box-days
   priced the whole 2.3M choice-bearing population; the slice production
   consults the net on is the fraction that can pay, and the sizing run above
   measures that fraction. Labels at authored nodes still shape the net's
   weights, so they are not worthless — they are second in line, and priced
   separately.
2. **Vulnerability** is already in the probe by then; the corpus carries it per
   row. Keep both cells separate through the retrain's A/B.
3. **The selector** — cross-fitting versus a larger `M`. Still jdh8's call.
   The gate-flip should run with the present split-half control; do not
   decide the selector before the population is known, because the cost of
   either is `7.3 ms × M × decisions` and `decisions` is what step 1 shrinks.
4. **Then, and only then**: relabel the slice → fit `T` → retrain → A/B on
   both scorers, read off [../measurement.md](../measurement.md)'s decision
   table, both vulnerabilities.

## Independent of all of the above, and not quite free

**`PASS_DEMOTION` → `3·T` still has no `T`**, and the claim that fitting one
costs "a training-report pass" is optimistic. The demotion is 3 raw nats
subtracted inside `competitive_gate`
([`instinct.rs:3705`](../../src/bidding/instinct.rs)) — on the net's logits,
and only when `competitive_accountant` is pinned on. Session 2's fitter
(`trainer/src/calibrate.rs`) runs at the *end of a training job*; the trainer
has no load-weights-and-evaluate mode, so it cannot be pointed at the shipped
`american_bba_v6.f32`. Two ways to get `T` for v6, pick by size:

* **A pons example** (`probe-v6-temperature`): read the v6 held-out split
  (the slice named in [../pdd-bank-ledger.md](../pdd-bank-ledger.md)), run
  `classify_bba_v6(features_v6(..))`, and minimise held-out NLL over `log T`
  with a 1-D golden-section search — the fitter is ~30 lines and `calibrate.rs`
  is the reference. Needs a reader for the dump format in `examples/`.
* **A `--weights-in` flag on the trainer** that skips training and runs the
  existing fit + sidecar write. Smaller diff if the trainer's data loader
  already reads the v6 dumps, which it does (`data::load_mixture`).

Either is an afternoon, not a corpus, and the result is the input plan.md's
M5.2 collar retune needs. It does not wait behind the relabelling question.

## Open calls for jdh8

1. **The epsilon fallback default** — `1e-4` recommended in session 2 and
   confirmed against session 3's census; already the default of both consumers.
   Unchanged ask, recorded in §4. Note the gate-flip experiment has no epsilon
   at all, so it neither confirms nor moves this.
2. **Cross-fitting versus a larger `M`** for the production selector. Cost:
   `7.3 ms × M × decisions` either way; `decisions` is now the net-served
   slice, so re-price after the sizing run.
3. **From D1:** is there a plan to delete book nodes in favour of the floor?
   If yes, the authored-node census is measuring the right population after
   all and D1 dissolves. If no, the gate-flip experiment above is the whole
   question.
4. **New:** widening `instinct::forced` to `pub` so an example can see the
   shell's rail decision. Precedent is `select_legal_call` in session 3;
   the alternative is a `Provenance` field naming which floor answered, which
   is a `src/` design change and should not ride on this experiment.

## What it returned

Run 2026-09-04, seed 1, `scripts/idle-run.sh`, one arm at a time; ~76 minutes of
box time against the two-hour budget. Full write-up in **§4b** of
[logit-calibration.md](logit-calibration.md); ledger row added to §6.

* **Verdict: the first row of the table above.** Both `M = 128` net-served cells
  are positive on both scorers with intervals excluding zero — held-out DD
  **+0.6134 ± 0.0821** (none) and **+0.6841 ± 0.1066** (both), PD
  **+0.4641 ± 0.0874** and **+0.5626 ± 0.1011** — and the two cells overlap, so
  the signal is not the vulnerability axis. That is ~6× §4's authored `M = 128`
  held-out DD of +0.105. The `M = 32` tie-break row was not needed.
* **D2 is answered, and it inverts.** On the *authored* population the fear is
  confirmed: 1 of 631 decisions passes, yet Pass is the target of 46.7% of
  plain-DD and **64.4%** of perfect-defense relabels. On the *net-served*
  population ~80% of decisions already pass and the relabels flow the other way
  (DD targets Pass on 9-13%, displaces a Pass on 74-78%). Plain DD is also the
  *larger* scorer here, reversing §4's 3× PD ratio — the opposite of the
  doubling-artifact signature.
* **The sizing guess was backwards.** The net-served slice is **4.8 decisions
  per deal**, three times the authored census's 1.60, so the `M = 128` rows ran
  at `-c 200`. Re-price the ~50-box-day figure against this before spending it.
* **The `T` question is closed independently.** `--weights-in` on the trainer
  (the smaller of the two options, per this file) fits `T` for a shipped
  artifact: **`T` = 1.1298** for `american_bba_v6`, so `PASS_DEMOTION` → `3·T` =
  **3.389 nats**. NLL 0.3010 → 0.2984, ECE 0.0117 → 0.0015.
* **Open call 4 is implemented as recommended** — `instinct::forced` is now
  `pub`. The refactor is proven inert on §4's population: the authored arm
  reproduces §4a digit-for-digit (631 decisions, −0.1040 ± 0.0891 DD,
  +0.1583 ± 0.1237 PD). Calls 1-3 remain jdh8's, and call 2 in particular should
  be re-priced now that the slice's size is measured.
* **One correction to this file.** D1 cites `OrElse::classify`
  (`compose.rs:74-79`) as the shadowing mechanism. `Bidder::or_else` has no
  non-test caller; `american()` attaches its floor as a **depth-0 root fallback**
  through `common.rs`'s `with_floors`, resolved by `Trie::classify_floored`. The
  behaviour D1 describes — the book's finite mass shadows the floor — is exactly
  right; only the citation names the wrong seam.
