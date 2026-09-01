#!/bin/sh
# ab-2d-double.sh — the `(2♦)` diamond penalty double, swept one axis at a time
# (docs/one-notrump-competitive.md §N4b).
#
#   JOBS=24 BOARDS=460800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-2d-double.sh ab-results/2d-double \
#       >ab-results/2d-double.log 2>&1 < /dev/null &
#
# What is being tested.  Responder's double of a `(2♦)` overcall of our 1NT is
# `len(♦, 2..=3) & hcp(8..)` today — `DoubleStyle::Optional`, a *cooperative*
# double asking opener to decide.  Against the reference opponent that gate names
# a suit nobody holds: BBA's `2♦` over our 1NT is a Multi, a single-suited 6-card
# major of about 12-15 (docs/ai-bidder/bba-multi-2d.md).  Meanwhile the shipped
# `(2♦)` structure spends `3♦` on a Jacoby transfer to hearts, so responder has no
# way at all to bid diamonds below `3NT`.  `--ns-2d-double LEN:SUITHCP:HCP` makes
# the double a real diamond penalty double and opener sits on it.
#
# A **replacement**, not an addition: every 2-3-diamond eight-count that doubles
# today stops doubling, and its outs are all shut (`2♥`/`2♠` want a five-card
# major, the `2NT` relay wants a long suit, `3♦` is the transfer), so it passes.
# Read every loss against that orphaning before blaming the idea — the
# orphaned-points trap, docs/convention-tuning.md.
#
# Arms, centred on `5:0:9` with one axis moving at a time:
#
#   base    the shipped cooperative double
#   len4/5/6   4+ / 5+ / 6+ diamonds, no quality gate, 9+ hcp
#   hcp8/hcp11 the strength floor either side of the centre
#   qual4/qual6  4+ / 6+ high-card points *in* the diamond suit
#
# `--filter-1nt` is the enriched-probing gate (balanced 15-17 somewhere, a raw
# hand test applied BEFORE any bidding) and rides EVERY arm, so the arms deal the
# same board set and stay seed-aligned for the paired diff.  Headline is IMPs per
# *accepted* deal; the `(2♦)` bucket is thin (794 boards in a 409.6k-board anchor
# census before enrichment), so read the per-fired figure, not the per-board wash.
#
# Scoring.  This is a doubling / defensive-contract-choice knob, so **plain DD is
# the arbiter** — perfect defense cannot price "double more" (docs/measurement.md)
# and DD is not blind here the way it is to obstruction.  `sddiff` is a
# tie-breaker only.  The `(2♦)` census bucket loses on plain (−0.41/bd) and is
# wash-to-positive on PD, so plain is both the target and the judge.
#
# Resumable: an existing arm dir or diff file is skipped, and SEED_BASE persists
# in $R/2d-double.seed.  Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-2d-double.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for 2d-double)
log "=== 2d-double SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base  "$v" --filter-1nt
    arm len4  "$v" --ns-2d-double 4:0:9  --filter-1nt
    arm len5  "$v" --ns-2d-double 5:0:9  --filter-1nt
    arm len6  "$v" --ns-2d-double 6:0:9  --filter-1nt
    arm hcp8  "$v" --ns-2d-double 5:0:8  --filter-1nt
    arm hcp11 "$v" --ns-2d-double 5:0:11 --filter-1nt
    arm qual4 "$v" --ns-2d-double 5:4:9  --filter-1nt
    arm qual6 "$v" --ns-2d-double 5:6:9  --filter-1nt
    for a in len4 len5 len6 hcp8 hcp11 qual4 qual6; do
        diffpair "$a" base "$v"
        sddiff "$a" base "$v"
    done
done

log "2d-double done"
