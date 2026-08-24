# BBA's Multi-Landy 2♦ over 1NT — distilled constraints

Reverse-engineered from the real EPBot engine (system 0, 2/1 GF) by
**sample-and-probe** with `examples/probe-bba-constraints`: deal random actor
hands, drive BBA for a fixed `(seat, auction)`, bucket each hand by the call it
returns, summarise every bucket in DSL vocabulary. Multi-Landy is forced on all
seats (`--conv "Multi-Landy=1"`, `Cappelletti=0`) so BBA both *bids* and
*interprets* the 2♦ as a Multi.

Published human counter-defenses to this exact `1NT (2♦)` object, including
values/penalty, replacement-Stayman, and transfer families, are compared in
[multi-landy-2d-counter-defense-research.md](multi-landy-2d-counter-defense-research.md).

```text
cargo run --release --example probe-bba-constraints -- --mode multi
cargo run --release --example probe-bba-constraints -- --mode advance
cargo run --release --example probe-bba-constraints -- --mode counter --vul none,both
cargo run --release --example probe-bba-constraints -- --mode counter-d-x2h --vul none,both   # the doubler's rebid (§3a)
cargo run --release --example probe-bba-constraints -- --mode custom --seat 0 --calls "1NT 2♦ X 2♥ - - X -" --filter-call 1NT --meanings 12
```

Every `sketch:` is a **candidate** to verify and hand-author, not a proof of
BBA's internals. Caveat that recurs below: the DSL renders each suit's length
band *independently*, so it cannot express a single-major **disjunction**
("6 ♠ **or** 6 ♥"); read those buckets with the relay (`advance`) that resolves
the major.

## 1. The 2♦ Multi itself (`multi`, overcaller seat over 1NT)

BBA's full direct-seat structure over (1NT), by frequency:

| call | freq | what it is |
|------|------|-----------|
| Pass | 82% | no overcall |
| **2♦** | **5.2%** | **the Multi — a single-suited major** (see below) |
| X | 2.8% | values/penalty, ~12–19, off-shape |
| 2♣ | 2.8% | **both majors** (the "Landy" half), 4+/4+, ~9–19 |
| 2♠ | 2.6% | natural, **5 spades**, ~9–21 |
| 2♥ | 2.6% | natural, **5 hearts**, ~9–19 |

The **2♦ Multi** bucket: `hcp 9–18 (median 13)`, and exactly one major is long —
♠ band `1–7 (median 6)`, ♥ band `1–6 (median 3)` — i.e. a **6-card major**, not
the 5-card holding that takes a natural 2♥/2♠. So BBA's compiled Multi is an
**intermediate-to-good single-suited major** (≈12–15, 6+ cards), *not* a classic
weak two. The bucket leans spades in the sample but the heart tail (up to 6) and
the relay below confirm it is genuinely either major.

## 2. The advancer relay (`advance`, overcaller's partner after 1NT - 2♦ -)

| call | freq | hand |
|------|------|------|
| 2♠ | 67% | `hcp 7–18 (median 11)`, no real shape — the catch-all / pass-or-correct-to-spades |
| 2♥ | 33% | `hcp 2–14 (median 6)`, weak — pass-or-correct (overcaller passes with ♥, corrects to ♠) |

This confirms 2♦ resolves to **one unknown major**: advancer's 2♥ is the weak
pass-or-correct and 2♠ the strength-showing/spade-tolerant catch-all — a split
by **strength**, not the textbook "bid the shorter major". The correction
mechanics ([bba-1nt-defense.md](bba-1nt-defense.md)): over 2♥ the overcaller
passes with hearts, corrects to 2♠ with spades, jumps 3M with a seven-carder;
over 2♠ it bids **2NT as a heart relay** (never 3♥) and the advancer places.

### Over our double (`advance-x`, `rebid-d-x2h` — added 2026-08-15 for N4)

