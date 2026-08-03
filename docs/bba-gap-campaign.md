# The BBA gap campaign — closing pons↔BBA, especially via the floor

The standing plan for the campaign metric: `american()` vs BBA's 2/1 card,
IMPs/board.  **As of 2026-07-19 (the floor swap, B4) the anchor's pons side is
`american_instinct()`** — `american()` now ships the BBA-distilled net floor,
whose off-book calls do not decompose into book buckets, so the
decompose-and-rank series stays on the deterministic system; the net floor's
one-step gap reduction (+0.11 non-vul / +0.25 vul) is recorded in B4.
History: **−2.59** (S.1 anchor, 2000 boards, 2026-06-15) →
**−1.997** after M6.1 alone (4000 boards) → **first seeded, decomposed anchor**
(2026-07-06, sha `62cf5c5`, `SEED_BASE=1783375064`, 204.8k boards,
replay-verified 100%): **vul none −1.675 / vul both −2.310**, pooled **−1.99
plain / −2.40 PD** (findings and re-ranking below) → **re-anchored 2026-07-07,
sha `5f16e68`, 409.6k boards** (buckets #2–#4 shipped): pooled **−1.99 plain /
−2.36 PD** — the metric held, the fixes moved mostly PD.  **From 2026-07-28
(`3c94802`) the anchor runs with disclosure on** — BBA is told what we play — so
anchors before and after that date are different series; see the `3c94802`
re-anchor below.  This doc holds the
campaign structure, the anchor protocol, and the runbook; ship rules stay in
[measurement.md](measurement.md), per-treatment history in
[ai-bidder/21gf-ledger.md](ai-bidder/21gf-ledger.md) and
[competitive-book.md](competitive-book.md).

Three facts drive the design (researched 2026-07-07):

1. **The gap was never attributed.** Until now no seeded anchor was persisted
   and no general decomposition existed — "the gap concentrates in competitive
   auctions" was anecdote.  Pillar A fixed this, and **the first anchor
   overturned the anecdote**: the gap is *book-dominated* and concentrated in
   *defensive* first-round bidding, not competitive (see the findings below).
2. **The learned champion is stale but ship-grade.** `american_neural_search()`
   (M3.3 round 2) beats the deterministic floor on both scorers in self-play,
   but was trained before M6.3/M6.4 and has never been measured on the real
   vs-BBA routing.  Pillar B refreshes and gates it.
3. **A scorer wall parks real value.** DD/PD are blind to obstruction and
   right-siding; ~9 treatment families wait as opt-in knobs.  `single_dummy_leads`
   already flipped the Woolsey verdict but isn't in the generic pipelines.
   Pillar C wires it.

## Pillar A — anchor and decompose (SHIPPED; first anchor run 2026-07-06)

**Tooling** (landed 2026-07-07): `bba-gen` dumps now record `seed` +
`gen_args`; `Stance::explain_call` (book.rs) attributes any call to its
provenance and winning rule; `examples/bba-decompose` turns shard dumps into a
ranked-bucket `report.md` + `boards.jsonl`; `scripts/anchor.sh` orchestrates.

**Protocol**: 16 shards × 6,400 boards × {vul none, both} = 204.8k boards,
one persistent `SEED_BASE` for the whole anchor **series** (the sanctioned
exception to fresh-seed-per-experiment: successive anchors are arms of one
longitudinal paired experiment; every ~3rd re-anchor, run a fresh-seed
confirmation).  Headline pooled CI ≈ ±0.023 IMPs/board; a 0.3%-fired bucket
still resolves.  **Ship decisions stay per-fix fresh-seed A/Bs** — the anchor
tracks and attributes, it never ships.

### First anchor findings and re-ranking (2026-07-06, sha `62cf5c5`)

204.8k boards, `SEED_BASE=1783375064`, both arms replay-verified 100%.
Report: `ab-results/anchor/2026-07-06-62cf5c5/report.md` (committed).

**The headline finding overturns the going-in assumptions.**  The gap is
**book-dominated, not floor-dominated**, and concentrated in **defensive**,
not competitive, auctions:

- **By provenance:** `book` −248k IMPs vs the *entire* `instinct()` floor
  ~−160k spread over dozens of rules.  The single largest floor rule is
  `floor#3` (the opaque *pass*) at −38k; no other floor rule exceeds −17k.
- **By phase:** Defensive −171k **>** Constructive −155k **>** Competitive −82k.
  "Concentrates in competitive" was wrong.
- **By family:** round-1 −213k, round-2 −110k, opening −68k, balancing −11k,
  deep −6k.  Balancing is the 2nd-*smallest* family — the B2 "balancing is
  highest expected value" guess is **falsified**; deprioritize it.
- **By direction** (net): overbid −129k, missed-game −89k, sold-out −77k,
  wrong-strain −45k, missed-slam −40k, missed-grand −6k, doubling −6k; we
  *gain* +248k on 44.8k boards, so the −408k net is a two-sided distribution.

**Ranked losing buckets — latest anchor `5f16e68`, 409.6k boards (work these
top-down):**

| # | bucket | boards | plain IMPs | /div | PD IMPs |
| --- | --- | --- | --- | --- | --- |
| 1 | Defensive / book / round-1 | 59437 | −142733 | −2.40 | −188939 |
| 2 | Constructive / book / opening | 47692 | −103480 | −2.17 | −106037 |
| 3 | Constructive / book / round-2 | 43212 | −98201 | −2.27 | −98215 |
| 4 | Constructive / book / round-1 | 29727 | −76291 | −2.57 | −86039 |
| 5 | Competitive / fallback@1 / round-1 | 13846 | −44169 | −3.19 | −47594 |
| 6 | Competitive / fallback@2 / round-1 | 12606 | −42221 | −3.35 | −48671 |
| 7 | Defensive / floor#3 / round-2 | 9900 | −31665 | −3.20 | −34371 |
| 8 | Defensive / floor#3 / round-1 | 8597 | −29193 | −3.40 | −26309 |

Source: `ab-results/anchor/2026-07-07-5f16e68/report.md`.  This anchor doubled
the board count to 409.6k (32 shards/vul), so the **raw IMP totals are ~2× the
first anchor's — compare buckets on `/div`**, which held: defensive book is
still #1 at −2.40/div, Constructive/opening *improved* −2.34→−2.17/div
(Rule-of-20 light openings, bucket #2), the rest within noise.  Pooled held
−1.99 plain / −2.36 PD.  **Per-fix "after fix" numbers live in the CHANGELOG
A/Bs, not here** — the anchor tracks and re-ranks, it never measures a single
fix (bucket #5, flat-4333, shipped after this anchor and lands in the next re-run).

**Re-anchor `4afc985` (2026-07-08, 409.6k boards, same seed):** the 5332 +
flat-4333 takeout-discipline ships landed — bucket #1 shrank to −2.29/div
(−188939→−167653 PD), pooled **−1.89 plain / −2.11 PD** (was −1.99 / −2.36).
Ranking otherwise held; the top *un-worked* book bucket was #3
`Constructive/book/round-2` (−98269 plain ≈ −97924 PD, never traced), now
**worked**: opener's minimum natural rebid had no upper strength bound, so
monsters underbid (`5+ ♦` alone −20k, 2578/2636 a flat `2♦`).  Fix = opener's
extras ladder (jump-rebid / reverse / jump-shift) in the two minor-opening
nodes, **shipped default-on** (+0.0203/+0.0332 plain, +0.0181/+0.0297 PD, all
CIs>0; see the CHANGELOG and 21gf-ledger).  Source:
`ab-results/anchor/2026-07-08-4afc985/report.md`.  Follow-ups: the two
major-opening rebid nodes (Meckstroth `3m` collision) and the `5+ ♣`/`6+ ♠`/`6+
♥` residual.

**Re-anchor `c864bad` (2026-07-08, 409.6k boards, same seed):** the minor extras
ladder folded in — pooled **−1.84 plain / −2.07 PD** (was −1.89 / −2.11).
Re-ranked and traced the residual: bucket #1 `Defensive/book/round-1` (PD *worse*
by 31k → obstruction wall, Pillar-C territory); #2 `opening` (light-open frontier
+ already-refuted weak-twos); #4 `round-1` dominant leak (`1♥→1♠`, −9295) is
`set_longer_major_response`, an *already-measured null* (compression pays a level
on the heart fits). The one plain-workable, un-refuted lever was #3's own
residual — the `6+ ♥`/`6+ ♠` major single-suiter underbids — **worked**:
opener's major jump-rebid `3M` (6+/16+) + responder's continuation, **shipped
default-on** (+0.0059/+0.0125 plain, +0.0046/+0.0104 PD, all CIs>0; the bare rung
without the continuation LOST −0.005/−0.009 — see CHANGELOG and 21gf-ledger).
Source: `ab-results/anchor/2026-07-08-c864bad/report.md`.

**Re-anchor `308bbd1` (2026-07-09, 409.6k boards, same seed):** the major
jump-rebid folded in — pooled **−1.827 plain / −2.056 PD** (was −1.84 / −2.07).
The re-rank showed the DD-workable *book* buckets mined to residuals (round-2 =
mixed RKCB slam accuracy, M6.4 territory; round-1's top lever the already-null
`1♥→1♠`), leaving ~57% of the gap (−233k) in the two "obstruction wall" buckets
(#1 defensive round-1, #2 opening). **Pillar C was built and used to price them
(sd-lead, 5000 bd/vul × 16 worlds, ours-vs-BBA via synthetic dumps into
`ab-dump-sd`).  Verdict: BOTH are REAL losses, not DD artifacts** — def-r1 sd
−1.82/−2.72 ≈ plain (−1.79/−2.67); opening sd −1.98/−2.58 *worse* than plain
(−1.68/−2.42: a realistic blind lead can't beat BBA's thin light-open
contracts).  This settles the #1 label in favour of overreach (below), not
obstruction: sd-lead's payoff here is **diagnostic** (which walls are real →
fix with plain DD, which sd validates as fair-or-optimistic), not
value-unlocking.  The next DD-workable lever it surfaces is **overcall /
competition structure** — within def-r1, our own positive calls
(overcall/1NT/raise) are −90735 plain / −122908 PD (67% of the bucket, PD-worse
⇒ real); the genuine-obstruction remainder (we pass, BBA competes) is only
−29k.  See `project_sd-wall-diagnosis` and `ab-results/sd-wall/`.

**First overcall slice (2-level minor overcall) — sd-wash REJECT.**  The `2♣`/`2♦`
overcall (5+, 11+) bleeds ~−2/bd across every points/shape/vul band, so
`set_two_level_minor_overcall_tight` raises its floor to 15 (losing 11–14
minimums → Pass).  A/B vs BBA: plain +0.0015 NV / +0.0061 vul, PD +0.0075 /
+0.0131 — **but sd-lead washes both** (−0.0021 [±0.0031] NV, +0.0025 [±0.0040]
vul).  For a competitive range sd is the arbiter, so the plain/PD gains are the
obstruction-wall artifact; kept opt-in, default byte-identical.

> **REVERSED 2026-07-26 by SD-PD** (dump rescore, `sd-pd-dumps.sh`, 204800
> boards/vul, 2.27%/2.29% fired).  Plain sd never doubles, and here the *looser*
> arm is the default — so plain sd let the 11–14 overcalls it keeps fail
> unpunished, and the "wash" was the missing axe.  Repricing the same trick
> counts with failures doubled: plain SD +0.0010 ±0.0030 NV / +0.0019 ±0.0039
> vul (washes, as published), **SD-PD +0.0063 ±0.0037 NV / +0.0081 ±0.0047 vul —
> CI-clear positive both, tracking PD (+0.0075/+0.0131)**.  Plain DD is
> non-negative both vuls, so this is the house pattern *plain-DD wash + PD win →
> shippable default-on*, and the tighten is a **promotion candidate from opt-in
> to default-on** (confirm on a fresh seed before flipping — this is a rescore of
> one board set).  "For a competitive range sd is the arbiter" is retired: plain
> sd is the arbiter of nothing on its own, being optimistic on both the lead and
> the doubling axis at once.  Default deliberately untouched.

The lesson: the
anchor's *ours-vs-BBA* sd deficit on the overcall does not mean *suppressing* it
helps — the actionable A/B sd (suppress-vs-keep) washed because our own pass-line
is equally bad.  The recoverable def-r1 value, if any, is the CONSTRUCTIVE
`1NT`-overcall slice (`1NT→X`, PD-worse −8958) or the takeout doubles (−16k,
PD-worse), not overcall suppression.

**1NT-overcall systems on — def-r1's first WIN (shipped default-on).**  The
`[1t, 1NT, P]` advance was **unauthored** (the floor guessed), the one distinct
mechanism the three washed call-swaps could not reach — because it *adds
capability* rather than swapping a call.  `set_nt_overcall_systems_on` grafts the
full opening-1NT response structure (Stayman, Jacoby/minor transfers, Smolen)
verbatim below `[1t, 1NT]`, so `1♦–1NT` = `1♣–1NT` = an opening 1NT — 4-4 major
fits found, right-sided through transfers.  Mechanism: one re-rooting
`Trie::graft` shares the constructive `register_one_nt` subtree (the defensive
book cannot rebase across to the constructive `1NT` node — the keys collide,
they-open-`1NT` vs we-open-`1NT` — so the subtree is grafted, not rebased); the
`Inferences` reading strips their opening (`(len−index)%4` is seat-invariant
under the removal) so the floor reads the advancer's artificial calls.  A/B vs
BBA (32×6400 bd/arm/vul, minor vs major split): **sd-lead — the arbiter for a
competitive range — is a clean WIN in all four cells** (minor +0.0079 NV /
+0.0156 vul, major +0.0083 / +0.0133), and **sd exceeds plain everywhere** (the
signature of right-siding value DD undercounts, the opposite of the wall-wash);
plain never loses (+0.0051/+0.0112 minor win, +0.0013/+0.0044 major wash).  The
`Inferences` reading (strip their opening, read the advance as an opening-1NT
auction) strengthened the sd win over a no-reading run — keeping the floor off a
phantom suit in the contested tails is real, sd-visible value.  This is
the campaign's first def-r1 lever to clear the sd arbiter — the "obstruction
wall, skip" verdict was wrong for the *capability-adding* slice.  Of the
remaining def-r1 takeout-double mass (−16k), the **five-card-major slice** was
NOT wall-bound (below); the we-pass-they-compete −29k stays wall-bound.

**Five-card-major takeout discipline — def-r1's second WIN (shipped default-on,
`5f9d6c2`).**  Doubling with a biddable unbid five-card+ major buries the suit
and risks partner responding in our short suit (the def-r1 overbid/wrong-strain
leak).  `set_suppress_5card_major_takeout` (default on) rejects such hands in the
book takeout-double shape gate so they route to the natural major overcall,
extending the 5332/flat-4333 disciplines; the live leak is over a **weak two**,
where the 12+ shapely double (weight 1.3) outguns the two-level overcall (1.0),
and only the 12–16 range is redirected (17+ falls through to the separate
`points(17..)` double).  A/B vs BBA (409.6k bd/arm/vul, both vuls): a **plain +
PD + sd-lead WIN at both vulnerabilities, every CI > 0** — plain +0.0190 NV /
+0.0493 vul, PD +0.0892 / +0.1129, sd-lead +0.0124 / +0.0413 IMPs/bd.
Plain-positive rules out a doubling artifact; sd (the competitive-range arbiter)
confirms the right-siding.  The sibling 5-card-**minor** (textbook double) and
17+ single-suiter (needs an authored strong overcall-then-jump) slices stay
deferred.

