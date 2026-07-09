# The BBA gap campaign — closing pons↔BBA, especially via the floor

The standing plan for the campaign metric: `american()` vs BBA's 2/1 card,
IMPs/board.  History: **−2.59** (S.1 anchor, 2000 boards, 2026-06-15) →
**−1.997** after M6.1 alone (4000 boards) → **first seeded, decomposed anchor**
(2026-07-06, sha `62cf5c5`, `SEED_BASE=1783375064`, 204.8k boards,
replay-verified 100%): **vul none −1.675 / vul both −2.310**, pooled **−1.99
plain / −2.40 PD** (findings and re-ranking below) → **re-anchored 2026-07-07,
sha `5f16e68`, 409.6k boards** (buckets #2–#4 shipped): pooled **−1.99 plain /
−2.36 PD** — the metric held, the fixes moved mostly PD.  This doc holds the
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
obstruction-wall artifact; kept opt-in, default byte-identical.  The lesson: the
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
   trusting buckets.
2. Commit the lean set: `seed`, `log`, `report.md`, `boards.jsonl`,
   `dd-cache.json` (shard dumps are regenerable in minutes from seed + SHA).
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
