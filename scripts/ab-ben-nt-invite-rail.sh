#!/usr/bin/env bash
#
# ab-ben-nt-invite-rail.sh — does the shipped 1NT-invite rail (both halves,
# default-on since 5fec4d95) hold against BEN?  Shipped vs both halves
# disarmed over BEN's 1NT, Tier F, `--filter-1nt --wide-1nt`.
#
#   SERVERS=48 BOARDS=8192 VULS=none DDSS_THREADS=32 setsid nohup \
#       scripts/ab-ben-nt-invite-rail.sh ab-results/ben-nt-invite-rail \
#       >ab-results/ben-nt-invite-rail.log 2>&1 < /dev/null &
#
# Needs exactly SERVERS (default 48) Tier-F servers already up and held
# unchanged across every arm (`scripts/ben-servers.sh start 48 f`; on this box
# with `BEN_PYTHON=$HOME/.local/opt/glibc-2.36/ben-python`).  JOBS is the
# server count, so BOARDS alone sets the sample.
#
# Why.  The rail shipped on BBA evidence only (scripts/ab-nt-invite-rail.sh).
# Under the honest reading of their minors (bba-gen's default since 88fd240a,
# and how ben-gen always reads BEN), the rail moved the contract on only 1–17
# boards per 196,608-board BBA cell (`readdxb − read`): our floor already
# passes most of what the rail masks.  BEN bids about 13 boards/s on 48
# servers, so a BBA-sized fire rate needs days per arm per cell.  This runner
# is therefore a PRE-COUNT first: one cell, a few thousand boards, to learn
# whether BEN invites into the rail's nodes far more often than BBA does.
#
# Arms, one fresh SEED_BASE shared by both:
#   off     = `--no-ns-nt-invite-double-veto --no-ns-nt-invite-bid-veto`
#   shipped = american as shipped (both halves on)
# The rail masks calls and publishes nothing new: no disclosure flags.  The
# trigger is on boards THEY open: gate `theirs`.
#
# PRE-REGISTERED 2026-10-20, before any number exists.  shipped − off per cell,
# IMPs/accepted board, δ = 0.002 (docs/measurement.md, decision table):
#   0. PRE-COUNT (BOARDS=8192, VULS=none).  The contract-divergent count
#      (ab-dump-diff's `fired`) is the whole result: no verdict line is read.
#      Under 5 fired → the BEN check is INFEASIBLE at BEN's throughput (50
#      divergent boards per cell would take more than 80,000 boards per arm per
#      cell); record it and stop: the rail stays on BBA evidence, a recorded
#      gap, no change.  5 or more → size the full run for 50 per cell
#      (BOARDS ≈ 8192 × 50 / fired, all four cells) and amend this header
#      before launching it.
#   1. Full run (only after 0): the standard table, worst cell, both scorers.
#      A BEN plain loss (upper bound below 0) in any cell on a default-on change
#      goes to jdh8 with the reversible default (both halves off, the disarming
#      flags as the arm); never reverted silently.  A wash is a BEN-neutral
#      confirmation of the BBA ship.
#
# Resumable; SEED_BASE persists in $R/ben-nt-invite-rail.seed.  Iron rule:
# do NOT rebuild binaries or restart the BEN fleet while this runs.
set -euo pipefail
R=${1:?usage: ab-ben-nt-invite-rail.sh RESULTS_DIR}
SERVERS=${SERVERS:-48}
GENERATOR=ben-gen
JOBS=$SERVERS
VULS=${VULS:-none}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
NOTE=$(ben_note)

SEED_BASE=$(seed_for ben-nt-invite-rail)
log "=== ben-nt-invite-rail SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul vuls=$VULS $NOTE"

FILTER='--filter-1nt --wide-1nt'
OFF='--no-ns-nt-invite-double-veto --no-ns-nt-invite-bid-veto'
for v in $VULS; do
    # shellcheck disable=SC2086
    arm off "$v" -t f --note "$NOTE" $FILTER $OFF
    # shellcheck disable=SC2086
    arm shipped "$v" -t f --note "$NOTE" $FILTER
    gatepair shipped off "$v" theirs
    diffpair shipped off "$v"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.shipped.vs.off.$v.plain.txt")
    log "pre-count shipped vs off $v: ${fired:-0} contract-divergent boards"
done
log "ben-nt-invite-rail done"
