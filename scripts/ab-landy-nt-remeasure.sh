#!/bin/sh
# ab-landy-nt-remeasure.sh — §N1-lia package D: `landy_notrump_no_major`
# re-measured against the routing `main` actually plays, one arm.
#
#   JOBS=24 PER_SHARD=192000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-nt-remeasure.sh ab-results/landy-nt-remeasure \
#       >ab-results/landy-nt-remeasure.log 2>&1 < /dev/null & disown
#
# Run AFTER packages A, B and C are decided: control = the then-current `main`
# HEAD, fresh SEED_BASE.
#
# What is being tested.  `competition.landy_notrump_no_major` — both `3NT`
# rungs gain `len(♥, ..=3) & len(♠, ..=3)`, so a game hand holding four-plus of
# a major they showed falls through to the `X` instead of declaring.  Stoppers,
# transfers and the two-suited family untouched.
#
#   base   today's `main`
#   nt     + `--ns-landy-notrump-no-major`
#
# Why this is not simply a re-run of §N1p's `nt` pair.  §N1p measured `nt` with
# `landy_major_jam` held **off** on both sides, and lost at both colours
# (`ab-landy-notrump-shape.sh`, whose header keeps that experiment's record).
# Two things have changed since:
#
#   1. **The seat above the double was broken then.**  §N1p's own falsifier 2
#      says so: the doubler's rebid was the floor's, and it pulled a values
#      double to `3NT` 49.5% of the time on two trumps, so the arm partly
#      measured that floor.  §N1-lia package A deleted the shadowing catch-all
#      and authored the three-card cells (shipped default-on 2026-08-30), which
#      is the repair that unblocks this re-measure.
#   2. **The jam shipped.**  `landy_major_jam` has been default-on since
#      2026-08-30 and package C's Texas since 2026-08-31, so a strong six-card
#      major now leaves via `4♣`/`4♦` — it never reaches `3NT`@168 and `nt`
#      cannot move it.  Holding the jam off to reproduce §N1p's framing would
#      measure a pool `main` no longer routes that way.
#
# So this runner takes the bare default as `base`, per the iron rule that a
# ship decision is measured against the real routing.  The moving pool is
# therefore the **four- and five-card** major game hands only, which is
# narrower than §N1p's and is the whole ship-relevant question.  This is a
# deliberate departure from the plan's "the `nt` pair only" wording, which was
# written before the jam shipped.
#
# Falsifiers, in order.
#   1. **PD is structurally blind to `nt`.**  Perfect defense already doubles
#      every failing contract, so it keeps the whole cost of a real penalty
#      double and none of the benefit (docs/measurement.md's domain addendum).
#      Arbitrate on plain DD with sd-PD as the tie-break; never call it dead on
#      its PD row alone.
#   2. **The displaced `3NT`s were making.**  If `nt` loses plain, split the
#      divergence by `call_off == "3NT"` and read what replaced it: either "we
#      defended a making game" (the idea is dead) or "opener pulled the double
#      badly" (a continuation defect, repairable — and now an *authored* seat,
#      so the pull is the book's to fix, not the floor's).
#   3. **A's repair does not reach far enough.**  A fixed the doubler's own
#      rebid.  If `nt` still loses on boards where the double was passed out or
#      answered below game, the defect is one seat further on and the arm
#      closes until that seat is authored.
#   4. **`--filter-landy` admits only strictly balanced 1NT openers** (§N1's
#      flagged item 5), so the wide-shape slice is invisible.  It does not
#      invalidate the A/B; it bounds the claim.
#
# Scoring: plain AND perfect defense off docs/measurement.md's decision table;
# sd-lead tie-breaks.  `probe-divergence --gate-opener ours` must read 0
# foreign BEFORE any headline.  Resumable; SEED_BASE persists in
# $R/landy-nt-remeasure.seed.  Iron rule: do NOT edit `src/` while this runs.
R=${1:?usage: ab-landy-nt-remeasure.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-nt-remeasure)
log "=== landy-nt-remeasure SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --filter-landy
    arm nt   "$v" --filter-landy --ns-landy-notrump-no-major

    gatepair nt base "$v"
    diffpair nt base "$v"
    sddiff   nt base "$v"
done

log "landy-nt-remeasure done"
