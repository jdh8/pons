#!/bin/sh
# ab-two-notrump-super-accept.sh — opener jumps to `4M` over a three-level
# transfer with four-card support, vs BBA, four cells, plain DD + perfect
# defense from one solve, plus SD-PD.  Arms are sequential and share one
# persistent fresh seed.
#
#   BOARDS=1638400 setsid nohup scripts/idle-run.sh \
#       scripts/ab-two-notrump-super-accept.sh ab-results/two-notrump-super-accept \
#       >ab-results/two-notrump-super-accept.log 2>&1 < /dev/null &
#
# Why.  On the BBA anchor 0772f1c4 (docs/bba-gap-campaign.md, round-2 cut,
# lead 4) our completion where BBA jumps to `4M` over `2NT - 3♦ -` /
# `2NT - 3♥ -` is 114 rows (63 deals) per 409,600 boards, -358 IMPs plain /
# -335 PD.  Traced 2026-11-22, it is a game lane, not the slam lane the cut
# guessed: 57 rows are responder passing our `3M` on 0-4 HCP with five
# trumps while BBA's `4M` makes (-302 plain); the slam rows net about -50.
# BBA's jump has no strength gate (every opener holds 4+ trumps).
#
#   base completion at three (`--no-ns-two-notrump-super-accept`; the
#        default when this ran, the arms then being `base` bare and `on`
#        `--ns-two-notrump-super-accept`)
#   on   notrump.two_notrump_super_accept: 4+ trumps bid 4M over
#        2NT - 3D/3H and the 2C - 2x - 2NT lanes; responder passes the
#        jump or asks keycards on 11+ support points (9+ in the 2C
#        lanes), and reads a 5C answer as four keycards unless holding
#        two (rounds 1 and 2 differ here; see the READs below)
#
# Not in this arm: an HCP or shape gate on the jump, the
# interfered tails (`2NT - 3♦ (X)`), the 1NT super-accept (its own knob,
# a measured wash).
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# READ 2026-11-22 (SEED_BASE 1795279032410175492, 1,638,400 bd/arm/cell,
# sha 69eed3af-dirty, isolation gate passed 4/4): NOT A WIN as built.
#   cell  fired  plain IMPs/bd (95% CI)      PD IMPs/bd      SD plain / SD-PD
#   none   908   -0.0004 ±0.0003 loss        -0.0006 loss    +0.0003 / +0.0001
#   ns     944   -0.0002 ±0.0004 non-inf     -0.0005 loss    +0.0009 / +0.0006 win
#   ew     897   -0.0003 ±0.0003 loss        -0.0006 loss    +0.0003 / +0.0001
#   both   934   -0.0002 ±0.0004 non-inf     -0.0005 loss    +0.0008 / +0.0005 win
# Traced (probe-divergence --imps, split by final level): the game lane wins,
# the unauthored slam tail loses more.  Boards ending at game or below:
# +374 / +831 / +382 / +801 plain IMPs (none/ns/ew/both), PD +169..+511.
# Boards where either arm reaches slam: -958 / -1160 / -953 / -1185 plain.
# The floor's answer to the jump bids 6M where base stopped at 3NT/4M
# (~400 boards/cell, about -650 plain) and 7M where base bid six (80-85
# boards, -325..-412).  Flip plan: author responder's continuations over 4M.
# Round 2 (results dir two-notrump-super-accept-r2, fresh seed) measures that
# authoring: `after_super_accept` plus the RKCB rows.
#
# READ 2026-11-23, round 2 (SEED_BASE 1795336735636577673, 1,638,400
# bd/arm/cell, sha 69eed3af-dirty, isolation gate passed 4/4): worst cell
# NON-INFERIOR on both scorers, so not a default-on win yet; SD wins 4/4.
#   cell  fired  plain IMPs/bd (95% CI)      PD IMPs/bd        SD plain / SD-PD
#   none   697   +0.0001 ±0.0002 non-inf     -0.0001 non-inf   +0.0005 / +0.0004 win
#   ns     722   +0.0003 ±0.0003 win         +0.0001 non-inf   +0.0009 / +0.0007 win
#   ew     688   +0.0001 ±0.0002 non-inf     -0.0001 non-inf   +0.0004 / +0.0003 win
#   both   714   +0.0003 ±0.0003 win         +0.0001 non-inf   +0.0010 / +0.0008 win
# Split (probe-divergence --imps, full divergent set; none/ns/ew/both plain):
#   jump, game or below        +533 / +844 / +526 / +836   (282-297 boards)
#   jump, either arm in slam   -370 / -394 / -371 / -377   (139-155 boards)
#   3M completion, game-        +19 / +179 /  +30 / +179   (114-130 boards)
#   3M completion, slam         -17 /  -69 /  -28 /  -69   (143-151 boards)
# The jump's slam tail, none cell: responder passes 4M under 11 HCP on shape
# where base's floor bid six (6M -> 4M 69 boards -253; 6NT -> 4M 17 boards
# -54), and signs off in 5M over a 5C answer that showed four keycards, not
# one (11 boards, -100).  Flip plan: a shape/length upgrade on the 4NT gate,
# and opener bids on over the sign-off holding four.
# Round 3 (results dir two-notrump-super-accept-r3, fresh seed) measures the
# gate on support points and, in place of opener bidding on, jdh8's rule: a
# combined one or two keycards is impossible facing 20+, so the asker reads
# 5C as four (`slam::rkcb_rows_facing_strong`).
#
# READ 2026-11-23, round 3 (SEED_BASE 1795365303630366253, 1,638,400
# bd/arm/cell, sha 69eed3af-dirty): WIN 4/4 on plain, PD, SD and SD-PD.
# Isolation gate FAILED 4/4 on the same single deal (see below).
#   cell  fired  plain IMPs/bd (95% CI)   PD IMPs/bd    SD plain / SD-PD
#   none   782   +0.0007 ±0.0003 win      +0.0005 win   +0.0009 / +0.0008 win
#   ns     802   +0.0010 ±0.0003 win      +0.0008 win   +0.0015 / +0.0013 win
#   ew     777   +0.0006 ±0.0003 win      +0.0005 win   +0.0011 / +0.0010 win
#   both   790   +0.0010 ±0.0003 win      +0.0008 win   +0.0014 / +0.0013 win
# Split (probe-divergence --imps, full divergent set; none/ns/ew/both plain):
#   jump, game or below        +607 / +931 / +607 / +923   (326-331 boards)
#   jump, either arm in slam   +393 / +501 / +360 / +518   (191-209 boards)
#   3M completion, game-       +211 / +368 / +211 / +369   (112-118 boards)
#   3M completion, slam        -128 / -134 / -117 / -155   (137-150 boards)
# The jump's slam tail flipped sign (round 2: -370..-394).  The residue is
# the 3M completion reading as at most three trumps: responder drives to
# slam where base stopped in game (none cell: 3NT -> 6NT 47 boards -99,
# 4H -> 6H 22 boards -94) and to a grand where base bid six (6H -> 7H 12
# boards -18, plus the 6NT -> 7NT rows heading the worst list).  PD on that
# bucket: -155 / -182 / -144 / -196.  Untraced.
# Gate failure: one deal BBA opened, `2C - 2D - 2NT - 3H - 4S - 4NT - 5C -
# 5D`, where our lead-directing X of 5D (base) becomes a pass (on) and BBA
# reaches 6S instead of 5S: -11 / -11 / -13 / -13 IMPs.  The knob reaches
# their auction through the reading path (our book mirrored onto BBA's 4S).
# Shipped default-on 2026-11-23 on this read.
#
# Resumable; SEED_BASE persists in $R/two-notrump-super-accept.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-two-notrump-super-accept.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for two-notrump-super-accept)

log "=== two-notrump-super-accept A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul" --no-ns-two-notrump-super-accept
    arm on "$vul"
    gatepair on base "$vul" || log "isolation gate on vs base ($vul) FAILED, scoring anyway"
    diffpair on base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.on.vs.base.$vul.plain.txt")
    log "pre-count on vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff on base "$vul" --off-no-ns-two-notrump-super-accept
done
log "=== two-notrump-super-accept A/B done"
