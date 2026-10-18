#!/bin/sh
# ab-nt-doubling-4cell.sh — the three 1NT-lane doubling knobs the review's § 6
# left owed, confirmed at the asymmetric cells (docs/research-method-review.md
# § 6 "Open").  The fourth, the accountant floor, deals unfiltered at its own
# size: scripts/ab-accountant-floor-4cell.sh.
#
# Why bba-gen and not the knobs' own harnesses.  `ab-lebensohl`,
# `ab-one-nt-runout`, `ab-accountant-floor` and `ab-landy` are seat-swap
# duplicates: the treated pair sits N/S at table A and E/W at table B under
# one absolute vulnerability.  At `--vulnerability ns` it is unfavourable at
# table A and favourable at table B, and every swing nets the two, so `ns`
# and `ew` measure the same mixture on different deals; the sign flip § 6 is
# about cancels inside each board.  bba-gen keeps us N/S against a fixed
# reference, so `ns` is unfavourable and `ew` favourable (measurement.md
# checklist 8).
#
# Arms, per cell, identical deals (`--filter-1nt --wide-1nt`):
#   default  american as shipped
#   runoff   --no-ns-one-nt-runout   `1NT (X)`: sit instead of escaping
#   trapoff  --no-ns-trap-pass       direct `3NT` on a too-good stopper
#   ppoff    --no-ns-penalty-pass    `1NT (2♣) X -`: answer, never sit
#
# PRE-REGISTERED (2026-10-18, before any number is read).  VULS="ns ew",
# fresh seed, BOARDS=2400000 accepted per arm per cell, results in
# ab-results/nt-doubling-4cell, density from the default arm, gate 0 foreign
# required (we open) for ppoff.  runoff and trapoff have no gate, because
# both knobs own auctions they opened: the runout anchors on a 1NT overcall
# too (`one_nt_anchor`, `(1♦) 1NT (X)`), and the trap pass is shared with
# the advance of partner's takeout double of their weak two (`(2♠) X - ?`).
# Both gates were dropped after they failed on exactly those auctions (trap
# pass in the preflight, 3 of 8; runout at ns, 2 of 1,093); the verdict rule
# itself is unchanged.
#   - All three change how often we sit in or convert to a doubled
#     contract, so plain DD arbitrates and PD is a double-blind column
#     (measurement.md's domain addendum).  The default stands in a cell
#     whose default-vs-off plain DD reads non-inferior or better.
#   - An off arm with 0 divergent boards in both cells is recorded as inert
#     against BBA (the trigger never fires there), not as confirmed.
#   - Any other reading is flagged for jdh8's decision, not applied.
#   - sd-lead (plain / SD-PD) is reported for each pair, the off arm read
#     through its own book (ab-dump-sd --off-no-ns-*), as a tie-break only.
#
#   JOBS=96 BOARDS=2400000 VULS="ns ew" DDSS_THREADS=96 \
#       BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen setsid nohup \
#       scripts/idle-run.sh scripts/ab-nt-doubling-4cell.sh \
#       ab-results/nt-doubling-4cell >ab-results/nt-doubling-4cell.log 2>&1 \
#       </dev/null &
#
# Resumable; SEED_BASE persists in $R/nt-doubling.seed.  Iron
# rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-nt-doubling-4cell.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for nt-doubling)
log "=== nt-doubling SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in $VULS; do
    arm default "$v" --filter-1nt --wide-1nt
    arm runoff  "$v" --filter-1nt --wide-1nt --no-ns-one-nt-runout
    arm trapoff "$v" --filter-1nt --wide-1nt --no-ns-trap-pass
    arm ppoff   "$v" --filter-1nt --wide-1nt --no-ns-penalty-pass
    DENSITY=$(density_of "$R/default-$v")
    gatepair default ppoff "$v"
    for off in runoff trapoff ppoff; do
        diffpair default "$off" "$v"
    done
    sddiff default runoff  "$v" --off-no-ns-one-nt-runout
    sddiff default trapoff "$v" --off-no-ns-trap-pass
    sddiff default ppoff   "$v" --off-no-ns-penalty-pass
done
log "nt-doubling done"
