#!/bin/sh
# balanced-takeout-ab.sh — attribute the 4-4-3-2 takeout suppression by what the
# opponents OPENED (real auction context, not inferred). 5332 is already shipped
# (no 4-card major, X can't find a fit). The anchor split (opener = the takeout-
# short suit) put the 4-4-3-2 loss over MAJOR openings (-3.2 to -3.8 IMPs/div,
# even holding the one unbid 4-card major) vs a mild loss over MINOR openings
# (-1.39). This measures each with the true opener. Three arms per vul:
#   base — default system (5332 suppressed, 4432 doubled)
#   maj  — base + --ns-suppress-4432-vs-major (pass 4432 over a major opening)
#   min  — base + --ns-suppress-4432-vs-minor (pass 4432 over a minor opening)
# diff maj vs base and min vs base price each opener slice. Both scorers, arms
# strictly sequential, one shared
# SEED_BASE (paired diffs need identical deals). Modeled on
# scripts/flat-4333-takeout-ab.sh; do NOT touch the codebase while it runs
# (bba-gen-parallel re-invokes cargo build; it must stay a no-op).
#
#   PER_SHARD=12800 setsid nohup scripts/idle-run.sh \
#       scripts/balanced-takeout-ab.sh ab-results/balanced-takeout-final \
#       >ab-results/balanced-takeout-final.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
set -eu
cd "$(dirname "$0")/.."

R=${1:?usage: balanced-takeout-ab.sh RESULTS_DIR}
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

log "=== balanced-takeout final A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm base "$vul"
    arm maj  "$vul" --ns-suppress-4432-vs-major
    arm min  "$vul" --ns-suppress-4432-vs-minor
    diffpair maj base "$vul"
    diffpair min base "$vul"
done
log "=== balanced-takeout final A/B done"
