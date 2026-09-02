#!/bin/sh
# M5.2 sequence corpus: dump-v6.sh with --feature-version 7, which writes the
# byte-identical v6 `.f32` plus a `.seq` sibling of one token per prior call.
#
# Reusing the v6 mixture's *exact* bank rows is DELIBERATE, not laziness: the
# LSTM and its MLP control must be fitted on the same deals and the same cells,
# or the fidelity gate measures the corpus instead of the architecture.  The
# `.f32` this writes is byte-identical to corpus-v6's, so the control is the
# same stems trained with --arch mlp.  Registered in docs/pdd-bank-ledger.md.
#
# Output goes to target/corpus-v7 like every other dump script.  This dump is
# ~13.6 GB (~7.6 GB of `.seq` on top of ~6.0 GB of `.f32`), so on a box where
# `/` is tight, point target/corpus-v7 at a data disk with a symlink (or set
# DUMP_OUT).  Not /nfs2/jdh8: it is read-only from this box, its tree being
# owned by jdh8's LDAP uid (133017) while this host runs him as uid 1016, so
# the bank reads fine and nothing writes back.
set -eu
cd "$(dirname "$0")/.."

BANK=/nfs2/jdh8/pons/22.pdd
OUT=${DUMP_OUT:-target/corpus-v7}
BIN=target/release/examples/dump-teacher
COMMON="--deals $BANK --teacher bba --configured --feature-version 7"
[ "${DUMP_VS_BBA:-false}" = true ] && COMMON="$COMMON --vs-bba"
mkdir -p "$OUT"

# shellcheck disable=SC2086
shard() {
    name=$1 skip=$2 boards=$3 seed=$4
    shift 4
    [ -s "$OUT/$name.json" ] && { echo "skip $name (exists)"; return 0; }
    $BIN $COMMON --skip "$skip" --boards "$boards" --seed "$seed" \
        --out "$OUT/$name" "$@" >"$OUT/$name.log" 2>&1 &
}

for i in 0 1 2 3 4 5 6 7; do
    shard "uniform-$i" $((3250000 + i * 31250)) 31250 $((100 + i))
done
for i in 0 1 2 3; do
    shard "enriched-$i" $((3500000 + i * 125000)) 125000 $((200 + i)) \
        --enrich 28:9 --replay --cell a-on/a-on --cell a-off/a-off
done
i=0
for bit in 0004 1000 2000 0002 4000 0800 8000 0020; do
    shard "axis-$bit" $((4000000 + i * 20000)) 20000 $((300 + i)) \
        --replay --cell "a-off/a-off" --cell "a-off+$bit/a-off"
    i=$((i + 1))
done
wait
echo "dump-v7: all shards done"
