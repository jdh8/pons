#!/bin/sh
# ab-landy-doubler-nt.sh — opener's **notrump out** over the N1j Landy
# doubler's advanced major (`1NT (2♣) X (2♥)`, `1NT (2♣) X (2♠)`).
#
#   SKIP_BUILD=1 JOBS=12 PER_SHARD=192000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-doubler-nt.sh ab-results/landy-doubler-nt \
#       >ab-results/landy-doubler-nt.log 2>&1 < /dev/null &
#
# What is being tested.  The 2026-08-27 census made `2♣` Landy the top cost
# bucket over our 1NT (551 bd, −275 plain, docs/one-notrump-competitive.md), and
# the bucket cut that opened §N1's re-work names this seat: our values `X`,
# their major, and everyone passing.  Split by our first call over their `2♣`,
# both vulnerabilities pooled:
#
#   -    281 bd  plain −307  PD   +7     X   67 bd  plain −75  PD  +8
#   3NT   36 bd  plain  +58  PD  +38    2NT  58 bd  plain  +6  PD +51
#
# and inside that `X`, `X (2♥)` passed out is 22 bd at −47 plain / −44 PD —
# **all** of it the `hcp 16+`-with-a-stopper cell (11 bd, −45 / −46), against
# +10 / +12 for the 15-counts and −11 / −18 for the stopperless.  On those same
# deals BBA bids and makes `3NT` at the other table for +400/+600.
#
# The mechanism is a reading fact, not a taste: the values double is authored
# `hcp(8..)` but `landy_bba_responder` ranks the ungated `3NT`@168 above it, so
# it reads back as **8–9** (`probe-call-reading --their-2c-landy "1N (2C) X
# (2H)"`).  16 opposite a known 8–9 is 24, and the hand holding their major
# stopped is *opener's* — the doubler is short in their major, which is why it
# could not bid `3NT` itself.  Yet the seat has no book node: `probe-decision
# "AJ5.KQ5.A932.Q54" "1NT (2♣) X (2♥)"` reads `fallback: Some(0)` and takes
# `Pass` at 11.9 with `2NT` a distant 2.3.
#
#   3NT  @135  `hcp(16..) & stopper_in(major)`, both legs, `Pass`@0 catch-all.
#              The gate, the weight and the ordering are
#              `kokish_kraft_doubler_major_answer`'s notrump out ported —
#              `competition.multi_doubler_notrump`, which won 4/4 cells in the
#              Multi lane on 2026-08-27.
#
# The hypothesis is specific and falsifiable: **the `X (2♥)` pass-outs are the
# `hcp 16+` stopper cell, so authoring the notrump out moves that branch to
# non-negative on both scorers.**  Two things falsify it.  (1) The node
# *shadows the floor* at this seat, and the floor's second choice there is a
# natural `3♦` at 6.3 — if the loss is the shadowed `3♦`, the arm reads a wash
# or worse and the follow-up is a natural-`3m` rung in the same table, not a
# different gate.  (2) The `(2♠)` leg is armed on the house's symmetric-table
# idiom, but the same cut reads its `hcp 16+` stopper cell at **+0 plain /
# +30 PD over 7 boards** — passing may be right there.  A split verdict (♥ leg
# wins, ♠ leg loses) is a real outcome, and the forensic that separates them is
# `ab-dump-diff --show` filtered by which major they advanced.
#
# Two arms, one seed set, `--their-2c-landy` is derived (`common::vs_bba_agreements`
# arms it for every BBA arm; see `bba-gen --help`), `--filter-landy` on both:
#
#   base   the shipped default — the seat is the floor's and it passes
#   nt     plus `3NT`@135 (`competition.landy_doubler_notrump`)
#
# ENRICHMENT.  This is the first runner to use `--filter-landy`, whose paired
# raw-hand scan puts the direct `1NT (2♣)` lane at **17.8%** of accepted boards
# against 0.50% under a plain `--filter-1nt` (bba-gen's own measurement at seed
# 424242).  The armed cell is ~3% of that lane, so 2.304M bd/arm/vul should fire
# on the order of 10k boards — decisive rather than marginal, which is the point
# for a rung this thin.  Arms under this flag **pair only with each other**:
# never diff one against a `--filter-1nt` arm at the same seed, and the headline
# is IMPs per *accepted* deal, so read the per-fired paired diff.
#
# Scoring.  Plain AND perfect defense off the decision table in
# docs/measurement.md; `sddiff` is the tie-breaker, never the verdict.  This is
# a **bid-more** knob — plain credits the games we now reach, PD prices the ones
# that fail — so a `win | loss` row is real, not an artifact, and both cells of
# each vulnerability have to be read.  Run `probe-divergence --gate-opener ours`
# BEFORE any headline: it must read **0 foreign**.
#
# Before declaring a loss dead, trace the worst divergent boards.  The named
# suspects, in order: the shadowed floor `3♦`; the `(2♠)` leg; and the 15-count,
# which this rung deliberately leaves passing (the Multi lane needed a second,
# separately seeded knob — `multi_doubler_minimum_notrump` — to extend the same
# ladder down one point, and its `2NT` rung exists on one leg only).
#
# Resumable: an existing arm dir, gate, or diff file is skipped, and SEED_BASE
# persists in $R/landy-doubler-nt.seed.  Iron rule: do NOT edit `src/` while
# this runs — `ab-lib.sh` exports SKIP_BUILD=1 and has already built once.
R=${1:?usage: ab-landy-doubler-nt.sh RESULTS_DIR}
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

SEED_BASE=$(seed_for landy-doubler-nt)
log "=== landy-doubler-nt SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --filter-landy
    arm nt   "$v" --filter-landy --ns-landy-doubler-notrump

    gatepair nt base "$v"
    diffpair nt base "$v"
    sddiff   nt base "$v"
done

log "landy-doubler-nt done"
