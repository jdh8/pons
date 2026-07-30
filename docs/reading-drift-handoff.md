# Handoff — why constraints and readings still disagree after DNF

**Written 2026-07-31**, out of the Gladiator audit. Two defects found in one
small convention, both of the same family, neither reachable by any check the
crate currently runs. This file names the family, prices it, and proposes the
program. It is a *diagnosis*, not a plan of record — nothing here is measured.

Prerequisites: [dnf-migration.md](dnf-migration.md) (what the DNF work did and
did not do), [reader-retirement.md](reader-retirement.md) (the hand-written
readers still standing), [ai-bidder/sampled-projection.md](ai-bidder/sampled-projection.md)
(read a call off the *bidder*, not off its rules).

## The question

DNF projection was supposed to end reader drift: instead of a hand-written
`Inferences` arm restating what a rule says, `project_authored` reads the rule
*itself* and unions its boxes. So why did the Gladiator audit still find a rule
that says `len(♦, 5..) & points(10..)` read as "six-plus diamonds, no strength
promise"?

## The answer: projection is gated on the alert

[inference.rs](../src/bidding/inference.rs), `project_authored`:

```rust
let alerted = !is_pass
    && rules.rules().iter().any(|rule| rule.call() == made && rule.alert().is_some());
let decode = (alerted && alert_reading()) || natural_reading();   // the second term is new
…
if let Some(projection) = projection.filter(|_| decode) { … }
```

A call's authored constraint reaches the reading **only if that call is
alerted**. That is deliberate and correct as disclosure — an unalerted call is
natural, and natural calls are read by the natural walk so that the reading
survives auctions the book never authored. But it means the crate has three
reading regimes, not one (`set_natural_reading`, below, is the knob that merges
1 and 2; it is off by default and unmeasured):

| regime | who reads it | drift possible? |
| --- | --- | --- |
| authored + **alerted** | `project_authored`, off the rule text | no — same expression |
| authored + **natural** | the natural walk, off auction *shape* | **yes — nothing ties them** |
| floor (no rule at all) | the natural walk | n/a — no constraint to disagree with |

Row 2 is the whole family. Every natural rule in the book is an *unverified
duplicate* of what the walk guesses from the auction's shape, and the two are
written in different files by different mechanisms. The DNF campaign did not
touch it: it replaced hand-written readers for **artificial** calls, which is
where the phantom-suit disasters were.

### Worked example (fixed 2026-07-31)

`gladiator_advances` authors the game-forcing three-level advances of our 1NT
overcall as `len(suit, 5..) & points(game..)` — natural, correctly unalerted.
The walk classified the same call as *the defending side's jumping overcall*
and stamped `len(suit, 6..)`, no points. Consequences:

- 16 of 256 sampled advancers were **excluded from their own reading** — the
  box the sampler deals against could not contain the hand that bid
  (`KQT.K8.AJT64.QJ4` bids `3♦`, read `♦ 6..=13`). A wrong box, not a loose one.
- The strength half was simply lost: a game-forcing advance read as nothing.

The fix was one predicate — teach the walk that our 1NT *overcall* takes the
same three-level reading as an opening 1NT — and the sweep
`gladiator_readings_admit_the_bidder` now runs unscoped over every advance.

## The second mechanism: a reading knob is a bidding knob

`systems_on_overcall_strip` deletes their opening from the auction so that
`[1M, 1NT, …]` reads exactly like an opening `1NT`. Gladiator turns the strip
**off** (its advances differ, so the identity fails). That is a pure *reading*
change — and it moved *calls*:

| `[1♠, 1NT, X]`, advancer | strip on | strip off (Gladiator) |
| --- | --- | --- |
| `8732.932.J973.T4` (1 HCP) | `P` | **`3♥`** |
| `932.7.QJ9764.KJ2` | `XX` | `2♦` |
| `93.874.J6.KQ9764` | `2♠` | `2♣` |

At *this* node `american_instinct()` — the deterministic floor — is identical
and sane in both columns, so the divergence is the **distilled net**, whose
features include the inference boxes: distilled with the strip in place, served
an auction picture it had never seen, it answered with a three-level escape on a
one-count.

**But it is not a net-only effect.** One node over, at `[1♠, 1NT, 3♠]`, the
*deterministic* floor diverges too — `P` with the strip off against `3NT` with it
on — because `set_inference_aware` (a foundational default-on win, plain +0.027 /
PD +0.024) makes the instinct floor consult the auction's interpretation. Any
floor that reads is a floor that a reading change can move.

So: **any** change to what a seat reads is a change to what the table bids, at
every node the book leaves to a floor. The layer cake is not one-directional any
more. Two consequences worth internalising:

1. A "reading-only" change cannot be shipped on a soundness argument. It needs
   the same A/B as a bidding change, or a book node that shadows the floor.
2. A knob that suppresses a reading (`--no-ns-rubens`, `set_alert_reading`,
   `set_dnf_reading`, the strip) is an *off-arm for the floor too*. Verdicts
   from those A/Bs are joint, never attributable to the reading alone.

That is why the first fix here was to author `gladiator_doubled_runout` rather
than to widen the strip: a finite book node shadows the floor, so the runout
stops depending on what the net makes of the picture. It fixed the advancer's
seat and measured (v5): `vs-X-escape` went 50 fired at PD −4.62/fired to 25 at
−0.44, and the treatment's whole loss became a wash.

