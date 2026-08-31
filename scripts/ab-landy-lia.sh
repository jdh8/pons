#!/bin/sh
# ab-landy-lia.sh — §N1-lia package B: Lia's re-rung counter ladder over their
# Landy `2♣`, one arm — a permutation, it cannot be decomposed.
#
#   JOBS=24 PER_SHARD=192000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-lia.sh ab-results/landy-lia \
#       >ab-results/landy-lia.log 2>&1 < /dev/null & disown
#
# Run AFTER package A (ab-landy-lia-doubler.sh) has been decided and its
# winner shipped: control = the then-current `main` HEAD, fresh SEED_BASE.
#
# What is being tested.  `competition.defense_2c_landy_lia` — IntoBridge's
# counter is ~80% our shipped N1j table, and its two better-informed deltas
# ship together because they are one permutation of the same rungs:
#
#   base  today's `main` — the N1j BBA ladder
#   lia   the minor ladder a full level lower, matching BBA's own coherent
#         self-play tree (`2♠`→♣ 5.9%, `3♣`→♦ 6.8%,
#         docs/ai-bidder/bba-1nt-landy-tree.md): `2♠` = 5+♣ weak or GF, `2NT`
#         = 7+♦ / good 6 / GF, natural `3♣`/`3♦` invitations restored, `2♥`
#         the only GF takeout (answer priority reversed: minors first, `2NT` =
#         spade stopper, `2♠` asks), opener answering the minor rungs by
#         length instead of completing, the N4-KK `4m` slam machinery re-hung
#         byte-identical on every leg.
#
# Falsifiers, in order.
#   1. **The N1c right-siding trade was right.**  N1h/N1i measured the
#      invitation-for-right-siding trade at `3♣ ← 2NT` −2.19 PD; lia partially
#      unwinds it (opener declares only the 3+-fit legs, and the invitations
#      return).  If the loss concentrates on `3♣`/`3♦`-invite boards, the
#      trade was right and lia's invitations are the drag — re-measure with
#      w167/w166 deleted before closing.
#   2. **Five-card club sign-offs are too light.**  The old transfer demanded
#      six; lia's `2♠` admits five and lands in `3♣` on a 5-2 when opener
#      holds a doubleton and responder signs off.  Read the `2♠`-weak
#      divergences' made/down split before the mean.
#   3. **The reversed takeout priority mis-sides notrump.**  Minors-first
#      means opener declares fewer notrumps; DD cannot see siding, so a plain
#      wash with a PD move is the expected signature either way
#      (docs/measurement.md) — read both scorers.
#   4. **The diamond rung starves.**  `2NT` narrowed to 7+/good-6/GF; the bad
#      weak six-carder with 7+ hcp now passes.  If the loss sits on passed
#      hands holding six diamonds, the hole is the `2♦` cap, not the ladder.
#
# Scoring: plain AND perfect defense off docs/measurement.md's decision table;
# sd-lead tie-breaks.  `probe-divergence --gate-opener ours` must read 0
# foreign BEFORE any headline.  Resumable; SEED_BASE persists in
# $R/landy-lia.seed.  Iron rule: do NOT edit `src/` while this runs.
#
# VERDICT (2026-08-31, SEED_BASE=1788122360, control 59cd46ee, 4.6M
# boards/arm/vul, both gates 0-foreign): **measured loss, stays default off.**
#   plain  +0.0050 ±0.0012 NV / -0.0384 ±0.0014 BV
#   PD     -0.0756 ±0.0016 NV / -0.1210 ±0.0018 BV
#   sd     +0.0374 / +0.0016 plain, -0.0289 / -0.0691 PD
# Plain splits by colour (BV is 27 sigma) and PD is an order of magnitude past
# package A's doubling artifact -- lia *removes* our penalty doubles (5.51% of
# divergent boards vs base's 7.79%), which is what perfect defense pays for.
#
# Forensic (probe-divergence --imps, bucketed by first differing call and by
# responder's hand) -- four named defects, none of them the concept:
#   1. Contested tails unauthored.  Every lia node needs the opponents to have
#      passed, so any opponent bid drops the rest to a floor with no forcing
#      channel: 95%/94% of the `2NT` rung's whole loss, -63,950/-80,789 on
#      `2♠`.  Not just the advancer -- they pass over `2♠` 85% of the time and
#      enter after opener's length answer.
#   2. Falsifier 2 confirmed, sharper: the weak five-card sign-off is
#      *vulnerability-dependent* -- uncontested weak, exactly-5 clubs is
#      +1.405 NV / -0.803 BV per fired while 6c (+0.993/+0.668) and 7+c
#      (+1.849/+1.683) win at both colours.  That cell is 45% of BV's PD loss.
#   3. Falsifier 4 confirmed: the `2NT` cap starves weak six-card diamond hands
#      into passing, -35,827 NV / -32,432 BV.
#   4. The sole `2♥` takeout is the worst per-fired rung (-4.07/-5.00).
# Falsifier 1 is REVERSED, not refuted: the N1c right-siding trade was wrong on
# plain DD -- the restored natural invitations are the ladder's biggest win
# (`3♣` +85,613 NV / +35,920 BV).  At BV defects 1-3 sum to -176,261 of a
# -176,837 total, so the rest of the ladder is roughly break-even.
# Repair queue before any re-measure, in size order: contested tails; a 6+ club
# floor for the weak `2♠` leg when vulnerable; a rung for the starved diamonds;
# the 2=3=4=4 merge into `2♥`.  Full record in docs/one-notrump-competitive.md
# §N1-lia package B.
R=${1:?usage: ab-landy-lia.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-lia)
log "=== landy-lia SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --filter-landy
    arm lia  "$v" --filter-landy --ns-landy-lia

    gatepair lia base "$v"
    diffpair lia base "$v"
    sddiff   lia base "$v"
done

log "landy-lia done"
