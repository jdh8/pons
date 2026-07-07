#!/bin/sh
# takeout-support-ab.sh — A/B for the anchor's #1 bucket (Defensive/book/round-1).
# Two grounded fixes to our defense over a one-suit / weak-two opening:
#   --ns-takeout-support lenient|strict  gates the 12+ takeout double on genuine
#       support for the unbid suits, so an off-shape one-suiter overcalls (or
#       waits for the 17+ any-shape tier) instead of doubling and pulling to the
#       3-level (BBA's two-regime X: 12+ with 3-suit support, else 17+).
#   --ns-overcall-discipline             raises the natural suit-overcall bands to
#       1-level 8-17 / 2-level 11-17 (opening values before a below-their-suit
#       2-level overcall) from the flat 8-16.
#
# One SEED_BASE for the whole experiment (paired diffs need identical deals),
# both vuls, both scorers, arms strictly sequential. See docs/measurement.md and
# the 21gf-ledger. Launch on the shared box as:
#
#   setsid nohup scripts/idle-run.sh scripts/takeout-support-ab.sh \
#       ab-results/takeout-support >ab-results/takeout-support.log 2>&1 &
#
# Resumable: an existing arm dir is skipped; SEED_BASE persists in $R/seed. Do
# NOT touch the codebase while this runs (bba-gen-parallel re-invokes cargo
# build; it must stay a no-op).
set -eu
cd "$(dirname "$0")/.."

R=${1:?usage: takeout-support-ab.sh RESULTS_DIR}
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

# persistent SEED_BASE for the whole experiment (fresh on first use)
if [ ! -s "$R/seed" ]; then date +%s >"$R/seed"; fi
SEED_BASE=$(cat "$R/seed")

log "takeout-support A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    # Explicit knobs on every arm — bba-gen's defaults now ship strict+discipline,
    # so `base` (the historical book) must opt out of both.
    arm base    "$vul" --ns-takeout-support off    --ns-overcall-discipline off
    arm lenient "$vul" --ns-takeout-support lenient --ns-overcall-discipline off
    arm strict  "$vul" --ns-takeout-support strict  --ns-overcall-discipline off
    arm disc    "$vul" --ns-takeout-support off     --ns-overcall-discipline on
    arm combo   "$vul" --ns-takeout-support lenient --ns-overcall-discipline on
    arm strictdisc "$vul" --ns-takeout-support strict --ns-overcall-discipline on
    diffpair lenient base    "$vul"   # takeout support gate (lenient) alone
    diffpair strict  base    "$vul"   # takeout support gate (strict) alone
    diffpair disc    base    "$vul"   # overcall discipline alone
    diffpair combo   base    "$vul"   # the full proposal (lenient + discipline)
    diffpair combo   lenient "$vul"   # marginal of overcall discipline atop lenient
    diffpair strictdisc base   "$vul" # strict support + discipline (best-default candidate)
    diffpair strictdisc strict "$vul" # marginal of discipline atop strict
done

log "takeout-support A/B done"
