#!/bin/sh
# ab-rkcb-doubled.sh — the RKCB asker tables under their double of the answer
# (docs/next-steps.md item 2): `slam::rkcb_rows` registered `{answer} -` only,
# so `4NT - 5♣ (X)` dropped the asker to the floor in every lane.  Now every
# key below an answer is registered under `(X)` too (`slam::HEARD`).  No knob,
# as in ab-rkcb-asker.sh: the OFF arm is a pre-change build — $R/bba-gen-off
# if present, else `BASE` (default: HEAD) built in a worktree under $R.
#
#   SKIP_BUILD=1 BOARDS=1024000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-rkcb-doubled.sh ab-results/rkcb-doubled \
#       >ab-results/rkcb-doubled.log 2>&1 < /dev/null &
#
#   on   the working tree
#   off  the pre-change build
#
# PARKED 2026-10-03 on `park/rkcb-doubled-answer` after two seeds (SEED_BASE
# 1791012778 in ab-results/rkcb-doubled, 1791015397 in ab-results/rkcb-doubled-2;
# 1,024,000 boards/arm/vul each, off = the build of 61857a27).  Pooled: plain
# DD -94 +-142 IMPs (none, 52 fired) / -120 +-180 (both, 58 fired), PD -121
# +-144 / -154 +-183 -- a wash by CI leaning negative in every cell, -1.8...-2.7
# IMPs/fired.  Not a wiring hole: the on arm bids the undoubled ladder, whose
# six-over-five made on 12 boards and failed on 17 once they had doubled.
# Flip plan in the park commit: make the asker read the double.
#
# Hypothesis (pre-run): a wash by CI — the trace priced the hole at about one
# deal in 600k against BBA, so expect a handful of divergent boards per 1M,
# each large (a slam or grand dropped at five).  Read the per-fired line;
# ships on the naturalness tiebreak (measurement.md) unless it loses.
R=${1:?usage: ab-rkcb-doubled.sh RESULTS_DIR}
BASE=${BASE:-HEAD}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for rkcb-doubled)

OFF_BIN="$R/bba-gen-off"
if [ ! -x "$OFF_BIN" ]; then
    base_sha=$(git rev-parse --short "$BASE")
    wt="$R/base-$base_sha"
    [ -d "$wt" ] || git worktree add --detach "$wt" "$BASE"
    log "build off arm from $base_sha in $wt"
    (cd "$wt" && cargo build --release --features serde --example bba-gen) >>"$R/log" 2>&1
    cp "$wt/target/release/examples/bba-gen" "$OFF_BIN"
fi

log "=== rkcb-doubled A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    BBA_GEN=$(realpath "$OFF_BIN") arm off "$vul"
    arm on  "$vul"
    diffpair on off "$vul"
done
log "=== rkcb-doubled A/B done"