**Re-anchor `5f9d6c2` (2026-07-09, 409.6k boards, same seed):** the
five-card-major discipline folded in — pooled **−1.758 plain / −1.864 PD** (was
−1.827 / −2.056 at `308bbd1`), replay-verified 100%.  Def-r1 shrank to −127014
plain / −146649 PD (was −134k / −164k; the discipline pulled its targeted
PD-heavy slice).  Re-rank: the DD-workable **book** buckets stay mined to
residuals (`opening` = refuted light-open wall; `round-2` = RKCB slam accuracy /
M6.4; `round-1` = the null `1♥→1♠` + splinter-slam).  The biggest **un-worked**
prize is now the two-sided **Competitive `fallback@1`/`fallback@2` round-1** pair
(−41021 + −37151 plain / −35146 + −34548 PD): our opening + their interference
where the floor's `0+ HCP` catch-all sells out — a Pillar-D classify + sd-lead
sub-campaign, not a one-shot fix.  Report:
`ab-results/anchor/2026-07-09-5f9d6c2/report.md`.

**Gladiator over the major-opening 1NT overcall — completed, WASH (parked opt-in).**
Over `1♥`/`1♠` the systems-on graft is only an sd win (plain/PD wash) — one
major is *theirs*, so symmetric both-major Stayman + two transfers misfire.
Gladiator (`set_nt_overcall_gladiator`, Belladonna/Helms shape economy, aligned
to the Crowborough write-up as an XYZ two-way relay: `2♣` = weak takeout **or**
any invitational hand, cue-of-their-major = Stayman for the one unbid major,
`2♦`/`2O` natural exactly-5 INV, `2NT` weak-6`♣` transfer, direct `3X`
game-forcing, splinter + Leaping Michaels) was the hypothesised fix.  First
measured a **loss on all three scorers both vuls** (major NV plain/PD/sd
−0.0075/−0.0120/−0.0102, vul −0.0135/−0.0152/−0.0178), diagnosed by branch as
the `2♣` relay + jump continuations dying **unauthored** below game while the
graft's full opening-1NT tree drove the same hands to 3NT/4M.  **Completing both
sides** (every overcaller answer + invitational relay rebids + the weak-club
transfer) erased the loss: re-measured A/B vs BBA (32×6400, minor/major split)
is a **wash on all three scorers both vuls** — major NV plain/PD/sd
+0.0006/−0.0004/+0.0004, vul +0.0005/+0.0027/−0.0015 (every CI straddles zero;
minor split 0-fired).  The diagnosis held: unauthored continuations were the
whole loss.  But completion only reaches **parity** — sd, the arbiter here, is
flat, so there is no measured win to justify flipping the graft default.  Kept
byte-identical opt-in as a faithful, complete alternative structure and a
single-dummy re-measure candidate.  Lesson restated: a half-authored replacement
loses to a fully-authored graft; a fully-authored one draws.

