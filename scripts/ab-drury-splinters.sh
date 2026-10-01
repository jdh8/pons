#!/bin/sh
# ab-drury-splinters.sh — opener's splinters over Reverse Drury
# (`response.drury_splinters`, docs/next-steps.md item 2).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-drury-splinters.sh ab-results/drury-splinters \
#       >ab-results/drury-splinters.log 2>&1 < /dev/null &
#
# What is being tested.  After `- - 1M - 2♣! -` opener's short game hands
# today bid `4M` (16–18) or `4NT` RKCB (19+ support points, which counts the
# shortness), and the ask reaches slams off a wasted honor opposite the void.
# On, a jump in a new suit (`3♦`, the other major at the three level, `4♣`)
# is a singleton or void with 16–20 support points (21+ keeps the direct
# `4NT`); responder bids `4M`, or the
# next step with no K/Q/J in the short suit and 10+ HCP, over which opener
# asks `4NT` on 19+.  Priced off the Drury A/B's dumps (boards where both
# tables reach `1♠ - 2♣ -`): BBA's three `1♠` splinters won +122 / +159 plain
# per 204,800 boards (none / both), its `1♥` splinters about zero.
#
#   off  Drury without splinters, `--no-ns-drury-splinters`
#   on   the splinters — the shipped default since 2026-10-01
#
# Run 1 (`ab-results/drury-splinters`, SEED_BASE 1790805316, 204,800
# boards/arm/vul; none / both): WASH, plain −0.0003 ±0.0009 / −0.0001
# ±0.0012, PD −0.0002 ±0.0009 / +0.0000 ±0.0012, 102 / 127 fired.  Two
# causes in the worst boards: responder's step barred the ace in the short
# suit (a control, not waste), and the splinter had no ceiling, so 21+
# monsters that used to ask `4NT` directly stopped in `4M` opposite a
# sign-off.  Run 2 admits the ace and caps the splinter at 20.
#
# Run 2 (`ab-results/drury-splinters2`, SEED_BASE 1790806037): WIN, plain
# +0.0008 ±0.0005 / +0.0010 ±0.0007, PD +0.0009 ±0.0005 / +0.0012 ±0.0008,
# 38 / 54 fired.  Run 3, the confirmation (`drury-splinters3`, SEED_BASE
# 1790806590): plain +0.0000 ±0.0007 / +0.0002 ±0.0010, PD +0.0001 ±0.0007
# / +0.0003 ±0.0010, 64 / 77 fired.  Pooled (409,600 boards/vul): plain
# +0.0004 ±0.0004 / +0.0006 ±0.0006, PD +0.0005 ±0.0004 / +0.0007 ±0.0006;
# SHIPPED default-on 2026-10-01.  Runs 1–3 were generated with the pre-flip
# `--ns-drury-splinters` arm.  The gains are the old direct `4NT` on 19–20
# support points with shortness reaching a failing five level or slam.
#
# Hypothesis: on wins ≈ +0.0005 plain / PD per board on ~0.05% fired; the
# slam-deciding boards are ±13 IMPs each, so read the pooled seeds, not one.
R=${1:?usage: ab-drury-splinters.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for drury-splinters)

log "=== drury-splinters A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-drury-splinters
    arm on "$vul"
    diffpair on off "$vul"
done
log "=== drury-splinters A/B done"
