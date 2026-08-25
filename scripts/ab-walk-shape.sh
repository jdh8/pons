#!/bin/sh
# ab-walk-shape.sh — the first Phase-4 follow-up of
# docs/authored-reading-handoff.md § *Phase 4 → Owed*: walk length floors under
# the exclusion fold.
#
# A `hcp(0..)` catch-all projects ⊤ and hands its call to the natural walk.
# Under `bid_exclusion` the fold makes `⊤ ∩ ¬(heavier siblings)` informative,
# `substitutes_natural` fires, and the walk is suppressed — taking its *length
# floors* with it.  `(1♥) 2♥ - 2♠` read the Michaels advance ♠ 0..13 instead of
# ♠ 3..13, `answer_trump`'s provable-eight rung then failed, and the keycard
# ladder keyed a suit the asker was void in (the two −16/−17 RKCB boards Phase
# 4's own A/B census filed).
#
# The fix splits the two halves of a reading: such a call keeps reading its
# SHAPE off the walk (`CallMasks::walk_shape`) and its STRENGTH off the fold —
# which is the half the fold alone knows, and the half Phase 4 shipped for
# (`bid_exclusion_admits_the_jacoby_sign_off` pins it, and stays green).
#
# Knobless — a projection correction, not a treatment — so the arms differ by
# BINARY, not by flag:
#   fix    the working tree
#   base   `main` HEAD
#
# Preparation (from the repo root, the fix in the working tree):
#   cargo +nightly build --release --features serde --example bba-gen --example ab-dump-diff
#   mkdir -p $R
#   cp target/release/examples/bba-gen      $R/bba-gen-fix
#   cp target/release/examples/ab-dump-diff $R/ab-dump-diff
#   git stash && cargo +nightly build --release --features serde --example bba-gen
#   cp target/release/examples/bba-gen      $R/bba-gen-base
#   git stash pop
#
#   setsid nohup scripts/idle-run.sh scripts/ab-walk-shape.sh \
#       /mnt/hdd-data/jdh8/pons-ab-results/p4-walk-shape \
#       >/mnt/hdd-data/jdh8/pons-ab-results/p4-walk-shape.log 2>&1 </dev/null &
#
# PRE-REGISTERED (before the numbers): this is a precision correction on the
# same soundness ledger as Phase 4 itself, so it takes the reading-drift bar —
# **non-loss ships it** (plain wash-or-win AND PD non-loss, both vuls).  A
# plain win with a PD loss is a doubling artifact and does not ship.  Any loss
# gets `ab-dump-bucket --by node` and its worst divergent boards traced first.
# `smoke-default --count 20000 --seed 1` is already byte-identical at
# `9c56a4b2…`, so self-play is untouched; the whole footprint is competitive.
#
# Resumable: existing arm dirs and non-empty diff files are skipped; SEED_BASE
# persists in $R/seed (a NEW dir -> a fresh seed).
set -eu
R=${1:?usage: ab-walk-shape.sh RESULTS_DIR}
cd "$(dirname "$0")/.."
mkdir -p "$R"

PER_SHARD=${PER_SHARD:-6400}
SHARDS=${JOBS:-$(nproc)}
SHOW=${SHOW:-40}

for bin in bba-gen-fix bba-gen-base ab-dump-diff; do
    [ -x "$R/$bin" ] || { echo "missing $R/$bin — see the preparation comment" >&2; exit 2; }
done

seed_f="$R/seed"
[ -s "$seed_f" ] || date +%s >"$seed_f"
SEED_BASE=$(cat "$seed_f")

log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }

# arm BIN NAME VUL — one arm, shard-parallel, same SEED_BASE both arms
arm() {
    bin=$1; name=$2; vul=$3
    dir="$R/$name-$vul"
    [ -d "$dir" ] && { log "skip $dir (exists)"; return 0; }
    log "generate $dir ($bin, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD})"
    mkdir -p "$dir.tmp"
    i=0
    while [ "$i" -lt "$SHARDS" ]; do
        "$R/$bin" --count "$PER_SHARD" --seed "$((SEED_BASE + i))" \
            --vulnerability "$vul" --output "$dir.tmp/shard-$i.json" &
        i=$((i + 1))
    done
    wait
    mv "$dir.tmp" "$dir"
}

log "=== walk-shape start, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm bba-gen-fix fix "$vul"
    arm bba-gen-base base "$vul"
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
log "=== walk-shape done"
