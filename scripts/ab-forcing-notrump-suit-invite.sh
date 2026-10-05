#!/bin/sh
# ab-forcing-notrump-suit-invite.sh — responder's invitational jump to three of
# a six-card side suit after the forcing `1NT` (`rebid.forcing_notrump_suit_invite`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-forcing-notrump-suit-invite.sh ab-results/fnt-suit-invite \
#       >ab-results/fnt-suit-invite.log 2>&1 < /dev/null &
#
# What is being tested.  After `1M - 1NT - 2x` responder's table has no
# long-suit invitation (the two-level runout is ≤9 HCP), so 10–12 with six
# hearts passes `2♣` or bids `2NT` / `3NT` and misses `4♥`.  BBA bids these
# hands `1♠ - 3♥` (9–11, six hearts): anchor `46d0dc14`, 438 rows vs our
# forcing `1NT`, −1,046 plain / −924 PD per 409,600 boards, ≈ −925 on 10–11.
#
#   off  --no-ns-forcing-notrump-suit-invite (no invite)
#   on   the shipped default
#
# First build, the jump in any six-card side suit over both majors (seed
# 1791192543, sha 01ac2a10-dirty, `ab-results/fnt-suit-invite`): plain +0.0001
# / +0.0001, PD +0.0003 / +0.0003 IMPs/board (none / both), 278 / 310 fired.
# Split by the invite, both vuls pooled: `3♥` over `1♠` +233 plain / +275 PD
# on 147 boards; `3♣`/`3♦` −225 / −233 on 405 (opener passes `3m` on a
# minimum where `2NT` found `3NT` or `4♥`).  The knob keeps the heart jump.
# Each new seed wants a fresh results dir.
#
# The heart jump, sha 01ac2a10-dirty, 204,800 boards/arm/vul, isolation gate
# passed.  IMPs/board none / both:
#
#   seed 1 (1791193528, fnt-heart-invite)    plain +0.0006 / +0.0008, PD +0.0006 / +0.0008, 91 / 92 fired
#   seed 2 (1791194031, fnt-heart-invite-2)  plain +0.0004 / +0.0005, PD +0.0003 / +0.0004, 83 / 100 fired
#   pooled                                   plain +0.0005 / +0.0007, PD +0.0005 / +0.0006
#
# +1.0 to +1.8 IMPs per fired; all eight cells positive.  **Shipped default-on
# 2026-10-05.**  (Measured with the knob default off and `on` armed by the
# retired `--ns-forcing-notrump-suit-invite`.)
R=${1:?usage: ab-forcing-notrump-suit-invite.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for fnt-suit-invite)

log "=== fnt-suit-invite A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-forcing-notrump-suit-invite
    arm on "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== fnt-suit-invite A/B done"
