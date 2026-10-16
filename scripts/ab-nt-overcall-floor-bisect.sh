#!/bin/sh
# ab-nt-overcall-floor-bisect.sh — the natural overcall of their 1NT: HCP floor
# 8 against floor 0, the authored advance on in both arms (the bisect
# scripts/ab-nt-natural-overcall-disclosed.sh reserved; docs/defensive-
# overcalls.md, docs/research-method-review.md § 4).
#
#   JOBS=96 BOARDS=393216 BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       DDSS_THREADS=96 setsid nohup scripts/idle-run.sh \
#       scripts/ab-nt-overcall-floor-bisect.sh ab-results/nt-overcall-floor-bisect \
#       >ab-results/nt-overcall-floor-bisect.log 2>&1 < /dev/null &
#
# Why.  The package (floor 8 + advance) against neither read loss | win under
# the disclosed SD leader (SEED_BASE 1792107984640740881): plain DD lost at
# `ew`, and the bucket trace put the whole gain on the advance while the
# floor's boards lost plain in all four cells.  That trace compares Pass with
# an advanced overcall, so it bounds the floor only loosely; this isolates it.
#
# Arms, one fresh SEED_BASE, `--filter-1nt --wide-1nt`, all four cells:
#   floor8 = shipped;  floor0 = `--ns-nt-overcall-hcp-floor 0` (advance on).
# The SD leader reads floor0 with `--off-ns-nt-overcall-hcp-floor 0`.
#
# PRE-REGISTERED 2026-10-16, before any number is read; floor8 − floor0 per
# cell, decision table on plain DD and PD:
#   - The floor stands if every cell is plain non-inferior or better with PD
#     not a loss.
#   - A plain loss in any cell with no PD and SD-PD win in that cell flags
#     the reversal `natural_overcall_hcp_floor = 0` (advance kept) for jdh8.
#   - Any other reading (e.g. loss | win again) is flagged, not applied.
#
# Resumable; SEED_BASE persists in $R/nt-overcall-floor-bisect.seed.  Iron
# rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-nt-overcall-floor-bisect.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for nt-overcall-floor-bisect)
log "=== nt-overcall-floor-bisect SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul vuls=$VULS"

FILTER='--filter-1nt --wide-1nt'
for v in $VULS; do
    # shellcheck disable=SC2086
    arm floor8 "$v" $FILTER
    # shellcheck disable=SC2086
    arm floor0 "$v" $FILTER --ns-nt-overcall-hcp-floor 0
    DENSITY=$(density_of "$R/floor0-$v")
    log "density $v: $DENSITY"
    gatepair floor8 floor0 "$v" theirs
    diffpair floor8 floor0 "$v"
    sddiff floor8 floor0 "$v" --off-ns-nt-overcall-hcp-floor 0
done

log "nt-overcall-floor-bisect done"
