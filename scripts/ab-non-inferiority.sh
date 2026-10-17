#!/bin/sh
# ab-non-inferiority.sh — the issue #7 re-measures
# (docs/archive/research-method-review-closed.md § "Issue #7: non-inferiority
# inventory").  Five shipped defaults whose plain-DD
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
#   xyz-second-major-first  5♠4♥ shows 2♥ before 2♠ after 1m - 1♠ - 1NT   --no-ns-xyz-second-major-first
#
#   xyz-invite-spade-preference  opener's 2♠ preference over the invitational
#                           1m - 1♠ - 1NT - 2♣ - 2♦ - 2♥     --no-ns-xyz-invite-spade-preference
#   xyz-weak-spade-preference  3-3 corrects the weak 1m - 1♠ - 1NT - 2♥
#                           to 2♠, responder passes (#10)     --no-ns-xyz-weak-spade-preference
#
# xyz-second-major-first shipped default-on on its run (2026-10-14), so its arm is
# now the off-switch and the diff reads `shipped vs off`; the recorded run, made
# before the flip, is `repair vs shipped` with the same sign.  Likewise
# xyz-invite-spade-preference (shipped 2026-10-15, results root
# ab-results/xyz-preference) and xyz-weak-spade-preference (shipped 2026-10-16,
# results root ab-results/xyz-preference-r2; the first run, without responder's
# pass, is superseded).
#
# reverse-levers (2026-10-15): the two-suiter trace's two remaining buckets, on
# opener's 1♥ - 1NT - 2♠ reverse, in one five-arm run on one SEED_BASE —
#
#   five    exactly five hearts (the 6-4 rebids 2♥)   --ns-forcing-nt-reverse-five-hearts
#   floor   15+ HCP as well as 15–17 points           --ns-forcing-nt-reverse-hcp-floor
#   both    the two together
#   off     no two-suiter rebids at all               --no-ns-forcing-nt-two-suiter
#
# Each lever is diffed against shipped (does it repair?) and against off (does the
# repaired reverse beat no reverse?), plus `shipped vs off` as the replication.
#
# Both levers shipped default-on on that run (2026-10-15), so the arms below are now
# spelled with the off-switches: `shipped` is the repaired reverse (= the recorded
# `both`), `old` is the unrepaired one (= the recorded `shipped`).  Same arms, same
# diffs, renamed.
#
# The status-unknown rows (2026-10-15): shipped defaults whose plain-DD CI was
# never recorded and whose dumps are gone, each against the alternative its own
# A/B shipped over, at the same declared 1,228,800 boards/arm/vul —
#
#   competitive-4333   flat 4-3-3-3 never cue-Staymans   --ns-competitive-4333 allow
#   takeout-support    12+ X needs every unbid suit      --ns-takeout-support off
#   diamond-transfer   our contested 2NT ♦ transfer      --no-ns-comp-over-diamond-transfer
#   point-scale        PointCount (linearised upgrade)   --ns-point-scale rule-of-n-floored
#
# point-scale covers two rows: PointCount and the linearised upgrade that won it
# back from rule of N+8 are one comparison (the cliff upgrade is gone).  The RKCB
# queen ask has no off arm to run (its knob was deleted: BBA always relays).
# penalty-no-pull ran too (2026-10-15) and moved 0 of 2,457,600 boards; its knob
# and `--ns-allow-pull` were removed, so the case is gone.
#
# diamond-captain (2026-10-15): the diamond-transfer repair, three arms on one
# SEED_BASE — captain (opener's 3NT rung gone over 1NT - 2NT (3♦/3♥/3♠), responder's
# answer to the penalty X authored; --ns-diamond-transfer-captain), shipped, and
# the package off.  Diffs are `captain vs shipped` and `captain vs off` (positive =
# captain gains).  The knob flips on if captain is non-inferior | win over shipped
# and not behind off; the package turns off if off beats both.  The first run
# (results root ab-results/issue7-unknown) lost to off on PD through a hole:
# opener's answer to responder's 4♦ pull was the floor's 4♠ on all 44 boards.
# The re-run (root ab-results/issue7-unknown-r2, fresh SEED_BASE) has opener pass.
#
# The captain shipped default-on on the re-run (2026-10-15), so the arms below are
# now spelled with the off-switch: `shipped` is the captain (= the recorded
# `captain`), `old` is the 3NT-rung table (= the recorded `shipped`).  Same arms,
# same diffs, renamed.
#
# Every other diff is `shipped vs <arm>`: positive = the shipped default gains.  The
# verdict is ab-dump-diff's Verdict line, worst vulnerability cell, on plain DD
# and PD; SD-PD is reported for the four knobs (the k8 floor diverges on too
# many boards to solve single-dummy at this size).
#
# Pre-declared sizes (boards/arm/vul): k8-floor 1,638,400 (8 × 204,800: its
# 409,600-board half-widths were ±0.0039 / ±0.0045); the four knobs 1,228,800,
# except point-scale 5,120,000: a 2,000-board pilot (2026-10-15) moved our
# auction on ~14.5% of boards at σ ≈ 2.3 IMPs/board, so a ±0.002 half-width
# needs ~5.1M.  point-scale skips SD at that size (plain DD + PD decide).
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
point-scale) BOARDS=5120000 ;;
*) BOARDS=1228800 ;;
esac
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)
# The tree may be dirty; keep what was measured beside the seed.
[ -s "$R/tree.patch" ] || git diff HEAD >"$R/tree.patch"

