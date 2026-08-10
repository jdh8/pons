#!/bin/sh
# ab-reading-drift.sh — the batched A/B for the reading-drift repairs
# (docs/reading-drift-handoff.md, campaign ledger).  The repairs are knobless
# code corrections (walk arms, the at-the-time projection context, the strip
# re-key, the transfer-GF 3NT gates), so the arms differ by BINARY, not by
# flag: prebuild both bidders and the diff harness, copy them into $R, and
# this script never invokes cargo — the no-rebuild-in-flight rule holds by
# construction.
#
# Preparation (from the repo root, fixes in the working tree):
#   cargo build --release --features serde --example bba-gen --example ab-dump-diff
#   mkdir -p ab-results/reading-drift
#   cp target/release/examples/bba-gen      ab-results/reading-drift/bba-gen-fix
#   cp target/release/examples/ab-dump-diff ab-results/reading-drift/ab-dump-diff
#   git stash && cargo build --release --features serde --example bba-gen
#   cp target/release/examples/bba-gen      ab-results/reading-drift/bba-gen-base
#   git stash pop && cargo build --release --features serde --example bba-gen
#
#   setsid nohup scripts/idle-run.sh scripts/ab-reading-drift.sh \
#       ab-results/reading-drift >ab-results/reading-drift.log 2>&1 </dev/null &
#
# Decision table (docs/measurement.md): the repairs are soundness corrections,
# so a non-loss (wash or win, plain and PD read together) ships them as they
# stand; a loss gets its worst divergent boards traced before any conclusion.
#
# Resumable: existing arm dirs and non-empty diff files are skipped;
# SEED_BASE persists in $R/seed (a NEW dir -> a fresh seed).
set -eu
R=${1:?usage: ab-reading-drift.sh RESULTS_DIR}
cd "$(dirname "$0")/.."
mkdir -p "$R"

PER_SHARD=${PER_SHARD:-6400}
SHARDS=${JOBS:-$(nproc)}
SHOW=${SHOW:-40}
# Optional extra bba-gen flags per arm (word-split intentionally), for A/Bs
# where the arms differ by knob rather than binary — the same binary is then
# copied to both bba-gen-fix and bba-gen-base.
FIX_ARGS=${FIX_ARGS:-}
BASE_ARGS=${BASE_ARGS:-}

for bin in bba-gen-fix bba-gen-base ab-dump-diff; do
    [ -x "$R/$bin" ] || { echo "missing $R/$bin — see the preparation comment" >&2; exit 2; }
done

seed_f="$R/seed"
[ -s "$seed_f" ] || date +%s >"$seed_f"
SEED_BASE=$(cat "$seed_f")

log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }

# arm BIN NAME VUL ARGS — one arm, shard-parallel, same SEED_BASE both arms
arm() {
    bin=$1; name=$2; vul=$3; args=${4:-}
    dir="$R/$name-$vul"
    [ -d "$dir" ] && { log "skip $dir (exists)"; return 0; }
    log "generate $dir ($bin $args, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD})"
    mkdir -p "$dir.tmp"
    i=0
    while [ "$i" -lt "$SHARDS" ]; do
        # shellcheck disable=SC2086 — $args word-splits by design
        "$R/$bin" --count "$PER_SHARD" --seed "$((SEED_BASE + i))" \
            --vulnerability "$vul" --output "$dir.tmp/shard-$i.json" $args &
        i=$((i + 1))
    done
    wait
    mv "$dir.tmp" "$dir"
}

log "=== reading-drift start, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm bba-gen-fix fix "$vul" "$FIX_ARGS"
    arm bba-gen-base base "$vul" "$BASE_ARGS"
    plain="$R/diff.fix.vs.base.$vul.plain.txt"
    pd="$R/diff.fix.vs.base.$vul.pd.txt"
    if [ -s "$plain" ] && [ -s "$pd" ]; then
        log "skip $plain + $pd (exist)"
    else
        log "diff fix vs base ($vul, plain+pd)"
        "$R/ab-dump-diff" "$R/fix-$vul" "$R/base-$vul" \
            --score both --out-plain "$plain" --out-pd "$pd" --show "$SHOW" >>"$R/log" 2>&1
    fi
done
log "=== reading-drift done"
