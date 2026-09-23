#!/bin/sh
# ab-landy-splinter-tails.sh — §N1r row 1 tails: our calls when they act over
# the passed `3NT` answer to our Landy splinter (`1NT (2♣) 3♥/3♠ - 3NT - - …`).
# Over their `(X)` opener passes and responder runs to `5m` on a void in the
# splintered major, else sits; over their `(4M)` opener doubles, responder
# passes it (and doubles `(4M)` when the overcaller pulls the `(X)`).
#
#   JOBS=24 BOARDS=4608000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-splinter-tails.sh ab-results/landy-splinter-tails \
#       >ab-results/landy-splinter-tails.log 2>&1 < /dev/null & disown
#
# The tails census (`examples/probe-landy-splinter-tails`, 2026-09-23, on the
# base dumps below), IMPs per seat board over live, plain / PD: sitting `3NTx`
# +0.96 / +2.01 (none, 641 boards), +1.90 / +2.85 (both, 1,085) — the floor
# redoubled 116 / 279 of them; the void run over sitting +2.9 / +1.5 (none),
# +2.1 / +0.2 (both); doubling `4M` +2.5 to +3.5 plain in every bucket
# (≈ 4,750 boards at none, ~10 at both).  The `(4M)` half adds doubles, so
# plain DD arbitrates and PD is the double-blind column.
#
# Arms: `base` = `main` then, `tails` = the flag on.  The base arms
# REUSE arm 1's on-arm dumps (`ab-results/landy-splinter-rebids/rebids-*`,
# seed 1789977169 = main's default since 1abbb254).  Seed the results dir
# before launching:
#   R=ab-results/landy-splinter-tails; mkdir -p $R
#   cp ab-results/landy-splinter-rebids/landy-splinter-rebids.seed $R/landy-splinter-tails.seed
#   ln -s ../landy-splinter-rebids/rebids-none $R/base-none
#   ln -s ../landy-splinter-rebids/rebids-both $R/base-both
#
# Round 1 (results in ab-results/landy-splinter-tails) won all eight columns
# but left opener's seat over responder's `5m` run to the floor, which jumped
# to six of the other minor on 33 / 47 of 162 / 228 runs; opener's `Pass`@0
# was added and round 2 (ab-results/landy-splinter-tails-r2) is the verdict.
#
# VERDICT 2026-09-23, round 2 (seed 1789977169, control 5c3ee25c + the knob,
# gates 0 foreign, 4.608M bd/arm/vul).  IMPs/board, DD plain / DD PD /
# sd-lead plain / sd-lead PD:
#   none  +0.0030 ±0.0003 / +0.0024 ±0.0003 / +0.0009 ±0.0002 / +0.0003 ±0.0002   5,095 fired
#   both  +0.0006 ±0.0001 / +0.0007 ±0.0001 / +0.0003 ±0.0001 / +0.0004 ±0.0001     791 fired
# A win on all eight columns; shipped default-on as
# `competition.landy_splinter_tails` — the flag is now
# `--no-ns-landy-splinter-tails`, so a rerun's `base` arm needs it and the
# `tails` arm is `main`.  Write-up: docs/one-notrump-competitive.md §N1r tails.
R=${1:?usage: ab-landy-splinter-tails.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-splinter-tails)
log "=== landy-splinter-tails SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base  "$v" --filter-landy --no-ns-landy-splinter-tails
    arm tails "$v" --filter-landy
    gatepair tails base "$v"
    diffpair tails base "$v"
done

for v in none both; do
    sddiff tails base "$v"
done

log "landy-splinter-tails done"
