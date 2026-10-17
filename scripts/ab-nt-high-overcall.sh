#!/bin/sh
# ab-nt-high-overcall.sh — N3: their three-level overcall of our 1NT
# (docs/one-notrump-competitive.md §N3).
#
# SCOPE (2026-10-16): strict 1NT scan, balanced openers only (no 5m422/6m322,
# 14.4% of the default 1NT).  Opener-side: re-run with --wide-1nt on fresh
# deals before relying on the verdict (docs/research-method-review.md,
# "Filter scope").
#
#   JOBS=24 BOARDS=460800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-nt-high-overcall.sh ab-results/nt-high-overcall \
#       >ab-results/nt-high-overcall.log 2>&1 < /dev/null &
#
# What is being tested.  `competition.nt_high_overcall_responses`
# (`bba-gen --ns-nt-high-overcall`) authors responder's one call over
# `1NT (3♣)`–`1NT (3♠)` and opener's one answer to each: a forcing five-card
# suit at the three level, a six-card major at game, takeout `X` with a
# four-card major and `points(8..)`, `3NT`, and the four-level minor priced
# *under* `3NT`.  Today the whole lane is floor-only — the census's top loser
# (405 bd, −1.07 plain/bd, PD negative at both vulnerabilities), where
# responder's new suit reads as nothing to opener and the floor's `X` fires on
# 6–7 HCP but not on 9–11 4-4-4-1.
#
# BBA's three-level calls over a 1NT opening are **natural seven-card
# preempts** (`hcp 4–10`, docs/ai-bidder/bba-1nt-counter-defense.md), so this
# is an ordinary competitive scheme, not a counter-defense — no disclosure
# channel, nothing keyed on their declaration.
#
# Three arms, one SEED_BASE:
#   base    the knob off (the shipped floor-only lane)
#   stop    the knob on, `direct_3nt_stopper` as shipped (true)
#   nostop  the knob on, `--ns-direct-3nt-stopper false` — jdh8's prior is that
#           the direct 3NT needs no stopper of its own, since partner opened
#           1NT.  The knob is *shared* with the two-level Lebensohl lane, so a
#           `nostop` win here buys the three-level table its own gate bit
#           rather than flipping the shared default.
#
# `--filter-1nt` (balanced 15-17 somewhere, a raw-hand test applied BEFORE any
# bidding) rides every arm so they deal the same board set and stay
# seed-aligned for the paired diff.  The `(3x)` bucket is ~0.2% of accepted
# boards, so read the per-fired figure alongside the headline.
#
# Scoring.  The standard gate (docs/measurement.md): plain wash + PD win ships
# default-on, a CI-clear plain loss stays opt-in.  `sddiff` is the tie-breaker.
# Read `probe-divergence --gate-opener ours` BEFORE the headline — nothing here
# is keyed on `their.*`, so the gate is the proof, not the design.
#
# Round 2 (`ROUND=2`), after the package shipped default-on 2026-08-18: two
# increments over the shipped default, both diffed against the reused `stop`
# arm — whose boards are byte-identical to a default-flag regeneration, checked
# before reuse (only the recorded `gen_args` metadata differs).
#   xfer    the `(3♣)` transfer variant (`--ns-nt-3c-transfers`).  That lane is
#           ~100 bd per 409k, so read the per-fired figure and pool a second
#           seed if the CI swallows it.
#   nogate  the three-level table's OWN stopper bit off
#           (`--ns-nt-high-overcall-3nt-stopper false`).  Round 1's `nostop` arm
#           answered this question through the *shared* `direct_3nt_stopper`,
#           and `probe-divergence --gate-opener ours` failed at 36% foreign: the
#           shared bit also governs advancing partner's takeout double of a weak
#           two, where dropping it costs −1.23/−1.92 PD per fired while this
#           lane gains +2.20/+1.62 plain.  Hence the private bit and this arm.
#
# Resumable: an existing arm dir or diff file is skipped, and SEED_BASE persists
# in $R/nt-high-overcall.seed.  Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-nt-high-overcall.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example ab-dump-bucket --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for nt-high-overcall)
log "=== nt-high-overcall SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

