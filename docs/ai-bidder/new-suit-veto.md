# The phantom-suit rail — an envelope-gated new-suit veto on the learned floor

**Status: MEASURED and REFUTED 2026-09-02 — stays opt-in, default off.**
Plain DD **−0.0212 ±0.0047** (vul none) and **−0.0164 ±0.0057** (vul both) over
204,800 boards/arm/vul against BBA, perfect defense a wash either way
(−0.0016 ±0.0056 / +0.0025 ±0.0067). Seed 1788352713, control `1040c84c`,
`scripts/ab-new-suit-veto.sh`. Plain DD was the pre-registered arbiter and a
plain-DD loss never ships, so `InstinctProfile::new_suit_veto` stays `false` in
`InstinctProfile::default()` and the shipped system is byte-identical
(`smoke-default --count 20000 --seed 1` = `38ee1e21…` at `1040c84c` and at the
change). §6 has the forensic, which is the durable part: the idea is not
narrowable by level, and its envelope gate turned out to do no work. This is [bba-floor.md](bba-floor.md) §7 row G, and the second
demotion-only stage in the floor's shell after
[competitive-accountant.md](competitive-accountant.md).

Origin: the 2026-08-14 post-ship decompose of the `1NT (2♣)` lane
(`docs/archive/one-notrump-competitive-closed.md` "Residue"), which named two
alternatives — "an M6.4-style rail (conversation-in-motion → instinct, **or an
envelope-gated new-suit veto scoped off agreed fits**), not another node ring".
The §N1-lia lia3 forensic (2026-09-02) measured the second one's class and made
it the general fix the Landy lane is parked behind
(`docs/one-notrump-competitive.md` §N1-lia "The rail evidence", "Disposition").

## 1. The decision this rail owns

The net names trump suits nobody has. It is asked for a call in a contested
seat the book left alone, and it answers with a suit bid in a suit where our
hand is short and partner's calls have promised nothing — a contract named off
the bare envelope. Concretely, from the lia3 forensic's own tables:

| node (default system, baseline arm) | n (none) | plain | per fired |
| --- | --- | --- | --- |
| `2NT - 3♣ (3♥) 4♣ - [N:4♥ own3+p0]` | 328 | −2,798 | −8.5 |
| `2NT - 3♣ (3♥) 4♣ - [S:4♥ own3+p0]` | 297 | −2,471 | −8.3 |
| `2NT - 3♣ (3♥) 4♣ - [S:4♥ own2+p0]` | 215 | −2,309 | −10.7 |
| `2NT (3♥) - - 4♣ - [S:4♥ own3+p0]` | 264 | −1,692 | −6.4 |
| `2NT - 3♣ - - 3♥ [N:4♥ own2+p0]` (both) | 88 | −1,004 | −11.4 |

Opener cue-bids their hearts on two or three cards after a club transfer. The
class is not a Landy artefact: on the lia3 divergent boards the *default
system's own floor* made such calls on **6,800 (none) / 3,844 (both) boards and
lost 5.2 / 6.5 IMPs per fired**. The lia arm only put more seats in front of
the same floor.

Those are pools over divergent boards — not bounds, and not a measurement. The
rail also fires on the millions of boards that never diverged, which is exactly
what the A/B is for.

## 2. Why the fix is on the outputs

`features_v6` (176 values, `src/bidding/features.rs`) carries a raw
*we-bid-this-strain* bit and partner's last bid, and `src/bidding/context.rs`
sets those strain bits for **artificial calls too**. There is no alert column
and no tag column. So a conventional `2♠` — a transfer, a cue, a Landy ask —
and a natural `2♠` reach the net as the same float. The net cannot distinguish
them, and it was trained on a teacher that could not either.

