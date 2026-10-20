#!/bin/sh
# ab-nt-invite-rail.sh — the 1NT-invite rail (`InstinctProfile::
# their_nt_invite_double_veto` / `their_nt_invite_bid_veto`) and the harness
# reading arm (`bba-gen --declare-their-european-minors`), vs BBA, on their
# 1NT auctions.  Step 2 of the 1NT-defense plan; the floor-rail series is
# reopened for this one family (jdh8, 2026-10-19; docs/floor-rail-campaign.md).
#
# Launch only after step 1's BEN A/B (ab-results/ben-nt-overcall-floor-2)
# finishes: sourcing ab-lib.sh rebuilds the shared release examples.
#
#   JOBS=96 BOARDS=196608 BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       DDSS_THREADS=96 setsid nohup scripts/idle-run.sh \
#       scripts/ab-nt-invite-rail.sh ab-results/nt-invite-rail \
#       >ab-results/nt-invite-rail.log 2>&1 < /dev/null &
#
# Why.  The clean `--isolate-defense natural` forensic at f8286a5a (none): when
# they open 1NT, our side has only passed and their last bid is an invitation
# (2NT, or a three-level raise of partner's strain), the v6 floor competes or
# doubles and goes for a number while BBA passes at every prefix — −0.0144
# plain / −0.0117 PD per accepted `--filter-1nt --wide-1nt` board (≈ −3.7 /
# −3.0 per 1,000 random deals), all 470 boards the un-narrowed gate fired on.
# The gate now reads only responder's 2NT or true major raise; it keeps the
# four priced nodes (366 boards) and drops ones netting ≈ 0 (responder's own
# suit again, Stayman follow-ups, opener's calls).  Nodes, per fired board
# plain/PD:
#   (1NT) - (2NT)               suit bid  160 bd  −2.00 / −2.92
#   (1NT) - (2NT) - -           X          78 bd  −6.85 / −1.37
#   (1NT) - (2♣) - (2♥) - (2NT) - -  3♣    76 bd  −8.57 / −9.97
#   (1NT) - (2♣) - (2♥) - (3♥)  X          52 bd −10.4  / −6.5
# Pass was the best surviving call on 56/56 sampled fired boards once the
# masked calls were removed.  Part of the cost is a misread, not junk: our
# side reads BBA's natural `(1NT) - (2NT)` invitation as our Puppet-scheme
# diamond transfer (Config::symmetric), which the read arm removes.
#
# Arms, one fresh SEED_BASE, `--filter-1nt --wide-1nt`, all four cells (we sit
# N/S, so `ns` is unfavourable and `ew` favourable):
#   off      bare — the shipped system (control)
#   dx       --ns-nt-invite-double-veto (the X half)
#   dxb      dx + --ns-nt-invite-bid-veto (suit bids of ≤5 cards too)
#   read     --declare-their-european-minors (harness: our side reads EPBot's
#            1NT minors as the European scheme EPBot plays — and, being a
#            declared opponent, every one of their calls through that book)
#   readdxb  read + dx + bid veto
#   woff     none only, descriptive: --isolate-defense --ns-notrump-defense
#            woolsey (the clean table-B-all-BBA isolation, our Woolsey defense)
#   wdxb     woff + dx + bid veto
# Diffs: every arm − off (gate, plain+PD, SD), readdxb − read (descriptive:
# the rail under the honest reading), and wdxb − woff (descriptive: the rail
# under Woolsey, plain+PD only — ab-dump-sd has no notrump-defense flag, so
# its leader would read our Woolsey calls as natural; no verdict rides it).
# The SD leader reads each read arm with
# `--{on,off}-declare-their-european-minors`; the rails need no leader flag
# (they move no reading, as the sibling rails).
#
# Gates.  dx, dxb, readdxb − read and wdxb − woff fire only on boards they
# open, so they gate `theirs` (0 foreign).  probe-divergence gates TABLE A
# only, while in these non-isolated arms the rail also fires at table B (our
# E/W defends BBA's 1NT there), so a gate's divergent count is about half of
# the divergent boards ab-dump-diff scores; the woff/wdxb pair is isolated
# (table B all-BBA), so there the two counts agree.  The read arm moves our
# reading of their calls on EVERY board — their overcall or X of our 1NT
# included — so read − off and readdxb − off are NOT gated: their divergence
# spans both openers by construction, and their headline is the whole
# protocol change, not a defensive attribution.
#
# Sizing.  196,608 accepted boards per arm per vul (JOBS=96 → 2,048/shard),
# five arms × four cells ≈ 3.9M bba-gen boards, plus the Woolsey pair.  δ is
# per accepted board: 0.002 / density, density from the off arm's shards
# (≈ 0.25, so δ ≈ 0.008); DENSITY is exported to every diffpair/sddiff so
# the verdict is not withheld.
# The Woolsey pair's BOARDS are kept (BBA-opens-1NT, we-defend) boards, its
# density kept ÷ scanned, so it deals about four times as many boards as an
# accepted-board arm.
#
# STOP RULE (pre-registered with the spec): fewer than 50 divergent boards in
# a cell's dx or dxb gate (table A, the gate line's "of N divergent boards")
# → stop before any DD, and extend: a fresh results dir seeded with this
# run's SEED_BASE (`cp $R/nt-invite-rail.seed $R2/`) and BOARDS doubled
# (393216).  Shard i still deals from SEED_BASE+i, so the extension is a
# superset of this run's boards, regenerated whole (ab-lib pins per-shard per
# dir).  The runner enforces the stop below.
#
# PRE-REGISTERED 2026-10-19, before any number exists (docs/measurement.md
# decision table, plain DD + PD, each scorer its own verdict, δ = 0.002
# IMPs/board scaled by density; arm − off per cell, the worst cell decides):
#   - An arm is a SHIP CANDIDATE if every cell reads plain `win` + PD `win`,
#     or plain `non-inferior` (lower bound above −δ) + PD `win`.  A plain win
#     that PD does not confirm is a doubling artifact; plain loss + PD win is
#     PD's synthetic-X artifact; neither is a candidate.  Plain
#     `inconclusive` (lower bound at or below −δ) is insufficient data, never
#     "no loss": extend on FRESH deals (a new SEED_BASE, BOARDS=393216,
#     fixed here before any read), or record a trade-off.  SD-PD is read
#     alongside (the masked X can be lead-directing), not instead.
#   - dx vs dxb: if dxb is a candidate and dx is not, propose dxb (both
#     knobs on).  If both are candidates, prefer dx (the narrower mask); the
#     bid veto then needs its own case (dxb − dx is the readout, owed only if
#     jdh8 asks).  Neither → both stay off.
#   - Any loss cell in dx or dxb: before calling it dead, trace the worst 30
#     boards (`ab-dump-diff --show 30`) for re-spent masked mass — the
#     silent_cue_veto failure (the net moves the masked X/bid mass onto other
#     junk, e.g. a doubled 3NT) — and for a natural long-suit competition the
#     6-card cut should have spared.
#   - The read arm is a HARNESS/PROTOCOL question, not a system change.  A
#     read win goes to jdh8 as "declare their 1NT minors in bba-gen by
#     default" — it would move the anchor series, so it is NOT applied by this
#     runner's verdict.  Caveat pre-registered with it: the declaration moves
#     every opponent-call reading (bba-gen --help), so a read win is a win of
#     the declared-opponent protocol, not of the minors axis alone.
#   - readdxb − read is descriptive: a rail candidate under `off` that loses
#     under `read` says the rail was compensating a misread — flagged for
#     jdh8, not applied.
# Nothing ships from this runner; every default is jdh8's call.
#
# Resumable; SEED_BASE persists in $R/nt-invite-rail.seed.  Iron rule: do NOT
# rebuild binaries while this runs.
R=${1:?usage: ab-nt-invite-rail.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for nt-invite-rail)
log "=== nt-invite-rail SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul vuls=$VULS"

