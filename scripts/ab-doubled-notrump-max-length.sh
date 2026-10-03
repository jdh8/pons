#!/bin/sh
# ab-doubled-notrump-max-length.sh — the longest suit responder's natural
# `1NT` over their double may hold, `1o (X) 1NT`
# (`competition.doubled_notrump_max_length`).
#
#   BOARDS=819200 setsid nohup scripts/idle-run.sh \
#       scripts/ab-doubled-notrump-max-length.sh ab-results/doubled-notrump-max-length \
#       >ab-results/doubled-notrump-max-length.log 2>&1 < /dev/null &
#
# What is being tested.  Since `weak_new_suit_length` 6 the natural `1NT`
# (6–9 HCP) also catches every five-card suit below opener's.  In the
# 2026-10-03 anchor's "we bid 1NT, BBA passes" cell the 26 boards with a
# five-card suit lost −3.50 / −4.42 IMPs each; BBA has no natural `1NT`
# here and passes.  Control is `main` with `doubled_notrump_pass` on, so
# opener's answer to the `1NT` is no longer the forcing notrump.
#
#   any   control: 13, no shape constraint
#   four  --ns-doubled-notrump-max-length 4: the five-card hands pass
#
# Hypothesis (pre-run): ~0.07% fired, ±1 IMP/fired — the sign is the doubt.
#
# SHIPPED default 4 2026-10-04 (SEED_BASE 1791062049, 819,200 boards/arm/
# vul, `ab-results/doubled-notrump-max-length`): plain −0.0001 ±0.0003 none
# (wash) / +0.0006 ±0.0004 both, PD +0.0012 ±0.0004 / +0.0023 ±0.0006;
# 787/806 fired (0.10%), 98% of them a pass where `1NT` was.  Run with the
# knob as an opt-in `--ns-doubled-notrump-max-length 4` on arm; the flags
# below are the post-flip spelling.
R=${1:?usage: ab-doubled-notrump-max-length.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for doubled-notrump-max-length)

log "=== doubled-notrump-max-length A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm any "$vul" --ns-doubled-notrump-max-length 13
    arm four "$vul"
    gatepair four any "$vul"
    diffpair four any "$vul"
done
log "=== doubled-notrump-max-length A/B done"
