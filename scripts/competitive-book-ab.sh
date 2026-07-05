#!/bin/sh
# competitive-book-ab.sh — the chained A/B campaign for the competitive-book
# packages (docs/competitive-book.md). One SEED_BASE per experiment shared
# across its arms (paired diffs need identical deals); a fresh base per
# experiment; arms strictly sequential; both scorers, both vulnerabilities.
#
#   setsid nohup scripts/idle-run.sh scripts/competitive-book-ab.sh \
#       ab-results/competitive-book >ab-results/competitive-book.log 2>&1 &
#
# Resumable: an arm directory that already exists is skipped, and each
# experiment's SEED_BASE persists in $R/<exp>.seed, so a restart regenerates
# nothing and stays seed-aligned. Do NOT touch the codebase while this runs
# (bba-gen-parallel re-invokes cargo build; it must stay a no-op).
#
# NOTE: the 2026-07 campaign ran at sha bc949dc with the pre-flip polarity
# below (--ns-* = on arm). The four winners have since shipped default-on
# (uvu-over-majors, strong-two-comp, major-support-double, jordan-truscott:
# now --no-ns-* for the OFF arm) — rerunning this script as-is would compare
# arms of the remaining opt-in knobs only. Adjust flags before reusing.
set -eu
cd "$(dirname "$0")/.."

R=${1:?usage: competitive-book-ab.sh RESULTS_DIR}
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

# seed_for EXP — a persistent per-experiment SEED_BASE (fresh on first use).
seed_for() {
    f="$R/$1.seed"
    if [ ! -s "$f" ]; then
        date +%s >"$f"
        sleep 1 # the next experiment's base differs even on a fast pass
    fi
    cat "$f"
}

log "campaign start, sha=$SHA, shards=$SHARDS x $PER_SHARD boards/arm/vul"

# --- simple two-arm experiments -------------------------------------------
# exp name, knob flag
run_two_arm() {
    exp=$1
    flag=$2
    SEED_BASE=$(seed_for "$exp")
    log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA"
    for vul in none both; do
        arm "$exp-off" "$vul"
        arm "$exp-on" "$vul" "$flag"
        diffpair "$exp-on" "$exp-off" "$vul"
    done
}

run_two_arm p1-uvu-majors --ns-uvu-over-majors
run_two_arm p2a-weak-two --ns-weak-two-comp
run_two_arm p2b-strong-two --ns-strong-two-comp
run_two_arm p3c-support-x --ns-major-support-double
run_two_arm p3a-high-ovc --ns-high-overcall
run_two_arm p4-jordan --ns-jordan-truscott

# --- the four-arm negative-double experiment ------------------------------
exp=p3bd-negx
SEED_BASE=$(seed_for "$exp")
log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA"
for vul in none both; do
    arm "$exp-off" "$vul"
    arm "$exp-free" "$vul" --ns-free-bids
    arm "$exp-modern" "$vul" --ns-negative-double-shape modern
    arm "$exp-cachalot" "$vul" --ns-negative-double-shape cachalot
    diffpair "$exp-free" "$exp-off" "$vul"
    diffpair "$exp-modern" "$exp-off" "$vul"
    diffpair "$exp-modern" "$exp-free" "$vul"
    diffpair "$exp-cachalot" "$exp-modern" "$vul"
    diffpair "$exp-cachalot" "$exp-off" "$vul"
done

log "campaign done"