**Re-anchor `50ad20b` (2026-07-10, 409.6k boards, same seed):** Fix 1 of the
fallback@1/@2 sub-campaign folded in (Modern negative doubles + forcing free
bids + `answer_free_bid`, default-on) — pooled **−1.732 plain / −1.891 PD**
(was −1.758 / −1.864 at `5f9d6c2`), replay-verified 100%.  Plain moved +0.026
(NV +0.039 / vul +0.013, matching the fresh-seed A/B); PD −0.027 is the
already-adjudicated vul-PD artifact the sd arbiter overruled.  The target pair
**Competitive `fallback@1`/`fallback@2` round-1** shrank −78.2k → **−51.7k
plain** (−27105 + −24572; PD −28288 + −27451) and drops to ranks 6/8 — Fix 1
cashed ~26k, the residual is Fix 2 (cue-context raises + Jordan rejection) +
Fix 4 (strong-values action) territory.  Re-rank: the top of the table is back
to the mined book buckets (def-r1 −126113, constructive opening/r2/r1 −94k /
−81k / −70k), then `Defensive floor#3` (r2+r1 ≈ −57k pass discipline).  Next
in queue ahead of those residuals: the **school tournament** (1-level Modern
vs Cachalot vs Sputnik, 2-level forcing vs NFB vs transfers) now that Fix 1
completed the books — P3d′/P3d″ were both-incomplete comparisons.  Report:
`ab-results/anchor/2026-07-10-50ad20b/report.md`.

