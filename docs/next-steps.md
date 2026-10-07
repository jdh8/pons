# Next-step candidates, ranked by potential IMP gain

**Ranked 2026-09-26, item 2 re-read 2026-09-30, 2026-10-03 and 2026-10-06** from the two
current anchors — BBA shipping arm at `25aea82c` (2026-10-06, re-anchored
after the responses-to-`1M`, `1m - 1♠` and contested-opening ships; opening
and round-2 tie at #1 / #2 across two seeds, so the ranking below stands;
[bba-gap-campaign.md](bba-gap-campaign.md)) and BEN Tier S at
`daa8bf4a` (2026-09-14, stale by the same window;
[ben-gap-campaign.md](ben-gap-campaign.md)). **Retrains are
deferred** (jdh8, 2026-09-26): items that need one are owed, not queued —
see [Owed / deferred](#owed--deferred).
Method: pool size on both references × how often the vein has actually
cashed ÷ effort. Figures are IMPs/board unless marked per-fired. Re-rank
after the next re-anchor; each item's ship decision is its own fresh-seed
A/B under [measurement.md](measurement.md).
Shipped, refuted and closed items are archived verbatim in
[archive/next-steps-done.md](archive/next-steps-done.md).

## 1. Floor junk-action rails — closed 2026-09-26

Five rails shipped, and the R6 re-census at `7e0bc648` found the pool dry
(−0.018 PD/board left, no lane at 0.5k); ledger and close-out in
[floor-rail-campaign.md](floor-rail-campaign.md). Its residue belongs to the
next matched retrain, which must also re-arbitrate every rail (stop
criterion 5). **Item 2 is now the top candidate.**

## 2. RKCB / slam accuracy (Constructive / book / round-2)

**Biggest unworked pool on both references.** The shipped lanes — passed
hand, strong `2♣`, preemptive minor raise, Modern double answer, cue-raise
sign-off, the 2026-10-05 re-cuts, the quantitative `6NT` and checkback, the
RKCB asker fix — are archived with their lessons in
[archive/next-steps-done.md](archive/next-steps-done.md); below are the live
ranking and each lane's unworked residue.

- **Strong `2♣` lane — residue.**  Six ships 2026-10-01…03 (archived,
  with what the lane taught).
  - Next, sized off `probe-strong-two-grand` (8M self-play deals): the grands behind the
    suit-positive `6NT` jumps (≈ +2.7k IMPs per 8M with both counts known,
    spread over a dozen auctions at +200–400 each, so no single table
    carries it); a Stayman-like ask over the `2NT` positive (an eight-card
    major fit out-scores notrump at slam level on 193 boards per 8M,
    +1,548 IMPs with perfect choice); responder's invitation on exactly 8
    opposite the counted `3NT` (24 opposite 8 makes 62% on 165 boards,
    ≈ +435 IMPs).  Each is below what three seeds resolve.  Then opener's
    game-in-hand jump rebids, opener's 28+ balanced rebid (falls to the
    `2NT` fallback), the contested tail (289 rows, −0.6k / −0.8k).
  - Flagged while tracing, not built (jdh8 to decide):
    - **A double of the keycard answer drops the asker to the floor** —
      built, a non-win, parked on `park/rkcb-doubled-answer` (archived).
      Two leads it found, neither built: opener's ask over a splinter raise
      is thin (census the undoubled `1M - splinter - 4NT` lane before
      touching the gate), and a doubled `5♦` is mostly none, not three
      (twelve boards per ~2M; too few to build or measure on).
    - **The classic `5NT` path in every other lane still wants all three
      side kings**, while the queen relay beside it and the grand rung bid
      seven on two ([ai-bidder/bba-kickback.md](ai-bidder/bba-kickback.md),
      "The two king asks disagree").  Proposed default: `grand_rkcb_rows`
      for every major lane as its own arm — the asker's `hcp(19..)` gate is
      a weaker promise outside the `2♣` lane, so it wants a combined-HCP
      cut first (`probe-trump-queen --grand-hcp`).
    - The same rung under `2♣ - 2♦ - 2M - 3M - 4NT` and `2♣ - 2♦ - 3m - 4m -
      4NT` (opener asks on 28+): a different population, unmeasured, left
      on the classic ladder.
