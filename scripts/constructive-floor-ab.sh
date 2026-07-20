#!/bin/sh
# constructive-floor-ab.sh — should the BBA-distilled net floor the *constructive*
# book, or does the deterministic instinct ladder keep that job?
#
# `with_floor` hands off-book uncontested auctions to instinct() and gives the
# learned net only the contested books (american.rs).  That split was justified
# on the grounds that the learned floors are trained on contested auctions only
# — true of the instinct-distilled v1/v2 nets, false of the BBA one: 37% of its
# corpus is constructive, and it validates no worse on that split.  So the split
# is a testable choice, and this prices it.
#
# Both arms are american() with identical books; the ONLY difference is which
# classifier catches an off-book constructive auction.  A constructive change →
# plain DD is the honest primary metric, pd guards the doubling tail.  Watch for
# the overbid fingerprint (plain flat, pd worse at `both` than at `none`): a net
# that outbids instinct() into failing games would show exactly that.
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/constructive-floor-ab.sh ab-results/constructive-floor \
#       >ab-results/constructive-floor.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir → a new seed).
R=${1:?usage: constructive-floor-ab.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== constructive floor start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul (bba-constructive vs american)"
for vul in none both; do
    arm american         "$vul"
    arm bba-constructive "$vul" --our-floor bba-constructive
    diffpair bba-constructive american "$vul"
done
log "=== constructive floor done"
