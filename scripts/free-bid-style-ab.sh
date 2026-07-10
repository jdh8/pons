#!/bin/sh
# free-bid-style-ab.sh — Stage B of the school tournament: 2-level responses
# to their overcall (ledger rows P3e/P3f). Arms: forcing (shipped default —
# new suits forcing one round, answer_free_bid) / negative (classic NFB: 2-level
# new suits non-forcing 5-11 with a 6+ suit or a strong 5-carder, ALL strong
# long-suit hands start with the widened X, X-then-new-suit = game force) /
# transfer (Cachalot-style rotation of the non-jump 2-level slots; opener
# completes and declares — the right-siding thesis, DD-blind, sd decides).
# Pairs vs forcing with plain+pd, plus sd-lead 16 worlds with the ON arm's
# style disclosed (OFF arm = forcing = ab-dump-sd's default stance).
#
#   setsid nohup scripts/idle-run.sh scripts/free-bid-style-ab.sh \
#       ab-results/free-bid-style >ab-results/free-bid-style.log 2>&1 &
#
# Resumable: existing arm dirs / diff files are skipped; SEED_BASE persists in
# $R/free-bid-style.seed. Do NOT rebuild the binaries while this runs.
R=${1:?usage: free-bid-style-ab.sh RESULTS_DIR}
SHOW=3
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

exp=free-bid-style
SEED_BASE=$(seed_for "$exp")
log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    arm "$exp-forcing" "$vul"
    arm "$exp-negative" "$vul" --ns-free-bid-style negative
    arm "$exp-transfer" "$vul" --ns-free-bid-style transfer
    diffpair "$exp-negative" "$exp-forcing" "$vul"
    diffpair "$exp-transfer" "$exp-forcing" "$vul"
    sddiff "$exp-negative" "$exp-forcing" "$vul" --on-ns-free-bid-style negative
    sddiff "$exp-transfer" "$exp-forcing" "$vul" --on-ns-free-bid-style transfer
done

log "campaign done"
