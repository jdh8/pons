#!/bin/sh
# remnant-fixes-ab.sh — fix-vs-shipped A/Bs for the point-count remnant
# families (docs/point-count-threshold-campaign.md).  Five build-time gate
# fixes, each measured through ab-point-count's two-book --fix path with both
# arms on the shipped floored scale, so each A/B isolates its gate:
#
#   strong-double-hcp:18   the overcall / double-first partition edge in HCP
#   two-suiter-hcp:8       Michaels + Unusual 2NT raw-HCP floor
#   redouble-answer        opener's pass-only answer over 1x-(X)-XX-(P)
#   nt-invite-hcp          1♥-1♠-2m 2NT invite gauged in HCP
#   opening-hcp-floor:10   sub-10-HCP freaks barred from 1-level openings
#
# Plain+PD (1M boards/vul each) from the pre-solved .pdd bank first — all five
# land in ~20 minutes; then the sd-lead legs (50k/vul, live solving) for the
# two competitive-range fixes where sd is the arbiter (convention-tuning.md).
# Arms strictly sequential (one run saturates the box); do NOT rebuild the
# binary while this runs.
#
#   cargo build --release --example ab-point-count
#   setsid nohup scripts/idle-run.sh scripts/remnant-fixes-ab.sh \
#       ab-results/remnant-fixes >ab-results/remnant-fixes.log 2>&1 < /dev/null &
#
# Resumable: a non-empty result file is skipped; the sd world seed persists.
set -eu
R=${1:?usage: remnant-fixes-ab.sh RESULTS_DIR}
DEALS=${DEALS:-/nfs2/jdh8/24.pdd}
BIN=target/release/examples/ab-point-count
mkdir -p "$R"

# Fresh sd world-sampling seed, persisted so a restart stays reproducible.
sd_seed() {
    f="$R/sd-seed"
    [ -s "$f" ] || date +%s >"$f"
    cat "$f"
}
SDSEED=$(sd_seed)

run() { # run NAME FIX VUL OFFSET COUNT ARGS...
    name=$1 fix=$2 vul=$3 offset=$4 count=$5
    shift 5
    out="$R/$name-$vul.txt"
    if [ -s "$out" ]; then
        echo "skip $name-$vul (already done)"
        return
    fi
    echo "=== $name ($fix) vul=$vul offset=$offset count=$count $(date -Is)"
    "$BIN" --fix "$fix" --deals "$DEALS" --offset "$offset" \
        --count "$count" --vulnerability "$vul" "$@" >"$out.tmp"
    mv "$out.tmp" "$out"
    cat "$out"
}

# Slice ledger cursor (never replay): pass OFF=<row> to start a fresh run.
OFF=${OFF:-12300000}

# Plain + PD, 1M boards/vul per fix (stored tables, no live solve), with
# bucket forensics.
run strong-double strong-double-hcp:18 none "$OFF"                    1000000 --show 40
run strong-double strong-double-hcp:18 both "$((OFF + 1000000))"      1000000 --show 40
run two-suiter    two-suiter-hcp:8     none "$((OFF + 2000000))"      1000000 --show 40
run two-suiter    two-suiter-hcp:8     both "$((OFF + 3000000))"      1000000 --show 40
run redouble      redouble-answer      none "$((OFF + 4000000))"      1000000 --show 40
run redouble      redouble-answer      both "$((OFF + 5000000))"      1000000 --show 40
run nt-invite     nt-invite-hcp        none "$((OFF + 6000000))"      1000000 --show 40
run nt-invite     nt-invite-hcp        both "$((OFF + 7000000))"      1000000 --show 40
run opening-floor opening-hcp-floor:10 none "$((OFF + 8000000))"      1000000 --show 40
run opening-floor opening-hcp-floor:10 both "$((OFF + 9000000))"      1000000 --show 40

# sd-lead tiebreak for the competitive-range fixes (solves live, slow).
run sd-strong-double strong-double-hcp:18 none "$((OFF + 10000000))" 50000 --sd --sd-seed "$SDSEED"
run sd-strong-double strong-double-hcp:18 both "$((OFF + 10050000))" 50000 --sd --sd-seed "$SDSEED"
run sd-two-suiter    two-suiter-hcp:8     none "$((OFF + 10100000))" 50000 --sd --sd-seed "$SDSEED"
run sd-two-suiter    two-suiter-hcp:8     both "$((OFF + 10150000))" 50000 --sd --sd-seed "$SDSEED"

echo "=== remnant-fixes A/B done $(date -Is); cursor now $((OFF + 10200000))"
