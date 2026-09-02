# Publishing assured length — the campaign, and the retrain gate that blocks it

**Status: arm 1 measured and REFUTED 2026-09-02; the campaign is retrain-gated.**

**Where the code is.** Arm 1 is **parked on `park/publish-assured-length`**, not
on `main` — a measured non-win whose flip needs a retrain, which is the `park/`
trigger rather than the knob one (CLAUDE.md § Workflow). Nothing of the knob,
its `bba-gen` flag or its runner exists on `main`; rebase the branch onto `main`
before re-measuring, and keep the control at `main` HEAD on the same
`SEED_BASE`. What *did* land on `main` is §6. This document is the campaign
ledger and lives on `main` so the census worklist and the retrain gate outlive
the branch.

## 1. The idea

jdh8, after the phantom-suit rail was refuted
([ai-bidder/new-suit-veto.md](ai-bidder/new-suit-veto.md)): artificiality is a
property of a call's **meaning**, not of the bidder's hand. **Natural = assured
length, and the logical negation is *possible* shortness, not certain
shortness.** A suit call is artificial in the suit it names iff its published box
union permits at most `most` cards there.

The predicate sits on the right side of the soundness asymmetry. `Inferences`
never over-promises, so `min` is a sound **lower** bound and "assured length" is
a claim the reading layer *can* carry — unlike "partner is short", the upper
bound the rail needed and could not get (`new-suit-veto.md` §6.3).
`EnvelopeUnion::hull` reduces with `Envelope::span`, so `hull.length(s).min` **is**
the min over boxes: the hull test and the union test are the same test.

## 2. Why it is not evaluable today

The book does not make the claim. Over `american()`, `min <= 2` on the named
suit fires on **70.2%** of suit-naming rules, of which **69.6% is pure `0..=13`
silence** — `min == 0` means the rule did not *speak*. Natural `1♦`
(`american/openings.rs:126`) is the emblem: its length lives in
`prefers_diamonds()`, a `described(...)` closure whose projection dependencies
are the vacuous default (`constraint.rs:972`), so the rule publishes `♦ 0..=13`
while the natural walk installs `3..` for the same call
(`inference/readers.rs:689`). Natural `1♣` publishes `3..` only because that rule
spells the term out.

So the prerequisite is that **natural rules publish their assured length**. This
is [authored-reading-handoff.md](authored-reading-handoff.md)'s "a natural bid
reads as nothing", seen from the projection side.

### The worklist

`silent_natural_suits` (`src/bidding/inference/tests.rs`, `#[ignore]`d) is the
census: it walks every authored rule of `american()` and `dutch()` under the
shipped reading profile and tallies unalerted suit-naming rules whose projected
named-suit range is `FULL_LENGTH`.

```
cargo test --all-features --lib silent_natural_suits -- --ignored --nocapture
```

34,740 silent instances in american, but ~30k are slam-zone contract placement
(`6♠`/`7♠`/`6♥`/`7♥`/`5♠`/`5♥`), where named-suit length is not the point — a
`6♠` bid is a contract, not a suit show. The actionable residue is **528
low-level instances**, and they collapse hard under seat rotation (every node
appears once per dealer seat):

| family | note |
| --- | --- |
| `1♦` opening | 4 nodes. **Arm 1, below.** |
| opener's `2♥`/`2♠` second-suit rebids over a minor response | e.g. `1♣ - 2♣ - 2♠`, `1♦ - 2♦ - 2♥` — reverses, true floor 4 |
| a three-level rebid family | `3♣`/`3♦`/`3♥`/`3♠`, the bulk of the 528 |

`1♣`, `1♥`, `1♠`, `2♣` and `2♦` have **no** silent unalerted rules at all: the
one-level openings are already covered except `1♦`.

## 3. Arm 1 — the `1♦` opening

`& len(Suit::Diamonds, 3..)` on the better-minor `1♦` rule, behind
`OpeningKnobs::one_diamond_publishes_length` (default off).

**The term is eval-inert, and this is proved, not assumed.** Both majors are
capped at four, so `c + d >= 5`; if `d <= 3` then `prefers_diamonds` forces
`d > c`, giving `c + d <= 2d − 1 <= 5`, hence `d == 3`. No hand the rule accepts
is rejected. Pinned exhaustively over shapes by `one_diamond_assures_three` and
over 256 dealt hands by `one_diamond_publishes_length_is_opt_in`. It duplicates
what the walk already installs. **It still moves bids** —
`smoke-default --count 20000 --seed 1` reads `38ee1e21…` off and `acc4ab9d…` on
— because it changes what the call *publishes*.

