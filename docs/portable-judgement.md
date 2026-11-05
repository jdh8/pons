# Portable judgement — does the chooser survive a change of book?

**Status: planned 2026-11-01; Phases 0–2 read (Phase 2 on 2026-11-05):
the union chooser ties `bba-d` on the held-out book, so Phase 3 does not
fire; D6 shipped 2026-11-05 (`bba` serves `bba-r`, won four cells); Phase 4 stays owed.** Each phase
below is one piece, with its build, its run, its read and its stop rule;
later phases wait on the earlier read. Decisions in §7 were taken at their
defaults to write the plan and are one line to overturn.

**One line.** The book is the *agreements* (which calls a hand may make and
what each means); the chooser is the *judgement* (which admitted call to
make). The `bba` system ([bba-system.md](bba-system.md)) proved the seam is
mechanical: `Rules::choose_by` lets any book delegate its choice. This plan
asks whether the judgement half, once optimised, is the same object under a
different book, and lays out the cheapest experiments that say yes or no.

## 0. The two questions (jdh8, 2026-11-01)

1. Can an accountant/judgement optimised under one book work under other
   bidding systems?
2. Would a simpler, arithmetic-only judgement like BBA's bilans be more
   reliable across systems?

Question 2 is already answered by data (§2); question 1 is what the phases
measure.

## 1. Where the system enters the judgement today

