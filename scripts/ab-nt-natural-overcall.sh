#!/bin/sh
# ab-nt-natural-overcall.sh — M1 + M2 of the `(1NT) 2♦` mirror forensic
# (docs/defensive-overcalls.md § "Defense to their 1NT — the (1NT) 2♦ mirror panel").
#
#   JOBS=24 PER_SHARD=19200 setsid nohup scripts/idle-run.sh \
#       scripts/ab-nt-natural-overcall.sh pack ab-results/nt-natural-overcall \
#       >ab-results/nt-natural-overcall.log 2>&1 < /dev/null &
#
# The lane: our four natural two-level overcalls of *their* 1NT
# (`chain_natural_overcalls`, `len(suit,5..) & points(8..=14)`).  Two knobs,
# both default-off and byte-identical off:
#
#   M1  `defense.natural_overcall_hcp_floor` — a raw-HCP floor on top of the
#       points band.  `point_count` is HCP *plus distribution*, so `points(8..)`
#       admits 6- and 7-HCP hands through shape; the ≤7-HCP tail is 12.3% of the
#       lane and the ONLY slice in the whole forensic negative on both scorers at
#       both vulnerabilities (264 bd, −120 plain / −158 PD; nv −43/−66, vul
#       −77/−92).  Two candidate floors, `k = 8` and `k = 9`.
#
#   M2  `defense.natural_overcall_advance_enabled` — the `(1NT) 2x - ?` advance,
#       today the instinct floor, whose `2NT` rung fires on 33 boards of the `2♦`
#       lane at −4.09/bd plain and fails 26 of 33.  Its own size across all four
#       suits is ≈ −0.0013 IMPs/bd of an arm — BELOW this A/B's resolution.  It
#       rides the package as the iron rule's completed continuation, and its
#       verdict is "correctness fix, unresolvable alone", never "a wash".
#
# PRE-REGISTERED, before any output is read:
#
#   * Ship candidate is the PACKAGE at `k = 8` — the cut whose removed slice is
#     negative on both scorers at both vulnerabilities.  `k = 9` (mode `nine`) is
#     exploratory: its plain component is near-wash non-vul, which is the
#     `plain win | PD loss` shape §O4 already lost with.
#   * Arbiter is the realism pair [plain DD, SD-PD] (measurement.md:188-212),
#     both vulnerabilities, two seeds.  Ship default-on only on a NON-LOSS on
#     plain DD **and** a WIN on SD-PD.  A plain-DD loss with a PD win is the §O4
#     shape and the knobs stay opt-in.
#   * EXPECT A TIGHTENING TO GIVE SOMETHING BACK.  §O4 killed the analogous blunt
#     points-floor move on 2026-08-12 at plain −0.0102 ±0.0021 NV, mechanism
#     "declare-vs-defend switch", conclusion *points are the wrong axis*.  M1 is
#     the HCP refinement that precedent argues FOR, not another band move — but
#     the same declare-vs-defend switch is live here.
#   * The anchor CANNOT penalty-double our two-level overcall (0 of 1135 doubled
#     boards ended in `2♦x`; its `X` is takeout).  So plain DD is the optimistic
#     end for the loose default and the pessimistic end for the tightening, and
#     PD is the reverse.  Neither column is an artifact to wave away.
#   * `bisect` is held IN RESERVE.  Run it only if `pack` is a loss or a
#     disagreement between the two arbiter columns — never to pick the better of
#     three arms after the fact.
#
# Design rules this script encodes:
#   * `--filter-1nt` rides EVERY arm, applied before any bidding, so all arms
#     deal the same board set and stay seed-aligned for paired diffs.  It gates
#     on *any* seat holding a 1NT opener, which is the cheap prefilter — the lane
#     itself (THEIR 1NT, our 5-card suit) is a subset.
#   * `--gate-opener theirs` must read ~0 foreign on every pair before a headline
#     is quoted: this is a *defensive* lane, so a divergent board opened by us is
#     the knob reaching an auction it does not own.
#   * `sddiff` discloses both arms' knobs to the blind leader
#     (`--on-ns-*` / `--off-ns-*`), or the sampled worlds are drawn under the
#     wrong overcall band and the SD-PD column is meaningless.
#   * Round 1 (seed set 1) is a SCREEN, not a ship verdict; seed set 2 replicates.
#     `SEED_BASE` is persisted per set so a killed run resumes on the same seeds.
#
# POLARITY, since the package SHIPPED 2026-08-23: both knobs are now the
# default, so `base` is the arm carrying flags (`--ns-nt-overcall-hcp-floor 0
# --no-ns-nt-overcall-advance`) and the treatment arm is bare.  A re-measure that
# forgets this measures nothing — check `arm base` still pins both OFF before
# quoting any number.
#
# Headline is IMPs per *accepted* deal (`--filter-1nt` is applied before any
# bidding, so an arm's boards ARE its accepted deals).
#
# RESULT: `pack` ran 2026-08-23 and SHIPPED — see the A/B table in
# docs/defensive-overcalls.md.  `ab-results/nt-natural-overcall/` holds those
# arms, generated under the PRE-ship polarity (base bare, pack8 flagged); they
# are not resumable against this script as it now stands.  Point a re-measure at
# a fresh results dir.
#
# Resumable: each seed set owns its seed, arms, probes and diffs.  Iron rule:
# do NOT rebuild binaries while this runs.
MODE=${1:?usage: ab-nt-natural-overcall.sh pack|nine|bisect RESULTS_DIR}
ROOT_R=${2:?usage: ab-nt-natural-overcall.sh pack|nine|bisect RESULTS_DIR}
R=$ROOT_R
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
PROBE=target/release/examples/probe-divergence

