#!/bin/sh
# ab-lebensohl-signoff-target.sh — `competition.lebensohl_signoff_raise` at a
# combined target of `24` against the shipped `23` and against `off`
# (docs/research-method-review.md § Issue #9, the follow-up to
# scripts/ab-lebensohl-signoff-raise.sh).
#
#   JOBS=96 BOARDS=384000 BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       DDSS_THREADS=96 setsid nohup scripts/idle-run.sh \
#       scripts/ab-lebensohl-signoff-target.sh ab-results/lebensohl-signoff-target \
#       >ab-results/lebensohl-signoff-target.log 2>&1 < /dev/null &
#
# Why.  At `23` the stretch read plain DD win | PD loss (seed
# 1792096531615205258).  Cross-tabbed on that seed, only 17 HCP with four-plus
# trumps won on both scorers in both cells; three trumps (17 HCP) and four
# trumps at 16 lost PD everywhere.  `24` is exactly that cut on a balanced
# 15-17 opener: no three-trump raise, 17 with four trumps, 16 with five.  The
# cut was chosen on that seed, so this run is a fresh one.
#
# Pre-registered, per vulnerability cell, before any number is read:
#   - `t24 vs t23` is the change.  Ship `24` in a cell on plain non-inferior
#     or better and PD win, or on win | non-inferior with SD-PD agreeing.
#   - `t24 vs off` must not be a loss on either scorer, or the knob goes to
#     `off` in that cell rather than `24`.
#   - Otherwise the cell keeps `23`.  The target is per vulnerability
#     (`WHITE:RED`), so the cells can ship apart.
# Density for the verdict = accepted ÷ scanned from the shards' `scanned`
# counts in this log; rescore with ab-dump-diff / ab-dump-sd --density.
#
# VERDICT (2026-10-16, code 6ce374db + the per-colour target, SEED_BASE
# 1792098317406025283, 384k accepted boards/arm/vul, density 0.004717, gate 0
# foreign in all four pairs).  IMPs/fired, white / red:
#   t24 vs t23 (954 / 832 fired):   plain +0.18 / -0.85  PD +1.33 / +0.80
#                                   sd-lead -0.56 / -1.82  SD-PD +0.37 / -0.48
#   t24 vs off (361 / 318 fired):   plain +1.25 / +2.60  PD +0.52 / +1.33
#                                   sd-lead +1.95 / +3.35  SD-PD +1.36 / +2.27
#   t23 vs off (1315 / 1150 fired): plain +0.21 / +1.33  PD -0.82 / -0.21
#                                   sd-lead +0.91 / +2.28  SD-PD +0.04 / +1.01
# White: t24 vs t23 non-inferior | win, and t24 vs off win | non-inferior,
# so `24` ships.  Red: t24 vs t23 loss | win, PD's artifact row, so red keeps
# `23`.  Default `Some((24, 23))`.
#
# Resumable; SEED_BASE persists in $R/lebensohl-signoff-target.seed.  Iron
# rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-lebensohl-signoff-target.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for lebensohl-signoff-target)
log "=== lebensohl-signoff-target SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm t23 "$v" --filter-lebensohl-raise --ns-lebensohl-signoff-raise 23
    arm t24 "$v" --filter-lebensohl-raise --ns-lebensohl-signoff-raise 24
    arm off "$v" --filter-lebensohl-raise --ns-lebensohl-signoff-raise off
    for base in t23 off; do
        gatepair t24 "$base" "$v"
        diffpair t24 "$base" "$v"
        sddiff   t24 "$base" "$v"
    done
done

log "lebensohl-signoff-target done"