FILTER='--filter-1nt --wide-1nt'
# Both rails shipped default-on 2026-10-20, so the arms now disarm them:
# off disarms both, dx disarms the bid veto, dxb is the bare default.
OFF='--no-ns-nt-invite-double-veto --no-ns-nt-invite-bid-veto'
DX='--no-ns-nt-invite-bid-veto'
DXB=''
# The declaration became bba-gen's default 2026-10-20 (and ab-dump-sd reads
# each arm as its dump records), so the symmetric arms now opt out and the
# read arms are the bare default.
SYM='--no-declare-their-european-minors'
READ=''
for v in $VULS; do
    # shellcheck disable=SC2086
    arm off "$v" $FILTER $OFF $SYM
    # shellcheck disable=SC2086
    arm dx "$v" $FILTER $DX $SYM
    # shellcheck disable=SC2086
    arm dxb "$v" $FILTER $DXB $SYM
    # shellcheck disable=SC2086
    arm read "$v" $FILTER $READ $OFF
    # shellcheck disable=SC2086
    arm readdxb "$v" $FILTER $READ $DXB
    DENSITY=$(density_of "$R/off-$v")
    log "density $v: $DENSITY"
    for a in dx dxb; do
        gatepair "$a" off "$v" theirs
        n=$(sed -n 's/.* of \([0-9]*\) divergent boards opened by .*/\1/p' \
            "$R/gate.$a.vs.off.$v.txt")
        if [ "${n:-0}" -lt 50 ]; then
            log "STOP $a $v: ${n:-0} divergent boards at table A (< 50); extend per the header"
            exit 1
        fi
        diffpair "$a" off "$v"
        sddiff "$a" off "$v"
    done
    for a in read readdxb; do
        diffpair "$a" off "$v"
        sddiff "$a" off "$v"
    done
    gatepair readdxb read "$v" theirs
    diffpair readdxb read "$v"
    sddiff readdxb read "$v"
done

# Descriptive: the rail under our Woolsey defense, none only (isolated).
WOOLSEY='--isolate-defense --ns-notrump-defense woolsey'
# shellcheck disable=SC2086
arm woff none $FILTER $WOOLSEY $OFF $SYM
# shellcheck disable=SC2086
arm wdxb none $FILTER $WOOLSEY $DXB $SYM
DENSITY=$(density_of "$R/woff-none")
log "density woolsey none: $DENSITY"
gatepair wdxb woff none theirs
diffpair wdxb woff none

log "=== nt-invite-rail done"
