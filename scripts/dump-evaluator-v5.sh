#!/bin/sh
# Phase 5 evaluator corpus and deal-disjoint held-out gate.  The live default
# ReadingProfile is part of `eval5`; no legacy closure/view switches apply.
set -eu
cd "$(dirname "$0")/.."

BIN=target/release/examples/dump-evaluator
BANK=/nfs2/jdh8/pons/22.pdd
OUT=target/corpus-eval-v5
mkdir -p "$OUT"

if [ ! -s "$OUT/train.json" ]; then
    "$BIN" --deals "$BANK" --skip 0 --count 450000 --seed 600 \
        --encoding eval5 --envelope-union --out "$OUT/train"
fi
if [ ! -s "$OUT/test.json" ]; then
    "$BIN" --deals "$BANK" --skip 450000 --count 50000 --seed 601 \
        --encoding eval5 --envelope-union --out "$OUT/test"
fi
echo "dump-evaluator-v5: train and held-out test done"
