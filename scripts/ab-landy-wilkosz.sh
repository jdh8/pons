#!/bin/sh
# ab-landy-wilkosz.sh — §N1s: a game-forcing Wilkosz `1NT (2♣) 3♦` (two
# five-card suits, at least one a major, `points(10..)`); opener names a
# three-card major, responder raises or retreats to `3NT`, opener finds the
# other major over the retreat.  Dead at favourable by face gate.
#
#   JOBS=24 BOARDS=4608000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-wilkosz.sh ab-results/landy-wilkosz \
#       >ab-results/landy-wilkosz.log 2>&1 < /dev/null & disown
#
# The census (`examples/probe-landy-wilkosz-oracle`, 2026-09-24, on the base
# dumps below), IMPs/board over the live `3NT`, plain / PD: +0.00061 /
# +0.00068 at none (0.075% of boards), +0.00058 / +0.00074 at both (0.056%);
# the per-board ceiling +0.0010.  Their double and raise of `3♦` are unpriced
# there — this run is their arbiter.
#
# Arms: `base` = `main` (375066af), `wilkosz` = the flag on.  The base arms
# REUSE round 2 of the splinter tails (seed 1789977169, `tails-*` = today's
# system at none/both; a 2,000-board prefix of shard 0 checked byte-identical
# 2026-09-24).  Seed the results dir before launching:
#   R=ab-results/landy-wilkosz; mkdir -p $R
#   echo 1789977169 > $R/landy-wilkosz.seed
#   ln -s ../landy-splinter-tails-r2/tails-none $R/base-none
#   ln -s ../landy-splinter-tails-r2/tails-both $R/base-both
#
# VERDICT 2026-09-24 (seed 1789977169, control 375066af + the knob, gates 0
# foreign, 4.608M bd/arm/vul).  IMPs/board, DD plain / DD PD / sd-lead plain /
# sd-lead PD:
#   none  +0.0005 ±0.0001 / +0.0007 ±0.0001 / +0.0003 ±0.0001 / +0.0004 ±0.0001   1,483 fired
#   both  +0.0004 ±0.0001 / +0.0006 ±0.0001 / +0.0002 ±0.0001 / +0.0003 ±0.0001     991 fired
# A win on all eight columns; shipped default-on as `competition.landy_wilkosz`
# — the flag is now `--no-ns-landy-wilkosz`, so a rerun's `base` arm needs it
# and the `wilkosz` arm is `main`.  Write-up: docs/archive/
# one-notrump-competitive-landy.md §N1s.
R=${1:?usage: ab-landy-wilkosz.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-wilkosz)
log "=== landy-wilkosz SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base    "$v" --filter-landy --no-ns-landy-wilkosz
    arm wilkosz "$v" --filter-landy
    gatepair wilkosz base "$v"
    diffpair wilkosz base "$v"
done

for v in none both; do
    sddiff wilkosz base "$v"
done

log "landy-wilkosz done"
