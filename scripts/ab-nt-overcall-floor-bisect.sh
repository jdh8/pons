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
# VERDICT (2026-10-16, code fded92a7, SEED_BASE 1792119766568383243, 393,216
# accepted boards/arm/vul, density 0.252942, gate 0 foreign, leader reading
# disclosed).  floor8 − floor0, IMPs/accepted board, plain / PD / SD-PD:
#   none  -0.0074 ±0.0015  -0.0051 ±0.0018  -0.0058 ±0.0019   (3257 fired)
#   ns    -0.0071 ±0.0017  -0.0044 ±0.0020  -0.0051 ±0.0020   (3234)
#   ew    -0.0076 ±0.0019  -0.0056 ±0.0022  -0.0066 ±0.0023   (3251)
#   both  -0.0076 ±0.0020  -0.0047 ±0.0024  -0.0060 ±0.0024   (3207)
# Loss on every scorer in every cell (plain SD too): the reversal condition
# is met.  FLAGGED for jdh8, nothing changed: proposed default
# `natural_overcall_hcp_floor = 0`, advance kept (docs/defensive-overcalls.md).
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
    arm floor8 "$v" $FILTER --ns-nt-overcall-hcp-floor 8
    # shellcheck disable=SC2086
    arm floor0 "$v" $FILTER --ns-nt-overcall-hcp-floor 0
    DENSITY=$(density_of "$R/floor0-$v")
    log "density $v: $DENSITY"
    gatepair floor8 floor0 "$v" theirs
    diffpair floor8 floor0 "$v"
    sddiff floor8 floor0 "$v" --on-ns-nt-overcall-hcp-floor 8 \
        --off-ns-nt-overcall-hcp-floor 0
done

log "nt-overcall-floor-bisect done"
