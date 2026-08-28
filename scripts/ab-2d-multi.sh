#!/bin/sh
# ab-2d-multi.sh — N4: their `(2♦)` over our 1NT read as a Multi
# (docs/one-notrump-competitive.md §N4).
#
#   JOBS=24 PER_SHARD=19200 setsid nohup scripts/idle-run.sh \
#       scripts/ab-2d-multi.sh ab-results/2d-multi \
#       >ab-results/2d-multi.log 2>&1 < /dev/null &
#
# What is being tested.  `their.two_diamonds_multi` (a disclosure about the
# opponents, `bba-gen --their-2d-multi`) routes `1NT (2♦)` to the Multi table:
# the Transfer-Lebensohl leg's Stayman/Smolen/Jacoby/Leaping-Michaels calls
# verbatim, with every diamond-keyed gate re-keyed — `X` = invitational-plus
# values (`hcp 8+`, no diamond claim), `3NT` = both majors stopped, the `2NT`
# relay adds a natural `3♦` sign-off, the `3♠`→♣ completion drops its diamond
# stopper — and opener's answers: sit over their pass of the double, double the
# advancer's pass-or-correct `2♥`/`2♠` with four trumps, else wait.  Five
# rounds later (v4, the final shape) the double family's continuations and
# the relay's competition are book nodes too — see §N4 for why the floor could
# not hold them.
#
# The base arm is the shipped natural-diamonds leg (`DoubleStyle::Optional`:
# `len(♦, 2..=3) & hcp(8..)`, opener cooperating).  Against BBA the difference
# is a *reinterpretation*: BBA's `2♦` is a Multi (docs/ai-bidder/bba-multi-2d.md),
# so the base arm's diamond gates name a suit nobody holds.
#
# `--filter-1nt` (balanced 15-17 somewhere, a raw-hand test applied BEFORE any
# bidding) rides both arms so they deal the same board set and stay seed-aligned
# for the paired diff.  Headline is IMPs per *accepted* deal; the `(2♦)` bucket
# is ~2% of accepted boards, so read the per-fired figure alongside.
#
# Scoring.  Half of this is a doubling knob (the values double, opener's trump
# double), so plain DD is the arbiter for those calls — perfect defense cannot
# price "double more" (docs/measurement.md); the constructive re-keying
# (3NT, relay 3♦, completions) is judged on the standard `plain wash | PD win`
# gate.  `sddiff` is a tie-breaker.
#
# Read `probe-divergence --gate-opener ours` BEFORE the headline: N4b's whole
# raw win was 84.9% foreign (their double of OUR 2♦ overcall, read through our
# agreement).  A disclosure keyed on their 2♦ cannot fire on boards they open,
# but the gate is the proof, not the design.
#
# Since the v7 ship the census default is ON, so the base arm is spelled
# `--their-2d-multi false` (a v6/v7-style rerun can symlink an existing base
# arm: the default system is byte-identical, same seeds → same boards).
#
# Resumable: an existing arm dir or diff file is skipped, and SEED_BASE persists
# in $R/2d-multi.seed.  Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-2d-multi.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for 2d-multi)
log "=== 2d-multi SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base  "$v" --their-2d-multi false --filter-1nt
    arm multi "$v" --their-2d-multi --filter-1nt
    diffpair multi base "$v"
    sddiff multi base "$v"
done

log "2d-multi done"
