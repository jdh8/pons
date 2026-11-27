#!/bin/sh
# ab-strawberry-lanes.sh — price one round of authoring under the Polish 1♣
# (docs/strawberry.md §6).  No knob (Strawberry takes none), so the round is
# two binaries on one seed, vs BBA on its 2/1 card, each arm's own card
# generated from its system:
#
#   advanced    the shipped default, `american` (this checkout)
#   before      `--our-floor strawberry` built at BEFORE_SHA, in a worktree
#               under $R and copied to $R/bin before any arm
#   strawberry  `--our-floor strawberry`, this checkout
#
# `strawberry vs before` prices the round and `--by node` files each board at
# the lane that moved it; `strawberry vs advanced` says where the system
# stands.  `american` is byte-identical across the round (`smoke-default`),
# so the `advanced` arm serves both.
#
# Round 2 (BEFORE_SHA d6552f49): the house XYZ borrowed under `1♣ - 1M - 1NT`,
# the `2♣!` puppet over `1♣ - 1♥ - 1♠`, the house 2NT structure under
# `1♣ - 1♦ - 2♦!`, responder's placement over `1♣ - 1♦ - 2M`, and opener's
# rebid over `1♣ - 3♦!` (a passed hand's `3♦` no longer forced to `3NT`).
# The five lanes cost −2,184 plain IMPs of −13,117 in the none cell of the
# first run (pueue 43, re-sliced by Strawberry's own auction).
#
#   BOARDS=409600 scripts/queue-ab.sh strawberry-lanes \
#       scripts/ab-strawberry-lanes.sh ab-results/strawberry-lanes
#
# Verdict per docs/measurement.md: plain DD is the verdict, PD the guard;
# delta = 0.002 IMPs/board.  Strawberry is not the default, so nothing ships
# on this run: a lane that loses is traced and re-authored.
#
# Resumable; SEED_BASE persists in $R/seed.  Iron rule: do NOT rebuild
# binaries while this runs.
R=${1:?usage: ab-strawberry-lanes.sh RESULTS_DIR}
BEFORE_SHA=${BEFORE_SHA:-d6552f49}
SHOW=${SHOW:-40}
BUILD_EXTRA='--example probe-divergence --example ab-dump-bucket'
VULS=${VULS:-none both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

out="$R/bin/before"
if [ ! -x "$out" ]; then
    wt="$R/wt-$BEFORE_SHA"
    [ -d "$wt" ] || git worktree add --detach "$wt" "$BEFORE_SHA" >>"$R/log" 2>&1
    log "build the before arm from $BEFORE_SHA in $wt"
    (cd "$wt" && cargo build --release --features serde --example bba-gen) >>"$R/log" 2>&1
    mkdir -p "$R/bin"
    cp "$wt/target/release/examples/bba-gen" "$out"
fi
BEFORE_BIN=$(realpath "$out")

log "=== strawberry-lanes start, sha=$SHA vs before=$BEFORE_SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm advanced "$vul"
    # Subshell: bash-as-sh keeps a prefix assignment past a function call.
    (BBA_GEN=$BEFORE_BIN arm before "$vul" --our-floor strawberry)
    arm strawberry "$vul" --our-floor strawberry
    diffpair strawberry before "$vul"
    diffpair strawberry advanced "$vul"
    target/release/examples/ab-dump-bucket "$R/strawberry-$vul" "$R/before-$vul" \
        --by node --top 60 >"$R/bucket.node.$vul.txt"
    # Where the system stands, by its own lane under each opening.
    target/release/examples/ab-dump-bucket "$R/strawberry-$vul" "$R/advanced-$vul" \
        --by prefix --top 80 >"$R/bucket.prefix.$vul.txt"
done
log "=== strawberry-lanes done"
