#!/bin/sh
# a3-run.sh — measurements for the A3 pass of the bidding-options audit
# (docs/bidding-options.md A3, "Our 1NT: competition, runouts & escapes").
#
# A3's unmeasured knobs have NO bba-gen flag, so they are measured with the
# self-play example binaries directly (NOT the bba-gen arm/diffpair mechanism in
# ab-lib.sh). Each example invocation runs one complete duplicate A/B and prints
# a headline IMPs/board; we capture stdout+stderr to a file under $R. Arms are
# ordered cheapest-first so the Lebensohl headline lands before the expensive
# escape battery. Resumable: a completed arm (non-empty output file) is skipped;
# a failed/interrupted arm leaves only an *.partial and re-runs. The shared
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
#
#   setsid nohup scripts/idle-run.sh scripts/a3-run.sh ab-results/a3 \
#       >ab-results/a3.log 2>&1 < /dev/null &
#
# Counts are env-overridable (a smoke run uses tiny ones):
#   LEB_COUNT   Lebensohl boards/arm            (default 400000)
#   RUN_COUNT   runout boards/arm (kept, filtered) (default 1000000)
#   ESC_COUNT   escape-* boards/arm (rare fire)  (default 3000000)
set -eu
cd "$(dirname "$0")/.."
R=${1:?usage: a3-run.sh RESULTS_DIR}
mkdir -p "$R"
SHA=$(git rev-parse --short HEAD)

# One shared SEED_BASE (two seeds S1,S2) — fresh on first use, persistent on resume.
SEEDF="$R/seed"
[ -s "$SEEDF" ] || date +%s >"$SEEDF"
S1=$(cat "$SEEDF"); S2=$((S1 + 1))

LEB=target/release/examples/ab-lebensohl
RUN=target/release/examples/ab-one-nt-runout

LEB_COUNT=${LEB_COUNT:-400000}
RUN_COUNT=${RUN_COUNT:-1000000}
ESC_COUNT=${ESC_COUNT:-5000000}

# Build both harnesses ONCE, up front. Iron rule: never rebuild a binary while an
# A/B is in flight, so nothing below (and no resume) may edit crate source.
cargo build --release --example ab-lebensohl --example ab-one-nt-runout

log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }

# run NAME CMD... — write $R/NAME unless it already exists non-empty. Output goes
# to *.partial first and is renamed on success, so an interrupted arm re-runs and
# one failing arm never aborts the campaign.
run() {
    out="$R/$1"; shift
    [ -s "$out" ] && { log "skip $1x (have $out)"; return 0; }
    log "run $out :: $*"
    if "$@" >"$out.partial" 2>&1; then
        mv "$out.partial" "$out"
    else
        log "FAILED (exit $?) — kept $out.partial"
    fi
}

log "=== A3 run start sha=$SHA seeds=$S1,$S2 counts leb=$LEB_COUNT run=$RUN_COUNT esc=$ESC_COUNT"

# 1) Lebensohl: Transfer (NS) vs Plain (EW). Plain-DD only (harness limitation).
#    Fires on every 1NT-overcalled board — cheapest, so first.
for v in none both; do
    for s in $S1 $S2; do
        run "leb.transfer-vs-plain.$v.seed$s.txt" \
            "$LEB" --count "$LEB_COUNT" -v "$v" --ns transfer --ew plain --seed "$s"
    done
done

# 2) Runout vs the passing floor (set_one_nt_runout), dual-scored, both vuls,
#    two seeds. `--no-universal` restricts to responder's direct seat; the delta
#    (full - direct) prices set_one_nt_runout_universal.
for v in none both; do
    for sc in plain pd; do
        for s in $S1 $S2; do
            run "runout.full.$v.$sc.seed$s.txt" \
                "$RUN" --compare runout --filter-1nt --count "$RUN_COUNT" -v "$v" --score "$sc" --seed "$s"
            run "runout.direct.$v.$sc.seed$s.txt" \
                "$RUN" --compare runout --no-universal --filter-1nt --count "$RUN_COUNT" -v "$v" --score "$sc" --seed "$s"
        done
    done
done

# 3) Escape penalty-X battery (set_penalize_escape_stack / _values) — fires
#    rarely (they run, we double), so a bigger count and a single seed. Last.
for v in none both; do
    for sc in plain pd; do
        run "escape-stack.$v.$sc.seed$S1.txt" \
            "$RUN" --compare escape-stack --filter-1nt --count "$ESC_COUNT" -v "$v" --score "$sc" --seed "$S1"
        run "escape-values.$v.$sc.seed$S1.txt" \
            "$RUN" --compare escape-values --filter-1nt --count "$ESC_COUNT" -v "$v" --score "$sc" --seed "$S1"
    done
done

log "=== A3 run done sha=$SHA"
