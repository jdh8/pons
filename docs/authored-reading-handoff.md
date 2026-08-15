# Authored calls speak for themselves — handoff

**Written 2026-08-15**, out of the N2 forensic
([one-notrump-competitive.md](one-notrump-competitive.md) §N2).
Found at one node; the defect is the whole book's. This file states the
principle, maps where the code violates it, prices it, and lays out the
program to fix `pons` as a whole — with N2 as the local testbed every phase
is checked on first.

Prerequisites: [reading-drift-handoff.md](reading-drift-handoff.md) (the three
reading regimes; its step 4 is this campaign), [dnf-migration.md](dnf-migration.md)
(what happens when a *true* reading meets a frozen net — C1, C2, F1, F2b),
[reader-retirement.md](reader-retirement.md), [ai-bidder/sampled-projection.md](ai-bidder/sampled-projection.md).

## The principle (jdh8, 2026-08-15)

> A bid being natural does not mean that other people can magically know its
> strength and length ranges. "Alert" is not for all authored nodes. Authored
> nodes should speak for themselves whether alerted or not.

Three separate things are conflated today:

| concept | what it is for | who consumes it |
| --- | --- | --- |
| **alert** | disclosure — "this call is artificial" | `.bbsa` cards, `artificial_calls_are_alerted`, opponents' rights |
| **reading** | what the call *promises* — the union of its live rules | sampler, authored gates, instinct floor, the nets |
| **the natural walk** | a guess from auction *shape* when there is no rule | unauthored calls, undeclared opponents |

The engine uses the first as the switch for the second, and lets the third
override the second wherever the first is absent. Target state: **the reading
of an authored call is its rules, two-sided, always; the walk fills in only
where nothing is authored; the alert decides nothing about reading.**

## The evidence, in one probe

`probe-decision "Q93.K43.AKJT.Q42" "1NT 2♠ 2NT - 3♣ - 3♦ -"` — opener to act
after responder's weak Lebensohl relay and sign-off, both authored:

| what responder's calls say | rule | what opener reads |
| --- | --- | --- |
| `2NT` relay | `points(..=8) & (5+ suit not theirs) & hcp(6..)`, alerted | `hcp 6..37`, `points 6..37`, suits `0..13` |
| `3♦` sign-off | `min_level_is(3,♦) & len(♦,5..)`, natural | nothing added — ♦ `0..13` |
| decision | — | floor (`depth 0, fallback 0`) bids **`3NT` 1.400** over Pass 0 |

Anchor cost of that one node: opener bids `3NT` over the weak `3♦` on 16 of 18
boards, −52 plain / −125 PD. `PROBE_SCOPE=all` (`ReadingScope::All`) gives ♦
`5..13` back; the ceiling stays `..37` and **the floor still bids `3NT`**. The
natural `1NT 2♥ 2♠ -` reads as *nothing at all* under the default.

## Where the principle is violated — the code map

### 1. The scope gate: reading is switched by the alert

[projection.rs `authored_effect`](../src/bidding/inference/projection.rs):

```rust
let decode = match scope {
    ReadingScope::None    => false,
    ReadingScope::Alerted => alerted,   // shipped default
    ReadingScope::All     => true,
};
```

`ReadingProfile::scope` ([knobs.rs](../src/bidding/inference/knobs.rs)) is
`Alerted` by default. `All` is built (`bba-gen --ns-reading-scope all`,
`probe-reading-sound --ns-natural-reading`), *intersects* the rule with the
walk (does not set the suppression bit), and is **unmeasured** — deferred by
the reading-drift ledger to "its own campaign". This is it.

Size: `src/bidding/{american,dutch}` hold ~1,400 `.rule(` calls and ~350
`.alert(` tags. Roughly **a thousand authored rules project nothing** under
the default; each is a regime-2 row of the reading-drift table.

### 2. Strength projects floor-only — every made-bid ceiling is lost

