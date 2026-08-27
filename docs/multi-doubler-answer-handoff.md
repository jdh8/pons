# Opener's answer to the K–K doubler's natural major — the measured hole

**Status 2026-08-27:** `competition.multi_doubler_major` shipped default-on
with a **negative both-vulnerable perfect-defense cell**. The traced repair,
`competition.multi_doubler_notrump`, **shipped default-on 2026-08-27** after
winning all four cells (§ *The repair, measured*) — but it recovers only
**~20%** of the parent's both-vulnerable deficit, so that cell is improved,
not retired. This document is why the cell exists, why it is not the rung's
fault, what the repair bought, and what is still owed. The next rung down —
`competition.multi_doubler_minimum_notrump`, the 15-count's `2NT` — **shipped
default-on 2026-08-27**, also a win in all four cells (item 2). The answer
table's ladder is now complete on the `2♠` leg; the `3♥` leg and the rest of
the both-vulnerable cell (item 3) are what remain.

## What shipped, and what it measured

`ab-results/2d-multi-doubler/`, `SEED_BASE=1787740671`, sha `96ecfd9a`,
`PER_SHARD=192000` × 12 = 2.304M bd/arm/vul, `--their-2d-multi --filter-1nt`,
arms `base` (no rung) vs `dm` (rung on). Isolation gate `--gate-opener ours`
read **0 foreign** at both vulnerabilities (0/787, 0/510).

| vul | fired | plain DD | PD | sd plain | sd PD |
| --- | ---: | ---: | ---: | ---: | ---: |
| none | 787 | +3.344 | +0.610 | +4.412 | +2.168 |
| both | 510 | +1.927 | **−1.737** | +3.399 | +0.243 |

IMPs per fired board. SE ≈ 5.39/√n, i.e. 0.192 (NV) and 0.239 (both), so every
figure except sd-PD-both is clear of resolution.

**The rung's own design claim is confirmed exactly.** Of 1 297 divergent
boards, **100% are "bid where the baseline passed" and 0% "passed where the
baseline bid"** — weight 100 never moved a call the shipped table already made.
336 of the 787 no-vul divergences became games the baseline never reached, and
0 went the other way.

**The negative cell is the decision table's `win | wash/loss` row**
([measurement.md](measurement.md)) — *"Doubling artifact… Suspect; don't ship
on this evidence."* The domain addendum does not excuse it: this knob **bids
more**, which is the direction that row was written for, so PD is pricing our
own overbids rather than being blind to a benefit. The sd-lead tie-breaker
lifts the cell to +0.243/fired (t ≈ 1.0) — a wash, not a win.

jdh8 shipped it anyway on 2026-08-26. That ruling stands; this document is the
repair queue, not a re-litigation.

## The traced cause — an incomplete answer table, not the rung

[measurement.md](measurement.md) step 10 says trace the worst boards before
declaring a loss dead, and names *an unauthored continuation* as the usual
culprit. It is the culprit here. Four of the five worst both-vul PD boards are
**not failing games** — they are two-level partscores in a bad fit:

```
N:AQ.KQ9.QJ975.Q82  E:532.AT7632.AK2.7  S:K984.854.83.AJ96  W:JT76.J.T64.KT543
on:  1NT (2♦) X (2♥) - - 2♠ - - -                                    [-14 IMP]
off: 1NT (2♦) X (2♥) - - -
```

South shows four spades. North holds **AQ doubleton** — and
`kokish_kraft_doubler_major_answer` is exactly

```
4M  @140  len(other, 4..) & hcp(16..)     game in a known 4-4
3♠  @130  len(♠, 4..)                     the invitational raise, spade leg only
P   @0                                    everything else
```

There is **no notrump rung and no escape**, so a 15–17 balanced hand with a
stopper in *their* major and two or three cards in *ours* must pass `2♠` and
play a 4-2 or 4-3. Vulnerable, that is −200 where `3NT` was cold and defending
their partscore was +100.

The table's original justification — *"a pass on three or fewer, where the 4-3
at the two level still beats selling out to their resolved partscore"* — was
written when the rung sat at weight 100 and therefore only stopperless hands
could reach it. It is a weight-100 argument, and it did not survive contact
with perfect defense at unfavourable vulnerability.

## What is already built

`competition.multi_doubler_notrump` (**default on since 2026-08-27**) is the
unbundled arm 1 below: the same rung, gated on its own knob so it could be
priced against the *shipped* default rather than against the split's
re-weighted (148) natural major. Flag `--ns-multi-doubler-notrump` on
`bba-gen`, `PROBE_MULTI_DOUBLER_NOTRUMP=0` on `probe-decision` (the disarming
direction, now that it ships on), script `scripts/ab-2d-multi-doubler-nt.sh`,
pinned by `kk_doubler_notrump_repairs_the_answer_table`.

