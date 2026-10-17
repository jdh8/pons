#!/bin/sh
# gate-queue-2026-10-17.sh — the two colour gates built from the 2026-10-16
# four-cell run, each preflighted for byte-identity at its three untouched
# cells and then measured at the one cell it moves.  Rules are pre-registered
# in the runners (`ab-landy-opener.sh` GATE=1, `ab-nt-high-overcall.sh`
# ROUND=11); this file orders them and pins the environment.
#
#   setsid nohup scripts/idle-run.sh scripts/gate-queue-2026-10-17.sh \
#       >ab-results/gate-queue.log 2>&1 </dev/null &
#
# A preflight passes iff every cell's gate report reads `0 divergent`; a
# failed preflight skips its A/B.  Each preflight cell runs as its own
# invocation, since a 0-divergent pair may stop a `set -e` runner.  Resumable:
# seeds persist per results dir.  Do NOT edit src/ while this runs.
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
        >>"ab-results/$name.log" 2>&1; then
        echo "=== $(date -u +%FT%TZ) $name done"
    else
        echo "=== $(date -u +%FT%TZ) $name exited $? (see ab-results/$name.log)"
    fi
}

# identical NAME ON OFF VUL... — every cell's gate report reads 0 divergent.
identical() {
    name=$1 on=$2 off=$3
    shift 3
    for v in "$@"; do
        f="ab-results/$name/gate.$on.vs.$off.$v.txt"
        if ! grep -q ' boards, 0 divergent' "$f" 2>/dev/null; then
            echo "=== PREFLIGHT FAILED $name $v: $(sed -n 2p "$f" 2>/dev/null)"
            return 1
        fi
    done
    echo "=== preflight $name byte-identical at: $*"
}

for v in none ns both; do
    step landy-opener-white-preflight "$v" 96000 ab-landy-opener.sh GATE=1
done
if identical landy-opener-white-preflight white rungs none ns both; then
    step landy-opener-white ew 4608000 ab-landy-opener.sh GATE=1
fi

for v in none ew both; do
    step nt-high-overcall-colour-preflight "$v" 96000 ab-nt-high-overcall.sh ROUND=11
done
if identical nt-high-overcall-colour-preflight colour base none ew both; then
    step nt-high-overcall-colour ns 2400000 ab-nt-high-overcall.sh ROUND=11
fi
echo "=== $(date -u +%FT%TZ) gate queue done"
