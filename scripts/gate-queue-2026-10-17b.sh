#!/bin/sh
# gate-queue-2026-10-17b.sh — the colour gates rebuilt on `except_at`.
#
# The first queue (gate-queue-2026-10-17.sh) shipped the Landy rungs gate and
# failed the N3 leave-in gate's preflight: a vulnerability atom inside `&`
# complements to ⊤, so the calls priced below a gated rule lost their negative
# inference at every colour.  Both gates now use `except_at`.  This queue
# re-preflights the Landy gate (now default on: `--no-` arm vs default) at its
# three untouched cells, then preflights the N3 gate and, if it passes, runs its
# `ns` A/B (rule pre-registered in `ab-nt-high-overcall.sh` ROUND=11).
#
#   setsid nohup scripts/idle-run.sh scripts/gate-queue-2026-10-17b.sh \
#       >ab-results/gate-queue-b.log 2>&1 </dev/null &
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
    step landy-opener-white-preflight2 "$v" 96000 ab-landy-opener.sh GATE=1
done
identical landy-opener-white-preflight2 white rungs none ns both

for v in none ew both; do
    step nt-high-overcall-colour-preflight2 "$v" 96000 ab-nt-high-overcall.sh ROUND=11
done
if identical nt-high-overcall-colour-preflight2 colour base none ew both; then
    step nt-high-overcall-colour ns 2400000 ab-nt-high-overcall.sh ROUND=11
fi
echo "=== $(date -u +%FT%TZ) gate queue b done"
