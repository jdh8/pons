#!/bin/sh
# passed-hand-overcall-ab.sh — marginal A/B for the passed-hand 2-level overcall
# carve-out atop the shipped strict+discipline default.
#
#   --ns-passed-hand-overcall  lets a *passed hand* take the disciplined 2-level
#       overcall lighter (9+ not the opening 11+): it cannot hold opening values,
#       so the 11+ floor would all but forbid the safe light overcall.  1-level
#       untouched.  Off by default.
#
# base = bare bba-gen (the shipped strict+discipline system); carve = base + the
# knob.  Fresh SEED_BASE for the whole experiment (paired diffs need identical
# deals), both vuls, both scorers, arms sequential.  See docs/measurement.md.
#
#   setsid nohup scripts/idle-run.sh scripts/passed-hand-overcall-ab.sh \
#       ab-results/passed-hand-overcall >ab-results/passed-hand-overcall.log 2>&1 &
#
# Resumable: an existing arm dir is skipped; SEED_BASE persists in $R/seed. Do
# NOT touch the codebase while this runs (bba-gen-parallel re-invokes cargo
# build; it must stay a no-op).
set -eu
cd "$(dirname "$0")/.."

R=${1:?usage: passed-hand-overcall-ab.sh RESULTS_DIR}
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

log "passed-hand-overcall A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    arm base  "$vul"                                # shipped strict+discipline
    arm carve "$vul" --ns-passed-hand-overcall      # + passed-hand 2-level 9+
    diffpair carve base "$vul"
done

log "passed-hand-overcall A/B done"