gatepair() {
    on=$1; off=$2; vul=$3
    out="$R/gate.$on.vs.$off.$vul.txt"
    [ -s "$out" ] && { log "skip $out (exists)"; return 0; }
    log "isolation gate $on vs $off ($vul)"
    "$PROBE" "$R/$on-$vul" "$R/$off-$vul" --gate-opener theirs >"$out"
}

# compare ON OFF VUL [disclosure flags for ab-dump-sd] — the three columns.
compare() {
    on=$1; off=$2; vul=$3; shift 3
    gatepair "$on" "$off" "$vul"
    diffpair "$on" "$off" "$vul"
    sddiff "$on" "$off" "$vul" "$@"
}

for seed_set in 1 2; do
    R="$ROOT_R/seed-$seed_set"
    mkdir -p "$R"
    SEED_BASE=$(seed_for nt-natural-overcall)
    log "=== nt-natural-overcall $MODE seed=$seed_set SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

    # The pre-ship system: both knobs pinned off.  OFF_FLAGS is also the
    # disclosure the blind leader needs for whichever arm carries them.
    OFF_FLAGS="--ns-nt-overcall-hcp-floor 0 --no-ns-nt-overcall-advance"
    for v in none both; do
        # shellcheck disable=SC2086
        arm base "$v" --filter-1nt $OFF_FLAGS
        case "$MODE" in
        pack)
            arm pack8 "$v" --filter-1nt
            compare pack8 base "$v" \
                --off-ns-nt-overcall-hcp-floor 0 --off-no-ns-nt-overcall-advance
            ;;
        nine)
            arm pack9 "$v" --filter-1nt --ns-nt-overcall-hcp-floor 9
            compare pack9 base "$v" --on-ns-nt-overcall-hcp-floor 9 \
                --off-ns-nt-overcall-hcp-floor 0 --off-no-ns-nt-overcall-advance
            ;;
        # Reserve: bisect a package verdict into its two halves.  M2 alone is
        # below resolution by construction — it is here to attribute, not to judge.
        bisect)
            arm m1 "$v" --filter-1nt --no-ns-nt-overcall-advance
            arm m2 "$v" --filter-1nt --ns-nt-overcall-hcp-floor 0
            compare m1 base "$v" --on-no-ns-nt-overcall-advance \
                --off-ns-nt-overcall-hcp-floor 0 --off-no-ns-nt-overcall-advance
            compare m2 base "$v" --on-ns-nt-overcall-hcp-floor 0 \
                --off-ns-nt-overcall-hcp-floor 0 --off-no-ns-nt-overcall-advance
            ;;
        *) echo "unknown mode $MODE (want pack|nine|bisect)" >&2; exit 2 ;;
        esac
    done
done

log "nt-natural-overcall $MODE done"
