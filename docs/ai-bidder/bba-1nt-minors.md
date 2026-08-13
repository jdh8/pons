# EPBot's 1NT minor scheme — the probe behind `european.rs`

**What this is for.** `src/bidding/american/notrump/european.rs` is not a system
we play. Puppet is our default and beats European head-to-head
([bidding-options.md](../bidding-options.md), `set_notrump_minors`). European
exists so we can *recognize* a continental opponent — BEN's declared card and
every vendored BBA card carry exactly its row pattern
(`vendor/ben/BEN-21GF.bbsa:9-13`: `1N-2S transfer to clubs = 1`,
`1N-3C transfer to diamonds = 1`, `1N-3C Puppet Stayman = 0`).

So its acceptance test is **fidelity to what EPBot actually plays, not IMPs**.
An opponent model that bids *better* than the opponent is a worse model. That
inverts the usual rule: no A/B gates a change here, and a soundness argument
does not outrank the probe.

**Why the vendored cards are not the evidence.** The FFI ignores `.bbsa` files
entirely — `examples/probe-bba-1nt/main.rs:5-8` records the strace showing the
`.so` opens no data file. Cards drive `BBA.exe`; the engine we link is
configured through `set_conventions`. They can and do disagree: `1N-3M splinter`
is `1` in `21GF.bbsa` and `0` in the engine. Only the live probe counts.

## Repro

`--conv` **replaces** the default convention set
(`probe-bba-constraints/main.rs`), so the whole family must be pinned or the
scheme silently reverts:

```bash
CONV=(--conv "1N-3C transfer to diamonds"=1 --conv "1N-3C Puppet Stayman"=0
      --conv "1N-2S transfer to clubs"=1  --conv "1N-2N transfer to diamonds"=0
      --conv "Multi-Landy"=1              --conv "Cappelletti"=0)

# 1a — the response classes
cargo run --release --example probe-bba-constraints -- \
  --mode nt-resp --samples 40000 --seed 7 --trim 0.0 --min-share 0.002 "${CONV[@]}"

# 1b — opener's answer to the diamond transfer
cargo run --release --example probe-bba-constraints -- \
  --mode nt-3c --samples 60000 --seed 7 --trim 0.0 --min-share 0.002 "${CONV[@]}"

# 1c — responder's rebid over the completion
cargo run --release --example probe-bba-constraints -- \
  --mode nt-3c-3d --samples 400000 --seed 7 --trim 0.0 --min-share 0.002 "${CONV[@]}"

# 1d — the same, one lane over: responder's rebid after the CLUB completion
cargo run --release --example probe-bba-constraints -- \
  --mode nt-2s-3c --samples 400000 --seed 7 --trim 0.0 --min-share 0.002 "${CONV[@]}"
```

`--trim 0.0` reports **hard min/max**, not percentiles — that is the whole point
for a class question, and it is what the `1N-3M splinter` study used. Raw output
in `ab-results/european-minors/`. The `nt-3c` and `nt-3c-3d` modes were added by
this study; `nt-3c-3d` filters on hands EPBot actually bids `3♣` with (enriched
probing — accept on the raw hand *before* the studied node), which is why 400k
samples yield 10,260 at the node.

## 1a — the response classes (`nt-resp`, n=40000, vul none)

| call | share | HCP (hard) | the tell |
| --- | --- | --- | --- |
| `2♣` | 32.1% | 6–26 | Stayman |
| Pass | 13.1% | 0–9 | — |
| `3NT` | 12.8% | 6–15 | longest minor 4–6 |
| `2♥` | 12.4% | 0–24 | ♠ **5–6** |
| `2♦` | 12.3% | 0–25 | ♥ **5–6** |
| `4♥` | 3.5% | 1–24 | ♠ **6–7** (Texas) |
| `4♦` | 3.5% | 0–24 | ♥ **6–7** (Texas) |
| **`3♣`** | **2.6%** | **0–23** | **♦ 6–7, ♣ 1–4** |
| `2NT` | 2.3% | 6–10 | balanced 80% |
| **`2♠`** | **2.3%** | **0–25** | **♣ 6–7** |
| `4♣` | 1.0% | 15–27 | balanced 82% (Gerber) |
| `3♦` | 0.9% | 9–26 | ♦ 5–6 |
| `5NT` | 0.5% | 18–21 | balanced 85% |
| `4♠` | 0.5% | 0–20 | ♣ **5–5** ♦ — 5-5 minors |

