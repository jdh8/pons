#!/bin/sh
# ab-xx-runout-rails.sh — the redoubled-penalty-double runouts, two
# alternatives against the shipped net floor, vs BBA, we defend their 1NT
# (1NT-defense plan step 2, package B): the ladder's runouts railed
# (`InstinctProfile::xx_runout_rails`) and the DONT-style book runout
# (`DefenseKnobs::xx_dont_runout`).
#
#   JOBS=96 BOARDS=393216 BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       DDSS_THREADS=96 setsid nohup scripts/idle-run.sh \
#       scripts/ab-xx-runout-rails.sh ab-results/xx-runout-rails \
#       >ab-results/xx-runout-rails.log 2>&1 < /dev/null &
#
# Launch only after step 1's BEN A/B (ab-results/ben-nt-overcall-floor-2)
# finishes: sourcing ab-lib.sh rebuilds the shared release examples.
#
# Why.  Our natural penalty `X` of their 1NT has no authored continuation; the
# v6 net owns every seat after it.  The two instinct-ladder runouts measured and
# shipped default-on in June 2026 — the advancer's (`advancer_xx_runout`,
# 782a4aaf) and the doubler's (`InstinctKnobs::doubler_xx_runout`, c1545636) —
# live only in the deterministic ladder, and neither node is a `forced` rail,
# so under the net floor both are inert.  Forensics (natural census, 393,216
# accepted boards per cell): at `(1NT) X (XX) - -` the doubler sits in 1NTxx on
# 56 (none) / 65 (both) boards per cell, 17 / 22 of them holding a five-plus
# suit; the −7.0 / −8.3 IMPs per board on those sits is a NON-CAUSAL comparison
# against the other table's result, not the value of running.  At
# `(1NT) X (XX)` the net passes a weak advancer holding a five-plus suit on ~7
# boards per cell.  This build covers the doubler arm and that five-card
# advancer piece — about 40% of the boards the verdict priced.  The "+4.2 to
# +6.6 IMPs per changed board" counterfactual for running from `(XX)` on 0–4
# HCP belongs to FOUR-card runouts this build does not make, so it is not this
# build's expected gain.  The rail hands the WHOLE decision at both nodes to
# the ladder, not only the runout hands: values advancers (8+ HCP) and doublers
# without a five-card suit Pass, and so do weak 4-4 / 4-3-3-3 advancers (the
# ladder has no four-card escape; the net sometimes runs those to 2♣).
#
# The four-card piece is the `dont` arm (owner decisions 2026-10-20): a book
# package, direct seat only.  The advancer's pass is FORCING (a 5+ one-suiter
# in ♣/♦/♥ or any 4-3-3-3) and the doubler relays 2♣ over it, never passing;
# 2♣ = ♣ + a higher suit, 2♦ = ♦ + a major, 2♥ = both majors (all 4-4+,
# alerted), 2♠ = 5+♠; pass-or-correct over the two-suiters, an SOS redouble
# when they double one, and every later double of theirs is sat.  Design and
# sources: docs/defensive-overcalls.md § "The DONT-style runout after
# `(1NT) X (XX)`".  The two are alternatives, not complements: a book node
# shadows the floor, so with both knobs on the book answers both nodes and the
# rail never fires; the `dont` arm therefore runs with the rails OFF.
#
# Arms, one fresh SEED_BASE shared by all three, `--filter-1nt --wide-1nt`,
# all four cells:
#   off   = american as shipped (net floor at both nodes)             (control)
#   rails = `--ns-xx-runout-rails` (the ladder answers both nodes)
#   dont  = `--ns-xx-dont-runout`, rails off (the book answers both nodes and
#           every continuation it keys)
# SD reading.  The rails runouts are natural and unalerted, and the reading of
# `(1NT) X (XX) 2♠` / `(1NT) X (XX) - - 2♠` was checked knob-invariant from both
# our and the East–West seat, so rails − off takes no disclosure flags.  The
# dont arm's alerted two-suiters, relays, SOS redouble and forcing pass DO
# change the reading, so its sddiffs pass `--on-ns-xx-dont-runout` (the arm
# whose dir is the first argument is the one that plays it).  The trigger is on
# boards THEY open: gate `theirs`.
#
# Sizing.  BOARDS=393216 accepted boards/arm/vul (JOBS=96 → 4096 per shard).
# The XX nodes are rare: 255 boards per cell reach `(1NT) X (XX)` at 393,216
# accepted (natural census, 2026-10-19 forensics,
# /data/public/jdh8/pons/forensics-2026-10-19/natural-x-advance/out/out-{none,both}.txt;
# 255 in both measured cells), 56–65 of them going on to the doubler's node.
# The contract-divergent set is the subset of those where the ladder and the
# net disagree, so it is smaller still.
#
# PRE-REGISTERED 2026-10-19, ship rule amended 2026-10-20 (jdh8: the standard
# decision table of docs/measurement.md for every step-2 runner), and the
# third arm and rule 6 added 2026-10-20, all before any number exists.  Each
# CANDIDATE − off per cell (candidates: rails, dont; each judged on its own
# pair, rules 0–5), IMPs per accepted board, δ = 0.002 / density (density_of
# the off arm), each scorer's verdict its worst cell:
#   0. PRE-COUNT.  The runner logs each cell's contract-divergent board count
#      (ab-dump-diff's `fired`).  A cell under 50 reads INSUFFICIENT DATA, not a
#      verdict, whatever its CI says; its boards are still traced.  If EVERY
#      cell of a candidate is under 50, the run is undersized for that
#      candidate and its verdict lines are not
#      read: re-run at double the size, BOARDS=786432 (JOBS=96 → 8192 per
#      shard), with the SAME SEED_BASE, in a fresh dir (ab-lib's per-shard
#      guard refuses a resume at a new BOARDS):
#        mkdir ab-results/xx-runout-rails-2x
#        cp ab-results/xx-runout-rails/xx-runout-rails.seed ab-results/xx-runout-rails-2x/
#      then the launch line above with BOARDS=786432 and the -2x dir.
#   1. SHIP (default on) = plain DD non-inferior in EVERY cell (95% lower bound
#      above −δ) AND a PD win (lower bound above 0) — the table's `win | win`
#      and `non-inferior | win` rows.
#   2. Not a ship:
#      - a plain win with PD merely non-inferior (or worse) — the table's
#        doubling-artifact row;
#      - a PD win without plain non-inferiority: plain inconclusive is
#        insufficient data (extend or record the trade-off), plain loss is the
#        PD-synthetic-X artifact.
#      The knob stays off-by-default and the numbers are recorded.
#   3. PD is biased AGAINST running here, and not by a little: `ns_score_bid`
#      scores a making 1NTxx as undoubled 1NT (it erases the redouble of any
#      making contract) and scores our failing runout doubled whether or not
#      they doubled it, so PD prices the off arm's sit at its cheapest and a
#      candidate's escape at its dearest.  The bias is named in the report, per
#      node (advancer / doubler / later, split by the first divergent call
#      off probe-divergence's records); it does NOT relax rule 1.  A PD shortfall
#      blamed on it is a recorded trade-off for jdh8, never a ship.
#   4. A plain LOSS in any cell → trace that cell's worst boards first (the
#      per-node split below and `$R/div.<arm>.vs.off.$v.jsonl`) before any
#      verdict: the usual culprits are the weak 4-4 advancer the rail now sits
#      with, a doubler pulling a values advancer's pass, an unauthored
#      continuation after the runout (dont: their bid over our runout, which
#      is floored), and the dont arm's 4-3-3-3 landing in a 4-2 fit.  Nothing
#      ships on a loss.
#   5. SD-PD (sddiff) is descriptive; it inherits PD's bias.
#   6. BOTH qualify under rule 1 (confirmed by jdh8 2026-10-20, before
#      the verdict is read): ship the one with the larger pooled plain gain
#      (mean of the four cells' candidate − off plain IMPs/board).  The
#      tie-break reads the paired dont − rails plain diff (descriptive pair
#      below): if its pooled 95% CI straddles 0, the two are not separable
#      and the RAILS ship, as the simpler change (no new alerts, no new
#      readings, no book nodes).  dont − rails is never a ship verdict on its
#      own.
# Nothing is changed by this runner's verdict; the default is jdh8's call.
#
# Owed: a BEN guard runner (`ben-gen --ns-xx-runout-rails` /
# `--ns-xx-dont-runout`, same arms) — BBA is the exploit guard, not the north
# star; this runner is BBA-only.
#
# Resumable; SEED_BASE persists in $R/xx-runout-rails.seed.  Iron rule: do NOT
# rebuild binaries while this runs.
R=${1:?usage: ab-xx-runout-rails.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for xx-runout-rails)
log "=== xx-runout-rails SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul vuls=$VULS"

