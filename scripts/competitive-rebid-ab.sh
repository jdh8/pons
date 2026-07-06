#!/bin/sh
# competitive-rebid-ab.sh — A/B for the opt-in competitive long-suit rebid floor
# (`set_competitive_rebid`, docs/competitive-book.md P5). One two-arm experiment:
# off (default) vs --ns-competitive-rebid, both vulnerabilities, both scorers,
# arms strictly sequential, one shared SEED_BASE (paired diffs need identical
# deals). Modeled on scripts/competitive-book-ab.sh; do NOT touch the codebase
# while it runs (bba-gen-parallel re-invokes cargo build; it must stay a no-op).
#
#   PER_SHARD=12800 setsid nohup scripts/idle-run.sh \
#       scripts/competitive-rebid-ab.sh ab-results/competitive-rebid \
#       >ab-results/competitive-rebid.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
set -eu
cd "$(dirname "$0")/.."

R=${1:?usage: competitive-rebid-ab.sh RESULTS_DIR}
mkdir -p "$R"
SHA=$(git rev-parse --short HEAD)
DIFF=target/release/examples/ab-dump-diff
PER_SHARD=${PER_SHARD:-12800}
SHARDS=$(nproc)

cargo build --release --features serde --example bba-gen --example ab-dump-diff

log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }

# a persistent SEED_BASE, fresh on first use
if [ ! -s "$R/seed" ]; then date +%s >"$R/seed"; fi
SEED_BASE=$(cat "$R/seed")

# arm NAME VUL [flags...] — generate one arm unless already present
arm() {
    name=$1; vul=$2; shift 2
    dir="$R/$name-$vul"
    [ -d "$dir" ] && { log "skip $dir (exists)"; return 0; }
    log "generate $dir (SEED_BASE=$SEED_BASE, flags: $*)"
    SEED_BASE=$SEED_BASE scripts/bba-gen-parallel.sh "$dir" "$PER_SHARD" -v "$vul" "$@" \
        >>"$R/log" 2>&1
}

# diffpair ON OFF VUL — per-shard paired diff, both scorers, 8 solvers wide
diffpair() {
    on=$1; off=$2; vul=$3
    for score in plain pd; do
        out="$R/diff.$on.vs.$off.$vul.$score.txt"
        [ -s "$out" ] && { log "skip $out (exists)"; continue; }
        log "diff $on vs $off ($vul, $score)"
        i=0
        while [ "$i" -lt "$SHARDS" ]; do
            "$DIFF" "$R/$on-$vul/shard-$i.json" "$R/$off-$vul/shard-$i.json" \
                --score "$score" --show 5 >"$out.shard-$i" 2>&1 &
            [ $(((i + 1) % 8)) -eq 0 ] && wait
            i=$((i + 1))
        done
        wait
        cat "$out".shard-* >"$out"; rm -f "$out".shard-*
    done
}

# Shipped default-on: the OFF arm disables it, the ON arm is the default system.
log "=== competitive-rebid A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-competitive-rebid
    arm on  "$vul"
    diffpair on off "$vul"
done
log "=== competitive-rebid A/B done"
