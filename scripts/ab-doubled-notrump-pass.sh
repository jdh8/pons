#!/bin/sh
# ab-doubled-notrump-pass.sh — opener's answer to the natural `1NT` over
# their double, `1M (X) 1NT -` (`competition.doubled_notrump_pass`).
#
#   BOARDS=819200 setsid nohup scripts/idle-run.sh \
#       scripts/ab-doubled-notrump-pass.sh ab-results/doubled-notrump-pass \
#       >ab-results/doubled-notrump-pass.log 2>&1 < /dev/null &
#
# What is being tested.  Responder's `1NT` over their double is 6–9 and
# non-forcing, but the systems-on rebase replays `1M - 1NT`, the uncontested
# forcing notrump: opener never passes, and rebids a three-card minor on a
# balanced 12-count.  In the 2026-10-03 anchor's "we bid 1NT, BBA passes"
# cell, over a major opening, opener's pass gained +2.18 / +1.45 IMPs/board
# (11 boards) while a new-suit rebid lost −2.33 / −2.31 (49) and the 2M
# rebid −1.43 / −0.97 (35).  Anchor cells overstate (BBA-vs-BBA reference).
#
#   off   control: the rebase's forcing-notrump rebid
#   pass  --ns-doubled-notrump-pass: a balanced hand of ≤17 HCP passes
#
# Hypothesis (pre-run): ~0.05% fired, +1…2 IMPs/fired.
#
# SHIPPED default-on 2026-10-04 (SEED_BASE 1791059766, 819,200 boards/arm/
# vul, `ab-results/doubled-notrump-pass`): plain +0.0001 ±0.0001 none /
# +0.0001 ±0.0001 both, PD +0.0002 ±0.0001 / +0.0002 ±0.0002; 87/91 fired
# (0.01%), +0.7…+1.8 IMPs/fired.  Rarer than guessed: advancer usually acts
# before opener's turn.  Known drift inside the win: a rejecting table still
# authors the reading, so opener's non-pass rebids read off it (`1♥ (X) 1NT
# - 3♠` reads 9–21 HCP, was 16–21).  Run with the knob as an opt-in
# `--ns-doubled-notrump-pass` on arm; the flags below are the post-flip
# spelling.
R=${1:?usage: ab-doubled-notrump-pass.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for doubled-notrump-pass)

log "=== doubled-notrump-pass A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-doubled-notrump-pass
    arm pass "$vul"
    gatepair pass off "$vul"
    diffpair pass off "$vul"
done
log "=== doubled-notrump-pass A/B done"
