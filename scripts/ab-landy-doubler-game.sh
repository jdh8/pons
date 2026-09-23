#!/bin/sh
# ab-landy-doubler-game.sh — §N1r row 9: the values doubler's rebids over
# their `(2M)` runout at favourable (`1NT (2♣) X (2M) - -` and the two
# `X (2♦) - (2M)` legs).  `3NT` on `points(10..)` for the game hand the
# favourable gate sent through the `X`, a natural `3♣`/`3♦` for the 8–9 hand
# (so `X – 3m` reads `8..9` again), opener passes the game and answers the
# minor.  Face-gated on favourable, so `ew` is the only cell that moves on
# table A (seeded identity check: none/both byte-identical, ns table A too).
#
#   JOBS=24 BOARDS=4608000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-doubler-game.sh ab-results/landy-doubler-game \
#       >ab-results/landy-doubler-game.log 2>&1 < /dev/null & disown
#
# Prior (§N1r step 0b): the tax this repairs measured −0.0034 sd-lead at
# `both` on the misfit arm; the shipped `ew` gate carries the same tax inside
# its +0.0042 / +0.0023 sd.  Pre-registered falsifier: the floor's own second
# `X` on the short-trump 10+ hand (§N1-lia A's +0.0036 NV) was worth more
# than the `3NT` that now claims it.
R=${1:?usage: ab-landy-doubler-game.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-doubler-game)
log "=== landy-doubler-game SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

arm base ew --filter-landy --no-ns-landy-doubler-game
arm game ew --filter-landy
gatepair game base ew
diffpair game base ew
sddiff game base ew

log "landy-doubler-game done"

# VERDICT 2026-09-23 (seed 1790142988, control 9d6a51ae + the knob, gate 0
# foreign, 4.608M bd/arm).  IMPs/board at ew, DD plain / DD PD / sd-lead
# plain / sd-lead PD:
#   +0.0085 ±0.0003 / +0.0117 ±0.0004 / +0.0078 ±0.0004 / +0.0103 ±0.0004
#   11,332 fired (0.25%), +3.45 / +4.76 IMPs/fired on DD
# A win on all four; shipped default-on as `competition.landy_doubler_game` —
# the flag is now `--no-ns-landy-doubler-game`, so a rerun's `base` arm needs
# it and the `game` arm is `main`.  Write-up: docs/one-notrump-competitive.md
# §N1r row 9.
