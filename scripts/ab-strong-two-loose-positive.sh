#!/bin/sh
# ab-strong-two-loose-positive.sh — BBA's looser positive to our strong `2♣`
# and the major-fit tables below a positive
# (`response.strong_two_loose_positive`, `rebid.strong_two_positive`,
# docs/next-steps.md item 2, the strong `2♣` lane).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-strong-two-loose-positive.sh ab-results/strong-two-loose \
#       >ab-results/strong-two-loose.log 2>&1 < /dev/null &
#
# What is being tested.  Our suit positive needed five cards to two of the top
# three honors and 8+ points, our `2NT` 8+ HCP balanced; everything else
# waited with `2♦`, and over opener's 22–24 `2NT` an 8-count then stopped in
# `3NT`.  BBA (EPBot defaults, `probe-bba-book --card none`) bids a positive
# on any five-card suit, or a balanced hand, with 7+, and after it bids
# naturally: a new five-card suit, a raise, keycards.  The 2026-09-30 anchor
# prices our `2♦` against its positive at 769 rows, −2.0k plain / −2.2k PD
# per 409,600 boards, and our floor's rebid after a positive at 393 rows,
# −0.9k / −1.0k — of which missed grands are 62 rows, −451, and `6NT` where
# BBA plays six of a major 80 rows, −183.
#
#   both   the default since 2026-10-01: a suit positive is any five-card
#          suit on 7+ HCP, `2NT` a balanced 7+; after a positive opener asks
#          `4NT` RKCB with three-card support for responder's major, else
#          shows a five-card major; responder raises it to game with three
#          (opener then asks), else bids `6NT` on 9+ and `3NT` below, over
#          which opener bids `6NT` on 23+, `4M` on six, or passes.  Any
#          other hand stays with the floor.
#   loose  `--no-ns-strong-two-positive`: the looser positive, the floor
#          below it
#   off    both disarmed
#
# SHIPPED default-on 2026-10-01, the pair together.  Three seeds (1790857656,
# 1790858639, 1790859531; `strong-two-loose{,-r2,-r3}`), 204,800
# boards/arm/vul each, every isolation gate passed.  Pooled (614,400
# boards/vul), IMPs/board none / both:
#
#   both vs off     plain +0.0017 ±0.0009 / +0.0022 ±0.0012
#                   PD    +0.0016 ±0.0010 / +0.0021 ±0.0012   (12/12 cells > 0)
#   loose vs off    plain +0.0008 ±0.0009 / +0.0011 ±0.0012
#                   PD    +0.0007 ±0.0010 / +0.0011 ±0.0012   (seed 2 negative)
#   both vs loose   plain +0.0009 ±0.0006 / +0.0010 ±0.0008
#                   PD    +0.0009 ±0.0006 / +0.0010 ±0.0008   (12/12 cells > 0)
#
# Seeds 1–2 ran the `both` arm before its doubled tail was closed: reached
# through the `2♣ (X)` systems-on rebase, a hand opener's table rejects does
# not fall through to the floor, and opener passed the positive.  The node at
# `2♣ (X) positive` is now total (`3NT`); on seed 1 that moves exactly 5
# boards per vulnerability, +21 / +35 plain, +20 / +33 PD
# (`bothfix-*` beside the arms).  Seed 3's `both` arm is the shipped build
# (an earlier form of the fix, which leaked a reading into the undoubled
# `3NT`, is kept under `strong-two-loose-r3/leaky`).
#
# The tables below a positive were first measured alone, against the old
# positive (`ab-results/strong-two-positive{,-r2,-r3}`, one seed each, vul
# none / both, by a runner since folded into this one):
#
#   round 1  a classical ladder — `3NT` on 22–24, a quantitative `4NT`,
#            `4M` on a minimum fit: plain −0.0029 (none only; gate failed, 2
#            boards through the reading mirror, since pinned).  It stopped in
#            game where slam pays: double-dummy, slam is right from about 31
#            combined HCP in this lane.
#   round 2  slam-seeking, the level still authored (`3NT` on a bare 22,
#            `6NT` on 9+ opposite): plain −0.0003 / −0.0004, PD the same.
#            The floor's own level judgement is at par or better.
#   round 3  the shipped tables, strain and keycards only: plain +0.0001 /
#            +0.0002, PD +0.0001 / +0.0001, 99 / 106 fired — a wash until the
#            looser positive doubled how often they are reached.
R=${1:?usage: ab-strong-two-loose-positive.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for strong-two-loose)

log "=== strong-two-loose A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off   "$vul" --no-ns-strong-two-loose-positive --no-ns-strong-two-positive
    arm loose "$vul" --no-ns-strong-two-positive
    arm both  "$vul"
    gatepair loose off "$vul"
    diffpair loose off "$vul"
    gatepair both off "$vul"
    diffpair both off "$vul"
    diffpair both loose "$vul"
done
log "=== strong-two-loose A/B done"
