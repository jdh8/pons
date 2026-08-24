# BBA's Multi 2♦ **opening** — the book, walked

BBA's shipped 2/1 card does **not** open Multi: its `2♦` is `Weak natural 2D`
(convention id 168, 4-10 with 6-7♦ — [bba-book.md §5.1](bba-book.md)).  Multi is
id **110**, a member of the same mutually-exclusive 2♦-opening radio group, and
turning it on is one flag away.  This document is that engine's book for the
Multi opening: what responder's calls mean, what the defenders' calls mean, and
what survives interference.

It is the reference [dutch-system.md](../dutch-system.md) Phase 3 needs — the
decision of 2026-07-20 adopts BBA's Multi 2♦ verbatim, so that these rows and
the WJ teacher net share a system.  Not to be confused with
[bba-multi-2d.md](bba-multi-2d.md), which is the Multi **2♦ overcall of 1NT**
(Woolsey Multi-Landy) — a different call with a different meaning (that one is
an intermediate 12-15; this one is weak).

Independent defenses to this opening — the ACBL split-takeout and Dixon
families, expert transfer variants, and their competitive tails — are surveyed
in [multi-2d-opening-defense-research.md](multi-2d-opening-defense-research.md).

## How it was read

```sh
# the tree (342 nodes, exhaustive to length 4, then the ceilings)
cargo run --release --features serde --example probe-bba-book -- \
    --prefix "2♦" --conv "Multi=1" --reach-depth 4 --vuls none --output multi.jsonl
cargo run --release --features serde --example probe-bba-book -- \
    --render <dir> --prefix "2♦"

# one lane, straight to stdout
cargo run --release --features serde --example probe-bba-book -- \
    --prefix "2♦ - 2NT -" --conv "Multi=1" --reach-depth 5 --vuls none
```

`--conv "Multi=1"` writes id 110 on all four seats over `cards/American.bbsa`;
the radio group means that one write clears 168, so **both sides** bid and
interpret the `2♦` as a Multi.  Everything else on the card stays as it is.

Three caveats carry over from [bba-book.md](bba-book.md), and they bound every
table below:

1. **These are the book's rules, not the floor's choices.**  A call labelled
   `calculated bid` means no rule matched and the bilans floor picked it; those
   children were not expanded here (no Multi-on self-play corpus exists).
2. **The DSL cannot express a disjunction.**  The opening renders as `♥ 0-6 /
   ♠ 0-6` because "6 hearts **or** 6 spades" has no form in it.  The 2NT relay
   resolves the major; read the opening through it.
3. **No vulnerability split.**  The opening and every direct-seat defence were
   re-read under all four vulnerabilities (`--vuls none,ns,ew,both`) and
   collapse to one reading each.  The tree below was walked at `none`.

## 1. The opening

| | |
| --- | --- |
| `2♦` | **`Multi`, alerted, 4-10 HCP, one 6-card major** |

Weak-only — there is no strong variant, at any vulnerability.  This matches the
independent BBA-WJ harvest (n=9166, declared `pts [4,10]`, observed HCP 1-10,
always a 6+ major) recorded in [dutch-system.md](../dutch-system.md) (§ *Measured facts about
BBA-WJ*).

## 2. Responses (`2♦ -`)

| call | reading |
| --- | --- |
| `2♥` | **pass-or-correct**, alerted, ≤17 — no point floor |
| `2♠` | pass-or-correct, 12-17 (♥ 2+, ♠ 1+) |
| `2NT` | **artificial ask**, alerted, 16+ |
| `3♣` | natural, 7+♣, 10-14 |
| `3♦` | **artificial**, alerted, 10+, both majors 2+ — the three-level try |
| `3♥` / `3♠` | natural, 7+, 10-14, to play |
| `4♣` | artificial, alerted, 15+ |
| `4♦` | artificial, alerted, ≤14 — pass-or-correct to 4M |
| `4♥` / `4♠` | natural, 7+, 13-30 |
| `P` | `minimum`, **6+♦** |

Opener over `2♥`: **`P` = 6♥** · **`2♠` = 6♠** (4-10) · **`3♥` / `3♠` = the same
six cards with a maximum** (10 exactly).  Opener's other calls there fall on
generic templates (`NT style`, `bidable suit` at 4-10) — the book has no rule
for them, so they are floor territory.

Opener over `2♠` is **entirely** floor territory: a re-walk at
`--prefix "2♦ - 2♠ -"` returns only the escape template (`6+ <suit>` at every
rung, with `2NT` reading `6+ ♥` at 9-10) plus `P` = six spades at 4-10.  There is
no book rule for the minimum heart correction, so a port authors that node
itself.

**The `2NT` ask — the cheapest step is the *maximum*:**

| answer | reading |
| --- | --- |
| `3♣` | 6 hearts, **8-10** |
| `3♦` | 6 spades, **8-10** |
| `3♥` | 6 hearts, **4-7** |
| `3♠` | 6 spades, **4-7** |
| `4♣` / `4♦` | `slam try`, 10 |
| `4♥` / `4♠` | natural, 10 |

Worth noting for anyone porting this: many European Multi cards answer
min-first.  BBA answers **max-first**, so `2NT - 3♥` is the weak hand.  (The
split re-reads as `3♣`/`3♦` = 8-10 and `3♥`/`3♠` = 4-7 at `--reach-depth 5`.)

The **asker's** continuation after an answer is floor territory too: at
`--reach-depth 8`, `2♦ - 2NT - 3♣ -` and `2♦ - 2NT - 3♥ -` return `bidable suit`
at every rung and no raise of the named major at all, so nothing there is a
rule.

`3♦` (10+, both majors 2+): opener answers `3♥` / `3♠` natural 5-10, jumping to
`4♥` / `4♠` with 10.

