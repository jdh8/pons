#!/bin/sh
# splinter-doubled-ab.sh — two-arm A/B for the systems-on-over-doubled-splinter
# rebase (set_splinter_doubled / bba-gen --ns-splinter-doubled). Mirrors
# sputnik-negx-ab.sh: one SEED_BASE for the experiment (paired diffs need
# identical deals), arms strictly sequential, both scorers, both vulnerabilities.
#
#   setsid nohup scripts/idle-run.sh scripts/splinter-doubled-ab.sh \
#       ab-results/splinter-doubled >ab-results/splinter-doubled.log 2>&1 &
#
# The knob only changes opener's rebid AFTER their double of our splinter, so the
# arms are byte-identical up through the double — no --advertise needed, and the
# divergent set is exactly the fired set (read IMPs/fired straight off).
#
# Resumable: an arm dir / diff file that already exists is skipped; the SEED_BASE
# persists in $R/splinter-doubled.seed. Do NOT rebuild the binaries while this
# runs (bba-gen-parallel re-invokes cargo build; keep it a no-op).
R=${1:?usage: splinter-doubled-ab.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"

exp=splinter-doubled
SEED_BASE=$(seed_for "$exp")
log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    arm "$exp-off" "$vul"
    arm "$exp-on" "$vul" --ns-splinter-doubled
    diffpair "$exp-on" "$exp-off" "$vul" # ship gate (vs the byte-identical default)
done

log "campaign done"
