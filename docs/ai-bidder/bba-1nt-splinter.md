# BBA's `1NT–3♥/3♠` splinter

What BBA/EPBot's `1N-3M splinter` toggle actually plays, read off the live engine
with [`examples/probe-bba-constraints`](../../examples/probe-bba-constraints/main.rs)
(`--mode nt-resp|nt-3h|nt-3s`), 40 000 random responder hands and 20 000 opener
hands, `--conv "1N-3M splinter"=1`. Bands are the 5th–95th percentile of the
sampled bucket unless the run is marked untrimmed. We do not author this slot
([src/bidding/american/notrump.rs:218](../../src/bidding/american/notrump.rs#L218)
has no `3♥`/`3♠` rule; our reader takes the call as a natural forcing 5+ suit,
[src/bidding/inference.rs:1575](../../src/bidding/inference.rs#L1575)).

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
