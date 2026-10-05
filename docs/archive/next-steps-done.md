# Next steps: done items

Shipped, refuted and closed items moved out of [../next-steps.md]( ../next-steps.md)
on 2026-10-06, verbatim (relative links rebased). Each lane's unworked residue
stayed there. Section names follow the source document.

## Item 2 — RKCB / slam accuracy

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
  - Flagged while tracing (decided or parked):
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
- **Re-ranked 2026-10-03 at `46d0dc14` — the two new lanes, both since shipped:**
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
    ([competitive-book.md](../competitive-book.md): "safely *because Modern's
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
  - **BBA's `1♠ - 3♥` (438 rows vs our forcing `1NT`, −1,046 / −924) was a
    hole one round later, now shipped.**  After `1♠ - 1NT - 2x` responder had
    no long-suit invite, so 10–12 with six hearts passed `2♣` or bid
    `2NT` / `3NT`.  `rebid.forcing_notrump_suit_invite` (the `3♥` jump, opener's
    `4♥` / `3NT` / pass) shipped default-on 2026-10-05: plain +0.0005 /
    +0.0007, PD +0.0005 / +0.0006, two seeds (CHANGELOG).  It keeps our weak
    `1♠ - 3♥`.  The same jump in a minor lost (opener passes `3m` where `2NT`
    found `3NT`).  Unworked beside it: `1♠ - 1NT - 2♥` has no raise of opener's
    hearts (a 10-count with six passes `2♥`; 12 rows, ≈ −56), and over `1♠ -
    1NT - 3♠` responder bids `3NT` on seven or eight hearts (18 rows, −67).
  - **`1♥ - 1♠` with four spades and a longer minor on a game force, where
    BBA bids the minor (419 rows, −535 / −474), shipped default-on 2026-10-05**
    as `response.two_over_one_minor_before_spades`: the minor first, a natural
    `2♠` next round, opener's no-fit `3♥` / `3NT` authored.  Plain +0.00025 /
    +0.00046, PD +0.00022 / +0.00042, two seeds, all eight cells positive but
    inside the CI (CHANGELOG).  Beside it: after `1♥ - 2m - 3♥` (six
    hearts and 15+ points; the rule has no suit-quality term) responder has no
    `3♠` and raises to `4♥` on two, losing a 4-4 spade fit (the worst board,
    −14).  `rebid.two_over_one_side_suit_first` (opener shows a four-card side
    suit before the jump) measured a wash and **shipped default-on
    2026-10-05** on naturalness: plain +0.00034 / +0.00053, PD +0.00029 /
    +0.00049, two seeds, all eight cells positive inside the CI, and a third
    seed on the shipped build a wash (CHANGELOG).  The gain is `2♥` over `1♠` and the
    minors; `2♠` over `1♥` lost on both seeds (−42 / −61 plain on 25 / 26
    boards), because `1♥ - 2m - 2♠ - 3♠` meets the second-suit keycard ask on
    `points(15..)`, which that opener always holds — **the next lever is that
    gate** (`game_force/second_suit.rs`, `opener_third_agree`), then re-measure.
    Also seen there: under the wider reading the keycard answerer raises the
    asker's `5M` sign-off to `6M` (17 / 18 boards, net −1 / −8 plain).
    `rebid.two_over_one_reverse_extras` (the `2♠` reverse needs 15+, as in
    BBA and BEN — none of their 263 reverses is below 15 HCP — and the minimum
    rebids `2NT`) measured a null the same day and **shipped default-on** on
    naturalness (jdh8's call: it is what the other natural bidders play):
    three seeds disagree in sign and sum to +15 / +14 plain IMPs, 0 / +2 PD,
    on 614,400 boards (CHANGELOG).
    **The keycard gate swept 2026-10-05 and refuted** (a throwaway knob,
    deleted with its script; `ab-results/second-suit-keycard`, seed
    1791206765): 17 loses plain
    −0.0003 / −0.0004 and 18 −0.0008 / −0.0010 (CI-clear), because every
    lane but the reverse pays — opener with 15–17 signs off and a 16+
    responder passes `4M`, responder's seat being the floor.  The lever that
    fell out is **responder's own ask** over the sign-off (also deleted;
    `ab-results/second-suit-responder-ask-2`, `-3`): n = 16 at gate 15 is all-positive on 8 boards per vul (+3 to +4 per
    fired, inside the CI) and a wash on seed 2 (0 / −13 IMPs on 7 / 6) —
    pooled +24 / +16 plain on 15 / 14 boards in 409,600, a wash; paired
    with gate 17 it recovers most but not all of the gate's loss (n = 16
    plain −0.0001 / −0.0001, n = 17 −0.0002 / −0.0003).  The 2/1 second-suit
    keycard seats are closed as a lever: the fit is found at the three level
    and the ask on 15 is already where the slams are.
    Unworked: five spades and a six-card minor still bid `1♠`.
