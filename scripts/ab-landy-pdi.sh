#!/bin/sh
# ab-landy-pdi.sh — §N1n, the **P branch** of the Landy penalty process: what
# our side's later doubles mean once opener has *passed* their advance.
#
#   JOBS=24 PER_SHARD=192000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-pdi.sh ab-results/landy-pdi \
#       >ab-results/landy-pdi.log 2>&1 < /dev/null & disown
#
# Queue this **after** scripts/ab-landy-opener.sh.  One run saturates the box,
# and §N1m is the older debt.
#
# The trigger theorem (docs/pdi.md).  A pass/double inversion has to locate at a
# **P**.  Hearing partner's `X` — RHO having passed — doubling is illegal, since
# the contract is already doubled by our side, so at that seat there is no `P`/`X`
# pair to invert at all; the choice is sit-or-pull.  Only a pass over their live
# bid has both legs legal.  In this lane that pass is opener's, at
# `1NT (2♣) X (2♥) -`, and the calls after it are the inverted ones.
#
# Why authoring rather than the dialect shell.  A divergent `X` has a synonym in
# the floor's own book — `--ns-pdi-translate` rewrites it to a pass and serves the
# net a picture it understands (the `pxt` arm of ab-landy-opener.sh).  A
# PDI-loaded `P` has none: no call means "pass, but as an election" in a book that
# does not play the agreement.  So the P branch is repairable by authoring alone,
# and none of its rows carries a `.pdi()` tag.
#
#   base    today's `main` — both seats below are the floor's, which reads every
#           double after our pass as takeout
#   pdi     `competition.landy_pdi`: two seats, each `X`@150 on `len(run, 4..)`
#           with a `Pass`@0 catch-all and partner's sit under it —
#             B1 `X (2M) - (2M')`     responder punishes the overcaller's
#                                     correction of the preference
#             B2 `X (2M) - - X (2M')` opener punishes the advancer's run from the
#                                     doubler's delayed `X`
#           `M'` is the other major at its cheapest legal level: `2♠` out of a
#           doubled `2♥`, `3♥` out of a doubled `2♠`.
#
# Independent of `--ns-landy-opener-px` by construction — the trie matches these
# patterns whether opener's pass and the delayed double came from the book or the
# floor — so this runs against a plain `--filter-landy` base.  Note B2 is the
# interfered tail of `landy_doubler_px`, which ships **default-on**: the base arm
# already makes that double and simply has no authored answer when they run.
#
# Falsifiers, in order.
#   1. **The seats are rare.**  B1 needs the overcaller to correct (the §N1l probe
#      says they pass the preference 94.5% / 96.7% of the time) and B2 needs their
#      run from a doubled contract, on top of a lane that is ~2% of boards.  Read
#      the fired count off `probe-divergence` BEFORE the headline: if it is small,
#      the honest verdict is "not measurable here", not "no effect".
#   2. **PD is blind to a real penalty double by construction**
#      (docs/measurement.md's domain addendum), exactly as in §N1l and §N1m.
#      Arbitrate on plain DD with SD-PD as tie-break.
#   3. **The catch-all shadows the floor.**  `Pass`@0 means a hand with three
#      trumps passes where the floor might have acted.  If `pdi` reads negative,
#      split the divergence stream by `call_on`/`call_off` before blaming the
#      double — the same accounting §N1l-flip needed, where 55.2% of the
#      divergences were passes where the baseline bid.
#
# Scoring: plain AND perfect defense off docs/measurement.md's decision table;
# sd-lead tie-breaks.  `probe-divergence --gate-opener ours` must read 0 foreign
# BEFORE any headline.  Resumable; SEED_BASE persists in $R/landy-pdi.seed.
# Iron rule: do NOT edit `src/` while this runs.
R=${1:?usage: ab-landy-pdi.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-pdi)
log "=== landy-pdi SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --filter-landy
    arm pdi  "$v" --filter-landy --ns-landy-pdi

    gatepair pdi base "$v"
    diffpair pdi base "$v"
    sddiff   pdi base "$v"
done

log "landy-pdi done"
