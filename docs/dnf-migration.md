# DNF migration ledger

Campaign: every authored bidding constraint yields a tight union-of-boxes
(`Dnf` of `Envelope`) reading instead of hulling to ⊤ at `Or` — then flip
`set_dnf_reading` on after measurement. Why readings collapse today and how to
read a call off the bidder instead:
[ai-bidder/sampled-projection.md](ai-bidder/sampled-projection.md). Layer
invariants: [bidding-architecture.md](bidding-architecture.md).

This file is the **ledger only**: what landed, under which knob, with which
verdict. Design lives in the docs above and the code.

## The migration rules

- New projection precision that changes a **knob-off hull** sits behind
  `dnf_reading()` (precedent: `AnyLen::project`). Multi-box values are born
  via `Dnf::disjoin` (knob-gated), never raw `union` — `Dnf::intersect` does
  not consult the knob and would prune knob-off.
- **The wrong-seat trap** (found by the C2 dump diff): a legacy composite
  whose legs read the auction (`support`, `!support`) replays its projection
  under the *reader's* context, so its knob-off reading is
  **context-sensitive** — ⊤ at one point of the auction (the leg contradicts
  its own `len` and the empty product widens out; this, not the `Or` hull
  alone, is the mechanism of the measured "every 2/1 reads 0..=37" bug) and a
  wrong-suit box at another. No static box list reproduces that, so a
  re-authoring must keep the legacy constraint for eval/describe/knob-off
  reading and swap in the boxes knob-on only — the `dnf_upgrade` adapter
  (constraint.rs). The per-rule ratchet cannot catch reader-context effects
  (it projects under the authoring-time context); only the `bba-gen` dump
  diff can. **Run the dump diff for every re-authored gate.**

## Knob matrix

| Knob | Default | Gates |
| --- | --- | --- |
| `set_dnf_reading` | off | `Dnf::disjoin` (build), `Inferences::admits` (accept), `Dnf::tidy` (hygiene), every ⊤→boxes projection upgrade (`shapes`, `Support::project`, `Balanced::project`, De Morgan complements, `dnf_upgrade`, `top_honors`, `Points`→hcp coupling) |
| `set_gauge_membership` | off | `Envelope::admits` (hence `Dnf::contains`, the overlay, and every sampler) also tests the `hcp` + `support_points` bands — the one knob that can *reject legal hands* if a projection over-claims; E0's eval⟹membership sweep is its soundness gate |

Both knobs have [bidding-options.md](bidding-options.md) Engine rows and
`bba-gen` arm flags (`--ns-dnf`, `--ns-gauge-membership`).

## Chops

Byte-identical chops keep the default system byte-identical (dual-knob
ratchet + `verify::compare`/exhaustive-shape tests + `bba-gen` dump diff).
MEASURED chops follow [measurement.md](measurement.md) and record their
verdict here.

The A→E0 wave landed as **one commit** (this session, 2026-07-23): the chops
were developed against shared files and verified byte-identical as a whole —
dump diff 4000 boards × both vuls + a second 4000-board seed, zero divergent
boards.

