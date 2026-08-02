# The configured net — one net that reads the convention card

**Status: phases 0–1 landed** (the `set_conv` read-back guard; `features_v4` and
`Context::with_config`). Nothing serves v4 yet. Supersedes the two-artifact twin
scheme
(`american_bba.f32` + `american_bba_kickback.f32`) that
[`bba-kickback.md`](bba-kickback.md) §7.7 introduced and §7.12 showed the cost
of.

## Why

`set_kickback` drives rule presence *and* the floor's weights, so every cell it
measures prices the package. That is not a harness defect to be tightened —
it is structural, because a kickback-blind net bids a natural 4♥ into the
auction where the ladder has claimed 4♥ as the diamond ask. One knob has to
drive both, and so the convention and the net can never be separated by an
arm.

§7.12 put numbers on what that costs. **93.5% of divergent boards saw no
keycard ask from either side** — those cannot be the relocation, and they carry
89% of the PD gain. Worse, the ♠ lane, where the relocation *provably* cannot
move a call (♠ has nowhere to relocate to; 4NT already is its ask), still reads
−1.939 PD/board over 722 boards. The measured quantity is dominated by
something that is not the convention.

Two nets also differ by **vintage**, not only by regime. `american_bba.f32` was
trained 2026-07-25 and the twin 2026-08-02, with **68 commits to `src/bidding`
between them, 58 of them `feat`/`fix`** — the queen relay, the merged reply,
the answer gates, the reading repairs. The twin is fitted to a week-newer
system as well as to kickback.

The fix is not a better control arm. It is to put the configuration **in the
features**, so one net serves every regime and an arm differs by a config row
instead of by an artifact.

## Why the earlier mixed net failed, and why this is different

`dump-teacher --mix-kickback` already built a single net on both regimes (866k
rows, alternating by board). It was the better net by aggregate val CE — 0.4004
against the twin's 0.4431 — **and it still bid the phantom 4♥**.

The diagnosis is in `neural.rs`: the regime was not in the features *at the
moment the call is chosen*. Readings describe the auction so far, and
`1♦ P 1♠ P 2♦` is three natural bids in either system, so both regimes present
that decision with **byte-identical inputs and contradictory targets** — 2♥
from the kickback teacher, 4♥ from the plain one. The net could only average
them. The ranges do diverge, but one ply too late: only once a relocated ask
has been made, which is the decision we needed it to get right.

Adding the config bit is exactly what makes those two rows distinguishable. The
mixed-net result is therefore evidence *for* this design, not against it: the
corpus and the tooling already exist, and the one thing that broke it is the
thing being added.

(Val CEs across the three nets are **not** directly comparable — each was
measured on its own validation corpus. Cited for provenance, not as a ranking.)

## What the config is

**The convention card, both pairs.** `card.rs` already generates the card from
live knob state, so there is no new vocabulary to invent and no recurring
"which knobs count?" argument — and the features stay disclosable by
construction, because the feature vector *is* the disclosure.

| block | width | source |
| --- | ---: | --- |
| `features_v3` | 88 | unchanged |
| our card | 140 | base system one-hot (5) + `SCHEMA` (133) + `PONS_SCHEMA` (2) |
| their card | 140 | same encoding, from the opponents' declared card |
| **`FEATURES_LEN_V4`** | **368** | |

`PONS_SCHEMA` is the two conventions EPBot's schema has no name for (South
African Texas, Queen ask by available bid). "King ask by available bid" is a
real BBA row, not one of ours.

**Both pairs, because a mixed table is the normal case in an A/B.** The two
arms play each other, so at every table one side relocates its asks and the
other does not. Today the net cannot see that at all — it reads identical
features whether or not the opponents play kickback. A net that cannot
represent the mixed table is out of distribution on exactly the boards the
measurement is about.

