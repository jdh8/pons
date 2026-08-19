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

# Round 4 (`ROUND=4`): the two census-queued increments to opener's answer to
# responder's takeout double, each its own knob, both against a common control.
# `--filter-preempt` replaces `--filter-1nt` here: the `1NT (3x)` lane is 14.8%
# of its accepted boards against 0.60%, so an arm buys ~25x the owned sample for
# 33x the (bidding-free) scan.  Arms under it pair only with each other.
#   fit   opener answers their `(3♠)` double `4♥` with four hearts — the rung
#         the default ladder has no room for, and the `(3♠) X (P) 3NT` cell it
#         targets is 20 bd / -94 plain / -123 PD on the fresh-seed anchor.
#   pass  opener may leave the double in with at most one of A/K/Q in their
#         seven-card suit.  BBA's advancer sits for it on 88% (minor) / ~50%
#         (major) and its preemptor never bids again.
if [ "${ROUND:-1}" = 4 ]; then
    for v in none both; do
        arm base "$v" --filter-preempt
        arm fit  "$v" --filter-preempt --ns-nt-high-overcall-x-major-at-four true
        arm pass "$v" --filter-preempt --ns-nt-high-overcall-x-leave-in true
        diffpair fit  base "$v"
        diffpair pass base "$v"
        sddiff   fit  base "$v"
        sddiff   pass base "$v"
    done
fi

# Round 5 (`ROUND=5`): the leave-in **v2**, re-gated on length after the v1 gate
# was refuted and its own dumps were re-sliced by opener's holding
# (docs/one-notrump-competitive.md §N3, "Round 6 — the leave-in re-sliced").
# `base` is today's shipped default, so the fit rung (`x_major_at_four`) is ON
# in every arm here — the boards it takes are the worst block in the v1 slice
# and they are already gone.
#
# The two candidate disjuncts are SEPARATE arms, not one bundled gate: the
# slice puts the whole surviving case in length (`len4+`: plain +1.92 NV /
# +3.58 vul per fired) while at fixed length every measured honor step costs
# (`len3 hon0` +0.62/+1.85 vs `len3 hon1` -0.75/+0.37).  Bundled, a win would
# ship the bad half and a loss would bury the good one.
#   length  opener passes with FOUR cards in their seven-card suit
#   three   ...and with three to two of the top three honors (the full v2
#           candidate).  `three` implies `length`, so read `three vs length`
#           for the extension's own price.
#
# Reading it: this knob's mechanism is **adding doubles**, so measurement.md's
# domain addendum applies — plain DD is the arbiter, sd-lead the tie-break, and
# PD is a double-blind column that neither rescues nor kills.  Prior to beat:
# v1's -2.06 IMPs/fired on sd-lead.  Run `probe-divergence --gate-opener ours`
# before the headline.
if [ "${ROUND:-1}" = 5 ]; then
    for v in none both; do
        arm base   "$v" --filter-preempt
        arm length "$v" --filter-preempt --ns-nt-high-overcall-x-leave-in true
        arm three  "$v" --filter-preempt --ns-nt-high-overcall-x-leave-in true \
                        --ns-nt-high-overcall-x-leave-in-three true
        diffpair length base   "$v"
        diffpair three  base   "$v"
        diffpair three  length "$v"
        sddiff   length base   "$v"
        sddiff   three  base   "$v"
        sddiff   three  length "$v"
    done
fi

log "nt-high-overcall done"
