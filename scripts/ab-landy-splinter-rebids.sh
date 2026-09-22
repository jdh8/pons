#!/bin/sh
# ab-landy-splinter-rebids.sh — §N1r row 1 arm 1: responder's rebids over
# opener's answer to our Landy both-minors splinter (`1NT (2♣) 3♥/3♠ - 3NT -`
# and `… - 4m -`), at none and both.
#
#   JOBS=24 BOARDS=4608000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-splinter-rebids.sh ab-results/landy-splinter-rebids \
#       >ab-results/landy-splinter-rebids.log 2>&1 < /dev/null & disown
#
# The row 1 census (`examples/probe-landy-splinter-oracle`, 2026-09-23, on
# step 0's base dumps) found responder's rebid over opener's `3NT` unauthored
# and the floor bidding `4♠` on the spade void it had just splintered in —
# 2,234 boards at none, 1,140 at both, −214 / −936 plain / PD per board at
# none — plus 518 passes of opener's `4m` game-force answer.  The arm authors
# `Pass` over `3NT` and `5m` over `4m` (knob `landy_splinter_rebids`).
# Contract-level oracle price of the pass node alone: +0.0063 / +0.0082 at
# none, +0.0036 / +0.0045 at both.  Pre-registered falsifier: the floor's
# `6♦` pulls over `3NT` (≈ 850 boards at none) were roughly break-even, so
# the pass gives back ≈ −1.9k IMPs there; a loss would have to come from the
# `(X)` twins or from opener's continuation reading the authored pass.
#
# Arms: `base` = `main`, `rebids` = `--ns-landy-splinter-rebids`.  The base
# arms REUSE step 0's dumps (`ab-results/landy-nt-vs-x/base-{none,both}`,
# seed 1789977169, code ab8bc884): every commit since (cfa536b7 favourable
# gate, face-gated and identity-checked at none/both; 74bed854 knob dropped;
# d0f7222c a probe) is byte-identical to that base at these two colours.
# Seed the results dir before launching:
#   R=ab-results/landy-splinter-rebids; mkdir -p $R
#   cp ab-results/landy-nt-vs-x/landy-nt-vs-x.seed $R/landy-splinter-rebids.seed
#   ln -s ../landy-nt-vs-x/base-none $R/base-none
#   ln -s ../landy-nt-vs-x/base-both $R/base-both
# Plain DD primary; PD is the sharper scorer here (every contract in the
# divergence is ours, undoubled); sd-lead the tie-break.
#
# VERDICT 2026-09-23 (seed 1789977169, control d0f7222c, gates 0 foreign,
# 4.608M bd/arm/vul).  IMPs/board, plain / PD, ±95% CI:
#   none  +0.0055 ±0.0003 / +0.0078 ±0.0004   8,341 fired, +3.03 / +4.29 per fired
#   both  +0.0039 ±0.0003 / +0.0047 ±0.0004   5,939 fired, +3.04 / +3.67 per fired
# A win on all four DD columns; shipped default-on as
# `competition.landy_splinter_rebids` — the flag is now
# `--no-ns-landy-splinter-rebids`, so a rerun's `base` arm needs it and the
# `rebids` arm is `main`.  sd-lead rows in docs/one-notrump-competitive.md
# §N1r arm 1.  The pre-registered falsifier did not fire.
R=${1:?usage: ab-landy-splinter-rebids.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-splinter-rebids)
log "=== landy-splinter-rebids SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base   "$v" --filter-landy
    arm rebids "$v" --filter-landy --ns-landy-splinter-rebids
    gatepair rebids base "$v"
    diffpair rebids base "$v"
done

for v in none both; do
    sddiff rebids base "$v"
done

log "landy-splinter-rebids done"
