#!/bin/sh
# ab-ben-law-raise.sh — the BEN guard of ab-law-raise.sh: the shipped Law-raise
# rail (`InstinctProfile::advance_law_raise`, default `Some(0)` since b554b985)
# against the rail off, vs BEN Tier F, four cells.
#
#   plain   american with the rail off (--ns-law-raise off)         (control)
#   law0    american as shipped (--ns-law-raise 0)
#
#   SERVERS=16 BOARDS=102400 setsid nohup scripts/idle-run.sh \
#       scripts/ab-ben-law-raise.sh ab-results/ben-law-raise \
#       >ab-results/ben-law-raise.log 2>&1 </dev/null &
#
# Needs exactly SERVERS (default 16) Tier-F servers already up and held
# unchanged across every arm (`scripts/ben-servers.sh start 16 f`).  JOBS is
# the server count, so BOARDS alone sets the sample.
#
# Why.  The rail shipped on BBA evidence (204,800 boards per cell, plain win
# 4/4, PD loss when we are vulnerable; docs/floor-rail-campaign.md R9).  BEN
# is the north star, so the ship owes a per-batch Tier-F check
# (docs/ben-gap-campaign.md, the ship rule).  Slack 1 is not an arm: it lost
# to slack 0 against BBA and a BEN arm costs hours.
#
# Sizing.  The rail fires on 0.43-0.55% of boards against BBA, so 102,400
# boards per cell is about 500 contract-divergent boards and a 95% interval
# near +-0.003 IMPs/board: enough to see a loss the size of the BBA win,
# thin for a non-inferiority proof at delta = 0.002.
#
# The trigger is on boards THEY open: gate `theirs`.  Cells run we-vulnerable
# first (ns, both: the two PD losses against BBA), so a run stopped early has
# read the cells that matter most.
#
# PRE-REGISTERED 2026-11-15, before any number exists.  law0 - plain per cell,
# IMPs/board, delta = 0.002, density 1 (docs/measurement.md, decision table):
#   1. CONFIRMED = plain DD non-inferior or win in every cell; the BBA ship
#      stands.  PD is recorded beside it, never a veto against BEN (its real
#      doubles are already in plain).
#   2. A plain LOSS in any cell (upper bound below 0) goes to jdh8 with the
#      reversible default (`Some(1)`, which never lost a cell on either scorer
#      against BBA) after tracing the worst boards; never reverted silently.
#   3. Inconclusive (lower bound at or below -delta, no loss) is recorded as
#      inconclusive, not as confirmation; the default stays on the BBA proof.
#   4. SD (sddiff, 16 worlds) is descriptive: part of the raise's worth is
#      obstruction that DD underprices.
#   5. On the read, re-check the two R9-trace leads on the BEN boards: the
#      `(1NT) 2M (X) 3M` slice and their-preempt slice.
#
# Resumable; SEED_BASE persists in $R/ben-law-raise.seed.  Iron rule: do NOT
# rebuild binaries or restart the BEN fleet while this runs.
R=${1:?usage: ab-ben-law-raise.sh RESULTS_DIR}
SERVERS=${SERVERS:-16}
GENERATOR=ben-gen
JOBS=$SERVERS
VULS=${VULS:-ns both none ew}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
NOTE=$(ben_note)
SEED_BASE=$(seed_for ben-law-raise)

log "=== ben-law-raise start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS, $NOTE"
for vul in $VULS; do
    arm plain "$vul" -t f --note "$NOTE" --our-floor american --ns-law-raise off
    arm law0 "$vul" -t f --note "$NOTE" --our-floor american --ns-law-raise 0
    gatepair law0 plain "$vul" theirs
    diffpair law0 plain "$vul"
    sddiff law0 plain "$vul"
done
log "=== ben-law-raise done"
