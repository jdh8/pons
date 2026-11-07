#!/bin/sh
# ab-strong-jump-rebids.sh — the continuations over opener's strong jump
# rebids after a one-level response, the `6141cdb2` anchor's sharpest round-2
# cluster (1,198 boards, −2,035 plain / −2,359 PD).
#
#   answers  (default-on; off = --no-ns-minor-jump-shift-answers)  responder's answers to the
#            `1m - 1M - 3x` jump shift and opener's rebids over them
#            (`rebid.minor_jump_shift_answers`); off, the floor passes the
#            game force on a weak hand and opener passes `3NT` with a 6-5.
#
# Round 1 (2026-11-07, seed 1794039684800295301) also carried a `shape` arm
# (`rebid.major_jump_rebid_shape`): it won 2/2 plain + PD and shipped
# default-on, so it is in both arms now.  `answers` was inconclusive with a
# negative point (−0.0008 / −0.0017 plain); round 2 measures its repair
# (the 11+ `4NT` major raise, opener's 21+ ask over `4M`, the six-card pull
# of `3NT`): non-inferior 2/2 (seed 1794059060547829818), shipped
# default-on 2026-11-07 on naturalness.  Kept for the record: re-running it
# measures the shipped default against the floor.
#
# Round 3 (`rebid.minor_jump_shift_slam`) measured the slam rungs over the
# shipped answers: won 4/4 (seed 1794063398144400977), shipped default-on
# 2026-11-07, so `base` is now the control with the rungs withheld:
#
#   slam     (off = --no-ns-minor-jump-shift-slam)  direct `6NT` and the `6NT` pull
#            on 14+ HCP, 11+ minor raises, a flat 4333 kept in `3NT`,
#            opener's 21+ ask over `3M`, the grand rung over a minor raise.
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-strong-jump-rebids.sh ab-results/strong-jump-rebids-r3 \
#       >ab-results/strong-jump-rebids-r3.log 2>&1 < /dev/null &
#
# Each new seed wants a fresh results dir.
R=${1:?usage: ab-strong-jump-rebids.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for strong-jump-rebids)

log "=== strong-jump-rebids A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in $VULS; do
    arm base "$vul" --no-ns-minor-jump-shift-slam
    arm slam "$vul"
    gatepair slam base "$vul"
    diffpair slam base "$vul"
done
log "=== strong-jump-rebids A/B done"
