#!/bin/sh
# ab-face-corroborate.sh — the second Phase-4 follow-up of
# docs/authored-reading-handoff.md § *Phase 4 → Owed*: the RKCB face-rung guard.
#
# *** REFUTED 2026-08-17, and the code change it prices was REVERTED. ***  Kept
# as the reproducibility record for the numbers in that doc's *Follow-up 2*.  To
# re-run it, first re-apply the one-line change it measures: in `answer_trump`
# (`src/bidding/instinct.rs`), `.or_else(|| face_trump(context.auction(), ask))`
# becomes `.or_else(|| face_trump(context.auction(), ask).filter(|&suit|
# corroborated(suit)))`.
#
# `answer_trump`'s two reading rungs already refuse the *answer's own suit*
# unless the pre-answer evidence justified it (the `corroborated` bar, authored
# after a `5♥` step answer was sat in a 4-1 "fit" for −20).  The face rung —
# the hand-free fallback — never carried that bar, and it reads the auction
# BELOW the ask, so it cannot see that the suit it keys is the suit the answer
# happens to name.  `1♥ (2♥) - 2♠ - 3♣ - 3♦ - 4NT - 5♦ -` keyed diamonds off
# our own `3♦` and the asker passed the "minor answer" holding a singleton
# (−16).  This arm extends `corroborated` to that rung, one `.filter`.
#
# The known cost is the mirror failure: the answerer never vetoes (at its turn
# the answer does not exist yet, so `corroborated` is vacuous), so a veto here
# is the ASKER declining to decode an artificial answer already on the table —
# `instinct.rs`'s "hijacked window ... strands the auction in the 1430 answer".
# Whether the judgement fallback bids past it or sits it IS the measurement.
#
# Knobless (an instinct correction), so the arms differ by BINARY:
#   fix    the working tree (walk-shape follow-up 1 + this filter)
#   base   follow-up 1 alone — the `bba-gen-fix` binary of p4-walk-shape,
#          i.e. this change is priced ON TOP of the shipped follow-up 1
#
# Preparation: see scripts/ab-walk-shape.sh's header; the only difference is
# that `bba-gen-base` is copied from p4-walk-shape/bba-gen-fix rather than
# rebuilt from `main`.
#
#   setsid nohup scripts/idle-run.sh scripts/ab-face-corroborate.sh \
#       /mnt/hdd-data/jdh8/pons-ab-results/p4-face-corroborate \
#       >/mnt/hdd-data/jdh8/pons-ab-results/p4-face-corroborate.log 2>&1 </dev/null &
#
# PRE-REGISTERED (before the numbers): this is a floor change, NOT a soundness
# correction — it takes the standard bar, **a win on both scorers ships it**, a
# wash leaves the rung alone (the guard is not free: it costs decodes).  A loss
# gets its worst divergent boards traced for the strand signature above before
# any conclusion.  `smoke-default --count 20000 --seed 1` moves
# (`9c56a4b2…` -> `41f8c9c9…`), so unlike follow-up 1 this one is not
# self-play-inert.
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a fresh seed).
set -eu
R=${1:?usage: ab-face-corroborate.sh RESULTS_DIR}
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

log "=== face-corroborate start, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
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
log "=== face-corroborate done"
