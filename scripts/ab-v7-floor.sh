#!/bin/sh
# ab-v7-floor.sh — M5.2: the v7 sequence floor against the shipped v6 floor.
#
# v6 flattens the auction into strain bitmasks plus cumulative hulls, so a
# transfer `2♠` and a natural `2♠` reach the net as the same 176 floats.  v7
# keeps that vector as a static block and prepends an LSTM over one token per
# prior call, each carrying the call plus the box its authoring rule projected
# (`features::call_tokens_v7`).  Same corpus, same deals, same cells, same head:
# `--feature-version 7` writes a byte-identical `.f32` and only adds the `.seq`
# sibling, so the architecture is the only difference between the arms.
#
# Two arms per vul, identical deals, both playing `american()` UNFILTERED:
#   american      the shipped default (v6 floor)
#   american-v7   the LSTM sequence floor
#
# FIDELITY GATE (passed 2026-09-03, two seeds each, docs/ai-bidder/plan.md M5.2):
#   val_ce   LSTM 0.2808 / 0.2813   control 0.3029 / 0.2998
#   top1     LSTM 90.6% / 90.7%     control 89.3% / 89.4%
#   contested top1 90.6% both seeds vs 89.6% both seeds
# The arms' ranges do not overlap on either criterion, and the largest
# within-arm seed spread is 0.0031 CE.  That earns this A/B and nothing more:
# `docs/ai-bidder/02-policy-net.md:225-234` records an arm whose near-identical
# top-1/CE sat on top of a −0.040/−0.115 IMP loss.  Fidelity is a filter here,
# never a predictor.
#
# RUN 2 (2026-09-03): the artifact in the tree is now the **advantage-weighted**
# net, not the raw-fidelity one.  Run 1 (raw fidelity, SEED_BASE 1788391271) was a
# measured loss: none plain +0.0059 +/- 0.0103 / PD -0.0239 +/- 0.0120; both plain
# +0.0038 +/- 0.0120 / PD -0.0134 +/- 0.0139 -- `wash | loss`, the mirror of the
# shippable row.  The traced mechanism was inherited aggression: the LSTM fits BBA
# better and therefore bids one more level high up (vul none, 5-level+ 9.412% ->
# 10.307%, doubled 5.476% -> 6.189%), which the competitive accountant did not
# absorb (fire rates matched within 1%).
#
# Run 2 reweights each corpus row by exp(0.10 * A), A = the IMP advantage of the
# call BBA actually chose, scored with conditional perfect defense (a contract
# down 2+ is assumed doubled, down 1 is not -- `examples/reweight-corpus
# --double-from 2`).  Fidelity therefore FALLS on purpose and is no longer a gate:
#   val_ce 0.3082 (vs 0.2808/0.2813 unweighted)  top1 90.1% (vs 90.6%/90.7%)
# Only the A/B below decides.
#
# RUN 3 (2026-09-03): the artifact is the **Monte-Carlo advantage** net.
#
# Run 2 (`exp(0.10*A)`, A = the chosen call priced as if it ended the auction,
# SEED_BASE 1788403940) inverted run 1's shape and still lost:
#   none  plain +0.0151 +/- 0.0105 WIN   PD -0.0344 +/- 0.0122 loss
#   both  plain +0.0308 +/- 0.0122 WIN   PD -0.0089 +/- 0.0141 wash
# `win | wash/loss` is the decision table's doubling-artifact row.  Cause, and
# the reason run 3 exists: pricing a *call* as a contract is level-biased.  63%
# of priced calls were 1- or 2-level, so a `1H` opening was scored as a 1H
# contract against a par of 620 and downweighted, while `Pass`/`X`/`XX` were
# unpriced at weight 1.  Mean weight by level ran 1.60 1.51 1.57 1.94 1.83 2.78
# 3.09 for levels 1-7 -- every bid outweighed every pass, and high bids outweighed
# low.  The A/B measured exactly that push: 5-level+ contracts over control grew
# +0.895pp -> +1.302pp (none) and +0.576 -> +1.056 (both).
#
# Run 3 prices the **auction outcome** against DDS par and credits that return to
# every decision in the auction, signed for the deciding side (`reweight-corpus`,
# unchanged beta 0.10).  Passes are priced for the first time; priced rows go
# from 14% to 63% of the corpus.  The level gradient inverts to
#   pass 1.471  L1 0.998  L2 0.925  L3 0.780  L4 0.748  L5 0.682  L6 0.500
# which is signal, not bias: a slam that makes only matches par while one that
# fails lands far below it.  Fidelity is again NOT a gate -- only the A/B decides.
#
# ARBITER.  This is a floor swap with no directional bid-more/bid-less prior, so
# the standard decision table applies unmodified (docs/measurement.md): win|win
# or wash|PD-win ships; a plain-DD loss never ships default-on.  Score both plain
# DD and perfect defense.
#
# SIZING.  House default 204,800 bd/arm/vul.  The v7 floor is ~11x v6's
# arithmetic per decision (a 20-step recurrence rebuilt per call, no state
# cache), so expect the bidding half to cost noticeably more wall clock; the
# double-dummy half is unchanged and normally dominates.
#
#   JOBS=32 BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-v7-floor.sh ab-results/v7-floor \
#       >ab-results/v7-floor.log 2>&1 < /dev/null & disown
#
# Iron rule: do NOT edit src/ or run cargo while this runs.
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-v7-floor.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== v7 sequence floor gate start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm american "$vul" --our-floor american
    arm american-v7 "$vul" --our-floor american-v7
    diffpair american-v7 american "$vul"
done
log "=== v7 sequence floor gate done"