if [ "${ROUND:-1}" = 1 ]; then
  for v in $VULS; do
    # Post-ship spellings: `base` is the pre-ship arm, `stop` the shipped default.
    arm base   "$v" --filter-1nt --ns-nt-high-overcall false
    arm stop   "$v" --filter-1nt
    arm nostop "$v" --filter-1nt --ns-direct-3nt-stopper false
    diffpair stop   base "$v"
    diffpair nostop base "$v"
    diffpair nostop stop "$v"
    sddiff   stop   base "$v"
    sddiff   nostop base "$v"
  done
fi

# Round 2: the two increments over the shipped default, against `stop`.
if [ "${ROUND:-1}" = 2 ]; then
    for v in $VULS; do
        # Skipped in round 1's dir (already there); generated in a fresh
        # pooling seed's dir, where `stop` IS the shipped default.
        arm stop   "$v" --filter-1nt
        arm xfer   "$v" --filter-1nt --ns-nt-3c-transfers true
        arm nogate "$v" --filter-1nt --ns-nt-high-overcall-3nt-stopper false
        diffpair xfer   stop "$v"
        diffpair nogate stop "$v"
        sddiff   xfer   stop "$v"
        sddiff   nogate stop "$v"
    done
fi

# Round 4 (`ROUND=4`): the two census-queued increments to opener's answer to
# responder's takeout double, each its own knob, both against a common control.
# `--filter-preempt` replaces `--filter-1nt` here: the `1NT (3x)` lane is 14.8%
# of its accepted boards against 0.60%, so an arm buys ~25x the owned sample for
# 33x the (bidding-free) scan.  Arms under it pair only with each other.
#   fit   opener answers their `(3♠)` double `4♥` with four hearts — the rung
#         the default ladder has no room for, and the `(3♠) X - 3NT` cell it
#         targets is 20 bd / -94 plain / -123 PD on the fresh-seed anchor.
#   pass  opener may leave the double in with at most one of A/K/Q in their
#         seven-card suit.  BBA's advancer sits for it on 88% (minor) / ~50%
#         (major) and its preemptor never bids again.
if [ "${ROUND:-1}" = 4 ]; then
    for v in $VULS; do
        arm base "$v" --filter-preempt
        arm fit  "$v" --filter-preempt --ns-nt-high-overcall-x-major-at-four true
        arm pass "$v" --filter-preempt --ns-nt-high-overcall-x-leave-in true
        diffpair fit  base "$v"
        diffpair pass base "$v"
        sddiff   fit  base "$v"
        sddiff   pass base "$v"
    done
fi

# Round 5 (`ROUND=5`): the leave-in **v2**, re-gated on length after the v1 gate
# was refuted and its own dumps were re-sliced by opener's holding
# (docs/one-notrump-competitive.md §N3, "Round 6 — the leave-in re-sliced").
# `base` is today's shipped default, so the fit rung (`x_major_at_four`) is ON
# in every arm here — the boards it takes are the worst block in the v1 slice
# and they are already gone.
#
# The two candidate disjuncts are SEPARATE arms, not one bundled gate: the
# slice puts the whole surviving case in length (`len4+`: plain +1.92 NV /
# +3.58 vul per fired) while at fixed length every measured honor step costs
# (`len3 hon0` +0.62/+1.85 vs `len3 hon1` -0.75/+0.37).  Bundled, a win would
# ship the bad half and a loss would bury the good one.
#   length  opener passes with FOUR cards in their seven-card suit
#   three   ...and with three to two of the top three honors (the full v2
#           candidate).  `three` implies `length`, so read `three vs length`
#           for the extension's own price.
#
# Reading it: this knob's mechanism is **adding doubles**, so measurement.md's
# domain addendum applies — plain DD is the arbiter, sd-lead the tie-break, and
# PD is a double-blind column that neither rescues nor kills.  Prior to beat:
# v1's -2.06 IMPs/fired on sd-lead.  Run `probe-divergence --gate-opener ours`
# before the headline.
if [ "${ROUND:-1}" = 5 ]; then
    for v in $VULS; do
        arm base   "$v" --filter-preempt
        arm length "$v" --filter-preempt --ns-nt-high-overcall-x-leave-in true
        arm three  "$v" --filter-preempt --ns-nt-high-overcall-x-leave-in true \
                        --ns-nt-high-overcall-x-leave-in-three true
        diffpair length base   "$v"
        diffpair three  base   "$v"
        diffpair three  length "$v"
        sddiff   length base   "$v"
        sddiff   three  base   "$v"
        sddiff   three  length "$v"
    done
