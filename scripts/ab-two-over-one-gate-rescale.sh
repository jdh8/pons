#!/bin/sh
# ab-two-over-one-gate-rescale.sh — re-probe the major no-fit 2/1 gate under the
# new PointCount scale (277059f, default points = raw HCP + linearised upgrade).
#
# Fix-vs-shipped through ab-point-count's two-book --fix path: BOTH arms build
# the full shipped system (fit leg ON — the fit cases are already good), and the
# treatment flips ONLY the no-fit gate. So only misfit (no-support) 2/1 hands
# move — the "sound 3NT opposite opener's minimum" population. Plain + PD from
# the pre-solved .pdd bank (no live solve). Arms strictly sequential; do NOT
# rebuild the binary while this runs.
#
#   cargo build --release --example ab-point-count
#   setsid nohup scripts/idle-run.sh scripts/ab-two-over-one-gate-rescale.sh \
#       ab-results/two-over-one-gate-rescale \
#       >ab-results/two-over-one-gate-rescale.log 2>&1 </dev/null &
#
# Resumable: a non-empty result file is skipped; slice offsets never replay
# (fresh cursor from bank high-water 22.5M → this run spans 23M..35M).
set -eu
R=${1:?usage: ab-two-over-one-gate-rescale.sh RESULTS_DIR}
DEALS=${DEALS:-/nfs2/jdh8/pons/24.pdd}
BIN=target/release/examples/ab-point-count
COUNT=${COUNT:-2000000}
OFF=${OFF:-23000000}
mkdir -p "$R"

run() { # run GATE VUL OFFSET
    gate=$1 vul=$2 offset=$3
    out="$R/$gate-$vul.txt"
    if [ -s "$out" ]; then
        echo "skip $gate-$vul (already done)"
        return
    fi
    echo "=== $gate vul=$vul offset=$offset count=$COUNT $(date -Is)"
    "$BIN" --fix "two-over-one-gate:$gate" --deals "$DEALS" --offset "$offset" \
        --count "$COUNT" --vulnerability "$vul" --show 40 >"$out.tmp"
    mv "$out.tmp" "$out"
    cat "$out"
}

i=0
for vul in none both; do
    for gate in points13 points12 hcp12; do
        run "$gate" "$vul" "$((OFF + i * COUNT))"
        i=$((i + 1))
    done
done

echo "=== two-over-one gate rescale A/B done $(date -Is); cursor now $((OFF + i * COUNT))"
