#!/bin/sh
# ab-xx-dont-major-first.sh — the follow-up to ab-xx-runout-rails.sh (its
# verdict: rails and DONT both stay off, 2026-10-20): the DONT runout with a
# five-card major ahead of the minor two-suiter, and the rails' doubler arm
# alone, vs BBA, we defend their 1NT (1NT-defense plan, step 2's tail).
#
#   JOBS=64 BOARDS=393216 BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       DDSS_THREADS=64 setsid nohup scripts/idle-run.sh \
#       scripts/ab-xx-dont-major-first.sh ab-results/xx-dont-major-first \
#       >ab-results/xx-dont-major-first.log 2>&1 < /dev/null &
#
# Why.  ab-results/xx-runout-rails-2 read `dont − off` as a plain win (+0.0015
# to +0.0019 per board) and a PD loss, almost all of the PD loss in the
# boards PD scores with bias (rule 3 there).  On the unbiased boards DONT was
# plain −92 to −169 per cell, and the loss sat in the advancer's two-suiters
# with a longer major: split by the advancer's shape, 5+M-4m via 2♣/2♦ was
# 94 boards, plain −203, PD −301 (5M-5m: 8 boards, plain −82, PD 0; 4-4
# two-suiters plain +1184, PD −601).  `rails − off` lost plain in ew: the
# advancer arm sits weak 4-4s in a making 1NTxx, while the doubler arm gained
# plain +87 to +151 per cell.
#
# Arms, one fresh SEED_BASE shared by all five, `--filter-1nt --wide-1nt`,
# all four cells:
#   off     = american as shipped (net floor at both XX nodes)       (control)
#   dont    = `--ns-xx-dont-runout` (the package as first measured; a paired
#             baseline for the two redesigns, never a ship candidate here)
#   longer  = dont + `--ns-xx-dont-major-first longer` (5+M strictly longer
#             than its 4+ minor: 5♠ bids 2♠, 5♥ passes and bids 2♥ over the
#             relay)
#   tied    = dont + `--ns-xx-dont-major-first at-least-as-long` (5-5 too)
#   doubler = `--ns-xx-runout-rails --no-ns-xx-runout` (the rail's advancer
#             arm off with the ladder's advancer runout it reads; the doubler
#             rail alone.  Pinned by instinct/tests.rs
#             `xx_runout_rails_are_auction_determined_and_follow_their_runouts`
#             and the logit identity in
#             `xx_runout_rails_hand_the_whole_node_to_the_ladder`.  Caveat: it
#             also turns off the ladder's advancer runout wherever else the
#             ladder answers that node, which under the net floor is nowhere.)
# SD reading: the DONT arms' alerts, relays and forcing pass change the
# reading, so their sddiffs pass the arm's disclosure flags; the doubler rail
# is natural (reading checked knob-invariant in ab-xx-runout-rails.sh), no
# flags.  The trigger is on boards THEY open: gate `theirs`.
#
# Sizing.  BOARDS=393216 accepted boards/arm/vul, as xx-runout-rails-2
# (108–176 contract-divergent boards per cell for dont − off).  JOBS=64 →
# 6144 per shard, leaving room for the BEN fleet beside it.
#
# PRE-REGISTERED 2026-10-20, before any number exists.  Candidates: longer,
# tied, doubler, each judged on its own CANDIDATE − off pair per cell, IMPs
# per accepted board, δ = 0.002 / density (density_of the off arm), each
# scorer's verdict its worst cell.  Rules 0–5 of ab-xx-runout-rails.sh apply
# verbatim (pre-count under 50 = insufficient data; SHIP = plain
# non-inferior in every cell AND a PD win; a plain win with PD short is no
# ship; PD is biased against running, named per bucket below, never relaxes
# rule 1; a plain loss is traced first; SD-PD descriptive), plus:
#   6. More than one candidate qualifies under rule 1: they are alternatives
#      (a book node shadows the rail), so ship the one with the larger pooled
#      plain gain (mean of the four cells' candidate − off).  Tie-breaks read
#      the paired diff between the two (descriptive pairs below): if its
#      pooled 95% CI straddles 0, ship the simpler — doubler over either DONT
#      arm (no alerts, no readings, no book nodes), longer over tied (fewer
#      hands moved).
#   7. No candidate qualifies: the trade-off goes to jdh8 with this run's
#      bias split.  The redesign is judged by the paired `longer − dont` /
#      `tied − dont` diffs: if the better of them is a plain win (pooled 95%
#      lower bound above 0), that mode replaces Off as the package's default
#      shape (the package itself stays off), and the trade-off recorded for
#      DONT is re-stated against that arm.
# Nothing is changed by this runner's verdict; the default is jdh8's call.
#
# Resumable; SEED_BASE persists in $R/xx-dont-major-first.seed.  Iron rule:
# do NOT rebuild binaries while this runs.
R=${1:?usage: ab-xx-dont-major-first.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for xx-dont-major-first)
log "=== xx-dont-major-first SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul vuls=$VULS"

