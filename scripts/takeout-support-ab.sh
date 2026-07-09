#!/bin/sh
# takeout-support-ab.sh — A/B for the anchor's #1 bucket (Defensive/book/round-1).
# Two grounded fixes to our defense over a one-suit / weak-two opening:
#   --ns-takeout-support lenient|strict  gates the 12+ takeout double on genuine
#       support for the unbid suits, so an off-shape one-suiter overcalls (or
#       waits for the 17+ any-shape tier) instead of doubling and pulling to the
#       3-level (BBA's two-regime X: 12+ with 3-suit support, else 17+).
#   --ns-overcall-discipline             raises the natural suit-overcall bands to
#       1-level 8-17 / 2-level 11-17 (opening values before a below-their-suit
#       2-level overcall) from the flat 8-16.
#
# One SEED_BASE for the whole experiment (paired diffs need identical deals),
# both vuls, both scorers, arms strictly sequential. See docs/measurement.md and
# the 21gf-ledger. Launch on the shared box as:
#
#   setsid nohup scripts/idle-run.sh scripts/takeout-support-ab.sh \
#       ab-results/takeout-support >ab-results/takeout-support.log 2>&1 &
#
# Resumable: an existing arm dir is skipped; SEED_BASE persists in $R/seed. Do
# NOT touch the codebase while this runs (bba-gen-parallel re-invokes cargo
# build; it must stay a no-op).
R=${1:?usage: takeout-support-ab.sh RESULTS_DIR}
SHOW=3
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "takeout-support A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    # Explicit knobs on every arm — bba-gen's defaults now ship strict+discipline,
    # so `base` (the historical book) must opt out of both.
    arm base    "$vul" --ns-takeout-support off    --ns-overcall-discipline off
    arm lenient "$vul" --ns-takeout-support lenient --ns-overcall-discipline off
    arm strict  "$vul" --ns-takeout-support strict  --ns-overcall-discipline off
    arm disc    "$vul" --ns-takeout-support off     --ns-overcall-discipline on
    arm combo   "$vul" --ns-takeout-support lenient --ns-overcall-discipline on
    arm strictdisc "$vul" --ns-takeout-support strict --ns-overcall-discipline on
    diffpair lenient base    "$vul"   # takeout support gate (lenient) alone
    diffpair strict  base    "$vul"   # takeout support gate (strict) alone
    diffpair disc    base    "$vul"   # overcall discipline alone
    diffpair combo   base    "$vul"   # the full proposal (lenient + discipline)
    diffpair combo   lenient "$vul"   # marginal of overcall discipline atop lenient
    diffpair strictdisc base   "$vul" # strict support + discipline (best-default candidate)
    diffpair strictdisc strict "$vul" # marginal of discipline atop strict
done

log "takeout-support A/B done"
