#!/bin/sh
# ab-probe-retrain.sh — docs/shrinkage-plan.md § A4, lap 1: the floor retrained
# under the probed boxes, read K = 8 vs K = 8 (Plan B's rule) on paired deals.
#
#   plain    shipped defaults (the shipped K = 8 v6 floor, the probed map empty)
#   k200     the shipped floor serving the probed boxes
#            (--ns-probe 100000 --ns-probe-vacuous, κ = crate default 200):
#            A3's kappa200 arm, paired here so the retrain's gain over it reads
#            on one deal set
#   a4       american-file: the K = 8 logit mean over NETS (eight draws of the
#            shipped recipe on the lap-1 corpus, whose features were read under
#            the same probed overlay), serving the probed boxes — the candidate
#   a4off    the same ensemble with the probed map empty: how far the net moved
#            on its own, read with scripts/ruler-tv.py against Plan B's K = 8
#            floor (TV 0.017 / 0.015 at depth 3)
#
# Pre-registered reading (the plan, § A4 and "Measured so far"):
#   a4 non-inferior to plain on plain DD (every cell's lower bound above −δ)
#   and a PD win → the serving ships default-on with the retrained floor;
#   a4 above k200 but still under plain → the lap moved the wall, not past it:
#   lap 2 (re-probe under a4, re-dump, retrain) is owed, stop rule = Plan B's floor;
#   a4 at or under k200 → the retrain did not reach the OOD channel; stop.
#
#   NETS=a.f32,b.f32,...,h.f32 JOBS=96 BOARDS=204800 DDSS_THREADS=96 \
#   BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#   setsid nohup scripts/idle-run.sh scripts/ab-probe-retrain.sh \
#       ab-results/probe-retrain >ab-results/probe-retrain.log 2>&1 &
#
# A probed arm re-probes 100k boards per shard (~10 min sequential per shard)
# before its deals.  Resumable; SEED_BASE persists in $R/seed.
R=${1:?usage: NETS=a.f32,...,h.f32 ab-probe-retrain.sh RESULTS_DIR}
NETS=${NETS:?set NETS to the comma-separated K = 8 blobs}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== probe-retrain start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
# Pin the blobs like ab-floor-file.sh does: a resume with other weights would
# diff an arm drawn from two different nets.
W=$(echo "$NETS" | tr , '\n' | xargs sha256sum)
[ -s "$R/weights" ] || echo "$W" >"$R/weights"
[ "$(cat "$R/weights")" = "$W" ] || { echo "ab-probe-retrain: $R was generated with other blobs (see $R/weights)" >&2; exit 1; }
echo "$W" | while read -r line; do log "  blob $line"; done
for vul in $VULS; do
    unset PONS_FLOOR_WEIGHTS
    arm plain "$vul"
    arm k200 "$vul" --ns-probe 100000 --ns-probe-vacuous
    PONS_FLOOR_WEIGHTS=$NETS; export PONS_FLOOR_WEIGHTS
    arm a4 "$vul" --our-floor american-file --ns-probe 100000 --ns-probe-vacuous
    arm a4off "$vul" --our-floor american-file
    unset PONS_FLOOR_WEIGHTS
    for a in k200 a4 a4off; do diffpair "$a" plain "$vul"; sddiff "$a" plain "$vul"; done
    diffpair a4 k200 "$vul"; sddiff a4 k200 "$vul"
    diffpair a4 a4off "$vul"; sddiff a4 a4off "$vul"
done
log "=== probe-retrain done"
