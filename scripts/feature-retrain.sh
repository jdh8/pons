#!/bin/bash
# feature-retrain.sh — retrain the K = 8 floor after a feature bump, with the
# M32 labels transplanted (docs/ai-bidder/features-v8.md §4, as the A4 lap-1
# driver ran it): re-walk the fleet's 188 chunk windows through the new
# extractor, splice the M32 cut's labels onto them, train eight draws of the
# shipped recipe on dl02's GPUs, fold the constant inputs, and stop with the
# NETS line the A/B runner takes.  Every stage verifies before the next; a
# failure stops the chain with its reason in the log, and the state file names
# the stage reached (a relaunch resumes from it).
#
#   NAME=v9 FEATURE_VERSION=9 setsid nohup scripts/feature-retrain.sh \
#       >/data/public/jdh8/pons/target/v9-pipeline.out 2>&1 </dev/null &
#
# Environment:
#   NAME             run name; chunks at $ROOT/corpus-$NAME-chunks, the spliced
#                    corpus at $ROOT/corpus-$NAME, nets at ab-results/$NAME/nets,
#                    dl02 scratch at /mnt/ssd-data/jdh8/pons-$NAME     (required)
#   FEATURE_VERSION  the extractor the re-walk dumps through               (required)
#   WALK_EXTRA       extra dump-teacher flags for the walk, e.g. the probed
#                    reading: "--ns-probe 100000 --ns-probe-vacuous"      (default none)
#   WORKERS          re-walk workers (STRIDE); each owns every WORKERS-th chunk (default 20)
#   EXPECT_RELABELLED  the labels the splice must transplant in total, ±TOLERANCE_PCT:
#                    the count the 2026-09-26 and lap-1 splices reached (default 424070, 2)
#   SEEDS            trainer init seeds, one net each (default "9 10 11 12 13 14 15 16")
#   TRAIN_PAR        trainers at once on dl02; a v9 corpus (416 wide) peaks at ≈ 2×13 GB
#                    per process while its stems concatenate, and the box has ≈ 39 GB
#                    spare — keep 1 above 200 columns                      (default 1)
#   DUMP_TEACHER     the walk's binary, default the release dump-teacher under
#                    ~/.local/opt/glibc-2.36/run (EPBot's glibc)
#   ROOT, FLEET, CUT, BANK   data roots (defaults below)
set -uo pipefail
cd "$(dirname "$0")/.."
NAME=${NAME:?set NAME}
FV=${FEATURE_VERSION:?set FEATURE_VERSION}
WALK_EXTRA=${WALK_EXTRA:-}
WORKERS=${WORKERS:-20}
EXPECT_RELABELLED=${EXPECT_RELABELLED:-424070}
TOLERANCE_PCT=${TOLERANCE_PCT:-2}
SEEDS=${SEEDS:-"9 10 11 12 13 14 15 16"}
TRAIN_PAR=${TRAIN_PAR:-1}
ROOT=${ROOT:-/data/public/jdh8/pons/target}
FLEET=${FLEET:-$ROOT/fleet-m32}
CUT=${CUT:-$ROOT/corpus-relabel-m32}
BANK=${BANK:-/nfs2/jdh8/pons/22.pdd}
# EPBot needs glibc ≥ 2.34; this box is RHEL 8 (CLAUDE.local.md), so the walk
# runs dump-teacher under the unpacked loader.  Two words, as relabel-worker
# expands $BIN unquoted.
DUMP_TEACHER=${DUMP_TEACHER:-"$HOME/.local/opt/glibc-2.36/run target/release/examples/dump-teacher"}
export DUMP_TEACHER
CH=$ROOT/corpus-$NAME-chunks
OUT=$ROOT/corpus-$NAME
STATE=$ROOT/$NAME-pipeline.state
LOG=$ROOT/$NAME-pipeline.log
DL=dl02
B=/mnt/ssd-data/jdh8/pons-$NAME
NETS_DIR=ab-results/$NAME/nets
SHARDS="axis-0002 axis-0004 axis-0020 axis-0800 axis-1000 axis-2000 axis-4000 axis-8000 enriched-0 enriched-1 enriched-2 enriched-3 uniform-0 uniform-1 uniform-2 uniform-3 uniform-4 uniform-5 uniform-6 uniform-7"

