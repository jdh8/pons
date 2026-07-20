#!/bin/sh
# book-value-ab.sh — what is the authored book actually worth?
#
# Both arms carry the SAME floor wiring (`NeuralFloorBba` on the contested
# books, the deterministic `instinct()` ladder on the constructive one); the
# only difference is whether there is an authored book above it:
#   american-floor  empty books — every call comes from the floor
#   american        the authored 2/1 books over that floor  (the champion)
#
# So `american` − `american-floor` prices the book.  Note it prices the book's
# *total* contribution: an empty book also stops projecting authored
# constraints into `Inferences`, so the net's `features_v3` inference block
# collapses to unknown.  The measured gap is the book as authored calls AND as
# disclosure, not the calls alone.
#
# This is a diagnostic, not a ship gate — nothing here changes `american()`.
# A small gap would be the interesting result: it would say the BBA-distilled
# floor has largely absorbed the book.  Read plain DD as primary with pd
# guarding the doubling tail (docs/measurement.md).
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/book-value-ab.sh ab-results/book-value \
#       >ab-results/book-value.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir → a new seed).
R=${1:?usage: book-value-ab.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== book value start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul (american vs american-floor)"
for vul in none both; do
    arm american-floor "$vul" --our-floor american-floor
    arm american       "$vul" --our-floor american
    diffpair american american-floor "$vul"
done
log "=== book value done"
