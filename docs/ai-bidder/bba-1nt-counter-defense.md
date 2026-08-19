# BBA's counter-defense to its own methods — the 1NT opener's side

[bba-1nt-defense.md](bba-1nt-defense.md) reads BBA/EPBot **defending** a 1NT
opening with Woolsey Multi-Landy. This doc reads the other side of the same
table: BBA as the **1NT opener**, answering each of those calls. It is the
anchor's own counter-defense to its own methods, and it covers exactly the
lanes [one-notrump-competitive.md](../one-notrump-competitive.md) still has
open packages against.

Same method and same caveats as its sibling: drive the real EPBot engine
(system 0) on thousands of random hands at a forced auction via
[`examples/probe-bba-constraints`](../../examples/probe-bba-constraints/main.rs),
bucket by the call returned, summarise in DSL vocabulary. Bands are the
10th–90th percentile of the bucket; a `sketch` is a candidate constraint, not a
proof of internal logic. Figures below are **NV, 40 000 sampled hands per
lane**; vulnerability is called out only where it moves something.

> **Read the probed hand as unconstrained.** The actor's hand is dealt at
> random, so it can contradict the overcall's claim (a responder holding six
> hearts behind a `2♣` that promised both majors). BBA bids on the assumption
> its reading is true, which is what we want to distil; it is not a fair basis
> for calling any of these calls a blunder.

## The headline: BBA routes on the declaration, but essentially only over `2♣`

`--meanings` reads EPBot's own prose label for the forced overcall
(`epbot_get_info_meaning`), and it names the convention outright. Running each
lane twice — once with `Multi-Landy=1` (declared), once with
`Multi-Landy=0, Cappelletti=0, Landy=0` (undeclared, so the same forced call
can only read as natural) — separates *reading* from *bidding*, because the
auction is forced and the defenders' own settings cannot change what they bid:

| their call | declared label | undeclared label | counter moves by |
| --- | --- | --- | ---: |
| `2♣` | `Multi-Landy, both majors` | `bidable suit` | **59.4%** |
| `2♦` | `Multi-Landy, Multi` | `bidable suit` | 7.9% |
| `2♥` / `2♠` | `Multi-Landy, 5M-4m` | `bidable suit` | 1.5% / 0.1% |
| `X` | `Multi-Landy, 4M-5m` | `takeout double` | 0.7% |
| `2NT` | `Unusual 2NT` | `Unusual 2NT` | 0.0% |
| `3♣`–`3♠` | *(none)* | *(none)* | 0.0% |

"Moves by" is the total-variation distance between the two call distributions.
So the anchor **does** consume the opponents' declared system — the reading
channel is real and it is convention-aware — but the only lane where it
changes the *bidding* is `2♣`, the one call whose natural and artificial
readings are furthest apart. `2NT` is invariant because Unusual 2NT is its own
convention row, not a Multi-Landy one; the three-level calls are invariant
because they are natural.

That is the same conclusion our own campaign reached the expensive way: `N1g`
(`reading.their_landy_reading`) was the wiring that made the `2♣` disclosure
move anything, and it is the only reading knob in the lane that has shipped.

### There is no disclosure channel — the reading follows the *caller's* side

`--their-conv` sets a convention on the side facing the probed seat only, which
separates "they hold the convention" from "we were told about it". Over the
Landy `2♣`, with the actor's own side explicitly `Multi-Landy=0` and only the
defenders' side set to `1`:

| arm | responder's top calls |
| --- | --- |
| both sides on | `3NT` 49.7%, Pass 26.8%, `2NT` 13.6%, `6NT` 2.9%, `3♣` 2.9%, `2♠` 2.8% |
| **their side only** | **identical, board for board** |
| both sides off | `X` 29.2%, Pass 20.4%, `3NT` 13.9%, `2♥` 11.1%, `2♦` 11.0% |