**Re-anchor `5b5115d` (2026-07-10, 409.6k boards, same seed):** the
post-`50ad20b` batch folded in — the natural 11-12 `2NT` jump over a 1-level
overcall, opener's balanced-18-19 notrump in a contested `1X (1Y)` auction, and
the rein on a minimum takeout doubler over-raising a *forced* advance (all
default-on; the Cachalot contested-`X` fix is opt-in, so it leaves the default
anchor unmoved) — pooled **−1.684 plain / −1.765 PD** (was −1.732 / −1.891 at
`50ad20b`), replay-verified 100%.  Plain moved +0.048, PD +0.126 (the vul-PD
doubling artifact the sd arbiter had overruled unwinds as the thin doubled games
clear).  Re-rank: the head is unchanged — def-r1 `Defensive/book/round-1` still
#1 but shrank −126113 → **−123392 plain** (−141682 PD), then constructive
`opening`/`round-2`/`round-1` (−93067 / −81168 / −69526), then `Defensive
floor#3` r2+r1 (≈ −55.5k pass discipline).  The target **Competitive
`fallback@1`/`fallback@2` round-1** pair holds ≈ flat (−26434 + −23647 = −50.1k
plain) — this batch was competitive-reopening + floor work, not the fallback
classify (Fix 2/4).  The **school tournament** resolved: Modern + Forcing keep
the defaults; Cachalot and Sputnik ship opt-in and are now surfaced as a radio
family on the web Settings tab.  Report:
`ab-results/anchor/2026-07-10-5b5115d/report.md`.

