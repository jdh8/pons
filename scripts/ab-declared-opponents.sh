#!/bin/sh
# ab-declared-opponents.sh — cell B at full A/B scale: E3 of
# docs/ai-bidder/card-manifold.md, the v5 retrain's go/no-go.
#
# Both arms seat the same mixed table — our american floor against a pons dutch
# book (`--their-floor dutch`) — and differ only in `--declare-opponents`,
# which feeds our net the card the opponents actually hold instead of
# `Config::symmetric`'s copy of our own.  Against a dutch seat that card moves
# exactly the genuinely *trained* coordinates (the base-system one-hot and
# `1D opening with 5 cards`), so unlike cell A this is an in-distribution
# declaration: it prices what the trained card channel is worth, not the
# off-manifold penalty the bias fold has since zeroed.
#
# The verdict gates the v5 corpus: a wash here says the trained coordinates are
# worth ~0 and the retrain is dead; a gain says the channel is real and worth
# thawing more axes for.  Its reader twin (`--declare-their-book`,
# ab-declared-book.sh) stays deliberately un-run here — one channel per A/B.
#
# A declaration change is a constructive-side change — plain DD is the honest
# primary metric, pd guards the doubling tail.
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/ab-declared-opponents.sh ab-results/card-fold-e3 \
#       >ab-results/card-fold-e3.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir → a new seed).
R=${1:?usage: ab-declared-opponents.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== card-manifold E3 start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm symmetric "$vul" --their-floor dutch
    arm declared "$vul" --their-floor dutch --declare-opponents
    diffpair declared symmetric "$vul"
done
log "=== card-manifold E3 done"
