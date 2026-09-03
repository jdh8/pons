# Session 4 handoff — does the relabelling program have a production payoff?

**Written 2026-09-04**, out of the review of session 3 (`533717c1`). Session 3
built the rollout harness and used it to refute the target rule it was meant to
enable; §6 of [logit-calibration.md](logit-calibration.md) then queued a
session 4 that spends **~50 box-days of double dummy** relabelling a corpus.
This file argues that number should not be spent yet, names the two doubts that
come first, and specifies the ~2-hour experiment that resolves them.

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

Re-run the `M`-series against the population the floor actually serves. One
contained change to one example; nothing in `src/` moves.

**Spec.**

1. **Harvest** `!provenance.is_authored()` instead — the decisions where the
   book has no mass and the floor answers. Keep the `admissible.len() > 1`
   requirement (still a choice-bearing decision).
2. **The own call** stays `select_legal_call(Some(logits), &auction)` on
   whatever the policy resolved, which at these nodes *is* the floor's pick —
   so the paired baseline needs no change at all.
3. **The candidate set** becomes the net's own top-`k` over the legal calls,
   ∪ the own call. There is no book-admissible set to restrict to, which also
   means **the epsilon gate disappears**: after `mask_illegal` the mass on legal
   calls is 1 by construction, so nothing can be `thin`. Report the count as 0
   rather than deleting the column, so the two censuses stay comparable.
4. **Everything else is unchanged**: one `2M` rejection-sampled draw halved, one
   shared solve per layout, both scorers, deal-clustered 95% intervals.

**Cost.** The same shape as §4's `M`-series: ~90 s per row at 400 deals and
`M = 8`, ~25 min at `M = 128`. Budget two hours for the series plus a headline.

**Read it like this.**

| held-out result at `M = 128` | verdict |
| --- | --- |
| positive on **both** scorers, intervals excluding zero | the floor's own population carries the signal — session 4 earns its double dummy, and the corpus-fed build below is the right next step |
| PD positive, plain DD indistinguishable from zero | the same ambiguity as the authored population; treat as a doubling artifact until a single-dummy read exists, and do **not** spend the corpus pass |
| neither positive | the program is priced on nodes production does not use — park it behind a book-deletion plan and close session 4 |

Whatever it returns, record it as §4b of [logit-calibration.md](logit-calibration.md)
and add a ledger row. A negative here is as valuable as a positive: it is the
difference between spending 50 box-days and not.

## If it survives: the session-4 build, in order

1. **Corpus-fed mode.** Harvest decisions from the shipped `dump-teacher`
   corpus with **BBA's label** as the paired baseline, rather than from a
   self-play walk. This is the axis that most changes the relabel *rates*, and
   it is the only way to price §4's asymmetric revert-to-BBA fallback, which
   has no counterpart in the present probe.
2. **The vulnerability axis.** The probe hardcodes
   `AbsoluteVulnerability::NONE`. The competitive book turns on vulnerability,
   so no label may feed a retrain without it. Both vulnerabilities, per the
   measurement playbook.
3. **The selector** — cross-fitting versus a larger `M`. *This is jdh8's call*
   (see below); cost decides it.
4. **Then, and only then**: relabel a slice → fit `T` → retrain → A/B on both
   scorers, read off [../measurement.md](../measurement.md)'s decision table.

## Independent of all of the above, and cheap

**`PASS_DEMOTION` → `3·T` still has no `T`.** The demotion is 3 raw nats on the
net's logits ([`instinct.rs:3572`](../../src/bidding/instinct.rs)) — a
magnitude read on an uncalibrated scale, and the one default-path consumer of
one. Session 2 already built the held-out-NLL fitter and the sidecar field; no
shipped v6 artifact has been run through it. Running it costs a training-report
pass, not a corpus, and it is the input [plan.md](plan.md)'s M5.2 collar retune
needs before it can be sized. There is no reason for this to wait behind the
relabelling question.

## Open calls for jdh8

1. **The epsilon fallback default** — `1e-4` recommended in session 2 and
   confirmed against session 3's census; already the default of both consumers.
   Unchanged ask, recorded in §4.
2. **Cross-fitting versus a larger `M`** for the production selector.
   Cross-fitting recovers the half of the layouts the split-half control
   spends; a larger `M` is simpler and buys a smaller curse directly. Cost:
   `7.3 ms × M × decisions` either way.
3. **New, from D1:** is there a plan to delete book nodes in favour of the
   floor? If yes, the authored-node census is measuring the right population
   after all and D1 dissolves. If no, the gate-flip experiment above is the
   whole question.
