#!/bin/sh
# ab-lebensohl-signoff-raise.sh — `competition.lebensohl_signoff_raise` on
# against off, head to head
# (docs/research-method-review.md § Issue #9).
#
#   JOBS=96 BOARDS=192000 BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       DDSS_THREADS=96 setsid nohup scripts/idle-run.sh \
#       scripts/ab-lebensohl-signoff-raise.sh ab-results/lebensohl-signoff-raise \
#       >ab-results/lebensohl-signoff-raise.log 2>&1 < /dev/null &
#
# What is being tested.  Over `1NT (2♠) 2NT - 3♣ - 3♥ -` (responder's weak
# relay sign-off), a maximum opener with a fit raises to `4♥` on 23 combined
# points against the relay's 6-HCP floor, one less per trump beyond three.
# Off, opener passes every hand, so the arms diverge exactly on the stretch.
# 0.10.0 shipped it on one seed, 500k unfiltered self-play boards, "positive
# in every cell" with no CI; no results directory survives.
#
# `--filter-lebensohl-raise` buys the slice on the scan: a 1NT candidate with
# three-plus hearts, its LHO with five-plus spades, its partner with five-plus
# hearts and at most 11 HCP.  Read the per-fired figure for the size; the
# per-board verdict needs --density = accepted ÷ scanned, summed from the
# shards' `scanned` counts in this log.
#
# VERDICT (2026-10-16, code e400e5a1 + the knob and the filter, SEED_BASE
# 1792096531615205258, 384k accepted boards/arm/vul, density 0.004719, gate 0
# foreign in both cells): plain DD **win** | PD **loss** — the decision
# table's doubling-artifact row.  raise − pass, IMPs/fired (1159 / 1049 fired):
#   none:  plain +0.31  PD -0.71  sd-lead +0.96  SD-PD +0.12
#   both:  plain +1.60  PD +0.04  sd-lead +2.69  SD-PD +1.41
# The weak sign-off is 6-7 HCP on 97% of fired boards, so the raise bids a
# 23-24 point game; the losses sit on three trumps and on 15-16 HCP openers
# that reach the bar by upgraded points.  Followed up by
# scripts/ab-lebensohl-signoff-target.sh: the target is now `24:23`.
# (This runner's arms leave the target at the bba-gen default, so a rerun
# measures the shipped `24:23` against off, not `23`.)
#
# Resumable; SEED_BASE persists in $R/lebensohl-signoff-raise.seed.  Iron rule:
# do NOT rebuild binaries while this runs.
R=${1:?usage: ab-lebensohl-signoff-raise.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for lebensohl-signoff-raise)
log "=== lebensohl-signoff-raise SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm raise "$v" --filter-lebensohl-raise
    arm pass  "$v" --filter-lebensohl-raise --ns-lebensohl-signoff-raise off
    gatepair raise pass "$v"
    diffpair raise pass "$v"
    sddiff   raise pass "$v"
done

log "lebensohl-signoff-raise done"
