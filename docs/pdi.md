# Pass/double inversion (PDI)

> **Status (2026-08-26): the action gates lost and were deleted; the reading-only
> replacement is authored, and a bid-only pre-count measured it INERT.**
> `pdi_latch` is a pure reading knob, default off: our post-trigger pass over
> RHO's live suit bid denies the trap, as a two-term `envelope_union` whose
> thresholds came from `examples/probe-pdi-population`. It changes the bidder's
> call on **10 boards in 409,600**, so no DD time was spent. Read
> [Why the reading is inert](#why-the-reading-is-inert--and-what-that-says-about-union-readings)
> before authoring any other post-walk union — the funnel there is not specific
> to PDI. [Verdicts](#verdicts) has the numbers.

Campaign doc for the pass/double-inversion mechanism: what a *trigger* is, how
the reading walk finds one, what the inverted half means, and what is still
owed.

Classic PDI (Rodwell; Bridge Winners) inverts pass and double in **forcing-pass**
auctions. This project inverts them in **penalty processes** instead, and the
mechanism is deliberately built so a forcing-pass trigger can be added later
without re-plumbing anything.

## The idea

Once our side **pulls a trigger** — makes a penalty-oriented double, or makes a
pass that converts partner's double to penalty — the meanings of our later `X`
and our later `P` swap roles:

- our later **`X` suggests penalty** and is *more vague* (a stack, or just
  "I am willing to defend"), rather than takeout on shortness;
- our later **`P` over RHO's bid becomes (possibly non-forcing) takeout** — "I
  have nothing more to say about defending; you decide".

The **X** half was arm 1 as an *action* gate, and it lost — twice, on both the
deterministic ladder and the served logits. The **P** half is arm 1 as a
*reading*, and it is what ships behind the knob today. The X half survives only
as a reading, deferred to arm 2 (follow-on 3).

## The trigger set

A trigger is a call by **our side**, at index `i`, of one of two kinds.

### 1. Rule-tagged penalty-oriented doubles

An authoring rule carries `.penalty()` (`Rules::penalty` /
`Rules::penalty_if`, mirroring `.alert(...)` / `.alert_if(...)`), which sets
`Rule::penalty_oriented()`. The inventory is **the tags themselves**:

```
grep -rn '\.penalty()\|\.penalty_if(' src/bidding
```

The census that seeded them (16 shipped/BBA-active penalty-oriented doubles,
4 conversion sites) was scratch work kept outside the repository; the grep is
the live list. What belongs in it: a double that starts or continues a penalty
process — a natural penalty double, a cards/values double whose point is to
punish their runout, a trump-length double of a runout. What does **not**:
negative, support and responsive doubles; ordinary and strong takeout doubles;
lead-directing doubles; DOPI/DEPO answers; stolen Stayman; convention-showing
doubles (DONT, Meckwell, Woolsey, Direct Landy); business and SOS redoubles.

**Why a tag, not the alert.** Alert is *disclosure*, not the reading switch
(docs/authored-reading-handoff.md): artificiality is bid-only, the shipped
natural `(1NT) X` is a penalty trigger and is **unalerted**, and conversion
passes are unalerted by construction — so alerts cannot even represent half the
trigger set. The tag is a private `bool` on `Rule`; forcing-pass PDI marks
*auction states*, not rules, so widening it to an enum later buys nothing.

An inert tag is harmless — unlike `alert_if`, whose tag must be *absent* rather
than merely inert (the kickback §7.4 trap), `.penalty_if(false)` is safe: the
field gates nothing, weighs nothing, and never reaches describe or the
projection fold.

### 2. Structural conversion passes

`auction[i] == Pass && auction[i - 1] == Pass && auction[i - 2] == Double`,
where `i - 2` is our partner and `i - 1` is RHO — **a pass that leaves partner's
double in**. That is an election to defend however the double started life
(takeout, negative, responsive), so it needs no tag at all, and one rule covers
every conversion site including the floor-made passes that have no rule to tag.

Two things this deliberately does not do:

- **No DOPI/ROPI false fire.** A keycard answer's pass sits over *their* double
  or bid at `i - 1`, so the pattern cannot match (`keycard_pass_answers_do_not_convert`).
- **A pass of partner's lead-directing double of an artificial bid does fire.**
  Documented, not exempted: sitting for that double genuinely elects to defend.

**Redouble conversions** (a pass of partner's business `XX`) are out of v1.

## Mechanism

```
Rule.penalty  ──►  CallMasks.penalty_trigger  ──┐
   (tag)          (projection.rs, both drivers) ├──►  Inferences.pdi_latched
                                                │      (read.rs, side-scoped)
auction[i-2..=i] == X P P ──► conversion_passes ┘
```

- **`CallMasks::penalty_trigger`** (src/bidding/inference/projection.rs) is
  recorded by `penalty_trigger_live`, an ANY over the live rules for the call
  made. It sits **outside `authored_effect`** on purpose: that function's
  compiled skip fast-paths and its decode gate both early-return before
  recording anything, and under the shipped `Alerted` reading scope an unalerted
  call never gets that far — an inside implementation would silently drop the
  natural `(1NT) X`. Recorded identically by the one-shot driver (inside
  `project_call`, so it covers the own loop, the table-alert loop and the pass
  walk) and by `AuthoringStepCache::prepare`'s commit loop; the `masks`
  `assert_eq` in `assert_step_cache_projection_parity` is the trip wire.
  Positions ≥ 64 carry no bit — the shared `CallMasks` limitation.
- **`conversion_passes`** (src/bidding/inference/read.rs) is rules-free and
  depends only on `auction[i - 2 ..= i]`, so an incrementally grown auction
  rescans it by construction.
- **`Inferences.pdi_latched`** collapses the two into one side-scoped `bool`.
  It is a `bool`, not the mask, because the systems-on overcall strip re-reads a
  *shortened* auction: a mask handed out would be indexed against the stripped
  auction while the caller holds the unstripped length, and the parity test
  would silently invert. Collapsing it inside `Inferences::read`, where the
  matching length is in scope, makes that unrepresentable.

  **Nothing in `src/` reads it today.** The action gates that used to are gone,
  and the shipped reading works off `pdi_triggers` directly — the mask is in
  scope where the reading runs, and `pdi_latched` is only stamped afterwards.
  It survives as the public, index-free statement of the fact, which is what
  `probe-pdi-population` and the mechanism tests consume.

Their side's triggers are recorded too (the mask is table-wide under
`table_alerts`), but `pdi_latched` is scoped to the side to act, so a trigger of
theirs never latches us.

## What is built

Knob `decision.reading.pdi_latch`, **default off**. It is a *reading* knob: the
trigger mechanism above feeds one post-walk claim about our post-trigger
**pass**, and nothing else. The two action gates that once hung off it were
measured and deleted.

- **The deterministic ladder is not widened.** `instinct::penalty_latched` still
  means the legacy `(1NT) X` lane alone, and the three wrappers
  (`penalty_latched_c`, `may_pull_penalty`, `not_penalty_latched`) still key off
  it. Widening them to the whole trigger set was **Mode A** of the P2 loss.
- **The configured neural floor has no PDI shell.** The v6 floor distils BBA
  (`teacher: bba`, `dd_weight: 0.0`), and BBA already plays expert post-trigger
  methods — its post-trigger doubles are penalty-suggestive, its passes "nothing
  more to say". A shell that re-inverts the served logits is a *second*
  inversion on an already-inverted policy. That was **Mode B**.
- **The X half is unread.** The generalized double adds no points or
  suit-length claim, and the legacy `(1NT) X` stack reader is left exactly as
  shipped where the two lanes overlap. Arm 2 owns it (follow-on 3).

### Why a distillation retrain cannot fix the action side

A retrain on BBA labels would *anti-teach* inversion: BBA labels latched
contexts with meanings that are already post-trigger-correct, so the net would
learn to undo whatever gate sits above it. That branch is **closed**. The only
instrument that could teach the post-trigger sit/pull/double decision is an
**oracle teacher** with DD/par labels on that decision — the competitive
accountant ([ai-bidder/competitive-accountant.md](ai-bidder/competitive-accountant.md)),
not a distillation retrain and not a latch input bit (a feature-version-7
programme with nothing for a BBA teacher to supply).

## The pass half (arm 1) — the shipped reading

Our pass **over RHO's live suit bid**, after our side pulled a trigger, denies
the **trap**: the hand that is long in their suit *and* strong enough to punish
it, because that hand now doubles. That is the negation of a conjunction, so it
is a **two-term union**, not an envelope:

```
[their-suit ≤ 4]  ∪  [points ≤ 11]
```

`envelope_union` ships default-on, so the pre-union-era "not expressible in the
interval envelope" is no longer the wall. Thresholds are the probe's, not a
priori (§ Task 3).

**The conversion pass is exempt structurally, not by a skip list** — it sits over
RHO's *pass*, so their bid never precedes it. Authored passes need no exemption
either: a pass names no suit, so no rule's own reading can contradict this one.

### Where it actually shows through — the load-bearing mechanism

A union of "short" with "weak" spans both axes, so **its hull is vacuous**. Every
consumer on the shipped bidding path reads a hull:
`Inferences::assemble` sets `announced_players[i] = announced_unions[i].hull()`,
`features_v6` and `features_eval_v5` push `announced(who)` into the nets, and
every book/instinct gate goes through `players[]`. Only `Inferences::admits`
sees the boxes, and its sole non-test callers are in `sampler.rs`, reached from
`ev_all` and the sd-lead harness — neither on the default bidding path.

So the union bites **exactly where the rest of the walk already contradicts one
term**, collapsing it to a single box that does narrow the hull:

| the walk already shows | the union collapses to | share of latched passes |
| --- | --- | --- |
| points ≥ 12 (e.g. the passer's own takeout double) | `their-suit ≤ 4` | **26.5%** |
| their-suit ≥ 5 | `points ≤ 11` | 0.4% |
| neither | nothing visible (only `admits` sees it) | 73.1% |

That is not a defect to design around, it is the claim behaving correctly: "with
values, if I had their suit I would have doubled" is precisely a conditional. It
is recorded here because a reader who assumes a union narrows the hull will
mis-predict the A/B, and because it sets the measurable surface — 621 of 409,600
boards at table A (0.152%), roughly double counting table B.

### Why the reading is inert — and what that says about union readings

The pre-count settles arm 1 empirically, and its explanation generalises well
beyond PDI, so it is recorded here rather than in a verdict cell.

`Inferences::assemble` recomputes **`announced_players[i]` from
`announced_unions[i].hull()`** — but leaves **`players[i]` alone**. So a
post-walk union reaches exactly one consumer on the shipped bidding path: the
per-seat inference block `features_v6` and `features_eval_v5` push into the nets
— `LEN_INFERENCE_V6 = 18` floats a seat (8 length endpoints, 2 points, 8
support-point endpoints), so 72 across the four seats for `features_v6` and 54
across the three hidden seats for `features_eval_v5`. Every deterministic book
and instinct gate reads `players` and never sees it; `Inferences::admits`, the
only consumer of the box *structure*, is called only from `sampler.rs`, which
the default bidder never reaches.

The funnel is brutal. Rows 1–3 are **our-side decisions at table A** of the P2
baseline arms; row 4 is **boards whose final contract moved** in the pre-count.
Different runs, but the same 409,600 boards and the same table, so the two are
comparable end to end — the last column is what each stage costs in boards:

| stage | table-A count | of 409,600 boards |
| --- | --- | --- |
| our side latched, passing over RHO's live suit bid | 2,343 decisions | 0.57% |
| …of those, the union collapses (walk floors points > 11) | 621 decisions | 0.152% |
| …of those, the length ceiling actually **moves the hull** | 462 decisions | 0.113% |
| …and the bidder's **call changes** | **2 boards** | **0.0005%** |

Counting table B as well — `bba-gen` seats our pair at both — the last row is 10
boards, 0.0024%. Note the units: rows 1–3 are decisions, and a board can carry
more than one (the 2,911 decisions of § Task 3 fall on 2,144 distinct boards).

The last step is the one nobody predicts: moving one of the eighteen floats that
describe a seat almost never flips an argmax. And the reach-maximal control —
`[their-suit ≤ 2] ∪ [points ≤ 5]`, deliberately far too strong to be true —
still only reaches 132 boards (0.032%), about 66 per vulnerability. That is the
**ceiling on the whole mechanism**, not on the thresholds.

Two traps for the next reader:

- **Lowering the point cap *increases* reach.** The union collapses when the
  walk already floors the seat *above* the cap, so a high cap (`pts ≤ 19`)
  almost never collapses and reaches **zero** boards — the first negative
  control run here was maximal in claim strength and minimal in reach.
- **A union's hull is its span.** `[short] ∪ [weak]` is vacuous on both axes by
  construction. Authoring one and expecting the floor to see it is the mistake;
  it is seen only where the rest of the walk kills a term.

If `players` is ever recomputed from the unions, this reading goes live for free
and the pre-count should be re-run. Until then the knob stays default off.

### Testbeds

| lane | role |
| --- | --- |
| UvU `1NT (2NT) X` | the tag-path testbed — book-authored trigger, default-armed, zero authoring cost |
| Kokish–Kraft | negative control: its double split is trie geometry, so the delta must be zero (`kokish_kraft_unchanged_under_pdi`) |
| takeout-X conversion (`(1♥) X - -`) | the dominant firing lane — 1894 of 2911 latched decisions come off a structural conversion pass |
| `(1NT) X (2♦) -` | the *uncollapsed* control: the passer has said nothing, so the hull must not move (`post_trigger_pass_narrows_the_hull_only_on_collapse`) |

### Legacy latch, left beside

The one-lane prototype — knob `ReadingProfile::penalty_latch`, detector
`penalty_x_reading_with_profile` keyed to "(1NT) X our first action", floor gate
`penalty_latched`, reader twin `penalty_latch_double_reading` — is untouched.
Re-keying it through the tag is follow-on (2) below.

## Verdicts

| date | change | arms | verdict |
| --- | --- | --- | --- |
| 2026-08-26 | Step 0: `penalty_latched` reads the *pinned* notrump-defense profile | — | shipped; `smoke-default --count 20000 --seed 1` byte-identical (the default defense is Natural) |
| 2026-08-26 | P0: tag + conversion detection, no consumer | — | shipped; byte-identical |
| 2026-08-26 | P1: `pdi_latch` X-half action gate, default off | — | shipped opt-in, then **deleted** — see P2 |
| 2026-08-26 | **P2: the full P/X logit swap** — the deterministic wrappers widened to the whole trigger set *and* a configured-floor shell giving Double the Pass logit | vs BBA, 204,800 bd/arm/vul, both vuls, `SEED_BASE=1787695294`, build `c1b3a846` + `source.patch` | **LOSS on all four cells.** Plain DD **−0.0045 ±0.0015** (none) / **−0.0055 ±0.0019** (both); PD **−0.0054 ±0.0015** / **−0.0066 ±0.0018**. 898/409,600 divergent (0.22%), ≈ −2.0 to −3.2 IMPs/fired. Artifacts `pdi-latch-p2-swap-20260826-c1b3a846/` |
| 2026-08-26 | ULP variant ("x-only": `logits[Double] = logits[Pass].next_up()`, Pass untouched) under the new `--declare-books-mutually` self-play | treated-us vs untreated-`american`, honest mutual books, 204,800 bd/arm/vul | **REJECT.** 10k pilot read +0.013/+0.016 plain; at full size plain washed (−0.0024 / +0.0003) and **PD lost (−0.0111 / −0.0123)**. *Non-standard evidence*: two design changes at once (mechanism **and** opponents), so it is not a clean read on either. Artifacts `pdi-latch-mutual-xonly{,-full}-20260826/` |
| 2026-08-26 | **Task 3 probe: the post-trigger passer and doubler populations**, `probe-pdi-population` over the P2 baseline arms (409,600 boards, both vuls) | — | thresholds set at `[their-suit ≤ 4] ∪ [points ≤ 11]`; every tag cleared; a level bound, a freshness gate and a `suit_hcp` axis all ruled out — see below |
| 2026-08-26 | **Arm 1 authored** — the pass-side union, knob-gated, `smoke-default --count 20000 --seed 1` byte-identical off | — | shipped opt-in |
| 2026-08-26 | **Arm 1 bid-only pre-count** — both arms bid at 204,800 bd/vul, `SEED_BASE=1787700673`, auctions diffed with **no solver** | on vs off, both vuls, both tables | **INERT: 10 divergent boards in 409,600 (0.0024%).** No DD time spent. A reach-maximal negative control (`[len ≤ 2] ∪ [pts ≤ 5]`, a knowingly false claim) reaches only 132 boards (0.032%) — see "Why the reading is inert" |

### The P2 forensic, and the premise it produced

The 40 worst boards split into two modes with different culprits:

- **Mode A — dead games.** `- - 2♣ 3♥ X - - -` sitting with `AK9874.A.K2.AKT4`
  while the off arm bids `4♠`; `1♠ 1NT 2♠ X - - -` against off's `4♥`; over and
  over. Both arms hold identical hands, prefix and net, and the off arm's game
  bid beat *both* the old Pass and the old Double logit — so a pure P↔X
  permutation cannot lift either above it. These sits are not the net and not
  the swap: they are the **deterministic forced-advance sit-latch**, which
  knob-on widened from its tuned home (defending their doubled 1NT) to every
  tagged trigger.
- **Mode B — doubling cascades.** `1NT X 2♣ X 2♥ 3♥ X - - -` against off's
  `3NT`; multi-`X` festivals ending in defended part-scores. That is the swap
  turning the teacher's quiet passes into vague doubles.

The premise that explains both: **the net inherits PDI from its teacher.** Both
action-side interventions were second inversions on an already-inverted policy.

### Task 3 — the probe that set the thresholds

`cargo run --release --features serde --example probe-pdi-population -- <base-none> <base-both>`
replays the baseline arms' own auctions and records, at every point where our
side is latched and RHO's live **suit** bid is the call to act over, what we did,
what we held, and what the walk had already shown us. It reads the dump under
`vs_bba_agreements`, the agreements `bba-gen` actually bid it with — under bare
`Agreements::default()` it drops 3.7% of the population.

**Coverage.** 2,911 such decisions in 409,600 boards, on 2,144 distinct boards
(**0.52%**); 2,343 are `Pass`, 231 `Double`, 337 a bid. 1,894 come off a
structural conversion pass, 1,017 off a tagged double. Note the units: decisions
are not boards, and `bba-gen` seats our pair at **both** tables, so the surface
our pair faces is about twice this. Restricting to their three level or below
keeps 1,309 and does *not* sharpen the population (the passers' mean their-suit
length is **higher** at low levels, 2.81 vs 2.39), so a level bound buys nothing
and none is authored.

**Choosing the cut.** Content is the prior mass the claim removes, measured
against the 687,747 structurally identical **unlatched** decisions the probe
collects as a control — never the count of real passers inside the zone, which
is small precisely *because the claim is true*. Cost is the share of real
passers the claim contradicts, judged against the shipped ambient
partner-exclusion rate of **0.974%**
([reading-drift-handoff.md](reading-drift-handoff.md)), not against zero.

| `[len ≤ c] ∪ [pts ≤ cap]` | passers contradicted | collapse rate | prior mass removed, on collapse | doublers separated |
| --- | --- | --- | --- | --- |
| c=5, cap=7 | 0 / 2343 (0.00%) | 63.0% | 1.58% | 3.5% |
| c=4, cap=7 | 34 (1.45%) | 63.0% | 6.43% | 18.2% |
| **c=4, cap=11** | **20 (0.85%)** | **26.5%** | **5.54%** | **8.7%** |
| c=4, cap=13 | 12 (0.51%) | 17.1% | 5.09% | 3.5% |
| c=3, cap=11 | 138 (5.89%) | 26.5% | 18.35% | 30.7% |

**4/11 ships.** It is the bridge-honest agreement — "post-trigger, with 12+
points, my pass over their suit bid denies five of it" — it contradicts fewer
real passers than the system's own ambient rate, and its collapse subset (0.152%
of boards at table A) is the same order as the 0.21–0.23% surface on which the
P2 swap returned a 3–4σ verdict. Loosening the cap to 7 doubles the reach but
buys it with hands that are 8–11 points and five trumps, which genuinely cannot
punish — the claim would be false, not merely tight.

**What the probe ruled out.**

- **A level bound** — see above; and the P2 forensic independently kills it
  (every losing converted contract was already ≤3).
- **A freshness gate.** Restricting to our side's very next turn after the
  trigger (measured exactly: the latch was off two calls earlier) leaves 1,562
  of 2,343 passes and, at 4/11, *raises* the contradiction rate to 0.96% while
  cutting the collapse subset to 0.090% of boards. It buys nothing at an honest
  threshold; it only helps at cuts too aggressive to author.
- **`suit_hcp` as the strength axis.** It separates the raw populations better
  (0.98% contradicted at `len ≥ 5 ∧ suit-HCP ≥ 4` for 3.03% of prior mass), but
  the walk almost never floors a seat's `suit_hcp`, so that branch never
  collapses and the claim would never reach the hull. The collapse test has to
  compare like with like: author the cap on the same `points` axis the walk
  floors.
- **Tag hygiene.** The forensic's two suspects — the strong-2♣ preempt double
  and the 1NT-overcall advancer double — are not leak sources. The contradicted
  passers are diffuse, one to three apiece across more than twenty of the 650
  lanes, so no untag set cleans anything up and every tag stays.

**Candidates left on the table**, both sound on this sample and both deferred to
keep arm 1 a single mechanism:

- a bare ceiling, post-trigger `P` shows `points ≤ 19` — 0 / 2343 contradicted,
  0.76% of the population, and it narrows the hull *unconditionally*. It is a
  support-edge cut (the observed maximum, from 2,343 samples), which is exactly
  what [ai-bidder/sampled-projection.md](ai-bidder/sampled-projection.md) says to
  distrust, so it wants its own arm.
- the double side, post-trigger `X` shows `points ≥ 4` — 0 / 231 contradicted,
  5.5% of the population removed. That is arm 2's floor (follow-on 3), and it is
  a far better claim than the legacy 4+ stack.

**Caveats on the numbers.** The zero-contradiction corners were selected from
roughly five hundred swept zones on the same data that justifies them, and a
zero in 2,343 samples bounds the true rate only at 0.13% (rule of three). The
4/11 cut is not one of them — it is chosen for the bridge, and its 0.85% is a
measured rate, not a selected zero. It also **holds out of sample**: split by
vulnerability arm it reads 11/1215 = 0.91% (none) and 9/1128 = 0.80% (both),
with collapse rates 26.9% and 26.1% and prior mass 1.48% and 1.40%.

## Follow-on queue

1. **Make a reading visible to the deterministic path**, or stop authoring
   post-walk unions for the floor. Arm 1 is correct and inert because
   `Inferences::assemble` never recomputes `players` from the unions. Deciding
   whether it should — and what that costs the shipped readings' byte-identity —
   is the prerequisite for arm 1, arm 2 and every other union-shaped reading.
   If it changes, re-run the pre-count
   (`/mnt/hdd-data/jdh8/pons-ab-results/pdi-pass-union-precount`, `SEED_BASE=1787700673`)
   before spending any DD time.
2. **Re-key the legacy 1NT latch through the tag** (user-mandated). Independent
   of everything else; needs its own `smoke-default` byte-identity proof.
   Unblocks retiring `penalty_latch_double_reading` — see
   docs/reader-retirement.md, "retire last, or never".
3. **The X half (arm 2)** — post-trigger `X` as penalty. Not the legacy 4+ stack:
   the probe says today's post-trigger doubler runs down to **0** cards and **0**
   HCP in the doubled suit, because without the agreement a post-trigger double
   is still the ordinary takeout double. The honest floor is `points ≥ 4`
   (0 / 231 contradicted, 5.5% of the population), possibly `hcp ≥ 6` at a 3.9%
   cost for 16.3%. Its own arm.
4. **The bare `points ≤ 19` ceiling** on the post-trigger pass — the one claim
   that narrows the hull unconditionally. Support-edge risk; its own arm.
5. **Positive conversion-pass readings** via the N3 catch-all-on-a-bid recipe
   (`nt_high_overcall.rs`). Structurally exempt from arm 1 — a conversion pass
   sits over RHO's *pass* — and it elects to defend, the opposite flavour, so it
   is its own design.
6. **The action side, once something can teach it.** Both gates lost because the
   BBA-distilled floor already plays post-trigger methods. The instrument that
   could improve on it is an oracle teacher with DD/par labels on the contested
   sit/pull/double — the competitive accountant
   ([ai-bidder/competitive-accountant.md](ai-bidder/competitive-accountant.md)),
   with the q calibration in
   [ai-bidder/doubling-calibration.md](ai-bidder/doubling-calibration.md).
7. **Their-side latch consumption** — the mask already records their triggers
   under `table_alerts`; nothing reads them.
8. **Forcing-pass PDI triggers** — the classic application. Marks auction
   states, not rules, which is why the tag stayed a `bool`.
9. **Card/`.bbsa` disclosure** — only if the knob ships default-on.
10. `competition.double_override` is tagged `.penalty_if(lo >= 2)` — the cut that
    separates the shipped `Optional` double (2..=3) from `Takeout` (..=3, which
    admits shortness). The probe cleared it: its lane is not a leak source.
    Revisit if a sweep ever wants a different boundary.

## Dropped, with reasons

Do not resurrect without new evidence.

- **The full P/X swap** — measured loss (P2, all four cells).
- **The ULP tie-break** — non-win in its only, caveated, measurement.
- **A level-bound (`≤3`) arm** — aimed at the wrong mode: the losing converted
  contracts (`2♠`, `3♥`, `3♦`) are all already ≤3, so it would have allowed
  essentially every losing board. The Task 3 probe independently kills it: a
  level bound does not sharpen the population either.
- **A distillation retrain to teach inversion** — anti-teaches; closed.
- **A freshness gate on the pass reading** — measured, buys nothing at an honest
  threshold (§ Task 3).
- **`suit_hcp` as the union's strength axis** — separates better but never
  collapses, so it never reaches the hull (§ Task 3).
- **A `park/pdi-latch` branch** — nothing to park. The default-off knob plus
  these verdict rows *is* the record (house rule: finished code → knob).
