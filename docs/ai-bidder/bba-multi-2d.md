# BBA's Multi-Landy 2♦ over 1NT — distilled constraints

Reverse-engineered from the real EPBot engine (system 0, 2/1 GF) by
**sample-and-probe** with `examples/probe-bba-constraints`: deal random actor
hands, drive BBA for a fixed `(seat, auction)`, bucket each hand by the call it
returns, summarise every bucket in DSL vocabulary. Multi-Landy is forced on all
seats (`--conv "Multi-Landy=1"`, `Cappelletti=0`) so BBA both *bids* and
*interprets* the 2♦ as a Multi.

```text
cargo run --release --example probe-bba-constraints -- --mode multi
cargo run --release --example probe-bba-constraints -- --mode advance
cargo run --release --example probe-bba-constraints -- --mode counter --vul none,both
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

## 2. The advancer relay (`advance`, overcaller's partner after 1NT-2♦-P)

| call | freq | hand |
|------|------|------|
| 2♠ | 67% | `hcp 7–18 (median 11)`, no real shape — the catch-all / pass-or-correct-to-spades |
| 2♥ | 33% | `hcp 2–14 (median 6)`, weak — pass-or-correct (overcaller passes with ♥, corrects to ♠) |

This confirms 2♦ resolves to **one unknown major**: advancer's 2♥ is the weak
pass-or-correct and 2♠ the strength-showing/spade-tolerant catch-all. The exact
mechanics are the convention's *offense* and don't bear on our counter-defense.

## 3. BBA's counter-defense (`counter`, 1NT-opener side over the Multi)

Responder's call after `1NT-(2♦ Multi)`. Vulnerability barely moves it (the X
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

## 4. Candidate counter-defense to author (to A/B, default opt-in)

Distilled from §3 + Multi theory, for our responder after `[1NT, (2♦)]` *when we
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

Open question the A/B answers: does this beat pons's *current* `[1NT,(2♦)]`
handling (the Lebensohl/Transfer package in `competition.rs`, which treats 2♦ as
**natural diamonds** — wrong-sided against a hand that actually holds a major)?
Prior: competitive/defensive conventions usually lose on plain-DD (obstruction
wall), but *constructive* responses to interference can win (the 1NT-doubled
runout and UvU both shipped DD-positive). Genuinely open; measured on
`bba-match --defense-2d-multi`.
