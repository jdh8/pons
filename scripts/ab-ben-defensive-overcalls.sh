#!/usr/bin/env bash
# Same-seed Tier-F guard for scripts/ab-defensive-overcalls.sh.  Requires 16
# unchanged BEN servers; 16 x 1,600 = 25,600 boards per arm/vulnerability.
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")/.."

MODE=${1:?usage: ab-ben-defensive-overcalls.sh o3 RESULTS_DIR}
BASE=${2:?usage: ab-ben-defensive-overcalls.sh o3 RESULTS_DIR}
case "$MODE" in
o3 | bar | seam | o1band) ;;
reading|o1|o2)
    echo "$MODE stopped: the direct-1NT reader gate failed" >&2
    exit 2
    ;;
o4)
    echo "O4 stopped: no probe model met the 95% held-out accuracy gate" >&2
    exit 2
    ;;
*)
    echo "unknown mode: $MODE (o3, bar, seam, o1band)" >&2
    exit 2
    ;;
esac
R=$BASE/$MODE
: "${SEED_BASE:?set SEED_BASE to the matching BBA experiment seed}"
PER_SHARD=${PER_SHARD:-1600}
SHOW=${SHOW:-8}
SERVERS=16
CONF=vendor/ben/BEN-21GF-F.conf
NOTE="tier-f-conf-sha256=$(sha256sum "$CONF" | awk '{print $1}')"

ports=$(pgrep -u "$USER" -af 'gameapi\.py' | grep -o -- '--port [0-9]*' | awk '{print $2}' | sort -nu || true)
count=$(printf '%s\n' "$ports" | awk 'NF { n++ } END { print n + 0 }')
[ "$count" -eq "$SERVERS" ] && [ "$(printf '%s\n' "$ports" | head -n 1)" = 8085 ] \
    && [ "$(printf '%s\n' "$ports" | tail -n 1)" = 8100 ] || {
    echo "need unchanged BEN ports 8085..8100; found $count: $ports" >&2
    exit 1
}

# ab-lib's arm/diffpair/sddiff: a manifest-checked, shard-verified resume and
# stamped reports (the old local arm skipped on a nonempty shard-0 alone).
# JOBS is the server count: ben-gen-parallel refuses any other fleet size.
GENERATOR=ben-gen
JOBS=$SERVERS
BUILD_EXTRA='--example ab-dump-sd'
. scripts/ab-lib.sh

# ben NAME VUL [flags...] — one Tier-F arm
ben() { arm "$1" "$2" -t f --note "$NOTE" "${@:3}"; }

# compare ON OFF VUL [sd flags...] — DD+PD, then SD+SD-PD with the weak jump
# overcall (default on) disclosed for both arms to the blind leader
compare() {
    diffpair "$1" "$2" "$3"
    sddiff "$1" "$2" "$3" --on-ns-direct-weak-jump-overcall \
        --off-ns-direct-weak-jump-overcall "${@:4}"
}

log "=== BEN defensive-overcalls $MODE start, sha=$(git rev-parse --short HEAD), SEED_BASE=$SEED_BASE, ${SERVERS}x${PER_SHARD} bd/arm/vul, $NOTE"
for vul in $VULS; do
    case "$MODE" in
    o3)
        ben control "$vul" --no-ns-direct-minor-weak-jump-overcall
        ben minor "$vul"
        compare minor control "$vul" --on-ns-direct-minor-weak-jump-overcall
        ;;
    # docs/takeout-double-layers.md — both knobs default off, so the control
    # arm is HEAD's defaults with no flag.
    bar)
        ben control "$vul"
        ben on "$vul" --ns-suppress-long-minor-takeout
        compare on control "$vul" --on-ns-suppress-long-minor-takeout
        ;;
    seam)
        ben control "$vul"
        ben on "$vul" --ns-defensive-seam-split
        compare on control "$vul" --on-ns-defensive-seam-split
        ;;
    o1band)
        ben control "$vul"
        ben on "$vul" --ns-nt-overcall-no-major --ns-nt-overcall-major-band
        compare on control "$vul" --on-ns-nt-overcall-no-major --on-ns-nt-overcall-major-band
        ;;
    esac
done
log "=== BEN defensive-overcalls $MODE done"
