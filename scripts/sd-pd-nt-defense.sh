#!/bin/sh
# sd-pd-nt-defense.sh — Tier 4 of the SD-PD re-adjudication: the GTO 1NT-defense
# tournament (docs/ai-bidder/gto-1nt-defense.md), re-run with the fourth scorer.
#
# The flagship.  That tournament's headline — "Woolsey, both vulnerabilities" —
# carries the doc's own "sd-lead-trusting asterisk": PD says pass at vul, plain
# SD says Woolsey, and plain SD is the optimistic one (blind lead *and* nobody
# doubles).  A 1NT-level partscore fight is exactly where doubling is realistic,
# so SD-PD arbitrates here rather than merely stress-testing.
#
# `ab-nt-defense-matrix` now carries an "sd-lead + perfect defense" matrix and
# feeds it to the fictitious-play equilibrium and the bootstrap, so the answer
# arrives as its own equilibrium row, not a number to eyeball.
#
# This is a **fresh measurement, not a rescore** — the harness re-bids, and the
# shipped book has moved since 2026-07-03, so the published cells will not
# reprint.  That costs nothing: the plain-SD vs SD-PD contrast is internal to
# one run, and both DD brackets come along to place it.
#
#   BINDIR=<path>/release/examples setsid nohup scripts/idle-run.sh \
#       scripts/sd-pd-nt-defense.sh ab-results/sd-pd-nt-defense \
#       >ab-results/sd-pd-nt-defense.log 2>&1 </dev/null &
#
# Resumable: a non-empty result file is skipped; the seed persists.
set -eu
R=${1:?usage: sd-pd-nt-defense.sh RESULTS_DIR}
BINDIR=${BINDIR:?set BINDIR to the release examples directory}
COUNT=${COUNT:-60000}
WORLDS=${WORLDS:-16}
mkdir -p "$R"

f="$R/seed"
[ -s "$f" ] || date +%s >"$f"
SEED=$(cat "$f")

i=0
for vul in none both; do
    out="$R/matrix.$vul.txt"
    i=$((i + 1))
    if [ -s "$out" ]; then
        echo "skip $vul (already done)"
        continue
    fi
    echo "=== matrix vul=$vul count=$COUNT worlds=$WORLDS seed=$((SEED + i)) $(date -Is)"
    "$BINDIR/ab-nt-defense-matrix" --count "$COUNT" -v "$vul" \
        --sd-worlds "$WORLDS" --seed "$((SEED + i))" >"$out.tmp"
    mv "$out.tmp" "$out"
    cat "$out"
done

echo "=== sd-pd 1NT-defense matrix done $(date -Is)"
