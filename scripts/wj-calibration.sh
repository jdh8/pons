#!/bin/sh
# wj-calibration.sh — Step 0 of the WJ-floor campaign: what is BBA's Polish Club
# worth against BBA's 2/1, with our authoring quality entirely out of the way?
#
# Both sides are EPBot, so this measures the SYSTEMS, not the bidders.  It bounds
# the whole campaign's upside: if WJ's minor structure buys nothing even when
# played by its own engine, distilling it as Dutch's floor cannot buy much
# either.  BBA's published bench claims +0.038/+0.054 over 2/1GF.
#
# Single-arm (no pons side), so this is bba-score on one dump, NOT ab-lib's
# paired diffpair — there is no ON/OFF pair to difference.
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/wj-calibration.sh ab-results/wj-calibration \
#       >ab-results/wj-calibration.log 2>&1 &
#
# Resumable: an existing $R/<vul> dir or a non-empty score file is skipped.
set -eu
cd "$(dirname "$0")/.."
R=${1:?usage: wj-calibration.sh RESULTS_DIR}
mkdir -p "$R"
PER_SHARD=${PER_SHARD:-6400}
SHARDS=${JOBS:-$(nproc)}
CARD=vendor/bba/WJ.bbsa
SEED_BASE=$(cat "$R/seed" 2>/dev/null || date +%s)
echo "$SEED_BASE" >"$R/seed"

cargo build --release --features serde --example bba-gen --example bba-score
log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }

log "=== WJ calibration start, sha=$(git rev-parse --short HEAD), SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/vul"
for vul in none both; do
    if [ -d "$R/$vul" ]; then
        log "skip $R/$vul (exists)"
    else
        log "generate $R/$vul"
        SEED_BASE=$SEED_BASE scripts/bba-gen-parallel.sh "$R/$vul" "$PER_SHARD" \
            -v "$vul" --our-card "$CARD" >>"$R/log" 2>&1
    fi
    for score in plain pd; do
        out="$R/score.$vul.$score.txt"
        [ -s "$out" ] && { log "skip $out (exists)"; continue; }
        log "score $vul $score"
        target/release/examples/bba-score "$R/$vul"/shard-*.json \
            --score "$score" >"$out" 2>&1
    done
done
log "=== WJ calibration done"