Masking the strain bit for alerted calls would shift the inputs off the
training distribution and needs a retrain (the two input-side arms queued behind
this one are in `docs/one-notrump-competitive.md` §N1-lia's handoff). A rail
acts on **outputs**, so it needs neither.

`docs/bidding-architecture.md` licenses this: "Because this is a
deterministic-floor change, changing its scorer alone does not imply
retraining." The shell owns rails; the net owns physics
([card-manifold.md](card-manifold.md): "v5 = same rails/mask as v4's shell").

## 3. The predicate

`new_suit_gate` (`src/bidding/instinct.rs`, beside `competitive_gate`) masks
`Call::Bid(bid)` when `bid.strain.suit()` is `Some(suit)` and

```text
own <= 4  and  own + partner_announced_min(suit) <= 5
```

where `own = hand[suit].len()` and the minimum comes from
`Inferences::announced(Relative::Partner).length(suit).min`.

Four properties, each a decision taken with jdh8 on 2026-09-02:

- **The announced envelope, not the walk hull.** `announced(Partner)` is the
  union-tightened hull the nets are fed and the one the forensic cut on;
  `partner()` is the sound projection `has_fit` reads. Their `lengths` disagree
  on 1.3% of decisions (`src/bidding/inference/read.rs`). The rail matches its
  evidence. Both are sound lower bounds, so neither can claim a fit that was
  never promised — soundness over tightness
  (`docs/bidding-architecture.md`).
- **No bid-identity term.** `Context::we_bid` / `partner_last_bid` are
  side-scoped strain bits set for artificial calls, so consulting them would
  exempt precisely the transfers and cues the rail exists for. The predicate is
  hand-and-envelope only.
- **Their suit is in scope.** A floor cue above game with no fit is a phantom by
  the same logic, and the default system's worst vetoable class *is* a cue. An
  exemption for `they_bid(strain)` would have removed the largest measured class
  from the arm.
- **No level gate, no fit exemption.** Per-call evidence by level was
  four-level −27,642/−26,536, three-level −5,241/−11,253, five-plus
  −1,529/−1,718, **two-level ~0** — so a level floor buys nothing the `own <= 4`
  cap does not already buy, and adds an untested constant. Likewise no
  `has_fit`-elsewhere exemption: an announced eight-card fit can never satisfy
  the predicate, so "scoped off agreed fits" is implicit in the predicate rather
  than a second test. The known cost of both choices is that a genuine control
  bid over an agreed trump suit is masked; the forensic buckets that.
  **The A/B refuted the level reasoning** — every level lost, and the four-level
  class the evidence pointed at was the *least* costly per fired. See §6.

Notrump is untouched: `Strain::suit()` is `None`, and a strain with no suit has
no length to be phantom about.

## 4. As built

| decision | as built |
| --- | --- |
| Stage site | one line after `competitive_gate` in **both** shells — `ConfiguredFloorV6::classify` (shipped) and `ConfiguredFloorBba::classify` (v4). Dutch rides the same shell (`src/bidding/dutch.rs` → `with_floor_v6`) and inherits it. |
| Path | judgement only. `forced()` returns instinct's logits before any stage runs, so a conversation in motion never meets the rail. |
| Order | after `mask_illegal`, so it never reasons about an insufficient bid. Both stages are demotion-only and read different inputs, so the order between them is inert. |
| Shape | `-∞` on the masked bid. Never touches `Call::Pass`, so `has_mass()` survives and `Trie::resolve_floored` can never reject the floor (§0.2). |
| Introduces | nothing. `select_with_legal_state` takes the next finite legal logit; the net keeps its monopoly on introducing calls. |
| Publishes | nothing. A logit-mask stage is projection-invisible — no rule, no alert, no envelope, no `.bbsa` row. Pinned by `a_reading_knob_leaves_the_card_alone`. |
| Attribution | `NEW_SUIT_FIRED: [AtomicU64; 2]` + `pub fn new_suit_counts() -> [u64; 2]`, printed per shard by `bba-gen`: `[0]` decisions whose top (pre-mask argmax) call the rail took, `[1]` candidate bids masked. The two differ by three orders of magnitude — 36,998 against 27,260,473 over the A/B — because a decision with a silent partner offers a short suit at every level. Only `[0]` is what a divergence is made of; a first cut of this counter reported `[1]` alone and was useless for attribution. |
| Knob | `InstinctProfile::new_suit_veto`, default **off**; `bba-gen --ns-new-suit-veto`; `probe-decision` `PROBE_NEW_SUIT_VETO=1`; `probe-layer-replay --ns-new-suit-veto` for replaying the ON arm. No `web/` row while default-off (`docs/bidding-options.md`: the registry is a player-facing chooser, not an A/B instrument). |
| Tests | three in `src/bidding/neural_floor/tests.rs`: `the_new_suit_gate_takes_the_phantom_cue` (knob-off finite, knob-on `-∞`, Pass and `X` finite, `has_mass()`, counters, the argmax actually moved, a five-card holding spared); `the_new_suit_gate_spares_an_announced_fit_in_both_shells` (a raise of partner's announced suit is never a phantom — the arithmetic *is* the "scoped off agreed fits" claim — and the rows run through `ConfiguredFloorV6` as well as the v4 twin the other helpers build, because the two shells duplicate their stage list by hand); `the_new_suit_gate_masks_the_phantom_overcall` (the reach row: a silent partner puts every four-card-or-shorter suit inside the predicate). |

### The reach is wider than the worst nodes

With partner having announced nothing — a silent or passing partner, the `p0`
column that dominates the evidence — `own + 0 <= 5` reduces to `own <= 4`, so
the rail masks **every** suit bid in every suit of four cards or fewer. On a
balanced hand in an unauthored contested seat the floor is left with notrump,
`Pass`, `X` and `XX`. That is the rail as specified and as the forensic cut it,
and the two-level class it sweeps up measured ~0 either way — but it is the
reason pass 1 pre-sizes the fired rate before any headline is quoted, and the
reason the forensic must bucket **what replaced the masked call**. Nothing
forces the runner-up to be sane.

## 5. Traps this rail was written against

- `net_collar` lost 12/12 by cutting the net where it was *right* (cold
  grands): "a veto wearing the other shape's name". The `own <= 4` cap is the
  guard here — a natural long suit is outside the rail by construction, and the
  six-card push class is deliberately outside it too.
- `set_free_bid_quality` was refuted for suppressing *winning* junk free bids.
  Same failure mode, same guard.
- The settle Stage-2 TTL's level-≥4 gate lost plain DD. That precedent is one
  reason no level gate was added on speculation.
- Over-matching a rail strands auctions (the keycard-window lesson,
  `docs/reading-drift-handoff.md`): the predicate is hand-conditioned and
  demotion-only, and never introduces a call.

## 6. Measurement

Pre-registered before the first board (`scripts/ab-new-suit-veto.sh` header):

- Harness `bba-gen` vs the BBA reference, **unfiltered**, both arms
  `american()`, differing only by the knob. Two arms per vul, house default
  204,800 bd/arm/vul for pass 1.
- **The knob bids less**, so `docs/measurement.md`'s `loss | win` row is the
  artifact row (PD credits phantom doubles of contracts we no longer bid).
  **Plain DD is the arbiter at both colours, PD required non-negative** — the
  competitive accountant's precedent, the only other demotion-only floor stage
  that ever shipped. SD is a second pass only if plain earns it, quoted as
  [SD-PD, plain SD]. Plain-DD loss never ships.
- **No isolation gate**: the rail fires in every unauthored seat under either
  opening, so `gatepair` would fail by construction. The ungated probe's
  ours/theirs opener split is an informational row.
- Fresh `SEED_BASE`, arms sequential under `scripts/idle-run.sh`, control =
  `main` HEAD on the same seed, never rebuild in flight.

### Verdict — REFUTED, 2026-09-02

Seed 1788352713, control `1040c84c`, 204,800 boards/arm/vul, unfiltered.

| vul | fired | plain DD | PD |
| --- | --- | --- | --- |
| none | 6,889 (3.36%) | **−0.0212 ±0.0047** | −0.0016 ±0.0056 |
| both | 6,181 (3.02%) | **−0.0164 ±0.0057** | +0.0025 ±0.0067 |

`loss | wash` on the pre-registered reading: plain DD loses at both colours with
the CI clear of zero, PD is a wash at both. Plain-DD loss never ships.

The mechanism did exactly what it was built to do — `probe-divergence` reports
**100% of first-differing calls ours** and **0% "bid where the baseline passed"**
at both colours, so the stage is demotion-only in production, not just in the
unit test. It is the *idea* that is wrong, in three measurable ways.

**1. It is not narrowable by level.** Plain IMPs per fired, by the level of the
masked call:

| level | none: n / IMPs / per fired | both: n / IMPs / per fired |
| --- | --- | --- |
| 1 | 1,323 / −1,401 / −1.06 | 1,316 / −1,300 / −0.99 |
| 2 | 3,155 / −1,631 / −0.52 | 2,928 / −720 / −0.25 |
| 3 | 1,699 / −1,075 / −0.63 | 1,450 / −1,316 / −0.91 |
| 4 | 638 / −214 / −0.34 | 456 / −55 / −0.12 |
| 5+ | 74 / −27 | 31 / +41 |

Every level is negative at both colours. This **refutes the lia3 pool that
motivated the rail**, which read four-level phantom bids as the bulk of the
loss: on the default system the four-level class destroys games 301-to-29 (vul
none) and is the *least* costly slice per fired. The pool was a correlation on
the divergent boards of a losing arm, never a claim that masking those calls
would help — the forensic said so itself ("pools, not bounds; the masked call is
replaced, not undone"), and this is the first actual test.

**2. The damage is entirely the four-card class.** By our own length in the
masked suit, as IMPs/board over the full 204,800:

| own length | none | both |
| --- | --- | --- |
| ≤ 1 | **+0.0005 ±0.0018** | **+0.0003 ±0.0019** |
| ≤ 2 | −0.0019 ±0.0024 | −0.0014 ±0.0028 |
| **= 4** | **−0.0178 ±0.0035** | **−0.0100 ±0.0044** |

`own == 4` alone carries 84% (none) and 61% (both) of the headline and is the
only CI-clear slice. Voids and singletons are sign-positive at both colours but
far inside noise. A four-card suit is a normal thing to bid — the rail was
forbidding the ordinary vocabulary of four-card majors, four-card overcalls and
second suits.

**3. The envelope gate does no work.** Partner's announced minimum in the masked
suit was **0 on 98.2% / 98.4%** of fires. So `own + partner_min <= 5` collapses
to `own <= 4` in practice, and the rail is operationally *"never name a suit you
hold four or fewer of"*, not *"never name a suit nobody has"*. This is the
structural error: an announced minimum of zero means partner has **not spoken
about** that suit, not that partner is **short** in it. `Inferences` is sound —
it never over-promises — and soundness is exactly why it cannot support this
inference. When partner has bid, their promise covers only the suit they bid;
60.6% of fires happened opposite a partner who *had* bid, and those carried the
worst damage (1,189 games lost against 515 gained).

Worst boards show the replacement risk realised: masking `3♥` promoted `3NT`
into a double and redouble (−21); masking a `2♠` rebid promoted `4NT`, landing
`5♦` doubled (−19). Nothing forces the runner-up to be sane, and the pre-mask
argmax is taken away without the net getting to re-rank against the mask.

### What would have to change for a next attempt

A veto keyed on *announced* length cannot work, because zero is the answer
almost always. Any successor needs a channel that distinguishes "unpromised"
from "denied" — the negative inference the reading layer does not currently
carry. That is the same missing axis M8.4 names for the forcing channel, and it
is an input-side change (a retrain), not another output-side rail. The `own ≤ 1`
slice is the only sign-positive evidence here and is too small to justify its
own arm on this data.

### The first successor was tried and refuted the same day (2026-09-02)

jdh8's redesign: artificiality is a property of a call's **meaning**, not of the
bidder's hand — "natural = assured length, and the logical negation is
*possible* shortness, not certain shortness". A suit call is artificial in the
suit it *names* iff its published box union permits at most `most` cards there.
The threshold was to be `0..=2` everywhere with a `0..=0` exception for `1♣`
(the catch-all opening: a doubleton or tripleton there is the natural case, so
only a possible void makes `1♣` not-clubs). Note `EnvelopeUnion::hull` reduces
with `Envelope::span`, so `hull.length(s).min` **is** the min over boxes — the
hull and the union give the same answer and there is nothing to choose there.

The principle is sound and sits on the right side of the soundness asymmetry:
`Inferences` never over-promises, so `min` is a sound *lower* bound and "assured
length" is a claim the reading layer can carry — unlike "partner is short",
the upper bound §6.3 showed it cannot. **The book, however, cannot answer it.**

- Natural `1♦` (`american/openings.rs:126`) publishes `♦ 0..=13`. Its length
  lives in `prefers_diamonds()`, a `described(...)` closure whose
  `projection_dependencies` are the vacuous default (`constraint.rs:972`), so it
  never reaches a box. Natural `1♣` publishes `3..` only because that rule
  happens to spell the term out.
- Over `american()`, `min <= 2` on the named suit fires on **70.2%** of
  suit-naming rules, of which **69.6% is pure `0..=13` silence**. `min == 0`
  means the rule did not *speak* about the suit — §6.3's conflation, one layer
  up.
- Restricted to rules that do speak (`range != FULL_LENGTH`): 508 instances at
  13 sites at `most = 2`, **every one a natural `len(major, 2..)` doubleton
  preference**; 256 instances and **zero unalerted sites** at `most = 1`. The
  calls the predicate exists to catch — red transfers, Texas, Michaels, unusual
  2NT, takeout/negative/responsive doubles, every transfer completion — are all
  in the silent class, indistinguishable from natural `1♦`.
- Measured directly on the invariant: widening `artificial_calls_are_alerted`
  with this witness is **green at `most = 1`** and **red at `most = 2`** — 240
  unalerted `american` calls, 228 Dutch, 16 Gladiator. So `0..=2` is not usable
  against the book as it stands, and the `1♣` exception is inert on the default
  system anyway (american's `1♣` already publishes `3..`).

**The prerequisite is that natural rules publish their assured length**, which
is the [authored-reading-handoff.md](../authored-reading-handoff.md) gap ("a
natural bid reads as nothing") seen from the projection side. That became its
own campaign — [publish-assured-length.md](../publish-assured-length.md) — whose
first arm was measured the same day and refuted, with a root cause that gates
the whole programme: **100% of the divergence is floored calls**, so a sound
reading improvement cannot be measured against a net distilled on the
unimproved reading. It is not free.
`1♦`'s floor is *derivable from the rule's own terms* — both majors are capped
at 4, so `c + d >= 5`; if `d <= 3` then `prefers_diamonds` forces `d > c`, so
`c + d <= 2d - 1 <= 5`, hence `d = 3` — so `& len(Suit::Diamonds, 3..)` rejects
no hand the rule accepts, and the natural walk *already* installs `3..` for a
`1♦` opening (`inference/readers.rs:689`). It is eval-inert and duplicates the
walk. **It still moved the shipped system**: `smoke-default --count 20000
--seed 1` went `38ee1e21…` -> `acc4ab9d…`. A provably eval-inert term changes
bids because it changes what the call *publishes* — the sharpest available
demonstration of `docs/reading-drift-handoff.md`'s rule that a reading change is
a bidding change under a neural floor. Any such program is therefore an A/B per
rule family, not a cleanup.

**What survived.** The witness itself, as `names_short`
(`inference/projection.rs`, `#[cfg(test)]` beside `artificial`), wired into
`artificial_calls_are_alerted` at `most = 1`. It costs no bidding change and
closes the blind spot the dual witness has by construction: `artificial` asks
whether the projection floors some *other* suit at four, which is vacuously
false for a splinter, so a future unalerted splinter-shaped call would silently
lose its decoding. Arm B — the same predicate as a floor rail — is **not
buildable**: `new_suit_gate` runs exactly where the book gave no mass,
`Classifier::as_rules()` is `None` for the learned floor, and an absent union
hulls to `0..=13`, so it would mask nearly every suit bid the contested floor
makes.

## 7. Disposition

**Opt-in knob, default byte-identical** — the house rule for a rejected-but-
interesting treatment. Nothing to revert: the knob was built default-off and the
smoke digest never moved. The code, the runner, the `probe-layer-replay`
passthrough and the three tests stay, because they are what a successor arm
would need and because the replay reproduced 100% of both arms' calls, which
makes this forensic re-runnable.

Not a `park/` branch: the code is finished and the measurement is done, so there
is no owed work to park — this is a settled loss, like `net_collar`. Anyone
re-litigating it needs a different criterion, not a fresh seed.

**Consequence for the Landy lane.** §N1-lia was parked behind this rail on
jdh8's direction — "try the suppress first; if a general fix happens to solve a
local problem, we don't need code for that local problem." The general fix does
not solve it, so that park has no rail to wait for. The lane's disposition
returns to the lia3 verdict (`docs/one-notrump-competitive.md` §N1-lia) and its
lia4 book changes stand on their own, unblocked and unhelped.