**Full width, not the varying rows only.** Most rows are constant within any
one corpus and will train to ≈0. Pruning to the varying rows would make the
artifact's meaning depend on the corpus that produced it, and would re-open the
width question every campaign. The cost is real and worth stating: input width
**88 → 368**, roughly 4× on layer 1.

Reserving spare slots was considered and rejected: a net learns weight ≈0 for
an always-zero input, so populating a reserved slot later still needs a
retrain. Reserving saves plumbing churn, not the expensive part.

## The sync: knob → code, knob → card, card → net

The three are synced, and the knob is the single source of truth. `set_kickback`
drives the floor's rules, `american_row` reads the same knob to set
`Kickback 1430`, and `encode_card` puts that row in the feature vector. Nothing
re-states the agreement anywhere; each layer reads the one below.

**But the card is a lossy projection of the code, and the loss is large.**
`src/bidding` has **222 `set_*` knobs**; `SCHEMA` has 133 rows, of which only
**24 match arms** in `american_row` read live state — roughly 26 expressible
axes. The other ~196 knobs move the bidding and are invisible to the card.

That yields one hard invariant for the corpus:

> **Only vary configuration the card can express.** A corpus that varies a
> knob with no card row reproduces the exact mixed-net failure — identical
> feature vectors, contradictory targets — with no symptom but a worse net.

Worth enforcing rather than remembering: a dump that samples config cells should
assert the cells differ in the encoded vector, not merely in the knob state.

The card also under-discloses in a second, milder way. Our king ask *is*
relocated under kickback, but BBA's schema has no king-ask row, so `card.rs`
pins it to 0 always. That is lossy but *consistent* — it moves with
`Kickback 1430` rather than against it — so it costs resolution, not
correctness.

## Two sides, and only one of them has knobs

Our card falls out of our own knob state. The opponents' does not: they are
ourselves, BBA, BEN, or another engine entirely, and none of those has a pons
knob to read. So the two blocks are built by different routes:

| side | source |
| --- | --- |
| ours | live knobs → `american_card()` / `dutch_card()` |
| theirs | a `.bbsa` file → `load_bbsa` → mapped onto `SCHEMA` order |

`Config::new(ours, theirs)` takes them separately for exactly this reason, and
`the_two_sides_are_independent` pins that they may disagree, base system
included.

**The base system is not optional.** `Card::system` is the only channel for
facts no row expresses — `dutch_card` differs from `american_card` by its header
(2/1 → WJ) plus a single row, and the header is carrying the whole wide
non-forcing 1♣. An earlier draft of `encode_card` dropped it, which would have
made a WJ opponent nearly indistinguishable from a 2/1 one. It is now a 5-wide
one-hot at the front of each side's block, pinned by `the_base_system_is_encoded`.

**Owed by phase 2:** a foreign `.bbsa` gives `name = value` pairs, not our
schema order. Mapping one onto the 135 slots needs a decision for both gaps —
rows they set that `SCHEMA` lacks (drop), and `SCHEMA` rows they never mention
(their engine default for that system, *not* zero, since zero is a claim). Until
that exists, only `Config::symmetric` is honest for a foreign opponent.

## Corpus and evaluation

- **Corpus deals: the banks at `/nfs2/jdh8/pons/`** (`24.pdd`, `22.pdd`).
  Double-dummy tables come pre-solved, so no solver runs. `dump-teacher
  --deals` already routes through `pons::pdd::load`, which accepts `.pdd`
  binary and GIB text alike — but it reads whole files, and `24.pdd` is 2 GB.
  Wire `pdd::load_slice` behind `--skip`, the pattern
  `probe-trick-variance` already uses. **Keep `24/22.pdd` byte-stable.**
- **Target: unchanged** — EPBot's chosen call. This is still distillation, not
  a value net.
- **Evaluation: freshly generated deals**, never bank rows. Both gates below
  run on a fresh `SEED_BASE`, so no held-out row was ever trained on.

### How many deals

