#!/bin/sh
# ab-nt-high-overcall.sh — N3: their three-level overcall of our 1NT
# (docs/one-notrump-competitive.md §N3).
#
#   JOBS=12 PER_SHARD=19200 setsid nohup scripts/idle-run.sh \
#       scripts/ab-nt-high-overcall.sh ab-results/nt-high-overcall \
#       >ab-results/nt-high-overcall.log 2>&1 < /dev/null &
#
# What is being tested.  `competition.nt_high_overcall_responses`
# (`bba-gen --ns-nt-high-overcall`) authors responder's one call over
# `1NT (3♣)`–`1NT (3♠)` and opener's one answer to each: a forcing five-card
# suit at the three level, a six-card major at game, takeout `X` with a
# four-card major and `points(8..)`, `3NT`, and the four-level minor priced
# *under* `3NT`.  Today the whole lane is floor-only — the census's top loser
# (405 bd, −1.07 plain/bd, PD negative at both vulnerabilities), where
# responder's new suit reads as nothing to opener and the floor's `X` fires on
# 6–7 HCP but not on 9–11 4-4-4-1.
#
# BBA's three-level calls over a 1NT opening are **natural seven-card
# preempts** (`hcp 4–10`, docs/ai-bidder/bba-1nt-counter-defense.md), so this
# is an ordinary competitive scheme, not a counter-defense — no disclosure
# channel, nothing keyed on their declaration.
#
# Three arms, one SEED_BASE:
#   base    the knob off (the shipped floor-only lane)
#   stop    the knob on, `direct_3nt_stopper` as shipped (true)
#   nostop  the knob on, `--ns-direct-3nt-stopper false` — jdh8's prior is that
#           the direct 3NT needs no stopper of its own, since partner opened
#           1NT.  The knob is *shared* with the two-level Lebensohl lane, so a
#           `nostop` win here buys the three-level table its own gate bit
#           rather than flipping the shared default.
#
# `--filter-1nt` (balanced 15-17 somewhere, a raw-hand test applied BEFORE any
# bidding) rides every arm so they deal the same board set and stay
# seed-aligned for the paired diff.  The `(3x)` bucket is ~0.2% of accepted
# boards, so read the per-fired figure alongside the headline.
#
# Scoring.  The standard gate (docs/measurement.md): plain wash + PD win ships
# default-on, a CI-clear plain loss stays opt-in.  `sddiff` is the tie-breaker.
# Read `probe-divergence --gate-opener ours` BEFORE the headline — nothing here
# is keyed on `their.*`, so the gate is the proof, not the design.
#
# Round 2 (`ROUND=2`), after the package shipped default-on 2026-08-18: two
# increments over the shipped default, both diffed against the reused `stop`
# arm — whose boards are byte-identical to a default-flag regeneration, checked
# before reuse (only the recorded `gen_args` metadata differs).
#   xfer    the `(3♣)` transfer variant (`--ns-nt-3c-transfers`).  That lane is
#           ~100 bd per 409k, so read the per-fired figure and pool a second
#           seed if the CI swallows it.
#   nogate  the three-level table's OWN stopper bit off
#           (`--ns-nt-high-overcall-3nt-stopper false`).  Round 1's `nostop` arm
#           answered this question through the *shared* `direct_3nt_stopper`,
#           and `probe-divergence --gate-opener ours` failed at 36% foreign: the
#           shared bit also governs advancing partner's takeout double of a weak
#           two, where dropping it costs −1.23/−1.92 PD per fired while this
#           lane gains +2.20/+1.62 plain.  Hence the private bit and this arm.
#
# Resumable: an existing arm dir or diff file is skipped, and SEED_BASE persists
# in $R/nt-high-overcall.seed.  Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-nt-high-overcall.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for nt-high-overcall)
log "=== nt-high-overcall SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

if [ "${ROUND:-1}" = 1 ]; then
  for v in none both; do
    # Post-ship spellings: `base` is the pre-ship arm, `stop` the shipped default.
    arm base   "$v" --filter-1nt --ns-nt-high-overcall false
    arm stop   "$v" --filter-1nt
    arm nostop "$v" --filter-1nt --ns-direct-3nt-stopper false
    diffpair stop   base "$v"
    diffpair nostop base "$v"
    diffpair nostop stop "$v"
    sddiff   stop   base "$v"
    sddiff   nostop base "$v"
  done
fi

# Round 2: the two increments over the shipped default, against `stop`.
if [ "${ROUND:-1}" = 2 ]; then
    for v in none both; do
        # Skipped in round 1's dir (already there); generated in a fresh
        # pooling seed's dir, where `stop` IS the shipped default.
        arm stop   "$v" --filter-1nt
        arm xfer   "$v" --filter-1nt --ns-nt-3c-transfers true
        arm nogate "$v" --filter-1nt --ns-nt-high-overcall-3nt-stopper false
        diffpair xfer   stop "$v"
        diffpair nogate stop "$v"
        sddiff   xfer   stop "$v"
        sddiff   nogate stop "$v"
    done
fi

log "nt-high-overcall done"
