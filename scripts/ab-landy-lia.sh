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