# node_split ON OFF VUL — per-node plain / PD sums off probe-divergence's
# priced records: the node is read off the OFF arm's auction just before the
# first divergent call (`XX` = the advancer, `XX - -` = the doubler, anything
# else is `other`: a leak for rails − off, a later node of the same lane for a
# pair with the dont arm, e.g. the doubler's answer to a two-suiter both arms
# bid).
node_split() {
    on=$1; off=$2; vul=$3
    jsonl="$R/div.$on.vs.$off.$vul.jsonl"
    out="$R/nodes.$on.vs.$off.$vul.txt"
    inputs=$(report_inputs "$PROBE" "$on" "$off" "$vul" --imps --jsonl)
    [ -s "$out" ] && fresh_report "$out.inputs" "$inputs" \
        && { log "skip $out (exists, inputs unchanged)"; return 0; }
    log "node split $on vs $off ($vul)"
    "$PROBE" "$R/$on-$vul" "$R/$off-$vul" -v "$vul" --imps --jsonl "$jsonl" >>"$R/log" 2>&1
    python3 -I - "$jsonl" >"$out" <<'EOF'
import json, sys
tally = {}
for line in open(sys.argv[1]):
    r = json.loads(line)
    before = r["auction_off"].split()[: r["first_diff"]]  # the OFF-side arm
    node = ("advancer" if before[-1:] == ["XX"]
            else "doubler" if before[-3:] == ["XX", "-", "-"] else "other")
    t = tally.setdefault(node, [0, 0, 0])
    t[0] += 1
    t[1] += r["imps_plain"] or 0
    t[2] += r["imps_pd"] or 0
for node, (n, plain, pd) in sorted(tally.items()):
    print(f"{node:9} {n:6} boards  plain {plain:+6} IMPs ({plain / n:+.2f}/bd)  "
          f"pd {pd:+6} IMPs ({pd / n:+.2f}/bd)  [pd biased against running]")
EOF
    printf '%s\n' "$inputs" >"$out.inputs"
}

