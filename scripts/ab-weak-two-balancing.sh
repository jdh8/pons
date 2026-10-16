#!/bin/sh
# ab-weak-two-balancing.sh — tournament-vs-BBA A/B for the two obstruction knobs
# that DD hid (the reason: their value is opponents mis-bidding under pressure,
# which a same-bidder DD A/B cannot see — only a fallible opponent, BBA, can be
# obstructed). One three-arm experiment on shared deals:
#
#   base       — neither knob (shipped default)
#   weak       — base + --ns-weak-two-comp   (competition.weak_two_competition: our
#                contested weak twos — business XX + systems-on Ogust over their
#                double, Ogust/values-X/preemptive raises over their overcall)
#   balancing  — base + --ns-balancing       (defense.notrump_balancing_enabled: extend our
#                1NT defense to the balancing seat, `(1NT) - - ?`)
#
# Each feature arm sets ONLY its own flag, so the paired per-shard diff vs base
# isolates exactly one knob against BBA. Both vulnerabilities, both scorers
# (plain DD + perfect-defense), arms strictly sequential, one shared SEED_BASE
# (paired diffs need identical deals). Do NOT touch the codebase while it runs.
#
#   JOBS=96 BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-weak-two-balancing.sh ab-results/weak-two-balancing \
#       >ab-results/weak-two-balancing.log 2>&1 &
#
# Resumable through ab-lib.sh (manifest-checked, shard-verified arms; stamped
# diffs); the SEED_BASE persists in $R/seed so a restart stays seed-aligned.
# The shard count is JOBS (default nproc), like every ab-lib runner; this
# script used to hard-code nproc and ignore JOBS.
R=${1:?usage: ab-weak-two-balancing.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "weak-two/balancing A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    arm base      "$vul"                       # shipped default (neither knob)
    arm weak      "$vul" --ns-weak-two-comp    # + contested weak twos
    arm balancing "$vul" --ns-balancing        # + 1NT balancing-seat defense
    diffpair weak      base "$vul"
    diffpair balancing base "$vul"
done

log "weak-two/balancing A/B done"
