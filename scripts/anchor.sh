#!/bin/sh
# anchor.sh — the pons↔BBA gap anchor (docs/bba-gap-campaign.md, Pillar A):
# generate both vulnerability arms at the persistent anchor seed, decompose
# into ranked IMP-loss buckets, and persist the report.
#
#   setsid nohup scripts/idle-run.sh scripts/anchor.sh \
#       >ab-results/anchor.log 2>&1 &
#
# The SERIES dir (ab-results/anchor) holds the persistent seed and the
# deal-keyed DD cache — the sanctioned exception to fresh-seed-per-experiment:
# successive anchors are arms of one longitudinal paired experiment, and the
# cache never invalidates because the seed series always deals the same
# boards.  Each run writes a SNAPSHOT subdir (date + sha) with the shard
# dumps, report.md, and boards.jsonl.  The first run pays the DD solve
# (~20-60 min); re-anchors after a batch of fixes take minutes (generation +
# cache-miss solves only).  Ship decisions stay per-fix fresh-seed A/Bs — the
# anchor is a tracking/attribution instrument (docs/measurement.md governs).
# Do NOT touch the codebase while it runs (bba-gen-parallel re-invokes cargo
# build; it must stay a no-op).
set -eu
cd "$(dirname "$0")/.."

R=${1:-ab-results/anchor}
mkdir -p "$R"
SHA=$(git rev-parse --short HEAD)
SNAP="$R/$(date -u +%F)-$SHA"
PER_SHARD=${PER_SHARD:-6400}

cargo build --release --features serde --example bba-gen --example bba-decompose

log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }

# A persistent SEED_BASE, fresh on first use, shared by every re-anchor.
if [ ! -s "$R/seed" ]; then date +%s >"$R/seed"; fi
SEED_BASE=$(cat "$R/seed")
export SEED_BASE

log "=== anchor start, sha=$SHA, SEED_BASE=$SEED_BASE, $(nproc)x$PER_SHARD bd/arm/vul -> $SNAP"
for vul in none both; do
    dir="$SNAP/$vul"
    [ -d "$dir" ] && { log "skip $dir (exists)"; continue; }
    log "generate $dir"
    # --our-floor american-instinct: bba-decompose replays through the
    # deterministic books; american() now ships the non-decomposable net floor.
    scripts/bba-gen-parallel.sh "$dir" "$PER_SHARD" -v "$vul" \
        --our-floor american-instinct >>"$R/log" 2>&1
done

log "decompose -> $SNAP/report.md"
target/release/examples/bba-decompose "$SNAP/none" "$SNAP/both" \
    --dd-cache "$R/dd-cache.json" \
    --report "$SNAP/report.md" \
    --jsonl "$SNAP/boards.jsonl" \
    2>&1 | tee -a "$R/log"
log "=== anchor done: $SNAP/report.md"
