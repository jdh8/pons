#!/bin/sh
# ab-declared-agreement.sh — is telling our net *which agreement* the opponents
# play worth anything, when the opponents are a pons book that really differs?
#
# The gate for the Phase-3 knob migration (docs/declarative-rows.md): the whole
# payoff of a config struct is that a card can be read for a seat other than our
# own, so before spending it, price the channel on the cell it was built for.
#
# Phase 2a (`--declare-opponents` vs EPBot) measured negative and 2b (the
# reader's twin) a wash, but both declared a card that differs from ours in
# *dozens* of rows — off the trained manifold, where every v4 corpus cell pair
# differs by at most one row.  These arms move exactly one row, so the net is
# asked a question it was trained on.
#
# Three arms per experiment, all against the same deviating pons opponent, whose
# book is byte-identical across the three — so their side reads us the same way
# in all of them and every IMP is ours:
#
#   symmetric  our net believes they play our card       (today's default)
#   wrong      our net is told they differ on the OTHER row
#   truth      our net is told the row they actually differ on
#
# `wrong` is what a two-arm design cannot separate: without it, a `truth` win
# could be "reading them correctly helps" or merely "reading them as anything
# other than ourselves helps".
#
# A reading/disclosure change is a constructive-side change, so plain DD is the
# honest primary metric and pd guards the doubling tail.
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/ab-declared-agreement.sh ab-results/declared-agreement \
#       >ab-results/declared-agreement.log 2>&1 &
#
# Resumable; one SEED_BASE per experiment persists in $R/NAME.seed (a NEW dir →
# new seeds).
R=${1:?usage: ab-declared-agreement.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"

# experiment NAME "THEIR-FLAGS" "OTHER-FLAGS" — the three arms and two diffs.
# Its own SEED_BASE, so the second experiment draws fresh hands.
# `sh` has no locals and ab-lib's helpers assign `name`/`vul`/`on`/`off`, so
# this function's own state lives in names none of them touch.
experiment() {
    exp=$1; deviation=$2; decoy=$3
    SEED_BASE=$(seed_for "$exp")
    log "=== $exp: they play [$deviation], mis-declared as [$decoy], SEED_BASE=$SEED_BASE"
    for v in none both; do
        arm "$exp-symmetric" "$v" --their-floor american --their-ns "$deviation"
        arm "$exp-wrong" "$v" --their-floor american --their-ns "$deviation" \
            --declare-opponents --declare-as "$decoy"
        arm "$exp-truth" "$v" --their-floor american --their-ns "$deviation" \
            --declare-opponents
        diffpair "$exp-truth" "$exp-symmetric" "$v"
        diffpair "$exp-wrong" "$exp-symmetric" "$v"
    done
}

log "=== declared-agreement start, sha=$SHA, ${SHARDS}x${PER_SHARD} bd/arm/vul"
# Garbage Stayman: a weak shapely responder passes 1NT - 2♣ - 2any out.  Fires
# opposite our 1NT openings, on their side and ours.
experiment garbage --no-ns-garbage-stayman --no-ns-major-game-tries
# Two-way game tries: which invitational major raise sequence they use.  Fires
# on every invitational major auction.
experiment tries --no-ns-major-game-tries --no-ns-garbage-stayman
log "=== declared-agreement done"
