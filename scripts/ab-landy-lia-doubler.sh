#!/bin/sh
# ab-landy-lia-doubler.sh — §N1-lia package A: the Landy doubler's rebid seat,
# the catch-all deleted and exactly-three trumps bought back cell by cell.
#
#   JOBS=24 PER_SHARD=192000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-lia-doubler.sh ab-results/landy-lia-doubler \
#       >ab-results/landy-lia-doubler.log 2>&1 < /dev/null & disown
#
# What is being tested.  The shipped `px` arm's `Pass`@0 catch-all is measured
# wrong and shipped anyway: it gives the node finite mass at three trumps or
# fewer, shadowing a floor that was already acting there — a takeout-shaped
# values double opener pulls to `3NT` ~49.5% of the time — at a summed cost of
# −14,171 IMPs plain non-vulnerable (+889 vulnerable, a wash) on the §N1l-flip
# stream.  The deletion was blocked on disclosure, not arithmetic; the
# `comp:landy-penalty` tag now reads "length or honour strength in their
# major", one claim across every cell, which unblocks it.  Two three-card
# cells then re-buy exactly-three trumps by top-honor class, cumulative:
#
#   base     today's `main` — the shipped px arm, catch-all intact
#   nocatch  `landy_doubler_catchall=false`: X@155 on 4+, all else floors
#   hon      + `landy_doubler_three_honors`: X@154 on len 3 & 2+ top honors
#   cells    + `landy_doubler_three_small`:  X@153 on len 3 & ≤1 top honor
#
# The prior on the cells comes from the sibling lane's re-slice
# (nt_high_overcall_x_leave_in/_three): `len3 hon0` +0.62/+1.85 plain per
# fired, `len3 hon1` −0.75/+0.37 — at three cards a lone A/K/Q *is* the
# stopper that made 3NT real, three small is pure values.  `hon2+` is
# genuinely unmeasured, so both cells run.
#
# Falsifiers, in order.
#   1. **The −14,171 was the same-seed split's artifact.**  It was summed off
#      the §N1l-flip divergence stream; a fresh stream must reproduce the
#      sign.  If `nocatch vs base` reads flat or negative plain NV, the
#      catch-all was innocent and the arm closes.
#   2. **The floor it un-shadows misreads the X.**  The book's X projects
#      length (3+/4+ per arm); the floor's short double at the same seat
#      publishes nothing.  If nocatch loses concentrated on boards where the
#      floor doubled short and partner sat, the re-worded tag did not save the
#      mechanism and the cells (which shrink the floor's share) should read
#      *better* than nocatch — the adjacent pairs decide.
#   3. **The honors cell buys a stopper it should keep.**  The sibling slice
#      says the lone-honor cell costs; if `hon vs nocatch` is negative while
#      `cells vs hon` is positive, the split is real and only the small cell
#      ships.
#   4. **PD is blind to every arm here** (docs/measurement.md's domain
#      addendum): penalty rungs are arbitrated on **plain DD**, PD reported
#      double-blind, sd as tie-break.
#
# Scoring: plain AND perfect defense off docs/measurement.md's decision table;
# sd-lead tie-breaks.  `probe-divergence --gate-opener ours` must read 0
# foreign BEFORE any headline.  Resumable; SEED_BASE persists in
# $R/landy-lia-doubler.seed.  Iron rule: do NOT edit `src/` while this runs.
R=${1:?usage: ab-landy-lia-doubler.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-lia-doubler)
log "=== landy-lia-doubler SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base    "$v" --filter-landy
    arm nocatch "$v" --filter-landy --no-ns-landy-doubler-catchall
    arm hon     "$v" --filter-landy --no-ns-landy-doubler-catchall \
        --ns-landy-doubler-three-honors
    arm cells   "$v" --filter-landy --no-ns-landy-doubler-catchall \
        --ns-landy-doubler-three-honors --ns-landy-doubler-three-small

    for a in nocatch hon cells; do
        gatepair "$a" base "$v"
        diffpair "$a" base "$v"
        sddiff   "$a" base "$v"
    done
    # The adjacent pairs isolate each cell on the same boards — falsifiers 2
    # and 3 read off these.
    gatepair hon nocatch "$v"
    diffpair hon nocatch "$v"
    sddiff   hon nocatch "$v"
    gatepair cells hon "$v"
    diffpair cells hon "$v"
    sddiff   cells hon "$v"
done

log "landy-lia-doubler done"
