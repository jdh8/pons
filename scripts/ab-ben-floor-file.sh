#!/bin/sh
# ab-ben-floor-file.sh — the BEN guard of ab-floor-file.sh: file-loaded floor
# candidates against the shipped K = 8 floor, vs BEN Tier F, four cells.
#
#   plain   american as shipped                                     (control)
#   NAME    american-file: the same book, rails and regime input, the net a
#           logit mean over the blobs given for NAME; diffs against plain
#
# Why.  A floor whose labels were priced against BBA (docs/reseed-gap-
# handoff.md §5) wins its A/B against BBA, the exploit guard; BEN is the
# north star, so the candidate must hold against it before it ships.
#
#   SERVERS=16 BOARDS=25600 setsid nohup scripts/idle-run.sh \
#       scripts/ab-ben-floor-file.sh ab-results/relabel-bba-ben \
#       new=s1.f32,...,s8.f32 reseed=s9.f32,...,s16.f32 \
#       >ab-results/relabel-bba-ben.log 2>&1 </dev/null &
#
# Needs exactly SERVERS (default 16) Tier-F servers already up and held
# unchanged across every arm (`scripts/ben-servers.sh start 16 f`).  JOBS is
# the server count, so BOARDS alone sets the sample.  Arms share one
# SEED_BASE; each arm's blobs are pinned like ab-floor-file.sh pins them.
#
# PRE-REGISTERED 2026-10-31, before any number exists.  NAME − plain per
# cell, IMPs/board, δ = 0.002, density 1 (docs/measurement.md, decision table):
#   1. CONFIRMED = plain non-inferior in every cell (95% lower bound above
#      −δ); the BBA verdict stands and the candidate may ship.  PD is
#      reported, not required: plain is the verdict and PD the guard
#      (measurement.md "Which end is the verdict", 2026-11-01; a 2026-11-01
#      morning amendment had briefly required PD and is reversed).
#   2. A plain LOSS in any cell (upper bound below 0) → the BBA win is an
#      exploit of BBA; trace the worst boards, nothing ships.
#   3. Inconclusive (lower bound at or below −δ, no loss): extend to
#      BOARDS=51200 with the same SEED_BASE before any verdict, in a fresh
#      results dir (ab-lib refuses a per-shard change on resume).
#   4. SD (sddiff, 16 worlds) is descriptive.
#
# Resumable; SEED_BASE persists in $R/ben-floor-file.seed.  Iron rule: do NOT
# rebuild binaries or restart the BEN fleet while this runs.
R=${1:?usage: ab-ben-floor-file.sh RESULTS_DIR NAME=a.f32,b.f32,...}
shift
[ $# -gt 0 ] || { echo "ab-ben-floor-file: give at least one NAME=blobs arm" >&2; exit 1; }
SERVERS=${SERVERS:-16}
GENERATOR=ben-gen
JOBS=$SERVERS
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
NOTE=$(ben_note)
SEED_BASE=$(seed_for ben-floor-file)

log "=== ben-floor-file start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul, $NOTE"
for a; do
    n=${a%%=*}
    f=$R/weights.$n
    W=$(echo "${a#*=}" | tr , '\n' | xargs sha256sum)
    [ -s "$f" ] || echo "$W" >"$f"
    [ "$(cat "$f")" = "$W" ] || { echo "ab-ben-floor-file: $n in $R was generated with other blobs (see $f)" >&2; exit 1; }
    log "arm $n: ${a#*=}"
    echo "$W" | while read -r line; do log "  blob $line"; done
done
for vul in $VULS; do
    unset PONS_FLOOR_WEIGHTS
    arm plain "$vul" -t f --note "$NOTE" --our-floor american
    for a; do
        n=${a%%=*}
        PONS_FLOOR_WEIGHTS=${a#*=}; export PONS_FLOOR_WEIGHTS
        arm "$n" "$vul" -t f --note "$NOTE" --our-floor american-file
        diffpair "$n" plain "$vul"
        sddiff "$n" plain "$vul"
    done
done
log "=== ben-floor-file done"
