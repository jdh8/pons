# The competitive-1NT campaign

Our 1NT opening when the opponents come in — both lanes: `1NT (X/2x)` (they
interfere directly) and `1NT - resp (..)` (they interfere after our response).
Read [bidding-architecture.md](bidding-architecture.md) first; every package
here ships (or doesn't) by [measurement.md](measurement.md).

Distinct from [competitive-book.md](competitive-book.md), whose P1–P6 are suit
openings: this lane has a different **opponent model** (their overcalls are
artificial) and a different **book/floor partition** (the floor owns the whole
`(X)` runout and every deep continuation).

## Opponent model — and the assumption we accept

BBA/EPBot, the anchor opponent, defends a 1NT opening with Woolsey
"Multi-Landy". **Every call is artificial**
(`reference_bba-1nt-defense`, distilled by sample-and-probe; BEN is a distilled
BBA-8730 and plays the same family):

| Their call | Woolsey Multi-Landy | Our book treats it as |
| --- | --- | --- |
| `X` | 4-card major **+ a longer (5–6) minor**, 12–19. **No penalty double exists** — a flat 22-count passes | a double to run from (the floor's runout) |
| `2♣` | **both majors**, ≥ 5-4 | natural clubs → **full systems-on rebase** |
| `2♦` | **Multi** — a 6+ *single* major, symmetric ♥/♠ | natural diamonds → Transfer Lebensohl's `(2♦)` leg |
| `2♥` / `2♠` | **Muiderberg** — exactly 5 in the major + a 4+ minor | natural 5+ major → Cohen Transfer Lebensohl |
| natural 2m | **never bid** — a 6-card minor one-suiter passes | — |

**Stated assumption.** We optimize for an artificial-defense field and accept
being wrong against a DONT/Cappelletti `2♦` that holds real diamonds. Routing
by the opponents' declared system is the `--declare-their-book` channel, which
measured plain-wash / **PD −0.0070** and is default-off; its own post-mortem
says the misreading was *"never the binding constraint — the calls it affects
are ones whose continuations are thin; reopening means authoring those
continuations first."* That is this campaign, so the reading channel stays out
of it until the calls are authored.

## The census — what each interference call actually costs

`examples/probe-1nt-interference` splits the 1NT slice of the anchor's
`Competitive / book / round-1` bucket by RHO's call. It reads an existing
anchor arm off the deal-keyed DD cache — seconds, no generation, no new solve:

```bash
cargo run --release --features serde --example probe-1nt-interference -- \
    ab-results/anchor/2026-08-12-ea2cde9-dirty/american-none \
    --dd-cache ab-results/anchor/dd-cache.json
# add --bucket "2♣" --show 8 for the worst boards of one bucket
```

Bucket the **shipped** arm (`american-*`, the v5 floor), not the
`american-instinct` reference arm.

**Attribution ceiling — read this before quoting a number.** The swing is the
*board's* IMPs, not the interference decision's. On these deals the same 1NT
hand opens at table B too, so the mirrored board carries our *defense to their
1NT* as well, and every later call is in there. The buckets **rank**; they do
not isolate. Isolation is a package's own A/B (`bba-gen --filter-1nt`, one
knob). The confound is broadly common across buckets, which is what leaves the
ranking usable.

### 2026-08-14, anchor `2026-08-12-ea2cde9-dirty`, 204,800 boards/vul

We open 1NT on **6.5%/6.7%** of boards; RHO contests **12.4%/10.4%** of those
(NV/vul) — so a contested 1NT is **0.80%/0.69% of all boards**.

| RHO | boards (NV+vul) | plain total | plain/bd | PD/bd NV | PD/bd vul |
| --- | --- | --- | --- | --- | --- |
| **`2♣`** Landy | 551 | **−406** | **−0.74** | **−0.70** | +0.08 |
| `2♦` Multi | 794 | −322 | −0.41 | −0.02 | +0.90 |
| `2♠` Muiderberg | 430 | −284 | −0.66 | −0.58 | +0.30 |
| `3+` | 405 | −254 | −0.63 | +0.01 | −0.13 |
| `X` Woolsey | 364 | −187 | −0.51 | −0.08 | +0.97 |
| `2NT` unusual | 118 | −139 | **−1.18** | −0.72 | −0.86 |
| `2♥` Muiderberg | 393 | −43 | −0.11 | −0.09 | +1.06 |
| **all contested** | 3055 | −1635 | −0.74 / −0.30 | −0.26 | **+0.48** |
| **uncontested 1NT** | 23868 | — | **+0.09 / −0.02** | — | — |

**Three findings.**

1. **The lane's whole headroom is ~0.004 IMPs/bd.** Contested costs
   −0.82 NV / −0.28 vul relative to *uncontested*, on 0.80%/0.69% of boards.
   Nothing here closes an anchor gap; this is hygiene and disaster removal at
   the standard ship gate, as scoped.
2. **Contested 1NT is above board average**, not a leak — −0.74/−0.30 against
   an anchor board average of −0.95/−1.16. The 1NT opening is one of our better
   boards even when contested.
3. **`2♣` is the one bucket that loses on both scorers** (plain −0.74, PD −0.70
   NV) and carries the largest total. `X` is *fine* (−0.05/bd vul, PD +0.97) —
   the floor's runout pays even though their double is Woolsey, not penalty, so
   that open question is closed. `2♦` is mild (PD −0.02 NV, **+0.90** vul).

### Named mechanism — why `2♣` loses

Over their `2♣` we play a **systems-on rebase**
([lebensohl.rs:388-405](../src/bidding/american/competition/lebensohl.rs)):
their `2♣` is stripped to a Pass and our whole uncontested response structure
goes live, with `X` transplanted onto the stolen `2♣` Stayman
([lebensohl.rs:416-425](../src/bidding/american/competition/lebensohl.rs)).
Against a *natural* club overcall that is sound and standard — `2♣` is the one
overcall that costs no space.

Against **Landy** it is actively bad. The worst boards show the structure
firing into a hand that has just shown both majors:

```text
us:  - 1NT 2♣ 2NT - 3♦ 3♠ 4♦ - 4♠ X - - -      [−18 IMPs]
us:  1NT 2♣ 2♦ 3♣ X 3♠ - 4♠ X - - -            [−10 IMPs]
us:  1NT 2♣ 2NT 3♥ X - 3NT - - -               [−10 IMPs]
```

`2♦` is a Jacoby transfer **to hearts** — one of the two suits they hold. `2NT`
and `2♠` are the minor transfers, pure constructive asks that hand them a free
run at their fit. `X` asks for a four-card major against a hand holding both.
Two of the eight worst boards end in `4♠` doubled.

### Second observation — `2NT` (worst per-board, small n)

118 boards pooled, so the CI is enormous, but the sign is consistent across
vulnerabilities (−1.18/bd plain, −0.72/−0.86 PD). The forensic pattern is that
BBA **doubles** their minors and we bid on:

```text
us:  1NT 2NT X 3♦ - - -
bba: 1NT 2NT X 3♦ - - X - - -
```

Worth a look once the top buckets are done; not actionable at this n.

## Coverage inventory

### Lane 1 — `1NT (X/2x)`, they interfere directly

| RHO | Owner | Anchor |
| --- | --- | --- |
| `(X)` | **floor, complete** — escape, business XX, 2NT scramble, SOS, balancing runout, encircling doubles | `instinct.rs:4641-4956`; `set_one_nt_runout` default-on, +0.039/+0.053 plain, 1.58% fired |
| `(2♣)` | systems-on rebase + stolen-Stayman `X` | `lebensohl.rs:388-405`, `:416-425`, `:436-446` |
| `(2♦)` | Transfer Lebensohl's Stayman/Smolen/Jacoby/Leaping-Michaels leg | `lebensohl.rs:450-462`, `rubensohl.rs:332`; continuations `lebensohl.rs:584-608` |
| `(2♥)`, `(2♠)` | Cohen Transfer Lebensohl | `lebensohl.rs:466`, `rubensohl.rs:98`; continuations `:550-566`, `:573-578` |
| `(2NT)` | Unusual-vs-Unusual | `uvu.rs:139`, `:21`, `:145-161` |
| `(3♣)`+ | **floor** — `high_overcall_responses` covers suit openings only | `high_overcall.rs:152` |

Multi counter exists but is **half-built**: `defense_2d_multi`
(`agreements.rs:171`, default `false`) swaps in `multi_responder`
(`lebensohl.rs:256`), but the continuation block at `lebensohl.rs:584` fires on
`style == Transfer && over == Diamonds` **without checking the Multi flag** —
so with the knob on, opener answers a natural `3♦` with
`transfer_completion(Hearts, ♦)`. Same mismatch for `Plain` + Multi at `:540`.
Dormant only because the knob defaults off.

### Lane 2 — `1NT - resp (..)`, they interfere after our response

Four packages, all keyed `P* 1NT - <our call>` and installed at
`competition.rs:304-318`:

| Our response | Coverage | Knob (default) |
| --- | --- | --- |
| `2♣` Stayman | authored; **2-level overcalls only** — `(2NT)`, `(3♣)`, `(3♦)`+ absent. Has the double-of-opener's-answer node | `competition_over_stayman` (**on**) |
| `2♦`/`2♥` Jacoby | partial: missing `(2♥)` over `2♦` and `(2♠)` over `2♥` — *they bid the major we are transferring to* ([over_our_jacoby.rs:100-103](../src/bidding/american/competition/over_our_jacoby.rs)) — and `(3♥)`/`(3♠)` | `competition_over_transfer` (**off**, measured loss) |
| `2♠` two-way minor | authored `(2NT)`–`(3♠)` | `competition_over_minor_transfer` (**on**, PUPPET-gated) |
| `2NT` ♦ transfer | authored `(3♣)`–`(3♠)` | `competition_over_diamond_transfer` (**on**, PUPPET-gated) |
| `3♣` Puppet, `3♦` both majors, `3♥/3♠` splinter, `3NT`, Texas `4♣/4♦`, direct `4M`, quantitative `4NT` | **absent** — floor | — |

Also absent everywhere but Stayman: the double of opener's answer
(`1NT - 2♦ - 2♥ (X)`), and *any* overcall of opener's answer.

## The book/floor line

**The book owns responder's first contested call and opener's one answer.
Everything deeper is floor, permanently.** "Complete" never means depth — a
book node with finite mass shadows the floor, and the floor is where deep
contested continuations get smarter. The deepest authored suffix in this lane
is 5 calls; the most common runout point is after opener's transfer completion
(`1NT (2♥) 3♠ - 4♠ -`), and it stays there.

## Package queue — ranked by the census

| # | Package | Knob | State |
| --- | --- | --- | --- |
| **N1** | **Landy `(2♣)` counter + N1c/d/e/f stack** | `their.two_clubs_landy` | **SHIPPED 2026-08-14** (base `wash \| win`, stack `win \| win`); was the top loser, both scorers |
| N1g | Landy **read-side** wiring — their `2♣` = majors in the floor's envelopes | `reading.their_landy_reading` | **SHIPPED DEFAULT-ON 2026-08-14** (`plain wash \| PD win` ×3 seeds, isolation gate 0 foreign); see §N1g |
| N2 | Muiderberg `(2♠)` calibration | — | −0.66/bd, PD −0.58 NV |
| N3 | `(3+)` overcalls of our 1NT | new | floor-only today; −0.63/bd |
| N4 | Multi `(2♦)` — finish + measure | `defense_2d_multi` | exists, half-built, gate bug above; plain-only loss; **owed the `their` disclosure migration N1 got** |
| N5 | Complete Jacoby, re-measure | `competition_over_transfer` | default-off on a measured loss *while missing its two most-fired cells* — a half-built loss, resumable |
| N6 | `(2NT)` penalty discipline | `uvu_encircle` et al. | worst per-board rate, n=118 — needs boards before it needs code |
| N7 | Absent responses contested | new | Puppet `3♣`, `3♦`, splinters, `3NT`, Texas, `4NT` — rarest in the system |

## N1* — the Landy `(2♣)` counter (**SHIPPED DEFAULT-ON 2026-08-14**)

The census's top loser, closed in five measured rounds in one day. This
section is the **shipped state**; the exploration that produced it is digested
at the end, and every measured verdict lives in the [ledger](#ledger).

### Engagement — a disclosure, not a knob

What their `2♣` means is a fact about the opponents, so the engagement bit is
**`their.two_clubs_landy`** in `Agreements::their` — the disclosure channel,
never our own knob space (`competition.defense_2c_landy` existed for one day
and was deleted; `defense_2d_multi` is owed the same migration). Undeclared
defaults to natural — the systems-on rebase — which self-play demands: our own
tables' `2♣` overcalls *are* natural.

A harness that knows its opponent derives the declaration. `bba-gen`'s
`their_2c_landy` plays explicit `--their-card`/`--their-conv` Landy-family
rows at **face value** (a declared no-Landy set reads natural — deviations
behind a declaration are *their* infraction) and, with no declaration,
defaults the 2/1 reference to Landy from its **measured behavior**, because
its own card lies: `21GF.bbsa` declares `Cappelletti=1, Landy=0,
Multi-Landy=0` while the engine bids Multi-Landy regardless. `bba-decompose`
applies the same correction when replaying dumps: `--landy-counter false` for
pre-N1 dumps, `--landy-stack false` for dumps generated between the
base-counter ship and the stack ship.

Structure knobs ride the declaration: `defense_2c_landy_transfer` (implies
the cues, `defense_2c_landy_cues`) plus the three repairs
`defense_2c_landy_cue_floor`, `_fit_answers`, `_competition` (each implies
`_transfer`) — **all default true**. They engage only under the declaration,
so the default *system* is byte-identical: `smoke-default --count 20000
--seed 1` SHA-256
`8ea2f5678a733cfe3ead79411d9cb31b8e95d37de52236e597fc38f9dec82bbb`, unchanged
by every ship in this package. `bba-gen`'s stack flags are `Option<bool>`
(unset = engine default; a pre-ship arm is spelled
`--defense-2c-landy-<knob> false`).

⚠ **A cue-constraint edit is a reading edit.** The mirror-read leak below
reflects the cue rows onto auctions *they* open; gate every arm pair.

**Disclosure to BBA**: its `.bbsa` schema has no row for our counter to
*their* Landy over *our* 1NT — nothing to wire in `card.rs`; golden cards and
`alert-sites.txt` unchanged.

### Responder's table over `1NT (2♣)`

`landy_responder` + overlays, `competition/lebensohl.rs`. Either/or with the
systems-on rebase, **not** an overlay — leaving the rebase registered would
remap the values `X` onto stolen Stayman a round later (the `defense_2d_multi`
gate bug).

| Call | Meaning | Weight |
| --- | --- | --- |
| `3NT` | game values, **both majors stopped, no six-card minor** | 180 |
| `3♣` / `3♦` | **INV (8-9), 6+ suit** | 176 / 175 |
| `2♥` / `2♠` (cue) | **INV+, `points(10..)`**, 5+ clubs / 5+ diamonds — alert `comp:landy-cue` | 173 / 172 |
| `3NT` | game values, ungated | 170 |
| `X` | **values**, `hcp(8..)`, penalty-oriented — alert `comp:landy-values` | 145 |
| `2♦` | weak natural, 5+, `points(..=9)` + the `natural_floor` | 140 |
| `2NT` | **transfer to clubs, weak 6+** — alert `comp:landy-transfer`; projects `len(♣,6..) & points(..=9)`, tight enough for `project_authored` | 110 |
| Pass | finite catch-all | 0 |

Design points, each measured or smoke-found:

- **The gated `3NT` outranks the cues** because opener declares any notrump
  contract (opener bid 1NT — Law 54), so responder's direct `3NT` costs no
  siding; denying a six-card minor sends sources of tricks through the cue.
- **`3NT` takes no stopper gate**: their `2♣` promises no clubs, and demanding
  a major stopper is no use — they hold both.
- **`X` floors on `hcp`, not `points`**: defending does not care about
  distribution; shapely weak hands belong in `2♦`/the transfer.
- **Cue floor 10** (N1d): at weight 173 against the double's 145, an 8+ floor
  took every 8-9 five-card-minor hand off the values double, worth
  −0.92/−2.53 PD per fired; flipping them back (cue→X, 55-60% of the repair's
  divergence) paid +2.0…+5.1 PD per fired.
- **`2NT` transfer**: their `2♣` is artificial, so clubs are ours;
  transferring puts the weak escape a level lower *and* right-sides it into
  the 15-17 hand. The package's biggest earner, and mostly a **new** call —
  the weak six-card club hand had no call at all under the base counter.
  Completion reuses `complete_lebensohl_relay()`, natural in the target and
  unalerted (`complete_advance_transfer` doctrine); responder passes it. The
  natural `2NT` invite it displaced carried almost nothing — the values `X`
  outranked it on every 8+ hcp hand.
- **`3m` INV is answered by the uncontested invite's own size decision**
  (`size_ask_accept_floor`, default 16): `3NT` from the top with both majors
  stopped, else sit — minor game is out of reach of a combined 23-26.

### Opener's answers

Natural calls (`landy_natural_answers`). Authored after the first A/B traced
its loss entirely to their absence: a call the book leaves unanswered is
phantom-completed by the floor as the default-system gadget it replaced.

| After | Opener |
| --- | --- |
| `X -` | Pass — sitting for the values double |
| `2♦ -` | Pass, always (`lebensohl_signoff_raise` doctrine) |
| `2NT -` | forced `3♣` (transfer completion) |
| `3♣`/`3♦ -` | `3NT` at `hcp(16..)` with both majors stopped, else Pass |
| `3NT -` | no node — audited clean |

Cue answers (`landy_cue_answer`, with N1e's fit answers): **level carries
strength — cheap is minimum — and every raise or ask promises 3+**. The
notrump rungs absorb the doubleton (*both majors stopped, or ≤2 support*) and
the terminal catch-all is `2NT`, so the 5-2 raise (measured −10/−8 PD per
fired) cannot be manufactured. A stopper is guaranteed only alongside a fit;
responder knows which story a rung told from the rung itself.

| Opener | Shows | Weight |
| --- | --- | --- |
| `3NT` | maximum — both majors stopped, or ≤2 support | 160 |
| `3♥` / `3♠` | maximum, asks for the stopper opener lacks, 3+ in the minor | 155 |
| `4m` | maximum, 3+, neither major stopped | 150 |
| `2NT` | minimum — both stopped or ≤2 support (terminal catch-all) | 145 |
| `2♠` | minimum ask — club cue only, the one rung below the 3-level | 140 |
| `3m` | minimum raise, 3+ | 100 |

Responder answers an ask by showing the stopper (cheaply on a minimum, so
opener still judges game) or retreating to the minor made safe. Over the
minimum `3m`, responder may **re-cue** `3♥`/`3♠` with a game force and a
stopper worry — opener bids `3NT` holding it, else takes the minor. Over
`2NT`, pass or `3NT`. **`4♣`/`4♦` over opener's minimum rebids is a slam try**
(13+ with a six-card suit); opener's continuation is deliberately the floor's —
a `4m` *suit* contract lets the floor cue-bid on to slam where a notrump rung
dies in `3NT`. Every other rung is authored down to the placing call, because
`Inferences` has no forcing channel: a rung left to the floor reads as bare
"5+ ♣, 8+ points" with no notion of an invitation.

### Competition over the counter (N1f)

Three shapes; everything deeper stays the floor's, deliberately:

- **Their `X` of a cue or ask** takes no room, so the answer is **verbatim**
  (the immediate table re-registered on the `(X)` suffix), and every deeper
  X-then-bid tail is stripped back onto the clean subtree by a
  `systems_on_over_double` rebase — the contested-Stayman idiom, one entry
  covering asks, rebids and re-cues.
- **Their raise over a cue** (`2♠`/`3♥`/`3♠`): compressed ladder — `3NT` =
  both stopped + maximum, the raise = 3+ by size, Pass = the rest (safe
  because responder is INV+ and guaranteed another turn).
- **The doubled club transfer** is still completed, sign-off intact.

`(4♠)`, overcalls of opener's answers, and everything deeper: floor.

### Ship evidence

Two ships, both at the standard gate; full per-version verdicts in the
[ledger](#ledger).

**The base counter** (N1, `their.two_clubs_landy` + `landy_natural_answers`):
plain wash + PD CI-clear win in both vuls — NV PD +0.0032 ±0.0028, vul
+0.0043 ±0.0032 (76.8k bd/arm/vul), **confirmed at 3× n** (PD +0.0032/+0.0028,
230.4k bd/arm/vul) after a single-seed non-replication scare.

**The stack** (N1c transfer + N1d/e/f repairs) against the shipped base,
pooled over seeds 1786694464 + 1786695954, 460.8k bd/vul — the package's
first `win | win`:

| `f↔on` pooled | plain DD | PD |
| --- | --- | --- |
| NV all | **+0.00068 ±0.00062** | +0.00075 ±0.00077 |
| NV ours | **+0.00091 ±0.00052** | **+0.00077 ±0.00064** |
| vul all | **+0.00085 ±0.00072** | **+0.00100 ±0.00087** |
| vul ours | **+0.00075 ±0.00058** | +0.00060 ±0.00070 |

Six of eight DD cells CI-clear positive, 8/8 sd cells positive at both seeds,
**no negative cell in 24 readings**. Increment attribution
(230.4k bd/arm/vul, seed 1786694464):

| increment | fired (NV/vul) | verdict |
| --- | --- | --- |
| `d↔xfer` cue floor | 169 / 136 | **the engine**: plain wash + PD **+0.0009 ±0.0008** NV / **+0.0015 ±0.0009** vul (+1.21/+2.49 per fired) |
| `e↔d` doubleton-NT answers | 3 / 1 | no population left post-floor; ships on naturalness (raises promise 3+; the alternative was a priced −10/−8 PD 5-2) |
| `f↔e` interfered tails | 17 / 11 | CI-wide wash; ships as the iron rule's convention-completion |

`ab-landy-counter.sh`'s arms are spelled in post-flip terms (landy-f *is* the
default; landy-on switches the stack off); the confirm pair is
`scripts/ab-landy-confirm.sh`.

### The mirror-read leak — open defect, gated

`read.rs` gates its 1NT sites on parity **relative to the opener, not to us**
([read.rs:386-389](../src/bidding/inference/read.rs)), and `their_profile`
falls back to *our own* profile whenever no foreign book is declared
([read.rs:333-335](../src/bidding/inference/read.rs)) — every arm in this
campaign. So when **they** open 1NT and **we** overcall `2♣` (our own Landy),
their next call is read through *our* counter table: under the house rule that
a knob picks what we bid, not how we read their bidding, the symmetric
fallback quietly makes **every counter knob a reading knob as well**.

Measured at 21–43% of divergent boards across this package's A/Bs. The IMP
impact was neutral-to-depressing on every headline (no verdict flipped; the
our-opened subsets are the honest figures, and they were *stronger*). **Now
enforced**: `probe-divergence --gate-opener ours` exits non-zero unless every
divergent board was opened by our side (`theirs` for a defensive package).
Pure book edits (`e↔d`, `f↔e`) pass at zero foreign boards; cue-*constraint*
edits fail. Run it on every arm pair in this lane; the real fix is splitting
`their_profile` from our own.

### Residue

- **Their *second* call still floors us** — **diagnosed and half-closed by
  N1g** (next section): N1f's worst board was
  `1NT (2♣) 2♥ (2♠) 3♣ (3♠) 4♦ - 4♠ X` — the floor bidding a phantom `4♠`
  off an envelope that claimed LHO held five clubs (−17 PD). The wiring
  fixes the inputs (and the N1g probes show phantom contracts being
  *corrected*); whatever phantom-bidding remains after a post-ship decompose
  is the true floor-discipline residue — an M6.4-style rail
  (conversation-in-motion → instinct, or an envelope-gated new-suit veto
  scoped off agreed fits), not another node ring.
- **The `3♣`→`2♥` row is CLOSED** (re-probed 2026-08-14 against the shipped
  stack on the N1g dumps, `read-on ↔ landy-on`): the pre-stack −1.09/−3.26 PD
  loss is gone — vul both **+3.12 plain / +2.38 PD per fired**, NV a small
  mixed wash (+0.42/−0.25), n=20 flips. No forcing-3m arm is warranted; the
  residual worst boards are the known `{cue}`→`4♥`-instead-of-3NT artifact,
  2-3 per vul.
- `{cue} - 3NT -` and `{cue} - 4m -` (opener's maximum rungs) remain
  unauthored; never surfaced in a dump.

### N1g — the read-side wiring (**SHIPPED DEFAULT-ON 2026-08-14**)

Decomposing the residue found **the floor's inputs are lies in this lane**:
`their.two_clubs_landy` had *zero read-side consumers*.  The disclosure moved
the book only; opponent-call decoding falls back to our own profile
([read.rs:333-335](../src/bidding/inference/read.rs)), whose shipped defaults
read their `2♣` through the natural walk as **5+♣, 8+** — so on every board
of this lane the learned floor's LHO envelope (its `features_v5` inference
block) claimed five clubs while LHO held both majors.  The residue boards are
exactly the floor's boards; the phantom `4♠` was bid off a false deal
picture, before any question of the net's weights.

The wiring, `ReadingProfile::their_landy_reading` (**default on**; the
pre-ship arm is `bba-gen --ns-their-landy-read false`): under the declaration
their `2♣` reads 4-4+ in the majors with no strength claim, their
`2♦`/`2♥`/`2♠` advances and direct `3M` raises are natural-suppressed (a
preference plays on a doubleton; the `3M` would otherwise read as a weak-jump
six-carder).  Implemented as a seat-gated hand reader
(`inference/readers.rs::their_landy_reading`) that fires only when the `1NT`
opener is on the *reader's* side — our own `2♣` overcalls cannot match — and
that does **not** extrapolate through the systems-on strip.  The disclosure
itself re-homed to `DecisionProfile::their` per the dual-read house rule,
proven byte-identical (smoke `8ea2f567…` unchanged).

**Ship evidence** (three seeds pooled, 230.4k bd/vul, `read-on ↔ read-off`,
`scripts/ab-landy-read.sh`, seeds 1786704432 / 1786705413 / 1786705763):
plain **wash** (NV −0.00051 ±0.00072, vul +0.00001 ±0.00078), PD **win both
vuls** (NV **+0.00104 ±0.00097**, vul **+0.00112 ±0.00104**; ≈ +1.0/+1.5 per
fired at 0.07–0.11% fired), sd agreeing in sign (sd-PD +0.00065/+0.00076,
sd-plain wash) — the decision table's `plain-wash + PD-win` ship row.  The
**isolation gate passed at zero foreign boards in both vuls** — the first
pair in this campaign to do so.  Mechanism (from the divergence probes): a
conservative shift off true envelopes — fewer thin NV games/slams (plain DD,
the optimism bound, dislikes exactly those; PD likes them), and partner's
phantom-`4♥` contracts *corrected* to the real fit (+17 PD boards).

Two lessons paid for en route: **seed 1 of the fixed build showed a CI-clear
NV-plain loss that seeds 2–3 refuted** (single-seed negatives are not design
inputs — again), and **v1 of the reader leaked through the systems-on
strip**: in `(1♣) 1NT (2♣)` lanes the strip re-reads our 1NT overcall as an
opening, the seat gate passed, and their *responder's* 2♣ read as Landy.  The
v1 worst boards were all this leak; the fix pins the disclosure out of the
strip recursion (`read.rs`, regression-tested).

A sibling defect found in the same sweep: the forced `3♣` completion of a
sohl `2NT` relay is `hcp(0..)` — it projects nothing, dodges the alert
invariant's artificiality witness, and reads as **four real clubs** where no
blanket covers it.  That lane is *advance-sohl* (their weak two, our takeout
`X`, the relay), not this one — after our own `1NT` opening the walk blankets
the whole structure, so plain Lebensohl and the N1c transfer completion are
latent.  The knob grew into the family `reading.completion_alerts`
(2026-08-14, superseding `lebensohl_completion_alert`; **shipped default-on
the same day** — `scripts/ab-completion-alerts.sh`, unfiltered, pooled over
three seeds at 614.4k boards/cell: vul plain +0.0005 ±0.0004 and vul PD
+0.0006 ±0.0005 both CI-clear, NV positive, sd sign-agreed): it alerts the
puppet (decodes ⊤, suppresses the club read)
and the rest of the completion family with it.  Never fold its arm into
N1g's.

### How it got here — exploration digest

Five measured rounds, all 2026-08-14; numbers in the ledger, probe files in
`ab-results/landy-*`.

1. **The first A/B lost all six cells** — not the idea, two leaks: opener's
   answers were unauthored (the floor phantom-completed each natural call as
   the gadget it replaced: Jacoby `2♥` 82% over the weak `2♦`, Puppet `3♦`
   85% over `3♣`), and the census had misread systems-on's minor transfers,
   which were *winning* the minor-partial boards. `landy_natural_answers`
   closed leak 1 and the base counter shipped `wash | win`.
2. **The UvU-style GF cue overlay (N1b) washed four times.** The `1♣ (2♣)`
   analogy held as *isomorphic, not identical* — expert counter-Landy
   structures (Cohen, Walker) independently reproduce the values-`X` +
   GF-minor-cues core, but the 15-17 captaincy re-spends the raise half; the
   minor-opening side of the skeleton is P7 in
   [competitive-book.md](competitive-book.md). `probe-divergence` decomposed
   the wash into four effects with different signs: the weak `3♣` escape was
   the earner, the cues the losers — first a sub-game cue answer (missed
   slams), then the poached values double, then the fit forensic (5-2 raises
   at −10/−8 PD per fired; interference dropping mid-convention auctions to a
   floor with no forcing channel).
3. **N1c re-spent the rungs the decomposition named** — weak escape → `2NT`
   transfer, weak `3♦` deleted, natural `2NT` invite deleted, direct 3m →
   INV six-carders — and was the first arm to substantially pass the
   isolation gate (0.8% foreign boards vs N1b's 27%).
4. **N1d/e/f repaired the cue** (floor 10, doubleton-NT answers, interfered
   tails), and the stack went `win | win` pooled over two seeds — the whole
   five-arm, two-vul final round ran in 19 minutes off the enriched filter.

Lessons the next package inherits: **decompose a wash before theorising** —
none of this package's washes was one effect; a **single-seed negative is not
a design input** (a vul-PD −0.0010 that drove a day of worry did not
replicate); and an artificial call is not complete until both sides'
continuations *and the interfered tails* are authored — every loss this
package ever measured was an unauthored continuation, never the idea.

## Measurement discipline

- **Counter-defense isolation gate:** on identical seeded deals, configuring
  the candidate against a natural defense must leave the auction dump
  byte-identical to the natural baseline.  Require natural interference to
  occur and the targeted artificial-defense arm to diverge, so the check is
  not vacuous; a face-call-wide reinterpretation (`2♣` always means Landy)
  fails this gate.  **Ownership half, automated:**
  `probe-divergence --gate-opener ours` fails the pair unless every divergent
  board was opened by our side — the half that caught nothing until
  2026-08-14 because it was discipline with no implementation.
- **Enriched probing** is the default here: `bba-gen --filter-1nt` (raw-hand
  gate, balanced 15-17 somewhere, applied *before* any bidding). Headline is
  IMPs per **accepted** deal; publish `per-board = conditional mean × trigger
  density` alongside and scale the CI the same way. Compare IMPs/divergent, not
  IMPs/board.
- One knob = one measured change; arms **sequential**, fresh
  `SEED_BASE=$(date +%s)` per experiment shared across its arms, never rebuild
  in flight.
- Any **reading** change is a second, separate A/B on the same enriched boards,
  so a loss attributes to calls or to reading, never to their sum. A counter
  knob is *also* a reading change until the mirror-read leak above is closed.
- **Decompose a wash before theorising about it.**
  [`probe-divergence`](../examples/probe-divergence/main.rs) pairs two arm dirs
  already on disk and classifies every divergent board — who bid differently
  first, whether a game was reached in one arm only, whether declarer swapped
  sides, how much room the opponents got — with `--jsonl` for per-board records
  and `--imps` to price a bucket. It reproduced this campaign's published
  headline exactly and then split it into three populations with different
  signs. Counting needs no solver at all.
- Ship rule: standard gate. Plain-DD wash + PD gain ships default-on; a CI-clear
  plain loss stays opt-in with the default byte-identical, and the leak gets
  named in the ledger.

## Ledger

| Package | Knob | Status | Verdict (plain / PD, IMPs) |
| --- | --- | --- | --- |
| census tool | — | **shipped** | read-only; picked N1 over the pre-census guess |
| N1 Landy `(2♣)` counter | `their.two_clubs_landy` (disclosure, not a knob; `defense_2c_landy` deleted) | **SHIPPED 2026-08-14** — engine undeclared=natural; `bba-gen` derives the declaration (2/1 reference → Landy, its card lies) and `bba-decompose` replays it; re-homing proven board-identical on the measured arm | **v2 (with `landy_natural_answers`, full audit fix)** on↔off: NV plain +0.0005 ±0.0022 / PD **+0.0032 ±0.0028** (+1.10/fired) / sd −0.0006 ±0.0025, SD-PD +0.0017 ±0.0030; vul plain +0.0013 ±0.0026 / PD **+0.0043 ±0.0032** (+1.65/fired) / sd +0.0001 ±0.0029, SD-PD +0.0030 ±0.0034. 0.26–0.30% fired, 76.8k bd/arm/vul, SEED_BASE 1786644715, sha 40a0946 — plain wash + PD CI-clear both vuls = ship. **Confirmed at 3× n** (230.4k bd/arm/vul, SEED_BASE 1786653231): NV plain −0.0002 ±0.0013 / PD **+0.0032 ±0.0017**, vul plain +0.0003 ±0.0015 / PD **+0.0028 ±0.0019**, sd-PD +0.0019/+0.0015 — the v3a run's non-replication was seed noise; NV is the stronger vul. **v1 (sha 8bc465a, SEED_BASE 1786642613): LOSS all six cells** (NV plain −0.0050 ±0.0024, vul −0.0049 ±0.0028, every CI<0) — leak 1: opener's answers unauthored, audit: phantom Jacoby `2♥` 82% over `2♦`, phantom minor-transfers 23% over `2NT`, phantom Puppet `3♦` 85% over `3♣`, passed force 62% over `3♦` (only `3NT` clean); leak 2: the census misread (systems-on's minor transfers were winning the minor-partial boards). |
| N1b GF minor cues | `defense_2c_landy_cues` | **measured 2026-08-14 ×4 — stays opt-in, but v4 is the first arm with a CI-clear positive and no negative cell**. **v4** (INV+ cues + level-as-strength stopper asks, 230.4k bd/arm/vul, SEED_BASE 1786653231, sha 8873e9c+dirty): NV plain **+0.0016 ±0.0010** / PD +0.0001 ±0.0013, vul plain **+0.0014 ±0.0012** / PD −0.0000 ±0.0014, sd plain **+0.0018/+0.0024**, SD-PD +0.0006 / **+0.0016 ±0.0015**. Lands on `win \| wash` (artifact row); SD splits vul-real / NV-artifact. `probe-divergence` decomposes it into four independent effects, replicating across both vuls (per fired, we-opened, NV/vul): `3♣` weak clubs **+3.41/+2.98 plain, +0.71/+0.61 PD**; `2♠` diamond cue **+1.87/+1.93**, +0.48/+0.12; `2♥` club cue −0.17/+0.06, **−0.76/−1.04 PD**; `3♦` weak diamonds −1.07/+0.55, **−1.71/−0.55 PD**. `2♥`'s whole loss is **9 boards** where we declare *hearts* — `{cue} - {ask} (X)` passed out (every registration ends in `-`) and the floor bidding `4♥` itself on non-book continuations; the other 162 are +81/−22 = wash. Mirror leak persists at 25–31%, PD-positive, `--gate-opener ours` fails. v2 = the *full* UvU skeleton (cues carry all GF one-suiters, direct 3m weak) on the fixed base; the `probe-divergence` post-mortem decomposed the v2 wash — cues −1.76 plain/−1.90 PD per fired (missed *slams*, from a sub-game cue answer), weak escapes +0.54/−1.59 (PD-negative vul only = going for a number), and 38% of divergences on boards the opponents opened (the mirror-read leak). v3a = opener's cue answer restored to `landy_minor_answer` (game level), −0.34…−0.62/fired, still every cell negative | **v2 (full skeleton, N1-win run)** cues↔on: NV plain −0.0005 ±0.0013 / PD −0.0006 ±0.0018, vul plain −0.0007 ±0.0016 / PD −0.0012 ±0.0021, sd −0.0004/−0.0012 (−0.43…−1.25/fired, 0.10–0.11% fired, all CIs ⊇ 0, every cell leaning negative). **v1 (pure cue addition, N1-loss run):** NV plain −0.0001 / PD +0.0004, vul plain +0.0001 / PD +0.0005, sd negative — unpriceable next to phantom sibling answers. |
| N1c club transfer + INV minors | `defense_2c_landy_transfer` (implies the cues) | **measured 2026-08-14 — opt-in for a day on the artifact row, then SHIPPED DEFAULT-ON 2026-08-14 as part of the N1d/e/f stack (next row), whose pooled `win \| win` retired the PD hesitation.** Increment over N1b (`xfer↔cues`, 230.4k bd/arm/vul, SEED_BASE 1786657996, sha f313f3d+dirty, 0.06–0.07% fired): plain wash + **PD +0.0008 ±0.0008 NV / +0.0011 ±0.0008 vul** (+1.10/+1.90 per fired), SD wash both. **Passes the isolation gate in substance** — 1 of 132 divergent boards opened by them (0.8%) vs 27% for N1b. Against the *shipped* counter (`xfer↔on`), pooled over two seeds (1786657996 + 1786659297, 460.8k bd/vul): plain **+0.0013 ±0.0007** NV / +0.0007 ±0.0008 vul, PD +0.0005 ±0.0009 / −0.0005 ±0.0010, plain SD **+0.0018 ±0.0008** / **+0.0011 ±0.0009**, SD-PD **+0.0012 ±0.0009** / +0.0000 ±0.0010 — four CI-clear positives, no CI-clear negative; seed 1's vul-PD −0.0010 did NOT replicate (−0.0001). Opt-in because plain/SD-win + PD-wash is the artifact row. Residue named: the cue poaches the values double (`X`→`2♠` −3.83 PD/fired vul, `X`→`2♥` −2.63) because it is `points(8..)` at weight 173 against X's 145 — N1c wins by pulling hands off it (`2♠`→`3♦` +6.06, `3♦`→pass +3.38, `2♥`→`3♣` +2.22, transfer +0.67). **N1d = raise the cue floor to `points(10..)`.** | `xfer↔cues` NV plain +0.0002 ±0.0006 / PD +0.0008 ±0.0008 / sd −0.0002 / sd-PD +0.0002; vul plain +0.0003 ±0.0006 / PD +0.0011 ±0.0008 / sd −0.0003 / sd-PD +0.0002. |
| N1d/e/f cue repairs | `defense_2c_landy_cue_floor` + `_fit_answers` + `_competition` (each implies `_transfer`; all four now default **true**) | **SHIPPED DEFAULT-ON 2026-08-14** — the package's first `win \| win`. Stack vs shipped base (`f↔on`), pooled seeds 1786694464 + 1786695954, 460.8k bd/vul: **six of eight DD cells CI-clear positive, 8/8 sd cells positive, no negative cell in 24 readings** (table in §Ship evidence). Engages only under the `their.two_clubs_landy` declaration — default system byte-identical, smoke `8ea2f567…` unchanged; `bba-gen` stack flags are `Option<bool>` (pre-ship arm = `--defense-2c-landy-<knob> false`), `bba-decompose --landy-stack false` replays between-ships dumps. Increment attribution: **N1d is the engine** (`d↔xfer` plain wash + PD **+0.0009 ±0.0008** NV / **+0.0015 ±0.0009** vul, cue→X = 55-60% of divergence at +2.0…+5.1 PD/fired — the poached-double rows reversed); N1e fired 3+1 boards post-floor (ships on naturalness: raises promise 3+); N1f the expected CI-wide wash (ships as the iron rule's convention-completion). Isolation gate: e/f pass at 0 foreign; d and f↔on fail at 18-43% (the cue-constraint mirror leak), foreign boards *depress* the headline — our-opened figures are stronger. Residue: their **second** call still floors us (phantom `4♠` one level deeper, −17 PD, 1 board); the `3♣`→`2♥` GF-six-carder row unread against the shipped stack. | `f↔on` pooled: NV plain **+0.00068 ±0.00062** / PD +0.00075 ±0.00077, vul plain **+0.00085 ±0.00072** / PD **+0.00100 ±0.00087**; ours-only NV plain **+0.00091 ±0.00052** / PD **+0.00077 ±0.00064**, vul plain **+0.00075 ±0.00058** / PD +0.00060 ±0.00070. |
| N1g read-side wiring | `reading.their_landy_reading` (default **true**) | **SHIPPED DEFAULT-ON 2026-08-14** — the disclosure finally read: their `2♣` = ♥4+/♠4+ (no strength claim), advances + direct-3M suppressed, via a seat-gated hand reader that cannot fire on our own `2♣` and does not extrapolate through the systems-on strip (v1 leaked there — `(1♣) 1NT (2♣)` read responder's 2♣ as Landy; fixed + regression test). `TheirDisclosures` re-homed to `DecisionProfile::their`, byte-identical. Pooled 3 seeds (1786704432/1786705413/1786705763, 230.4k bd/vul, 0.07-0.11% fired): plain wash, **PD win both vuls**, sd agreeing in sign — the `plain-wash \| PD-win` ship row. **Isolation gate: 0 foreign boards, both vuls — the campaign's first.** Mechanism: conservative shift off true envelopes (fewer thin NV games; phantom `4♥` corrected to the real fit). Fixed-build seed 1 showed a CI-clear NV-plain loss that seeds 2-3 refuted. The `3♣`→`2♥` re-probe rode the same dumps and closed (wash-to-win). | pooled: NV plain −0.00051 ±0.00072 / PD **+0.00104 ±0.00097**, vul plain +0.00001 ±0.00078 / PD **+0.00112 ±0.00104**; sd-plain −0.00053/−0.00024, sd-PD +0.00065/+0.00076. |
