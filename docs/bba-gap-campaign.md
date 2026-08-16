# The BBA gap campaign — closing pons↔BBA, especially via the floor

The standing plan for the campaign metric: `american()` vs BBA's 2/1 card,
IMPs/board.  **Standing at `0d8b755` (2026-08-10): −0.627 plain / −0.585 PD for
what ships, −1.069 / −1.205 for the deterministic side the buckets decompose.**
**As of 2026-07-19 (the floor swap, B4) the anchor's pons side is
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
`gen_args`; `Partnership::explain_call` (book.rs) attributes any call to its
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
+ already-refuted weak-twos); #4 `round-1` dominant leak (`1♥ → 1♠`, −9295) is
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
`1♥ → 1♠`), leaving ~57% of the gap (−233k) in the two "obstruction wall" buckets
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
minimums → -).  A/B vs BBA: plain +0.0015 NV / +0.0061 vul, PD +0.0075 /
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

> **REFUTED 2026-08-12 on the fresh seed — promotion cancelled.**  The
> confirmation the reversal asked for, run at `abdafcc` (seed `1786488117`,
> 409,600 bd/arm/vul, double the July count): plain DD **−0.0102 ±0.0021 NV** /
> −0.0011 ±0.0027 vul, PD +0.0074 ±0.0026 / +0.0198 ±0.0034, plain SD
> −0.0157 ±0.0022 / −0.0096 ±0.0028, **SD-PD −0.0008 ±0.0026 NV / +0.0090 ±0.0033
> vul**.  The NV plain-DD interval is entirely below zero, so the veto applies:
> no Tier-F guard was run and the default is untouched.  The NV SD-PD win
> collapsed to a wash; only the vul cell reproduced.  plain-DD loss beside a PD
> win is the decision table's *artifact of PD's synthetic X*, not a win.
>
> The reversal never described current code.  July was measured at `daf6c0e`;
> `abdafcc` (generalize defensive overcalls) widened the trigger from
> 1.43%/1.49% to **1.76%/1.75%** fired (sd 2.27%/2.29% → 2.50%/2.53%), so the two
> runs do not cover the same hands.  Forensics on the 20 worst NV boards:
> **trigger-too-broad**, not a missing continuation.  9–12 of 20 show the loose
> arm's 2m overcall enabling a profitable doubled sacrifice against a making game
> (`1♥ 2♣ 3♣ - 4♥ - - 5♣ X` on a 12-count with six clubs; the tight arm passes
> and defends `4♥`), and a smaller group shows the stranded hand re-entering
> later at a higher level and being doubled.  A flat points floor is the wrong
> gate for a hand whose value is length — §O4's own diagnosis, this knob having
> only ever been its crude fallback.  Stays opt-in, default byte-identical.
> Artifacts: `ab-results/two-level-minor-overcall-refresh/`.

> **Forensics corrected 2026-08-12 — the population pass** (same day,
> `scripts/ab-classify.py` over all 10,250 auction-diverged NV boards).  The
> worst-20 "trigger-too-broad" read described the tail, not the mechanism: the
> knob is a **declare-vs-defend switch**.  Loose declares 35.1% of fired boards
> vs tight's 10.5%, is doubled 3× as often (7.8% vs 2.6%), and still wins
> 0.58 IMPs per score-diverged board under plain DD — the profitable doubled
> sacrifices *are* where the default earns its edge, and BBA doubles our
> contracts at ≈15% in both arms while we double theirs at 1.2–3.6%.  The
> improvable decision is the contested 5-level node, now designed:
> [ai-bidder/competitive-accountant.md](ai-bidder/competitive-accountant.md),
> evidence + P(double) calibration in
> [ai-bidder/doubling-calibration.md](ai-bidder/doubling-calibration.md).

The lesson: the
anchor's *ours-vs-BBA* sd deficit on the overcall does not mean *suppressing* it
helps — the actionable A/B sd (suppress-vs-keep) washed because our own pass-line
is equally bad.  The recoverable def-r1 value, if any, is the CONSTRUCTIVE
`1NT`-overcall slice (`1NT → X`, PD-worse −8958) or the takeout doubles (−16k,
PD-worse), not overcall suppression.

