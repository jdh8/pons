# The Dutch system — full bidding spec (authoring reference)

Canonical transcription of jdh8's own system book **Watermelon Dutch** (荷蘭
1♣) — the reference the `dutch()` campaign authors against. Source:
[jdh8/watermelon-dutch-doubleton](https://github.com/jdh8/watermelon-dutch-doubleton)
(`src/1C.md`, `src/1C/1D.md`; pull **raw** markdown on `main` — the rendered
page's small-model summaries garble the tables). Campaign plan, decisions, and
measured results live in [dutch-system.md](dutch-system.md); this doc is spec
only.

**Legend.** HCP unless a cell says "pts" (rule-of-N+8). `5=♠` = exactly five;
`4–5♥` = four or five; `6+#` = six-plus in the suit bid; `2OM` = the other
major; `SPL` = splinter; `!` = artificial/conventional. **[status]** tags what
pons has authored:

- **[1]** shipped in Phase 1 · **[2.1]** authored Phase 2.1 (gates/A/B pending)
- **[2.2]** transcribed, not yet authored · **[—]** dropped in pons (with reason)

Where pons deviates from the book it is called out inline (`pons:`).

---

## Openings **[1]**

| Call | Meaning |
| --- | --- |
| **1♣** | 11–23, 1+♣ — the wide catch-all (`pons:` floored `2+♣`, behaviourally identical). Hosts strong balanced hands and every 4-diamond hand but the 4441. Rule-of-20 gated. |
| **1♦** | 5+♦, or the singleton-club 4=4=4=1 — never 3♦. (`pons:` (xx)45 [4♦5♣] and all other 4-diamond hands open 1♣; the book's online "open 1♦ for (xx)45" is stale per jdh8.) |
| **1♥ / 1♠** | 5+ cards, 10–20, Rule of 20 |
| **2♣!** | strong: 21–23 with a 5-card major or 6-card minor, or any 24+ |
| **2♦ / 2♥ / 2♠ / 2NT** | `pons:` still american weak-twos / strong 2NT until Phase 3 (Multi / Muiderberg / UNT) |

---

## Responses to 1♣ **[2.1]**

| Call | Meaning |
| --- | --- |
| P | 0–4, 3–5♣ (content to play 1♣) |
| **1♦!** | relay: 7–9 5=♠ 4–5♥, **or** 0–11 / 16+ with no other suitable call |
| 1♥ / 1♠ | 7+, 4+ (up the line) |
| 1NT | 8–10, no 4-card major |
| 2♣ | invite+, 5+♣, no 4-card major |
| 2♦ | game-forcing, 5+♦, no 4-card major |
| 2♥ / 2♠! | 0–6 pts, weak jump (`pons:` exactly six; 7+ preempts at the 3-level) |
| 2NT | 10–11, no 4-card major |
| 3♣ / 3♦! | 9–11 pts, 6+ (shapely invite) |
| 3♥ / 3♠! | 3–6 pts, 7+ (preemptive) |
| 3NT | 11–15, no 4-card major, to play (`pons:` encoded 12–15 to dedupe 2NT/3NT at 11) |

**Passed hand (P–1♣) [2.2]:** `2♣ / 2♦` = 9–11, 5–6, invite · `3♥ / 3♠` =
9–11, 6+, strong invite.

---

## Opener's rebid after 1♣–1♦ **[2.1]**

The relay's own content: `0–7` any · `7–9` 5=♠ 4–5♥ · `8–11` unbalanced no 4M ·
`16+` 2–3♠ 2–3♥ 3–4♦ 3–4♣.

| Call | Meaning |
| --- | --- |
| 1♥ / 1♠ | 11–17, 3+ (up the line) — the minimum default |
| 2♣ | 11–17, 5+♣ (`pons:` encoded 11–20, so an 18–20 no-4M no-6♣ five-club hand has a rebid) |
| 1NT | 18–20 balanced (2–6♣, 2–4 elsewhere). **A minimum balanced hand may NOT rebid 1NT** — it shows a 3-card major or 5♣. |
| 2♥ / 2♠ | 18–20, 4+ (reverse) |
| 3♣ | 18–20, 6+♣ |
| **2♦!** | 21–23, no specific shape (2–5♣, 1–4 elsewhere) — the artificial catch-all; diamond reversals abandoned |
| **2NT!** | 21–23, 5+♦ 5+♣ — **[—]** dropped in pons: 5-5 minors open 1♦, never reach 1♣–1♦ |

---

## Opener's rebid after 2♣ / 2♦ **[2.2 authored]**

`pons:` original (no source-system table) — overwrites american's inverted-raise
(`2♣`) and weak-jump-shift (`2♦`) continuations, which misread the Dutch meanings.
Since 1♣ denies a 5-card major and responder's `2♣`/`2♦` deny a 4-card major, **no
major fit exists** — the pure inverted-minors world (minor / notrump / slam).
Opener borrows american's `after_inv_raise` ladder.

**After 2♦ (game force, 5+♦) — forcing:**

| Call | Meaning |
| --- | --- |
| 3♦ | 4-card diamond support — the known nine-card fit (1♣ hosts most 4♦ hands) |
| 3♣ | a real 5+ club second suit (no diamond fit) |
| 3NT | balanced extras (15+), both majors stopped, to play |
| 2♥ / 2♠ | a single major stopper, up the line toward 3NT (both-stopped → notrump) |
| 2NT | catch-all — minimum / stopper-shy (never Pass) |

**After 2♣ (invite+, 5+♣) — non-forcing:**

| Call | Meaning |
| --- | --- |
| 3NT | accept — balanced max stopped, or a 17+ maximum forcing (28+ opposite the invite) |
| 3♣ | decline — club support, non-forcing (capped ≤16 so a max never leaves it in) |
| 2NT | decline — balanced minimum, non-forcing / catch-all |

**Responder's continuation (authored — the redo).** The opener-only first cut
left responder to the floor; the A/B refuted it (the floor **dropped the game
force**, passing opener's forcing `3♣` over `2♦`, and **blasted slam** blind over
opener's `3NT`/stopper-shows, `−1.38 IMPs/fired`). Responder is now authored to
honour the force and cap at the right game:

| After opener's | Responder |
| --- | --- |
| `2♦`→ `3♦`/`3♣`/`2♥`/`2♠`/`2NT` | `3NT` — name the game (never pass the force) |
| `2♦`→ `3NT` (15+ balanced, to play) | Pass |
| `2♣`→ `3NT` (accept) | Pass |
| `2♣`→ `3♣`/`2NT` (non-forcing decline) | `3NT` with the GF end (12+), else Pass |

Slam beyond game is deferred: the A/B's dominant loss was blind slam *blasts*,
not missed keycard slams, so the first correct cut lands the game cleanly. The
`3♦` diamond-fit branch is the natural home for a later RKCB reuse (widening
`american::slam::install_rkcb` past `pub(super)`) — pending a re-A/B that shows
the game cap leaking slams. The help-suit game try (`2♥`/`2♠` after `2♣`) stays
dropped — a cheap accept/decline lands the same games without the extra read.

---

## Deep relay continuations

Until authored, the instinct floor handles these — a soft misread, measured
not fixed blind.

### 1♣–1♦–1M (opener 11–17, 3+#) **[2.2 authored]**

Responder's second call. Mostly natural/non-forcing; note support = 7–9 both
majors, 2NT = 16+ balanced (rightsides), 2OM = artificial both-minor invite.

`pons:` `2M!` is gated **exactly** on Reverse Flannery (5=♠, 4–5♥, 7–9) — an
ordinary invitational raiser never reaches here (it raised or bid on round one),
so no natural-raise row is authored; such hands fall to the floor. `2NT` (16+)
is alerted so projection discloses the slam strength (american would read it as
an invite). `1♠`/`1NT`/`2♣`/`2♦`/`3♣` are natural; a weak fit passes.

| Call | Meaning |
| --- | --- |
| 1♠ | 0–6, 4+♠ (only after 1♥) |
| 1NT | natural, usually 5–7 |
| 2♣ | 0–9, 5+♣ |
| 2♦ | 5–9, 6+♦ |
| 2M! | 7–9, 5=♠, 4–5♥ (support with the two-suiter) |
| 2OM! | 9–11, both minors 5+/4+, invite |
| 2NT | 16+ balanced |
| 3♣ | 6–9, 6+♣ |

### 1♣–1♦–1NT (opener balanced 18–20) **[2.2 deferred]**

As the 1NT opening minus Puppet Stayman (no 5-card major here). Reuses the 1NT
transfer machinery. `pons:` deferred — opener's `1NT` is rare (18–20) and its
strength self-discloses to the floor via projection; author after measuring the
minimum-rebid continuations.

| Call | Meaning |
| --- | --- |
| 2♣! | (Garbage) Stayman — asks a 4-card major |
| 2♦! | transfer, 5+♥ |
| 2♥! | transfer, 5+♠ |
| 2♠! | any 6+♣, or invite to 3NT |
| 2NT! | transfer to diamonds — 5+♦ 4+♣, or 6+♦ |
| 3♣! | natural invite, 6+♣ |
| 3♦! | invite+, 5+♠ 5+♥ |
| 3♥! | SPL — 0–1♥, 0–3♠, 4–6♦, 4–6♣ |
| 3♠! | SPL — 0–1♠, 0–3♥, 4–6♦, 4–6♣ |
| 3NT | to play |
| 4♣! | South African transfer, 6+♥ |
| 4♦! | South African transfer, 6+♠ |
| 4♥ / 4♠ | to play |
| 4♠! | weaker quantitative, to 6NT or 7NT |
| 4NT | stronger quantitative, to 6NT |
| 5NT | stronger quantitative, to 7NT |

### 1♣–1♦–2♣ (opener 11–17, 5+♣) **[2.2 authored]**

Responder's second call; the earlier weak jump lets the major rebids turn
artificial. `pons:` `2♥!` = the Reverse Flannery two-suiter; club support is
inverted — `2♠!` invitational (9–11), natural `3♣` minimum (7–9). `2NT` (16+)
alerted as above.

| Call | Meaning |
| --- | --- |
| 2♦ | 7–9, 5+♦ |
| 2♥! | 7–9, 4–5♥, 5=♠ |
| 2♠! | 9–11, 4+♣ |
| 2NT | 16+ balanced |
| 3♣ | 7–9, 4+♣ |

### 1♣–1♦–2♦ (opener 21–23, artificial) **[2.2 deferred]**

`pons:` deferred — same reasoning as `1♣–1♦–1NT` (rare, self-disclosing).

Opener has shown 21–23 with no specific shape — the top of the 1♣ range. If
responder has no game interest and 1♣ rates to be a fine spot, they should have
passed 1♣ originally (e.g. 0–3 HCP 3=3=2=5). A full transfer structure.

| Call | Meaning |
| --- | --- |
| P | to play, 4+♦ |
| 2♥ | non-forcing, 4+♥ |
| 2♠ | non-forcing, 4+♠ |
| 2NT! | either minor, 6+ |
| 3♣! | Stayman — asks a 4-card major |
| 3♦! | transfer, 5+♥ |
| 3♥! | transfer, 5+♠ |
| 3♠! | forced transfer to 3NT |
| 3NT! | 5–7, 2–3♠, 2–3♥, 4–5♦, 4–5♣ |
| 4♣! | both-minor slam try, usually 5+♦ 5+♣ · `4♣–4♦` = ♦ trump · `4♣–4♥+` = ♣ trump · `4♣–5♣♦ / 6♣♦` = to play |
| 4♦! | transfer, usually 7+♥ |
| 4♥! | transfer, usually 7+♠ |

---

## Not yet transcribed

The book's `1♣–1M` chapter (`src/1C/1M.md`) and the 2-level openings
(Multi/Muiderberg/UNT, Phase 3) are not pulled here yet — fetch raw from the
repo when those phases come up.
