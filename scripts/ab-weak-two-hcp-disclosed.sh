#!/bin/sh
# ab-weak-two-hcp-disclosed.sh — re-measure the rejected raw-HCP weak-two
# gauge (`set_weak_two_hcp`, `ab-point-count --weak-two-hcp 5:10`) against the
# shipped `points(5..=10)` under the fixed self-play SD leader
# (docs/research-method-review.md § 4, "Open"; 430fac29;
# docs/archive/point-count-threshold-campaign.md, "Weak-two band — REJECTED").
#
#   cargo build --release --example ab-point-count     # before, never during
#   RAYON_NUM_THREADS=96 DDSS_THREADS=96 \
#       setsid nohup scripts/idle-run.sh scripts/ab-weak-two-hcp-disclosed.sh \
#       ab-results/weak-two-hcp-disclosed \
#       >ab-results/weak-two-hcp-disclosed.log 2>&1 < /dev/null &
#
# Why.  2026-07-15 rejected `hcp(5..=10)` on sd-lead alone: plain DD
# +0.0017 ±0.0017 NV / +0.0011 ±0.0023 vul, PD +0.0131 / +0.0099 (CIs clear),
# sd-lead −0.0045 ±0.0080 / −0.0018 ±0.0108 (50k boards/vul), read as "the
# marginal weak twos over-disclose to the blind lead".  That sd-lead leader
# read through a plain partnership, which decodes the declaring side's calls at
# `ReadingScope::Alerted` only: the unalerted weak two fell to the natural walk
# in both arms, so the band's disclosure was never priced.  Today's decision
# table (measurement.md, 2026-10-14) would also read the July plain+PD pair as
# `non-inferior | win` — a ship row — so the rejection stands on the SD column
# alone, and that column was the broken one.
#
# Arms not re-run: every other weak-two gauge of the campaign (CCCC matched and
# wide, CCCC floors, NLTC band and ceilings, the major-only HCP carve) failed
# on plain DD with a CI clear of zero in at least one vul (CCCC wide: NV
# −0.0020 ±0.0013), and the leader fix does not touch plain DD or PD.  Their
# verdicts did not lean on the SD column.
#
# Harness.  `ab-point-count` is a self-play seat-swap duplicate: table A seats
# the candidate N/S, table B E/W, so in every cell the candidate team plays one
# table at each vulnerability.  `ns` and `ew` are therefore MIRROR cells (each
# has the candidate unfavourable at one table and favourable at the other; a
# 90° rotation of random deals maps one onto the other), not separate
# conditions: a disagreement between them is noise.  All four cells are run
# anyway, so the worst-cell rule has its usual four inputs.  Each invocation
# prints plain DD, PD, sd-lead plain and sd-lead PD (SD-PD) with a `Verdict`
# line each, plus `--show 20` forensics (worst boards, first-divergence
# buckets).
#
# SIZING (timed 2026-10-16 in the worktree, RAYON_NUM_THREADS=16
# DDSS_THREADS=16 nice, box at load ~590/640):
#   * 40,000 boards, no --sd: 27.8 s wall, 400 s user, start-up 1.4 s
#     → bidding + live DD of the divergent boards ≈ 10 CPU-ms/board.
#   * 8,000 boards: 6.7 s without --sd, 30.2 s with (158 divergent)
#     → SD ≈ 1.94 CPU-s per divergent board (two leads × 16 worlds).
#   * Divergence 2.0–2.6% (July's 1M run: 2.35%), so per board
#     10 + 0.022 × 1940 ≈ 53 CPU-ms.
#   * COUNT=512,000 boards × 5 invocations (four cells + one legacy-reader
#     control) = 2.56M boards × 53 ms = 136k CPU-s; / 96 threads = 1,410 s
#     ≈ 24 min at linear scaling, ≈ 32 min at 75% efficiency.  Peak RSS ~2 GB.
#   * Expected 95% half-widths at 512k (from the 8k/40k runs, ×√(N0/N)):
#     plain DD ±0.0022, PD ±0.0025, SD-PD ±0.0025 IMPs/board.
#   * Instrument pilot (sizing seed 23456, sd seed 9, 8,000 boards, none; NOT
#     decision data and never pooled): the reader moves this lane — SD-PD
#     +53 IMPs disclosed vs −2 legacy on the same deals, plain SD +78 vs +36.
#     So the leader does read the weak-two band now; the sign is not evidence.
#
# PRE-REGISTERED 2026-10-16, before any number is read:
#   * One fresh SEED_BASE (date +%s%N) and one fresh sd world seed, minted on
#     first use into $R/seed and $R/sd-seed; deal generator v2, so no board is
#     shared with July (which used `24.pdd` rows anyway).  COUNT=512000 per
#     cell, VULS="none ns ew both", 16 worlds.  One seed, no extension: an
#     inconclusive cell stays inconclusive (a follow-up is a new
#     pre-registration on a new seed, never a top-up of this one).
#   * Population check: report each cell's divergent rate; outside 2.0–2.7%
#     (July 2.32–2.41%) means the population moved since July, and the run is a
#     new experiment on today's code, not a replication of July.
#   * Decision, per cell: the decision-table row on plain DD × PD `Verdict`
#     lines; the experiment's row is its worst cell.  The July rejection is
#     OVERTURNED (proposed flip to default-on, flagged for jdh8) iff every cell
#     reads a ship row (plain `win`/`non-inferior` AND PD `win`) AND SD-PD reads
#     no `loss` in any cell.  It STANDS iff some cell is plain `loss`, or SD-PD
#     reads `loss` in some cell.  Anything else (an `inconclusive` plain cell,
#     or ship rows with SD-PD `inconclusive` somewhere but no loss) is
#     INSUFFICIENT DATA: the knob stays off as a recorded trade-off, and the
#     ledger says the July rationale is unproven, not confirmed.
#   * The "over-disclose to the blind lead" rationale STANDS iff, in at least
#     one cell, SD-PD reads `loss` (upper bound < 0) AND the SD-PD mean sits
#     below that cell's PD mean (July's inverted signature: sd under both DD
#     brackets).  It FAILS if SD-PD reads `win` or `non-inferior` in all four
#     cells.  Otherwise it is UNPROVEN.  Plain SD (sd-lead plain) is quoted
#     beside SD-PD but decides nothing (measurement.md, "Plain SD is not an
#     arbiter").
#   * Reader control: the first cell is re-run with `--legacy-leader-reading`
#     (the July reader) on the same deals and sd seed.  Its plain DD and PD
#     lines must equal the disclosed run's to the IMP (the reader touches SD
#     only; a mismatch voids the comparison).  If its SD lines also equal the
#     disclosed run's, the leader does not price the band in this lane and the
#     SD columns are reported as uninformative about disclosure.  The SD-PD
#     shift (disclosed − legacy) is reported as a direction only; no CI is
#     available for that paired difference here.
#   * Forensics: `--show 20` buckets are read for the rationale (e.g. whether
#     the SD loss sits in `[] - → 2M`, the added weak twos) and never decide.
#   * NO DEFAULT CHANGES.  Nothing in src/ moves on this run; any flip, either
#     way, is flagged for jdh8 with a proposed reversible default.
#
# Resumable: a finished cell (non-empty output) is skipped; an interrupted one
# leaves *.partial and re-runs on the stored seeds.  Arms run sequentially.
# Iron rule: do NOT rebuild binaries while this runs.
set -eu
cd "$(dirname "$0")/.."
# VERDICT (2026-10-16, SEED_BASE 1792122226364649837, 512,000 boards/cell,
# divergent 2.05-2.26%): plain DD win in all four cells (none +0.0045 ±0.0025,
# ns +0.0056 ±0.0028), PD inconclusive in all four, SD-PD win (ns) /
# non-inferior (none) / inconclusive (ew, both), loss nowhere.  Legacy control
# matches plain and PD to the IMP.  Pre-registered branch: INSUFFICIENT DATA;
# the knob stays off as a recorded trade-off, the July rationale unproven.
#
R=${1:?usage: ab-weak-two-hcp-disclosed.sh RESULTS_DIR}
mkdir -p "$R"
SHA=$(git rev-parse --short HEAD)
COUNT=${COUNT:-512000}
VULS=${VULS:-none ns ew both}
BIN=target/release/examples/ab-point-count

