#!/bin/sh
# ab-drury.sh — Reverse Drury: a passed hand's `2♣!` limit raise of a
# third/fourth-seat `1M` (`response.drury`, docs/next-steps.md item 2).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-drury.sh ab-results/drury \
#       >ab-results/drury.log 2>&1 < /dev/null &
#
# What is being tested.  A passed hand's response to a third/fourth-seat `1M`
# is the second half of the passed-hand lane: 1,890 rows, −1,931 plain /
# −2,395 PD per 409,600 boards at the 494f0c4b anchor, of which BBA's Drury
# `2♣` is 1,144 (−719 / −810).  Off, a passed hand's limit raise is `3M`
# (four trumps) or the forcing `1NT` (three), and opener's answers to either
# never learn the fit-plus-strength picture in one call.  On, `2♣!` is
# three-plus trumps and 10+ support points; opener answers `2M` (≤12),
# `2♦!` (13–15) or `4M` (16+); responder signs off or bids game; their
# double is systems on and their overcall gets a small ladder.  BBA reads
# ours off the `Reverse drury` card row and plays the original itself.
#
#   off  no Drury (`3M` / forcing `1NT`), `--no-ns-drury`
#   on   Reverse Drury — the shipped default since 2026-10-01
#
# SHIPPED default-on 2026-10-01 on run 4, the finished build (`ab-results/
# drury5`, SEED_BASE 1790801205, 204,800 boards/arm/vul; none / both):
#   plain +0.0013 ±0.0012 / +0.0024 ±0.0017, PD +0.0010 ±0.0013 / +0.0020
#   ±0.0019, 458 / 566 fired.  Pooled with runs 2–3 (819,200 boards/vul):
#   plain +0.0010 ±0.0006 / +0.0019 ±0.0009, PD +0.0007 ±0.0007 / +0.0017
#   ±0.0010.  Runs 1–4 were generated with the pre-flip `--ns-drury` arm.
#
# Run 1 (`ab-results/drury`, SEED_BASE 1790798057) was CONFOUNDED: the Drury
# rule sat on the shared responder table and its box leaked into every
# unpassed `1M - 2♣` reading (projection-blind seat gate); 2,979 / 3,492
# divergent tables per vul, of which only 253 / 298 were the passed-hand 2♣
# lane.  It still read non-negative: plain +0.0006 ±0.0018 / +0.0024
# ±0.0024, PD +0.0002 ±0.0019 / +0.0026 ±0.0026 (none / both).
#
# Runs 2 and 3 (`ab-results/drury2`, seed 1790799288; `drury3`, seed
# 1790799914) had the passed-hand seats keyed separately, but still answered
# a passed hand's natural 2♣ over their double with the Drury ladders (26
# tables/vul) and left their double of the 2♦! relay to the floor, which
# passed it out (the worst boards).  Non-negative on all eight cells, plain
# +0.0016 ±0.0012 / +0.0021 ±0.0019 and +0.0006 ±0.0013 / +0.0017 ±0.0020,
# PD +0.0013 ±0.0013 / +0.0017 ±0.0020 and +0.0000 ±0.0014 / +0.0009
# ±0.0021 (none / both).  Run 4 onward is the finished build (both tails
# authored, 4NT on 19+).
#
# Hypothesis: on wins plain and PD in both vulnerabilities, ≈ +0.002 plain /
# +0.002 PD per board on ~0.5% fired.  A loss points first at the relay's
# sign-off (`2♦! - 2M` played opposite 13–15 where `3M` or the old `1NT`
# scored better), then at the overcall ladder.
R=${1:?usage: ab-drury.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for drury)

log "=== drury A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-drury
    arm on "$vul"
    diffpair on off "$vul"
done
log "=== drury A/B done"