- **Re-ranked 2026-10-06 at `25aea82c`.**  The bucket is #2 on the
  shipping arm's plain column behind opening, a tie (the fresh-seed
  confirmation keeps it #1), and still #1 on PD: −21,551 plain / −22,651 PD
  on 33,184 rows (−25,495 / −26,677 at `46d0dc14`); headline, tables and the
  paired window in [bba-gap-campaign.md](bba-gap-campaign.md).  The prefix
  cut, per 409,600 boards (rows to their shard auctions, keyed by the
  auction before the divergent call, leading passes folded; both snapshots
  cut by the same scratch script, so the `46d0dc14` column differs slightly
  from the 2026-10-03 table's row counts):

  | lane | rows | plain | PD | at `46d0dc14` | note |
  | --- | --- | --- | --- | --- | --- |
  | responses to `1♥` / `1♠` | 9,739 | −8,710 | −10,077 | −9,855 / −11,113 | still the largest, still spread thin.  The forcing `1NT` vs BBA's `3♥` pair is gone (the suit invite); worst now our `2♣` vs BBA's `2♥` over `1♠` 369 rows (−1,053 / −892) — the suit choice `response.two_over_one_hearts_first` washed on — and our `1♠` vs BBA's `2♠` over `1♥` 316 rows (−848 / −855) |
  | our `2♣` opening, every continuation | 2,930 | −3,604 | −3,582 | identical | no ship in the window |
  | `1♥ - 1♠ -` opener's rebid | 1,458 | −2,393 | −2,282 | −2,436 / −2,329 | now the largest round-2 prefix, unmoved; no pair above 350 IMPs (`2♥` vs BBA's `2♦` 181 rows −329 / −345, `3♥` vs `4♥` 120 rows −308 / −338) |
  | `1♣ - 1♠ -` opener's rebid | 1,088 | −1,600 | −1,709 | −1,978 / −2,063 | `3♦` vs BBA's `2♦` 89 rows (−237 / −251), `2♣` vs its `1NT` 480 rows (−218 / −272) |
  | `1♦ - 1♠ -` opener's rebid | 2,278 | −1,396 | −1,755 | −2,518 / −3,080 | nearly halved by the unbalanced `1NT` and the `3NT` jump |
  | `1♠ - 1NT -` opener's rebid | 1,680 | −1,235 | −1,160 | −1,891 / −1,850 | the forcing-NT raises |
  | responses to our weak `2♠` | 948 | −867 | −1,606 | identical | obstruction, which DD cannot price — not a lane for this harness |
  | `1m - 3m -` | 77 | −163 | −155 | −1,093 / −1,466 | shipped, done |
  | `1m (1♥) X -` | 457 | +107 | +351 | −1,479 / −2,188 | shipped, now a gain |
  | `1♥ (2♠) 3♠ -` | 35 | +30 | +38 | −594 / −794 | shipped, now a gain |

  No new sharp lane: the round-2 pool is now the thin tail of many
  rebids, none worth a lane alone.  The one window loss is a keycard
  captaincy hole, below under the re-cut.

