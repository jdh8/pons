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
# ============================ VERDICT ============================
#
# **MEASURED A LOSS 2026-09-01.  `defense_2c_landy_lia` stays default off.**
# SEED_BASE=1788264406, control sha 32242d63 (`main` HEAD), 4,608,000
# boards/arm/vul, both isolation gates **0 foreign / PASSED**.
#
#   vul | fired          | plain DD           | PD
#   ----+----------------+--------------------+-------------------
#   NV  | 143,158 (3.11%)| -0.0077 +-0.0009   | -0.0127 +-0.0011
#       |                | (-35,676; -0.249/f)| (-58,701; -0.410/f)
#   BV  | 123,212 (2.67%)| -0.0059 +-0.0010   | -0.0172 +-0.0012
#       |                | (-27,354; -0.222/f)| (-79,454; -0.645/f)
#
# Plain DD is the pre-registered arbiter at both colours and both cells are
# negative, so there is no plain win to ship on and the knob stays off.  The
# sd pass was killed unrun: four negative cells need no lead model.  Both arm
# directories and `imps-{none,both}.jsonl` are KEPT — the forensic below is
# read off them and is the best evidence this lane has.
#
# THE LOSS IS THE DIAMOND LEG.  Splitting the divergence by which leg moved
# (baseline `2NT` = its club transfer, `3♣` = its diamond transfer; candidate
# `2♠`/`3♣` clubs, `2NT`/`3♦` diamonds):
#
#   leg      | n (NV) | plain NV          | plain BV
#   ---------+--------+-------------------+------------------
#   club     | 57,495 | +58,256 (+0.0126) | +63,018 (+0.0137)
#   diamond  | 49,380 | -92,508 (-0.0201) | -83,517 (-0.0181)
#   rest     | 36,283 |  -1,424 (-0.0003) |  -6,855 (-0.0015)
#
# The club leg is a clean win and `2NT -> 3♣` is the whole run's biggest cell
# (+46,715 NV / +54,641 BV): unwinding N1c's right-siding trade for a natural
# club rung is right.  Every diamond cell is a loss.
#
# FIVE FINDINGS THE NEXT ARM IS BUILT ON.
#
# 1. **No weak diamond rung beats the baseline's wide transfer.**  Re-solving
#    the boards where the baseline bid its `3♣`->diamond transfer, bucketed by
#    responder's own diamond holding, every candidate call is negative on
#    plain DD and the INV+ rung is the least bad:
#
#      responder holds        | lia bid | n (NV)  | plain/fired NV | BV
#      ----------------------+---------+---------+----------------+-------
#      6d thin, 0-4 hcp      | 3D      | 13,533  | -1.966         | -1.773
#      6d thin, 5+ hcp       | 2D      |  9,223  | -2.534         | -2.464
#      6d two top honors     | 2D      |  1,361  | -2.617         | -2.290
#      7+d                   | 2D      |  2,345  | -4.238         | -4.859
#      7+d                   | 3D      |  2,897  | -2.701         | -2.713
#      any                   | 2NT     | 11,839  | -0.709 .. -0.76| -0.34..-0.71
#
#    So the refinement gives the diamond leg back to a transfer at `2NT` and
#    re-cuts what is left: `3D` re-gates to EXCESSIVE diamonds (seven, or six
#    with two of the top three) and the `2D` escape's ceiling rises to eight
#    HCP to take everything below it.  **Pre-registered residue**: this still
#    leaves the weak six-card diamond hand off the transfer, which is the
#    cell above at -1.97/fired.  If the next arm's diamond leg is still
#    negative, the answer is the wide transfer (`points(2..)` on `2NT`) and
#    the INV+ gate is what has to go.
#
# 2. **The Pass@0 sell-outs were selling out to a floor that was right.**
#    Cell `3D -> -` (baseline competed to `3D`, candidate passed): 14,717
#    boards, **-22,119 plain NV / -13,364 BV**, 14,699 of them after our own
#    `2D` escape, concentrated in `1NT (2C) 2D (2S) -` (6,836 bd, -9,724) and
#    `1NT (2C) 2D (2H) -` (5,895 bd, -11,878).  The 2026-09-01 census read
#    the floor's behaviour right (it does bid) and drew the wrong conclusion.
#    The refinement DELETES `landy_lia_overcalled`'s registrations over the
#    weak rungs and the escape, and strips its `Pass`@0 so the residue over
#    the INV+ rungs reaches the floor by rejection -- at exact
#    `Pattern::node`s, since a guarded fallback's all-inf logits are returned
#    unchecked (package A's silent no-op).  The comment at
#    `lebensohl.rs:2536-2544` calling the floor's `3D` "a law-of-total-tricks
#    violation" was wrong and is gone.
#
# 3. **Right-siding is NOT null here, and the earlier note saying so read the
#    wrong column.**  Declaring SIDE flips on 123 NV / 66 BV boards -- that is
#    the zero.  Declarer SEAT flips on **26,664 NV / 27,911 BV** same-contract
#    boards, worth **-6,406 (-0.0014/bd) / -10,416 (-0.0023/bd)** plain, worst
#    cell `2NT -> 3C` (-3,630 / -5,600).  Package C's rule applies: DD prices
#    the lead direction, so this is real and it is the natural rungs giving
#    declarership back.  The refinement's Max-break+ answer table makes the
#    COMPLETION opener's minimum default, which recovers it on both legs.
#
# 4. **The by-length answer answered the wrong question.**  `comp:landy-length`
#    told responder which partscore to pick; the rung is invitational, so what
#    it needs is whether this is a game.  Replaced by super-accept / `3NT` /
#    completion under a new `comp:landy-super`.
#
# 5. **The `X`-vs-takeout cell is genuinely mixed, and the refinement picks
#    the double.**  Cell `X -> 2H` (baseline doubled, candidate took out),
#    split by responder's majors: the 8,113 NV / 6,236 BV boards with 2-2 or
#    better in the majors are **+1.854 / +1.154 plain** per fired for the
#    takeout and **-1.305 / -2.169 PD**.  Plain says take out; PD says double;
#    and this header's own arbitration note says the one PD loss this lane
#    does not wave through is a mechanism that REMOVES our penalty doubles,
#    which is what taking out with values does.  So the refinement narrows the
#    values `X` to `len(H,2..) & len(S,2..)` and drops the takeout below it,
#    in two bands and with no major term.  **Falsifier 1 of the next arm**: if
#    the club leg's win shrinks and this cell is where it went, flip the two
#    weights back.
#
# The refinement is `scripts/ab-landy-lia3.sh` (owed), fresh SEED_BASE,
# control = then-current `main`.
# =================================================================
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
