#!/bin/sh
# ab-landy-kk.sh — §N1n, the full Kokish–Kraft minor core over their Landy
# `(2♣)` (`competition.defense_2c_landy_kk`), one package arm.
#
#   JOBS=12 PER_SHARD=192000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-kk.sh ab-results/landy-kk \
#       >ab-results/landy-kk.log 2>&1 < /dev/null &
#
# What is being tested.  Five rungs of `landy_bba_responder` move; everything
# else — `X`@145, `2♦`@140, `P`@0, both `3NT` rows, the splinters — carries over
# verbatim, so the two tables differ exactly where the source does:
#
#   call   Kokish–Kraft (armed)                  shipped N1j ladder
#   2♥     both minors, competitive (hcp ..=7)   GF takeout, exact ♥ doubleton
#   2♠     both minors, INV+ (hcp 8..)           GF takeout, exact ♠ doubleton
#   2NT    weak escape, EITHER six-card minor    wide transfer to clubs
#   3♣     diamonds, GF, unbalanced              transfer to diamonds, any str.
#   3♦     clubs, GF, unbalanced                 (does not exist)
#
# The trade, stated so the verdict can be attributed: we give up the shipped
# lane's **doubleton encoding** (real right-siding information, spent here on a
# strength split) and its **wide transfers** (escape-only and GF here); we gain
# a weak escape into either minor, a `2♠` that separates INV from competitive,
# and a `3♦` rung that does not exist today.  Those two losses are the named
# suspects and **they fail differently** — on a loss, trace the worst divergent
# boards BY ROW before parking (`park/landy-kk-core`), never as one pool.
#
# Falsifiers.
#   1. **The routing hole is known and deliberate.**  The ungated `3NT`@168 is
#      `points(10..)` and sits above the values `X`@145, so an eight-count with
#      a six-card minor blasts `3NT` instead of doubling — the table's own
#      P/X-at-8, GF-at-10 routing is not coherent end to end.  The one-line
#      reversible alternative (re-gate that row to `hcp(10..)` in this variant
#      only) is flagged in §N1n and **not taken**.  A loss concentrated on
#      one-suited eight-counts is that hole, not the KK core.
#   2. **`landy_doubler_rebids` (§N1l) is a prerequisite, not a sibling** —
#      it is where this table's 8-9 one-suited minors are supposed to go.  If
#      N1l did not ship, those hands land on a floored seat and this arm is
#      being charged for N1l's absence.  Control MUST be post-Phase-1 `main`.
#   3. **The bands are `hcp`, not `points`, on purpose.**  A call that shows
#      four-four in the minors is graded up by the shape it announces;
#      `points(..=7)` on the escape relay leaves a seven-count with a six-card
#      minor passing — the precise hand the relay exists to rescue.  Do not
#      "fix" a loss by moving the bands back to points without re-probing that.
#
#   base   post-Phase-1 `main` — the shipped N1j (BBA-derived) ladder
#   kk     plus `competition.defense_2c_landy_kk`
#
# Scoring: plain AND perfect defense off docs/measurement.md's decision table;
# sd-lead tie-breaks.  Gate must read 0 foreign before any headline.
# Resumable; SEED_BASE persists in $R/landy-kk.seed.  Do NOT edit `src/` while
# this runs.
R=${1:?usage: ab-landy-kk.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-kk)
log "=== landy-kk SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --filter-landy
    arm kk   "$v" --filter-landy --ns-defense-2c-landy-kk

    gatepair kk base "$v"
    diffpair kk base "$v"
    sddiff   kk base "$v"
done

log "landy-kk done"
