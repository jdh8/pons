#!/bin/sh
# ab-blind-inference.sh — the reading program's NEGATIVE CONTROL.
#
# Every generator of readings competes for one prize: the IMPs that flow from
# our nets reasoning about what the other three seats have shown.  Authored
# `project`, the agreement overlay (`--ns-announced-reading`), and any future
# sampled projection all *tighten* a box and measure the derivative of that
# prize — and the derivative keeps landing in the noise (the agreement overlay
# came back a wash in all four cells at 204800 bd/arm/vul).
#
# A derivative near zero has two explanations and they call for opposite work:
#
#   the prize is large and we are near its optimum  -> generate readings better
#   the prize is small                              -> stop pricing readings in IMPs
#
# So measure the LEVEL instead of the derivative.  `--ns-blind-inference` blanks
# all four inference blocks in both feature vectors — every seat reads
# `Envelope::unknown` and the nets reason from the auction alone.  The arm's
# loss is a CEILING on the whole program: no reading, however generated, can be
# worth more than what deleting every reading costs.
#
# Only the nets go blind.  The sampler's containment test (`within_ranges`),
# `admits`, and the opening-lead sampling read the `Inferences` directly and are
# untouched, so this is not a soundness experiment — hands are dealt exactly as
# before, and only the decisions taken on them move.
#
# Two arms per vul, identical deals:
#   seen   the shipped default (readings as authored)
#   blind  --ns-blind-inference
#
# Expect a LOSS.  The number is the point; the sign is not news.  Read it as:
#
#   >= 0.10 IMPs/bd   readings are load-bearing — build the sampled projection
#   0.02 .. 0.10      real but small — spend on disclosure-driven sites only
#   <= 0.02           the program is a correctness feature; stop pricing it
#
# MEASURED 2026-07-26 (SEED_BASE=1785072635, 204800 bd/arm/vul): -0.6501 plain /
# -1.1052 PD at vul none, -0.8850 / -1.2685 at both — six to twelve times the
# top band, and about the whole remaining gap to BBA (-1.152 / -1.355).  Broad,
# not a tail: a third of boards diverge and the mean divergent board loses 1.8
# to 3.9 IMPs.  Verdict recorded in docs/ai-bidder/sampled-projection.md.
#
#   setsid nohup scripts/idle-run.sh scripts/ab-blind-inference.sh \
#       ab-results/blind-inference >ab-results/blind-inference.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-blind-inference.sh RESULTS_DIR}
SHOW=${SHOW:-40}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== blind inference start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm seen "$vul"
    arm blind "$vul" --ns-blind-inference
    diffpair blind seen "$vul"
done
log "=== blind inference done"
