#!/bin/sh
# ab-lebensohl-signoff-target.sh — `competition.lebensohl_signoff_raise` at a
# combined target of `24` against the shipped `23` and against `off`
# (docs/archive/research-method-review-closed.md § Issue #9, the follow-up to
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
# Rescored 2026-10-16 with --density 0.004717 (accepted ÷ scanned, 96
# shards), adding the direct t23 vs off pair; IMPs/accepted board ±95% CI:
#   t24 vs t23 white: plain +0.0005 ±0.0008 non-inferior, PD +0.0033 ±0.0009 win
#   t24 vs t23 red:   plain -0.0018 ±0.0011 loss, PD +0.0017 ±0.0013 win,
#                     sd-lead -0.0039 ±0.0012, SD-PD -0.0010 ±0.0013
#   t23 vs off red:   plain +0.0040 ±0.0013 win, PD -0.0006 ±0.0015
#                     non-inferior, sd-lead +0.0068 ±0.0014, SD-PD +0.0030
#                     ±0.0016 win
# Scope: --filter-lebensohl-raise then took only a strict balanced opener
# (4333/4432/5332), where points = HCP, so `24` meant no three-trump raise
# *there*.  The default also opens 1NT on 5m422/6m322, which upgrades a point:
# a 17-HCP one still raises on three trumps at `24`.  That slice was not
# measured.
#
# Wide confirmation (pre-registered 2026-10-16, before any number is read).
# The filter now takes any 15-17 HCP candidate with no suit under two, a
# superset of the default opener.  Fresh seed, BOARDS=384000, results in
# ab-results/lebensohl-signoff-target-wide.  Per vulnerability cell:
#   - White `24` stands if t24 vs t23 is plain non-inferior or better with PD
#     not a loss, and t24 vs off is not a loss on either scorer.
#   - Red `23` stays the recorded trade-off unless t23 vs off reads a plain
#     loss.
#   - Any other reading is flagged for a decision, not applied.
# WIDE VERDICT (2026-10-16, SEED_BASE 1792101907466221783, density 0.005522,
# gate 0 foreign), IMPs/accepted board, white / red:
#   t24 vs t23: plain +0.0007 ±0.0008 / -0.0015 ±0.0010,
#               PD +0.0031 ±0.0009 / +0.0017 ±0.0012
#   t24 vs off: plain +0.0010 ±0.0005 / +0.0020 ±0.0007,
#               PD +0.0003 ±0.0006 / +0.0009 ±0.0008
#   t23 vs off: plain +0.0003 ±0.0009 / +0.0034 ±0.0013,
#               PD -0.0028 ±0.0010 / -0.0008 ±0.0015
# Every condition met: white `24` and red `23` stand.  13% of white `23`'s
# raises were 5m422/6m322 openers; `24` still raises ten on three trumps.
#
# Asymmetric cells (pre-registered 2026-10-16, before any number is read;
# docs/archive/research-method-review-closed.md § 6).  VULS="ns ew", fresh seed,
# BOARDS=384000, wide filter, ab-results/lebensohl-signoff-target-nsew.  We
# sit N/S, so `ns` is red against white (the red target, `23`, applies) and
# `ew` white against red (`24` applies):
#   - ew: `24` stands under the white conditions above.
#   - ns: `23` stays the recorded trade-off unless t23 vs off reads a plain
#     loss.
#   - Any other reading is flagged for a decision, not applied.
# NS/EW VERDICT (2026-10-16, SEED_BASE 1792104633669095399, density
# 0.005478, gate 0 foreign), IMPs/accepted board:
#   ew t24 vs t23: plain -0.0001 ±0.0008 non-inferior, PD +0.0022 ±0.0009 win
#   ew t24 vs off: plain +0.0009 ±0.0005 win, PD +0.0003 ±0.0006 non-inferior
#   ns t23 vs off: plain +0.0035 ±0.0013 win, PD -0.0011 ±0.0015 non-inferior
#   ns t24 vs t23: plain -0.0016 ±0.0011 loss, PD +0.0019 ±0.0012 win
# Every condition met: the colour split holds in all four cells.
#
# Resumable; SEED_BASE persists in $R/lebensohl-signoff-target.seed.  Iron
# rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-lebensohl-signoff-target.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for lebensohl-signoff-target)
log "=== lebensohl-signoff-target SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in $VULS; do
    arm t23 "$v" --filter-lebensohl-raise --ns-lebensohl-signoff-raise 23
    arm t24 "$v" --filter-lebensohl-raise --ns-lebensohl-signoff-raise 24
    arm off "$v" --filter-lebensohl-raise --ns-lebensohl-signoff-raise off
    for pair in "t24 t23" "t24 off" "t23 off"; do
        set -- $pair
        gatepair "$1" "$2" "$v"
        diffpair "$1" "$2" "$v"
        sddiff   "$1" "$2" "$v"
    done
done

log "lebensohl-signoff-target done"
