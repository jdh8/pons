# Pass/double inversion (PDI)

> **Status (2026-08-29): a second, separate mechanism now exists — the
> [dialect-translation shell](#the-dialect-translation-shell-2026-08-29),
> knob `pdi_translate`, default off, A/B owed.** It is *not* a resurrection of
> the P2 logit swap: it triggers off a new per-rule divergence tag rather than
> off penalty-ness, and it rewrites the net's **input** instead of permuting its
> output. The rest of this document is the 2026-08-26 latch campaign, unchanged.
>
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

### The trigger theorem — a trigger locates at a P

Inversion needs *both* legs legal at the seat it governs, and one seat cannot
supply them. Hearing partner's `X` (RHO having passed), **doubling is illegal**:
the contract is already doubled by our side. So at that seat there is no `P`/`X`
pair to invert — the choice is sit-or-pull, not pass-or-double, and the only
thing an inversion could say there is already said by the pass.

A **pass over their live bid** is the seat that has both. It is also where the
information is: partner has to read our pass as an election, and the floor reads
it as ordinary. That is why the shipped triggers are *conversion passes* and why
the Landy suite below activates on **opener's pass**, not on a double.

The corollary matters for the shell: a divergent `X` has a synonym in the
teacher's dialect (S1 rewrites it to a `P`), but a **PDI-loaded `P` has none**
— there is no call that means "pass, but as an election" in a book that does not
play the agreement. So an `X`-side divergence can be translated and a `P`-side
divergence can only be **authored**. The two components of the Landy suite are
exactly those two repairs.

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
- **`Inferences.pdi_flip`** is the third, independent mask on the same rails —
  the divergence tag, no conversion passes folded in. See
  [the dialect-translation shell](#the-dialect-translation-shell-2026-08-29).
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
- **The configured neural floor has no PDI shell keyed to *this* trigger set.**
  The v6 floor distils BBA (`teacher: bba`, `dd_weight: 0.0`), and BBA already
  plays expert post-trigger methods — its post-trigger doubles are
  penalty-suggestive, its passes "nothing more to say". A shell that re-inverts
  the served logits is a *second* inversion on an already-inverted policy. That
  was **Mode B**. The floor *does* now carry a shell keyed to a different,
  narrower trigger — the per-rule divergence tag, on seats where BBA's book
  measurably reads our call as something else, translating its **input** rather
  than permuting its output. See
  [the dialect-translation shell](#the-dialect-translation-shell-2026-08-29).
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

**It does not go live if `players` is recomputed from the unions** — that was the
standing hypothesis, and `union_hull` (2026-08-26) refuted it. See "The union-hull
answer" below. The knob stays default off.

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

## The dialect-translation shell (2026-08-29)

A second mechanism, sharing nothing with the latch above but the file it is
documented in. The latch asks *what our own pass means to us*. This asks *what
our double means to the net we floor with*.

### The problem, in one measurement

The v6 floor is distilled from BBA, so it speaks BBA's book. Re-rendered under
the right card on 2026-08-29 (`probe-bba-book --conv "Multi-Landy=1" --vuls
none`, one node per row, `X` only):

| seat | our rule | BBA's label | their major | divergent? |
| --- | --- | --- | --- | --- |
| `1NT (2♣) X (2♥)` — §N1m opener | `X`@150 `len(♥,4..)`, penalty | **takeout double**, 17 HCP | ♥ 2–4 | **yes** |
| `1NT (2♣) X (2♠)` — §N1m opener | `X`@150 `len(♠,4..)`, penalty | **takeout double**, 17 HCP | ♠ 2–4 | **yes** |
| `1NT (2♣) X (2♥) - -` — §N1l doubler | `X`@155 `len(♥,4..)`, penalty | **reopening double**, 6–11 | ♥ 0–2 | yes, but see below |
| `1NT (2♣) X (2♠) - -` — §N1l doubler | `X`@155 `len(♠,4..)`, penalty | **reopening double**, 6–11 | ♠ 0–2 | yes, but see below |
| `1NT (2♣) X (2♦) - (2♥)` — §N1l escape leg | `X`@155 `len(♥,4..)`, penalty | **penalty**, 7–11 | ♥ 3–5 | **no** |
| `1NT (2♣) X (2♦) - (2♠)` — §N1l escape leg | `X`@155 `len(♠,4..)`, penalty | **penalty**, 7–11 | ♠ 3–5 | **no** |

The cached walk said the same thing about the §N1l seat a week earlier
(`ab-results/bba-book/2026-08-23-08c54312-dirty/00512.jsonl:3-4`, caveat
Multi-Landy=0), and the served floor behaves accordingly: it pulls our double to
`3NT` 49.5% of the time and converts ~17% (§N1l-flip caveat table;
`agreements.rs` says it outright). Every floor-owned node downstream of our
penalty double — their runout `X (2♥) X (2♠)` is deliberately the floor's
(`lebensohl.rs`) — is decided by a net that has misread partner.

### The tag

`Rule.pdi`, builder `.pdi()`, accessor `Rule::pdi_divergent` — a third,
independent bit beside `.alert()` (disclosure) and `.penalty()` (the latch's
reading switch). There is no `.pdi_if`: divergence is a measured fact about one
authored seat against the teacher's book, not a knob-conditional style.

**This is why the shell is not P2.** P2 swapped output logits over the whole
`.penalty()`-plus-conversion trigger set, which fires in dialect-**matching**
lanes too (the K–K Multi second `X` is penalty in both books) — so it
anti-taught the swap on every such board. The trigger here is a per-rule tag on
confirmed-divergent seats only. That is the new evidence "Dropped, with reasons"
demands.

**Only §N1m carries it.** §N1l's doubler rebid is deliberately left untagged,
and the table above is the reason: on its **preference** legs the tagged `X` sits
in the pass-out seat, so rewriting it to a pass ends the auction and S4 declines
— those legs can never translate. On its **escape** legs, which are the only ones
that *can* translate, BBA already reads the double as penalty. A tag there would
fire in exactly the dialect-matching lanes and nowhere else — P2's failure mode
in miniature. One `.pdi()` in `landy_doubler_rebid` reverses this if the
escape-leg reading is ever re-measured; the per-rule granularity is the point.

### The mechanism

`pdi_swap(auction, flips) -> Option<PdiPicture>` in `neural_floor.rs`, a pure
function; `flips` is the reading's tag mask scoped to the side to act.

- **S1** — a tagged `X` becomes `P`.
- **S2** — our sit over a tagged `X` (the `[X, P, P]` window, tag on the `X`)
  becomes `X`. A wider window is impossible: a third pass would have ended the
  real auction.
- **S3** — nothing moved, no picture.
- **S4** — replay through `Auction::try_extend`; **illegal or ended** → no
  picture, serve untranslated. One check subsumes the §N1l preference legs, their
  immediate `XX` of our tagged `X`, and every other inexpressible rewrite.
- **S5** — the auction ends `X P` with the tag on that `X`: the two pictures
  disagree about whether our double still stands, so `Pass` and `Double` trade
  **logits** before `mask_illegal(real)`. Dormant in this lane (that node is
  book-owned by `multi_signoff_pass`), built and unit-tested for the next tag
  site.

`forced(context)`, `mask_illegal` and `competitive_gate` all stay on the **real**
auction and context, so the shell can never introduce an illegal call.
`Fallback::Rebase` composes — rebased classifiers already classify the real
auction. Both configured floors get the shell (v6 and the v4
`ConfiguredFloorBba`); they share the teacher.

The lane's two floor-owned nodes, verified legal:

| real auction | served to the net |
| --- | --- |
| `1NT (2♣) X (2♥) X (2♠)` | `1NT (2♣) X (2♥) - (2♠)` |
| `1NT (2♣) X (2♥) X - - (2♠)` | `1NT (2♣) X (2♥) - - X (2♠)` |

**Known v1 approximation.** The second row is S2 moving the double to
responder's seat: the side aggregate is right, the seat attribution of the four
trumps is wrong. Accepted and priced by the A/B (falsifier 5 in
`scripts/ab-landy-opener.sh`), not papered over.

### Plumbing

```
Rule.pdi  ──►  CallMasks.pdi_flip  ──►  Inferences.pdi_flip  ──►  pdi_picture
  (tag)      (trigger_tags_live,        (read.rs, table-wide,      (neural_floor.rs,
              both drivers, one scan)     serde-skipped)            & our_side_mask)
```

`trigger_tags_live` replaced `penalty_trigger_live` and reads both tags in one
pass, consulting `face_live` only for a rule carrying a tag still unset — so an
untagged table costs exactly what it did before. `conversion_passes` is
deliberately **not** folded into `pdi_flip`: a structural conversion says nothing
about whether the teacher reads our call the way we mean it.

`Inferences.pdi_flip` is a mask rather than the latch's collapsed `bool` because
the shell needs positions. The hazard that justified collapsing `pdi_latched` is
handled instead by **zeroing the mask across the systems-on overcall strip**,
whose shortened auction would put every bit one index left of the caller's
length; the floor then serves untranslated, which is always safe.

### The knob

`InstinctProfile::pdi_translate`, **default off** — `--ns-pdi-translate`,
`set_pdi_translate` in the web registry. It lives in `DecisionProfile`, so
decision-cache identity is automatic. Cost: one extra uncached reading per
*translated* node only; `flips == 0` is one field read off the already-cached
reading.

**It is inert in the shipped default by construction.** The only tagged rule
exists behind `competition.landy_opener_px`, which is itself off, so the default
config has no tagged call to translate —
`pdi_translation_is_inert_in_the_default_config` pins that in-crate and
`smoke-default --count 20000 --seed 1` is byte-identical
(`38ee1e212f105204e84be973bf472ea78801785a7b67cace688f400ef0831afc`). That is
the KR1 non-inferiority proof, and it is why no default-config `xlate` arm is
owed.

### Measurement

`scripts/ab-landy-opener.sh` (owed, unrun) carries the arm: `pxt` =
`--ns-landy-opener-px --ns-pdi-translate`, beside `base | px | rungs`. Gates
0-foreign, both scorers, both vulnerabilities.

- **`landy_opener_px`** arbitrates on plain DD with SD-PD as tie-break
  (falsifier 4 — PD is blind to penalty doubles by construction), reading both
  `px vs base` and `pxt vs base`: the shell may be what makes `px` shippable,
  since falsifier 1's runout tail is exactly what it repairs.
- **`pdi_translate`** ships default-on only if `pxt vs px` reads win|win or
  wash|win. Its default-config exposure is already settled above. Note the arm
  narrowed once the tail was authored: `pxt vs px` now prices the deep floor tail
  alone, so a wash there is a real finding, not a null run.

§N1n has its own runner, `scripts/ab-landy-pdi.sh` (`base | pdi`), queued
**after** §N1m's — sequential, one run saturates the box, fresh `SEED_BASE`.

### The full suite against Landy (2026-08-29)

The shell alone is half a repair. Applied to the `1NT (2♣)` lane the whole thing
is two components, and the trigger theorem says which is which.

**1. Fully author the penalty `X`, tail included.** §N1m's `X`@150 is an
artificial call whose interfered continuations were the floor's — and that floor
reads the call as takeout, so disclosing our true length does not help it. The
completeness rule applies, and the tail is now book:

| node | seat | rows |
| --- | --- | --- |
| A1 `X (2M) X (2M′)` | responder | `X` penalty `len(M′,4..)` · `Pass`@0 |
| A2 `X (2M) X - - (2M′)` | opener | `X` penalty `len(M′,4..)` (4-4 in their majors) · `Pass`@0 |
| A3 `X (2M) X (XX)` | responder | `Pass`@0 — partner sits over their redouble |
| sits | partner of each new `X` | `{path} X -` → `multi_signoff_pass` |

**2. Activate PDI on opener's pass** — knob `competition.landy_pdi`, default off,
§N1n. Opener's `P` over the advance is the trigger, and no input translation can
repair its consequences, so the calls after it are authored rows contrary to the
floor:

| node | seat | rows |
| --- | --- | --- |
| B1 `X (2M) - (2M′)` | responder | `X` penalty `len(M′,4..)` · `Pass`@0 |
| B2 `X (2M) - - X (2M′)` | opener | `X` penalty `len(M′,4..)` · `Pass`@0 |
| sits | partner of each new `X` | `{path} X -` → `multi_signoff_pass` |

`M′` is the other major at its cheapest legal level — `2♠` out of a doubled
`2♥`, `3♥` out of a doubled `2♠`, so the spade leg's whole chase sits one level
up. Every gate is the lane's twice-measured `len(major, 4..)`, every new `X`
carries `.alert(LANDY_PENALTY)` (one claim, many seats — guarded by an explicit
`Alerted`-scope test, since the package invariant cannot see a suffix-guarded
double) and `.penalty()`.

**§N1l is already the P branch's centrepiece**: `landy_doubler_px`, shipped
default-on, is the doubler's delayed `X` at `X (2M) - -` — a double after our own
pass. B2 is that shipped rung's *own* interfered tail, which no arm had ever
authored. §N1n is deliberately **independent** of `landy_opener_px`: the trie
matches these patterns whether opener's pass and the delayed double came from the
book or from the floor.

**No new row is `.pdi()`-tagged.** A-nodes are book-owned, so there is nothing
for the shell to translate; B-nodes' divergent call is a pass, which by the
theorem has no synonym to translate into.

**Consequence for the shell's scope.** After full authoring, `pxt ≡ px` at every
book-owned node — the shell never touches a book decision — so `pxt vs px` now
reads only the **deep floor tail** below the authored chase (they run twice, they
run to a minor, they run over the redouble). Kept as an arm anyway: that residual
is what the tag was built for, and a null result there is itself the answer to
"how much is left once the book is complete".

This supersedes `LANDY_PENALTY`'s old polarity note — "a double after our *pass*
is takeout and stays the floor's". It is takeout on the floor, which is the
defect; §N1n is the repair.

### Follow-ons

- **Their-side translation.** The mask records their tagged calls too under
  `table_alerts`; the shell scopes itself to our side and nothing reads theirs.
- **Forced-rail interplay.** `forced(context)` runs on the real auction and short
  -circuits before the shell. If a future tag site sits under a rail, the two
  need a stated precedence.
- **Future tag sites.** S5 is dormant here. The first tagged seat whose sit node
  is *not* book-owned exercises it live, and should be measured knowing that.
- **§N1l's escape legs**, if the "BBA reads it as penalty there" render is ever
  overturned.

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
| 2026-08-29 | **The dialect-translation shell**: `.pdi()` tag, `pdi_swap` S1–S5, `pdi_translate` knob; §N1m tagged, §N1l deliberately not | — | shipped opt-in, **inert in the default config by construction**; `smoke-default --count 20000 --seed 1` byte-identical (`38ee1e21…`). A/B owed: `pxt` arm of `scripts/ab-landy-opener.sh` |
| 2026-08-29 | **The full suite against Landy** — §N1m's runout tail authored (A1/A2/A3 + sits, riding `landy_opener_px`) and the P branch authored behind new `competition.landy_pdi` (B1/B2 + sits), off the trigger theorem | — | shipped opt-in, both branches default off; `smoke-default --count 20000 --seed 1` byte-identical. A/Bs owed: `scripts/ab-landy-opener.sh` (px now carries its tail) then `scripts/ab-landy-pdi.sh` |
| 2026-08-29 | **Step 0 renders** — both §N1m seats and all four §N1l legs, `probe-bba-book --conv "Multi-Landy=1" --vuls none` | — | §N1m reads *takeout* (their major 2–4) at both legs; §N1l reads *reopening* (0–2) on the preference legs but **penalty** (3–5) on the escape legs — which is what disqualified §N1l from the tag |
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

## The union-hull answer (2026-08-26) — follow-on 1, closed negative

Follow-on 1 asked whether recomputing `players` from `unions` would make arm 1
live. `decision.reading.union_hull` (`--ns-union-hull`, default off) does exactly
that, narrow-only, and the answer is **no, and not for arm 1's reasons**.

### What the knob does

`Inferences::assemble` sets `players[i] = unions[i].hull()` when the hull is
inside the walk's box. The guard is not ceremony: [`Range::intersect`] *spans*
instead of inverting on a contradicted axis, so `intersect_owned`'s
all-boxes-contradict fallback can leave `unions[i]` **wider** than the box it was
cut from — 8 seats in 21,151 replayed decisions, and 1,264 in 2.14M on the
`announced` twin, which has no such guard (see "What this exposed").

### The numbers

`scripts/precount-union-hull.sh`, `SEED_BASE=1787700673` (the arm-1 pre-count's
seed, so the two are board-for-board comparable), 204,800 bd/arm/vul vs BBA,
results in `/mnt/hdd-data/jdh8/pons-ab-results/union-hull-precount`.

| | none | both |
| --- | --- | --- |
| our decisions replayed | 2,139,817 | 2,115,663 |
| a seat's hull moves | 1,090,132 (50.9%) | 1,041,405 (49.2%) |
| …on axis `hcp` / `support_points` | 1,435,394 / 1,398,104 | 1,369,457 / 1,339,277 |
| …on axis `lengths` / `points` | 25,647 / 4,963 | 24,026 / 5,269 |
| **the call the bidder makes flips** | **0** | **0** |

Contract diff of the generated arms: **0 boards moved in 819,200 tables** (both
vulnerabilities, both tables) — the two arms are byte-identical. Arming
`pdi_latch` on *both* sides first, so arm 1's union is present for `union_hull` to
expose, adds 588 + 511 drifted decisions and 676 + 601 more `lengths` narrowings
— the PDI union reaching `players` exactly as designed — and **still flips zero
calls**.

### Why

The drift is real and enormous, and it is on the wrong axes. Exclusion folds
(`bid_exclusion`, shipped on) disjoin on strength, so each box canonicalizes its
own `hcp` and `support_points` from its own `points` slice and the union's hull
beats the walk's single canonicalize. But the deterministic book gates and
`instinct()` gauge on `points` and `lengths` — which move on 0.2% and 1.2% of
decisions — and the v6 floor's argmax does not turn on eighteen floats a seat.
That is the same lesson as arm 1's funnel, one layer up: **the last step is the
one nobody predicts.**

Generalisation for any future union-shaped reading: making it visible to the
deterministic path is *not* the blocker. Either the claim lands on `points` or
`lengths`, or it will not move a call no matter which field it reaches.

### What this exposed

Two doc/code discrepancies, flagged rather than resolved:

1. **`Inferences::players` is not "a redundant cache of `unions[i].hull()`"**, and
   **`announced_players` is not "equal to `players`" with the `announced` knob
   off.** Both were true until `envelope_union` shipped default-on (chop F2b,
   `docs/dnf-migration.md`) and neither comment was updated. Measured on the
   shipped default: `get()` and `announced()` disagree on **51.0%** of our
   decisions, on 89.4% of boards. The **nets have always read the union-tightened
   hull** (`features_v6`/`features_eval_v5` take `announced(who)`) while the book
   gates read the walk's looser one. Field docs corrected in `read.rs`; the
   behaviour is untouched.
2. **The announced side has no narrow-only guard**, so 1,264 of 2.14M decisions
   hand the nets a hull *wider* than the walk's own box. Applying the guard there
   is a change to the nets' inputs, so it needs its own proof — filed, not
   shipped.

## Follow-on queue

1. ~~**Make a reading visible to the deterministic path.**~~ **Closed negative
   2026-08-26** — see "The union-hull answer" above. The knob exists
   (`union_hull`, default off) and moves zero calls in 4.26M decisions, PDI armed
   or not. Its live value is the other direction: flipping it aligns the book
   gates with what the nets already see, so it becomes a candidate again only
   behind a floor retrained on the tighter hulls.
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
10. **The dialect-translation shell's own queue** — their-side translation, the
    forced-rail precedence, and the first tag site that exercises S5 live. See
    [the section above](#the-dialect-translation-shell-2026-08-29).
11. **The Landy suite's deep tail** — the chase closes at one double per side
    per branch. Below it (they run twice, they run to a minor, they run over the
    redouble) the floor is back, reading the whole penalty process as takeout.
    Whether that is worth authoring is exactly what the narrowed `pxt vs px`
    reads.
12. `competition.double_override` is tagged `.penalty_if(lo >= 2)` — the cut that
    separates the shipped `Optional` double (2..=3) from `Takeout` (..=3, which
    admits shortness). The probe cleared it: its lane is not a leak source.
    Revisit if a sweep ever wants a different boundary.

## Dropped, with reasons

Do not resurrect without new evidence.

- **The full P/X swap** — measured loss (P2, all four cells). The
  [dialect-translation shell](#the-dialect-translation-shell-2026-08-29) is the
  new evidence this list demands, and it differs on both axes: a per-rule
  divergence tag instead of the penalty trigger set, and input-side translation
  instead of an output-side permutation.
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
