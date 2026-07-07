#!/bin/sh
# splinter-doubled-ab.sh — two-arm A/B for the systems-on-over-doubled-splinter
# rebase (set_splinter_doubled / bba-gen --ns-splinter-doubled). Mirrors
# sputnik-negx-ab.sh: one SEED_BASE for the experiment (paired diffs need
# identical deals), arms strictly sequential, both scorers, both vulnerabilities.
#
#   setsid nohup scripts/idle-run.sh scripts/splinter-doubled-ab.sh \
#       ab-results/splinter-doubled >ab-results/splinter-doubled.log 2>&1 &
#
# The knob only changes opener's rebid AFTER their double of our splinter, so the
# arms are byte-identical up through the double — no --advertise needed, and the
# divergent set is exactly the fired set (read IMPs/fired straight off).
#
# Resumable: an arm dir / diff file that already exists is skipped; the SEED_BASE
# persists in $R/splinter-doubled.seed. Do NOT rebuild the binaries while this
# runs (bba-gen-parallel re-invokes cargo build; keep it a no-op).
set -eu
cd "$(dirname "$0")/.."

R=${1:?usage: splinter-doubled-ab.sh RESULTS_DIR}
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
                --score "$score" --show 5 >"$out.shard-$i" 2>&1 &
            [ $(((i + 1) % 8)) -eq 0 ] && wait
            i=$((i + 1))
        done
        wait
        cat "$out".shard-* >"$out"
        rm -f "$out".shard-*
    done
}

exp=splinter-doubled
[ -s "$R/$exp.seed" ] || date +%s >"$R/$exp.seed"
SEED_BASE=$(cat "$R/$exp.seed")
log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    arm "$exp-off" "$vul"
    arm "$exp-on" "$vul" --ns-splinter-doubled
    diffpair "$exp-on" "$exp-off" "$vul" # ship gate (vs the byte-identical default)
done

log "campaign done"