## The repair, measured

`ab-results/2d-multi-doubler-nt/`, `SEED_BASE=1787749549`, sha `4810aa6e`,
`PER_SHARD=384000` × 12 = **4.608M bd/arm/vul** (double the parent run's),
`--their-2d-multi --filter-1nt`, arms `base` (shipped default) vs `nt` (plus
opener's notrump out).

| vul | fired | plain DD | PD | sd plain | sd PD |
| --- | ---: | ---: | ---: | ---: | ---: |
| none | 167 | +2.910 | +2.096 | +3.508 | +2.933 |
| both | 106 | **+4.264** | **+3.264** | +5.477 | +4.729 |

IMPs per fired board. SE ≈ 5.39/√n = 0.417 (NV) and 0.523 (both), so every
cell clears resolution by 3–4×: `win | win` at both vulnerabilities, with the
16-world single-dummy tie-breaker agreeing in the same direction.

Both isolation gates read **0 foreign**. Of 273 divergent boards, **100% are
"bid where the baseline passed"** and **270 reach a game the baseline never
bid** (164/167 and 106/106) — the design claim, confirmed as exactly as the
parent's was.

**The hypothesis was right, and it was not the whole cell.** The both-vul cell
is the *larger* of the two, which is what "the negative cell is these
pass-outs" predicts. But the surfaces are not the same size: the rung fires on
1 board in 43 000 at both-vul against the parent's 1 in 4 500, so in per-board
terms it returns +75 µIMP/bd against the parent's −384 µIMP/bd — about **20%**
of the deficit, leaving an estimated −309 µIMP/bd standing. (Different seeds
and board counts, same generator and filter, so this composition is an
estimate, not a measured cell.)

Under `competition.multi_px_split` (default off, **measured a loss 2026-08-27**) the answer table
gains

```
3NT @135  hcp(16..) & stopper_in(major)
```

below the `4M`@140 game in a known 4-4 and above the `3♠`@130 invite, so it
fires on exactly the hands that used to pass. It is **knob-gated deliberately**:
unconditionally it would change the behaviour the 2026-08-26 A/B just measured,
which would make the shipped number a lie.

Pinned by `kk_px_splits_the_double_by_information`
(`src/bidding/american/competition/rubensohl/tests.rs`) on the losing board
verbatim — `AQ.KQ9.QJ975.Q82` bids `3NT` under `px_split` and passes without it
— plus two assertions that 135 steals nothing from `4♠`@140 or `3♠`@130.

## Owed, in priority order

1. **The unconditional arm — SHIPPED 2026-08-27, default-on.** Won all four
   cells; see § *The repair, measured*. Original sizing note kept below,
   because its reasoning was validated: predicted ~1 in 60 000 at both-vul,
   measured 1 in 43 000.

   *(historical)*
   `competition.multi_doubler_notrump`, two arms (`base` = shipped default,
   `nt` = plus the rung), `--their-2d-multi --filter-1nt`, isolation gate
   first. The hypothesis is specific and falsifiable: *the both-vul PD cell is
   the 4-2/4-3 pass-outs, so authoring the notrump out should move that cell to
   non-negative without touching the no-vul win.*

   Sized at **4.608M bd/arm/vul** (`JOBS=12 PER_SHARD=384000`), double the
   doubler run's, on the smoke's own reading: 120 000 boards at both-vulnerable
   diverge on **2**, i.e. ~1 in 60 000 against the parent rung's 1 in 4 500.
   The seat is a subset of the parent's pass-outs (`hcp 16+`, a stopper, and
   fewer than four of ours — the rest take `4M`@140 or `3♠`@130), so the
   surface is roughly an order of magnitude thinner and the handoff's original
   2.304M would have reached ~40 boards. Both smoke divergences are "bid where
   the baseline passed" with **0 foreign** on `--gate-opener ours`. Results in
   `ab-results/2d-multi-doubler-nt/`, `SEED_BASE=1787749549`.
2. **The 15-count — SHIPPED 2026-08-27, default-on. Won all four cells.**
   `competition.multi_doubler_minimum_notrump`. `multi_doubler_notrump` floors
   at `hcp(16..)`, so a 15-count with a stopper and short support passed `2♠`
   into a 4-2 or 4-3 — board 1 of that run's worst tail
   (`A82.A53.KJ75.K74`, 15 HCP, three spades). The rung, on the `2♠` leg only:

   ```
   2NT @120  hcp(15..) & stopper_in(major)     opener, below the 3♠@130 invite
   3NT @140  hcp(10..)                         responder accepts — 25 opposite a known 15
   ```

   `ab-results/2d-multi-doubler-min-nt/`, `SEED_BASE=1787769822`, sha
   `3107742e`, `PER_SHARD=384000` × 12 = **4.608M bd/arm/vul**,
   `--their-2d-multi --filter-1nt`, arms `base` (the `hcp(16..)` floor) vs
   `minnt`.

   | vul | fired | plain DD | PD | sd plain | sd PD |
   | --- | ---: | ---: | ---: | ---: | ---: |
   | none | 144 | **+2.007** | **+1.597** | +1.986 | +1.816 |
   | both | 92 | **+2.011** | **+1.413** | +2.617 | +2.160 |

   IMPs per fired board. SE ≈ 5.39/√n = 0.449 (NV) and 0.562 (both), so
   t = 4.5 / 3.6 and 3.6 / 2.5: `win | win` at both vulnerabilities, with the
   16-world single-dummy tie-breaker agreeing in the same direction in all
   four. Isolation gates **0 foreign** (0/144, 0/92); 100% of divergences are
   "bid where the baseline passed".

   **The designed-for risk showed up and was paid for.** 15 opposite `hcp 8+`
   is 23 combined, so the invitation is sometimes wrong in both directions —
   the worst boards are `2NT` and accepted `3NT` going down — but 48% of
   divergences reach a game the baseline never bid. The both-vul PD cell is the
   thinnest at 2.5 SE, which is where that risk would show, and it stays clear.

   **A benefit the design did not predict, and double-dummy under-counts.** On
   8 boards (6 NV, 2 both) the *baseline* reached a game and the candidate did
   not: passing `2♠` let the opponents balance into a game of their own, and
   `2NT` shut them out. The census reads it as "room handed to the opponents:
   more in the baseline, 9 boards".

   Flag inverted on ship to `--no-ns-multi-doubler-minimum-notrump` /
   `PROBE_MULTI_DOUBLER_MINIMUM_NOTRUMP=0`; `kk_arm()` pins it off so every
   existing K–K test keeps its meaning and `kk_min_nt` stays the contrast.
   Pinned by `kk_doubler_minimum_notrump_bids_the_fifteen_count`.

   *Sizing lesson, paid for on this run:* a 120 000-board smoke read **5**
   divergences and sized the surface at 1 in 24 000; the truth is 1 in 32 000
   (NV) and 1 in 50 000 (both). Five boards is Poisson noise — treat a smoke
   that thin as order-of-magnitude only, and never shrink an arm on one.

3. **The rest of the both-vul cell — NOW THE HEAD OF THE QUEUE.** Item 1
   answered its own half: the pass-outs were worth ~20% of the deficit, so
   ~80% is still unexplained and the `win | loss` row is **not** retired. The
   next suspect is the one this document already named: the `3♥` leg's
   `hcp 16+` game answer, which has no invitational rung under it and so bids
   game on 24 combined at the four level. Trace it the way the pass-outs were
   traced — worst boards of `ab-results/2d-multi-doubler/` both-vul PD, minus
   the 4-2/4-3 pass-outs item 1 has now removed.

## Do not re-derive

- The census that motivated the rung (293 bd / −824 plain on the `X` branch's
  pass-outs; +1230 PD on the `-` branch's) is in
  [one-notrump-competitive.md §N4-KK](one-notrump-competitive.md#inside-the-two-big-branches--where-x-and--actually-bleed-2026-08-26).
- The rung's leg-by-leg mechanism (why `X (2♥) - (2♠)` is excluded, why
  `X (2♠) - -` was withheld) is in the same section and in
  `kokish_kraft_entries`.
- The `multi_px_split` design, including why the 148 re-weight is a *reading*
  literal as well as a routing one, is
  [§N4-KK "The `P`/`X` information split"](one-notrump-competitive.md#the-px-information-split--competitionmulti_px_split-measured-loss-2026-08-27-stays-default-off).

## Operational note, paid for on this run

`scripts/bba-gen-parallel.sh:40` runs `cargo build --release` at the head of
**every arm**, so the iron rule "never rebuild binaries while an A/B is in
flight" is really *"never edit `src/` while one is in flight"*. Export
`SKIP_BUILD=1` for the whole run — `ab-lib.sh` still builds once at the top —
and start on a clean tree. This run was killed and restarted for exactly that
reason, and a second time because `pkill` left the first run's twelve workers
**orphaned** (reparented to init) racing the restart on the same shard paths
with a different seed. Verify with `ps -eo pid,ppid,args | grep bba-gen` that
exactly one generation is live before trusting an arm.

**Applied 2026-08-27** (jdh8's ruling): `ab-lib.sh` now `export`s
`SKIP_BUILD=1` immediately after its own single `cargo build`, so every
`scripts/ab-*.sh` that sources it is immune by construction. Details and the
two hand-fixed direct callers in
[shared-machine-data-gen.md](shared-machine-data-gen.md#check-for-live-abs-before-building-or-editing).
