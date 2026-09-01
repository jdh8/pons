#!/bin/sh
# ab-2d-multi-px.sh — the Kokish–Kraft `P`/`X` information split
# (docs/one-notrump-competitive.md §N4-KK, docs/one-notrump-multi.md).
#
#   SKIP_BUILD=1 JOBS=24 BOARDS=230400 setsid nohup scripts/idle-run.sh \
#       scripts/ab-2d-multi-px.sh ab-results/2d-multi-px \
#       >ab-results/2d-multi-px.log 2>&1 < /dev/null &
#
# `SKIP_BUILD=1` is not optional politeness.  `bba-gen-parallel.sh` rebuilds
# `target/release/examples/bba-gen` at the head of *every* arm, so without it an
# edit made to `src/` while the run is in flight silently re-arms the harness
# between arms.  `ab-lib.sh` still builds once at the top, from whatever the
# tree holds when the script starts — so start it on a clean tree.  (Paid for
# 2026-08-26: this script's own sibling had to be killed and restarted for it.)
#
# What is being tested.  `competition.multi_px_split` splits responder's first
# call over their `(2♦)` Multi by **information** instead of by strength alone:
#
#   X   `hcp(10..) | (hcp(8..=9) & (len(♥, 4..) | len(♠, 4..)))`, where K–K
#       plays a flat `hcp 8+`.  Weight 130, the `comp:kk-values` alert and the
#       `.penalty()` PDI tag are all unchanged, so the only thing that moves is
#       which hands make the call.
#   -   picks up 8–9-no-four-card-major by exclusion.  That complement — NOT
#       the new disjunct, which is the half the double keeps — is what changes
#       branch, and it is narrower than it reads: a five-card major already
#       escapes at 140 or transfers at 180, a six-card minor already takes the
#       floorless transfer at 176/178, and 8–9 HCP with 10+ *points* on
#       distribution already takes `3NT`@150 or `3♠`@152.
#
# FOUR mechanisms ride this one knob.  Say so out loud, because `px` vs `base`
# confounds all four and no single arm here separates them:
#
#   1  the double's own constraint, above;
#   2  the doubler's natural other major (shipped default-on 2026-08-26 at
#      weight 100) re-weighted to **148** — above the natural invitational
#      `2NT`@145, below `3NT`@150.  An 8–9 doubler now always holds a four-card
#      major, so showing it is the message rather than a last resort;
#   3  the `X (2♠) - -` leg, withheld from `multi_doubler_major` on a 25-board
#      census cell, is **re-armed** — under the split a hearts-only hand there
#      is the population that used to sell out, not the misfit tail.
#      `X (2♥) - (2♠)` stays excluded either way: the mechanism there is
#      opener's *pass* denying four hearts, which no responder-side split can
#      change;
#   4  one answer table moves.  The pass branch's delayed `2NT` stops being a
#      `hcp == 7` relic and opener accepts on `hcp 16+`
#      (`kokish_kraft_invite_answer`).
#
# **Mechanism 4 used to have a second half, and no longer does.**  Opener's
# `3NT`@135 answer to the natural other major was part of this split until
# `competition.multi_doubler_notrump` shipped default-on 2026-08-27 (a win in
# all four cells, `docs/multi-doubler-answer-handoff.md`).  It is now in the
# `base` arm too, so this A/B measures the P/X information split *alone* —
# which is the cleaner isolation, and the reason the arms were left as they
# are.  The old reasoning still holds mechanically: re-weighting to 148 sends
# more traffic to that answer table (the 8–9 doublers with a stopper, who used
# to bid `2NT`), so the split needs the repair under it — it just inherits it
# from the default now instead of carrying it.  Both knobs still emit one rung
# (`multi_px_split || multi_doubler_notrump`), so disarming the default with
# `--no-ns-multi-doubler-notrump` would put it back in the `px` arm only.
#
# Two arms, one seed set, `--their-2d-multi --filter-1nt` on both.  There is no
# `dm` arm any more: `multi_doubler_major` shipped default-on 2026-08-26, so it
# is *in* `base`, and the split is measured against the book as it now stands.
#
#   base   the shipped K–K table, including the rung at weight 100 on two legs
#   px     `--ns-multi-px-split` — all four mechanisms above
#
# SIZE.  Unlike `ab-2d-multi-doubler.sh`, whose rung fires inside a 0.9% bucket
# and needed 2.3M bd/vul to reach ~1300 fired, this arm's divergence surface is
# **every 8–9 board that used to double** — the whole X/P frontier of the lane,
# which the K–K A/B reached at 230 400 bd/vul.  So start at N4-KK scale
# (`BOARDS=230400`) and scale up only if the per-fired paired diff is
# under-powered.  The resolution constant is unchanged: 5.39 IMPs sd per
# contract-divergent board, so a cell needs |Δ| > 10.56/√n_div IMPs per
# divergent board.
#
# Scoring.  Plain AND perfect defense, read off the decision table in
# docs/measurement.md; `sddiff` is the tie-breaker.  Read
# `probe-divergence --gate-opener ours` BEFORE any headline — the K–K table
# feeds the mirror lane, and the mirror-read leak (fixed at `29f93561`) is what
# makes a counter knob a reading knob.  The gate must read **0 foreign**.
#
# Watch cells for the forensic, in the order the evidence ranks them:
#   1  both-vul perfect defense on the `2M`/`3M` rung — this is where the
#      shipped sibling lost, and mechanism 2 enlarges the population;
#   2  the delayed `2NT` (49 bd, −166 PD at its old `hcp == 7` band; the split
#      widens the band and accepts on 16, so 23 combined can reach `3NT`);
#   3  the `3♥` leg's `hcp 16+` game answer, which has no invitational rung
#      under it and so bids game on 24 combined at the four level.
#
# Resumable: an existing arm dir, gate, or diff file is skipped, and SEED_BASE
# persists in $R/2d-multi-px.seed.
R=${1:?usage: ab-2d-multi-px.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
PROBE=target/release/examples/probe-divergence

gatepair() {
    on=$1; off=$2; vul=$3
    out="$R/gate.$on.vs.$off.$vul.txt"
    [ -s "$out" ] && { log "skip $out (exists)"; return 0; }
    log "isolation gate $on vs $off ($vul)"
    "$PROBE" "$R/$on-$vul" "$R/$off-$vul" --gate-opener ours >"$out"
}

SEED_BASE=$(seed_for 2d-multi-px)
log "=== 2d-multi-px SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --their-2d-multi --filter-1nt
    arm px   "$v" --their-2d-multi --ns-multi-px-split --filter-1nt

    gatepair px base "$v"
    diffpair px base "$v"
    sddiff   px base "$v"
done

log "2d-multi-px done"