So EPBot reads an opponent's call through the convention object attached to
**that opponent's own side**, not through anything the reader was configured to
believe. There is no separate disclosure input at this node: the declaration
*is* the opponents' settings. Two consequences worth keeping —

- `bba-gen`'s `--advertise-natural` / `--advertise-landy`, which flip the
  opponent bot's `Landy`/`Multi-Landy`/`Cappelletti` rows at our table, are
  structurally the right lever; there was never a second one to find.
- Our own `TheirDisclosures` + `their_landy_reading` split is a pons-side
  reconstruction of what BBA gets for free by holding both sides' cards. That
  is a cost of our architecture, not a missing feature of the anchor's.

## Lane by lane — responder at `1NT (X) ?`

### `(2♣)` Landy — a notrump ladder, no double, minor transfers only

The lane our census made package N1. BBA drops **every major-seeking gadget**
and bids notrump by strength:

| call | share | band | reading |
| --- | ---: | --- | --- |
| **3NT** | **49.7%** | `hcp 9–17` (med 12), bal 49% | to play |
| Pass | 26.8% | `hcp 2–8` | too weak |
| **2NT** | 13.6% | `hcp 7–10` | **natural invite** — opener passes at 15-16, bids `3NT` at 16-17 |
| 6NT | 2.9% | `hcp 18–21` | slam blast (33+ combined) |
| **3♣** | 2.9% | `len(♦, 6..)`, `hcp 5–18` | **transfer to diamonds** — opener completes `3♦`, 100% |
| **2♠** | 2.8% | `len(♣, 6..)`, `hcp 5–18` | **transfer to clubs** — opener completes `3♣`, 100% |
| 7NT | 0.5% | `hcp 22+` | grand blast |

**There is no double at all** — 0% at `--min-share 0`, the only lane in the set
without one. No Stayman, no major transfer, no natural major: their `2♣` claimed
both majors, so every call that would look for one is gone. What survives is
precisely the part of the uncontested structure that survives the claim — the
notrump ladder and the two minor transfers (`2♠`→♣, `3♣`→♦, BBA's own
`1N-2S transfer to clubs` / `1N-3C transfer to diamonds` scheme), running on top
of the stolen `2♣`.

This settles `vendor/bba/21GF.bbsa`'s `Transfers if RHO bids clubs = 1` against
the engine for the first time — the disclosure sweep filed that row as
**cosmetic at 0 of 8406 decisions moved** ([bba-disclosure-sweep.md](bba-disclosure-sweep.md))
and [21gf-ledger.md](21gf-ledger.md) lists it as a gap. It is real, and it is
*selective*: systems-on for the minors and the notrump ladder, systems-**off**
for the majors.

Undeclared, the same forced `2♣` produces the ordinary shape instead — `X` 29.2%,
Pass 20.4%, `3NT` 13.9%, `2♥` 11.1%, `2♦` 11.0% — which is the systems-on rebase
our own book plays in this lane.

### `(2♦)` Multi — X = values, naturals elsewhere

Distilled in [bba-multi-2d.md](bba-multi-2d.md) §3 and not repeated here: `X`
41%, Pass 15%, `3NT` 13%, `2NT` 10%, naturals below. Reproduced exactly by the
current tool, so that table stands.

New here is **opener's answer to the 41% double**, at `1NT (2♦) X - ?`:

| opener | share | band |
| --- | ---: | --- |
| `2♥` | 36.4% | `len(♥, 4..)` |
| `3♦` | 36.1% | no length gate, balanced |
| `2♠` | 27.5% | `len(♠, 4..)` |

**Opener never passes** — 0%. The double is takeout of the unknown major, and
opener answers by showing a four-card major, else cue-bidding `3♦`. Vulnerability
moves nothing.

### `(2♥)` / `(2♠)` Muiderberg — **Lebensohl**, and X shows the other major

Package N2's lane, the census's second-biggest loser (−0.66/bd). The two are
symmetric; `(2♥)` shown, `(2♠)` differing only by suit:

| call | share | band | reading |
| --- | ---: | --- | --- |
| **X** | 25.3% | `len(♠, 4..)`, `hcp 5–17` | takeout **showing the other major** — not a values double |
| **2NT** | 25.0% | `hcp 9–19` (med 13), bal 61% | **Lebensohl relay** — opener bids `3♣`, 100% |
| Pass | 23.3% | `hcp 2–9` | — |
| `3♥` (their suit) | 6.5% | `hcp 9–15`, no length | cue |
| `3NT` | 6.2% | `hcp 8–16` | to play |
| `2♠` | 2.6% | `len(♠, 5..)` | natural |
| `4♠` | 2.5% | `len(♠, 6..)` | natural, to game |
| `3♣` / `3♦` | 2.2% / 2.1% | `len(m, 5..)`, `hcp 5–12` | natural |

Over `(2♠)` the double shows `len(♥, 4..)` and the cue is `3♠`; everything else
is the mirror image.

The `2NT` is the interesting one. Its band is far too wide (`hcp 9–19`) to be an
invite, and opener answers it with a **forced `3♣` on 100% of 1NT openings at
both vulnerabilities** — that is plain **Lebensohl**, confirming
`Lebensohl after 1NT = 1` on BBA's card against the engine. Contrast the `(2♣)`
lane above, where `2NT` is a *natural* invite that opener passes two-thirds of
the time: the relay is on over a suit overcall and off over the club one, which
is exactly what `Transfers if RHO bids clubs` selects.

Opener over the double (`1NT (2♥) X - ?`) is the same never-pass rule as the
Multi lane: `3♦` 65.3%, `2♠` 34.7% (`len(♠, 4..)`).

Vulnerable, responder shifts off the relay toward game: `2NT` 25.0% → 20.5%,
`3NT` 6.2% → 10.5%.

### `(X)` Woolsey — full systems-on, plus a business redouble

| call | share | band | reading |
| --- | ---: | --- | --- |
| **2♣** | 32.4% | `hcp 7–18`, bal 57% | **Stayman**, systems on |
| **XX** | 22.9% | `hcp 6–17`, bal 68% | business |
| `2♥` | 12.4% | `len(♠, 5..)` | Jacoby transfer to spades |
| `2♦` | 12.3% | `len(♥, 5..)` | Jacoby transfer to hearts |
| Pass | 7.7% | `hcp 1–5` | — |
| `4♥` / `4♦` | 3.5% / 3.3% | `len(♠, 6..)` / `len(♥, 6..)` | Texas transfers |
| `2♠` / `3♣` | 2.5% / 2.5% | `len(♣, 6..)` / `len(♦, 6..)` | minor transfers |

The whole uncontested structure runs on top of the double — Stayman, both
Jacoby transfers, both Texas transfers, both minor transfers — with `XX` as the
business call and Pass reserved for `hcp 1–5`. This is the statistical version
of the archetype finding already recorded in
`reference_bba-1nt-doubled-runout`, and it is **invariant to whether the double
is read as Woolsey or as ordinary takeout** (0.7%): BBA's runout does not care
what the double meant.

**Reading the runout was measured and is inert in the deterministic BBA
match.** On 2026-06-27, `set_read_their_runout` narrowed responder's transferred
major after `1NT X 2♦/2♥`; over 20,000 same-seed isolated-defense boards it
fired on **127 boards and changed 0 auctions**. The reason is structural:
`instinct()` consumes `partner().points` at its two inference sites and never
reads an opponent's suit length. Opponent-shape readings belong in the sampled
DD search path, not this book+instinct path. Bridge-wise the immediate defensive
actions are also runout-agnostic: double the suit you hold whether their call is
natural or a transfer, and the completion discloses the real suit one round
later. Do not retry this as a vs-BBA bidding lever unless the measured path has a
consumer for opponent shape.

### `(2NT)` both minors — X, then majors

The census's worst per-board bucket (−1.18/bd, n=118), from their side:

| call | share | band | reading |
| --- | ---: | --- | --- |
| **X** | **46.7%** | `hcp 7–18`, bal 59% | values / penalty |
| Pass | 23.0% | `hcp 1–8` | — |
| `3♠` / `3♥` | 11.9% / 11.8% | `len(M, 5..)`, `hcp 8–18` | natural |
| `3♣` | 3.8% | `len(♥, 4..) & len(♠, 4..)`, bal 67% | **cue = both majors** (Unusual vs Unusual) |
| `3NT` | 2.2% | `hcp 8–13` | to play |

This is the mechanism behind the forensic already in the census —
"BBA **doubles** their minors and we bid on" — now with the rate attached: it
doubles nearly half the time. Opener over that double (`1NT (2NT) X - ?`) is the
same never-pass rule again: `3♥` 36.4% (`len(♥, 4..)`), `3♦` 36.1%, `3♠` 27.5%
(`len(♠, 4..)`).

### `(3♣)`–`(3♠)` — natural seven-card preempts, not artificial calls

Package N3's lane (−0.63/bd) turns out to need no counter-*defense* at all.
`--mode multi --min-share 0` shows BBA's three-level calls over (1NT) are
**natural 7-card preempts** — `3♦` 0.5%, `3♣` 0.4%, `3♠` 0.3%, `3♥` 0.3%, each
`hcp 4–10` with a seven-card suit and nothing else long. Together with `2NT`
(0.6%) they are the census's `2NT` + `3+` buckets.

The counter is an ordinary competitive scheme, invariant to the declaration
(0.0% in all four):

- **Pass** 31–36% (`hcp 2–10`), **3NT** 25–38% (`hcp 9–17`) to play, **6NT** 1–2%
  (`hcp 18–21`).
- **New suit at the three-level**, natural 5+, `hcp 7–18`, ~11–12% each.
- **`4♣` / `4♦`** 1.6–1.8%, `len(m, 6..)`, `hcp 4–11` — natural, to play.
- **X** 7–24%: over `(3♥)` it is `len(♠, 4..)` — takeout showing the other
  major; over `(3♣)`/`(3♦)` it is `balanced()`, values; over `(3♠)` ungated.

#### `(3♣)` per call — **no transfers** (`--mode counter-3c`, 8k/vul, 2026-08-18)

Recorded because our N3 `(3♣)` transfer arm needs the answer: does BBA read
`3♦`/`3♥`/`3♠` over its own `3♣` as transfers?  It does not — all three are
plain naturals, and the shares are vulnerability-flat.

| call | share (none/both) | band | reading |
| --- | ---: | --- | --- |
| Pass | 31.8% / 33.3% | `hcp 2–9` / `2–11` | — |
| `3NT` | 24.9% / 23.3% | `hcp 9–17`, bal 67% | to play, no stopper gate |
| `3♠` | 11.8% / 11.8% | `len(♠, 5..)`, `hcp 7–18` | **natural** |
| `3♥` | 11.3% / 11.4% | `len(♥, 5..)`, `hcp 8–18` | **natural** |
| `3♦` | 10.7% / 10.6% | `len(♦, 5..)`, `hcp 7–18` | **natural** |
| `X` | 7.6% / 7.7% | `hcp 7–19`, bal 86% | values |
| `6NT` | 1.1% / 1.1% | `hcp 18–21` | to play |

So our transfer arm ([one-notrump-competitive.md](../one-notrump-competitive.md)
§N3) is not an alignment move: it wins or loses on its own merit, and what it
buys is the invitational five-card major (which the natural table can only show
as `X` or a pass) plus right-siding, which double dummy cannot price.

### Their side of the lane — the advancer, the preemptor's rebid, and `(4x)`

