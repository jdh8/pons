#!/bin/sh
# ab-competitive-accountant.sh — price the contested game-level decision.
#
# Above the three-level the book is silent by construction: every double rule is
# gated `their_live_bid_at_most(3)` and the competitive raise ladder stops at
# four, so when they buy a game only the floor's unpriced judgement logits act.
# `set_competitive_accountant` reprices that node from the same forward pass the
# constructive gates read — both declarers' trick Gaussians composed with the
# exact score table, our vulnerability, and the measured doubling rate — and
# demotes the calls the economics refuse.  Three actions, all demotions: veto a
# losing candidate bid, mask a phantom penalty double, charge Pass when the
# double is the better bet.
#
# Two arms per vul, identical deals:
#   off      the shipped default (judgement logits alone above the three-level)
#   capped   --ns-competitive-accountant
#
# `capped` is the same knob as the v1 `priced` arm plus `DOUBLE_PUSH_CEILING`:
# no double is *pushed* over a slam, because the v1 run's whole loss tail was
# six- and seven-level doubles they ran out of.  Re-run into the v1 results dir
# and the deals pair with it ($R/seed persists), the `off` arm is skipped as
# already generated — knob-off is byte-identical across the cap, which
# `smoke-default --count 20000 --seed 1` pins — and `capped vs priced` scores
# the cap itself on the same boards.
#
# Everything except the trick estimate is closed form and was calibrated before
# any board was spent: q = 0.33 making / 0.65 failing at this node's own trigger
# (docs/ai-bidder/doubling-calibration.md), the opponent trick columns cleared
# their reliability gate at +0.0225 tricks of MAE, and the trigger reaches 15.9%
# of boards.  So the A/B judges the gate, not its parameters.
#
# Plain DD holds the veto — the gate's whole business is the doubled tail, and
# perfect defense doubles every failing contract, which is exactly the bracket
# an unpriced floor already flatters.  Read [plain DD, SD-PD] as the honest pair.
# If it lands close, sweep `COMPETITIVE_MARGIN` (300 points) before concluding;
# it is the one underived constant in the gate.
#
#   setsid nohup scripts/ab-competitive-accountant.sh ab-results/competitive-accountant \
#       >ab-results/competitive-accountant.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-competitive-accountant.sh RESULTS_DIR}
SHOW=${SHOW:-40}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== competitive accountant start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm capped "$vul" --ns-competitive-accountant
    diffpair capped off "$vul"
    # The v1 uncapped arm, when this dir is its dir.
    if [ -s "$R/priced-$vul/shard-0.json" ]; then diffpair capped priced "$vul"; fi
done
log "=== competitive accountant done"
