#!/bin/sh
# rule-of-20-ab.sh — A/B for the opt-in Rule-of-20 light openings
# (`set_rule_of_20`, docs/bba-gap-campaign.md bucket #2). One two-arm experiment:
# off (default = pass sound 10-11 counts) vs --ns-rule-of-20, both vulnerabilities,
# both scorers, arms strictly sequential, one shared SEED_BASE (paired diffs need
# identical deals). Modeled on scripts/competitive-rebid-ab.sh; do NOT touch the
# codebase while it runs (bba-gen-parallel re-invokes cargo build; must stay a no-op).
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/rule-of-20-ab.sh ab-results/rule-of-20 \
#       >ab-results/rule-of-20.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
set -eu
cd "$(dirname "$0")/.."

R=${1:?usage: rule-of-20-ab.sh RESULTS_DIR}
mkdir -p "$R"
SHA=$(git rev-parse --short HEAD)
DIFF=target/release/examples/ab-dump-diff
PER_SHARD=${PER_SHARD:-6400}
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

# Opt-in default-off: the OFF arm is the current default system, ON adds Rule of 20.
log "=== rule-of-20 A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm on  "$vul" --ns-rule-of-20
    diffpair on off "$vul"
done
log "=== rule-of-20 A/B done"
