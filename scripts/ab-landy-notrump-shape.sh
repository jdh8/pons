#!/bin/sh
# ab-landy-notrump-shape.sh — §N1p: an unlimited values double over their Landy
# `2♣`, bought by restricting `3NT` rather than by promoting the `X`.
#
#   JOBS=24 PER_SHARD=192000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-notrump-shape.sh ab-results/landy-notrump-shape \
#       >ab-results/landy-notrump-shape.log 2>&1 < /dev/null & disown
#
# What is being tested.  `landy_bba_responder`'s `X`@145 is constrained
# `hcp(8..)` — unlimited on top — but the table's **ungated** `3NT`@168 on bare
# `points(10..)` outranks it, so every ten-plus-point hand declares and the
# double is capped at nine points (`probe-call-reading --their-2c-landy` reads
# partner back as `points 8..9`).  That cap is flagged item 2 of §N1 and has
# never been measured either way.
#
#   base   today's `main`
#   nt     `competition.landy_notrump_no_major`: both `3NT` rungs gain
#          `len(♥, ..=3) & len(♠, ..=3)`, so the game hands holding four-plus
#          of a major they showed fall through to the `X`.  Stoppers,
#          transfers and the two-suited family untouched — a six-card minor
#          still transfers and short stoppers still count.
#   jam    `nt` plus `landy_major_jam`: `4♠`@172 / `4♥`@171 on
#          `len(major, 6..) & points(10..)`, above the restricted `3NT`@168 and
#          below the transfers, with opener sitting.  Weak six-carders keep
#          defending.
#
# Why the notrump and not the double.  Promoting the `X` above `3NT`@168 buys
# the same hands, but promoting it above the *gated* `3NT`@180 means promoting
# it above the transfers and the GF both-minors family too — there is no weight
# between 180 and 178.  Restricting `3NT` moves exactly the intended traffic and
# leaves three separately-defended orderings alone.  It also carries the
# reading for free: `reading.bid_exclusion` intersects each rule with what its
# heavier siblings deny, so the double's published reading widens off
# `points 8..9` with no new slug, no alert change and no `.bbsa` row.
#
# Priors, all from this lane.
#   * N1d priced taking eight-to-nine point hands *off* this double at
#     −0.92/−2.53 PD per fired, and flipping them back at +2.0…+5.1.
#   * §N1m's oracle (`probe-landy-opener-oracle`, 103,653 + 81,023 seat boards)
#     prices defending their major **doubled** at +3.5…+8.1 IMPs/board in every
#     four-plus-trump bucket at both vulnerabilities, minimum or maximum, with
#     a stopper or without, PD column flat.
#   * The 2026-08-27 census makes `2♣` Landy the lane's top cost by total,
#     −275 IMPs on 551 boards.
#
# Falsifiers, in order.
#   1. **PD is structurally blind to `nt`.**  Perfect defense already doubles
#      every failing contract, so it keeps the whole cost of a real penalty
#      double and none of the benefit (docs/measurement.md's domain addendum).
#      Read `nt` on plain DD with SD-PD as the tie-break, and never call it
#      dead on its PD row alone.
#   2. **The displaced `3NT`s were making.**  If `nt` loses plain, split the
#      divergence by `call_off == "3NT"` and read what replaced it: either "we
#      defended a making game" (the idea is dead) or "opener pulled the double
#      badly" (a continuation defect, repairable).  The seat above the double
#      is the floor's on `main`, and it pulls a values double to `3NT` 49.5% of
#      the time on two trumps — so this arm partly measures that floor.
#   3. **The jam is obstruction, which DD cannot see.**  A negative
#      `jam vs nt` on plain DD is partly the harness, not the idea (the iron
#      rule).  Read the made/down split of the `4M` contracts before the IMP
#      mean.  Its other risk is the sit node: opener passes `4M`
#      unconditionally, so the arm forgoes slam on the fifteen-plus slice.
#   4. **`--filter-landy` admits only strictly balanced 1NT openers**
#      (§N1's flagged item 5), so the wide-shape slice is invisible to all
#      three arms.  It does not invalidate the A/B; it bounds the claim.
#
# Scoring: plain AND perfect defense off docs/measurement.md's decision table;
# sd-lead tie-breaks.  `probe-divergence --gate-opener ours` must read 0 foreign
# BEFORE any headline.  Resumable; SEED_BASE persists in
# $R/landy-notrump-shape.seed.  Iron rule: do NOT edit `src/` while this runs.
R=${1:?usage: ab-landy-notrump-shape.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-notrump-shape)
log "=== landy-notrump-shape SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --filter-landy
    arm nt   "$v" --filter-landy --ns-landy-notrump-no-major
    arm jam  "$v" --filter-landy --ns-landy-notrump-no-major --ns-landy-major-jam

    for a in nt jam; do
        gatepair "$a" base "$v"
        diffpair "$a" base "$v"
        sddiff   "$a" base "$v"
    done
    # The jam rung priced alone, on the same boards: does bidding the game beat
    # defending it doubled?
    gatepair jam nt "$v"
    diffpair jam nt "$v"
    sddiff   jam nt "$v"
done

log "landy-notrump-shape done"
