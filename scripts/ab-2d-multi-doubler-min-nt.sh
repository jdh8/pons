#!/bin/sh
# ab-2d-multi-doubler-min-nt.sh — the **15-count's** notrump out over the
# Kokish–Kraft doubler's natural other major
# (docs/multi-doubler-answer-handoff.md item 2).
#
#   SKIP_BUILD=1 JOBS=12 PER_SHARD=384000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-2d-multi-doubler-min-nt.sh ab-results/2d-multi-doubler-min-nt \
#       >ab-results/2d-multi-doubler-min-nt.log 2>&1 < /dev/null &
#
# What is being tested.  `competition.multi_doubler_notrump` shipped default-on
# 2026-08-27 (a win in all four cells, `ab-results/2d-multi-doubler-nt`), but it
# floors at `hcp(16..)`.  A **15**-count with their major stopped and fewer
# than four of ours therefore still passes `2♠` into a 4-2 or 4-3 — board 1 of
# that run's worst tail, `A82.A53.KJ75.K74`, is exactly that hand, and
# `kk_doubler_notrump_repairs_the_answer_table` pins it as unrepaired.
#
#   2NT  @120  `hcp(15..) & stopper_in(major)` — below the `3♠`@130 invite so
#              four-card support still raises, and below the `3NT`@135 and
#              `4M`@140 the `hcp 16+` hands already take, so ordering confines
#              it to the 15-count with short support.
#   3NT  @140  responder's acceptance on `hcp(10..)` — the 25 opposite a known
#              15, since the double was `hcp 8+`.
#
# ONE LEG ONLY, and that is the point.  `2NT` outranks spades at the same
# level, so the rung exists over the `2♠` leg; over the `3♥` leg there is
# nothing below `3NT` and its 15-count keeps passing.  The two legs are not one
# treatment, which is why this gets its own knob and its own seed rather than
# being a one-point relaxation of `multi_doubler_notrump`'s floor.
#
# The rung SHIPPED default-on 2026-08-27 on this script's own run: a win in all
# four cells (none +2.007 plain / +1.597 PD over 144 fired; both +2.011 /
# +1.413 over 92), 0 foreign on both gates, single-dummy agreeing in the same
# direction everywhere.  The designed-for risk showed up and was paid for: the
# worst boards are `2NT` and accepted `3NT` going down, but 48% of divergences
# reach a game the baseline never bid, and on 8 boards the baseline's pass let
# the opponents balance into a game of *their* own that `2NT` shut out.
#
# The hypothesis it tested: **the same rung one point lower keeps the same
# sign.**  The
# `hcp 16+` version won all four cells by 3–4 SE with 270 of 273 divergences
# reaching a game the baseline never bid; if the 15-count's `2NT` is the same
# population one trick shallower, it should read non-negative everywhere.  What
# would falsify it is real: 15 opposite `hcp 8+` is 23 combined, so this
# invitation can be wrong in *both* directions — accepted into a bad 3NT, or
# declined into a 2NT that plays worse than defending their partscore.  A
# negative cell here is evidence the ladder should stop at 16, not evidence of
# a missing rung.
#
# Two arms, one seed set, `--their-2d-multi --filter-1nt` on both:
#
#   base    the `hcp(16..)` floor: `4M`@140 / `3NT`@135 / `3♠`@130 / pass
#   minnt   plus `2NT`@120 and responder's acceptance — the default since
#           2026-08-27, so `base` now carries the disarming flag and a re-run
#           measures the same delta in the same direction
#
# SIZE IT AT THE NOTRUMP-OUT RUN'S SCALE (4.608M bd/arm/vul, JOBS=12
# PER_SHARD=384000) — the measured surface is 1 divergence in 32 000 at no-vul
# and 1 in 50 000 at both, i.e. 144 and 92 fired at this scale, against the
# notrump out's 167 and 106.  **A 120 000-board smoke read 5 divergences and so
# sized it at 1 in 24 000, twice the truth**; five boards is Poisson noise, so
# treat a smoke that thin as order-of-magnitude only and do not shrink an arm on
# one.  Read the per-fired paired diff, not the per-board headline: the
# resolution constant is 5.39 IMPs sd per divergent board, so |Δ| > 10.56/√n_div
# per fired board clears 2 SE — 0.90 and 1.12 at these counts.
#
# Scoring.  Plain AND perfect defense off the decision table in
# docs/measurement.md; `sddiff` is the tie-breaker.  Read `probe-divergence
# --gate-opener ours` BEFORE any headline — it must read **0 foreign**, as it
# did on both parent runs.
#
# Before declaring a loss dead, trace the worst divergent boards.  Two named
# suspects, in order: responder's `hcp(10..)` acceptance (24 combined at the
# three level is the same overbid shape the `3♥` leg is already suspected of —
# handoff item 3), and the 2NT partscores themselves in a 4-2.
#
# Resumable: an existing arm dir, gate, or diff file is skipped, and SEED_BASE
# persists in $R/2d-multi-doubler-min-nt.seed.  Iron rule: do NOT edit `src/`
# while this runs — `bba-gen-parallel.sh` rebuilds at the head of every arm
# unless SKIP_BUILD=1 is exported, and `ab-lib.sh` has already built once.
R=${1:?usage: ab-2d-multi-doubler-min-nt.sh RESULTS_DIR}
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

SEED_BASE=$(seed_for 2d-multi-doubler-min-nt)
log "=== 2d-multi-doubler-min-nt SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base  "$v" --their-2d-multi --filter-1nt --no-ns-multi-doubler-minimum-notrump
    arm minnt "$v" --their-2d-multi --filter-1nt

    gatepair minnt base "$v"
    diffpair minnt base "$v"
    sddiff   minnt base "$v"
done

log "2d-multi-doubler-min-nt done"
