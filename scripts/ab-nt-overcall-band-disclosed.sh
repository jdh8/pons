#!/bin/sh
# ab-nt-overcall-band-disclosed.sh — re-measure the `8:14` points band of our
# natural two-level overcall of their 1NT (`ReadingProfile::
# natural_overcall_points`, `bba-gen --ns-overcall LO:HI`) against its two
# tightenings under the fixed SD leader (docs/research-method-review.md § 4;
# docs/archive/point-count-threshold-campaign.md, "SD-PD re-adjudication";
# docs/convention-tuning.md).
#
#   JOBS=96 BOARDS=294912 BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       DDSS_THREADS=96 RAYON_NUM_THREADS=96 VULS="none ns ew both" \
#       setsid nohup scripts/idle-run.sh scripts/ab-nt-overcall-band-disclosed.sh \
#       ab-results/nt-overcall-band-disclosed \
#       >ab-results/nt-overcall-band-disclosed.log 2>&1 < /dev/null &
#
# Why.  The 2026-07-26 verdict ("NV `8:14` holds, SD-PD a wash; vul
# undetermined, leaning tighten") came from `ab-dump-sd` rescoring the
# 2026-07-15 screen dumps under the pre-7c60e309 leader, which read only our
# alerted calls: the unalerted overcall read as `points 8..37` in every arm,
# and `--on-ns-overcall 20:37` reproduced the run to the last IMP.  That SD
# column never priced the band's disclosure when we declare.  Fresh deals,
# current code (the HCP floor of 8 and the authored advance shipped since,
# so every arm carries them), the disclosed reader.
#
# Arms (one SEED_BASE, every arm `--filter-1nt --wide-1nt`, same accepted set):
#   base  `8:14` (shipped)     t9  `--ns-overcall 9:14`     t10 `--ns-overcall 10:14`
# ON = the tightening, OFF = base; the leader reads each arm's band
# (`--on-ns-overcall` / `--off-ns-overcall 8:14`).
#
# PRE-REGISTERED 2026-10-16, before any number is read:
#   * Sizes: BOARDS=294912 accepted boards/arm/vul (96 shards × 3072), four
#     cells VULS="none ns ew both" (we sit N/S: `ns` = we vul, `ew` = we NV).
#     (Shrunk from 393216 before launch: two SD pairs per cell at ~7 min per
#     7.7k fired boards would run past an hour.)  One seed; no extension.  Density = accepted ÷ scanned (density_of),
#     passed to every report.
#   * Gate: `probe-divergence --gate-opener theirs`, 0 foreign in every cell.
#   * Decision, per cell and candidate: the decision-table row on plain DD and
#     PD.  `8:14` holds in a cell unless a candidate reads a ship row there
#     (plain `non-inferior` or `win` AND PD `win`); such a cell is flagged for
#     jdh8 (the band is one value for every vulnerability, so acting on a
#     vul-only tighten needs a vul-split band first).  Nothing changes in src/.
#   * The 07-26 SD-PD findings, re-tested:
#       - NV half ("SD-PD a dead wash at both candidates, `8:14` holds"):
#         stands iff neither candidate's SD-PD Verdict is `win` at none or ew.
#       - Vul half ("leaning tighten": PD and SD-PD both CI-clear positive):
#         stands iff a candidate's PD and SD-PD Verdicts are both `win` at
#         both and ns.
#   * Disclosure check (measurement.md: verify a harness moves before trusting
#     it): t9 vs base at the first cell is rescored with the absurd
#     `--on-ns-overcall 20:37`.  If its Delta lines equal the honest report's,
#     the SD columns do not price the band and are reported as such.
#   * Coverage: the lane needs their 1NT opening; a BBA 1NT outside the wide
#     15–17 candidate is unscanned and outside the verdict.
#
# VERDICT (2026-10-16, code 178f301b + the harness edits of 32dc6fd4,
# SEED_BASE 1792110704895967184, 294,912 accepted boards/arm/vul, density
# 0.252979, gate 0 foreign in all eight pairs, leader reading disclosed).
# tightening − 8:14, IMPs/accepted board ±95% CI:
#           plain DD              PD                   sd-lead          SD-PD
#   t9  none −0.0044 ±0.0013 loss +0.0009 ±0.0017 NI   −0.0073 ±0.0014  −0.0030 ±0.0017 loss
#   t9  ns   −0.0017 ±0.0015 loss +0.0049 ±0.0020 win  −0.0044 ±0.0016  +0.0014 ±0.0020 NI
#   t9  ew   −0.0038 ±0.0017 loss +0.0012 ±0.0021 NI   −0.0066 ±0.0018  −0.0027 ±0.0021 loss
#   t9  both −0.0014 ±0.0018 NI   +0.0047 ±0.0023 win  −0.0052 ±0.0019  +0.0002 ±0.0023 NI
#   t10 none −0.0154 ±0.0022 loss +0.0018 ±0.0029 NI   −0.0239 ±0.0025  −0.0100 ±0.0029 loss
#   t10 ns   −0.0081 ±0.0025 loss +0.0123 ±0.0033 win  −0.0167 ±0.0026  +0.0014 ±0.0033 NI
#   t10 ew   −0.0145 ±0.0028 loss +0.0027 ±0.0036 NI   −0.0228 ±0.0031  −0.0089 ±0.0037 loss
#   t10 both −0.0082 ±0.0030 loss +0.0117 ±0.0039 win  −0.0158 ±0.0032  +0.0020 ±0.0038 NI
# Decision: `8:14` holds in seven of eight candidate-cells; the exception is
# t9 at `both`, `non-inferior | win` (a ship row) — FLAGGED, with ns (also
# we vul) a plain loss.  NV half of 07-26: stands, and harder — SD-PD is now
# a loss for both tightenings at none and ew.  Vul half ("leaning tighten"):
# does NOT stand — PD wins at ns/both but SD-PD is only non-inferior there.
# Disclosure check: `--on-ns-overcall 20:37` moved t9 none from SD-PD
# −0.0030 / plain SD −0.0073 to −0.0034 / −0.0077 (the leader prices the band).
# Proposed reversible default: keep `8:14` (a vul-split band does not exist
# and one cell of one seed does not justify building it).
#
# Resumable; SEED_BASE persists in $R/nt-overcall-band-disclosed.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-nt-overcall-band-disclosed.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for nt-overcall-band-disclosed)
log "=== nt-overcall-band-disclosed SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul vuls=$VULS"

