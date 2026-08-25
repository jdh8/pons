# Pass/double inversion (PDI)

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

The X-half is arm 1 (shipped as a knob, below). The P-half is arm 2 and is not
built yet.

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
- **`Inferences.pdi_latched`** is the single source of truth the floor and the
  reading both consume — strictly stronger than the one-knob-field contract the
  legacy 1NT latch uses. It is a `bool`, not the mask, because the systems-on
  overcall strip re-reads a *shortened* auction: a mask handed out would be
  indexed against the stripped auction while the caller holds the unstripped
  length, and the parity test would silently invert. Collapsing it inside
  `Inferences::read`, where the matching length is in scope, makes that
  unrepresentable.

Their side's triggers are recorded too (the mask is table-wide under
`table_alerts`), but `pdi_latched` is scoped to the side to act, so a trigger of
theirs never latches us.

## The X half (arm 1)

Knob `decision.reading.pdi_latch`, **default off** pending the A/B.

- **Floor** — `instinct::pdi_latched(context)` is
  `penalty_latched(context) || (profile.pdi_latch && context.inferences().pdi_latched())`,
  consumed through `Context::inferences()` (memoized on every path), never the
  raw authored-projection attachment (absent below depth 8 and on plain
  classify). Knob-off short-circuits to today's expression exactly. It is swapped
  into the three existing wrappers — `penalty_latched_c`, `may_pull_penalty`,
  `not_penalty_latched` — which covers the sit, the generic latch `X` and its
  Optional sibling, and the takeout-X / notrump suppression. No new rules or
  weights, so no tie-report exposure. Fires only through the full `Partnership`:
  a bare context has no prefixes, so the tag half of the trigger set is empty.
- **Reading** — post-walk in `read.rs`, keyed off the same mask (not a new
  `readers.rs` entry — see docs/reader-retirement.md for the direction of
  travel). Each of our doubles after the earliest same-side trigger, whose most
  recent preceding bid is theirs, narrows the doubler to **4+ in the doubled
  suit**, mirroring `penalty_latch_double_reading`. "More vague" means no points
  claim. The legacy 1NT reader makes the identical claim where the two lanes
  overlap, so no skip list is needed — intersecting a range with itself is a
  no-op. **Authored doubles are skipped**: their own rule already says what they
  promise, and a book that keeps a *takeout* double alive after a trigger (the
  Kokish–Kraft delayed double is exactly that) would otherwise be intersected
  with a contradictory four-plus and stop admitting its own bidder.
- **v1's pass side reads nothing.** "I cannot punish them" is the negation of a
  conjunction and is not expressible in the interval envelope. The documented
  route for a lane that wants a positive conversion-pass reading is the N3
  catch-all-on-a-bid recipe (`nt_high_overcall.rs`).

### Testbeds

| lane | role |
| --- | --- |
| UvU `1NT (2NT) X` | the tag-path testbed — book-authored trigger, default-armed, zero authoring cost |
| Kokish–Kraft | negative control: its double split is trie geometry, so the delta must be zero (`kokish_kraft_unchanged_under_pdi`) |
| takeout-X conversion (`(1♥) X - -`) | the dominant firing lane; the A/B measures the knob whole |

### Legacy latch, left beside

The one-lane prototype — knob `ReadingProfile::penalty_latch`, detector
`penalty_x_reading` hard-coded to "(1NT) X our first action", floor gate
`penalty_latched`, reader twin `penalty_latch_double_reading` — is untouched in
v1 by decision. Re-keying it through the tag is follow-on (1) below.

## Verdicts

| date | change | arms | verdict |
| --- | --- | --- | --- |
| 2026-08-26 | Step 0: `penalty_latched` reads the *pinned* notrump-defense profile | — | shipped; `smoke-default --count 20000 --seed 1` byte-identical (the default defense is Natural) |
| 2026-08-26 | P0: tag + conversion detection, no consumer | — | shipped; byte-identical |
| 2026-08-26 | P1: `pdi_latch` X-half, default off | — | shipped opt-in; **A/B owed** |

## Follow-on queue

1. **Re-key the legacy 1NT latch through the tag** (user-mandated). Needs its
   own `smoke-default` byte-identity proof. Unblocks retiring
   `penalty_latch_double_reading` — see docs/reader-retirement.md, "retire last,
   or never".
2. **The P half (arm 2).** Partner reads our post-trigger `P` as takeout; the
   floor reopens/balances behind it.
3. **Positive conversion-pass readings** via the N3 recipe.
4. **Their-side latch consumption** — the mask already records their triggers
   under `table_alerts`; nothing reads them.
5. **Forcing-pass PDI triggers** — the classic application. Marks auction
   states, not rules, which is why the tag stayed a `bool`.
6. **Card/`.bbsa` disclosure** once the knob ships default-on.
7. `competition.double_override` is tagged `.penalty_if(lo >= 2)` — the cut that
   separates the shipped `Optional` double (2..=3) from `Takeout` (..=3, which
   admits shortness). Revisit if a sweep ever wants a different boundary.
