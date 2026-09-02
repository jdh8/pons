#!/bin/sh
# ab-one-diamond-length.sh — publish the `1♦` opening's assured diamond length.
#
# The better-minor `1♦` rule carries no diamond length term: its length lives in
# `prefers_diamonds()`, a `described(...)` closure whose projection dependencies
# are the vacuous default, so the *rule* publishes `♦ 0..=13` while the natural
# walk installs `3..` for the same call.  `OpeningKnobs::one_diamond_publishes_
# length` makes the rule say what it already means: `& len(Suit::Diamonds, 3..)`.
#
# THE TERM IS EVAL-INERT.  Both majors are capped at four, so `c + d >= 5`; if
# `d <= 3` then `prefers_diamonds` forces `d > c`, giving `c + d <= 2d − 1 <= 5`,
# hence `d == 3`.  No hand the rule accepts is rejected — pinned exhaustively
# over shapes by `one_diamond_assures_three`, and over 256 dealt hands by
# `one_diamond_publishes_length_is_opt_in`.  What moves is what the call
# *publishes*, and that alone changes bids: `smoke-default --count 20000
# --seed 1` reads 38ee1e21… off and acc4ab9d… on.  That is the whole hypothesis.
#
# Two arms per vul, identical deals, both playing `american()` UNFILTERED:
#   off    the shipped default (crate default off — byte-identical, pinned by
#          `smoke-default --count 20000 --seed 1` at this commit and at control)
#   pub    --ns-one-diamond-length
#
# First arm of the publish-assured-length campaign (docs/ai-bidder/
# new-suit-veto.md §6, "the first successor"), whose worklist is the
# `silent_natural_suits` census in src/bidding/inference/tests.rs.  The campaign
# exists because "natural = assured length" is a sound claim the reading layer
# CAN carry — `Inferences` never over-promises, so `min` is a sound lower bound
# — but 69.6% of the book's suit-naming rules never make it.
#
# NO PRE-REGISTERED DIRECTION.  Unlike a demotion-only rail this knob neither
# bids less nor more by construction; it discloses.  So the standard decision
# table applies unmodified (docs/measurement.md): plain DD and PD at both
# colours, ship default-on only on a non-loss on both scorers, and read
# `loss | win` as the doubling artifact it normally is rather than as this
# knob's expected shape.
#
# NO ADVERTISE FLAGS.  The generated `.bbsa` card is unchanged: its
# "1D opening with 4 cards" / "1D opening with 5 cards" rows are constants
# (src/bidding/card.rs:566 — publishing `3..` makes the opening neither 4+ nor
# 5+), so both arms disclose the identical system to BBA and the treatment is
# purely our own reading of our own opening.  `ab-dump-sd` needs none either.
#
# ISOLATION GATE ARMED, unlike the phantom-suit rail's runner.  The knob can
# only fire on a board where WE open `1♦`, so `gatepair … ours` must read
# 0 foreign; a foreign divergence would mean the term is not eval-inert after
# all and is changing which hands open what.  That is the cheapest possible
# check on the claim the whole campaign rests on.
#
# SIZING.  Pass 1 is the house default 204,800 bd/arm/vul.  We open 1♦ on
# roughly a fourteenth of boards, so read the divergent count off the diff
# before quoting anything per-fired; if it is thin, pass 2 goes in a FRESH dir
# (fresh seed) with BOARDS raised rather than reusing this one.
#
#   JOBS=32 BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-one-diamond-length.sh ab-results/one-diamond-length \
#       >ab-results/one-diamond-length.log 2>&1 < /dev/null & disown
#
# Iron rule: do NOT edit src/ or run cargo while this runs.
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-one-diamond-length.sh RESULTS_DIR}
SHOW=${SHOW:-40}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== 1D published length start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm pub "$vul" --ns-one-diamond-length
    gatepair pub off "$vul" ours
    diffpair pub off "$vul"
done
log "=== 1D published length done"

# Post-hoc forensic, per vul — run by hand once the headline is read:
#   $PROBE $R/pub-$v $R/off-$v --imps --jsonl $R/imps-$v.jsonl
# The question a losing arm has to answer: publishing `3..` can only tighten a
# reading, so a loss means some consumer was *relying* on the looser box —
# bucket the divergences by which seat's decision moved, not by level.