[ -s "$R/seed" ] || date +%s%N >"$R/seed"
[ -s "$R/sd-seed" ] || date +%s%N >"$R/sd-seed"
SEED=$(cat "$R/seed")
SDSEED=$(cat "$R/sd-seed")

log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }

# run NAME ARGS... — write $R/NAME unless it already exists non-empty.
run() {
    out="$R/$1"
    shift
    if [ -s "$out" ]; then
        log "skip $out"
        return 0
    fi
    log "run $out :: $*"
    if "$BIN" "$@" >"$out.partial" 2>&1; then
        mv "$out.partial" "$out"
    else
        log "FAILED (exit $?) — kept $out.partial"
    fi
}

log "=== weak-two-hcp-disclosed SEED_BASE=$SEED sd-seed=$SDSEED sha=$SHA count=$COUNT vuls=$VULS rayon=${RAYON_NUM_THREADS:-unset} ddss=${DDSS_THREADS:-unset}"

ARGS="--weak-two-hcp 5:10 --count $COUNT --seed $SEED --sd --sd-seed $SDSEED --show 20"
for v in $VULS; do
    # shellcheck disable=SC2086
    run "hcp510.$v.txt" $ARGS --vulnerability "$v"
done
first=${VULS%% *}
# shellcheck disable=SC2086
run "hcp510.$first.legacy.txt" $ARGS --vulnerability "$first" --legacy-leader-reading

for f in "$R"/hcp510.*.txt; do
    log "--- $(basename "$f")"
    grep -E '^Divergent|^ *(plain DD|perfect defense):|^sd-lead|^Verdict|^leader reading' "$f" |
        tee -a "$R/log" >&2 || true
done
log "weak-two-hcp-disclosed done"