[constraint.rs](../src/bidding/constraint.rs) `Points::project` /
`Hcp::project` write `floor..=37` ("floor only, matching every hand-written
reader; the ceiling returns in `project_band`, widened by
`hcp_ceiling_slack`"). `project_band` — the two-sided one — is used **only for
the pass reading** (`project_pass`).

The rationale is stale. The rule evaluates `point_count(hand) ≤ 8` on the same
deterministic scale the envelope's `strength.points` records, so the ceiling is
**exact on its own axis**; the other axis follows with the known slack
(`hcp ≤ points ≤ hcp + upgrade_ceiling`), which `hcp_band` /
`upgrade_ceiling` already compute. The comment predates the two-axis
`Strength`. Fuzzy strength (`fuzzy_fifths`) is off; when on, widen by the fuzz
exactly as `hcp_ceiling_slack` does — a knob, not a reason to drop the bound.

Size: **244 explicit ceilings** (`points(..=N)`, `hcp(..=N)`, `lo..=hi`
bands) in authored rules, 169 of them two-sided bands. Every one of them reads
as `..37` on a made bid, alerted or not. Every weak sign-off, every limited
raise authored as a band, every "to play" call reads *unlimited* to the floor.
This is the **binding** defect at N2 (`All` alone leaves the `3NT` in place)
and the one the reading-drift handoff filed as "the other three known leaks,
first bullet".

### 3. The walk overrides, and the projection only intersects

[read.rs](../src/bidding/inference/read.rs): after a 1NT opening the walk
*blankets* every suit bid on the opening side except a lane's first
three-level call (`nt_blanket`, `over_one_notrump`, `stayman_artificial`,
`nt_splinter_artificial`, `nt_structure_artificial` — one predicate per
convention that ever contradicted it). Right for the uncontested transfer
structure; wrong for the sohl sign-off, and it is why cause 1 alone cannot
save the sign-off's length. Under `All` the rule is *intersected with* the
walk's guess so the walk's bookkeeping (natural-suit lanes, agreed fits, cue
detection) survives — which is why "wrong walk ∧ sound rule = **empty box**"
(reading-drift step 4's warning). The bookkeeping is derived from the bid's
*face*, not from what the bid promised.

### 4. Who consumes the reading — why this is a bidding change

- **The nets.** [features.rs `push_inference`](../src/bidding/features.rs):
  per hidden seat, exactly `4 × (len.min, len.max) + (points.min, points.max)`.
  Ceilings and lengths flow straight into the v5 policy floor and the
  evaluator (`trick_estimates`). Both were fit on Alerted-scope, floor-only
  readings. `net_points` is the existing precedent for keeping a frozen net
  on-distribution while the reader changes underneath it.
- **The sampler.** `Inferences::admits` tests lengths and `points` — a
  ceiling that arrives *removes* hands the sampler was dealing outside the
  stated band (C2's 249 rejections). Unlike C1 this is information-carrying,
  so a retrain can be earned on it.
- **Authored gates** read `.min` (`support_floor`, `fit_sum_game`,
  `keycard_trump`) and, once ceilings exist, will read `.max`
  (`combined_hcp`, the NT-slam milestones).
- **The instinct floor** consults the auction's interpretation
  (`set_inference_aware`) — the deterministic floor moves too.

So per reading-drift's second mechanism: **every phase below is an A/B, read
from the decision table, never a soundness argument.**

## Target design

1. **Reading = the rules, always.** For every authored call — ours, and theirs
   when a book is declared — the reading is the union of the call's live
   rules under the bidder's at-the-time context (`authored_effect` already
   does this; drop the `Alerted` gate). Sound by construction: with a finite
   catch-all at every table (iron rule) a hand makes call C only through a
   C-rule it satisfies, so the union is a superset of the truth — loose,
   never wrong. Alerts keep their disclosure job untouched.
2. **Two-sided strength.** Forward projection of `points`/`hcp` becomes the
   band (`project_band`'s arithmetic), exact on the rule's own axis, slacked
   on the other; fuzz-widened when a fuzzy gauge is on. Passes already do
   this — made bids catch up.
3. **Substitute, don't intersect.** An authored call *owns* its reading and
   sets the walk's suppression bit; the walk's lane bookkeeping is derived
   from the projected envelope (a suit projected `4+`/`5+` counts as bid
   naturally; a projected fit is an agreed fit) instead of from the bid's
   face. Then `nt_blanket` and its per-convention exceptions become dead code
   for authored calls, and the walk shrinks to genuinely unauthored auctions
   and undeclared opponents.
4. **Negative inference** (later). Argmax means "made C" also means "no
   higher-weighted rule at that node held": fold `project_complement` of the
   higher rules into C's reading. Exact for argmax; it is what turns "passed
   the relay rather than transferring" into `< 9 points` with no reader.
5. **Frozen nets get a compat view, then a retrain.** Separate the *reading*
   (truth, served to the sampler, gates and instinct floor) from the
   *encoding* served to nets that were fit on the old reading:
   `features::legacy_view` reconstructs the training-time hull (Alerted-scope,
   floor-only) at every node — the `net_points` fold generalised — so phases
   1-3 can be measured with the nets held on-distribution. Then
   `features_v6` = the honest reading, regen + retrain by the F2b recipe
   (`dump-evaluator`, `dump-teacher` with the new profile; held-out gate;
   knob-matched twin selected per call), flip, and delete the view.

## The soundness gate

Everything ships through the behavioural sweep, not the static one:

- `readings_admit_the_bidder` ([inference/tests.rs](../src/bidding/inference/tests.rs))
  — replay the *bidder* over seeded hands at a node, require partner's reading
  to admit the hand, both regimes. Rows land with the repair that greens them.
- `probe-reading-sound` — the ambient exclusion rate (partner **2.114%** on
  the 2026-08-16 re-baseline, 40,000 boards, seed 20260816; the recorded
  2.36% was a smaller, unseeded run). Re-run per phase; a phase that *raises*
  it has a walk defect it just exposed (empty boxes are the diagnostic), and
  that defect is cleared before the A/B, not after. **Pin `-s`** — the probe
  seeds randomly when the flag is omitted, and two unseeded arms are not a
  comparison.

## Phase 1 — the soundness gate (2026-08-16)

jdh8: *"this is a general problem, just found when developing N2 — prove
general soundness first."* So the whole-book proof came before the N2 probe
and before any A/B. It is green; what follows is the record.

### What is in the code

`ReadingProfile::strength_ceilings` (**default off**). On, the forward folds
of `points`, `hcp` and `support_points` *are* their `project_band` — one
branch each in [constraint.rs](../src/bidding/constraint.rs); `And`/`Or`/
`Flip`/`ReadsAs`/`EnvelopeUnionUpgrade`/`announce` compose it for free.
`DecisionProfile::legacy_view` (**default off**) is the frozen-net hedge:
`Context::net_inferences` serves [features.rs](../src/bidding/features.rs)
and the evaluator's trick estimates a *second* reading taken with the
ceilings switched back off, while the sampler, the authored gates and the
instinct floor keep the true one. Surfaces: `bba-gen --ns-strength-ceilings`
/ `--ns-legacy-view`, `probe-reading-sound --ns-strength-ceilings`,
`probe-decision`'s `PROBE_CEILINGS=1` / `PROBE_LEGACY_VIEW=1`, and both web
knobs.

### The proof, in five layers

| layer | what it claims | where |
| --- | --- | --- |
| fold arithmetic | `point_count − raw_hcp ∈ [−flat_hcp_slack, +hcp_ceiling_slack]` on all four scales, 20k seeded hands — the bound `hcp_band` and `Points::project_band` rest on | `point_scale_slacks_bound_the_upgrade` |
| leaf + combinator | knob-on, a finite `eval` implies membership of the projected union: 2k hands × 3 bands × 4 scales × 2 union regimes × {`points`, `hcp`, `support_points`, `&`, `\|`, `!`}, with `gauge_membership` **on** so every axis is strict | `forward_ceilings_admit_every_accepted_hand` |
| the identity itself | knob-on `project == project_band` for the three, composition included, and a catch-all still projects ⊤ | `forward_projection_is_the_band_under_ceilings` |
| **book-wide (E0)** | the same eval ⟹ membership claim replayed under the ceilings profile over **every authored rule** of `american()`'s three tries and `dutch()` constructive, plus the fallback layer | `authored_rules_eval_within_projection`, second pass |
| behavioural | replay the *bidder* at a node, require partner's reading to admit the hand — now over the 4-cell grid `{Alerted, All} × {ceilings off, on}` (`READING_REGIMES`) | `readings_admit_the_bidder`, `completion_readings_admit_the_bidder`, `gladiator_readings_admit_the_bidder` |

Plus the two hedge tests — `legacy_view_serves_the_nets_the_pre_ceilings_reading`
(the N2 relay reads `points 6..=8` to everything, `6..=37` to the nets,
memoised once) and `legacy_view_reproduces_the_pre_ceilings_feature_vector`
(the held `features_v5` vector is byte-identical to the shipped one, and the
view-off arm is not — so the two A/B arms genuinely measure different things).

**Default inertness while the knobs were off** was the seeded diff, not the
knob default: `smoke-default --count 20000 --seed 1` still hashed
`18aba5ce…`.  Both knobs shipped default-on 2026-08-16 and the constant is now
`cf583ff5f46d7e7ffdf0ab065dcb285680a6b7d865df42cf5e139f0b74ab7b90`
— a *deliberate* re-base, and the first thing to re-verify if a later
"byte-identical" claim in this repo cites the old hash.

E0 cost 53.8s → 69.7s (a quarter hand pool on the second pass; the sweep is
hand-proportional and this axis only moves strength bounds).

### The ambient probe — 40,000 boards, seed 20260816

| arm | partner (ours) | LHO (them) | RHO (them) |
| --- | --- | --- | --- |
| `Alerted`, floors (HEAD) | 2.114% | 7.622% | 7.714% |
| `Alerted`, **ceilings** | **2.105%** | 8.036% | 8.084% |
| `All`, floors | 2.135% | 10.447% | 10.612% |
| `All`, **ceilings** | **2.111%** | 11.192% | 11.304% |

The gate is partner, and the ceilings *lower* it in both scopes. Under
`Alerted` the partner worklist's top 30 is **byte-identical** between arms —
no node newly excludes its own bidder, so there is no empty-box repair queue
to clear and the cap of ~5 repairs was never touched. Under `All` one node
improves (`2♠ 3♥ - 3♠ - 4♥`, 20→18) and one 10/12 node enters the bottom of
the list.

**The finding to carry into the A/B**: our reading of *their* calls gets
measurably wronger — LHO/RHO exclusions rise 0.41/0.37pp under `Alerted`,
0.75/0.69pp under `All`. That is not a defect in our book; it is the honest
price of reading a foreign system with our own, now tighter, meanings, and it
is the mechanism by which a *sound* correction could still measure as a loss.
If Stage C loses, the first thing to try is not a retrain but scoping the
ceilings to our own two seats, the way `blind_opponents` already cuts at the
source. Do not build that knob before the A/B says it is needed.

### Corrections to the sections above

- §2's fuzz remark is moot for `points`/`hcp`: `fuzzy_fifths` routes through
  `Fifths`, which projects ⊤ and is untouched here. It is Phase 2 territory.
- §2's "244 explicit ceilings" is Phase 1+2's reach. Under the shipped
  `Alerted` scope Phase 1 reaches **alerted** calls only.
- `SupportPoints::project` had the same defect and is in scope; it is exact
  on its own dedicated per-suit gauge, so its band needs no slack at all.
- The **pass** reading is knob-invariant: `project_pass` folds only
  `project_band` and `project_complement`, neither of which reads the knob.
  `passes_read_within_their_table` therefore needs no ceilings arm.
- `strength_dial` stays out of scope **by design** — it shifts `eval` and
  nothing else (eval-only, default 0), so no projection tracks it in either
  direction. Recorded in the test doc rather than left to be rediscovered.

### The whole-book A/B, and why it is a wash

`--ns-strength-ceilings` against the same binary, **three independent seeds**,
204,800 boards/arm/vul each (`ab-results/p1-ceilings{,-s2,-s3}`, seeds
1786827680 / 1786828611 / 1786829005), firing on 0.10-0.11% of boards:

| cell | Σ IMPs (3 seeds) | per board | pooled CI | per-seed signs |
| --- | ---: | ---: | ---: | --- |
| NV plain | −204 | −0.00033 | ±0.00052 | `- - -` |
| NV PD | +188 | +0.00031 | ±0.00058 | `+ + -` |
| vul plain | −312 | −0.00051 | ±0.00069 | `- - -` |
| vul PD | +140 | +0.00023 | ±0.00081 | `- + +` |

Every cell's CI straddles zero, so by the letter this is the non-loss the
pre-agreed rule would ship. **Do not read it that way.** Plain DD is negative
in every seed at both vulnerabilities, six readings for six — but the two
vulnerabilities are the *same deals* re-priced (`ab-reading-drift.sh` passes
one `SEED_BASE` to both), so the independent count is three, not six, and the
sign test is p ≈ 0.25, not 0.03. Do not pool the two vulnerabilities into one
CI either, for the same reason; read the NV and vul rows separately. What
survives that discipline is weaker but still one-directional: **plain DD never
came out positive, on any seed, at either vulnerability**, while PD leans the
other way — the shape the decision table calls *"artifact of PD's synthetic X.
Not a win."* On this evidence the raw arm is a marginal non-loss at best.

The N2-enriched testbed (`--filter-1nt` on both arms, seed 1786828220) is the
one population that leans positive throughout — NV +0.0002/+0.0006, vul
+0.0004/+0.0007, all washes — which is consistent with the ceilings being
mildly *right* where weak calls are dense and mildly *wrong* where they read
the opponents.

The 0.10% firing rate is the number to explain, and the reason is not that the
book has few ceilings. It is that **almost nothing reads one**:

| channel | reads a strength ceiling? | evidence |
| --- | --- | --- |
| instinct floor | **no** | zero strength-`.max` reads in `instinct.rs`; `Strength` has no ceiling accessor at all — `hcp_floor()`, `support_floor()`, `shown_floor()` all return a `.min`, and the first two touch `.max` only as a *populated* test |
| authored book | **no** | `PartnerShownPoints::eval` (constraint.rs:2717) tests `shown.min`; `PartnerShownLen` likewise |
| `hcp_floor()` / `support_floor()` flipping `None → Some` | gated off | their only consumers sit behind `nt_hcp_read: false` and `fit_sum_support_read: false` (instinct.rs:527-528) |
| sampler (`Envelope::admits_on`) | yes, genuinely two-sided — but `sample_layouts` is reached only from `single_dummy.rs:187` and `ev::ev_all`, so it moves **sd-lead**, not the bidding decision |
| nets | **yes**, and this is the dominant live channel: `features.rs` pushes `points.max` / `hcp.max`, fed through `Context::net_inferences`. Measured at **98.6%** of the divergence — see the `legacy_view` arm below |
| gates / slam asks | **yes, thinly** — the `legacy_view` arm leaves 3 boards in 204,800, all phantom keycard asks. Not predicted by this census; the gate sites keep `inferences()` by design |

So what the A/B priced is a **stale-net input perturbation**, not "the floor
now respects the ceiling" — which is precisely the confound `legacy_view`
exists to separate, and the reason the arm order matters. The consistent
plain-DD negative is what an off-distribution input perturbation of a frozen
net looks like.

### The `legacy_view` arm confirms the census

`--ns-strength-ceilings --ns-legacy-view` against the same base arms
(`ab-results/p1-legacy`, seed 1786829479, 204,800 boards/arm/vul). Holding the
nets at the pre-ceilings reading collapses the firing rate by ~70×, from
0.10% (212 boards NV) to **3 boards in 204,800**, and every cell is positive:

| cell | fired | Σ IMPs | per board | CI |
| --- | ---: | ---: | ---: | ---: |
| NV plain | 3 (0.00%) | +10 | +0.0000 | ±0.0001 |
| NV PD | 3 (0.00%) | +11 | +0.0001 | ±0.0001 |
| vul plain | 3 (0.00%) | +12 | +0.0001 | ±0.0001 |
| vul PD | 3 (0.00%) | +13 | +0.0001 | ±0.0001 |

That is the census made empirical: **98.6% of the raw arm's divergence was the
nets**, and the remaining 1.4% is three boards. Two corroborations, both cheap:

- **Runtime cost ~1%, not the 4× the doc comment feared.** Same binary, same
  4000 deals, seed 777: baseline 22.47s, ceilings 22.42s, ceilings+legacy
  22.68s. The second reading walk is uncompiled (`get_for_profile` returns
  `None` on profile mismatch) but it runs once per decision behind a
  `OnceLock`, and DD solving dominates the wall clock regardless. The
  *A/B* wall-clock is a bad proxy here — the legacy arm finishes **faster**
  than the raw arm because divergent boards are what cost DD time.
- **The accountant counters are a decision-side fingerprint.** Baseline and
  ceilings+legacy agree exactly (1489 bid vetoes / 164 double masks / 291 pass
  demotions); ceilings-only differs (1487 / 161 / 296). `legacy_view` restores
  the pre-ceilings *decision*, not merely the pre-ceilings feature vector.

The three survivors are one lane, and the ceilings win all three — they are the
N2 pathology in a **constructive** seat:

```
on:  - 1♦ - 3♦ - 3NT - - -
off: - 1♦ - 3♦ - 4NT - 5♦ - 6♦ - - -
```

An invitational jump read as unlimited drives a phantom keycard ask into a
failing slam; the ceiling signs off in 3NT. This is the first *measured* board
where a lost ceiling costs real IMPs, and it is worth keeping as the seed of a
consumer-side test — the channel is the gates/sampler side, which by design
keeps `inferences()` rather than `net_inferences()`.

### Where that leaves the ship decision

The evidence ranks the arms **legacy > raw > baseline**, which the pre-agreed
read-out does not cleanly cover: it branches on "raw loss" vs "raw non-loss",
and raw is neither — every CI straddles zero while plain DD leans negative in
3/3 independent seeds. Read by the letter, branch 1 fires (ship ceilings
alone, `legacy_view` opt-in) — but that ships the arm the evidence likes
*least*. Read by intent, branch 2 fires (ship both).

Arguments actually on the table:

| for shipping both on | for holding both off |
| --- | --- |
| It is the arm the data supports: 4/4 positive cells vs raw's 3/3 negative plain-DD lean | Measured gain is 3 boards in 204,800 — nothing, statistically |
| The default reading becomes **true**; a weak sign-off reading as unlimited is a bug, not a tuning choice | `legacy_view` is scaffolding with a scheduled removal (Phase 5 retrain); shipped scaffolds calcify |
| Removes a standing 2-arm cross (with/without ceilings) from every Phase 2-5 and C2 A/B | Two knobs on, one existing only to neutralise the other, is a confusing production state |
| Cost is ~1% throughput, measured | Zero change is the reversible state |

**Shipped both default-on 2026-08-16** on jdh8's call, taking branch 2 by
intent over branch 1 by letter: the arm the data actually likes is the one
that ships. `legacy_view` ships *as scaffolding* — Phase 5 retires it by
retraining the nets on honest readings, and its doc comment says so.

The flip is not free, and the six test failures it produced are worth reading
as evidence rather than chores — five of them were pinning the bug:

| test | was | now |
| --- | --- | --- |
| `landy_conditions_partner` | `points 8..37` | `8..15` — and the test itself sets `convention_points = (8, 15)` |
| `woolsey_conditions_partner` | `points 10..37` | `10..19`, likewise its own `(10, 19)` |
| `projection_reproduces_the_declarative_readers` | Landy `8..37` | `8..15`; reader and projection moved **together**, so the oracle still holds |
| `project_band_carries_ceilings` | asserted `project` is floor-only | split: band under the shipped default, floor-only under an explicit ceilings-off profile |
| `wrong_hand_does_not_fill_scoped_trick_cache`, `configured_floor_clone_reuses_the_decision_cache` | counted reads in `inference_inits` | the one read lands in the **legacy** slot under `legacy_view`; both now assert `plain + legacy == 1` |

Leaping Michaels kept `points 14..37` and stayed green throughout — its rule
carries no ceiling, so nothing to read. That is the control.

Post-ship checks: the `.bbsa` cards are **byte-identical** (`bba-card`
re-run, no diff), which is the "reading, not disclosure" claim surviving the
ship. The seeded default dump re-bases, deliberately:
`smoke-default --count 20000 --seed 1` moves from `18aba5ce…` to
`cf583ff5…`. A direct 20,000-board arm diff (`bba-gen --seed 1` default
against `--ns-strength-ceilings false --ns-legacy-view false`) shows **2
boards differing**, both the predicted lane:

```
on:  - 1♣ - 3♣ - 4♣ - - -            on:  - 1♦ - 3♦ - 3NT - - -
off: - 1♣ - 3♣ - 4NT - 5♣ - 6♣ - - -  off: - 1♦ - 3♦ - 4NT - 5♦ - - -
```

Both example flags became `Option<bool>` opt-outs on the repo's shipped-knob
idiom (unset = engine default; a pre-ship arm is `--ns-strength-ceilings
false`), because they *assigned* the knob unconditionally and would otherwise
have forced it off in every future run. `probe-decision` likewise reads
`PROBE_CEILINGS=0` / `PROBE_LEGACY_VIEW=0` now.

**⚠ Warn before flipping `nt_hcp_read` or `fit_sum_support_read`**: with the
ceilings on, `hcp_floor()`/`support_floor()` stop returning `None`, so either
flip silently makes `strength_ceilings` a floor-behaviour knob. `support_floor(suit)`
returns that suit's slot min, which is ≤ `shown_floor()`, so `fit_sum_game`
would get *more conservative* as a side effect.

### N2's `3NT` is not a strength decision at all

`PROBE_CEILINGS=1` fixes the reading at the N2 sign-off (`points 6..=37` →
`6..=8`) and the decision does not move: `3NT 1.400`, `P 0.000`, byte-identical.
Traced:

`opener_forced_past_invitation` (instinct.rs:3820-3824) is *"we opened a strong
notrump and partner forced past invitation with a three-level suit bid"* —

```rust
(our_strong_notrump(context, 1, false) || our_strong_notrump(context, 2, false))
    && partner_last_call(context.auction())
        .is_some_and(|bid| bid.level.get() == 3 && bid.strain != Strain::Notrump)
```

— pure auction shape. It sets `Interpretation::forced_to_game`, so `forced()`
makes `ConfiguredFloorV5::classify` take the deterministic rail (the net never
runs), and `auction_forces_game()` then pre-satisfies the game-milestone `Or`
at instinct.rs:5042, so `combined_hcp` is never evaluated. Weight 140 is the
1.400.

Verified on the probe: a **12-HCP** opener still bids `3NT`; the only
hand-dependent gate on the node is `stopper_in_their_suits()` (take the spade
stopper away and the `3NT` vanishes). The rule's own doc comment says it out
loud — *"so passing below game is wrong, whatever our hand"*.

**The predicate cannot tell a game-forcing three-level bid from a Lebensohl
sign-off** — the one three-level suit call that promises at most eight. That
is the N2 defect. It is independent of this whole campaign, it is a smaller
change than N2a, and it fixes every sign-off lane at once rather than one
node. It is now Phase 0b below.

## Program

Each phase: one knob (or two-binary), `bba-gen --filter-1nt` on N2 first
(the local testbed — decompose with `probe-1nt-interference --bucket 2♠
--responses 8`, the `2NT`/`3♦` rows and the opener-`3NT` count are the
target), then the whole-book anchor (`scripts/anchor.sh` /
`scripts/ab-reading-drift.sh`'s two-binary protocol), plain + PD, both vuls,
verdict from [measurement.md](measurement.md)'s table. Non-loss ships a
soundness correction; a loss traces its worst boards before any conclusion.

| # | Phase | Mechanism | Knob / protocol | Gate | Status |
| --- | --- | --- | --- | --- | --- |
| 0b | **`opener_forced_past_invitation` learns the sign-off** | instinct.rs:3820 forces to game off *any* partner three-level suit bid over our strong 1NT, Lebensohl sign-offs included; the rail then bypasses the net and pre-satisfies the milestone `Or` | knob or straight fix; measure on `--filter-1nt` | standard | **found 2026-08-16 by the Phase 1 probe** — the actual N2 defect, independent of every reading phase, and strictly smaller than N2a |
| 0 | **N2a** — opener passes the relay's minor sign-off | book node (`{relay} 3♦ -` → `Pass`, a `landy_signoff_answer` twin) shadows the floor; independent of every reading phase | knobless; measure on `--filter-1nt` | standard | queued — cheapest, fixes 16/18 regardless |
| 1 | **Strength ceilings** | `Points::project` / `Hcp::project` / `SupportPoints::project` → band | `reading.strength_ceilings` **+ `DecisionProfile::legacy_view`, both default-on since 2026-08-16**; pre-ship arm is `bba-gen --ns-strength-ceilings false --ns-legacy-view false` | admits sweep + `probe-reading-sound` unchanged-or-better; A/B | **SHIPPED 2026-08-16.** Soundness gate green (E0 book-wide + 4-cell behavioural grid + probe partner 2.114→2.105%); A/B 3 seeds raw + 1 legacy — raw leans plain-DD-negative, **legacy arm 3 boards/204,800, 4/4 cells positive, ~1% cost**. Shipped on the legacy arm. Cards byte-identical; smoke re-based `18aba5ce…` → `cf583ff5…`. C2's re-open trigger ("a two-sided forward projection") is met by it |
| 2 | **`ReadingScope::All`** as default | built; drop the alert gate | `--ns-reading-scope all`; same two-arm protocol | clear the empty-box worklist *before* measuring (`probe-reading-sound --ns-natural-reading`, bucketed) | queued behind 1 |
| 3 | **Substitute, don't intersect** | authored calls set suppression; walk bookkeeping from the projection; retire `nt_blanket` & co. for authored calls | two-binary (a refactor, not a knob) | byte-identity where projection ⊇ walk is *not* expected — this moves readings by design; A/B | queued behind 2 |
| 4 | Negative inference | fold `project_complement` of higher rules | knob | admits sweep (must stay green — it tightens) | later |
| 5 | **features_v6 + retrain** | honest reading in, `legacy_view` and `net_points` fold out | F2b recipe (dump, held-out gate, twin, flip) | held-out NLL/MAE ≥ shipped; A/B of the flip | after 1-3 land |

Why this order: 1 before 2 because at N2 the missing ceiling, not the missing
length, is what the floor spends; 2 before 3 because 3 is the cleanup that 2
makes possible (once every authored call projects, the walk's exceptions have
nothing left to protect); 5 last because it is the only phase that cannot be
undone with a knob.

## The N2 testbed — exact recipe

```bash
# the node, before/after any phase
cargo run --release --example probe-decision -- "Q93.K43.AKJT.Q42" "1NT 2♠ 2NT - 3♣ - 3♦ -" both
PROBE_SCOPE=all cargo run --release --example probe-decision -- "Q93.K43.AKJT.Q42" "1NT 2♠ 2NT - 3♣ - 3♦ -" both

# the lane, off an anchor arm (seconds, deal-keyed DD cache)
cargo run --release --features serde --example probe-1nt-interference -- \
    ab-results/anchor/<arm>/american-none --dd-cache ab-results/anchor/dd-cache.json \
    --bucket "2♠" --responses 8            # our-response / BBA-response / why-passed tables
cargo run --release --features serde --example probe-1nt-interference -- \
    ab-results/anchor/<arm>/american-none --dd-cache ab-results/anchor/dd-cache.json \
    --bucket "2♠" --show 40 --next "2NT"   # the relay boards; count "3♦ - 3NT"

# the isolated A/B for a phase, N2-enriched
scripts/idle-run.sh scripts/bba-gen-parallel.sh ab-results/<exp>/<arm> 19200 -v none --filter-1nt <knob flag>
```

Success at N2, per phase: opener's `3NT` over `2NT - 3♣ - 3♦` gone (0/18);
the `2NT` relay row no longer the lane's worst per board; `1NT 2♥ 2♠ -` reads
♠ `5..13` with a ceiling.

## Traps recorded elsewhere that apply here

- **A true reading can lose 4/4** (dnf-migration C1: hull *endpoints* moved
  without mass; nets non-invariant to the encoding + gates reading `.min`
  against thresholds tuned without it). Ceilings *do* move mass, so they are
  earnable — but the frozen nets still see displaced endpoints. That is what
  the `legacy_view` arm is for: it separates "the reading is right" from "the
  net was fit on the wrong one" in one run.
- **Wrong-seat trap** (dnf-migration F2b′): project under the bidder's
  at-the-time context; context-relative legs (`support(..)`, `min_level_is`)
  re-target if projected under the reader's context. `authored_effect` already
  does this; keep it when substituting for the walk.
- **⊤ is sometimes the only sound projection**: negations, `pred` net gates,
  RKCB answers, `Fallback::classify` logit rewrites (Gladiator's stolen relay).
  Substitution must fall back to the walk for a call whose projection is ⊤ on
  every axis, or the walk's bookkeeping loses a genuinely natural bid.
- **A reading knob is a bidding knob** (reading-drift §2): the instinct floor
  is inference-aware; every phase moves calls at every floor node.
- **Alerts and `.bbsa` cards** are untouched by this campaign by construction
  — reading no longer depends on them. Do not "fix" a reading by alerting a
  natural call (the completion-alert precedent was for a *puppet*, which is
  artificial).
- **Train/serve skew** (dnf-migration F1 side finding): the policy net serves
  on prefixed features and was trained on bare-context ones. Phase 5 is the
  place to close it, not before.
- **The strip** (`systems_on_overcall_strip`) is a claim that two structures
  coincide; phase 3 must keep it scoped per RHO call exactly as
  `gladiator_keeps_the_strip_where_it_has_no_structure` pins.

## Decisions for jdh8

1. ~~Phase 1 knob name and default~~ — **answered 2026-08-16**:
   `reading.strength_ceilings`, off in code, **ship on non-loss** (plain
   wash-or-win + PD non-loss, both vuls). Scope is all three point gauges
   (`points`, `hcp`, `support_points`) under one knob.
2. ~~`legacy_view` as a first-class arm~~ — **answered 2026-08-16**: built now
   as `DecisionProfile::legacy_view`, and it is byte-exact (see §Phase 1).
   Arm order: control + raw first; the legacy arm runs **only if raw is not
   non-loss**. Read-out: raw non-loss → ship ceilings, view stays opt-in;
   raw loss ∧ legacy non-loss → ship both, Phase 5 retires the view; both
   lose → trace before any conclusion.
3. Phase 3's walk-bookkeeping-from-projection: refactor in place, or a
   parallel `walk_v2` behind the two-binary protocol until it wins?
4. ~~Whether N2a lands now~~ — **answered 2026-08-16**: it **waits**. A book
   node with finite mass shadows the floor, so authoring it would shadow the
   exact N2 signal Phase 1 is being measured on. It is the fallback if a
   `3NT` survives the ceilings.

## Ledger

| date | item | status |
| --- | --- | --- |
| 2026-08-15 | N2 forensic; principle stated; this handoff | written |
| 2026-08-16 | Phase 1 built: `reading.strength_ceilings` + `DecisionProfile::legacy_view`, flags and web knobs | default byte-identical (smoke `18aba5ce…`) |
| 2026-08-16 | Phase 1 **general soundness gate**: 3 new fold tests, E0 extended book-wide, 3 behavioural sweeps generalised to the 4-cell `READING_REGIMES` grid, 2 `legacy_view` tests | **green**, no repairs needed |
| 2026-08-16 | `probe-reading-sound` re-baselined and run 4 paired arms (40k boards, seed 20260816) | partner 2.114→2.105% (`Alerted`), 2.135→2.111% (`All`); their-seat exclusions +0.4/+0.7pp — the A/B's predicted risk |
| 2026-08-16 | whole-book A/B seed 1786827680, 204,800 bd/arm/vul | **4 washes, non-loss** (NV −0.0001/+0.0005, vul −0.0006/−0.0002), fires 0.10% |
| 2026-08-16 | consumer census: nothing but the nets reads a ceiling today | explains the wash; `Strength` has no ceiling accessor, `PartnerShownPoints` reads `.min`, sampler moves sd-lead only |
| 2026-08-16 | **N2's `3NT` traced**: `opener_forced_past_invitation` forces off any 3-level suit bid, sign-off included | Phase 0b opened; ceilings cannot reach this node |
| 2026-08-16 | raw arm pooled over **3 independent seeds** (1786827680 / 1786828611 / 1786829005) | plain DD negative 3/3 at both vuls (CIs straddle zero); the two vuls are the *same deals* re-priced, so n=3 not 6 (sign test p≈0.25) |
| 2026-08-16 | **`legacy_view` arm** (seed 1786829479, 204,800 bd/arm/vul) | **3 boards fired, 4/4 cells positive** (+10/+11/+12/+13 IMPs) — census confirmed, nets are 98.6% of the divergence |
| 2026-08-16 | `legacy_view` runtime cost measured directly (4000 deals, seed 777) | **~1%** (22.68s vs 22.42s), not the 4× the doc comment feared; accountant counters byte-identical to baseline |
| 2026-08-16 | 3 surviving boards are one lane: `1♦ - 3♦ - 3NT` vs `1♦ - 3♦ - 4NT - 5♦ - 6♦` | first measured IMPs from a lost ceiling — invitational jump read as unlimited drives a phantom keycard ask; ceilings win all 3 |
| 2026-08-16 | **SHIPPED both default-on** on jdh8's call — branch 2 by intent, not branch 1 by letter | 6 tests moved, 5 of them pinning the old bug (Landy `8..37`→`8..15`, Woolsey `10..37`→`10..19`, both matching their own configured `convention_points`); cards byte-identical; smoke `18aba5ce…` → `cf583ff5…`; example flags converted to `Option<bool>` opt-outs |
