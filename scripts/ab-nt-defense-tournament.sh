#!/bin/sh
# ab-nt-defense-tournament.sh — step 3 of the 1NT-defense plan: one
# tournament, our five defenses to their strong 1NT as rows, six opponents as
# columns (docs/ai-bidder/gto-1nt-defense.md § Step 3).
#
#   JOBS=96 DDSS_THREADS=96 RAYON_NUM_THREADS=96 SERVERS=48 \
#   BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       setsid nohup scripts/idle-run.sh scripts/ab-nt-defense-tournament.sh \
#       ab-results/nt-defense-tournament \
#       >ab-results/nt-defense-tournament.log 2>&1 </dev/null &
#
# PHASE=all (default) builds every harness once, then runs the phases in
# order: bba (≈ 9 h here), self (≈ 10 h), ben (≈ 23 h).  PHASE=bba|self|ben
# runs one.  Each phase resumes; the ben phase needs exactly SERVERS Tier-F
# BEN servers up and unchanged throughout (scripts/ben-servers.sh).
#
# Rows (ranked; jdh8 2026-10-21 put all five in):
#   pass     --ns-notrump-defense always-pass        (the datum, ≡ 0)
#   natural  --ns-notrump-defense natural            (shipped default)
#   dont     --ns-notrump-defense direct-dont --ns-dont-one-suiter-min 6
#   woolsey  --ns-notrump-defense woolsey            (completed in step 2)
#   meckwell --ns-notrump-defense meckwell
# each as `ab-nt-defense-matrix` builds it (its Woolsey row fields the
# shipped card since 2026-10-21; DONT/Meckwell carry the 8–14 `2NT` through
# `common::set_notrump_defense` in every harness).
#
# Columns: BBA (`bba-gen --isolate-defense --filter-1nt --wide-1nt`, table B
# all-BBA, BOARDS kept boards per arm per vul), BEN (the same through
# `ben-gen`, Tier F, BEN_BOARDS), and the four self-play counters of
# `ab-nt-defense-matrix` (default, penalty-X, soft, sit; MATRIX_COUNT kept
# boards per vul).  Every vul cell: none, both, ns, ew (we sit N/S).
#
# Caveats, stated before any number:
#   - DONT and Meckwell were not completed in step 2 (Woolsey and natural
#     were), so their cells price the implementation as well as the card.
#   - BBA's card has no DONT or Meckwell row: our generated card tells BBA
#     "natural" for both, so BBA reads our artificial calls as natural.  BEN
#     reads us with its own system.  The self-play columns read every call
#     off its authoring rule.  Woolsey is read natively by BBA (Multi-Landy).
#   - Units are IMPs per kept board (they opened 1NT).  The columns' kept
#     sets differ (each opponent's own 1NT), so a row's columns are
#     comparable as "per board where this opponent opened 1NT", not as one
#     population.
#
# PRE-REGISTERED 2026-10-21, before any number exists.
#   Cells.  M[r][c] = row r − pass on the same boards, IMPs per kept board,
#   with its 95% CI: BBA/BEN from `diff.<r>.vs.pass` (plain DD) and
#   `sd.<r>.vs.pass` (SD-PD line); self-play from the matrix's plain-DD and
#   "sd-lead + perfect defense" tables.  pass ≡ 0 in every column.
#   The realism pair is [plain DD, SD-PD].  PD and plain SD are reported,
#   never read.
#   1. Per (vul, scorer): V_r = min over the six columns of M[r][c]; the
#      maximin row W = argmax V_r over the five rows (V_pass = 0).
#   2. W is RESOLVED over a rival R iff in every column c the lower bound of
#      M[W][c] − V_R is above 0, combining the two CIs as independent
#      (conservative for the common-random-number cells).  Over pass that is
#      every M[W][c] lower bound above 0.
#   3. THE CARD is a row that is the resolved maximin winner, over every
#      rival, in all eight (vul, scorer) cells.  Anything else (a split by
#      vul or by scorer, or an unresolved cell) yields no card; the table and
#      the per-cell winners go to jdh8.  A mixture over cards is not
#      playable, so a mixed equilibrium is reported, never shipped.
#   4. The card replaces the shipped default (natural) only through the
#      standard decision table: `diff.<card>.vs.natural` on BBA, plain DD
#      non-inferior and PD win in every vul cell (δ = 0.002 / density), BEN
#      as the guard.  If the card is pass or natural, nothing moves.
#   5. Every isolation gate (`gate.<r>.vs.pass`) must read 0 foreign before
#      its cells are quoted.
# Read the table with scripts/nt-defense-tournament.py RESULTS_DIR.
#
# Iron rule: never rebuild this checkout's release examples while it runs.
# Run it from its own worktree (the launch above, from that worktree, with
# PONS_ROOT set to it so BBA_GEN's wrapper runs that tree's bba-gen).
set -eu
ROOT=${1:?usage: ab-nt-defense-tournament.sh RESULTS_DIR}
HERE=$(cd "$(dirname "$0")" && pwd)