log "=== non-inferiority $E start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in $VULS; do
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
        arm off "$vul" --no-ns-xyz-second-major-first
        diffpair shipped off "$vul"
        sddiff shipped off "$vul"
        ;;
    xyz-weak-spade-preference)
        arm off "$vul" --no-ns-xyz-weak-spade-preference
        diffpair shipped off "$vul"
        sddiff shipped off "$vul"
        ;;
    xyz-invite-spade-preference)
        arm off "$vul" --no-ns-xyz-invite-spade-preference
        diffpair shipped off "$vul"
        sddiff shipped off "$vul"
        ;;
    reverse-levers)
        arm off "$vul" --no-ns-forcing-nt-two-suiter
        arm old "$vul" --no-ns-forcing-nt-reverse-five-hearts --no-ns-forcing-nt-reverse-hcp-floor
        arm five "$vul" --no-ns-forcing-nt-reverse-hcp-floor
        arm floor "$vul" --no-ns-forcing-nt-reverse-five-hearts
        diffpair old off "$vul"
        sddiff old off "$vul"
        for lever in five floor shipped; do
            diffpair "$lever" old "$vul"
            sddiff "$lever" old "$vul"
            diffpair "$lever" off "$vul"
            sddiff "$lever" off "$vul"
        done
        diffpair shipped floor "$vul"
        ;;
    competitive-4333)
        arm allow "$vul" --ns-competitive-4333 allow
        diffpair shipped allow "$vul"
        sddiff shipped allow "$vul"
        ;;
    takeout-support)
        arm off "$vul" --ns-takeout-support off
        diffpair shipped off "$vul"
        sddiff shipped off "$vul"
        ;;
    diamond-transfer)
        arm off "$vul" --no-ns-comp-over-diamond-transfer
        diffpair shipped off "$vul"
        sddiff shipped off "$vul"
        ;;
    diamond-captain)
        arm old "$vul" --no-ns-diamond-transfer-captain
        arm off "$vul" --no-ns-comp-over-diamond-transfer
        diffpair shipped old "$vul"
        diffpair shipped off "$vul"
        sddiff shipped old "$vul"
        sddiff shipped off "$vul"
        ;;
    point-scale)
        arm ruleofn "$vul" --ns-point-scale rule-of-n-floored
        diffpair shipped ruleofn "$vul"
        ;;
    *) echo "ab-non-inferiority: unknown experiment $E" >&2; exit 2 ;;
    esac
done
log "=== non-inferiority $E done"
