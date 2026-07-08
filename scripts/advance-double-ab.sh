#!/bin/sh
# advance-double-ab.sh — price the RICH advance of partner's takeout double of a
# one-of-a-suit opening ([1t, X, P] → advancer acts).  The flat floor gives the
# advancer only a cheapest natural suit, a 3NT, and a penalty pass — the whole
# 10+ invitational-or-better band collapses into "bid your cheapest suit," with
# no way to invite or force.  --ns-rich-advance adds the cue-asks-for-a-major
# Stayman-ask, a 1NT/2NT/3NT ladder, weak shapely game jumps, and a forced
# 3-card response when broke.  Two arms per vul:
#   base — default system (flat advance_double)
#   rich — base + --ns-rich-advance
# diff rich vs base prices the change.  Both scorers, arms strictly sequential,
# one shared SEED_BASE (paired diffs need identical deals).  Modeled on
# scripts/balanced-takeout-ab.sh; do NOT touch the codebase while it runs.
#
#   PER_SHARD=12800 setsid nohup scripts/idle-run.sh \
#       scripts/advance-double-ab.sh ab-results/advance-double \
#       >ab-results/advance-double.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
set -eu
cd "$(dirname "$0")/.."

R=${1:?usage: advance-double-ab.sh RESULTS_DIR}
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

log "=== advance-double A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm base  "$vul"
    arm rich  "$vul" --ns-rich-advance
    arm rubens "$vul" --ns-rich-advance --ns-advance-rubens
    diffpair rich   base "$vul"   # baseline (known wash)
    diffpair rubens rich "$vul"   # the Rubens increment (right-siding is DD-invisible)
done
log "=== advance-double A/B done"
