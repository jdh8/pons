#!/bin/sh
# ab-illegal-rules.sh — the book drops rules whose call is illegal at their
# node (16ed7221), vs BBA, four cells, plain DD + perfect defense from one
# solve.  No knob (a straight fix), so the arms are two binaries, both built
# in worktrees under $R and copied to $R/bin before any arm:
#
#   off  ac22eaca (slam sign-offs named as `Pass`; byte-identical to 2ff0e36c)
#   on   16ed7221 (the filter)
#
#   BOARDS=819200 setsid nohup scripts/idle-run.sh \
#       scripts/ab-illegal-rules.sh ab-results/illegal-rules \
#       >ab-results/illegal-rules.log 2>&1 < /dev/null &
#
# Why.  A finite rule on a call illegal at its node shadowed the floor and
# resolved to Pass: `1♠ - 2♥ - 3♥ -` passed a game force on 9 boards per
# 3.28M (six-plus hearts).  The filter changes 46 nodes of the default book:
# 15 game_force `1x - 2y - 3z -` (the floor bids instead of passing) and 31
# cue_raise nodes over a cue at 4M or above (unreachable in self-play).
# Self-play smoke-default 400k seed 1: 72 moved, all game_force.
#
# Bar: KR1 non-inferiority.  Plain DD is the verdict (every cell's 95% lower
# bound above -0.002 IMPs/board), PD the guard.  No win expected.  Read the
# isolation gate first: every divergent auction ours, starting `1x - 2y - 3z -`
# with responder passing in the off arm; count our side only (table A seats
# BBA East-West, and its auctions match the same prefixes).
#
# Resumable; SEED_BASE persists in $R/illegal-rules.seed.  Iron rule: do NOT
# rebuild binaries while this runs.
R=${1:?usage: ab-illegal-rules.sh RESULTS_DIR}
OFF_SHA=${OFF_SHA:-ac22eaca}
ON_SHA=${ON_SHA:-16ed7221}
SHOW=50
DIFF_JSONL=1
BUILD_EXTRA='--example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for illegal-rules)

# build_at SHA NAME — bba-gen at SHA in its own worktree, copied to $R/bin/NAME
build_at() {
    out="$R/bin/$2"
    [ -x "$out" ] && return 0
    wt="$R/wt-$1"
    [ -d "$wt" ] || git worktree add --detach "$wt" "$1" >>"$R/log" 2>&1
    log "build $2 arm from $1 in $wt"
    (cd "$wt" && cargo build --release --features serde --example bba-gen) >>"$R/log" 2>&1
    mkdir -p "$R/bin"
    cp "$wt/target/release/examples/bba-gen" "$out"
}
build_at "$OFF_SHA" off
build_at "$ON_SHA" on
OFF_BIN=$(realpath "$R/bin/off")
ON_BIN=$(realpath "$R/bin/on")

log "=== illegal-rules A/B start, on=$ON_SHA vs off=$OFF_SHA (runner sha=$SHA), SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    # Subshells: bash-as-sh keeps a prefix assignment past a function call.
    (BBA_GEN=$OFF_BIN arm off "$vul")
    (BBA_GEN=$ON_BIN arm on "$vul")
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== illegal-rules A/B done"
#
# VERDICT (2026-11-18, SEED_BASE 1794984344382769611, 16x51,200 bd/cell): WIN.
# Plain DD wins in all 4 cells: none +179 (+0.0002 ±0.0001), ns +316 (+0.0004 ±0.0002),
# ew +166 (+0.0002 ±0.0001), both +306 (+0.0004 ±0.0002) on 60/64/58/63 fired.
# PD: none +134, ns +270 and both +260 win; ew +121 (±0.0002) is non-inferior.
# Isolation gate clean: every divergent board is ours, is a game_force
# `1x - 2y - 3z -` pass in the off arm, and reaches game in the on arm. No cue_raise
# boards. Biggest buckets: 1♠ - 2♣ - 3♠ (10, +57..+81) and 1♠ - 2♦ - 3♠ (9-10, +42..+69).
