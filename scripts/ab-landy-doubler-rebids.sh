#!/bin/sh
# ab-landy-doubler-rebids.sh — §N1l, our doubler's own rebid after the Landy
# `(2♣)` values double: `1NT (2♣) X (2♥) - -`, `X (2♠) - -`, and the two escape
# legs `X (2♦) - (2♥)` / `X (2♦) - (2♠)`.
#
#   JOBS=24 BOARDS=4608000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-doubler-rebids.sh ab-results/landy-doubler-rebids \
#       >ab-results/landy-doubler-rebids.log 2>&1 < /dev/null &
#
# What is being tested.  Same 67 bd / −75 plain `X` branch §N1k cut, one call
# later: the seat is the floor's today (`fallback: Some(0)` on every hand in
# the band) and the floor is wrong in four places — it bids `3NT` holding KJ98
# of *their* major, and passes the 8–9 invitation, the 8–9 five-card club suit
# and the 8–9 five-card diamond suit alike.  `landy_doubler_rebid` is
# `kokish_kraft_doubler_rebid` ported with the probe's three corrections: no
# `ran` fork (their overcaller passes the preference 94.5%/96.7%), a natural
# `3m` where the twin has the other major, and the two escape legs added
# (~7.9% of the branch).  Table and probe numbers: docs/one-notrump-competitive.md §N1l.
#
# Falsifiers, in order.
#   1. **The floor already makes this table's penalty `X`.**  §N1k was REFUTED
#      on 2026-08-27, and its forensic is why: on 3 of the 5 worst plain-DD
#      boards at *each* vulnerability the OFF arm's floor, left alone, found a
#      delayed penalty double at exactly this seat (`X (2♥) - - X`) and beat
#      N1k's `3NT` with it.  So part of what this table authors is already
#      happening emergently, and the measurable delta is the difference between
#      the authored ladder and the floor's improvisation — NOT the branch's
#      whole 67 bd / −75 plain.  A small win is the expected shape; read the
#      fired count and the per-fired figure, not the per-board headline.
#      (Control is today's `main`: opener passes at the seat above, because
#      N1k did not ship.  BBA's opener does *not* simply sit there — it passes
#      67% non-vul / 80% vul and bids a natural `3♣` otherwise, `--mode
#      opener-c-x2h` — but that is BBA's seat, not our pool's: our net floor
#      passes it (`P` 11.9 vs `X` 3.0).  The *instinct* floor doubles instead
#      (`rule #382`), so anchor arms take a different path and their pool
#      differs; read the net-floor cells for the ship decision.)
#   2. **The top two rungs are dead in self-play.**  `landy_bba_responder`'s
#      ungated `3NT`@168 caps the double at nine points, so `4NT`@160 and
#      `3NT`@150 can only fire opposite a partner not bidding this table.  What
#      fires is `X` / `2NT` / `3♣` / `3♦` / `Pass` — do not read a verdict as
#      evidence about the quantitative rung.
#   3. **The penalty `X` is the doubling-artifact rung.**  It is the one rung
#      that adds doubles, so PD is blind to its benefit and keeps its whole
#      cost (measurement.md's domain addendum).  If plain wins and PD loses,
#      split by first rebid with `ab-dump-diff --show` before reading the row.
#
# Two arms, one seed set, `--filter-landy` on both:
#
#   base   today's `main` — the seat is the floor's (N1k refuted, not shipped)
#   reb    plus `landy_doubler_rebid` (`competition.landy_doubler_rebids`)
#
# Scoring: plain AND perfect defense off docs/measurement.md's decision table;
# sd-lead tie-breaks.  `probe-divergence --gate-opener ours` must read 0 foreign
# BEFORE any headline.  Resumable; SEED_BASE persists in $R/landy-doubler-rebids.seed.
# Iron rule: do NOT edit `src/` while this runs.
R=${1:?usage: ab-landy-doubler-rebids.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-doubler-rebids)
log "=== landy-doubler-rebids SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --filter-landy
    arm reb  "$v" --filter-landy --ns-landy-doubler-rebids

    gatepair reb base "$v"
    diffpair reb base "$v"
    sddiff   reb base "$v"
done

log "landy-doubler-rebids done"