**The finding.** Both minor transfers are **strictly six-card one-suiters**.
`3♣`'s diamond length has a hard minimum of 6 over 1042 hands; `2♠`'s club
length likewise. EPBot routes the shapes that are *not* six-card one-suiters
elsewhere: 5♦ with values → `3♦` (5–6♦, 9+ HCP), 5♦ without → `3NT`, and 5-5
minors → `4♠`.

This refutes the class `european.rs` shipped from commit `2bcbd3e` until
2026-08-13: `len(♦,6..) | (len(♦,5..) & len(♣,4..))`. That disjunction was
deleted from Puppet's `2NT` rule and pasted byte-identically into *both*
`puppet_minors()` and `european_minors()`, then rationalized in a doc comment
("no room below 3♦ to show the clubs"). It was never probed and no test pinned
the two-suiter arm. Puppet keeps it — that class is measured, and Puppet is a
system we play. European loses it.

Note the fold was *worse* for European than for Puppet on its own terms:
Puppet's `3♣` denial gives a 5♦4♣ hand a pass-or-correct escape, while
European's unconditional completion gives it none.

## 1b — opener's answer (`nt-3c`, n=2968 after the 1NT filter)

| call | share | HCP | balanced |
| --- | --- | --- | --- |
| `3♦` | **100.0%** | 15–17 | 98% |

One bucket, no second call at any share. The completion is unconditional — there
is no super-accept — which is what `european_three_club_answer`'s `hcp(0..)`
already said. Confirmed rather than assumed.

## 1c — responder's rebid (`nt-3c-3d`, n=10260 at the node)

Every bucket has ♦6+ — the filter's own guarantee, restated by the data.

| call | share | HCP (hard) | shape tell |
| --- | --- | --- | --- |
| Pass | 46.4% | 0–7 | ♦6–7 |
| `3NT` | 19.7% | 8–14 | ♦ exactly 6 |
| `4♣` | 11.1% | 10–18 | ♣ 1–4, **median 3** → control cue |
| `5♦` | 8.5% | 1–15 | ♦6–7, signoff |
| `4NT` | 6.5% | 7–28 | keycard (trump AKQ mean 2.09) |
| `4♥` | 3.6% | 10–18 | ♥ **1–3** → control cue |
| **`4♠`** | 2.3% | 7–23 | **♠ 0–0** — void |
| **`5♥`** | 1.6% | 9–21 | **♥ 0–0** — void |
| **`5♣`** | 0.4% | 4–19 | **♣ 0–0** — void, ♦7–9 |

**The finding.** EPBot *does* have shortness-showing calls here, but they are
nothing like the Puppet lane's:

- **Void-only.** `4♠`/`5♥`/`5♣` have the short suit pinned to **0–0** on hard
  min *and* max. A singleton never triggers them. Our Puppet twin splinters on a
  void *or* a low singleton (`splinter_short`).
- **Different rungs.** There is **no `3♥`/`3♠` bucket at any share** — the two
  calls the Puppet lane uses. EPBot's `4♥` and `4♣` occupy the mid rungs as
  ordinary control cues (1–3 and 1–4 cards in the bid suit, medians 2 and 3).
- Two of the three void shows sit *above* `5♦`, so this is not a
  "let opener pick the game" mechanism at all — it is slam machinery running
  beside a plain `4NT` keycard ask.

