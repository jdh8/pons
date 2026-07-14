#!/bin/sh
# weak-two-ab.sh — fix-vs-shipped A/B for the weak-two remnant (Root A):
# gauge the weak-two opening (+ Ogust min/max) in raw HCP `hcp(5..=10)` instead
# of the shipped rule-of-N+8 `points(5..=10)`.  Both arms stay on the floored
# scale, so this isolates the gate (docs/point-count-threshold-campaign.md,
# docs/measurement.md).  A weak two is a preempt (competitive range) → sd-lead
# is the honest arbiter; plain DD is the obstruction wall.
#
# Deals come from the pre-solved .pdd bank by --offset (the slice ledger cursor
# at 8,100,000); plain+PD score its stored tables with no live solve, only --sd
# solves.  Arms strictly sequential (one run saturates the box); do NOT rebuild
# the binary while it runs.
#
#   cargo build --release --example ab-point-count
#   setsid nohup scripts/idle-run.sh scripts/weak-two-ab.sh \
#       ab-results/weak-two >ab-results/weak-two.log 2>&1 < /dev/null &
#
# Resumable: a non-empty result file is skipped; the sd world seed persists.
set -eu
R=${1:?usage: weak-two-ab.sh RESULTS_DIR}
DEALS=${DEALS:-/nfs2/jdh8/24.pdd}
BAND=${BAND:-5:10}
BIN=target/release/examples/ab-point-count
mkdir -p "$R"

# Fresh sd world-sampling seed, persisted so a restart stays reproducible.
sd_seed() {
    f="$R/sd-seed"
    [ -s "$f" ] || date +%s >"$f"
    cat "$f"
}
SDSEED=$(sd_seed)

run() { # run NAME VUL OFFSET COUNT ARGS...
    name=$1 vul=$2 offset=$3 count=$4
    shift 4
    out="$R/$name-$vul.txt"
    if [ -s "$out" ]; then
        echo "skip $name-$vul (already done)"
        return
    fi
    echo "=== $name vul=$vul offset=$offset count=$count band=$BAND $(date -Is)"
    "$BIN" --weak-two-hcp "$BAND" --deals "$DEALS" --offset "$offset" \
        --count "$count" --vulnerability "$vul" "$@" >"$out.tmp"
    mv "$out.tmp" "$out"
    cat "$out"
}

# Slice ledger cursor (never replay): pass OFF=<row> to start a fresh run.
OFF=${OFF:-8100000}
# Plain + PD, 1M boards/vul (stored tables, no live solve), with bucket forensics.
run plain none "$OFF"               1000000 --show 40
run plain both "$((OFF + 1000000))" 1000000 --show 40
# sd-lead tiebreak, 50k boards/vul (solves divergent boards live).
run sd    none "$((OFF + 2000000))" 50000   --sd --sd-seed "$SDSEED"
run sd    both "$((OFF + 2050000))" 50000   --sd --sd-seed "$SDSEED"

echo "=== weak-two A/B done $(date -Is); cursor now $((OFF + 2100000))"
