#!/bin/sh
# panel.sh — the DEVIATION PANEL: what is reading *their* calls worth against
# an opponent who does not play the card we read them with?
#
# Our `Inferences::read` applies *our* meanings to the opponents' calls, and
# `probe-reading-sound` measured what that costs against BBA alone: our boxes
# exclude the opponent's true hand 8.2/8.3% of the time (LHO/RHO) against 3.3%
# for partner, with BBA's weak twos at 33-37% and their Multi 2D at 100%.  The
# goal is not brilliant self-play against one bot — it is to beat humans, who
# deviate.  So evaluate against a *population* of perturbed natural bidders
# (domain randomization on the opponent) instead of one fixed opponent.
#
# Three deviation axes, eleven members:
#   A  BBA's own base systems and convention swaps       (zero pons code)
#   B  the antisymmetric strength dial: their openings/overcalls x points
#      lighter, their responses/advances x heavier — pair calibration is
#      preserved, so their continuations stay coherent
#   C  shape indiscipline: 4-card overcalls, off-shape 1NT, wild weak twos
#
# PRIMARY STATISTIC — paired reading value, per member.  Two arms on identical
# deals:
#   seen   the shipped default
#   blind  --ns-blind-opponent-reading (LHO/RHO readings blanked; partner and
#          our own stay live)
# and `seen - blind` is what reading that opponent is worth.  The *absolute*
# score against a deviant member is confounded — a member playing a weaker
# system hands us IMPs while we misread it more — so read the paired column,
# not the level.
#
# NOT comparable to scripts/blind-inference-ab.sh (-0.65 .. -1.27 IMPs/bd):
# that control blinded all four seats and only the nets; this one blinds the
# two opponent seats at the `Inferences` level, so the sampler and the floor go
# blind with them.
#
# Read it as: reading value that COLLAPSES or FLIPS SIGN on the realistic
# members (B x=1, C 4-card overcalls) is the case for funding slack on opponent
# boxes.  Reading value that holds across the panel says the reading layer
# generalizes and the 8.2% exclusion is survivable.
#
#   setsid nohup scripts/idle-run.sh scripts/panel.sh \
#       ab-results/panel >ab-results/panel.log 2>&1 &
#
# Resumable; ONE SEED_BASE for the whole panel ($R/seed), so every member's
# columns sit on identical deals.  MEMBERS=... restricts the run to a subset.
R=${1:?usage: panel.sh RESULTS_DIR}
PER_SHARD=${PER_SHARD:-2500}
SHOW=${SHOW:-5}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

# name:flags, one member per line.  The A-axis base systems come from the
# vendored cards' own `System type` header — do NOT bind `epbot_system_name`,
# which is exported but segfaults.
MEMBERS=${MEMBERS:-'
sayc            --system 1 --their-card vendor/bba/Sayc.bbsa
wj              --system 2 --their-card vendor/bba/WJ.bbsa
precision       --system 3 --their-card vendor/bba/PC.bbsa
acol            --system 4 --their-card vendor/bba/Acol.bbsa
multi           --their-conv "Weak natural 2D=0" --their-conv "Multi=1"
cappelletti     --their-conv "Multi-Landy=0" --their-conv "Cappelletti=1"
dial1           --their-floor american --their-dial 1
dial2           --their-floor american --their-dial 2
overcall4       --their-floor american --their-overcall-four-card
offshape1nt     --their-floor american --their-offshape-1nt
wildweak2       --their-floor american --their-wild-weak-two
'}

log "=== deviation panel start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
# `member`, not `name`: ab-lib's `arm` assigns an unscoped `name`.
echo "$MEMBERS" | while read -r member flags; do
    [ -n "$member" ] || continue
    log "--- member $member: $flags"
    # `eval set --` rather than word-splitting: BBA convention names contain
    # spaces (`--their-conv "Weak natural 2D=0"`).
    eval "set -- $flags"
    for vul in none both; do
        arm "$member-seen" "$vul" "$@"
        arm "$member-blind" "$vul" "$@" --ns-blind-opponent-reading
        diffpair "$member-seen" "$member-blind" "$vul"
    done
done
log "=== deviation panel done"