# precount ON OFF VUL — log the pair's contract-divergent board count (header
# rule 0) and count the cell into that candidate's undersized tally.
precount() {
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.$1.vs.$2.$3.plain.txt")
    if [ "${fired:-0}" -lt 50 ]; then
        log "pre-count $1 vs $2 $3: $fired contract-divergent boards (< 50): INSUFFICIENT DATA for this cell"
        return 1
    fi
    log "pre-count $1 vs $2 $3: $fired contract-divergent boards"
}

FILTER='--filter-1nt --wide-1nt'
DONT_SD='--on-ns-xx-dont-runout'
cells=0
under_rails=0
under_dont=0
for v in $VULS; do
    # shellcheck disable=SC2086
    arm off "$v" $FILTER
    # shellcheck disable=SC2086
    arm rails "$v" $FILTER --ns-xx-runout-rails
    # shellcheck disable=SC2086
    arm dont "$v" $FILTER --ns-xx-dont-runout
    DENSITY=$(density_of "$R/off-$v")
    log "density $v: $DENSITY"
    cells=$((cells + 1))

    # Candidate 1: rails − off (no disclosure flags: the reading is unchanged).
    gatepair rails off "$v" theirs
    diffpair rails off "$v"
    precount rails off "$v" || under_rails=$((under_rails + 1))
    node_split rails off "$v"
    sddiff rails off "$v"

    # Candidate 2: dont − off (the dont arm's reading differs).
    gatepair dont off "$v" theirs
    diffpair dont off "$v"
    precount dont off "$v" || under_dont=$((under_dont + 1))
    node_split dont off "$v"
    sddiff dont off "$v" $DONT_SD

    # Descriptive: dont − rails (header rule 6's tie-break; never a verdict).
    gatepair dont rails "$v" theirs
    diffpair dont rails "$v"
    node_split dont rails "$v"
    sddiff dont rails "$v" $DONT_SD
done

for c in rails dont; do
    eval "under=\$under_$c"
    if [ "$cells" -gt 0 ] && [ "$under" -eq "$cells" ]; then
        log "pre-count $c: every cell under 50 divergent boards -- undersized; do not read its verdicts, re-run at BOARDS=786432 with this SEED_BASE (header rule 0)"
    fi
done
log "xx-runout-rails done"
