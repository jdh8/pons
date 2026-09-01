#!/bin/sh
# ab-landy-lia-repair.sh — §N1-lia package B, repaired: Lia's re-rung counter
# ladder over their Landy `2♣` with the forensic's four named defects fixed.
#
#   JOBS=24 PER_SHARD=192000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-lia-repair.sh ab-results/landy-lia-repair \
#       >ab-results/landy-lia-repair.log 2>&1 < /dev/null & disown
#
# Supersedes `ab-landy-lia.sh`, which measured the unrepaired ladder a loss on
# 2026-08-31 (+0.0050 NV / **−0.0384 BV** plain, PD −0.0756/−0.1210, seed
# 1788122360, control 59cd46ee).  Fresh SEED_BASE, control = the then-current
# `main` HEAD.  Package D's own-runner precedent (`ab-landy-nt-remeasure.sh`):
# a re-measure of a decided knob gets its own script so the old runner's
# VERDICT block stays the record of what it actually measured.
#
# What is being tested.  `competition.defense_2c_landy_lia`, unchanged in
# concept — the same permutation of the same rungs — with every edit the
# 2026-08-31 forensic's repair queue named, all of them behind the same
# off-by-default knob (no new knobs; the default system stays byte-identical,
# `smoke-default --count 20000 --seed 1` verified twice):
#
#   base   today's `main` — the N1j BBA ladder
#   lia    the level-down ladder, repaired:
#          1. **the contested surface authored** (defect 1, the only defect
#             negative at both colours, −63,950/−80,789 on the club rung and
#             95%/94% of the diamond rung's whole loss).  Every lia node used
#             to require the opponents to have passed — only `{rung} -` and
#             `{rung} (X)` were registered — so any opponent *bid* dropped the
#             rest of the auction to a learned floor with no forcing channel.
#             Now both seats are authored: opener over the advancer's call on
#             our minor rung, and responder over their entry after opener's
#             length answer.  Compressed tables in the
#             `landy_bba_takeout_overcalled` idiom, not a node per bid.
#          2. **the weak `2♠` leg wants a sixth club when vulnerable**
#             (defect 2): `points(..=7) & (len(♣, 6..) | !vulnerable())`.  The
#             uncontested weak rung splits on colour, not on length alone —
#             exactly five clubs is +1.405 IMPs/fired white and −0.803 red,
#             while six (+0.993/+0.668) and seven-plus (+1.849/+1.683) win at
#             both.  That one cell was 45% of the arm's whole both-vul PD
#             deficit.
#          3. the `2NT` quality legs capped to the weak range, and the `2♦`
#             escape's cap lifted, so the weak six-card diamond hand the
#             narrowed rung starved into passing has a rung again (defect 3,
#             −35,827/−32,432).
#          4. the `2♥` takeout's 2=3=4=4 merge revisited (defect 4, the worst
#             per-fired rung at −4.07/−5.00).
#
# What is NOT being repaired, deliberately.  `landy_recue_answer`'s `4m`@20
# ends with no answer node (`2♠ - 3♣ - 3♥ - 4♣ -` is floor-owned), which is
# where the four worst both-vul boards ran away to `6♥` doubled — but that seat
# is **shared with the base arm**, which runs away the same way one level lower
# and undoubled.  Fixing it lia-gated would make this arm measure a repair the
# control wants too.  It is its own owed A/B; see the campaign doc.
#
# Expected shape of the verdict.  The unrepaired arm's plain DD split by colour
# (+0.0050 NV, −0.0384 BV) with the whole both-vul deficit in three named
# defects, so the repair has to move BV plain across zero without giving back
# NV.  PD was an order of magnitude past package A's doubling artifact because
# lia *removes* our penalty doubles (5.51% of divergent boards against base's
# 7.79%) and perfect defense is exactly the scorer that pays for doubles we no
# longer make; the contested tails put penalty doubles back, so PD is the
# column that should move most.
#
# **The knob's direction is mixed, and that decides how PD is read**
# (docs/measurement.md, the domain addendum).  Edit 1 *adds* doubles — the
# contested tails' `X` is penalty by this lane's polarity rule — and for a knob
# that doubles them more, `ns_score_pd` is blind to the benefit while keeping
# the whole cost, so **that half is arbitrated on plain DD with PD reported
# double-blind**, exactly as package A pre-registered it.  Edit 2 runs the
# other way: it *removes* a bid (the red five-card sign-off passes), where PD
# is the honest pessimistic end of the bracket and belongs in arbitration.  So
# read plain DD as primary at both colours, quote PD as a column, and do not
# net the two against each other — they are not commensurable here.  Ships on
# the decision table's plain-DD row at both colours.
#
# Falsifiers, in order.
#   1. **The tails were not the problem — the concept was.**  If the contested
#      share of the `2♠`/`2NT` buckets does NOT shrink against the 2026-08-31
#      split, authoring them bought nothing and the level-down ladder itself is
#      what loses.  Re-read the bucket table before anything else.
#   2. **The vulnerability gate is the wrong cut.**  The BV exactly-five-clubs
#      cell should leave the deficit while NV keeps roughly +1.4 IMPs/fired.
#      If NV falls with it, the gate is trading a real white win for a red
#      saving and the cut belongs on club quality, not colour.
#   3. **The starved diamonds were not starved.**  The `Pass` bucket should
#      shrink by about the mass the `2♦`/`2NT` re-rung admits.  If it does not,
#      the hands were passing for a reason the cap never caused.
#   4. **The PD deficit is structural, not a doubling artifact.**  It should
#      shrink toward package A's scale as penalty-`X` density recovers.  If PD
#      stays an order of magnitude out while plain recovers, the ladder is
#      buying partscores by selling defense and the arm is a `win | loss`.
#
# Scoring: plain AND perfect defense off docs/measurement.md's decision table;
# sd-lead tie-breaks.  `probe-divergence --gate-opener ours` must read 0
# foreign BEFORE any headline.  Resumable; SEED_BASE persists in
# $R/landy-lia-repair.seed.  Iron rule: do NOT edit `src/` while this runs.
#
# Harness caveat, carried forward from package D: `ab-dump-sd` has no
# `--on-ns-landy-*` disclosure flag, so both arms' auctions are read by a
# leader built from `Agreements::default()` — every sd row in this campaign
# prices a lia auction against a leader who believes we still play the N1j
# ladder.  Left alone here on D's recorded default so the sd column stays
# comparable across §N1p, D and this arm; quote it with the caveat.
#
# The bucket forensic is NOT part of this script.  No runner calls
# `probe-divergence --imps`; the split that produced the numbers quoted above
# came from a manual post-hoc invocation over the kept arm directories:
#
#   ./target/release/examples/probe-divergence \
#       $R/lia-both $R/base-both --imps --jsonl $R/imps-both.jsonl
#
R=${1:?usage: ab-landy-lia-repair.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-lia-repair)
log "=== landy-lia-repair SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --filter-landy
    arm lia  "$v" --filter-landy --ns-landy-lia

    gatepair lia base "$v"
    diffpair lia base "$v"
    sddiff   lia base "$v"
done

log "landy-lia-repair done"
