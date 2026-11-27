#!/bin/sh
# ab-strawberry.sh — price Strawberry Polish Club (docs/strawberry.md).
#
# Three arms per cell, identical deals, all vs BBA on its 2/1 card, each arm's
# own card generated from its system:
#   advanced             the shipped default, `american` (the control)
#   strawberry           `--our-floor strawberry`: its own openings and Polish
#                        1♣, the house book grafted under 1♦/1♥/1♠/1NT, the
#                        house defense, the same v6 floor; BBA is told WJ
#   strawberry-instinct  `--our-floor strawberry-instinct`: the same books on
#                        the deterministic ladder
#
# `strawberry vs advanced` prices the authored chapters: the borrowed chapters
# and the defense are the house book's, so every divergent board starts at an
# opening or under 1♣.  Expected: a loss, as a half-built system measures
# (the Dutch Phase 2.1 precedent); the headline is `--by node`, which ranks the
# holes.  `strawberry-instinct vs strawberry` says what the learned floor is
# worth under a 1♣ it was never trained on.  Two cells: no rule here reads the
# colours.
#
#   BOARDS=204800 scripts/queue-ab.sh strawberry scripts/ab-strawberry.sh ab-results/strawberry
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.
#
# MEASURED 2026-11-27 (sha fae1bb9a; seed 1795771032426658059; 204,800
# bd/cell).  IMPs/board none / both:
#
#   strawberry          vs advanced    plain -0.0640 / -0.0835  (loss 2/2)
#                                      PD    -0.0870 / -0.0995  (loss 2/2)
#   strawberry-instinct vs strawberry  plain -0.3371 / -0.4133  (floor +)
#                                      PD    -0.3424 / -0.4941
#
# Expected loss for a half-built system.  Attribution (pueue 44,
# $R/bucket.*.txt) re-ranked docs/strawberry.md §5: strong hands through
# 1♣ cost a third of the plain loss, the minimum 1♣ a fifth, passing the
# weak twos 9-21%; the light 1M openings win plain and lose PD.
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-strawberry.sh RESULTS_DIR}
SHOW=${SHOW:-40}
BUILD_EXTRA='--example probe-divergence'
VULS=${VULS:-none both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== strawberry start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm advanced "$vul"
    arm strawberry "$vul" --our-floor strawberry
    diffpair strawberry advanced "$vul"
    arm strawberry-instinct "$vul" --our-floor strawberry-instinct
    diffpair strawberry-instinct strawberry "$vul"
done
log "=== strawberry done"
