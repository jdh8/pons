#!/bin/sh
# a6-run.sh — measurements for the A6 pass of the bidding-options audit
# (docs/bidding-options.md A6, "Floor & inference engine toggles"). Isolates the
# `unmeasured`/`unmeasured (net)` engine toggles, plus a fresh re-measure of the
# two `stale-PD` ones (settle-floor, alert-reading) so the whole section lands on
# the current plain-DD harness in one pass.
#
# Most of these are *constructive* reading toggles — our own partnership reading
# its own calls (auction interpretation, alerts, the invite raise, the strength
# gauge). Their value is DD-visible and self-play measures it: each toggle has a
# dedicated `examples/ab-*` seat-swap / two-pass duplicate that bids every board
# twice (feature off vs on), solves the divergent set once, and prints BOTH
# brackets with CIs (plain DD + perfect defense) via `common::report_brackets`.
# The lone contested one, set_rubens_transfer_reading (unblinds the *overcaller*),
# has no self-play harness → bba-gen diffpair vs EPBot at the end.
#
#   JOBS=12 setsid nohup scripts/idle-run.sh scripts/a6-run.sh ab-results/a6 \
#       >ab-results/a6.log 2>&1 < /dev/null &
#
# COUNT (default 1_000_000) sets boards/example/vul for the self-play arms; the
# report_brackets CI is the verdict (these toggles diverge on 0.5-18% of boards,
# so 1M yields a real fired set for all but the rarest). PER_SHARD (default 6400)
# × JOBS shards × 2 arms sizes the bba rubens diffpair. A smoke run uses tiny
# COUNT / PER_SHARD.
#
# Resumable: an existing output file / arm dir / diff is skipped; each
# experiment's seed persists in $R/<exp>.seed. Iron rule: do NOT rebuild binaries
# while this runs — ab-lib builds every harness ONCE up front (BUILD_EXTRA).
R=${1:?usage: a6-run.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-inference-floor --example ab-nt-invite
    --example ab-fuzzy-strength --example ab-fifths-companion
    --example ab-alert-reading --example ab-settle-floor'
. "$(dirname "$0")/ab-lib.sh"

COUNT=${COUNT:-1000000}

# --- self-play toggles: dual-bracket duplicate vs the feature-off floor. --------
# Single invocation prints the report_brackets bracket; written to *.partial
# first, renamed on success, so an interrupted arm re-runs and a failing arm
# never aborts the campaign (a3/a5 idiom).
sp_run() {
    out="$R/$1"; shift
    [ -s "$out" ] && { log "skip $out (exists)"; return 0; }
    log "run $out :: $*"
    if "$@" >"$out.partial" 2>&1; then
        mv "$out.partial" "$out"
    else
        log "FAILED (exit $?) — kept $out.partial"
    fi
}

# sp_toggle EXP BIN [extra binary flags...] — one self-play toggle across both
# vuls at its own persistent seed (shared by both vuls of that experiment).
sp_toggle() {
    exp=$1; bin=$2; shift 2
    seed=$(seed_for "$exp")
    log "=== $exp seed=$seed sha=$SHA extra:[$*]"
    for v in none both; do
        # shellcheck disable=SC2086  # deliberate word-split of the extra flags
        sp_run "$exp.$v.txt" \
            "target/release/examples/$bin" --count "$COUNT" -v "$v" --seed "$seed" $*
    done
}

sp_toggle inference-aware  ab-inference-floor
sp_toggle nt-invite        ab-nt-invite
sp_toggle fifths-companion ab-fifths-companion
sp_toggle alert-reading    ab-alert-reading
sp_toggle settle-floor     ab-settle-floor
# fuzzy strength: one binary, three policies (points / fifths / both = the
# umbrella set_fuzzy_strength) — maps to the three A6 fuzzy cells.
sp_toggle fuzzy-both       ab-fuzzy-strength --policy both
sp_toggle fuzzy-points     ab-fuzzy-strength --policy points
sp_toggle fuzzy-fifths     ab-fuzzy-strength --policy fifths

# --- contested toggle (no self-play harness): bba-gen diffpair vs EPBot. --------
# set_rubens_transfer_reading is DEFAULT-ON, so the ON arm is bare and the OFF arm
# passes --no-ns-rubens-reading; both arms deal the same boards (same SEED_BASE),
# only the reading differs. plain + pd off the divergent (fired) set.
if [ "${A6_BBA:-1}" = 1 ]; then
    SEED_BASE=$(seed_for rubens-reading)
    log "=== rubens-reading SEED_BASE=$SEED_BASE sha=$SHA (bba diffpair, default-on knob)"
    for v in none both; do
        arm rubens-on  "$v"                          # bare = reading default-on
        arm rubens-off "$v" --no-ns-rubens-reading   # reading suppressed
        diffpair rubens-on rubens-off "$v"           # on - off, plain + pd
    done
fi

log "a6 campaign done"