The chooser is three things: the K = 8 v6 net, the two accountants (the
constructive `break_even` and the competitive collar over the evaluator
net's `trick_estimates`), and the five rails. The book reaches each of them
through named channels. Read off the code at `e92c635b`:

| channel | carries the system? | where | note |
| --- | --- | --- | --- |
| hand block (10) | no | `features.rs` `features_v3` head | own cards only |
| auction-context block (36) | no | `push_auction_context` | strains named, contract to beat, partner's last bid, penalty, seat, passes: mechanical facts of the auction |
| vulnerability (2) | no | `push_vul` | |
| **inference block** (4 seats × `LEN_INFERENCE_V6`) | **yes, in distribution** | `push_inference_v6` from `Inferences` | the *readings* of every prior call. The columns mean the same thing under any book (HCP bands, length hulls, support points), but which values occur at a node depends on the book that read the calls. This is trap 5 of bba-system.md |
| **compact agreements block** (2 × 28) | **yes, by design** | `CompactConfig` from `Agreements` | the regime input: 13 live slots per side after the fold ([card-manifold.md](ai-bidder/card-manifold.md)). It *is* a domain label |
| artificial block (12, v8) | yes, derived | `call_artificial` masks | which prior calls were artificial, read off the book |
| **teacher labels** | yes | `dump-teacher --teacher bba` | BBA's softmax on BBA's own call. The label is BBA's judgement under BBA's agreements; the features are the same decision read under *ours*. A matched pair only when the reader is BBA's book |
| **relabel labels** (M32, BBA-opponent) | yes | `dump-teacher --relabel` | rollouts bid out by *our* system; the value of a call is the value of our continuations after it |
| evaluator net (54 floats) | readings only | `features_eval`, [evaluator-net.md](ai-bidder/evaluator-net.md) | "no auction in the input, ever": own hand plus the four seats' readings. System-blind modulo the inference channel above |
| competitive collar thresholds | yes, by calibration | [competitive-accountant.md](ai-bidder/competitive-accountant.md) "The collar is calibrated to one floor's distribution" | tuned to the v6 floor's contract-height distribution; did not react to a 43% rise in doubled contracts under v7 |
| economics (q table, `break_even`, par, scoring) | no | `stats.rs`, the gate | exact arithmetic, book-free by construction |

So the portable interface already exists and is the **reading**: a book turns
calls into `Inferences`, and everything downstream consumes those. What is
not portable is (a) the *distribution* the net learned over that interface,
(b) the regime block, which names the system outright, (c) the labels, which
were priced under one book, and (d) the collar's constants. None of these is
a design flaw; each is a corpus or a calibration, and each has a known fix.

**The ML frame.** The net learned p(call | x) on x drawn under `american()`.
Under BBA's bands the same columns take different values: p(x) moved,
p(call | x) need not have. That is **covariate shift**. If the net's function
is right everywhere, nothing breaks; if it learned shortcuts that only hold
on the training distribution (and a net with no incentive to generalise
will), it breaks exactly where the shift is. Training across several books
and testing on one it never saw is **domain generalisation**, and the
compact block is already the domain one-hot a multi-domain trainer would
add. Each of these is a matmul we already run; only the data changes.

## 2. What is already measured

| arm, vs BBA, plain IMPs/board (none / both) | reading |
| --- | --- |
| `american`: our book, our chooser | −0.267 / −0.270 |
| `bba`: BBA's book, our chooser | −0.634 / −0.729 |
| `bba-ladder`: BBA's book, the deterministic ladder at constructive nodes | −1.482 / −1.870 |
| `american_instinct` under `basic`: our book, the ladder (vs the net, [system-variants.md §5a](system-variants.md#5a-verdict-2026-10-14)) | −0.317 / −0.403 behind the net |

Source: [bba-system.md §9.3](bba-system.md#93-gate-2--read) and
[system-variants.md](system-variants.md). Two readings follow.

- **Question 2 is closed.** "Simpler is more portable" predicts the ladder's
  loss to the net *shrinks* under a foreign book. It grew: 0.32–0.40 under
  ours, 0.44–0.60 under BBA's. The ladder is not more robust, it is simply
  worse, and more so where its natural precedence puts no mass on the
  admitted calls. The arithmetic physics lost twice before that
  ([competitive-accountant.md "The backend question"](ai-bidder/competitive-accountant.md#the-backend-question--do-we-need-bba-style-arithmetic-accounting)):
  the BBA-shaped least-squares rung was 0.34 tricks of MAE behind the net,
  and `net_collar` lost all four cells. BBA's own Stage 4 is a
  points-to-level ladder with a finite-difference bucket
  ([bba-floor.md §5.5](ai-bidder/bba-floor.md#55-the-arithmetic-itself-read--2026-07-26--closes-row-b)),
  not an engine worth copying. What makes BBA portable is its Stage 1: the
  evaluator consumes a reconstructed deal, not the auction. We already have
  that split; the arithmetic half (economics) is exact and book-free, and
  the learned half (physics, choice) reads the reading. **No arm is spent on
  question 2.**
- **Question 1 is open, and the number that answers it is not yet
  measured.** The `bba` arm's extra loss (−0.37 / −0.46 over `american`)
  confounds two things: BBA's agreements are worth less under any chooser,
  or our chooser misreads BBA's bands because it never trained on them.
  bba-system.md §9.3 names the split as the one owed experiment: a matched
  retrain on a `bba`-read corpus. That is Phase 1.

## 3. Design — three domains, one chooser

A **domain** is a book that projects readings. Three are cheap:

| domain | book | teacher | cost to stand up |
| --- | --- | --- | --- |
| `american` | authored 2/1 | BBA on its 2/1 defaults (the shipped corpus) | exists |
| `bba` | BBA's 2/1 compiled from the `--card none` walk | the same BBA; **its book is the bands**, so teacher and reader finally match | the walk exists (`ab-results/bba-book/2026-10-29-8fb078de-none`); the reader is `dump-teacher --system bba` (built 2026-11-01) |
| `bba-pc` | BBA's Polish Club (EPBot system 2) compiled from a second walk | BBA playing system 2 (`dump-teacher --card`, `deviation-panel.md` row 2) | a walk (23 s) and a reach corpus (7 s) under the Polish card, plus the same loader; **no authoring** |

The third domain is the point of the plan: a system no one here authored,
compiled from BBA's own rules, with BBA as its native teacher. If a chooser
trained on the first two bids it well, the judgement ports. The `basic`
variant is a fourth, nearly free (an `Agreements` value), but it is the
`american` family with gadgets off and differs from `american` on folded
slots only, so it is a weak test of transfer; it stays the cheap sanity
check, not the held-out domain.

The experiments pair **training domain × serving domain**, always against
BBA on the same deals, always K = 8 against K = 8 of the same recipe and
seeds (the reseed rule, [reseed-gap-handoff.md §6](reseed-gap-handoff.md#6-what-stays-decided)):

| trained on ↓ served on → | `american` | `bba` | `bba-pc` |
| --- | --- | --- | --- |
| `american` (shipped) | −0.27 (anchor) | −0.63 / −0.73 (§2) | Phase 2 |
| `bba` | Phase 2 | **Phase 1** | Phase 2 |
| `american` ∪ `bba` | Phase 2 | Phase 2 | **Phase 2, the held-out read** |

The diagonal says what each book is worth with a matched chooser; the
off-diagonal says what a mismatch costs; the union row says whether one
chooser can hold two books at once and whether that buys anything on a third.

## 4. Phases, cheapest first

### Phase 0 — the shift, measured without DD (≈ half a day, no boards)

**Build (done 2026-11-01).** `dump-teacher --system bba`: the reader
partnership is `common::seat_floor("bba")` (`BbaBook::system` over the
walk `BBA_BOOK` names; build with `--features serde`) instead of
`american()`, the BBA teacher plays `bba_book::card()` on both sides, and
the sidecar records `our_system: "bba"` and `bba_book`. The walk was
`--card none` with no kickback, so `--system bba` takes the one cell
`a-off/a-off` and refuses a kickback or flipped cell, a bare context and
the v4 card block. The deal bank, the feature version and every other flag
stay. The chooser attached to the reader is irrelevant for the dump: a
floor projects nothing, and the bands do the reading. The `american` side
of the pair is the same command with `--cell a-off/a-off`, so the two
corpora share dealer, vulnerability and cell on every board.

    BBA_BOOK=ab-results/bba-book/2026-10-29-8fb078de-none \
      dump-teacher --deals <bank> --skip S --boards N --teacher bba \
      --configured --feature-version 6 --seed 7 --system bba --out <stem>

Smoke, 300 deals of bank 22 at skip 0: 2,761 of the `american` reader's
3,126 rows pair (same hand, auction-context block and DD table), 83% of
paired rows move at least one inference float, the compact block is
identical on every pair, and the teacher's argmax agrees on 97.7%. Not a
Phase 0 read, only the check that the channel moves.

**Run.** One re-walk of the fleet's 188 chunk windows under each reader
(`feature-retrain.sh`'s stage 1, no DD, ≈ 10 min on 20 cores per domain),
same bank, same seeds. The two corpora are paired row for row where the
auction is the same and diverge after the first call the two books read
differently.

**Read, three numbers.**

1. **Column drift** of the inference block: per column, mean and sd under
   each reader on the paired rows; the columns that move most are the
   bands BBA reads differently from us (`1NT` forcing 5–12, the 12+ 2/1).
2. **A domain discriminator**: logistic regression on the inference block
   alone (frozen features, one matmul and a sigmoid), label = which reader
   produced the row, 10% held out. AUROC near 0.5 says the readings are
   indistinguishable and Phase 1 will read flat; AUROC near 1 says the
   shift is large and the shipped net had every chance to learn a shortcut.
   Report it per phase (constructive, contested, defensive).
3. **Label drift**: the teacher's argmax agreement on paired rows. The
   plan first assumed 100% (same engine, same deal, same prefix), but the
   matched teacher plays EPBot's defaults where the `american` corpus's
   teacher plays `american_card`, so the labels move where the card's
   rows do (97.7% agreement on the smoke). Phase 1 is therefore a
   covariate-shift test with a small, measured label shift on top; report
   the agreement per phase beside the AUROC.

**Stop rule.** None; Phase 0 cannot fail, it sizes Phase 1's expectation.
If the discriminator's AUROC is under ~0.6 everywhere, pre-register Phase 1
as "expected flat" and treat a win there as a draw until reseeded.

**Read (2026-11-01, d1323fa3, Compute-Node02).** Both re-walks of the 188
windows, `--feature-version 6`, every shard pinned to `--cell a-off/a-off`
(§8: the stock recipe's cells are refused under `bba`), 20 workers: 6 min
for `american`, 2.5 min for `bba`. Corpora at
`/data/public/jdh8/pons/target/phase0-{american,bba}-chunks`; probe
`scripts/domain-shift-probe.py` (numbers below from its seeded run; the
AUROCs move ±0.01 across samplings), JSON beside the chunks.

| phase | rows `american` / `bba` | paired | paired rows moved | label agreement | AUROC paired / all |
| --- | --- | --- | --- | --- | --- |
| constructive | 1.759M / 1.747M | 1.577M (89.7%) | 72.4% | 95.4% | **0.81** / 0.84 |
| contested | 2.964M / 2.950M | 2.493M (84.1%) | 91.7% | 98.6% | **0.89** / 0.90 |

1. **Column drift.** The columns that move most are the point ceilings,
   and `bba` always reads them tighter. In constructive auctions RHO's
   `points.max` drops from 24.2 to 21.1 (mean |Δ| 4.5 HCP per paired row)
   and LHO's from 29.5 to 27.5. In contested auctions partner's drops from
   23.4 to 19.1 (|Δ| 5.8) and our own from 25.8 to 23.0. Next come the
   floors: under `bba` every support-point floor rises to the HCP floor
   (partner 4.2 → 5.4 constructive). Length ceilings move about 1 card,
   in the same direction.
2. **The discriminator is high, and one gauge pair carries it.** The paired
   AUROC is 0.81 constructive and 0.89 contested, so it is well clear of
   the 0.6 line. Fitting each column family alone shows where the
   separation comes from:

   | paired AUROC on | constructive | contested |
   | --- | --- | --- |
   | lengths (32) | 0.59 | 0.62 |
   | points (8) | 0.59 | 0.66 |
   | support points (32) | 0.59 | 0.63 |
   | lengths + points (40) | 0.62 | 0.69 |
   | lengths + support (64) | 0.65 | 0.66 |
   | **points + support (40)** | **0.82** | **0.88** |
   | all 72 | 0.81 | 0.89 |

   Points plus support points reproduces the whole block; every other
   family stays near 0.6. The separating direction is the contrast
   between the two gauges. The walked bands are HCP-only
   (`hcp(lo..=hi)` in `examples/common/bba_book.rs`), so under `bba`
   `Hcp::project` and `canonicalize` floor `points` and all four
   `support_points` at the same HCP floor. Under `american`, the
   fit-showing rules give support points a range of their own. On the
   first 20 `uniform` chunks, 74% of `american` rows have some seat whose
   support floor differs from its points floor, against 46% of `bba`
   rows. Without that contrast the shift is modest: 0.62 / 0.69 on
   lengths plus points.
3. **Label drift is small.** The teacher's argmax agrees on 95.4% of
   paired constructive rows and 98.6% of paired contested rows. The card
   change (EPBot defaults against `american_card`) moves constructive
   labels more than contested ones.

**What it sizes.** Phase 1 is not pre-registered as flat. The shift is
large, but most of it sits in one structural feature: support points are
missing from the `bba` reading. What the shipped net does with a raise
when no seat shows support points is the first place to look if `bba-d`
beats the shipped floor on `bba`. A band-content shift on its own (the
tighter point ceilings) would have read about 0.6 / 0.7, close to the
flat line in constructive auctions.

### Phase 1 — the matched retrain (the owed experiment)

**Build (done 2026-11-01).** `--our-floor bba-file` is `bba` with the
net a logit mean over `PONS_FLOOR_WEIGHTS`, as `american-file` is to
`american`: `bba_book::Chooser::File`, on every walked node and on both
contested roots (`BbaBook::system_with` takes the blobs directly, for
tests). `bba-gen` takes it as a name; `bba-decompose --our-floor` gained
`american-file` and `bba-file`, so both file arms replay. `ben-gen` still
takes only `american` and `american-file`; Phase 1 runs against BBA.
The runner is `scripts/ab-portable-judgement.sh` (the four arms below,
the four paired diffs, the four decomposes); it pins each file arm's
blobs like `ab-floor-file.sh`. Proof: a test that `bba-file` over the
eight shipped blobs gives `bba`'s logits bit for bit (a single blob fails
it at `1♠ -`), and a 64-board smoke of the runner with both file arms on
the shipped blobs: both file-vs-shipped diffs read 0 on every board,
every arm replays at 100.00%.

**Train.** Two K = 8 ensembles under arm R's recipe and seeds 9–16
(the reseed row of the 2026-10-31 retrain, [reseed-gap-handoff.md §5](reseed-gap-handoff.md#5-the-parallel-cpu-track-the-relabel-pilot-two-arms)),
on dl02's GPUs, one trainer at a time:

- `american-d`: distillation only (BBA's softmax, no relabel) on the
  `american`-read corpus. **This is the control**, not the shipped floor:
  the shipped K = 8 carries the M32 relabel and a lucky draw, and neither
  is available to the `bba` arm yet.
- `bba-d`: the same recipe on the `bba`-read corpus.

**Corpora (built 2026-11-01).** The two Phase 0 re-walks, cut by
`dump-teacher --cut 0` (the chunks concatenated, teacher labels kept) into
20 stems each at
`/data/public/jdh8/pons/target/phase0-{american,bba}-corpus` (4.72M /
4.70M rows, `SHA256SUMS` beside them; both `a-off/a-off`, §8). They go to
dl02 once its BEN extension is done. Arm R's recipe is Plan B's flags in the
shipped stem order with `--row-split`, which `feature-retrain.sh`'s
`ls` order does not give (reseed-gap-handoff.md §2):

    STEMS=$(for s in uniform-0 uniform-1 uniform-2 uniform-3 uniform-4 \
        uniform-5 uniform-6 uniform-7 enriched-0 enriched-1 enriched-2 \
        enriched-3 axis-0004 axis-1000 axis-2000 axis-0002 axis-4000 \
        axis-0800 axis-8000 axis-0020; do printf -- '--data %s/%s ' "$D" "$s"; done)
    pons-trainer --arch mlp --cuda --device-index $GPU --hidden 256 \
        --epochs 300 --lr 0.001 --wd 0 --batch 4096 --val-frac 0.10 \
        --dd-weight 0 --row-split --init-seed $SEED $STEMS --weights-out $N/seed$SEED

then `scripts/fold-constant-inputs.py` per net against the same stems.

**Run.** `scripts/ab-bba-system.sh` extended with the two file-loaded arms,
fresh `SEED_BASE`, the two vulnerability cells bba-system.md used (`none`,
`both`), 204,800 boards per arm per cell, both scorers, both decomposes,
paired `ab-dump-diff`. Arms: `american` (shipped), `american-d`, `bba`
(shipped net), `bba-d`. Every arm replays at 100.00% in `bba-decompose`.

**Read.** Let G(arm) be the arm's plain loss to BBA. The judgement-transfer
number is

    T = [G(bba-d) − G(american-d)] − [G(bba) − G(american)]

i.e. how much of the `bba` arm's extra loss a matched chooser gives back.

| read | meaning | next |
| --- | --- | --- |
| `bba-d` − `bba` is a win ≥ 0.1 and T recovers most of −0.37 / −0.46 | the chooser was the problem; judgement ports with a retrain | Phase 2 |
| `bba-d` − `bba` flat, inside the ≈ 0.01 draw noise | the agreements are the problem; our chooser already read BBA's bands as well as it could | Phase 2 still runs for the held-out read, but the union row is the only one with a chance; §4 of bba-system.md's mining stays closed |
| `american-d` − `american` loses more than ≈ 0.05 | the distillation-only control is too far below shipped to be the baseline | add the M32 transplant (D1) to both arms and re-read; no verdict from this lap |

**Stop rule.** The second row with Phase 0's AUROC under 0.6: the readings
hardly moved and the chooser could not have done better. Then the gap vs
BBA is BBA's bilans judging *its own* bands, which is search-priced
judgement, and the plan hands over to the search road
([ben-gap-campaign.md](ben-gap-campaign.md)).

**Read (2026-11-02, 3a084f04, dl02): row 3, no verdict.** Driver and
nets are in `/mnt/ssd-data/jdh8/pons-phase1/`, and the A/B is in
`ab-results/portable-judgement/` (`SEED_BASE=1793603525228576315`,
204,800 boards per arm per cell). The two K = 8 trained side by side in
40 min. The first A/B attempt died when 5 of the 32 `bba` workers were
OOM-killed (≈ 1.6 GB each on a 61 GiB box shared with other users). The
rerun used `PAR=16` on the same seed base (`scripts/bba-gen-parallel.sh`)
and kept the two verified `american` arms. Every arm replays at 100.00%.
The shipped arms (`american`, `bba`) run the BBA-relabel K = 8 shipped
at 3a084f04.

Every arm bids against BBA on the same deals, and the numbers below score
those same games in two ways. The **paired diff** (`ab-dump-diff`) IMPs one
arm's score directly against the other's on each board. **G**
(`bba-decompose`) IMPs each arm against BBA's own result at the other table,
and is quoted per arm. A difference of two G values is not the paired diff:
IMPs are not linear, so the two scales disagree (about 2× here).

Paired diff, IMPs/board, plain DD / PD (95% half-width on plain):

| diff | none | both | fired |
| --- | --- | --- | --- |
| `american-d` − `american` | **−0.083** ±0.009 / −0.098 | **−0.120** ±0.011 / −0.150 | 13.0% / 11.2% |
| `bba-d` − `bba` | −0.003 ±0.011 / −0.006 | −0.011 ±0.014 / −0.029 | 19.4% / 18.4% |
| `bba` − `american` | −0.192 ±0.016 / −0.193 | −0.251 ±0.020 / −0.233 | 39.6% / 38.0% |
| `bba-d` − `american-d` | −0.114 ±0.015 / −0.100 | −0.142 ±0.019 / −0.111 | 36.9% / 35.7% |

G against BBA (`decompose.<arm>.txt`), plain / PD:

| arm | none | both |
| --- | --- | --- |
| `american` | −0.180 / −0.016 | −0.183 / −0.039 |
| `american-d` | −0.355 / −0.219 | −0.411 / −0.330 |
| `bba` | −0.564 / −0.411 | −0.666 / −0.502 |
| `bba-d` | −0.576 / −0.430 | −0.699 / −0.565 |

1. **Row 3 fires.** On the same book, the control loses 0.08–0.12 a board
   to the shipped floor in the paired diff, and 0.18–0.23 in G. The
   threshold was ≈ 0.05, so this lap gives no verdict.
2. **T is positive, but the control's collapse supplies it.** T against
   BBA is +0.163 / +0.195 plain (+0.184 / +0.227 PD), and the paired
   form is +0.080 / +0.109. Almost all of it is `american-d` − `american`;
   `bba-d` sits at or below `bba` in every reading. T as defined charges
   the control's own loss to the `bba` side, so it measures nothing until
   the control stands near shipped.
3. **Row 2's reading, if the control held.** `bba-d` − `bba` is inside
   the draw noise in both cells (and slightly negative against BBA). The
   same recipe costs 0.08–0.12 under `american` but ≈ 0.01 under `bba`,
   which fits §8's count: the net serves 48% of decisions under
   `american` and 19% under `bba`. A weaker chooser costs less where it
   chooses less. This leans toward "the agreements are the problem", but
   it is not read as that until a control stands near shipped.
4. **What the control lacks.** Against the shipped K = 8, `american-d`
   differs in three ways. (a) Labels: the shipped cut carries the
   BBA-opponent relabel (380,714 labels moved; that relabel alone won
   +0.044 / +0.050 over the M32-labelled K = 8, reseed-gap-handoff.md
   §5). (b) Corpus: 4.72M rows pinned to `a-off/a-off`, against 6.78M
   rows of the stock cell mix (§8). (c) Draw: the shipped reseed is the
   better of two draws by ≈ 0.006. Only (a) is what D1's transplant
   restores.

**Attribution arm `american-s` (2026-11-02, dl02): the labels
are the whole gap.** This is a fifth arm on the same deals and on
Phase 1's 3a084f04 binaries (`AMERICAN_S` in the runner; driver and nets in
`/mnt/ssd-data/jdh8/pons-phase1/stock/`). It is a distillation-only K = 8
trained on the shipped cut's own walk: `dump-teacher --cut 0` of the HEAD
relabel chunks, 6,781,198 rows of the stock cell mix, with teacher labels.
It uses the same stem order, seeds 9–16, trainer and 32-column fold. So it
is the shipped K = 8 with the BBA-opponent relabel taken out, and nothing
else changed. It replays at 100.00%. Paired diff, plain / PD
(half-width on plain):

| diff | none | both |
| --- | --- | --- |
| `american-s` − `american` | **−0.109** ±0.009 / −0.108 | **−0.151** ±0.011 / −0.171 |
| `american-s` − `american-d` | **−0.026** ±0.006 / −0.010 | **−0.032** ±0.008 / −0.022 |

On G, `american-s` reads −0.403 / −0.470 plain (−0.234 / −0.366 PD).

5. **The relabel is worth 0.11–0.15 a board on its own recipe.**
   `american-s` − `american` isolates it. The labels also move the
   training trajectory, so the draw (≈ 0.006–0.012) is inside this
   number, not added to it. This is the first measure of the
   BBA-opponent relabel against pure distillation. The +0.044 / +0.050
   of reseed-gap-handoff.md §5 measured it against the M32 labels.
6. **The corpus is not the gap; the pinned corpus is better.** The
   stock cell mix, with 44% more rows, loses 0.026 / 0.032 to the pinned
   `a-off/a-off` corpus under the same labels. Point 4's (b) therefore
   works in the control's favour, and all of its gap, and more, is (a).
   *Why* the pinned corpus wins is not isolated. The two corpora differ
   in three ways at once:
   - **Cells.** `a-off/a-off` is the served cell (`american()` plays
     plain RKCB). The stock mix also trains on kickback cells
     (uniform, enriched) and flipped regime bits (axis), which the A/B
     never serves.
   - **Replay rows.** The stock enriched shards run `--replay`, and the
     pinned ones dropped it (§8).
   - **Row count.** 6.78M against 4.72M.

   The code version is ruled out. The walks are at 54659d41 (stock) and
   d1323fa3 (pinned), and the only `src/` commit between them, 342b7f11,
   is byte-identical for the default system by its own proof. On
   `uniform-0`, 33.1% of the pinned rows occur byte for byte in the stock
   corpus. That is the one stock table in three whose cell is
   `a-off/a-off`, walked identically.
7. **For D1.** The table's third row asked for a control near shipped.
   On this evidence, only labels can put it there, so the transplant is
   the right lever. But its value depends entirely on how many of the
   380,714 moved labels land on the pinned chunks (§8), and on the `bba`
   side they are priced under `american`'s continuations.
8. **A possible KR1 lever, not measured.** If 6 holds under relabel
   labels too, the shipped recipe on an `a-off/a-off`-only relabel cut
   could beat shipped by ≈ 0.03. That has not been tested. The 0.03 is
   a distillation-label number, the relabel may interact with the cell
   mix, and 6 leaves cells, replay rows and row count confounded. A
   clean test pins one factor at a time on one walk: `a-off/a-off` with
   `--replay` kept, then the same with the replay rows dropped.
   Recorded here, not in next-steps, until jdh8 rules on it.

D1 is reopened for jdh8 (§7). Two premises of its row have moved, and
both are flagged in §8.

**D1 decided (jdh8, 2026-11-03): relabel both pinned walks under their
own reader.** The transplant is skipped. Its coverage hinges on the
cell-refusal rule, and on the `bba` side its labels would be priced under
`american`'s continuations. Both Phase 0 walks (the 188 windows,
`a-off/a-off`, `--replay` dropped) are instead relabelled as the shipped
relabel-bba was: `--rollout-opponent bba`, M = 32 (64 layouts), with
`.dd` seeded from the HEAD relabel, and each arm is cut at M = 32. The
`american` walk mostly reuses those tables, so its cost is BBA's serial
bid-out. The `bba` walk diverges, so it solves most of its own layouts.
Before launch, one defect was found and fixed: the rollout told BBA the
American card under `--system bba` (CHANGELOG, Fixed). Run on
Compute-Node02 with poker drained, at fe5eae10 plus that fix
(`/data/public/jdh8/pons/target/phase1-relabel/{driver.sh,recipe,driver.log}`).
The `american` arm has 64 workers × 1 solver thread and the `bba` arm
48 × 4, which is the box's 256-thread budget. Next come two K = 8
(`american-r`, `bba-r`) under arm R's recipe on the two cuts, and Phase
1's A/B re-read with those two as the arms. Row 3 then asks whether
`american-r` sits near shipped.

**Read (2026-11-04, f24c6656, dl02): the chooser is a quarter of the
gap.** Relabel done 2026-11-04 (slurm 626, 188/188 chunks per arm, cut
at M = 32; cuts and sha256 in `/mnt/ssd-data/jdh8/pons-phase1/{american,bba}-r/`).
`american-r` and `bba-r` were trained with arm R's recipe, seeds 9–16,
and folded (`pons-phase1/driver-r.sh`, 40 min). They went into the
`american-d` / `bba-d` slots of `ab-results/portable-judgement-r/`
(`SEED_BASE=1793777022241251944`, 204,800 boards per arm per cell,
`PAR=16`, `BBA_BOOK` 2026-10-29-8fb078de-none). Every arm replays at
100.00%. Paired diff, IMPs/board, plain DD / PD (95% half-width on
plain):

| diff | none | both | fired |
| --- | --- | --- | --- |
| `american-r` − `american` | −0.002 ±0.007 / +0.006 | −0.004 ±0.009 / +0.008 | 8.8% / 7.2% |
| `bba-r` − `bba` | **+0.045** ±0.011 / +0.049 | **+0.036** ±0.014 / +0.021 | |
| `bba` − `american` | −0.200 ±0.016 / −0.204 | −0.242 ±0.020 / −0.235 | |
| `bba-r` − `american-r` | −0.152 ±0.016 / −0.159 | −0.203 ±0.020 / −0.220 | 39.7% / 38.0% |

G against BBA, plain / PD:

| arm | none | both |
| --- | --- | --- |
| `american` | −0.180 / −0.020 | −0.191 / −0.036 |
| `american-r` | −0.191 / −0.015 | −0.209 / −0.029 |
| `bba` | −0.565 / −0.414 | −0.660 / −0.489 |
| `bba-r` | −0.474 / −0.306 | −0.576 / −0.414 |

1. **Row 3 is cleared.** The control is within noise of shipped in
   both cells and both scorers. (Its lower bounds, −0.010 / −0.012, do
   not prove non-inferiority, but nothing here ships `american-r`.)
   The 2026-11-02 control lost 0.08–0.12, so the relabel under the
   control's own reader closes that gap, as points 5 and 7 predicted.
2. **T = +0.103 / +0.102 plain on G** (+0.103 / +0.069 PD). The paired
   form is +0.048 / +0.038. The `bba` arm's extra loss on G is
   0.386 / 0.469, so a matched chooser gives back **22–27%** of it.
3. **The read falls between rows 1 and 2.** `bba-r` − `bba` is a real
   win: both cells clear zero on plain, and PD agrees. But it is under
   row 1's 0.1, and T recovers a quarter, not "most". The table has no
   row for this (§8). The reading: the chooser is part of the problem,
   and the agreements are the larger part. `bba-r` still loses
   0.15–0.20 a board to `american-r` in the paired diff.
4. **Next: Phase 2.** Rows 1 and 2 both lead there, so the middle does
   too. The stop rule does not fire, since it needs row 2.
5. **Not decided: a `bba`-own net.** `bba-r` beats the net that `bba`
   serves today (the shipped `american` K = 8) by 0.036–0.045. Giving
   the `bba` system its own default net is a KR1 change to a research
   system and is jdh8's call (§7 D6). Default: no change; `bba-r` stays
   at `pons-phase1/bba-r/nets-folded`.

### Phase 2 — one chooser, two books, a third never seen

**Build.** The `bba-pc` domain: `scripts/bba-book.sh` under the Polish
card (D2 names the card), a reach corpus under the same card, the walk
registered beside the `--card none` run; `dump-teacher --system bba-pc
--card <polish>` so the teacher plays what the reader reads;
`BBA_BOOK=` already selects the walk, so the serving side needs only the
name. The union corpus is a concatenation; the trainer reads stems in the
unshuffled order the reseed handoff fixed, so the order of the two domains
is itself a recipe choice (D3).

**Train.** `union-d` on `american` ∪ `bba`, same recipe, K = 8. Optionally
`union-d-nobit` with the compact block zeroed (the domain label removed),
to read whether the net needs to be *told* the book or can infer it from the
readings; one more ensemble, run only if `union-d` reads something.

**Run.** The six cells of §3's table that are not yet read, all against BBA
on the same deals: `american-d`, `bba-d`, `union-d` each served on
`american`, `bba`, `bba-pc`. Nine arm-domain pairs, three of them from
Phase 1; six new, two cells each.

**Read.**

- **Transfer symmetry**: `bba-d` on `american` against `american-d` on
  `american`. If it loses about what the shipped net lost on `bba`, the
  shift is symmetric and the cost is data, not the direction.
- **The union's price**: `union-d` on `american` against `american-d` on
  `american`, and likewise on `bba`. Non-inferior on both says a chooser
  can hold two books without paying on either. A loss on both says the
  width or the recipe is the limit (D4).
- **The held-out read, the plan's verdict**: `union-d` on `bba-pc` against
  `american-d` on `bba-pc` and `bba-d` on `bba-pc`. If the union chooser
  loses least on a book it never trained on, judgement generalises across
  books with more books in training; if all three tie, the inference
  channel carries what matters and one book's corpus is as good as two.
  Either way the number is the answer to question 1 at this instrument's
  resolution.

**Stop rule.** If `union-d` loses on *both* training domains against the
single-domain nets by more than the draw noise, the net is capacity-bound
or the recipe is wrong for mixed stems, and nothing is learned about
transfer until that is fixed (D4). Do not read the held-out cell in that
case.

**Build record (2026-11-04).**

- **The `pc` walk**: `ab-results/bba-book/2026-11-04-93ec168e-pc`, `--card 2
  --vuls none --corpus <96 × 1,250 self-play boards, card 2> --min-reach 2`,
  1,118 shards, 71 s on dl02; 56,840 nodes, the same shape as the `none`
  walk's 56,854. The root reads as WJ: `1♣(11+)[Polish 1C]!`, a
  precision-style `2♣`, Multi `2♦`, Polish two-suiters on `2♥`/`2♠`. The
  walker's `--card` now takes a bare EPBot system id (D2's "once it accepts
  one"), and `bba_book::card()` discloses the walk's system, read off its
  `RUN`; a `none` walk still says 2/1, byte for byte.
- **No `pc` corpus.** The held-out book needs a walk, not a dump, since no
  net trains on it. `dump-teacher --system bba-pc` is not built.
- **Opponents on `pc` play WJ, not 2/1** (§8, against trap 5).
- **`union-r`**: K = 8, seeds 9–16, arm R's recipe, on `american-r`'s 20
  cut stems then `bba-r`'s (D3), one trainer at a time on the 4090 (9.9 GB
  RSS, 9.2 GB GPU, ≈ 12 min a seed), scan-folded against all 40 stems.
  Driver `dl02:/mnt/ssd-data/jdh8/pons-phase2/driver.sh`.
- **The A/B**: `scripts/ab-portable-judgement-2.sh`, nine arms
  `<net>-<domain>` (nets `american-d` = `american-r`, `bba-d` = `bba-r`,
  `union-d` = `union-r`; domains `am`, `bba`, `pc`), two cells, 204,800
  boards, a fresh `SEED_BASE`, in `ab-results/portable-judgement-2`. All nine
  are re-run on one deal set rather than borrowing Phase 1's arms, so every
  read is paired. Seven diff pairs, one per read above, and a
  `decompose.<arm>.md` per arm for G. A 64-board smoke replayed 100.00% on
  all nine, and a `union-d` stand-in equal to `bba-d` came out
  byte-identical (0 fired).

**Read (2026-11-05).** The A/B ended 2026-11-04 23:19 +08
(`SEED_BASE=1793799266423355794`, 204,800 boards per arm per cell,
`PAR=16` after an OOM at `PAR` unset). All nine arms and seven diffs are
in. The script then aborted at `decompose american-d-pc` on a replay
mismatch (§8), so the `pc` arms have no G. Paired diff, IMPs/board,
plain DD (95% half-width) / PD:

| diff | served on | none | both | fired |
| --- | --- | --- | --- | --- |
| `union-d` − `american-d` | `american` | −0.004 ±0.007 / **−0.016** | −0.004 ±0.009 / **−0.031** | 8.5% / 7.0% |
| `union-d` − `bba-d` | `bba` | −0.007 ±0.007 / +0.024 | −0.005 ±0.009 / +0.037 | 8.2% / 7.5% |
| `bba-d` − `american-d` | `american` | **−0.136** ±0.010 / −0.104 | **−0.155** ±0.011 / −0.141 | 15.4% / 12.9% |
| `american-d` − `bba-d` | `bba` | **−0.052** ±0.011 / −0.039 | **−0.052** ±0.014 / −0.019 | 18.5% / 17.6% |
| `bba-d` − `american-d` | `pc` | **+0.050** ±0.011 / +0.061 | **+0.058** ±0.014 / +0.058 | 18.5% / 17.6% |
| `union-d` − `american-d` | `pc` | **+0.048** ±0.010 / +0.073 | **+0.059** ±0.013 / +0.077 | 16.3% / 15.6% |
| `union-d` − `bba-d` | `pc` | −0.002 ±0.008 / +0.012 | −0.000 ±0.010 / +0.019 | 10.6% / 9.8% |

G against BBA, plain / PD:

| arm | `american` none | `american` both | `bba` none | `bba` both |
| --- | --- | --- | --- | --- |
| `american-d` | −0.177 / −0.001 | −0.199 / −0.030 | −0.591 / −0.399 | −0.700 / −0.487 |
| `bba-d` | −0.451 / −0.222 | −0.517 / −0.316 | −0.482 / −0.322 | −0.591 / −0.440 |
| `union-d` | −0.187 / −0.039 | −0.214 / −0.098 | −0.497 / −0.275 | −0.599 / −0.367 |

1. **The stop rule does not fire.** `union-d` is a plain wash on both
   training books, so D4 is not triggered. It is not proven
   non-inferior: the lower bounds, −0.011 / −0.012 on `american` and
   −0.014 / −0.014 on `bba`, reach past −δ. PD splits: a loss on
   `american` and a win on `bba`.
2. **Transfer is asymmetric.** `bba-d` served on `american` loses
   0.136–0.155, which is 2.6–3× what `american-d` loses served on `bba`
   (0.052 in both cells). G says the same: 0.27 / 0.32 against 0.11 /
   0.11. The cost of a mismatch depends on its direction, so it is not
   only "how much data". Not diagnosed. One candidate: `american` keeps
   authored nodes and readings that a `bba`-trained net never saw, while
   under `bba` nearly every decision is net-served (Phase 1).
3. **The union's price: a plain wash on both books, a PD loss on
   `american`.** One chooser holds two books at plain resolution. Its
   PD G on `american` is 0.04 / 0.07 worse than `american-d`'s, so the
   union doubles or gets doubled differently there. That is the PD
   guard's signal, recorded, not a veto on a research arm.
4. **The held-out read fits neither pre-registered branch** (§8). On
   `pc`, `union-d` ties `bba-d` (plain within ±0.010, PD +0.012 /
   +0.019), and both beat `american-d` by about 0.05 on plain and PD. A
   second training book buys nothing over `bba-d` on the held-out book,
   and the three nets do not tie. What ports to Polish Club is training
   on a BBA-compiled book, not training on more books. Caveat: `pc` is
   itself a BBA walk, so it is a new system in a familiar book format,
   not a foreign book. Question 1's answer at this resolution: judgement
   ports between two systems compiled by one engine. The cost against
   a `pc`-matched net is unmeasured, because there is no `pc` corpus.
5. **Phase 3 does not fire.** Its trigger is a held-out loss, and
   `union-d` loses to neither net. `union-d-nobit` ("only if `union-d`
   reads something") is not run: on the held-out book it reads the same
   as `bba-d`. Default: not run. One ensemble to overturn.
6. **D6 is back with jdh8.** "`union-d` holds both domains" is true only
   as a plain wash, with a PD loss on `american`. Proposed default:
   `bba-r`, after a four-cell run (D5).

#### D6 read — `bba` gets its own net (2026-11-05)

`scripts/ab-bba-own-net.sh ab-results/bba-own-net` compares two arms on
paired deals against BBA, which is told the arm's own card. `bba` serves the
shipped K = 8. `bba-own` is `bba-file` over the eight `bba-r` blobs. Built
at `13560068`, `BBA_BOOK=ab-results/bba-book/2026-10-29-8fb078de-none`,
`SEED_BASE` 1793836935556905108, 204,800 boards per cell. The delta is
`bba-own` − `bba` in IMPs/board, ±95% CI:

| vul | plain DD | PD | fired |
| --- | --- | --- | --- |
| none | +0.0352 ±0.0110 | +0.0397 ±0.0123 | 18.7% |
| ns | +0.0453 ±0.0132 | +0.0406 ±0.0149 | 18.0% |
| ew | +0.0146 ±0.0122 | +0.0139 ±0.0137 | 18.5% |
| both | +0.0421 ±0.0138 | +0.0332 ±0.0154 | 17.8% |

A win in all four cells on both scorers. Every lower bound is above 0, and
the thinnest is plain `ew` at +0.0024. In plain G against BBA, `bba` reads
−0.582 / −0.523 / −0.515 / −0.688 and `bba-own` reads −0.496 / −0.452 /
−0.430 / −0.592. Divergence from BBA's auction falls from about 71% to 62%.
The worst losing boards are slam tries that stop at the five level where
the control bid `4NT` and the slam, plus a few competitive doubles. No
trigger is over-broad: the whole floor moves, by design.

**Shipped.** `--our-floor bba` serves `bba-r`
(`examples/common/bba_book.rs` `bba_r`, the blobs in
`examples/common/weights/`, kept out of the published crate). `bba-ladder`
keeps the shipped net. Its constructive nodes are the instinct ladder, and
it was not measured with `bba-r`. Default: unchanged, one A/B to overturn.
Phase 1's `bba` numbers above are against the shipped net. Every later
`bba` read is against `bba-r`.

### Phase 3 — if, and only if, the held-out read is a loss

The representation levers, each a separate knob and a separate retrain,
ranked by what they remove from §1's table:

1. **Zero the compact block for all domains** (`union-d-nobit`, above):
   removes the domain label; the net must infer the regime from the
   readings. Cheapest, and a necessary condition for an *unseen* book to be
   bid well, since an unseen book has no regime bits.
2. **Replace the inference hull with the sampled projection**
   ([sampled-projection.md](ai-bidder/sampled-projection.md)): the net
   reads a draw of consistent hands rather than the hull's corners, which is
   BBA's Stage-1 input. The exact-posterior lane is parked
   ([exact-posterior.md §5 "Parked"](exact-posterior.md#parked--the-whole-lane-jdh8-2026-10-02))
   and stays so; the *sampled* projection is the one that ships today.
3. **Meaning encoder** (M5.3, [ai-bidder/plan.md](ai-bidder/plan.md)): each
   prior call's reading as a vector the net attends over, the long-run
   design. Phase 2's union corpus is the second-system corpus M5.3 has
   waited on since 2026-06; this plan produces it as a by-product.

### Phase 4 — the arithmetic half, made a pipeline step

Independent of Phases 1–3 and owed anyway. The collar is "calibrated to one
floor's distribution" and so to one book's: under BBA's book the floor
proposes different heights and the same thresholds pass a different
doubled-contract rate.

**Build.** A calibration pass in `feature-retrain.sh` after the fold: run
the demotion census (`bba-decompose --demotion-census`) on a fixed 20,000
boards under the new floor and the serving book, print the doubled rate and
the veto rate, and fit `PASS_DEMOTION` as `3·T` nats from the net's
temperature (the restatement already in [ai-bidder/plan.md](ai-bidder/plan.md)
arm 1) so the collar is a function of the floor, not a constant.

**Read.** The doubled-contract rate on divergent boards per (floor, book)
cell, against the 11.78% the v6 floor was tuned to. A cell more than ~2
points off is re-tuned before its A/B is read; otherwise the A/B measures
the pair.

Everything exact (q table, `break_even`, par) is already book-free and is
asserted so by construction; no work.

## 5. Traps, each with its default

1. **Reading the shipped floor as the control** (Phase 1). The shipped
   K = 8 is relabelled and a lucky draw (≈ 0.012 over its own reseed). The
   `bba` arm cannot have either, so the control is `american-d`, the same
   recipe without them. A win over `american-d` that is a loss to shipped
   ships nothing and still answers the question.
2. **Draw noise** ≈ 0.006–0.012 IMPs/board for K = 8 vs K = 8. Nothing
   under 0.02 is read as a recipe effect without a second seed set.
3. **The reach gate.** The `bba` walk is exhaustive to depth 4 and
   reach-gated below; off-walk nodes are the plain floor on the natural
   walk, so a `bba`-trained net also learns `american`'s readings there.
   The corpus sidecar should count walked vs off-walk rows (the census's
   `walked` column, 78.4% of decisions); if Phase 1 reads flat, check that
   the loss is not concentrated off-walk before concluding.
4. **Polish Club's walk under the glibc loader** on this box: every shard
   needs `PROBE_BBA_BOOK_LOADER`; the dump needs `DUMP_TEACHER` under the
   loader. On dl02 neither applies.
5. **Opponents' readings.** In every A/B the opponents are BBA on its 2/1
   defaults; our reading of *their* calls is the declared natural walk and
   does not change with our book. Under `bba-pc` our opponents are still
   2/1 BBA; the Polish domain is ours only. Fine for a transfer read, and
   the deviation panel's row 2 is the reference if a Polish *opponent* is
   ever wanted. **Superseded for the A/B 2026-11-04**: the `bba` floors read
   their calls off the walk, so `pc` arms face WJ (§8).
6. **The BEN guard** applies only to a ship. Nothing in Phases 0–2 ships;
   the union chooser becomes a ship candidate for the default floor only if
   it is non-inferior on `american` under the house rule (plain verdict, PD
   guard, [measurement.md "Which end is the verdict"](measurement.md)), and
   then it takes the 51,200-board BEN extension like any floor.
7. **Trainer memory.** Two 368-wide corpora concatenated are ≈ 2× the v6
   stems; one trainer at a time on dl02 (the v9 lesson, 416 wide peaked at
   2 × 13 GB).

## 6. Cost

| step | wall | where |
| --- | --- | --- |
| Phase 0 build (`--system bba`, sidecar) | half a day | dl02 |
| Phase 0 re-walks, two domains, no DD | 6 + 2.5 min on 20 workers (measured) | Compute-Node02 |
| Phase 0 probe (numpy, logistic regressions) | 25 s (measured) | Compute-Node02 |
| Phase 1 build (`bba-file` chooser, runner arms) | done 2026-11-01 | Compute-Node02 |
| Phase 1 training, two K = 8, one trainer at a time | 2 × 8 × ≈ 10 min | dl02 GPUs |
| Phase 1 A/B, four arms × two cells, both scorers, decomposes | ≈ 40 min | dl02, or here with the box |
| Phase 2 walk + reach corpus (Polish card) | ≈ 1 min | either |
| Phase 2 third corpus + union training | 10 min + 8 × ≈ 20 min | dl02 |
| Phase 2 A/B, six new arm-domain pairs × two cells | ≈ 1 h | dl02, or here |
| Phase 4 calibration pass | a day to build, minutes to run | dl02 |

No double-dummy solving for labels anywhere in Phases 0–2: distillation
labels are BBA's softmax, and the A/Bs solve only the divergent union. The
relabel (29 h per re-price) enters only through D1.

## 7. Decisions owed to jdh8

Taken at their defaults to write the plan.

| # | decision | default | why |
| --- | --- | --- | --- |
| D1 | Phase 1 labels: distillation only, or the M32/BBA-opponent relabel transplanted onto both arms | **distillation only** for the first lap; **decided 2026-11-03: a fresh BBA-opponent relabel of both pinned walks** (§4, after the Phase 1 read) | the transplant's splice key is the net-served slice under `american`'s book; under `bba` nearly every decision is net-served through `choose_by`, so the key covers a different slice. A relabel *under* `bba` is a 29 h re-price per domain and is Phase 1's third row only if the control falls too far below shipped |
| D2 | the third domain's card | **EPBot system 2 (Polish Club) with no `.bbsa`**, walked with the walker's card argument once it accepts a bare system id | `vendor/bba/WJ.bbsa` is cited by `next-steps.md`, `deviation-panel.md` and `dump-teacher`'s help but **is not in the tree** (§8); `cards/American-Basic.bbsa` is the other option and is the weak `basic`-like test |
| D3 | union corpus stem order | `american` stems first, then `bba`, unshuffled | matches the reseed handoff's recipe; the reverse order is one more ensemble if D4 fires |
| D4 | if the union net loses on both training domains | widen once (the v6 widths × 1.5), one ensemble | the cheapest capacity test; the LSTM is parked and stays so |
| D5 | vulnerability cells | two (`none`, `both`) as bba-system.md ran, four for any ship candidate | transfer reads need the pair, a ship needs the house four |
| D6 | give the `bba` system its own default net (`bba-r`, +0.036 / +0.045 over the shared shipped K = 8, §4 Phase 1 read 2026-11-04) | **no change**; the nets stay in `pons-phase1/bba-r/`. **jdh8 2026-11-04: Phase 2 first, revisit after** (if `union-d` holds both domains it is the natural `bba` default; if not, `bba-r` after a four-cell run). **Phase 2 read 2026-11-05: `union-d` holds both only as a plain wash, with a PD loss on `american`; proposed `bba-r` after a four-cell run. **jdh8 2026-11-05: decided, `bba-r` after a four-cell run. SHIPPED 2026-11-05: won 4/4 plain and PD (§4 D6 read)** | `bba` is a research system; a swap moves every later `bba` read, and a ship needs the house four cells (D5) |

## 8. Discrepancies flagged, not resolved

- **Phase 0 promised paired labels it cannot have** (found building it,
  2026-11-01): see read 3 above. Proposed reversible default: keep the
  teacher matched to the reader (the bba-system D5 disclosure) and report
  the label agreement. The other choice, the `bba` corpus's teacher on
  `american_card`, pairs the labels exactly but trains `bba-d` on a
  teacher that plays a different card from the book it reads, which is
  the mismatch §3 exists to remove.
- **`dump-teacher`'s per-board `config`/`reader` are dead under
  `--configured`** (found the same day): both are built only when
  `cells.is_empty()`, but `--configured` with no `--cell` takes
  `DEFAULT_CELLS`, so `cells` is never empty there, and a bare dump builds
  neither. The `--their-system` flag's only effect is inside that branch,
  so it is a no-op too. Proposed reversible default: leave it; a cleanup
  commit deletes the branch and the flag, with a seeded byte-identity of a
  `--configured` and a bare dump as its proof.

- **The fleet recipe cannot be walked under `bba` as written** (found
  running Phase 0, 2026-11-01). `relabel-worker.sh`'s recipe gives some
  shards their own cells: `enriched-*` runs `--cell a-on/a-on --cell
  a-off/a-off --replay` and `axis-*` runs `a-off+<bit>/a-off`.
  `--system bba` refuses both, so Phase 0 walked a `RECIPE_FILE` with the
  same windows, skips, seeds and `--enrich 28:9`, `--replay` dropped, and
  every shard pinned to `--cell a-off/a-off`, under both readers so the
  rows pair. As a result, the `american` corpus here is not the shipped
  corpus's cell mix (uniform shards take `DEFAULT_CELLS` there). Proposed
  reversible default: Phase 1's control `american-d` trains on this
  pinned corpus too, so `american-d` and `bba-d` differ only by reader.
  The shipped floor's mix stays the reference it is compared to.
- **The sidecar's `card` key names the flag, not the teacher's card**
  (found the same day). `dump-teacher` writes `"card": "engine
  defaults"` whenever `--card` is absent, but under `--system american`
  the teacher plays `american_card`. Both Phase 0 corpora therefore say
  "engine defaults", and only `our_system` tells them apart. Proposed
  reversible default: leave it (the key has always meant the flag), and
  read `our_system` as the teacher's card under `--configured`.
- **D1's premise reads the other way in the relabel hook's count**
  (found cutting the Phase 0 corpora, 2026-11-01). D1 says that under
  `bba` "nearly every decision is net-served through `choose_by`". But the
  `.ret` sidecars of the two `--layouts 0` re-walks count 2,259,559
  net-served decisions under `american` (48% of rows) and 899,421 under
  `bba` (19%). So either the hook does not count a `choose_by` delegation
  as net-served, or the bands leave a single admitted call at most nodes.
  Either way, the M32 splice key would cover less of `bba` than of
  `american`, not a different slice of the same size. D1's default
  (distillation only) does not depend on this. Proposed reversible
  default: leave D1 as it is, and measure which reading holds (a census
  of admitted-call counts per walked node) only if D1's transplant row is
  ever run.
- **D1's transplant is no longer "M32", and the pinned corpora refuse
  most of it** (found reading Phase 1, 2026-11-02). The shipped labels
  are now the BBA-opponent relabel cut
  (`dl02:/mnt/ssd-data/jdh8/pons-relabel-bba/cut`), not M32's. Also,
  `splice-labels.py` refuses any pair whose cell differs, and the Phase 1
  chunks are all `a-off/a-off`. The cut's uniform tables cycle
  `DEFAULT_CELLS` (three cells), its axis tables run `a-off+<bit>`, and
  its enriched tables run two cells under `--replay`. By that rule, a
  default splice carries the relabel only on the cut's `a-off/a-off`
  tables, which is roughly a third of the uniform shards and none of the
  axis shards. `--any-identity` carries more of it, at the price of
  labels priced under another cell's continuations. Proposed reversible
  default: before any transplant retrain, run `splice-labels.py
  --validate` per shard on both arms and record the share of the 380,714
  moved labels that land. The Phase 0 chunks it needs are on
  Compute-Node02 only.
- **`vendor/bba/WJ.bbsa` does not exist** (checked 2026-11-01: `vendor/bba`
  holds the engine and `cards/` holds `American.bbsa`, `American-Basic.bbsa`),
  while [next-steps.md](next-steps.md) "Add Polish Club", `deviation-panel.md`
  row 2, `dump-teacher`'s `--card` help and `probe-bba-conventions`'s module
  doc all name it as a machine teacher. Proposed reversible default: D2 walks
  EPBot's built-in system 2 with no card file; if a `.bbsa` is wanted, the
  BBA GUI exports one and it goes under `cards/` like the other two.
- ~~**`dump-teacher --system` accepts only `american`**~~ Resolved
  2026-11-01: it accepts `bba`, and the module doc now says the bands
  read for `bba` as the authored rules do for `american`.
- **The collar re-tune has been owed since 2026-09-03** (the v7 finding) and
  is parked with M5.2's flip plan; Phase 4 takes it out of the LSTM's
  shadow because every retrain, not only a sequence floor, moves the
  distribution it was tuned to.
- **Phase 1's read table has no middle row** (found 2026-11-04). The
  matched retrain read a real win of 0.036–0.045 with T recovering a
  quarter, between row 1 (≥ 0.1, most) and row 2 (flat). Default: read
  it as "chooser part, agreements most" and go to Phase 2, where both
  rows lead; the table is left as written.
- **Trap 5's "under `bba-pc` our opponents are still 2/1 BBA" contradicts
  the floor** (found building Phase 2, 2026-11-04). The walk seats the card
  on all four seats, and the `bba` floors read every call, theirs included,
  off the walk (`bba-gen::declares_european_minors`). Against 2/1 opponents
  the `pc` book would read their natural `1♣` as a Polish club. Under the
  `none` walk the two agree, so Phase 1 never saw this. Default taken:
  `pc` arms play BBA on WJ (`bba-gen --system 2`), so the domain is
  self-consistent and all three nets on `pc` face the same opponents. To
  reverse it, walk with `BbaOracle::with_opponents` (our side WJ, theirs
  2/1) and merge the two seatings by whose turn it is, which is a walker
  change.
- **`bba-decompose` fails replay on the `pc` arms** (found reading Phase 2,
  2026-11-05). `american-d-pc` at none: 129 of 2,031,176 of our calls
  mismatched. The `am`/`bba` arms replay at 100.00%. Suspect: the
  replay rebuilds our partnership from the gen args without what the
  `--system 2` opponents changed. Not diagnosed. Paired diffs come
  straight from `bba-gen` and do not depend on it. Default: no `pc` G.
  Fix the replay before any read that needs one.
- **Phase 2's held-out read has no branch for "union ties one single-book
  net and both beat the other"** (2026-11-05). Default: read as "the
  BBA-compiled training book carries the transfer". The branches are
  left as written.

## 9. Glossary for this plan, in the house framing

- **Covariate shift.** p(x) changes between training and serving; p(y | x)
  does not. Here x is the feature row, y the call. The readings' columns
  keep their meaning across books, so a book change is a covariate shift by
  construction. Phase 0 measures how large.
- **Domain discriminator.** A classifier that predicts the domain from x.
  Its AUROC is a scalar for "how different are the two input
  distributions". Frozen features, one matmul, one sigmoid; the cheapest
  honest number in ML.
- **Domain generalisation.** Train on several domains, test on one never
  seen. The net can only pass if it uses what the domains share (the
  readings' meaning) and not what names them (the compact block). Phase 2's
  held-out cell.
- **Label shift**, for contrast: p(y) changes, p(x | y) does not. Not our
  case, because the teacher's call on a paired row is the same under both
  readers; worth naming because a relabel *would* introduce it (the value
  of a call depends on which book continues after it).
- **Multi-domain training with a domain bit.** Concatenate the corpora and
  give the net a one-hot of the domain. The compact block already is that
  bit for the books we author; `union-d-nobit` is the ablation that asks
  whether the bit is needed.
