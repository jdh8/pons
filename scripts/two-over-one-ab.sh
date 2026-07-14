#!/bin/sh
# two-over-one-ab.sh — self-play A/Bs for the 2/1 response-band ideas
# (docs/point-count-threshold-campaign.md remnant; docs/measurement.md rules).
#
# Exp 1 (seed file seed-cog):  baseline vs the 1M-3NT choice of games
#                              (`set_major_choice_of_games`).
# Exp 2 (seed file seed-gate): baseline vs each major 2/1 entry-gate arm —
#                              the fit leg (`set_two_over_one_fit`) and the
#                              hcp13/hcp12 no-fit gauges
#                              (`set_two_over_one_gate`).
#
# Arms strictly sequential (one run saturates the box), a fresh SEED_BASE per
# experiment shared across that experiment's runs and vulnerabilities,
# opponents silenced (constructive value only), both scorers reported.
# Do NOT rebuild the tree while it runs.
#
#   cargo build --release --example ab-major-continuations
#   setsid nohup scripts/idle-run.sh scripts/two-over-one-ab.sh \
#       ab-results/two-over-one >ab-results/two-over-one.log 2>&1 &
#
# Resumable: a non-empty result file is skipped; seeds persist in $R/seed-*.
set -eu
R=${1:?usage: two-over-one-ab.sh RESULTS_DIR}
COUNT=${COUNT:-1000000}
BIN=target/release/examples/ab-major-continuations
mkdir -p "$R"

# One seed per experiment, persisted so a restart stays seed-aligned; the
# offset keeps the two experiments' deal sets distinct even when both seeds
# are minted in the same second.
seed_for() {
    f="$R/seed-$1"
    [ -s "$f" ] || echo $(($(date +%s) + $2)) >"$f"
    cat "$f"
}

run() { # run NAME SEED VUL ARGS...
    name=$1 seed=$2 vul=$3
    shift 3
    out="$R/$name-$vul.txt"
    if [ -s "$out" ]; then
        echo "skip $name-$vul (already done)"
        return
    fi
    echo "=== $name vul=$vul seed=$seed count=$COUNT $(date -Is)"
    "$BIN" --count "$COUNT" --vulnerability "$vul" --seed "$seed" "$@" >"$out.tmp"
    mv "$out.tmp" "$out"
    cat "$out"
}

S1=$(seed_for cog 0)
for vul in none both; do
    run cog "$S1" "$vul" --choice-of-games
done

S2=$(seed_for gate 1000003)
for vul in none both; do
    run fit "$S2" "$vul" --two-over-one-fit
    run hcp13 "$S2" "$vul" --two-over-one-gate hcp13
    run hcp12 "$S2" "$vul" --two-over-one-gate hcp12
done

echo "=== two-over-one A/B done $(date -Is)"
