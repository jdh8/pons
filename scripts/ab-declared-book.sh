#!/bin/sh
# ab-declared-book.sh — rows Phase 2b: does reading the opponents' calls off
# *their* books beat reading them off ours?
#
# Both arms seat the same mixed table — our american floor against a pons dutch
# book (`--their-floor dutch`) — and differ only in `--declare-their-book`,
# which points our reader's opponent channel at the dutch stance
# (`Stance::with_opponents`).  Our bidding table does not move; only what we
# believe their alerted calls and passes showed.  Asymmetric on purpose: the
# dutch side keeps reading us as dutch in both arms, so every IMP is ours.
#
# A reading change is a constructive-side change — plain DD is the honest
# primary metric, pd guards the doubling tail.  Its twin, the *net's* card
# channel (`--declare-opponents`, Phase 2a), measured negative and is default
# off; the two are deliberately not run together.
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/ab-declared-book.sh ab-results/rows-phase2b \
#       >ab-results/rows-phase2b.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir → a new seed).
R=${1:?usage: ab-declared-book.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== rows Phase 2b start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm ours     "$vul" --their-floor dutch
    arm declared "$vul" --their-floor dutch --declare-their-book
    diffpair declared ours "$vul"
done
log "=== rows Phase 2b done"