**1NT-overcall systems on — def-r1's first WIN (shipped default-on).**  The
`(1t) 1NT -` advance was **unauthored** (the floor guessed), the one distinct
mechanism the three washed call-swaps could not reach — because it *adds
capability* rather than swapping a call.  `set_nt_overcall_systems_on` grafts the
full opening-1NT response structure (Stayman, Jacoby/minor transfers, Smolen)
verbatim below `(1t) 1NT`, so `(1♦) 1NT` = `(1♣) 1NT` = an opening 1NT — 4-4 major
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
M6.4; `round-1` = the null `1♥ → 1♠` + splinter-slam).  The biggest **un-worked**
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
the constructive gate/eval ships (fit-split 2/1 + `1M - 3NT` choice of games, the
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

> **"By construction" was wrong (corrected 2026-08-12).** The replay was 89–90%
> because these runs decomposed a v5 dump through `american_instinct()`. Since
> `7af286d`, `bba-decompose --our-floor american` replays it through
> `american()` and the mismatch is **zero** — the bucket rows of a shipping
> report are valid, and `anchor.sh` passes the flag. The IMP figures in this
> row are unaffected; only the "not valid" verdict on its buckets is.

That total hides a split worth a re-measure: the floor is **+0.167 plain /
+0.236 PD at vul both**, but at **vul none it is +0.094 plain and −0.034 PD** —
it gains on plain DD while *losing* under perfect defense, the signature of
calls that buy the contract and then get doubled.  The unpaired CIs overlap
([−1.1754, −1.1248] vs [−1.1416, −1.0905]), so this is suggestive rather than
decided; the arms share deals, so a paired NV A/B of the floor's routing gate
would settle it cheaply.  For context, B4's routing gate recorded +0.11 NV /
+0.25 vul at `7122756`, eight net-floor commits ago.

> **Settled 2026-08-06 under the configured floor — the NV pathology is gone.**
> The paired A/B this paragraph asked for, run at `e650a86` on the anchor seed
> (1783375064, 204 800 bd/vul), with **both arms generated at the same sha**:
>
> | `american` − `american-instinct`, v4 floor | plain DD | perfect defense | fired |
> | --- | --- | --- | --- |
> | vul none | **+0.1745** ±0.0128 | **+0.2540** ±0.0156 | 26.15% |
> | vul both | **+0.2247** ±0.0162 | **+0.3802** ±0.0194 | 24.33% |
>
> NV perfect defense goes **−0.034 → +0.2540**: the calls that bought the
> contract and got doubled are no longer there, and the floor now wins on both
> scorers at both vulnerabilities. Pooled, the floor is worth **+0.200 plain /
> +0.317 PD** against v3's +0.131 / +0.101.
>
> Two method notes, both of which cut against over-reading the improvement.
> First, v3's figure was a **difference of two absolute vs-BBA gaps** (hence
> "the unpaired CIs overlap"); the table above is a **paired** `ab-dump-diff`,
> same quantity but a much tighter instrument. Second, **do not diff a fresh
> `--our-floor american` arm against an older snapshot's `american-instinct`
> arm.** Doing exactly that against `3c94802` — 87 `src/bidding` commits back,
> several of them behaviour-changing for `american_instinct()` itself — reads
> **+0.1905/+0.2832 NV and +0.2489/+0.4214 vul**, overstating the floor by
> ≈9–11% because the *baseline* had improved in the interval. Regenerate the
> control at HEAD.

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

**Re-anchor `0d8b755` (2026-08-10, 409.6k boards, same seed).**  Pooled
**−1.069 plain / −1.205 PD** (vul none −0.967 / −1.006, both −1.171 / −1.405),
from −1.113 / −1.273 — **+0.044 plain / +0.068 PD** for the deterministic side.
The batch is the reading-drift tail (face-trump, cramped-doubled, DOPI/ROPI/DEPO,
rung-2, ask-gate recalibration, cue-face + the NT dichotomy) plus the knob-home
refactors; the v4/v5 floor swaps are invisible here **by construction** — this
series bids `american_instinct()`.  For what ships, see the companion below.

Head order unchanged for the sixth anchor running: `Defensive/book/round-1`
**−84479** (from −88313), then constructive `opening`/`round-2` −66278 / −38547
(−69131 / −40047), and constructive `round-1` −31669 (−33747).  `floor#3`
defensive r2+r1 −37776 → −37219.  One row moved provenance rather than value:
**`Competitive/book/round-1` enters the head at −32251 while `fallback@1`
collapses −38179 → −440** — the shipped competitive-book packages now author
calls the fallback used to catch, so the loss moved rows without moving the
total.  Compare competitive rows on the *phase*, not the provenance, across
this boundary.  Report: `ab-results/anchor/2026-08-10-0d8b755/report.md`.