FILTER='--filter-1nt --wide-1nt'
first=1
for v in $VULS; do
    # shellcheck disable=SC2086
    arm base "$v" $FILTER
    # shellcheck disable=SC2086
    arm t9 "$v" $FILTER --ns-overcall 9:14
    # shellcheck disable=SC2086
    arm t10 "$v" $FILTER --ns-overcall 10:14
    DENSITY=$(density_of "$R/base-$v")
    log "density $v: $DENSITY"
    for t in 9 10; do
        gatepair "t$t" base "$v" theirs
        diffpair "t$t" base "$v"
        sddiff "t$t" base "$v" --on-ns-overcall "$t:14" --off-ns-overcall 8:14
    done
    if [ "$first" = 1 ]; then
        first=0
        out="$R/sd.t9.vs.base.$v.absurd-20-37.txt"
        if ! grep -q '^Delta' "$out" 2>/dev/null; then
            log "disclosure check: t9 vs base ($v) read as 20:37"
            "$SD" "$R/t9-$v" "$R/base-$v" -v "$v" --sd-worlds 16 --show 0 \
                --density "$DENSITY" --on-ns-overcall 20:37 --off-ns-overcall 8:14 \
                >"$out" 2>&1
        fi
    fi
done

log "nt-overcall-band-disclosed done"
