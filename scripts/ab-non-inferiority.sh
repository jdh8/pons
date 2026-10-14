#!/bin/sh
# ab-non-inferiority.sh — the issue #7 re-measures (docs/research-method-review.md
# § "Issue #7: non-inferiority inventory").  Five shipped defaults whose plain-DD
# lower bound reached −δ (δ = 0.002 IMPs/board) or was never recorded, each
# re-run vs BBA on fresh deals at a board count declared here, before any data:
#
#   experiment              control (shipped)        candidate arm
#   k8-floor                K = 8 logit-mean floor   K = 4 (seeds 1–4, american-file)
#   push-natural-only       push natural-only        --no-ns-competitive-push-natural-only
#   longer-major-response   longer-major reading     --no-ns-longer-major-response
#   forcing-nt-two-suiter   15–17 two-suiter rebids  --no-ns-forcing-nt-two-suiter
#   forcing-nt-jump-shifts  natural jump shifts      --ns-meckstroth-adjunct (what they replaced)
#
# Two repairs the traces of those re-measures proposed, run only when named; their
# diffs are `repair vs shipped` (positive = the repair gains):
#
#   reverse-heart-game      4♥ on a doubleton over 1♥ - 1NT - 2♠   --ns-forcing-nt-reverse-heart-game
#   xyz-second-major-first  5♠4♥ shows 2♥ before 2♠ after 1m - 1♠ - 1NT   --ns-xyz-second-major-first
#
# Every diff is `shipped vs <arm>`: positive = the shipped default gains.  The
# verdict is ab-dump-diff's Verdict line, worst vulnerability cell, on plain DD
# and PD; SD-PD is reported for the four knobs (the k8 floor diverges on too
# many boards to solve single-dummy at this size).
#
# Pre-declared sizes (boards/arm/vul): k8-floor 1,638,400 (8 × 204,800: its
# 409,600-board half-widths were ±0.0039 / ±0.0045); the four knobs 1,228,800.
#
#   SKIP_BUILD=1 setsid nohup scripts/idle-run.sh scripts/ab-non-inferiority.sh \
#       ab-results/non-inferiority >ab-results/non-inferiority.log 2>&1 < /dev/null &
#
# No experiment named: runs all five, sequentially, each in RESULTS_ROOT/<name>
# with its own fresh SEED_BASE.  Resumable.
ROOT=${1:?usage: ab-non-inferiority.sh RESULTS_ROOT [EXPERIMENT]}
if [ $# -eq 1 ]; then
    for e in k8-floor push-natural-only longer-major-response \
        forcing-nt-two-suiter forcing-nt-jump-shifts; do
        sh "$0" "$ROOT" "$e" || exit
    done
    exit 0
fi
E=$2
R=$ROOT/$E
case $E in
k8-floor) BOARDS=1638400 ;;
*) BOARDS=1228800 ;;
esac
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)
# The tree may be dirty; keep what was measured beside the seed.
[ -s "$R/tree.patch" ] || git diff HEAD >"$R/tree.patch"

log "=== non-inferiority $E start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm shipped "$vul"
    case $E in
    k8-floor)
        W=src/bidding/weights
        PONS_FLOOR_WEIGHTS=$W/american_bba_v6.f32,$W/american_bba_v6_seed2.f32,$W/american_bba_v6_seed3.f32,$W/american_bba_v6_seed4.f32
        export PONS_FLOOR_WEIGHTS
        arm k4 "$vul" --our-floor american-file
        diffpair shipped k4 "$vul"
        ;;
    push-natural-only)
        arm off "$vul" --no-ns-competitive-push-natural-only
        diffpair shipped off "$vul"
        sddiff shipped off "$vul"
        ;;
    longer-major-response)
        arm off "$vul" --no-ns-longer-major-response
        diffpair shipped off "$vul"
        sddiff shipped off "$vul"
        ;;
    forcing-nt-two-suiter)
        arm off "$vul" --no-ns-forcing-nt-two-suiter
        diffpair shipped off "$vul"
        sddiff shipped off "$vul"
        ;;
    forcing-nt-jump-shifts)
        arm meckstroth "$vul" --ns-meckstroth-adjunct
        diffpair shipped meckstroth "$vul"
        sddiff shipped meckstroth "$vul"
        ;;
    reverse-heart-game)
        arm repair "$vul" --ns-forcing-nt-reverse-heart-game
        diffpair repair shipped "$vul"
        sddiff repair shipped "$vul"
        ;;
    xyz-second-major-first)
        arm repair "$vul" --ns-xyz-second-major-first
        diffpair repair shipped "$vul"
        sddiff repair shipped "$vul"
        ;;
    *) echo "ab-non-inferiority: unknown experiment $E" >&2; exit 2 ;;
    esac
done
log "=== non-inferiority $E done"
