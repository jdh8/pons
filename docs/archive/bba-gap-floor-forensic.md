# Regressed `floor#N` rows — the ea2cde9 → 53a3c254 anchor forensic (closed 2026-08-18)

> **Closed (2026-08-18).** Extracted verbatim from `docs/bba-gap-campaign.md`
> at `1ed0f58d`. All four work items are settled — B2.4 rejected in isolation,
> B2.5 retrain-gated, B2.6 sound with no repair, B2.7 rejected — so this is a
> method record: how a cross-snapshot "regression" list decomposes into churn /
> other-table / here, why the rule that names a bucket is never the rule that
> moved, and the two mechanisms (`ReadingScope::All` displacing the natural walk;
> the reading feeding a net whose break-even is the decision). The reusable loop
> (`scripts/anchor-diff.py`, worktree replay with `PROBE_FLOOR=instinct`) lives
> in the live [../bba-gap-campaign.md](../bba-gap-campaign.md) runbook; the one
> surviving work item (B2.5's `walk_shape` axis mask at the next matched retrain)
> is in its *Open work*.

The 35 rows the `53a3c254` re-anchor flagged (35 of 487 buckets worse on at
least one scorer, every one a `floor#N` or fallback row — see the re-anchor
paragraph in [bba-gap-anchor-history.md](bba-gap-anchor-history.md)) were filed
as a queue, not a regression list.
`scripts/anchor-diff.py` turns one into a verdict: it joins two snapshots'
`boards.jsonl` on `(vul, seed, board)` — the seed series deals the same boards
forever — and splits each bucket's move into the boards that stayed, entered and
left.

```sh
python3 scripts/anchor-diff.py ab-results/anchor/2026-08-12-ea2cde9-dirty \
                               ab-results/anchor/2026-08-17-53a3c254
python3 scripts/anchor-diff.py A B --bucket 'Defensive / floor#20 / balancing' \
                               --lane here --show 8
```

It reproduces the 35 exactly — the threshold behind that number is **≥300
divergent boards in both snapshots**; without it, 215 of the 465 shared buckets
are worse on a scorer — and Σ over its buckets matches `report.md` to the IMP
(−432,364 → −408,168 plain, −490,599 → −463,429 PD, the +24,196 / +27,170 the arm
gained). Mechanisms were named by replaying each board against a build of
`ea2cde9` (`git worktree add`, `probe-decision` back-ported — it did not exist
then) using `PROBE_FLOOR=instinct`, which this pass added to
[probe-decision](../../examples/probe-decision/main.rs): under the shipped net floor
a `floor#N` row prints `(floor / no rule)` and the forensic stalls.

**Three corrections to the paragraph above.**

1. *"every one of them is a `floor#N` bucket"* is wrong: three of the 35 are
   guarded fallbacks — `Competitive/fallback@3/round-1`, `fallback@3/round-2` and
   `fallback@4/round-2`, the last the largest genuine regression in the set.
   *"No book bucket regressed"* does hold.
2. The two caveats named (churn, small n) miss the largest mislabeller, below.
3. The drift runs the **opposite** way from the guess. It is not that
   "tightening what the floor reads moves floor rules both ways" — the reads that
   moved mostly got **looser**, and the loosening is what costs.

## The third confound: which table the rule bid at

A board is bid twice — at `table_a` our pair sits N/S, at `table_b` it sits E/W
(`bid_out(ours, opponent, conv_is_ns)`) — and `div_index` is the first index at
which the two auctions differ. A row's `our_call` is therefore taken from
*whichever table our pair held that seat at*, so a `floor#N` rule fired at exactly
one of the two tables. A bucket's Δ can then move entirely because of our bidding
at the **other** table, on a board the bucket does not own.
`Defensive/floor#64/round-1` is the clean case: of its −306/−400, only −79/−99 is
the slice its own rule can be blamed for; −130/−188 is the other table. The script
splits `stayed` into `here` and `other-table` for this reason, and `--lane here`
keeps only the boards a bucket owns.

**The split over all 35:**

| slice | plain | PD | reading |
| --- | --- | --- | --- |
| churn (entered/left) | **−1700** | **−2121** | not these rules — an *earlier* call moved and the boards changed buckets |
| other-table | −278 | −201 | our bidding at the table this rule did not bid at |
| `here` | **−845** | **−900** | the only floor work in the set |

Twenty-one of the 35 rows are churn, nine other-table, **five** carry their loss
on boards they own. The window's honest price is **−845 plain / −900 PD** over
~750 boards against +24,196 / +27,170 gained — **3.5%**, not a regression list.

**Three of the four rows the re-anchor named worst are churn**, and the script
names where their boards came from:

| bucket | Δ plain = here + other + churn | the churn |
| --- | --- | --- |
| `Defensive/floor#146/round-1` | −91 = **+18** + 1 − 110 | 18 boards newly divergent, 9 in from `Defensive/floor#3/round-1`; 8 out to `floor#148/round-1` |
| `Defensive/floor#382/balancing` | −187 = −32 − 16 − 139 | **59 in from `Competitive/floor#3/round-2`**, 11 from `floor#30/round-2`; 42 out to `Competitive/floor#61/round-2` |
| `Competitive/floor#3/round-1` | −526 = −32 + 9 − 503 | **44 in from `Competitive/floor#31/round-1`**, 23 newly divergent |

`floor#146` actually got *better* on the boards it kept. Only
`Competitive/floor#46/round-2` of the four is real, and even there the loss is a
continuation, not the `floor#46` call.

## The rule that names the bucket is never the rule that moved

Across every bucket traced, the `floor#N` call itself replays byte-identically in
both arms; the changed call is always a **later continuation**, usually partner's
answer. Counted exhaustively per bucket: 146/146 boards for
`Defensive/floor#20/balancing`, 84/84 for `Defensive/floor#61/round-2`, 78/78 for
`Competitive/floor#382/balancing`, 42/42 for `Defensive/floor#64/round-1`, 40/40
for `Competitive/floor#46/round-2` — **0 "at the bucket call" in every one.** A
`floor#N` bucket names where the auction diverged from BBA, not where our play got
worse. Fixing the named rule would have been wasted work in all five.

## Mechanism, part 1: `ReadingScope::All` displaces the natural walk

Replaying the 175 worst `here` boards across seven buckets, **175/175 reproduce**
their arm under the paired builds, and `PROBE_SCOPE=alerted` alone recovers the
pre-window call on **106**. None of `PROBE_CEILINGS=0`, `PROBE_BID_EXCLUSION=0`,
`PROBE_FORCING_CEILING=0` moves anything on its own; `PROBE_UPGRADE_CLOSURE=0` is
occasionally needed beside the scope flip.

| bucket | here plain/PD | here boards | `SCOPE=alerted` restores the call | verdict |
| --- | --- | --- | --- | --- |
| `Competitive/fallback@4/round-2` | −143/−282 | 171 | 24/25 sampled, upheld on a 40-board audit | reading drift |
| `Defensive/floor#20/balancing` | −154/+11 | 146 | **131/146** (exhaustive) | reading drift |
| `Competitive/floor#382/balancing` | −134/−136 | 78 | **61/61** of the dominant family | reading drift |
| `Competitive/floor#46/round-2` | −96/−136 | 69 | 16/25 | mixed |
| `Defensive/floor#46/round-2` | −66/−81 | 27 | 9/25 | mixed |
| `Defensive/floor#61/round-2` | −69/−164 | 84 | **25/84** with all five knobs (exhaustive) | not reading |
| `Defensive/floor#64/round-1` | −79/−99 | 42 | **4/42** — while the *read* is restored 42/42 | not reading |

Each row was re-derived by an independent pass told to refute it, on boards the
first pass had not cited. Two survived unchanged (`fallback@4`, `floor#382`), one
was corrected from "one cause" to "three, one of them outside the reading layer"
(`floor#20`, 131 of 146), and `floor#64`'s reading verdict was overturned outright
— see part 2.

The defect is the one [authored-reading-handoff.md](../authored-reading-handoff.md)
predicts: under `All`, an **unalerted natural call's authored rule takes over its
reading and projects weaker than the natural walk it displaced**. `floor#64`'s own
rule reads `1♠ is the cheapest bid, 5+ ♠, 8–16 points`, yet partner reads that same
1♠ as `♠ 4..13, points 0..37` — weaker than the rule's own constraint, and weaker
than `PROBE_SCOPE=none`'s `♠ 5..13, points 8..37`. `floor#20`'s balancing 2♣
(`5+ ♣, 10–16 points`) projects to *nothing*. It runs the other way too:
`fallback@4` has an unalerted simple raise read as an exact `♠3..3` and a pass read
with a raised point floor.

**The sharpest instance, and it is not confined to these buckets: the `1♦` opening
loses its length floor under the shipped default.**

| opening | `scope=All` | `scope=Alerted`/`None` |
| --- | --- | --- |
| `1♣` | `♣3..13`, pts 12..21 | `♣3..13`, pts 10..21 |
| **`1♦`** | **`♦0..13`**, pts 12..21 | **`♦3..13`**, pts 10..21 |
| `1♥` | `♥5..13`, pts 11..21 | `♥5..13`, pts 10..21 |
| `1♠` | `♠5..13`, pts 11..21 | `♠5..13`, pts 10..21 |

Only `1♦`. `All` buys a real strength read (an `hcp` floor and a tighter `points`
floor) and pays a length floor — and that length floor is what gates
`3+ ♦ shown by partner` on the raise rules, so the loss lands on **every 1♦
auction in the system**, not only the balancing bucket that surfaced it.

## Mechanism, part 2: the reading feeds a net whose break-even is the decision

Sixty-nine of the 175 boards do not come back under any reading knob, and the
per-bucket restoration rate swings from 131/146 to 4/42 under the *same* knob.
That spread is the clue, and the refutation pass found what unifies it.

`Defensive/floor#64/round-1` states the puzzle: swept over **all 42** of its `here`
boards, `PROBE_SCOPE=alerted` restores partner's and RHO's read to A's
**byte-for-byte on 42/42** and restores the *call* on **4**. Restoring the read is
therefore not sufficient — so the read is an input to something else, not the
decision.

That something is the evaluator net. Under the shipped pair
`accountant_floor: true` / `net_collar: false` — identical in both arms —
`points_or_net` ([instinct.rs](../../src/bidding/instinct.rs), the `points_or_net`
helper) collapses to its **net arm alone**: the authored arithmetic leg never gets
to veto, so the game milestones that answer these floor calls (the fitted-major
rule #148 at weight 150, its 3NT sibling #141 at 140, and the `fit_sum_game` family
generally) are decided by the net's break-even verdict. The window swapped that net
(`evaluator_v3_exclusion` → `evaluator_v5_honest`). So `ReadingScope::All` and the
evaluator swap are **not two independent mechanisms**: the reading change moves the
net's input, and whether the old call comes back depends on which side of the new
net's decision boundary the moved input lands. Where the boundary is far, flipping
the read restores the call (`floor#20`, 131/146); where it is near, restoring the
read changes nothing (`floor#64`, 4/42).

This is the refutation pass's finding, not the first pass's: two independent
skeptics reached it from opposite ends — `Competitive/floor#46/round-2` by showing
the authored strength leg the first trace blamed is *dead code* under the shipped
knobs (rule #148's strength arm is `fit_sum_game`, not `combined_points`), and
`Defensive/floor#61/round-2` by sweeping its whole 84-board lane: **all five
reading knobs together restore the read on 82/84 and the call on 25**, so a
reading-layer mechanism is falsified on 68% of it. `net_collar` is the standing
lever — turning it on lets the authored `fit_sum_game` arithmetic veto the net.

**What this does not prove.** The five `PROBE_*` knobs do not span the window's
reading changes: `pass_exclusion` was **deleted** in the window (`97206fcc`) and
nothing restores it, `their_landy_reading` / `their_multi_reading` /
`completion_alerts` have no knob, and `probe-decision` prints partner's and RHO's
reads but not LHO's. So the unrestorable boards are proven "not the partner/RHO
read", not "not any reading". A ea2cde9-build replay is the only complete
counterfactual, which is why the recipe leads with the worktree.

## Work items

Each is its own fresh-seed A/B per [measurement.md](../measurement.md), and **none is
a fix to the rule its bucket is named after**.

- **B2.4 — the `1♦` length floor — REJECTED in isolation (2026-08-18).** The
  exact candidate added the implied `len(Diamonds, 3..)` atom, restoring
  `♦3..13` under `ReadingScope::All`; it passed the projection/evaluation
  gates and improved fixed-seed partner exclusions from 1.299% to 1.293%
  (2194 → 2184). The pre-registered non-loss A/B did not pass: three seeds
  (1787039750 / 1787040222 / 1787040684), each 204,800 boards/arm/vul, were
  negative in 11/12 cells. Pooled over 614,400 boards/vul, plain/PD were
  **−0.0010 ± 0.0016 / −0.0022 ± 0.0020** IMPs/board non-vulnerable and
  **−0.0024 ± 0.0020 / −0.0024 ± 0.0024** vulnerable; the PD
  non-vulnerable and plain vulnerable CIs exclude zero. Fired rates were
  1.26–1.41%.

  A side split rules out an opponent-reading artifact. All 10,691/9,480
  call-divergent boards (none/both) followed **our pair's** `1♦` opening; the
  aligned BBA-opener subsets (71,869/72,160 boards) had zero call and contract
  divergences. This is expected: `ReadingScope::All` projects our side's
  unalerted calls, while an undeclared opponent's unalerted natural calls stay
  on the walk. The loss is therefore the partner-reading effect the item meant
  to repair, not the candidate helping BBA or tightening our model of them.

  The required `--by node` trace says this is decision calibration downstream
  of a sound read. The largest repeated vulnerable bucket is
  `1♦ - 3♦ 3♠ - - ⟨4♦ vs -⟩` (15 boards, −120 PD IMPs); on a traced
  board responder really has six diamonds opposite four, but the restored
  floor lifts `4♦` only 0.70 logits over Pass. The worst tail (−21 PD) has
  a genuine three-card-diamond opener and four-card responder: the candidate
  cues `4♠` and reaches `6♦`, while control doubles `3♠`. The atom and test
  do not ship; revisit the floor only with an attributable fit/competition
  calibration arm. Candidate smoke `babb6234…` → `ac0bb20e…`; cards and the
  rendered-atom ratchet were byte-identical, contrary to the plan's expected
  re-bless, because the BBA card has only four-/five-card-diamond booleans.

  The read-only sibling census found the same opaque-comparator hole in the
  fully blind `both_majors` relay choices `1NT - 2♣ - 2NT - 3♣/3♦`;
  `longer_major` and Texas retain explicit named-suit floors. No sibling was
  changed here; the relay needs its own A/B.
- **B2.5 — RETRAIN-GATED (2026-08-18).** The extended authored-reading
  soundness sweep passes the two weak-reading probes: `floor#64`'s 1♠
  (`5+ ♠, 8–16 points` → `♠4+, 0+`) and `floor#20`'s balancing 2♣
  (`5+ ♣, 10–16 points` → nothing) are sound but loose. `probe-decision`
  locates both at the union over all face-live rules for that call in the
  monolithic instinct table, which tops the more precise natural walk. A fifth
  isolated tightening arm is pre-refuted by the four-loss ledger above. At the
  next matched policy/evaluator retrain, widen the existing `walk_shape` bit to
  an axis mask so the walk survives only on axes the authored union leaves open.
- **B2.6 — SOUND; NO REPAIR (2026-08-18).**
  `bids_read_within_their_table` now includes Pass and the four exact instinct
  contexts for `floor#64`, `floor#20`, and `fallback@4`'s simple raise and later
  Pass. All 133 probe hands are admitted wherever they win or tie the table.
  The simple raise's `♠3..3` is the sound intersection of the authored
  `3+ ♠, 6–9 points` rule and declining the heavier four-card raise; the later Pass
  adds no further band on the anchor witness. The 40,000-board census remains
  exactly 2194/168,957 partner exclusions (1.299%), and `smoke-default` remains
  `babb6234…`. The conditional B2.6 repair, knob, A/B and re-anchor therefore
  do not exist.
- **B2.7 — REJECTED (2026-08-18).** Turning `net_collar` on to restore the
  authored `fit_sum_game` veto lost all 12 cells across fresh seeds 1787044967 /
  1787045416 / 1787045860 at `4767a5ef`, 204,800
  boards/arm/vulnerability each. Pooled over 614,400 boards/vulnerability,
  plain/PD were **−0.0323 ± 0.0025 / −0.0320 ± 0.0026** IMPs/board
  non-vulnerable and **−0.0445 ± 0.0032 / −0.0419 ± 0.0033** vulnerable; fired
  rates were 1.09% and 1.19%. The worst-tail trace repeatedly shows the collar
  suppressing making games and slams that the v6 evaluator's break-even accepts
  (including cold 7♥/7NT contracts). Keep the shipped net-only default; this
  confirms the forensic's decision-calibration diagnosis rather than an
  unsound reading defect.
- **Accepted as churn:** the other 21 rows, no floor work.