**Re-anchor `973d681` (2026-07-19, 409.6k boards, same seed):** first anchor
after the floor swap — `american()` now ships the non-decomposable BBA net, so
the harness was repointed to the deterministic side (`anchor.sh` generates with
`--our-floor american-instinct`, `bba-decompose` replays through
`american_instinct()`); replay-verified **100%** (0 of 4.24M calls mismatched),
confirming the reference is bit-reproducible again.  Pooled **−1.500 plain /
−1.683 PD** (vul none −1.300 / −1.390, both −1.700 / −1.976), from −1.684 / −1.765
at `5b5115d`.  The **+0.184 plain / +0.082 PD** is the interim default-on batch —
the constructive gate/eval ships (fit-split 2/1 + 1M-3NT choice of games, the
fit-sum major-game gate, points-as-rule-of-N+8 + `support_points` + the four
point-count gate fixes, Wide6322 as the default 1NT, the forcing-1NT major
two-suiter, the Meckstroth adjunct) plus the advance-of-`X` / passed-hand-overcall
competitive work.  Re-rank: the head order is unchanged but every book bucket
shrank as those ships landed — def-r1 `Defensive/book/round-1` still #1 at
**−111127** (was −123392), then constructive `opening`/`round-2`/`round-1`
(−86728 / −64692 / −49912, from −93067 / −81168 / −69526); the competitive
`fallback@1`/`fallback@2` round-1 pair holds ≈ flat (−24201 + −21439).  The
**shipped** floor is the BBA net on top of this deterministic prior: +0.11 NV /
+0.25 vul (B4), which does not decompose and is measured as a separate
routing-gate A/B.  Report: `ab-results/anchor/2026-07-19-973d681/report.md`.

**Re-anchor `eb02d9d` (2026-07-26, 409.6k boards, same seed):** 109 commits of
default-on ships since `973d681`, none of them individually anchored — pooled
**−1.152 plain / −1.355 PD** (vul none −1.024 / −1.116, both −1.279 / −1.593),
from −1.500 / −1.683.  Both arms replay-verified **100%** (0 of 4.23M calls).
The **+0.348 plain / +0.328 PD** is the largest single-batch move the series has
recorded, and unlike earlier batches it is *not* concentrated: every phase
improved (Defensive −251275 → −194513, Constructive −232496 → −168727,
Competitive −130622 → −108429) and so did both provenances (`book` −322470 →
−245400, `floor#3` −85435 → −64160).  The batch is the shared-vocabulary work —
`points` → `PointCount`, suit-indexed `support_points`, the DNF projection flip
(which moves the floor twice: tightened hulls into `partner_shown_*`, and a
swapped weight artifact into the bilans evaluator) and bilans itself going
default-on — so it lifts every consumer at once rather than one bucket.

Re-rank: **the head order is unchanged for the fourth anchor running** — def-r1
`Defensive/book/round-1` still #1 at **−85805** (was −111127), then constructive
`opening`/`round-2`/`round-1` (−71779 / −45155 / −35084, from −86728 / −64692 /
−49912); `floor#3` defensive pass discipline r2+r1 −51372 → −40338; the
competitive `fallback@1`/`fallback@2` round-1 pair −45640 → −38645.  One row
moved differently from the rest: `Constructive/book/opening` gained **+29348
PD** against only +14949 plain (PD −88698 → −59350), i.e. the openings we now
avoid were the ones getting doubled — a PD-shaped win that plain DD understates.

**What the *shipping* pair scores (`--our-floor american`, same deals, same
seed):** pooled **−1.021 plain / −1.254 PD** (vul none −0.929 / −1.150, both
−1.112 / −1.357), i.e. the BBA net floor is worth **+0.131 plain / +0.101 PD**
on top of the deterministic prior.  Read the headline only — replay verification
is 89–90% *by construction* (the net floor's off-book calls do not reproduce
through `american_instinct()`), so `report-american.md`'s bucket rows are not
valid and only the IMP figures, which come from the recorded auctions, are.

That total hides a split worth a re-measure: the floor is **+0.167 plain /
+0.236 PD at vul both**, but at **vul none it is +0.094 plain and −0.034 PD** —
it gains on plain DD while *losing* under perfect defense, the signature of
calls that buy the contract and then get doubled.  The unpaired CIs overlap
([−1.1754, −1.1248] vs [−1.1416, −1.0905]), so this is suggestive rather than
decided; the arms share deals, so a paired NV A/B of the floor's routing gate
would settle it cheaply.  For context, B4's routing gate recorded +0.11 NV /
+0.25 vul at `7122756`, eight net-floor commits ago.

**Re-anchor `3c94802` (2026-07-28, 409.6k boards, same seed) — and the series
changes meaning here.**  `bba-gen --disclose` now defaults to `generated`, so
from this snapshot on **BBA is told what we play**: every earlier anchor faced a
BBA that took us for a BBA, and part of what those numbers measured was its
misreading of our conventions.  Pooled **−1.113 plain / −1.273 PD** (vul none
−1.004 / −1.064, both −1.222 / −1.481), from −1.152 / −1.355.  Both arms
replay-verified **100%**.

