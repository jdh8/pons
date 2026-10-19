#!/usr/bin/env bash
#
# ab-ben-nt-overcall-floor.sh — is the shipped `natural_overcall_hcp_floor = 0`
# a system or a BBA exploit?  Floor 8 (the default before 2026-10-16) against
# floor 0 over BEN's 1NT, Tier F, `--filter-1nt --wide-1nt`, all four cells.
#
#   BOARDS=100000 DDSS_THREADS=96 setsid nohup scripts/idle-run.sh \
#       scripts/ab-ben-nt-overcall-floor.sh ab-results/ben-nt-overcall-floor \
#       >ab-results/ben-nt-overcall-floor.log 2>&1 </dev/null &
#
# Needs exactly SERVERS (default 48) Tier-F servers already up and held
# unchanged across every arm (`scripts/ben-servers.sh start 48 f`; on this box
# with `BEN_PYTHON=$HOME/.local/opt/glibc-2.36/ben-python`).  JOBS is the
# server count, so BOARDS alone sets the sample (rounded up per shard).
#
# Why.  Floor 0 shipped on BBA evidence only (scripts/ab-nt-overcall-floor-
# sweep.sh, SEED_BASE 1792143998575635198): floor 8 − floor 0 lost on plain,
# PD and SD-PD in every cell, which is not the usual obstruction-wall shape.
# A 0–6 HCP overcall that gains with failures doubled may gain because BBA
# mis-bids after it.  BEN is the second opponent; this reads whether the gain
# survives it.  Floor 8, not 7: it fires 4× as often (3,227 vs 823 of 393k
# accepted boards against BBA), so it resolves at a quarter of the boards.
#
# PRE-REGISTERED 2026-10-19, before any number is read.  floor8 − floor0 per
# cell, IMPs/accepted board, δ = 0.002 (docs/measurement.md, decision table):
#   - SYSTEM (floor 0 stands): in every cell floor 0 is plain non-inferior
#     (floor8 − floor0 upper bound below +δ), and floor 8 wins no cell on PD
#     or SD-PD.  Nothing changes.
#   - EXPLOIT: floor 8 wins plain DD (lower bound above 0) in 2+ cells, or is
#     plain non-inferior with PD and SD-PD wins in 2+ cells.  The proposed
#     reversible default goes to jdh8, not applied: floor 0 behind a knob
#     named for BBA, the default back to the sound partition (7 or 8).
#   - Anything else is INCONCLUSIVE: a second fresh SEED_BASE on all four
#     cells before any reading.  The same holds for a clean reading that
#     contradicts the BBA sweep's sign in 2+ cells.
#   - Gate: 0 foreign in every pair (`gatepair … theirs`), else no headline.
#
# Resumable; SEED_BASE persists in $R/ben-nt-overcall-floor.seed.  Iron rule:
# do NOT rebuild binaries or restart the BEN fleet while this runs.
set -euo pipefail
R=${1:?usage: ab-ben-nt-overcall-floor.sh RESULTS_DIR}
SERVERS=${SERVERS:-48}
GENERATOR=ben-gen
JOBS=$SERVERS
SHOW=${SHOW:-50}
# `both` early: it carries the verdict if the run is cut short.
VULS=${VULS:-none both ns ew}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
NOTE=$(ben_note)

SEED_BASE=$(seed_for ben-nt-overcall-floor)
log "=== ben-nt-overcall-floor SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul vuls=$VULS $NOTE"

FILTER='--filter-1nt --wide-1nt'
for v in $VULS; do
    # shellcheck disable=SC2086
    arm floor0 "$v" -t f --note "$NOTE" $FILTER --ns-nt-overcall-hcp-floor 0
    # shellcheck disable=SC2086
    arm floor8 "$v" -t f --note "$NOTE" $FILTER --ns-nt-overcall-hcp-floor 8
    DENSITY=$(density_of "$R/floor0-$v")
    log "density $v: $DENSITY"
    gatepair floor8 floor0 "$v" theirs
    diffpair floor8 floor0 "$v"
    sddiff floor8 floor0 "$v" --on-ns-nt-overcall-hcp-floor 8 \
        --off-ns-nt-overcall-hcp-floor 0
done

log "=== ben-nt-overcall-floor done"
