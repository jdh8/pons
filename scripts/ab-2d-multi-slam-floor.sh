#!/bin/sh
# ab-2d-multi-slam-floor.sh — `competition.multi_minor_slam_try` at `15` (the
# shipped default) against `13`, head to head (docs/minor-transfer-slam.md
# Round 2; docs/archive/research-method-review-closed.md § Issue #9).
#
#   JOBS=96 BOARDS=192000 BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       DDSS_THREADS=96 setsid nohup scripts/idle-run.sh \
#       scripts/ab-2d-multi-slam-floor.sh ab-results/2d-multi-slam-floor \
#       >ab-results/2d-multi-slam-floor.log 2>&1 < /dev/null &
#
# Why a third round.  `15 ⊂ 13`, so `s15 vs s13` is the 13–14 slice in
# isolation.  Round 2 (2.3M `--filter-1nt` boards/arm/vul) saw it fire on 55
# boards, t +0.70 plain / +0.43 PD after round 1's opposite sign: `15` shipped
# as a choice, not a measured preference.  Separating the floors unfiltered
# needs ~20M boards/arm/vul.
#
# `--filter-multi-slam` buys that sample on the scan instead: a 1NT candidate,
# its LHO with a six-card major, its partner with a six-card minor and 9+ HCP.
# Piloted at seed 4242: 1 accepted per 802 scanned, s13/s15 diverging on 15 of
# 2,000 accepted boards (0.75%, ~300× round 2's density, and about the same
# per-scanned-deal rate, so the filter keeps essentially the whole slice).
# 192k accepted/arm/vul ≈ 1,400 fired per vul, 25× round 2's slice.
#
# Read the per-fired figure, not the per-board verdict: at this density every
# per-board CI clears δ, so either floor is non-inferior to the other by
# construction.  The question is the sign and size of the 13–14 slice.
# Density for the verdict line = accepted ÷ scanned, summed from the shards'
# `scanned` counts in this log; pass it to ab-dump-diff --density to rescore.
#
# VERDICT (2026-10-16, code 2bdc97a2 + the filter, gate 0 foreign in all
# four cells): `13` over `15` is win | win on plain DD and PD in every cell of
# two fresh seeds, sd-lead agreeing.  IMPs/fired, 13 − 15, none/both:
#   seed 1792092549718922360, 192k:  plain +1.04/+1.54  PD +1.23/+1.78
#   seed 1792092824585311714, 384k:  plain +0.72/+0.74  PD +0.92/+1.04
# (the second seed is ab-results/2d-multi-slam-floor-r2, BOARDS=384000).
# Default flipped to `Some(13)`.
#
# Scope: both seeds ran --filter-multi-slam with a strict balanced 1NT
# candidate (4333/4432/5332), missing the default's 5m422/6m322 openers.
# Wide confirmation (pre-registered 2026-10-16, before any number is read):
# the filter now takes any 15-17 HCP candidate with no suit under two; fresh
# seed, BOARDS=384000, results in ab-results/2d-multi-slam-floor-wide.  `13`
# stands if 13 over 15 is not a loss on either scorer in either cell;
# anything else is flagged for a decision, not applied.
# WIDE VERDICT (2026-10-16, SEED_BASE 1792102740877734887, density 0.001824,
# gate 0 foreign): 13 over 15 win | win in both cells, per accepted board
# none/both plain +0.0018 ±0.0012 / +0.0017 ±0.0011, PD +0.0023 ±0.0013 /
# +0.0020 ±0.0012; per fired plain +0.72/+1.29, PD +0.90/+1.53.  `13` stands.
#
# Resumable; SEED_BASE persists in $R/2d-multi-slam-floor.seed.  Iron rule: do
# NOT rebuild binaries while this runs.
R=${1:?usage: ab-2d-multi-slam-floor.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for 2d-multi-slam-floor)
log "=== 2d-multi-slam-floor SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in $VULS; do
    arm s13 "$v" --their-2d-multi --ns-multi-minor-slam-try 13 --filter-multi-slam
    arm s15 "$v" --their-2d-multi --ns-multi-minor-slam-try 15 --filter-multi-slam
    gatepair s15 s13 "$v"
    diffpair s15 s13 "$v"
    sddiff   s15 s13 "$v"
done

log "2d-multi-slam-floor done"
