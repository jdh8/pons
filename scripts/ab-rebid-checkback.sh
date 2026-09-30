#!/bin/sh
# ab-rebid-checkback.sh — the checkback and forcing majors over the 18–19
# `2NT` rebid (`notrump.rebid_checkback`, docs/next-steps.md item 2).
#
#   SKIP_BUILD=1 setsid nohup scripts/idle-run.sh \
#       scripts/ab-rebid-checkback.sh ab-results/rebid-checkback \
#       >ab-results/rebid-checkback.log 2>&1 < /dev/null &
#
# What is being tested.  Over `1m - 1M - 2NT` responder's shipped table is
# `3NT` / `4NT` (12–13) / `6NT` (14+) / pass: no call finds a 5-3 or 6-2 major
# fit, and the anchor (2026-09-26 snapshot, round-2 book bucket) prices our
# `3NT`/`4NT`/pass against BBA's `3♥`/`3♠`/checkback at about −1.8k PD per
# 409,600 boards.  BBA's structure, read off `probe-bba-book` 2026-09-30:
# three of the new minor is "NMF after 2NT rebid" (exactly five of the
# major, 8+), `3M` is six-plus forcing, `3♠` over `1♥` is four spades.
#
#   on   the checkback (opener answers 3-card support / other 4-card major /
#        3NT), the forcing `3M` (always raised; RKCB at 12+), `3♠` over
#        `1♥`, RKCB at 14+ after a found fit, the `4NT`/`6NT` pair after a
#        denial — the shipped default since 2026-09-30
#   off  `--no-ns-rebid-checkback`, the pre-2026-09-30 table (the disarming
#        flag)
#
# SHIPPED default-on 2026-09-30 on this script's first run (SEED_BASE
# 1790760791, 204,800 boards/arm/vul): plain DD +0.0015 (none) / +0.0015
# (both), PD +0.0016 / +0.0017 IMPs/board, CI ±0.0008–0.0011, 269 / 287
# fired (+1.1 / +1.2 per fired).  Worst boards: the doubled checkback
# (`3♦ (X)` drops to the floor — the owed contested tail) and RKCB on a
# 12-count over the six-card raise.
#
# Hypothesis: a plain-DD and PD win in both vulnerabilities, ≈ +0.003–0.005
# per board (the census: ~1,000 divergent rows per 409.6k at about −1.8
# PD each).  The surface is well under 1% of boards — read the paired diff.
# A loss points at the slam legs first (the RKCB floors at 12/14), then the
# `points(6..)` floor of the six-card `3M`.
R=${1:?usage: ab-rebid-checkback.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for rebid-checkback)

log "=== rebid-checkback A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-rebid-checkback
    arm on  "$vul"
    diffpair on off "$vul"
done
log "=== rebid-checkback A/B done"
