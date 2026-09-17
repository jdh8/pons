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
#   str   `competition.defense_2c_landy_strength_majors`: `2♠`@177 is the whole
#         4+♣4+♦ invitational-or-better band (four-four allowed, `points(8..)`,
#         unlimited above), above the transfers; `2♥`@141 is the weak five-four
#         band down beside the `2♦` escape it outranks; the splinters move to
#         179/178.  Neither two-level rung claims anything about the majors, so
#         both answer tables are new (minors only over `2♥`; `2NT`/`3m`/`3NT`
#         over `2♠`) and the shortness ask is gone.
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
# NOT YET RUN.
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