`--mode advance-x` probes the advancer at `1NT 2♦ X`, with our side declared
natural (`--their-conv Multi-Landy=0` etc., how `bba-gen` models us; BBA
labels our X "negative double"): **2♠ 66.9% / 2♥ 33.0% / pass 0.0%** — the
undoubled relay verbatim. **BBA's advancer never sits for our double**; the
"they sit 43%" once recorded for N4b was the foreign lane (see
docs/one-notrump-competitive.md §N4). `--mode rebid-d-x2h` (overcaller at
`1NT 2♦ X 2♥ -`): pass 36.7% (6 hearts), 2♠ 49.8% (6 spades), 3♠ 7.7% /
3♥ 5.8% (7+). `--mode rebid-d-x` (overcaller after `X - -`) exists but the
node is unreachable against BBA.

## 3. BBA's counter-defense (`counter`, 1NT-opener side over the Multi)

Responder's call after `1NT (2♦ Multi)`. Vulnerability barely moves it (the X
floor is identical NV vs both-vul); both-vul only adds rare slam tries.

| call | freq | distilled | reading |
|------|------|-----------|---------|
| **X** | **41%** | `hcp 5–17 (median 9)`, balanced 60%, suits ~2–4 | **the backbone — values / takeout** of the unknown major |
| Pass | 15% | `hcp 1–9 (median 5)`, balanced | too weak to act |
| 3NT | 13% | `hcp 10–16 (median 12)` | **to play** game (stopper implied) |
| 2NT | 10% | `hcp 9–19 (median 13)`, **balanced 82%**, minors longer | balanced invite / Lebensohl-ish relay |
| 2♠ / 2♥ | ~3% / ~2% | `hcp 3–11`, **5–6 card** major | natural, weak, to play |
| 3♣/3♦/3♥/3♠ | ~2% each | `hcp 5–12`, **5–6 card** suit | natural, constructive single-suiter |
| 4♠ / 4♥ | ~3% / ~2% | `hcp 7–15`, **6–7 card** major | long major straight to game |

**Shape of the counter-defense:** *double = values* is the workhorse (41%,
vul-insensitive, broad and balanced-leaning ⇒ takeout/competitive, not pure
penalty), everything else **natural** — new suits to play / constructive, 2NT
the balanced invitational zone, 3NT to play, 4M the long-major shot, Pass the
junk. This is the standard expert answer to a Multi: **X = values, naturals
everywhere else.**

### 3a. The double's second turn — the seats BBA's advancer actually gives us (added 2026-08-15, N4 v6)

