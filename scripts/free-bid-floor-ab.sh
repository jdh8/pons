#!/bin/sh
# free-bid-floor-ab.sh — sweep the 1-level free-bid floor (set_free_bid_floor /
# --ns-free-bid-floor) to trim the vulnerable-PD leak the whole free-bid family
# (free bids, Modern, Cachalot, Sputnik) inherits. Prior campaign: free6 and
# Modern6 both lose vs off on the vul cells to the 6-count 1-level/1NT floor.
#
# Sweep {6,7,8} on the plain free-bid arm to find the floor, then confirm the
# winner on Modern (the family representative — Cachalot ≈ Modern ≈ Sputnik).
# One SEED_BASE for the experiment (paired diffs need identical deals), arms
# strictly sequential, both scorers, both vulnerabilities.
#
#   setsid nohup scripts/idle-run.sh scripts/free-bid-floor-ab.sh \
#       ab-results/free-bid-floor >ab-results/free-bid-floor.log 2>&1 &
#
# Resumable: an arm dir / diff file that already exists is skipped; the
# SEED_BASE persists in $R/free-bid-floor.seed. Do NOT rebuild the binaries
# while this runs.
set -eu
cd "$(dirname "$0")/.."

R=${1:?usage: free-bid-floor-ab.sh RESULTS_DIR}
mkdir -p "$R"
SHA=$(git rev-parse --short HEAD)
DIFF=target/release/examples/ab-dump-diff
PER_SHARD=6400
SHARDS=$(nproc)

cargo build --release --features serde --example bba-gen --example ab-dump-diff

log() { echo "$(date -u +%FT%TZ) $*" >>"$R/log"; }

# arm NAME VUL [bba-gen flags...] — generate one arm unless already present.
arm() {
    name=$1
    vul=$2
    shift 2
    dir="$R/$name-$vul"
    if [ -d "$dir" ]; then
        log "skip $dir (exists)"
        return 0
    fi
    log "generate $dir (SEED_BASE=$SEED_BASE, flags: $*)"
    SEED_BASE=$SEED_BASE scripts/bba-gen-parallel.sh "$dir" "$PER_SHARD" -v "$vul" "$@" \
        >>"$R/log" 2>&1
}

# diffpair ON OFF VUL — per-shard paired diff, both scorers, 8 solvers wide.
diffpair() {
    on=$1
    off=$2
    vul=$3
    for score in plain pd; do
        out="$R/diff.$on.vs.$off.$vul.$score.txt"
        if [ -s "$out" ]; then
            log "skip $out (exists)"
            continue
        fi
        log "diff $on vs $off ($vul, $score)"
        i=0
        while [ "$i" -lt "$SHARDS" ]; do
            "$DIFF" "$R/$on-$vul/shard-$i.json" "$R/$off-$vul/shard-$i.json" \
                --score "$score" --show 3 >"$out.shard-$i" 2>&1 &
            [ $(((i + 1) % 8)) -eq 0 ] && wait
            i=$((i + 1))
        done
        wait
        cat "$out".shard-* >"$out"
        rm -f "$out".shard-*
    done
}

exp=free-bid-floor
[ -s "$R/$exp.seed" ] || date +%s >"$R/$exp.seed"
SEED_BASE=$(cat "$R/$exp.seed")
log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    arm "$exp-off" "$vul"
    arm "$exp-free6" "$vul" --ns-free-bids --ns-free-bid-floor 6
    arm "$exp-free7" "$vul" --ns-free-bids --ns-free-bid-floor 7
    arm "$exp-free8" "$vul" --ns-free-bids --ns-free-bid-floor 8
    arm "$exp-modern6" "$vul" --ns-negative-double-shape modern --ns-free-bid-floor 6
    arm "$exp-modern8" "$vul" --ns-negative-double-shape modern --ns-free-bid-floor 8
    diffpair "$exp-free6" "$exp-off" "$vul"       # reproduce the leak (anchor)
    diffpair "$exp-free7" "$exp-off" "$vul"       # floor 7 vs baseline
    diffpair "$exp-free8" "$exp-off" "$vul"       # floor 8 vs baseline (ship gate)
    diffpair "$exp-free8" "$exp-free6" "$vul"     # mechanism: what the bump buys
    diffpair "$exp-modern8" "$exp-off" "$vul"     # family ship gate
    diffpair "$exp-modern8" "$exp-modern6" "$vul" # mechanism on Modern
done

log "campaign done"
