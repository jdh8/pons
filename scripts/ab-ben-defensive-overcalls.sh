#!/usr/bin/env bash
# Same-seed Tier-F guard for scripts/ab-defensive-overcalls.sh.  Requires 16
# unchanged BEN servers; 16 x 1,600 = 25,600 boards per arm/vulnerability.
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")/.."

MODE=${1:?usage: ab-ben-defensive-overcalls.sh o3 RESULTS_DIR}
BASE=${2:?usage: ab-ben-defensive-overcalls.sh o3 RESULTS_DIR}
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
: "${SEED_BASE:?set SEED_BASE to the matching BBA experiment seed}"
PER_SHARD=${PER_SHARD:-1600}
SHOW=${SHOW:-8}
SERVERS=16
CONF=vendor/ben/BEN-21GF-F.conf
NOTE="tier-f-conf-sha256=$(sha256sum "$CONF" | awk '{print $1}')"
DIFF=target/release/examples/ab-dump-diff
SD=target/release/examples/ab-dump-sd
mkdir -p "$R"

log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }

ports=$(pgrep -u "$USER" -af 'gameapi\.py' | grep -o -- '--port [0-9]*' | awk '{print $2}' | sort -nu || true)
count=$(printf '%s\n' "$ports" | awk 'NF { n++ } END { print n + 0 }')
[ "$count" -eq "$SERVERS" ] && [ "$(printf '%s\n' "$ports" | head -n 1)" = 8085 ] \
    && [ "$(printf '%s\n' "$ports" | tail -n 1)" = 8100 ] || {
    echo "need unchanged BEN ports 8085..8100; found $count: $ports" >&2
    exit 1
}

cargo build --release --features serde --example ben-gen --example ab-dump-diff --example ab-dump-sd

arm() {
    local name=$1 vul=$2
    shift 2
    local dir="$R/$name-$vul"
    [ -s "$dir/shard-0.json" ] && { log "skip $dir (exists)"; return; }
    log "generate $dir (flags: $*)"
    SEED_BASE=$SEED_BASE SKIP_BUILD=1 scripts/ben-gen-parallel.sh "$dir" "$PER_SHARD" \
        -v "$vul" -t f --note "$NOTE" "$@" >>"$R/log" 2>&1
}

compare() {
    local on=$1 off=$2 vul=$3
    shift 3
    local plain="$R/diff.$on.vs.$off.$vul.plain.txt"
    local pd="$R/diff.$on.vs.$off.$vul.pd.txt"
    log "diff $on vs $off ($vul, DD+PD)"
    "$DIFF" "$R/$on-$vul" "$R/$off-$vul" --score both \
        --out-plain "$plain" --out-pd "$pd" --show "$SHOW" >>"$R/log" 2>&1
    log "sd-diff $on vs $off ($vul, SD+SD-PD)"
    "$SD" "$R/$on-$vul" "$R/$off-$vul" -v "$vul" --sd-worlds 16 --show 0 \
        --on-ns-direct-weak-jump-overcall --off-ns-direct-weak-jump-overcall \
        "$@" >"$R/sd.$on.vs.$off.$vul.txt" 2>&1
}

log "=== BEN defensive-overcalls $MODE start, sha=$(git rev-parse --short HEAD), SEED_BASE=$SEED_BASE, ${SERVERS}x${PER_SHARD} bd/arm/vul, $NOTE"
for vul in none both; do
    case "$MODE" in
    o3)
        arm control "$vul" --no-ns-direct-minor-weak-jump-overcall
        arm minor "$vul"
        compare minor control "$vul" --on-ns-direct-minor-weak-jump-overcall
        ;;
    # docs/takeout-double-layers.md — both knobs default off, so the control
    # arm is HEAD's defaults with no flag.
    bar)
        arm control "$vul"
        arm on "$vul" --ns-suppress-long-minor-takeout
        compare on control "$vul" --on-ns-suppress-long-minor-takeout
        ;;
    seam)
        arm control "$vul"
        arm on "$vul" --ns-defensive-seam-split
        compare on control "$vul" --on-ns-defensive-seam-split
        ;;
    esac
done
log "=== BEN defensive-overcalls $MODE done"