fi

# Round 8 (`ROUND=8`, numbered to match the doc's §N3 rounds): the leave-in's
# suit gradient priced out-of-sample.  Round 7 shipped `length` and observed —
# in-sample — that it pays most vs `(3♠)` and least vs `(3♣)` on sd-lead.
#
# No suit-gate knob is authored for this probe, deliberately: each divergent
# board's window holds exactly one `(3x)`, so the overcall suits PARTITION the
# fired set, and any candidate suit-subset gate's paired diff against the
# shipped default is byte-for-byte a suit-bucket subset of `base vs noleave`.
# Two arms price all fourteen candidate narrowings at once; read them with
#   ab-dump-bucket $R/base-VUL $R/noleave-VUL --by holding
#   ab-dump-sd     $R/base-VUL $R/noleave-VUL -v VUL --sd-worlds 16 --show 0 --by holding
# (ON must be the leave-in arm = `base`, the shipped default; the `(other)`
# bucket must read zero).  A gate knob is authored only if some suit reads
# CI-clear negative here — plain DD is the arbiter (the mechanism keeps
# doubles in), sd-lead the tie-break, PD a double-blind column.
#
# The WIDENING candidate (the refuted `three` extension restricted to spades,
# where Round 7's hon2+ gradient peaked) is NOT here: it needs its own arm,
# and whether it ever earns one is decided first by re-slicing Round 7's
# existing `three vs length` dumps with `--by holding` — only a positive spade
# bucket there buys the arm.
#   base     today's shipped default (leave-in on, `len(over, 4..)`)
#   noleave  --ns-nt-high-overcall-x-leave-in false (the pre-Round-7 ladder)
if [ "${ROUND:-1}" = 8 ]; then
    for v in $VULS; do
        arm base    "$v" --filter-preempt
        arm noleave "$v" --filter-preempt --ns-nt-high-overcall-x-leave-in false
        diffpair base noleave "$v"
        sddiff   base noleave "$v"
    done
fi

# Round 9 (`ROUND=9`): FOUR-CELL CONFIRMATION of the leave-in (pre-registered
# 2026-10-16, before any number is read; docs/research-method-review.md § 6).
# Its v1 flipped sign by vulnerability, so the asymmetric cells are the real
# test of the length gate.  Round 8's arms with `--wide-1nt` (the preempt scan's
# 1NT candidate then covers the default opener), VULS="ns ew", fresh seed,
# BOARDS=2400000, results in ab-results/nt-high-overcall-nsew.  The mechanism
# keeps doubles in, so plain DD arbitrates and PD is a double-blind column; the
# default stands in a cell whose plain DD reads non-inferior or better, any
# other reading is flagged for a decision, not applied.
if [ "${ROUND:-1}" = 9 ]; then
    for v in $VULS; do
        arm base    "$v" --filter-preempt --wide-1nt
        arm noleave "$v" --filter-preempt --wide-1nt --ns-nt-high-overcall-x-leave-in false
        gatepair base noleave "$v"
        DENSITY=$(density_of "$R/base-$v")
        diffpair base noleave "$v"
        sddiff   base noleave "$v"
    done