log() { echo "$(date -Is) $*" >>"$LOG"; }
stage() { echo "$1" >"$STATE"; log "stage $1"; }
die() { log "FAILED: $*"; echo "failed: $*" >"$STATE"; exit 1; }
# Resume: skip every stage before the one recorded (a failed state restarts its stage).
reached() { local s; s=$(cat "$STATE" 2>/dev/null || true); s=${s#failed: }; [ -n "$s" ] && [ "$s" != "$1" ] && after "$s" "$1"; }
after() { local order="walk splice upload train fold nets-ready" a b; a=${order%%$1*}; b=${order%%$2*}; [ ${#a} -gt ${#b} ]; }

log "=== $NAME pipeline start: feature v$FV, walk extra '$WALK_EXTRA', sha=$(git rev-parse --short HEAD)"
[ -x target/release/examples/dump-teacher ] || die "no release dump-teacher (cargo build --release --example dump-teacher)"

if ! reached walk; then
    stage walk
    mkdir -p "$CH"
    for k in $(seq 0 $((WORKERS - 1))); do
        OUT=$CH STRIDE=$WORKERS OFFSETS=$k LAYOUTS=0 \
            DUMP_COMMON="--deals $BANK --teacher bba --configured --feature-version $FV $WALK_EXTRA" \
            setsid nohup scripts/idle-run.sh scripts/relabel-worker.sh >"$CH/worker-$k.log" 2>&1 </dev/null &
    done
    sleep 60
    while :; do
        n=$(find "$CH" -name 'chunk-*.json' | wc -l)
        p=$(pgrep -fc 'scripts/relabel-worker.sh' || true)
        [ "$n" -ge 188 ] && [ "$p" -eq 0 ] && break
        [ "$p" -eq 0 ] && die "walk stopped at $n/188 chunks with no worker alive (see $CH/worker-*.log)"
        sleep 120
    done
    log "walk complete: $n chunks"
    v=$(python3 -c "import json,sys; m=json.load(open(sys.argv[1])); print(m['feature_version'], m['features_len'])" "$CH/axis-0004/chunk-0.json")
    [ "${v%% *}" = "$FV" ] || die "chunks are feature v${v%% *}, not v$FV"
    log "chunk layout: feature v$v"
fi

if ! reached splice; then
    stage splice
    mkdir -p "$OUT"
    total=0
    for s in $SHARDS; do
        [ "$(find "$CH/$s" -name 'chunk-*.json' | wc -l)" = "$(find "$FLEET/$s" -name 'chunk-*.json' | wc -l)" ] || die "$s: chunk count differs from the fleet"
        # Audit in the default mode (exit 1 on a tag mismatch); transplant under
        # --any-identity, the 2026-09-26 and lap-1 choice the expected count assumes.
        python3 scripts/splice-labels.py --validate "$CH/$s" "$FLEET/$s" "$CUT/$s" >"$OUT/$s.validate" 2>&1 || die "$s: validate exit $? (see $OUT/$s.validate)"
        python3 scripts/splice-labels.py --any-identity "$CH/$s" "$FLEET/$s" "$CUT/$s" "$OUT/$s" >"$OUT/$s.splice" 2>&1 || die "$s: splice exit $?"
        [ -s "$OUT/$s.f32" ] && [ -s "$OUT/$s.json" ] && [ -s "$OUT/$s.tags" ] || die "$s: stem incomplete"
        line=$(tail -1 "$OUT/$s.splice")
        log "spliced $s: $line"
        n=$(sed -n 's/.*, \([0-9]*\) labels transplanted$/\1/p' <<<"$line")
        total=$((total + ${n:-0}))
    done
    # The transplanted total is the check that the key's offsets are right for
    # this layout: a wrong vulnerability offset matches nothing and every row
    # silently keeps the teacher's one-hot (shrinkage-plan.md, review D2).
    lo=$((EXPECT_RELABELLED * (100 - TOLERANCE_PCT) / 100)); hi=$((EXPECT_RELABELLED * (100 + TOLERANCE_PCT) / 100))
    log "labels transplanted in total: $total (expected $EXPECT_RELABELLED ± $TOLERANCE_PCT%)"
    [ "$total" -ge "$lo" ] && [ "$total" -le "$hi" ] || die "transplanted $total labels, outside [$lo, $hi]"
    ln -sfn "$OUT" "target/corpus-$NAME"
    (cd "$OUT" && sha256sum *.f32 *.json *.tags >SHA256SUMS)
fi

if ! reached upload; then
    stage upload
    ssh -o BatchMode=yes $DL "mkdir -p $B/corpus $B/nets" || die "ssh $DL"
    scp -q "$OUT"/*.f32 "$OUT"/*.json "$OUT"/*.tags "$OUT"/SHA256SUMS $DL:$B/corpus/ || die "scp"
    ssh -o BatchMode=yes $DL "cd $B/corpus && sha256sum -c --quiet SHA256SUMS" || die "upload checksum"
fi

if ! reached train; then
    stage train
    # The trainer must know this feature version: sync its source and rebuild
    # (CUDA 12.8 on the path; the system nvcc rejects the 4090).
    rsync -a --delete trainer/src/ $DL:src/pons/trainer/src/ || die "rsync trainer source"
    ssh -o BatchMode=yes $DL "export PATH=/usr/local/cuda-12.8/bin:\$PATH LD_LIBRARY_PATH=/usr/local/cuda-12.8/lib64:\${LD_LIBRARY_PATH:-}; cd src/pons/trainer && cargo build --release --features cuda 2>&1 | tail -2" >>"$LOG" 2>&1 || die "trainer build"
    {
        echo '#!/bin/bash'
        echo "# $NAME retrain: eight draws of the shipped v6 recipe (Plan B README), $TRAIN_PAR at a time."
        echo 'set -euo pipefail'
        echo 'export PATH=/usr/local/cuda-12.8/bin:$PATH LD_LIBRARY_PATH=/usr/local/cuda-12.8/lib64:${LD_LIBRARY_PATH:-}'
        echo 'cd ~/src/pons/trainer'
        echo "B=$B; D=\$B/corpus; N=\$B/nets; mkdir -p \$N"
        echo 'STEMS=$(ls $D/*.f32 | sed "s/\.f32$//" | sed "s/^/--data /" | tr "\n" " ")'
        echo 'run() { ./target/release/pons-trainer --arch mlp --cuda --device-index $2 --hidden 256 --epochs 300 --lr 0.001 --wd 0 --batch 4096 --val-frac 0.10 --dd-weight 0 --init-seed $1 $STEMS --weights-out $N/seed$1 >$N/seed$1.log 2>&1; }'
        i=0
        for s in $SEEDS; do
            # Alternate the two GPUs; wait after every TRAIN_PAR launches.
            echo "run $s $((i % 2)) &"
            i=$((i + 1))
            [ $((i % TRAIN_PAR)) -eq 0 ] && echo wait
        done
        echo wait
        echo 'echo "train.sh done $(date -Is)" >> $N/README'
    } >"$ROOT/$NAME-train.sh"
    scp -q "$ROOT/$NAME-train.sh" $DL:$B/train.sh || die "scp train.sh"
    ssh -o BatchMode=yes $DL "chmod +x $B/train.sh; rm -f $B/nets/README; setsid nohup $B/train.sh >$B/train.log 2>&1 </dev/null & echo started" || die "train start"
    while ! ssh -o BatchMode=yes $DL "grep -q 'train.sh done' $B/nets/README 2>/dev/null"; do
        ssh -o BatchMode=yes $DL "pgrep -f pons-trainer >/dev/null || grep -q 'train.sh done' $B/nets/README 2>/dev/null" || die "trainer died (see dl02:$B/train.log, nets/*.log)"
        sleep 300
    done
    want=$(for s in $SEEDS; do printf "seed%s.f32 " "$s"; done)
    ssh -o BatchMode=yes $DL "cd $B/nets && ls $want >/dev/null && grep -h 'final val top1\|calibration' seed*.log" >>"$LOG" 2>&1 || die "nets incomplete"
fi

if ! reached fold; then
    stage fold
    mkdir -p "$NETS_DIR"
    scp -q "$DL:$B/nets/*" "$NETS_DIR/" || die "scp nets"
    STEMS=$(ls "$OUT"/*.f32 | sed 's/\.f32$//' | sed 's/^/--data /' | tr '\n' ' ')
    for s in $SEEDS; do
        # shellcheck disable=SC2086
        python3 scripts/fold-constant-inputs.py "$NETS_DIR/seed$s" $STEMS >"$NETS_DIR/seed$s.fold" 2>&1 || die "fold seed$s"
        log "fold seed$s: $(grep -i 'nonzero folded' "$NETS_DIR/seed$s.fold" | head -1)"
    done
    { echo "# $NAME nets, trained on dl02 $(date -Is): Plan B recipe, init seeds $SEEDS, corpus target/corpus-$NAME (feature v$FV${WALK_EXTRA:+, walk $WALK_EXTRA}, M32 labels via --any-identity splice)"; sha256sum "$NETS_DIR"/seed*.f32; } >"$NETS_DIR/README"
fi

stage nets-ready
NETS=$(for s in $SEEDS; do printf "%s/%s/seed%s.f32," "$PWD" "$NETS_DIR" "$s"; done); NETS=${NETS%,}
log "nets ready: NETS=$NETS"
echo "NETS=$NETS"
