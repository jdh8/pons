#!/bin/sh
# ab-landy-splinter-stopper.sh — §N1r row 1 arm 2: opener's `3NT` over our
# Landy both-minors splinter (`1NT (2♣) 3♥/3♠ -`) needs a DOUBLE stopper in
# the short major (two of A-K-Q) or no four-card minor; a single stopper with
# a four-card minor answers `4m` (which responder now raises to `5m`, arm 1).
#
#   JOBS=24 BOARDS=4608000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-splinter-stopper.sh ab-results/landy-splinter-stopper \
#       >ab-results/landy-splinter-stopper.log 2>&1 < /dev/null & disown
#
# The row 1 census (`examples/probe-landy-splinter-oracle`, 2026-09-23)
# reversed the "wastage" idea's sign: two top honours in the short major are
# where `3NT` is best; the one bucket where `5m` beats `3NT` is a single
# stopper with a four-card minor — +0.50 / +0.93 plain / PD IMPs per seat
# board at none (15,828 boards), +0.69 / +1.18 at both (11,755), ≈ +0.002 /
# +0.003 per board.  Pre-registered falsifier: the census cut `stop1` on
# A-K-Q count, so `Jxxx`/`Qxx` and a bare `A` sit in the same class as `Kx`;
# if the arm reads a wash the class is too coarse, not the idea wrong, and the
# next cut is by exact stopper type.
#
# Arms: `base` = `main` (arm 1 on), `stopper` = `--ns-landy-splinter-stopper`.
# The base arms REUSE arm 1's on-arm dumps
# (`ab-results/landy-splinter-rebids/rebids-{none,both}`, seed 1789977169,
# generated at d0f7222c + the knob = main's default since 1abbb254).  Seed the
# results dir before launching:
#   R=ab-results/landy-splinter-stopper; mkdir -p $R
#   cp ab-results/landy-splinter-rebids/landy-splinter-rebids.seed $R/landy-splinter-stopper.seed
#   ln -s ../landy-splinter-rebids/rebids-none $R/base-none
#   ln -s ../landy-splinter-rebids/rebids-both $R/base-both
# Plain DD primary, PD the sharper scorer (a constructive contract boundary:
# game placement in our own hands), sd-lead the tie-break.
#
# VERDICT 2026-09-23 (seed 1789977169, control 1abbb254, gates 0 foreign,
# 4.608M bd/arm/vul).  IMPs/board, DD plain / DD PD / sd-lead plain / sd-lead PD:
#   none  +0.0020 ±0.0003 / +0.0022 ±0.0004 / -0.0003 ±0.0003 / -0.0004 ±0.0003   14,928 fired
#   both  +0.0019 ±0.0003 / +0.0029 ±0.0004 / -0.0008 ±0.0003 / -0.0002 ±0.0004   11,495 fired
# NON-WIN: the DD win is erased on sd-lead at both colours — the lead seam
# against the 3NT the knob stops declaring (§N1-lia D's mechanism).  The knob
# stays opt-in, default off; the pre-registered falsifier (the A-K-Q class is
# too coarse) is the refinement if the seat is reopened.  Write-up:
# docs/one-notrump-competitive.md §N1r arm 2.
R=${1:?usage: ab-landy-splinter-stopper.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-splinter-stopper)
log "=== landy-splinter-stopper SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base    "$v" --filter-landy
    arm stopper "$v" --filter-landy --ns-landy-splinter-stopper
    gatepair stopper base "$v"
    diffpair stopper base "$v"
done

for v in none both; do
    sddiff stopper base "$v"
done

log "landy-splinter-stopper done"
