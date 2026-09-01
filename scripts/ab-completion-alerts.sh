#!/bin/sh
# ab-completion-alerts.sh — the uniform completion-alert doctrine
# (docs/reader-retirement.md §the alert question; CHANGELOG 2026-08-14).
# `reading.completion_alerts` alerts every forced completion, transfer
# completion and conventional answer in one move: Jacoby/Texas completions and
# super-accepts, Stayman/Puppet/minor-transfer/European answers, Smolen and
# Rubensohl completions, the contested Jacoby tails, and the Lebensohl `3♣`
# (the retired `lebensohl_completion_alert`'s lone site).  Alerted, each
# decodes its own rule's projection and suppresses the natural walk — the
# advance-sohl `3♣` stops reading as a four-card club holding.
#
#   JOBS=32 BOARDS=204800 setsid nohup scripts/idle-run.sh scripts/ab-completion-alerts.sh \
#       ab-results/completion-alerts >ab-results/completion-alerts.log 2>&1 < /dev/null &
#
# UNFILTERED on purpose (the ab-landy-read.sh note): the sibling defect's
# fired population is the advance-sohl lanes (their weak two, our takeout X),
# which any 1NT filter throws away, and the family also moves the whole 1NT
# response structure — no single filter covers both populations.
#
# A reading knob is a bidding knob under the neural floor
# (docs/reading-drift-handoff.md): plain+PD decide from the decision table,
# sd the tie-breaker.
#
# Resumable: an existing arm dir or diff file is skipped, and SEED_BASE
# persists in $R/completion-alerts.seed.  Iron rule: do NOT rebuild binaries
# while this runs.
R=${1:?usage: ab-completion-alerts.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for completion-alerts)
log "=== completion-alerts SEED_BASE=$SEED_BASE sha=$SHA"

# Shipped default-on 2026-08-14 (pooled 3 seeds; vul plain/PD CI-clear): the
# off arm now needs the explicit `false`.
for v in none both; do
    arm alerts-off "$v" --ns-completion-alerts false
    arm alerts-on "$v" --ns-completion-alerts true
    diffpair alerts-on alerts-off "$v"
    sddiff alerts-on alerts-off "$v"
done

log "completion-alerts done"
