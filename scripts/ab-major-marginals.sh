#!/bin/sh
# ab-major-marginals.sh — #6 re-run of the two major-continuation marginals
# that shipped on ~+0.002 plain with no CI and a 99%-overlapping second seed:
# rebid.fourth_suit_forcing and response.limit_raise_acceptance.  The original
# ab-major-continuations baseline was a stripped, uncontested system; this one
# turns each knob off against the shipped default, vs BBA.
#
#   BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-major-marginals.sh ab-results/major-marginals \
#       >ab-results/major-marginals.log 2>&1 < /dev/null &
#
# One SEED_BASE for all three arms (they share the control); none and both
# vulnerabilities are one deal set (measurement.md checklist 5).  Resumable.
R=${1:?usage: ab-major-marginals.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for)
log "=== major-marginals start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for v in $VULS; do
    arm main "$v"
    arm fsf-off "$v" --no-ns-fourth-suit-forcing
    arm lra-off "$v" --no-ns-limit-raise-acceptance
done
# A positive diff means the knob helps.  The gates are report-only: a knob
# that changes our book also changes how we read their identical call (the
# mirror), so a foreign board here is the shipped routing, not a leak into
# their side.  Quote the gate files' foreign boards beside the headline.
for v in $VULS; do
    for off in fsf-off lra-off; do
        gatepair main "$off" "$v" || log "gate main vs $off ($v) FAILED, report-only; see gate file"
        diffpair main "$off" "$v"
    done
done
log "=== major-marginals done"
