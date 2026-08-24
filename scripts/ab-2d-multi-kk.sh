#!/bin/sh
# ab-2d-multi-kk.sh — N4-KK: the Kokish–Kraft counter-defense to their `(2♦)`
# Multi (docs/one-notrump-competitive.md §N4-KK, docs/one-notrump-multi.md).
#
#   JOBS=12 PER_SHARD=19200 setsid nohup scripts/idle-run.sh \
#       scripts/ab-2d-multi-kk.sh ab-results/2d-multi-kk \
#       >ab-results/2d-multi-kk.log 2>&1 < /dev/null &
#
# What is being tested.  `competition.multi_kokish_kraft`
# (`--ns-multi-kokish-kraft`) swaps responder's whole `1NT (2♦)` subtree for the
# Eric Kokish–Beverly Kraft table, the most complete published package for this
# exact object (`docs/ai-bidder/multi-landy-2d-counter-defense-research.md` §1).
# Five things move at once — this is a whole-table arm, not a rung:
#
#   X          `hcp 8+`, no shape promise (v7: BBA's mimic `hcp 6+`), so the
#              6-7 band takes a *designed* neutral pass with its own delayed
#              table instead of doubling
#   2NT / 3♣   floorless transfers to ♣ / ♦ on a six-card suit — a preempt of
#              their unknown major *and* the start of a game force; they
#              replace the weak `2NT` relay, which dies structurally
#   3♠         both minors, game-forcing, 5-4 or better (v7: the forced `3♠`→♣
#              game force, which the `2NT` transfer now carries)
#   -          a *designed* neutral pass, with its own delayed table once they
#              name the major (v7 authors nothing at that seat)
#   X again    penalty (v4's trump-length gate), while the double after the
#              *neutral pass* is takeout — the delayed-double split every
#              exact-object source in the survey makes, where v7 has one
#              takeout double and no pass table at all
#   4♥ / 4♠    the uncontested direct slam-try tier copied under the overcall,
#              RKCB ladder included
#
# Unchanged and shared with the shipped lane: `3♦`/`3♥`, the weak `2♥`/`2♠`
# escape and its interfered tail, Leaping Michaels `4♣`/`4♦`, and the whole
# double-family answer set.  One deliberate departure from the design sketch is
# recorded at the rule: `3NT` keeps its both-majors stopper gate, because bare
# it confines the values double to `points 8..9` and re-runs the stopperless
# blast perfect defense priced at −3.7/−4.3 a board in N4 v2/v3.
#
# Two arms, one seed set, `--their-2d-multi` on both so the only difference is
# the table:
#
#   base   the shipped v7 lane
#   kk     the Kokish–Kraft swap
#
# `--filter-1nt` (balanced 15-17 somewhere, a raw-hand test applied BEFORE any
# bidding) rides both arms so they deal the same board set and stay seed-aligned
# for the paired diffs.  Headline is IMPs per *accepted* deal, with
# `per-board = conditional mean × trigger density` alongside.
#
# Scoring.  Plain AND perfect defense, read off the decision table in
# docs/measurement.md; `sddiff` is the tie-breaker.  Read
# `probe-divergence --gate-opener ours` BEFORE any headline: this campaign's
# mirror-read leak makes a counter knob a reading knob, and the gate must read
# **0 foreign**.
#
# Read the gate first, and know why.  The competitive book is keyed by call
# shape with no seat gate on the reader side, so when *they* open 1NT and *we*
# overcall a natural `2♦`, their `2NT`/`3♣` decode off this table.  The shipped
# lane leaks a strength claim there; K–K's floorless transfers leak a hard
# **six-card suit** — the campaign's mirror-read leak one notch louder.  Every
# raw headline in this bucket has historically been 60-70% foreign, so
# `probe-divergence --gate-opener ours` is not a formality here.
#
# Interpretation caveat.  The floorless minor transfers are partly *obstructive*
# — they take away their advancer's whole pass-or-correct room — and DD play
# scoring is blind to obstruction, concealment and lead effects.  A plain-DD
# wash with a PD win is the shippable row; a plain-DD loss is not automatically
# the idea's fault (docs/measurement.md).  Before declaring a loss dead, trace
# the worst divergent boards: unauthored continuation and over-broad trigger
# first.
#
# Trace list if this reads a loss, in order (docs §N4-KK "Known residues"):
# the doubler's second `X` is *penalty* here, so v7's takeout rung — the one
# BBA rung that measured positive on both scorers, +2.4 plain / +1.6 PD per
# fired NV — is gone; then the uncapped floorless transfers, which take every
# strong long-minor hand off the double.
#
# Follow-ups already recorded, each owed its own seed: the `3♠` 5-5 fallback
# (tighten `len(m,5..)` in both minors), a bare stopperless `3NT`, a ceiling on
# the minor transfers, and widening the `4M` band to `15..=18`.
#
# Resumable: an existing arm dir, gate, or diff file is skipped, and SEED_BASE
# persists in $R/2d-multi-kk.seed.  Iron rule: do NOT rebuild binaries while
# this runs.
R=${1:?usage: ab-2d-multi-kk.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
PROBE=target/release/examples/probe-divergence

gatepair() {
    on=$1; off=$2; vul=$3
    out="$R/gate.$on.vs.$off.$vul.txt"
    [ -s "$out" ] && { log "skip $out (exists)"; return 0; }
    log "isolation gate $on vs $off ($vul)"
    "$PROBE" "$R/$on-$vul" "$R/$off-$vul" --gate-opener ours >"$out"
}

SEED_BASE=$(seed_for 2d-multi-kk)
log "=== 2d-multi-kk SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --their-2d-multi --filter-1nt
    arm kk   "$v" --their-2d-multi --ns-multi-kokish-kraft --filter-1nt

    gatepair kk base "$v"
    diffpair kk base "$v"
    sddiff   kk base "$v"
done

log "2d-multi-kk done"
