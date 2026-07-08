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
