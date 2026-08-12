# Reading drift — diagnosis and campaign ledger

**Written 2026-07-31**, out of the Gladiator audit. Two defects found in one
small convention, both of the same family, neither reachable by any check the
crate currently runs. This file names the family, prices it, and proposes the
program; the [ledger](#campaign-ledger) at the bottom tracks the campaign that
took it up (same day).

Prerequisites: [dnf-migration.md](dnf-migration.md) (what the historical DNF campaign did and
did not do), [reader-retirement.md](reader-retirement.md) (the hand-written
readers still standing), [ai-bidder/sampled-projection.md](ai-bidder/sampled-projection.md)
(read a call off the *bidder*, not off its rules).

## The question

Envelope-union projection was supposed to end reader drift: instead of a hand-written
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
`(1M) 1NT …` reads exactly like an opening `1NT`. Gladiator turns the strip
**off** (its advances differ, so the identity fails). That is a pure *reading*
change — and it moved *calls*:

| `(1♠) 1NT (X)`, advancer | strip on | strip off (Gladiator) |
| --- | --- | --- |
| `8732.932.J973.T4` (1 HCP) | `P` | **`3♥`** |
| `932.7.QJ9764.KJ2` | `XX` | `2♦` |
| `93.874.J6.KQ9764` | `2♠` | `2♣` |

At *this* node `american_instinct()` — the deterministic floor — is identical
and sane in both columns, so the divergence is the **distilled net**, whose
features include the inference boxes: distilled with the strip in place, served
an auction picture it had never seen, it answered with a three-level escape on a
one-count.

**But it is not a net-only effect.** One node over, at `(1♠) 1NT (3♠)`, the
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
   `set_envelope_union_reading`, the strip) is an *off-arm for the floor too*. Verdicts
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
  (Made bids only: passes fold through `project_band`, which keeps both bounds.)
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
   call should be read by its envelope union. Two implementation choices worth knowing before
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
   evaluator twin (F2b), exactly as `set_envelope_union_reading` did — the bare flip lost
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
  *(Stale since the v5/v7 re-measures, same day: the row is now filed as
  opt-in **WASH**, mechanism fixed — the owed re-measure happened.)*

## Campaign ledger

**Opened 2026-07-31.** Scope fixed with jdh8: steps 1–3 of the program above;
step 4 (measuring `set_natural_reading` as a default, with its knob-matched
evaluator twin) deferred to its own campaign. Repair policy this pass: **walk
fixes only** — genuinely-conventional offenders are *filed* as alert-candidates,
not alerted. All walk fixes ship through **one batched A/B** (non-loss ships
default-on; a loss gets its divergent boards traced first). Sweep rows land
only together with the repair that makes them green.

### Instrument (step 1 — done)

`probe-reading-sound` grew the missing pieces (step 1 of the program was
already substantially built — the probe bucketed by `(seat, prefix)` before
this campaign, contrary to the freshly-written section above):

