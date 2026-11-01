# Portable judgement — does the chooser survive a change of book?

**Status: planned 2026-11-01; Phase 0's build done the same day
(`dump-teacher --system bba`), its re-walks and probe not run.** Each phase
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

### Phase 1 — the matched retrain (the owed experiment)

**Build.** A `bba-file` chooser: `Chooser::Net` with the blobs from
`PONS_FLOOR_WEIGHTS`, mirroring `american-file` (`bba-gen`, `bba-decompose`
and `ben-gen` already take `--our-floor`). One name, one match arm.

**Train.** Two K = 8 ensembles under arm R's recipe and seeds 9–16
(the reseed row of the 2026-10-31 retrain, [reseed-gap-handoff.md §5](reseed-gap-handoff.md#5-the-parallel-cpu-track-the-relabel-pilot-two-arms)),
on dl02's GPUs, one trainer at a time:

- `american-d`: distillation only (BBA's softmax, no relabel) on the
  `american`-read corpus. **This is the control**, not the shipped floor:
  the shipped K = 8 carries the M32 relabel and a lucky draw, and neither
  is available to the `bba` arm yet.
- `bba-d`: the same recipe on the `bba`-read corpus.

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
   ever wanted.
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
| Phase 0 re-walks, two domains, no DD | 2 × 10 min on 20 cores | dl02 |
| Phase 0 probe (pandas + one logistic regression) | minutes | dl02 |
| Phase 1 build (`bba-file` chooser, runner arms) | half a day | dl02 |
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
| D1 | Phase 1 labels: distillation only, or the M32/BBA-opponent relabel transplanted onto both arms | **distillation only** | the transplant's splice key is the net-served slice under `american`'s book; under `bba` nearly every decision is net-served through `choose_by`, so the key covers a different slice. A relabel *under* `bba` is a 29 h re-price per domain and is Phase 1's third row only if the control falls too far below shipped |
| D2 | the third domain's card | **EPBot system 2 (Polish Club) with no `.bbsa`**, walked with the walker's card argument once it accepts a bare system id | `vendor/bba/WJ.bbsa` is cited by `next-steps.md`, `deviation-panel.md` and `dump-teacher`'s help but **is not in the tree** (§8); `cards/American-Basic.bbsa` is the other option and is the weak `basic`-like test |
| D3 | union corpus stem order | `american` stems first, then `bba`, unshuffled | matches the reseed handoff's recipe; the reverse order is one more ensemble if D4 fires |
| D4 | if the union net loses on both training domains | widen once (the v6 widths × 1.5), one ensemble | the cheapest capacity test; the LSTM is parked and stays so |
| D5 | vulnerability cells | two (`none`, `both`) as bba-system.md ran, four for any ship candidate | transfer reads need the pair, a ship needs the house four |

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
