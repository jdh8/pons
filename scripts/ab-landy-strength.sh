#!/bin/sh
# ab-landy-strength.sh — §N1q: the Landy counter's two-level majors sorted by
# STRENGTH instead of by major shortness, and opener's doubles over it.
#
#   JOBS=24 BOARDS=4608000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-strength.sh ab-results/landy-strength \
#       >ab-results/landy-strength.log 2>&1 < /dev/null & disown
#
# Three arms, SEQUENTIAL, one fresh SEED_BASE shared across all of them.  Full
# design in docs/one-notrump-competitive.md §N1q.  In one screen:
#
#   base  today's `main` — the shipped N1j BBA ladder.  `2♥`@178 / `2♠`@177 are
#         game-forcing both-minor takeouts naming EXACTLY a doubleton in the bid
#         major; `3♥`/`3♠`@176/175 are the 0-1 splinters.  A weak both-minor
#         hand has no call and an invitational four-four hand has none either.
#   str   `competition.defense_2c_landy_strength_majors`: `2♠`@177 is the
#         4+♣4+♦ invitational-or-better band (four-four allowed, `points(8..)`,
#         unlimited above), above the transfers; `2♥`@141 is the weak five-four
#         band down beside the `2♦` escape it outranks; the splinters move to
#         179/178.  Neither two-level rung claims anything about the majors, so
#         both answer tables are new (minors only over `2♥`; `2NT`/`3m`
#         over `2♠`) and the shortness ask is gone.
#         RUN 2 (the flip arm, 2026-09-18, `ab-results/landy-strength2`, measured a non-win — see VERDICT): the
#         8-9 half of `2♠` needs a singleton-or-void major and no six-card
#         minor; opener's `3NT`@160 accept over `2♠` is GONE; the `5m` over
#         opener's minor pick needs thirteen, not ten.  Run 1's build is the
#         VERDICT block's.
#   strx  `+ defense_2c_landy_strength_doubles`: opener's `X`@120 over their
#         `(2♠)` raise of our weak `2♥` is TAKEOUT (3+3+ minors, no spade
#         stopper), and opener's `X`@140 over their `(3♥)`/`(3♠)` raise of our
#         `2♠` is PENALTY (`comp:landy-penalty`, `.penalty()`, 4+ of the major
#         they raised).  Both are explicit rules — nothing in this lane
#         mechanises the pass/double polarity (docs/pdi.md).
#
# SIZING.  4,608,000 boards/arm/vul (24 x 192,000), byte-for-byte the sizing of
# every §N1-lia run, so all of them stay power-comparable; MDE ~0.002
# IMPs/board.  Both knobs are OFF by default, so each needs a WIN, not a wash.
#
# THE CENSUS THAT SIZED IT (2026-09-18, `ab-results/landy-lia3/base-none`,
# 4,608,000 boards, table A only; the throwaway scanner is not kept).
# `1NT (2♣)` reaches responder on 821,414 boards (17.83%), and of those:
#
#   responder's hand            | boards  | % of all | today's call
#   ----------------------------+---------+----------+-----------------------
#   5-4+ minors,   0-7 points   |  81,765 |   1.77%  | P 59.6, 2♦ 21.5,
#                               |         |          | 2NT 10.8, 3♣ 8.2
#   4-4  minors,   0-7 points   |  52,937 |   1.15%  | P 100.0
#   4-4+ minors,   8+ points    | 123,110 |   2.67%  | X 28.0, 3♥ 17.1,
#                               |         |          | 3♠ 15.2, P 12.8,
#                               |         |          | 2♥ 10.9, 2NT 5.3,
#                               |         |          | 3♣ 3.7, 2♠ 3.4,
#                               |         |          | 3NT 1.9, 2♦ 1.8
#
# So the strong rung's single biggest donor is the VALUES DOUBLE (28% of the
# band), which is §N1p's convicted mechanism in miniature, and the weak rung's
# is the pass.  The four-four weak band keeps passing by design: it has no
# second suit to run to and the `2♥` rung is gated five-four.
#
# PRE-REGISTERED ARBITRATION.  Plain DD is primary at BOTH colours, PD is a
# reported column.  `str` moves traffic OFF our penalty doubles, so a PD loss
# here is NOT waved through as the auto-double artifact — it is the one PD loss
# measurement.md says to read straight (the mechanism removes our doubles).
# sd only if plain is worth reading.
#
# FALSIFIERS, pre-registered:
#   1. **The weak `2♥` fires and loses at both-vul.**  This is lia3's exact
#      catastrophe one rung over (`- → 2♥` −42,572 plain / −107,283 PD, routed
#      through a `3NT`@160 accept).  Two things differ here — the weak answers
#      have NO notrump rung at all, and the shape floor is five-four, not
#      five-four-with-exactly-four-diamonds.  If it fires anyway, the flip arm
#      is a `!vulnerable()` gate on `2♥`: a one-line knob edit.
#   2. **The strong `2♠` loses to the values `X` it displaced.**  Bucket the
#      `X → 2♠` cell by responder's own major lengths; the lia2 forensic found
#      that cell genuinely mixed (two-plus in each major: +1.854/+1.154 plain
#      for the takeout, −1.305/−2.169 PD), and §N1p found the same split
#      monotone in the SHORT major.  If it loses, the repair is a major term on
#      the `2♠`, not a retreat from the band.
#   3. **Phantom suits in the floored seats after `2♠`.**  The floor reads an
#      artificial `2♠` as spades bid (no alert column in its vector) — the
#      named class of lia3's loss.  Every seat this package does not author
#      after `2♠` is exposed; `probe-layer-replay` splits book from floor.
#   WATCH (no number): `2♥ (3♥)` / `2♥ (3♠)` are deliberately the floor's
#   (lia2's sell-out conviction found the floor RIGHT on that class), and the
#   weak band never speaks again after them.
#
# READING GATE (pre-launch, 2026-09-18): PASSED.  `probe-call-reading
# --their-2c-landy --ns-landy-strength "1N (2C) 2S P" "1N (2C) 2H P"
# "1N (2C) 3H P" "1N (2C) X P" "1N (2C) 2N P"` — read at OPENER's seat, which
# is the four-call spelling; the three-call one reads the ADVANCER's partner
# and answers about their overcall, which is the trap the lia3 gate documented.
#
#   2♠  -> points 8.. , ♣ 4.. , ♦ 4..          (the rule verbatim)
#   2♥  -> points 5..7, ♣ 4..5, ♦ 4..5         (floors + the transfers' cap)
#   3♥  -> points 10.., ♣ 4.., ♦ 4.., ♥ ..1    (the splinter, unmoved)
#   X   -> points 8..9, ♣ ..5 , ♦ ..5          (NEW shape half: 4-4+ at 8+ now
#                                               bids 2♠, so the double excludes
#                                               it — no new slug, the reading
#                                               falls out of `bid_exclusion`)
#   2NT -> points 2.., ♣ 6..                   (the transfer, unmoved)
#
# Scoring: plain AND perfect defense off docs/measurement.md's decision table.
# `probe-divergence --gate-opener ours` must read 0 foreign BEFORE any
# headline.  Resumable; SEED_BASE persists in $R/landy-strength.seed.  Resume
# with the same two env vars.  Iron rule: do NOT edit `src/` or run any cargo
# build while this runs.
#
# The bucket forensic is manual and post-hoc, over the kept arm dirs:
#
#   ./target/release/examples/probe-divergence \
#       $R/str-both $R/base-both --imps --jsonl $R/imps-both.jsonl
#   ./target/release/examples/probe-layer-replay $R/str-both \
#       --jsonl $R/imps-both.jsonl --out $R/layers-both.jsonl --ns-landy-strength
#   python3 scripts/divergence-buckets.py $R/imps-both.jsonl
#   python3 scripts/divergence-layers.py  $R/imps-both.jsonl $R/layers-both.jsonl [veto]
#
# ============================ VERDICT ============================
#
# RUN 2026-09-18, SEED_BASE=1789675210, sha dd2dd0ad, 4,608,000 bd/arm/vul,
# all four isolation gates 0 foreign.  MEASURED LOSS for `str`; `strx` is a
# small win on its own base and moot on a losing parent.  Both stay OFF.
#
#   str vs base     plain             PD                sd plain / sd PD
#   NV              +0.0056 ±0.0007   −0.0115 ±0.0009   +0.0168 / +0.0026
#   both-vul        −0.0029 ±0.0008   −0.0192 ±0.0010   +0.0101 / −0.0040
#   strx vs str
#   NV              +0.0003 ±0.0001   +0.0005 ±0.0001   +0.0004 / +0.0006
#   both-vul        +0.0001 ±0.0001   +0.0001 ±0.0001   +0.0003 / +0.0003
#
# FORENSIC (probe-divergence --imps + divergence-buckets.py, both cells;
# per-cell major-length cuts by a throwaway script, not kept):
#   Falsifier 1 did NOT fire: `- → 2♥` wins plain at both colours (+6,049 BV /
#   +15,286 NV; PD −15,443 / −7,981), and so does `2♦ → 2♥`.
#   Falsifier 2 DID: `X → 2♠` is −16,865 plain BV (−0.0037/bd, more than the
#   whole headline) and −1,605 NV, monotone in responder's SHORT major exactly
#   as §N1p found — per fired, BV: 2-3 −1.160, 1-4 −1.800, 2-2 −0.763, 1-3
#   −0.495 (NV +0.492); 1-2 +0.255, 0-4 +1.311, 0-5 +2.873.  By opener's
#   answer the cell's loss is the NOTRUMP rungs: `2♠ - 3NT` −10,403 (−5.18
#   per fired; 3NT fails on 1,434 of 2,009 boards), `2♠ - 2NT` −8,564
#   (−1.38, half of it responder's pass of a failing 2NT); the minor picks are
#   −0.20.  Two secondary cells: `2♥ → 2♠` (N1j's GF takeout hand) −5,110 BV /
#   −3,754 NV, all of it `2♠ - 3m - 5m` — responder has NO authored rebid over
#   opener's minor pick and the floor jumps to game at −3.2/fired; and
#   `3♣ → 2♠` (the 6+♦ transfer hand, majors 1-2/0-3) −4,712 / −5,406 — the
#   `2♠`@177 shadows the transfer and opener's `3♣` pick strands a 6-4 in the
#   four-four fit.
#
# FLIP ARM (built 2026-09-18 as run 2 — see the `str` arm description): (a) a major term on `2♠` — a singleton-or-void
# major (`len(♥, ..=1) | len(♠, ..=1)`) returns 2-2/2-3 to the values X, and
# both minors ≤5 returns the six-carders to their transfers; (b) opener's
# `3NT`@160 accept over `2♠` gated harder or removed (24-26 combined opposite
# their two known majors is not a game); (c) responder's rebid over opener's
# `3m` pick authored (3NT / raise / pass).  Re-bucket `X → 2♠` by short major
# after (a) before deciding (b).
#
# RUN 2 — the flip arm — LAUNCHED 2026-09-18 07:28Z, SEED_BASE=1789716492,
# sha dd2dd0ad-dirty (the flip build uncommitted), JOBS=30, 4,608,000
# bd/arm/vul, `ab-results/landy-strength2`.  Reading gate re-run and PASSED:
# `2♠` -> points 8.. ♣4.. ♦4.. (the Or projects to its hull — sound, not
# tight), every other call byte-identical to run 1's gate.
#
# RUN 2 VERDICT (2026-09-19, all four gates 0 foreign) — a NON-WIN; both knobs
# stay off, §N1q closes as measured twice.
#   str  vs base  NV    plain +0.0067 ±0.0006  PD −0.0031 ±0.0008  sd +0.0109 / +0.0026
#   str  vs base  both  plain +0.0006 ±0.0007  PD −0.0076 ±0.0009  sd +0.0050 / −0.0023
#   strx vs str   NV    plain +0.0003 ±0.0001  PD +0.0006 ±0.0001  sd +0.0004 / +0.0007
#   strx vs str   both  plain +0.0002 ±0.0001  PD +0.0002 ±0.0001  sd +0.0003 / +0.0004
# NV is the decision table's win|loss row (a doubling artifact), both-vul is
# wash|loss; a default-off knob needs a win.  Forensic (probe-divergence --imps
# + divergence-buckets.py + band/shape cuts): the flip repaired what it named —
# `3♣ → 2♠` is gone, `2♦ → 2♠` flipped to +1,024/+2,175 plain, `X → 2♠` halved
# and reads +410 plain / +6,148 PD non-vulnerable — but the pre-registered
# falsifier FIRED both-vul: `X → 2♠` −7,409 plain, all of it through opener's
# `2NT`@150 (−7,753 on 4,845 boards; pass −1.81/fired, pull to 3♣ −1.62, to
# 3♦ −1.43) while the direct picks read +1.00 (3♦) / −0.68 (3♣); donors are
# the 1-4-4-4s (−1.87/fired) and the 1-3 shapes — the boards where OFF's
# values X is converted by §N1m's penalty double of their advance (2♥x/2♠x
# −2/−3 at −9/−12 a board).  The SCORED RE-SOLVE (2026-09-19, `imps2-*.jsonl`,
# probe-divergence now writes score/tricks/DD; `scripts/divergence-sit.py`)
# puts the rest in THREE FAMILIES OF FLOOR PHANTOMS at unauthored nodes, not
# in the convention's contracts: (i) our 3NT in the weak rung's contested
# tails (`2♥ (3♥) X - 3NT`, `2♥ - 3♣ - - (X) - - 3♦ - 3NT`, …: 3,103/4,243
# boards, PD −27,292/−28,263 — more than the rung's whole deficit; the quiet
# 3m is +13,525/+12,535 plain, −3,300/+1,822 PD); (ii) opener's 4♣ over the
# MAKING 3NT raise at `2♠ - 2NT - 3NT -` (860/1,105 boards, −5,006/−4,398
# plain; sat +171/+332) — the whole `2♥ → 2♠` loss, not fix (c); (iii)
# responder's 3NT at 8-9 over their raise of 2♠ (1,448/2,754 boards, PD
# −7,499/−9,349; sat +2,785/+4,102).  Sat together (--sit 3NT@0-9 --sit
# 4♣@10-13 --sit 4♦@10-13): NV +0.0075/+0.0055, both +0.0029/−0.0005.
# Flagged, not fixed: the shared, unauthored
# `2NT - 3♣ - 3M - 3NT -` node pulls to 4♣ in the ON arm only (`- → 4♣`
# −2,795/−2,781, −6.7/−5.5 per fired; run 1 had the same ~400 boards) — the
# floor reading the knob's regime input, owned by the base lane.
# THIRD ARM (credible, not built, the user's call): Pass rails at the three
# phantom families; a colour gate on the 8-9 band is the fallback for the
# vulnerable `X → 2♠` residual.  Details in the doc's §N1q.
# NOTE: this file was edited while run 2 executed it (the header grew), so dash
# resumed at a stale offset after the last `log` and ran a comment fragment as
# `se` — the `exited 127` in landy-strength2.log is that, results complete.
# =================================================================
#
R=${1:?usage: ab-landy-strength.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-strength)
log "=== landy-strength SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

# Plain-DD first, sd second — the §N1-lia two-pass order: the primary cells
# land first and the lead-model column follows only if the headline earns it.
for v in none both; do
    arm base "$v" --filter-landy
    arm str  "$v" --filter-landy --ns-landy-strength
    arm strx "$v" --filter-landy --ns-landy-strength --ns-landy-strength-doubles

    gatepair str  base "$v"
    gatepair strx str  "$v"
    diffpair str  base "$v"
    diffpair strx str  "$v"
done

for v in none both; do
    sddiff str  base "$v"
    sddiff strx str  "$v"
done

log "landy-strength done"
