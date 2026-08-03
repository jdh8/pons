#!/bin/sh
# announced-reading-ab.sh — split disclosure from projection, and measure the
# half that is not bid-inert.
#
# `Inferences` serves two masters that want opposite things.  The sampler needs a
# box that *contains* the truth or `within_ranges` rejects the hands the auction
# was bid on; disclosure needs the box the partnership *agreement* names.  For an
# authored rule the two coincide.  They part company where a learned criterion
# decides: the evaluator net accepts hands no box contains, so its sound
# projection is ⊤ and always will be, and the call reads as nothing.
#
# `set_announced_reading` builds a second, agreement overlay off
# `Constraint::announce` (which defaults to `project`, so it diverges only where
# a rule called `announced`).  The nets' feature vectors consume it; the sampler,
# `admits`, and the opening-lead sampling keep the sound projection untouched.
#
# The overlay has two halves and they are NESTED, not independent:
#
#   1. the union is over *alerted* rules only — an alerted call is disclosed as
#      the convention, not as the unalerted catch-all sharing its bid;
#   2. the floor's 4NT RKCB ask announces `points(11..)` (`set_rkcb_announce`,
#      DELETED 2026-08-02 with its probe — a false disclosure; history below).
#
# (2) cannot bite without (1): the weight-0.3 catch-all on 4NT would union the
# agreement back to ⊤.  Same-seed divergence over 60k boards, from the deleted
# `examples/probe-announced-rkcb`:
#
#   (1) alone      1400 boards (2.333%)
#   (1) + (2)      1405 boards (2.342%)
#
# So (2) is **bid-inert** — five boards in sixty thousand — and rides as a
# disclosure fix on the `set_pass_reading` / `set_cue_reading` precedent.  This
# script therefore measures (1), the half that actually moves boards.
#
# Two arms per vul, identical deals:
#   off        the shipped default (one reading, ⊤ at every net gate)
#   announced  --ns-announced-reading
#
# No criterion moves in either arm — every divergent board is a net reacting to
# a tighter box for some seat, so the mechanism to check first if it loses is
# whether the tighter box is *wrong* (the agreement is calibrated, not sound).
# `RKCB_ASK_ANNOUNCE = 11` covers 95% of firings; the rest of the tightening
# comes from (1) and is exact.
#
# (The attribution arm this comment used to suggest rode `--no-ns-rkcb-announce`,
# which is gone with the knob; the live arms below are unaffected.)
#
#   setsid nohup scripts/idle-run.sh scripts/announced-reading-ab.sh \
#       ab-results/announced-reading >ab-results/announced-reading.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: announced-reading-ab.sh RESULTS_DIR}
SHOW=${SHOW:-40}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== announced reading start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm announced "$vul" --ns-announced-reading
    diffpair announced off "$vul"
done
log "=== announced reading done"
