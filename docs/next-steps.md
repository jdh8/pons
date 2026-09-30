# Next-step candidates, ranked by potential IMP gain

**Ranked 2026-09-26** from the two current anchors — BBA shipping arm at
`7e0bc648` (2026-09-26, re-anchored after the floor-rail series;
[bba-gap-campaign.md](bba-gap-campaign.md)) and BEN Tier S at `daa8bf4a`
(2026-09-14, [ben-gap-campaign.md](ben-gap-campaign.md)). **Retrains are
deferred** (jdh8, 2026-09-26): items that need one are owed, not queued —
see [Owed / deferred](#owed--deferred).
Method: pool size on both references × how often the vein has actually
cashed ÷ effort. Figures are IMPs/board unless marked per-fired. Re-rank
after the next re-anchor; each item's ship decision is its own fresh-seed
A/B under [measurement.md](measurement.md).

## 1. Floor junk-action rails — closed 2026-09-26

Five rails shipped, and the R6 re-census at `7e0bc648` found the pool dry
(−0.018 PD/board left, no lane at 0.5k); ledger and close-out in
[floor-rail-campaign.md](floor-rail-campaign.md). Its residue belongs to the
next matched retrain, which must also re-arbitrate every rail (stop
criterion 5). **Item 2 is now the top candidate.**

## 2. RKCB / slam accuracy (Constructive / book / round-2)

**Biggest unworked pool on both references; needs a new lever, not a re-run.**

- Pool: #1 on **both** scorers on the BBA shipping arm at `7e0bc648`
  (−36,474 plain / −42,267 PD ≈ −0.10/board; was #1 PD only at `c3bb94a7`,
  −45,145); vs BEN
  −1.58 plain / −1.75 PD per divergent board, the worst PD/div of the BEN
  top-3, ratio only 1.4–1.5× (i.e. a genuine shared weakness, not a
  BEN-strength artifact).
- Levers on file: the Kickback three-arm A/B
  ([ai-bidder/bba-kickback.md §7](ai-bidder/bba-kickback.md) — blocked on the
  claim guard + grand discipline prerequisites), Stayman major-fit RKCB and
  post-Smolen continuations
  ([one-notrump-constructive.md](one-notrump-constructive.md)), minor-keycard
  grand handling, the parked four-four shape-slam thresholds.
- **New lever, found 2026-09-27:** `slam::rkcb_rows` authors nothing for the
  **answerer** after the asker's `5M` signoff. Wherever the strong hand
  answers (responder asks), the answerer falls to the floor and raises the
  signoff — `5♠ - 6♠`, `5♥ - 7♥` on the forcing-NT jump-shift trace
  ([bidding-options.md](bidding-options.md), `set_forcing_nt_jump_shifts`).
  The row exists as `slam::rkcb_answerer_rows` (2026-09-27), appended beside
  `rkcb_rows` in the jump-shift lane only; wiring it into `rkcb_rows` itself
  moves every default RKCB lane — census how often the shipped book puts the
  strong hand in the answerer's seat, then one default A/B.
  **Censused 2026-09-27, refuted — do not wire it.** `probe-rkcb-answerer`
  (1M uncontested deals × 2 seeds): ~6.6k book signoff windows, the floor
  answerer overrules 510, and passing instead **loses** −0.002 IMPs/board
  (−4.1 plain / −4.0 PD per divergent; seed 2 alike). The floor's raise is
  patching two asker-table holes: `asker_after_5d` signs off on **1** keycard
  though its own doc says ≤2 read partner for three (doc/code discrepancy),
  and `asker_after_5h` signs off on 2 keycards **with** the trump queen —
  four plus the queen, which `asker_after_5s` already bids six on. The fix
  (six on those hands) prices at only +0.0002…+0.0005 plain /
  +0.0001…+0.0004 PD per board (~160 divergent per 1M; the floor already
  reaches six on the rest) — a correctness fix, not a lever. The jump-shift
  lane's wired answerer rows may be paying the same tax; unmeasured.
  **Fix shipped 2026-09-30** (`scripts/ab-rkcb-asker.sh`, off arm = a
  pre-fix worktree build via `BBA_GEN`, `ab-results/rkcb-asker`): won 4/4,
  plain +0.0013 / +0.0019, PD +0.0012 / +0.0018 IMPs/board (none / both),
  73 / 79 fired per 204,800 — four times the census's pricing, which counted
  only the exact-book-node windows the floor had not already patched. The
  1-keycard `5♦` case bids six outright; routing it through the queen relay
  (the combined count is four, as in the 4-keycard branch) is unmeasured.
- **Kickback is not a score lever (2026-09-27,
  [ai-bidder/bba-kickback.md §7.16](ai-bidder/bba-kickback.md)).** On the v6
  floor with the net blinded to the regime bit, its rules diverge on 0.03% of
  boards and price at parity under every instrument (sd-blend +0.00003); the
  claim guard and grand discipline target 16 boards per 200k. What remains of
  item 2 is the slam *decision* — reaching or skipping slam — not the keycard
  mechanics; the next step is a census of the RKCB/slam bucket's divergent
  boards by lane and direction (missed vs overbid) against BBA.
- **Censused 2026-09-30** (`scripts/slam-census.py` on anchor `7e0bc648`; the bucket's 35,820 rows
  joined to their shard auctions; direction is `bba-decompose`'s own tag).
  The slam-flavoured share (either table at 6+ or through 4NT) is 7,402
  rows, −19.6k of the bucket's −40.0k PD; the rest is game-level round-2.
  **Missed slams dominate 3:1**: missed-slam 1,073 rows −12.7k PD +
  missed-grand 182 rows −2.4k, against overbid-slam 373 rows −4.5k; the
  wrong-strain/level slams ("other", 5,754 rows) net **+4.1k** for us. The
  misses are spread thin — no lane above 131 rows (`1♥ - 1♠` −1.6k, `1♦ - 1♠`
  −1.1k, `1♣ - 1♠` −0.9k, `1♦ - 1♥` −0.8k, `1NT - 2♣` −0.7k, `1♠ - 2♣`
  −0.7k) — and the one recurring shape is **our round-2 `3NT` where BBA
  makes a forcing or slam-try call** (261 rows, −3.2k PD; BBA's call there is
  5NT/4NT/4♣/3♣/4♦/3♦/3♥): responder's or opener's 3NT is a settle over a
  strong 5-5 / 6-5 or a fitting 18+ (e.g. `1♥ - 1♠ - 2NT - 3NT` on
  AK8543.7.2.AT943, BBA `3♠` then RKCB; `1♥ - 2♣ - 3♥ - 3NT` on
  x.KQJT96.QJT95.A3, BBA `3♦`). Next in size: opener's 2-level rebid vs
  BBA's jump (`2♦→3♦` 45, `2♥→3♥` 38 rows, −1.0k) and `1♦ - 1M - 2♦` vs
  BBA's `2♣` (44 rows, −0.5k; the measured-wash opt-in). Lever: a
  shape/strength gate that turns the round-2 `3NT` into the forcing bid on
  those hands; ceiling ≈ +0.008/board PD if fully captured, so it needs the
  slam continuation to be authored too, not just the trigger.
  **Classified 2026-09-30:** all 2,551 round-2 `3NT` rows (−4.1k PD) are
  sign-offs — the book has no serious/non-serious `3NT` (after a 2/1 fit
  opener's only calls are `4NT` 15+ or `4M`). By class: `1m - 1M - 2NT - 3NT`
  894 rows −2.2k (no checkback, no forcing `3M`, and **14+ has no slam
  call** — `4NT` is 12–13); 2/1 `1M - 2m - x - 3NT` 260 rows −0.7k (choice
  of games, BBA bids `4m`); `2NT - 3♣ - 3♦ - 3NT` 61 rows −0.5k (**no
  quantitative call**); `1M - 1x - 2y - 3NT` 270 rows −0.3k (no FSF);
  `1NT - 2♣ - 2x` no-fit 166 rows (**18+ has no call above `4NT`**). The
  three bold holes are one knob, `notrump.quantitative_six_notrump`,
  **shipped default-on 2026-09-30**: +0.0031 / +0.0036 IMPs/board (none /
  both), plain = PD, 82 fired per 204,800 (`ab-results/quantitative-6nt`);
  the checkback / forcing `3M` over the 18–19 `2NT` (BBA's `3♣`/`3♥`/`3♠`,
  the lane's other −1.5k across wrong-strain and missed-slam rows) is
  **shipped default-on 2026-09-30 as `notrump.rebid_checkback`** (BBA's
  structure verbatim — new-minor checkback, forcing six-card `3M`, `3♠`
  over `1♥`): +0.0015 / +0.0015 plain, +0.0016 / +0.0017 PD IMPs/board
  (none / both), 269 / 287 fired per 204,800
  (`ab-results/rebid-checkback`). Owed: the doubled checkback's `(X)`
  tail (floor-owned, two of the ten worst boards) and the 12+ RKCB floor
  over the six-card raise.
- Every cheap lever here is spent (bucket marked mined-to-residual on the BBA
  side); expect design work. Even a 10% capture ≈ +0.01/board — more than
  any single rail.

## 2b. The floor sweep — seed noise, ensembles, the recipe's free parameters

**Closed 2026-09-30: the K = 8 logit mean of init seeds 1–8 is the shipped
floor.** Phase 1's K = 4 won plain DD +0.045 / +0.056 and PD +0.057 /
+0.077 IMPs/board (none / both); K = 8 added a pooled PD +0.008 / +0.008
over it at a plain-DD wash, confirmed on fresh deals, rails re-arbitrated,
at 1.7× K = 4's bidding latency (~2% of an A/B). No Phase 2 recipe trial
won: keep width 256, 300 epochs and `--lr 0.001`. The one open lever, the
rollout-override upweighting, needs a dump and sits in the deferred list.
Numbers, limits and runbook are in
[archive/floor-sweep.md](archive/floor-sweep.md). Training-only work
was authorized 2026-09-28; the M32 corpus stayed frozen and the separate
dump/relabel deferral remains.

## 3. BEN Defensive / book / round-1 — the next slices

Still #2 vs BEN on both scorers (−1.46 / −1.35 per div, 2.0–2.5× the BBA
ratio). The 2♣ wall is **closed** (O4-vul measured 2026-09-25, not shipped —
meta verdict: BEN's direct penalty doubles overprice the lane; do not
re-litigate). Live pieces, all in
[defensive-overcalls.md](defensive-overcalls.md):

- **O1 + the found band clause.** The unbid 5-card major at exactly 15–17
  HCP is 80% of the 1NT-overcall node's −446 plain / −1,002 PD, and BEN bids
  the major on 207/256 such hands. Formally blocked on the failed direct-1NT
  reader, but the 2026-08-13 trigger memo says a narrowed-band arm avoids
  the dilution that sank A5. Ceiling ≈ +0.004 plain / +0.008 PD.
- **O6 three-level jumps** (`P → 3x` slice, −875 plain). Opt-in knob, one
  SD-PD measurement authorized; expect DD pessimism, sd-check first.
- Standing caution from O4-vul: any slice whose PD win is doubling-driven
  needs the meta check before the paired Tier-F run.

## 4. Competitive accountant, session D arm 2

The tight-2m saga's own conclusion: both refutations said the improvable
node is the contested 5-level **declare-vs-defend decision**, not the
overcall band — and BEN, unlike BBA, actually doubles, so the lane is
finally priceable. Pool: the "we act where BEN passes" PD losses (−1,144 PD
over 572 trace boards) plus part of the −29k wall-bound remainder.
Design-heavy. Docs:
[ai-bidder/competitive-accountant.md](ai-bidder/competitive-accountant.md),
[ai-bidder/doubling-calibration.md](ai-bidder/doubling-calibration.md).

## 5. The retrain batch — deferred 2026-09-26

**Not scheduled**: jdh8 declined retrain work for now; listed in
[Owed / deferred](#owed--deferred).

Measured-and-parked wins gated on the next **matched policy/evaluator
retrain**; individually small, but they ride one retrain for free — schedule
as a batch, not as separate efforts:

- B2.5 axis mask ([bba-gap-campaign.md](bba-gap-campaign.md) open-work 7);
- the phantom-suit rail campaign (refuted 3/3 as output-side rails; the
  input-side fix is the retrain — [ai-bidder/new-suit-veto.md](ai-bidder/new-suit-veto.md));
- the reading-drift `1♦ ♦3` atom
  ([reading-drift-handoff.md](reading-drift-handoff.md));
- per [floor-rail-campaign.md](floor-rail-campaign.md) stop-criterion 5, the
  retrain also re-arbitrates every shipped rail.

~~Blocked on the M5.3 corpus question~~ — **unblocked 2026-09-26** by the
label transplant ([ai-bidder/features-v8.md](ai-bidder/features-v8.md)): a
feature bump no longer needs a relabel. v8 itself measured *suspect* (plain
win, PD erases it; opt-in) — B2.5 and the `1♦ ♦3` atom still ride the next
bump the same way, but a bump alone does not re-arbitrate the rails.

## 6. Cheap insurance + one owed decision

Near-zero direct IMPs; protects banked wins and unblocks measurement:

- Owed fresh-seed confirmations: N3-fit
  ([one-notrump-competitive.md](one-notrump-competitive.md) queue),
  pass_exclusion's two seeds. (Meckstroth `3m` jumps: confirmed, demoted
  2026-09-26; forcing-NT two-suiter: not confirmed, stays on —
  [bidding-options.md](bidding-options.md).)

## Deliberately not ranked

Recorded so future sessions don't re-derive them:

- **PDI's two owed A/Bs** — the `3237e037` anchor proved PDI, `union_hull`
  and the latch detector changed **0 boards in 819,200 table-auctions**;
  inert at scale. Still owed for completeness, not for IMPs
  ([pdi.md](pdi.md)).
- **Anything in the N4-KK lane** — below headline CI by construction (the
  lane fires on 0.06–1.17% of table-auctions).
- **Constructive opening** — mined; the light-open wall and weak-two levers
  are both refuted.
- **N3-x / N2d** (−2.9 to −3.1 per board but 25–43 boards ≈ +0.0005/board
  total) — batch them into the next 1NT-lane visit
  ([one-notrump-competitive.md](one-notrump-competitive.md)).

## Parked big ideas

Four big ideas were weighed on 2026-09-28; the floor sweep (item 2b) won.
The other three are parked here with the evidence that parked them, so a
future session re-opens one only against new evidence, not from scratch.

- **Improve the defensive bidding system.** Already the most-mined lane in
  the repo. [defensive-auctions-reference.md](defensive-auctions-reference.md)
  §7 (2026-09-27) probed every lever L1–L5 plus the raise-aggression trace and
  fourth-seat sub-lanes: none clears what a Tier-F A/B resolves; vs BBA the
  v6 floor is at par in these lanes and the gap vs BEN is BEN's search-priced
  judgement spread over ~40 cells. The book-side lanes
  ([defensive-overcalls.md](defensive-overcalls.md),
  [takeout-double-layers.md](takeout-double-layers.md)) have their residue in
  item 3 above. **Re-open when:** a new net (item 2b or a retrain) moves the
  defensive floor buckets, or DD-search-at-leaves
  ([ben-gap-campaign.md](ben-gap-campaign.md) Phase 3) exists — the remaining
  gap is judgement, not structure.
- **Add Polish Club.** The nearest precedent is the retired `dutch()` system
  ([archive/dutch-system.md](archive/dutch-system.md)): a naturalised Polish
  Club that never beat `american()` as a whole (Phase 2.1 lost, WJ-floor arms
  B and C lost, the Multi lost) while every win was a per-gadget knob. A true
  Polish Club is a larger diff than Dutch was, and its cost is not the rows:
  the reader and the neural floor are american-tuned, so the system needs its
  own corpus and floor (the reading-knob-under-a-neural-floor mechanism that
  sank 5542, 2026-09-24), i.e. exactly the dump-and-retrain work now deferred.
  What it would buy is indirect — a second-system corpus for M5.3
  ([ai-bidder/plan.md](ai-bidder/plan.md)) and a card BBA already plays
  (`vendor/bba/WJ.bbsa` is a machine teacher). **Re-open when:** retrains are
  un-deferred *and* the goal is a second disclosed system rather than IMPs for
  the default, or a Polish gadget can be measured as a knob on `american()`
  (the house method). Spec: jdh8's Strawberry Polish Club
  (<https://polish.club/>).
- **LSTM to store long bidding sequences.** Built, measured, refuted 3/3 as
  M5.2 (`park/lstm-floor`, 2026-09-03; verdict and the overbidding diagnosis
  in [ai-bidder/plan.md](ai-bidder/plan.md) M5.2). The corpus showed only
  0.75% of decisions at the `T = 20` cap and a mean of 5.66 prior calls, so
  auction *length* is not the binding constraint. Its flip plan is owed (arm 1:
  retune the accountant collar against v7 from the unweighted net; arm 2: a
  paired policy baseline instead of par) and rides the deferred retrain. Item
  2b's Phase 0 also supplies the seed-noise caveat every LSTM arm lacked.
  **Re-open when:** the flip plan's two arms can run, i.e. retrains are
  un-deferred, and only after 2b's σ_seed is known.

## Owed / deferred

The live list of work that is **owed but not scheduled**. Retrains are
deferred by jdh8 (2026-09-26: too long to run now); everything in the first
group waits on that decision, the second group does not.

**Retrain-gated (deferred):**

- Re-arbitrate the five shipped floor rails against the new net — runners and
  rule in [floor-rail-campaign.md](floor-rail-campaign.md) (stop criterion 5).
- `features_v8`'s three levers — the v8-only keycard-ask rail, re-sampling
  the retired Dutch rows, a relabel at HEAD
  ([ai-bidder/features-v8.md](ai-bidder/features-v8.md) §5).
- The retrain batch of §5 above (B2.5 axis mask, the `1♦ ♦3` atom, the
  phantom-suit input fix).
- The floor-rail residue: `1NT - 2♣ - 2♥ - 2NT` `3♣` (38 bd, −410 PD — one
  exact sequence) and the long "we act / BBA passes" tail at `7e0bc648`
  (no lane at 0.5k PD).
- The advance that passes a textbook raise: clean `(1x) 1y -`, 3-card
  support, 8–10 HCP. The v6 floor passes where both references raise
  (−4.0 PD/bd vs BEN, −2.1 vs BBA, but ≤ 0.003 IMPs/board). A labelled
  example for the retrain's eval set, not a rail
  ([defensive-auctions-reference.md](defensive-auctions-reference.md) §7).

**Not retrain-gated (owed, unscheduled):**

- **Forcing-NT jump shifts — default-on since 2026-09-27** (Meckstroth off):
  round 5 won vs the Meckstroth `2NT` on every bracket of every cell (SD-PD
  +0.0016/+0.0012 NV, +0.0011/+0.0009 vul, 400k × 2 seeds; numbers and the
  trace in [bidding-options.md](bidding-options.md),
  `set_forcing_nt_jump_shifts`). **Owed:** a vs-BBA run of the flip — 42 of
  the 200 auctions `smoke-default` moved (0.2% of deals) have an opponent's
  call after the opening, which `ab-meckstroth-2nt` never sees (it silences
  them). Residual levers, each worth ≈ 50–100 SD-PD IMPs per 400k cell:
  1. The `3♣` bucket is negative in all four cells: the floor's re-ask
     sometimes raises straight to `6M` (`3♠ - 4♠ - 6♠`) or wanders (`4NT -
     5♦ - 5♥ - 6♣ - 6♠`). An authored 12+ re-ask **lost** to it on the same
     deals (round 6), so this is a floor lever, not a row.
  2. The natural `2NT` lane (`2NT -> 2NT`) is negative in all four cells.
     **Probed 2026-09-27** (board 5791, deal seed 1790452526): opener
     AK952.A82.K4.A53 (19, balanced), responder 643.KQ753.AJ93.4 (10, three
     spades, stiff club). `2NT - 3♠` (fit slam try, 10+) `- 4♠` (sign-off:
     opener accepts on 20+) `- Pass`; Meckstroth's floor re-asks and makes
     6♠. Every call follows the system; what is wrong is the **reading**:
     responder reads the natural `2NT` as **HCP 11–21**, not the rule's 18+
     (`probe-decision "643.KQ753.AJ93.4" "- 1♠ - 1NT - 2NT -"`), and as
     11–19 after the sign-off, so the floor cannot see 29+. Two levers,
     both A/Bs: (a) repair the reading (an authored-reading ticket —
     [authored-reading-handoff.md](authored-reading-handoff.md)); (b) opener
     accepts the slam try on 19+ (19 opposite 10–12 is 29–31 before
     shortness) — a row change that keeps the sign-off meaning *no slam*,
     where (a) only helps the floor overrule it. **(b) measured 2026-09-27,
     not shipped** (same deals as round 5, `pons-ab-results/jump-shifts/`
     `run8.sh`/`run9.sh`, patches beside them): 19+ in every lane (round 7)
     lost plain DD and PD in all four cells (−168/−200/−61/−77 plain) — the
     `3♣` jump shift's bucket worse in each; 19+ below the natural `2NT` only
     (round 8) is a wash — Δ plain DD −23/−28/+7/+6, PD −29/−37/−4/−8,
     SD-PD −0.0003/+0.0004/+0.0003/−0.0002 NV/vul × two seeds. Its bucket
     gains ≈ +52 SD-PD per cell while plain DD moves −23…+7: the extra slams
     are the blind lead's, and at slam SD-PD is only a stress test. That
     leaves (a).
  3. Opener's 21+ ask over the uncapped 4-4 `4♥` (`3♥ - 4♥ - 4NT … 6♥` down
     on 231698/297815): the asker table bids six on three keycards plus one;
     a cap or a 22+ threshold is one small A/B, but the `3♥` bucket is the
     lane's biggest winner (+145…+251), so low priority.
  4. The displaced 5-5 (`3♥ -> 2♥`) swings ±100 between cells — noise, no
     lever; the real hole is shared: `responder_after_forcing_notrump` cannot
     raise opener's second suit (four-card support with 8–9 HCP passes `2♥`).
  5. The seven-card `4M`: the floor raises it to `6M`/`7M` (`4♠ - 4NT - 5♠ -
     7♠`); a `Pass`/`ask_or_pass` row above it is one small A/B.

- ~~Trace the shipping-only constructive move in the `7e0bc648` window~~
  **Closed 2026-09-26**: it is the rails at the mirror table (a board is
  bucketed by its first divergence, so our defence where BBA opened is
  charged to our opening bucket); 2NT-double +3,614 of the +5,178 PD. The
  reading ships were not involved. Table in
  [bba-gap-campaign.md](bba-gap-campaign.md), current ranking.
- The game-pull rail reads their bid's level, not its meaning: its worst A/B
  boards are their artificial `5♣`/`5♦` (keycard replies, cues). An
  alert-exempt variant is one small A/B; the shipped rail wins without it.
  **Closed 2026-09-26**: built as `their_game_pull_alert_exempt` (a "code"
  is alerted *and* promises < 3 cards in the strain, since
  `completion_alerts` alerts Smolen/transfer completions too); fires on 1
  board in 204.8k and loses it — code dropped (`83989dd4`). The
  level-reading rail is right.
