#!/bin/sh
# ab-strong-two-notrump.sh — responder's bands over the 22–24 `2NT` rebid
# after `2♣` (`notrump.strong_two_notrump_floors`, docs/next-steps.md item 2,
# the strong `2♣` lane).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-strong-two-notrump.sh ab-results/strong-two-notrump \
#       >ab-results/strong-two-notrump.log 2>&1 < /dev/null &
#
# What is being tested.  `2♣ - 2♦ - 2NT` and `2♣ - 2♥ - 2NT` reuse the
# `2NT`-opening response table, set for 20–21: Stayman 5+, `3NT` 5–10, pass
# below 5.  Opposite 22–24 responder passes 3–4 counts; BBA (EPBot defaults,
# `probe-bba-book --card none`, 2026-10-01) plays `3♣` Stayman on 3+ and
# passes on 0–2.  The 2026-09-30 anchor prices our pass against its `3♣` /
# `3NT` at 138 rows, −0.7k plain / −0.7k PD per 409,600 boards.
#
#   on   every band two lower over the `2♣` rebid (Stayman 3+, `3NT` 3–8,
#        `4NT` 9–10, `6NT` 11–14, pass below 3); the direct `2NT` opening
#        untouched — the shipped default since 2026-10-01
#   off  `--no-ns-strong-two-notrump-floors`, the `2NT`-opening bands
#
# SHIPPED default-on 2026-10-01 on this script's first run (SEED_BASE
# 1790830947, 204,800 boards/arm/vul): plain DD +0.0009 (none) / +0.0016
# (both), PD +0.0006 / +0.0013 IMPs/board, CI ±0.0005–0.0008, 79 / 83 fired.
# Worst boards: opener's 28-count rebidding `2NT` through the 22+ fallback
# (the `3NT` rebid stops at 27), and a 3-count's `3NT` going down.
#
# Hypothesis: a small win on both scorers, ≈ +0.001–0.002 per board; the
# surface is ~0.1% of boards, so read the paired diff, not the headline.
R=${1:?usage: ab-strong-two-notrump.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for strong-two-notrump)

log "=== strong-two-notrump A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-strong-two-notrump-floors
    arm on  "$vul"
    diffpair on off "$vul"
done
log "=== strong-two-notrump A/B done"
