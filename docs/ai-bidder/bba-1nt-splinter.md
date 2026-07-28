# BBA's `1NT–3♥/3♠` splinter

What BBA/EPBot's `1N-3M splinter` toggle actually plays, read off the live engine
with [`examples/probe-bba-constraints`](../../examples/probe-bba-constraints/main.rs)
(`--mode nt-resp|nt-3h|nt-3s`), 40 000 random responder hands and 20 000 opener
hands, `--conv "1N-3M splinter"=1`. Bands are the 5th–95th percentile of the
sampled bucket unless the run is marked untrimmed. When this was written we left
the slot unauthored and read the call as a natural forcing 5+ suit; since
2026-07-28 we author a *different* convention in the same slot — see
[Ours](#ours-the-bridge-world-standard--polish-club-form) below.

## Default: off, in the engine

| source | `1N-3D splinter` | `1N-3M splinter` |
| --- | --- | --- |
| **engine default, system 0** (`get_conventions`) | 0 | **0** |
| `vendor/bba/21GF.bbsa` card | 0 | 1 |
| `vendor/ben/BEN-21GF.bbsa` | 1 | 1 |
| `cards/American.bbsa` (ours) | 0 | 0 |

The card and the engine **disagree**: stock 21GF *claims* the treatment, the
compiled default does not play it. Since the `.so` ignores the cards, every
anchor run to date has faced a BBA that does **not** splinter over its own 1NT —
the toggle only bites when replayed through `epbot_set_conventions` (`--our-card`
/ `--conv`). BEN, which does load its card, plays it.

Control: with `--conv "1N-3M splinter"=0` the `3♥` and `3♠` buckets vanish
entirely from the response ladder (0 of 40 000). The toggle owns the slot.

## What the call shows

Untrimmed over 40 000 hands, seed 7 — the bounds below are hard min/max, not
percentiles:

| | `1NT–3♥` (n=341, 0.9%) | `1NT–3♠` (n=377, 0.9%) |
| --- | --- | --- |
| bid major | **0–1** (med 1) | **0–1** (med 1) |
| other major | **exactly 4** | **exactly 4** |
| clubs | 3–5 (med 4) | 3–5 (med 4) |
| diamonds | 3–5 (med 4) | 3–5 (med 4) |
| HCP | 9–24 (med 11) | 9–20 (med 12) |
| balanced | 0% | 0% |

So the shape is pinned much harder than "short major, minor-oriented": it is

> **shortness in the bid major (0 or 1), *exactly four* cards in the other
> major, and the rest in the minors** — i.e. 4-1-4-4 / 4-1-5-3 / 4-0-5-4 and
> mirrors, from **9 HCP** up with no ceiling.

The minors are 4-4, 5-3 or 5-4; a 5+ minor is *not* required, and 5-5 in the
minors never appears (the other major eats four cards). The defining feature is
the **exactly-four other major** — the hand that cannot Stayman-and-stop and has
no five-card suit to transfer into. A hand with five of the other major
transfers instead, which is why the length is capped, not merely floored.

Note the 9-HCP floor: BBA is counting shape, and it forces to game (below).
The `--trim 0.05` run gives 9–17 med 12, so the mass is invitational-strength
points with game-forcing playing strength.

## What opener does over it

Opener at `[1NT, P, 3♥, P]` and `[1NT, P, 3♠, P]`, filtered to hands BBA actually
opens 1NT (n≈1000 each, every bucket 15–17 HCP, ≈100% balanced):

| opener's call | over `3♥` | over `3♠` |
| --- | --- | --- |
| `3NT` | 45.8% | 52.1% |
| **4 of the other major** | `4♠` **31.5%** (spades 4–5) | `4♥` **34.0%** (hearts 4–5) |
| `3♠` (below game) | 7.7% | — |
| `4♦` | 7.1% (diamonds 5) | 5.6% |
| `4♣` | 6.3% | 6.9% |
| `4NT` | 1.7% | 1.4% |

Two things fall out:

- **Opener never bids four of the bid major** — 0 of ~1000 in each run. The call
  is not natural to BBA, confirming the splinter reading from the other side.
- **Opener never passes and never bids below 3NT except `3♠` over `3♥`** (a
  cheap 4-card-spade probe, still forcing). The auction is **game forcing**.
- The 31–34% `4M` bucket is opener holding 4+ of the *other* major: the 4-4 fit
  the splinter was looking for. `4♣`/`4♦` are the minor-fit / slam-try lanes.

## Reading of the treatment

It is the standard "both minors and a 4-card major, short in the other major"
game force, occupying the two slots we leave empty. Frequency ≈0.9% each, so
1.8% of responses to 1NT — a thin but real slot, and one where our reader would
currently take the call as natural hearts/spades if an opponent's system (or
BEN's) produced it. See [ben-gap-campaign.md](../ben-gap-campaign.md) for where
this sits in the BEN gap ledger.

Not measured here: whether adopting it wins. That is an
`author-convention` + `measure-ab` job; this document is the read, not the case.

## Ours: the Bridge World Standard / Polish Club form

We author the *other* convention in this family. `set_nt_splinter` (**shipped
default-on**, 2026-07-28) fills both slots with shortness in the bid major,
**2–3** in the other, **exactly four** diamonds and **five or six** clubs.

### Why not BBA's

BBA/EPBot plays the **GIB** form. GIB's own system notes word it

> Singleton or void in the suit bid, at least 4 cards in the other 3 suits, no
> 5-card major, forcing to game.

which derives the measured shape above exactly: "≥4 in the other three suits"
floors the other major at four and "no 5-card major" caps it there. But a
four-card major opposite our 1NT is what Stayman is *for* — BBA's splinter
competes with its own `2♣`. The Bridge World form does not:

| authority | `1NT–3M` |
| --- | --- |
| **BWS 2017** IV.F(e) | "three of another suit = both minors strong … three of a major = **at most one card in the suit bid**" |
| **BWS 2001** poll 814c | "both minors with shortness in the bid suit" — **(37, 29)**, the expert plurality (814a natural-and-forcing: 21, 25) |
| Polish Club 2020 *Expert* | usually **(31)(54)** |
| [loebbridge](https://loebbridge.com/index.php/bridge-articles/splinter-response-to-1nt) | "3♥ jump: 3 spades, 1 heart, and 5-4 or 4-5 in the minors" |
| **GIB / BBA** | 0–1 bid major, **exactly 4** other major, rest minors |

### The shape is closed, not floored

Every neighbouring slot already owns its half of the residue, so pinning each
axis costs nothing and buys opener an exact 13-card read:

| axis | value | what would otherwise claim it |
| --- | --- | --- |
| bid major | void or **low** singleton | a stiff A/K un-wastes opener's honors → `3NT` |
| other major | 2–3 | four is Stayman, five transfers |
| diamonds | exactly 4 | `♦5+ & ♣4+` is the `2NT` transfer |
| clubs | 5–6 | `♣7+` keeps the `2♠` transfer |

Since `♣ = 9 − (both majors)` that closes to three shapes:

| shape | ♣ | had a home before? |
| --- | --- | --- |
| `3-1-4-5` / `1-3-4-5` | 5 | **no** — the hand this exists for |
| `1-2-4-6` / `2-1-4-6` | 6 | `2♠`, but see below |
| `0-3-4-6` / `3-0-4-6` | 6 | `2♠`, but see below |

`3-1-4-5` has no route at all in the shipped system: too few majors for Stayman,
too few diamonds for `2NT`, too few clubs for `2♠`, and not `balanced()` for
Puppet `3♣`. At 9+ HCP it blasts `3NT`; at 8 it **passes 1NT holding a singleton
opposite 15-17**. The `♣6` rows do qualify for the `2♠` club transfer and the
splinter outranks it (weight 1.7 against 1.3) deliberately — after `2♠` responder
can never show the four diamonds, and that is the whole 6-4 slam lane.

**Splinter, not fragment.** [Bridge Winners](https://bridgewinners.com/article/view/1nt-3-splinters-fragments-and-slam-exploration/)
argues for naming the three-card major instead, since "opponents are less likely
to make a lead directing double" and a DD study found the same trick count on
91.6% of deals. Rarer, but worse when it happens: an LDX of a *fragment* runs the
lead through responder's real three cards with the doubler's length sitting
behind them, while an LDX of a *splinter* directs into a void or singleton and
warns us for free. DD is blind to right-siding and lead-direction alike, so this
one is settled on theory — an A/B here would measure ≈0 and tell us nothing.

### The floor could not use the reading

Responder's rule alone was measured **inert**. With opener left to the floor —
which receives the whole pinned shape through the alert decode and is already
barred from passing by `opener_forced_past_invitation` — a 600 000-board
self-play run fired 217 times, diverged on **9** boards, and moved −4 IMPs plain
/ −22 PD. A direct probe explains it: over `[1NT, P, 3♥, P]` the floor bid `3NT`
on *every* opener hand tried, including `♥KQJ` (total wastage) and five spades
opposite a known three-card holding.

So `nt_splinter_answer` is authored, and it places the game on one pivot — a
guard in the short major:

| opener | call |
| --- | --- |
| `stopper_in(short)` | `3NT` |
| no guard, `♣3+` | `5♣` |
| no guard, `♣≤2`, `♦4+` | `5♦` |
| catch-all | `3NT` |

Responder holds 0–1 in the short major and opener 2–4, so the opponents own
eight to eleven cards there: unguarded, they cash out before `3NT` runs nine
tricks, while the minor game has a known 8–9 card fit and a ruffing value.
Opener places the *game* rather than probing at the four level because `4♣`/`4♦`
is below game in a game-forcing auction, and a floor-generated pass would strand
the partnership in a partscore. The 4-4 major fit is not hunted: responder holds
2–3 in the other major and opener at most five, so the best case is a 5-3, and
over `3♠` there is no room below `3NT` to look anyway.

### The measurement

Self-play, `examples/ab-nt-splinter`, arm 0 = slot empty / arm 1 = splinter,
5 000 000 boards per vulnerability, `SEED_BASE=1785227147`. Opponents silenced:
this is constructive value only.

| vul | fired | divergent | plain | PD |
| --- | --- | --- | --- | --- |
| none | 1995 (0.040%) | 176 | **+0.56** IMPs/fired (+0.0002/bd) | **+0.67** IMPs/fired (+0.0003/bd) |
| both | 1995 (0.040%) | 176 | **+0.69** IMPs/fired (+0.0003/bd) | **+0.81** IMPs/fired (+0.0003/bd) |

Win in all four cells, so it ships **default-on**. Read the effect off the 176
divergent boards, not the 1995 fired: 9 firings in 10 reach the same contract
either way (`3NT` is usually right), and on the tenth the convention is worth
**+6.4 IMPs plain / +7.6 PD** at none and **+7.8 / +9.2** at both. IMPs/board is
near zero because 0.04% of boards is 0.04% of boards — a thin slot bid well.

The [`set_splinter_doubled`](../../CHANGELOG.md) comparison from the plan
(+15.4 IMPs/fired at the same 0.04% frequency) sets the scale: half that, which
is what a *constructive* slot should look like next to a *competitive* one.

The `set_nt_splinter_floor` 8-vs-9 sweep is a separate run at the same seed
(arm 0 is identical, so the two arm-1 totals subtract).

### Owed: reading BEN's form

Our reader now decodes `1NT–3M` off the alert instead of the natural five-plus
walk — but only for **our** book's shape. BEN loads its card and plays the GIB
form, so against BEN the box is still wrong (2–3 in the other major where BEN
holds exactly four). Deferred rather than guessed, because it is **unmeasurable
today**: BBA's engine default for the toggle is off, so no anchor run ever sees a
`1NT–3M`, and there is no BEN harness yet. Land it with `ben-gen`.
