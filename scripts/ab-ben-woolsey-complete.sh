#!/usr/bin/env bash
#
# ab-ben-woolsey-complete.sh — does the Woolsey completion (doubled-Muiderberg
# sit + tails, default-on inside the opt-in Woolsey card since ea10ad6d) hold
# against BEN?  Shipped vs both withheld, our Woolsey over BEN's 1NT, Tier F,
# `--isolate-defense --filter-1nt --wide-1nt`.
#
#   SERVERS=48 BOARDS=8192 DDSS_THREADS=32 setsid nohup \
#       scripts/ab-ben-woolsey-complete.sh ab-results/ben-woolsey-complete \
#       >ab-results/ben-woolsey-complete.log 2>&1 < /dev/null &
#
# Needs exactly SERVERS (default 48) Tier-F servers already up and held
# unchanged across every arm (`scripts/ben-servers.sh start 48 f`; on this box
# with `BEN_PYTHON=$HOME/.local/opt/glibc-2.36/ben-python`).  JOBS is the
# server count, so BOARDS (kept boards under --isolate-defense) alone sets the
# sample.
#
# Why.  The completion shipped on BBA evidence only (scripts/ab-woolsey-
# complete.sh, ab-results/woolsey-complete): both − base plain +0.0525 to
# +0.0619 per kept board, PD win, 0.25–1.21% of kept boards contract-divergent
# at 102,400 kept boards per cell.  BBA is the exploit guard, BEN the north
# star: the sit ("sit their double of our Muiderberg on a doubleton") and the
# tails could be priced off how EPBot doubles and runs.
#
# Arms, one fresh SEED_BASE shared by both, every arm `--isolate-defense
# --ns-notrump-defense woolsey --filter-1nt --wide-1nt` (table B all-BEN, so
# the swing is pure defense):
#   base    = `--no-ns-woolsey-muiderberg-sit --no-ns-woolsey-tails`
#   shipped = Woolsey as shipped (both on)
# SD reading: both arms read as Woolsey, base withholding both completions.
# The trigger is on boards THEY open: gate `theirs`.
# Built with `ReadingProfile::with_house_defense` (2026-10-20): we read BEN's
# calls over OUR 1NT by the house defense, not as our Woolsey.  Isolation
# keeps only boards BEN opens anyway.
#
# Sizing.  BOARDS=8192 kept boards/arm/vul.  At BBA's spread (±0.0066 per
# kept board at 102,400) that is ≈ ±0.023, against an effect of ≈ +0.06 and
# δ = 0.002 / density ≈ 0.034 per kept board.  `both` and `none` first: they
# carry the verdict if the run is cut short.
#
# PRE-REGISTERED 2026-10-20, before any number exists.  shipped − base per
# cell, IMPs per kept board, δ = 0.002 / density (density_of the base arm),
# each scorer's verdict its worst cell (docs/measurement.md, decision table):
#   0. A cell with fewer than 50 contract-divergent boards reads
#      INSUFFICIENT DATA; if every cell is under 50, re-run at BOARDS=16384
#      with the same SEED_BASE in a fresh dir.
#   1. CONFIRMED = plain non-inferior in every cell (95% lower bound above
#      −δ) — the change stays as shipped.  A PD win on top is reported, not
#      required (the ship was on BBA; this is the guard).
#   2. A plain LOSS in any cell (upper bound below 0) → trace the worst
#      boards first (the sit's doubled 4-2 / 5-2, and the tails' floor
#      hand-off), then the reversible default goes to jdh8: the two knobs off
#      inside Woolsey, named for BBA.  Never reverted silently.
#   3. Inconclusive (lower bound at or below −δ, no loss): insufficient data,
#      extend to BOARDS=16384 with the same SEED_BASE before any verdict.
#   4. SD-PD (sddiff) is descriptive.
#
# Resumable; SEED_BASE persists in $R/ben-woolsey-complete.seed.  Iron rule:
# do NOT rebuild binaries or restart the BEN fleet while this runs.
set -euo pipefail
R=${1:?usage: ab-ben-woolsey-complete.sh RESULTS_DIR}
SERVERS=${SERVERS:-48}
GENERATOR=ben-gen
JOBS=$SERVERS
VULS=${VULS:-both none ns ew}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
NOTE=$(ben_note)

SEED_BASE=$(seed_for ben-woolsey-complete)
log "=== ben-woolsey-complete SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul vuls=$VULS $NOTE"

BASE='--isolate-defense --ns-notrump-defense woolsey --filter-1nt --wide-1nt'
WITHHELD='--no-ns-woolsey-muiderberg-sit --no-ns-woolsey-tails'
READ='--on-ns-notrump-defense woolsey --off-ns-notrump-defense woolsey
      --off-no-ns-woolsey-muiderberg-sit --off-no-ns-woolsey-tails'
for v in $VULS; do
    # shellcheck disable=SC2086
    arm base "$v" -t f --note "$NOTE" $BASE $WITHHELD
    # shellcheck disable=SC2086
    arm shipped "$v" -t f --note "$NOTE" $BASE
    DENSITY=$(density_of "$R/base-$v")
    log "density $v: $DENSITY"
    gatepair shipped base "$v" theirs
    diffpair shipped base "$v"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.shipped.vs.base.$v.plain.txt")
    log "pre-count shipped vs base $v: ${fired:-0} contract-divergent boards"
    # shellcheck disable=SC2086
    sddiff shipped base "$v" $READ
done
log "ben-woolsey-complete done"
