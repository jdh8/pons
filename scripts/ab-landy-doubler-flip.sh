#!/bin/sh
# ab-landy-doubler-flip.sh — §N1l's flip: the doubler's rebid ladder cut down
# to the rungs the 2026-08-28 per-rung split actually credited.
#
#   JOBS=24 BOARDS=4608000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-doubler-flip.sh ab-results/landy-doubler-flip \
#       >ab-results/landy-doubler-flip.log 2>&1 < /dev/null &
#
# What is being tested.  The full ladder (`landy_doubler_rebids`) measured
# MIXED — SD-PD +0.523 none / −0.741 both IMPs/fired — and the split says the
# mixture is *by rung*, not by noise: the penalty `X`@155 priced +7.489 (none)
# / +9.196 (both) plain per fired, which is the entire vulnerable plain win
# (+63,332 of +90,066 IMPs), while every constructive rung was positive white
# and negative red, worst the `2NT` invitation (−3.695 PD/fired, and its
# declined half loses BOTH scorers at both vul).  So two smaller arms:
#
#   base   today's `main` — the seat is the floor's
#   px     `competition.landy_doubler_px`: the penalty `X`@155 and the
#          `Pass`@0 catch-all, every constructive rung deleted
#   white  `competition.landy_doubler_white`: `px` plus `3NT`@150, and the rest
#          of the constructive family — the `2NT`@145 invitation and the
#          natural `3♣`@100 / `3♦`@99 — gated `& !vulnerable()`.  Only `4NT`
#          is deleted; it never fired once in either measured cell.  Keeping
#          the minors pays §N1l's completeness debt: opener's `3♣ -` / `3♦ -`
#          answer tables are authored here for the first time (`3NT` on a
#          maximum holding their suit, else pass).
#
# Why the gate and not deletion.  Re-reading `div.reb.vs.base.*.jsonl` grouped
# by first differing call says the constructive family splits by **colour**,
# not by kind.  Per fired, plain / PD:
#
#            non-vulnerable          vulnerable
#   2NT      +1.888 / −1.667         +0.733 / −3.695     (36% of divergences)
#   3♣       +1.953 / −0.797         +0.338 / −3.071     (25%)
#   3♦       +1.696 / −0.607         +0.183 / −2.481     (21%)
#   X        +7.489 / −0.091         +9.196 / −0.148     (12%)
#   -        −0.445 / +2.929         +1.206 / +5.048     (6%)
#
# Every rung flips sign with colour, and the natural minors are the *cheaper*
# half white — so §N1l's first sketch (delete the minors, gate the `2NT`) would
# have kept the worst white rung and dropped the best two.  `4NT` and `3NT`
# together account for 40 of 129,765 divergences: `4NT` never fires, `3NT`
# stays because it is the table's only game rung.
#
# **A fresh SEED_BASE is mandatory.**  Both arms are rung sets *selected from*
# seed 1787917699's own split; re-measuring them on that seed is textbook
# overfit.  `seed_for` writes a new one into $R on first use — do not copy the
# old `.seed` file in.
#
# Falsifiers, in order.
#   1. **The `X` win was a seed artifact.**  It is the rung the split picked
#      out of the same data, so the honest test is whether it survives a fresh
#      board stream.  If `px` reads flat on plain DD at both vuls, the
#      +7.489/+9.196 was selection, and §N1l is closed for good.
#   2. **The attribution was wrong: the ladder wins as a whole, not by rung.**
#      If `white` beats `px` at *both* vulnerabilities, then the constructive
#      rungs were not the drag — the rungs interact (a hand that would double
#      badly bids `2NT` instead), and a per-rung split cannot see that.  The
#      `white vs px` pair at the foot of each vul cell is what reads this.
#      Note the arms are near-identical at both-vul by construction — the gate
#      leaves only `3NT` between them — so that cell is a consistency check and
#      the white cell is where the pair carries information.
#   3. **Vulnerability is the wrong axis for the invitation.**  The gate is
#      built on a *conditional* read: the constructive rungs were positive on
#      the ordinary rows non-vulnerable.  Those rows are selection-biased (the
#      `-` row is hands the floor chose to bid), so if `white` loses white too,
#      the invitation is simply bad and the answer is `px`.
#   4. **PD is blind to `px` by construction.**  It is a pure doubling knob:
#      perfect defense already doubles every failing contract, so it keeps the
#      whole cost of a real penalty double and none of the benefit
#      (docs/measurement.md's domain addendum).  Read `px` on plain DD, with
#      SD-PD as the tie-break, and never call it dead on its PD row alone.
#
# Scoring: plain AND perfect defense off docs/measurement.md's decision table;
# sd-lead tie-breaks.  `probe-divergence --gate-opener ours` must read 0 foreign
# BEFORE any headline.  Resumable; SEED_BASE persists in $R/landy-doubler-flip.seed.
# Iron rule: do NOT edit `src/` while this runs.
R=${1:?usage: ab-landy-doubler-flip.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-doubler-flip)
log "=== landy-doubler-flip SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base  "$v" --filter-landy
    arm px    "$v" --filter-landy --ns-landy-doubler-px
    arm white "$v" --filter-landy --ns-landy-doubler-white

    for a in px white; do
        gatepair "$a" base "$v"
        diffpair "$a" base "$v"
        sddiff   "$a" base "$v"
    done
    # The ordering question, paired on the same boards: does adding the two
    # notrump rungs back help or hurt?  Falsifier 2 reads off this pair.
    gatepair white px "$v"
    diffpair white px "$v"
    sddiff   white px "$v"
done

log "landy-doubler-flip done"