# bias_split ON OFF VUL — the rule-3 split off probe-divergence's priced
# records: `sits` = the OFF arm plays a making 1NTxx (PD erases the
# redouble), `fails` = the ON arm's runout fails undoubled (PD doubles it),
# `unbiased` = the rest.  Also the advancer's node, read off the OFF arm's
# auction before the first divergent call, as in ab-xx-runout-rails.sh.
bias_split() {
    on=$1; off=$2; vul=$3
    jsonl="$R/div.$on.vs.$off.$vul.jsonl"
    out="$R/bias.$on.vs.$off.$vul.txt"
    inputs=$(report_inputs "$PROBE" "$on" "$off" "$vul" --imps --jsonl)
    [ -s "$out" ] && fresh_report "$out.inputs" "$inputs" \
        && { log "skip $out (exists, inputs unchanged)"; return 0; }
    log "bias split $on vs $off ($vul)"
    "$PROBE" "$R/$on-$vul" "$R/$off-$vul" -v "$vul" --imps --jsonl "$jsonl" >>"$R/log" 2>&1
    python3 -I - "$jsonl" >"$out" <<'EOF'
import json, sys
tally = {}
def makes(r, side):
    return r["tricks_" + side] >= r["level_" + side] + 6
for line in open(sys.argv[1]):
    r = json.loads(line)
    before = r["auction_off"].split()[: r["first_diff"]]
    node = ("advancer" if before[-1:] == ["XX"]
            else "doubler" if before[-3:] == ["XX", "-", "-"] else "other")
    if r["contract_off"].startswith("1NTxx") and r["declarer_off"] == "EW" and makes(r, "off"):
        bucket = "sits"
    elif r["declarer_on"] == "NS" and not r["doubled_on"] and not makes(r, "on"):
        bucket = "fails"
    else:
        bucket = "unbiased"
    for key in (bucket, node + "/" + bucket):
        t = tally.setdefault(key, [0, 0, 0])
        t[0] += 1
        t[1] += r["imps_plain"] or 0
        t[2] += r["imps_pd"] or 0
for key, (n, plain, pd) in sorted(tally.items()):
    print(f"{key:18} {n:6} boards  plain {plain:+6} IMPs ({plain / n:+.2f}/bd)  "
          f"pd {pd:+6} IMPs ({pd / n:+.2f}/bd)")
EOF
    printf '%s\n' "$inputs" >"$out.inputs"
}

# precount ON OFF VUL — header rule 0, as ab-xx-runout-rails.sh.
precount() {
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.$1.vs.$2.$3.plain.txt")
    if [ "${fired:-0}" -lt 50 ]; then
        log "pre-count $1 vs $2 $3: $fired contract-divergent boards (< 50): INSUFFICIENT DATA for this cell"
        return 1
    fi
    log "pre-count $1 vs $2 $3: $fired contract-divergent boards"
}

# The package shipped on with `longer` (2026-10-21), so the off and doubler
# arms now pass `--no-ns-xx-dont-runout` and the dont arm `off` explicitly.
FILTER='--filter-1nt --wide-1nt'
OFF='--no-ns-xx-dont-runout'
OFF_SD='--off-no-ns-xx-dont-runout'
for v in $VULS; do
    # shellcheck disable=SC2086
    arm off "$v" $FILTER $OFF
    # shellcheck disable=SC2086
    arm dont "$v" $FILTER --ns-xx-dont-major-first off
    # shellcheck disable=SC2086
    arm longer "$v" $FILTER --ns-xx-dont-major-first longer
    # shellcheck disable=SC2086
    arm tied "$v" $FILTER --ns-xx-dont-major-first at-least-as-long
    # shellcheck disable=SC2086
    arm doubler "$v" $FILTER $OFF --ns-xx-runout-rails --no-ns-xx-runout
    DENSITY=$(density_of "$R/off-$v")
    log "density $v: $DENSITY"

    # Candidates − off (the verdicts).
    for c in longer tied doubler dont; do
        case $c in
            longer) sd="$OFF_SD --on-ns-xx-dont-major-first longer" ;;
            tied) sd="$OFF_SD --on-ns-xx-dont-major-first at-least-as-long" ;;
            dont) sd="$OFF_SD --on-ns-xx-dont-major-first off" ;;
            *) sd="$OFF_SD --on-no-ns-xx-dont-runout" ;;
        esac
        gatepair "$c" off "$v" theirs
        diffpair "$c" off "$v"
        precount "$c" off "$v" || true
        bias_split "$c" off "$v"
        # shellcheck disable=SC2086
        sddiff "$c" off "$v" $sd
    done

    # Descriptive pairs (rules 6 and 7).  The OFF side of each also plays a
    # DONT mode, so these sddiffs need --off-* disclosure too; skipped (SD-PD
    # is descriptive and inherits PD's bias).
    for pair in "longer dont" "tied dont" "tied longer" "doubler longer" "doubler tied"; do
        # shellcheck disable=SC2086
        set -- $pair
        gatepair "$1" "$2" "$v" theirs
        diffpair "$1" "$2" "$v"
        bias_split "$1" "$2" "$v"
    done
done
log "xx-dont-major-first done"