**It did not fix the seats the book still leaves to the floor**, and the v5
forensic named them: `vs-X-pass` (the *overcaller* reopening at index 5 after
advancer passes the double — my node only covers index 3) and `contested-other`
(RHO jumping to the three level, which `insert_sohl_over` never covered). Both
are auctions where Gladiator and systems-on play *the same thing*, so the strip
identity holds there and switching it off was simply wrong. The scope is now
per-RHO-call:

| RHO over our 1NT | Gladiator plays | systems-on plays | strip? |
| --- | --- | --- | --- |
| pass | the Gladiator advances | the 1NT responses | no |
| `2♣` | the stolen relay (rebase) | systems on | no |
| `2♦`/`2M` | Transfer Lebensohl | its own sohl | no |
| **X** | a natural runout | a natural runout | **yes** |
| **3-level+** | the floor | the floor | **yes** |

Pinned by `gladiator_keeps_the_strip_where_it_has_no_structure`. The general
lesson is the narrower one: a strip is a claim that *two structures coincide*,
and it should be scoped to the auctions where they actually do — not to a whole
convention.

## The other three known leaks (unchanged, listed for completeness)

- **`Points::project` is floor-only** ([constraint.rs](../src/bidding/constraint.rs)).
  No `points(..X)` ceiling survives projection, so every alerted call reads with
  an unbounded top. Fixing it is a book-wide reading change → its own A/B.
- **Negations project nothing.** `!flat_4333()` and friends contribute ⊤, so a
  box is always looser than its rule. Sound, so not a defect — but it means
  "the projection *is* the rule" is false even in regime 1.
- **`Fallback::classify` is a projection blind spot** (dnf-migration.md). A
  classifier that rewrites logits — e.g. Gladiator's stolen-relay transplant —
  has no rules to project.

## What this costs, and how to find out

The ambient number already exists: `probe-reading-sound` measures how often a
seat's box **excludes the hand that actually holds the cards**. Last run
(deviation-panel.md): **8.2 / 8.3%** for LHO/RHO, **3.3%** for partner — and
partner's 3.3% is recorded as an *open defect*, because our own partner's calls
are the ones we author. Every regime-2 rule is a candidate contributor. Nobody
has attributed that 3.3% to specific nodes.

## Proposed program (in order, each cheap and independently useful)

1. **Attribute the 3.3%.** Extend `probe-reading-sound` to bucket exclusions by
   `(node, call)` and dump the top offenders. This is a report, not a change;
   it turns a scalar into a work-list. Nothing below is worth doing before it.
2. **Generalise the behavioural sweep.** `gladiator_readings_admit_the_bidder`
   is the right shape and is 40 lines: seed hands, replay the *bidder* at a
   node, assert `Inferences::admits(Partner, hand)`. Promote it to a
   table-driven invariant test over a list of nodes, seeded from step 1's
   work-list. It is the only check that catches regime 2 — the static
   `authored_rules_eval_within_projection` compares a rule to *its own*
   projection and is blind to a walk that never consulted the rule.
3. **Then, per offender, choose one of two repairs** — and say which:
   - the walk is wrong for a whole auction shape (today's case) → fix the walk;
     it costs one predicate and fixes every convention sitting on that shape;
   - the rule is genuinely conventional → alert it, and let regime 1 read it.
     Cheaper to write, but it changes disclosure and the `.bbsa` cards, so it
     is an A/B, not a desk fix.
4. **Regime 2 projects on a knob now — `set_natural_reading`, default off**
   (`bba-gen --ns-natural-reading`). Built 2026-07-31 on jdh8's call that *every*
   call should be read by DNF. Two implementation choices worth knowing before
   measuring it:

   - It **intersects with** the walk rather than replacing it: an unalerted call
     does **not** set its suppression bit. Suppressing a natural call would
     delete the walk's lane bookkeeping — natural-suit masks, agreed fits, the
     cue detection later calls depend on — which is a far bigger change than the
     reading itself.
   - So where the walk is *wrong*, the box can go **empty**: a wrong walk claim
     intersected with a sound rule claim is still wrong, and now visibly so. That
     is a *diagnostic* — it converts silent regime-2 drift into a hard failure
     the `admits` sweep catches — but it means step 1's work-list should be
     cleared before the knob is measured, not after. `set_natural_reading(true)`
     is the cheapest instrument for step 1 there is.

   Still the most dangerous change in this file, and still unmeasured: it
   tightens thousands of readings at once, and per dnf-migration.md's C1 finding
   a tightening that moves *endpoints without mass* is close to pure feature
   perturbation for the frozen nets. Expect it to need the knob-matched
   evaluator twin (F2b), exactly as `set_dnf_reading` did — the bare flip lost
   there and the twin is what made it ship.

## Do not conclude from this

- That the DNF campaign missed something. It scoped itself to artificial calls
  and closed that scope; regime 2 was never in it.
- That the walk is bad code. It is the only layer that reads auctions nobody
  authored, which is most of them, and it has to answer without a rule.
- That today's two fixes are measured. They are not. `set_nt_overcall_gladiator`
  is still filed as a **loss** on numbers taken *before* both, and the diagnosis
  of that loss (≈40% in the `(X)` branch) is exactly what the runout node
  addresses — so the treatment is owed a re-measure before its row means
  anything. See [bidding-options.md](bidding-options.md).
