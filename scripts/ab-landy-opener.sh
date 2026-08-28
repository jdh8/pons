#!/bin/sh
# ab-landy-opener.sh — §N1m, **opener's** own rebid over their Landy advance:
# `1NT (2♣) X (2♥)` and `X (2♠)`.  The seat §N1k authored a `3NT` at, lost, and
# gave back to the floor.
#
#   JOBS=24 PER_SHARD=192000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-opener.sh ab-results/landy-opener \
#       >ab-results/landy-opener.log 2>&1 < /dev/null &
#
# What is being tested, and where the design came from.  `probe-landy-opener-oracle`
# priced every contract opener could steer to from this seat on 103,653
# (non-vulnerable) + 81,023 (vulnerable) boards taken off the §N1l base arms,
# against the contract our live method actually reaches.  IMPs/board, plain DD:
#
#   opener's trumps    2Mx      2NT@op   3NT@op   par     (non-vulnerable)
#   2, hcp 15..17     -3.6..-0.7  +1.9..+2.5  +0.7..+3.7  +2.5..+5.2
#   3, hcp 15..17     -1.6..+2.5  +1.2..+1.5  -0.2..+3.4  +2.8..+5.2
#   4+, hcp 15..17    +2.8..+6.8  -1.0..-0.1  -2.6..+1.9  +1.7..+3.8
#
#   opener's trumps    2Mx      2NT@op   3NT@op   par     (both vulnerable)
#   2, hcp 15..17     -4.5..-0.8  -0.0..+1.0  -1.3..+3.5  +2.0..+5.8
#   3, hcp 15..17     -1.7..+3.3  -1.4..-0.5  -2.2..+3.3  +2.4..+5.8
#   4+, hcp 15..17    +3.8..+8.1  -3.7..-2.8  -4.5..+1.1  +0.6..+3.3
#
# Three readings drive the arms.  (1) `2Mx` wins **every** four-plus-trump
# bucket at both vulnerabilities and its PD column is flat (−1.2…+0.3) — the
# signature of a real penalty double.  (2) On two or three trumps that same
# double is negative except at 17 HCP, where `3NT` matches it, so `len(major,
# 4..)` is the whole gate.  (3) Declaring is a **white** idea except for the
# 16–17 with a stopper, which holds up red too.  Two arms:
#
#   base    today's `main` — the seat is the floor's, and it passes 98.5% /
#           99.5% of the time
#   px      `competition.landy_opener_px`: `X`@150 `len(major, 4..)` +
#           `Pass`@0, plus the doubler's sit at `{path} X -`
#   rungs   plus `competition.landy_opener_rungs`: `3NT`@135 `hcp(16..) &
#           stopper_in` and `2NT`@120 `hcp(15..) & stopper_in & !vulnerable()`,
#           each a sign-off the doubler passes
#
# Falsifiers, in order.
#   1. **The oracle assumes they sit.**  Every candidate is priced as the
#      contract the auction stops in, so `2Mx` is scored with the advancer
#      never running — which is why it beats *par* in the four-trump buckets
#      (par lets them escape).  If `px` reads flat or negative, the first thing
#      to check is the runout: `probe-divergence --jsonl` on the `X` rows,
#      split by their next call.  That tail is deliberately the floor's.
#   2. **The floor already doubles here.**  The *instinct* floor makes a
#      takeout double at this seat on 12+ with at most three cards in each of
#      their suits, so on the anchor's pool part of this is emergent.  The net
#      floor the A/B measures passes 98.5% / 99.5% (`probe-landy-opener-oracle`
#      live-call census), so the pool here is nearly all passes — read the
#      fired count, and do not import an anchor-arm intuition.
#   3. **§N1k, again.**  Its `3NT` on `hcp(16..) & has_stopper` was REFUTED on
#      2026-08-27 because `has_stopper` is length-blind: it fired on the
#      four-trump hands the oracle now prices at −1.1…−4.5 and it shadowed the
#      floor's delayed penalty double.  Arm `rungs` is only meaningful *under*
#      `px`, which is why it is not run alone — if `rungs` beats `px` the
#      §N1k geometry is genuinely repaired; if it loses, the cap was the whole
#      story and `px` ships alone.
#   4. **PD is blind to `px` by construction** (docs/measurement.md's domain
#      addendum): perfect defense doubles the same failing contracts by fiat,
#      so `px` keeps the whole cost of a real penalty double and none of its
#      benefit.  Arbitrate `px` on plain DD with SD-PD as tie-break.
#
# Scoring: plain AND perfect defense off docs/measurement.md's decision table;
# sd-lead tie-breaks.  `probe-divergence --gate-opener ours` must read 0 foreign
# BEFORE any headline.  Resumable; SEED_BASE persists in $R/landy-opener.seed.
# Iron rule: do NOT edit `src/` while this runs.
R=${1:?usage: ab-landy-opener.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-opener)
log "=== landy-opener SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base  "$v" --filter-landy
    arm px    "$v" --filter-landy --ns-landy-opener-px
    arm rungs "$v" --filter-landy --ns-landy-opener-px --ns-landy-opener-rungs

    for a in px rungs; do
        gatepair "$a" base "$v"
        diffpair "$a" base "$v"
        sddiff   "$a" base "$v"
    done
    # Falsifier 3, read directly: does adding the notrump rungs under the
    # double repair §N1k's geometry, or was the ≤3-trump cap the whole story?
    gatepair rungs px "$v"
    diffpair rungs px "$v"
    sddiff   rungs px "$v"
done

log "landy-opener done"