- **Re-cut 2026-10-07 — responder's second call over `1♥ - 1♠ - 2♥`.**  The
  densest responder node at `25aea82c`: 398 rows, −756 / −870.  **Shipped
  2026-10-07:** `rebid.heart_rebid_invite_accept` — opener accepts the `3♥`
  invite on 13+ points (12+ with seven hearts), not 14+ (was: our `3♥` vs
  BBA's `4♥`, 97 rows, −364 / −306, a vulnerable game left in `3♥`).  Pooled
  two seeds: plain +0.0005 / +0.0010, PD +0.0004 / +0.0007, every cell
  outside its CI.  Unworked: our `2NT` invite where BBA passes on short
  hearts (115 rows, −230 / −422; the rung is `points(10..=12)`, outside
  `nt_invite_hcp`'s reach); no slam try on 16+ (our `4♥` vs BBA's `4NT`, 26
  rows, −120); likewise `1♥ - 1♠ - 2♣ - 2♦ - 2♠` offers only `4♠`.
- **Re-cut 2026-10-07 — missed slams after `1♥ - 1♠`.**  Our `1♠` vs BBA's
  strong jump shift `2♠` (328 rows, −817 / −837) is 137 boards of BBA-only
  slams (−948 plain): responder's 15–20 count signs off at round 2, not
  the first call.  The uncontested missed-slam pool (BBA ≥ 6, us < 6) is
  6,238 boards, −22.8k plain; `1♥ - 1♠` its largest lane (620, −3.5k), then
  `1♠ - 2♣` (−2.4k), `1♦ - 1♠` (−1.4k), `1♣ - 1♠` (−1.3k).  **Shipped
  2026-10-07:** `rebid.heart_rebid_keycard` — `4NT` RKCB over `1♥ - 1♠ - 2♥`
  (2+ hearts, 16+) and the `3♥` jump (14+); pooled two seeds plain +0.0017 /
  +0.0020, PD +0.0017 / +0.0020, every cell outside its CI (CHANGELOG).
  **Shipped 2026-10-07:** `rebid.diamond_rebid_fourth_suit` — over `1♥ - 1♠
  - 2♦` the `3♣` fourth-suit game force (13+ HCP), with `4♥` (three hearts,
  13–15) and `3NT` (13–15, no major fit) as fast arrivals beside it; opener
  answers naturally, responder places or asks keycards for hearts.  Was 88
  boards of `3NT` on three hearts, −561, BBA's slam in hearts on 34; pooled
  two seeds plain +0.0007 / +0.0009, PD +0.0008 / +0.0009, eight of eight
  cells positive (CHANGELOG).  **Next lever in the lane, found by its worst
  boards:** `rebid_one_heart_one_spade` never calls the extras ladder, so
  opener has no jump shift over `1♥ - 1♠` and its `2♣` / `2♦` is 12–19+; a
  19-count answers the fourth suit with `3NT` and gets passed (26 / 23
  boards per seed, −22 / −7).  Author `3♣` / `3♦` jump shifts (18+, 5-4) on
  the minor lanes' ladder, then re-read the `3NT` answer as 12–17.  Residue:
  BBA's diamond slams there (29 boards) have no rung; opener's call over
  the `4♥` raise is the floor's (it asks on 15 opposite the 13–15 cap).
  The fourth-suit tail `2♣ - 2♦`'s major-fit keycard ask measured a wash
  2026-10-07 (opt-in `rebid.fourth_suit_keycard`; its club and notrump
  slams unworked, [fourth-suit.md](fourth-suit.md) § Owed).  Unworked: the
  misfit `3NT` over `2♥` / `3♥` (108, −772) is BBA's spade slam on
  responder's own long spades.
- **Fourth-suit census, 2026-10-07** — moved to
  [docs/fourth-suit.md](fourth-suit.md): 4SF is authored in the `1♥ - 1♠`
  lane only, XYZ owns the one-level prefixes (jdh8), the reverses measured a
  wash and sit on `park/reverse-weak-responses`.  Owed there: the
  `2♣ - 2♦` slam try, the card-row / ledger-58 correction (disclosure
  decision awaits jdh8), the `1♠ - 2♣ - 2♦ - 4♥` natural-`2♥` flag.
- **Residue of the 2026-10-03 ships.**  Preemptive minor raise: the
  contested tail (`1m - 3m (X)`, their overcall) stays the floor's.  Modern
  double answer: the `2♥`/`2♠` strength cue on 16+ (BEN's tool for the
  strong raise) and the splinters, not built.
- **Re-cut 2026-10-05 (`46d0dc14`) — responses to `1♥` / `1♠`.**  The 2/1
  suit choice washed; the forcing-NT suit invite, minor before spades,
  side-suit-first and reverse-extras shipped; the second-suit keycard seats
  swept and closed (all archived).  **A first-call census prices BBA's
  continuations along with its call** — read that before pricing anything
  here off a census.  Unworked:
  - after `1M - 2♦ - 3♦` we reach `6NT` / `7NT` where `1M - 2♣ - 2♦ - 3♦`
    finds the diamond slam (mostly 5-5 minors); not traced;
  - over `1♠ - 1NT - 3♠` responder bids `3NT` on seven or eight hearts (18 rows,
    −67);
  - after `1♥ - 2m - 3♥` responder has no `3♠` and raises on two, losing a
    4-4 spade fit; the keycard answerer raises the asker's `5M` sign-off to
    `6M` (17 / 18 boards, net −1 / −8 plain);
  - **the keycard answerer overrode the asker's `5♠` sign-off — fixed
    2026-10-07.**  `1♠ - 2x - 3y - 3♠ - 4NT - 5♥ - 5♠ - 6♠` on 29 anchor
    boards (−240 / −240), surfaced by `e0736f44` (opener's `3♣` now reads
    `♠5..13`, lifting floor rule #156 over the sign-off).  The cause was the
    floor's `respect_keycard_signoff`, not the lane: any answerer with two+
    keycards drove past a placement, a patch for the pre-2026-09-30
    `asker_after_5h`.  The proposed fix (answerer pass rows in every lane)
    was the 2026-09-27 census's refuted idea; re-run on `main`
    (`probe-rkcb-answerer`, 1M deals) it split cleanly — passing wins on the
    `5♥ - 5♠` overrules (+185 / +248, 160 boards), loses on the `5♣`
    corrections (−239, 22).  Shipped instead: the answerer drives on only
    holding the high reading of its own answer.  Two seeds, all eight cells
    positive inside the CI, pooled plain +0.0002 / +0.0003, PD +0.0003 /
    +0.0004 (CHANGELOG).  The `♥0..13` projection is not drift: every
    `1♠ - 2♥ - 3♣` path still holds at most three hearts (6♠ + 4♣, or below
    the `3♥` raise); the projection just cannot see a 13-card-sum cap.
    The `1♥ - 2m - 3♥` 5M→6M override above is the same rule, unretraced;
  - five spades and a six-card minor still bid `1♠`;
  - **shipped 2026-10-06:** `rebid.forcing_notrump_doubleton_raise` —
    over `1M - 1NT - 2M` the `3M` limit raise takes a doubleton, as BBA
    bids it (our `2NT` vs its `4♠`, 136 rows, −576 / −564).  Pooled two
    seeds: plain +0.0020 / +0.0034, PD +0.0021 / +0.0033 IMPs/board, every
    cell outside its CI.  **Shipped 2026-10-06 beside it:**
    `rebid.forcing_notrump_heart_raise` — over `1♠ - 1NT - 2♥` responder
    raises to `3♥` on four hearts and 10–12, and `2♠` denies four hearts
    (was: our `2NT` vs BBA's `4♥`, 31 rows, −256 / −268).  Pooled two
    seeds: plain +0.0012 / +0.0018, PD +0.0014 / +0.0021, every cell
    outside its CI.  **Lane closed 2026-10-06** — both residues refuted
    double dummy, by re-scoring the two ships' on-arm boards (both seeds)
    against the counterfactual call (`bba-score`, IMPs per fired, none /
    both): the floor's `3NT` over `1♠ - 1NT - 2♥ - 2♠` (15–17, 5-4; 80
    boards) beats passing `2♠`, plain +0.34 / +2.50, PD −0.63 / +1.15 — so
    the node keeps its no-catch-all design (a `Pass` there already lost
    −242 IMPs on 47 boards, `jump_shifts.rs`); opener's `3NT` on six trumps
    over `1M - 1NT - 2M - 2NT` (now ≤ 1 trump opposite) beats `4M` in every
    cell, ♠ (55 boards) plain +0.22 / +0.89, PD +1.04 / +1.89, ♥ (18) +0.3
    to +0.6.  The authored 5-5 `3♥` invite there is mildly negative (100
    boards, plain −0.82 / +0.20, PD −1.90 / −1.37 per fired), about
    −0.0001 IMPs/board — below what an A/B resolves.  BBA's direct `4♥` on
    shape stays unbuilt: its whole parent pair was 31 rows;
  - our game-forcing `2♣` where BBA bids `1NT` (448 rows,
    −521 / −874, mostly 12 HCP and shapely 11s — the `Points13` gate); the
    strong jump shift (316 rows, −848 / −855, slam misses on 16–17 HCP with
    five spades; it costs our weak `2♠`, +113 / +181).  The weak jump shifts
    as a family are a plain wash and −1,003 PD against BBA's pass.  Read the
    census lesson above before pricing any of these.
- **Re-cut 2026-10-05 — opener's rebid over `1m - 1♠`.**
  `rebid.unbalanced_1nt_rebid` shipped (archived).  Unworked: the rest of
  the `1♣ - 1♠` `2♣` vs `1NT` pair (5♣4♦, no four hearts, BBA `1NT`; the
  1435 cell gains for us).  **Shipped 2026-10-06:**
  `rebid.minor_jump_notrump` — opener's `3NT` on a six-card minor and 18+
  HCP, so the `3m` jump keeps 16–17 (was: `1♦ - 1♠ - 3♦` vs BBA's `3NT`,
  191 rows, −360 / −296; the whole `1m - 1x - 3m` family 667 rows, −1,148
  / −979).  Pooled two seeds: plain +0.0011 / +0.0011, PD +0.0009 /
  +0.0009, every cell outside its CI.  **Also shipped 2026-10-06:**
  `rebid.minor_jump_notrump_slam_try = Some(9)` — responder's `4m` slam try
  over that `3NT` (2+ support, 9–12 HCP), opener RKCB on 20+ else `5m`.
  Pooled two seeds: plain +0.0002 / +0.0005, PD +0.0002 / +0.0005 (both-vul
  outside its CI, none-vul a positive wash).  Unworked: the 13+ fit hand
  still bids the floor's blind `6NT` rather than `4m` and keycards, and the
  18–19 sign-off `5m` goes down where `3NT` makes (the worst boards).
- Levers on file: Stayman major-fit RKCB and post-Smolen continuations
  ([one-notrump-constructive.md](one-notrump-constructive.md)), minor-keycard
  grand handling, the parked four-four shape-slam thresholds.  (Kickback is
  not a score lever: [ai-bidder/bba-kickback.md §7.16](ai-bidder/bba-kickback.md).)
- **RKCB asker tables — fix shipped 2026-09-30** (archived; wiring
  `rkcb_answerer_rows` into every lane was censused and refuted).
  Unmeasured: routing the 1-keycard `5♦` case through the queen relay, and
  whether the jump-shift lane's wired answerer rows pay the same tax.
- **Slam census 2026-09-30** (`scripts/slam-census.py`, archived): missed
  slams dominate 3:1, spread thin; every round-2 `3NT` is a sign-off.
  Shipped from it: `notrump.quantitative_six_notrump`,
  `notrump.rebid_checkback`.  **Its unworked classes are suspect
  (re-read 2026-10-06):** the script reads `boards.jsonl`, the
  *instinct* arm, not the shipping arm (`boards-american.jsonl`), and its
  slam-flavoured filter conditions on the outcome.  Re-cut on the
  shipping arm at `46d0dc14` with every outcome kept, opener's 2-level
  rebid vs BBA's jump (all suits) is 390 rows, −71 plain / **+226** PD,
  and responder's `1M - 2m - x - 3NT` is 133 rows, −353 PD (BBA's `4♣`
  there flat, +14 on 44): both dead as levers.  `1M - 1x - 2y - 3NT`
  (270 rows, −0.3k; no FSF) is unchecked the same way.  Owed on
  checkback: the doubled `(X)` tail and the 12+ RKCB floor over the
  six-card raise.

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
A fifth joined them on 2026-10-02 and a sixth on 2026-10-03.

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

- **Exact hand posteriors as a bidding input.** Parked 2026-10-02 after
  every gate was read ([exact-posterior.md](exact-posterior.md)). The counter
  is built and pinned against the web one, but each consumer that used the
  *readings* as a posterior failed: the evidence gate (AUROC 0.554), the
  narrowness gate (0.5% of boards), the reply forecast (a reading is not
  one), and a rollout over worlds drawn from the readings (plain +0.074
  ±0.053, PD −0.013 ±0.060 per decision at `1M - 2M`). The same rollout over
  worlds our own bidder reproduces **passed** (+0.0017 ±0.0007 plain /
  +0.0022 ±0.0008 PD IMPs/board at that one seam, about half of it since
  shipped as `response.major_raise_slam_try`), which is evidence for M8, not
  for the counter. **Re-open when:** one of the four measured triggers in
  that doc's §5 "Parked" fires — a search consumer that samples a starved
  auction, the range arm of `probe-seam-lookahead` passing on a fresh seed,
  an invitation's answer getting a reading, or the narrow-reading reach
  moving off 0.5%.
- **M8 search as a seam-gated rollout lookahead.** Parked 2026-10-03 by jdh8:
  too slow at run time. Branch `park/m8-search` (`8eeb9fc6`; design, gate and
  flip plan in its `docs/ai-bidder/plan.md`, M8.0). Validation passed at the
  two `1M - 2M -` seams — +0.0011 ±0.0006 plain / +0.0012 ±0.0007 PD
  IMPs/board, self-play, 600,000 deals — but at 0.69 s per gated decision on
  1.36% of deals, and a bidder that solves inside `classify` cannot sit in a
  rayon harness. No wider seam list was run. **Re-open when:** the per-decision
  cost drops by an order of magnitude (fewer worlds, a confidence gate, or a
  cheaper pricer than a DD solve per world).

## Owed / deferred

The live list of work that is **owed but not scheduled**. Retrains are
deferred by jdh8 (2026-09-26: too long to run now); everything in the first
group waits on that decision, the second group does not.

**Retrain-gated (deferred):**

- Re-measure `strong_two_waiting` with the reading mirror unpinned (their
  `2♣ - 2♦` decoded as waiting, as BBA plays it): lost on the v6 floor
  (item 2, the strong `2♣` lane); a win after the retrain would also retire
  the mirror book every default build now carries.
  `strong_two_loose_positive`, `strong_two_positive`, `strong_two_grand` and
  `strong_two_positive_notrump` are pinned off in the same mirror, unmeasured
  unpinned (the first two's first trial leaked two boards per 204,800 through
  it): re-measure all five together.
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
