#!/bin/sh
# ab-silent-cue-veto.sh — the silent-cue rail
# (`InstinctProfile::silent_cue_veto`) against plain american, vs BBA.
#
#   plain   american as shipped                                      (control)
#   on      american with our silent side's 2-/3-level cue of their suit on
#           8 HCP or fewer masked once both opponents have bid
#           (--ns-silent-cue-veto)
#
# Found by the R5 trace of the 2026-09-20 shipping-arm decompose (c3bb94a7):
# over their 2/1 `1♠ - 2♦` the v6 floor cues 2♠ on junk; the family is 1,376
# boards, -1,343 plain / -2,391 PD IMPs over 409.6k with BBA passing.  Only
# fires on boards they open, so the gate reads `theirs`.
#
#   BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-silent-cue-veto.sh ab-results/silent-cue-veto \
#       >ab-results/silent-cue-veto.log 2>&1 &
R=${1:?usage: ab-silent-cue-veto.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== silent cue veto start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm plain "$vul" --our-floor american
    arm on    "$vul" --our-floor american --ns-silent-cue-veto
    gatepair on plain "$vul" theirs
    diffpair on plain "$vul"
    sddiff on plain "$vul"
done
log "=== silent cue veto done"
