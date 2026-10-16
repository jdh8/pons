#!/bin/sh
# ab-nt-overcall-floor-sweep.sh — the natural overcall of their 1NT: every live
# HCP floor against floor 0, the authored advance on in every arm (the sweep
# the bisect left open; docs/defensive-overcalls.md, docs/research-method-
# review.md § 4).
#
#   JOBS=96 BOARDS=393216 BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       DDSS_THREADS=96 setsid nohup scripts/idle-run.sh \
#       scripts/ab-nt-overcall-floor-sweep.sh ab-results/nt-overcall-floor-sweep \
#       >ab-results/nt-overcall-floor-sweep.log 2>&1 < /dev/null &
#
# Why.  The bisect (SEED_BASE 1792119766568383243) read floor 8 − floor 0 a
# loss on every scorer in every cell and proposed floor 0.  Its write-up asked
# whether an intermediate floor "4–6" should be swept first, but the rule is
# `len(t, 5..) & points(8..=14) & hcp(k..)` and `points` is raw HCP plus an
# upgrade capped at 2, so every overcall already holds 6+ HCP: floors 1–6 are
# floor 0 (checked: floor 6 and floor 0 bid 32,000 seeded boards identically,
# floor 7 does not).  The live floors are 7, 8 and 9; 9 is also the `k = 9`
# arm the 08-23 package left untested.
#
# Arms, one fresh SEED_BASE, `--filter-1nt --wide-1nt`, all four cells:
#   floor0 = `--ns-nt-overcall-hcp-floor 0` (the proposed default)
#   floor7 / floor9 = `--ns-nt-overcall-hcp-floor 7` / `9`
#   floor8 = `--ns-nt-overcall-hcp-floor 8` (shipped when this ran, bare);
#   against floor0 it replicates the bisect.
# The SD leader reads each arm with the matching `--{on,off}-ns-nt-overcall-
# hcp-floor`.  floor7 − floor8 is diffed too, descriptive only.
#
# PRE-REGISTERED 2026-10-16, before any number is read; floorK − floor0 per
# cell, decision table on plain DD and PD (δ = 0.002 IMPs/accepted board):
#   - floorK is a candidate if every cell is plain non-inferior or better
#     (lower bound above −δ) with PD not a loss.  Among candidates, prefer the
#     one with the most PD/SD-PD wins; a tie keeps floor 0.
#   - If no K is a candidate, floor 0 is confirmed against every live floor:
#     the flip `natural_overcall_hcp_floor = 0` (advance kept) stands proposed
#     for jdh8 with a second seed behind it.
#   - floor8 − floor0 reading a plain loss in 3+ cells replicates the bisect;
#     anything else is flagged as a seed disagreement before any proposal.
#   - Any other reading (e.g. loss | win) is flagged, not applied.
# Nothing is changed by this runner's verdict; the default is jdh8's call.
#
# VERDICT (2026-10-16, code edebcc80, SEED_BASE 1792143998575635198, 393,216
# accepted boards/arm/vul, density 0.253419, gate 0 foreign in every pair,
# leader reading disclosed).  floorK − floor0, IMPs/accepted board,
# plain / PD / SD-PD:
#   7 none -0.0013 ±0.0008  -0.0020 ±0.0010  -0.0011 ±0.0010   (823 fired)
#   7 ns   -0.0014 ±0.0009  -0.0023 ±0.0011  -0.0017 ±0.0010   (845)
#   7 ew   -0.0013 ±0.0010  -0.0021 ±0.0011  -0.0013 ±0.0012   (818)
#   7 both -0.0010 ±0.0010  -0.0022 ±0.0012  -0.0011 ±0.0012   (776)
#   8 none -0.0082 ±0.0016  -0.0052 ±0.0019  -0.0066 ±0.0019   (3227)
#   8 ns   -0.0074 ±0.0017  -0.0041 ±0.0021  -0.0050 ±0.0021   (3261)
#   8 ew   -0.0085 ±0.0019  -0.0062 ±0.0023  -0.0067 ±0.0023   (3249)
#   8 both -0.0073 ±0.0020  -0.0044 ±0.0024  -0.0068 ±0.0024   (3189)
#   9 none -0.0205 ±0.0023  -0.0087 ±0.0028  -0.0153 ±0.0028   (7712)
#   9 ns   -0.0155 ±0.0025  -0.0007 ±0.0031  -0.0081 ±0.0031   (7745)
#   9 ew   -0.0219 ±0.0028  -0.0101 ±0.0035  -0.0176 ±0.0035   (7745)
#   9 both -0.0167 ±0.0029  -0.0020 ±0.0037  -0.0113 ±0.0037   (7631)
# No floor is a candidate (7 loses PD in every cell, plain in three); floor 8
# replicates the bisect (plain loss in all four).  Floor 0 confirmed; jdh8 set
# the default to 0 (advance kept) the same day.
#
# Resumable; SEED_BASE persists in $R/nt-overcall-floor-sweep.seed.  Iron
# rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-nt-overcall-floor-sweep.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for nt-overcall-floor-sweep)
log "=== nt-overcall-floor-sweep SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul vuls=$VULS"

FILTER='--filter-1nt --wide-1nt'
for v in $VULS; do
    # shellcheck disable=SC2086
    arm floor0 "$v" $FILTER --ns-nt-overcall-hcp-floor 0
    # shellcheck disable=SC2086
    arm floor7 "$v" $FILTER --ns-nt-overcall-hcp-floor 7
    # shellcheck disable=SC2086
    arm floor8 "$v" $FILTER --ns-nt-overcall-hcp-floor 8
    # shellcheck disable=SC2086
    arm floor9 "$v" $FILTER --ns-nt-overcall-hcp-floor 9
    DENSITY=$(density_of "$R/floor0-$v")
    log "density $v: $DENSITY"
    for k in 7 8 9; do
        gatepair "floor$k" floor0 "$v" theirs
        diffpair "floor$k" floor0 "$v"
        sddiff "floor$k" floor0 "$v" --on-ns-nt-overcall-hcp-floor "$k" \
            --off-ns-nt-overcall-hcp-floor 0
    done
    diffpair floor7 floor8 "$v"
done

log "nt-overcall-floor-sweep done"
