#!/bin/sh
# ab-diamond-floor.sh — B2.4 of docs/bba-gap-campaign.md.
#
# Under ReadingScope::All the authored natural 1♦ rule substitutes for the
# walk, but its opaque prefers_diamonds() predicate projected no diamond axis.
# The fix states the implied 3+ ♦ floor explicitly, restoring the reading that
# partner_shown_len(♦, 3..) gates throughout the system.
#
# Knobless — a projection correction, not a treatment — so the arms differ by
# BINARY, not by flag:
#   fix    the working tree
#   base   main HEAD (04bd6432)
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
#   setsid nohup scripts/idle-run.sh scripts/ab-diamond-floor.sh \
#       /mnt/hdd-data/jdh8/pons-ab-results/b2-4-diamond-floor \
#       >/mnt/hdd-data/jdh8/pons-ab-results/b2-4-diamond-floor.log 2>&1 </dev/null &
#
# PRE-REGISTERED (before the numbers): B2.4 is a soundness correction on the
# same ledger as Phases 1–4, so it takes the reading-drift bar — non-loss ships
# it (plain wash-or-win AND PD non-loss, both vuls). A plain win with a PD loss
# is a doubling artifact and does not ship. Any loss gets
# `ab-dump-bucket --by node` and its worst divergent boards traced before any
# conclusion. Likely suspects are slam decisions: partner_shown_len(♦, 3..)
# feeds known_eight_card_fit and fit_sum_game, and the floor also reaches the
# v6 nets through features::push_inference.
#
# RESULT (2026-08-18): REJECTED. Three 204,800-board seeds per arm/vul
# (1787039750 / 1787040222 / 1787040684) were negative in 11/12 cells. The
# 614,400-board pools lost under PD non-vul (-0.0022 +/- 0.0020 IMP/bd) and
# plain DD vul (-0.0024 +/- 0.0020), so the atom was not retained. Every moved
# call followed our pair's 1♦ opening; BBA-opened 1♦ subsets moved zero calls.
# See the B2.4 row in docs/bba-gap-campaign.md for the soundness and node trace.
#
# Resumable: existing arm dirs and non-empty diff files are skipped; SEED_BASE
# persists in $R/seed (a NEW dir -> a fresh seed).
set -eu
R=${1:?usage: ab-diamond-floor.sh RESULTS_DIR}
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

log "=== diamond-floor start, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
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
log "=== diamond-floor done"
