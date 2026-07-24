#!/bin/sh
# two-over-one-gate-sd-ab.sh — sd-lead re-measure of the points13 & hcp12 no-fit
# 2/1 gates vs shipped Hcp13, the arbiter between plain-DD and perfect-defense
# for this plain-positive / PD-fragile treatment (docs/convention-tuning.md:
# sd-lead is the tiebreak when plain and PD disagree on a game-reaching action).
#
# Fix-vs-shipped (`--fix`, fit leg ON both arms) so only misfit 2/1s move.
# `--sd` prices the BLIND opening lead single-dummy (16 MC worlds) on divergent
# boards only — LIVE-SOLVING, so the poker worker MUST be stopped first. Arms
# strictly sequential; one fresh sd-seed shared across all arms; do NOT rebuild
# while running.
#
#   systemctl --user stop poker-worker@lines-mtt89.service
#   cargo build --release --example ab-point-count
#   setsid nohup scripts/idle-run.sh scripts/two-over-one-gate-sd-ab.sh \
#       ab-results/two-over-one-gate-sd \
#       >ab-results/two-over-one-gate-sd.log 2>&1 </dev/null &
#   # ... then: systemctl --user start poker-worker@lines-mtt89.service
#
# Resumable: a non-empty result file is skipped; sd-seed persists in $R/sd-seed.
# Fresh slice 36M..40M of 24.pdd (past the 35M plain+PD cursor; bank = 61.70M).
set -eu
R=${1:?usage: two-over-one-gate-sd-ab.sh RESULTS_DIR}
DEALS=${DEALS:-/nfs2/jdh8/pons/24.pdd}
BIN=target/release/examples/ab-point-count
COUNT=${COUNT:-1000000}
OFF=${OFF:-36000000}
mkdir -p "$R"
[ -s "$R/sd-seed" ] || date +%s >"$R/sd-seed"
SDSEED=$(cat "$R/sd-seed")

run() { # run GATE VUL OFFSET
    gate=$1 vul=$2 offset=$3
    out="$R/$gate-$vul.txt"
    if [ -s "$out" ]; then
        echo "skip $gate-$vul (already done)"
        return
    fi
    echo "=== $gate vul=$vul offset=$offset count=$COUNT sdseed=$SDSEED $(date -Is)"
    "$BIN" --fix "two-over-one-gate:$gate" --deals "$DEALS" --offset "$offset" \
        --count "$COUNT" --vulnerability "$vul" --sd --sd-seed "$SDSEED" >"$out.tmp"
    mv "$out.tmp" "$out"
    cat "$out"
}

i=0
for gate in points13 hcp12; do
    for vul in none both; do
        run "$gate" "$vul" "$((OFF + i * COUNT))"
        i=$((i + 1))
    done
done

echo "=== two-over-one gate sd-lead A/B done $(date -Is); cursor now $((OFF + i * COUNT))"