The shipped nets are distilled from far less than one might guess — the
sidecars record `data_rows` **422,914** (`american_bba`) and **432,033** (the
twin), which at ~10.2 calls a board is **roughly 42,000 deals** apiece. One row
per decision, forced passes skipped, so that is a lower bound on deals.

Sizing v4 against that:

| driver | factor |
| --- | --- |
| params 98,342 → 170,022 (input 88 → 368) | 1.70× |
| config cells to cover — ours × theirs, at minimum 4 | 4× |

| corpus | rows | ≈ deals |
| --- | ---: | ---: |
| old per-cell density × 4 cells | 1.69M | ~167k |
| + width headroom | 2.54M | **~250k** |
| comfortable | 3.38M | ~333k |

### ⚠ A uniform corpus cannot teach the kickback bit

Sizing by *rows* is not enough, because the rows that depend on the config are
vanishingly rare. §7.12's census: **the relocated ask fires on 107 of 200,000
boards, 0.054%.** At 250k deals that is ~135 boards and a few hundred deciding
rows out of ~2.5M — and exactly **one row of the 270** differs between the arms
(the king-ask relocation has no BBA row at all and is pinned to 0). The net is
being asked to key a rare behaviour off a single input whose deciding rows are
~0.01% of the corpus; that weight will barely leave its initialisation.

**This turns gate 2 into a false negative generator.** It would report "no
difference" without distinguishing *the convention is worth nothing* from *the
net never learned to read the bit* — the same confound this document exists to
kill, one level down.

The fix is the repo's standing answer to a rare trigger: **a mixture corpus.**
Keep the uniform draw as the bulk, so the net stays calibrated on ordinary
auctions, and add an enriched slice accepting deals that reach a slam face with
an agreed non-spade trump. Evaluation stays on freshly generated uniform deals,
so the oversampling never reaches the verdict.

Two things to settle before dumping millions of rows, in this order:

1. **Measure the reach rate** — how often a random deal reaches a slam face with
   an agreed non-spade trump. That, not the ask rate, sets the enriched slice's
   yield and therefore its cost.
2. **Pick the mixture ratio** against that number, and record it beside the
   artifact: a net's behaviour is only interpretable next to the distribution it
   was fitted on.

A cheaper fallback if enrichment proves expensive: accept that gate 2 is
underpowered for kickback specifically, and read it only as a bound.

**Take ~250k deals, 500k to be generous** for the uniform bulk. Against `22.pdd`'s 31,404,048 rows
that is **1.6% of the bank**, and it is a *training* draw, so it does not
advance the never-replay cursor. **The bank is not the constraint here** — the
binding cost is dump time, ≈5M EPBot calls at 500k deals. Draw from `22.pdd`
and leave what remains of `24.pdd` (~19.7M rows) for A/B slices that genuinely
need pre-solved tables.

## Acceptance — two gates

| gate | arms | question |
| --- | --- | --- |
| **1. Quality** | v4 net `kickback=off` vs shipped `WEIGHTS_BBA`, **both playing the plain system** | Is the configured net a better bidder at fixed rules? |
| **2. Convention** | ±kickback **within the v4 net** | What is the relocation worth, alone? |

Gate 1 holds the rules fixed and varies only the net, so it also finally prices
how much of the current +0.0705 PD/board was a two-week-newer net rather than a
convention. It subsumes the placebo arm (two same-system nets, different data
seeds) that would otherwise be needed to calibrate net-to-net variance.

Gate 2 is the fair comparison this whole campaign has lacked: **same weights**,
arms differing by one config row and the rules it gates. The confound is gone
by construction rather than corrected for.

Both scored on plain DD **and** perfect-defense, read off `docs/measurement.md`'s
decision table, arms sequential, fresh seed shared across arms.

## Sequence

