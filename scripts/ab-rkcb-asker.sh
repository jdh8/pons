#!/bin/sh
# ab-rkcb-asker.sh — the RKCB asker-table repair (docs/next-steps.md item 2,
# censused 2026-09-27 by `probe-rkcb-answerer`): `asker_after_5d` bids six on
# one keycard opposite partner's three (was: exactly two), `asker_after_5h`
# bids six on two keycards plus the trump queen (was: three), the diamond
# mirror alike.  No knob: `slam::rkcb_rows` has twenty callers and no knob
# path, so the OFF arm is a pre-fix build — `BASE` (default: HEAD) checked out
# in a worktree under $R and passed to bba-gen-parallel.sh as BBA_GEN.
#
#   SKIP_BUILD=1 setsid nohup scripts/idle-run.sh \
#       scripts/ab-rkcb-asker.sh ab-results/rkcb-asker \
#       >ab-results/rkcb-asker.log 2>&1 < /dev/null &
#
#   on   the working tree (the fix)
#   off  the build of $BASE
#
# SHIPPED 2026-09-30 on this script's first run (SEED_BASE 1790766739,
# 204,800 boards/arm/vul, base 6f70fbb1): plain DD +0.0013 (none) / +0.0019
# (both), PD +0.0012 / +0.0018 IMPs/board, CI ±0.0009–0.0011, 73 / 79 fired
# (+3.4…+4.9 per fired).  Worst boards are the honest bet: four keycards
# plus the queen with an outside loser.
#
# Hypothesis (pre-run): a wash by CI (the census priced the fix at +0.0002…+0.0005
# plain / +0.0001…+0.0004 PD per board, ~160 divergent per 1M boards, mostly
# the floor answerer already raising to six); ships default-on on the
# naturalness tiebreak (measurement.md) — six on four keycards plus the queen
# is textbook RKCB.  Read the per-fired line, not the headline.
R=${1:?usage: ab-rkcb-asker.sh RESULTS_DIR}
BASE=${BASE:-HEAD}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for rkcb-asker)

# The off binary: build $BASE once, before any arm, in its own worktree.
base_sha=$(git rev-parse --short "$BASE")
wt="$R/base-$base_sha"
OFF_BIN="$wt/target/release/examples/bba-gen"
if [ ! -x "$OFF_BIN" ]; then
    [ -d "$wt" ] || git worktree add --detach "$wt" "$BASE"
    log "build off arm from $base_sha in $wt"
    (cd "$wt" && cargo build --release --features serde --example bba-gen) >>"$R/log" 2>&1
fi

log "=== rkcb-asker A/B start, sha=$SHA vs base=$base_sha, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    BBA_GEN=$OFF_BIN arm off "$vul"
    arm on  "$vul"
    diffpair on off "$vul"
done
log "=== rkcb-asker A/B done"
