#!/bin/sh
# ab-2d-multi-slam.sh — the `4m` slam try above a completed Kokish–Kraft minor
# transfer (docs/minor-transfer-slam.md, docs/one-notrump-competitive.md §N4-KK
# residues 3 and 6).
#
#   JOBS=24 BOARDS=460800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-2d-multi-slam.sh ab-results/2d-multi-slam \
#       >ab-results/2d-multi-slam.log 2>&1 < /dev/null &
#
# What is being tested.  `competition.multi_minor_slam_try` is a `points` floor,
# not a bool, and the three arms are `off` / `13` / `15`.  What it authors:
#
#   4m         responder's slam try over the completion, `points(N..) &
#              len(minor, 6..)` at w151 — between the lowest two-suiter step
#              (152) and `3NT` (150).  `13` is `landy_bba_transfer_rebid`'s own
#              rung verbatim; `15` is the narrow arm.
#   4NT / 5m   opener's answer: a maximum (`hcp 16+`) asks keycard and
#              `slam::rkcb_rows` supplies the ladder, anything else declines to
#              game in the minor.  The gate is a **constant across both arms**,
#              so the arms differ in responder's floor and nowhere else.
#   4m again   one round later, when they compete over the completion: the
#              shortness placement (`len(major, ..=1) & len(minor, 6..) &
#              points(10..)`, w145) that residue 6 named, ranked between the
#              `3NT` (150) and the penalty `X` (140).  Opener sits on it.
#
# There is no room under `3NT` at this seat, so **every one of these rungs buys
# its information by giving `3NT` up**.  That is the trade the arms price, and
# it is why the floor is a payload: `13` is ~11-13 HCP opposite 15-17, which is
# 26-28 combined and a 3NT hand more often than a slam hand, while `15` only
# bypasses `3NT` at 28-30.  Landy runs `13` and gets away with it because its
# stopper cues at w150/149 skim the one-stopper hands off first; K–K has no cue
# rungs, so the same number fires on a materially wider class.
#
# Why opener's answer is authored, against N1's slam-exploration doctrine ("a
# `4m` suit contract lets the floor cue-bid on to slam").  Measured, it does
# not: at `1NT (2♦) 2NT - 3♣ - 4♣ -` with the rung live, the floor's entire
# vocabulary is `{6NT, 4♥, Pass}` and it takes `4♥` — a contract in the suit
# their Multi showed — on a minimum.  It cannot reach for keycard at all, since
# `instinct`'s `4NT` ask is gated on `Context::undisturbed` and this lane is
# disturbed by construction.  Leaving the seat floored would have measured the
# floor, not the idea.
#
# Three arms, one seed set, `--their-2d-multi` on all three so the table is the
# only difference (Kokish–Kraft is the shipped default and needs no flag):
#
#   base   the shipped K–K table, ladder ending at `3NT`
#   s13    the slam try at `points(13..)` — Landy's floor
#   s15    the slam try at `points(15..)` — the narrow arm
#
# `--filter-1nt` (balanced 15-17 somewhere, a raw-hand test applied BEFORE any
# bidding) rides all three so they deal the same board set and stay seed-aligned
# for the paired diffs.
#
# Scoring.  Plain AND perfect defense, read off the decision table in
# docs/measurement.md; `sddiff` is the tie-breaker.  Read
# `probe-divergence --gate-opener ours` BEFORE any headline — the mirror-read
# leak (fixed at `29f93561`) is what makes a counter knob a reading knob, and
# the gate must read **0 foreign**.  Both new rungs live inside the subtree
# keyed on their `2♦` disclosure, which `System::opponents` clears, so the
# mirror should stay inert; a non-zero reading here means the mirror regressed.
#
# Interpretation caveat.  The rung fires rarely — a completed minor transfer,
# then a hand above the floor — so expect a small per-board number with a large
# per-fired one, and read the per-fired CI, not the point estimate.  The `s15`
# arm fires strictly less often than `s13`; a wash there may be sample size
# rather than indifference.  Trace the worst divergent boards before calling a
# loss: the first suspects are an opener declining to `5m` where `3NT` was cold,
# and the contested placement landing in `4m` on a 6-2.
#
# Follow-up already agreed, whatever the verdict: port the winning shape back to
# `landy_bba_transfer_rebid`, whose own `4m` has **no authored answer** today —
# the same floored seat this arm found, in a lane that is shipped default-on
# (docs/minor-transfer-slam.md).
#
# Resumable: an existing arm dir, gate, or diff file is skipped, and SEED_BASE
# persists in $R/2d-multi-slam.seed.  Iron rule: do NOT rebuild binaries while
# this runs.
R=${1:?usage: ab-2d-multi-slam.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
PROBE=target/release/examples/probe-divergence

gatepair() {
    on=$1; off=$2; vul=$3
    out="$R/gate.$on.vs.$off.$vul.txt"
    [ -s "$out" ] && { log "skip $out (exists)"; return 0; }
    log "isolation gate $on vs $off ($vul)"
    "$PROBE" "$R/$on-$vul" "$R/$off-$vul" --gate-opener ours >"$out"
}

SEED_BASE=$(seed_for 2d-multi-slam)
log "=== 2d-multi-slam SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --their-2d-multi --filter-1nt
    arm s13  "$v" --their-2d-multi --ns-multi-minor-slam-try 13 --filter-1nt
    arm s15  "$v" --their-2d-multi --ns-multi-minor-slam-try 15 --filter-1nt

    for a in s13 s15; do
        gatepair "$a" base "$v"
        diffpair "$a" base "$v"
        sddiff   "$a" base "$v"
    done
    # The two arms against each other: which floor, given both beat base.
    diffpair s15 s13 "$v"
done

log "2d-multi-slam done"