**Do not read the +0.039 / +0.083 as disclosure's effect.**  Thirty commits
landed between the snapshots — the v3 calls-tail evaluator (+0.018/+0.028
measured), the 1NT 3♥/3♠ splinter, the vulnerable weak-two overcall gate, the
card-generator fixes — so the batch confounds them with the flip.  Disclosure's
isolated cost remains `ab-disclose.sh`'s −0.009/board, measured with the old
static card; re-running it against the *generated* card is the outstanding
question, and cheap now that a full anchor generates in 12 minutes.

The re-rank is the tell.  Every phase improved (Defensive −194513 → −192542,
Constructive −168727 → −156731, Competitive −108429 → −106717) and so did both
provenances (`book` −245400 → −238966, `floor#3` −64160 → −60087) — but
**`Defensive/book/round-1`, the #1 bucket, went the other way**: −85805 →
−88313, the only head bucket to lose ground in a batch that lifted everything
else.  That is exactly where disclosure should bite.  We defend by overcalling
and doubling, and those are the calls the card now explains to them.  Head order
otherwise unchanged for the fifth anchor running: constructive
`opening`/`round-2`/`round-1` −69131 / −40047 / −33747 (from −71779 / −45155 /
−35084), `floor#3` defensive r2+r1 −40338 → −37776, the competitive
`fallback@1`/`fallback@2` round-1 pair −38645 → −38179.

*Caveat on cross-anchor `floor#N`*: the labels are only stable within a build.
`floor#3` carries identical rule text in both reports so its row is comparable;
`floor#246`/`floor#247` rows are new numbering and are **not** to be diffed
against `973d681`.

**#1 is the real prize and it is a *book* item, not a floor item.**  Our
defensive first-round structure — overcalls, takeout doubles, two-suiters
over their opening — bleeds −2.40/div (−142733 raw at 409.6k bd), and PD is
*worse* (−188939), so it is genuine overreach, not a doubling artifact (the
worst boards are our own 3♥x / 4♣x / 2♥x going down).  The biggest *floor*
lever is `floor#3` pass discipline in defense (buckets 7–8, ~−61k combined:
our floor passes where BBA acts).  This
re-ranks the campaign: **Pillar D defensive book first (bucket 1), then
constructive openings/rebids (2–4); Pillar B2 balancing drops to backlog and
its floor effort points at `floor#3` pass discipline instead.**

### First-anchor runbook (any machine with the BBA submodule)

```sh
git pull && git submodule update --init vendor/bba
setsid nohup scripts/idle-run.sh scripts/anchor.sh \
    >ab-results/anchor.log 2>&1 &
```

Generation ≈ minutes; the one-time DD solve of the divergent union is the
bottleneck (tens of minutes).  Re-anchors after a batch of fixes take ~5
minutes: the DD cache (`ab-results/anchor/dd-cache.json`) keys on deals,
which never change under the fixed seed.  Afterwards:

1. Check `report.md`'s **replay verification = 100%** — below that the dump
   was generated with non-default knobs or a drifted revision; fix before
   trusting buckets.  **The usual cause is a stale `bba-gen` clap default.**
   `bba-gen` applies ~130 `set_*` knobs from CLI defaults while `bba-decompose`
   replays on crate defaults, so when a crate default flips and the matching
   `default_value` is not updated in the same commit, an unflagged run silently
   measures a system we do not ship.  That is what a *tiny* miss looks like
   (`eb02d9d`: 1447 of 4.23M calls, 0.034%, from `--ns-two-over-one-gate` still
   defaulting to `hcp13` a day after `39a5eb6` shipped `points13`) — a knob that
   is wholly on or off mismatches far more loudly.  Confirm it is systematic by
   re-running the decompose on one shard twice: an identical count is drift, a
   varying one would be nondeterminism.  **When you flip a crate default, grep
   `examples/bba-gen/main.rs` for its `set_*` in the same commit.**
2. Anchor outputs stay **untracked**. `/ab-results` is gitignored and `6a5cbdb`
   dropped the 21 previously-committed reports: every snapshot regenerates from
   the series `seed` + the SHA, so the repo carries the headline (here and in
   CHANGELOG.md), not the artefact.
3. Record the headline in the 21gf-ledger campaign-metric line and
   CHANGELOG.md.

**Reading the report**: rank on plain DD, PD printed beside (a plain/PD sign
flip is flagged as a doubling artifact); preempt-shaped defensive buckets are
DD-pessimistic (obstruction wall) — sd-lead re-check before working them;
same-contract divergences (right-siding) are counted and excluded.  The
composite key is *phase / provenance / family*: `floor#N` names the exact
instinct rule (stable within a build), `book` an exact node, `fallback@d` a
guarded fallback at depth d.  The steady-state loop:

```text
anchor report → worst bucket → trace its boards → fix (floor / book / node)
→ fresh-seed ship A/B (measure-ab skill) → re-anchor (~5 min) → next bucket
```

## Pillar B — the floor track

### B1. Learned-floor round 3