case ${PHASE:-all} in
all)
    cd "$HERE/.."
    cargo build --release --features serde --example bba-gen --example ben-gen \
        --example ab-dump-diff --example ab-dump-sd --example probe-divergence \
        --example ab-nt-defense-matrix
    for p in bba self ben; do
        PHASE=$p SKIP_BUILD=1 "$HERE/ab-nt-defense-tournament.sh" "$ROOT"
    done
    echo "nt-defense-tournament done $(date -u +%FT%TZ)"
    exit 0
    ;;
self)
    BINDIR=$HERE/../target/release/examples COUNT=${MATRIX_COUNT:-60000} \
        VULS=${VULS:-none both ns ew} exec "$HERE/sd-pd-nt-defense.sh" "$ROOT/self"
    ;;
bba)
    R=$ROOT/bba
    GENERATOR=bba-gen
    BOARDS=${BOARDS:-102400}
    ;;
ben)
    R=$ROOT/ben
    GENERATOR=ben-gen
    JOBS=${SERVERS:-48}
    BOARDS=${BEN_BOARDS:-16384}
    ;;
*)
    echo "PHASE must be all, bba, self or ben" >&2
    exit 1
    ;;
esac

VULS=${VULS:-none both ns ew}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$HERE/ab-lib.sh"

ROWS='natural dont woolsey meckwell'
COMMON='--isolate-defense --filter-1nt --wide-1nt'
EXTRA=
if [ "$GENERATOR" = ben-gen ]; then
    NOTE=$(ben_note)
    EXTRA="-t f --note $NOTE"
fi

family() {
    case $1 in
        pass) echo always-pass ;;
        dont) echo direct-dont ;;
        *) echo "$1" ;;
    esac
}
bid_flags() {
    echo "--ns-notrump-defense $(family "$1")"
    if [ "$1" = dont ]; then echo --ns-dont-one-suiter-min 6; fi
}
read_flags() {
    echo "--on-ns-notrump-defense $(family "$1") --off-ns-notrump-defense always-pass"
    if [ "$1" = dont ]; then echo --on-ns-dont-one-suiter-min 6; fi
}

SEED_BASE=$(seed_for tournament)
log "=== nt-defense-tournament/$PHASE SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul vuls=$VULS"

for v in $VULS; do
    for r in pass $ROWS; do
        # shellcheck disable=SC2046,SC2086
        arm "$r" "$v" $EXTRA $COMMON $(bid_flags "$r")
    done
    DENSITY=$(density_of "$R/pass-$v")
    log "density $v: $DENSITY"
    for r in $ROWS; do
        gatepair "$r" pass "$v" theirs
        diffpair "$r" pass "$v"
    done
    # Rule 4's input: each conventional row against the shipped default.
    for r in dont woolsey meckwell; do
        diffpair "$r" natural "$v"
    done
    for r in $ROWS; do
        # shellcheck disable=SC2046
        sddiff "$r" pass "$v" $(read_flags "$r")
    done
done
log "nt-defense-tournament/$PHASE done"