> **Replay verification stopped being exact, and that is a defect.**  Both arms
> report 69 and 66 mismatched calls (of 2.12M and 2.10M — 3×10⁻⁵, so the
> printed rate still rounds to 100.00%, but the sub-100% warning fires).  Every
> earlier anchor in the series replayed **0** mismatched.  The IMP headline is
> unaffected (it is computed from the recorded auctions), but bucket attribution
> is approximate at that rate for this row only.  **Cause found and fixed
> (2026-08-11).**  `bba-gen` builds our floor from `arm_knobs(args)` — the CLI's
> *armed* `Agreements` — while `bba-decompose` replays
> `american_instinct(&Agreements::default())`, so the series has always relied on
> `arm_knobs(default args) == Agreements::default()`.  `--uvu` broke it: the flag
> dates from when Unusual-vs-Unusual was opt-in and read `if args.uvu {
> set_uvu(true); … }`, which a default run skipped, leaving the shipped default
> intact.  `2a18843` ("one home for the competitive knobs") rewrote it as the
> unconditional `agreements.competition.uvu = args.uvu`, and `default_value_t =
> false` turned a force-on flag into a kill switch — so every arm at this sha bid
> UvU off (the shipping companion below too — same `bba-gen`, and its numbers are
> therefore a hair pessimistic)
> while the replay bid it on, and the mismatch rate is exactly what an auction
> needing our 1NT *and* their both-minors 2NT should cost.  Renamed `--no-uvu`;
> the equality is now a `bba-gen` unit test
> (`default_args_arm_the_shipped_system`), which prints the offending field pair.
> **Read this as a rule, not an incident:** a flag whose default disagrees with
> the crate's silently re-points every A/B at a system nobody plays, and only the
> replay check ever notices.

**Corrective re-anchor `42454d2` (2026-08-11 local / 2026-08-10 UTC) — exact
again.** The persistent seed `1783375064` was regenerated after the `--no-uvu`
fix: 204,800 boards per vulnerability, **0 replay mismatches** in both arms.
The headline is pooled **−1.069 plain / −1.206 PD** (none −0.9673/−1.0061,
both −1.1714/−1.4060), so the approximate row's gap estimate was unaffected,
but this report replaces it for attribution. `Defensive/book/round-1` remains
#1 at **52,523 boards, −84,484 plain / −102,011 PD**; the next authored buckets
are constructive opening −66,313 and constructive round-2 −38,547. Report:
`ab-results/anchor/2026-08-10-42454d2/report.md`.

The report was re-emitted at clean `7af286d` with the new generic JSON fields
and PD bucket CIs; the same cache reproduced every headline with zero new DD
solves and zero replay mismatches. For the BEN Phase-1 shared-residual gate,
constructive round-2 is excluded as already worked/mined in this ledger, leaving
`Defensive/book/round-1` as the largest eligible authored bucket. Its clearest
unworked slice is our simple `1M` overcall versus the reference's weak `2M`
jump: 4,895 boards, **−2.454 ±0.194 plain / −2.785 ±0.228 PD** per divergent
board (BEN independently: 1,348, −1.102 ±0.345 / −0.976 ±0.416). More than 98%
are exactly six-card majors and all are 6–11 HCP. The
`direct_weak_jump_overcall` treatment and fresh-seed runner are now authored;
the 204.8k/arm/vulnerability BBA gate passed at seed `1786431801`. It fired
1.16%/1.11% none/both: plain **+0.0002 ±0.0028 / +0.0031 ±0.0035**, PD
**+0.0026 ±0.0032 / +0.0051 ±0.0040**, plain SD **+0.0010 ±0.0030 /
+0.0037 ±0.0036**, and SD-PD **+0.0032 ±0.0033 / +0.0057 ±0.0040**. All
four arms contain 32×6,400 boards with identical deal streams and zero runner
failures. The decision-table verdict is plain wash plus PD win/wash with no SD
refutation. The same-seed Tier-F gate then washed in every cell: plain
**+0.0098 ±0.0099 / +0.0013 ±0.0128**, PD **+0.0101 ±0.0120 / −0.0020
±0.0154**, and SD-PD **+0.0058 ±0.0120 / −0.0087 ±0.0147** none/both on
12,800 boards per cell (145/143 fired). Its pooled point estimates are +0.0055
plain / +0.0040 PD and no SD cell refutes it. The two-reference gate therefore
ships the treatment default-on; `false` / `--no-ns-direct-weak-jump-overcall`
restore the historical simple overcall. A fresh exact anchor follows the
default flip.

