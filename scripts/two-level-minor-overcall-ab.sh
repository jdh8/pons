#!/bin/sh
# two-level-minor-overcall-ab.sh — A/B for the opt-in tight 2-level minor
# overcall (`set_two_level_minor_overcall_tight`, docs/bba-gap-campaign.md
# def-r1 overcall lever). OFF arm = shipped default (2♣/2♦ overcall at 11+);
# ON arm = --ns-two-level-minor-overcall-tight (demand 15+, strand the losing
# 11–14 minimums into Pass). Both vulnerabilities, THREE scorers (plain + pd via
# ab-dump-diff, sd-lead via ab-dump-sd — the trustworthy scorer for a
# competitive range), arms strictly sequential, one shared SEED_BASE. Do NOT
# touch the codebase while it runs (bba-gen-parallel re-invokes cargo build;
# must stay a no-op).
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/two-level-minor-overcall-ab.sh ab-results/two-level-minor-overcall \
#       >ab-results/two-level-minor-overcall.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
set -eu
cd "$(dirname "$0")/.."

R=${1:?usage: two-level-minor-overcall-ab.sh RESULTS_DIR}
mkdir -p "$R"
SHA=$(git rev-parse --short HEAD)
DIFF=target/release/examples/ab-dump-diff
SD=target/release/examples/ab-dump-sd
PER_SHARD=${PER_SHARD:-6400}
SHARDS=$(nproc)

cargo build --release --features serde --example bba-gen --example ab-dump-diff --example ab-dump-sd

log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }

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

# diffpair ON OFF VUL — per-shard paired diff, plain + pd, 8 solvers wide
diffpair() {
    on=$1; off=$2; vul=$3
    for score in plain pd; do
        out="$R/diff.$on.vs.$off.$vul.$score.txt"
        [ -s "$out" ] && { log "skip $out (exists)"; continue; }
        log "diff $on vs $off ($vul, $score)"
        i=0
        while [ "$i" -lt "$SHARDS" ]; do
            "$DIFF" "$R/$on-$vul/shard-$i.json" "$R/$off-$vul/shard-$i.json" \
                --score "$score" --show 4 >"$out.shard-$i" 2>&1 &
            [ $(((i + 1) % 8)) -eq 0 ] && wait
            i=$((i + 1))
        done
        wait
        cat "$out".shard-* >"$out"; rm -f "$out".shard-*
    done
}

# sddiff ON OFF VUL — sd-lead paired delta over all shards concatenated
sddiff() {
    on=$1; off=$2; vul=$3
    out="$R/sd.$on.vs.$off.$vul.txt"
    [ -s "$out" ] && { log "skip $out (exists)"; return 0; }
    log "sd-diff $on vs $off ($vul, 16 worlds)"
    i=0
    while [ "$i" -lt "$SHARDS" ]; do
        "$SD" "$R/$on-$vul/shard-$i.json" "$R/$off-$vul/shard-$i.json" \
            -v "$vul" --sd-worlds 16 --show 0 >"$out.shard-$i" 2>&1 &
        [ $(((i + 1) % 8)) -eq 0 ] && wait
        i=$((i + 1))
    done
    wait
    cat "$out".shard-* >"$out"; rm -f "$out".shard-*
}

log "=== two-level-minor-overcall A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm on  "$vul" --ns-two-level-minor-overcall-tight
    diffpair on off "$vul"
    sddiff on off "$vul"
done
log "=== two-level-minor-overcall A/B done"