So the rain check that commit `3d0f376` wrote against this node — *"revisit only
if the diamond splinter measures a win under Puppet"* — resolves to **no**, and
for a reason unrelated to the one recorded there. That commit's stated premise
(European's `3♦` is "a blind transfer completion, not a fit promise, so the
premise is absent") was false on its own terms: responder holds 6+♦ and opener
is balanced, which is an eight-card fit by arithmetic. The correct reason is
simply that EPBot does not bid `3♥`/`3♠` here, and a model that did would be
less faithful.

## 1d — responder's rebid, club lane (`nt-2s-3c`, n=9929 at the node)

The lane 1c never probed. `probe-bba-constraints` had `nt-3c` and `nt-3c-3d` and
**nothing for clubs**, so `european_two_spade_rebid()` shipped `3♦`/`3♥`/`3♠`
splinters at priority 100 on no evidence at all — inherited from Puppet's two-way
`2♠`, exactly the copy-paste `420d891` removed from the diamond lane.

Every bucket has ♣6+ — the filter's own guarantee, restated by the data.

| call | share | HCP (hard) | shape tell |
| --- | --- | --- | --- |
| Pass | 47.7% | 0–7 | ♣6–7 |
| `3NT` | 19.8% | 8–15 | ♣6–7, ♦1–4 ♥1–3 ♠1–3 |
| `4♦` | 11.3% | 10–18 | ♦ **1–4** → control cue |
| `5♣` | 6.6% | 3–15 | ♣6–7, signoff |
| `4NT` | 6.5% | 7–26 | keycard (trump AKQ mean 2.18) |
| `4♥` | 4.0% | 10–18 | ♥ **1–3** → control cue |
| **`4♠`** | 2.1% | 7–21 | **♠ 0–0** — void |
| **`5♥`** | 1.6% | 6–21 | **♥ 0–0** — void |
| **`5♦`** | 0.4% | 7–18 | **♦ 0–0** — void, ♣7–9 |

**The finding — the diamond lane's, verbatim.** There is **no `3♦`/`3♥`/`3♠`
bucket at any share**; indeed no three-level call at all but `3NT`. Shortness is
void-only (`4♠`/`5♥`/`5♦`, 0–0 on hard min *and* max), the mid rungs are ordinary
control cues, and two of the three void shows sit above the `5♣` signoff. Even
the shares line up within a point or two of 1c, lane for lane. The two lanes are
one mechanism, so the three splinter rungs are deleted and
`european_two_spade_rebid` is now the exact twin of
`diamond_transfer_game(8, false)`.

Note also that EPBot's `3NT` bucket runs `♦1–4 ♥1–3 ♠1–3` — **singletons
included**. So the old `club_no_shortness(8)` (which demanded 2+ in every side
suit) was doubly wrong: it barred from `3NT` precisely the hands it sent to a
splinter EPBot never makes.

Cross-checked against BEN's own auctions (`ab-results/ben-anchor/`, deduped):
**0 splinters in 6 splinter-eligible hands** (6+♣, 8+ HCP, singleton or void) —
e.g. `KQ8.T82.6.KQJ986`, 11 HCP 3-3-1-6, where we bid `3♦` and BEN bids `3NT` —
and **0 three-level calls other than `3NT`** across all 23 hands at the node.
Model agreement rises **52% → 65%** on those 23; the diamond lane sits at 70%.

### The eight-count boundary: BEN and its teacher disagree

All 8 residual disagreements fall in buckets this file already declares
unmodelled — but they are not noise-shaped:

| BEN's call | n | our call | HCP |
| --- | --- | --- | --- |
| `4NT` (keycard) | 4 | `3NT` | 13–14 |
| Pass | 4 | `3NT` | **8, all four** |

The `4NT` group is the documented keycard gap. The Pass group is not a gap: it is
**four for four at exactly 8 HCP**, where EPBot's probe puts a hard `3NT`
minimum. The diamond node showed the same thing at n=4. Two independent lanes now
agree that **BEN's Pass/`3NT` boundary is 9, not EPBot's 8**.

Recorded, not acted on. EPBot's probe is unambiguous (Pass 0–7 hard *max*, `3NT`
8–15 hard *min*, over ~20k hands per lane) and this file's charter is fidelity to
the probe. But n=8 across two lanes is the first measured crack in the
"probe EPBot as a cheap oracle for BEN" method
([ben-gap-campaign.md](../ben-gap-campaign.md)) — precisely the BBA-plus-search
deviation that method is flagged provisional for. Moving the boundary to 9 is a
one-character change; what it needs first is a probe of BEN itself at the node,
not more EPBot.