| phase | work | note |
| --- | --- | --- |
| 0 | **Guard `set_conv` against silent no-ops** | **done** — `BbaOracle::verify_card`, a read-back at card acceptance; the return code cannot see a bogus row |
| 1 | `features_v4` + config plumbing | the open design question below |
| 2 | `dump-teacher`: `--skip`/`load_slice`, per-board config sampling, per-side teacher config, both config vectors per row | |
| 3 | Train on GPU | off-crate, as always |
| 4 | Artifact + candle-parity fixture, wire `classify_bba` to v4, retire the twin selection | the twin artifact can go once gate 1 passes |
| 5 | Gate 1, then gate 2 | fresh deals |

## Resolved: the opponents' config rides on `Context`

Our own card falls out of live knob state, exactly as `card()` reads it today.
The opponents' does not — nothing at a seat currently knows what the other
partnership has agreed, so it has to be supplied.

**Decision (jdh8): carry it on `Context`**, the tidy path, over a thread-local
set beside the knobs. A thread-local would have been two lines in the harness
and no signature changes, but in a codebase this size an ambient global that
silently changes what a feature vector means is the wrong trade.

`Context::new` keeps its signature. The cards become fields with defaults —
**opponents default to our own card**, mirroring how `oracle.rs` already treats
undeclared opponents — plus a `with_opponents` setter. So the 116 existing
`Context::new` call sites are untouched, and only the A/B harness and
`dump-teacher` say anything about the opposition.

## Measured: the teacher configures per SIDE, and the return code is blind

Two drafts of this section disagreed, so it was settled by measurement rather
than by reading decompiled code. `examples/probe-set-conv` is the instrument;
run it against any card to reproduce. Findings, `libEPBot.so`, system 0:

| question | answer |
| --- | --- |
| is argument 2 a seat or a side? | a **side** — 0 and 1 answer, **2 and above return −2** from the setter *and* the getter |
| what does an unknown name return? | **0**, and it reads back **0** |
| are the two slots independent? | **yes** — systems (0, 8) disagree on `Landy`/`Multi-Landy` |
| rows of `cards/American.bbsa` that do not stick | **3** |

So the older `bba-kickback.md` §0 "FFI trap" note was **right** and the earlier
draft of this section was wrong: EPBot holds `cc = new TYP_SYSTEM[2]`, one
convention set per partnership, which is the natural ABI for an *agreement*.
Only `new_hand`/`set_bid` are per-seat.

`oracle.rs` was nonetheless *functionally* correct, by the accident that note
described: of each pair `[actor, actor + 2]` exactly one index is in range, and
because a side is a seat's parity it is always the right one — the other half
returned −2 and did nothing. It now names the side (`actor % 2`) instead, so
the accident is gone. `with_opponents` carries a wholly separate card, which is
how BEN's declared `.bbsa` is loaded as the other side, and the slots being
independent means **asymmetric-config corpora need no new capability**.

### Phase 0 is a read-back guard, not a return-code check

The hazard is real and this design sharpens it: `card.rs` documents that a name
EPBot does not know is a *silent no-op*. Under this design we push **135 rows
per side** — 135 chances for a renamed row to do nothing while the corpus
records the config we *intended* against a teacher playing the config we
actually got. Every row wrong in the same direction is the one failure no
downstream check can see.

**But the return code cannot detect it** — an unknown name returns 0, exactly
like a successful write. The guard has to write the card and *read it back*.
That is now `BbaOracle::verify_card`, run once on a scratch bot when a card is
accepted, leaving the per-decision path a bare write. Verified by construction:
misspelling one row of `cards/American.bbsa` as `1N-3C Puppet Staymen` fails
the load with `wrote 1, read 0 (set returned 0)`.

Three rows are allowlisted in `KNOWN_UNSTICKY`, and they are two different
problems. `South African Texas` and `Queen ask by available bid` are our own
`PONS_SCHEMA` filler names — EPBot ignoring them is exactly the property that
makes them safe to park on spare slots. `Reverse Bergen` is BBA's own: written
0, it reads back 1, an engine coupling rather than a typo of ours.
