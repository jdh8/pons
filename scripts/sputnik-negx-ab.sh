#!/bin/sh
# sputnik-negx-ab.sh — four-arm negative-double A/B adding the Sputnik residual
# double (NegativeDoubleShape::Sputnik) alongside the both-majors default,
# free-bids, and Modern arms. Mirrors the p3bd-negx block of
# competitive-book-ab.sh: one SEED_BASE for the experiment (paired diffs need
# identical deals), arms strictly sequential, both scorers, both vulnerabilities.
#
#   setsid nohup scripts/idle-run.sh scripts/sputnik-negx-ab.sh \
#       ab-results/sputnik-negx >ab-results/sputnik-negx.log 2>&1 &
#
# Resumable: an arm dir / diff file that already exists is skipped; the
# SEED_BASE persists in $R/sputnik-negx.seed. Do NOT rebuild the binaries while
# this runs (bba-gen-parallel re-invokes cargo build; keep it a no-op).
R=${1:?usage: sputnik-negx-ab.sh RESULTS_DIR}
SHOW=3
. "$(dirname "$0")/ab-lib.sh"

exp=sputnik-negx
SEED_BASE=$(seed_for "$exp")
log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    arm "$exp-off" "$vul"
    arm "$exp-free" "$vul" --ns-free-bids
    arm "$exp-modern" "$vul" --ns-negative-double-shape modern
    arm "$exp-sputnik" "$vul" --ns-negative-double-shape sputnik
    diffpair "$exp-sputnik" "$exp-off" "$vul"    # ship gate (vs the default)
    diffpair "$exp-sputnik" "$exp-modern" "$vul" # shape isolation (both free-bid)
    diffpair "$exp-sputnik" "$exp-free" "$vul"   # shape vs plain free bids
    diffpair "$exp-modern" "$exp-off" "$vul"     # anchor to the prior verdict
done

log "campaign done"
