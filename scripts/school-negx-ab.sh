#!/bin/sh
# school-negx-ab.sh — Stage A of the school tournament: 1-level competitive-
# response schools over their overcall (P3d′/P3d″ re-adjudication). The prior
# verdicts compared INCOMPLETE books — no school had opener answers to free
# bids; Fix 1 completed them (answer_free_bid + Cachalot 4d narrowing +
# Sputnik raise discipline). Arms: modern (shipped default, no flags) /
# cachalot / sputnik. Pairs vs modern with plain+pd, plus sd-lead 16 worlds —
# right-siding is DD-blind and is Cachalot's whole thesis, so sd is the
# arbiter. The ON arm's school is disclosed to the blind leader; the OFF arm
# (modern) is ab-dump-sd's default stance, correct by construction.
#
#   setsid nohup scripts/idle-run.sh scripts/school-negx-ab.sh \
#       ab-results/school-negx >ab-results/school-negx.log 2>&1 &
#
# Resumable: existing arm dirs / diff files are skipped; SEED_BASE persists in
# $R/school-negx.seed. Do NOT rebuild the binaries while this runs.
R=${1:?usage: school-negx-ab.sh RESULTS_DIR}
SHOW=3
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

exp=school-negx
SEED_BASE=$(seed_for "$exp")
log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    arm "$exp-modern" "$vul"
    arm "$exp-cachalot" "$vul" --ns-negative-double-shape cachalot
    arm "$exp-sputnik" "$vul" --ns-negative-double-shape sputnik
    diffpair "$exp-cachalot" "$exp-modern" "$vul"
    diffpair "$exp-sputnik" "$exp-modern" "$vul"
    sddiff "$exp-cachalot" "$exp-modern" "$vul" --on-ns-negative-double-shape cachalot
    sddiff "$exp-sputnik" "$exp-modern" "$vul" --on-ns-negative-double-shape sputnik
done

log "campaign done"
