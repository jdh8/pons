#!/bin/sh
# ab-bba-own-net.sh — D6 of docs/portable-judgement.md: should the `bba` system
# serve its own net?  Two arms on paired deals, the house four cells, each
# against BBA told the arm's own card:
#
#   bba      BBA's walked bands, the shipped K = 8 choosing (the default until
#            D6 shipped 2026-11-05; `bba` now serves `bba-r` itself)
#   bba-own  bba-file: the same bands, the net a logit mean over BBA_OWN
#            (`bba-r`, the Phase 1 re-read's K = 8 on the bba-read relabel)
#
#   BBA_BOOK=ab-results/bba-book/<run> BBA_OWN=b1.f32,...,b8.f32 \
#       VULS="none ns ew both" BOARDS=204800 PAR=16 \
#       setsid nohup scripts/idle-run.sh scripts/ab-bba-own-net.sh \
#       ab-results/bba-own-net >ab-results/bba-own-net.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir → a new seed).
R=${1:?usage: ab-bba-own-net.sh RESULTS_DIR}
: "${BBA_BOOK:?set BBA_BOOK to the walked book the bba arms play}"
: "${BBA_OWN:?set BBA_OWN to the candidate blobs, a.f32,b.f32,...}"
export BBA_BOOK
BUILD_EXTRA='--example bba-decompose'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)
echo "$BBA_BOOK" >"$R/bba-book"

# Pin the blobs: a resume with other weights would diff an arm drawn from two nets.
W=$(echo "$BBA_OWN" | tr , '\n' | xargs sha256sum)
[ -s "$R/weights.bba-own" ] || echo "$W" >"$R/weights.bba-own"
[ "$(cat "$R/weights.bba-own")" = "$W" ] || { echo "ab-bba-own-net: $R was generated with other blobs" >&2; exit 1; }

log "=== bba own net start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul, VULS=$VULS, BBA_BOOK=$BBA_BOOK"
for vul in $VULS; do
    # bba-gen --provenance reports PONS_FLOOR_WEIGHTS whenever it is set.
    (unset PONS_FLOOR_WEIGHTS; arm bba "$vul" --our-floor bba) || exit 1
    (export PONS_FLOOR_WEIGHTS=$BBA_OWN; arm bba-own "$vul" --our-floor bba-file) || exit 1
    diffpair bba-own bba "$vul"
done

for a in bba bba-own; do
    out="$R/decompose.$a.md"
    [ -s "$out" ] && { log "skip $out (exists)"; continue; }
    log "decompose $a vs BBA (all vuls, book census)"
    if [ $a = bba-own ]; then floor=bba-file; w=$BBA_OWN; else floor=bba; w=; fi
    # shellcheck disable=SC2046
    env ${w:+PONS_FLOOR_WEIGHTS=$w} target/release/examples/bba-decompose \
        $(for vul in $VULS; do printf '%s ' "$R/$a-$vul"; done) \
        --our-floor "$floor" --dd-cache "$R/dd-cache.json" --report "$out" --book-census \
        >"$R/decompose.$a.txt" 2>>"$R/log"
done
log "=== bba own net done"
