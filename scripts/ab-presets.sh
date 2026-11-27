#!/bin/sh
# ab-presets.sh — re-price the two named presets against today's default,
# each on its own generated card (docs/bws-2017.md §5, docs/system-variants.md
# §5a).  Three arms per cell, identical deals, all vs BBA on its 2/1 card:
#
#   advanced  the shipped default, no flag
#   bws       `--ns-preset bws`: Bridge World Standard 2017 where authored
#   basic     `--ns-preset basic`: the beginner card
#
#   BOARDS=819200 scripts/queue-ab.sh presets scripts/ab-presets.sh ab-results/presets
#
# Why.  Both recorded prices predate two changes.  The default took the
# semiforcing `1NT` on 2026-11-27 (ab-bws.sh's control still pins it off), and
# the cards' "after double" rows were mis-keyed until 2026-11-28: `basic` and
# `bws` play plain lebensohl advances of `(2x) X` and now tell BBA so
# (`Lebensohl after double = 1`).  The declared EPBot book also pins the
# `1NT` super-accept off since 2026-11-28, which `bws` turns on.  `basic` was
# last priced on 2026-10-14 at 204,800 boards over two cells.
#
# Expected: both lose, as before; the question is whether the price moved.
# A preset's number is a price, not a ship gate, so there is no isolation
# gate: a preset moves both sides of the table by design.
#
# Read plain DD as the verdict and PD as the guard (docs/measurement.md).
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-presets.sh RESULTS_DIR}
SHOW=${SHOW:-40}
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== presets start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm advanced "$vul"
    arm bws "$vul" --ns-preset bws
    diffpair bws advanced "$vul"
    arm basic "$vul" --ns-preset basic
    diffpair basic advanced "$vul"
done
log "=== presets done"