Probed 2026-08-19 (`--mode custom`, seed 20260819, `--vul none,both`,
`--min-share 0`, `--meanings 50`; commands in [Reproduce](#reproduce), dumps in
`ab-results/probe-3level/`). Everything above reads BBA in *our* seats; this
reads BBA in *its own* — the seat that advances the preempt, and the preemptor's
second turn. It is the evidence behind
[one-notrump-competitive.md](../one-notrump-competitive.md) §N3's v2 queue.

**Read the shares as per-random-hand, not per-auction.** The probe deals the
actor a uniform random hand and asks EPBot for its call: EPBot sees the auction,
but the *hand distribution* is not conditioned on our side having shown 23+
points. Every advancer share below is therefore biased **towards action**, and
the bias is an order of magnitude, not a rounding. Realized rates, counted over
14,120 `1NT (3x)` boards of a `--filter-preempt` arm (`ab-results/nt-answer-tie/
base-none`, sha `7f8fa998`, NV):

| after | probe says Pass | **realized** Pass |
| --- | ---: | ---: |
| `1NT (3♣) X` | 88.2% | **99.7%** |
| `1NT (3♦) X` | 88.2% | **100.0%** |
| `1NT (3♥) X` | 50.2% | **97.1%** |
| `1NT (3♠) X` | 50.9% | **98.3%** |
| `1NT (3♥) 3NT` | 34.6% | **93.3%** |
| `1NT (3♠) 3NT` | 35.4% | **90.9%** |

So use the probe for **structure** — which calls exist, on what shape, and which
do *not* exist at all — and an arm dump for **frequency**. The structural facts
below (no advancer gadgetry, no `(3NT)` row, the preemptor never rebids, they
sit for our double) survive the reweighting; the shares do not.

Our own realized responses on the same 14,120 boards, for scale:

| their call | n | our response |
| --- | ---: | --- |
| `(3♣)` | 4106 | Pass 28.0%, `X` 28.0%, `3♠` 14.6%, `3♥` 12.0%, `3♦` 11.2%, `4M` 5.6%, `3NT` 0.6% |
| `(3♦)` | 4356 | `X` 33.9%, Pass 28.6%, `3♠` 14.1%, `3♥` 13.1%, `3NT` 4.5%, `4M` 5.7% |
| `(3♥)` | 2828 | `X` 29.6%, Pass 28.2%, `3NT` 21.3%, `3♠` 17.9%, `4♠` 2.4% |
| `(3♠)` | 2830 | Pass 30.9%, `X` 26.4%, `4♥` 23.5%, `3NT` 18.6% |

#### The advancer over its partner's preempt (seat 3, 40k/vul)

| lane | Pass | raise | other |
| --- | ---: | --- | --- |
| `1NT (3♣) -` | 84.6% | `4♣` 7.5%, `5♣` 3.8% | `3NT` 1.6% (`hcp 13–22`, bal 84%), `4NT` 1.0% |
| `1NT (3♦) -` | 84.1% | `4♦` 8.2%, `5♦` 3.9% | `3NT` 1.2%, `4NT` 1.1% |
| `1NT (3♥) -` | 48.0% | **`4♥` 49.8%** (`hcp 4–18`) | `4NT` 1.3% |
| `1NT (3♠) -` | 48.3% | **`4♠` 49.6%** (`hcp 4–18`) | `4NT` 1.2% |

There is **no advancer structure to counter** — no cue, no artificial raise, no
new suit above 0.4%. Over a minor it passes or raises the minor; over a major it
raises to game or passes, with essentially no strength gate (`hcp 4–18` on the
raise). The `4NT` row is the both-minors two-suiter reappearing one level up.

#### Sit-vs-rescue over **our** takeout double (seat 3, 40k/vul)

This is the row the penalty-pass item needed. `X` is read as `takeout double`.

| lane | **Pass** | rescue / raise | `XX` |
| --- | ---: | --- | ---: |
| `1NT (3♣) X` | **88.2%** | `4♣` 4.2%, `5♣` 3.0%, `6♣` 0.7% | 3.1% (`hcp 18–23`) |
| `1NT (3♦) X` | **88.2%** | `4♦` 4.3%, `5♦` 3.0%, `6♦` 0.7% | 3.2% (`hcp 18–23`) |
| `1NT (3♥) X` | 50.2% | `4♥` 29.3% (`hcp 3–14`, 4+♥), `3♠` 20.0% (`hcp 10–20`) | 0.3% |
| `1NT (3♠) X` | 50.9% | `4♠` 44.8% (`hcp 4–17`) | 3.4% (`hcp 18–23`) |

**BBA sits.** Over a minor preempt it passes our double ~88% of the time even
with an action-biased hand distribution; over a major it sits half the time and
otherwise raises to game. So a leave-in row in `nt_answer_double` *can* fire —
this is not the `(2♦)` lane, where the runout was unconditional. The `XX` row is
a business redouble on 18+, which our opener's leave-in must survive: it is 3%
per random hand and rarer still opposite our 23+.

#### The preemptor never bids again (seat 1, 300k/vul, filtered)

Six two-ply lanes, each keeping only hands BBA actually preempts with
(`--filter-call 3x --filter-prefix "1NT"`, ~0.4% acceptance, n≈900–1300 kept):

| lane | Pass |
| --- | ---: |
| `1NT (3♣) X - 3♥ (?)` | **100.0%** |
| `1NT (3♦) X - 3♥ (?)` | 99.8% (`4♦` 0.2%, 8+♦) |
| `1NT (3♥) X - 3♠ (?)` | **100.0%** |
| `1NT (3♠) X - 4♥ (?)` | **100.0%** |
| `1NT (3♣) 3♥ - 4♥ (?)` | 99.4% (`5♣` 0.6%, 8+♣) |
| `1NT (3♥) 3♠ - 4♠ (?)` | **100.0%** |

The preemptor's hand is spent. Every `(4z)` tail in this lane therefore comes
from the **advancer**, never from the preemptor — which is what prices the
`X (4z)` node: over a minor it is the 11.8% of rescues above, over a major the
`4M` raise.

#### The advancer over our constructive responses (seat 3, 20k/vul)

| our call | advancer Pass | its action |
| --- | ---: | --- |
| `(3♣) 3NT` | 54.9% | `4♣` 38.2%, `5♣` 4.1%, `6♣` 1.4%, **`X` 0.9%** |
| `(3♦) 3NT` | 53.2% | `4♦` 40.1%, `5♦` 3.9%, `6♦` 1.5%, **`X` 0.9%** |
| `(3♥) 3NT` | 34.6% | **`4♥` 63.4%**, `6♥` 1.5%, `X` 0.4% |
| `(3♠) 3NT` | 35.4% | **`4♠` 62.6%**, `6♠` 1.5%, `X` 0.3% |
| `(3♣) 3♦/3♥/3♠` | 60–65% | `4♣` 28–33%, `5♣` ~4% |
| `(3♥) 3♠` | 42.2% | `4♥` 56.2% |
| `(3♠) 4♥` | 25.7% | **`4♠` 68.7%**, `X` 4.1% |
| `(3♥) 4♥` | 93.2% | `5♥` 4.5%, `4NT` 1.9% |

Two structural facts: BBA **almost never doubles** our constructive response
(≤0.9% everywhere), and it competes to the four level over `3NT` far more often
than it doubles it. Our `4♥` over `(3♥)` buys the auction (93% pass) because it
takes their suit away; the same `4♥` over `(3♠)` is overcalled `4♠` two thirds
of the time.

#### `(4x)` triggers — eight-card suits, and **there is no `(3NT)`**

The direct seat rerun at 200k/vul with `--min-share 0`, which is what exposes
rows the 40k table rounded away:

| call | share (none) | band |
| --- | ---: | --- |
| `3♦` / `3♣` | 0.44% / 0.40% | `hcp 4–10`, **7+** in the suit |
| `3♠` / `3♥` | 0.31% / 0.30% | `hcp 4–9`, **7+** |
| `5♣` / `5♦` | 0.055% / 0.051% | `hcp 6–19` / `5–18`, **8+** |
| `4♥` / `4♠` | 0.049% / 0.046% | `hcp 4–10` / `5–9`, **8+** |
| `4NT` | 0.035% | `hcp 5–17`, 5-5 minors |
| `4♦` / `4♣` | 0.012% / 0.006% | `hcp 3–8` / `4–7`, **8+** |
| `6♣` / `6♦` | 0.0015% / 0.001% | `hcp 19–23`, 7+ |

**BBA never bids `3NT` directly over our 1NT** — the row does not exist at
200,000 hands per vulnerability, at either vulnerability. The `(3NT)` half of
the v2 queue has no trigger against this opponent and is closed. The four-level
rows are all **eight**-card suits: `(4x)` is not a widened `(3x)`, it is a
different (and six times rarer) hand class, and `(5m)` is as common as `(4M)`.

#### The advancer over their own `(4x)` (seat 3, 20k/vul)

| lane | Pass | action |
| --- | ---: | --- |
| `1NT (4♣) -` / `(4♦) -` | 86.3% / 86.4% | `5m` 9.5% / 9.3%, `4NT` 2.5% / 3.1% |
| `1NT (4♥) -` / `(4♠) -` | 95.3% / 95.5% | `4NT` 3.2% / 3.1%, scattered 5-level 0.5% |
| `1NT (4♣) X` / `(4♦) X` | 96.7% / 96.8% | `6m` 2.6% / 2.5% |
| `1NT (4♥) X` | **97.5%** | `6♥` 2.2% |
| `1NT (4♠) X` | **99.9%** | `XX` 0.1% |

They sit for our double of a `(4x)` preempt essentially always. Our floor cannot
double there at all (`their_live_bid_at_most(3)`), so the whole `(4x)` double is
a book-only opportunity — against an opponent that never runs from it.

## What this says about our packages

Read as evidence about the anchor, not as a design to copy — nothing here has
been measured in our system, and [measurement.md](../measurement.md)'s gate
applies to any of it.

- **Adopted 2026-08-15 as N1j** (`defense_2c_landy_bba`, default-on): our
  counter now plays this lane's ladder shape — wide minor transfers, no
  gadget cues — with the values `X` kept and a GF both-minors
  takeout/splinter family added.  Shipped at a pre-pinned non-inferiority
  gate on the structural-alignment rationale;
  [one-notrump-competitive.md](../one-notrump-competitive.md) §N1j holds the
  verdicts and the reading ceiling (row 122 projects our *uncontested*
  Puppet scheme onto the lane, so alignment is structural, never literal).
- **N1 (shipped) agrees with BBA on the ladder and disagrees on the double.**
  Our counter's gated `3NT` outranking the cues, and N1c's weak-6+ minor
  transfer, both have BBA analogues (`3NT` 49.7%; `2♠`→♣ and `3♣`→♦ completed
  100%). Our values `X` at `hcp(8..)` has none — BBA never doubles Landy. Three
  of our own experiments (N1d, N1h, N1i) priced hands *onto* that double and
  liked it, so this is a disagreement with evidence on our side, not an
  oversight to fix.
- **N2's open question has an answer to compare against**: BBA plays plain
  **Lebensohl** over Muiderberg with a takeout double showing the *other* major,
  where we play Cohen **Transfer** Lebensohl. That is a concrete A/B, not a
  redesign.
- **N3 is misnamed.** There is no artificial three-level call to counter — it is
  a natural-preempt lane, so the package is competing over a preempt, and the
  `defense_to_preempt` machinery is the relevant code, not this campaign's.
- **N4 gains opener's side of the double**: over `1NT (2♦) X`, BBA's *opener*
  **never passes** — it shows a four-card major, else cues `3♦`. A penalty
  double of the Multi is answered by opener, not sat for, so pricing one has to
  count opener's pull. (~~A separate probe of the *overcalling* side found
  they sit 43% of the time over our own double.~~ **Retracted 2026-08-15**:
  that count mixed in the foreign lane; `--mode advance-x` shows BBA's
  advancer never passes our double — see docs/one-notrump-competitive.md §N4.)
