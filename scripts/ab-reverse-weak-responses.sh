#!/bin/sh
# ab-reverse-weak-responses.sh — responder's weak answers (at most 7 HCP) to
# opener's reverse (`1♣ - 1♥ - 2♦`, `1♣ - 1♠ - 2♦` / `2♥`, `1♦ - 1♠ - 2♥`):
# the weak rebid of a six-card suit, the weak raise of the reverse suit, the
# preference to opener's minor; 8+ HCP and the weak misfit stay the floor's;
# opener passes the weak raise below 19 (`rebid.reverse_weak_responses`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-reverse-weak-responses.sh ab-results/reverse-weak-responses \
#       >ab-results/reverse-weak-responses.log 2>&1 < /dev/null &
#
# What is being tested.  Off, the net floor passes the forcing reverse on a
# weak hand (43 of 943 reverses at anchor `25aea82c`, shipping arms).  The
# full BBA structure (fourth suit 9+ GF, `3NT` 9–14, `6NT` 15+, this table
# below) read a wash on seed 1791363144 (`ab-results/reverse-fourth-suit-v2`):
# plain −0.0002 / −0.0003, PD −0.0001 / −0.0002, 61 / 69 fired; every gain a
# weak hand the floor had passed, every loss a game or slam the floor had
# evaluated better than an HCP band.  This arm keeps the gains only.
#
#   off  the default
#   on   --ns-reverse-weak-responses
#
# sha a77ecbe4-dirty, 204,800 boards/arm/vul, isolation gate passed.
# IMPs/board none / both:
#
#   seed 1 (1791366826, reverse-weak-responses)    plain −0.0000 / −0.0001, PD −0.0000 / −0.0001, 17 / 17 fired
#   seed 2 (1791367325, reverse-weak-responses-2)  plain +0.0002 / +0.0001, PD +0.0002 / +0.0001, 18 / 21 fired
#
# Pooled plain +0.0001 / +0.0000, PD +0.0001 / +0.0000, every cell inside its
# CI — **a wash, default off (2026-10-07)**.  Earlier forms: the full BBA
# structure (`reverse-fourth-suit-v1`, `-v2`) lost; the weak table with a
# `2NT` catch-all, unguarded (`reverse-weak-responses-v3`, `-v3-2`) then
# guarded (`-v4`, `-v4-2`), pooled the same sign and size as this.  The one
# consistent loser is the weak heart raise on five-card support where the
# floor's direct `4♥` makes.
# Each new seed wants a fresh results dir.
R=${1:?usage: ab-reverse-weak-responses.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for reverse-weak-responses)

log "=== reverse-weak-responses A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm on "$vul" --ns-reverse-weak-responses
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== reverse-weak-responses A/B done"
