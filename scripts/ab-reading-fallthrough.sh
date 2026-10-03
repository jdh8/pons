#!/bin/sh
# ab-reading-fallthrough.sh — the reading twin of the rejection fall-through:
# a rule table with no row for the call made did not make it, so the reader
# skips it to the next candidate, as the bidder did (`trie::may_have_made`).
# No knob: the off arm is the build of `BASE` (default a3a68aed, the commit
# before the fix), passed to bba-gen-parallel.sh as BBA_GEN.
#
#   BOARDS=819200 setsid nohup scripts/idle-run.sh \
#       scripts/ab-reading-fallthrough.sh ab-results/reading-fallthrough \
#       >ab-results/reading-fallthrough.log 2>&1 < /dev/null &
#
#   on   the working tree (the fix)
#   off  the build of $BASE
#
# What is being tested.  Since 7abcf96f a guarded table that rejects a hand
# falls through to the next candidate when *bidding*, but the reader still
# decoded the call off that table.  Seen at `doubled_notrump_pass`: opener's
# jump shift `1♥ (X) 1NT - 3♠` read 9–21 HCP (was 16–21), and partner drove
# to a failing 6♥.  Every partial table and every Pass a table has no row
# for move the same way.
#
# No isolation gate: the reader change is not a package, and it moves their
# openings too (a 204,800-board pilot, seed 1791065559, diverged on 2 boards,
# one of them theirs).
#
# Hypothesis (pre-run): few boards (0.01–0.05%), positive per fired; a wash
# by CI ships on correctness.
R=${1:?usage: ab-reading-fallthrough.sh RESULTS_DIR}
BASE=${BASE:-a3a68aed}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for reading-fallthrough)

base_sha=$(git rev-parse --short "$BASE")
wt="$R/base-$base_sha"
OFF_BIN="$wt/target/release/examples/bba-gen"
if [ ! -x "$OFF_BIN" ]; then
    [ -d "$wt" ] || git worktree add --detach "$wt" "$BASE"
    log "build off arm from $base_sha in $wt"
    (cd "$wt" && cargo build --release --features serde --example bba-gen) >>"$R/log" 2>&1
fi

log "=== reading-fallthrough A/B start, sha=$SHA vs base=$base_sha, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    BBA_GEN=$OFF_BIN arm off "$vul"
    arm on "$vul"
    diffpair on off "$vul"
done
log "=== reading-fallthrough A/B done"
