#!/bin/sh
# ab-landy-texas.sh — §N1-lia package C: the four-level over their Landy `2♣`
# rides South African Texas, one arm.
#
#   JOBS=24 PER_SHARD=192000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-texas.sh ab-results/landy-texas \
#       >ab-results/landy-texas.log 2>&1 < /dev/null & disown
#
# Run AFTER packages A and B are decided: control = the then-current `main`
# HEAD (whichever responder ladder it carries — the knob composes with both),
# fresh SEED_BASE.
#
# What is being tested.  `competition.landy_texas` — the `4♣`/`4♦` seat is
# floor-owned today, so the shipped jam declares from the wrong side and a 16+
# hand cannot look for slam:
#
#   base   today's `main` — the jam declares `4M` from responder
#   texas  `4♦`→♠ / `4♣`→♥ carry the jam's gate (`points(10..) & len(6..)`,
#          `landy_texas_floor` NOT swept here — this package changes only
#          which call carries the hand) with opener completing; the freed
#          direct `4♥`/`4♠` are the uncontested NF slam-try tier
#          (`hcp(15..=direct_4m_max)`, opener launching RKCB from 17); a 16+
#          hand transfers and drives its own `4NT` above the completion.  The
#          drive seat carries an authored `Pass`@0 sit rail — this lane's
#          floor is the learned one §N1o caught cue-bidding a dead four-level
#          to `6♥` doubled.
#
# Expected shape of the verdict: **a wash on plain DD is real, not an
# artifact** — right-siding is invisible to the harness by construction
# (docs/measurement.md: right-siding-only ideas measuring zero is real).  The
# DD-visible half is the slam reroute: 16+ now reaches RKCB instead of
# jamming.  Ships on a non-loss.
#
# Falsifiers, in order.
#   1. **The slam reroute overbids.**  The drive asks on bare `hcp(16..)`
#      opposite 15-17 with the suit breaking badly behind the Landy hand; read
#      the made/down split of the 6M contracts before the mean.
#   2. **The transfer leaks information.**  `4♦`/`4♣` are alerted (`texas`),
#      so the defense knows the anchor major before the opening lead in a way
#      the direct jam never told them; sd-lead is the scorer that can see it.
#   3. **The sit rail forgoes the floor's rare good pull.**  The rail passes
#      every sub-16 completion by authorship; if the loss sits on boards where
#      base's floor found a making slam, the rail is too blunt — relax to a
#      15-count re-try before closing.
#
# Scoring: plain AND perfect defense off docs/measurement.md's decision table;
# sd-lead tie-breaks.  `probe-divergence --gate-opener ours` must read 0
# foreign BEFORE any headline.  Resumable; SEED_BASE persists in
# $R/landy-texas.seed.  Iron rule: do NOT edit `src/` while this runs.
R=${1:?usage: ab-landy-texas.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-texas)
log "=== landy-texas SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base  "$v" --filter-landy
    arm texas "$v" --filter-landy --ns-landy-texas

    gatepair texas base "$v"
    diffpair texas base "$v"
    sddiff   texas base "$v"
done

log "landy-texas done"
