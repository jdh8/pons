#!/bin/sh
# sd-pd-readjudicate.sh — replay a self-play sd A/B at its PUBLISHED seed so the
# new SD-PD row can be read beside the plain-SD number that decided it.
#
# Why replay instead of a fresh seed: the treatment is unchanged and only the
# *scorer* moves, so the original boards isolate the doubling with zero sampling
# noise against the published figure.  Fresh seeds are for new treatments; a
# flipped verdict then gets a fresh-seed confirmation before anything is acted on.
#
# THE GATE (docs/measurement.md): the plain-DD, PD and plain-SD figures must
# reproduce the published run EXACTLY.  A mismatch is seed/binary drift, not a
# verdict — stop and re-derive rather than reading the SD-PD number.
#
# Arm 1 — set_fuzzy_points, docs/bidding-options.md:263.  Shipped default-on on
# plain-SD +0.1639/+0.1939 while perfect defense said -0.0363/-0.0399: the
# textbook plain-SD-overrules-PD ship, and the top of the re-adjudication queue.
# Published (ab-results/a6-fuzzy-sd, seed 1783925001, 300k boards, 16 worlds):
#   vul none: plain +0.1073, PD -0.0322, sd-lead +0.1639, divergent 41261
#   vul both: plain +0.1191, PD -0.0356, sd-lead +0.1939, divergent 41261
#
#   BIN=<path>/ab-fuzzy-strength setsid nohup scripts/idle-run.sh \
#       scripts/sd-pd-readjudicate.sh ab-results/sd-pd-fuzzy \
#       >ab-results/sd-pd-fuzzy.log 2>&1 </dev/null &
#
# Resumable: a non-empty result file is skipped.  Arms strictly sequential —
# this live-solves, so one run saturates the box.
set -eu
R=${1:?usage: sd-pd-readjudicate.sh RESULTS_DIR}
BIN=${BIN:?set BIN to the ab-fuzzy-strength release binary}
SEED=${SEED:-1783925001}
COUNT=${COUNT:-300000}
WORLDS=${WORLDS:-16}
mkdir -p "$R"

for vul in none both; do
    out="$R/fuzzy-points.$vul.txt"
    if [ -s "$out" ]; then
        echo "skip $vul (already done)"
        continue
    fi
    echo "=== fuzzy-points vul=$vul count=$COUNT seed=$SEED worlds=$WORLDS $(date -Is)"
    "$BIN" --count "$COUNT" --seed "$SEED" --vulnerability "$vul" --policy points \
        --sd --sd-worlds "$WORLDS" --sd-seed "$SEED" >"$out.tmp"
    mv "$out.tmp" "$out"
    cat "$out"
done

echo "=== sd-pd re-adjudication (fuzzy_points) done $(date -Is)"
echo "Compare plain/PD/sd-lead-plain against the published figures in the header"
echo "before reading the sd-lead perfect-defense line."