| # | What | Status |
| --- | --- | --- |
| pre | `Dnf` machinery + `set_dnf_reading` | cff8919 |
| pre | `Strength` gauges (hcp + support_points axes) | dd90e2c |
| pre | Stage A `Flip` complements | 1e11c49 |
| pre | `authored_calls_read_what_they_gate` leak ratchet | 095ac85 |
| A | this ledger | landed |
| B | ratchet upgrade: shared trie walk, `dutch()`, per-gauge noun columns, dual-knob exact pins, per-box leak predicate | landed |
| C | `impl Constraint for Envelope + Dnf`: strict `Envelope::accepts` eval (all gauges, ceilings), identity projection, noun-compatible `describe` | landed |
| C2 | pilot: 2/1 fit-split boxes via `dnf_upgrade` (legacy eval/describe/knob-off reading kept; exact two-box reading knob-on) | landed |
| D1 | bounded-band complements → 2-box DNF via `disjoin`; De Morgan on `And`/`Or` + `Flip` double-negation `project_complement` (knob-gated); `SupportPoints::project_complement` (new, knob-gated) | landed |
| D1b | exact shape DNFs via `shapes()`: `balanced` (cube {2..=4}⁴ + four 5(332)), `semi_balanced` ({2..=5}⁴ ∪ four {s 6..=7, rest 2..=3}), `notrump_shape` per variant ({M 2..=4, m 2..=cap} ∪ two 5M(332)); each pinned exhaustively over the 560-shape lattice | landed |
| D1c | knob-on box hygiene (`Dnf::tidy`): sum-feasibility prune + containment dedup | landed |
| D2 | `Support::project` forward-boxes partner's suit, knob-gated | landed |
| E0 | book-wide eval ⟹ strict-membership sweep over american()+dutch(), knob-on, forward + band (inference.rs, beside the ratchet) | landed |
| G | the conversion tail, knob-on meter → **0 in every column**: comparative staircases (`longer_suit`/`at_least_as_long`/`equal_length`, exact `∪ₖ {b ≤ k, a ≥ k+gap}` — 15 `described` closures replaced, labels kept, eval pinned exhaustively); transfer reroutes + `splinter_short` via `dnf_upgrade` (boxes sound in both treatment states, authoring-time `union` never `disjoin`); `top_honors` floors its suit length + raw HCP (2/5/9 for 1/2/3 honors); `Points` → `hcp` gauge coupling (a points floor implies an HCP floor slacked by `hcp_ceiling_slack` — without it, tidy's *correct* containment dedup swallows the `hcp` arm of `points(22..) \| hcp(22..)` and loses the knowledge); `Balanced::project_complement` (unbalanced = 4 short + 4 long + 12 two-suiter boxes, exact); sniffer stops counting context claims ("partner's last suit is ♠") and no-op caps ("≤13 ♠") | landed |
| E | **MEASURED** `set_gauge_membership`: gauges get membership teeth. Knob + harness landed (default off, byte-identical); `ab-dnf-sd-lead` runs the **in-process knob matrix** off/dnf/gauge/both on one bid-out (tighter than the planned per-arm `--arm` flag — every arm prices the identical lead question). **Verdict: WASH every cell** (20000 boards × both vuls, seed 1784779888, 16 worlds): gauge vs off −0.0034 ±0.0262 NV / −0.0042 ±0.0309 vul; on top of dnf +0.0140 ±0.0261 / +0.0002 ±0.0317. The expected outcome — no regression, teeth are free; the knob stays **default-off** as the independent kill-switch, its on/off ride-along decided with F. *Side reading, F caution:* the post-G `dnf` arm's NV cell is a significant sd-lead **loss** (−0.0318 ±0.0267) where the pre-G dnf arm measured a wash; vul-both +0.0121 ±0.0315, pooled a wash. If F's match loses, first suspect a G-era box family over-tightening the leader's sampled worlds | landed |
| F | **MEASURED** flip `set_dnf_reading` default-on. Harness: `--ns-dnf` + `--ns-gauge-membership` arm flags in `bba-gen`, `scripts/dnf-flip-ab.sh` (off/dnf/both arms). **Verdict: flip REFUTED as-is — LOSS in all four cells** (204800 bd/arm/vul, SEED_BASE 1784784503, sha bb32624): dnf-vs-off plain −0.0038 ±0.0030 NV / −0.0054 ±0.0036 vul, PD −0.0039 ±0.0036 / −0.0053 ±0.0043; fired 1.02%/0.91%, −0.37/−0.60 per fired. Worst-tail fingerprint = **under-reach** (ON stops in 5♣ / passes out 5♥X where OFF bids the making slam) — consistent with the plan's predicted feature shift: the frozen BBA-distilled net is calibrated to *hulled* features, so truthfully tighter hulls read as "partner is weaker" (hypothesis, not traced board-by-board). `both`-vs-`dnf`: **0 fired in 409600** — gauge membership is bidding-inert, confirmed at scale. Knob stays **default-off opt-in**; the flip's remaining path was thought to be regen+retrain with knob-on features — **refuted by F1** (features are knob-invariant; see the F1 row). Ratchet re-pin and `--no-ns-dnf` rename are flip-day tasks and stay parked | landed (flip refused) |
| F1 | **FORENSIC — F's loss traced to the bilans evaluator reading knob-shifted ranges; the retrain that matters is the *evaluator*, not the policy floor net.** Full-list re-diff of the surviving F artifacts (`ab-dump-diff --show 3000`, verdicts reproduced exactly) splits the delta: **contested first-divergence is net positive** (+177 NV / +162 vul over ~1750/1500 boards — the sampler pinning works as designed) and the **entire loss is constructive** (−957 NV / −1276 vul on ~350 boards), dominated by slam/level conversions deep in 2/1 auctions (pass→6NT over 3NT −250/−312, 6NT→7NT −140/−189, pass→4♣/4♦ raises, and under-raises like 4♠→pass vul — wrong in *both* directions). Board-level trace (probe-classify `--dnf`, board `KQ964.Q2.KQJ8.A4` at `1♠-2♣-2♦-3NT`): the 6NT blast fires knob-on from instinct's NT-slam milestone with `combined_hcp(33)` **false** (18+0) — the rule enters through `points_or_net`'s **net arm**, because `set_bilans_floor` ships **default-on** (ca18fb4). Mechanism: **two hull regimes.** Bare-context hulls (`Context::new`, the `dump-teacher` feature path) are knob-invariant — measured byte-identical over a 21K-row dump (temporary `--dnf` toggle, removed); but **prefixed-context** hulls (`Stance::infer`, what the bidder actually sees) DO tighten knob-on via the authored-projection overlay (the traced board: partner gains ♠≤3, sp≥13 from the C2 fit-split boxes). `trick_estimates` feeds those prefixed ranges to the **evaluator net, which was fit on knob-off prefixed readings** (evaluator.rs's own distribution warning) — knob-on inputs are out-of-distribution and the bilans game/slam gates misfire both ways. Consequences: (1) retraining the **policy** floor net from `dump-teacher` as-is is a no-op (bare-context corpus is knob-invariant); (2) the flip's real coupling is `set_dnf_reading` × `set_bilans_floor` — candidate next chops: **F2a** compat shim (compute `trick_estimates`' ranges under knob-off reading while the sampler keeps the boxes; isolates the contested gain from the evaluator OOD loss) and **F2b** regen+retrain `evaluator_v2` on knob-on prefixed features, then re-flip; (3) **side finding, knob-independent:** the policy net *serves* on prefixed features (`NeuralFloorBba::classify` gets the trie context) but was *trained* on bare-context features — a standing train/serve skew worth its own experiment | landed (docs + probe tooling) |
| F1 | **REFUTED PREMISE — retrain is a no-op.** The F verdict's "regen+retrain with knob-on features" path assumed the flip shifts the net's inputs. Probed 2026-07-23: a `--dnf` build of `dump-teacher` (`--teacher bba --boards 2000 --seed 1`, 21,456 rows) produced a **byte-identical `.f32`** to the knob-off dump — `features_v3` is knob-**invariant**, so retraining consumes identical data and emits the identical net. Structural reason, twofold: (a) `Dnf::disjoin`'s off-path is exactly the on-path's bounding box (hull of a union = union of hulls, componentwise), so born disjunctions never move the hull; (b) the hand-authored `Inferences::read` walk already narrows what the knob-gated projection upgrades (e.g. `Support::project`) would add, so the overlay's knob-on hulls intersect in redundantly. `PartnerShownLen`/`PartnerShownPoints` gates also read the hull (`constraint.rs` eval via `.partner()`), so they are knob-invariant too — the F entry's "hull tightening → net features + PartnerShownLen gates" mechanism is dead. **The flip's entire bidding delta flows through membership** (`Inferences::admits` any-box → the sampler → DD-priced decisions), which re-points the under-reach at the chop-E side-reading suspect: a box family (likely G-era) over-tightening sampled worlds so DD prices slams as failing. Next path: forensic trace of the existing `ab-results/dnf-flip` divergent boards to the flipped decision and its box family — **not** a retrain. The stale claim in `set_dnf_reading`'s doc comment is corrected in the same change | probed, retrain path closed |
| G0 | **MEASURED** 2NT shape redesign {M 2..=4, m 2..=6} (drops 5M(332), adds wide minors) — a treatment, not a rewrite. Knob `set_two_notrump_wide` (default off, byte-identical) + reading widen (opener's minors 2–6 under the knob); harness `--ns-two-nt-wide` in `bba-gen` + `scripts/two-nt-wide-ab.sh` (off/wide arms, plain+PD). sd-lead parked (disclosure mismatch — needs an in-process knob matrix). **Verdict: micro-positive WASH, SETTLED — not shippable, not refuted.** First run (SEED_BASE 1784788055, 204800 bd/arm/vul) leaned **+0.0009/bd** in all four cells but every CI spanned 0. The 5× re-measure (SEED_BASE 1784789763, **1024000 bd/arm/vul**) regressed the lean by half: plain none **+0.0004 ±0.0005**, pd none +0.0005 ±0.0005, plain both +0.0004 ±0.0006, pd both +0.0004 ±0.0007 — 3/4 cells still span 0 and plain-DD primary never clears → **WASH**. Effect is *real but negligible*: all four cells stay positive & same-order (not pure noise), yet per-fired regressed **+0.5 → +0.28 IMPs** (4/4 same sign) and it fires only **0.15%**. Worst tails repeat identically across all four cells (−14…−21 IMP slam/game misroutes: `A93.AKQT9.KJ.QJ9`, `KQT.Q9.AKT.QJT94`, `AKQ98.AQ.KT8.Q62`, `AK953.853.AK.AQT`); the many small minor-fit wins just outweigh them. **Stop here** — 3σ would need ~9M bd/cell at a magnitude beneath shipping relevance; not a re-measure candidate anymore. Knob stays default-off opt-in. Byte-identity holds (reading widen is knob-gated) | harness landed, opt-in |
| G-tail | term-merge if `debug_assert!(< 64)` ever fires; `described` closures whose labels name no axis (e.g. "spades longer than hearts, or equal five-plus" in the 1M responses) are invisible to the meter and stay ⊤ — convert on demand | parked |

## Ratchet counts

`authored_calls_read_what_they_gate` pins leak counts (a leak = an authored
rule whose describe names an axis while **no box** of its band constrains
it). Knob-off pins are the byte-identity guard (exact — must not move in
either direction); knob-on pins are the migration meter later chops drive
toward 0. Cells are knob-off/knob-on; knob-off never moved during the wave.

| After chop | HCP | length | points | support | support points |
| --- | --- | --- | --- | --- | --- |
| 095ac85 (old columns) | — | ≤49 | ≤61 (HCP+points mixed) | — | — |
| B | 17/0 | 71/29 | 3/0 | 84/84 | 18/0 |
| D1 | 17/0 | 71/24 | 3/0 | 84/84 | 18/0 |
| D1c | 17/7 | 71/47 | 3/3 | 84/84 | 18/12 |
| D2 (wave tip) | 17/7 | 71/47 | 3/3 | 84/0 | 18/12 |
| G | 17/**0** | 59/**0** | 3/**0** | 84/**0** | 18/**0** |

G's knob-off `length` 71 → 59 is **sniffer precision, not a reading change**
(dump diff clean): the twelve dropped entries are the eight "partner's last
suit is x" context claims and the four "≤13 x" deliberate no-op caps, which
never claimed anything about the hand.  The knob-on meter is **done** — every
authored rule that names an axis now reads a box constraining that axis; the
exact pins guard the zero.

D1c's knob-on **rise** is the meter getting honest, not a regression: an `Or`
with an opaque arm used to read `[tight-box, ⊤]`, which is membership-wise
just ⊤ — containment dedup collapses it to `[⊤]`, so phantom axis knowledge
no longer counts. The knob-on residue now points at real work: the opaque
`described`/`pred` arms (G) still co-gating rules that name an axis.

To list the rules behind any cell, zero that pin and run the test — the
failure dumps the per-column rule labels. The knob-on lists are chop G's
worklist.

## Irreducible tail (stays ⊤, by design)

Honor-location atoms (stoppers, `suit_hcp`, alt scales, keycards) have no
`Envelope` axis — though `top_honors` shows the pattern for squeezing a sound
*floor* out of one (n honors ⟹ n cards + their minimum HCP); `suit_hcp` could
do the same if a leak ever names it. Context atoms (`min_level_is`,
`they_bid`, seats, vulnerability) are hand-vacuous — ⊤ is *exact*, not a
leak.
