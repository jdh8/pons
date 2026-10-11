#!/bin/sh
# ab-competitive-demotion.sh — session D arm 2 of the competitive accountant:
# price the Pass demotion, the gate's one action that *adds* a call.
#
# `bba-decompose --demotion-census` (docs/ai-bidder/competitive-accountant.md,
# "Session D arm 2") sized the demotion offline on both anchors: its pushed
# doubles that stood are worth +0.008 / +0.017 IMPs/board (none / both) vs
# BBA on plain DD — the shipped gate's whole measured plain win — and ~70% of
# its flips sit over an artificial four-plus bid of theirs (a cue, a splinter,
# Blackwood), the fourth leaf approximation the design recorded.  What the
# census cannot price is the continued auction after the push (they run, we
# bid on), so the A/B decides.  Three arms per vul, identical deals:
#   control   the shipped default (demotion on, every bid pushed)
#   off       --no-ns-competitive-pass-demotion   (veto + X-mask only)
#   natural   --ns-competitive-push-natural-only  (push only over their first
#             bid or a strain they already named)
#
# Plain DD holds the veto (the knob changes how often we double; PD erases a
# double both ways and is blind to it — competitive-accountant.md "Verdict").
# Then the BEN guard on the same SEED_BASE: scripts/ab-ben-competitive-demotion.sh.
#
#   setsid nohup scripts/idle-run.sh scripts/ab-competitive-demotion.sh \
#       ab-results/competitive-demotion >ab-results/competitive-demotion.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-competitive-demotion.sh RESULTS_DIR}
SHOW=${SHOW:-40}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
export SKIP_BUILD=1
SEED_BASE=$(seed_for)

log "=== competitive demotion start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm control "$vul"
    arm off "$vul" --no-ns-competitive-pass-demotion
    arm natural "$vul" --ns-competitive-push-natural-only
    diffpair off control "$vul"
    diffpair natural control "$vul"
    # The gate publishes nothing (projection-invisible), so the blind leader
    # needs no knob disclosure.
    sddiff off control "$vul"
    sddiff natural control "$vul"
done
log "=== competitive demotion done"
