# Doubling calibration — P(double) for the competitive accountant

**Status: population pass done 2026-08-12 and recorded here (previously in no
committed doc); q table pending; the were-the-doubles-right DD check pending;
BBA's `expected_double` still unread.** This doc owns the *evidence and
calibration* for the competitive accountant; the gate itself is designed in its
sibling [competitive-accountant.md](competitive-accountant.md). It supersedes
the committed **"trigger-too-broad"** forensic classification of the
`two_level_minor_overcall_tight` refutation (CHANGELOG, `docs/bidding-options.md`,
`docs/bba-gap-campaign.md`, `docs/defensive-overcalls.md` §O4 — each carries a
dated correction pointing here). The refutation itself — all eight scorer cells,
the veto, the fired-rate drift — is recorded in those four places and is not
restated; this doc holds what the *population* said afterwards.

Why calibration lives outside the A/B: **an A/B cannot arbitrate the parameter
that decides the behaviour it is scoring.** The refuted knob measured
−0.0102 plain / +0.0074 PD on the same NV boards; the entire gap between those
brackets *is* the doubling model (PD doubles every failing contract, plain DD
never punishes). Any gate built on an assumed P(double) will reproduce whichever
bracket its assumption matches. So q is fixed here, from out-of-band data,
*before* the gate's A/B — and the A/B then judges the gate, not the parameter.

## Evidence — the population pass (2026-08-12)

Source: the retained fresh-seed refutation dumps,
`ab-results/two-level-minor-overcall-refresh/` (seed `1786488117`, sha
`abdafcc`, 409,600 boards/arm/vulnerability, four arms × 32 shards × 73 MB,
gitignored — the dumps are one disk failure from gone; this table and
`scripts/ab-classify.py` are the durable record). Opponents are BBA/EPBot, our
default `american()` floor both arms; ON = tight (overcall floor 15), OFF =
loose (the shipped default). Non-vulnerable arms; a board **fires** when the two
arms' auctions differ — 10,250 of 409,600 (2.50%). Note the two denominators:
`ab-dump-diff` reports 7,213 fired because it counts *score*-diverged boards;
the population pass counts *auction*-diverged. No fired board passes out in
either arm.

| final contract, per fired NV board (n = 10,250) | tight (ON) | loose (OFF, default) |
| --- | ---: | ---: |
| we declare | 1,079 (10.5%) | **3,602 (35.1%)** |
| doubled, either side declaring | 271 (2.6%) | **801 (7.8%)** |
| at the 5-level or higher | 545 (5.3%) | 814 (7.9%) |
| at the 5-level or higher, doubled | 65 (0.6%) | 296 (2.9%) |
| at the 1-level | 1,372 (13.4%) | 0 |

The tight arm loses **−0.581 IMPs per score-diverged board** under plain DD
(−0.0102 ±0.0021 IMPs/board; the veto cell of the refutation). Read against the
table: the loose arm is doubled **3× as often and still wins**. The knob was
never a points gate — it is a **declare-vs-defend switch**. Tight, we defend
their partscore (13.4% of its contracts are at the *one* level: the auction dies
where the overcall never happened); loose, we compete, declare 3.5× as often,
and walk into the high-level contested decisions. The whole value of the loose
2-level overcall is those later contested contracts, so tightening the entry is
an indirect way of never facing the 5♣ decision — at the measured price of
0.58 IMPs per diverged board. **The decision worth improving is the one at the
5-level, not the entry gate.** That conclusion is the charter of
[competitive-accountant.md](competitive-accountant.md).

Doubling, by direction:

| direction | tight (ON) | loose (OFF, default) |
| --- | ---: | ---: |
| they double the contract we declare | 158 / 1,079 = **14.6%** | 561 / 3,602 = **15.6%** |
| we double the contract they declare | 113 / 9,171 = **1.2%** | 240 / 6,648 = **3.6%** |

Two findings:

- **BBA doubles our contracts at ≈15%, nearly identically in both arms** — a
  usable empirical prior for `P(they double | we declare)` against this
  opponent, robust to a 3.3× change in how often we declare.
- **We double them 4–12× less than they double us.** Part of this is
  mechanical: every instinct-book double rule is gated
  `their_live_bid_at_most(3)` (`src/bidding/instinct.rs`), so the book cannot
  double a 4- or 5-level contract at all — above the 3-level only the learned
  floor's judgement logits can say X, unpriced. Whether the residual is also a
  defensive *judgement* leak is unowned (out of scope below), but the asymmetry
  is why the gate prices our X alongside pass and bid-on.

**Correction of the committed forensics.** The refutation's committed post-mortem
read **"trigger-too-broad"** off the 20 worst NV boards (9–12 of 20 showing the
loose overcall buying a profitable doubled sacrifice). The population pass
refutes that generalization: the worst-board tail is real but *is the point* —
those sacrifices and 5-level decisions are where the loose arm's +0.58/diverged
is earned, not leaked. A tail of 20 hand-picked disasters described the variance,
not the mean. Method lesson, standing: **classify the population before naming
the mechanism**; the worst-board trace is for finding *candidate* mechanisms
only (this is `docs/measurement.md` step 9's intent, now with a measured
counterexample for skipping the population half).

Reproduce (~3 s, pure JSON, no DD solve; first argument is ON):

```sh
python3 scripts/ab-classify.py ab-results/two-level-minor-overcall-refresh/on-none \
                               ab-results/two-level-minor-overcall-refresh/off-none
```

## The q table — `P(doubled | we declare)` by level and vulnerability *(pending)*

