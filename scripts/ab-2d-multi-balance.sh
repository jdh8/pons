#!/bin/sh
# ab-2d-multi-balance.sh — N4f + the two Multi reading knobs
# (docs/one-notrump-competitive.md §N4f, docs/one-notrump-multi.md).
#
#   JOBS=24 BOARDS=460800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-2d-multi-balance.sh ab-results/2d-multi-balance \
#       >ab-results/2d-multi-balance.log 2>&1 < /dev/null &
#
# Three independent knobs, three aligned arms, each pinning the other two off.
#
#   balance  `competition.multi_balance` — `1NT (2♦) - (2M) ?` is the lane's one
#            seat with no book node at all (57% of the `(2♦)` bucket: 253 bd,
#            −426 plain, −199 PD on the `1e9a47e2` arms), where the floor reads
#            their `2♦` as diamonds and sells out at the two level.  The arm
#            authors `X` on **five** cards in the major they named — penalty,
#            trump length — plus responder's sits over it.
#
#            The shape is probed, not borrowed.  `probe-bba-constraints --mode
#            custom --seat 0 --calls "1NT 2♦ - 2♥" --filter-call 1NT` (4000
#            hands/vul) reads Pass **94.2%** / `X` 5.8% over `(2♥)` and Pass
#            92.7% / `X` 7.3% over `(2♠)`, the double distilling to
#            `hcp(15..=17) & len(M, 5..) & balanced()`, with no natural rung at
#            any share — so the anchor plays a trump-length *penalty* double
#            here, not the delayed *takeout* double of Multi theory.  Read the
#            ceiling first: acting on 6% of hands cannot carry −426 plain, so
#            this is disaster removal, not the bucket's cause.
#
#   advance  `reading.their_multi_advance_reading` — the shipped reader borrows
#            `advancer_artificial`, which matches only `2♦`/`2♥`/`2♠` because
#            the Landy reader shares it and *its* three-level advances really
#            are natural.  Over a Multi they are not: measured on this build,
#            `1N (2D) X (3H)` reads **♥ 6..13** and `1N (2D) X (4D)` reads
#            **♦ 3..13** — a raise of an unknown major decoded as a suit of
#            their own, on boards where the advance held one diamond.  The arm
#            widens suppression to the whole ladder.  **Suppression only** as
#            of round 2: round 1 also published `♥3+ & ♠3+` on the jump rungs
#            and measured negative in ALL EIGHT cells, its worst boards showing
#            our side talked out of a correct `4♠` save by spade length the
#            advancer did not hold.  `probe-bba-constraints --mode custom
#            --seat 3 --calls "1NT 2♦ 2♠"` (6000 hands) says why: their `3♥` is
#            `♥ 2–5 (median 3) / ♠ 2–4` — so the natural walk's `♥ 6..13` is
#            false across most of the band (suppression is right) but `♠3+` is
#            false at its tail (the claim was not).
#
#   xfloor   `reading.their_multi_double_reading` — `1NT (2♦) X` is authored
#            `hcp(6..)` but `responder_overcall_double_reading` hard-codes the
#            `DoubleStyle` 8+ for every `1NT (2X) X`, so the call reads
#            `points 8..` and asserts two points responder never promised.  A
#            false assertion is strictly worse than none, but the `X` lane it
#            lands in measures −1.02 plain / **+0.67 PD** and the census's
#            verdict there is "no new work" — hence its own arm rather than a
#            silent fix.
#
# Design rules this script encodes:
#   * `--their-2d-multi` and `--filter-1nt` ride EVERY arm — the disclosure is
#     the lane, and the filter is applied before any bidding so all four arms
#     deal the same board set and stay seed-aligned for paired diffs.
#   * Every arm pins the other two knobs off, so each effect is independently
#     measurable (`ab-2d-multi-residue.sh`'s rule).
#   * `probe-divergence --gate-opener ours` must read 0 foreign on every pair
#     before any headline is quoted.  Expect the campaign's known mirror-read
#     leak as residue (~1 board/cell: their `2NT` decoding off our
#     `multi_2d_responder` relay row when *they* opened 1NT); price it, do not
#     gate on it.
#   * Verdicts are pre-registered so neither can be chosen after the fact.
#     `balance`'s mechanism is *doubling more*, so the measurement.md domain
#     addendum applies: plain DD is the arbiter and `plain win | PD wash` is
#     shippable (v7 shipped on exactly that row).  Both reading arms move
#     what the floor believes rather than what it doubles, so PD is a real
#     arbiter there and they need `wash | win` or better.
#   * Round 1 (seed set 1) is a SCREEN, not a ship verdict.  Seed sets 2-3
#     replicate whatever survives; `SEED_BASE` is persisted per set so a
#     killed run resumes on the same seeds.
#
# Headline is IMPs per *accepted* deal (`--filter-1nt` is applied before any
# bidding, so an arm's boards ARE its accepted deals), with
# `per-board = conditional mean × trigger density` alongside.
#
# Resumable: each seed set owns its seed, arms, probes, and diffs.  Iron rule:
# do NOT rebuild binaries while this runs.
ROOT_R=${1:?usage: ab-2d-multi-balance.sh RESULTS_DIR}
R=$ROOT_R
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
PROBE=target/release/examples/probe-divergence

for seed_set in 1 2; do
    R="$ROOT_R/seed-$seed_set"
    mkdir -p "$R"
    SEED_BASE=$(seed_for multi-balance)
    log "=== 2d-multi-balance seed=$seed_set SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

    for v in none both; do
        arm base "$v" --their-2d-multi true --filter-1nt
        arm balance "$v" --their-2d-multi true --ns-multi-balance --filter-1nt
        arm advance "$v" --their-2d-multi true --ns-their-multi-advance-read --filter-1nt
        arm xfloor "$v" --their-2d-multi true --ns-their-multi-double-read --filter-1nt

        for pair in balance:base advance:base xfloor:base; do
            on=${pair%%:*}
            off=${pair#*:}
            gatepair "$on" "$off" "$v"
            diffpair "$on" "$off" "$v"
            sddiff "$on" "$off" "$v"
        done
    done
done

log "2d-multi-balance done"
