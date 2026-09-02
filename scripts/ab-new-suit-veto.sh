#!/bin/sh
# ab-new-suit-veto.sh — the envelope-gated new-suit veto on the learned floor.
#
# The net names trump suits nobody has.  Its input vector carries a raw
# we-bid-this-strain bit set for artificial calls too and no alert column, so a
# conventional `2♠` and a natural one are the same 176 floats; repairing that is
# a retrain.  `InstinctProfile::new_suit_veto` is the output-side alternative: after the
# legality mask, every suit bid in a suit where we hold at most four cards and
# our length plus partner's *announced* minimum reaches at most five is set to
# `-∞`.  No eight-card fit can satisfy that, so the rail is scoped off agreed
# fits by the predicate itself, and it carries no bid-identity term on purpose.
#
# Two arms per vul, identical deals, both playing `american()` UNFILTERED:
#   off    the shipped default (crate default off — byte-identical, pinned by
#          `smoke-default --count 20000 --seed 1` at this commit and at control)
#   veto   --ns-new-suit-veto
#
# Evidence it rests on (docs/one-notrump-competitive.md §N1-lia "The rail
# evidence"; scripts/ab-landy-lia3.sh VERDICT): on the lia3 forensic's divergent
# boards the *default system's own* floor made such calls on 6,800 (none) /
# 3,844 (both) boards and lost 5.2 / 6.5 IMPs per fired.  Those are pools over
# divergent boards, not bounds and not a measurement — the rail also fires on
# the millions of boards that never diverged, which is what this A/B is for.
#
# PRE-REGISTERED ARBITER.  This knob **bids less**, so read the direction before
# the row (docs/measurement.md:294-320): perfect defense doubles every failing
# contract and therefore flatters a rail that stops bidding them, which makes
# `loss | win` (:289) the artifact row, not a verdict.  Plain DD is the arbiter
# at BOTH colours with PD required non-negative — the competitive accountant's
# precedent, the only other demotion-only floor stage that ever shipped.  SD is
# a second pass only if plain earns it (:424-426), quoted as [SD-PD, plain SD]
# (:164).  Plain-DD loss never ships (:620-621).
#
# NO ISOLATION GATE.  The rail fires in every unauthored seat under either
# side's opening, so `gatepair` would fail by construction; neither
# ab-competitive-accountant.sh nor ab-net-collar.sh calls it either.  The
# ungated probe's ours/theirs opener split is recorded as an informational row
# instead — a 100%-one-side split would be the anomaly.
#
# `ab-dump-sd` has no disclosure flags for this and needs none: a floor rail
# changes no rule, alert or envelope, so the blind leader already reads both
# arms under exactly the right semantics.
#
# SIZING.  Pass 1 is the house default 204,800 bd/arm/vul (~8 min for all four
# cells).  Read the fired rate off bba-gen's stderr in $R/log; if the divergent
# count is thin, pass 2 goes in a FRESH dir (fresh seed) with BOARDS raised.
#
#   JOBS=32 BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-new-suit-veto.sh ab-results/new-suit-veto \
#       >ab-results/new-suit-veto.log 2>&1 < /dev/null & disown
#
# Iron rule: do NOT edit src/ or run cargo while this runs.
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-new-suit-veto.sh RESULTS_DIR}
SHOW=${SHOW:-40}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== new-suit veto start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm veto "$vul" --ns-new-suit-veto
    # Opener split + the per-board records the forensic replays; no
    # --gate-opener by design (see NO ISOLATION GATE above).
    split="$R/split.veto.vs.off.$vul.txt"
    grep -q 'who opened' "$split" 2>/dev/null || {
        log "opener split veto vs off ($vul)"
        "$PROBE" "$R/veto-$vul" "$R/off-$vul" --jsonl "$R/div-$vul.jsonl" >"$split"
    }
    diffpair veto off "$vul"
done
log "=== new-suit veto done"

# Post-hoc forensic, per vul — run by hand once the headline is read:
#   $PROBE $R/veto-$v $R/off-$v --imps --jsonl $R/imps-$v.jsonl
#   ./target/release/examples/probe-layer-replay $R/veto-$v --jsonl $R/imps-$v.jsonl \
#       --out $R/layers-$v.jsonl --ns-new-suit-veto
#   python3 scripts/divergence-layers.py $R/imps-$v.jsonl $R/layers-$v.jsonl veto
# Bucket what REPLACED the masked call: nothing forces the runner-up to be sane,
# and that is the one failure mode the predicate cannot guard against.
