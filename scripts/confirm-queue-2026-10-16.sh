#!/bin/sh
# confirm-queue-2026-10-16.sh — the research-method review's § 6 four-cell
# confirmations plus the N3-fit fresh seed, back to back.  Each runner's
# header holds its pre-registration; this file only orders them and pins
# the environment.  Cheapest first, so verdicts arrive early.
#
#   setsid nohup scripts/idle-run.sh scripts/confirm-queue-2026-10-16.sh \
#       >ab-results/confirm-queue.log 2>&1 </dev/null &
#
# Shared-box settings (CLAUDE.local.md): JOBS=96, DDSS_THREADS=96, the
# glibc-2.36 BBA wrapper.  Resumable: every runner skips finished arms and
# diffs, and seeds persist in each results dir.  One failing runner does not
# stop the rest.  Do NOT edit src/ while this runs.
set -u
cd "$(dirname "$0")/.."
export JOBS=${JOBS:-96} DDSS_THREADS=${DDSS_THREADS:-96}
export BBA_GEN=${BBA_GEN:-$HOME/.local/opt/glibc-2.36/bba-gen}

# step NAME VULS BOARDS RUNNER [ENV=VAL...] — run one runner into ab-results/NAME.
step() {
    name=$1 vuls=$2 boards=$3 runner=$4
    shift 4
    echo "=== $(date -u +%FT%TZ) $name: VULS=\"$vuls\" BOARDS=$boards $runner $*"
    if env VULS="$vuls" BOARDS="$boards" "$@" "scripts/$runner" "ab-results/$name" \
        >"ab-results/$name.log" 2>&1; then
        echo "=== $(date -u +%FT%TZ) $name done"
    else
        echo "=== $(date -u +%FT%TZ) $name FAILED (exit $?), see ab-results/$name.log"
    fi
}

step 2nt-double-veto-nsew      "ns ew"     409600  ab-2nt-double-veto.sh
step 3nt-pull-veto-nsew        "ns ew"     409600  ab-3nt-pull-veto.sh
step competitive-4cell-nsew    "ns ew"     1228800 ab-competitive-4cell.sh
step landy-opener-nsew         "ns ew"     4608000 ab-landy-opener.sh CONFIRM=1
step landy-doubler-px-nsew     "ns ew"     4608000 ab-landy-doubler-flip.sh CONFIRM=1
step nt-high-overcall-nsew     "ns ew"     2400000 ab-nt-high-overcall.sh ROUND=9
step nt-high-overcall-n3fit    "none both" 2400000 ab-nt-high-overcall.sh ROUND=10
echo "=== $(date -u +%FT%TZ) confirm queue done"