Also under-powered and likewise recorded only: BEN passed 2 hands sitting inside
the probed `4♠`/`5♣` void-show constraints (n=2 — distillation smooths 0.4–2.3%
tails).

## The stock defaults already play this scheme

Settled while the FFI was warm, because no `=0` control existed anywhere in
`ab-results/` and the sibling study `bba-1nt-splinter.md:14-27` found the card
and the engine disagreeing on exactly this kind of row:

```bash
cargo run --release --example probe-bba-conventions -- vendor/ben/BEN-21GF.bbsa --all
```

| row | engine default | BEN's card |
| --- | --- | --- |
| `1N-2S transfer to clubs` | **1** | 1 |
| `1N-3C transfer to diamonds` | **1** | 1 |
| `1N-3C Puppet Stayman` | **0** | 0 |
| `1N-2N transfer to diamonds` | **0** | 0 |

Card and engine agree here, unlike `1N-3M splinter`. The consequence is bigger
than the confirmation: **EPBot's compiled-in system-0 defaults are European**, so
every unconfigured BBA opponent in every A/B we have ever run has been playing
`2♠` = clubs and `3♣` = diamonds while our reader modelled them as Puppet. The
`--conv` forcing in the repro above is belt-and-braces, not the cause of the
result.

## Reading a European opponent

`european.rs` is an opponent model, and until 2026-08-13 every setter of
`notrump_minors` was our-side: **zero coverage on the
`Partnership::with_opponents` reading path**, the one thing the scheme is for.
`Inferences` holds a single `profile: ReadingProfile` populated from *our*
system, and two branches decided European-vs-Puppet meaning off it:

- `read.rs`, responder's `2NT`. Gated by `is_opening_side`, which is parity
  relative to the **opener**, not to us — so it fires on the opponents' auction
  and answered from our knob.
- `nt_structure_artificial`, which blankets the `1NT - 2♠`/`3♣` continuation as
  relays. Reached with **no side gate at all**, unlike its neighbours
  `nt_splinter_artificial` and `nt_blanket`.

The fix is one lookup — `context.their_system()`, which `Context::with_system`
already stores — folded into a `side_profile` chosen by `opener_lane` parity. It
degrades to today's behaviour when no opponent is declared, because
`Partnership::opponents` returns ours. `Inferences.profile` stays a single field:
it also serves as the valuation gauge in `admits`, and "our valuation of their
hand" is defensible where "our claim about their agreement" is not.

### The missing side gate is **not** shipped — it is an A/B, and a bigger bug

Adding the `is_opening_side` gate to `nt_structure_artificial` was in this
change's plan and was **backed out** after measuring it:
`smoke-default --count 40000 --seed 1` moved **826 of 40000 boards** of the
*shipped Puppet default*, and **680 of those contain no 1NT at all**.

Tracing them found the larger defect. `nt_structure_artificial` tests only
`auction[opening_index + 2]`, with **no `opening_bid == 1NT` check**, so
`1♣ (1♠) 3♣` and `1♠ (2♦) 2♠` "enter" the notrump minor structure and get their
whole continuation blanketed as relays — on both sides. The side gate does not
fix that; it merely stops half of it leaking, which is why it moves so much.

Both defects are recorded on the function's own doc comment. Fixing either is a
live bidding change on the default system and needs the A/B the iron rules
demand — the same reason the ~30 other side-blind reading sites in `readers.rs`
are out of scope for a fidelity-gated commit.

### Confinement gate: 2 of 3840, and defect 1 again

`ben-gen` at 1920 boards on a shared seed (`SEED_BASE=770001`, 16 shards × 120,
tier F, vul none), pre-change vs post-change, diffing the `boards` array:

```
1920 boards, 2 changed auctions  (2 / 3840 tables = 0.05%)
```

The plan's gate said every changed board must be one where BEN opened 1NT and
responder bid `2♠`/`3♣`/`2NT`. **Neither of these two is.** Both are:

```
1♥ X 2NT 3♣ 3♥ - 4♥ 5♣ X - - -      →  1♥ X 2NT 3♣ 3♥ - 4♥ - - -
- - - 1♥ 2♣ 2NT 3♣ 3♥ - - -         →  - - - 1♥ 2♣ 2NT 3♣ 3♥ - - 4♣ - - -
```

BEN opened **`1♥`** at both, and its partner's second call is `2NT`. Puppet's
`entered` set contains `2NT`, European's does not — so under the declaration the
bogus relay blanket lifts and our side reads the following suit bids naturally.
That is **defect 1 above**, firing on a `1♥` opening because
`nt_structure_artificial` never checks the opening was 1NT.

Shipped anyway, deviating from the plan's literal gate, because the gate's stated
failure mode is provably not what happened: the opponent book differs from ours
in *exactly one field*, and the leak outside 1NT auctions is a pre-existing
reader bug, not a wide declaration. Both changes move in the corrective
direction. Recorded here so a future reader does not have to re-derive it —
and so that when defect 1 is fixed under its own A/B, this gate becomes clean.

No anchor re-baseline. A board-exact diff showing 2 changed auctions in 1920
boards is strictly stronger evidence than an anchor mean, and the affected boards
are uncontested besides.

`examples/ben-gen` had no `with_opponents` call anywhere, so it now declares an
opponent book identical to ours **except `notrump_minors`** — default-on, no
flag. Declaring BEN's whole system is the Phase-2b treatment (plain wash, PD
−0.0070, `docs/declarative-rows.md` §2b); confining the delta to one axis keeps
that mechanism from firing. `bba-gen` gets the axis as `--ns-european-minors`,
which reaches the opponent seat through `--their-ns`.

**Sizing, honestly.** The misread covers ~0.8% of boards, all *uncontested*, so
the only cash-out is the opening lead and defence — and the anchor scorer is
double-dummy on the reached contract, which cannot express that. `sd-lead` *is*
reading-sensitive, but `ab-dump-sd` skips boards whose arms bid alike, which is
every board here. Budget if a harness were built: ~0.001–0.004 IMPs/board. Not
worth harness work, and under this file's charter unnecessary — fidelity is the
gate.

## What we model, and what we do not

Modelled after 2026-08-13:

- `2♠` = 6+♣, `3♣` = 6+♦ — both exact against the probe.
- Opener's unconditional `3♦`/`3♣` completion — exact, 23/23 and 24/24.
- Responder's Pass (`hcp(..8)`) / `3NT` (`hcp(8..)`) split, **both lanes** —
  covers each node's two biggest buckets (66.1% diamonds, 67.5% clubs), and the 8
  boundary matches EPBot's Pass 0–7 / `3NT` 8–14 exactly.

**Not** modelled, deliberately, in either lane:

- The `5♦`/`5♣` signoff, the `4NT` keycard, and the two control cues.
- The three void shows (`4♠`/`5♥`/`5♣` diamonds, `4♠`/`5♥`/`5♦` clubs).

Together that is ~34% of the diamond node and ~32% of the club node. These are
recognition gaps, not bidding losses — we never take these calls ourselves. The
pins `no_three_level_splinter_over_the_diamond_completion` and
`no_three_level_splinter_over_the_club_completion`
(`tests/american_european_minors.rs`) keep the Puppet rungs out; adding the void
shows is the natural next fidelity increment, and needs opener's answers authored
with them.

## Ledger

Closes row 12 of [21gf-ledger.md](21gf-ledger.md) (`1N-3♣ transfer to ♦`), open
with an empty A/B column since it was written. Row 9 (`1N-2♠ transfer to clubs`)
is now probed too, in its continuation.
