#!/bin/sh
# a4-run.sh — measurements for the A4 pass of the bidding-options audit
# (docs/bidding-options.md A4, "Competitive auctions — they overcall / double
# our opening").  Closes the two cue-answer knobs, the A4 `unmeasured` knobs that
# carry a bba-gen flag, so the standard ab-lib.sh contested arm/diffpair applies
# (unlike a3-run.sh, whose knobs had no flag and drove self-play directly).
#
# Both cue knobs are shipped default-on, so the off arm uses the `--no-ns-*`
# off-switch.  The knob changes only opener's answer to partner's cue-raise, deep
# in the auction, so the arms are byte-identical until it fires — the divergent
# set is the fired set (read IMPs/fired straight off), no --advertise needed.
#
#   JOBS=12 setsid nohup scripts/idle-run.sh scripts/a4-run.sh ab-results/a4 \
#       >ab-results/a4.log 2>&1 < /dev/null &
#
# Shared-box cap: set JOBS (e.g. 12) to bound worker processes — bba-gen-parallel
# creates JOBS shards and ab-lib's diffpair reads exactly JOBS shards.  PER_SHARD
# (default 6400) sets boards/shard/arm/vul; a smoke run uses tiny values.
#
# NOT in this pass: delayed_cue and direct_3nt_stopper (the other two A4
# `unmeasured` knobs) have no bba-gen flag and need bespoke self-play distillation
# (ab-sohl-after-double --delayed-cue / ab-lebensohl --pd-3nt) — deferred.
#
# Resumable: an existing arm dir / diff file is skipped; each experiment's
# SEED_BASE persists in $R/<exp>.seed.  Do NOT rebuild binaries while this runs.
R=${1:?usage: a4-run.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"

# One experiment per knob — a fresh SEED_BASE each, so the two run on independent
# deals (paired diffs still share deals within an experiment).
for knob in cue-raise-answer cue-minor-raise-answer; do
    exp=$knob
    SEED_BASE=$(seed_for "$exp")
    log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD boards/arm/vul"
    for vul in none both; do
        arm "$exp-on" "$vul"
        arm "$exp-off" "$vul" "--no-ns-$knob"
        diffpair "$exp-on" "$exp-off" "$vul" # ship gate vs the byte-identical default
    done
done

log "a4 campaign done"
