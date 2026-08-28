#!/bin/sh
# ab-2d-multi-doubler-nt.sh — opener's **notrump out** over the Kokish–Kraft
# doubler's natural other major (docs/multi-doubler-answer-handoff.md item 1).
#
#   SKIP_BUILD=1 JOBS=24 PER_SHARD=192000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-2d-multi-doubler-nt.sh ab-results/2d-multi-doubler-nt \
#       >ab-results/2d-multi-doubler-nt.log 2>&1 < /dev/null &
#
# What is being tested.  `competition.multi_doubler_major` shipped default-on
# 2026-08-26 with a negative both-vulnerable perfect-defense cell (−1.737
# IMPs/fired on 510 boards, `ab-results/2d-multi-doubler`) — the decision
# table's `win | loss` row, shipped on jdh8's ruling with this follow-up owed.
# The trace says the rung is not at fault: four of the five worst boards are
# **not failing games**, they are `2♠` played in a 4-2 or 4-3 because
# `kokish_kraft_doubler_major_answer` has no notrump rung and no escape, so a
# 15-17 balanced maximum with a stopper in *their* major and two or three cards
# in *ours* must pass.  Vulnerable that is −200 where `3NT` was cold.
#
#   3NT  @135  `hcp(16..) & stopper_in(major)` — below the `4M`@140 game in a
#              known 4-4, above the `3♠`@130 invitational raise, so it fires on
#              exactly the hands that pass today.  No length cap: four of the
#              other major with `hcp 16+` already took the game at 140.
#
# The same rung `competition.multi_px_split` carries, unbundled from the split
# so it can be priced against the *shipped* default rather than against the
# split's re-weighted (148) natural major.
#
# The hypothesis is specific and falsifiable: **the both-vul PD cell is the
# 4-2/4-3 pass-outs, so authoring the notrump out moves that cell to
# non-negative without touching the no-vul win.**  A no-vul regression falsifies
# it; so does a both-vul PD cell that stays negative, and the next suspect is
# then the `3♥` leg's `hcp 16+` game answer, which has no invitational rung
# under it and so bids game on 24 combined at the four level.
#
# Two arms, one seed set, `--their-2d-multi --filter-1nt` on both:
#
#   base   the natural other major, no notrump out (the pre-2026-08-27 default)
#   nt     plus `3NT`@135 — the shipped default since 2026-08-27
#
# The rung SHIPPED default-on 2026-08-27 on this script's own run: a win in all
# four cells (none +2.910 plain / +2.096 PD over 167 fired; both +4.264 /
# +3.264 over 106), 0 foreign on both gates.  The arms are therefore inverted
# relative to that run — `base` now carries the disarming flag — so a re-run
# measures the same delta in the same direction.
#
# SIZE THIS ARM BIG, at the doubler run's own scale (2.304M bd/arm/vul).  This
# seat is *inside* that rung's already-thin 0.03% divergence surface — the
# pass-outs of a table reached on ~1300 boards per 2.3M — so a smaller arm
# cannot resolve it.  Read the per-fired paired diff, not the per-board
# headline; the doubler run's resolution constant is 5.39 IMPs sd per divergent
# board, i.e. |Δ| > 10.56/√n_div per fired board to clear 2 SE.
#
# Scoring.  Plain AND perfect defense off the decision table in
# docs/measurement.md; `sddiff` is the tie-breaker.  Read `probe-divergence
# --gate-opener ours` BEFORE any headline — it must read **0 foreign**, as it
# did at both vulnerabilities on the doubler run (0/787, 0/510).
#
# Before declaring a loss dead, trace the worst divergent boards.  The named
# suspect is the 15-count: the rung floors at `hcp(16..)`, so a 15 with a
# stopper and short support still passes `2♠` (handoff item 2, deliberately not
# built — it rides this arm's forensic rather than pre-empting it).
#
# Resumable: an existing arm dir, gate, or diff file is skipped, and SEED_BASE
# persists in $R/2d-multi-doubler-nt.seed.  Iron rule: do NOT edit `src/` while
# this runs — `bba-gen-parallel.sh` rebuilds at the head of every arm unless
# SKIP_BUILD=1 is exported, and `ab-lib.sh` has already built once by then.
R=${1:?usage: ab-2d-multi-doubler-nt.sh RESULTS_DIR}
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

SEED_BASE=$(seed_for 2d-multi-doubler-nt)
log "=== 2d-multi-doubler-nt SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --their-2d-multi --filter-1nt --no-ns-multi-doubler-notrump
    arm nt   "$v" --their-2d-multi --filter-1nt

    gatepair nt base "$v"
    diffpair nt base "$v"
    sddiff   nt base "$v"
done

log "2d-multi-doubler-nt done"
