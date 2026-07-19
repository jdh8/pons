#!/bin/sh
# two-over-one-slam-strength-ab.sh — A/B for teaching the floor that a live 2/1
# promises values (`set_two_over_one_slam_strength`), alone and paired with the
# re-audit's candidate #2 deletion (`--no-ns-opener-third`).
#
# The defect: the 2/1 response carries `.alert(GAME_FORCE)`, so the inference
# walk skips its natural reading and defers to the rule's projection — and the
# rule gates on `points(13..)`, which on the rule-of-N+8 scale soundly projects
# to *no* high-card floor (a 13-point hand can be an eight-count with a six-card
# suit). Partner therefore reads as ZERO through an established game force, and
# the floor's slam-entry gate (29 combined) can never fire: opener holding a
# 26-count opposite the force counts 26 + 0 and signs off in game.
#
# Measured against the real routing (BBA), not `ab-major-continuations` — that
# harness forces every treatment knob off on BOTH arms, so its baseline is a
# stripped system rather than the shipped one, and this gate never engages
# there (0 divergent in 2M boards, confirmed by an in-harness counter).
#
# Three arms, both vulnerabilities, both scorers, strictly sequential, one
# shared SEED_BASE (paired diffs need identical deals).
#
#   PER_SHARD=12800 setsid nohup scripts/idle-run.sh \
#       scripts/two-over-one-slam-strength-ab.sh ab-results/two-over-one-slam \
#       >ab-results/two-over-one-slam.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: two-over-one-slam-strength-ab.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

# base     — the shipped system, unchanged.
# strength — the floor learns the 2/1's promised values; every node stands.
# rail     — strength *plus* the candidate #2 deletion, so the floor owns
#            `1M–2r–R–3M` and can now actually ask there. This is the pairing
#            the game backstop's `--ns-two-over-one-force` rail modelled:
#            delete the node AND restore by rule what it held by omission.
log "=== 2/1-slam-strength A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm base     "$vul"
    arm strength "$vul" --ns-two-over-one-slam-strength
    arm rail     "$vul" --ns-two-over-one-slam-strength --no-ns-opener-third
    diffpair strength base     "$vul"
    diffpair rail     base     "$vul"
    diffpair rail     strength "$vul"
done
log "=== 2/1-slam-strength A/B done"