fi

# Round 10 (`ROUND=10`): the owed FRESH-SEED CONFIRMATION of N3-fit
# (`nt_high_overcall_x_major_at_four`, the `4♥` rung over `1NT (3♠) X`;
# pre-registered 2026-10-16, before any number is read).  It shipped on Round
# 4's single seed; Round 4's block above predates the ship, so its `base` and
# `fit` arms are now identical.  The rung does not read vulnerability, so the
# ship's cells: VULS="none both", `--wide-1nt` on both arms, fresh seed,
# BOARDS=2400000, results in ab-results/nt-high-overcall-n3fit.  The default
# stands in a cell that reads plain DD non-inferior or better with PD not a
# loss; any other reading is flagged for a decision, not applied.
if [ "${ROUND:-1}" = 10 ]; then
    for v in $VULS; do
        arm base "$v" --filter-preempt --wide-1nt --ns-nt-high-overcall-x-major-at-four false
        arm fit  "$v" --filter-preempt --wide-1nt
        gatepair fit base "$v"
        DENSITY=$(density_of "$R/base-$v")
        diffpair fit base "$v"
        sddiff   fit base "$v"
    done
fi

# Round 11 (`ROUND=11`, pre-registered 2026-10-17, before any number is read):
# the colour gate `nt_high_overcall_x_leave_in_unless_unfavourable`, built
# from Round 9's `ns` reading (plain DD non-inferior, sd-lead a loss on both
# columns).  The gate fires at `ns` only, so the A/B is VULS=ns, fresh seed,
# BOARDS=2400000, `--wide-1nt`, results in ab-results/nt-high-overcall-colour;
# a preflight at VULS="none ew both" (small BOARDS, its own dir) must read 0
# divergent first.
#   base    the ungated leave-in (on at every colour), now
#           --no-ns-nt-high-overcall-x-leave-in-unless-unfavourable
#   colour  the gate: no leave-in when we are vulnerable and they are not
#           (default since 2026-10-17; the flag spelling changed with the
#           flip, the arms did not)
# The knob takes doubles OUT, so this is the addendum's lane read the other
# way: plain DD arbitrates, the sd-lead bracket tie-breaks, PD is a
# double-blind column.  Rule: the gate ships default-on if `ns` reads plain DD
# non-inferior or better with sd-lead plain not a loss; anything else is
# flagged, not applied.
#
# First preflight FAILED 2026-10-17 (9de51c9f, ab-results/nt-high-overcall-
# colour-preflight: 9 / 11 / 6 divergent per 96,000 at none / ew / both).  The
# gate was spelled `long & (!vulnerable() | they_vulnerable())`, whose
# complement is ⊤, so the `3NT` below stopped denying the length at every
# colour and responder's later calls moved.  Rebuilt on `except_at`, which
# keeps every projection fold off its cell; preflight re-run in
# ab-results/nt-high-overcall-colour-preflight2.
#
# MEASURED 2026-10-17, sha e092aa82, SEED_BASE 1792236493237230273,
# ab-results/nt-high-overcall-colour, preflight2 0 divergent at none/ew/both,
# gate 0 foreign of 4,837 divergent at `ns`: plain −0.0002 ±0.0005
# (non-inferior), PD −0.0031 ±0.0005 (double-blind), sd-plain +0.0055
# ±0.0005, SD-PD +0.0030 ±0.0005 (+2.72 / +1.48 per fired).  SHIPPED
# default-on.
if [ "${ROUND:-1}" = 11 ]; then
    for v in $VULS; do
        arm base   "$v" --filter-preempt --wide-1nt --no-ns-nt-high-overcall-x-leave-in-unless-unfavourable
        arm colour "$v" --filter-preempt --wide-1nt
        gatepair colour base "$v"
        DENSITY=$(density_of "$R/base-$v")
        diffpair colour base "$v"
        sddiff   colour base "$v"
    done
fi

log "nt-high-overcall done"
