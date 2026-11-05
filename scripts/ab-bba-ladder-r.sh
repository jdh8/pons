#!/bin/sh
# ab-bba-ladder-r.sh — should `bba-ladder` serve `bba-r` as `bba` does since D6
# (docs/portable-judgement.md §4 D6)?  Two arms on paired deals, the house four
# cells, each against BBA told the arm's own card.  Both are `--our-floor
# bba-ladder`; they differ by BINARY, the net off the constructive nodes:
#
#   ladder    $R/bin/bba-gen-shipped, built at e42e6e74: the shipped K = 8
#   ladder-r  the working tree: `bba-r`
#
# Preparation (done 2026-11-05): build e42e6e74, copy bba-gen and bba-decompose
# to $R/bin/{bba-gen,bba-decompose}-shipped, then apply the change.
#
#   BBA_BOOK=ab-results/bba-book/2026-10-29-8fb078de-none \
#       VULS="none ns ew both" BOARDS=204800 PAR=16 \
#       setsid nohup scripts/idle-run.sh scripts/ab-bba-ladder-r.sh \
#       ab-results/bba-ladder-r >ab-results/bba-ladder-r.log 2>&1 </dev/null &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir → a new seed).
R=${1:?usage: ab-bba-ladder-r.sh RESULTS_DIR}
: "${BBA_BOOK:?set BBA_BOOK to the walked book the bba arms play}"
export BBA_BOOK
BUILD_EXTRA='--example bba-decompose'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)
echo "$BBA_BOOK" >"$R/bba-book"
BIN=$PWD/$R/bin
[ -x "$BIN/bba-gen-shipped" ] || { echo "ab-bba-ladder-r: $BIN/bba-gen-shipped missing" >&2; exit 1; }
unset PONS_FLOOR_WEIGHTS

log "=== bba ladder r start, sha=$SHA, control=$(cat "$BIN/shipped.sha"), SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul, VULS=$VULS, BBA_BOOK=$BBA_BOOK"
for vul in $VULS; do
    (BBA_GEN=$BIN/bba-gen-shipped; export BBA_GEN; arm ladder "$vul" --our-floor bba-ladder) || exit 1
    (unset BBA_GEN; arm ladder-r "$vul" --our-floor bba-ladder) || exit 1
    diffpair ladder-r ladder "$vul"
done

for a in ladder ladder-r; do
    out="$R/decompose.$a.md"
    [ -s "$out" ] && { log "skip $out (exists)"; continue; }
    log "decompose $a vs BBA (all vuls, book census)"
    if [ $a = ladder ]; then bin=$BIN/bba-decompose-shipped; else bin=target/release/examples/bba-decompose; fi
    # shellcheck disable=SC2046
    "$bin" $(for vul in $VULS; do printf '%s ' "$R/$a-$vul"; done) \
        --our-floor bba-ladder --dd-cache "$R/dd-cache.json" --report "$out" --book-census \
        >"$R/decompose.$a.txt" 2>>"$R/log"
done
log "=== bba ladder r done"
