#!/usr/bin/env bash
# Paired BBA experiments for docs/defensive-overcalls.md.  One mode per run,
# one persistent fresh seed, sequential arms, DD/PD plus SD/SD-PD.
set -euo pipefail

MODE=${1:?usage: ab-defensive-overcalls.sh o3 RESULTS_DIR}
BASE=${2:?usage: ab-defensive-overcalls.sh o3 RESULTS_DIR}
case "$MODE" in
o3 | bar | seam) ;;
reading|o1|o2)
    echo "$MODE stopped: the direct-1NT reader gate failed" >&2
    exit 2
    ;;
o4)
    echo "O4 stopped: no probe model met the 95% held-out accuracy gate" >&2
    exit 2
    ;;
*)
    echo "unknown mode: $MODE (o3, bar, seam)" >&2
    exit 2
    ;;
esac
R=$BASE/$MODE
SHOW=${SHOW:-8}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
# `ab-lib` built every current harness above; freeze those binaries for every
# arm in this experiment.
export SKIP_BUILD=1
SEED_BASE=$(seed_for)

# The shipped direct-major weak jump is present in every treatment here.  Pass
# both systems symmetrically; `ab-dump-sd` currently does not price opponent
# disclosure, so these flags are provenance rather than a disclosure claim.
sdpair() {
    sddiff "$@" --on-ns-direct-weak-jump-overcall --off-ns-direct-weak-jump-overcall
}

log "=== defensive-overcalls $MODE start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    case "$MODE" in
    o3)
        arm control "$vul" --no-ns-direct-minor-weak-jump-overcall
        arm minor "$vul"
        diffpair minor control "$vul"
        sdpair minor control "$vul" --on-ns-direct-minor-weak-jump-overcall
        ;;
    # docs/takeout-double-layers.md.  Both control arms are HEAD's defaults —
    # each knob is off by default, so `arm control` needs no flag.
    bar)
        arm control "$vul"
        arm on "$vul" --ns-suppress-long-minor-takeout
        diffpair on control "$vul"
        sdpair on control "$vul" --on-ns-suppress-long-minor-takeout
        ;;
    seam)
        arm control "$vul"
        arm on "$vul" --ns-defensive-seam-split
        diffpair on control "$vul"
        sdpair on control "$vul" --on-ns-defensive-seam-split
        ;;
    esac
done
log "=== defensive-overcalls $MODE done"