The round-2 champion's training data predates the current books;
`search_floor.rs` already pins the round-2 net as the rollout policy, so
regenerating the search-dump today *is* the M3.2 iteration.  Wiring (half a
day): `dump-search --features-version 2` (mirror `dump-teacher`), trainer
`--truncate-features 160` (train v1 + the v2-tagged head from one dump —
tests M5.1's "tags pay off on the search target"), `bba-gen --our-floor
neural-search` (one cfg'd arm next to `neural-v3`, main.rs ~1167),
`bba-gen-parallel.sh` `FEATURES` passthrough.  Data: 10k boards ≈ 27–30 h
single-stream under idle-run (never concurrent with another heavy job).
Acceptance (accept-only-gains): `ab-neural-floor` 20k × both vuls × both
scorers, round-3 ≥ round-2 and ≥ the round-2 bar vs the deterministic floor;
then **the decisive new gate — the real routing**: paired `bba-gen` runs
(`--our-floor american` vs `neural-search`), ~102.4k boards/arm, both vuls,
`ab-dump-diff` plain+PD.  A floor that wins self-play but bleeds vs the
mature reference does not advance.

**Promotion stance (user, 2026-07-07): harness default only.**  If the
routing gate passes, campaign measurement runs adopt the champion floor as
the default arm; the **crate default stays `instinct()`** — the disclosure
objection stands (the net cannot `describe`/`project` its calls).  Revisit
only if Pillar A shows floor buckets dominating the remaining gap.

### B2. Deterministic `instinct()` improvements

**Re-prioritized by the first anchor.**  The floor is a *minority* of the gap
(~−160k vs the book's −248k), so B2 is second in line behind the defensive
book (Pillar D).  The three themes below were pre-anchor guesses; the anchor's
actual largest floor lever is **`floor#3` pass discipline in defense** (a new
item 0, ~−25k: our floor passes where BBA acts — reopens, doubles, competes),
and balancing-*seat* value is small (−11k family), so old item 1 drops to
backlog.  Author parametrically on the ladder (suit loops + context
predicates, never a node per sequence), one `set_*` knob + `bba-gen` flag
each, measured per the M6.4 protocol (~204.8k boards/round vs BBA, both vuls,
both scorers, `ab-instinct-floor` telemetry to confirm the rule fires
unshadowed):

0. **`floor#3` pass discipline in Defensive round-1/round-2** (the anchor's
   top floor lever): trace buckets 7–8 — where our floor passes and BBA
   reopens/doubles/competes for gain — and tighten the pass predicate.  PD is
   the honest scorer.
1. **Balancing/reopening block** (backlog — small per the anchor; `defense.rs`
   notes the "toxic balancing doubles"): a `pass_out_seat()` predicate,
   reopening ranges ~3 points lighter than direct seat, borrowed-king X on
   shortness, balancing 1NT band, and an explicit *sit* rule (trump
   stack/misfit → defend).  PD is the honest scorer.
2. **Help-suit trials over Rubens advances** (instinct.rs `ponytail:` at the
   Rubens block): parametric try-bid + accept/sign-off — DD-visible
   constructive value in the competitive-advances theme.
3. **Floor 5NT king-ask + book minors king-ask** (missed-grands theme):
   extend the M6.4 floor-RKCB ladder (instinct decodes instinct, same
   derived-trump gates); low fired-rate × huge swing → read IMPs/fired.

Backlog (only if Pillar A shows the buckets bleeding): misfit runout pull,
advancer 4-4 bust escape.

### B3. BBA steal-list verdicts (settled — don't re-derive)

