#!/bin/sh
# a7-run.sh — measurements for the A7 pass of the bidding-options audit
# (docs/bidding-options.md A7, "Slam & keycard"), under the new slam-boundary
# rule (docs/measurement.md): every DD-play scorer is *optimistic* for the arm
# bidding more slams, so each experiment is scored plain + pd + sd-lead + the
# **sd-declarer playout** (`ab-dump-sd --sd-declarer` / `ab-slam-entry --sd`),
# and a plain win that the playout reverses is a DD-optimism artifact.
#
#   JOBS=12 setsid nohup scripts/idle-run.sh scripts/a7-run.sh ab-results/a7 \
#       >ab-results/a7.log 2>&1 < /dev/null &
#
# Experiments:
#   1. slam-entry     — self-play `ab-slam-entry --sd`: re-arbitrate the shipped
#                       29 gate against the 33 baseline (shipped on +0.005 plain,
#                       thin enough for optimism to flip).
#   2. floor-rkcb     — bba diffpair, `--no-ns-floor-rkcb` off arm (stale-pop).
#   3. transfer-slam-try / texas-slam-drive — bba diffpairs for the two shipped
#                       1NT slam drives (default-on; off arms via --no-ns-*).
#   4. minor-keycard  — self-play `ab-minor-keycard --sd` at KC_COUNT (default
#                       10M, the original sample: it diverges ~1 in 50k, so a
#                       bba diffpair could never feed the fired set).  The new
#                       `set_minor_keycard` knob replaces the unreproducible
#                       worktree revert of 99da1b3 as the off arm.
#
# Slam knobs fire rarely (~0.1-0.3%), so the bba arms default to PER_SHARD=26667
# (× JOBS=12 ≈ 320k boards/arm/vul, the original transfer-slam-try sample).
# COUNT (default 1_000_000) sizes the self-play slam-entry arms.
#
# Resumable: existing outputs are skipped; per-experiment seeds persist in
# $R/<exp>.seed. Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: a7-run.sh RESULTS_DIR}
PER_SHARD=${PER_SHARD:-26667}
BUILD_EXTRA='--example ab-dump-sd --example ab-slam-entry --example ab-minor-keycard'
. "$(dirname "$0")/ab-lib.sh"

COUNT=${COUNT:-1000000}
KC_COUNT=${KC_COUNT:-10000000}

# sdddiff ON OFF VUL — sd-declarer playout paired delta over the divergent set
# (16 lead worlds × 16 line worlds; sequential per board, so divergent-sized).
sdddiff() {
    on=$1; off=$2; vul=$3
    out="$R/sdd.$on.vs.$off.$vul.txt"
    [ -s "$out" ] && { log "skip $out (exists)"; return 0; }
    log "sd-declarer diff $on vs $off ($vul)"
    "$SD" "$R/$on-$vul" "$R/$off-$vul" -v "$vul" --sd-declarer \
        --sd-worlds 16 --declarer-worlds 16 --show 0 >"$out" 2>&1
}

# sp_run EXP VUL CMD... — one self-play cell, written to *.partial first so an
# interrupted arm re-runs and a failing one never aborts the campaign.
sp_run() {
    out="$R/$1.$2.txt"; shift 2
    [ -s "$out" ] && { log "skip $out (exists)"; return 0; }
    log "run $out :: $*"
    if "$@" >"$out.partial" 2>&1; then
        mv "$out.partial" "$out"
    else
        log "FAILED (exit $?) — kept $out.partial"
    fi
}

# --- 1. slam-entry re-arbitration (self-play, plain + pd + sd rows) -----------
seed=$(seed_for slam-entry)
log "=== slam-entry 29-vs-33 seed=$seed sha=$SHA (self-play, --sd row)"
for v in none both; do
    sp_run slam-entry "$v" target/release/examples/ab-slam-entry \
        --count "$COUNT" --threshold 29 --vulnerability "$v" --seed "$seed" \
        --sd --sd-seed "$seed"
done

# --- 4. minor-keycard re-measure (self-play, rare divergence → KC_COUNT) ------
seed=$(seed_for minor-keycard)
log "=== minor-keycard seed=$seed sha=$SHA (self-play, --sd row, $KC_COUNT boards)"
for v in none both; do
    sp_run minor-keycard "$v" target/release/examples/ab-minor-keycard \
        --count "$KC_COUNT" --vulnerability "$v" --seed "$seed" \
        --sd --sd-seed "$seed"
done

# --- 2-4. bba diffpairs: plain + pd + sd-lead + sd-declarer -------------------
# bba_knob EXP OFF_FLAG — default-on knob: bare ON arm vs --no-ns-* OFF arm.
bba_knob() {
    exp=$1; off_flag=$2
    SEED_BASE=$(seed_for "$exp")
    log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA (bba diffpair, off: $off_flag)"
    for v in none both; do
        arm "$exp-on" "$v"
        arm "$exp-off" "$v" "$off_flag"
        diffpair "$exp-on" "$exp-off" "$v"
        sddiff   "$exp-on" "$exp-off" "$v"
        sdddiff  "$exp-on" "$exp-off" "$v"
    done
}

bba_knob floor-rkcb        --no-ns-floor-rkcb
bba_knob transfer-slam-try --no-ns-transfer-slam-try
bba_knob texas-slam-drive  --no-ns-texas-slam-drive

log "a7 campaign done"