The number the gate consumes. Extraction: final-contract pass over the
**default (loose) arms** of the retained dumps — `off-none` and `off-both` —
recording (level, vulnerability, doubled) for every board we declare; a small
extension of `ab-classify.py`. Fill:

| level | vul | n declared | n doubled | q | 95% CI |
| ---: | --- | ---: | ---: | ---: | --- |
| 1–2 | none | | | | |
| 3 | none | | | | |
| 4 | none | | | | |
| 5+ | none | | | | |
| 1–2 | both | | | | |
| 3 | both | | | | |
| 4 | both | | | | |
| 5+ | both | | | | |

Rules: no cell ships on n < 200 declared — thin cells inherit the level-pooled
rate; Wilson intervals (counts are small at 5+). Named caveats, accepted for v1:
**opponent-specific** (BBA, the exploit guard — not BEN, the north star),
**level-marginal** (no auction-shape conditioning), and drawn from boards where
a knob fired (competitive-heavy by construction — which is the population the
gate acts on).

## Were the doubles right — the one-DD-pass check *(pending)*

Converts the marginal 15% into what the gate actually needs:
`P(X | we fail)` vs `P(X | we make)`. The dumps carry every deal and auction;
solve just the **doubled final contracts** of the default arms double-dummy
(~800 NV + the vul arms' share; minutes, `Solver` on the main thread) and fill:

| slice | n doubled | made anyway | down 1 | down 2+ | net IMPs of the X |
| --- | ---: | ---: | ---: | ---: | ---: |
| they double us, NV | | | | | |
| they double us, vul | | | | | |
| we double them, NV | | | | | |
| we double them, vul | | | | | |

If BBA's doubles land overwhelmingly on failing contracts, the gate applies q to
the failing branch only (`P(X | fail) ≈ q / P(fail)` , capped at 1); if they are
indiscriminate, q applies to both branches and doubling matters less than the
15% suggests. The we-double-them rows also price the missing-X leak: the net
IMPs of the doubles we *did* find is the empirical value of extending them.

## BBA's own estimator — `expected_double`, read *(pending)*

BBA's Stage 4 prices being doubled — `expected_double`, `probable_kontra`
(*kontra* = double), `korekta_kontraktu`, `potencjalny_zapis_z_naszej_gry`,
`zalozenia_to_vulnerable`, under `C_IMP_SCORE`/`C_MP_SCORE` — and none of that
arithmetic has been read yet ([bba-floor.md](bba-floor.md) §5.5 read the level
ladder and the 50/70/90 buckets, not the doubling functions). Reading it is the
cross-check on the empirical table: a general-opponent model beside our
BBA-specific one. Land the results as a graded-evidence table in §5.5's style
(`symbol | status | reading`), in this section.

Standing warnings, carried from `bba-floor.md`: none of Stage 4's scoring
surface crosses the FFI (`examples/probe-bba-bilans` dumps only
`probable_levels`, whose **scale is undecoded — never fit against it**), and
§5.5's headline is that Stage-4 "expected score" prices the *chosen* level
rather than choosing it — expect `expected_double` to be a correction term, not
an integral.

Reproduce:

```sh
dotnet tool install -g ilspycmd --version 9.1.0.7988  # pin; 10.x needs .NET 9
ilspycmd -o /tmp/epbot-decompiled -p vendor/bba/EPBot64.dll  # ~6.5 s
grep -rn "expected_double\|probable_kontra\|potencjalny_zapis\|korekta_kontraktu" \
    /tmp/epbot-decompiled/EPBot64/
cargo run --features serde --example probe-bba-bilans -- --self-check
```

## Named alternatives and their failure modes

The canonical table; the design doc inherits it by reference.

| model | failure mode | precedent |
| --- | --- | --- |
| always doubled (q = 1) | reproduces the PD bracket's behaviour, which on this exact knob measured as a **plain-DD loss** — the veto bracket. Right for `ev.rs`'s rollouts (its cardplay already assumes perfect defense, so undoubled penalties let the search chase phantom saves — a deliberate, internal-consistency choice) and wrong as a live-opponent model | `src/bidding/ev.rs` module doc; the refutation's plain/PD split |
| never doubled (q = 0) | failing sacrifices priced at undoubled penalties → the M3.1 7NT sacrifice flood (grand:game reached 1.00 in self-play before PD-doubling fixed the rollouts) | `project_m31-7nt-sacrifice-instability`; `stats.rs` par has always used `min(normal, doubled)` |
| constant q ≈ 0.15 | level-blind: overprices being doubled in partscore competition (where BBA rarely doubles) and underprices it at the 5-level (where sacrifices live) | the population pass, once the q table splits by level |
| **empirical q(level, vul)** — chosen | opponent-specific (BBA); recalibration is data, not design — rerun the extraction per opponent | this doc |

## Out of scope (decided, not neglect)

- **Our under-doubling as its own campaign.** The 1.2–3.6% finding is recorded
  above; raising the book's 3-level double wall or auditing the net's X
  judgement is separate work, not part of this calibration.
- **BEN Tier-F recalibration** — the second opponent population (~3 h at
  16-wide); the fleet is down. The q table's provenance column exists so this
  slots in later.
- **Per-auction-shape q** (e.g. "they freely bid game" vs "they saved") — v2
  conditioning, only if the gate's A/B forensics demand it.
- **Redoubles.** XX pricing is out of the gate's v1 scope entirely
  ([competitive-accountant.md](competitive-accountant.md) §Out of scope).
