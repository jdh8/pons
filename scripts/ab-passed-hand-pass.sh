#!/bin/sh
# ab-passed-hand-pass.sh — opener's pass of a passed hand's `1M` response
# (`rebid.passed_hand_major_pass`, docs/next-steps.md item 2).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-passed-hand-pass.sh ab-results/passed-hand-pass \
#       >ab-results/passed-hand-pass.log 2>&1 < /dev/null &
#
# What is being tested.  After `- - 1x - 1M -` (third or fourth seat, so
# responder is a passed hand) our rebid table is forcing: a balanced minimum
# with three-card support rebids `1NT` and responder drives on.  BBA passes —
# the 494f0c4b anchor prices our `1NT` against its pass at −1,766 plain /
# −2,683 PD per 409,600 boards (1,042 rows; −2.0 / −1.7 / −0.8 plain per row at
# 12 / 13 / 14 HCP).  BBA's pass is exactly three-card support and balanced;
# it passes most 12–13s and about a third of its 14s.
#
#   off  the forcing table (`--ns-passed-hand-major-pass off`)
#   h13  pass on exactly three-card support, balanced, at most 13 HCP
#   h14  the same through 14 HCP — the shipped default since 2026-09-30
#
# SHIPPED `Some(14)` default-on 2026-09-30 on this script's first run
# (SEED_BASE 1790771281, 204,800 boards/arm/vul; none / both):
#   h14 vs off  plain +0.0031 ±0.0008 / +0.0041 ±0.0012,
#               PD    +0.0043 ±0.0010 / +0.0067 ±0.0015, 538 / 610 fired
#   h13 vs off  plain +0.0023 ±0.0006 / +0.0037 ±0.0008,
#               PD    +0.0031 ±0.0007 / +0.0056 ±0.0011, 383 / 421 fired
#   h14 vs h13  plain +0.0008 ±0.0005 / +0.0004 ±0.0008,
#               PD    +0.0012 ±0.0006 / +0.0011 ±0.0010, 160 / 190 fired
#
# Hypothesis: h13 a plain-DD and PD win in both vulnerabilities, ≈ +0.004
# plain / +0.006 PD per board on ~0.35% fired.  h14 vs h13 is the open
# question — the anchor's 14s lose less per row and BBA bids on with most.  A
# loss points first at the contested tail (they balance over the pass and the
# floor owns our side), then at responder's 5-card suits opposite a
# doubleton-free `1NT` that would have played better.
R=${1:?usage: ab-passed-hand-pass.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for passed-hand-pass)

log "=== passed-hand-pass A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --ns-passed-hand-major-pass off
    arm h13 "$vul" --ns-passed-hand-major-pass 13
    arm h14 "$vul" --ns-passed-hand-major-pass 14
    diffpair h13 off "$vul"
    diffpair h14 off "$vul"
    diffpair h14 h13 "$vul"
done
log "=== passed-hand-pass A/B done"
