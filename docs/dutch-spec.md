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

## Deep relay continuations **[2.2]** (transcribed, not yet authored)

Until authored, the instinct floor handles these — a soft misread, measured
not fixed blind.

### 1♣–1♦–1M (opener 11–17, 3+#)

Responder's second call. Mostly natural/non-forcing; note support = 7–9 both
majors, 2NT = 16+ balanced (rightsides), 2OM = artificial both-minor invite.

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

### 1♣–1♦–1NT (opener balanced 18–20)

As the 1NT opening minus Puppet Stayman (no 5-card major here). Reuses the 1NT
transfer machinery.

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

### 1♣–1♦–2♣ (opener 11–17, 5+♣)

Responder's second call; the earlier weak jump lets the major rebids turn
artificial.

| Call | Meaning |
| --- | --- |
| 2♦ | 7–9, 5+♦ |
| 2♥! | 7–9, 4–5♥, 5=♠ |
| 2♠! | 9–11, 4+♣ |
| 2NT | 16+ balanced |
| 3♣ | 7–9, 4+♣ |

### 1♣–1♦–2♦ (opener 21–23, artificial)

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
