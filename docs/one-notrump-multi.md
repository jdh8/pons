# The `1NT (2♦)` tree — their Multi over our 1NT

BBA's Woolsey Multi-Landy overcalls our 1NT with an artificial `2♦`: **one
unknown six-card major**, symmetric ♥/♠. `their.two_diamonds_multi` is the
disclosure that swaps our natural-`(2♦)` leg for the N4 tables; while it is
`false` (the engine default, and every arm that is not a BBA anchor) none of
this lane exists and their `2♦` is read as diamonds.

BBA's own counter under the natural and Multi readings is compared in
[bba-book.md §5.5.1](ai-bidder/bba-book.md#551-bbas-counter-to-1nt-2-natural-versus-multi).

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

# ...and the opt-in Kokish–Kraft variant of the same lane (§N4-KK)
cargo run --release --example render-book -- --their-2d-multi \
    --ns-multi-kokish-kraft --prefix "1NT 2♦"
cargo run --release --example probe-call-reading -- --their-2d-multi \
    --ns-multi-kokish-kraft "1N (2D) X -" "1N (2D) 2N -" "1N (2D) - (2H) - - X -"

# What one call reads as, from the seat about to act
cargo run --release --example probe-call-reading -- --their-2d-multi "1N (2D) X -"
# ...and on the two opt-in reading arms
cargo run --release --example probe-call-reading -- --their-2d-multi \
    --ns-their-multi-advance-read "1N (2D) X (3H)" "1N (2D) X (4D)"
# What their advancer actually holds, which is what settled the readings above
cargo run --release --example probe-bba-constraints -- --mode custom --seat 3 \
    --calls "1NT 2♦ 2♠" --samples 6000 --min-share 0.01
cargo run --release --example probe-call-reading -- --their-2d-multi \
    --ns-their-multi-double-read "1N (2D) X -"

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
| `X` | `points 8..37`, ♥ ≤4, ♠ ≤4 — `points 6..` on `--ns-their-multi-double-read` (open item 1) |
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
| `(3♥)` / `(3♠)` after our `X` | **♥ 6..13** / **♠ 6..13**; ⊤ on `--ns-their-multi-advance-read` | **no** → fixed on the knob — probed at `♥ 2–5, median 3`, so `6..` is false across most of the band |
| `(3♦)` / `(4♦)` after our `X` | **♦ 3..13**; ⊤ on the knob | **no** → fixed on the knob — phantom suit; the advance denies nothing in diamonds (♦1 on one loss board) |
| `(4♥)` after our `X` | nothing (`⊤`) | — |
| `(3♥)`/`(3♠)`/`(4♦)` after our `2♠`/`2NT`/`3♣`/`3♥` | nothing (`⊤`) | harmless, but the major fit never reaches the floor |

On the 17 measured `(4♦)` boards our response was `2♥/2♠/2NT/3♣/3♥`, never `X`,
so the phantom-♦ read is **reachable but did not fire there** — those boards
lost with the advance reading as nothing at all and the floor bidding blind
(one is opener doubling `4♥` holding ♥AJ4 opposite responder's heart void).

`advancer_artificial` (`readers.rs:360`) matches **only `2♦`/`2♥`/`2♠`** —
the two-level rung. Every rung above it falls through to the natural walk, and
the readings are identical with and without `their.two_diamonds_multi`.
`multi_advance_ladder` covers the rest, under its own knob (open item 3).

## The holes — seats the floor owns

| seat | authored? | cost (same arms) |
| --- | --- | --- |
| **`1NT (2♦) - (2M) ?`** — responder passed, they named the major, opener to act | **behind `competition.multi_balance`, default off** (§N4f) | **253 bd, −426 plain, −199 PD** — 57% of the bucket, negative on *both* scorers |
| `1NT (2♦) 2♥ (4♦)` and anything above `3♠` over the escape | no — tail stops at `3♠` | inside the `(4♦)` row above |
| `1NT (2♦) X (3♥ \| 3♠ \| 4♦)` | no — the family keys on `(2♥)`/`(2♠)` only | not separately measured |
| `1NT (2♦) 3♣ (X)`, `3♦ (3♠)`, … any interference over a constructive call | no — every constructive key ends in `-` | not separately measured |
| `1NT (2♦) X (2♠) - (2NT)` and siblings | node exists, but it only passes | — |

The first row was the lane's one named hole; `multi_balance` now authors it,
default off and unresolved after two below-resolution rounds. **Read its
ceiling before its headline**: the
anchor passes that seat **94.2% / 92.7%** (probed 2026-08-22, below), so at most
~6% of those 253 boards are reachable by *any* opener action, and the −426 plain
is overwhelmingly earned elsewhere. The rest of the family hangs off responder's
`X`; when responder passes — 264 of 816 boards, the biggest single response —
every later seat is the learned contested floor's.

### `1NT (2♦) - (2M) ?` — what the anchor does there

`probe-bba-constraints --mode custom --seat 0 --calls "1NT 2♦ - 2♥"
--filter-call 1NT`, 4000 hands per vulnerability, `--min-share 0.005`:

| seat | BBA's table |
| --- | --- |
| `1NT (2♦) - (2♥) ?` | Pass **94.2%** · `X` **5.8%** = `hcp(15..=17) & len(♥, 5..) & balanced()` · `3♣` 0.5% (n=1) |
| `1NT (2♦) - (2♠) ?` | Pass **92.7%** · `X` **7.3%** = `hcp(15..=17) & len(♠, 5..) & balanced()` |

It is a **trump-length penalty double** of the suit they named — *not* the
delayed takeout double of Multi theory ("pass 2♦, then double 2♥ for take-out")
— and there is no natural rung at any share, even though `NotrumpShape::Wide6322`
admits a five-card major and a six-card minor. `multi_balance` authors exactly
that, at `len(M, 5..)`: `multi_penalty_answer`'s four trumps raised to five,
because partner passed rather than doubling and opener is short of the values
half of the structure.

## Open items and traps (flagged; no system change here)

**N4 closed 2026-08-23** ([the queue](one-notrump-competitive.md#package-queue--open-work-ranked-by-the-census)).
Everything below is a **recorded residue**, not open work: the three knobs
stay opt-in — `multi_balance` below this harness's resolution, the two readers
trigger-gated — and responder's natural minor single-suiter stays unbuilt at
≈zero value. Reopening the lane needs a new bucket, not a new seed.

1. **The values double reads two points too strong.** Rule
   `.rule(Call::Double, 130, hcp(6..))` (`rubensohl.rs:531`); measured reading
   `points 8..`. The source is `responder_overcall_double_reading`
   (`readers.rs:599`): with `their_multi_double_reading` off, its base path
   publishes the generic `DoubleStyle` 8+ floor for every `1NT (2X) X`. §N4's
   table claims "Read: `points 6..`, every suit ⊤" — stale on both halves (the
   ♥≤4/♠≤4 cap is sound: by weight ordering a 5-card major always escapes or
   transfers instead).
   **Implemented 2026-08-22 behind `reading.their_multi_double_reading`**
   (`bba-gen --ns-their-multi-double-read`, `probe-call-reading` likewise),
   default off: on the knob the reader follows the lane's own rule and the
   double reads `points 6..`. Its own arm rather than riding
   `their_multi_reading`, so the shipped reader's base does not move; a reading
   knob is a bidding knob.
2. **Read the escape with the knob set.** `probe-call-reading` used to
   overwrite `multi_weak_escape` with `None` whenever `--ns-multi-weak-escape`
   was absent, so the escape read `points 5..8` — the *pre-ship* rule — and
   looked out of step with `lebensohl.rs:2030-2041`. Absent now leaves the
   shipped `Some(6)` alone (`0` turns it off), and the pair reads `points
   0..8` on both halves, as the ship claimed. No system change; only the probe
   moved.
3. **The advancer's ladder above `2♠` is unread** — items 3/4 of the reading
   table. **Implemented 2026-08-22 behind
   `reading.their_multi_advance_reading`** (`--ns-their-multi-advance-read`),
   default off, in two halves:

   - *Suppression*, widened to the whole ladder
     (`2♥/2♠/3♥/3♠/4♣/4♦/4♥/4♠`). `advancer_artificial` is **not** widened in
     place — the Landy reader shares it and *its* three-level advances really
     are natural — so the Multi reader gets its own `multi_advance_ladder`.
   - *No positive claim.* Round 1 also published `♥3+ & ♠3+` on the jump
     rungs, on the theory that an advancer choosing a three- or four-level
     contract must be able to play either major. **Refuted twice over**
     (§N4f round 1) and removed.

   `4♣` is included in the ladder on the user's call (`4♣`/`4♦` both land in
   either 4M) though the census has zero measured `(4♣)` advances — that rung
   is assumption, not evidence.

   **Why suppression is the half worth keeping.**
   `probe-bba-constraints --mode custom --seat 3 --calls "1NT 2♦ 2♠"`, 6000
   hands NV:

   | their call | share | ♥ | ♠ | hcp |
   | --- | ---: | --- | --- | --- |
   | `3♥` | 38.2% | **2–5 (med 3)** | 2–4 (med 3) | 7–13 |
   | Pass | 29.7% | 1–5 | 2–5 | 2–12 |
   | `2NT` | 16.7% | 2–5 | 1–5 | 14–20 |
   | `4♦` | 11.9% | 3–5 (med 4) | 3–6 (med 4) | 3–14 |
   | `X` | 1.5% | 0–1 | 3–6 | 14–19 |

   Their `3♥` is `♥ 2–5`, so the natural walk's `♥ 6..13` is false across most
   of the band — suppression removes a genuinely wrong read. But `♠ 2–4` (and
   a 10th-percentile tail below it) is why `♠3+` was wrong, and `4♦`'s `♠ 3–6`
   has the same tail. And **no strength claim is published** either: the
   envelope has no HCP axis, `points` would fold in the advancer's
   distribution, and `(2♠)` would refuse one anyway (`bba-multi-2d.md §2`:
   `hcp 7–18`, the strength-showing catch-all).
4. `docs/ai-bidder/bba-1nt-defense.md` documents no four-level advance at all.

## The Kokish–Kraft variant — `competition.multi_kokish_kraft` (opt-in, measured 2026-08-25 — stays off)

Everything above maps the **shipped** v7 lane. One knob replaces most of it with
a different published table for the same object: the Eric Kokish–Beverly Kraft
notes, the most complete exact-object package in
[the survey](ai-bidder/multi-landy-2d-counter-defense-research.md). Built and
measured 2026-08-25 — the owned lane reads plain-wash / PD-win (the shippable
shape) but the mirror-read leak fails the isolation gate, so it **stays off**
pending an ownership gate; the numbers, the leak forensic, the design-sketch
repairs and what is owed live in
[§N4-KK](one-notrump-competitive.md#n4-kk--the-kokishkraft-counter-as-an-opt-in-whole-table-variant-measured-2026-08-25-stays-off).

Registered *instead of* the v7 subtree, never over it (`kokish_kraft_entries`,
`lebensohl.rs`) — the two tables disagree on `2NT`, `3♣`, `3♠` and both delayed
doubles.

### Responder's table — `1NT (2♦) ?` on the variant

| call | w | constraint | branch |
| --- | ---: | --- | --- |
| `4♦` | 200 | 5+♥, 5+♠, 10+ pts | Leaping Michaels — unchanged |
| `4♣` | 200 | 5+♣ and a 5-card major, 10+ pts | Leaping Michaels — unchanged |
| `4♥`/`4♠` | 260 | 6+ M, ≤4 oM, `hcp 15..=`[`direct_4m_max`](../src/bidding/american/notrump/texas.rs) — **exactly 15** under the shipped `texas_slam_drive` | **new** — the uncontested direct slam-try tier, `slam_try_answer` + the 1430 ladder below it. See the residue note below |
| `3♦`/`3♥` | 180 | 5+♥ / 5+♠, 9+ pts | transfers to ♥/♠ — unchanged, auto-driven to game |
| `3♣` | 178 | **6+♦, no point floor** | **new** — transfer to diamonds (v7: Stayman) |
| `2NT` | 176 | **6+♣, no point floor** | **new** — transfer to clubs (v7: the weak relay) |
| `3♠` | 152 | 4+♣ *and* 4+♦, one of them 5+, 10+ pts | **new** — both minors, game-forcing (v7: the forced `3♠`→♣ GF) |
| `3NT` | 150 | 10+ pts, **both** majors stopped | unchanged from v4–v7 — see the repair note in §N4-KK |
| `2♥`/`2♠` | 140 | the weak escape, `multi_weak_escape` rung included | unchanged |
| `X` | 130 | **`hcp 8+`, no shape promise** | **changed** — invitational-plus (v7: `hcp 6+`) |
| `P` | 0 | catch-all — and now a **designed** action | **new** — the 6–7 band, with its own delayed table |

Reads (`probe-call-reading --their-2d-multi --ns-multi-kokish-kraft`): `X` →
`points 8..` unbounded; `2NT` → `♣ 6..13, points 0..`; `3♣` → `♦ 6..13,
points 0..`; `3♠` → `♣ 4..5, ♦ 4..5, points 10..`; `2♠` → `♠ 5.., points 0..8`.

### The delayed-double split

The one structural idea every exact-object source in the survey shares, and the
shipped lane's table does not have:

| after | second `X` means | opener answers with |
| --- | --- | --- |
| `1NT (2♦) X (2M) - -` | **penalty**, four-plus of their resolved major | a sit (`multi_signoff_pass`) |
| `1NT (2♦) - (2M) - -` | **takeout**, four of the *other* major and ≤2 of theirs | `multi_takeout_answer` |

v7 has one double at the first row (takeout) and nothing authored at the second
at all. The neutral pass's table also carries a natural `2NT` (`hcp 7..=9` with
their major stopped) and competitive `3♣`/`3♦` on a six-card suit.

### The minor transfers, which are two-way

`2NT`→`3♣` and `3♣`→`3♦`, completed unconditionally (doubled or not), then:

- **Pass** — the sign-off. This is the whole point of the floorless rung: a
  0-count with six clubs preempts their unknown major and stops.
- **`3NT`** — the plain six-bagger's choice of games, `points 10+`.
- **the source's two-suiter steps**, game-forcing with a four-card second suit,
  which are *not* next-suit-up: after `3♣` → `3♦` = +♥, `3♥` = +♠, `3♠` = +♦;
  after `3♦` → `3♥` = +♠, `3♠` = +♥. Opener bids the major game on four-card
  support, else `3NT`.

Their pass-or-correct above the completion gets a **guarded sit** from opener —
the transfer promised no values, so opener cannot act — and responder's values,
if any, act again (`3NT` with their now-named major stopped, else pass).

### Residue — the 16+ six-card major has no slam try here

`direct_4m_max` is `15` whenever `notrump.texas_slam_drive` is on (the shipped
default), because uncontested a 17+ six-card major takes South African Texas at
`4♣`/`4♦` and drives its own RKCB. **Under their `(2♦)` those two calls are
Leaping Michaels**, so that route does not exist and the 16+ hand falls back on
the `3♦`/`3♥` transfer, reaching `4M` through `transfer_completion` with its
slam try left to the floor.

That is *not* a K–K regression — the shipped v7 lane routes the identical hand
the identical way, and K–K only adds the exactly-15 direct rung on top. It is
recorded because the fix is one token (`15..=18`, i.e. ignore
`texas_slam_drive` in this lane, where Texas is not available) and it is a
behaviour change, so it wants its own arm rather than riding this one.

### Reading residues

The variant's readings are sound but two of them moved, and both are recorded
in [§N4-KK](one-notrump-competitive.md#known-residues--priced-by-the-ab-not-fixed-in-the-build)
rather than repaired:

- the values `X` publishes `points 8.. ♥ 0..13 ♠ 0..13` where the shipped table
  publishes `♥ 0..4 ♠ 0..4` — deleting the `2NT` relay removed the rung the
  projector negated the five-card majors from. Looser, not false.
- the floorless minor transfers publish a hard six-card suit, which the
  **mirror lane** picks up: when *they* open 1NT and *we* overcall a natural
  `2♦`, their `2NT`/`3♣` decode off this table. That leak is campaign-wide
  ([the measurement discipline section](one-notrump-competitive.md#measurement-discipline))
  and K–K makes it louder, so the isolation gate is load-bearing on this arm.

### What the variant leaves alone

`multi_weak_escape`'s `2M` rung and its whole interfered tail, Leaping Michaels
and its advances, every answer of the double family, and `multi_balance` (a
different seat, so it composes). `multi_stopper_ask` goes **inert** — its `3♠`
is the both-minors call here. Off, or with their `2♦` undeclared or natural, the
knob changes nothing at all.

## See also

- [one-notrump-competitive.md](one-notrump-competitive.md) — the campaign, the
  census, the ledger; §N4/§N4e own this lane's verdicts, §N4-KK the opt-in
  Kokish–Kraft variant.
- [ai-bidder/multi-landy-2d-counter-defense-research.md](ai-bidder/multi-landy-2d-counter-defense-research.md)
  — the six published counter-defense families and their sources.
- [authored-reading-handoff.md](authored-reading-handoff.md) — why a
  reading-only change is a bidding change, and the mirror-read leak that makes
  our `1NT (2♦)` table decode *their* `2NT`.
- [bidding-architecture.md](bidding-architecture.md) — book shadows floor; to
  give the floor a seat, delete the node.
