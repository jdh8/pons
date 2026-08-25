#!/bin/sh
# ab-landy-minor-slam.sh — opener's answer to the N1j Landy `4m` slam try
# (docs/minor-transfer-slam.md queue item 2).
#
#   JOBS=12 PER_SHARD=19200 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-minor-slam.sh ab-results/landy-minor-slam \
#       >ab-results/landy-minor-slam.log 2>&1 < /dev/null &
#
# What is being tested, and what is NOT.  `landy_bba_transfer_rebid`'s `4m`
# (`points(13..) & len(minor, 6..)`, weight 130) has shipped **default-on since
# N1** and no arm here touches it.  The only difference between the arms is the
# seat *above* it: `1NT (2♣) 2NT - 3♣ - 4♣ -`, which was the floor's before
# this campaign.
#
# Why that seat cannot be left to the floor.  N1 wrote the rule down — "opener's
# continuation is deliberately the floor's, a `4m` suit contract lets the floor
# cue-bid on to slam" — and it is wrong here for a reason N1 could not have
# known.  Two gates, one cause:
#
#   1. `instinct`'s `4NT` keycard ask is gated on `Context::undisturbed`, so the
#      floor can never keycard in *any* lane the opponents have bid in.  This
#      one is disturbed by construction.
#   2. The same ask carries `combined_points(29)` against own + partner's
#      **shown** floor, and an unauthored call shows a floor of zero.
#
# Probed at the seat, arm `base`: opener's whole vocabulary is
# `{6NT 1.600, 4♥ 1.500, Pass 0.000}` and a balanced 16 takes the `4♥` — a
# contract in a major the Landy `2♣` advertised.  Arm `ans`: `4NT 1.600`.
#
# Two arms, one seed set:
#
#   base   `--no-ns-landy-minor-slam-answer`: the former table, no answer
#   ans    the shipped table: opener asks keycard (`4NT`, RKCB via
#          `american::slam::rkcb_rows`) on `hcp(16..)`, else declines to `5m`,
#          at the quiet seat and at their double of the try.  `16` is a
#          constant, not a payload — this arm prices the *answer*.
#
# The responder floor is deliberately NOT an arm.  The sibling treatment one
# lane over (`multi_minor_slam_try`) shipped `Some(15)` after two rounds, but
# the numbers do not transplant by symmetry: Landy's `4m` sits under stopper
# cues at weight 150/149 that the Kokish–Kraft table does not have, so `13`
# fires on a materially narrower class here.  Sweeping it is a separate seed.
#
# `--filter-1nt` (balanced 15-17 somewhere, a raw-hand test applied BEFORE any
# bidding) rides both arms so they deal the same board set and stay
# seed-aligned for the paired diffs.  `--their-2c-landy` is left *derived* on
# purpose: the 2/1 reference defaults to Landy from its measured behavior, and
# forcing it would price a declaration we did not make.
#
# Scoring.  Plain AND perfect defense, read off the decision table in
# docs/measurement.md; `sddiff` is the tie-breaker.  Read
# `probe-divergence --gate-opener ours` BEFORE any headline: every node this
# arm adds lives under `P* 1NT (2♣)`, which only exists when **we** opened, so
# the gate must read **0 foreign**.  A non-zero reading means the mirror book
# regressed, not that the treatment works.
#
# Interpretation caveat.  The rung is rare — a completed minor transfer, then a
# 13+ hand with six of the minor, then opener holding a maximum for the ask to
# fire at all — and it is *skimmed from above* by the stopper cues, so expect a
# smaller fired count than the K–K sibling's.  Report both quantities: the
# filter's acceptance density converts the total into an unconditional
# raw-deal equivalent, while the per-fired delta and one-sample `t` describe
# the treatment when it acts.  Do not mistake the tiny unconditional mean for
# a weak conditional result.
#
# Trace before calling a loss: the first suspects are opener asking on a
# maximum with a *misfit* (the ask does not gate on minor length) and the `5m`
# decline landing above a cold `3NT` that `base` never left.
#
# Resumable: an existing arm dir, gate, or diff file is skipped, and SEED_BASE
# persists in $R/landy-minor-slam.seed.  Iron rule: do NOT rebuild binaries
# while this runs.
R=${1:?usage: ab-landy-minor-slam.sh RESULTS_DIR}
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

SEED_BASE=$(seed_for landy-minor-slam)
log "=== landy-minor-slam SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --no-ns-landy-minor-slam-answer --filter-1nt
    arm ans  "$v" --filter-1nt

    gatepair ans base "$v"
    diffpair ans base "$v"
    sddiff   ans base "$v"
done

log "landy-minor-slam done"
