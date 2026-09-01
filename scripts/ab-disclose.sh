#!/bin/sh
# ab-disclose.sh — what does it cost us when BBA is told what we play?
#
# `cards/American.bbsa` describes our 2/1 in BBA's own vocabulary, but until
# `--disclose` existed nothing replayed it onto the seats pons occupies: every
# anchor to date faced a BBA that took us for a BBA.  EPBot's bidding genuinely
# consults those seats (examples/probe-bba-disclosure.rs measures one toggle
# there moving its call), so switching disclosure on changes BBA's calls — about
# 3.7% of boards diverge — and therefore moves the anchor.
#
# Two arms per vul, identical deals and an identical pons side:
#   blind   --disclose off                 (every anchor recorded before 2026-07-28)
#   told    --disclose cards/American.bbsa (BBA reads our alerts)
#
# Both arms name their partnership explicitly: `--disclose` now defaults to
# `generated`, so neither is the default any more and an unflagged arm would
# quietly be a third thing.
#
# Unlike every other runner here, the knob is on THEIR side: our bidding is
# byte-identical across the arms and only BBA's model of us differs.  The swing
# is still credited to our pair, so a NEGATIVE delta means a BBA that
# understands us beats us harder — the expected direction, and the whole point
# of measuring before defaulting it on.
#
# Verdict per docs/measurement.md: plain DD and pd both reported.  This is not a
# ship/reject on a treatment — it sizes an instrument error, and decides whether
# the anchor series needs re-basing.
#
#   setsid nohup scripts/idle-run.sh scripts/ab-disclose.sh \
#       ab-results/disclose >ab-results/disclose.log 2>&1 &
#
# ~50k boards/arm/vul on a 32-core box.  Resumable; SEED_BASE persists in
# $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-disclose.sh RESULTS_DIR}
[ -n "${PER_SHARD:-}" ] || BOARDS=${BOARDS:-50000}   # total bd/arm/vul; ab-lib derives PER_SHARD
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

CARD=${CARD:-cards/American.bbsa}

log "=== disclose start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul (--disclose $CARD)"
for vul in none both; do
    arm blind "$vul" --disclose off
    arm told  "$vul" --disclose "$CARD"
    diffpair told blind "$vul"
done
log "=== disclose done"
