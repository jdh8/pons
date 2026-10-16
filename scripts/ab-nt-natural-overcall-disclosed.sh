#!/bin/sh
# ab-nt-natural-overcall-disclosed.sh — re-measure the shipped natural-overcall
# package over their 1NT (`defense.natural_overcall_hcp_floor` = 8 plus
# `defense.natural_overcall_advance_enabled`) under the fixed SD leader
# (docs/research-method-review.md § 4; docs/defensive-overcalls.md, "The A/B").
#
#   JOBS=96 BOARDS=393216 BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       DDSS_THREADS=96 RAYON_NUM_THREADS=96 VULS="none ns ew both" \
#       setsid nohup scripts/idle-run.sh scripts/ab-nt-natural-overcall-disclosed.sh \
#       ab-results/nt-natural-overcall-disclosed \
#       >ab-results/nt-natural-overcall-disclosed.log 2>&1 < /dev/null &
#
# Why.  The 2026-08-23 ship (scripts/ab-nt-natural-overcall.sh pack) was
# pre-registered on "plain-DD non-loss AND SD-PD win".  Its SD-PD column came
# from `ab-dump-sd` before 7c60e309, whose East–West leader read only our
# *alerted* calls: the unalerted `(1NT) 2x` overcall read as `points 8..37`
# in both arms, so the HCP floor's disclosure was never priced when we
# declared.  Fresh deals, current code, the disclosed reader.
#
# Arms (one SEED_BASE, every arm `--filter-1nt --wide-1nt` so all deal the same
# accepted set; `--wide-1nt` scans every 15–17 hand with no suit under two):
#   shipped  bare — floor 8 + the authored advance (the default since 08-23)
#   base     `--ns-nt-overcall-hcp-floor 0 --no-ns-nt-overcall-advance`
# ON = shipped, OFF = base; the SD leader reads OFF with
# `--off-ns-nt-overcall-hcp-floor 0 --off-no-ns-nt-overcall-advance`.
#
# PRE-REGISTERED 2026-10-16, before any number is read:
#   * Sizes: BOARDS=393216 accepted boards/arm/vul (96 shards × 4096), four
#     cells VULS="none ns ew both" (competitive treatment; ns/ew are new,
#     research-method-review § 6).  One seed; no extension.
#   * Density = accepted ÷ scanned from this log (density_of), passed to every
#     diff and sd report, so each Verdict line reads against δ / density.
#   * Gate: `probe-divergence --gate-opener theirs` 0 foreign in every cell.
#   * Decision: the measurement.md decision-table row from the worst cell of
#     plain DD and of PD.  The default stands on `non-inferior|win` or better.
#   * Original rationale (SD-PD win) stands iff the SD-PD Verdict is `win` in
#     every cell.  SD-PD non-inferior/inconclusive in a cell: the rationale is
#     weakened there and the default rests on plain+PD (recorded).  SD-PD
#     `loss` in any cell, or a decision row below ship: flagged for jdh8 with a
#     proposed reversible default, nothing changed in src/.
#   * Coverage: the lane needs their 1NT opening; a BBA 1NT on a hand outside
#     the wide 15–17 candidate (a 14-count upgrade, a singleton) is unscanned
#     and outside the verdict.
#
# VERDICT (2026-10-16, code 178f301b + this commit's harness edits, SEED_BASE
# 1792107984640740881, 393,216 accepted boards/arm/vul, density 0.253354,
# gate 0 foreign in all four cells; leader reading: disclosed).
# shipped − base, IMPs/accepted board ±95% CI (fired ≈ 1.5% of boards):
#           plain DD             PD                  sd-lead             SD-PD
#   none  −0.0017 ±0.0018 NI  +0.0094 ±0.0024 win  −0.0045 ±0.0019 loss  +0.0047 ±0.0023 win
#   ns    +0.0008 ±0.0021 NI  +0.0134 ±0.0028 win  −0.0020 ±0.0022 NI    +0.0090 ±0.0028 win
#   ew    −0.0035 ±0.0021 loss +0.0081 ±0.0027 win −0.0066 ±0.0023 loss  +0.0032 ±0.0027 win
#   both  +0.0003 ±0.0024 NI  +0.0133 ±0.0031 win  −0.0017 ±0.0025 NI    +0.0094 ±0.0030 win
# Decision row (worst cell): plain `loss` (ew) | PD `win` — "artifact of PD's
# synthetic X", below ship.  The ew loss is −0.0009 IMPs per random board
# (inside δ, but its upper bound is below 0, which the table reads as loss).
# SD-PD is `win` in every cell, so the original rationale's SD-PD half stands
# under the disclosed reader; its plain half fails in the new ew cell.
# Trace (`ab-dump-bucket --by node`, first differing call), plain / PD IMPs:
#   M1 slice `1NT ⟨- vs 2x⟩` (the floor removes the overcall):
#     none −2591/−601, ns −1661/+748, ew −3067/−1053, both −2118/+164
#   M2 slice `(1NT) 2x - ⟨…⟩` (the authored advance):
#     none +1748/+4769, ns +2685/+6255, ew +1569/+4837, both +2575/+6244
# The package's whole win is the advance; the HCP floor's slice loses plain
# in all four cells.  (The slice compares Pass with an overcall advanced by
# the floor, so it bounds M1 alone only loosely.)  FLAGGED for jdh8; nothing
# changed.  Proposed reversible default: keep both knobs until the reserved
# `bisect` (advance on, floor 0, vs shipped) runs on a fresh seed; if the
# floor's own A/B reads plain loss, `natural_overcall_hcp_floor = 0` with the
# advance kept on.
#
# Resumable; SEED_BASE persists in $R/nt-natural-overcall-disclosed.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-nt-natural-overcall-disclosed.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for nt-natural-overcall-disclosed)
log "=== nt-natural-overcall-disclosed SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul vuls=$VULS"

FILTER='--filter-1nt --wide-1nt'
for v in $VULS; do
    # shellcheck disable=SC2086
    arm shipped "$v" $FILTER
    # shellcheck disable=SC2086
    arm base "$v" $FILTER --ns-nt-overcall-hcp-floor 0 --no-ns-nt-overcall-advance
    DENSITY=$(density_of "$R/base-$v")
    log "density $v: $DENSITY"
    gatepair shipped base "$v" theirs
    diffpair shipped base "$v"
    sddiff shipped base "$v" --off-ns-nt-overcall-hcp-floor 0 --off-no-ns-nt-overcall-advance
done

log "nt-natural-overcall-disclosed done"
