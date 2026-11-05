#!/bin/sh
# ab-xyz-2nt-invite.sh — the XYZ `2NT` invite range sweep, vs BBA, four cells,
# plain DD + perfect defense from one solve, plus SD-PD.  Arms are sequential
# and share one persistent fresh seed.
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-xyz-2nt-invite.sh ab-results/xyz-2nt-invite \
#       >ab-results/xyz-2nt-invite.log 2>&1 < /dev/null &
#
# Why.  The 2026-11-06 re-anchor (docs/bba-gap-campaign.md § "Current
# ranking (`07864122`, 2026-11-06)") put responder's XYZ `2♣` relay atop the
# Constructive / book / round-2 bucket, and almost all of it is
# `… 1z - 2♣ - 2♦ - 2NT`: 2,332 boards, −2,251 plain / −3,324 PD.  Two leaks:
# responder invites on a balanced 10 (−1,441 over 1,125 boards; BBA passes
# `1NT` on 90%), and opener declines with 13 facing 12 (−520 over 217).
#
# The sweep (docs/convention-tuning.md, sweep): two knobs, both arms against
# the shipped `base` (floor 10, accept 14).
#   f11     rebid.xyz_notrump_invite_floor = 11: after `1x - 1y - 1NT` a 10-count
#           with no suit invite passes `1NT` (suit invites at 10 still relay)
#   a13     rebid.xyz_notrump_accept = 13: opener bids `3NT` over the `2NT`
#           invite on 13+
#   f11a13  both
# Both knobs move readings (the relay's and the `1NT` pass), so sddiff reads
# each arm with its own values.
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; δ = 0.002 IMPs/board.  Read each cell's `fired` count first.
# Nothing is changed by this runner's verdict; the default is jdh8's call.
#
# READ 2026-11-06 (SEED_BASE 1793912710052222453, arm − base, IMPs/board ±95%,
# none / ns / ew / both; every isolation gate passed):
#   f11     plain +0.0016±6 +0.0020±8 +0.0015±6 +0.0018±8 (e-4)  PD +0.0027 +0.0039 +0.0027 +0.0037
#   a13     plain −0.0000±9 +0.0017±14 −0.0000±9 +0.0016±14      PD −0.0021 −0.0018 −0.0021 −0.0018
#   f11a13  plain +0.0021±10 +0.0042±15 +0.0021±10 +0.0040±15    PD +0.0019 +0.0037 +0.0019 +0.0035
#   SD-PD f11a13 +0.0026 +0.0051 +0.0026 +0.0047; fired ~250 / ~385 / ~565 per cell.
#   f11a13 wins 4/4 on every scorer → shipped as the default (floor 11,
#   accept 13); `base` now needs `--ns-xyz-notrump-invite-floor 10
#   --ns-xyz-notrump-accept 14` to re-run.
#
# Resumable; SEED_BASE persists in $R/xyz-2nt-invite.seed.  Iron rule: do NOT
# rebuild binaries while this runs.
R=${1:?usage: ab-xyz-2nt-invite.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for xyz-2nt-invite)

F11='--ns-xyz-notrump-invite-floor 11 --ns-xyz-notrump-accept 14'
A13='--ns-xyz-notrump-invite-floor 10 --ns-xyz-notrump-accept 13'

log "=== xyz-2nt-invite A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul" --ns-xyz-notrump-invite-floor 10 --ns-xyz-notrump-accept 14
    # shellcheck disable=SC2086
    arm f11 "$vul" $F11
    # shellcheck disable=SC2086
    arm a13 "$vul" $A13
    # shellcheck disable=SC2086
    arm f11a13 "$vul" --ns-xyz-notrump-invite-floor 11 --ns-xyz-notrump-accept 13
    for on in f11 a13 f11a13; do
        gatepair "$on" base "$vul"
        diffpair "$on" base "$vul"
        fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.$on.vs.base.$vul.plain.txt")
        log "pre-count $on vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    done
    B='--off-ns-xyz-notrump-invite-floor 10 --off-ns-xyz-notrump-accept 14'
    # shellcheck disable=SC2086
    sddiff f11 base "$vul" $B --on-ns-xyz-notrump-accept 14
    # shellcheck disable=SC2086
    sddiff a13 base "$vul" $B --on-ns-xyz-notrump-invite-floor 10
    # shellcheck disable=SC2086
    sddiff f11a13 base "$vul" $B
done
log "=== xyz-2nt-invite A/B done"