`opener-d-x` (§5) is the `X -` node, which BBA's advancer never produces
(`advance-x`: pass 0.0%). The reachable seats, probed with the new modes
`opener-d-x2h` / `opener-d-x2s` (opener over `X (2M)`), `counter-d-x2h` /
`counter-d-x2h2s` / `counter-d-x2s` (the doubler's rebid, hands filtered to
BBA's own first-turn double), and `--mode custom --seat N --calls "…"
--filter-call X --filter-prefix "1NT 2♦"` for one-off nodes:

**Opener over the pass-or-correct is passive.** `1NT (2♦) X (2♥) ?`: Pass
92.3%, `2NT` 6.0% (`hcp 17`, balanced), `2♠` 1.7% (five spades, 16–17);
over `X (2♠)` Pass 93.0% / `2NT` 7.0%. It never doubles `2M`. The double's
work is done by the doubler:

| after `X (2♥) - (-)` | share | band | reading |
| --- | ---: | --- | --- |
| `3NT` | 29.5% | `hcp 9–15` (med 12), hearts 3–4, spades 2–4 | to play — **no stopper gate**, the length in their suit is what correlates |
| Pass | 26.8% | `hcp 5–9` (med 6) | — |
| **`X`** | 12.6% | `hcp 6–17`, **spades 4–4, hearts 1–2** | BBA's label: *reopening double* — takeout of hearts showing spades |
| `2NT` | 8.0% | `hcp 8–9`, hearts 4–5 | natural invite |
| `4NT` | 7.6% | `hcp 16–21` | quantitative |
| `2♠` | 5.9% | five spades, `hcp 6–8` | to play |
| `3♠` | 5.2% | four spades, hearts 2–3, `hcp 9–13` | a spade game try |
| `3♦` / `3♣` | 1.6% / 1.5% | 5–6 cards, `hcp 7–13` (med 8) | natural |

After `X (2♠) - (-)`: Pass 34.4%, `3NT` 28.9% (9–15), `X` 13.4% (**hearts
4–5, spades 1–2** — the mirror takeout), `2NT` 8.8% (8–9), `4NT` 7.4%, `3♣`
2.9%, `3♦` 2.8%; no `3♥`/`2♥` analogue above 1%. After `X (2♥) - (2♠)` (the
weak pass-or-correct corrected to spades): **`X` 32.6%** (`hcp 5–16`, spades
3–5 med 4, hearts 2–4 — **penalty**, not takeout), `3NT` 23.6% (9–15), Pass
21.4% (5–10), `2NT` 7.2% (8–10), `4NT` 6.1%, `3♦`/`3♣` 4%/3.6%. Vulnerability
moves nothing beyond a point of pass/2NT.

**Opener over the reopening double** (`1NT (2♦) X (2♥) - (-) X -`, `custom`,
`--meanings`) is opaque: `2NT` 33.7% (balanced, spades 2–5 med 4), `3♣` 26.3%
(4+), `3♦` 23.5% (4+), Pass 16.4% (hearts 3–5 med 4 — the penalty pass),
`3NT` 5.8% vul with 16–17; **never `2♠`**, even though its partner's double
showed exactly four. Over the penalty double after they ran to spades
(`X (2♥) - (2♠) X -`) opener passes 100%.

The overcaller facing our double (`rebid-d-x2h`, §2) is unchanged: pass with
six hearts, `2♠` with six spades, `3M` with seven.

## 4. Candidate counter-defense to author (to A/B, default opt-in)

Distilled from §3 + Multi theory, for our responder after `1NT (2♦)` *when we
treat their 2♦ as a Multi* (faithful for the A/B vs BBA, whose 2♦ is always a
Multi). Tighten BBA's loose floors slightly for DD penalty discipline:

- **X** — values, takeout of the unknown major. `points(7..)` (BBA floors near 5;
  7 is cleaner for doubled-contract discipline). The dominant action.
- **2♥ / 2♠** — natural, ~5+ card major, weak–competitive, to play. `len(M,5..)`.
- **2NT** — natural invitational, balanced ~11–12 (`balanced() & points(11..=12)`);
  Lebensohl relay is the alternative if the natural invite underperforms.
- **3♣/3♦/3♥/3♠** — natural, constructive 5+ suit, ~`points(9..)` forcing-ish.
- **3NT** — to play, `points(13..)` with a stopper in the majors.
- **4♥/4♠** — long (6+) major to game.
- **Pass** — everything else (weak), handled by the floor.

**Superseded 2026-08-15 by N4 (docs/one-notrump-competitive.md §N4)**, which
kept the shipped Transfer-Lebensohl `(2♦)` leg's constructive calls (Stayman +
Smolen, Jacoby transfers, Leaping Michaels) and re-keyed only the diamond-keyed
gates: `X` = values `hcp(8..)` as above, `3NT` = both majors stopped, the `2NT`
relay gains a natural `3♦`, opener sits over their pass of the double and
doubles the pass-or-correct `2M` with four trumps. This natural sketch — 3x
natural single-suiters, natural `2NT` — was **not run** as an arm; it stays
here as the recorded alternative. Engagement is `their.two_diamonds_multi`
(`TheirDisclosures`), `bba-gen --their-2d-multi`. **Shipped 2026-08-15 as v7**:
responder's second turn is §3a's table minus its PD-refused game bids —
the takeout X of the resolved major, `3NT` with a stopper, `4NT`, the weak
`2♠` — and the first-turn double is BBA's `hcp 6+`; `bba-gen` now derives
the declaration from the census (`their_2d_multi`), engine default still
undeclared.

## 5. The rest of the set

§3 is one lane of BBA's counter-defense. The other five —
`(X)`, `(2♣)`, `(2♥)`, `(2♠)`, `(2NT)`, the three-level preempts, and opener's
answer one round deeper in each — are in
[bba-1nt-counter-defense.md](bba-1nt-counter-defense.md), which reproduces §3's
table unchanged. Two facts from there bear on this lane: opener **never passes**
the 41% double (`2♥`/`2♠` with a four-card major, else `3♦`), and this lane's
counter moves only **7.9%** between the declared and undeclared readings of the
`2♦`, against 59.4% for the Landy `2♣` — so a Multi package here is buying much
less reading than N1 did.
