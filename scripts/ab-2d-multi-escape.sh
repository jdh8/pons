#!/bin/sh
# ab-2d-multi-escape.sh — N4e: the floorless weak escape over their `(2♦)` Multi
# (docs/one-notrump-competitive.md §N4e).
#
#   JOBS=24 BOARDS=460800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-2d-multi-escape.sh ab-results/2d-multi-escape \
#       >ab-results/2d-multi-escape.log 2>&1 < /dev/null &
#
# What is being tested.  `competition.multi_weak_escape` (`--ns-multi-weak-escape`):
# below 5 HCP, `Pass` is today the only finite row in responder's table over a
# declared Multi — the natural `2♥`/`2♠` wants a five-card major and `hcp 5+`,
# the `2NT` relay a five-card suit and `hcp 6+`.  The census prices that pass at
# **−3.92 plain / −4.84 PD per board** on the `≤5 hcp, 6+ suit` class, the worst
# per-board cell in the campaign, while the relay lane those hands want measures
# −0.08 plain / +0.14 PD.  `Some(n)` adds a floorless `len(suit, n..)` rung to
# both outlets, drops opener's sign-off raise to the matching floor so the
# reading and the game bar move together, and authors the escape's interfered
# tail (`1NT (2♦) 2M (X/2♠/2NT/3x)`) — unauthored today, and the floor bid
# *their* suit at the four level over partner's escape in the dumps.
#
# Three arms, one seed set, `--their-2d-multi` on all of them so the only
# difference is the escape:
#
#   base   None      the shipped lane
#   six    Some(6)   target cell 37 bd, −145 plain / −179 PD
#   five   Some(5)   adds the 97-bd five-card class (−168 plain / −81 PD)
#
# `five` is priced against `base` **and** paired against `six`: the five-card
# band is exactly what `lebensohl_relay_shape`'s PD-distilled floor was
# distilled against, so a `six` win plus a `five` loss is a live and expected
# outcome (N3 rounds 6–7 are the precedent for splitting rather than bundling).
#
# `--filter-1nt` (balanced 15-17 somewhere, a raw-hand test applied BEFORE any
# bidding) rides every arm so they deal the same board set and stay seed-aligned
# for the paired diffs.  Headline is IMPs per *accepted* deal, with
# `per-board = conditional mean × trigger density` alongside.
#
# Scoring.  Plain AND perfect defense, read off the decision table in
# docs/measurement.md; `sddiff` is the tie-breaker.  Read
# `probe-divergence --gate-opener ours` BEFORE any headline: this campaign's
# mirror-read leak makes a counter knob a reading knob, and the gate must read
# **0 foreign** on every pair.
#
# Resumable: an existing arm dir, gate, or diff file is skipped, and SEED_BASE
# persists in $R/2d-multi-escape.seed.  Iron rule: do NOT rebuild binaries while
# this runs.
R=${1:?usage: ab-2d-multi-escape.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
PROBE=target/release/examples/probe-divergence

SEED_BASE=$(seed_for 2d-multi-escape)
log "=== 2d-multi-escape SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --their-2d-multi --ns-multi-weak-escape off --filter-1nt
    arm six  "$v" --their-2d-multi --ns-multi-weak-escape 6   --filter-1nt
    arm five "$v" --their-2d-multi --ns-multi-weak-escape 5   --filter-1nt

    for pair in six:base five:base five:six; do
        on=${pair%%:*}
        off=${pair#*:}
        gatepair "$on" "$off" "$v"
        diffpair "$on" "$off" "$v"
        sddiff "$on" "$off" "$v"
    done
done

log "2d-multi-escape done"
