#!/bin/sh
# ab-floor-file.sh — file-loaded floor candidates against the shipped v6 floor,
# vs BBA (docs/archive/floor-sweep.md).
#
#   plain   american as shipped                                     (control)
#   NAME    american-file: the same book, rails and regime input, the net a
#           logit mean over the v6 blobs given for NAME (K = 1 is one
#           file-loaded net); every candidate diffs against plain
#
# One binary serves the whole sweep: candidates are files, never builds.
# Arms are NAME=a.f32,b.f32,...; with none, one arm `file` from
# PONS_FLOOR_WEIGHTS (the Phase 0/1 form).  All arms share one SEED_BASE.
#
#   BOARDS=204800 setsid nohup scripts/idle-run.sh scripts/ab-floor-file.sh \
#       ab-results/sweep-k8 k4b=s5.f32,s6.f32,s7.f32,s8.f32 k8=s1.f32,...,s8.f32 \
#       >ab-results/sweep-k8.log 2>&1 &
R=${1:?usage: ab-floor-file.sh RESULTS_DIR [NAME=a.f32,b.f32,...]...}
shift
[ $# -gt 0 ] || set -- "file=${PONS_FLOOR_WEIGHTS:?set PONS_FLOOR_WEIGHTS or pass NAME=blobs arms}"
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== floor-file start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
# Pin each arm's blobs like ab-lib pins the shard count: a resume with other
# weights would diff an arm drawn from two different nets.
for a; do
    n=${a%%=*}
    f=$R/weights; [ "$n" = file ] || f=$R/weights.$n
    W=$(echo "${a#*=}" | tr , '\n' | xargs sha256sum)
    [ -s "$f" ] || echo "$W" >"$f"
    [ "$(cat "$f")" = "$W" ] || { echo "ab-floor-file: $n in $R was generated with other blobs (see $f)" >&2; exit 1; }
    log "arm $n: ${a#*=}"
    echo "$W" | while read -r line; do log "  blob $line"; done
done
for vul in none both; do
    arm plain "$vul" --our-floor american
    for a; do
        n=${a%%=*}
        PONS_FLOOR_WEIGHTS=${a#*=}; export PONS_FLOOR_WEIGHTS
        arm "$n" "$vul" --our-floor american-file
        diffpair "$n" plain "$vul"
        sddiff "$n" plain "$vul"
    done
done
log "=== floor-file done"
