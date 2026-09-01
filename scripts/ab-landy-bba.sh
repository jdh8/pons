#!/bin/sh
# ab-landy-bba.sh — N1j of the competitive-1NT campaign
# (docs/one-notrump-competitive.md §N1j): the BBA-ladder Landy counter — the
# anchor-aligned table — against the shipped stack, plus its weak-2♦ cap arm.
#
#   JOBS=24 BOARDS=153600 setsid nohup scripts/idle-run.sh scripts/ab-landy-bba.sh \
#       ab-results/landy-bba >ab-results/landy-bba.log 2>&1 < /dev/null &
#
# Arms, all with `--their-2c-landy true --filter-1nt` (the true declaration of
# the reference opponent plus the enriched raw-hand gate, applied to every arm
# so they stay seed-aligned):
#
#   bba-off  the shipped stack (N1c+N1d/e/f defaults)
#   bba-on   N1j: the BBA ladder — wide transfers 2NT→♣ / 3♣→♦, the GF
#            both-minors takeout/splinter family, values X and weak 2♦ verbatim
#   bba-cap  N1j + the weak-2♦ hcp(..=6) cap (the N1i `2♦ → Pass` lead)
#
# Pairs: bba-on↔bba-off is the alignment verdict — ship gate is
# **non-inferiority** (zero CI-clear negative cells across pooled
# {NV,vul}×{plain,PD}), the rationale being structural alignment with the
# anchor rather than IMPs; bba-cap↔bba-on is the clean 2♦ increment.  Guard:
# the pre-ship decompose must isolate the `2M ← X` migration rows — a CI-clear
# negative there drops the takeout pair from the bundle, and the rest re-arms.
#
# The isolation gate leaks here by construction (the ladder deletes the cue
# *constraints*, which is what reopens the mirror-read leak); read
# `probe-divergence --gate-opener ours` as a split, not pass/fail.
#
# **Both knobs SHIPPED DEFAULT-ON 2026-08-15** — the ladder on the
# non-inferiority gate (all eight DD cells leaning positive, zero CI-clear
# negatives, the `2M ← X` guard vacuous), the cap on the standard gate
# (plain wash | PD win both vuls, isolation gate 0 foreign).  Arms below are
# spelled in post-flip terms: bba-cap *is* the default; bba-off pins the
# ladder off, bba-on pins the cap off.
#
# Pool three seeds: run this script three times with R = ab-results/landy-bba,
# …-v2, …-v3 (each dir keeps its own persistent seed).  Iron rule: do NOT
# rebuild binaries while this runs.
R=${1:?usage: ab-landy-bba.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-bba)
log "=== landy-bba SEED_BASE=$SEED_BASE sha=$SHA"

for v in none both; do
    arm bba-off "$v" --their-2c-landy true --defense-2c-landy-bba false --filter-1nt
    arm bba-on "$v" --their-2c-landy true --defense-2c-landy-bba true \
        --defense-2c-landy-weak-2d-cap false --filter-1nt
    arm bba-cap "$v" --their-2c-landy true --defense-2c-landy-bba true \
        --defense-2c-landy-weak-2d-cap true --filter-1nt
    diffpair bba-on bba-off "$v"
    sddiff bba-on bba-off "$v"
    diffpair bba-cap bba-on "$v"
    sddiff bba-cap bba-on "$v"
done

log "landy-bba done"
