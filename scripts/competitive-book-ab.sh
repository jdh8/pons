#!/bin/sh
# competitive-book-ab.sh — the chained A/B campaign for the competitive-book
# packages (docs/competitive-book.md). One SEED_BASE per experiment shared
# across its arms (paired diffs need identical deals); a fresh base per
# experiment; arms strictly sequential; both scorers, both vulnerabilities.
#
#   setsid nohup scripts/idle-run.sh scripts/competitive-book-ab.sh \
#       ab-results/competitive-book >ab-results/competitive-book.log 2>&1 &
#
# Resumable: an arm directory that already exists is skipped, and each
# experiment's SEED_BASE persists in $R/<exp>.seed, so a restart regenerates
# nothing and stays seed-aligned. Do NOT touch the codebase while this runs
# (bba-gen-parallel re-invokes cargo build; it must stay a no-op).
#
# Polarity: the four winners shipped default-on (uvu-over-majors, strong-two-
# comp, major-support-double, jordan-truscott), so their experiments measure the
# knob by removing it on the OFF arm (--no-ns-*); the diff is still on-vs-off.
# The two opt-in knobs (weak-two-comp, high-overcall) add --ns-* on the ON arm.
# Re-check a knob's default in bba-gen before adding an experiment.
R=${1:?usage: competitive-book-ab.sh RESULTS_DIR}
SHOW=3
. "$(dirname "$0")/ab-lib.sh"

log "campaign start, sha=$SHA, shards=$SHARDS x $PER_SHARD boards/arm/vul"

# --- simple two-arm experiments -------------------------------------------
# run_two_arm EXP ON_FLAGS OFF_FLAGS — arms $EXP-on / $EXP-off + their diff.
#   opt-in knob:  ON_FLAGS=--ns-knob    OFF_FLAGS=''            (on arm adds it)
#   shipped knob: ON_FLAGS=''           OFF_FLAGS=--no-ns-knob  (off arm drops it)
run_two_arm() {
    exp=$1; on_flags=$2; off_flags=$3
    SEED_BASE=$(seed_for "$exp")
    log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA"
    for vul in none both; do
        arm "$exp-off" "$vul" $off_flags
        arm "$exp-on"  "$vul" $on_flags
        diffpair "$exp-on" "$exp-off" "$vul"
    done
}

run_two_arm p1-uvu-majors  ''                 --no-ns-uvu-over-majors     # shipped
run_two_arm p2a-weak-two   --ns-weak-two-comp ''                          # opt-in
run_two_arm p2b-strong-two ''                 --no-ns-strong-two-comp     # shipped
run_two_arm p3c-support-x  ''                 --no-ns-major-support-double # shipped
run_two_arm p3a-high-ovc   --ns-high-overcall ''                          # opt-in
run_two_arm p4-jordan      ''                 --no-ns-jordan-truscott     # shipped

# --- the four-arm negative-double experiment ------------------------------
exp=p3bd-negx
SEED_BASE=$(seed_for "$exp")
log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA"
for vul in none both; do
    arm "$exp-off" "$vul"
    arm "$exp-free" "$vul" --ns-free-bids
    arm "$exp-modern" "$vul" --ns-negative-double-shape modern
    arm "$exp-cachalot" "$vul" --ns-negative-double-shape cachalot
    diffpair "$exp-free" "$exp-off" "$vul"
    diffpair "$exp-modern" "$exp-off" "$vul"
    diffpair "$exp-modern" "$exp-free" "$vul"
    diffpair "$exp-cachalot" "$exp-modern" "$vul"
    diffpair "$exp-cachalot" "$exp-off" "$vul"
done

log "campaign done"