The generated `.bbsa` card is unchanged (`card.rs:566`: the
`1D opening with N cards` rows are constants, and publishing `3..` makes the
opening neither 4+ nor 5+), so both arms disclosed the identical system to BBA.

### Verdict — REFUTED

`scripts/ab-one-diamond-length.sh`, seed 1788361323, 204,800 bd/arm/vul,
unfiltered vs BBA.

| vul | fired | plain DD | PD |
| --- | --- | --- | --- |
| none | 2,883 (1.41%) | −0.0007 ±0.0029 | −0.0015 ±0.0034 |
| both | 2,568 (1.25%) | **−0.0041 ±0.0035** | **−0.0057 ±0.0042** |

`wash | wash` at none, **`loss | loss` at both**, both CIs clear of zero. A
plain-DD loss never ships and PD does not rescue it, so the knob stays default
off; a second seed cannot change the disposition, because the best case is a
wash and a wash does not ship default-on either.

**The isolation gate PASSED at both colours** — 0 of 2,568 / 2,883 divergent
boards opened by the other side, 100% of first-differing calls ours. That is the
eval-inertness claim confirmed in production: the term changed no opening
decision, only what the opening publishes.

## 4. The root cause — the campaign is retrain-gated

Replaying the baseline arm through `probe-layer-replay` (100% of records
`matches: true`) and cutting the loss by the layer that produced the baseline's
call at the first differing index:

| baseline's call at the first difference | n | plain IMPs |
| --- | --- | --- |
| **floored** | **2,568 (100%)** | **−831** |
| book | 0 | 0 |

Against a **55.6% floored base rate over all calls on the same boards**. Not one
book call moved.

That is the whole result. The book's gates read the *walk hull*, which already
said `3..`, so publishing the term is invisible to them. The only consumer of
the difference is `features_v6`, which reads `Inferences::announced` — and the
shipped net was distilled on the **untightened** reading. Publishing a true fact
moved the net's input off its training distribution, and the net degraded. Half
the loss is doubles (`X -> -` 206 bd/−149, `- -> X` 86/−100, `X -> 2♠` 20/−103,
`X -> 5♥` 4/−53), the floor's most input-sensitive decision; depths 4–5 carry
−565 of the −831.

**The generalisation, which is the durable part:** a sound reading improvement
cannot be measured against a net distilled on the unimproved reading. The A/B
prices the net's distribution shift, not the reading's quality. Every arm of
this campaign changes `features_v6`'s input and will therefore read as a loss
against the frozen net, however true the published fact is. This is the same
input-side axis M8.4 names, and it is the exact conclusion the phantom-suit rail
reached from the other side — "a successor needs an input-side retrain, not
another output-side rail". The campaign *is* that input-side change.

## 5. What would unblock it

Publish the whole worklist at once, then **retrain the floor on the tightened
reading**, and A/B the pair against the shipped net + untightened reading.
Publishing one family at a time and measuring against the frozen net can only
ever re-measure the distribution shift. `docs/ai-bidder/card-manifold.md`'s
sibling-factory rule applies: a system name must reach the same net on its
declared and undeclared paths.

Do **not** add a `features` row for the knob. The vector has fixed slots; a new
one changes its width and invalidates the shipped net.

## 6. What survived on `main`

`names_short` (`inference/projection.rs`, `#[cfg(test)]` beside `artificial`),
wired into `artificial_calls_are_alerted` at `most = 1`. It costs no bidding
change and closes a blind spot the dual witness has by construction: `artificial`
asks whether the projection floors some *other* suit at four, which is vacuously
false for a splinter, so a future unalerted splinter-shaped call would silently
lose its decoding. Measured green on `american()` and `dutch()` at `most = 1`;
**red at `most = 2`** — 240 unalerted `american` calls, 228 Dutch, 16 Gladiator,
every one a natural `len(major, 2..)` doubleton preference. So jdh8's preferred
`0..=2` bar is not usable against the book as it stands, and its `1♣` carve-out
(`0..=0` for the catch-all opening, so a short-club `1♣` stays natural and a
Precision/Polish one does not) is inert on the default system anyway, american's
`1♣` already publishing `3..`.
