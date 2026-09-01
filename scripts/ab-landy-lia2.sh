#!/bin/sh
# ab-landy-lia2.sh — §N1-lia package B, rebuilt on the CORRECTED probe: Lia's
# actual counter to our `1NT (2♣)` Landy lane.
#
#   JOBS=24 BOARDS=4608000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-lia2.sh ab-results/landy-lia2 \
#       >ab-results/landy-lia2.log 2>&1 < /dev/null & disown
#
# Supersedes BOTH earlier runners, which measured builds that no longer exist:
# `ab-landy-lia.sh` (the first ladder, a loss on 2026-08-31 — +0.0050 NV /
# −0.0384 BV plain, seed 1788122360, control 59cd46ee) and
# `ab-landy-lia-repair.sh` (its four-defect repair, STOPPED mid-flight on
# 2026-09-01 with one NV cell recorded — +0.0191 plain / −0.0382 PD, seed
# 1788247951, control ce94faeb).  Both keep their VERDICT blocks as the record
# of what they actually scored; own-runner precedent, `ab-landy-nt-remeasure.sh`.
#
# WHY A THIRD RUNNER.  Lia is IntoBridge's AI — an online service, no code, so
# her system is known only by hand-probing her on cuebids.com.  The original
# probe of her responder table was **wrong**: it had the ladder inverted (weak
# transfers at the two level, natural invitations at the three) when she plays
# the opposite.  Both earlier arms were therefore repairs to nobody's system.
# `competition.defense_2c_landy_lia` was redefined in place on 2026-09-01 (it
# never shipped; the superseded semantics are pinned by sha 8a778178).
#
#   base   today's `main` — the shipped N1j BBA ladder
#   lia    Lia's ladder as re-probed, four rungs changed and nothing else:
#            2♥   UNBAL both-minors takeout, 4+♣ 4+♦ — spelled
#                 `len(♥,..=2) & len(♠,..=2)`, which with 4+4+ in the minors
#                 *is* unbalanced, and still excludes the 2=3=4=4 merge the
#                 first build's forensic convicted (−4.074 IMPs/fired, all
#                 3,069 divergent boards that one shape).  Band is ours:
#                 `points(8..)`, unlimited above, weak 4-4 hands pass.  Lia's
#                 shape contains the splinters', so they move above it
#                 (3♥/3♠ @179/178, takeout @177) instead of being disjoint.
#            2♠   natural 6+ CLUBS, invitational or better, uncapped
#            2NT  natural 6+ DIAMONDS, invitational or better, uncapped
#            3♣   natural 6+ clubs, sign-off (`points(..=7)`), @141
#            3♦   natural 6+ diamonds, sign-off, @139
#          Everything else is the shared table verbatim: 3NT@180/@168, the 4M
#          jam, X@145, 2♦@140, Pass@0.
#
# TWO CONSEQUENCES OF THE RUNG ORDER, both deliberate and both measurable.
# (a) `3♣`@141 is ABOVE the weak `2♦`@140 and `3♦`@139 is BELOW it, so the
#     escape keeps every hand it can take and `3♦` picks up exactly the ones it
#     refuses — the 6+♦ hands with 0-4 HCP that failed its five-HCP
#     `natural_floor`.  That is the first ladder's defect 3 (the "starved
#     diamonds": 12,956 of 14,156 passed-out boards, −33,530 NV / −30,610 BV)
#     closed by rung order alone, with no widening and no knob.
# (b) Nothing in the package reads `vulnerable()` any more.  The colour gate the
#     old defect-2 finding bought died with the five-card weak leg it gated:
#     Lia wants six cards at both ends, which is also what that finding said
#     (exactly-5 +1.405 white / −0.803 red, six +0.993/+0.668, seven-plus
#     +1.849/+1.683).
#
# WHAT IS AUTHORED AROUND THE RUNGS.  Opener answers the minor rungs by length
# (`comp:landy-length`) or accepts at `3NT` from the top with both of their
# majors stopped — the rung the invitational band cannot do without, since a
# describe-only structure walks a 24-count into `3♣`.  Responder's placements
# are re-banded for INV+ (`landy_lia_pick_rebid`, `landy_lia_takeout_rebid`):
# the game-forcing `5m`@100 / `3NT`@100 catch-alls are gated on game values and
# `Pass`@0 is the finite rung.  The `2♠` ask's catch-all drops from `4♣`@20 to
# `3♣`@20 (natural, in a fit that must exist — opener denied four in both
# minors), which also retires an unalerted artificial call.  The whole contested
# surface is authored on both seats, including the six lia-only node families
# the 2026-09-01 pre-flight audit found still reaching the floor.  And the
# minor-transfer-slam rule is honoured above opener's acceptance: `4m` with an
# authored answer and the RKCB ladder, because an unauthored `4m` reads as
# nothing and the floor's keycard ask is gated on `undisturbed`, which this lane
# never is.
#
# SIZING AND RESOLUTION.  4,608,000 boards/arm/vul (24 x 192,000), byte-for-byte
# the sizing of both prior runs, so all three are power-comparable; MDE is
# ~0.002 IMPs/board.  Against the first ladder's −0.0384 BV that is a ~19-sigma
# scale, so even a partial move reads cleanly.  The knob is OFF by default, so
# it needs a plain-DD **win**: BV must clear zero, not merely reach it, and a BV
# wash leaves it off.
#
# PRE-REGISTERED ARBITRATION — stated here, before launch, which is the thing
# the stopped run got wrong.  This knob's direction is MIXED and the two halves
# are not commensurable, so they are not netted:
#
#   * The contested tails and the restored forcing channel make us DOUBLE MORE
#     and BID MORE.  For those, `ns_score_pd` is blind to the benefit while
#     keeping the whole cost (docs/measurement.md, the domain addendum), so
#     **plain DD arbitrates and PD is reported double-blind** — package A's
#     rule, and the one the stopped runner should have stated for its own
#     bid-more mechanism instead of leaving the row unresolved.
#   * The six-card floors at `2♠`/`2NT`/`3♣`/`3♦` make us BID LESS (the
#     exactly-five hands pass or take the escape).  There PD is the honest
#     pessimistic end of the bracket and belongs in arbitration.
#
#   So: read plain DD as primary at BOTH colours; quote PD as a column; ship on
#   the decision table's plain-DD row at both colours.  A plain-DD win with a
#   PD loss of package-A's ORDER (single-digit thousandths) is the artifact and
#   ships; a PD loss an order of magnitude out — the first ladder's shape,
#   −0.0756/−0.1210 — is NOT waved through, because there the mechanism was
#   removing our penalty doubles and this one is not.  If PD is that large
#   again, read falsifier 4 first.
#
# FALSIFIERS, in order.  The first is the correction's own predicted loss.
#   1. **The exactly-five 8-9 hand has nowhere to go.**  The first ladder's
#      single biggest win was the natural five-card club invitation (`3♣`
#      +85,613 IMPs NV / +35,920 BV) and its best contested rung was the
#      five-card diamond one (+1.576 / +1.931).  True Lia has no rung for that
#      band: five cards and 8-9 points now double or pass.  If the arm loses,
#      bucket THOSE hands first (`probe-divergence --imps --jsonl`, responder
#      holding exactly five of a minor and 8-9 points).  A loss concentrated
#      there is not a verdict on the corrected ladder — it is a verdict on
#      Lia's rung set against ours, and the answer is a hybrid nobody has
#      built yet, not a revert.
#   2. **The six-card floor is the wrong cut** (the mirror of the old defect 2,
#      which was measured on a five-card leg that no longer exists).  The
#      passed-out bucket should GAIN the exactly-five weak hands and lose
#      nothing else.  If it also gains six-card hands, a floor is mis-gated.
#   3. **`3♦`@139 did not pick up the starved band.**  The `Pass` bucket's
#      six-card diamond mass at 0-4 HCP (14,156 boards in the old solve, 12,956
#      of them in that band) should move to `3♦`, and `2♦`'s own traffic should
#      be UNCHANGED — the rung sits below it precisely so the escape keeps its
#      hands.  If `2♦` moves, the weights are wrong.
#   4. **The contested tails bought nothing again.**  The contested share of the
#      `2♠`/`2NT` buckets should shrink against the 2026-08-31 split.  If it
#      does not, the level-down ladder is what loses, not its tails.
#   5. **Defect 1's closure is again partial by construction.**  Authoring a
#      `Pass` always creates fresh traffic one seat below it, and the six
#      families this build closes were merely the ones the census could price.
#      Re-census the floor's share at responder's and opener's seats over the
#      kept arm dirs and name what is left, whatever the headline says.
#
# Scoring: plain AND perfect defense off docs/measurement.md's decision table.
# sd-lead is a **lead-model column, not a disclosure price** — `ab-dump-sd` has
# no `--on-ns-landy-*` flag, but `--on-ns-*` flags on this harness are measured
# inert (docs/measurement.md:179-197), so both arms are read identically and the
# missing flag would change nothing if it existed.
#
# `probe-divergence --gate-opener ours` must read 0 foreign BEFORE any headline.
# Resumable; SEED_BASE persists in $R/landy-lia2.seed.  **Resume with the same
# two env vars** — `JOBS=24 BOARDS=4608000`, the launch line verbatim.  Iron
# rule: do NOT edit `src/` or run any cargo build while this runs.
#
# The bucket forensic is NOT part of this script.  It is a manual post-hoc
# invocation over the kept arm directories:
#
#   ./target/release/examples/probe-divergence \
#       $R/lia-both $R/base-both --imps --jsonl $R/imps-both.jsonl
#
R=${1:?usage: ab-landy-lia2.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-lia2)
log "=== landy-lia2 SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

# Two passes, not one.  The sd rescore is ~78% of this run's wall clock for a
# column the caveat above calls a tie-break, not an arbiter — so both plain-DD
# cells, which the header pre-registers as primary, land in ~3 h instead of
# ~10 h.  Identical work in a different order; every guard in ab-lib.sh is a
# file-existence (or gate-PASSED) check, so this is resume-safe.
for v in none both; do
    arm base "$v" --filter-landy
    arm lia  "$v" --filter-landy --ns-landy-lia

    gatepair lia base "$v"
    diffpair lia base "$v"
done

for v in none both; do
    sddiff lia base "$v"
done

log "landy-lia2 done"