**Post-ship re-anchor `782f09e` (2026-08-11, persistent seed `1783375064`).**
Both 204,800-board vulnerability arms replay exactly (0/2,112,342 and
0/2,100,834 mismatched calls). The new headline is none **−0.9566 plain /
−0.9991 PD**, both **−1.1608 / −1.4006**, pooled **−1.059/−1.200**. Against
the exact `42454d2` pre-fix row, that is +0.010 plain / +0.006 PD pooled. The
target bucket responds directly: `Defensive/book/round-1` falls from 52,523
boards and −84,484 plain / −102,011 PD to 49,068 boards and **−74,202 /
−90,593**. Report: `ab-results/anchor/2026-08-11-782f09e/report.md`.

> **Retracted 2026-08-12 — this paragraph's shipping numbers were `0d8b755`'s,
> relabelled.** It read: *"What the shipping pair scores (`--our-floor
> american`, same deals, same seed, both arms generated at this sha): pooled
> −0.627 plain / −0.585 PD (vul none −0.555 / −0.499, both −0.699 / −0.671) …
> replay is 90.19% / 90.99% … the net floor is worth +0.442 plain / +0.620 PD."*
> No `american-*` arm was ever generated at `782f09e` — the snapshot dir has
> only the two instinct arms, and every figure quoted is `ab-results/
> anchor-american.log`'s `0d8b755` run rounded to three places (−0.5551,
> −0.4991, −0.6993, −0.6713; replay 90.19% / 90.99% to the digit). The
> "generated at this sha" claim is the one part that was not true, which is the
> part that matters: it is the control-at-HEAD rule broken in the doc that
> states the rule. `anchor.sh` now generates the shipping pair itself, in the
> same snapshot, so the claim is structural rather than remembered.

**Post-ship re-anchor `ea2cde9` (2026-08-12, persistent seed `1783375064`,
snapshot `ab-results/anchor/2026-08-12-ea2cde9-dirty`).** The re-anchor for the
competitive accountant, and the first one to carry all four arms — both
instinct and both shipping — in one snapshot at one sha. "dirty" is honest: the
tree carried the `bba-gen` flag-polarity fix the ship needed and had not got
(see CHANGELOG), without which the `american-*` arms would have measured the
pre-ship system. Whole run, generation through both decomposes and both paired
diffs: **19 minutes**.

| arm | vul | plain | perfect defense |
| --- | --- | --- | --- |
| `american-instinct` (decompose series) | none | −0.9539 | −0.9972 |
| `american-instinct` | both | −1.1572 | −1.3983 |
| **`american` (shipping)** | none | **−0.5383** | **−0.4944** |
| **`american` (shipping)** | both | **−0.6702** | **−0.6630** |

Pooled: instinct **−1.056 / −1.198**, shipping **−0.604 / −0.579**. All four
arms replay **100.00%** (0 of 2.11–2.13M our-side calls mismatched, each arm).

- *Instinct side*, against `782f09e`'s −1.059 / −1.200: **+0.003 / +0.002**,
  i.e. flat, which is the expected reading — `competitive_gate` is reachable
  only from `neural_floor.rs`, so the ship cannot touch this arm.
- *Shipping side*, against the `0d8b755` run the paragraph above retracts:
  **+0.023 plain / +0.006 PD** pooled. The accountant's own paired A/B claims
  +0.0088 / +0.0140 → ≈ +0.011 pooled, so it is roughly half of the plain move
  and the rest is `abdafcc` plus `782f09e`'s weak-jump-overcall ship. Loose
  attribution by construction (a difference of absolute gaps across changed
  code); the paired instrument below is the tight one.

**The shipping report's bucket rows are valid now — the "89–90% by
construction" claim was wrong.** Passing `--our-floor american` to
`bba-decompose` (the flag `7af286d` added on 2026-08-11, hours before the
`782f09e` anchor, and which the hand-rolled shipping runs never picked up)
replays the v5 dumps through `american()` instead of `american_instinct()`, and
the mismatch goes to zero. It was never a property of the net's off-book calls.
So `report-american.md` now ranks the **shipped** system's losses, including
rows the instinct-arm decomposition structurally cannot show — the net floor's
own: `Defensive/floor/round-2` −13,891 plain, `Competitive/floor/round-2`
−13,593, `Defensive/floor/round-1` −8,307. Head of the shipping table:

| bucket | boards | net plain | net PD |
| --- | --- | --- | --- |
| Defensive / book / round-1 | 48,652 | −41,244 | −56,178 |
| Constructive / book / round-2 | 39,688 | −40,693 | −45,496 |
| Constructive / book / opening | 53,194 | −40,619 | −21,546 |
| Constructive / book / round-1 | 24,891 | −24,661 | −27,838 |
| Competitive / book / round-1 | 14,692 | −20,541 | −18,221 |

`Defensive/book/round-1` stays #1 on the shipped system as it is on the instinct
prior, at roughly half the instinct arm's −74,202 — the floor already eats half
of the campaign's #1 bucket, and the def-r1 redesign
([defensive-overcalls.md](defensive-overcalls.md)) is still aimed at the right
place.

**The net floor's paired worth (`ab-dump-diff`, same snapshot, same sha — the
tight instrument the `e650a86` note asked for and no anchor had yet run):**

| `american` − `american-instinct` | plain DD | perfect defense | fired |
| --- | --- | --- | --- |
| vul none | **+0.2141** ±0.0131 | **+0.2570** ±0.0157 | 26.81% |
| vul both | **+0.2557** ±0.0165 | **+0.3752** ±0.0196 | 24.90% |

Pooled **+0.235 plain / +0.316 PD**. That retires the retracted +0.442 / +0.620:
the unpaired difference-of-gaps overstated the v5 floor by ≈1.9×, which is the
`e650a86` warning landing a second time and much harder. Against v4's paired
+0.200 / +0.317 at `e650a86`, v5 is +0.035 plain and level on PD — but across
87+ book commits, so read it as "v5 is not worse", not as a v4↔v5 measurement.

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
   `examples/bba-gen/main.rs` for its `set_*` in the same commit.**  Since
   2026-08-11 the equality itself is a unit test —
   `default_args_arm_the_shipped_system` in `bba-gen`, run by `cargo test`
   (`[[example]] test = true`) — so this class of drift fails the build instead
   of a report line.  Its first catch was `--uvu`, which `2a18843` inverted from
   a force-on into a kill switch by rewriting an `if args.uvu { … }` guard as an
   unconditional assignment; watch for that shape whenever an opt-in knob's
   arming is mechanically rewritten.
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

**Promotion partnership (user, 2026-07-07): harness default only.**  If the
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
- **Partnership → PROMOTED (2026-07-19, the floor swap):** the routing-gate win
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
- **SUPERSEDED 2026-08-05 by the configured net.** The v3 artifact this section
  shipped (and its later kickback twin) were deleted when `ConfiguredFloorBba`
  over `american_bba_v4` became the default floor, on gate 1's
  +0.1933/+0.2469 plain and +0.5256/+0.5358 PD at 2M fresh boards per cell
  (`docs/ai-bidder/configured-net.md` phase 6). Two figures here are now
  historical: B4's own routing-gate numbers, and the **+0.131 plain / +0.101 PD**
  shipping-pair side-run below, which priced the v3 floor against
  `american_instinct()` on the anchor's deals. A re-run of that side-run is owed
  — gate 1 measured v4 winning on *both* scorers at *both* vulnerabilities,
  which is evidence the NV PD loss recorded there is gone. **The anchor series
  itself is unaffected**: it is anchored on `american_instinct()` precisely
  because `american()` carries a net floor, and `bba-decompose` hard-codes that.

## Pillar C — measurement unlock (sd-lead third scorer)

Wire `single_dummy_leads` into the generic pipelines; it plausibly
adjudicates 7 of 9 parked families (lead direction, disclosure, trick-one
right-siding).  Mid-play concealment stays unmeasurable — that is the future
MC-cardplay effort, explicitly out of scope here.

- Library: promote `ns_score_tricks` (from `ab-nt-defense-matrix`) into
  `src/scoring.rs`; add `LeadQuestion::read(deal, dealer, vul, auction,
  partnership)` to `src/single_dummy.rs` (owns the leader-prefix cut +
  `Partnership::infer`).
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

## Ledger (memory compaction, 2026-08-16)

- **Historical full-disclosure diagnostic (2026-06-24, superseded by the
  per-table `--advertise-natural` harness):** with BBA reading our natural `2♣`
  as Multi-Landy, the bucket appeared **+2.01 IMPs/bd**. Turning
  `Multi-Landy=0, Cappelletti=0` on all four seats moved that bucket **+328 →
  −64 IMPs** and the isolated defense **+0.013/bd → −0.274/bd**, CI
  [−0.41,−0.14]. That run also changed the all-BBA reference table and is
  confounded — the diagnostic that exposed the artifact, not the honest
  verdict; the per-table result in `CHANGELOG.md` supersedes its magnitude.
- `bba-decompose`'s `boards.jsonl` `board` field is an index **within its
  shard**, paired with that shard's `seed`, not a flat arm index — flat
  concatenation produced impossible 0-HCP hands for 12+-HCP rules. The emitted
  actor `hand` field (S.H.D.C) is canonical; re-run decompose when an older
  dump lacks it, and sanity-check hand-derived analyses against the rule's
  stated HCP floor.