`4♣` (15+): opener answers **`4♦` = hearts, `4♥` = spades, both alerted** — a
transfer, so the 15+ hand declares.  `4♦` (≤14) is the plain pass-or-correct:
opener bids `4♥` / `4♠` unalerted.

## 3. Defence — the natural-2♦ table with the diamond hooks voided

| call | over their **Multi** 2♦ | over their **natural** 2♦ (same engine, Multi off) |
| --- | --- | --- |
| `X` | takeout, 12+, **3+ in every suit, ♦ included** | takeout, **♦ ≤4** — denies diamonds |
| `3♦` | natural, 12+, 5+♦ | **Michaels cue bid** (5-5 majors) |
| `4♣` / `4♦` | natural | **Leaping Michaels** |
| `3NT` | 22+ `balanced` | 22+ **with a ♦ stopper** |

The rest: `2♥` / `2♠` natural 5+, 12-17 · `2NT` 15-17 NT style · `3♣` natural
12+ · `3♥` / `3♠` natural 6+, 15+ · `4NT` `Unusual 4NT` 16+ · `P` ≤16.

So **BBA has no Multi-specific defence at all**.  Every 2♦-aware tool — the cue,
the two-suiter jumps, the stopper-showing 3NT — is keyed to a real diamond suit
and silently evaporates when the 2♦ is artificial, leaving the side with *no
cue-bid* over a Multi and a takeout double that asks for a suit the opener
cannot have.  Advancing the double (`2♦ (X) -`) reuses BBA's generic
**Rubensohl-after-double** ladder: `2♥` / `2♠` natural, `2NT` and up Rubensohl.

## 4. Competitive

**Over their `X`** (`2♦ (X)`) — responder's table is *unchanged* (same `2♥`
pass-or-correct, `2♠`, `2NT` ask, `3♦`), with one addition: **`XX` is the same
16+ artificial ask as `2NT`**.

When the auction comes back to opener after a redouble or a second double, the
rebids switch to the escape table: every call reads `6+ <suit>`, i.e. names a
real six-card holding.

**Over their suit overcall** — responder resolves the Multi to *the other
major*, symmetrically, on the inference that the overcaller bid the one they
hold:

| | after `(2♥)` | after `(2♠)` |
| --- | --- | --- |
| support | `2♠` 8+ (♠ 2+) · `2NT` 14+ | **`2NT` 14+ (♥ 2+) only** |
| limit raise or better | `3♥` in ♠, 17+, alerted | `3♠` in ♥, 17+, alerted |
| penalty | `X` 14+, ≤1♠ | `X` 14+, ≤1♥ |
| natural | `3♠` 11-24 6+♠ · `3NT` 20-26 ♥stop | `3♥` 20-24 · `3NT` 20-26 ♠stop |
| artificial | `4♣` 15+ | — |
| `P` | ≤20 | ≤20 |

**The `(2♠)` column is a hole.**  The pass-or-correct rung is gone — `2♥` is no
longer available — so support starts at `2NT` and needs 14+, and a weak
responder who would have played 2♥ opposite hearts has nothing: `3♥` reads
20-24 natural.  The book's answer is `P` with ≤20, leaving the opponents in 2♠
whenever responder is weak, which is most of the time.

One qualification the table above hides, because the walk marks it a ceiling
dead end rather than a rung: the `4♦` pass-or-correct **is** live in both
columns, at `≤14` with `♥ 3+` **and** `♠ 3+`.  So the hole is not "every weak
responder"; it is every weak responder without three cards in *both* majors —
still the large majority, and still a rung-less one.

Opener's own competitive rebids (`2♦ (2♥) 2♠ (3♣)` and friends) are mostly
`artificial` cues and `calculated bid` — the floor, not the book.

## 5. What this means for us

We have a Multi 2♦ **opening** as of 2026-08-24 (Dutch Phase 3, default off, two
variants — see below) but still **no defence to theirs**.  `defense_to_weak_two` in
[src/bidding/american/defense/weak_two_defense.rs](../../src/bidding/american/defense/weak_two_defense.rs)
derives its overcall levels from `their_opening`'s suit and treats a `2♦`
opening as natural diamonds; no anchor opponent opens Multi, so that path has
never been exercised against one.

Two rows for Phase 3's ledger when it authors these:

1. **Answer direction is a decision, not a copy.**  BBA's max-first `2NT`
   answers are shared with the WJ teacher net, so matching them keeps book and
   teacher on the same rows — that is the argument for copying, and it outranks
   the style preference.
2. **The `(2♠)` hole is inherited if we copy verbatim.**  Authoring a weak
   pass-or-correct rung there (`2NT` as the weak relay, or `X` as pass-or-correct)
   diverges from the teacher, so it is an A/B, not a free repair.

**Both rows are now settled, 2026-08-24.**  Row 1: the port answers max-first,
in both variants — the champion page answers max-first too, so there was no
direction conflict to trade off.  Row 2: the hole is inherited in both variants
and pinned by a test; the repair stays a separate A/B.  The rows live in
[src/bidding/dutch/multi.rs](../../src/bidding/dutch/multi.rs) behind
`opening.multi_two_diamonds`, with a second variant behind
`opening.multi_two_diamonds_champion`; the tables and the ledger are in
[dutch-system.md](../dutch-system.md) §*Phase 3*.

A third row the walk did not anticipate: **BBA states bands but no precedence**,
and several of its responder bands overlap (`2NT` 16+ against `4♣` 15+, `2♠`
12-17 against `3♦` 10+, `3♦` against `4♦`).  The port's weight order is therefore
its own, not a copy, and is documented as such at the rule site.  Together with
the two floor-territory nodes above, that is everything in the lane the phrase
"copied verbatim" does **not** cover.