Suit templating and parametric rules: **already pons house style** (Rust
suit loops = BBA's templates; `partner_shown_len`/derived trump = "calculated
bid") — no work item.  Weighted-table vs strict precedence: **dropped** —
M7.0's −2.96 regression plus the provability of the shadowing invariants;
keep only a *shadowing audit* (when a bucket bleeds, check worst boards for a
book node shadowing a smarter floor and fix that node locally).

### B4. BBA-distilled floor (`neural-bba`) — routing gate PASSED (2026-07-19, sha `7122756`)

The B1 wiring, realized against the **BBA** teacher instead of the search
teacher. `dump-teacher --teacher bba` dumps `(features_v3, BBA-argmax)` rows —
one-hot targets, since the oracle exposes only its chosen call, so this is hard
behavioral cloning, not soft-target distillation; the existing candle trainer
fits them unchanged; `neural::classify_bba` + `NeuralFloorBba` +
`american_bba_neural()` seat the net in the same disclosable-v3 shell as
`neural-v3`; `bba-gen --our-floor neural-bba` is the cfg'd arm;
`bba-gen-parallel.sh` gained a `FEATURES` passthrough.

- **Learnability:** held-out top-1 vs BBA 85.9% (constructive 86.7%, contested
  85.4%), 40k-board dump. Below v3's 95.3% cloning `american()` — the one-hot
  argmax target + disclosable-only features, as predicted; `val_ce` bottomed
  ~epoch 170 then overfit, so capacity is not the limit.
- **Routing gate (the decisive one):** paired `bba-gen --our-floor american` vs
  `neural-bba` vs live BBA, two seeds (1784412234 × 51.2k, 1784414157 × 102.4k
  per cell), both vuls, `ab-dump-diff` plain+PD. **`neural-bba − american`:
  +0.12/+0.13 (none plain), +0.10/+0.09 (none PD), +0.23/+0.22 (both plain),
  +0.29/+0.26 (both PD) IMPs/bd** — every cell a plain-AND-PD win, both seeds,
  CIs excluding 0 by 3.5–9σ, fired ~27–29%. PD ≥ plain ⇒ not a doubling
  artifact. Teacher-isolation vs `neural-v3`: +0.32…+0.47 (clears the
  american-distilled net cleanly).
- **Disclosure audit (clean):** neural-bba's call distribution matches
  `american`'s in shape — same 38 calls, no new artificial-call class, no gadget
  spike (4NT 0.37% vs 0.34%); the extra IMPs come from natural aggression (more
  5-level competing / slam tries, fewer takeout doubles). No book nodes added,
  so `artificial_calls_are_alerted` is untouched — disclosure posture identical
  to the shipped `neural-v3` floor.
- **Stance → PROMOTED (2026-07-19, the floor swap):** the routing-gate win
  shipped as the **crate default** — `american()` now floors off-book with the
  BBA net, and the deterministic pre-swap system is `american_instinct()` (the
  anchor's new pons side, the distillation teacher, and the integration-test
  target). `american_bba_neural()` is now an alias of `american()`; the BBA net
  path is always compiled. The earlier follow-on "(a) contested-only partition"
  was **dropped as backwards**: the floor has no phase gate — it already floors
  *constructive* off-book decisions too (the net was trained on the whole
  auction; `instinct()` is a shallow heuristic exactly where the auction runs
  past the book), so routing constructive→instinct would *give back* measured
  IMPs. Remaining follow-on: reach *past* BBA by putting the live DD search on
  top of this stronger prior (M8). Not compared against the search champion
  `neural-search` — different category.

## Pillar C — measurement unlock (sd-lead third scorer)

Wire `single_dummy_leads` into the generic pipelines; it plausibly
adjudicates 7 of 9 parked families (lead direction, disclosure, trick-one
right-siding).  Mid-play concealment stays unmeasurable — that is the future
MC-cardplay effort, explicitly out of scope here.

- Library: promote `ns_score_tricks` (from `ab-nt-defense-matrix`) into
  `src/scoring.rs`; add `LeadQuestion::read(deal, dealer, vul, auction,
  stance)` to `src/single_dummy.rs` (owns the leader-prefix cut +
  `Stance::infer`).
- Pipelines: `bba-score` + `ab-dump-diff` gain `--score sd`, `--sd-worlds`
  (default 16, the validated GTO setting), `--sd-seed`, `--sd-sanity`
  (Pavlicek anchor, must land ≈ +0.2..+0.4 tricks at the 1–2 level).
  Divergence granularity becomes *auction* divergence; each arm's auctions
  are read by **its own arm's book**, rebuilt from the dump's `gen_args`
  (kills silent knob drift).  Shared chunk helper in `examples/common/sd.rs`;
  split `bba-gen`'s `Args`+knob application into `examples/bba-gen/args.rs`
  for reuse.
- Decision table extension (measurement.md; **plain-DD loss never ships**
  stays iron): new row *wash/wash + sd-win (CI>0) → shippable default-on*;
  plain-loss + sd-win re-classifies to "sd-positive, blocked on plain loss"
  with mandatory forensics.  sd verdicts count for competitive/lead-shaped
  treatments below slam level only.
  **RETIRED 2026-07-25 — the "sd-win" here meant *plain* SD, which is not an
  arbiter.**  Plain SD relaxes the defenders' lead *and* leaves every failing
  contract undoubled, so it is friendlier to aggression than plain DD itself;
  this rule is how several defaults shipped over a negative PD.  The sd-win now
  has to be an **SD-PD** win (`ns_score_pd_tricks`, failures doubled), quoted
  beside its plain-SD twin, and the plain-SD number alone decides nothing.  See
  measurement.md §"Plain SD is not an arbiter"; the verdicts this rule produced
  are on the re-adjudication queue.
- Exploitation guard: a vs-BBA sd win must be confirmed by self-play sd or an
  advertised rerun (`--advertise-*`); on sign disagreement, ship on the
  self-play side.
- Re-adjudication queue (mass × decidability): 1NT-defense closeout →
  Cachalot/Sputnik right-siding (also the go/no-go for resurrecting
  Rubensohl) → P2a preemptive raises + Jordan 3o flip (fix the two named P2a
  leaks first) → DoubleStyle/responsive-overcall → delayed-cue → free-bid
  family (authoring-blocked: shape gate first).

## Pillar D — book batches (ledger-driven)

Work the [21gf-ledger](ai-bidder/21gf-ledger.md) batches, re-ranked by the
Pillar-A report after each anchor: Batch 1 competitive (Woolsey #43, Unusual
1NT #126, two-suit T/O X #123, Rubensohl-after-1m #105, maximal doubles #83,
transfers-if-RHO-bids-clubs #122), Batch 2 slam tools (Gerber, Exclusion,
DOPI/ROPI, BROMAD), Batch 3 constructive (Drury, two-way game tries, Garbage
Stayman, Bergen/mixed-raise, Namyats), plus the competitive-book follow-ups
(P2a leak fixes, P3a 12+ re-measure, P3b shape gate, "off-shape X stronger",
alert invariant over `Trie::fallbacks()`, P4 contested tails, balancing-seat
two-suiter reading) and the bba-multi-2d counter-defense.  Process per item:
the `author-convention` + `measure-ab` skills, unchanged.

## Sequencing

```text
DONE 2026-07-06:               first anchor run + committed (findings above)
next, data-driven:             bucket 1 (Defensive/book/round-1) → trace →
                               fix defensive book → ship A/B → re-anchor (~5m)
then:                          constructive openings/rebids (buckets 2–4)
in parallel (idle box):        B1 wiring + round-3 dump (27-30 h) → gates
when a bucket hits the wall:   build Pillar C, drain the sd queue
```

Iron hygiene throughout: one `SEED_BASE` per experiment shared across arms
(anchor series excepted, documented above); arms sequential under
`scripts/idle-run.sh`; never rebuild during a run; both scorers always; ship
by the decision table; CHANGELOG + ledger for every measured result.
