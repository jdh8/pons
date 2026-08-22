# The `1NT (2♦)` tree — their Multi over our 1NT

BBA's Woolsey Multi-Landy overcalls our 1NT with an artificial `2♦`: **one
unknown six-card major**, symmetric ♥/♠. `their.two_diamonds_multi` is the
disclosure that swaps our natural-`(2♦)` leg for the N4 tables; while it is
`false` (the engine default, and every arm that is not a BBA anchor) none of
this lane exists and their `2♦` is read as diamonds.

This document is the **map**: every authored node, who owns each seat, and what
each call reads as. The campaign ledger — which round shipped what, and the
A/B numbers — stays in
[one-notrump-competitive.md §N4](one-notrump-competitive.md#n4--their-2-as-a-multi-shipped-2026-08-15--v7-seven-rounds-default-on-vs-bba-via-the-census)
and its archive. Nothing here is a verdict; everything here is regenerable.

**Scope.** Only `P* 1NT (2♦)`. Since `d54ef73f` the systems-on strip keeps
`(1x) 1NT (2♦)` — *they* opened, *we* overcalled 1NT — out of the lane; nothing
below that key is authored at all.

## Regenerate

```bash
# The tree: 89 sections, the whole lane, rules with their own English
cargo run --release --example render-book -- --their-2d-multi --prefix "1NT 2♦"

# What one call reads as, from the seat about to act
cargo run --release --example probe-call-reading -- --their-2d-multi "1N (2D) X -"

# What each cell costs against BBA (needs an anchor arm on disk)
cargo run --release --features serde --example probe-1nt-interference -- \
    ab-results/anchor-confirm/2026-08-21-1e9a47e2/american-none \
    --dd-cache ab-results/anchor-confirm/dd-cache.json --bucket "2♦" --responses 6
```

`render-book` prints raw trie keys — no parentheses, `-` for pass. This
document uses the house convention (`1NT (2♦) X (2♥) - -`); they are the same
auction.

## Responder's table — `1NT (2♦) ?`

The one node every branch hangs off (`multi_2d_responder`,
`rubensohl.rs:499`).
Weights decide, so read the table top-down: the first rule whose constraint the
hand satisfies at the highest weight wins.

| call | w | constraint | branch |
| --- | ---: | --- | --- |
| `4♦` | 200 | 5+♥, 5+♠, 10+ pts | both majors, opener picks |
| `4♣` | 200 | 5+♣ and a 5-card major, 10+ pts | Leaping Michaels |
| `3♣` | 185 | exactly 4♥ or exactly 4♠, 10+ pts, not flat 4333 | Stayman → Smolen |
| `3♦` | 180 | 5+♥, 9+ pts | transfer to hearts |
| `3♥` | 180 | 5+♠, 9+ pts | transfer to spades |
| `3NT` | 150 | 10+ pts, stoppers in **both** majors | to play |
| `3♠` | 145 | 6+♣, 10+ pts | the long-club outlet |
| `2♥` / `2♠` | 140 | ≤8 pts, and (5-card suit with 5+ HCP **or** 6-card suit, no HCP floor) | the weak escape (N4e, `multi_weak_escape = Some(6)`) |
| `2NT` | 135 | ≤8 pts, and (a 5-card suit with 6+ HCP, or any 6-card suit) | relay to `3♣`, then sign off |
| `X` | 130 | 6+ HCP | **values**, alerted `comp:multi-values` — they name the major, we act |
| `P` | 0 | — | the catch-all |

The double is the workhorse and the design's hinge: it is *not* a diamond
penalty double (they have no diamonds) and *not* takeout — it waits for the
advancer to resolve the major, then places the contract.

## The tree

### The `X` family — 20 nodes, the whole authored depth

```
1NT (2♦) X -                    opener names a 4-card major: 2♥ (4+♥) w141 / 2♠ (4+♠) w140 / P
1NT (2♦) X (2♥)                 opener: X (4+♥) w150 / P          ← the trump penalty double
1NT (2♦) X (2♠)                 opener: X (4+♠) w150 / P
1NT (2♦) X (2♥) - -             responder, opener sat: 4NT (16+) / 2♠ (5+♠ ≤8) / X (4+♠ ≤2♥, takeout) / 3NT (10+ & ♥ stop) / P
1NT (2♦) X (2♥) - - X -         opener answers the takeout X: P (4+♥ = convert) / 2♠ (4+♠) / 3♣ / 3♦ / 2NT
1NT (2♦) X (2♥) - - 2♠ -        opener answers responder's 2♠ signoff
1NT (2♦) X (2♥) - - 4NT -       quantitative: 6NT (17+) / P
1NT (2♦) X (2♥) - (2♠)          they ran; responder again: 4NT / X (4+♠ 7+) / 3NT / P
1NT (2♦) X (2♥) - (2♠) X -      P — sit (the `ran` branch never pulls)
1NT (2♦) X (2♥) - (2♠) 4NT -    6NT / P
1NT (2♦) X (2♥) X (2♠)          opener doubled, they ran — same responder table
1NT (2♦) X (2♥) X (2♠) X -      P
1NT (2♦) X (2♥) X (2♠) 4NT -    6NT / P
1NT (2♦) X (2♠) - -             the spade leg: 4NT / X (4+♥ ≤2♠) / 3NT / P
1NT (2♦) X (2♠) - - X -         P (4+♠) / 3♥ (4+♥) / 3♣ / 3♦ / 2NT
1NT (2♦) X (2♠) - - 4NT -       6NT / P
1NT (2♦) X (2♥) X -             P     ┐
1NT (2♦) X (2♠) X -             P     │ sits: `multi_stopper_ask` is Off,
1NT (2♦) X (2♠) X (2NT)         P     │ so these seats only pass
1NT (2♦) X (2♠) - (2NT)         P     ┘
```

Four *resolved* paths carry the family — `X (2♥) - -`, `X (2♥) - (2♠)`,
`X (2♠) - -`, `X (2♥) X (2♠)` — because the pass-or-correct resolves the major
and the resolved suit is what every later double keys on.

### The weak escape — `2♥` / `2♠`, 15 nodes

```
1NT (2♦) 2♥ -                   opener: 4♥ on a monster fit (3♥+23 / 4♥+22 / 5♥+21 pts) else P
1NT (2♦) 2♥ (X | 2♠ | 2NT | 3♣ | 3♦ | 3♥ | 3♠)    opener: 3♥ (3+♥ 16+) / X (16+) / P
1NT (2♦) 2♠ -                   opener: 4♠ on the same ladder else P
1NT (2♦) 2♠ (X | 2NT | 3♣ | 3♦ | 3♥ | 3♠)         opener: 3♠ (3+♠ 16+) / X (16+) / P
```

Over their `X` and over `3♥`/`3♠` (nothing left to bid below the fit) the table
collapses to `X (16+) / P`. **The tail stops at `3♠`** — the build's own words,
"above that, and after opener's answer, the floor keeps the seat"
(`lebensohl.rs:2068-2082`).

### The `2NT` relay — 23 nodes plus 18 guarded sits

```
1NT (2♦) 2NT -                  opener must bid 3♣ (w100, unconditional)
1NT (2♦) 2NT (X)                still 3♣
1NT (2♦) 2NT - 3♣ -             responder signs off in the cheapest 5-card suit: 3♦ / 3♥ / 3♠, or P = clubs
1NT (2♦) 2NT - 3♣ (X)           the same three sign-offs
1NT (2♦) 2NT (X) 3♣ -           the same three sign-offs
… each sign-off × {-, X, any overcall ≤7♠} → P
```

Eighteen of the lane's 89 sections are those trailing "P w0" sits, which is why
the relay lane measured free (79 bd, −0.08/bd plain, +0.14 PD).

### The constructive branch — 12 nodes

```
1NT (2♦) 3♣ -                   opener: 3♥ (4+♥) / 3♠ (4+♠ ≤3♥) / 3♦ (w50 = no major)
1NT (2♦) 3♣ - 3♦ -              Smolen: 3♥ = exactly 4♥ + 5♠ / 3♠ = exactly 4♠ + 5♥ / 3NT
1NT (2♦) 3♣ - 3♦ - 3♥ -         opener: 4♠ (3+♠) / 3NT (≤2♠)
1NT (2♦) 3♣ - 3♦ - 3♠ -         opener: 4♥ (3+♥) / 3NT (≤2♥)
1NT (2♦) 3♣ - 3♥ -              responder: 4♥ (4+♥) / 3NT
1NT (2♦) 3♣ - 3♠ -              responder: 4♠ (4+♠) / 3NT
1NT (2♦) 3♦ -                   transfer completion: 4♥ (3+♥) / 3NT (≤2♥) / P
1NT (2♦) 3♥ -                   4♠ (3+♠) / 3NT (≤2♠) / P
1NT (2♦) 3♠ -                   3NT
1NT (2♦) 4♦ -                   opener picks: 4♠ (4+♠) / 4♥ (4+♥) / 4♠ (3+♠) / 4♥
1NT (2♦) 4♣ -                   opener relays 4♦
1NT (2♦) 4♣ - 4♦ -              responder: 4♥ / 4♠ (5+) / 5♣
```

**Every one of these keys ends in `-`.** The whole constructive branch is
authored on the assumption that the advancer passes; see the holes below.

## Their side — what the advancer actually does

From the shipped anchor arms (`1e9a47e2`, seed 1787064872, NV+vul pooled, 816
contested boards, `--responses 1` aggregated by the advancer's call):

| their advance | bd | plain tot | plain/bd | PD tot | PD/bd | what it is |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `(2♥)` | 269 | −455 | −1.69 | −88 | −0.33 | pass-or-correct |
| `(2♠)` | 215 | −180 | −0.84 | +56 | +0.26 | pass-or-correct |
| `(4♦)` | 17 | −79 | **−4.65** | −66 | −3.88 | pass-or-correct **to 4M** |
| `(3♥)` | 21 | −45 | −2.14 | −30 | −1.43 | pass-or-correct at the 3-level |
| `(3♠)` | 9 | 0 | 0.00 | +10 | +1.11 | " |
| `(P)` | 284 | +15 | +0.05 | +38 | +0.13 | we took their room away |

`(4♦)` is **not** natural and not a strength-showing acceptance: the advancer's
hands on those boards are 4-4-4-1 / 3-4-5-1 / 4-3-6-0 / 3-4-5-1 / 3-4-5-1 /
5-3-1-4 with **1–7 HCP** — three or four cards in *both* majors, and the
overcaller always corrects to the real suit. Diamond length runs from one card
to six. The whole ladder (`2♥/2♠`, `3♥/3♠`, `4♦`) is one call: *bid your major*.

## What we read

Measured with `probe-call-reading --their-2d-multi`, as the seat about to act
sees it. Our own calls, read by opener:

| our call | reads as |
| --- | --- |
| `X` | `points 8..37`, ♥ ≤4, ♠ ≤4 |
| `2♥` / `2♠` | `points 0..8`, suit 5+ — in step with the floorless rung |
| `2NT` | `points 0..8`, shape ⊤ — the relay carries the same floorless arm |
| `3♣` | `points 10..`, shape ⊤ |
| `3♦` / `3♥` | `points 9..`, the *transferred-to* major 5+ |
| `3♠` | `points 10..`, ♣ 6+, ♥ ≤4, ♠ ≤4 |
| `4♣` | `points 10..`, ♣ 5+ |
| `4♦` | `points 10..`, ♥ 5+, ♠ 5+ |
| `P` | nothing (⊤ on all five axes) |

Their calls:

| their call | reads as | correct? |
| --- | --- | --- |
| `(2♦)` the Multi | suppressed (`⊤`) under `their_multi_reading`; **♦ 5..13, points 8..** without it | yes / no |
| `(2♥)` / `(2♠)` advance | suppressed (`⊤`) | yes — names no holding of its own |
| `(3♥)` / `(3♠)` after our `X` | **♥ 6..13** / **♠ 6..13** | **no** — a raise of an unknown major, not a suit of their own |
| `(3♦)` / `(4♦)` after our `X` | **♦ 3..13** | **no** — phantom suit; the advance denies nothing in diamonds (♦1 on one loss board) |
| `(4♥)` after our `X` | nothing (`⊤`) | — |
| `(3♥)`/`(3♠)`/`(4♦)` after our `2♠`/`2NT`/`3♣`/`3♥` | nothing (`⊤`) | harmless, but the major fit never reaches the floor |

On the 17 measured `(4♦)` boards our response was `2♥/2♠/2NT/3♣/3♥`, never `X`,
so the phantom-♦ read is **reachable but did not fire there** — those boards
lost with the advance reading as nothing at all and the floor bidding blind
(one is opener doubling `4♥` holding ♥AJ4 opposite responder's heart void).

`advancer_artificial` (`readers.rs:295-317`) matches **only `2♦`/`2♥`/`2♠`** —
the two-level rung. Every rung above it falls through to the natural walk, and
the readings are identical with and without `their.two_diamonds_multi`.

## The holes — seats the floor owns

| seat | authored? | cost (same arms) |
| --- | --- | --- |
| **`1NT (2♦) - (2M) ?`** — responder passed, they named the major, opener to act | **no node at all** | **253 bd, −426 plain, −199 PD** — 57% of the bucket, negative on *both* scorers |
| `1NT (2♦) 2♥ (4♦)` and anything above `3♠` over the escape | no — tail stops at `3♠` | inside the `(4♦)` row above |
| `1NT (2♦) X (3♥ \| 3♠ \| 4♦)` | no — the family keys on `(2♥)`/`(2♠)` only | not separately measured |
| `1NT (2♦) 3♣ (X)`, `3♦ (3♠)`, … any interference over a constructive call | no — every constructive key ends in `-` | not separately measured |
| `1NT (2♦) X (2♠) - (2NT)` and siblings | node exists, but it only passes | — |

The first row is the lane's real work. The authored family hangs entirely off
responder's `X`; when responder passes — 264 of 816 boards, the biggest single
response — every later seat is the learned contested floor's, and opener sells
out at the two level.

## Open items and traps (flagged; no system change here)

1. **The values double reads two points too strong.** Rule
   `.rule(Call::Double, 130, hcp(6..))` (`rubensohl.rs:531`); measured reading
   `points 8..`. The source is `responder_overcall_double_reading`
   (`readers.rs:527`), which hard-codes the `DoubleStyle` 8+ floor for every
   `1NT (2X) X` and is not Multi-aware. §N4's table claims "Read: `points 6..`,
   every suit ⊤" — stale on both halves (the ♥≤4/♠≤4 cap is sound: by weight
   ordering a 5-card major always escapes or transfers instead).
   *Proposed reversible default:* make the reader's floor follow the lane's own
   rule, gated on `their_multi_reading`; the reading knob is a bidding knob, so
   it needs its own A/B.
2. **Read the escape with the knob set.** `probe-call-reading` used to
   overwrite `multi_weak_escape` with `None` whenever `--ns-multi-weak-escape`
   was absent, so the escape read `points 5..8` — the *pre-ship* rule — and
   looked out of step with `lebensohl.rs:2030-2041`. Absent now leaves the
   shipped `Some(6)` alone (`0` turns it off), and the pair reads `points
   0..8` on both halves, as the ship claimed. No system change; only the probe
   moved.
3. **The advancer's ladder above `2♠` is unread** — item 3/4 of the reading
   table. *Proposed reversible default:* widen `advancer_artificial` to the
   whole pass-or-correct ladder (`3♦/3♥/3♠/4♦`, and `4♥/4♠` for symmetry),
   which only ever *removes* a false length — the soundness argument in its own
   doc comment. The **positive** read ("3+ in both majors, ≤8 HCP") is a new
   assertion and belongs to the declared-opponent book, not to a local reader.
4. `docs/ai-bidder/bba-1nt-defense.md` documents no four-level advance at all.

## See also

- [one-notrump-competitive.md](one-notrump-competitive.md) — the campaign, the
  census, the ledger; §N4/§N4e own this lane's verdicts.
- [authored-reading-handoff.md](authored-reading-handoff.md) — why a
  reading-only change is a bidding change, and the mirror-read leak that makes
  our `1NT (2♦)` table decode *their* `2NT`.
- [bidding-architecture.md](bidding-architecture.md) — book shadows floor; to
  give the floor a seat, delete the node.
