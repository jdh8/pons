#!/bin/sh
# ab-woolsey-wide-2nt.sh — the strong both-minors hand inside our opt-in
# Woolsey (Multi-Landy) defense to their 1NT: `defense.woolsey_wide_unusual_2nt`
# against Woolsey as shipped (1NT-defense plan, step 2's last owed decision;
# docs/defensive-overcalls.md, the "Dropped" paragraph).
#
# Launch only after any A/B in flight finishes: sourcing ab-lib.sh rebuilds
# the shared release examples.
#
#   JOBS=96 BOARDS=102400 BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       DDSS_THREADS=96 setsid nohup scripts/idle-run.sh \
#       scripts/ab-woolsey-wide-2nt.sh ab-results/woolsey-wide-2nt \
#       >ab-results/woolsey-wide-2nt.log 2>&1 < /dev/null &
#
# Why.  Woolsey's X needs a four-card major or a minor one-suiter, its 2♣/2♦
# are artificial, and the unusual 2NT stops at 13 points, so a 5-5 in the
# minors above that passes their 1NT.  BBA's Unusual 2NT over 1NT is 9–25
# HCP.  jdh8 chose the route 2026-10-21: 2NT, its floor kept and its ceiling
# dropped; the advance is unchanged (the longer minor at the three level)
# and the strong hand bids again (floor).  Census: 5-5+ minors with 12+ HCP
# is about 1 hand in 300 unconditioned, fewer facing a 15–17 1NT, so about
# 0.2–0.4% of kept boards should diverge (200–400 per cell here).
#
# Arms, one fresh SEED_BASE, every arm `--isolate-defense --ns-notrump-defense
# woolsey --filter-1nt --wide-1nt` (BBA opens 1NT, we defend; table B is
# all-BBA), all four cells:
#   base = Woolsey as shipped (sit and tails on)
#   wide = base + `--ns-woolsey-wide-2nt`
# Diff wide − base with the isolation gate (`theirs`) and the SD leader
# reading both arms as Woolsey, the ON arm with the wide 2NT.
#
# PRE-REGISTERED 2026-10-21, before any number exists; wide − base per cell,
# the standard docs/measurement.md decision table, δ = 0.002 IMPs per random
# deal (`--density` converts), each scorer's verdict its worst cell:
#   0. Under 50 contract-divergent boards (the diff's `fired`) in any cell: insufficient
#      data; rerun into a new RESULTS_DIR with BOARDS doubled, read alone.
#   1. Ship into the Woolsey card (default on inside NotrumpDefense::Woolsey;
#      the natural default system is untouched) iff plain DD is non-inferior
#      or a win in every cell AND PD is a win.
#   2. A PD-only win is a doubling artifact; a plain win PD erases is
#      suspect: no ship either way.
#   3. Plain + PD both non-inferior: stays opt-in, flagged for jdh8 (it
#      completes the card; a recorded trade-off is his call).
#   4. Any plain-DD loss: trace the worst boards before calling it dead.  The
#      named suspects: the floor's rebid after the advancer's 3m signoff (the
#      strong hand passing, or bidding a failing game), and the advancer's
#      forced 3m opposite a hand that could have defended 1NT.
#   5. SD-PD is descriptive; it neither ships nor kills.
# Nothing is changed by this runner's verdict; the default is jdh8's call.
#
# Resumable; SEED_BASE persists in $R/woolsey-wide-2nt.seed.  Iron rule: do
# NOT rebuild binaries while this runs.
R=${1:?usage: ab-woolsey-wide-2nt.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for woolsey-wide-2nt)
log "=== woolsey-wide-2nt SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul vuls=$VULS"

BASE='--isolate-defense --ns-notrump-defense woolsey --filter-1nt --wide-1nt'
READ='--on-ns-notrump-defense woolsey --off-ns-notrump-defense woolsey'
for v in $VULS; do
    # shellcheck disable=SC2086
    arm base "$v" $BASE
    # shellcheck disable=SC2086
    arm wide "$v" $BASE --ns-woolsey-wide-2nt
    DENSITY=$(density_of "$R/base-$v")
    log "density $v: $DENSITY"
    gatepair wide base "$v" theirs
    diffpair wide base "$v"
    # shellcheck disable=SC2086
    sddiff wide base "$v" $READ --on-ns-woolsey-wide-2nt
done

log "woolsey-wide-2nt done"
