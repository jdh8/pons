#!/bin/sh
# ab-landy-nt-vs-x.sh — §N1r step 0: `3NT` vs the values `X` over their Landy
# `(2♣)`, at ALL FOUR vulnerabilities.
#
#   JOBS=24 BOARDS=4608000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-nt-vs-x.sh ab-results/landy-nt-vs-x \
#       >ab-results/landy-nt-vs-x.log 2>&1 < /dev/null & disown
#
# §N1p measured `landy_notrump_no_major` (game hands with a 4+ major double
# instead of declaring `3NT`) a plain-DD loss at none and both.  Those two cells
# share one game:penalty ratio; the asymmetric cells do not.  `bba-gen` seats us
# North/South at table A and the diff reads table A only, so
#
#   -v ns   we vulnerable, they not — `3NT` = 600, down two doubled = 300
#   -v ew   they vulnerable, we not — `3NT` = 400, down two doubled = 500
#
# and the hypothesis is a colour flip at `ew`.  If it shows, the follow-up arm is
# a one-line `they_vulnerable() & !vulnerable()` gate on `no_major`.
#
# Arms: `base` = `main`, `nt` = `--ns-landy-notrump-no-major`.  Same sizing as
# every §N1 run.  Plain DD primary; PD is reported but §N1p showed it is the
# auto-double artifact here (the arm ADDS our doubles; PD doubles for `base` too).
#
# To reuse run 5's default-system dumps (code-identical to `main`: 6f66ae69..HEAD
# only flipped `nv_invite` on), seed the results dir before launching:
#   mkdir -p $R && cp ab-results/landy-strength5/landy-strength-residue.seed $R/landy-nt-vs-x.seed
#   ln -s ../landy-strength5/nv2-none $R/base-none; ln -s ../landy-strength5/nv2-both $R/base-both
#
# VERDICT 2026-09-22 (seed 1789977169, control ab8bc884, gates 0 foreign).
# IMPs/board, DD plain / DD PD / sd-lead plain / sd-lead PD:
#   ew   +0.0147 / +0.0127 / +0.0042 / +0.0023   win on all four
#   ns   -0.0107 / -0.0144 / -0.0206 / -0.0235   loss on all four
#   none +0.0007 / -0.0009 / -0.0108 / -0.0123   DD wash, sd loss
#   both +0.0052 / +0.0004 / -0.0054 / -0.0092   DD win, sd loss (package D's row)
# The flip showed.  Shipped default-on as `landy_notrump_no_major_favourable`
# (face-gated, byte-identical to `nt` at ew and to `base` elsewhere, so this
# run is its A/B).  Full write-up: docs/one-notrump-competitive.md §N1r step 0.
R=${1:?usage: ab-landy-nt-vs-x.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-nt-vs-x)
log "=== landy-nt-vs-x SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

# Asymmetric cells first: they are the new information.
for v in ew ns none both; do
    arm base "$v" --filter-landy
    arm nt   "$v" --filter-landy --ns-landy-notrump-no-major
    gatepair nt base "$v"
    diffpair nt base "$v"
done

for v in ew ns none both; do
    sddiff nt base "$v"
done

log "landy-nt-vs-x done"
