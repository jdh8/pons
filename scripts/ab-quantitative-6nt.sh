#!/bin/sh
# ab-quantitative-6nt.sh — the direct `6NT` above each quantitative `4NT`
# (`notrump.quantitative_six_notrump`, docs/next-steps.md item 2).
#
#   SKIP_BUILD=1 setsid nohup scripts/idle-run.sh \
#       scripts/ab-quantitative-6nt.sh ab-results/quantitative-6nt \
#       >ab-results/quantitative-6nt.log 2>&1 < /dev/null &
#
# What is being tested.  The 2026-09-30 census of the anchor's round-2 `3NT`
# rows (scripts/slam-census.py) found every one of them a sign-off, and three
# lanes whose table ends at the invite: `1m - 1M - 2NT` has `4NT` on 12–13 and
# `3NT` on 6+, so a 14+ responder (32+ combined) bids `3NT`; `1NT - 2♣ - 2x`
# no-fit has `4NT` on 16–17 and nothing above; `2NT - 3♣ - 3♦` has no
# quantitative call at all.  BBA reaches `6NT` on those hands (4NT/5NT).
#
#   on   `6NT`@120 on 14+ / 18+ / 13–16 in those lanes, plus `4NT`@120 (11–12)
#        and opener's 21+/24+ answer after `2NT - 3♣ - 3♦` — the shipped
#        default since 2026-09-30
#   off  the pre-2026-09-30 tables (the disarming flag)
#
# SHIPPED default-on 2026-09-30 on this script's first run (SEED_BASE
# 1790756588, 204,800 boards/arm/vul): +0.0031 (none) / +0.0036 (both)
# IMPs/board, identical on plain DD and PD (no doubling in the lane), 82
# fired per arm, CI ±0.0009 / ±0.0011.  Two of the five worst boards were the
# direct-2NT rung overriding the floor's making 7NT on a 17-count; the rung
# was capped at 13–16 after the run (unmeasured, 2 boards per 204,800).
#
# Hypothesis: a plain-DD and PD win in both vulnerabilities, sized by the
# census at ≈ +0.001–0.002/board (the missed-slam rows in those lanes total
# −0.5k PD per 409.6k boards).  A loss says the 32-count is not enough
# opposite our 18–19 / 15–17 / 20–21 ranges — then lift each floor by one.
# Read the paired diff, not the headline; the surface is well under 1% of
# boards.
R=${1:?usage: ab-quantitative-6nt.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for quantitative-6nt)

log "=== quantitative-6nt A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-quantitative-six-notrump
    arm on  "$vul"
    diffpair on off "$vul"
done
log "=== quantitative-6nt A/B done"
