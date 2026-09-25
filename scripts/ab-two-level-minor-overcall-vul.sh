#!/bin/sh
# ab-two-level-minor-overcall-vul.sh — A/B for the opt-in *vulnerable-only*
# tight 2-level minor overcall (`defense.two_level_minor_overcall_vul_tight`,
# docs/defensive-overcalls.md §O4). OFF arm = shipped default (2♣/2♦ overcall
# at the disciplined band everywhere); ON arm =
# --ns-two-level-minor-overcall-vul-tight (demand 15+ only when vulnerable;
# non-vul byte-identical by construction, so the none cell is the reading
# no-op check and `both` carries the verdict). This is the BBA guard of the
# BEN Phase 2 fix; the paired Tier-F run is
# scripts/ab-ben-two-level-minor-overcall-vul.sh on the same SEED_BASE.
# Both vulnerabilities, THREE scorers (plain + pd via ab-dump-diff, sd-lead
# via ab-dump-sd), arms strictly sequential, one shared SEED_BASE. Do NOT
# touch the codebase while it runs (bba-gen-parallel re-invokes cargo build;
# must stay a no-op).
#
#   BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-two-level-minor-overcall-vul.sh ab-results/two-level-minor-overcall-vul \
#       >ab-results/two-level-minor-overcall-vul.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: ab-two-level-minor-overcall-vul.sh RESULTS_DIR}
SHOW=4
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== two-level-minor-overcall-vul A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm on  "$vul" --ns-two-level-minor-overcall-vul-tight
    diffpair on off "$vul"
    sddiff on off "$vul" --on-ns-two-level-minor-overcall-vul-tight
done
log "=== two-level-minor-overcall-vul A/B done"
