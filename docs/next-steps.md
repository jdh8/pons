# Next-step candidates, ranked by potential IMP gain

**Ranked 2026-09-26, item 2 re-read 2026-09-30 and 2026-10-03** from the two
current anchors — BBA shipping arm at `46d0dc14` (2026-10-03, re-anchored
after the passed-hand and strong-`2♣` ships; bucket order unchanged, so the
ranking below stands; [bba-gap-campaign.md](bba-gap-campaign.md)) and BEN Tier S at
`daa8bf4a` (2026-09-14, stale by the same window;
[ben-gap-campaign.md](ben-gap-campaign.md)). **Retrains are
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

**Biggest unworked pool on both references. The passed-hand lane is done
(opener's pass shipped 2026-09-30, Reverse Drury and opener's splinters over
it shipped 2026-10-01, below); BBA's other rungs and its passed-hand `2NT`
priced and left alone. The strong `2♣` lane (re-cut 2026-10-01, below) has
six ships, the notrump count over the `2NT` positive the latest
(2026-10-03); what is left of the direct `6NT` jumps is thin.**

- **Re-anchored 2026-09-30 (`494f0c4b`) — the passed-hand lane.** Still #1
  on both scorers (−31,106 plain / −33,752 PD, 36,007 rows). Re-cutting the
  bucket by auction prefix with leading passes folded finds one lane no
  earlier census named, because its rows are game-level and were spread over
  seats: **BBA treats a passed hand's response as non-forcing and plays
  Drury; we do neither.** Shipping arm, per 409,600 boards:
  - *Opener passes a passed hand's `1M` response* (`- - 1x - 1M -`, third
    or fourth seat): 1,042 rows, **−1,766 plain / −2,683 PD**. BBA's pass is
    three-card support (1,040 rows) on 12–14 HCP (1,023 rows; −2.0 / −1.7 /
    −0.8 plain per row at 12 / 13 / 14); we rebid `1NT` on
    1,008 of them and responder drives on. Both seats and both majors price
    alike. One rule, one knob, and a Pass reading — the biggest single lever
    found in this bucket (ceiling ≈ +0.004 plain / +0.007 PD per board).
    **Shipped default `Some(14)` 2026-09-30** as
    `rebid.passed_hand_major_pass` (exactly three-card support, balanced, at
    most 14 HCP): won 4/4, plain +0.0031 / +0.0041, PD +0.0043 / +0.0067
    IMPs/board (none / both), 538 / 610 fired per 204,800
    (`ab-results/passed-hand-pass`); the 13 ceiling won less. Not tried: the
    unbalanced three-card pass (BBA passes ~14 of ~70 per HCP step there) and
    a passed hand's forcing `1NT`.
  - *A passed hand's response to third/fourth-seat `1M`*: 1,890 rows,
    −1,931 plain / −2,395 PD. BBA's Drury `2♣` is 1,144 of them (−719 /
    −810); its passed-hand `2NT` 176 rows at −4.1 per row (−726 / −702,
    meaning not yet looked up — `probe-bba-book`); its pass where we respond
    227 rows (−173 / −560). **Reverse Drury shipped default-on 2026-10-01**
    as `response.drury`: the `2♣!`, opener's `2M` / `2♦!` / `4M` / `4NT`
    ladder, responder's rungs, their X of the raise or the relay (systems
    on) and their overcall (opener's ladder).  Finished build plain +0.0013
    / +0.0024, PD +0.0010 / +0.0020 (none / both); pooled four seeds plain
    +0.0010 ±0.0006 / +0.0019 ±0.0009, PD +0.0007 ±0.0007 / +0.0017 ±0.0010
    (`scripts/ab-drury.sh`, CHANGELOG).  We play the reverse form; BBA reads
    it off the `Reverse drury` card row and plays the original itself.  **Opener's
    splinters over it shipped default-on 2026-10-01** as
    `response.drury_splinters` (`3♦` / other major / `4♣`, 16–20 support
    points; responder's no-waste step, opener's `4NT` on 19+): pooled two
    seeds plain +0.0004 ±0.0004 / +0.0006 ±0.0006, PD +0.0005 ±0.0004 /
    +0.0007 ±0.0006 (`scripts/ab-drury-splinters.sh`, CHANGELOG).  Priced off
    the Drury dumps and **not authored**: BBA's natural `2♥` over `1♠` and
    its minimum `2♠` lose to our ladder; its `2NT` / `3♣` / `3NT` rungs are
    small and noisy.  BBA's passed-hand `2NT` is its floor's natural
    11-count, not a convention: uncontested ceiling ≈ +50 IMPs per 204,800
    boards, the rest BBA's own defenders misdefending a `2NT`.  The lane is
    closed.

  Order: the opener's pass first (self-contained, shipped), Drury second. The slam
  share is still thin — missed-slam 961 rows −11.4k, missed-grand 176 rows
  −2.2k on the instinct arm, no lane above 129 rows (`1♥ - 1♠`) — and the
  largest call-pair classes are the ones already listed below (`3NT` vs
  BBA's `4m` / `4NT`, opener's `2x` vs BBA's jump, `1♦ - 1M - 2♦` vs `2♣`).
- **Re-cut 2026-10-01 — the strong `2♣` lane.**  The passed-hand lane was
  found by cutting rows by auction prefix rather than by bucket; the same cut
  over every shipping-arm bucket at `494f0c4b` (rows joined to their shard
  auctions, leading passes folded; the script is a scratch join, not kept)
  ranks the clean lanes, per 409,600 boards:

  | lane | rows | plain | PD | note |
  | --- | --- | --- | --- | --- |
  | our `2♣` opening, every continuation | 3,054 | −7,003 | −6,630 | −2.2 per row; split over opening / round-1 / round-2 and seats, so no bucket showed it |
  | responses to `1♥` / `1♠` | 10,970 | −10,951 | −12,258 | −1.0 per row, spread thin; "mined" (row 5 of the ranking) |
  | `1♦ - 1♠ -` opener's rebid | ~2,200 | −3.0k | −3.8k | `2♦` vs BBA's `2♣` / `1NT`; the `2♣` arm is a measured-wash opt-in |

  Inside the `2♣` lane: the response 2,017 rows (−4.9k / −4.6k), opener's
  rebid 1,099 (−1.6k / −1.8k), responder's second call 431 (−1.1k / −1.1k),
  their interference 289 (−0.6k / −0.8k).  The call pairs: our `2♥` double
  negative vs BBA's `2♦` waiting 680 rows (−1.7k / −1.2k); our `2♦` vs BBA's
  natural positive (`2♥` 169, `2♠` 165, `3♣` 145, `3♦` 115 rows, −1.7k /
  −1.9k together — we have no heart positive, and ours need two top honors
  and 8+); our pass of `2♣ - 2♦ - 2NT` vs BBA's `3♣` / `3NT` 138 rows
  (−0.7k / −0.7k); opener's `6NT` jump over the `2NT` positive vs BBA's
  exploration ~140 rows (−0.4k).  BBA's own structure (EPBot defaults,
  `probe-bba-book --card none --prefix="2♣ -"`): `2♦` waiting, natural
  positives on 7–21 with a five-card suit, `2NT` balanced 7+; opener's `2M`
  rebid is passable (responder passes ≤5 without three-card support, `4M` on
  ≤2 with it); `2♣ - 2♦ - 2NT` is 22–23 with `3♣` Stayman on 3+ and pass on
  ≤2.
  - **`notrump.strong_two_notrump_floors` shipped default-on 2026-10-01**:
    plain +0.0009 / +0.0016, PD +0.0006 / +0.0013 IMPs/board (none / both),
    79 / 83 fired (CHANGELOG).
  - **`decision.strong_two_waiting` shipped default-on 2026-10-01** (no
    double negative, `2♥` a positive, the bust passes opener's suit rebid,
    the floor's `2♣` force for opener after `2♦`): pooled two seeds plain
    +0.0007 / +0.0013, PD +0.0004 / +0.0011 (CHANGELOG).  The reading
    mirror pins it off — decoding *their* `2♣` as waiting too lost (plain
    −0.0005 / −0.0008, PD −0.0002 / −0.0011, two seeds), a reading change
    the v6 floor was not trained on; re-measure the unpinned mirror at the
    next retrain, when it would also drop the always-built mirror book.
  - **`response.strong_two_loose_positive` and `rebid.strong_two_positive`
    shipped default-on together 2026-10-01** (BBA's positive — any five-card
    suit or a balanced hand on 7+ — and, below a positive, opener's five-card
    major, the three-card raise and RKCB): pooled three seeds plain +0.0017
    ±0.0009 / +0.0022 ±0.0012, PD +0.0016 ±0.0010 / +0.0021 ±0.0012, 12/12
    cells positive (`scripts/ab-strong-two-loose-positive.sh`, CHANGELOG).
    What the lane taught, so it is not re-derived:
    - *Census first.*  Our `2♦` against BBA's positive was 769 rows, −2.0k /
      −2.2k — bigger than the floor's rebid after a positive (393 rows,
      −0.9k / −1.0k), and inside that rebid the loss is **missed grands** (62
      rows, −451) and **`6NT` where BBA plays `6M`** (80 rows, −183);
      stopping in game where we bid slam is only −101 plain on 169 rows.
    - *The floor's level is at par.*  Double-dummy, slam pays from about 31
      combined HCP in this lane (stops at 29 gain, at 30 lose a little, at
      31+ lose 4–11 a board).  A classical ladder (22–24 `3NT`, quantitative
      `4NT`) lost −0.0029; a slam-seeking one that still authored the level
      washed.  Author strain and keycards, leave the count to the floor.
    - *Neither half wins alone.*  The looser positive alone reads +0.0008 /
      +0.0011 with one seed of three negative (its worst boards are the
      floor's `3NT` on an unbalanced five-card major); the tables alone
      against the old positive wash at 100 fired per 204,800.
  - **`rebid.strong_two_grand` shipped default-on 2026-10-01** (the grand
    rung: seven on two of the three side kings below a positive, through the
    book's asks and below the floor's `4NT` over a minor positive): pooled
    three seeds plain +0.0009 ±0.0005 / +0.0012 ±0.0006, PD +0.0009 ±0.0005
    / +0.0011 ±0.0006, 12/12 cells positive, 71% of the new grands making
    (`scripts/ab-strong-two-grand.sh`, CHANGELOG).  What it taught:
    - *Census by final contract.*  Our shipped arm against BBA's table on
      the three `strong-two-loose` seeds (the boards we open `2♣` and hear a
      positive, 1,228,800 boards): −2,316 plain / −2,828 PD, of which
      missed grands 235 rows, −2,022 / −2,010; missed small slams 166 rows,
      −616; our extra slams 961 rows, **+830** plain / +326 PD.  The lane's
      deficit was the grand, not the level below it.
    - *Kings, not points, once the keycards are in.*
      `probe-strong-two-grand` (8M self-play deals, 27,427 in the lane):
      with five keycards and the trump queen, `7M` makes on 43% / 69% / 86%
      with one / two / three side kings and `7m` on 30% / 64% / 84%; without
      the queen an eight-card fit is 21% / 32% / 42%.  Combined HCP below
      30 is the one weak cell on two kings (49%, ~130 boards per 8M) and is
      not gated.  `7NT` on the same boards trails the suit by 8–16 points of
      make rate, so the rung plays the suit.
    - *The floor's ask is fine to keep.*  Book rows below the floor's `4NT`
      over a minor positive take over its ladder; their small-slam
      placements agree with the floor's on all but ~90 boards per 8M, net
      +27 IMPs.  The price is the floor's own 37-point sevens with fewer
      kings, now stopped in six: 30 boards per 8M, −79 IMPs.
  - **`rebid.strong_two_positive_notrump` shipped default-on 2026-10-03**
    (over `2♣ - 2NT`, a hand with no five-card major bids `3NT` to 24 HCP,
    `6NT` on 25–29, `7NT` on 30+; responder raises the `3NT` to six on 9+
    and seven on 14+, the `6NT` to seven on 11+): pooled three seeds plain
    +0.0005 ±0.0004 / +0.0005 ±0.0005, PD +0.0006 ±0.0004 / +0.0006
    ±0.0006, 11/12 cells positive (`scripts/ab-strong-two-positive-notrump.sh`,
    CHANGELOG); the same-seed self-play pair reads +0.00055 ±0.00013 /
    +0.00067 ±0.00015 on 8M deals.  What it taught:
    - *The direct `6NT` jumps were a level leak, not a missed grand.*
      Per 8M self-play deals (seed 20261003, 27,524 in the lane), 9,868
      boards ended in `6NT` with no keycard ask.  By combined HCP `6NT`
      makes on 28% / 43% / 60% / 77% / 89% at 29 / 30 / 31 / 32 / 33, and
      seven on 57% / 71% / 88% at 34 / 35 / 36.  With both hands' HCP known,
      stopping at 30 and below is worth +5,875 IMPs and seven at 35+
      +3,462 — and `2♣ - 2NT - 6NT` alone (2,860 boards, 58% making) holds
      +4,262 of the first.  The direct jumps over a suit positive
      (`2♣ - 2♠ - 6NT` and its three siblings) make 66–74%, and stopping
      there is worth about +700 between the four.
    - *The looser positive changed the verdict on authoring the level.*
      The floor's `6NT` there is the evaluator net's, bid on 22 opposite
      the positive's floor; under the old 8+ positive that was at par
      (rounds 1–2 above), under the 7+ one opener's 22–24 opposite 7 makes
      27% on 744 boards.  The count is authored for this one auction only.
    - *The grand share is small.*  The new sevens are 46 boards per 8M for
      +306 IMPs; the floor's own sevens on 26–29 that the table now stops
      in six are 80 boards for −78.
  - Next in the lane, sized off the same probe: the grands behind the
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
    - ~~**Responder's `7NT` on 14 over the counted `3NT` can be off an
      ace**~~ — **decided 2026-10-03 by jdh8: keep 14.**  `7NT` on 36
      combined, one point under the textbook 37; an ace off is an
      accepted stake.  (`2♣ - 2NT - 3NT - 7NT` on K92.T642.KQ9.KQJ
      opposite AQJ3.KQJ3.AJT6.A, 36 HCP, seed 1's worst board.  The probe
      prices the row at 82% of 28 boards per 8M; 15+ would cost about
      −180 IMPs per 8M double-dummy.)  The `6NT - 7NT` row on 11 is the
      same 36 and stays too.
    - **A double of the keycard answer drops the asker to the floor in
      every RKCB lane** (`rkcb_rows` registers `{answer} -` only): `2♣ - 3♦
      - 3♠ - 4♠ - 4NT - 5♣ (X) 5♠` on a cold grand.  **Built and measured
      2026-10-03, a non-win, parked on `park/rkcb-doubled-answer`** (jdh8's
      call; `slam::HEARD` and `scripts/ab-rkcb-doubled.sh` live there,
      results in `ab-results/rkcb-doubled{,-2}`): every key below an answer
      registered under `(X)` too, the double ignored.  A wash on both
      scorers with a negative lean, two seeds, 2,048,000 boards per arm per
      vulnerability: plain −0.00005 ±0.00007 / −0.00006 ±0.00009, PD
      −0.00006 ±0.00007 / −0.00008 ±0.00009 IMPs/board, 52 / 58 fired
      (26–29 per 1M, not the trace's one in 614,400), −1.8…−2.7 per fired.
      `main` keeps the floor's blind sign-off.  The flip plan is in the
      park commit: make the asker read the double.  The trace found no
      wiring hole; what it found instead, neither built:
      - **Opener's ask over a splinter raise is thin** — 31 of the 52 fired
        boards are `1M - splinter - 4NT`, for −66 of the −94 IMPs.  Over
        all lanes, where the book bid six and the floor signed off at five,
        six made on 12 boards and failed on 17 (none, plain; both contracts
        failed on 9 more).  The doubled sample only exposes it:
        undoubled, both arms bid the same six.  Census the undoubled lane
        before touching the ask's gate.
      - **A doubled `5♦` is none, not three** — partner held none on 10 of
        12 boards where the asker with one or two keycards read three (the
        doubler tends to hold the `♦A`, a keycard partner then cannot
        have).  A sign-off there recovers about +40 of the −94 IMPs on this
        sample: twelve boards, some six per 1M, too few to build or to
        measure on.
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
- **Re-cut 2026-10-03 (`46d0dc14`) — the preemptive minor raise.**  The
  same prefix cut over the re-anchor found one more unauthored node: over
  our weak `1m - 3m` the floor's raise rung bid `4m` on any 13+ hand, where
  BBA passes on 11–16 and bids `3NT` on 17–21 balanced (`1♣ - 3♣ - 4♣` 155
  rows, −658 plain / −886 PD; `1♦ - 3♦ - 4♦` 136 rows, −278 / −454, per
  409,600 boards).  **Shipped default `Some(16)` 2026-10-03** as
  `response.preemptive_minor_raise_pass` (pass to the ceiling, `3NT` on a
  balanced 17+, the rest to the floor): two seeds pooled plain +0.0016 /
  +0.0020, PD +0.0022 / +0.0030 IMPs/board (none / both), 286 / 293 fired
  per 409,600 (`scripts/ab-preemptive-minor-raise.sh`, CHANGELOG).  Not
  tried: the contested tail (`1m - 3m (X)` and their overcall stay the
  floor's) and BBA's `4m` on an unbalanced 16–20, which the floor already
  bids.
- **Re-ranked 2026-10-03 at `46d0dc14`.**  The bucket is still #1 on the
  shipping arm on both scorers, and a fifth smaller: −25,495 plain / −26,677
  PD on 34,991 rows (−31,106 / −33,752 at `494f0c4b`); headline, tables and
  the paired window in [bba-gap-campaign.md](bba-gap-campaign.md).  The
  prefix cut over every shipping-arm bucket, per 409,600 boards (a scratch
  join again: rows to their shard auctions, leading passes folded):

  | lane | rows | plain | PD | note |
  | --- | --- | --- | --- | --- |
  | responses to `1♥` / `1♠` | 8,062 | −9,855 | −11,113 | still the largest, still spread thin (10,970 rows, −10,951 / −12,258 before Drury).  Worst pairs: our forcing `1NT` vs BBA's `3♥` over `1♠` 438 rows (−1,046 / −924), our `1♠` vs BBA's `2♠` over `1♥` 316 rows (−848 / −855); both looked up and the lane re-cut 2026-10-05, below |
  | our `2♣` opening, every continuation | 2,277 | −3,604 | −3,582 | halved by the lane's six ships (3,054 rows, −7,003 / −6,630) |
  | `1♦ - 1♠ -` opener's rebid | 2,161 | −2,518 | −3,080 | as before: `2♦` vs BBA's `1NT` 760 rows, vs its `2♣` 847 (the measured-wash opt-in) |
  | `1♥ - 1♠ -` opener's rebid | 1,147 | −2,436 | −2,329 | `3♥` vs BBA's `4♥` 128 rows (−354 / −387); no pair above 200 rows |
  | **`1m (1♥) X -` opener's rebid** | 507 | −1,323 | −2,726 | **new** — the floor jumps to game on a minimum; below |
  | **`1♥ (2♠) 3♠ -` opener's answer** | 58 | −700 | −908 | **new, a book bug** — opener passes the cue raise; below |
  | responses to our weak `2♠` | 789 | −867 | −1,606 | our raises where BBA passes or raises lower (`4♠` vs `3♠` 89 rows, `3♠` vs pass 268): obstruction, which DD cannot price — not a lane for this harness |
  | `(2M) - (2NT) 4NT` | 77 | −671 | −714 | not new: the floor-rail series' R4b, a measured wash ([floor-rail-campaign.md](floor-rail-campaign.md)) — do not retry |
  | `1m - 3m -` | 156 | −628 | −868 | shipped after this snapshot (above) |

  The two new lanes, both since shipped:
  - **`1♥ (2♠) 3♠ -`: opener passes partner's cue raise.**  A book node at
    depth 4 whose only finite call for a minimum is `3♥`
    (`probe-decision "8.AJ742.QT7.AT53" "1♥ 2♠ 3♠ -"` → `3♥`, rule `0+
    HCP`), which is not a legal call over `3♠`; the dump shows a pass, and
    we play `3♠` in their suit — −12 IMPs a row where BBA bids `4♥` (50
    rows) or `3NT` (8).  A scan of every book pass over `1x (y) cue -` in
    the snapshot finds no other lane: the weak jump overcall in spades over
    our `1♥` is the one cue raise that outranks three of our major.
    **Shipped 2026-10-03** (`scripts/ab-cue-raise-sign-off.sh`, seed
    1791029195, `ab-results/cue-raise-sign-off-2`): the sign-off is the
    cheapest bid of the major above the cue, chosen per node (`4♥` here and
    over the jump cue `1M (1NT) 3NT`, `3M` everywhere else).  Plain +0.0011
    / +0.0015, PD +0.0018 / +0.0020 IMPs/board (none / both), 33 / 34 fired,
    all in this lane.  The minor table already floats its decline rung
    (`3m`/`4m`), and `1♠ (3♥)` sits above the major package's `2♠` overcall
    cap (the floor's), so no sibling is owed.  A `min_level_is`-anchored
    first build won the same but drifted opener's `4M` reading in every
    other node (CHANGELOG); see the table's doc comment.
  - **`1m (1♥) X -`: the floor jumps to game on a minimum.**  Opener's
    answer to the Modern negative double rides the floor by design
    ([competitive-book.md](competitive-book.md): "safely *because Modern's
    double shows the major*").  It bids `4♠` with four spades where BBA
    bids `1♠` / `2♠` (312 rows, −966 / −1,861) and `3NT` where BBA bids
    `1NT` / `1♠` / `2♣` (195 rows, −357 / −865): JT75.AQT.KJ973.T bids
    `1♦ (1♥) X - 4♠` at weight 1.45 over `2♦` 1.10 and `1♠` 1.05, with
    partner read as exactly four spades and 6–37 HCP.  The weights are the
    deterministic floor's, not the net's: `PROBE_FLOOR=instinct
    probe-decision` gives the same three numbers and names the rules —
    #6 (`4+ ♠, 11+ points`) for the `4♠`, #2 (`13+ HCP, stopper in their
    suit`) for the `3NT`.  Both read partner's double as game values it
    does not promise.  Ceiling ≈ +0.003 plain / +0.007 PD per board: the
    sharpest unworked lane in this cut, about twice the preemptive minor
    raise on PD.  Proposed default: an authored answer table (the spade
    raise by strength, notrump by range, the minor rebid) behind a knob at
    that one node, with a fresh-seed A/B vs BBA; responder's continuations
    over it must be checked before the measurement, not after.  The
    alternative is gating rules #6 and #2 on what the double shows, which
    moves every auction they fire in.  The `(1♠)` and `(1♦)` siblings were
    not cut, and the instinct arm's rows for this lane were not counted.
    **Shipped default-on 2026-10-03** as `competition.modern_double_answer`
    (`modern_answer.rs`, `scripts/ab-modern-double-answer.sh`, seed
    1791031962): the authored table at `1m (1♥) X -` and `1m (1♠) X -`, from
    what BBA's book and BEN's dumps agree on — both raise to `2♠` on a
    four-card minimum, complete in `1♠` on three, bid `1NT` 12–14 with a
    stopper, and **never pass** (the floor also converted the double with
    four hearts, in front of the overcaller, 12–20% of the lane).  Plain
    +0.0019 / +0.0023, PD +0.0059 / +0.0065 IMPs/board (none / both), 592 /
    608 fired.  The doubler's second turn stays the floor's (probed sane).
    The `1♣ (1♦) X -` both-majors sibling **shipped default-on 2026-10-03**
    as `competition.modern_double_both_majors` (seed 1791035800): plain
    wash +0.0002 / +0.0002, PD +0.0007 / +0.0008, only 56 fired per
    204,800.  Its doubler's second turn is authored too, because the net
    floor's was not sane there.  Not built: the `2♥`/`2♠` strength cue on 16+
    (BEN's tool for the strong raise), and the splinters.
- **Re-cut 2026-10-05 (`46d0dc14`) — responses to `1♥` / `1♠`.**  Shipping
  arm rows joined to their auctions, responder's first call to an
  uncontested `1M`, per 409,600 boards.  BBA's meanings are its own 2/1
  defaults (`probe-bba-book --card none --prefix="1♠ -"`): `1♠ - 3♥` is
  natural and invitational, 9–11 HCP with six hearts; `1♥ - 2♠` a strong jump
  shift, 15+ with five spades; `1♠ - 2♥` promises five; `1M - 2m` four.
  - **The 2/1 suit choice (1,738 rows, −3,212 / −2,742) washed.**  Ours race
    on weight, clubs before diamonds before hearts, whatever the lengths; BBA
    bids the longest (a four-card tie up the line, a five-card tie the
    higher).  BBA's whole order washed (seed 1791188672); split by the changed
    call, `2♥` for a minor over `1♠` gained +229 plain on 244 boards and `2♦`
    for `2♣` lost −393 on 210.  The heart half alone,
    `response.two_over_one_hearts_first`, washed over two seeds (plain +0.0001
    / −0.00002, PD +0.00015 / +0.00001; CHANGELOG) and stays opt-in.  **A
    first-call census prices BBA's continuations along with its call**: −1,568
    plain on the `2♥` pairs became ≈ +0 against our own continuations.
  - **A hole the first build found:** after `1M - 2♦ - 3♦` we reach `6NT` /
    `7NT` on hands where `1M - 2♣ - 2♦ - 3♦` finds the diamond slam (the
    −393 above, mostly 5-5 minors).  Not traced further.
  - **Left in the lane, not built:** `1♠ - 3♥` invitational (438 rows vs our
    forcing `1NT`, −1,046 / −924, ≈ −925 of it on 10–11 HCP; it costs our weak
    `3♥`, which is +49 plain / −154 PD against BBA's pass); `1♥ - 1♠` with four
    spades and a longer minor on a game force, where BBA bids the minor (419
    rows, −535 / −474); our game-forcing `2♣` where BBA bids `1NT` (448 rows,
    −521 / −874, mostly 12 HCP and shapely 11s — the `Points13` gate); the
    strong jump shift (316 rows, −848 / −855, slam misses on 16–17 HCP with
    five spades; it costs our weak `2♠`, +113 / +181).  The weak jump shifts
    as a family are a plain wash and −1,003 PD against BBA's pass.  Read the
    first lesson before pricing any of these off the census alone.
- Pool at `7e0bc648`: #1 on **both** scorers on the BBA shipping arm
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
- The slam-side cheap levers are spent; the 2026-09-30 passed-hand lane above
  is the one cheap lever left in the bucket. Even a 10% capture of the pool
  ≈ +0.01/board — more than any single rail.

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

- **`1M - 2M`: the slam try through a game try — shipped 2026-10-02**
  (`response.major_raise_slam_try`, default on).  Found by
  `probe-seam-lookahead`'s static control: the `4NT` ask on 22+ ended in
  `5M` going down one time in five or `6M` making 58%.  A 22+ hand with a
  four-card side suit now makes the long-suit try instead.  Vs BBA, three
  seeds, 614,400 boards/vul: plain +0.00038 ±0.00040 / +0.00044 ±0.00049,
  PD +0.00044 ±0.00041 / +0.00052 ±0.00050, 12/12 cells positive.  Residue,
  none of it measured as a leak: after an accept opener mostly passes and
  that washes against the ask (+0.10 / +0.41 plain per board), so an authored
  ask there has no prior; the 38-in-600k one-suited 22+ hands still ask.
- **A rejecting table does not fall through under a rebase** (found
  2026-10-01, doc/code discrepancy).  `rows.rs` and
  [bidding-architecture.md](bidding-architecture.md) say an exact node that
  rejects a hand falls through to the floor.  Reached through a
  `ReplaceNext` rebase (systems on over their double) it does not: the
  fall-through pass skips only the *original* auction's exact node, resolves
  the rebase again, and lands on the same rejecting table — all-−∞, and the
  driver passes.  `Trie::resolve_floored`'s own `ponytail:` note names the
  ceiling.  It cost the first cut of `strong_two_positive` five boards per
  204,800 (opener passing `2♣ (X) 2NT`); that package now carries a total
  node at `2♣ (X) positive`.  Every other deliberately partial table under
  a systems-on rebase (the quantitative 15+/17+ holes) is exposed the same
  way, unverified.  Proposed fix: carry the rejected classifier, not a
  `skip_exact` flag, through `resolve_at` and skip it by identity wherever a
  rebase lands on it — three twins (`trie.rs`, `book.rs`, `decoder.rs`) —
  with a `smoke-default` byte-identity proof, or an A/B if it moves boards.
  Default until decided: leave the core alone, keep rebased tables total.
  **Guarded half fixed 2026-10-03** (`Trie::resolve_with_mass`): a rejecting
  guarded table (every `P*` row table — it bit `1♣ (1♦) X - 2♣ -`, Lebensohl
  package A, and the Landy doubler) is now skipped and the walk goes on to
  the floor; `smoke-default` byte-identical at 20k (seed 1) and 100k (seed
  777000).  The rebase half is still open, and now **measured to move
  boards**: skipping a rejected rebase moved 5 per 100,000, all
  `1NT (2♣) 2♦ - 2♥ -`-type systems-on rebases, because the skip lands on
  the **contested net floor** rather than the instinct ladder the rewritten
  auction gets uncontested — `2♠` on 3 HCP where uncontested passes.  The
  fix that matches "systems on" is to classify the constructive ladder in
  the rewritten context.  **Measured 2026-10-03, closed — keep the Pass.**
  Three policies vs today's Pass, vs BBA, seed 1791042704, 819,200
  boards/arm/vul: skip to the contested floor, re-classify on the rewritten
  auction (instinct), or skip into an authored generic residue table
  (game / invite / pass by HCP).  The lane barely exists vs BBA (its `(2♣)`
  over our 1NT is artificial): 11–12 / 2–3 / 1 boards moved per arm, every
  delta 0.0000 IMPs/board, raw totals all at or below Pass (floor −29 / −20
  PD, instinct −6 / −29, residue −6 / −8 none / both).  The variants were
  deleted, not kept as knobs.  **Flagged, untraced:** the boards instinct
  moved were mostly `1♠ (X) 2♣ - 3♣ -`-type, which systems on over their
  double rewrites to uncontested `1♠ - 2♣ - 3♣ -` — a 2/1 game force — so
  re-classifying there drove to game opposite a responder the double left
  non-forcing.  Today's Pass is right on those boards by accident.
  **Ruled 2026-10-04 (jdh8): no 2/1 after their X** — strong hands with no
  fit redouble, so a new suit is weaker than in every other case.  The
  book already said so for responder's first call and opener's answer
  (`over_their_double.rs`); responder's *second* turn still replayed the
  2/1 tree, so it is now authored at `1o (X) 2y - 3y -`
  (`competition.weak_new_suit_rebid`, `scripts/ab-weak-new-suit-rebid.sh`):
  **shipped default-on 2026-10-04** on the naturalness tiebreak — vs BBA
  (seed 1791047769, 409,600 boards/arm/vul) the arms bid every board
  identically; the node came up 5–6 times per vul, always on a minimum.
  Opener's strong hands over the weak `2y` followed
  (`competition.weak_new_suit_extras`,
  `scripts/ab-weak-new-suit-extras.sh`): **shipped default-on
  2026-10-04**, plain +0.0002/+0.0003, PD +0.0003/+0.0003 IMPs/board,
  all four CIs > 0.  Responder's first call was the larger lever: `2y`
  now needs six cards (`competition.weak_new_suit_length`,
  `scripts/ab-weak-new-suit-length.sh`), **shipped default 6 2026-10-04**,
  plain +0.0006/+0.0008, PD +0.0009/+0.0011, all four CIs > 0.  Opener's
  answer to the natural `1NT` was the rebase's forcing notrump; opener now
  passes a balanced ≤17 (`competition.doubled_notrump_pass`,
  `scripts/ab-doubled-notrump-pass.sh`), **shipped default-on 2026-10-04**,
  plain +0.0001/+0.0001, PD +0.0002/+0.0002, all four CIs > 0; and the
  `1NT` denies a five-card suit (`competition.doubled_notrump_max_length`
  4, **shipped 2026-10-04**, plain wash/+0.0006, PD +0.0012/+0.0023).  The
  `1♦ (X) 3♦ (3♠) X - 3NT` pull was an instinct-floor artifact (closed,
  [competitive-book.md](competitive-book.md)'s deferrals).  The reading
  twin of the rejection fall-through shipped 2026-10-04 on correctness (a
  table with no row for the call made no longer authors its reading;
  `scripts/ab-reading-fallthrough.sh`, 11/6 boards moved per 819,200, a
  wash on both scorers).
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