- `--ns-natural-reading` runs the census under the regime-2 knob;
- every cell counts **both predicates** — `Inferences::admits` (the table
  reading every in-crate consumer sits on; the invariant's predicate) and
  `announced_union().contains` (the recorded baseline). Under default knobs the
  two are byte-identical (the announce overlay is a clone of the projection
  overlay until `set_announced_reading` is on), which the first run confirmed;
- the worklist buckets **partner only** — every partner offender is a node we
  author, so the whole list is actionable.

**Follow-up filed:** an opponent-seat worklist (LHO 8.26% / RHO 8.49%,
jumping to 10.73% / 11.02% under the knob) — our readings of *their* calls,
a mix of defensive-book drift and foreign-system noise, not worked this pass.

### Attribution (10k deals, seed 20260731, BBA 2/1 opponents)

| regime | LHO | partner | RHO | distinct partner nodes |
| --- | --- | --- | --- | --- |
| knob off | 8.262% | **3.156%** | 8.492% | 12756 |
| knob on | 10.732% | **3.241%** | 11.023% | 12910 |

The partner worklist is **families, not a flat tail** — the top offenders
cluster by mechanism (share of the 1338 knob-off partner exclusions in the
top-40: ~20%, but each family covers many tail keys):

| family | exemplar (excluded/readings) | wrong claim |
| --- | --- | --- |
| preempt raise to game | `3♥ - 4♥` (13/13) | points capped 1..=11 — the to-make raise excluded |
| preference/raise of a shown 5-6 suit | `1♠ - 1NT - 2♦ - 2♠` (10/11), `…2♥ - 2♠` (8/8), `…3♠ - 4♠` (11/14) | support 3.. — doubleton preference excluded |
| rebid inflation through artificial calls | XYZ `…2♣ - 2♦ - 2♠` (12/14), transfer `1NT - 2♦ - 2♥ - 3♦` (8/8) | ♠6.. / ♦6.. — the relay counted as a natural suit bid |
| opener's X of RHO's overcall | `1♦ - 1♥ (1♠) X` (9/9) | support-double stamp (♥3..=3, 10..=21) the bidder never matches |
| cue raise | `1♥ (1♠) 2♠` (8/15) | hull ⊤ but an envelope-union box excludes — projection-level |
| strip node, both-majors 3♦ | `(1♠) 1NT - 3♦` (sweep catch) | reads walk-natural ♦5.. instead of the alerted both-majors; floor also bids it on 5 HCP |

### Invariant (step 2 — done)

`readings_admit_the_bidder` (src/bidding/inference.rs): table-driven, default
knobs, both regimes, 256 seeded hands per node replayed through the *bidder*.
Landed with rows for openings (1st/2nd seat), responses to 1♥/1♠, and the
systems-on 1NT-overcall advance and runout nodes — the advance row is what
caught the both-majors 3♦ defect above on its first run.

### Verification (probe re-run, same 10k deals/seed, after all repairs)

| regime | LHO | partner | RHO |
| --- | --- | --- | --- |
| knob off | 8.262% → 7.517% | **3.156% → 2.358%** | 8.492% → 7.665% |
| knob on | 10.732% → 10.166% | **3.241% → 2.430%** | 11.023% → 10.360% |

A quarter of partner's ambient exclusion rate attributed and repaired; the
opponents improved ~0.7-0.9 points free of charge (the projection-context and
strip fixes serve their calls too). Every repaired family left the worklist
top. The tail is now flat: the top cells are the bare openings at 1.2-1.95%
*rate* — an opening's bucket also collects its interference variants
(`1♥ (X)`, `1♥ (2♣)`, … all key as `1♥`), so that family is diffuse and
unattributed — and everything else sits at ≤9 counts per 42k readings.
Named-but-unrepaired (filed above and by the triage notes): the minor raise
`1♣ - 2♣` (9/18), Michaels advances `(1♥) 2♥ - 2♠` (8/9), the sixth-card-rebid
stamp's narrow excuse (`1♠ - 3NT - 4♠`), the cue-agreement-blind
`agreed_re_raise` (`(2♦) 2♠ - 3♦ - 3♥`), and lebensohl continuations.

### Batched A/B (launched 2026-07-31)

`scripts/ab-reading-drift.sh` — two-binary protocol (the repairs are
knobless): `bba-gen-base` (HEAD 240a573) vs `bba-gen-fix` (all repairs),
both prebuilt into `ab-results/reading-drift/` before launch so nothing
rebuilds in flight. 32×6400 bd/arm/vul, SEED_BASE **1785449222**, arms
sequential under `idle-run.sh`, plain + PD diffs at `--show 40`. Read the
verdict from `ab-results/reading-drift/diff.fix.vs.base.{none,both}.{plain,pd}.txt`
against the measurement.md decision table: **non-loss ships** (the repairs
are soundness corrections; a loss traces its worst divergent boards first).

**Verdict (read 2026-07-31, 204800 bd/arm/vul):**

| vul | plain DD | perfect-defense |
| --- | --- | --- |
| none | −0.0013 [±0.0044] wash | **−0.0060 [±0.0052] loss** |
| both | −0.0031 [±0.0053] wash | −0.0016 [±0.0061] wash |

Fired 2.50%/2.27% (none/both). Three washes and one just-significant PD
loss at vul none — not the clean non-loss the rule ships on, so the worst
divergent boards were traced (iron rule) before concluding.

**Trace: the readings are right; the floor spends them in an unauthored
lane.** The dominant motif (9 of the loss cell's 40 worst carry an on-arm
4NT; the raise-shape family covers ~⅔ of the tail) is contested auctions
where a competitive raise (`1♦ (2♣) 2♠ - 3♠`, `1♥ (2♠) 3♠`, `2♥ - 3♥`) now
reads as real support — the fix arm's floor sees the fit-plus-values it was
previously blind to and drives slam via 4NT in lanes with **no authored
keycard machinery**. The continuation shatters in every way the iron rule
predicts of an unread artificial call: 4NT passed out, the 5♦/5♣ answer
passed out and played, a 5♥ signoff pulled to a dead 6♣, a doubled answer
redoubled and left in (`5♣ (X) XX`). A second motif is higher part-score
competition/sacrifices that perfect defense punishes (five doubled-contract
`XX` left-ins among the worst 40). A minority are the mirror image — the
base arm reaching a good slam the fix arm now declines. The loss
concentrates at vul none (cheap to over-compete) and in the PD scorer
(which punishes every overreach); plain DD calls all four cells a wash.

Disposition of round 1: **superseded** — the exposed defect was the
contested-4NT floor lane (an intra-floor phantom convention), not any
reading. Reverting provably-correct readings to hide a floor hole would be
backwards; the lane was authored instead and the batch re-measured.

### The keycard-window rail (authored 2026-07-31, joins the batch)

4NT in competition with an agreed suit is RKCB, not quantitative (jdh8's
call, and every worst board agrees — no contested 4NT in the tail was
quant). The M6.4 machinery already knew how to run the conversation but was
gated on whole-auction `undisturbed()`, so in contested lanes the net
freewheeled it. The repair, in `src/bidding/instinct.rs` +
`neural_floor.rs`:

- **Window, not auction, must be quiet** (`opponents_quiet_since`): the
  three continuation gates (`keycard_asked`, `keycard_answered`,
  `respect_keycard_signoff`) now tolerate any contested prefix and their
  X *inside* the window; only their bid inside it stands the machinery
  down. The ladder's own 4NT *ask* stays undisturbed-gated — contested
  asks remain the net's judgement; we only complete them.
- **Delegation** (`keycard_conversation_now`, a new `forced()` arm): a live
  keycard window — partner's decodable 4NT, partner's 1430 answer to ours,
  partner's placement over our answer — is forced-rail territory, so
  the distilled floor shell (`ConfiguredFloorBba` since 2026-08-05, and
  `NeuralFloorBba` when this was written) hands it to the deterministic ladder
  instead of the net.
  Auction-determined like the other rails; shares the window helper with
  the machinery's gates so the two cannot disagree.
- **`raised_major`** — the trump derivation that works when readings are
  vacuous: a major bid by *both* members of our side (opponents' suits
  excluded) is agreed on the auction's face, no hand, no readings. Needed
  because the contested walk stamps nothing for a free bid yet (`1♦ (2♣)
  2♠ - 3♠ - 4NT` read partner ♠ `0..13` — a **coverage** hole the
  exclusion probe cannot see, filed below). `keycard_answered` now derives
  through the same `answer_trump` ladder as the answerer — sharing the
  function is the only both-seats-agree guarantee.
- **The six rung re-checks the slam entry**: decoding an *unvetted* ask
  into an automatic "one missing → six" converted every frivolous net 4NT
  into a slam (board 1's 10-count asker would have hoisted 6♠ off 25
  combined). Six now requires `slam_entry_reached()`; the five-level
  signoff/pass rungs widened to `keycard_total(..)` as the catch-all
  placement. One shipped pin adjudicated accordingly
  (`floor_asker_continues_after_the_answer`: a 15-count opposite a limit
  raise signs off at five now; the small slam needs the entry in hand).

Pinned by `contested_keycard_window_answers_and_places` (the board-1 lane:
1430 answer, placement over their X — never a redouble — and the respected
signoff), `keycard_conversation_is_forced_rail_territory`, and the shell's
`keycard_window_delegates_in_competition`.

New follow-up filed: **contested free bids read as nothing** — partner's
`2♠` over `(2♣)` and even our own `3♠` raise stamp no length in the bare
walk (all `0..13`). Soundly vacuous, so `probe-reading-sound` cannot rank
it; the cost is coverage (the reading program's priced axis), and the
keycard rail had to route around it with `raised_major`.

### Batched A/B, round 2 (2026-07-31 — all four cells wash; one rail defect traced and gated)

Same two-binary protocol, fresh dir `ab-results/reading-drift-2/`:
`bba-gen-base` (HEAD 240a573) vs `bba-gen-fix` (seven reading repairs +
the keycard-window rail), SEED_BASE **1785481388**, 32×6400 bd/arm/vul,
arms sequential under `idle-run.sh`, plain + PD.

| vul | plain DD | perfect defense |
| --- | --- | --- |
| none | −0.0012 [±0.0051] wash | −0.0049 [±0.0060] wash |
| both | −0.0025 [±0.0061] wash | −0.0023 [±0.0070] wash |

Fired 3.00 % / 2.69 %. The round-1 PD-none *loss* (−0.0060 [±0.0052]) is
gone — the rail did its job on the disaster lane. But the PD worst-board
lists exposed a defect **in the rail itself**: the `answered`/`placed`
window shapes lacked the decodability gate the `asked` shape carried. On
contested minor auctions with `2NT` interference (for example, a line beginning
`1♦ … 3NT` and ending `4NT - 5♥`), the
shell hijacked the asker's rebid, `answer_trump` decoded nothing, no
continuation rung fired, and the ladder **passed out the 1430 answer** —
5♥ in a diamond deal — where the base net bid the making minor slam
(three of the four worst PD-none boards, −20 to −21 each). The
"slightly over-matching is safe" assumption was measured false.

**Repair:** all three shapes of `keycard_conversation_now` now share one
`recognizable(ask_index)` gate — not an opening, not over the asker's
side's own notrump (quantitative), and trump-decodable (`raised_major`
on the auction's face, or a genuinely shown five-card major). An
unrecognizable 4NT is judgement all the way down; the net keeps the
node. Pinned in `keycard_conversation_is_forced_rail_territory`.

### Batched A/B, round 3 (2026-07-31 — non-loss in all four cells, SHIPPED)

`ab-results/reading-drift-3/`: base 240a573 vs seven repairs + rail +
recognizability gate, SEED_BASE **1785482709**, same protocol (base
binary reused from round 2 — same commit).

| vul | plain DD | perfect defense |
| --- | --- | --- |
| none | +0.0007 [±0.0050] wash | −0.0010 [±0.0058] wash |
| both | +0.0049 [±0.0058] wash | **+0.0067 [±0.0067]** edge-of-win |

Fired 2.91 % / 2.58 %. The 5♥-passout signature is gone; per the
decision rule (soundness corrections, non-loss ships) the batch is the
new default — knobless code corrections, nothing to toggle.

One worst board is evidence for the filed vacuous-readings follow-up,
not a gate defect: `2♥ (X) 3♥ (X) - (4♠) - (4NT) - (5♥) X (XX)` (−23). Partner's
jump to 4♠ after two doubles shows spades to any human, but the
contested walk stamps nothing and no raise exists on the auction's
face, so `recognizable` was blind, the window stayed judgement, and
the net redoubled the doubled answer. Stamping contested free bids
(the coverage follow-up) would have put the ladder on this board —
the rail's reach grows exactly as that hole closes.

### Experiment: the face-trump rung (launched 2026-07-31, verdict pending)

jdh8's design, from the round-3 XX board: the trump derivation gains a
final **auction-face** rung, `face_trump(auction, ask)` — (1) the known
fit, a suit both members of the asking side bid below the ask (their
named suits excluded, most recent agreement wins; fit precedence keeps a
control bid from masquerading — `1♠ - 3♠ - 4♣ - 4NT` asks in spades),
else (2) **the side's last bid is a suit** → that suit (`… (X) 4♠ - 4NT`
asks in spades, `1♦ - 4NT` in diamonds; a notrump last bid vetoes —
quantitative — and a cue of their suit is no trump). Hand-independent
and keyed on the physical ask index, so every seat and the rail derive
identically; readings-independent, so it works exactly where the
contested walk is vacuous (the XX board becomes rail territory with no
coverage repair needed). The pre-experiment derivation stays above it
(hand-seen fit, shown-five readings — the branches that see through
transfers and splinters, where the face mislabels the artificial call),
and the explicit opening/quantitative gates stay. It generalizes
`raised_major` to all four suits and subsumes it; minors now qualify
(the asker rungs were already built per suit). A/B in
`ab-results/keycard-face-trump/`: base **3745c13** vs the rung,
SEED_BASE **1785485168**, usual protocol. This widens the rail's reach
(a bidding change, not a soundness correction), so the plain decision
table applies.

**Verdict (2026-07-31): WIN in all four cells — SHIPPED default-on.**

| vul | plain DD | perfect defense |
| --- | --- | --- |
| none | **+0.0023 [±0.0017]** | **+0.0044 [±0.0020]** |
| both | **+0.0037 [±0.0020]** | **+0.0050 [±0.0022]** |

Fired 0.23 % / 0.19 % — only the trump-derivation delta diverges, but at
**+1.0 to +2.6 IMPs per fired board** (keycard auctions are slam
boards). Plain win + PD win, both vuls: the table's best case.

Follow-up filed off the worst boards — **the cramped doubled answer**:
`1♦ - 1♥ (1♠) - (3♠) 4♦ - 4NT - 5♥ (X)` passed out (−20). The fit rule
correctly keys diamonds and the 1430 answer 5♥ is *right* (two
keycards, no ♦Q), but with two keycards missing and the answer already
past 5♦, the asker's `no_room_six` rung (weight 0.3) was outweighed and
the ladder passed partner's doubled off-fit answer. jdh8's earlier
observation is the candidate repair: after a doubled answer past
five-of-trump, escaping to six of the trump — or drifting to 5NT or
another known fit — beats playing the answer suit doubled. Needs its
own rung + A/B.

*Resolved 2026-07-31 — and the forensics **inverted this narrative***:
the fit rule did *not* key diamonds. See "The cramped doubled answer"
section below (SHIPPED, 8f71f0e).

### Experiment: the floor authors the readings — vacuous-scoped probed serving (2026-07-31, LOSS — knob stays opt-in, retrain queued)

jdh8's question, aimed at the vacuous-contested follow-up: *can the floor
author the readings — calls read as how the floor decides them, as close as
possible?* The machinery exists (`Partnership::probe`, Stage B of
docs/ai-bidder/sampled-projection.md): a probed key's box **is** "the hands
the bidder actually chose this call with", and it is the only reader that
reaches the floor's calls. What was refuted (v1, 2026-07-30) was the
*serving* — every box, both sides, tightening axes that already read;
"as close as possible" is a description virtue and a bidding vice (the
census rewards tightness, and tightness was the loss channel).

**The v2 serving** (`set_probed_vacuous_reading`, `--ns-probe N
--ns-probe-vacuous`, default off) scopes the same probed map to the
failure-free slice:

- **own-side calls only** — the probe replays our system; its boxes model
  partner correctly and opponents wrongly (v1's redouble trap ran through
  the opponents' boxes);
- **fully-open axes only** — a probed axis folds in only where the complete
  symbolic reading says nothing (`0..=37` points, `0..=13` length); an axis
  that already reads is never touched.  Latest call first, so the sharpest
  prefix fills and earlier keys leave it alone.  The fold lives at the end
  of `Inferences::read`, *after* the natural walk — masking inside
  `project_authored` judged "open" too early and tightened half-open axes
  (caught by the pin `probed_vacuous_fills_only_open_axes_on_contested_own_calls`);
- **contested prefixes only** — a key serves only from the index where both
  sides have acted.  Without this gate the first smoke (200 boards, seed 42)
  fired on **23% of boards at −0.67 IMPs/board**, all constructive net-OOD
  grand blasts (`1NT - 2♦ - 2♥ - 3NT - 7NT (X)`): filling constructive ⊤ axes shrinks
  the sampler's σ on slam auctions, the exact signature of the
  pass-exclusion retrain's worst boards.  With the gate the same smoke reads
  8% fired, +0.015 [±0.212] — competitive judgment moving in both
  directions, which is the A/B's question.

`Partnership::probe`'s fixed-point iteration now serves through whichever fold
is armed (the vacuous knob is set *before* probing), so the boxes are
fixed-pointed under the serving policy that consumes them; 100k probe
boards store 527 keys, 203 drifting between iterations.

A/B in `ab-results/probed-vacuous/`: one binary (4e544f7 + this tree — the
knob off is byte-identical, pinned), arms differ by
`FIX_ARGS="--ns-probe 100000 --ns-probe-vacuous"`, SEED_BASE
**1785493701**, 32×6400 bd/arm/vul, plain + PD.  Interpretation guard: the
nets consume readings as features, so a pre-retrain plain loss is a floor,
not a verdict (the pass-exclusion lesson) — disposition on a loss is knob
off + probe-first retrain queue, not thread closed.

**Verdict: LOSS in all four cells — the knob ships off, permanently
knob-gated until a retrain earns a re-measure.**

| vul | plain DD | perfect defense |
| --- | --- | --- |
| none | −0.0467 [±0.0086] | −0.1118 [±0.0106] |
| both | −0.0658 [±0.0104] | −0.1337 [±0.0125] |

Fired 10.40 % / 9.40 %, −0.45 to −1.42 IMPs/fired.  Real on both scorers
(not a PD doubling artifact), PD ≈ 2× plain — the doubling channel
amplifies it (7 redoubled finals in the top-40 worst alone).  The worst
boards are one mechanism throughout, and it is the pre-registered one, not
a soundness defect: the contested floor net, fed partner boxes tighter
than anything in its training distribution, **keeps acting where the base
arm settles** — reopens (`1♠ (2♣) - -` X instead of 3♠, ending 5♣ XX −21),
blasts competitive slams (one path starts `1NT (2♣)` and reaches `6♥ (X)`;
another starts in 2♠/3♣ competition and reaches 6♥ redoubled), doubles on
confidence and gets redoubled.  The exclusion retrain's σ-shrink
signature, on the contested slice where the floor net makes most of its
decisions.

Disposition per the pre-registration: **off + retrain queue** — (1) the
probe-first gate (`probe-closure-features` under this knob: kill only on
the C1 picture, endpoints moving with moments unmoved); (2) if earned, the
F2b twin recipe (dump the eval corpus knob-on, train the twin, serve it
under this knob, re-measure).  The box-side lever (quantile widening with
a coverage guarantee) remains if the retrain washes.  What stands
regardless: the machinery (probe fixed-pointed under its serving fold, the
three-gate scoping, the pins), and the measured map of *why* each scope
gate exists.

### Repairs (step 3 — seven landed this pass)

Per-family disposition, each pinned by a `readings_admit_the_bidder` row and
joining the one batched A/B:

- **Strip node (`(1♠) 1NT - 3♦`) — FIXED, mechanism-level.** The strip re-read
  the stripped auction on a bare keyless `Context::new`; `project_authored`
  needs the trie prefixes, so it silently skipped *every* authored rule at
  stripped nodes and the walk's off-book arm stamped the alerted both-majors
  `3♦` as natural `♦5..`. Re-keyed through the attached partnership —
  `(1♠) 1NT - 3♦` now reads byte-identical to `1NT - 3♦`, and the repair
  covers every authored-alerted call at stripped nodes (runout, Puppet,
  splinters, Texas), not just `3♦`. No bidder bug: the "5-HCP" witnesses are
  8–9 on the PointCount gauge the rule shipped with; a `points(8..)` tighten
  would be convention-tuning on a measured treatment, not a defect.
- **Delayed preferences/raises of a shown 5-6 suit — FIXED.** The blanket
  raise-shows-3-support stamp excluded 81% of the actual forcing-NT
  preference bidders (`♠=2` on every single exclusion; the points band
  excluded nobody). New floor: partner shown **6+** → no length claim at all
  (game raises on a stiff honour are real); a *delayed, non-jump* return to a
  shown **5** → two (zero singletons in ~2500 replayed decisions, so 2 is
  sound and tight); jump returns qualify only when the suit was bid twice —
  the guard that keeps the slam machinery intact (loosening jump returns to a
  once-shown 5-suit let the sampler deal doubleton responders and dragged the
  accountant grand-slam estimate below break-even: the handoff's "a reading knob
  is a bidding knob" made live in a unit test).
- **Rebid inflation through artificial calls — FIXED, two mechanisms.**
  (1) `over_one_notrump` now requires the lane's *first* bid, so post-transfer
  continuations fall under the notrump-structure blanket instead of the
  natural walk — the alerted Jacoby 2♦ no longer counts as a first diamond
  bid (`1NT - 2♦ - 2♥ - 3♦` read ♦6.. against an actual 4; super-accepts and
  post-Stayman 3m had the same exposure). (2) An XYZ-aware five-card floor on
  responder's 2M rebid, both routes — the direct sign-off
  (`1♦ - 1♠ - 1NT - 2♠`, the single largest offender measured: 143/198) and
  the 2♣-relay invite. Gated on `xyz()`; a genuine natural 1♠-then-2♠ still
  reads 6+. Left open: the XYZ 2♦-GF route and NMF (knob-off) still read 6+;
  the underlying `lane_suits` pollution by suppressed calls stands outside
  the 1NT blanket (flagged — the principled `natural_lane_suits` switch
  interacts with cue detection, larger than the measured defects justify).
- **Reader-context projection skew (`support(...)`) — FIXED,
  mechanism-level.** Two agents independently converged on the same root
  cause: `project_authored` projected own-side calls under the *reader's*
  full-auction context, and `Support::project` resolves `partner_last_suit()`
  seat-relatively — so the cue raise's `support(n..)` stamped n+ cards of the
  **cue suit** (`1♣ (1♦) 2♦` 24/24 excluded; the hull stayed near-⊤, which is
  why only the envelope-union probe saw it) and the support double's `support(3..=3)`
  stamped exactly-3 on the **opened minor** (`1♦ - 1♥ (1♠) X` 9/9 — the bidder
  plays a textbook support double, every doubler has exactly 3 hearts; the
  reading was the wrong suit). Fixed by projecting under the bidder's
  **at-the-time context** (auction cut at the call, vul parity-flipped),
  exactly as the table-alert and pass branches already did. For plain raises
  the two contexts coincide, which is how this survived every raise-reader
  sweep. Note the ♥ slot *tightens* 0..=4 → 3..=3 — correct-by-construction
  (the rule gated the bid), but a tightening the batched A/B must carry.
- **Partial-table projection at the transfer choice-of-games 3NT — FIXED,
  and a new mechanism class named.** Knob-on, `1NT - 2♦ - 2♥ - 3NT`
  excluded the 9-count and 4-card-minor game-forcers that bid it. Not a
  wrong rule and not `Fallback::classify`: the 3NT came from the **instinct
  floor** falling through a *deliberately partial* rules table
  (mass-aware `classify_floored`), while `authoring_classifier` resolves
  structurally — so the projection stamped the node rule's `points(10..)` +
  minor-caps box onto a call the floor authored. The projection's premise —
  "some rule sharing this bid produced it" — fails at any partial table over
  a total floor. Minimal repair: both transfer-GF 3NT gates become the
  floor's own seam (`hcp(9..16)`, minor caps dropped) so the book owns every
  3NT the node produces — verified **bidder-inert** over 16,384 honest
  replays (call maps byte-identical; the 9-count force just moves floor →
  book, one provenance pin updated). The *family* closure (project the
  fall-through classifier's same-call union knob-on, or a totality marker on
  `Rules`) is a design decision, filed for the step-4 campaign.
- **Test-hygiene audit (side finding).** The sweep was flaky in-suite: the
  committed `transfer_gf_majors_*` tests hold the GF knobs across assertion
  windows, and a full audit found ~14 tests ending with non-default knob
  state (envelope-union/table-alert readings, leaping-michaels, woolsey points,
  choice-of-games, lebensohl/double styles, negative-double shape, Stayman
  minor slam try) — every one a same-thread landmine for later tests.
  One-line restores applied to all.
- **Preempt raise to game (`3♥ - 4♥` 13/13) — FIXED, two hunks.** The
  responder-raise strength band lacked an `opening_one_suit` gate, so a raise
  of a preempt inherited the constructive band's `1..=11` image and excluded
  every to-make raiser; and the generic raise-shows-3-support stamp is
  unsound once partner has shown 6+ (game raises on a stiff honour). Both
  purely loosening; `1♥ - 2♥` / `1♥ - 3♥` readings byte-identical before and
  after. Triage of lookalikes found **distinct** mechanisms left on the
  worklist: the sixth-card-rebid stamp's five-card excuse is too narrow
  (choice-of-games `1♠ - 3NT - 4♠`), the cue-agreement is invisible to
  `agreed_re_raise` (`(2♦) 2♠ - 3♦ - 3♠`), and doubleton 2/1 power raises hit
  the raise-3 stamp on a known-**5** suit — deliberately not lifted (it would
  loosen raises of every overcall system-wide; own A/B if pursued).

### The cramped doubled answer (2026-07-31, Part A SHIPPED 8f71f0e; Part B DOPI/ROPI/DEPO)

**Forensics first, and they inverted the filed narrative.** Replaying the
−20 board (`1♦ - 1♥ (1♠) - (3♠) 4♦ - 4NT - 5♥ (X)` passed out) through
`probe-classify`: the fit rule did **not** key diamonds. The asker decodes
the trump *after* the answer, and the natural walk reads the artificial 5♥
answer as six real hearts — so `answer_trump`'s shown-5+ rung minted a
**phantom heart trump**, the 1.80 sit rung (`answer_is_five_of`) matched
"5♥ is five of trump", and the ladder *sat* the doubled artificial answer.
`no_room_six` (0.3) was never the decision. The answerer, deriving
pre-answer, keyed diamonds — the two seats disagreed on the trump across
time. Lesson for the ledger: **the answer being decoded must not mint the
trump it is counted against.**

**Part A — pre-answer corroboration + the escape ladder (8f71f0e).**

- `answer_trump` corroborates the answer's own suit against a *pre-answer*
  reading (`Context::new` over `auction[..ask + 2]`, me/partner mapped by
  seat parity): the suit survives only if a real fit or shown 5+ predates
  the answer. Ceiling: a bare prefix context under-reads authored calls
  (no partnership keys) — the transfer/Jacoby lanes recover through the face
  rung, which is why the corroboration filters rungs 1–2 only.
- Over their double of a cramped answer the asker escapes rather than
  sits — *we never play a suit we have no fit in* (jdh8's rule): hand-seen
  six-of-trump @1.73, stopped 5NT @1.72, six of another *seen* fit @1.71
  (gated `!answer_is_five_of(other)` — the phantom suit must not be the
  refuge either), fallback six-of-trump @1.70. All below the 1.80–1.86
  vetted rungs, above every retreat.
- `respect_keycard_signoff` learns the 5NT escape in the doubled window
  only (X at n−3), deriving trump through the shared `answer_trump` — the
  undisturbed book's 5NT king ask is untouched.

Verdict (A/B vs 69864ab, SEED_BASE 1785500157, 204,800 bd/arm/vul):
**win in all four cells — default-on, knobless.**

| vul | plain DD | perfect defense |
| --- | --- | --- |
| none | **+0.0007 [±0.0007]** | **+0.0012 [±0.0009]** |
| both | **+0.0005 [±0.0007]** | **+0.0010 [±0.0009]** |

Fired 0.04 % / 0.03 %, ~+1.8 plain (+3.2 to +3.7 PD) IMPs per fired
board.

**Part B — DOPI/ROPI below five-of-trump, DEPO at/above (classic
D0P1/R0P1, user-confirmed).** Their *bid* over our 4NT stood the whole
window machinery down (`opponents_quiet_since` tolerates only Pass/X)
while the card declared DOPI/ROPI with no implementation — a machinery
hole and a false disclosure at once. Authored on the floor: answerer
ROPI over their X (XX=0, P=1, 5♣ up = 2, 3), DOPI over their bid below
five-of-trump (X=0, P=1, cheapest bid=2, next=3), DEPO at/above (X=even,
P=odd, optimistic decode); asker's `keycard_answered` decodes all three
windows with 1430-style wraparound arithmetic and feeds the existing
placement rungs including Part A's escapes; `keycard_conversation_now`
rail gains the interfered shapes (one enemy bid, directly over the ask);
card DEPO=1, goldens re-blessed. Filed, not pre-engineered: the third
round after a non-bid DOPI answer is left to judgment, and the asker's
1.82 signoff pulls what might be a fine penalty double after DOPI X=0.

Verdict (A/B vs Part A 8f71f0e, SEED_BASE 1785500660, 204,800
bd/arm/vul): **NULL in all four cells — ships default-on per the
pre-registration** (a wash also repairs the false disclosure).

| vul | plain DD | perfect defense |
| --- | --- | --- |
| none | +0.0000 [±0.0003] | +0.0001 [±0.0003] |
| both | −0.0002 [±0.0003] | +0.0000 [±0.0004] |

Fired 16 / 13 boards *total* per vul (0.01 % — BBA rarely bids over
4NT, as expected). Worst-board trace of the both-plain −34 raw IMPs
found no decode defect — three judgment mechanisms: the filed
signoff-pulls-a-fine-penalty-X gap (−12), doubling channels on the same
final contract (−11), and one new filed follow-up — **the own-shown-five
5-2 trump**: `answer_trump`'s rung 2 takes `shown = max(partner, me)`,
so the asker's *own* shown 5-card heart suit qualified hearts as trump
with no fit evidence, the pre-answer corroboration accepted it (the
shown-five criterion is satisfied by the asker's own hand), and the sit
rung passed a DOPI 5♥ step answer out in a 5-2 (−13; the off arm's free
judgment bid 6♠ making). Old rung-2 behavior newly exposed by the live
window — repairing it (fit evidence, not one-hand length, as the
shown-five bar) moves every keycard auction's trump derivation and
needs its own A/B (below).

### The own-shown-five 5-2 trump repair (2026-07-31, no fit without proof)

Design settled by grilling. Doctrine: **one hand's shown five never
synthesizes a fit.** Trump comes from a provable eight, a
self-sufficient own seven, or the auction's face by fiat — the ask's
*placement* carries the asker's intent (`2♥ (X) 4♥ (4♠) 4NT` assumes
spades), so a suit the face can see is agreed by the ask itself, and a
shown five the face cannot see was never agreed.

- `answer_trump` rung 2: the either-seat `shown ≥ 5` filter becomes
  `hand + partner_floor ≥ 8 || hand ≥ 7` (actual holding, majors-only
  scope kept, both seats). Surviving population vs rung 1: the 6-2/7-1
  fits its three-card bar refuses, and unshown 7-baggers.
- `corroborated` mirrors the same bar (`seen ≥ 8 || hand ≥ 7`, the old
  `≥ 5` disjunct deleted). This half is **load-bearing**: fixing rung 2
  alone leaves the pollution path open — the 5♥ answer read naturally
  bumps partner's floor to 5, *rung 1* fires the phantom, and only the
  corroboration stands between it and the sit rung. An answerer's
  five-level bid in its own shown five-carder now reads as natural
  flight, harmonizing with Part A's escape ladder.
- Untouched by decision: `face_trump` (fiat, uncorroborated stands);
  `keycard_trump` and both direct call sites — the ask still never
  *initiates* on a 6-2, so the −14/bd 6NT-reroute lesson stays sealed.
- Filed: the ask-gate's decodability proxy (instinct.rs:4023) still
  models the old rung 2 — if worst boards show asks counted against the
  wrong suit (answerer showed five early, last pre-ask bid a second
  suit), recalibrate the gate in its own A/B.

Verdict (A/B vs 8ba8844, SEED_BASE 1785504001, 204,800 bd/arm/vul):
**positive in all four cells — ships default-on, knobless** (the
pre-registered bar was a mere non-loss).

| vul | plain DD | perfect defense |
| --- | --- | --- |
| none | +0.0005 [±0.0008] | +0.0003 [±0.0009] |
| both | +0.0002 [±0.0008] | +0.0003 [±0.0010] |

Divergent 93 / 85 boards per vul (0.04–0.05 %), +0.38 to +1.15 IMPs
per divergent board. Worst-board mechanisms, neither a decode defect:
4NTs the floor never initiated as RKCB now read quantitative and get
passed (the old arm answered against an unprovable suit and sometimes
landed on its feet), and the face yields None when our side's last bid
was a cue of their suit (`1♥ (3♦) 4♦ - 4NT`: the cue blocks partner's
solo-bid hearts — a step-back-past-cues face refinement is a filed
candidate). The −13 DOPI board verified flipped by `probe-classify`
replay: the answerer now passes over their 5♦ instead of minting the
5♥ step, so the phantom-heart sit cannot arise (the face's most-recent
agreement there is the *real* 4-6 club fit, 2♣ opposite 4♣).

### Experiment: the ask-gate recalibration (launched 2026-07-31, verdict pending)

The filed follow-up, taken up by grilling (decisions jdh8's): the ask
gate's decodability proxy (instinct.rs, the 4NT ask rule) still modelled
the **old** rung 2 — either seat's reading showing 5+ of the trump.
Stale both ways: it blocked face-decodable raised 4-4 fits (the raiser
proves the eight in hand while the table shows 4+3; the old floor then
blast-bid the milestone slam *blind* — the fixture's 66.3% DD includes
every off-two-keycards board), and it passed shown-5 asks the ba07b26
answerer no longer corroborates (five shown early, a second suit last —
the answerer's face keys the wrong suit).

The swap, mirroring the answerer's doctrine from the asker's seat (the
answerer's hand is unknowable, so "share the function" becomes "share
the guarantee"): the trump must be **provable on the table** (my shown
floor + partner's shown floor ≥ 8 — partner's hand is at least partner's
floor, so any seat proves it) **or keyed by the face** (`face_trump` at
the ask's own index, hand-independent, computed on the identical prefix
by every seat).  One addition forced by the pin sweep: the ask reuses
`known_eight_card_fit`'s measured flat-4333 carve
(`bare_four_four_own_flat`, now a shared helper) — a bare 4-4 opposite
our own flat hand is not a playing fit, so it is no RKCB trump either.
Untouched by decision: `undisturbed()` on the ask, majors-only
initiation and the 3-card bar in `keycard_trump`, every answer rung.

Doctrine pinned along the way (jdh8): directly over the Stayman answer
or the transfer completion, 4NT is QUANT — the only call exploring the
uncertain major fit and the misfit 6NT at once — and slam interest cues
the other major.  Both lanes are book territory (the 3OM slam try, the
quantitative 4NT), so the gate change is inert there; pinned by
`one_notrump_lanes_stay_book_quant` on provenance, not just calls.
Veto symmetry verified in code: the gate's `partner_last_call != NT` is
the answerer's `auction[n − 4]`-was-NT quant veto seen from the other
seat.  Pins: `face_agreed_four_four_fit_asks` (the 1♦ - 1♥ - 2♥ - 3♥ raiser
asks; the answerer decodes the same trump), `unprovable_fit_never_asks`
(opener's shown five over our bare three: face keys the second suit —
the wrong-suit clash lane now stays judgment), and the two blast-board
pins adjudicated to the vetted route (4NT → 5♦ → 6♠: the same slam,
now entered through 1430).

A/B round 1 in `ab-results/keycard-ask-gate/`: base **ba07b26** vs the
swap, two-binary protocol (knobless), SEED_BASE **1785507043**, 32×6400
bd/arm/vul.  Widening the gate is a bidding change → the plain decision
table applies (the face-trump precedent).

**Round-1 verdict: LOSS in all four cells** (plain −0.0014/−0.0023, PD
−0.0016/−0.0025, fired 0.16/0.18 %, −0.9 to −1.4 IMPs/fired).  Worst-board
trace, three families: (A, dominant) newly-enabled asks over *settled*
auctions (`1m - 1♠ - 3♠ - 4♠ - 4NT`) decoding the ambiguous 1430 step
(`5♦` = {0,3}, `5♣` = {1,4}) on the high branch and driving six off
two-plus keycards; (B, mirror) the same ambiguity decoded low, the asker
signs off and the answerer's value-gated correction never fires, missing
slams the base arm blast-bid; (C, accepted) the narrowing dropped a few
old wrong-suit asks that landed on their feet.  Common thread: every
worst board was a **~26–28-combined** ask over a limited raise.

**Round 2 — jdh8's doctrine collapses the machinery.**  An intermediate
design (pessimistic quiet-ladder decode + the answerer's arithmetic
high-count correction of the relay signoff, strength-projection
disambiguation) was built and then **reverted**: *a partnership that
cannot assume three combined keycards should not be seeking slam at all,
and inside combined 3..=5 every 1430 step is unambiguous* (the two
readings differ by exactly three, so at most one fits the window — the
existing optimistic decode is exact under the assumption).  The repair
is therefore one constraint on the **ask**, not a decode protocol:
`combined_points(29)` joins the gate — the strength floor that buys the
three-keycard assumption (at 29+ the opponents hold ≤ 11 HCP, and three
keycards need 11 packed exactly — jdh8's "almost impossible").  The
accountant entry prices tricks; this floor prices the *conversation*.  Every
round-1 worst family dies at the floor (all were sub-29).  Discovered en
route, filed: the invite re-raise (`1♦ - 1♥ - 2♥ - 3♥`) stamps **no
strength** — an uncontested vacuous-reading instance that keeps the
whole lane below any conversation floor.

Round 2 A/B in `ab-results/keycard-ask-gate-2/`: same base, fix = gate
swap + flat-4333 carve + `combined_points(29)`, SEED_BASE **1785508970**,
same protocol.

**Round-2 verdict: WIN in all four cells — SHIPPED default-on, knobless.**

| vul | plain DD | perfect defense |
| --- | --- | --- |
| none | **+0.0022 [±0.0022]** | **+0.0028 [±0.0022]** |
| both | **+0.0032 [±0.0027]** | **+0.0040 [±0.0027]** |

Fired 0.30 / 0.32 %, **+0.74 to +1.24 IMPs per fired board** — the
table's best case (plain win + PD win, both vuls), and PD > plain in
every cell: the conversation's edge grows when the defense punishes
every overreach.  The residual worst boards are the accepted mirror
(the base arm's old shown-5 asks and sub-floor blasts sometimes landed
on their feet) and ordinary slam-judgment margins; **no decode defect
in the tail** — no passed-out 4NT, no wrong-suit count, no redoubled
answer left in.

Filed as follow-ups, in order: cue-blocked face (step-back-past-cues,
next separate A/B), minors initiation (new: `keycard_trump` gains ♣/♦
under the provable-8 bar, watching the 3NT/quant collision), DOPI
residue, contested free-bid stamps, the strength-silent invite
re-raise.

### Experiment: cue-blocked face + the NT dichotomy (2026-07-31, positive all four cells — SHIPPED)

Taken up by grilling (decisions jdh8's).  The filed defect: `face_trump`
set `last` *before* the cue check, so `1♥ (3♦) 4♦ - 4NT` overwrote
partner's solo-bid hearts with the cue and rung 2 died.  While settling
the step-back semantics jdh8 pinned the general doctrine — **when 4NT is
ambiguous, it is RKCB if the side's last non-cue bid below the ask is a
suit, quantitative if notrump** — with one carve descending from the
3NT ruling: over an agreed **major**, 3NT is non-serious (minimum game
force beside control bids), the fit survives an NT last bid; over an
agreed **minor**, 3NT is *sign-off* — the NT last bid re-opens the
strain and the subsequent 4NT is quantitative.

**BBA probed live** (ctypes against the vendored libEPBot, dealer
canonicalized, methodology validated on `1♠ - 3♠ - 4NT` where answers
are genuine keycard steps tracking the hand): after `1♦ - 3♦ - 3NT`,
BBA's own slam move is **4♣ = Gerber** — steps count *aces*, proven by
the discriminator hand (one ace, no trump K) answering 4♥, the second
step, not the first — and a *forced* 4NT there draws an **unconditional
6♦**, from 19 HCP with three keycards down to 12 HCP with zero.  BBA
never plays that 4NT as RKCB; its resolution of the minor-3NT cell is a
cheaper ask vehicle, not a quant reading.  **Gerber rejected for pons**
(jdh8): with clubs the agreed strain, a 4♣ ask is ambiguous against the
4♣ sign-off/pull — the phantom-lane class this campaign kills.  The
dichotomy already gives minors their RKCB route: keep the side's last
bid a suit and 4NT asks; only the 3NT-sign-off lane is quant.  Carried
into the minors-initiation filing as its reference design constraint.

The change, two edits in one function (`face_trump`), bundled per the
seven-repair precedent: (1) **cue-skip** — a suit bid already named by
the opponents *at the time it was made* (`theirs` is built
incrementally, the correct was-it-a-cue semantics) never becomes
`last`, so the walk steps back past cues and stops at a real suit
(face) or notrump (veto stands); (2) **the minor carve** — `agreed`
yields when the last non-cue bid is notrump and the agreed suit is a
minor, falling to rung 2 whose NT veto answers `None`.  Both propagate
free to all three consumers (answerer ladder, `recognizable`/rail, the
ask gate — 49f4837 routed the gate through `face_trump`): the gate
stops minting 4NT asks after a minor-fit 3NT sign-off, and the rail
reads the cue-blocked lanes.  Five-case unit test
(`face_trump_steps_past_cues_and_reads_the_nt_dichotomy`) pins the
doctrine table; `readings_admit_the_bidder` stays green.

A/B in `ab-results/reading-drift-cue-face/`: two-binary
`ab-reading-drift.sh` protocol, base **49f4837**, SEED_BASE
**1785512396**, 32×6400 bd/arm/vul, arms sequential under
`idle-run.sh`, plain + PD.  **Pre-registered bar: non-loss ships**
(doctrine-pinning reading repair, the seven-repair-batch/DOPI class); a
loss traces its worst divergent boards first.

**Verdict (read 2026-08-01, 204,800 bd/arm/vul): positive in all four
cells — SHIPPED default-on, knobless.**

| vul | plain DD | perfect defense |
| --- | --- | --- |
| none | +0.0002 [±0.0005] | +0.0005 [±0.0006] |
| both | +0.0003 [±0.0006] | +0.0006 [±0.0007] |

Fired 39/35 boards per vul (0.02% — the narrowest trigger of the
campaign), **+0.85 to +3.40 IMPs per fired board**, and PD > plain in
every cell — the dichotomy's edge grows when the defense punishes the
base arm's phantom asks.  The bar was a mere non-loss; the table clears
it in every cell.
