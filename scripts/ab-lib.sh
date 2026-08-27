# ab-lib.sh — shared plumbing for the scripts/*-ab.sh A/B runners.  Not
# executable on its own; a runner sets R (and optionally the knobs below) then
# sources it, and is left with only its experiment body — the arms and diff
# pairs — to spell out.  Sourcing turns on `set -eu`, cds to the repo root,
# builds the harnesses, and defines log / arm / gatepair / diffpair / sddiff /
# seed_for.
#
# Honored if set before sourcing:
#   PER_SHARD    boards per shard per arm per vul     (default 6400)
#   SHOW         worst boards ab-dump-diff prints      (default 5)
#   BUILD_EXTRA  extra `cargo build` --example flags   (e.g. --example ab-dump-sd)
#
# Sourcing exports SKIP_BUILD=1, so the build it does here is the only one the
# whole experiment gets; see the comment at that line.
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
PROBE=target/release/examples/probe-divergence
PER_SHARD=${PER_SHARD:-6400}
SHOW=${SHOW:-5}
SHARDS=${JOBS:-$(nproc)}   # shard count bba-gen-parallel.sh creates; runners log it

# BUILD_EXTRA is a deliberately word-split flag list, not one argument.
# shellcheck disable=SC2086
cargo build --release --features serde --example bba-gen --example ab-dump-diff ${BUILD_EXTRA:-}

# One build per experiment, here.  bba-gen-parallel.sh otherwise rebuilds at the
# head of *every arm*, so a `src/` edit mid-run makes late arms measure different
# code from early ones with nothing in the log to say so.  With this exported it
# hard-fails on a missing binary instead.  Cost: a run started on a dirty or
# stale tree measures this build for all its arms -- which is the point.
export SKIP_BUILD=1

log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }

# arm NAME VUL [bba-gen flags...] — generate one arm unless already present
#
# The resume check tests for a SHARD, not for the directory: bba-gen-parallel
# mkdirs before it launches a worker, so an arm that died on startup (a stale
# flag, a bad card path) leaves an empty dir behind.  Skipping on `-d` would
# then silently resume past an arm that was never generated, and diffpair would
# score it against nothing.
arm() {
    name=$1; vul=$2; shift 2
    dir="$R/$name-$vul"
    [ -s "$dir/shard-0.json" ] && { log "skip $dir (exists)"; return 0; }
    log "generate $dir (SEED_BASE=$SEED_BASE, flags: $*)"
    SEED_BASE=$SEED_BASE scripts/bba-gen-parallel.sh "$dir" "$PER_SHARD" -v "$vul" "$@" \
        >>"$R/log" 2>&1
}

# gatepair ON OFF VUL — the isolation gate that must precede any headline
# (docs/measurement.md): it must read **0 foreign**, i.e. no divergent board was
# opened by the other side.  Needs BUILD_EXTRA='--example probe-divergence'.
#
# Runners written before 2026-08-27 define this themselves and shadow it; the
# bodies are identical, so nothing changes for them.
gatepair() {
    on=$1; off=$2; vul=$3
    out="$R/gate.$on.vs.$off.$vul.txt"
    [ -s "$out" ] && { log "skip $out (exists)"; return 0; }
    log "isolation gate $on vs $off ($vul)"
    "$PROBE" "$R/$on-$vul" "$R/$off-$vul" --gate-opener ours >"$out"
}

# diffpair ON OFF VUL — paired diff over the whole arm, plain + pd.  ab-dump-diff
# folds every shard of the arm dir into a single DDS fan-out — one solver owns
# all cores instead of one process per shard oversubscribing the box.
#
# One invocation, both brackets: a scorer only re-prices solved tables, so
# `--score both` pays for the shard parse and the DDS fan-out once.  Running it
# twice, once per scorer, paid for each twice.
#
# The skip guard wants BOTH files, so a run killed between the two writes redoes
# the cell instead of resuming past a half-written pair.
diffpair() {
    on=$1; off=$2; vul=$3
    plain="$R/diff.$on.vs.$off.$vul.plain.txt"
    pd="$R/diff.$on.vs.$off.$vul.pd.txt"
    [ -s "$plain" ] && [ -s "$pd" ] && { log "skip $plain + $pd (exist)"; return 0; }
    log "diff $on vs $off ($vul, plain+pd)"
    "$DIFF" "$R/$on-$vul" "$R/$off-$vul" --score both \
        --out-plain "$plain" --out-pd "$pd" --show "$SHOW" >>"$R/log" 2>&1
}

# sddiff ON OFF VUL [ab-dump-sd flags...] — sd-lead paired delta over the whole
# arm, 16 worlds, one solver; extra flags (e.g. --on-ns-negative-double-shape)
# disclose the ON arm's knobs to the blind leader.
sddiff() {
    on=$1; off=$2; vul=$3; shift 3
    out="$R/sd.$on.vs.$off.$vul.txt"
    [ -s "$out" ] && { log "skip $out (exists)"; return 0; }
    log "sd-diff $on vs $off ($vul, 16 worlds$*)"
    "$SD" "$R/$on-$vul" "$R/$off-$vul" -v "$vul" --sd-worlds 16 --show 0 "$@" >"$out" 2>&1
}

# seed_for [NAME] — a persistent SEED_BASE in $R/[NAME.]seed, fresh on first use.
# No NAME: one $R/seed for the whole run; NAME: one $R/NAME.seed per experiment.
seed_for() {
    f="$R/${1:+$1.}seed"
    if [ ! -s "$f" ]; then date +%s >"$f"; sleep 1; fi
    cat "$f"
}
