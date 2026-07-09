#!/bin/sh
# passed-hand-overcall-ab.sh — marginal A/B for the passed-hand 2-level overcall
# carve-out atop the shipped strict+discipline default.
#
#   --ns-passed-hand-overcall  lets a *passed hand* take the disciplined 2-level
#       overcall lighter (9+ not the opening 11+): it cannot hold opening values,
#       so the 11+ floor would all but forbid the safe light overcall.  1-level
#       untouched.  Off by default.
#
# base = bare bba-gen (the shipped strict+discipline system); carve = base + the
# knob.  Fresh SEED_BASE for the whole experiment (paired diffs need identical
# deals), both vuls, both scorers, arms sequential.  See docs/measurement.md.
#
#   setsid nohup scripts/idle-run.sh scripts/passed-hand-overcall-ab.sh \
#       ab-results/passed-hand-overcall >ab-results/passed-hand-overcall.log 2>&1 &
#
# Resumable: an existing arm dir is skipped; SEED_BASE persists in $R/seed. Do
# NOT touch the codebase while this runs (bba-gen-parallel re-invokes cargo
# build; it must stay a no-op).
R=${1:?usage: passed-hand-overcall-ab.sh RESULTS_DIR}
SHOW=3
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "passed-hand-overcall A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    arm base  "$vul"                                # shipped strict+discipline
    arm carve "$vul" --ns-passed-hand-overcall      # + passed-hand 2-level 9+
    diffpair carve base "$vul"
done

log "passed-hand-overcall A/B done"
