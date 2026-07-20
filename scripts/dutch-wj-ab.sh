#!/bin/sh
# dutch-wj-ab.sh — A/B B of the WJ-floor campaign: does a floor distilled from a
# teacher that actually plays a Polish minor structure beat one distilled from
# 2/1, over the subtree where Dutch leaves american?
#
# Both arms are the SAME authored Dutch books (`bare_dutch`) and the same
# `NeuralFloorBba` everywhere except one place:
#   dutch      NeuralFloorBba throughout                   (the baseline, A/B A's winner)
#   dutch-wj   NeuralFloorWj under *our* 1♦, BBA elsewhere (the treatment)
#
# A/B A had to land first — B's non-1♦ subtrees *are* A's treatment. It did:
# +0.176/+0.276 plain, +0.168/+0.347 pd (none/both), 204800 bd/arm/vul.
#
# The routing is guarded on *our* side having opened, because the opponents bid
# american: their natural 1♦ must not be read as Polish. Smoke-tested at 2000
# paired boards — 81 divergent auctions, every one of them opened 1♦, zero
# leakage into any other subtree.
#
# Floor swap → plain DD is primary, pd guards the doubling tail.
#
# PRE-REGISTERED, from the campaign plan — record these before reading a verdict:
#
#  1. Effect concentrates SHALLOW and on OPENER's side, decaying with auction
#     depth. That is what the decomposition predicts; a flat diff means the net
#     is not transferring at all, and the range mismatch is the first suspect.
#  2. Effect concentrates BELOW 18 HCP. Exactly 9.7% of Dutch's 1♦ hands are
#     18+, and WJ routes every one of those through its forcing 1♣ instead, so
#     the WJ net has never seen a strong 1♦ opener. Split 11-17 vs 18+.
#  3. Step 0 measured BBA-WJ as an OVERBIDDER vs BBA-2/1: +0.086 plain both
#     vuls but 0.00 -> -0.038 pd, worse at `both` than at `none`. If the WJ net
#     inherited that bias, B wins plain and washes-or-loses pd, worse at `both`.
#     Count DD-failing contracts per arm; that is the fingerprint, not judgement.
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/dutch-wj-ab.sh ab-results/dutch-wj \
#       >ab-results/dutch-wj.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir → a new seed).
R=${1:?usage: dutch-wj-ab.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== dutch WJ floor start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul (dutch-wj vs dutch)"
for vul in none both; do
    arm dutch    "$vul" --our-floor dutch
    arm dutch-wj "$vul" --our-floor dutch-wj
    diffpair dutch-wj dutch "$vul"
done
log "=== dutch WJ floor done"
