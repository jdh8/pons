#!/bin/sh
# a5-run.sh — measurements for the A5 pass of the bidding-options audit
# (docs/bidding-options.md A5, "Defending their 1NT & their overcalls"). Isolates
# the five A5 `unmeasured` worklist knobs.
#
# Four carry a bba-gen flag → ab-lib.sh arm/diffpair (a4 template). All are
# default-OFF, so the ON arm passes the flag and the OFF arm is bare — the arms
# are byte-identical until the knob fires, so read IMPs off the fired set. Three
# of the four are DD-blind or DD-negative for a real bridge reason (balancing /
# passed-seat obstruction / lead-directing transfer), so they add an sd-lead read
# (sddiff): ab-dump-sd reads the two natural ones correctly, but has no disclosure
# flag for the artificial minor-transfer, so THAT sd figure is a floor, not a
# verdict. The fifth knob, set_responsive_overcall, has no bba-gen flag → self-play
# ab-responsive (a3 template), scored perfect-defense vs the passing floor.
#
#   JOBS=12 setsid nohup scripts/idle-run.sh scripts/a5-run.sh ab-results/a5 \
#       >ab-results/a5.log 2>&1 < /dev/null &
#
# Shared-box cap: set JOBS (e.g. 12) to bound worker processes — bba-gen-parallel
# creates JOBS shards and ab-lib reads exactly JOBS shards. PER_SHARD (default
# 6400) sets boards/shard/arm/vul; RESP_COUNT (default 400000, `--filter`ed) sets
# the self-play board count. A smoke run uses tiny values.
#
# Resumable: an existing arm dir / diff / sd / responsive file is skipped; each
# bba-gen experiment's SEED_BASE persists in $R/<exp>.seed. Iron rule: do NOT
# rebuild binaries while this runs — ab-lib builds bba-gen, ab-dump-diff,
# ab-dump-sd and ab-responsive ONCE up front.
R=${1:?usage: a5-run.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example ab-responsive'
. "$(dirname "$0")/ab-lib.sh"

RESP_COUNT=${RESP_COUNT:-400000}

# bba_knob EXP SD ON COMMON — one default-off knob across both vuls. ON arm gets
# ON + COMMON flags, OFF arm gets COMMON only, so the two arms deal the SAME
# board set (COMMON is a deterministic deal filter) and stay seed-aligned for the
# paired diff — only the knob (ON) may differ. SD="sd" adds the sd-lead read.
# ON and COMMON are word-split flag lists (may be empty).
#
# Locals are k-prefixed / `v`: ab-lib.sh's arm/diffpair/sddiff use unscoped
# globals `on`/`off`/`vul`, so a plain `on`/`vul` here would be clobbered mid-loop.
bba_knob() {
    kexp=$1; ksd=$2; kon=$3; kcommon=$4
    SEED_BASE=$(seed_for "$kexp")
    log "=== $kexp SEED_BASE=$SEED_BASE sha=$SHA sd=$ksd on:[$kon] common:[$kcommon]"
    for v in none both; do
        # shellcheck disable=SC2086  # deliberate word-split of the flag lists
        arm "$kexp-on"  "$v" $kon $kcommon
        # shellcheck disable=SC2086
        arm "$kexp-off" "$v"      $kcommon
        diffpair "$kexp-on" "$kexp-off" "$v"           # ship gate: plain + PD
        if [ "$ksd" = sd ]; then
            sddiff "$kexp-on" "$kexp-off" "$v"          # sd-lead: obstruction/lead read
        fi
    done
}

# Natural discipline, DD-visible → plain+PD only.
bba_knob nt-overcall-no-major    -  --ns-nt-overcall-no-major       ""
# Semi-natural balancing / natural passed-seat overcall → DD-blind/negative, add sd.
bba_knob notrump-balancing       sd --ns-balancing                  ""
bba_knob passed-hand-overcall    sd --ns-passed-hand-overcall       ""
# Artificial lead-directing defense of their 1NT (defended boards only); the
# defended-board filter goes on BOTH arms; sd is a floor (ab-dump-sd cannot
# disclose the transfer to the blind leader).
bba_knob minor-transfer-defense  sd --ns-minor-transfer-defense     "--isolate-defense --filter-1nt"

# --- self-play knob (no bba-gen flag): ab-responsive vs the passing floor. ------
# Perfect-defense scored; single invocation prints an IMPs/board headline. Written
# to *.partial first, renamed on success, so an interrupted arm re-runs and a
# failing arm never aborts the campaign (a3 idiom).
RESP=target/release/examples/ab-responsive
resp_run() {
    out="$R/$1"; shift
    [ -s "$out" ] && { log "skip $out (exists)"; return 0; }
    log "run $out :: $*"
    if "$@" >"$out.partial" 2>&1; then
        mv "$out.partial" "$out"
    else
        log "FAILED (exit $?) — kept $out.partial"
    fi
}

for vul in none both; do
    resp_run "responsive-overcall.$vul.txt" \
        "$RESP" --conv overcall --filter -v "$vul" --count "$RESP_COUNT"
done

log "a5 campaign done"
