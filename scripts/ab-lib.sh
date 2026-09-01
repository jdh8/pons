# ab-lib.sh — shared plumbing for the scripts/*-ab.sh A/B runners.  Not
# executable on its own; a runner sets R (and optionally the knobs below) then
# sources it, and is left with only its experiment body — the arms and diff
# pairs — to spell out.  Sourcing turns on `set -eu`, cds to the repo root,
# builds the harnesses, and defines log / arm / gatepair / diffpair / sddiff /
# seed_for.
#
# Honored if set before sourcing:
#   BOARDS       total boards per arm per vul; sets PER_SHARD = BOARDS/JOBS
#   PER_SHARD    boards per shard per arm per vul     (default 6400)
#   SHOW         worst boards ab-dump-diff prints      (default 5)
#   BUILD_EXTRA  extra `cargo build` --example flags   (e.g. --example ab-dump-sd)
#
# Sourcing exports SKIP_BUILD=1, so the build it does here is the only one the
# whole experiment gets; see the comment at that line.
#
# A runner that scores single-dummy sets BUILD_EXTRA='--example ab-dump-sd' and
# calls sddiff; the two split-by-opening runners override diffpair/sddiff with
# dir-based variants after sourcing (their shard dirs are split by strain).
set -eu
cd "$(dirname "$0")/.."

: "${R:?source ab-lib.sh with R set to the results dir}"
mkdir -p "$R"
SHA=$(git rev-parse --short HEAD)$(git diff --quiet HEAD || echo -dirty)
DIFF=target/release/examples/ab-dump-diff
SD=target/release/examples/ab-dump-sd
PROBE=target/release/examples/probe-divergence
SHOW=${SHOW:-5}
SHARDS=${JOBS:-$(nproc)}   # shard count bba-gen-parallel.sh creates; runners log it
# BOARDS = total boards per arm per vul, the knob that actually sets statistical
# power; when given, PER_SHARD is derived so JOBS is pure parallelism (rounded up
# to a multiple of 4 per shard for dealer balance).  PER_SHARD alone still works,
# but note total = PER_SHARD × JOBS, so changing JOBS then rescales the sample.
if [ -n "${BOARDS:-}" ]; then
    PER_SHARD=$(( (BOARDS / SHARDS + 3) / 4 * 4 ))
fi
PER_SHARD=${PER_SHARD:-6400}

# Pairing guard: every arm of one results dir must be generated with the same
# shard count — shard i seeds SEED_BASE+i, so resuming a dir with a different
# JOBS would diff arms drawn from different deal sets.
# Both halves: a resume that keeps JOBS but drops BOARDS falls back to 6400/shard
# (ab-lib.sh:38), and if BOTH arms of a vulnerability are still missing it
# regenerates them aligned -- every assert passes and the headline is quoted on a
# silently smaller sample.  Pin the count too, so that fails loudly instead.
for k in shards:$SHARDS per-shard:$PER_SHARD; do
    f="$R/${k%%:*}"; want="${k#*:}"
    if [ -s "$f" ]; then
        [ "$(cat "$f")" = "$want" ] || {
            echo "ab-lib: $R was generated with ${k%%:*}=$(cat "$f"), not $want; rerun with JOBS=$(cat "$R/shards") and BOARDS=<shards x per-shard as recorded in $R>, or use a fresh results dir" >&2
            exit 1
        }
    else
        echo "$want" >"$f"
    fi
done

# BUILD_EXTRA is a deliberately word-split flag list, not one argument.
# shellcheck disable=SC2086
cargo build --release --features serde --example bba-gen --example ab-dump-diff ${BUILD_EXTRA:-}

# One build per experiment, here.  bba-gen-parallel.sh otherwise rebuilds at the
# head of *every arm*, so a `src/` edit mid-run makes late arms measure different
# code from early ones with nothing in the log to say so.  With this exported it
# hard-fails on a missing binary instead.  Cost: a run started on a dirty or
# stale tree measures this build for all its arms -- which is the point.
export SKIP_BUILD=1

log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }

