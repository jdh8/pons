#!/bin/bash
# ab-ben-competitive-demotion.sh — the BEN Tier-F guard for
# scripts/ab-competitive-demotion.sh, same SEED_BASE, same three arms.
#
#   BASE=ab-results/competitive-demotion
#   SEED_BASE="$(<"$BASE/seed")" PER_SHARD=1600 scripts/idle-run.sh \
#       scripts/ab-ben-competitive-demotion.sh "$BASE/ben"
#
# Needs 16 unchanged BEN servers on ports 8085..8100 (see ab-ben-defensive-
# overcalls.sh).  Ship = BBA pass + BEN non-refute, plain DD the arbiter.
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")/.."

R=${1:?usage: ab-ben-competitive-demotion.sh RESULTS_DIR}
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

ports=$(pgrep -af 'gameapi.py' | grep -o -- '--port [0-9]*' | awk '{print $2}' | sort -nu)
count=$(printf '%s\n' "$ports" | grep -c . || true)
[ "$count" = "$SERVERS" ] && [ "$(printf '%s\n' "$ports" | head -n 1)" = 8085 ] \
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
    log "diff $on vs $off ($vul, DD+PD)"
    "$DIFF" "$R/$on-$vul" "$R/$off-$vul" --score both \
        --out-plain "$R/diff.$on.vs.$off.$vul.plain.txt" \
        --out-pd "$R/diff.$on.vs.$off.$vul.pd.txt" --show "$SHOW" >>"$R/log" 2>&1
    log "sd-diff $on vs $off ($vul, SD+SD-PD)"
    "$SD" "$R/$on-$vul" "$R/$off-$vul" -v "$vul" --sd-worlds 16 --show 0 \
        >"$R/sd.$on.vs.$off.$vul.txt" 2>&1
}

log "=== BEN competitive demotion start, sha=$(git rev-parse --short HEAD), SEED_BASE=$SEED_BASE, ${SERVERS}x${PER_SHARD} bd/arm/vul, $NOTE"
for vul in none both; do
    arm control "$vul"
    arm off "$vul" --no-ns-competitive-pass-demotion
    arm natural "$vul" --ns-competitive-push-natural-only
    compare off control "$vul"
    compare natural control "$vul"
done
log "=== BEN competitive demotion done"