- **Re-cut 2026-10-05 (`46d0dc14`) — opener's rebid over `1m - 1♠`.**  The
  lane's largest pairs are our `2m` against BBA's `1NT` (`1♦`: 760 rows, −817
  plain / −1,097 PD; `1♣`: 995 rows, −453 / −540, per 409,600 boards), and
  every row is a 5m-4♥ opener: BBA's `2m` promises six.  Over our `2m`
  responder's seat is the floor and never shows four hearts — the lost 4-4
  fit is −504 / −294 plain.  `rebid.unbalanced_1nt_rebid` (the `1NT` rebid,
  plus responder's `3♥` over XYZ's `2♦ - 2NT`) measured a wash with a
  positive lean over two seeds, all eight cells positive (plain +0.0005 /
  +0.0004 and +0.0008 / +0.0006, PD +0.0001 / +0.0001 and +0.0005 / +0.0002;
  CHANGELOG).  **Shipped default-on 2026-10-05** on naturalness (jdh8's
  call: BBA's treatment).  Its losing classes are the style's own: responder passing `1NT`
  with a diamond fit, and the invitational `2NT` after the relay.
  Unworked beside it: the rest of the `1♣ - 1♠` `2♣` vs `1NT` pair (5♣4♦,
  no four hearts, BBA `1NT`; the 1435 cell gains for us), and `1♦ - 1♠ - 3♦`
  vs BBA's `3NT` (191 rows, −360 / −296).
- Pool at `7e0bc648`: #1 on **both** scorers on the BBA shipping arm
  (−36,474 plain / −42,267 PD ≈ −0.10/board; was #1 PD only at `c3bb94a7`,
  −45,145); vs BEN
  −1.58 plain / −1.75 PD per divergent board, the worst PD/div of the BEN
  top-3, ratio only 1.4–1.5× (i.e. a genuine shared weakness, not a
  BEN-strength artifact).
- **New lever, found 2026-09-27:** `slam::rkcb_rows` authors nothing for the
  **answerer** after the asker's `5M` signoff. Wherever the strong hand
  answers (responder asks), the answerer falls to the floor and raises the
  signoff — `5♠ - 6♠`, `5♥ - 7♥` on the forcing-NT jump-shift trace
  ([bidding-options.md](../bidding-options.md), `set_forcing_nt_jump_shifts`).
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
  [ai-bidder/bba-kickback.md §7.16](../ai-bidder/bba-kickback.md)).** On the v6
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

## Owed / deferred — not retrain-gated

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
  [bidding-architecture.md](../bidding-architecture.md) say an exact node that
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
  [competitive-book.md](../competitive-book.md)'s deferrals).  The reading
  twin of the rejection fall-through shipped 2026-10-04 on correctness (a
  table with no row for the call made no longer authors its reading;
  `scripts/ab-reading-fallthrough.sh`, 11/6 boards moved per 819,200, a
  wash on both scorers).
- ~~Trace the shipping-only constructive move in the `7e0bc648` window~~
  **Closed 2026-09-26**: it is the rails at the mirror table (a board is
  bucketed by its first divergence, so our defence where BBA opened is
  charged to our opening bucket); 2NT-double +3,614 of the +5,178 PD. The
  reading ships were not involved. Table in
  [bba-gap-campaign.md](../bba-gap-campaign.md), current ranking.
- The game-pull rail reads their bid's level, not its meaning: its worst A/B
  boards are their artificial `5♣`/`5♦` (keycard replies, cues). An
  alert-exempt variant is one small A/B; the shipped rail wins without it.
  **Closed 2026-09-26**: built as `their_game_pull_alert_exempt` (a "code"
  is alerted *and* promises < 3 cards in the strain, since
  `completion_alerts` alerts Smolen/transfer completions too); fires on 1
  board in 204.8k and loses it — code dropped (`83989dd4`). The
  level-reading rail is right.