# arm NAME VUL [bba-gen flags...] — generate one arm unless already present
#
# The resume check COUNTS shards, and tests neither the directory nor shard-0.
# bba-gen-parallel mkdirs before it launches a worker, so an arm that died on
# startup (a stale flag, a bad card path) leaves an empty dir behind; and its
# fan-out ends in a bare `wait`, which reports zero even when a worker was
# OOM-killed, so an arm can be short by any shard but the 0th.  Skipping on `-d`
# resumes past an arm that was never generated; skipping on shard-0 alone wedges
# a short arm forever, and if both arms lost the SAME index (one deterministic
# crash, same deal stream) every downstream assert still passes.
arm() {
    name=$1; vul=$2; shift 2
    dir="$R/$name-$vul"
    [ "$(ls "$dir"/shard-*.json 2>/dev/null | wc -l | tr -d " ")" = "$SHARDS" ] \
        && { log "skip $dir (complete)"; return 0; }
    log "generate $dir (SEED_BASE=$SEED_BASE, flags: $*)"
    SEED_BASE=$SEED_BASE scripts/bba-gen-parallel.sh "$dir" "$PER_SHARD" -v "$vul" "$@" \
        >>"$R/log" 2>&1
}

# gatepair ON OFF VUL [ours|theirs] — the isolation gate before any headline
# (docs/measurement.md): it must read **0 foreign**, i.e. no divergent board was
# opened by the other side.  Needs BUILD_EXTRA='--example probe-divergence'.
#
# Do NOT shadow this in a runner.  Eleven did (all with the pre-fix `-s` guard
# below), which made the 2026-09-01 fix inert for them until they were deleted;
# ab-nt-natural-overcall.sh needed `theirs`, which is now the 4th argument.
gatepair() {
    on=$1; off=$2; vul=$3; side=${4:-ours}
    out="$R/gate.$on.vs.$off.$vul.txt"
    # Guard on the PASSED line, not on the file.  probe-divergence writes its
    # FAILED summary to the same stdout it is judged by (examples/probe-
    # divergence/main.rs:419 bails *after* the shell redirect has filled $out),
    # so an `-s` guard makes a failed gate sticky: the resume skips it and goes
    # on to print the very headline the runner pre-registers the gate against.
    grep -q 'isolation gate PASSED' "$out" 2>/dev/null && { log "skip $out (passed)"; return 0; }
    log "isolation gate $on vs $off ($vul)"
    "$PROBE" "$R/$on-$vul" "$R/$off-$vul" --gate-opener "$side" >"$out"
}

# diffpair ON OFF VUL — paired diff over the whole arm, plain + pd.  ab-dump-diff
# folds every shard of the arm dir into a single DDS fan-out — one solver owns
# all cores instead of one process per shard oversubscribing the box.
#
# One invocation, both brackets: a scorer only re-prices solved tables, so
# `--score both` pays for the shard parse and the DDS fan-out once.  Running it
# twice, once per scorer, paid for each twice.
#
# The skip guard wants BOTH files, so a run killed between the two writes redoes
# the cell instead of resuming past a half-written pair.
diffpair() {
    on=$1; off=$2; vul=$3
    plain="$R/diff.$on.vs.$off.$vul.plain.txt"
    pd="$R/diff.$on.vs.$off.$vul.pd.txt"
    [ -s "$plain" ] && [ -s "$pd" ] && { log "skip $plain + $pd (exist)"; return 0; }
    log "diff $on vs $off ($vul, plain+pd)"
    "$DIFF" "$R/$on-$vul" "$R/$off-$vul" --score both \
        --out-plain "$plain" --out-pd "$pd" --show "$SHOW" >>"$R/log" 2>&1
}

# sddiff ON OFF VUL [ab-dump-sd flags...] — sd-lead paired delta over the whole
# arm, 16 worlds, one solver; extra flags (e.g. --on-ns-negative-double-shape)
# disclose the ON arm's knobs to the blind leader.
sddiff() {
    on=$1; off=$2; vul=$3; shift 3
    out="$R/sd.$on.vs.$off.$vul.txt"
    # Guard on the result line, not on the file -- same reason as gatepair, and
    # worse here: `2>&1` below folds a panic (ab-dump-sd asserts the arms are
    # aligned) into the very file the guard reads, so an `-s` guard skips the
    # cell permanently and the sd column silently goes missing.
    grep -q '^Delta' "$out" 2>/dev/null && { log "skip $out (scored)"; return 0; }
    log "sd-diff $on vs $off ($vul, 16 worlds$*)"
    "$SD" "$R/$on-$vul" "$R/$off-$vul" -v "$vul" --sd-worlds 16 --show 0 "$@" >"$out" 2>&1
}

# seed_for [NAME] — a persistent SEED_BASE in $R/[NAME.]seed, fresh on first use.
# No NAME: one $R/seed for the whole run; NAME: one $R/NAME.seed per experiment.
seed_for() {
    f="$R/${1:+$1.}seed"
    if [ ! -s "$f" ]; then date +%s >"$f"; sleep 1; fi
    cat "$f"
}
