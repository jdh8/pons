# ab-lib.sh — shared plumbing for the scripts/*-ab.sh A/B runners.  Not
# executable on its own; a runner sets R (and optionally the knobs below) then
# sources it, and is left with only its experiment body — the arms and diff
# pairs — to spell out.  Sourcing turns on `set -eu`, cds to the repo root,
# builds the harnesses, and defines log / arm / diffpair / sddiff / seed_for.
#
# Honored if set before sourcing:
#   PER_SHARD    boards per shard per arm per vul     (default 6400)
#   SHOW         worst boards ab-dump-diff prints      (default 5)
#   BUILD_EXTRA  extra `cargo build` --example flags   (e.g. --example ab-dump-sd)
#
# A runner that scores single-dummy sets BUILD_EXTRA='--example ab-dump-sd' and
# calls sddiff; the two split-by-opening runners override diffpair/sddiff with
# dir-based variants after sourcing (their shard dirs are split by strain).
set -eu
cd "$(dirname "$0")/.."

: "${R:?source ab-lib.sh with R set to the results dir}"
mkdir -p "$R"
SHA=$(git rev-parse --short HEAD)
DIFF=target/release/examples/ab-dump-diff
SD=target/release/examples/ab-dump-sd
PER_SHARD=${PER_SHARD:-6400}
SHOW=${SHOW:-5}
SHARDS=${JOBS:-$(nproc)}   # must match the shard count bba-gen-parallel.sh creates

# BUILD_EXTRA is a deliberately word-split flag list, not one argument.
# shellcheck disable=SC2086
cargo build --release --features serde --example bba-gen --example ab-dump-diff ${BUILD_EXTRA:-}

log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }

# arm NAME VUL [bba-gen flags...] — generate one arm unless already present
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
                --score "$score" --show "$SHOW" >"$out.shard-$i" 2>&1 &
            [ $(((i + 1) % 8)) -eq 0 ] && wait
            i=$((i + 1))
        done
        wait
        cat "$out".shard-* >"$out"; rm -f "$out".shard-*
    done
}

# sddiff ON OFF VUL [ab-dump-sd flags...] — sd-lead paired delta over all
# shards, 16 worlds; extra flags (e.g. --on-ns-negative-double-shape) disclose
# the ON arm's knobs to the blind leader.
sddiff() {
    on=$1; off=$2; vul=$3; shift 3
    out="$R/sd.$on.vs.$off.$vul.txt"
    [ -s "$out" ] && { log "skip $out (exists)"; return 0; }
    log "sd-diff $on vs $off ($vul, 16 worlds$*)"
    i=0
    while [ "$i" -lt "$SHARDS" ]; do
        "$SD" "$R/$on-$vul/shard-$i.json" "$R/$off-$vul/shard-$i.json" \
            -v "$vul" --sd-worlds 16 --show 0 "$@" >"$out.shard-$i" 2>&1 &
        [ $(((i + 1) % 8)) -eq 0 ] && wait
        i=$((i + 1))
    done
    wait
    cat "$out".shard-* >"$out"; rm -f "$out".shard-*
}

# seed_for [NAME] — a persistent SEED_BASE in $R/[NAME.]seed, fresh on first use.
# No NAME: one $R/seed for the whole run; NAME: one $R/NAME.seed per experiment.
seed_for() {
    f="$R/${1:+$1.}seed"
    if [ ! -s "$f" ]; then date +%s >"$f"; sleep 1; fi
    cat "$f"
}