- **N6 (`2NT`, n=118) now has a mechanism**: BBA doubles 46.7% and plays UvU
  with `3♣` as the both-majors cue. Still needs boards before it needs code.

## Reproduce

```text
# responder's counter, per interference call (`counter` = the 2♦ Multi lane)
cargo run --release --example probe-bba-constraints -- --mode counter-x   --vul none,both --samples 40000
cargo run --release --example probe-bba-constraints -- --mode counter-c   --vul none,both --samples 40000
cargo run --release --example probe-bba-constraints -- --mode counter     --vul none,both --samples 40000
cargo run --release --example probe-bba-constraints -- --mode counter-h   --vul none,both --samples 40000
cargo run --release --example probe-bba-constraints -- --mode counter-s   --vul none,both --samples 40000
cargo run --release --example probe-bba-constraints -- --mode counter-2nt --vul none,both --samples 40000
cargo run --release --example probe-bba-constraints -- --mode counter-3c  --vul none,both --samples 40000  # ...-3d, -3h, -3s
# opener's answer one round deeper
cargo run --release --example probe-bba-constraints -- --mode opener-c-2nt --vul none,both --samples 40000
cargo run --release --example probe-bba-constraints -- --mode opener-c-2s  --vul none,both --samples 40000
cargo run --release --example probe-bba-constraints -- --mode opener-c-3c  --vul none,both --samples 40000
cargo run --release --example probe-bba-constraints -- --mode opener-d-x   --vul none,both --samples 40000
cargo run --release --example probe-bba-constraints -- --mode opener-h-2nt --vul none,both --samples 40000
cargo run --release --example probe-bba-constraints -- --mode opener-s-2nt --vul none,both --samples 40000
cargo run --release --example probe-bba-constraints -- --mode opener-h-x   --vul none,both --samples 40000
cargo run --release --example probe-bba-constraints -- --mode opener-2nt-x --vul none,both --samples 40000
# the reading half: BBA's own label, then nobody declared, then their side only
cargo run --release --example probe-bba-constraints -- --mode counter-c --meanings 200
cargo run --release --example probe-bba-constraints -- --mode counter-c --samples 40000 \
    --conv "Multi-Landy=0" --conv "Cappelletti=0" --conv "Landy=0"
cargo run --release --example probe-bba-constraints -- --mode counter-c --samples 40000 \
    --conv "Multi-Landy=0" --conv "Cappelletti=0" --conv "Landy=0" --their-conv "Multi-Landy=1"
# the three-level calls being natural preempts
cargo run --release --example probe-bba-constraints -- --mode multi --min-share 0 --vul none,both --samples 40000
# their side of the three-level lane (2026-08-19); full sweep in scripts under
# ab-results/probe-3level/, every lane at --vul none,both --min-share 0 --meanings 50 --seed 20260819
P='cargo run --release --example probe-bba-constraints -- --mode custom'
$P --seat 3 --calls "1NT 3♣ -"   --samples 40000   # advancer, per (3♣)…(3♠)
$P --seat 3 --calls "1NT 3♣ X"   --samples 40000   # ...sit-vs-rescue over our takeout X
$P --seat 3 --calls "1NT 3♣ 3NT" --samples 20000   # ...over our constructive response
$P --seat 1 --calls "1NT"        --samples 200000  # the direct table at 200k: (4x) rows, no (3NT)
$P --seat 3 --calls "1NT 4♥ X"   --samples 20000   # advancer over our double of a (4x)
$P --seat 1 --calls "1NT 3♣ X - 3♥" --filter-call 3♣ --filter-prefix "1NT" --samples 300000
```

The last of those is the arm that shows the reading follows the caller's own
side; it reproduces the both-sides-on table exactly.
