#!/bin/sh
# ab-cue-raise-sign-off.sh — opener's decline of the major cue-raise when the
# cue outranks `3M` (docs/next-steps.md, the `1♥ (2♠) 3♠ -` lane from anchor
# 46d0dc14: 58 rows, −700 plain / −908 PD, −12 IMPs a row).  The decline rung
# of `answer_cue_raise` was a bare `3M`, illegal over a `3♠` cue, so a minimum
# opener fell through to the floor's pass and we played `3♠` in their suit.
# Fix: the decline is the cheapest legal bid of the major, legality-anchored
# with `min_level_is` like the minor twin — `4♥` here.  No knob (a correctness
# fix to an existing table), so the OFF arm is a pre-fix build — `BASE`
# (default: HEAD) checked out in a worktree under $R and passed to
# bba-gen-parallel.sh as BBA_GEN.
#
#   SKIP_BUILD=1 setsid nohup scripts/idle-run.sh \
#       scripts/ab-cue-raise-sign-off.sh ab-results/cue-raise-sign-off \
#       >ab-results/cue-raise-sign-off.log 2>&1 < /dev/null &
#
#   on   the working tree (the fix)
#   off  the build of $BASE
#
# Hypothesis (pre-run): about +0.002 per board on both scorers at full capture
# (58 rows per 409,600 boards, −12 each); at that size it resolves on one
# seed.  Read the per-fired line: every fired board should be a `3♠` cue over
# `1♥` where the off arm passed.
#
# SHIPPED 2026-10-03 on the second run (SEED_BASE 1791029195, 204,800
# boards/arm/vul, base 76eae12a, `ab-results/cue-raise-sign-off-2`), IMPs/board
# none / both: plain +0.0011 ±0.0005 / +0.0015 ±0.0007, PD +0.0018 ±0.0007 /
# +0.0020 ±0.0009; 33 / 34 fired, every one a `3♠` cue over `1♥`.
#
# The first run (SEED_BASE 1791025858, `ab-results/cue-raise-sign-off`) tested
# a `min_level_is`-anchored `4M` rung: it won by the same margin in this lane
# (40 / 47 fired, +293…+510 IMPs) but fired 18 / 14 boards *outside* it at
# −33…−9 IMPs — `min_level_is` projects as unconstrained, so the extra rung
# widened opener's `4M` reading to 11+ in every cue-raise node and responder's
# slam tries over it vanished.  The shipped build chooses the rung per node.
R=${1:?usage: ab-cue-raise-sign-off.sh RESULTS_DIR}
BASE=${BASE:-HEAD}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for cue-raise-sign-off)

# The off binary: build $BASE once, before any arm, in its own worktree.
base_sha=$(git rev-parse --short "$BASE")
wt="$R/base-$base_sha"
OFF_BIN="$wt/target/release/examples/bba-gen"
if [ ! -x "$OFF_BIN" ]; then
    [ -d "$wt" ] || git worktree add --detach "$wt" "$BASE"
    log "build off arm from $base_sha in $wt"
    (cd "$wt" && cargo build --release --features serde --example bba-gen) >>"$R/log" 2>&1
fi

log "=== cue-raise-sign-off A/B start, sha=$SHA vs base=$base_sha, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    BBA_GEN=$OFF_BIN arm off "$vul"
    arm on  "$vul"
    diffpair on off "$vul"
done
log "=== cue-raise-sign-off A/B done"
