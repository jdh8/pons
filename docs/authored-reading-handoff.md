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
| `2NT` relay | `points(..=9) & (5+ suit not theirs) & hcp(6..)`, alerted | `hcp 6..37`, `points 6..37`, suits `0..13` |
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

`ReadingProfile::scope` ([knobs.rs](../src/bidding/inference/knobs.rs)) remains
`Alerted` by default. `All` is selected explicitly with
`bba-gen --ns-reading-scope all` or
`probe-reading-sound --ns-reading-scope all`; omission inherits the engine
default. It *intersects* the rule with the walk (does not set the suppression
bit). Phase 2 measured this arm and held it after the whole-book PD gate lost;
see [Phase 2 measurement — held](#phase-2-measurement--held).

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

**Superseded by Phase 3 on 2026-08-17.** This section records the old failure
mode. Informative authored projections now substitute for the walk, while
projection-derived suit and fit state preserves the lane bookkeeping.

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
"byte-identical" claim in this repo cites the old hash. (Phase 0b re-based it
again the same day, to
`f33d8caf785b5f8eda1d5bae0380748675a544b946aa1b02b905a4acfce8e9a4`; `cf583ff5…`
is Phase 1's number, not the current constant.)

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
| `hcp_floor()` / `support_floor()` flipping `None → Some` | **structurally dead**, not merely gated off | their only consumers sit behind `nt_hcp_read: false` and `fit_sum_support_read: false` — *and* inside `points_or_net`/`points_and_net`, whose authored arm is constant-false under the shipped accountant defaults. Flipping the knobs does not revive them; see the resolution below |
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

**Resolved 2026-08-16 — the warning is moot under shipped defaults, and both
knobs are refused.** Censused on paired `bba-gen` arms over identical deals
(640 bd × 32 shards): each fires **0 of 20,480 boards**, because every call
site of both consumers
sits inside `points_or_net`/`points_and_net`, whose authored arm is guarded by
`net_collar() | !accountant_floor()`. Shipped defaults (`accountant_floor:
true`, `net_collar: false`) make that guard constant false — the evaluator net
owns the game and slam milestones and the point arithmetic is masked out. So
**no ceiling this program populates can reach these two gates** while the
accountant is uncollared; the ⚠ above goes live only if the collar ships.

Under `--ns-net-collar`, the one live configuration, `nt_hcp_read` fires 0.16%
and loses on both scorers (−0.0085 plain / −0.0081 PD per board, 32 boards,
CI-clear). All five worst divergences are slams no longer bid (`6NT`→`3NT`,
`7NT`→`6NT`) on hands with a long running suit — the raw-HCP premise holds for
a ruffing value and fails for a source of notrump tricks.
`fit_sum_support_read` fires 0 even collared. Both stay opt-in.

The transferable lesson: the consumer census in the table above asked *which
channels read a ceiling* and found these two "gated off" by their knob
defaults. It should also have asked whether the **surrounding rule arm** is
reachable. A knob default of `false` and a constant-false guard look identical
from the knob's side, and only the second is unfixable by flipping the knob.

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
sign-off** — the one three-level suit call that promises at most nine (the rule
is `points(..=9)`; the *reading* at the sign-off is `6..=8`, both confirmed on
the probe). That is the N2 defect. It is independent of this whole campaign and
it is a smaller change than N2a. It is now Phase 0b below.

⚠ The first draft of this section also claimed it "fixes every sign-off lane at
once". **That is a Phase 2 claim, not a Phase 0b one** — see the reach census in
Phase 0b below.

## Phase 0b — the predicate learns the sign-off (2026-08-16)

`InstinctProfile::forcing_ceiling_read`, **shipped default-on 2026-08-16**. On,
`opener_forced_past_invitation` keeps its auction-shape test and additionally
requires partner's projected `points` ceiling to reach `DIRECT_THREE_LEVEL_POINTS`
= 10 — the points the *direct* three-level suit promises
(`lebensohl.rs` forcing arm, `points(10..)`) and the disturbed game-force floor
in `game_forces` (`hcp(10..)`, whose nine-count sibling is guarded by
`undisturbed()`). **Not** `nt_responder_game_floor`: that is 9, the relay's cap
is 9, and `9 < 9` would never fire.

This is the **first consumer of a strength ceiling in `instinct.rs`** — the
census in Phase 1 found none. It therefore makes `strength_ceilings` a
floor-behaviour knob deliberately, which is the same coupling the ⚠ above
raises for `nt_hcp_read` / `fit_sum_support_read`.

Dropping the force cannot *bar* game: the milestone `Or`'s sibling arm then
prices `points_or_net(combined_hcp(25), …)` on the actual hands.

Probe, shipped defaults, `PROBE_FORCING_CEILING=1`:

```
partner read: hcp 6..8  points 6..8          # the alerted relay's ceiling
  P    9.001   (floor / no rule)             # was 3NT 1.400 on the rail
  3NT  7.792
```

### The reach census — Phase 0b touches exactly one lane

Only **alerted** calls project their rules under the shipped `Alerted` scope, so
the fix bites only where partner's envelope already carries a ceiling below 10.
Measured per node:

| node | ceiling reaching partner's read | fires? |
| --- | --- | --- |
| Lebensohl relay sign-off | `points(..=9)` from the alerted `2NT` relay, monotone through the unalerted sign-off | **yes** |
| `1NT - 2NT - 3♣ - 3♦` (`pass_out`) | own rule's `hcp(..8)` is **unalerted**; the alerted `2NT` is shape-only | no |
| `1NT - 2♠ - 2NT - 3♣` (`pass_out`) | own rule's `hcp(..8)` unalerted; the alerted `2♠` is a disjunction with an unbounded clubs arm | no |
| `1NT - 2♦ - 2♥ - 3♥` (`accept_sixcard_invitation`) | none — the rule has no upper bound in any form ("No upper bound is needed") | no |
| `1NT - 3♦` (`five_five_major_answer`) | alerted, but `points(8..)` is a floor and the `≤16` cap is inside a `described` closure, which projects ⊤ | no |

So the four book nodes documented as floor workarounds are **not retired by this
phase**. Two of them (`pass_out`'s sites) are near misses whose own `hcp(..8)`
would project `points ≤ 9` once Phase 2 drops the alert gate — they are Phase 2
cleanup. `accept_sixcard_invitation` and `five_five_major_answer` would each
need a ceiling *authored* first. And `accept_invitation`
(`1NT - 2♣ - 2x - 2NT -`) was never in scope at all: its node sits over a
level-two notrump call, so the predicate cannot fire there — its docstring was
simply wrong and has been corrected.

**Do not fix this by alerting the two retreats.** Alert is disclosure; alerting a
natural call to buy a reading is the move this whole handoff exists to stop.

### Test

`lebensohl_signoff_is_not_a_game_force` (`tests/american_competition.rs`).
It lives in the *integration* suite by necessity: `instinct/tests.rs`'s
`best_with` builds a bare `Context` with no authored overlay, so partner reads
`0..37` there and no reading-dependent floor predicate can be exercised at all.
Worth knowing before writing the next one.

The test pins the off arm exactly (`3NT`) and the on arm only by `assert_ne!` —
which call replaces the blast is the floor's business and moves with every
retrain — plus a control that the genuinely forcing direct `3♦` is untouched.

### The A/B — three seeds, 12/12 cells positive

`--ns-forcing-ceiling-read` against the same binary, 204,800 boards/arm/vul per
seed (`ab-results/p0b-forcing-ceiling{,-s2,-s3}`, seeds 1786835216 /
1786835669 / 1786836059), firing on 0.01%:

| cell | Σ IMPs (3 seeds) | per board | per-seed signs |
| --- | ---: | ---: | --- |
| NV plain | +62 | +0.00010 | `+ + +` |
| NV PD | +162 | +0.00026 | `+ + +` |
| vul plain | +74 | +0.00012 | `+ + +` |
| vul PD | +172 | +0.00028 | `+ + +` |

Seed 3's CIs clear zero in all four cells. Per fired board: +0.5 to +3.4 plain,
+3.4 to +8.0 PD; 9-10 of the 13 divergent boards win outright and the losses are
the ordinary "sometimes the blast makes anyway" variance. **Plain DD leans
positive on every seed at both vulnerabilities** — the opposite of Phase 1's raw
arm, and the signature of a floor correction rather than an input perturbation.

The read-out was pre-agreed with jdh8 *before* the numbers (a wash would have
shipped it as a correctness fix — the discipline Phase 1's ship decision
lacked). It beat that bar. Smoke re-bases `cf583ff5…` →
`f33d8caf785b5f8eda1d5bae0380748675a544b946aa1b02b905a4acfce8e9a4`; the `.bbsa`
cards are byte-identical, so this is a floor change and not disclosure.

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
| 0b | **`opener_forced_past_invitation` learns the sign-off** | instinct.rs forces to game off *any* partner three-level suit bid over our strong 1NT, Lebensohl sign-offs included; the rail then bypasses the net and pre-satisfies the milestone `Or` | `instinct.forcing_ceiling_read`, default off; `bba-gen --ns-forcing-ceiling-read`, `PROBE_FORCING_CEILING=1`, web knob | standard; **a wash ships it** (pre-agreed with jdh8 before the numbers) | **SHIPPED default-on 2026-08-16.** Probe: `P 9.001` over `3NT 7.792`. Reach censused at **one lane** — the four "workaround" nodes do not qualify, `pass_out` re-files to Phase 2. A/B 3 seeds × 204,800 bd/arm/vul, **12/12 cells positive** (+0.0001 plain / +0.0003 PD both vuls), firing 0.01%. Smoke `cf583ff5…` → `f33d8caf…`; cards byte-identical |
| 0 | **N2a** — opener passes the relay's minor sign-off | book node (`{relay} 3♦ -` → `Pass`, a `landy_signoff_answer` twin) shadows the floor; independent of every reading phase | knobless; measure on `--filter-1nt` | standard | queued — cheapest, fixes 16/18 regardless |
| 1 | **Strength ceilings** | `Points::project` / `Hcp::project` / `SupportPoints::project` → band | `reading.strength_ceilings` **+ `DecisionProfile::legacy_view`, both default-on since 2026-08-16**; pre-ship arm is `bba-gen --ns-strength-ceilings false --ns-legacy-view false` | admits sweep + `probe-reading-sound` unchanged-or-better; A/B | **SHIPPED 2026-08-16.** Soundness gate green (E0 book-wide + 4-cell behavioural grid + probe partner 2.114→2.105%); A/B 3 seeds raw + 1 legacy — raw leans plain-DD-negative, **legacy arm 3 boards/204,800, 4/4 cells positive, ~1% cost**. Shipped on the legacy arm. Cards byte-identical; smoke re-based `18aba5ce…` → `cf583ff5…`. C2's re-open trigger ("a two-sided forward projection") is met by it — **and C2 shipped on it the same day**, 12/12 cells positive once `legacy_view` shields the nets from it too (ledger below) |
| 2 | **`ReadingScope::All`** as default | built; drop the alert gate | `--ns-reading-scope all` (default); `alerted` is the off arm | clear the empty-box worklist before measuring (`probe-reading-sound --ns-reading-scope all`, bucketed); whole-book non-loss under plain DD and PD at both vulnerabilities | **SHIPPED default-on 2026-08-16.** First run held (PD loss, all six cells); its forensic found the whole loss in the four `1x (1NT)` lanes — the side-blind systems-on strip — and with that fixed (nets held by `legacy_view`) 3 seeds × 204,800 bd/arm/vul read **12/12 cells positive**, plain +0.0078…+0.0125 / PD +0.0073…+0.0111. Smoke `edb618b8…` → `bdd1a80e…`; cards byte-identical. The two `pass_out` nodes stay (their deletion is its own A/B) |
| 3 | **Substitute, don't intersect** | authored calls set suppression; walk bookkeeping from the projection; retire `nt_blanket` & co. for authored calls | two-binary (a refactor, not a knob) | byte-identity where projection ⊇ walk is *not* expected — this moves readings by design; A/B | **SHIPPED 2026-08-17.** Partner exclusions 1.877%→1.308%; N2 4/4 cells non-negative; two 204,800-board whole-book seeds are wash/wash at both vuls (pooled NV +0.00023/+0.00052 plain/PD, vul −0.00072/−0.00064). Smoke `bdd1a80e…`→`d532f04b…`.  **Polish A′ 2026-08-17: LOST 12/12 cells** vs `a376c324` (three seeds, both scorers, both vuls) — the bundle was face-suit record + `substitute_authored` net shield + fit-write-back drop + mask refactor. Shield and write-back drop **reverted**. **Polish A′′ 2026-08-17: WASH, shipped on KR2** — face-suit record + `CallMasks` refactor alone vs `a376c324`, 3 seeds × 204,800 bd/arm/vul, 9/12 cells positive, pooled +112 IMPs plain (+0.00009/bd) / +101 PD (+0.00008/bd) on 58 diverging boards, every cell's CI straddling zero (`ab-results/bd-only-s{1,2,3}`, smoke `d532f04b…`→`cb090e54…`, cards byte-identical). The fix over-registers in the opposite direction — its worst board is traced to the walk's rebid arm firing on a floor control bid in competition — so a refinement is queued as its own arm. Paired soundness re-baseline at `ba8f7305`: partner 1.308%→**1.315%** (+12 exclusions), LHO/RHO flat — a record that says more excludes more. See *Why the wash* |
| 4 | Negative inference | fold `project_complement` of higher rules | knob | admits sweep (must stay green — it tightens) | later |
| 5 | **features_v6 + retrain** | honest reading in, `legacy_view` and `net_points` fold out | F2b recipe (dump, held-out gate, twin, flip) | held-out NLL/MAE ≥ shipped; A/B of the flip | after 1-3 land |

Why this order: 1 before 2 because at N2 the missing ceiling, not the missing
length, is what the floor spends; 2 before 3 because 3 is the cleanup that 2
makes possible (once every authored call projects, the walk's exceptions have
nothing left to protect); 5 last because it is the only phase that cannot be
undone with a knob.

## Phase 2 measurement — held, then shipped

*(The hold below stood for an afternoon; the forensic that follows it found
the loss was one lane's reading bug, and `All` shipped default-on the same
day.  Kept as written: the numbers and the trap are the record.)*

The paired binaries were built from control HEAD
`c9a69ad20fd55d9619af1c5bd83ccc87170ffffb`. No treatment commit was created;
the exact executables used are pinned instead: control `bba-gen` SHA-256
`aeefbc79a7a319e47b7775ac070c162925d21aec6975a53d560aa56e7772f06b`,
treatment `bba-gen` SHA-256
`12575355ed9734224aa73df9c064f884aba50cc150feaa37aa189fdab3dda87d`.
All arms used prebuilt binaries sequentially under `scripts/idle-run.sh`.

The paired 40,000-board soundness probe at seed `20260816` passed: the
`Alerted` control excluded the true partner on 3,551/168,006 readings
(2.114%), while repaired `All` excluded 3,548/168,211 (2.109%). Repairs were
limited to seams newly exposed by `All`, including the partial minor-response
table's shared-floor 3NT reading and the systems-on 1NT-overcall strip. The
witnesses read as required: the Lebensohl ♦ sign-off publishes ♦5+, and
`1NT 2♥ 2♠ -` publishes ♠5+ with its strength ceiling.

The N2 diagnostic used seed `1786882843`, 19,200 boards/arm/vulnerability and
`--filter-1nt`. It failed before the whole-book run: NV plain
−0.0191 ±0.0151, PD −0.0461 ±0.0192; vulnerable plain
−0.0317 ±0.0201, PD −0.0538 ±0.0244; firing was about 3.2%. The exact
`(2♠)` table-A lane was byte-identical over its 101 matched boards; the loss
was concentrated in other 1NT contexts, especially 1NT overcalls.

Whole-book results below are IMPs/board ±95% CI; each row is 204,800 paired
boards per arm and vulnerability. `Fired` is divergent boards and rate.

| Seed | Vul | Plain DD | PD | Fired |
| --- | --- | ---: | ---: | ---: |
| 1786883097 | none | +0.0024 ±0.0030 | −0.0041 ±0.0035 | 2,144 (1.05%) |
| 1786883097 | both | +0.0035 ±0.0038 | −0.0030 ±0.0043 | 2,133 (1.04%) |
| 1786883532 | none | −0.0003 ±0.0030 | −0.0053 ±0.0035 | 2,190 (1.07%) |
| 1786883532 | both | −0.0009 ±0.0039 | −0.0066 ±0.0044 | 2,237 (1.09%) |
| 1786883968 | none | +0.0020 ±0.0030 | −0.0024 ±0.0035 | 2,123 (1.04%) |
| 1786883968 | both | +0.0028 ±0.0038 | −0.0018 ±0.0044 | 2,156 (1.05%) |

Pooled over 614,400 boards/vulnerability: NV plain +830 IMPs,
**+0.00135 ±0.00173/board**, +0.129/fired at 6,457/614,400 (1.051%);
NV PD −2,405, **−0.00391 ±0.00202**, −0.372/fired. Vulnerable plain
+1,110, **+0.00181 ±0.00221**, +0.170/fired at 6,526/614,400 (1.062%);
vulnerable PD −2,331, **−0.00379 ±0.00252**, −0.357/fired. The recurring
worst divergences were competitive continuations after our 1NT overcall and
`All`-triggered suit grand slams replacing 6NT. Because PD lost at both
vulnerabilities on every seed, the package failed the stated non-loss gate:
the default and the two retreat-node deletions were rolled back together.
The shipped package therefore preserves the deliberately rebased
`smoke-default --count 20000 --seed 1` SHA-256
`edb618b8cba3aec2a4d434680039a176d4276392059ddd4555cbac511490e804`.
The generated American and Dutch cards and the alert-site fixture remain
byte-identical.

### The forensic — one lane, one bug (2026-08-16)

The iron rule ("trace the worst boards before declaring a loss dead") paid
out in an afternoon.  `ab-dump-bucket --by lane` (new: buckets every
divergent board by the opening and the first call over it, PD-sorted) on the
arms above, no regeneration:

| cell | `1♣ (1NT)` | `1♦ (1NT)` | `1♥ (1NT)` | `1♠ (1NT)` | **four lanes** | **rest of the book** | total PD |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| s1 NV | −650 | −500 | −896 | −459 | **−2,505** | **+1,672** | −833 |
| s2 vul | −687 | −650 | −648 | −636 | **−2,621** | **+1,272** | −1,349 |
| s3 NV | −474 | −547 | −376 | −651 | **−2,048** | **+1,552** | −496 |

**We open a suit, BBA overcalls 1NT** is the whole loss; every other lane —
1NT openings, 2/1, weak twos, contested minors, our own 1NT overcalls — is
net positive under `All` on both scorers.  Not a retune, not a retrain, not
Phase 3: `probe-decision "K6542.Q5.KT8.A96" "- 1♠ 1NT X -"` read partner's
negative double as **`points 15+`** under `Alerted` and **`hcp 15+, every suit
2–5`** under `All`, and RHO's `1NT` overcall as ⊤ under `All`.

Root cause: [`systems_on_overcall_strip`](../src/bidding/inference/read.rs)
matched *shape* only — "a one-suit opening immediately overcalled 1NT" —
never *side*.  When they overcall our opening it stripped **our** opening
and read the rest as an opening-1NT auction *by them*: partner's negative X
became `penalty_x_reading`'s "our penalty double of their 1NT" (15+ under
`Alerted`; under `All` the defensive book's own `(1NT) X` rule projected on
top — 15+ balanced), partner's free bid an overcall of a 1NT opening, and
opener's own suit vanished.  Nothing hand-authored at `P* 1x (1NT)` was ever
read — including the *alerted* negative double.  So the bug also lives on
`main`; `All` merely sharpened a loose wrong box into a tight wrong one and
sent opener pulling the "penalty" double into doubled partscores
(−3/fired across the lane).  The soundness probe's top-20 worklist missed it
because the node is rare per prefix (a few dozen readings in 40k boards)
though ~90% wrong when it fires — a **rate**, not a **count**, worklist would
have caught it; noted under Decisions.

Fix (`read.rs`): the strip fires only when the 1NT overcaller is our side
(`(len − (open+1))` even), and the walk reads *their* direct 1NT overcall of
our one-suit opening off their scheme's opening-1NT box (`apply_opening` with
`their_profile`) — the very box the strip used to deliver by accident, so
responder still sees 15–17 balanced.  Pinned by
`their_one_notrump_overcall_does_not_strip_our_opening` (fails on the old
strip: partner read `hcp 15..`).  Their advancer's calls now read off the
natural walk instead of the stripped 1NT-response structure (probed: their
`2♣` reads ⊤ before and after, their `2♥` still reads as a spade transfer) —
opponents' seat, left for the declared-opponent program.

**Measured** (204,800 boards/arm/vul each; IMPs/board ±95% CI):

| arm | seed | vul | plain DD | PD | fired |
| --- | --- | --- | ---: | ---: | ---: |
| **A-raw**: fix alone vs `main`, nets unshielded | 1786890931 | NV | −0.0007 ±0.0009 | −0.0016 ±0.0012 | 333 (0.16%) |
| | | vul | −0.0014 ±0.0012 | −0.0025 ±0.0015 | ~290 |
| **B-raw**: `All` on the raw fix, vs the raw fix | 1786891656 | NV | **+0.0092 ±0.0024** | **+0.0087 ±0.0025** | 1,014 (0.50%) |
| | | vul | **+0.0131 ±0.0030** | **+0.0118 ±0.0032** | ~1,110 |

Two lessons.  **B**: with the lane fixed, Phase 2 is a clean win on both
scorers, both vulnerabilities — seven times the size of the held run's
plain-DD lean, PD now *agreeing* with plain.  **A**: the fix alone loses
through the **frozen nets** — `probe-decision "AQ63.QT4.Q.KQ763" "- - 1♣ 1NT
- -"`: the truth now reads partner's pass as `0..11` where the strip read ⊤,
and the v5 floor, fit on ⊤ at that node, reopens `2♠` on a four-card suit
(`2♠ 7.79 / P 6.63`; on `main` `P 8.00 / 2♠ 5.99`).  The same mechanism as
Phase 1's raw arm, so the same remedy: `legacy_view` now also serves the nets
the **side-blind strip** (`ReadingProfile::strip_side_blind`, default off; the
view sets it) — the training-time reading, byte-exact again — while the
sampler, gates and instinct read the fixed lane.  With the view on the probe's
truth still reads `0..11` and the floor passes as before.  Retired with the
view at Phase 5.

**Shielded re-run** — fix binary SHA-256 `1cf2b7e270cf1ade…`, sequential
under `idle-run` (`ab-results/p2-rescue-run2.sh`), 204,800 boards/arm/vul:

*A′* — the shielded fix alone vs the pinned `main` control (`aeefbc79…`),
`ab-results/p2-strip-fix-shield-s1`, seed 1786892476: **0 boards fired**
at either vulnerability.  With the nets held on the side-blind strip and no
truth consumer acting on the corrected lane, the fix is byte-identical to
`main` under `Alerted`; `smoke-default --count 20000 --seed 1` stays
`edb618b8…`.  Seeds 2–3 skipped as redundant.  It ships to `main` on the
smoke proof.

*B′* — `All` on the shielded fix (`ab-results/p2-reading-all-v2-shield-s{1,2,3}`):

| seed | vul | plain DD | PD | fired |
| --- | --- | ---: | ---: | ---: |
| 1786893145 | NV | +0.0078 ±0.0024 | +0.0073 ±0.0025 | 1,069 (0.52%) |
| 1786893145 | both | +0.0102 ±0.0031 | +0.0088 ±0.0032 | 1,153 (0.56%) |
| 1786893844 | NV | +0.0087 ±0.0025 | +0.0082 ±0.0026 | 1,103 (0.54%) |
| 1786893844 | both | +0.0125 ±0.0032 | +0.0111 ±0.0033 | 1,190 (0.58%) |
| 1786894518 | NV | +0.0085 ±0.0024 | +0.0081 ±0.0026 | 1,029 (0.50%) |
| 1786894518 | both | +0.0123 ±0.0031 | +0.0107 ±0.0033 | 1,136 (0.55%) |

**12/12 cells positive, every CI clear of zero, PD tracking plain** —
+1.4…+2.2 IMPs per fired board.  `ReadingScope::All` is the default;
`Alerted` remains selectable (`--ns-reading-scope alerted`,
`PROBE_SCOPE=alerted`) and is what `legacy_view` serves the nets.  Smoke
re-based `edb618b8…` → `bdd1a80ebe7d90ee6dc26ed8915b8d0ee5017d2e24e004fd88534871c72ac507`
(a reading flip moves calls by design); the generated cards are byte-identical
(`the_checked_in_cards_match_the_generator`).  Three tests that pinned the
alert-gated regime moved with it: the queen-relay integration tests now agree
spades with a *single* raise (the limit raise's own `support(4..)` makes a
nine-card fit and `queen_moot` fires — correct, so the relay is exercised
where the queen is a live question), the optional-double contrast asserts "not
the promised five" instead of "nothing", and `probed_vacuous` pins `Alerted`
explicitly (the hole it fills is closed under `All`).

The two `pass_out` retreat nodes remain; deleting them is a separate A/B.

## Phase 3 measurement — shipped (2026-08-17)

Phase 3 was refactored in place: an informative authored projection suppresses
the natural semantic walk, and per-call projected 3+/4+/5+/6+ suit masks retain
chronological fit, shown-suit, rebid, cue and raise bookkeeping. An unalerted
authored rule whose projection is top falls back to the walk; an alerted top
stays suppressed as artificial. This also retires the notrump blanket and its
structural exceptions for authored calls; they remain only for unauthored calls.
No second walker or runtime knob was added.

Control was HEAD `bccb6e3d746d2a5f5ab4fbf19298a76576e5a8cf`. The pinned
`bba-gen` SHA-256 hashes were `084c484fa23229cb298e3e315ae62e76f876b5db351644f8951c73f215cfeb95`
(control) and `e7d729b5d35b219aa6ff53c9c5863ee13f59cbbdd487891af07ffe26bf788886`
(treatment). Every arm used the prebuilt binaries sequentially under
`scripts/idle-run.sh`.

The 40,000-board soundness probe at seed `20260816` passed: partner exclusions
fell from 3,156/168,138 (**1.877%**) to 2,199/168,085 (**1.308%**). LHO/RHO
exclusions also fell, 8.059%→7.748% and 8.111%→7.834%. The N2 diagnostic at
seed `1786905331`, 19,200 boards/arm/vulnerability, fired 31–32 boards and was
non-negative in all four cells: NV +0.0008 plain / +0.0014 PD, vulnerable
+0.0044 / +0.0056, with every CI spanning zero. The exact Lebensohl witness
remained Pass over `3NT`, with partner read as 6–8 and five-plus diamonds.

The whole-book A/B used seeds `1786905418` and `1786906127`, each 204,800
boards/arm/vulnerability:

| Seed | Vul | Plain DD | PD | Fired |
| --- | --- | ---: | ---: | ---: |
| 1786905418 | none | −0.0008 ±0.0023 | −0.0005 ±0.0026 | 1,383 (0.68%) |
| 1786905418 | both | −0.0013 ±0.0027 | −0.0009 ±0.0030 | 1,160 (0.57%) |
| 1786906127 | none | +0.0013 ±0.0022 | +0.0016 ±0.0025 | 1,295 (0.63%) |
| 1786906127 | both | −0.0002 ±0.0027 | −0.0004 ±0.0030 | 1,173 (0.57%) |

Pooled over 409,600 boards/vulnerability: NV +0.00023 plain / +0.00052 PD;
vulnerable −0.00072 / −0.00064. This is wash/wash at both vulnerabilities and
passes the pre-registered non-loss bar for a soundness correction. The shipped
default smoke changes 248/20,000 auctions and re-bases
`bdd1a80ebe7d90ee6dc26ed8915b8d0ee5017d2e24e004fd88534871c72ac507` to
`d532f04b5a5d0ba11b15c8a777f4e4fbab466ec59f361618fcd495f844253915`.

### Why the wash — the polish pass (2026-08-17)

A soundness correction of that size measuring as a wash is the Phase-1-raw and
Phase-2-A-raw signature, and the review of the shipped commit proposed two
causes. **Only the first survived measurement.** The second — that the frozen
nets needed shielding from substitution — was refuted by its own A/B and is
recorded below as a dead end, kept because the reasoning looked identical to
Phases 1 and 2 and will look tempting again.

**1. The lane lost the suit.** The substituted branch recorded a call's suits
from its *projection floors* only — `four_plus`, plus a fit. A `1♦` opening
whose rule union admits a three-card diamond projects no floor at all, so
diamonds never entered `lane_suits`, the lane's mechanical bid-history. The
walk's `i_bid_it` keys on exactly that set, so opener's own `3♦` rebid read as
a **first showing** (♦4+) instead of a rebid (♦6+). The measured cost is on
disk in the commit itself: `competitive_rebid_reaches_the_missed_game` fell
`5♦` → `4♦` on *both* its arms — the cold game the fixture's own comment calls
11 tricks double-dummy — and was re-pinned rather than traced.

The fix keeps Phase 3's rule intact — the projection owns *meaning* — while
restoring the one thing the projection cannot state: which suit the call
**named**. A substituted natural bid now writes its face suit into
`lane_suits` (never `natural_lane_suits`, which stays the projection's). The
artificial calls are excluded by a new `CallMasks::artificial` bit, since a
transfer's face suit is precisely the phantom holding suppression exists to
kill. Both pins are back at `5♦`.

This also reframes the probe: a **weaker** reading trivially excludes fewer
hands, so 1.877%→1.308% measured the correction *and* the lost inference
together. Re-run it paired after any change to what substitution records.

**The bug was wider than the rebid ladder — it lost the trump suit.** The
`5♦`→`4♦` fixture is the cheap symptom; the expensive one showed up only when
the fix was measured alone. `lane_suits` also feeds `i_bid_it` /
`partner_bid_it`, which is how the engine knows *which suit is agreed*, so a
dropped face suit misidentified the trump suit after a keycard ask and put
slams in a phantom strain. Both directions appear in the diverging boards:

```
1♦ (2♠) X (3♠) 4♦ - 4NT - 5♠ -     fix 6♦   control 6♥   +14   ♥98 opposite a void
1♦ - 1♥ (3♠) 4♦ - 4NT - 5♠ -       fix 6♦   control 6♥X  +19   same shape
- 1♦ (1♠) 2♥ - 3♦ - 4NT - 5♣ -     fix 6♦X  control 5♦   −11   fix emboldened into a bad slam
```

So the repair is **not** one-directional: fixing trump identity moves slam
decisions both ways, and the two-sided tail is the honest picture. Because the
trigger needs a substituted natural bid whose projection floor is short, it
fires on ~0.005% of boards (6–13 per 204,800-board cell) at 2–4 IMPs each — an
inert fleet score carrying a real correctness win, and far too few boards for
the per-fired mean to be worth reading. Judge this class of fix by inspecting
its boards, not by its IMPs/board.

**A′′ verdict (`ba8f7305` vs `a376c324`, 3 seeds × 204,800 bd/arm/vul):**

| cell | plain | PD | fired |
| --- | ---: | ---: | ---: |
| s1 NV  | +34 (+0.0002) | +35 (+0.0002) | 9 |
| s1 vul | +36 (+0.0002) | +41 (+0.0002) | 10 |
| s2 NV  | +2 (+0.0000) | −4 (−0.0000) | 6 |
| s2 vul | −24 (−0.0001) | −40 (−0.0002) | 8 |
| s3 NV  | +24 (+0.0001) | +29 (+0.0001) | 12 |
| s3 vul | +40 (+0.0002) | +40 (+0.0002) | 13 |

**9/12 cells positive**, the three negatives totalling −68 IMPs and none
reaching 0.0002 IMPs/board against per-cell CIs of ±0.0002…±0.0004 — every
cell straddles zero. (Read raw IMPs here, not the per-board column: at 0.00%
firing the fourth decimal hides the sign, which is how the count first went
down as "10/12 non-negative".) Pooled over 1,228,800 boards/scorer, +112 IMPs
plain (+0.00009/bd) and +101 PD (+0.00008/bd) on 58 diverging boards. So this
is a **wash** — non-loss on both scorers at both vulnerabilities, shipped on
KR2 under the objective's rule.

**The fix has its own failure mode: it over-registers.** Recording the named
face suit unconditionally makes `i_bid_it` true *more* often, and a merely
named suit can then outrank a genuine fit. The worst board in the run is this,
not the bug it repairs:

```
- 1♣ (1♠) 2♥ - 3♥ - 3♠ - 4♣ - 4NT - 5♠ -      fix 6♣X   control 6♥   −18
      ♠Q842 ♥KQ6 ♦93 ♣AJ73   opposite   ♠A9 ♥A9874 ♦AJ8 ♣K84
```

♥KQ6 opposite ♥A9874 is an 8-card fit holding AKQ; the fix parks it in a
7-card club fit and is doubled. The two arms of the trade — suits wrongly
forgotten (the `6♦` boards, +14/+19/+21) against suits wrongly promoted (this
one) — very nearly cancel, which is exactly why the pooled number is zero
rather than the win seed 1 alone suggested.

**Traced 2026-08-17, and the guessed lever was wrong.** `probe-decision` on S
(dealer W, pons N-S, both vul) reads partner as `♣ 6..13` at the `4NT` and
answers `6♣` at the six-level node, so the chain is:

1. N's `4♣` is **unauthored floor** — a control bid with hearts agreed.
2. ♣ is in `lane_suits[N]` from the substituted `1♣` opening's face suit, so
   `4♣` hits the walk's own-suit **rebid** arm ([read.rs:770-820]) and takes
   its floor of 6 ([read.rs:815]) — S now reads partner for a six-card club
   suit that partner never showed.
3. [`keycard_trump`] maximises `our length + partner's shown floor` over all
   four suits, so ♣ (3+6=9) outranks ♥ (5+3=8) and the ask keys on clubs.

The same board carries a **second, pre-existing phantom**: after N's `5♠`
(the 1430 answer) S's read of partner's spades moves `0..4` → `3..4`, because
S's own `3♠` cue was written to `lane_suits[S]` by the unconditional
bid-history write at [read.rs:1154-1155], making `5♠` a *raise* of a suit
only the opponents hold. Not Phase 3's doing and not on this board's critical
path — recorded here, owed its own arm.

Trump selection *already* prefers length — that lever is a no-op. The real
one is step 2: `classify_high_bid`, the only control-bid detector, is gated
off in every contested auction ([read.rs:694], `!side_acted[defending_parity]`),
so in competition a slam-zone bid of a named suit can only read as a rebid.

**Queued refinement, unmeasured:** teach the walk that a 4–5 level bid of a
suit is a *control bid, not a rebid*, when another suit is already agreed in
that lane pair — the same guard on the `partner_bid_it` raise arm, since a
control bid in partner's suit is not a raise either. Loosens only, so it is
soundness-safe by construction. Its own arm, never a rider. (The alternative,
gating the face-suit record on the projection not contradicting it, tightens
and would have to clear the soundness probe first.)

[read.rs:770-820]: ../src/bidding/inference/read.rs#L770-L820
[read.rs:815]: ../src/bidding/inference/read.rs#L815
[read.rs:694]: ../src/bidding/inference/read.rs#L694
[read.rs:1154-1155]: ../src/bidding/inference/read.rs#L1154-L1155
[`keycard_trump`]: ../src/bidding/instinct.rs#L1642-L1658

**2. REFUTED — "substitution had no knob, so the nets saw it."**
[`legacy_view`][net-shield] reconstructs the training-time reading by flipping
*reading knobs* on a clone — `scope`, `strength_ceilings`, `upgrade_closure`,
`strip_side_blind`. Substitution was unconditional, so the clone substituted
too and stopped reproducing the hull the v5 nets were fit on: a substituted
call skips `apply_opening` (the strong `2♣`), the alerted takeout-double floor
and the whole strength tail (`nt_invite`, opener's notrump rebid bands, the
responder raise bands, the extras ladder, Stayman). The inference — shield the
nets and Phase 3's wash becomes a win, exactly as the Phase 2 strip fix went
from −0.0007/−0.0016 raw to +0.0078…+0.0125 shielded — was **wrong**.

A `substitute_authored` knob folded off in `net_inferences`, bundled with the
face-suit fix and the write-back drop, measured **12/12 cells negative** over
three 204,800-board seeds against `a376c324` (NV −0.0035/−0.0034,
−0.0007/−0.0006, −0.0017/−0.0016; vul −0.0024/−0.0015, −0.0025/−0.0025,
−0.0006/−0.0008 plain/PD). Divergence was narrow and expensive — 0.46–0.58% of
boards fired, at −0.13…−0.30 IMPs each — i.e. a specific mechanism misfiring,
not broad drift. The knob and the write-back drop were reverted; the face-suit
fix went back out alone.

The knob was also wrong by house rule before it was wrong by measurement: it
existed *only* to neutralise Phase 3 for the nets, which CLAUDE.md calls
scaffolding rather than a knob, and `legacy_view` is itself scheduled for
demolition at Phase 5. The standing hypothesis for the loss — untested, and
not worth runs on reverted code — is that a net and a sampler reading
*different* auctions costs more than both reading the same off-distribution
one. If shielding is ever revisited, that is the thing to test first, and it
should be measured **alone**, not bundled.

Two review findings did **not** survive the code, and a third did not survive
measurement. The `!authored_call` gate on the notrump blanket is sound and *is*
tested — an authored call reaching it has a live rule and no alert, so
`artificial_calls_are_alerted` makes it natural, and
`top_authored_projection_falls_back_to_the_walk` pins it; kept, ungated as it
shipped. Dropping the fit write-back into partner's lane was argued from
analysis alone — `fit` can be sourced from a partner's **3+** projection, and
promoting that to "partner naturally showed it" makes their later unauthored
rebid claim six (`i_bid_it`) and an opponent's bid of the suit read as a cue —
but it went into the losing bundle unmeasured and was reverted with it. It
remains a live, *unmeasured* soundness question: re-propose it as its own arm,
never as a rider.

[net-shield]: ../src/bidding/context.rs

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
3. ~~Phase 3's walk-bookkeeping-from-projection~~ — **answered 2026-08-17**:
   refactored in place; the two-binary measurement met the non-loss gate, so a
   parallel `walk_v2` and permanent knob would be dead scaffolding.
4. ~~Whether N2a lands now~~ — **answered 2026-08-16**: it **waits**. A book
   node with finite mass shadows the floor, so authoring it would shadow the
   exact N2 signal Phase 1 is being measured on. It is the fallback if a
   `3NT` survives the ceilings.

5. ~~`probe-reading-sound`'s partner worklist ranks nodes by excluded
   **count**~~ — **built 2026-08-17**: a second table ranked by **rate**,
   count floor ≥ 10 readings, same `--top`, no new flag.  It paid for itself
   on the first run (three 100%-wrong nodes the count table buried; see the
   ledger).

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
| 2026-08-16 | **Phase 0b built**: `instinct.forcing_ceiling_read`, threshold 10, first ceiling read in `instinct.rs` | default byte-identical (smoke `cf583ff5…`); probe moves the node to `P` |
| 2026-08-16 | **Phase 0b SHIPPED default-on**, 3 seeds × 204,800 bd/arm/vul | **12/12 cells positive**, +0.0001 plain / +0.0003 PD both vuls, fires 0.01% at +2 to +6 IMPs each; plain DD positive on every seed — a floor correction, not a net perturbation. Smoke → `f33d8caf…`, cards byte-identical |
| 2026-08-16 | Phase 0b **reach census**: only alerted ceilings project, so the fix touches **one lane** | the four floor-workaround nodes do not qualify — `pass_out`×2 are Phase 2 near-misses, the other two need a ceiling authored, `accept_invitation` was never in scope (its node sits over a level-2 notrump call; docstring corrected) |
| 2026-08-16 | **C2 (`reading.upgrade_closure`) re-measured on the trigger Phase 1 met** — `scripts/ab-upgrade-closure.sh`, 3 seeds × 204,800 bd/arm/vul | **SHIPPED default-on**: 12/12 cells positive (+0.00015 plain / +0.00024 PD NV, +0.00016 / +0.00022 vul). `legacy_view`'s clone now folds C2 off too — unshielded it measures **−0.0037/bd on 18 of 20,480 deals** (talks the evaluator out of slams), shielded **+0.0006 on 2**. Smoke `f33d8caf…` → `edb618b8…`; exclusion 2.114% → 2.114% |
| 2026-08-16 | **SHIPPED both default-on** on jdh8's call — branch 2 by intent, not branch 1 by letter | 6 tests moved, 5 of them pinning the old bug (Landy `8..37`→`8..15`, Woolsey `10..37`→`10..19`, both matching their own configured `convention_points`); cards byte-identical; smoke `18aba5ce…` → `cf583ff5…`; example flags converted to `Option<bool>` opt-outs |
| 2026-08-16 | **Phase 2 `ReadingScope::All` measured and HELD** | soundness 2.114%→2.109%; N2 diagnostic loses all four cells; three 204,800-board whole-book seeds lean plain-positive but PD loses all six seed/vulnerability cells (pooled −0.00391/−0.00379 NV/vul). Default stays `Alerted`, both retreat nodes stay, cards/alerts byte-identical, smoke remains `edb618b8…` |
| 2026-08-16 | **Phase 2 forensic**: `ab-dump-bucket --by lane` on the held arms — the loss is the four `1x (1NT)` lanes (−2.0k…−2.6k PD/cell), the rest of the book +1.3k…+1.7k | root cause `systems_on_overcall_strip` firing on *their* 1NT overcall (side-blind); fixed ours-only + explicit their-1NT-overcall walk box; two A/Bs in flight (A: fix alone vs `main`; B: `All` on the fix) — see *The forensic — one lane, one bug* |
| 2026-08-16 | **Phase 2 `ReadingScope::All` SHIPPED default-on** — strip fixed ours-only, nets held (`strip_side_blind` under `legacy_view`), 3 seeds × 204,800 bd/arm/vul | **12/12 cells positive** (plain +0.0078…+0.0125, PD +0.0073…+0.0111); the fix alone is byte-identical to `main` (0 fired, smoke `edb618b8…` unchanged); flip re-bases smoke to `bdd1a80e…`; cards byte-identical |
| 2026-08-17 | **Phase 3 substitute-don't-intersect SHIPPED** — in-place walker, projection-derived lane bookkeeping, top fallback | Soundness partner exclusions 1.877%→1.308%; N2 4/4 non-negative; two 204,800-board whole-book seeds wash/wash at both vuls (pooled NV +0.00023/+0.00052 plain/PD, vul −0.00072/−0.00064); smoke `bdd1a80e…`→`d532f04b…` |
| 2026-08-17 | **Phase 3 polish A′ — MEASURED LOSS, bundle split** | The review's four changes went out as one arm vs `a376c324` and lost **12/12 cells** (3 seeds × 204,800 bd/arm/vul; NV −0.0035/−0.0034, −0.0007/−0.0006, −0.0017/−0.0016; vul −0.0024/−0.0015, −0.0025/−0.0025, −0.0006/−0.0008 plain/PD), on 0.46–0.58% of boards at −0.13…−0.30 IMPs/fired. Bundling four changes in one arm was the process error. **Reverted:** `reading.substitute_authored` (net shield — refuted, and scaffolding by house rule) and the fit-write-back drop (unmeasured rider; still an open soundness question, owed its own arm). **Kept, ungated as shipped:** the `!authored_call` blanket escape (sound, and pinned by `top_authored_projection_falls_back_to_the_walk`). **Re-measured alone:** the face-suit `lane_suits` record + the `CallMasks` refactor — `ab-results/bd-only-s{1,2,3}`, smoke `d532f04b…`→`cb090e54…`, cards byte-identical |
| 2026-08-17 | **Phase 3 polish A′′ — the split arm measured**, `ba8f7305` vs `a376c324`, 3 seeds × 204,800 bd/arm/vul, both scorers, both vuls | **WASH, shipped on KR2.** 9/12 cells positive (+34/+35, +36/+41, +2/−4, −24/−40, +24/+29, +40/+40 plain/PD), pooled +112 plain (+0.00009/bd) / +101 PD (+0.00008/bd) on 58 diverging boards; every cell's CI (±0.0002…±0.0004) straddles zero. Splitting the A′ bundle was the whole lesson: alone the change is a non-loss, bundled it was 12/12 down |
| 2026-08-17 | **A′′ worst board (−18) traced** with `probe-decision` — the guessed lever was wrong | Not trump selection (`keycard_trump` already maximises *length* + partner's floor). N's `4♣` is unauthored floor; the substituted `1♣`'s face suit puts ♣ in `lane_suits[N]`, so the walk's own-suit **rebid** arm claims ♣6+ and the ask keys on a 7-card club fit over an 8-card heart fit holding AKQ. Root cause: `classify_high_bid` is gated off in every contested auction (`!side_acted[defending_parity]`), so in competition a slam-zone bid can only read as a rebid. Refinement queued as its own arm: 4–5 level bid + another suit already agreed in the lane pair ⇒ control bid, no length, on both the rebid and raise arms |
| 2026-08-17 | **Second phantom on the same board, pre-existing**: N's `5♠` 1430 answer reads as a spade **raise** (partner ♠ `0..4`→`3..4`) | S's own `3♠` cue is written to `lane_suits[S]` by the unconditional bid-history write (`read.rs:1154-1155`), so the answer looks like a raise of a suit only the opponents hold. Off Phase 3's critical path; recorded, owed its own arm |
| 2026-08-17 | **Rate-ranked partner worklist built** (`probe-reading-sound`, second table, `bad/readings` desc, floor ≥ 10 readings, same `--top`, no new flag) | Print-only, reversible. Ranking by count alone is what hid the side-blind strip bug; the floor keeps single-digit noise out of the top slots |
| 2026-08-17 | **Phase 3 soundness re-baseline** at `ba8f7305` (40,000 boards, seed 20260816, same flags as the recorded pair) | partner **2,211/168,086 = 1.315%** (was 2,199/168,085 = **1.308%** at `a376c324`), LHO 7.749% (7.748%), RHO 7.834% (7.834%). The face-suit record costs **+12 partner exclusions, +0.007pp** — the expected direction: a record that says *more* excludes *more*, which is the doc's own caveat that a falling exclusion rate never on its own proves a reading got better |
| 2026-08-17 | **Rate table's first catch, filed not fixed**: three ~always-wrong partner nodes, all invisible in the count table | `1♥ - 2NT - 4♥` **25/25 = 100%** and `1♠ - 2NT - 4♠` **17/17 = 100%** — the Jacoby minimum 4M is authored as the pure catch-all `rule(4M, 50, hcp(0..))` (`raises/jacoby.rs:74`), so it projects nothing, falls back to the walk, and the walk's jump ladder reads opener at **`points 16..21`** — the exact inverse of "minimum". `2♥/2♠/2♦ - 2NT - 3♣` **95%/77%/86%** — the Ogust `3♣` answer is unread, so it reads as a natural **♣4..13** phantom suit (corroborates the queued Ogust reader fix). Also `2♦ 2♥ - 3♦ - 3♥` 10/10. Each is a book/reading change owing its own A/B |

### Memory compaction notes (2026-08-16)

- The refused ceiling consumers shipped as opt-in surfaces in `f6a6657b`; the
  canonical refusal write-up was recorded in `7451c4f0`. The one-off census
  script was deliberately not retained.
- Older notes calling `strength_ceilings` and `legacy_view` "default-off
  pending a ship call" are stale: both shipped default-on 2026-08-16.
