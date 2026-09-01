#!/bin/sh
# ab-2d-multi-doubler.sh — the Kokish–Kraft doubler's natural other major
# (docs/one-notrump-competitive.md §N4-KK residue 4, docs/one-notrump-multi.md).
#
#   JOBS=24 BOARDS=4608000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-2d-multi-doubler.sh ab-results/2d-multi-doubler \
#       >ab-results/2d-multi-doubler.log 2>&1 < /dev/null &
#
# What is being tested.  `competition.multi_doubler_major` gives responder a
# natural bid of the *other* major once their pass-or-correct has resolved
# theirs.  Two rungs and two answer tables:
#
#   2♠      `1NT (2♦) X (2♥) - -`, on `len(♠, 4..)` at **weight 100** — below
#           every rung of `kokish_kraft_doubler_rebid` and above its catch-all,
#           so it fires on exactly the hands that pass today and cannot move a
#           call the shipped table already makes.  No shortness conjunct: four
#           of *their* major doubles at weight 155, so the ordering supplies the
#           cap, and by the same ordering responder's suit here is exactly four
#           (any five-card major took `2M` or the `3♦`/`3♥` transfer instead).
#   3♥      `1NT (2♦) X (2♥) X (2♠)` — opener *doubled* `(2♥)` on four-plus
#           hearts, so this is a known 4-4 at the three level with 23+ combined.
#   answer  opener bids game in the fit from the top of the range (`hcp 16+`),
#           takes the invitational raise where there is room below game, else
#           passes; responder accepts the invitation on `points 11+` — eleven
#           opposite a known fifteen is twenty-six.
#
# Two of the four resolved paths, deliberately.  `multi_penalty_answer` doubles
# their `(2M)` on `len(major, 4..)` at weight 150 against a weight-0 catch-all,
# so opener's pass over `(2♥)` **denies** four hearts and its double **shows**
# four.  `X (2♥) - (2♠)` is therefore excluded on a mechanism — `3♥` there
# finds a 4-3 at best.  `X (2♠) - -` is **withheld pending a ruling**: opener
# said nothing about hearts, so a four-card suit at the three level opposite
# unknown support fires only on the misfits, and the census gives that leg 25
# boards worth −30 plain / +8 PD, i.e. nothing to repair.  One token in
# `kokish_kraft_entries` re-arms it if the user wants it priced.
#
# Why this rung.  Replaying the shipped K–K arms (`ab-results/2d-multi-kk-gated`,
# 230 400 bd/arm/vul) through `probe-1nt-interference --bucket "2♦" --responses`
# splits the `X` branch by what responder does once the auction resolves:
#
#   responder PASSES  (X 2♥ P P P, X 2♥ X 2♠ P, X 2♥ P 2♠ P)   293 bd  −824 plain / +65 PD
#   responder DOUBLES (X 2♥ P P X, X 2♥ P 2♠ X)                 44 bd  +182 plain / +191 PD
#
# and the dumped boards say why: five of the sixteen worst `X (2♥) - -` boards
# and seven of the twelve worst `X (2♥) X (2♠)` boards are a 4-4 major fit we
# never find, passed out at −110 while BBA bids `4M` and makes it.  The PD
# column is a wash, not a win, so **this is a plain-DD repair with a PD
# non-inferiority requirement**, not a both-scorer bet — read both columns.
#
# Two arms, one seed set, `--their-2d-multi` on both so the rung is the only
# difference (Kokish–Kraft is the shipped default and needs no flag):
#
#   base   the shipped K–K table, nothing natural at the resolved seat
#   dm     the natural other major and its two answer tables
#
# `--filter-1nt` (balanced 15-17 somewhere, a raw-hand test applied BEFORE any
# bidding) rides both so they deal the same board set and stay seed-aligned for
# the paired diffs.
#
# SIZE THIS ARM BIG.  The rung fires inside a bucket that is 0.9% of a filtered
# arm, and the K–K A/B's own numbers give the resolution constant: 5.39 IMPs sd
# per contract-divergent board, so a cell needs |Δ| > 10.56/√n_div IMPs per
# divergent board.  At 230 400 bd/vul this rung reaches ~130 boards and cannot
# clear; at 2.3M — `multi_minor_slam_try`'s round-2 size — it reaches ~1 300 and
# a +1 IMP/fired effect is a comfortable `t`. Read the **per-fired** paired diff,
# not the per-board headline, exactly as `ab-2d-multi-slam.sh` did.
#
# Scoring.  Plain AND perfect defense, read off the decision table in
# docs/measurement.md; `sddiff` is the tie-breaker.  Read
# `probe-divergence --gate-opener ours` BEFORE any headline — the mirror-read
# leak (fixed at `29f93561`) is what makes a counter knob a reading knob, and
# the gate must read **0 foreign**.  Both rungs live inside the subtree keyed on
# their `2♦` disclosure, which `System::opponents` clears, so the mirror should
# stay inert; a non-zero reading here means the mirror regressed, not this.
#
# Before declaring a loss dead, trace the worst divergent boards.  The two named
# suspects: opener passing `2♠` on three-card support into a 4-3 that goes down
# where defending their partscore was +50, and the invitational `3♠` stopping
# below a game the 16-count would have bid.
#
# Resumable: an existing arm dir, gate, or diff file is skipped, and SEED_BASE
# persists in $R/2d-multi-doubler.seed.  Iron rule: do NOT rebuild binaries
# while this runs.
R=${1:?usage: ab-2d-multi-doubler.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
PROBE=target/release/examples/probe-divergence

gatepair() {
    on=$1; off=$2; vul=$3
    out="$R/gate.$on.vs.$off.$vul.txt"
    [ -s "$out" ] && { log "skip $out (exists)"; return 0; }
    log "isolation gate $on vs $off ($vul)"
    "$PROBE" "$R/$on-$vul" "$R/$off-$vul" --gate-opener ours >"$out"
}

SEED_BASE=$(seed_for 2d-multi-doubler)
log "=== 2d-multi-doubler SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --their-2d-multi --no-ns-multi-doubler-major --filter-1nt
    arm dm   "$v" --their-2d-multi --filter-1nt

    gatepair dm base "$v"
    diffpair dm base "$v"
    sddiff   dm base "$v"
done

log "2d-multi-doubler done"
