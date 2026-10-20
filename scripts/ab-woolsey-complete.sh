#!/bin/sh
# ab-woolsey-complete.sh — complete our opt-in Woolsey (Multi-Landy) defense
# to their 1NT: the two completion knobs against Woolsey as it stands
# (1NT-defense plan, step 2, package C; docs/one-notrump-competitive.md is
# the 1NT-contested campaign, this is our *defense* to their 1NT).
#
# Launch only after step 1's BEN A/B (ab-results/ben-nt-overcall-floor-2)
# finishes: sourcing ab-lib.sh rebuilds the shared release examples.
#
#   JOBS=96 BOARDS=102400 BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       DDSS_THREADS=96 setsid nohup scripts/idle-run.sh \
#       scripts/ab-woolsey-complete.sh ab-results/woolsey-complete \
#       >ab-results/woolsey-complete.log 2>&1 < /dev/null &
#
# Why.  Woolsey is a row in step 3's 1NT-defense tournament, and its rows
# must be completed to the same standard as the others.  The forensics (a
# clean `--isolate-defense` Woolsey run at f8286a5a, none vul, 204,800 kept
# boards, table B all-BBA) priced its holes per kept board, plain / PD:
#   - `(1NT) 2M (X) 2NT`, the doubled-Muiderberg escape: −0.0109 / −0.0178
#     gross, all of it ending in 3♣x/3♦x, 94% of it escaping on a doubleton
#     in the major (BBA escapes on 0–1 in ~82%: 128/156 over 2♥, 126/151
#     over 2♠).  Sitting does not buy 2Mx — E/W left it in on 3 of 735 —
#     so the net estimate is ≈ +0.009 / +0.012 once the run-out is passed;
#   - `(1NT) 2M (X) - (3♦)`: our floor re-competes in the major on all 235
#     boards where BBA passes, −0.0023 / −0.0065;
#   - `(1NT) 2♦ (X) 2♥ - - (X)`: 96 boards, −0.0017 / −0.0026, the floor
#     runs to 2♠ on 59 where BBA passes;
#   - `(1NT) 2♦ (2♥)` / `(2♠)`: 172 boards, −0.0022 / −0.0010, BBA bids the
#     2♠ / 3♥ pass-or-correct;
#   - the `(1NT) 2♣` advance has no finite catch-all (12–13 points, no
#     four-card major, falls to the floor) — iron-rule hygiene, no price.
#
# Arms, one fresh SEED_BASE, every arm `--isolate-defense --ns-notrump-defense
# woolsey --filter-1nt --wide-1nt` (BBA opens 1NT, we defend; table B is
# all-BBA), all four cells:
#   base  = Woolsey as shipped (opt-in)
#   sit   = base + `--ns-woolsey-muiderberg-sit` (defense.woolsey_muiderberg_sit:
#           the 2NT escape only void/singleton, else sit 2Mx; overcaller sits
#           `(1NT) 2M (X) - -` and passes their run-out)
#   tails = base + `--ns-woolsey-tails` (defense.woolsey_tails: the 2♣ advance
#           catch-all, `(1NT) 2♦ {-|(X)} 2♥ - - (X)` and its direct-double
#           twin `… 2♥ (X) - -`, the answers to `(1NT) 2♦ (2♥)`/`(2♠)`, the
#           overcaller's answers to those and its balance after `- -`)
#   both  = base + both flags
# The kept set is the same in every arm: the knobs act only after BBA's 1NT.
# Diffs: sit − base, tails − base, both − base, each with the isolation gate
# (`theirs`: every divergent board opened by BBA) and the SD leader reading
# both arms as Woolsey with the matching `--{on,off}-ns-woolsey-*` flags.
#
# Sizing: BOARDS=102400 kept per arm per cell (PER_SHARD rounds up to a
# multiple of 4 per shard).  Density = kept ÷ scanned via density_of
# (≈ 0.0586 expected), so δ per kept board = 0.002 / density ≈ 0.034 IMPs.
#
# PRE-REGISTERED 2026-10-19, before any number exists; knob − base per cell,
# the standard docs/measurement.md decision table, δ = 0.002 IMPs per random
# deal (`--density` converts), each scorer's verdict is its worst cell, each
# knob read on its own:
#   - Ship the knob **into the Woolsey card** (default on inside
#     `NotrumpDefense::Woolsey` only; the natural default system is untouched,
#     since Woolsey itself is opt-in) iff plain DD is non-inferior or a win
#     (every cell's 95% lower bound > −δ) AND PD is a win (every cell's
#     lower bound > 0).
#   - A PD-only win (plain DD not non-inferior) is a doubling artifact: no
#     ship.  A plain win with PD merely non-inferior (or worse) is a plain
#     win PD erases: suspect, no ship.
#   - Plain non-inferior + PD non-inferior: the completion stays opt-in (no
#     proof of gain); flagged for jdh8 as iron-rule hygiene that may ship on
#     naturalness as a recorded trade-off — his call.
#   - Any cell `inconclusive` on plain DD (PD not a loss): insufficient data.
#     The extension is fixed now: rerun this runner into a new RESULTS_DIR
#     (fresh SEED_BASE, `deal-overlap` checked) with BOARDS doubled —
#     BOARDS=204800 kept per arm per cell at the 102400 launch above — all
#     four cells, the same arms; read the extension alone, never pooled
#     with the first run.  No further top-up.
#   - Any cell `loss` on either scorer: split the divergent boards by node
#     (which knob node fired first) before calling the knob dead; a loss
#     confined to one node retires that node, not the knob.
#   - both − base is read only as an interaction check: if it is worse than
#     the better single knob by more than its CI, flag it before shipping both.
#   - SD-PD is descriptive; it neither ships nor kills.
# Nothing is changed by this runner's verdict; the default is jdh8's call.
#
# Resumable; SEED_BASE persists in $R/woolsey-complete.seed.  Iron rule: do
# NOT rebuild binaries while this runs.
R=${1:?usage: ab-woolsey-complete.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for woolsey-complete)
log "=== woolsey-complete SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul vuls=$VULS"

BASE='--isolate-defense --ns-notrump-defense woolsey --filter-1nt --wide-1nt'
# Both knobs shipped default-on inside Woolsey 2026-10-20, so the arms now
# disarm them: base disarms both, sit and tails each disarm the other.
NOSIT='--no-ns-woolsey-muiderberg-sit'
NOTAILS='--no-ns-woolsey-tails'
READ='--on-ns-notrump-defense woolsey --off-ns-notrump-defense woolsey
      --off-no-ns-woolsey-muiderberg-sit --off-no-ns-woolsey-tails'
for v in $VULS; do
    # shellcheck disable=SC2086
    arm base "$v" $BASE $NOSIT $NOTAILS
    # shellcheck disable=SC2086
    arm sit "$v" $BASE $NOTAILS
    # shellcheck disable=SC2086
    arm tails "$v" $BASE $NOSIT
    # shellcheck disable=SC2086
    arm both "$v" $BASE
    DENSITY=$(density_of "$R/base-$v")
    log "density $v: $DENSITY"
    for a in sit tails both; do
        gatepair "$a" base "$v" theirs
        diffpair "$a" base "$v"
    done
    # shellcheck disable=SC2086
    sddiff sit base "$v" $READ --on-no-ns-woolsey-tails
    # shellcheck disable=SC2086
    sddiff tails base "$v" $READ --on-no-ns-woolsey-muiderberg-sit
    # shellcheck disable=SC2086
    sddiff both base "$v" $READ
done

log "woolsey-complete done"
