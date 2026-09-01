#!/bin/sh
# ab-landy-lia-repair.sh — §N1-lia package B, repaired: Lia's re-rung counter
# ladder over their Landy `2♣` with the forensic's four named defects fixed.
#
#   JOBS=24 BOARDS=4608000 setsid nohup scripts/idle-run.sh \
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
#          3. **the `2NT` diamond rung takes the N1j transfer's own shape gate
#             back** — `len(♦, 6..) & points(2..)`, the quality legs deleted and
#             no upper cap (lebensohl.rs:1387-1391).  One widening, two jobs: it
#             re-rungs the six-card diamond hands stranded in the seam between
#             `2NT`'s old quality gate and the `2♦` escape's five-HCP
#             `natural_floor` — 12,956 of 14,156 passed-out boards hold **0-4
#             HCP**, not the 7+ band the repair plan assumed (defect 3,
#             −35,827/−32,432) — and it absorbs the six-card invitations, below.
#             The plan's other half, lifting the `2♦` escape's cap, was measured
#             a **no-op** (5♦ × 7-8 HCP is empty under `natural_floor` (5,0)) and
#             is **not built**: `2♦`@140 sits outside the `lia` branch on the
#             pre-existing `defense_2c_landy_weak_2d_cap` and its *rung* is
#             identical in both arms.  Its contested tail is not — that belongs
#             to edit 1, and is where the escape's whole loss lives.
#         3b. **the restored invitations split on length, not colour.**  `3♦`@166
#             keeps its `len(♦, 5..) & points(8..=9)` rule, but the widened
#             `2NT`@173 now outranks it on every six-carder, so the natural
#             invitation sees only exactly-five hands.  Emergent from the
#             weights, not from a new constraint — and the second of the two
#             repair-plan rules the re-solved forensic **inverted**: `3♦` at
#             six-plus is negative in all four cells (−0.910/−0.858 quiet,
#             −4.668/−4.195 contested) while exactly-five is the package's best
#             contested rung (+1.576/+1.931).
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
# Also NOT repaired, and pre-registered here because they bound how a wash is
# read: edit 1 authors opener's **sit** but not the seat immediately after it,
# so six node families — all lia-only, none reachable in the base arm — still
# hand the rest of the auction to the learned floor.  `{rung} - {leg} - - (X)`
# (their **balancing double**: `landy_lia_entries` yields bids only, and
# `systems_on_over_double` cannot catch it because the first suffix call is a
# Pass); `{rung} - {leg} - - ({over}) - -`; `2♦ ({over}) - -`; `{fit} ({over})
# - -` and `{fit} - - ({over}|X)` (responder after the natural invitation — the
# ladder's biggest measured win, contested on ~22% of its traffic); and
# `{rung} ({over}) - - X ({run})`, their runout from the penalty double this
# repair newly authors.  So defect 1's closure is **partial by construction**:
# if the arm reads a wash rather than a win, this is the first place to look,
# and the falsifier-1 reading below should be taken as a lower bound on what
# authoring the tails is worth.
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
# Resolution and the bar, pre-registered.  The sample is byte-for-byte the
# 2026-08-31 sizing (4,608,000 boards/arm/vul, 24 × 192,000), so the two runs
# are power-comparable and the minimum detectable effect is ≈0.002 IMPs/board —
# about 5% of the −0.0384 deficit, which makes −0.0384 → 0 a ~54σ move and
# resolves even a partial repair cleanly.  Two consequences.  (a) The knob is
# **off by default**, so it needs a plain-DD *win*: BV must **clear** zero, not
# merely reach it, and a BV wash leaves it off.  (b) The four repairs were
# fitted to the very divergence sets this run must overturn, so every per-cell
# figure quoted above is an **in-sample** selection on seed 1788122360 and
# should be expected to shrink.  A BV plain reading in (−0.010, 0) is therefore
# pre-registered as "the repair reached most of the deficit but the ladder still
# loses or washes and stays off" — a real result, not a measurement failure.
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
#      Instrument: this one is **not readable off the headline** — the arm
#      bundles five edits and the `!vulnerable()` term is inert at NV by
#      construction.  Check it post-hoc on `probe-divergence --imps` bucketed by
#      club length × colour over the kept arm dirs.
#   3. **The starved diamonds were not starved.**  The `Pass` bucket's six-card
#      diamond mass (14,156 boards, 12,956 of them **0-4 HCP**, −33,530 NV /
#      −30,610 BV) should move to `2NT`.  If `Pass` does not shrink by roughly
#      that mass, the hands were passing for a reason the quality gate never
#      caused.  `2♦` is *not* part of this: the starved band fails the escape's
#      five-HCP `natural_floor` in both arms, so only the `2NT` widening admits
#      it.
#   4. **The PD deficit is structural, not a doubling artifact.**  It should
#      shrink toward package A's scale as penalty-`X` density recovers.  If PD
#      stays an order of magnitude out while plain recovers, the ladder is
#      buying partscores by selling defense and the arm is a `win | loss`.
#   5. **The invitation length split is the wrong cut** (edit 3b, which none of
#      1-4 covers).  The `3♦` bucket should move to its exactly-five cells and
#      the six-carders should appear under `2NT`.  If `3♦` at exactly five does
#      not carry its forensic sign, the N1c right-siding trade the restored
#      invitations reverse was not wrong after all, and falsifier 1's reversal
#      was an artifact of the six-card hands the widening has now removed.
#
# Scoring: plain AND perfect defense off docs/measurement.md's decision table.
# sd-lead is reported as a **lead-model column, not a disclosure price** (see
# the caveat below).  `probe-divergence --gate-opener ours` must read 0 foreign
# BEFORE any headline.  Resumable; SEED_BASE persists in
# $R/landy-lia-repair.seed.  **Resume with the same two env vars** —
# `JOBS=24 BOARDS=4608000`, the launch line verbatim: `ab-lib.sh`'s guard
# compares the shard count only, so a resume that keeps JOBS and drops BOARDS
# silently falls back to 6400/shard (153,600 total, 30× under) and burns ~32
# min per regenerated arm before dying at the diff.  Iron rule: do NOT edit
# `src/` **or run any cargo build/release-example** while this runs —
# `SKIP_BUILD=1` stops this runner from rebuilding, not another session on this
# shared box from swapping `target/release/examples/bba-gen` between arms.
#
# Harness caveat, corrected from package D's wording.  `ab-dump-sd` has no
# `--on-ns-landy-*` flag, but the mechanism is not that the leader "believes"
# the N1j ladder: `--on-ns-*` flags on this harness are **measured inert**
# (docs/measurement.md:179-197 — an absurd `--on-ns-overcall 20:37` reproduces
# its run to the last IMP), so no setting on our book reaches the leader's
# model in either direction and the missing flag would change nothing if it
# existed.  Both arms are therefore read identically, the sd verdict **stands**
# as a lead-model column, and it is not a mis-disclosure penalty charged to the
# candidate.  What it cannot do is price disclosure or lead-direction value.
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

# Two passes, not one.  The sd rescore is ~78% of this run's wall clock (≈12 h
# of ≈15 h) for the column the caveat above says is a lead-model tie-break, not
# an arbiter — so both plain-DD cells, which the header pre-registers as
# primary, land in ≈3 h instead of ≈10 h.  Identical work in a different order:
# every guard in ab-lib.sh is a file-existence (or gate-PASSED) check, so this
# is resume-safe and moves no measured number.
for v in none both; do
    arm base "$v" --filter-landy
    arm lia  "$v" --filter-landy --ns-landy-lia

    gatepair lia base "$v"
    diffpair lia base "$v"
done

for v in none both; do
    sddiff lia base "$v"
done

log "landy-lia-repair done"
