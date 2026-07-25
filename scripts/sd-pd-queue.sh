#!/bin/sh
# sd-pd-queue.sh — the three remaining SD-PD re-adjudications, back to back.
#
# Cheapest first, so verdicts arrive early: the dump rescores (replay only, no
# bidding), then the Meckstroth `3m` leg, then the runout marginal (heaviest —
# 1M filtered boards per cell, four cells).
#
#   BINDIR=<path>/release/examples setsid nohup scripts/idle-run.sh \
#       scripts/sd-pd-queue.sh ab-results/sd-pd-queue \
#       >ab-results/sd-pd-queue.log 2>&1 </dev/null &
#
# Resumable throughout: a non-empty result file is skipped, seeds persist.
set -eu
R=${1:?usage: sd-pd-queue.sh RESULTS_DIR}
BINDIR=${BINDIR:?set BINDIR to the release examples directory}
mkdir -p "$R"

f="$R/seed"
[ -s "$f" ] || date +%s >"$f"
SEED=$(cat "$f")

# run NAME CMD... — write $R/NAME unless already non-empty.
run() {
    out="$R/$1.txt"
    shift
    if [ -s "$out" ]; then
        echo "skip $out (already done)"
        return 0
    fi
    echo "=== $out :: $* $(date -Is)"
    # ponytail: one failing cell must not take the rest of the queue with it —
    # the runs behind it are hours long and independent.
    if "$@" >"$out.tmp" 2>&1; then
        mv "$out.tmp" "$out"
        cat "$out"
    else
        echo "FAILED (exit $?) — kept $out.tmp"
    fi
}

# 1) Batches 2 and 3 of the dump rescores (batch 1 already has result files, so
#    those rows skip).  Own driver, own results dir — it has the reproduction
#    gate comments per row.
BINDIR="$BINDIR" sh "$(dirname "$0")/sd-pd-dumps.sh" \
    /home/jdh8/src/pons/ab-results/sd-pd-dumps ||
    echo "dumps driver exited nonzero — continuing with the self-play runs"

# 2) The Meckstroth adjunct's invitational `3m` leg, isolated.  The merged knob
#    was CONFIRMED on all four brackets, but its 2NT machine dominates the
#    divergent boards, so the PD-negative `3m` leg still rides on its original
#    plain-SD row (plain wash, PD -0.0036/-0.0019, sd-lead +0.0012/+0.0042) —
#    precisely the shape SD-PD refuted for set_forcing_nt_two_suiter.
#    --minor-jumps-only keeps the GF 2NT in both arms and moves only the jumps.
for vul in none both; do
    run "meckstroth-3m.$vul" \
        "$BINDIR/ab-meckstroth-2nt" --count 400000 -v "$vul" \
        --minor-jumps-only --sd --sd-seed "$SEED"
done

# 3) set_one_nt_runout_universal: "marginal = full runout - direct-only", so it
#    takes two runs per vulnerability whose A/B deltas are subtracted.  Same deal
#    seed in both, so the boards cancel and only the knob moves.  Published:
#    plain +0.009/+0.011, PD -0.004/-0.005, never sd-measured ("sd-lead
#    candidate", bidding-options.md:156) — this is its first SD reading, and the
#    SD block prices both brackets in one pass.
for vul in none both; do
    run "runout-full.$vul" \
        "$BINDIR/ab-one-nt-runout" --compare runout --filter-1nt \
        --count 1000000 -v "$vul" --seed "$SEED" --sd --sd-seed "$SEED"
    run "runout-direct.$vul" \
        "$BINDIR/ab-one-nt-runout" --compare runout --filter-1nt --no-universal \
        --count 1000000 -v "$vul" --seed "$SEED" --sd --sd-seed "$SEED"
done

echo "=== sd-pd queue done $(date -Is)"
