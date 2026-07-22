#!/bin/sh
# two-over-one-nofit-floor-ab.sh — is the shipped 13-HCP no-fit 2/1 floor too
# strict? Head-to-head: baseline hcp13 (shipped) vs hcp12 / points12 (Rule of
# 20) / points13 (legacy), both arms otherwise stripped (ab-major-continuations'
# own --baseline-gate protocol; docs/archive/point-count-threshold-campaign.md
# is the prior art for this exact comparison).
#
# Arms strictly sequential, one fresh SEED_BASE shared across every arm and
# vulnerability, opponents silenced (constructive value only), both scorers
# reported. Do NOT rebuild the tree while it runs.
#
#   cargo build --release --example ab-major-continuations
#   setsid nohup scripts/idle-run.sh scripts/two-over-one-nofit-floor-ab.sh \
#       ab-results/two-over-one-nofit-floor >ab-results/two-over-one-nofit-floor.log 2>&1 &
#
# Resumable: a non-empty result file is skipped; the seed persists in $R/seed.
set -eu
R=${1:?usage: two-over-one-nofit-floor-ab.sh RESULTS_DIR}
COUNT=${COUNT:-1000000}
BIN=target/release/examples/ab-major-continuations
mkdir -p "$R"

seed_for() {
    f="$R/seed"
    [ -s "$f" ] || echo "$(date +%s)" >"$f"
    cat "$f"
}

run() { # run NAME SEED VUL GATE
    name=$1 seed=$2 vul=$3 gate=$4
    out="$R/$name-$vul.txt"
    if [ -s "$out" ]; then
        echo "skip $name-$vul (already done)"
        return
    fi
    echo "=== $name vul=$vul seed=$seed count=$COUNT $(date -Is)"
    "$BIN" --count "$COUNT" --vulnerability "$vul" --seed "$seed" \
        --baseline-gate hcp13 --two-over-one-gate "$gate" >"$out.tmp"
    mv "$out.tmp" "$out"
    cat "$out"
}

S=$(seed_for)
for vul in none both; do
    run hcp12 "$S" "$vul" hcp12
    run points12 "$S" "$vul" points12
    run points13 "$S" "$vul" points13
done

echo "=== two-over-one no-fit-floor A/B done $(date -Is)"
