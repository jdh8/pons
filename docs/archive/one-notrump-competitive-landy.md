# Competitive 1NT — the Landy `(2♣)` sequel, 2026-08-28 → 2026-09-24

> **Archived 2026-09-24.** Every section here is a **closed verdict** — an
> A/B pinned to a sha and a seed, or a census that killed an idea at step 0 —
> from the second Landy campaign: the doubler's rebids (§N1l, §N1l-flip),
> opener's rebid over their advance (§N1m), the unlimited values double
> (§N1p), Lia's counter-defense (§N1-lia, four packages), the strength-sorted
> two-level majors (§N1q, five runs) and the `3♦`+ idea queue (§N1r).  The
> first Landy campaign (N1–N1j, 2026-08-14/15) is in
> [one-notrump-competitive-closed.md](one-notrump-competitive-closed.md).
> The lane's **current state** — which knobs are on, what is still owed —
> is [one-notrump-competitive.md §N1](../one-notrump-competitive.md#n1--the-landy-2-counter-shipped-default-on-2026-08-15);
> the pooled numbers are in [its ledger](../one-notrump-competitive.md#ledger).
> Nothing here is re-run for freshness.

### N1l — the doubler's own rebid (`landy_doubler_rebids`, **measured 2026-08-28: mixed, stays off**)

Salvaged from `park/landy-kk` as its own default-off knob; the branch's other
three (`landy_splinter_hcp`, `landy_tail_completion`, `defense_2c_landy_kk`)
stay parked. Every weight and gate below was re-derived against `main`'s code
rather than carried over from the branch's tables.

The seat: our doubler's own rebid after the values `X`, once their advance has
named the major — `1NT (2♣) X (2♥) - -`, `X (2♠) - -`, and the two legs where
their artificial `2♦` escape was pulled to a major. The 2026-08-27 census
prices the branch at 67 bd / −75 IMPs plain, the auction dying after our double.

**The polarity rule this authors, and why it needs a node.** Our subsequent `X`
is penalty after our own `X`, and stays the floor's takeout after our `P`.
Nothing mechanises that. `inference::readers::penalty_x_reading_with_profile`
requires **their** 1NT opening — it scans forward from `opening_index` and
returns `None` on the first non-pass that is a bid — so it returns `None` at
`1NT (2♣)` exactly as it does at `1NT (2♦)`, and `penalty_latch` (default on)
therefore cannot latch anything in either lane whatever its setting. `pdi_latch`
is default off, reading-only and measured inert (`docs/pdi.md:284`). The `1NT
(2♦)` twin's penalty meaning is 100% authored node — the rule's own
`len(major, 4..)` read back through the ordinary projection plus
`.alert(MULTI_PENALTY)` — and this lane copies that, not a latch.

**The table** (`landy_doubler_rebid`, all four legs), `kokish_kraft_doubler_rebid`'s
ladder ported one suit down:

| call | w | gate | note |
| --- | ---: | --- | --- |
| `4NT` | 160 | `hcp(16..)` | quantitative; answer = `multi_quant_answer` (`6NT` on 17+) |
| `X` | 155 | `len(their major, 4..)` | **penalty**, `LANDY_PENALTY` + `.penalty()`; opener sits (`multi_signoff_pass`) |
| `3NT` | 150 | `points(10..) & stopper_in(major)` | |
| `2NT` | 145 | `hcp(8..=9) & stopper_in(major)` | invite; answer = `kokish_kraft_invite_answer` (`3NT` on 16+) |
| `3♣`/`3♦` | 100/99 | `len(minor, 5..)` | natural — replaces the twin's other-major rung, which this opponent's 4-4+ major shape makes impossible |
| `P` | 0 | catch-all | |

Two structural differences from the twin, both probe-driven. There is **no
`ran` fork**: the Landy overcaller passes the preference 94.5%/96.7% of the
time (probed at seat 1, hands filtered to the ones BBA actually overcalls `2♣`
with), so the preference is final and there is no correction to fork on. And
the twin's weight-100 *other major* becomes a natural **minor**, because this
opponent holds both majors; that rung is also the only route for an 8–9
one-suited minor, the wide transfers above it being game-forcing.

**The top two rungs are dead in self-play, and are kept anyway.** Unlike the
Multi twin — whose `3NT`@150 needs *both* major stoppers, so a one-stopper game
hand really does double first — `landy_bba_responder` carries an **ungated**
`3NT`@168 on `points(10..)`. Every 10-plus-point hand bids `3NT` directly and
never doubles, capping the double at nine points. Verified, not assumed:
`probe-call-reading --their-2c-landy --ns-landy-doubler-rebids` reads partner
back as `points 8..9` at every rung of this table. So `4NT`@160 and `3NT`@150
can only fire opposite a partner not bidding this table. They stay because the
table must be **total** — deleted, a strong hand here would take `Pass`@0,
strictly worse than the floor this node shadows. What fires in self-play is
`X` / `2NT` / `3♣` / `3♦` / `Pass`.

**Verified on the shipped tree.** `probe-decision` prints `fallback: Some(0)`
at all four nodes with the knob off and a depth-2 book node with it on;
`probe-call-reading` reads the penalty `X` as `♥ 4..13` and the `2NT` below it
as `♥ 0..3` (denying the double it declined); `smoke-default --count 20000
--seed 1` is byte-identical to `main`.

**As measured (2026-08-28).** `scripts/ab-landy-doubler-rebids.sh`,
`SEED_BASE 1787917699`, sha `ba003a30`, 4,608,000 bd/arm/vul (24 × 192,000),
isolation gate **0 foreign at both vuls**; fired 1.56% none / 1.26% both.
IMPs/fired; every /board CI ≤ ±0.0008:

| cell | DD plain | DD-PD | SD plain | SD-PD |
| --- | ---: | ---: | ---: | ---: |
| none | **+2.365** | −0.759 | **+3.059** | **+0.523** |
| both | **+1.556** | −2.281 | **+2.539** | **−0.741** |

The knob is mixed-direction (one rung adds doubles, the rest bid more), so the
row read is per rung — `probe-divergence --jsonl --imps` on each cell, split by
`call_on`, DD-priced, ±95% CI on the /fired mean:

| rung | none: plain / PD | both: plain / PD |
| --- | --- | --- |
| penalty `X` (~12%) | **+7.489**±.083 / −0.091 | **+9.196**±.094 / −0.148 |
| `2NT` invite (~36%) | +1.888±.055 / −1.667 | +0.733±.088 / **−3.695** |
| `3♣` + `3♦` (~46%) | +1.7..2.0 / −0.6..−0.8 | +0.2..+0.3 / −2.5..−3.1 |
| `-` (table passes, floor bid) (~6%) | −0.445 / +2.929 | +1.206 / **+5.048** |

Falsifier 3 is **refuted**: the penalty `X` is the payoff, not the artifact —
it carries the *entire* vulnerable plain win (+63,332 of +90,066 IMPs), the
addendum arbitrates it on plain DD, and even its double-blind PD column is
flat. Falsifier 1's "small win" shape did arrive, but concentrated: the
authored ladder beats the floor's improvisation exactly where it doubles. **The
drag is the constructive family, vulnerable.** Every constructive rung is
win/loss on the ordinary rows at none but wash-to-thin/loss at both; the
arbiter (SD-PD) flips sign with vulnerability; and the `2NT` invite's declined
half (`2NT` passed out, n=10,051) loses **both scorers** at both vul
(−0.612±.087 plain / −4.313±.130 PD) — the measured mistake is declaring a
thin vulnerable notrump part-score instead of defending their `2♥`, not the
accepted `3NT`s alone.

**Verdict: mixed — not shippable default-on as built; the knob stays off.**
The flip plan is cheap and finished-code-shaped: keep `X`@155 and the
catch-all, tighten or vulnerability-gate the constructive rungs (the `2NT`
invite first), re-measure. A vulnerable band hand would then defend via
`Pass`@0, which the `-` row prices positive on both scorers at both vul
(selection-biased — those are hands the floor chose to bid — but the sign is
encouraging). Opener's max-with-stopper `3NT` jam one seat earlier was
considered against this data and stays with flagged item 1: opener already
declares every notrump ending in both arms (opener bid `1NT` first), so a jam
moves nothing on same-contract boards, and its two live deltas — thin games
added opposite the 7-point doublers, doubler penalty-`X` boards removed —
both point the wrong way here.

### N1l-flip — the two cut-down arms (`landy_doubler_px` **SHIPPED DEFAULT-ON 2026-08-29** / `landy_doubler_white` **not a win, stays off**)

The measurement above is a verdict **per rung**, so the flip is a choice of
*subset*, not a new table. `landy_doubler_rebid` takes a `DoublerLadder` and
three knobs name three subsets of the same four nodes:

| arm | knob | rungs, top to bottom |
| --- | --- | --- |
| `px` | `competition.landy_doubler_px` | `X`@155 `len(major, 4..)` · `Pass`@0 |
| `white` | `competition.landy_doubler_white` | `X`@155 · `3NT`@150 `points(10..) & stopper_in` · `2NT`@145 `hcp(8..=9) & stopper_in & !vulnerable()` · `3♣`@100 / `3♦`@99 `len(minor, 5..) & !vulnerable()` · `Pass`@0 |
| `full` | `competition.landy_doubler_rebids` | the ladder as measured, kept as the comparison arm |

**The axis is vulnerability, not the rung.** Re-reading
`div.reb.vs.base.*.jsonl` grouped by first differing call — the plan's
"adjust before building" step — moved the design. Per fired, plain / PD:

| rung | share | non-vulnerable | vulnerable |
| --- | ---: | --- | --- |
| `2NT` | 36% | +1.888 / −1.667 | +0.733 / **−3.695** |
| `3♣` | 25% | +1.953 / −0.797 | +0.338 / −3.071 |
| `3♦` | 21% | +1.696 / −0.607 | +0.183 / −2.481 |
| `X` | 12% | **+7.489** / −0.091 | **+9.196** / −0.148 |
| `-` (table passes) | 6% | −0.445 / +2.929 | +1.206 / **+5.048** |
| `3NT` | 0.02% | −2.105 / −3.579 (n=19) | +0.222 / −0.222 (n=9) |
| `4NT` | 0% | never fires | never fires |

Every constructive rung flips sign with colour, and the natural minors are the
*cheaper* half white, not the drag: `3♦` costs −0.607 PD per fired against the
`2NT` invitation's −1.667. So the first sketch of this flip — delete the
minors, gate the `2NT` — would have kept the worst white rung and dropped the
best two. `white` gates the whole family instead. Only `4NT` is deleted
outright (it never fired in either cell, because responder's ungated `3NT`@168
caps this double at nine points), and `3NT`@150 stays ungated as the table's
only game rung on 28 fires in 9.2M boards.

**Keeping the minors pays §N1l's completeness debt.** `{path} 3♣ -` and
`{path} 3♦ -` were the Multi-twin hole — authored rungs with a floor-owned
continuation. `landy_minor_rebid_answer` closes it: responder is capped at 8–9
and bid the minor *below* the `2NT`@145 invitation, which denies the stopper
that rung requires, so the stopper must be opener's and 16 opposite 9 is the 25
that bids the game — `3NT`@100 on `hcp(16..) & stopper_in(major)`, else `Pass`.
Total. Note this makes the `full` arm no longer bit-identical to the one
measured on 2026-08-28: it gains the same two answer tables. Its historical
numbers were measured without them.

**Answer tables move with their rungs.** `{path} X -` (opener sits for the
penalty double) is registered by every arm, because every arm carries the `X`;
`{path} 2NT -`, `3♣ -` and `3♦ -` by `white` and `full`; `{path} 4NT -` by
`full` alone. A node with finite mass shadows the floor
([bidding-architecture.md](../bidding-architecture.md)), so an answer to a
question no arm asks is not merely dead — it is a live book node standing in
the floor's way.

**The vulnerability gate is real plumbing.** `vulnerable()`
(`constraint.rs`) reads `Context::vul()` and is already used by
`points_by_vul`; `& !vulnerable()` on each constructive constraint is the whole
gate, and it renders in the disclosure (`8–9 HCP, stopper in ♥, and not
(vulnerable)`). It is *our* side's vulnerability, the axis the measurement
moved on.

**What the A/B can and cannot separate.** The arms are generated at `-v none`
and `-v both` only, so the gate makes `white` ≡ `full`-minus-`4NT`-plus-answers
in the white cell and `white` ≡ `px`-plus-`3NT` in the red one. The white
`white vs px` pair therefore isolates the whole constructive family cleanly,
and `probe-divergence --jsonl --imps` split by `call_on` separates the rungs
inside it for free; the red pair is a near-empty consistency check.

**Falsifiers** (`scripts/ab-landy-doubler-flip.sh` states them in full): (1)
the `X` win was selection — both subsets were *chosen from* seed `1787917699`'s
split, so a fresh `SEED_BASE` is mandatory and a flat `px` closes §N1l; (2) the
attribution was wrong and the ladder wins as a whole — read the `white vs px`
pair; (3) vulnerability is the wrong axis and the constructive family is simply
bad; (4) `px` is a pure doubling knob, so plain DD arbitrates and its PD row
keeps the whole cost of the doubles with none of the benefit.

#### The verdict (2026-08-29)

`scripts/ab-landy-doubler-flip.sh`, fresh `SEED_BASE=1787942099`, sha
`de59ad86`, 4.608M boards per arm per vulnerability, **all six isolation gates
0 foreign**.

| pair | vul | plain DD | DD-PD | sd-plain | SD-PD |
| --- | --- | ---: | ---: | ---: | ---: |
| **`px` vs base** | none | **+0.0107** ±.0004 | **+0.0039** ±.0003 | +0.0061 | +0.0000 ±.0003 |
| **`px` vs base** | both | **+0.0142** ±.0004 | **+0.0061** ±.0003 | +0.0100 | **+0.0028** ±.0003 |
| `white` vs base | none | +0.0409 ±.0006 | **−0.0091** ±.0007 | +0.0547 | +0.0140 ±.0007 |
| `white` vs base | both | *(≡ `px`)* | | | |
| `white` vs `px` | none | +0.0302 ±.0005 | **−0.0132** ±.0007 | +0.0484 | +0.0138 ±.0007 |
| `white` vs `px` | both | 0 fired | 0 | 0 | 0 |

**`px` ships default-on.** Every column is a win except the one non-vulnerable
SD-PD wash — the decision table's `win | win` row, with no need for the domain
addendum's rescue.  Off-switch `--no-ns-landy-doubler-px`.

**`white` is not a win and stays off.** `win | loss` at both readings of the
non-vulnerable cell, and vulnerable it is the same arm as `px` (`white vs px`
fires on **0** boards, so the gate leaves only the `3NT`, which responder's
ungated `3NT`@168 caps out of existence).  The sd bracket dissents — sd-plain
+0.0547, SD-PD +0.0140 — which is not nothing in a 1NT lane, where DD's
killing lead is the documented bias and runs against the arm that *declares*
notrump.  A conflict, not a clearance.

#### The falsifiers, answered

1. **Selection — refuted.**  Split by first differing call on the fresh
   stream, the `X` rung prices **+7.554** (none, n=8,341) / **+9.189** (both,
   n=7,007) IMPs/fired plain, against the **+7.489 / +9.196** that selected
   it — inside 1%.  Its PD row stays flat (−0.129 / −0.188), the domain
   addendum's signature.  The rung is real.
2. **Whole-ladder attribution — refuted.**  Vulnerable the pair is empty by
   construction; non-vulnerable `white vs px` reproduces the split's sign
   pattern and sharpens it.  The rungs do not rescue each other.
3. **Colour is the wrong axis — open, and the gate is now known to be
   under-specified.**  `!vulnerable()` reads **our own** vulnerability only,
   and the A/B spans just the symmetric diagonal.  Favourable and unfavourable
   are unmeasured and are where a constructive/doubling split should diverge
   most.  A retry owes a *relative* gate and the two asymmetric cells
   (jdh8, 2026-08-29).
4. **PD blindness — moot.**  `px` wins DD-PD outright at both colours, so the
   excuse was never needed.

#### Two caveats, measured and recorded

**The `Pass`@0 catch-all is wrong and shipped anyway.** It gives the node
finite mass at three trumps or fewer, so it shadows a floor that was *already
acting* there.  Summing every `call_on == "-"` row of the divergence split, the
suppression costs **−14,171 IMPs plain non-vulnerable** (+889 vulnerable, a
wash) — enough to take the white cell from +0.0107 to roughly **+0.0138**.

What the floor does there is **not** a penalty double.  Opener pulls it:

| our length in their major | pulled to `3NT` | `2NT` | `3♦`/`4♠` | converted for penalty |
| --- | ---: | ---: | ---: | ---: |
| 2 (n=2,461) | **49.5%** | 11.0% | 7.7% | 16.7% |
| 3 (n=1,719) | **49.6%** | 17.3% | 18.5% | ~4% |

It is a takeout-shaped values double that **opener answers by declaring
notrump** — which is independently what the oracle's third reading says is
right (`@op` beats `@dbl` in all 36 buckets).  So the lane already had a
working mechanism nobody authored, and both flip arms break it in opposite
directions: `px` ends the auction, `white` rebuilds the same games from the
**wrong side**.  That is the leading candidate mechanism for `white`'s DD-PD
row, and it is untested.

Deleting the catch-all is therefore the **owed follow-up arm** — and it is not
shipped here because of the second caveat.

**Disclosure.** `comp:landy-penalty` publishes *four-plus* of the major their
advance named.  That is honest for the book rung.  With the catch-all gone the
same `X` at the same seat would *also* be the floor's takeout double on three
or fewer, under one published reading claiming four-plus — the phantom-suit
class.  The catch-all currently keeps the floor off that seat, so the conflict
is latent; the no-catch-all arm owes this tag a decision before it can ship.
Recorded in the precedent block in
[`src/bidding/card.rs`](../../src/bidding/card.rs).

**Where `white`'s loss actually lives.** `div.white.vs.px.none.jsonl` split by
the rung `white` bids and `px` passes (61,651 divergences, all solved):

| rung | n | share | plain/fired | DD-PD/fired | share of the −60,805 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `2NT` | 27,105 | 44.0% | +2.048 | **−1.921** | **85.6%** |
| `3♣` | 17,878 | 29.0% | +2.407 | −0.381 | 11.2% |
| `3♦` | 16,668 | 27.0% | +2.442 | −0.115 | 3.2% |

The invitation is 44% of the traffic and **86% of the damage**; the natural
minors are plain-positive at +2.4 and PD-neutral to within a rounding error
(`3♦` −0.115).  `3NT` does not appear at all — it fires 28 times in 9.2M
boards, as designed.  So §N1l's *first* sketch had the rung right and the verb
wrong: the `2NT` is the drag, and deleting it (rather than gating the whole
family on colour) is the arm the data now points at — plain ≈ +0.0182,
DD-PD ≈ −0.0019, roughly a sevenfold reduction in the PD cost.  It is still a
`win | loss`, so it does not ship on this evidence either; it is the natural
partner to the right-siding question above, since the `2NT` is exactly the rung
that declares notrump from the wrong hand.

**Not evidence:** BBA's behaviour at *this* seat is unknown.  The
`opener-c-x2h`/`opener-c-x2s` probes read **opener's** seat (§N1m), and the
*"bidable suit"* label is on our **first** `X`.  A position-6 probe is
unrun.

### N1m — **opener's** own rebid over their advance (`landy_opener_px` + `landy_opener_rungs`, **SHIPPED DEFAULT-ON 2026-09-16 as a package**)

`1NT (2♣) X (2♥)` and `X (2♠)` — the seat §N1l's is one call later, and the
seat **§N1k authored a `3NT` at, lost on 2026-08-27, and gave back to the
floor**. Flagged item 1 proposed leaving it there and re-opening it "only as
its own arm after §N1l's verdict". This is that arm, and it is designed off a
probe rather than off a hunch.

#### The oracle (Phase 0)

`examples/probe-landy-opener-oracle/` streams an existing arm dump, keeps the
~2% of boards that reach the seat, solves them, and prices **every contract
opener could steer to** against the contract our live method actually reaches:
defend their major undoubled, defend it **doubled**, `2NT`/`3NT` from either
side of our partnership, a natural `3m`, the unnamed major's `3OM`, and par.
The cut is the design question stated as buckets — opener's length in *their*
major × a stopper in it × opener's HCP.

```text
cargo run --release --features serde --example probe-landy-opener-oracle -- \
    ab-results/landy-doubler-rebids/base-none \
    --dd-cache ab-results/landy-opener-oracle/dd-cache.json --min 50
```

Run on the §N1l base arms (`SEED_BASE 1787917699`): **103,653** seat boards
non-vulnerable (2.25% of 4.608M) and **81,023** vulnerable (1.76%), 105,334
distinct deals solved. Reports in `ab-results/landy-opener-oracle/`.

**What it can and cannot say.** Every candidate is priced as *the contract
opener's call leads to if the auction stops there*: the oracle prices
contracts, not auctions. It cannot see partner pulling, their advancer running
from the double, or the information a bid leaks. It is an upper bound per rung
and a reliable *ordering* between rungs on the same boards. The tell is in the
data itself — `2Mx` beats **par** in the four-trump buckets, which is only
possible because par lets them escape and the oracle does not.

**The seat is the floor's and the floor passes it.** 98.5% non-vulnerable /
99.5% vulnerable, the rest a natural `3♦` on ~1%. That is §N1l's "the auction
dies after our values double", one call earlier.

#### What the oracle says

Plain-DD IMPs/board over today's floor, the **stopper-in-their-major** rows
(the no-stopper rows differ by less than half an IMP except in the `3NT`
column, and never change an ordering):

| len × hcp | `2Mx` | `2NT`@op | `3NT`@op | par |
| --- | ---: | ---: | ---: | ---: |
| **none-vul** 2 × 15 | −3.666 | **+2.130** | +1.137 | +2.659 |
| 2 × 16 | −2.352 | **+2.443** | +2.311 | +3.947 |
| 2 × 17 | −0.992 | +2.491 | **+3.697** | +5.241 |
| 3 × 15 | −1.257 | **+1.371** | +0.478 | +2.900 |
| 3 × 16 | +0.657 | +1.560 | **+1.852** | +4.126 |
| 3 × 17 | +2.462 | +1.462 | **+3.363** | +5.231 |
| **4+** × 15 | **+3.524** | −0.608 | −1.415 | +1.810 |
| **4+** × 16 | **+5.265** | −0.403 | +0.297 | +2.946 |
| **4+** × 17 | **+6.754** | −0.343 | +1.851 | +3.800 |
| **both-vul** 2 × 15 | −4.546 | **+0.441** | −0.658 | +2.287 |
| 2 × 16 | −2.909 | +0.813 | **+1.325** | +4.016 |
| 2 × 17 | −1.418 | +0.973 | **+3.478** | +5.784 |
| 3 × 15 | −1.219 | −0.550 | −1.146 | +2.620 |
| 3 × 16 | +1.091 | −0.458 | **+1.099** | +4.254 |
| 3 × 17 | +3.327 | −0.861 | **+3.334** | +5.759 |
| **4+** × 15 | **+4.872** | −3.411 | −3.506 | +0.573 |
| **4+** × 16 | **+6.688** | −3.567 | −1.145 | +2.006 |
| **4+** × 17 | **+8.111** | −3.716 | +1.093 | +3.328 |

Bucket sizes run 2,086–18,234 boards; every figure's 95% CI is ±0.08…±0.35.

Three readings, and they are the whole design.

1. **The `X` gate is length, and nothing else.** `2Mx` wins *every*
   four-plus-trump bucket at *both* vulnerabilities, on a minimum as well as a
   maximum, with or without a stopper — and its perfect-defense column stays
   flat (−1.2…+0.3), which is the signature of a real penalty double that plain
   DD sees and PD is structurally blind to (measurement.md's domain addendum).
   On a **doubleton** it loses at every strength (−0.7…−4.5). On **three** it is
   negative at 15, marginal at 16, and positive only at 17 — and there `3NT`
   matches or beats it whenever opener has a stopper (+3.363 against +2.462
   white, +3.334 against +3.327 red). So `len(major, 4..)`, no HCP floor and no
   stopper test, and the K–K reference's "three plus good defense" is
   **rejected**.

   One cell pays for that simplicity: three trumps, 17 HCP, **no** stopper —
   `2Mx` +1.959 (n=1,409) white and +2.942 (n=1,049) red, against a rung set
   that passes. That is ~4.5% of the `X`'s total value, and buying it costs
   more than it is worth: `comp:landy-penalty` publishes *four-plus* of that
   major, so a three-card double under the same alert would make the alert
   false and the reading wrong. It would need its own slug, its own reading and
   its own arm. **Residue, recorded, not built.**
2. **Declaring notrump is a non-vulnerable idea, except 16–17 with a stopper.**
   Red, the whole declaring family collapses (over the direct leg: `3NT` +0.008,
   `2NT` −0.583) — but the 16–17-with-a-stopper cells hold up at both colours
   (`3NT` +1.10…+3.48 red). The 15-with-a-stopper cells invert with colour:
   `2NT` is +2.130 / +1.371 white and +0.441 / −0.550 red, so the anchor's old
   "the 15s prefer passing" read is right vulnerable and wrong non-vulnerable.
3. **Opener declares.** `@op` beats `@dbl` in every one of the 36 buckets at
   both vulnerabilities — free right-siding evidence, and unsurprising: opener
   holds the 15–17 and the stoppers, and their major sits under it.

#### Why §N1k lost, in this data

§N1k's `3NT` was gated `hcp(16..) & has_stopper` with nothing above it, and
`has_stopper` is **length-blind**. 17.4% of that gate's traffic is the
four-plus-trump slice, where the oracle prices `3NT` at −1.1…+1.9 against the
double's +5.3…+8.1: the rung was not merely mediocre there, it **forwent
+7.0…+7.8 IMPs/board** by shadowing the floor's delayed penalty double — which
is exactly what §N1k's forensic saw, the OFF arm's floor finding
`X (2♥) - - X` on 3 of the 5 worst plain boards per cell. The oracle explains
the refutation rather than contradicting it, which is the cross-check this
probe was run to pass.

The repair is ordering, not a better constraint: **`X`@150 above the notrump
rungs supplies the ≤3-trump cap `has_stopper` cannot express**, for free.

#### The arms as built

| arm | knob | table |
| --- | --- | --- |
| `px` | `competition.landy_opener_px` | `X`@150 `len(major, 4..)` (`comp:landy-penalty`, `.penalty()`) · `Pass`@0, plus the doubler's sit at `{path} X -` |
| `rungs` | `competition.landy_opener_rungs` (needs `px`) | plus `3NT`@135 `hcp(16..) & stopper_in` · `2NT`@120 `hcp(15..) & stopper_in & !vulnerable()`, each a sign-off the doubler passes |

**Three rungs the plan sketched are absent, not deferred.** A natural `3m` is
dominated by notrump on its *own* boards at both colours (+1.86 against `2NT`'s
+1.99 and `3NT`'s +2.19 white; +0.29 against +1.13 red). `3OM` in the major
they did not name is the **worst of all seven candidates** on its own 2.7%
surface (−0.78 plain, −4.5 PD) — they hold four-plus of it. And the relay leg
(`X (2♦) - (2♥) - -`, verified seat-math; 5.1% / 3.3% of the seat) is a
*balancing* seat where the live method already defends their `2♥` and every
candidate prices negative red, so it is not authored either.

**Their runout over our double stays the floor's** (flagged item 4, decided by
smallest diff). The alert publishes opener's four-plus length, so the floor
decides on true information rather than a phantom, and the §N1l twin one call
later takes the same shape. The alert slug is shared with the doubler's seat
(flagged item 3, default taken): one claim, two seats, and who is still to
speak is a matter for the continuation tables rather than for disclosure.

**Falsifiers** (`scripts/ab-landy-opener.sh` states them in full): (1) the
oracle assumes they sit for the double — if `px` reads flat, split the `X` rows
by their next call before anything else; (2) the *instinct* floor already
doubles here, so anchor intuitions do not transfer (the net floor the A/B
measures passes 98.5%); (3) `rungs` is only meaningful under `px`, and if it
loses, the ≤3-trump cap was the whole story; (4) `px` is a pure doubling knob —
plain DD arbitrates.

#### A measurement caveat this probe turned up

`--filter-landy`'s `is_1nt_opener` gate is **strictly balanced** (no
singleton/void, at most one doubleton). Our 1NT opening is `NotrumpShape::Wide6322`,
which also admits 5m(422) and 6m(322) — both of which have two doubletons. So
**no wide-shape 1NT opener ever enters a `--filter-landy` pool**: the probe
found 15,861 boards with a five-card club suit and 14,900 with five diamonds
(all 5(332)) and **zero** with a six-card minor in 103,653 seat boards. This
does not break any A/B — both arms share the filter and the headline is
IMPs per *accepted* board — but every §N1 verdict measured under it is blind to
that slice, and the "5+ vs 6+" question the `3m` rung was supposed to answer is
**unanswerable in this pool**. Flagged below.

#### Verdict — the package ships; `px` alone is refused by the single-dummy bracket

`scripts/ab-landy-opener.sh`, `SEED_BASE=1789547524`, sha `00c0421e`,
4,608,000 boards/arm/vul, 24 shards, both colours. **Isolation gate 0 foreign
on all six pairs**, 100% ours-opened.

| pair | fired | DD plain | DD-PD | sd-plain | SD-PD |
| --- | ---: | ---: | ---: | ---: | ---: |
| `px vs base` none | 78,476 (1.70%) | **+0.0099** ±.0009 | +0.0307 ±.0009 | **−0.0234** ±.0009 | **−0.0075** ±.0009 |
| `px vs base` both | 60,060 (1.30%) | **+0.0249** ±.0010 | +0.0438 ±.0010 | **−0.0077** ±.0010 | +0.0081 ±.0010 |
| **`rungs vs base` none** | 69,615 (1.51%) | **+0.0140** ±.0007 | +0.0251 ±.0008 | +0.0004 ±.0008 | **+0.0095** ±.0009 |
| **`rungs vs base` both** | 49,692 (1.08%) | **+0.0220** ±.0009 | +0.0341 ±.0010 | **+0.0030** ±.0009 | **+0.0135** ±.0010 |
| `rungs vs px` none | 50,488 (1.10%) | +0.0023 ±.0006 | −0.0077 ±.0007 | **+0.0224** ±.0007 | **+0.0156** ±.0007 |
| `rungs vs px` both | 19,973 (0.43%) | −0.0030 ±.0006 | −0.0096 ±.0006 | **+0.0100** ±.0006 | **+0.0049** ±.0006 |

**The shipping unit is `px + rungs`, and only that.** Against `main` it is
non-negative on all four scorers at both colours — four of four vulnerable,
three of four white with sd-plain flat — which clears the decision table on the
knob's own terms. Both knobs are default-on since 2026-09-16, with off-switches
`--no-ns-landy-opener-px` (which drops the rungs with it, since they hang under
that node) and `--no-ns-landy-opener-rungs` (which reproduces the `px`-alone
arm).

**`px` alone is the §N3 honour-half shape, and it is refused.** Plain DD — the
arbiter the runner pre-registered for a doubling knob, falsifier 4 — likes it at
*both* colours, and the sd bracket is what says no: −1.209 IMPs/fired sd-plain
white, with SD-PD negative there too. The gate census says why, and it is not
the double: `px`'s first differing call is **"passed where the baseline bid" on
85.8%** of divergent boards white, with game reached in the baseline only on
74.9%. The `Pass`@0 catch-all, authored to keep the floor off the seat so the
double could be read cleanly, silences the floor's whole competitive
repertoire — the same catch-all §N1-lia deleted one seat later, for the same
reason, and worth ≈ +14,171 IMPs plain on that stream. Under the rungs that
bucket falls to 34.8% pass / 50.6% a different bid.

**Falsifier 3 is answered in the affirmative: the geometry was the problem, not
the cap.** `rungs vs px` is **+1.819 / +1.923 IMPs per fired sd-plain** and
positive on SD-PD at both colours, while DD reads ≈0 (+0.0023 white, −0.0030
red). So the notrump rungs are not a mediocre addition tolerated for the sake of
the double — on the honest-lead scorers they are what makes the package a win,
and the `X`@150 above them is what keeps them off the four-trump hands that
refuted §N1k. Read the two together: DD prefers defending doubled because its
defense is clairvoyant, the sd scorers prefer declaring because a real opponent
must find the lead, and the ordering `X` > `3NT` > `2NT` > `Pass` is what lets
each hand class take the side it wins on.

**What did *not* transfer from the oracle.** The oracle is a contract pricer and
it ranked `2Mx` first on four-plus trumps by +2.8…+8.1 IMPs/board; the A/B's
`px` arm collects that on DD and gives it back under a blind lead. Both readings
are true of different scorers, and the design survived because the rungs — added
for the ≤3-trump hands the oracle *also* priced — carry the seat where the
double cannot. An oracle that prices contracts cannot see this: it has no
column for "what the floor would have done with the rest of the auction".

**Residues, recorded, not built.**

1. The three-trump 17-HCP **no-stopper** cell still passes (`2Mx` +1.96 white /
   +2.94 red, ~1.3% of the seat, ~4.5% of the `X`'s value): it needs its own
   slug, since `comp:landy-penalty` publishes four-plus.
2. `px`-alone remains reachable by flag and **is not a shippable
   configuration** — the knob doc says so at the declaration.
3. The `--filter-landy` wide-shape blindness above is unchanged by this
   verdict: 5m(422) and 6m(322) openers never enter the pool.
4. §N1l's four doubler walks in `tests/american_competition.rs` now start from
   §N1m off, because opener's double (and its `3NT` on a stopped sixteen) fires
   at the seat one call *before* every one of them — the fielded composite
   reaches §N1l's lane only when opener holds three or fewer of their major
   without a stopper. The lane interaction is real and deliberate; only the
   test isolation is new.

### N1p — an **unlimited** values double (`landy_notrump_no_major` **loss, stays off**; `landy_major_jam` **shipped default-on 2026-08-30**)

Responder's direct seat, `1NT (2♣) ?`. The `X`@145 is constrained `hcp(8..)` —
**unlimited on top in its own constraint** — but the table's ungated `3NT`@168
on bare `points(10..)` outranks it, so every ten-plus-point hand declares and
the double never sees a game hand. Verified, not assumed: `probe-call-reading
--their-2c-landy` reads partner back as `points 8..9`.

This is [flagged item 2](#flagged-not-fixed-n1--reversible-defaults-proposed)
taken up. The flag proposed leaving it because *re-gating* `3NT` "moves every
shape in the lane"; the arm below moves only the shape that matters.

#### Verdict — `nt` loses at both colours; the `jam` rung is a stranded win

Runner `scripts/ab-landy-notrump-shape.sh`, `SEED_BASE=1788005427`,
4,608,000 boards/arm/vul, isolation gate **0 foreign** on all four pairs.

| pair | fired | DD plain | DD-PD | sd-plain | **SD-PD** |
| --- | ---: | ---: | ---: | ---: | ---: |
| `nt vs base` none | 54,987 (1.19%) | **−0.0124** ±.0007 | +0.0181 ±.0008 | **−0.0266** ±.0007 | **−0.0012** ±.0007 |
| `nt vs base` both | 37,778 (0.82%) | **−0.0076** ±.0007 | +0.0195 ±.0008 | — | — |
| `jam vs base` none | 55,093 | −0.0109 | +0.0186 | −0.0247 | −0.0002 |
| `jam vs nt` none | 1,567 (0.03%) | **+0.0015** ±.0001 | **+0.0006** | **+0.0020** ±.0001 | **+0.0013** ±.0001 |

`nt` loses plain DD at **both** vulnerabilities — no colour flip, unlike
§N1l's rungs — and loses the SD-PD arbiter white. Only the DD-PD column is
positive, and falsifier 1 is aimed the wrong way to rescue it: perfect defense
already doubles every failing contract, so **deleting our own penalty double
costs nothing in PD while the extra competitive room still scores**. The
+2.383 IMPs/fired PD row is that artifact, not a result. Both knobs stay off.

The `jam vs nt` pair wins **all four scorers** (+5.541 IMPs/fired sd-plain) on
its 1,567 boards, and the sit node never needed relaxing. **It cannot ship on
that number**, and not merely because it rode a losing arm — it measured the
wrong substitution:

| | what `4M` replaced | measured |
| --- | --- | --- |
| `jam vs nt` | the **`X`**@145 — `nt` gates `3NT`@168 to deny 4+ majors, and six of one *is* four-plus | +5.541/fired sd-plain |
| jam standalone | the **`3NT`**@168 — ungated on `main`, and `4♠`@172 / `4♥`@171 outrank it | **unrun** |

Same hands, different comparison, and the §N1p loss was overwhelmingly *"we
stopped reaching game"* — a cost the standalone jam does not pay, because `4M`
**is** a game. So the +5.541 transfers to nothing and the real question is open.

#### The standalone arm (`landy_major_jam` decoupled, 2026-08-30)

The `deny_major &&` conjunct was dropped from both the rungs and their two sit
nodes. The generalisation is **behaviour-preserving where the two overlap** —
with both knobs on the table is exactly what §N1p measured — so the verdict
above stands unchanged.

It also corrects a doc/code discrepancy: `landy_major_jam`'s knob doc claimed
the rung "never fires" without `landy_notrump_no_major` because the ungated
`3NT`@168 swallows the hands. **False** — 172 and 171 both outrank 168. It was
the conjunct that suppressed the rung, not the weight ladder, which is why
decoupling costs nothing.

**The standalone arm has zero reading drift**, verified not assumed:
`probe-call-reading --their-2c-landy "1N (2C) X -"` returns `points 8..9` with
the same suit ranges on `main` and with `--ns-landy-major-jam`. `3NT`@168 stays
ungated, so the exclusion that caps the double is untouched, and the `4M`
denials are already implied by that cap. This is exactly the mechanism §N1p
tripped over — its 16.0% / 13.4% "bid where the baseline passed" bucket — and
the jam does not touch it.

The bridge case is that their `2♣` shows **both majors**, so our own six-card
major sits opposite known length: the suit breaks badly, trump control beats
the ninth trick, and `4M` takes the four-level away from a pair that has
advertised a fit. §N1p measured the candidate handing the opponents more room
on 72–75% of divergent boards by doubling instead of declaring; the jam does
the opposite.

Runner `scripts/ab-landy-major-jam.sh` (arms `base | jam`, both vulnerabilities,
fresh `SEED_BASE`), render `render-book --their-2c-landy --ns-landy-responder
jam-only --prefix "1NT 2♣"`. Its named risks, in the header: obstruction is
invisible to DD (read the sd pair first); `4M` may simply be an overbid, since
the rung has **no quality gate** and a ratty six-bagger with soft side values is
the hand `3NT` was right on (read the made/down split before the IMP mean); the
sit still forgoes slam on the fifteen-plus slice; the slice is thin (~0.03%).

##### Verdict — an eight-of-eight sweep; **shipped default-on**

`SEED_BASE=1788033942`, sha `52fbc7c1`, 4,608,000 boards/arm/vul, 24 shards.

| scorer | none (1,381 fired, 0.03%) | both (1,013 fired, 0.02%) |
| --- | ---: | ---: |
| DD plain | **+1.443**/fired (+0.0004 ±.0001) | **+1.611**/fired (+0.0004 ±.0001) |
| DD perfect-defense | **+1.635**/fired (+0.0005 ±.0001) | **+1.957**/fired (+0.0004 ±.0001) |
| sd-plain (16 worlds) | **+1.435**/fired (+0.0004 ±.0001) | **+1.600**/fired (+0.0004 ±.0001) |
| **SD-PD** (arbiter) | **+1.558**/fired (+0.0005 ±.0001) | **+1.866**/fired (+0.0004 ±.0001) |

Every cell positive, every CI excluding 0, isolation gate **0 foreign** at both
colours. This is not a doubling artifact — plain DD wins on its own and the PD
column only widens the margin, which is the signature of a *contract* gain, not
of auto-doubles.

The divergence census says the rung is doing exactly one thing, and nothing
else: **100.0%** "a different bid" at both colours — zero boards where an arm
bid and the other passed, zero pass-outs, and game reached in **both** arms on
100.0% of divergent boards. §N1p's fatal buckets are all empty here. Compare:

| bucket | §N1p (`nt`) | the standalone jam |
| --- | ---: | ---: |
| bid where the baseline passed | 16.0% / 13.4% | **0.0% / 0.0%** |
| game reached, baseline only | 81.5% / 84.8% | **0.0% / 0.0%** |
| more room handed to the opponents | 72.3% / 75.0% | **0.0% / 0.0%** (6.2% / 0.2% *less*) |
| declarer changed sides | 91.1% / 94.9% | 4.9% / 0.2% |

So the two arms bid the same auction up to the rung and reach game either way;
the only question priced is `4M` versus `3NT` on a strong six-card major
opposite a pair that has advertised both majors. `4M` wins it by 3:1 in IMPs.

The named risks resolve as follows. **Obstruction**: not needed — the win is
already there on plain DD, and the tiny room asymmetry runs *our* way. **The
overbid**: real but priced. Every one of the five worst boards at every scorer
is the same shape, a making `3NT` traded for a failing `4M` (the 6––4 flat
holdings such as `85.AT9763.6.KQ73` opposite `KQJ.54.KQJ94.AJ4`, where the
notrump has nine top tricks and the heart game has a trump loser plus two).
They cost −11…−16 IMPs each and are outweighed threefold, so a quality gate is
a *tuning* follow-up, not a ship blocker. **The sit's forgone slam**: invisible
at this fire rate; unchanged and still the first thing to relax.

Follow-ups, both optional and both unstarted: a quality gate on the six-card
suit (the losing boards are all texture-poor), and relaxing `multi_signoff_pass`
on the fifteen-plus slice now that the rung ships.

##### Falsifier 2 resolves against the idea, not against a continuation

The divergence split (`probe-divergence --gate-opener ours`) makes the
`3NT`→`X` substitution itself the dominant bucket, not the reading drift at
opener's seat:

| bucket, *our first differing call* | none | both |
| --- | ---: | ---: |
| a different bid (the `3NT`→`X` swap) | 83.6% | 86.4% |
| bid where the baseline passed (reading drift above the `X`) | 16.0% | 13.4% |
| game reached, **baseline only** | 81.5% | 84.8% |
| more room handed to the opponents in the candidate | 72.3% | 75.0% |
| declarer changed sides | 91.1% | 94.9% |

Four of the five worst white boards are a quiet made `3NT` traded for a
competitive train wreck — `off: 1NT 2♣ 3NT - - -` against
`on: 1NT 2♣ X - - 2♠ 4♥ - 4♠ X - - XX - - -`. That is falsifier 2's *"we
defended a making game"* branch, which the runner header marked **idea dead**
rather than repairable.

##### Why: the double is outside the floor's teacher's vocabulary

Flagged item 1 already recorded that BBA labels our `X` over Landy **"bidable
suit"** (`12-17, 5+♣`) and never doubles at opener's seat. A further probe
(jdh8, 2026-08-30) adds that BBA's 1NT opener **does not double and reads the
double as takeout**, contrary to expert practice. So the floor — distilled from
BBA — has no learned concept of a values double in this lane.

§N1l shipped because it authored *both* sides of the call: the doubler's own
rebids are book. §N1p does the opposite — it routes several times more traffic
**into** the double, gives the opponents a free round of bidding they never got
over `3NT`, and leaves both continuations to a floor that misreads the call.
The 72–75% "more room handed to the opponents" is that cost, measured.

**The rule this buys**: in this lane, widening a call's traffic by a *reading*
change is only safe where the floor's teacher shares the concept. Where it does
not, the widening owes authored continuations first. See
[docs/reading-drift-handoff.md](../reading-drift-handoff.md) — a reading knob is a
bidding knob under a neural floor.

#### The fix is on the notrump, not on the double

The first sketch promoted the `X` above `3NT`@168. That does buy the same
hands, but "unlimited" then has to mean above the *gated* `3NT`@180 too — and
there is no weight between 180 and the GF both-minors family at 178, so
promoting past the notrump means promoting past the transfers and the splinters
as well. Three separately-defended orderings would move at once: the transfers
outrank the double *by design* ("a six-carder never defends"), and the
two-suited family outranks the transfers so a 6-4 shows the whole picture.

Restricting `3NT` moves exactly the intended traffic and nothing else
(jdh8, 2026-08-29): **`3NT` never gets a four-card major; short stoppers are
welcome; the transfers still outrank the `X`.**

And the gate is not an arbitrary cut chosen to move traffic — it is what `3NT`
*means*. **Bidding `3NT` denies interest in penalising them** (jdh8), and
holding four of a suit they have just shown is exactly interest in penalising
them: their fit in it is at best 4-3, our trumps sit over the overcaller, and
the hand wants to defend. So `len(major, ..=3)` is `3NT`'s own honest
precondition, which is why the reading it publishes (`♥ ≤3 ♠ ≤3`, verified) is
a *narrowing to the truth* rather than a claim the bidder does not honour. The
same statement read the other way is the double's floor: four-plus of their
major is the shape that wants to defend at any strength, which is what
§N1m's oracle then prices at +3.5…+8.1 IMPs/board one seat later.

| | as built |
| --- | --- |
| `nt` | `competition.landy_notrump_no_major` — both `3NT` rungs (@180 and @168) gain `len(♥, ..=3) & len(♠, ..=3)`. Paired `rule` calls, not a conditional constraint: the two constraints are different types (the `landy_doubler_white` idiom) |
| `jam` | `nt` plus `competition.landy_major_jam` — `4♠`@172 / `4♥`@171 on `len(major, 6..) & points(10..)`, above the restricted `3NT`@168 and below the transfers, with `multi_signoff_pass` at `4♠ -` / `4♥ -` |
| `jam-only` | `competition.landy_major_jam` alone — the same two rungs and sits over an **ungated** `3NT`@168, so `4M` substitutes for the game. **This is what ships**, and `nt` stays off |

Where the displaced hands land, in order: a 6+ minor still transfers; the GF
both-minors shapes still fire (4+♦ *and* 4+♣ leaves at most three in each major
anyway, so the only overlap is the 4=1=4=4 splinter); everything else reaches
the `X`@145. `4♠` outranks `4♥` because with 6-6 the better game is `4♠`, and
nothing else satisfies both. The jam rung is natural, so it carries no alert and
`alert-sites.txt` is unchanged.

#### The reading comes for free

`reading.bid_exclusion` intersects each rule with what its strictly heavier
siblings deny, which is the whole mechanism that caps the double today. Narrow
`3NT` and the intersection loosens: the double's published reading widens from
`points 8..9` to *8+ hcp, and with game values four-plus of a major*, and `3NT`
gains an honest denial of major length. **No new slug, no alert change, no
`.bbsa` row, no `comp:landy-penalty`-style disclosure decision** — which is what
separates this from the no-catch-all arm §N1l-flip still owes.

**Free of disclosure cost, not of behavioural cost** — established by the A/B,
recorded here against the claim above. The reading attaches to the *call*, so
the widening also republishes every pre-existing eight-to-nine point values
double as `points 8..37`, and the floor one seat up acts on it. That is the
16.0% / 13.4% "bid where the baseline passed" bucket in the verdict: a real
cost, secondary to the substitution but not zero.

#### Priors

- **N1d** priced taking eight-to-nine point hands *off* this double at
  −0.92/−2.53 PD per fired, and flipping them back at +2.0…+5.1. The double has
  been measured under-fed before.
- **§N1m's oracle** prices defending their major **doubled** at +3.5…+8.1
  IMPs/board in every four-plus-trump bucket at both vulnerabilities, on a
  minimum as well as a maximum, with a stopper or without, PD column flat. That
  is opener's seat, not responder's — it is a prior about *length*, not a
  measurement of this arm.
- The 2026-08-27 census makes `2♣` Landy the lane's **top cost by total**,
  −275 IMPs on 551 boards.

#### Falsifiers

1. **PD is structurally blind to `nt`.** Perfect defense already doubles every
   failing contract, so it keeps the whole cost of a real penalty double and
   none of the benefit. Read `nt` on plain DD with SD-PD as the tie-break.
2. **The displaced `3NT`s were making.** If `nt` loses plain, split the
   divergence by `call_off == "3NT"` and read what replaced it: "we defended a
   making game" kills the idea, "opener pulled the double badly" is a
   continuation defect. The seat above the double was the **floor's** when this
   ran (§N1m was still off; it shipped 2026-09-16), and it pulls a values double
   to `3NT` 49.5% of the time on two trumps — so this arm partly measures that
   floor, and a re-measure would not.
3. **The jam is obstruction, which DD cannot see.** A negative `jam vs nt` on
   plain DD is partly the harness. Read the made/down split of the `4M`
   contracts before the IMP mean.
4. **`--filter-landy` admits only strictly balanced 1NT openers** (flagged item
   5), so the wide-shape slice is invisible to all three arms.

#### The two scales do not leave a hole — checked, not assumed

`3NT`@168 floors on **`points`** and the `X`@145 on **`hcp`** (deliberately —
"defending does not care about distribution"), which looks like it could strand
a shapely hand between them. It cannot. The shipped `ReadingProfile` uses
`PointScale::PointCount`, i.e. `raw_hcp + upgrade`, and `upgrade` is
`unbalanced + (two longest ≥ 10) − wasted`, so it is **capped at 2**. Therefore
`points ≥ 10 ⟹ hcp ≥ 8`, and every hand the gate displaces from `3NT`@168
clears the double's `hcp(8..)`.

The rungs in between are exhaustive too: a displaced hand with a 6+ minor
transfers, a 4=1=4=4 lands on the splinter@176, and everything else reaches the
`X`. **No hand falls to the `Pass`@0 because of this gate** — which is what
makes `nt` a clean substitution of one call for another rather than a mixture
of a substitution and a suppression.

#### Recorded, reversible

The `4♠ -` / `4♥ -` sit is `multi_signoff_pass` — opener passes unconditionally,
so the jam arm **forgoes slam** on the fifteen-plus / six-card-major slice. It
is there because §N1o's forensic caught the floor cue-bidding this lane's
four-level to `6♥` doubled. Proposed reversible default: **keep the sit**, and
delete it first if the jam arm reads mixed. **Settled 2026-08-30**: the jam arm
won all four scorers with the sit in place, so it was never relaxed — and it
carries forward unchanged into the decoupled-`4M` follow-up.

Runner `scripts/ab-landy-notrump-shape.sh`, arms `base | nt | jam` at both
vulnerabilities, `SEED_BASE=1788005427`, `jam vs nt` paired on the same boards
to price the jam rung alone. Renders: `render-book --their-2c-landy
--ns-landy-responder <off|nt|jam> --prefix "1NT 2♣"`. The run was stopped after
`jam-both` began (0 shards written, nothing lost); `jam vs base` and
`jam vs nt` are therefore white-only, and the both-vul `nt vs base` DD cells
were scored off the two completed arms. The sit node was never relaxed — the
jam arm read a win, not mixed.

### N1-lia — Lia's counter-defense: the minor ladder a level down, the doubler unshadowed, Texas at the four level (**packages A and C shipped default-on; B and D measured non-wins; B's refinement measured a loss 2026-09-02 — lane parked behind the floor rail**)

`1NT (2♣)` is the lane's top cost bucket by total (−275 IMPs plain on 551
boards, the census above) and has had no open package at census level since
§N1p closed. **Lia is IntoBridge's AI** — an online service, no code access,
so everything here about her system comes from probing her by hand on
cuebids.com. She plays a counter that is ~80% our shipped §N1j table.

**The original probe of her responder table was wrong, and the "four deltas"
characterisation below is void** (2026-09-01). It read her ladder inverted —
weak transfers at the two level, natural invitations at the three — when she
plays the opposite; the delta called *worse* ("her 5+5+ takeout leaves the 4-4
hand with a singleton major homeless") was a misreading of an **UNBAL 4+♦ 4+♣**
takeout, and the `3NT` = 2-3 majors attribution is now **unconfirmed** rather
than asserted. What survives: the `3NT` delta, whichever system it belongs to,
is `landy_notrump_no_major`, measured a plain-DD loss at both colours — first
with the doubler's known-broken `Pass`@0 catch-all in place, then again on the
repaired seat as package D, where it stayed a non-win. Package B was rebuilt
on the corrected probe and is the only package the correction touches: A, C
and D neither read `defense_2c_landy_lia` nor are gated on it.

Everything ships only on its own A/B: **A** the doubler seat (first — D is
blocked on it, and the −14,171 IMPs live here), **B** the ladder permutation,
**C** the four-level, **D** the `landy_notrump_no_major` re-measure on A's
winner. Six knobs, all default states byte-identical (`smoke-default --count
20000 --seed 1` byte-identical before/after the build, verified twice — once
more after the package-A registration change below, and again after B's
2026-09-01 rebuild).

#### Package A — the full ladder **shipped default-on 2026-08-30**: `landy_doubler_catchall` now **false**, `landy_doubler_three_honors` and `landy_doubler_three_small` both **true**

**Verdict** (`ab-landy-lia-doubler.sh`, SEED_BASE=1788088630, 4.6M
boards/arm/vul, every isolation gate 0-foreign): every adjacent pair on the
cumulative ladder base → nocatch → hon → cells is a **plain-DD win at both
vulnerabilities**, and every sd-lead (16-world) tie-break stays positive —
the whole ladder ships.

| adjacent pair | vul | plain | PD | sd-plain | fired |
| --- | --- | --- | --- | --- | --- |
| nocatch vs base | NV | **+0.0036** ±0.0002 | −0.0027 | +0.0032 | 0.21% |
| nocatch vs base | BV | **+0.0014** ±0.0003 | −0.0044 | +0.0009 ±0.0003 | 0.14% |
| hon vs nocatch | NV | **+0.0008** ±0.0001 | −0.0003 | +0.0006 (+2.03/fired) | 0.03% |
| hon vs nocatch | BV | **+0.0008** ±0.0001 | −0.0004 | +0.0005 (+2.29/fired) | 0.02% |
| cells vs hon | NV | **+0.0104** ±0.0005 | −0.0138 | +0.0007 ±0.0005 | 0.68% |
| cells vs hon | BV | **+0.0118** ±0.0006 | −0.0142 | +0.0020 ±0.0006 | 0.54% |

Falsifier 1 refuted: the −14,171-IMP catch-all cost was no same-seed
artifact — the deletion's sign reproduces on a fresh stream at both colours.
Falsifier 2's loss tail is real (the floor's undisclosed short double does
get sat for the worst boards, −11/−12 IMPs each) but the deletion is a net
win anyway, and the cells shrink the floor's share exactly as the falsifier
asked. Falsifier 3 refuted: the honors cell is positive on its own, and at
+2.0–2.3 sd IMPs/fired it is the ladder's cleanest rung — the sibling lane's
lone-honor caveat did **not** carry to this seat. The small cell's headline
plain margin (+2.0/fired) is mostly the DD-lead artifact — sd-lead shrinks
it to +0.10/+0.37 per fired — but the tie-break holds above zero at both
vulnerabilities, matching the `len3 hon0` sibling prior. PD is negative
throughout: the pre-registered doubling artifact, reported double-blind and
excluded from arbitration per the script header. Package D
(`landy_notrump_no_major`) is now unblocked on the repaired seat — and, run
there, [measured a non-win](#verdict--measured-non-win-2026-09-01-landy_notrump_no_major-stays-default-off):
the repair reached, but it moved the undoubled scorers only.

##### As designed (build record)

The §N1l-flip shipped two caveats; this package takes both up. Deleting the
catch-all (`landy_doubler_catchall=false`) un-shadows the floor's
takeout-shaped values double below the rungs — worth ≈ **+14,171 IMPs plain
non-vulnerable** on the flip stream — and was blocked on disclosure:
`comp:landy-penalty` published *four-plus* while the un-shadowed floor call is
short. **The tag is re-worded** (`competition.rs`, `card.rs`) to **"length or
honour strength in their major"** — one claim across every cell that can fire
under it, so the arms differ only in the rule, never in disclosure. The two
three-card cells then buy exactly-three trumps back by top-honor class
(`X`@154 on `top_honors(2..)`, `X`@153 on `top_honors(..=1)`, both
`.penalty()` under the same tag), priors from the sibling
`nt_high_overcall_x_leave_in` re-slice (`len3 hon0` **+0.62/+1.85** plain per
fired, `len3 hon1` **−0.75/+0.37**; `hon2+` unmeasured).

**As built — the deletion was a silent no-op until the registration moved.**
`Trie::resolve_floored` is a deliberate single fall-through: an **exact
node**'s rejection falls to the floor, but a **guarded fallback**'s rejection
returns its all-−∞ logits unchecked and the driver passes — behaviourally the
same Pass the arm was meant to delete. The doubler tables were `Pattern::after`
guards; they are now `Pattern::node` at the four explicit paths (byte-identical
at defaults, re-verified), and `landy_doubler_cells_split_three_trumps` pins
the floored/not-floored split per arm. Any future "let the floor own it by
deleting the catch-all" change in a **row package** has to cross the same
seam — the trie comment (`trie.rs`, "single fall-through") is the marker.

#### Package B — `defense_2c_landy_lia`: **misprobed, redefined in place, measured a loss, then refined on its own forensic 2026-09-01 — A/B owed**

> **Misprobe annotation (2026-09-01).** Everything in this section down to
> "The repair" is a true record of a **built ladder that no one plays**. The
> 2026-08-31 measured loss and the 2026-09-01 repair are facts about that
> build; they are not facts about Lia's counter, because the probe it was
> built from had her ladder inverted. What she actually plays:
>
> | Call | True Lia | As built (misprobed) |
> | --- | --- | --- |
> | `2♥` | **UNBAL** takeout, 4+♦ 4+♣ | GF takeout 4+♦4+♣ (repair: exact heart doubleton) |
> | `2♠` | **INV+**, 6+♣ (rarely 5) | weak(≤7)/GF two-way, 5+♣ |
> | `2NT` | **INV+**, 6+♦ (rarely 5) | weak transfer, `len(♦,6..) & points(2..)` |
> | `3♣`/`3♦` | **S/O**, 6+ cards | natural 5+ invitations (8–9) |
>
> The unlisted calls (`X`@145, `2♦`@140, `3M`, `3NT`, `Pass`) are confirmed
> unchanged, so the correction is exactly these four rungs. `defense_2c_landy_lia`
> was **redefined in place** — it never shipped, its off state is
> byte-identical, and the old semantics stay pinned by sha (`8a778178`; the
> measured-loss build is `59cd46ee`-control). The rebuild is
> ["Rebuilt as true Lia"](#rebuilt-as-true-lia-2026-09-01--this-is-the-build-ab-landy-lia2sh-measured) below.
>
> The forensic **transfers**, and it partly predicts well for the corrected
> ladder: her sign-offs demand six cards (defect 2 — 6+ won at both colours,
> exactly-5 flipped on vulnerability), and her INV+ rungs restore the forcing
> channel whose absence was defect 1, the only defect negative at both
> colours. What it predicts badly is the one cell true Lia has no home for:
> the exactly-five 8-9 invitation, the forensic's single biggest win (`3♣`
> +85,613 NV) and its best contested rung (`3♦` +1.576/+1.931).

**Verdict of the built ladder** (SEED_BASE=1788122360, control `59cd46ee` =
package A's ship, 4.6M boards/arm/vul, both isolation gates 0-foreign):

| vul | plain | PD | sd-plain | sd-PD | fired |
| --- | --- | --- | --- | --- | --- |
| NV | **+0.0050** ±0.0012 | −0.0756 ±0.0016 | +0.0374 ±0.0013 | −0.0289 ±0.0016 | 5.90% |
| BV | **−0.0384** ±0.0014 | −0.1210 ±0.0018 | +0.0016 ±0.0014 | −0.0691 ±0.0017 | 4.92% |

Plain DD splits by colour and the both-vul loss is 27σ, so there is no plain
win to ship on; the PD deficit is an order of magnitude past package A's
doubling artifact and cannot be waved through as one. The mechanism runs the
**opposite** way to A's: lia *removes* our penalty doubles (5.51% of divergent
boards against base's 7.79%), and perfect defense is exactly the scorer that
pays for doubles we no longer make. The knob stays default-off, off-state
byte-identical, on the `multi_px_split` precedent.

**The loss decomposes into four named defects, none of them the ladder's
concept.** `probe-divergence --imps` over both divergence sets, bucketed by our
first differing call and by responder's own hand (seat resolved from dealer +
auction; self-validating — all 101,767 boards in the `2♠` bucket come back
holding 5+ clubs, exactly what the rung constrains). Plain IMP totals, NV / BV:

| bucket | plain NV | plain BV | per fired NV/BV |
| --- | --- | --- | --- |
| `3♣` natural club invitation | **+85,613** | **+35,920** | +1.31 / +0.68 |
| `3♦` natural diamond invitation | +14,381 | −14,911 | +0.31 / −0.40 |
| `2♠` club rung | +15,581 | −115,870 | +0.13 / −1.14 |
| `2NT` diamond rung | −23,533 | −23,095 | −1.92 / −2.42 |
| `Pass` (lia passes where base bid) | −35,827 | −32,432 | −2.26 / −2.24 |
| `2♦` | −18,178 | −14,121 | −2.02 / −1.79 |
| `2♥` sole takeout | −12,525 | −10,408 | −4.07 / −5.00 |
| **total** | **+22,996** | **−176,837** | |

**Defect 1 — the contested tails are unauthored, and this is the only defect
negative at both colours.** Every authored lia node requires the opponents to
have passed (`{rung} -`, `{rung} (X)`, `{completed} …`), so an opponent bid
*anywhere* drops the remainder to a floor with no forcing channel — the
`4♣-4♥-5♣-5♦-6♣-6♥` runaways in the worst boards, one landing in `6♥` on a
void.

| rung | quiet NV/BV | contested NV/BV |
| --- | --- | --- |
| `2♠` | +79,531 (+1.29) / −35,081 (−0.57) | −63,950 (−1.13) / −80,789 (−2.00) |
| `2NT` | −1,125 (−0.28) / −1,427 (−0.44) | −22,408 (−2.72) / −21,668 (−3.42) |

The diamond rung is the clean demonstration: uncontested it is nearly free, and
**95%/94% of its entire loss is the contested tail**. Note this is *not* only
the advancer's seat — they pass over `2♠` on 85% of boards, and their immediate
advance is only −26,462 of BV's −115,870; the rest is opponents entering later,
after opener's length answer. The whole contested surface is owed, not one node.

**Defect 2 — the weak five-card sign-off, and it is vulnerability-dependent.**
97% of the club rung's BV loss sits in 0–7 HCP. Within that band, uncontested,
plain per fired (NV / BV):

| weak, quiet | n (BV) | plain/fired NV | plain/fired BV |
| --- | --- | --- | --- |
| exactly 5 clubs | 51,521 | **+1.405** | **−0.803** |
| 6 clubs | 6,187 | +0.993 | +0.668 |
| 7+ clubs | 1,137 | +1.849 | +1.683 |

Six-plus wins at **both** colours; exactly five flips sign with vulnerability —
light five-card sign-offs are profitable white and ruinous red, which is
falsifier 2 confirmed in a sharper form than it was posed (the old N1j transfer
demanded six). That one cell is also the biggest PD cell in the arm:
**−106,027 NV / −249,689 BV**, the latter 45% of the whole arm's PD deficit.

**Defect 3 — the `2NT` cap starves diamonds** (falsifier 4 confirmed): the weak
six-carder with 7+ HCP now passes, worth −35,827 NV / −32,432 BV.
**Defect 4 — the sole `2♥` takeout** is the worst per-fired rung in the ladder
(−4.07/−5.00), on small volume; the 2=3=4=4 merge is the suspect.

At both-vul the first three sum to −176,261 against a −176,837 total, so they
account for essentially the whole deficit — which means the rest of the ladder
is roughly break-even and the restored invitations are a real win.
**Falsifier 1 is not merely refuted but reversed**: the N1c right-siding trade
was *wrong* on plain DD, and unwinding it is the single biggest positive here.

**Repair queue** before any re-measure (fresh seed, control = then-current
`main`), in size order: (1) author the contested tails across both rungs at
every level; (2) gate the weak `2♠` leg to 6+ clubs vulnerable, 5+ white;
(3) restore a rung for the starved weak six-card diamond hands; (4) revisit the
2=3=4=4 merge into `2♥`. Packages C and D are unaffected — neither knob is
gated on `landy_lia` and neither runner passes it, so their control is
unchanged by this loss.

##### As designed (build record) — **superseded, kept as the record of what was measured**

Every rung below is the **misprobed** ladder. It is kept because the loss and
the repair were measured against it, and a verdict is only readable against
the build it scored. The live design is
["Rebuilt as true Lia"](#rebuilt-as-true-lia-2026-09-01--this-is-the-build-ab-landy-lia2sh-measured).

One arm; a permutation of the same rungs, so it cannot be decomposed. As
rendered (`render-book --their-2c-landy --ns-landy-responder lia --prefix
"1NT 2♣"`):

As first built (2026-08-31, the arm that measured the loss); the three rungs the
2026-09-01 repair moved are marked, with the repaired form beside them:

| Call | As measured | After the repair | vs §N1j |
| --- | --- | --- | --- |
| `3NT`@180/168, `X`@145, `2♦`@140, `3♥`/`3♠`@176/175, `Pass` | unchanged | unchanged | — |
| `2♥`@178 | GF takeout, 4+♦ 4+♣, 2+ in **both** majors | **`len(♥, 2..=2)`** — N1j's exact heart doubleton | the only takeout; the merge is reverted, 2=3=4=4 re-routes to `3NT` |
| `2♠`@174 | 5+♣, weak (≤7) **or** GF (10+) | **`points(..=7) & (len(♣, 6..) \| !vulnerable())`** on the weak leg | was `2NT`→♣ 6+ wide |
| `2NT`@173 | 6+♦ and (7+♦ \| `top_honors(2..)` \| GF) | **`len(♦, 6..) & points(2..)`** — the N1j transfer's own shape gate | was `3♣`→♦ 6+ wide |
| `3♣`@167 / `3♦`@166 | natural 5+ invitations (8-9) | unchanged in the rule; `3♦` now sees only five-card hands, by weight | restored — the N1c right-siding trade unwound **per length** |

The level-down ladder matches BBA's own coherent self-play tree
(`docs/ai-bidder/bba-1nt-landy-tree.md`: `2♠`→♣ 5.9%, `3♣`→♦ 6.8%, direct `X`
0/4,074); §N1j's `2NT`→♣ was aligned to the older actor-only reading that
corpus overturned. Clubs and diamonds are deliberately asymmetric: `2♦` gives
diamonds a cheap natural outlet, clubs have none below 2NT, so `2♠` stays
genuinely two-way.

**No forced completion — opener answers the minor rungs by length**
(`comp:landy-length`, a new always-on alert so the reader decodes the exact
bands instead of the walk's four-card raise floor): the cheap raise = 3+
(`3♣` over `2♠`, `3♦` over `2NT`), the step below it a doubleton (`2NT` over
`2♠`, a contract; `3♣` over `2NT` — balanced with two diamonds implies 3+
clubs). Responder rebids off `landy_bba_transfer_rebid` **verbatim** on the
fit legs; the doubleton legs add one rung — a sign-off in the minor
(`3♣`/`3♦`@60 on `points(..=9)`), and opener sits (`multi_signoff_pass`).
The restored `3♣`/`3♦` invitations get the stack lane's acceptance table
(`landy_minor_invite_answer`: `3NT` from the top with both majors stopped,
else sit) — caught by the build review, not the design: floor-owned, the
seat answered the N1j gadget the natural `3m` replaced with a **phantom
`3♦` transfer completion** on every probed hand, which would have
confounded the arm's own falsifier 1.
The N4-KK `4m` slam try and `landy_slam_answer` (+ RKCB, `-` and `(X)` tails)
re-hang **byte-identical on all four legs**; the P6 residue (accept gate
counts HCP, not controls) is left alone so the ladder arm stays attributable.
Probed readings: `2♠` → partner ♣ 5-13, no spade claim; raise → ♣ 3-6;
doubleton → ♣ exactly 2 / ♦ exactly 2. Two design-sketch ambiguities resolved,
both reversible: *"INV+ may pass"* opener's 2NT is resolved to
**sign-off-always** (the two-way rungs carry no 8-9 band, so the pass had no
traffic — a weight change re-opens it), and the `2NT` GF leg requires **6+♦**
(a GF 5♦4♣ hand rides the takeout, a GF 5♦ balanced hand bids `3NT`).

**After `2♥` the answer priority reverses** (`landy_lia_takeout_answer`): a
four-card minor first (cheapest with both — the guaranteed 4-4 fit is the
takeout's point), `2NT` = spade stopper *specifically*, `2♠` = neither
(asks; `comp:landy-ask`, alerted by hand — vacuous constraint). Lia's takeout
names no short major, so `2NT` answers the one question nothing else in the
structure ever answers — nothing promises hearts, and LHO leads their longer
major; responder resolves hearts with the `3♥` cue
(`landy_bba_takeout_rebid`/`landy_bba_ask_answer` reused verbatim, the asked
major flipped to hearts). Over the `2♠` ask responder needs its own spade
stopper for notrump: `3NT` with both, `3♥` cue with spades only, else `4♣`@20
(the ask-answer's own stopper-dead catch-all one seat over). Splinters and
their raise/`(X)` tails keep the shipped tables; lia's raise tails key the
stopper on the suit **they raised** instead of a short major the takeout no
longer names.

##### The repair (2026-09-01) — **superseded by the probe correction**; its A/B was stopped mid-flight

**Verdict: superseded.** The repair fixed four defects in a ladder nobody
plays, and its A/B (`scripts/ab-landy-lia-repair.sh`, SEED_BASE=1788247951,
control `ce94faeb`) was **stopped** when the probe correction landed. One cell
had completed and is recorded rather than discarded, since it is the only
measurement the repaired build will ever get:

| vul | plain | PD | gates |
| --- | --- | --- | --- |
| NV | **+0.0191** ±0.0011 | **−0.0382** ±0.0014 | 0-foreign |
| BV | *never run* | *never run* | — |

That NV row is the decision table's doubling-artifact shape (plain win, PD
loss) on a mechanism that *bids more* — and the runner's header stated the
arbitration rule for it **wrong**, which is why `ce94faeb` exists. The
arbitration question is now **moot**: there is no both-vul cell, and the
build it scored has been replaced. Nothing is owed on it. The arms stay on
disk under `ab-results/landy-lia-repair/` for forensics.

The four defects it named remain the best evidence available about this lane,
and the rebuild below uses them — see the misprobe annotation for which of
them transfer and which one predicts a loss.


The four defects above are repaired behind the same off-by-default knob (**no
new knobs**; `smoke-default --count 20000 --seed 1` byte-identical against
`main` HEAD, verified twice). Step 0 re-solved the kept divergence sets
(`probe-divergence --imps --jsonl` over `ab-results/landy-lia/`; the August
run's JSONL had been in `/tmp` and was gone) and **two of the plan's three
pre-registered decision rules came back inverted**, so the built repair is not
the one the plan sketched.

**Step 0(a) — the invitations split on length, not on colour.** The rule was
"tighten `3♦` to six-plus iff the exactly-five quiet cell is negative at both
colours". Exactly-five is *positive*, and the six-plus cell is the loser:

| rung × length | quiet NV | quiet BV | contested NV | contested BV |
| --- | --- | --- | --- | --- |
| `3♣` exactly 5 | **+1.796**/fired (+63,697) | +0.756 (+23,210) | +0.818 (+4,243) | −2.721 (−5,521) |
| `3♣` 6+ | +0.768 (+16,365) | +1.035 (+19,876) | +0.369 (+1,308) | −1.394 (−1,645) |
| `3♦` exactly 5 | **+0.920** (+23,105) | −0.459 (−10,032) | +1.576 (+15,504) | +1.931 (+11,701) |
| `3♦` 6+ | **−0.910** (−6,995) | **−0.858** (−5,749) | **−4.668** (−17,233) | **−4.195** (−10,831) |

The plan's rule ("tighten `3♦` to six-plus iff exactly-five quiet is negative at
**both** colours") therefore does **not** fire — exactly-five quiet is +0.920 NV
against −0.459 BV — but the cell it was aimed at is the wrong one. `3♦` **6+**
is negative in all four cells, so the N1c right-siding trade splits on **suit
length**: the natural invitation wins the five-card hand and the transfer's
right-siding wins the six. The six-card hand is given back to the rung above,
and exactly-five stays (its two contested cells, +1.576 / +1.931, are the
package's best). Clubs are positive at both lengths *quiet* at both colours and
are left alone; their one negative cell (exactly-five contested BV, −2.721) is
a contested tail, which defect 1 addresses rather than the rung.

**Step 0(c) — the escape's loss is entirely its tail.** `2♦` is quiet
**+1.110**/fired NV, **+1.360** BV and contested **−2.455** / **−2.106**, on
traffic that is 88% / 91% contested — the most-contested call in the lane, and
the one rung that never even had the `(X)` arm. The plan's follow-on (lift the
cap to `hcp(..=8)`) was then measured to be a **no-op** and is not built: the
escape is `hcp(..=6)` under a `natural_floor` of `(5, 0)`, so it already spans
exactly 5-6, and the passed-out bucket contains **zero** boards at 5 diamonds ×
7-8 HCP at either colour.

**Step 0(e) — defect 3 is a seam between two floors, not a quality judgement.**
The starved hands are 6+ diamonds with **0-4 HCP** (12,956 of 14,156 passed-out
six-card diamond boards, −33,530 plain NV; −30,610 BV): they failed `2NT`'s
quality gate and then failed the `2♦` escape's five-HCP floor. The base arm
bids its wide transfer on 14,153 of the 14,156.

**Step 0(b) — the `2♥` merge convicts itself.** Every one of the 3,069 divergent
`2♥` boards is the merged 2=3=4=4 shape, at −4.074 IMPs/fired quiet NV and
−4.935 BV, ending in a forced `5♣` (−3.832) or `5♦` (−4.324).

**Defect 2 re-confirmed on the fresh solve** at both colours, unchanged from the
August split: weak and quiet, exactly five clubs is **+1.405** IMPs/fired NV and
**−0.803** BV (−41,372 plain, −249,689 PD), while six (+0.993 / +0.668) and
seven-plus (+1.849 / +1.707) win at both.

###### What was built

| # | edit | evidence |
| --- | --- | --- |
| 1 | **the contested surface, both seats plus two the plan did not name** | defect 1, below |
| 2 | weak `2♠` leg → `points(..=7) & (len(♣, 6..) \| !vulnerable())` | defect 2; `over_overcall`'s free-bid idiom, length as the quality term |
| 3′ | `2NT` → `len(♦, 6..) & points(2..)`, the N1j transfer's own shape gate | 0(a) + 0(e), one edit for both |
| 4 | *not built* — the `2♦` cap lift is a measured no-op | 0(c) |
| 5 | `2♥` → `len(♥, 2..=2)`, the merge reverted | 0(b) |

**Defect 1 as built.** The measured failure is not that the floor is silent at
the unauthored nodes — **it is that the floor bids.** Censused over package B's
own `lia` arm, at opener's seat it pushes `4♣` on **72%** of `2♠ (3♠)` boards
and **96%** of `3♣ (3♠)`, `4♦` on 91% of `2♦ (3♥)`, and `3♦` on 91% of
`2♦ (2♥)`. Authoring `Pass`@0 is a decision to sell out, not a no-op, and it is
the substance of the repair. Two tables, and the asymmetry between them is the
design:

* **opener sits** (`landy_lia_overcalled`) — it cannot know which half of a
  two-way rung responder holds, and Pass is safe by the *auction's shape*
  rather than by a game force (their bid cannot be followed by three passes
  without responder speaking again). The exceptions ride only a rung that
  already promised values, i.e. the natural `3♣`/`3♦` invitations: the accept
  where their call left room below `3NT`, and the `len(major, 4..)` penalty
  double `probe-landy-opener-oracle` measured at opener's other seat in this
  lane.
* **responder captains** (`landy_lia_contested_rebid`) — it knows its half, and
  it is the seat that plays *over* the re-entering hand. It carries `3NT` on
  both majors stopped, the penalty double on `hcp(10..) & len(major, 4..)`
  (`hcp`, not `points`: distribution does not defend, and the `2NT` rung's weak
  arm reaches `points(10..)` on a seventh diamond alone), a finite `4m` game
  force so the strong half is **never** stranded, and the misfit leg's `3m`
  escape so a contested tail does not delete a rung the quiet tail offers.

The seat rotation is worth recording because it inverts the obvious reading and
was verified against the arm dumps, not argued: with `O L R A` clockwise,
`2♠ - 3♣ (3♥)` indexes `O L R A O` **`L`** — the hand that re-enters after
opener's length answer is the **overcaller**, whose partner has already passed,
bidding shape into a known 15-17.

Two nodes the plan did not name, both found by pricing the census:

* **the four level.** A first draft stopped the band at three, on the argument
  that the floor's delayed double there (47% of `2♠ (4♥)`, 74% of `2♠ (4♠)`)
  might be right. It is not: those two cells are **−3.434** and **−4.268**
  IMPs/fired, the worst per-fired cells in the whole `2♠` bucket.
* **the balancing seat** `{rung} - {leg} - - ({over})`, where responder passed
  opener's length answer and the *advancer* reopens: −5,509 (`(3♥)`) and −2,838
  (`(3♦)`) plain NV on the club rung alone, about a third of its contested
  deficit, at a node no draft had reached.

###### Flagged, not built

* **`landy_recue_answer`'s `4m`@20 has no answer node**, so
  `2♠ - 3♣ - 3♥ - 4♣ -` is floor-owned — and that floor is the one §N1o caught
  cue-bidding to `6♥` doubled. It is four of the five worst both-vul boards
  (`… 5♣ - 5♦ - 6♣ - 6♥ X`, −16.0 IMPs/fired on 59 NV boards). **The seat is
  shared with the base arm**, which runs away the same way one level lower and
  undoubled, so repairing it lia-gated would make this arm measure a repair the
  control wants too. Owed as its own A/B; the reversible default is a
  `multi_signoff_pass()` sit rail at `{completed} {3M} - {4m} -` in both arms.
* **`defense_2c_landy_lia` was in no `assert_package_invariants` sweep.** Every
  lia row had shipped with no totality check, no weight-tie check and no alert
  check — `landy_counter_package_invariants` now carries a lia × `landy_texas` ×
  `weak_2d_cap` arm.
* **`alert-sites.txt` does not move**, contrary to the repair plan: all four of
  its sections are built at `defense_2c_landy_lia = false`, and no `card.rs` row
  reads any `landy_*` knob. Nothing to re-bless while the knob stays off — which
  held again through the rebuild, `comp:landy-minor` included.
* **The seat immediately after opener's authored sit stays the floor's — six
  node families, not the one this list used to name.** Enumerated from
  `landy_bba_entries`' 201 registrations during the 2026-09-01 pre-flight audit
  of `ab-landy-lia-repair.sh`; all six are lia-only (none reachable in the base
  arm), so none of them cancels in the paired diff:

  | Family | Why it is not covered |
  | --- | --- |
  | `{rung} - {leg} - - (X)` — their **balancing double** | `landy_lia_entries` yields *bids only*, so the balancing loop never registers an `X` arm; `systems_on_over_double` cannot catch it either, because its guard is `s.first() == Some(&Call::Double)` and the first suffix call here is a Pass |
  | `{rung} - {leg} - - ({over}) - -` | responder after opener's authored balance-pass |
  | `2♦ ({over}) - -` | responder after opener's sit on the escape — opener only is registered |
  | `{fit} ({over}) - -` and `{fit} - - ({over}\|X)` | responder after the natural invitation is contested or declined. This is the ladder's **biggest measured win** (`3♣` +85,613 NV) and is contested on ~22% of its traffic; only `{fit} ({over})` and `{fit} ({over}) X -` are registered |
  | `{rung} ({over}) - - X ({run})` | their runout from the penalty double the repair itself newly authors |
  | `{rung} ({over}) - ({over2})` | one round deeper than opener's pass (the family previously listed here) |

  So **defect 1's closure is partial by construction**, and the repair's own
  authored `Pass` is what creates the fresh traffic — the same mechanism the
  repair exists to close, one seat later. Pre-registered in the runner header:
  if the arm reads a wash rather than a win, this is the first place to look,
  and falsifier 1's reading is a lower bound on what authoring the tails buys.
  The reversible default is (a) `landy_lia_entries(leg).map(Some).chain([None])`
  with an `Option<Bid>` in `landy_lia_overcalled`, the shape
  `landy_lia_contested_rebid` already takes, so their balancing `X` gets the
  sit, and (b) `multi_signoff_pass()` at the four responder-follow-up families.
  Not built at the time: it would have changed what that A/B measured.
  **All six are closed by the 2026-09-01 rebuild**, which took exactly those
  defaults once the A/B they would have disturbed was superseded — see
  ["The six flagged families are closed"](#the-six-flagged-families-are-closed).
* **`park/landy-pdi` owes a rebase.** The repair adds two `.penalty()` rows
  (`landy_lia_overcalled`'s invitation double, `landy_lia_contested_rebid`'s),
  and [docs/pdi.md](../pdi.md) makes `grep -rn '\.penalty()' src/bidding` the live
  trigger inventory — so both enter it automatically. The branch has no row on
  `main` to annotate (`git branch --list 'park/*'` is its whole index), so the
  note lives here: rebasing it onto this repair widens §N1n's trigger set by
  two lia-gated sites, both inert while the knob is off.

##### Rebuilt as true Lia (2026-09-01) — **this is the build `ab-landy-lia2.sh` measured**

The knob was **redefined in place** — it never shipped, its off state is
byte-identical, and pinning the old semantics by sha is cheaper than a second
knob whose only job is to neutralise the first (the house rule on scaffolding
knobs). Everything below is behind `defense_2c_landy_lia`, still default off.

###### Responder's table

| Call | Weight | Rule | Alert |
| --- | --- | --- | --- |
| `2♥` | 177 | `4+♣ & 4+♦ & len(♥, ..=2) & len(♠, ..=2) & points(8..)` | `comp:landy-tko` |
| `3♥`/`3♠` | **179/178** | splinters, rule unchanged | `comp:landy-spl` |
| `2♠` | 174 | `len(♣, 6..) & points(8..)` | `comp:landy-minor` |
| `2NT` | 173 | `len(♦, 6..) & points(8..)` | `comp:landy-minor` |
| `3♣` | **141** | `len(♣, 6..) & points(..=7)` | none — natural |
| `3♦` | **139** | `len(♦, 6..) & points(..=7)` | none — natural |

Everything else is the shared table verbatim: `3NT`@180/@168, the `4M` jam,
`X`@145, `2♦`@140, `Pass`@0.

Four decisions worth the ink.

**The takeout is spelled as shape, and the shape is the whole delta.** Lia
states *UNBAL, 4+♦ 4+♣* and no point count, so the band is ours (invitational
up, unlimited above; the weak 4-4 minor hand passes). "Unbalanced with 4+4+ in
the minors" is exactly `len(♥, ..=2) & len(♠, ..=2)` — with eight-plus cards in
the minors and no three-card major the holding is nine-plus in the minors, which
is every unbalanced shape in the family and no balanced one. It therefore still
**excludes 2=3=4=4**, the merge the first build let in and the forensic
convicted (−4.074 IMPs/fired, all 3,069 divergent boards that one shape); those
hands re-route to `3NT` by weight, where the base arm plays them. The
misprobe's "her 5+5+ leaves the 4-4 singleton-major hand homeless" was never
her rule, and under the corrected one those hands are *inside* the takeout.

**Which forces the splinters above it.** Lia's shape contains the splinters',
so unlike N1j's exact doubletons the two families overlap and the weights have
to arbitrate: `3♥`/`3♠` move to 179/178 and the takeout to 177, so a
game-forcing 0-1 major makes the more descriptive call and the same shape at
8-9 takes the cheaper takeout. N1j's ordering is untouched.

**The minors invert, and six cards is the rule at both ends.** `2♠`/`2NT` are
*natural* six-card minors — responder declares, no transfer, so
`comp:landy-transfer` is not the tag and `comp:landy-minor` is new. Opener
answers by length (`comp:landy-length`, unchanged) or accepts at `3NT` from the
top of the range with both of their majors stopped — the new rung, and the one
the invitational band cannot do without: a describe-only structure walks a
24-count into `3♣`. The sign-offs one step higher want six too, which is Lia's
own "6+, rarely 5" and also what the forensic said about the *first* build's
five-card weak leg (exactly-5 **+1.405 white / −0.803 red**, 45% of the arm's
both-vul PD deficit; six and seven-plus won at both). With exactly five and a
bust there is now no rung, which is the correction's one predicted loss — see
falsifier 1 below.

**The sign-offs straddle the escape, and that closes defect 3 for free.**
`3♣`@141 sits *above* `2♦`@140 because clubs have no cheaper outlet; `3♦`@139
sits *below* it, so the escape keeps every hand it can take and `3♦` picks up
exactly the ones it refuses — the 6+♦ hands with **0-4 HCP** that failed the
escape's five-HCP `natural_floor` and were package B's "starved diamonds"
(12,956 of 14,156 passed-out boards, −33,530 NV / −30,610 BV), plus the
seven-point hands its `hcp(..=6)` cap excludes. No widening, no knob: rung
order alone. Nothing in the package reads `vulnerable()` any more.

###### Continuations

* **Opener's takeout answer** keeps the reversed priority — four-card minor,
  then `2NT` = spade stopper, then the `2♠` ask — with `3NT`@160 on top.
* **Responder's placements re-banded for INV+.** `landy_bba_pick_rebid`'s
  `5m`@100 and `landy_bba_takeout_rebid`'s `3NT`@100 were game-forcing
  catch-alls; the lia twins (`landy_lia_pick_rebid`, `landy_lia_takeout_rebid`)
  gate them on game values and make `Pass`@0 the finite rung, so 8 opposite a
  minimum stops in `2NT` or `3m` instead of bidding 24-point games.
* **The `2♠` ask's catch-all is `3♣`@20, not `4♣`@20.** Two things were wrong
  with the four-level version and the invitational band made the second fatal:
  it was an unalerted artificial call (vacuous constraint, so the invariant's
  witness could not see it), and it committed to the five level opposite eight
  points. `3♣` names a suit responder holds by the takeout, so it is natural
  and the flagged item retires. It is not a *fit* promise — our `1NT` caps both
  majors at four, so an opener denying four in both minors is 3-3, 3-2 or 2-3
  there and the landing can be 4-2 — and dropping below `3NT` opens a node the
  four-level rung could not: unauthored, the floor bids `3NT` on **every**
  probed hand, in the strain opener's own ask just denied a stopper in. One
  edit answers both: `landy_lia_ask_landing` has opener correct to `3♦` on a
  club doubleton with three diamonds, which turns the 4-2 into a 4-3, and pass
  otherwise. Found by the build's adversarial review.
* **`landy_lia_misfit_rebid`'s `signoff`@60 now carries real traffic.** The
  first build resolved "INV+ may pass" to sign-off-always on the grounds that
  the two-way rungs had no 8-9 band; they do now, and the rung is what stops
  the invitational hand being stranded in a doubled 6-2 notrump.
* **The minor-transfer-slam rule is honoured on the acceptance too.** `2♠`/`2NT`
  are uncapped, so opener's `3NT` can land opposite a slam hand;
  [minor-transfer-slam.md](../minor-transfer-slam.md) requires a `4m` rung above
  that `3NT` **with an authored answer**, because an unauthored `4m` reads as
  nothing and the floor's keycard ask is gated on `undisturbed`, which this
  lane never is. `landy_lia_accept_rebid` is the rung; `landy_slam_answer` and
  `slam::rkcb_rows` re-hang on it exactly as on the length legs.
* **`values` is now true on the contested tails of `2♠`/`2NT`.** The first
  build's rungs were bust-or-game-force, so opener had nothing to act on; Lia's
  promise 8+, so opener's `3NT` accept and its four-trump penalty double ride a
  promise the rung makes. Over the six-card sign-offs opener still sits on
  everything. That double needs the mirror of responder's — a `multi_signoff_pass()`
  sit at `{rung} ({over}) X -` and at the balancing `{rung} - {leg} - - ({over}) X -`
  — because the registration that used to cover it rode the invitation rung
  that has become a sign-off. Caught by the build's adversarial review, not by
  the invariants, which do not know a seat is missing.

###### The six flagged families are closed

The 2026-09-01 pre-flight audit found six lia-only node families still reaching
the floor after the repair, and deliberately left them (they would have changed
what the stopped A/B measured). That constraint died with the A/B, so the
rebuild takes the reversible defaults the audit wrote:

| Family | Now |
| --- | --- |
| `{rung} - {leg} - - (X)` — their **balancing double** | `landy_lia_entries(leg).map(Some).chain([None])` and `landy_lia_overcalled` takes `Option<Bid>`; the penalty rung drops, the sit and the accept stay |
| `{rung} - {leg} - - ({over}) - -` | responder sits (`multi_signoff_pass`) — it passed the length answer once |
| `2♦ ({over}\|X) - -` | responder sits; the escape's `(X)` arm exists now too |
| `{3m} ({over}\|X)`, `{3m} ({over}) - -`, `{3m} - - ({over}\|X)` | opener sits under every entry including their double; responder sits wherever the auction returns |
| `{rung} ({over}) - - X ({run})` | opener sits through their runout from responder's penalty double |
| `{rung} ({over}) - ({over2})` | responder captains (`landy_lia_contested_rebid`) — it has not spoken since the rung |

What is still floor-owned is now one seat deeper again, and the honest statement
is that authoring a `Pass` always creates fresh traffic below it; the difference
is that the families above were the ones the census could *price*.

##### Verdict of the rebuild — **measured a loss 2026-09-01; the knob stays default off**

`scripts/ab-landy-lia2.sh`, SEED_BASE 1788264406, control `main` HEAD sha
`32242d63`, 4,608,000 boards/arm/vul, both isolation gates **0 foreign**:

| vul | fired | plain DD | PD |
| --- | --- | --- | --- |
| NV | 143,158 (3.11%) | **−0.0077** ±0.0009 (−35,676; −0.249/fired) | −0.0127 ±0.0011 (−58,701; −0.410/fired) |
| BV | 123,212 (2.67%) | **−0.0059** ±0.0010 (−27,354; −0.222/fired) | −0.0172 ±0.0012 (−79,454; −0.645/fired) |

Plain DD was the runner's pre-registered arbiter at both colours and both cells
are negative, so there was no plain win to ship on. The sd pass was **killed
unrun** — four negative cells need no lead model. Both arm directories and
`imps-{none,both}.jsonl` are kept; everything below is read off them.

**The loss is the diamond leg, and the club leg is a clean win.** Splitting the
divergence by which leg moved (baseline `2NT` = its club transfer, `3♣` = its
diamond transfer; candidate `2♠`/`3♣` clubs, `2NT`/`3♦` diamonds):

| leg | n (NV) | plain NV | plain BV |
| --- | --- | --- | --- |
| club | 57,495 | **+58,256** (+0.0126/bd) | **+63,018** (+0.0137/bd) |
| diamond | 49,380 | **−92,508** (−0.0201/bd) | **−83,517** (−0.0181/bd) |
| rest | 36,283 | −1,424 (−0.0003/bd) | −6,855 (−0.0015/bd) |

`2NT → 3♣` is the whole run's biggest cell (**+46,715 NV / +54,641 BV**), so
unwinding N1c's right-siding trade for a natural club rung is right and the
club leg is kept. Falsifier 1 (the exactly-five 8-9 hand has nowhere to go) is
therefore **not** where the loss went. The five worst cells NV are all diamond
or sell-out: `3♣ → 2♦` −36,875, `3♣ → 3♦` −36,119, `3♦ → -` −22,119,
`3♣ → 2♥` −10,344, `3♣ → 2NT` −8,708.

**Five findings, each of which is a row of the refinement below.**

1. **No weak diamond rung beats the baseline's wide transfer.** Re-solving the
   boards where the baseline bid its `3♣`→♦ transfer, bucketed by responder's
   own diamond holding, every candidate call is negative on plain DD and the
   INV+ rung is the only one inside −1.0 per fired:

   | responder holds | lia bid | n (NV) | plain/fired NV | BV |
   | --- | --- | --- | --- | --- |
   | 6♦ thin, 0-4 HCP | `3♦` | 13,533 | **−1.966** | −1.773 |
   | 6♦ thin, 5+ HCP | `2♦` | 9,223 | **−2.534** | −2.464 |
   | 6♦ two top honors | `2♦` | 1,361 | −2.617 | −2.290 |
   | 7+♦ | `2♦` | 2,345 | **−4.238** | −4.859 |
   | 7+♦ | `3♦` | 2,897 | −2.701 | −2.713 |
   | any | `2NT` | 11,839 | −0.709 … −0.762 | −0.342 … −0.712 |

   This is the "hybrid nobody has built" from the repair queue, priced: the
   diamond leg wants a transfer, and the weak rungs it replaced are worth about
   −2 IMPs/fired each.

2. **The `Pass`@0 sell-outs were selling out to a floor that was right.** Cell
   `3♦ → -` — the baseline competed to `3♦`, the candidate passed — is 14,717
   boards at **−22,119 plain NV / −13,364 BV**, and 14,699 of them are after
   our own `2♦` escape: `1NT (2♣) 2♦ (2♠) -` (6,836 bd, −9,724) and
   `1NT (2♣) 2♦ (2♥) -` (5,895 bd, −11,878). The 2026-09-01 census read the
   floor's behaviour correctly (it does bid) and drew the wrong conclusion from
   it. **The code comment at `lebensohl.rs` calling the floor's `3♦` "a
   law-of-total-tricks violation" was wrong** — the doc/code discrepancy
   flagged to jdh8 on 2026-09-01 is resolved in the measurement's favour, and
   the comment is gone.

3. **Right-siding is not null here, and the first reading of it took the wrong
   column.** Declaring *side* flips on 123 NV / 66 BV boards — that is the
   zero that was recorded. Declarer *seat* flips on **26,664 NV / 27,911 BV**
   same-contract boards, worth **−6,406 (−0.0014/bd) / −10,416 (−0.0023/bd)**
   plain, worst cell `2NT → 3♣` (−3,630 / −5,600). Package C's correction to
   the iron rule applies exactly: DD prices the lead direction, so a seat flip
   is visible and real, and what the natural rungs gave back was declarership.

4. **The by-length answer answered the wrong question.** `comp:landy-length`
   told responder which partscore to pick; an invitational rung's question is
   whether this is a game.

5. **The `X`-versus-takeout cell is genuinely mixed.** Cell `X → 2♥` — the
   baseline doubled, the candidate took out — split by responder's own majors:
   the 8,113 NV / 6,236 BV boards holding two-plus in each major are **+1.854 /
   +1.154 plain** IMPs per fired for the takeout and **−1.305 / −2.169** under
   perfect defense. Plain says take out, PD says double, and the runner's own
   arbitration note says the one PD loss this lane does *not* wave through is a
   mechanism that removes our penalty doubles — which is what taking out with
   values does.

##### Refined on the lia2 forensic (2026-09-01) — **measured a LOSS 2026-09-02 (lia3)**

Same knob, still default off, defaults byte-identical. The refinement keeps the
club leg intact and re-cuts everything the forensic convicted.

###### Responder's table

| Call | Weight | Rule | Alert |
| --- | --- | --- | --- |
| `3♥`/`3♠` | 179/178 | splinters, unchanged | `comp:landy-spl` |
| `2♠` | 174 | `len(♣, 6..) & points(8..)` | `comp:landy-minor` |
| `2NT` | 173 | `len(♦, 6..) & points(8..)` | `comp:landy-minor` |
| `X` | 145 | `hcp(8..) & len(♥, 2..) & len(♠, 2..)` — **narrowed** | `comp:landy-values` |
| `2♥` | **144** | `4+♣ & 4+♦ & points(8..)` — **no major term**, and below the double | `comp:landy-tko` |
| `2♥` | **143** | `len(♣, 5..) & len(♦, 4..=4) & points(4..=7)` — the weak band, **new** | `comp:landy-tko` |
| `3♦` | **142** | `(len(♦, 7..) \| len(♦, 6..) & top_honors(♦, 2..)) & points(..=7)` | none — natural |
| `3♣` | 141 | `len(♣, 6..) & points(..=7)` | none — natural |
| `2♦` | 140 | `len(♦, 5..) & hcp(..=8) & floors` — **ceiling raised** | none — natural |

`3NT`@180/@168, the `4M` jam and `Pass`@0 are the shared table verbatim.

**The takeout moves under the double, and the double gains a shape term.** Lia
states the takeout as shape in the minors and says nothing about the majors, so
neither band carries a major term and the rung *order* does that work instead:
8+ with two-plus in each of their suits doubles, 8+ with a short one takes out,
10+ with a singleton splinters. That is finding 5, resolved for the double —
plain favours the takeout by +1.854/+1.154 per fired and PD favours the double
by −1.305/−2.169, and this lane does not wave through a PD loss whose mechanism
is removing our penalty doubles. **Pre-registered as falsifier 1 of the next
arm**: if the club leg's win shrinks and this is where it went, flip the two
weights back. The deliberate orphan is the 8-9 hand with a short major and not
both minors, which now has no rung.

**The weak takeout band is ours, not hers**, and it exists for the hand no
other rung reaches: five-plus clubs and exactly four diamonds at 4-7, too short
of diamonds for the escape and too short of clubs for `3♣`. Its five-card club
guarantee is what makes opener's `3♣`-before-`3♦` answer priority safe.

**The diamond leg is re-cut three ways** (finding 1). `2NT` carries the whole
INV+ band — it is the least-bad diamond cell in the forensic by a factor of
three — `3♦`@142 re-gates to *excessive* diamonds, and the `2♦` escape's
ceiling rises to eight HCP to take everything they leave ("bid `2♦` if
possible"). `defense_2c_landy_weak_2d_cap` keeps governing the base arm only:
it caps a rung this ladder has re-cut, so crossing them would measure two edits
at once. **Pre-registered residue**: the weak six-card diamond hand is still
off the transfer, and that is the −1.97/fired cell. If the next arm's diamond
leg is still negative, the answer is the wide transfer (`points(2..)` on `2NT`)
and the INV+ gate is what has to go.

###### Continuations

* **Opener answers the minor rungs off one Max-break+ table**
  (`landy_lia_max_break`), the same shape on both legs:

  | Answer | Meaning | Weight |
  | --- | --- | --- |
  | the step below the completion (`2NT` over `2♠`, `3♣` over `2NT`) | super-accept: maximum with three-card support (`comp:landy-super`, **new**) | 161 |
  | `3NT` | maximum, no super-accept — `landy_lia_accept` verbatim | 160 |
  | the completion (`3♣` over `2♠`, `3♦` over `2NT`) | the minimum default, and the finite catch-all | 100 |

  Findings 3 and 4 in one edit: the completion puts **opener** in the contract
  on the common branch, which recovers the right-siding the length table gave
  back, and the two rungs above it answer whether this is a game.
  `comp:landy-length` retires with `landy_lia_minor_answer` and
  `landy_lia_misfit_rebid`. Three-card support is the reversible default on the
  super-accept — opposite the rung's six that is a nine-card fit; flip it to
  `4..` if a probe disagrees.
* **Responder over the super-accept** (`landy_lia_super_rebid`): the `4m` slam
  try, `3NT` on both majors held, else retreat to three of the minor, which is
  the **finite catch-all** rather than a `Pass` — on the diamond leg the
  super-accept is `3♣` and passing it would leave a six-one fit on the table.
  Opener sits for the retreat. The `3NT` gate is `points(9..)`, one below every
  other lia rung, because opener has published `hcp(16..)` here and nowhere
  else; at ten it would strand exactly the hand the super-accept exists to
  find.
* **Responder over the completion**: `landy_bba_transfer_rebid` verbatim — the
  cues, the `4m` slam try, `3NT`, and `Pass`@0 for the invitation declined.
  `landy_recue_answer` rides this leg only; the super-accept's rebid has no
  cues, and a node under a call no arm makes is a book node shadowing the floor
  for nothing.
* **Responder over the `3NT` accept**: `landy_lia_accept_rebid` unchanged, with
  `landy_slam_answer` and `slam::rkcb_rows` re-hanging on all three legs
  ([minor-transfer-slam.md](../minor-transfer-slam.md)).
* **The weak takeout band gets one rung**: `3♣`@60 pulls opener's `2NT` on
  `len(♣, 5..) & points(..=7)`, and opener sits for it (`2♥ - 2NT - 3♣ -`).
  Opener denied a four-card minor to bid `2NT`, so it is facing a five-two club
  fit with no values, and sitting it is the one thing the weak hand must not
  do. Unauthored the floor bids a phantom `3♦` there.

###### The contested surface, restricted to the rungs that promise something

Finding 2 as built. `landy_lia_overcalled` loses its `values` parameter and its
`Pass`@0, and its registrations over the weak `3m` sign-offs and the `2♦`
escape are **deleted** — those seats are the floor's again, which is what the
measurement says they should have been. What remains rides `2♠`/`2NT` only and
is three narrow rungs with no catch-all:

* `3NT` — the accept, where their call left room for it below the game.
* `X` — penalty on `len(major, 4..)`, the oracle's gate, at the three level and
  below.
* the **completion**, while their entry has left it cheap enough to be legal —
  new, and the same right-siding argument as the quiet table. In practice it
  fires at `2NT (3♣)` and nowhere else: the club leg's completion is `3♣`
  itself, and every entry above `2♠` is already past it. **A new rung owes a
  new seat**, and the build's adversarial review caught this one: responder is
  uncapped over `2NT (3♣) 3♦`, and left to the floor that seat is sound on
  strength (pass 8-11, `3NT` 12-14) but bids a phantom `4♥` on ~8% of the band,
  heart voids included, with opener answering `4♠` on every hand.
  `landy_lia_contested_rebid` takes it, its `(X)` tail answers verbatim, and
  opener sits for the game-forcing `4m`.

Those registrations are exact **`Pattern::node`s**, not `Pattern::after`
guards. This is package A's finding re-used: `Trie::resolve_floored` returns a
guarded fallback's all-−∞ logits unchecked and the driver passes, so deleting a
`Pass`@0 behind a guard is a silent no-op. `landy_lia_contested_rebid` —
responder's captain table — is unchanged and keeps its `Pass`@0: it is the seat
that knows its strength, and it must never be stranded.

###### Verdict (2026-09-02) — lia3 lost four of four; the lane parks behind a general floor rail

`scripts/ab-landy-lia3.sh`, `SEED_BASE=1788290089`, control `deeb0252`, 4.608M
boards/arm/vul, both isolation gates 0-foreign. Plain DD, the pre-registered
arbiter at both colours, is negative at both:

| vul | fired | plain DD /board | PD /board | SD plain | SD-PD |
| --- | --- | --- | --- | --- | --- |
| NV | 197,920 (4.30%) | **−0.0056** ±0.0011 (−25,685) | −0.0331 ±0.0013 | +0.0058 | −0.0158 |
| BV | 163,430 (3.55%) | **−0.0254** ±0.0012 (−117,094) | −0.0569 ±0.0014 | −0.0090 | −0.0342 |

The knob stays off. Against lia2 (−0.0077/−0.0059) NV is a shade better and
BV four times worse, on a third more fired boards. The VERDICT block in the
runner carries the full forensic (every number below was recomputed by an
independent twelve-claim adversarial pass the same day; its corrections are
folded in). What it says, in order of size:

* **The diamond leg did not move** (−86,528 / −91,463 plain, −0.0188 /
  −0.0198 per board against lia2's −0.0201 / −0.0181) and every one of its
  cells loses to the baseline's wide `3♣` transfer: the six thin diamonds
  that now pass (`3♣ → -`, −35,563 / −34,235 on 13,552 / 12,757 boards), the
  INV+ transfer (`3♣ → 2NT`, −26,135 / −27,514), the escape (`3♣ → 2♦`,
  −19,540 / −16,145) and the excessive-diamond sign-off (`3♣ → 3♦`, −19,274 /
  −17,213). Finding 1's pre-registered residue is answered: no rung set for
  the diamond hand beats the wide transfer, so on that leg **the INV+ gate is
  what has to go**.
* **The club leg's win halved** (+27,990 / +14,627, was +58,256 / +63,018).
  `2NT → 3♣` still carries it (+40,933 / +39,039); the INV+ rung flipped
  (`2NT → 2♠` −6,848 / −11,862, was +14,368 / +14,194) through the Max-break+
  answer (the `2♠ - 2NT` super-accept prefix, −8,890 / −11,951 — finding 4's
  replacement loses) and the contested seats the refinement handed to the
  floor (`2♠ (3♠)` −3,848 plain NV at −4.8 per fired; its `4♠` class −9.9).
* **The weak band is the both-vul catastrophe.** `- → 2♥` (baseline passed,
  lia took out on the 4-7 band) is −2,662 NV but **−42,572 BV** on 28,817
  boards (−107,283 PD). The route is the flagged accept: `2♥ - 3NT` is
  −8,317 / −36,378, and split by responder's HCP (a proxy; the band is in
  points) the 0-7 band is **−14,515 / −38,483** (10,284 / 8,841 boards)
  while the 8-9 band is +6,198 / +2,105. Opener's `3NT`@160 opposite four
  points is doubled and sat.

**The floor: association first, then the fact that holds.** A new tool,
`examples/probe-layer-replay`, re-bids every divergent board through the arm's
own partnership and stamps each of our calls book-or-floor (0 of 361,350 boards
failed to reproduce). Boards on which the learned floor made at least one of
our calls at or after the divergence: NV 124,936 boards, −72,224 plain
(−118,074 PD); BV 94,870, −132,357 (−181,836); boards the book bid to the end
+46,539 / +15,263. That is an association, not an attribution: the **base**
arm's floor is involved on *more* of the same boards (NV 141,958 vs 124,936,
BV 107,264 vs 94,870), the divergent call is the book's on 177,112 / 148,603
boards, and half of the lia floor-involved plain loss sits on boards where the
floor only ever passed (NV −38,515 of −72,224, BV −37,199; PD-positive there).
The fact that holds: **lia doubles the boards on which the floor makes a
substantive, non-pass call** — NV 86,491 against the baseline's 42,994, BV
62,135 against 23,066 — and the cell where lia's floor bids while the
baseline's book or floor only passed is NV 67,239 boards / −29,948 plain /
−165,161 PD, BV 53,103 / −97,643 / −213,559. (Every bucket here is over
divergent boards only; identical boards are absent by construction.) Two
floor classes are worth naming:

* **Phantom suits** — floored suit bids on ≤4 cards where partner's announced
  minimum makes ≤5 combined: `2♠ (3♠) 4♠` on a doubleton (−10 per fired), `3♦
  - - (3♠) 4♦ - 4♠` (−10), `2♥ - 3♣ - - (X) - - 3♦ - 3♥` (−5 to −8).
  `probe-decision` shows the net reading partner correctly (♣6+, 8+ points)
  and bidding `4♠` anyway, logit 9.41 over `4♣`'s 9.34. The mechanism is the
  input, not the reading. The shipped floor's vector (`features_v6`, 176
  values: the disclosable hand summary, a 36-value context block, all four
  seats' announced envelopes, vulnerability, both compact convention cards)
  carries a *we-bid-this-strain* bit per strain and partner's last bid — raw
  call identity, set for every bid artificial or not (`context.rs:616`) — and
  **no alert or tag column** (measured at +0.004 NLL and left out; the
  evaluator's call-identity tail reaches the floor only through
  `competitive_gate` and the accountant). Over `2♠ (3♠)` the net is told "we
  bid spades, partner's last bid was `2♠`, partner has ♣6+", a joint the BBA
  corpus never showed it; the compact card has no lia slot
  (`defense_2c_landy_lia` is never read in `features.rs`), so the regime
  vector is N1j's, under weights from 2026-08-18 (`9fb333f5`). So alerting
  more changes nothing it sees, and masking the strain bit for alerted calls
  would move its inputs off the training distribution the other way — a
  retrain, not a rail.
* **Six-card pushes** — floored `3♦`/`4♦` on the weak rung's own suit over
  their raise (`3♦ - - 3♠ [4♦]` −9.3 per fired; the balancing `- 2♥ - - [3♦]`
  −3.9): level judgment, not phantom, and outside a suit veto.

**Falsifiers.** 1 **does not fire on its own terms**: the club leg's win did
shrink, but it went to `2NT → 2♠` and `2NT → 2♥`, not to `X → 2♥`, which is a
CI-clear plain win (+18,457 / +6,138; PD −3,064 / −14,130, the artifact
shape) — with the caveat the pre-registration missed, that the cell is 100%
short-major 8-9 hands because the 2-2-major hands the flip gave to `X` now
match the baseline's `X` and never diverge (`2♥ → X`: 0 boards), so the
flip's central move is unmeasured. 2 **fires** (above; the recorded repair is
the accept below the minor picks). 3 **not supported on plain DD and
PD-negative**: exactly-five 8-9 diamond hands are NV `X` 4,082 boards +6,007
plain / −2,795 PD, `2♦` 1,851 +2,414 / +1,003, pass 365 +327 (BV `X` +1,470 /
−8,541, `2♦` +402 noise, pass +453; class −1,358 / −9,329 PD) — and the `X`
bucket is not the ladder's `X`: the baseline doubled on every one of those
boards and 3,375 / 3,248 of them diverge only at responder's later floored
`3♦` over `X (2M) - -` (+6,512 plain of the +6,007), the floor's pull. 4
**fires**: `3♦ - - 3♠` 605 boards −5,543 at −9.2 per fired and `3♦ - - 3♥`
759 / −4,130 (BV 139 / −1,355, 154 / −894); over `3♠` the floor pushes `4♦`
on 550 of 605 (always 6+) and, when RHO passes it, always phantom-cues `4♠`
(427 boards, own ≤4 on 415); over `3♥` it passes as often as it pushes (396
vs 363) and the passes cost nearly as much (−1,679 vs −2,451). lia2's −22,119
`3♦ → -` sell-out is gone by **removal**, not reversal — its node `2♦ (2M)`
keeps 2 / 6 boards and the residual cell (1,126 / 271, +244 / +474) sits at
`X (2M)`, floored in both arms — lia3 measures nothing about the floor at
the old node. 5 **partly** (`2♥ (2♠) -` −4,401 /
−3,631, the floor sits at −1.1 / −1.5 per fired; `2♥ (3♥) X` −875 / −7,358
plain but −13,846 / −19,418 PD; nothing here is the takeout's problem, the
band is). Watch list `2♥ (4♥/4♠)`: 344 / 302 boards, +45 / −18 — nothing.
Predictions: diamond leg non-negative **no**; club leg positive yes, halved;
`3♦ → -` flips — removed, see 4; declarer-seat flips recover **no** (same
contract with doubled status, both NS, other seat: −6,728 on 28,345 NV /
−9,091 on 27,378 BV against lia2's −6,910 / −10,323 by the same definition,
flat at −0.0015 / −0.0020 per board).

**Three book mechanisms the verification pass named** (the lia4 list below
is built on them, not on the leg totals):

* **The diamond leg's biggest cell is a hand with no rung.** `3♣ → -` is
  13,534 of 13,552 NV and 12,751 of 12,757 BV boards of responder **0-4 HCP
  with exactly six diamonds**: `2♦`@140 needs five HCP (`floors`), `3♦`@142
  seven diamonds or two top honours, `2NT`@173 eight points — so `Pass`@0.
  −35,552 / −34,223 plain, a PD wash. In lia2 the same class bid `3♦` at
  −1.97 / −1.77 per fired (−26,610 / −22,615): the re-gate cost −8,942 /
  −11,608 *more*. The baseline's wide transfer takes this hand at
  `points(2..)`; that, not the INV+ gate as such, is what "the wide transfer
  wins" means.
* **Both INV+ rungs lose through `landy_lia_super_rebid`'s `retreat`@0.**
  Opener's super-accept relay@161 outranks its own `3NT`@160, so it may hold
  the stoppers; responder's `3NT`@120 then needs both major stoppers and
  `4m`@130 needs 13+, and the 10-12 HCP hand retreats to three of the minor:
  diamonds NV 2,050 boards / −9,737, BV 1,434 / −10,491; clubs NV 2,084 /
  −9,486, BV 1,591 / −10,822 (≈ −19.2k / −21.3k plain) — `3NT` (base) → `3m`
  (lia) on 1,751 / 1,251 boards at −5.5 / −8.3 per fired. This is the club
  leg's flip; the "Max-break+ answer loses" line above is this table, not
  the accept itself.
* **A floored-both cell the reading moved.** `- → X` (NV 8,422 boards /
  −11,801 plain / −32,459 PD; BV 6,200 / −12,162 / −31,976, the third-largest
  PD cell) is 6,558 / 4,977 boards at `X (2♠) - -` where **both** arms are
  floored (lia doubles again, base passes): the narrowed `X` reading moves
  the floor's reopening.

**Colour and the SD seam.** The diamond leg is colour-neutral (−86.5k /
−91.5k); the 4-7 takeout band is the whole colour effect — the `2♥` rung's
0-7 HCP boards are −12,856 NV against **−58,640 BV**, half the BV loss, and at
NV the `2♥ - 3NT` loss is entirely the doubled `3NT`s (3NTx 2,398 / −17,205,
undoubled 6,724 / −873) while BV also loses undoubled (5,918 / −11,810); a
vulnerability gate on the weak band is the obvious untested arm. The NV
sd-lead plain +0.0058 ±0.0011 is a CI-clear win the size of the DD loss (DD −
SD = −0.0114 NV / −0.0164 BV, the opposite sign to package D's +0.014), so the
NV verdict sits inside the lead-model seam; BV loses on all four columns. And
the sd pass ran with no `--on-ns-landy-lia` disclosure — `ab-dump-sd` has none
— the caveat this doc already attaches to D.

**The rail evidence.** Cutting the floored suit bids by the two inputs an
envelope-gated veto would read — the bidder's own length and partner's
announced minimum in the suit, no bid-identity term — the class *floored suit
bid, own ≤4, own + partner-min ≤5* at or after the divergence: on the lia
arm NV 12,513 boards, net **−30,369 plain** (gross −54,120 lost / +23,751
won; −65,663 PD), BV 7,177, net **−33,277** (gross −46,975 / +13,698; −55,043
PD); per call by level, four-level −27,642 / −26,536, three-level −5,241 /
−11,253, five-plus −1,529 / −1,718, two-level nothing (per call, so a board
with two vetoable calls counts twice; the level sum overstates the per-board
net by ~13% — four-level deduplicated 10,686 / −26,687 NV, 5,667 / −24,843
BV). **The base arm on the
same boards** carries one on 6,800 / 3,844 boards and lost on them — **5.2 /
6.5 IMPs per fired**, +35,346 / +25,145 plain net from the candidate's side
(gross +45,554 / +32,010 won against −10,208 / −6,865): the default system's
own floor phantom-bids the same way, lia only put more seats in front of it.
These are the pools a veto would act on, not bounds and not measurements: the
masked call is replaced, not undone; the veto also fires on the ~4.4M
non-divergent boards absent from these files; and a floor change moves both
arms, which is why its A/B is on the default system. The six-card push class
is outside it.

**Disposition (jdh8, 2026-09-02): the general fix first.** The
envelope-gated new-suit veto on the floor — the M6.4-style rail named as this
lane's residue in `docs/archive/one-notrump-competitive-closed.md` since the
2026-08-14 post-ship decompose — gets built and measured on the **default
system** with its own non-inferiority A/B, as its own task. No local nodes
for lia until that has run; if the rail flips the phantom classes here, the
lane re-measures under it (lia4) with the book findings above as its only
book changes: the accept below the minor picks (falsifier 2), the 0-4 HCP
six-diamond hand back on the wide transfer, a values `3NT` in the
super-accept rebid, and a vulnerability gate on the weak band. Recorded and **not built**: restoring
`landy_lia_overcalled`'s `Pass`@0 over the INV+ rungs (lia2 had it, at sha
`32242d63`, and that leg won). Kept: both arm directories, `imps-*.jsonl`,
`layers-*.jsonl` (candidate and base), and the bucket tables under
`ab-results/landy-lia3/`.

###### Owed, and flagged

* **The A/B** — `scripts/ab-landy-lia3.sh`, **run 2026-09-02, lost 4/4 —
  see the verdict above**; the falsifier outcomes are there too. As
  pre-registered: fresh `SEED_BASE`, control = then-current `main`, both
  colours, sized to match the three prior runs. The falsifiers were: the `X`-versus-takeout weight flip (1), opener's accept over
  the two-band takeout (2), the diamond leg's remaining weak-hand residue (3),
  whether handing the weak rungs' contested seats back to the floor is
  worth the −22,119 the sell-outs cost (4), and the contested takeout's own
  floor handoff (5, next bullet). The lia2 verdict block above
  carries the numbers each of them is measured against. On the forensic
  **watch list**, no pre-stated number: `2♥ (4♥/4♠)`, where the floor's
  delayed double was the worst per-fired cell class one rung over
  (`2♠ (4M)`, −3.434/−4.268) but this seat was never priced.
* **The contested `2♥ ({raise})` seat is the floor's (2026-09-02).** The
  pre-launch review caught `landy_bba_takeout_overcalled` riding the two-band
  takeout: its `Pass`@0 was authored safe "because responder's game force
  guarantees another turn", and no lia band is a game force — the weak band
  never bids again, making the `Pass`@0 the sell-out class the lia2 A/B
  convicted, and its free 2NT/3NT-on-a-stopper a game bid opposite a possible
  four-count (the same shape as the flagged `3NT`@160 accept, one seat
  later). The registration is skipped under `lia_takeout` — the same handoff
  as the weak rungs' contested tails; the splinters (`points(10..)`) keep the
  ladder. The old forensic tallied this seat at 16 boards, −35/−10, but the
  weak band raises its frequency. Reversible: drop the `!lia_takeout` guard
  in `landy_bba_entries`. **Falsifier 5**: if the lia3 forensic shows the
  floor selling out or blind-pushing these cells, the seat wants a lia table
  gated by band-consistent values, not the old ladder back. **Outcome: partly**
  — the floor sits on `2♥ (2♠)` at −1.1 / −1.5 per fired, and the doubles at
  both-vul are PD-heavy; the band, not the seat, is the loss.
* **The pre-launch reading gate PASSED (2026-09-02)** — with the correct
  invocation. `probe-call-reading --their-2c-landy --ns-landy-lia` reads every
  re-cut call soundly and tightly: `2♥` 4-9 points, ♣4+, ♦4-5 (the strong
  band's 8-9 cap is genuine rung-order inference — 10+ both-minors hands are
  shape-forced into `X`/`3NT`/splinter first), `2NT` 8+ with ♦6+ ♣≤5, `X` 8-9
  with the 2+2+ major terms (the refinement's owed reading-union item, present
  after all), sign-offs 0-7 on their six-card suits, escape 5-9 on ♦5-6. The
  review's first probe omitted `--their-2c-landy`, under which
  `--ns-landy-lia` is **inert** — `lebensohl_package` registers nothing and
  `1NT (2♣) …` resolves through the systems-on rebase to the constructive
  tables, whose "♦2-2 on `2NT - 3♣`" is that lane's own sound
  diamond-transfer-break description, bidder and reading agreeing via the same
  `Fallback::Rebase`. Half a day chased that as a phantom before the
  cross-build comparison surfaced it. The example now warns on the inert
  combination and its flag doc carries the "needs `--their-2c-landy`" sentence
  its doubler-rebids sibling always had; the ~dozen other `--ns-landy-*`
  modifier flags share the trap and are **flagged, not fixed** — a warn or an
  implied disclosure per flag is a one-line decision each, owed to the flag
  audit, not this package. The A/B harness derives the disclosure from the
  opponent's card, so no measurement was ever affected.
* **Opener's `3NT`@160 accept over `2♥` can now fire opposite the weak band.**
  The accept was authored when the takeout promised 8+, so a maximum meant 24+
  combined; with the 4-7 band under the same call it can be 20. `2♥ - 3NT -`
  is an authored sit, so responder cannot pull it. This is a defect the weak
  band introduced and it is deliberately left standing rather than patched
  blind — the alternative is to weight the accept *below* opener's minor
  picks, which protects the weak hand at the cost of missing the 8-9 × maximum
  game, and choosing between two unmeasured tables on analysis alone is what
  the iron rule forbids. Both are one weight. **Falsifier 2 of the next arm**:
  bucket `2♥ → 3NT` boards by responder's own strength; if the 4-7 half is
  where the takeout's loss is, the accept moves down. **Outcome: fires** —
  the 0-7 band is −14,515 / −38,483 plain against +6,198 / +2,105 for 8-9;
  the accept moves below the minor picks in any lia4.

* **The `3♣` sign-off's contested tail is now the floor's on inference, not on
  measurement.** The convicted cell was the `2♦` escape's tail; the `3♣` one is
  deleted by the same argument but was never priced on its own, and the census
  says the floor pushes `4♣` on 96% of `3♣ (3♠)` boards. Reversible: restore
  the `{3m} ({over})` loop.
* **Six thin diamonds under the escape's five-HCP floor now pass.** `3♦`'s
  re-gate to excessive length reopens part of the old "starved diamonds" seam
  for the 6♦ 0-4 HCP hand — deliberate, since that band measured −1.97/fired on
  the rung it is losing, but it is a pass where the baseline transfers.
* **`landy_recue_answer`'s `4m`@20 still has no answer node** — unchanged,
  still shared with the base arm, still owed its own A/B.
* **`park/landy-pdi` owes a rebase** for the two `.penalty()` rows, unchanged
  by the refinement.
* **`alert-sites.txt` does not move.** `comp:landy-super` is a new slug and
  `comp:landy-length` retires, but the fixture's four sections are all built at
  `defense_2c_landy_lia = false`. Both slugs' disclosure records are in
  `card.rs`, so a ship needs no new decision.
* **Byte-identity**: `smoke-default --count 20000 --seed 1` byte-identical
  against `main` HEAD, verified twice.

#### Package C — `landy_texas`: **shipped default-on 2026-08-31**, an eight-of-eight sweep

Seed 1788181796, control `8dca085a`, 4,608,000 boards/arm/vul, both isolation
gates **0 foreign**. Every cell positive:

| vul | fired | plain | PD | sd-plain | sd-PD |
| --- | --- | --- | --- | --- | --- |
| none | 1280 (0.03%) | **+0.616**/fired (+788) | **+0.816** (+1045) | +0.220 (+281) | +0.305 (+390) |
| both | 931 (0.02%) | **+0.711**/fired (+662) | **+0.996** (+927) | +0.352 (+328) | +0.506 (+471) |

Per board that is +0.0002/+0.0001, because the rung is rare; the per-fired
plain cells are >6.7σ NV / >5.6σ BV off the printed per-board CI bound. Ships
default-on under the decision table's plain-DD-win row, not the non-loss row
the design asked for.

##### The mechanism is the reverse of the one designed for

The package was built expecting a **plain-DD wash** — right-siding invisible
to the harness — carried by the **DD-visible slam reroute**. Both halves came
back inverted, and the correction is worth more than the IMPs:

* **The slam reroute is the invisible half.** Level 5+ is reached on **5 of
  2211** divergent boards, and **0** at both-vul. The 16+ drive is authored and
  correct, but at this frequency it contributed nothing measurable. Falsifier 1
  (the drive overbids) is moot rather than refuted.
* **Right-siding is what the harness priced.** 96.4% NV / 96.3% BV of divergent
  boards are the **same contract played from the other seat** — every one of
  them a declarer-seat flip, none a change of declaring side. That bucket is
  the whole win.

The general lesson, which corrects a load-bearing assumption in the iron rules:
**double dummy is blind to right-siding's *concealment*, not to its *lead
direction*.** The solver prices a specific (contract, declarer) pair, and
North-declaring `4♥` puts East on lead where South-declaring puts West. Those
are different hands, so the trick count honestly differs — leading *through*
the 1NT opener's tenaces versus *into* them is a double-dummy fact. What needs
single dummy is the extra edge from the leader not being able to see the closed
strong hand. sd-lead confirms the split from the other side: with a blind
leader the killing lead is found less often **in both arms**, so the gap
narrows to about a third (+0.220/+0.352) — still positive at both colours,
and the more realistic number of the two.

**Adopted 2026-09-01** (jdh8: "the iron rule is good, it is just the minor claim
that DD is blind to right-siding that needs refinement"). CLAUDE.md's iron rule
and [measurement.md](../measurement.md)'s rule list now read *right-siding is
half-visible — DD prices the lead direction, not the concealment, so it sees
the idea iff declarer actually moves*; the "measuring zero is real" clause
survives, narrowed to the no-flip case that §N1o's notrump endings exhibit.
Three live items that deferred on the old premise were requeued: N3-xfer's
`(3♣)` transfers (above), the `(3♣)` transfer design note, and the
splinter-vs-fragment question in
[bba-1nt-splinter.md](../ai-bidder/bba-1nt-splinter.md), whose whole argument is
about which defender leads and through what — the half the solver can see.
Historical ledger rows that read "DD-blind right-siding" stand as records of
what was concluded at the time.

Falsifier 2 (the alerted transfer tells the defense the anchor major before the
lead) is **refuted** by the same rows: sd-lead is the scorer that could see an
information leak, and it is positive at both vulnerabilities under both
scorers. Falsifier 3 needed no relaxation — the sit rail costs nothing
visible.

**Residual cost, for anyone who revisits this rung.** The transfer hands the
opponents room on 4.1% NV / 4.0% BV of fired boards (52 / 37 boards, and zero
the other way), and draws 33 / 30 doubles the direct jam never drew — those
boards are the entire worst-5 list in both reports (`1NT 2♣ 4♣ - 4♥ - - 4♠ …`,
where the direct `4♥` had shut the auction). The right-siding gain outweighs it
about 5:1, so this is a refinement target, not a defect: a gate that keeps the
direct jam when the transfer's extra round is likeliest to be used against us.

PD exceeding plain here is **not** the usual doubling artifact and does not
need discounting: perfect defense prices a failing contract as doubled either
way, so its uplift comes from the baseline's wrong-sided contract failing
*more* under a defense that always finds the killing lead. Same direction as
plain, larger magnitude, coherent mechanism.

##### As designed (build record)

The `4♣`/`4♦` seat was floor-owned before this shipped, so the jam declared
from the
wrong side and a 16+ hand cannot look for slam. On, the jam rides South
African Texas — `4♦`@170 → ♠ / `4♣`@169 → ♥ (`4♦` outranks `4♣` as `4♠`@172
outranks `4♥`@171 today: 6-6 wants spades), the uncontested `texas` alert
slug reused (now `pub`), the jam's gate carried verbatim
(`points(landy_texas_floor..) & len(6..)`; the floor knob exists for a later
sweep and is **not** swept here — this package changes only which call
carries the hand). The freed direct `4♥`/`4♠` are the uncontested NF
slam-try tier verbatim (`len 6+ & other ≤4 & hcp(15..=direct_4m_max)`,
`slam_try_answer` + RKCB above); a 16+ hand transfers and drives its own
`4NT` (`texas_slam_drive_rebid` + RKCB above the completion). Completions
answer the `-` and `(X)` tails plus the systems-on rebase; deeper overcall
tails stay the floor's, as the minor transfers' do. No `4♣` collision: the
direct `4♣` and the `4m` slam try sit at different prefixes.

**One deviation from the uncontested twin, deliberate:** the drive seat
carries a `Pass`@0 sit rail. Uncontested that seat belongs to `instinct()`,
which never pulls a completed transfer; this lane's floor is the **learned**
one §N1o caught cue-bidding a dead four-level to `6♥` doubled — the same
measured defect the jam's sit node insures against. Expected verdict shape:
**a plain-DD wash is real, not an artifact** (right-siding is invisible to
the harness by construction); the DD-visible half is the slam reroute. Ships
on a non-loss.

#### Package D — `landy_notrump_no_major` re-measured on A's winner

§N1p's `nt` loss was measured with the broken catch-all in place — its own
falsifier 2 says the doubler's rebid was the floor's, pulling a values double
to `3NT` 49.5% of the time on two trumps, so that arm partly measured the
floor. Package A deleted the shadowing catch-all and authored the three-card
cells, which is the repair that unblocks this re-measure. Control is
post-C `main`, fresh seed.

**New runner, and a deliberate departure from the plan's wording.** The plan
said "the `nt` pair only" of `scripts/ab-landy-notrump-shape.sh`, whose arms
hold `landy_major_jam` **off** on both sides — the framing §N1p measured. Two
things have changed since, so package D runs
`scripts/ab-landy-nt-remeasure.sh` (`base` = bare `main`, `nt` = `+
--ns-landy-notrump-no-major`) instead, and §N1p's runner keeps its own
experiment's record untouched:

* The jam has been default-on since 2026-08-30 and C's Texas since
  2026-08-31, so a strong six-card major now leaves via `4♣`/`4♦`. It never
  reaches `3NT`@168, and `nt` cannot move it either way. Holding the jam off
  would measure a pool `main` no longer routes that way — against the iron rule
  that a ship decision is measured against the real routing.
* The moving pool is therefore the **four- and five-card** major game hands
  only. Narrower than §N1p's, and the whole ship-relevant question.

C is inert in the comparison either way: `landy_texas` is gated on
`landy_major_jam`, and with the jam on it is on identically in both arms.

##### Verdict — **measured non-win 2026-09-01, `landy_notrump_no_major` stays default off**

`scripts/ab-landy-nt-remeasure.sh`, `SEED_BASE=1788191041`, control
`60115871`, 4,608,000 bd/arm/vul. Both isolation gates **0 foreign** (of
54,309 / 36,591 divergent) before any headline.

| column | not vulnerable | both vulnerable |
| --- | --- | --- |
| fired | 54,309 (1.18%) | 36,591 (0.79%) |
| plain DD | **+0.0077** ±0.0007 (+0.652/fired) | **+0.0075** ±0.0008 (+0.943/fired) |
| PD | +0.0145 ±0.0008 (+1.228/fired) | +0.0146 ±0.0008 (+1.835/fired) |
| plain SD (sd-lead) | **−0.0061** ±0.0007 (−0.459/fired) | **−0.0056** ±0.0008 (−0.634/fired) |
| SD-PD | −0.0011 ±0.0008 (−0.084/fired) | +0.0000 ±0.0008 (+0.004/fired) |

(The two sd rows print their own denominator, 61,454 / 40,521 fired, against
the DD rows' 54,309 / 36,591 — `ab-dump-sd` and `probe-divergence` count a
moved board differently. Compare the **IMPs/board** columns, which share one
denominator; the per-fired figures do not.)

**Why this is not a ship, despite two CI-clear positive columns.** Read the
scorers by what each one's synthetic double does to *this* knob's mechanism —
the candidate stops bidding `3NT` and doubles instead, so the baseline's game
is the contract a synthetic `X` lands on:

* **PD is the decision table's `loss | win` row in its literal form** — "it
  credits phantom doubles of contracts we no longer bid". Every failing
  baseline `3NT` is doubled for free; the candidate, which no longer bids
  them, banks the difference. §N1p dismissed the same +0.018/+0.020 as the
  auto-double artifact and this is the same artifact on the same lane.
  Falsifier 1 pre-registered it. Discount the column.
* **The two columns with no synthetic double straddle zero**: plain DD
  +0.0077/+0.0075, plain SD −0.0061/−0.0056. They differ *only* in the lead
  model, and the knob's whole measured effect is smaller than the gap between
  them at both colours.
* **SD-PD — the column §N1p itself named "the arbiter" — carries the artifact
  in the candidate's favour and still cannot clear zero** (−0.0011, +0.0000).
  With a thumb on its scale it reads a wash.

So there is no scorer-independent win, and A's repair did not reverse §N1p:
on the arbiter column the two experiments read **−0.0012** (§N1p) and
**−0.0011 / +0.0000** (D). Falsifier 3 is answered — the repair *did* reach,
and what it bought is visible, but it bought it in the undoubled scorers only:

| | plain DD | plain SD | PD | SD-PD |
| --- | --- | --- | --- | --- |
| §N1p `nt` (NV) | −0.0124 | −0.0266 | +0.0181 | −0.0012 |
| D `nt` (NV) | +0.0077 | −0.0061 | +0.0145 | −0.0011 |
| move | **+0.0201** | **+0.0205** | −0.0036 | +0.0001 |

(A between-experiment comparison across a routing change — §N1p held the jam
off — so suggestive, not a measurement, per the series-break caution in
[measurement.md](../measurement.md).) The **lead-model seam is bigger than the
knob**: plain DD reads this knob **+0.0142** above plain SD at §N1p NV,
**+0.0138** at D NV and **+0.0131** at D BV — stable across two experiments,
two routings and two vulnerabilities, and roughly double the plain-DD effect
it is being asked to adjudicate. The sign is the tell: the candidate's whole
mechanism is *declaring `3NT` less often*, and the scorer that hands the
defence a clairvoyant opening lead is the one that likes it. This is the
documented 1NT-end blind-lead seam ("DD pessimistic
for declarer; sd-lead corrects it"), and the knob's plain-DD sign is decided
by the lead model rather than by the treatment.

##### Falsifier 2's split — the gate is a bundled disjunction with opposite-signed halves

`probe-divergence --imps --jsonl`, both vulnerabilities, bucketed by the
**mover's own major lengths** (the knob reads exactly that: `len(♥, ..=3) &
len(♠, ..=3)` on the bidder of `3NT`). `off=3NT` is 74.3%/79.9% of the
divergence and `3NT → X` is 98.9%/99.0% of that bucket, so the substitution
is clean. Inside it, plain-DD IMPs per fired board by the mover's major shape,
short-long:

| majors | n (NV) | plain NV | n (BV) | plain BV |
| --- | --- | --- | --- | --- |
| 4-4 | 1,754 | **+4.743** | 1,288 | **+5.902** |
| 4-5 | 532 | **+3.474** | 393 | **+4.947** |
| 3-4 | 10,258 | **+3.534** | 7,228 | **+4.736** |
| 3-5 | 2,074 | **+3.108** | 1,529 | **+4.504** |
| 2-4 | 12,470 | −0.125 | 9,012 | +0.307 |
| 2-5 | 4,006 | −0.199 | 2,948 | −0.271 |
| 1-4 | 4,734 | **−2.079** | 3,447 | **−1.864** |
| 1-5 | 3,609 | **−1.842** | 2,736 | **−2.304** |

Monotone in the **short** major across all eight cells, replicating
independently at both vulnerabilities, with the sign break between two and
three. The bridge reason is plain: they showed both majors, so a mover holding
3+ in each has them in a genuine misfit with nowhere to run and defending is
right, while a mover with a singleton major has handed them a big fit — the
double gets run out to something making, and `3NT` (where the short major is
partner's problem, not ours) was fine. Summed: the `min major ≥ 3` half is
**+52,861 NV / +50,663 BV** plain IMPs on 14,618 / 10,438 boards, the
`≤ 2` half **−18,838 / −10,758** on 24,819 / 18,143. The bundle nets the
difference, which is why it reads as a small win on the column that likes it.

This is [measurement.md](../measurement.md)'s **disjunctive-gate rule** — "when a
candidate gate is a disjunction and any slice gives its disjuncts different
signs, they are two arms, always" — and the bundle is currently burying a
+3.1-to-+5.9-per-fired cell inside a losing one. The split variable is a hand
feature the bidder knows before choosing, so it is a legal gate, not a
post-hoc outcome slice.

**Owed, not done: the narrowed arm.** Today's gate bars `3NT` whenever either
major is 4+. The gate the split asks for bars it only when *both* majors are
3+ **and** one of them is 4+ — the misfit hands — so `3NT` survives on every
hand with a doubleton or singleton major. On plain DD it would move
+0.0115 / +0.0110 IMPs/board against the bundle's +0.0077 / +0.0075, on
roughly a third of the traffic. Whether it clears the lead-model seam is an
A/B and not an extrapolation — though the direction is favourable, since the
seam lives on hands whose `3NT` stands or falls on the opening lead, and those
are exactly the singleton-major hands the narrowing stops moving. Until that
arm runs, `landy_notrump_no_major` stays **default off** and §N1p's flag 2
stays settled.

**Residual worth a line for whoever takes the narrowed arm.** The bucket where
the double did *not* end in defending — `both NS`, 15.4%/14.1% of divergence —
costs **−1.710 / −2.657 per fired**: opener pulls, and we declare something
worse than the `3NT` we gave up. That is a continuation defect on an authored
seat (A's cells), so it is the book's to fix, and it is inside the narrowed
arm's traffic too.

##### Flagged, not fixed: the sd columns disclose no Landy knob

`ab-dump-sd` has `--on-ns-*` disclosure flags for the free-bid, negative-double
and overcall families, and **none for any `landy_*` knob** (`--help | grep -i
landy` is empty). So the blind leader reads both arms with control semantics:
under the ON arm the responder's `X` is wider and its `3NT` narrower, and the
leader is not told. Every sd row in §N1-lia — package A's tie-breaks, C's
corroboration and D's arbiter column — carries that caveat. It does not
invalidate them (the seam's stability across §N1p, which ran under the same
condition, argues it is a lead-model effect rather than a disclosure one), but
it bounds them. Proposed reversible default: **leave the harness alone and
quote the caveat**, since adding flags is a harness change that would
re-baseline the sd column mid-campaign; add `--on-ns-landy-*` when the
narrowed arm above runs, so its sd row is measured with the knob disclosed.

#### The arms

Sequential, fresh `SEED_BASE` per package, 4,608,000 bd/arm/vul at the §N1p
scale, `probe-divergence --gate-opener ours` **0 foreign** before any
headline; every verdict bounded by flagged item 5 (`--filter-landy` admits
strictly balanced openers only). Runners with per-arm falsifiers in their
headers: `scripts/ab-landy-lia-doubler.sh` (A: `base | nocatch | hon | cells`,
adjacent pairs isolate each cell; penalty rungs arbitrated on **plain DD**,
PD double-blind), `scripts/ab-landy-lia.sh` → `ab-landy-lia-repair.sh` →
**`ab-landy-lia2.sh`** (B, three runners: the first two carry VERDICT blocks
for builds the probe correction superseded, the third is the live and
unlaunched one), `scripts/ab-landy-texas.sh`
(C). Renders: `--ns-landy-responder lia` and `--ns-landy-texas` in
`render-book`; every new knob has a `bba-gen`/`probe-call-reading` flag.
Invariants: four new profiles in `gated_profiles_preserve_alert_invariant`
(`their-landy-lia`, `their-landy-texas`, `their-landy-lia-texas`,
`their-landy-doubler-cells`); `cards/*.bbsa` unchanged throughout (the
`comp:landy-ask`/`comp:landy-super`/`comp:landy-minor` no-schema-name records
are in `card.rs`'s precedent block, written at build time (`comp:landy-length`
retired with the by-length answer in the lia2 refinement) — and BBA's schema has
no row for Texas over *their* Landy either, only for our own).
`tests/fixtures/alert-sites.txt` was unchanged while every knob was default-off
and moved once on C's ship, in its `[their-landy]` section only: `texas
80 -> 88` (the eight transfer sites), `completion 696 -> 692`, `rkcb
18404 -> 19232` (the drive's keycard ladder above the completion), with
`[kokish-kraft]` following its anchor. **The default profile did not move** —
the fixture is the standing proof that the knob is inert while their `2♣` is
undeclared or natural.

### Flagged, not fixed (§N1 — reversible defaults proposed)

1. **BBA's opener does not sit at `1NT (2♣) X (2M)` the way it does in the
   Multi lane.** New probes `opener-c-x2h`/`opener-c-x2s` (2026-08-28,
   100,000 hands/vul filtered to the 5,005 BBA opens 1NT with) read **67.3%
   / 67.6% Pass non-vulnerable and 79.3% / 80.5% vulnerable**, with a natural
   `3♣` on four-plus clubs taking the rest; the Multi twins on the same
   sample size pass 91.7–93.0%. `epbot_get_info_meaning` says why: BBA labels
   our `X` **“bidable suit”** over Landy and **“negative double”** over the
   Multi — it has no values double in this lane at all. Full table in
   [the counter-defense research](../ai-bidder/landy-2c-counter-defense-research.md).
   What survives for §N1l: BBA never *doubles* at that seat in either lane (no
   `X` bucket over 0.5% in eight cells), which is the evidence the scope call
   rests on. What does not: “opener sits, as in the Multi lane.” Proposed
   reversible default: **leave opener's seat to the floor** — §N1k authored it
   and lost — and re-open it only as its own arm after §N1l's verdict.
   **Acted on 2026-08-29**: that arm is
   [§N1m](#n1m--openers-own-rebid-over-their-advance-landy_opener_px--landy_opener_rungs-shipped-default-on-2026-09-16-as-a-package),
   built off the oracle, and **shipped default-on 2026-09-16** as the
   `px + rungs` package — so this item is **closed**: opener's seat is ours
   again, and the reversible default it proposed is the one that lost. The
   evidence the scope call rested on survives unchanged — BBA never doubles at that seat in either
   lane — and the oracle now says that is BBA leaving +2.8…+8.1 IMPs/board on
   the table whenever it holds four of the major it advanced.
2. **`landy_bba_responder`'s `3NT`@168 is ungated on high cards**
   (`points(10..)` alone, no stopper test), which is what caps the values
   double at nine points and kills §N1l's top two rungs in self-play. Proposed
   reversible default: **leave it**; re-gating moves every shape in the lane,
   not just this table's traffic, so it is a wider arm and a separate decision.
   **Acted on 2026-08-29** (jdh8): that arm is
   [§N1p](#n1p--an-unlimited-values-double-landy_notrump_no_major-loss-stays-off-landy_major_jam-shipped-default-on-2026-08-30),
   and it is narrower than the flag feared — the gate is `len(♥, ..=3) &
   len(♠, ..=3)` on the two `3NT` rungs alone, so the stoppers, the transfers
   and the two-suited family keep their orderings and only the four-plus-major
   game hands move. **Measured 2026-08-30 and lost at both colours** — so the
   flag's original proposed default was right, for the reason it gave in
   reverse: re-gating `3NT` moved only this table's traffic, and that traffic
   was better off declaring. The flag is now **settled, not merely proposed**.
   **Re-measured 2026-09-01** as §N1-lia package D, on the seat package A
   repaired, and it stays settled — but the split says *why* more precisely
   than "better off declaring": on plain DD only the traffic with a **short**
   major was, and the whole plain-DD verdict sits inside the lead-model seam.
   The gate bundles that half with a `min major ≥ 3` half worth +3.1…+5.9
   IMPs/fired, and the narrowed gate is this flag's one live descendant.
3. **The `X (2NT)` leg is unauthored on purpose** — opener's `X (2M)` seat was
   too, until §N1m re-opened it; the rest of this item stands.
   After the strong advance the overcaller jumps to `4M` 54.3% of the time and
   to slam another 13.5%; nothing invitational survives. `park/landy-kk` also
   registers a total-pass node at `X (2♦) - (2NT)`; it is **not** salvaged here,
   because a total pass shadows the floor at a seat this plan never scoped.
   Proposed reversible default: **leave the two remaining seats to the
   floor.**
4. **`artificial_calls_are_alerted` cannot cover `LANDY_PENALTY`.**
   `unalerted_artificial` skips `Double`/`Redouble` rules reached through
   row-package fallbacks by design — the node key cannot witness which strain a
   suffix-guarded double doubles. Same hole covers `MULTI_PENALTY` one lane
   over. The guard here is instead an explicit `ReadingScope::Alerted` arm in
   `landy_doubler_rebid_alerts_publish_the_trump_length`, which fails if the
   alert is dropped. Proposed reversible default: **leave the invariant's
   exemption alone**, and keep per-call alerted-scope assertions for penalty
   doubles. §N1m's opener-seat double shares the slug and is guarded the same
   way, in the same test.
5. **`--filter-landy` admits only strictly balanced 1NT openers.**
   `is_1nt_opener` (`examples/bba-gen/main.rs`) requires no singleton/void and
   at most one doubleton, but our shipped opening is
   `NotrumpShape::Wide6322`, which also opens 5m(422) and 6m(322) — both
   two-doubleton shapes. Measured consequence in §N1m's probe: **zero**
   six-card-minor openers in 103,653 seat boards, and every five-card minor a
   5(332). Every §N1 verdict measured under this filter (and §N3's, via
   `--filter-preempt`'s identical gate) is therefore blind to the wide-shape
   slice. It does not invalidate an A/B — both arms share the filter and the
   headline is IMPs per *accepted* board, which the flag's own rustdoc already
   says — but it does mean a rung gated on a long minor cannot be evaluated
   here at all. Proposed reversible default: **leave the filter alone**
   (widening it changes the accepted set, so every arm under it would have to
   be re-generated and no old pair would stay comparable), and instead **state
   the blind spot wherever a shape-gated rung is priced**. A
   `--filter-landy-wide` sibling is the clean fix when a lane actually needs
   the slice.

### N1q — the two-level majors sorted by **strength**, not by shortness (`defense_2c_landy_strength_majors`, **SHIPPED DEFAULT-ON 2026-09-20** on the third arm; `_doubles` and the `landy_recue_signoff` rail on since 2026-09-21)

Knobs `competition.defense_2c_landy_strength_majors` and
`competition.defense_2c_landy_strength_doubles` (the first **on since
2026-09-20**, so `bba-gen --no-ns-landy-strength` turns it off and
`--ns-landy-strength-doubles` adds the second; `render-book
--ns-landy-responder off|strength-doubles`; `probe-call-reading` carries
both).  Code:
`landy_bba_responder`'s `strength` arm plus `landy_strength_*` in
[src/bidding/american/competition/lebensohl.rs](../../src/bidding/american/competition/lebensohl.rs).
Runner `scripts/ab-landy-strength.sh`.  **Read only under
`defense_2c_landy_bba`, and inert under `defense_2c_landy_lia`** — the two are
alternative re-cuts of the same two rungs.

#### The hole

N1j spends both two-level majors on *which major is the doubleton* and gates
both at `points(10..)`:

| N1j call | meaning | weight |
| --- | --- | --- |
| `2♥` / `2♠` | GF takeout, 4+♦ 4+♣, **exactly two** in the bid major | 178 / 177 |
| `3♥` / `3♠` | GF splinter, 4+♦ 4+♣, 0-1 in the bid major | 176 / 175 |

So a **weak** both-minor hand has no call, and an **invitational** four-four
hand has none either.  §N1q re-sorts the same two rungs by how much the hand
holds:

| §N1q call | meaning | weight |
| --- | --- | --- |
| `2♠` | 4+♦ 4+♣ (four-four allowed), **INV+** `points(8..)`, unlimited above | 177 |
| `2♥` | 4+♦ 4+♣ **five-four or better**, `points(..=7)` + the escape's `floors` | 141 |
| `3♥` / `3♠` | the N1j splinters, **re-weighted above `2♠`** | 179 / 178 |

Neither rung claims anything about the majors, so the shortness ask and the
`3M` cue over it are gone and both answer tables are new.  The splinters move
up for the same reason they do under §N1-lia: the strength rung's shape
contains theirs, so the weights have to arbitrate and a game-forcing short
major makes the more descriptive call.

#### Step 0 — the census that sized it

`ab-results/landy-lia3/base-none`, 4,608,000 boards, table A only (the
throwaway scanner is not kept).  `1NT (2♣)` reaches responder on **821,414
boards, 17.83%**; of those:

| responder's hand | boards | % of all boards | today's call |
| --- | --- | --- | --- |
| 5-4+ minors, 0-7 points | 81,765 | **1.77%** | `P` 59.6, `2♦` 21.5, `2NT` 10.8, `3♣` 8.2 |
| 4-4 minors, 0-7 points | 52,937 | 1.15% | `P` 100.0 |
| 4-4+ minors, 8+ points | 123,110 | **2.67%** | `X` 28.0, `3♥` 17.1, `3♠` 15.2, `P` 12.8, `2♥` 10.9, `2NT` 5.3, `3♣` 3.7, `2♠` 3.4, `3NT` 1.9, `2♦` 1.8 |

Both bands clear the 0.1% stop-and-ask threshold by an order of magnitude.
Two readings the A/B design turns on:

* the strong rung's **single biggest donor is the values double** (28.0% of the
  band).  That is §N1p's convicted mechanism in miniature — a candidate that
  stops us defending — and it is why this arm's PD column is *not* waved
  through as the auto-double artifact.
* the weak rung's biggest donor is the **pass** (59.6%), then the `2♦` escape
  (21.5%).  The six-card minors keep transferring: `2NT`@174 / `3♣`@173
  outrank `2♥`@141 by a mile.
* the **four-four weak band keeps passing** (all 52,937 of it).  That is
  deliberate: it has no second suit to run to, and the diamond half of it has
  the escape a level cheaper.

#### What is authored

Responder: `2♠`@177 `both_minors & (points(10..) | (points(8..) & short_major
& no_six))` — at 8-9 a singleton-or-void major and no six-card minor, ten-plus
unconstrained in the majors (`comp:landy-minors-inv`; **flip arm** — the first
build was the whole `points(8..)` band),
`2♥`@141 `both_minors & (len(♣,5..) | len(♦,5..)) & points(..=7) & floors`
(`comp:landy-minors-weak`), splinters at 179/178.  Everything else in the
table — `3NT`@180/@168, the transfers, the `4M` jam and its Texas, the `X`@145,
the `2♦` escape, `Pass`@0 — is N1j verbatim.

Opener over `2♥`: `3♣`@100 / `3♦`@99 / `3♣`@0.  **No notrump rung**, because a
`2NT` on stoppers is a game try opposite a possible five-count and that was
lia3's both-vul catastrophe one rung over.

Opener over `2♠`: `2NT`@150 on both stoppers, `3♣`@100 / `3♦`@99, `3♣`@0.
**No `3NT` accept** — the first build's `3NT`@160 (`landy_lia_accept`) is the
convicted rung (verdict below); reversible as one line gated `hcp(17..)`.
Responder over `2NT`: `3NT`@120 on ten-plus, `3♣`@100 / `3♦`@99 on **five**
cards at 8-9, `Pass`@0.

> **Correction to the design sketch.**  The sketch gave the `3♣` sign-off
> `len(♣, 4..)`.  Responder is four-four or better by the rung's own
> constraint, so a four-card gate fires on *every* hand in the band and the
> `Pass`@0 below it is dead registration — `2NT` could never be passed.  Both
> sign-offs are gated at five, cheapest first: leaving a made `2NT` at 23-26
> combined needs a ninth trump, not an eighth.

Responder over `2♠ - 3m -`: `landy_strength_pick_rebid`, lia's table with the
`5m`@100 gate raised from ten to **thirteen** (the first build reused
`landy_lia_pick_rebid` verbatim, and its ten-point `5m` was the `2♥ → 2♠`
cell's whole loss).  Sit rails (`Pass`@0) at `2♥ - 3m -` and
`2♠ - 2NT - 3m -`, per the lia2
phantom-suit lesson: unauthored, those seats are the floor's, and the floor
reads an artificial `2♠` as spades bid.

Tails: `2♥ (X)` / `2♠ (X)` re-register the answer verbatim with the systems-on
rebase; `2♥ (2♠)` gets opener's table and `2♥ (2♠) X -` responder's pick;
`2♠ (3♥)` / `2♠ (3♠)` get the compressed ladder.  **`2♥ (3♥)` and `2♥ (3♠)`
are deliberately the floor's** — lia2's forensic convicted the `Pass`@0
sell-outs on exactly that class and found the floor *right* there.

The second knob adds the two doubles: takeout `X`@120 (3+3+ minors, no spade
stopper) over their `(2♠)` raise of `2♥`, penalty `X`@140 (4+ of the raised
major, `comp:landy-penalty`, `.penalty()`) over their raise of `2♠`.  Both are
explicit rules, not a PDI trigger — nothing in this lane mechanises the
polarity ([pdi.md](../pdi.md)), and an unalerted double here reads as the takeout
of a suit nobody holds.

#### Four lessons from the parked lia lane that bind this design

1. **The weak band at `2♥` was lia3's both-vul catastrophe** (`- → 2♥` −42,572
   plain / −107,283 PD), routed through a `3NT`@160 accept.  Here the weak
   answers have no notrump rung at all and the shape floor is five-four.
   Falsifier 1; if it fires, the flip arm is a `!vulnerable()` gate on `2♥`.
2. **Phantom suits.**  The floor's vector carries a raw we-bid-this-strain bit
   and no alert column, so an artificial `2♠` reads as spades bid.  Every
   floored seat after `2♠` is exposed; the mitigation is the authored `Pass`@0
   rails above.  Falsifier 3.
3. **A weak rung's contested tails with `Pass`@0 catch-alls were lia2's
   sell-out class, and the floor was right there.**  Hence the split: the cheap
   `(2♠)` gets a table, the `3M` raises go to the floor.  This narrows the
   design sketch's "notrump@150 above the takeout X" — there is no notrump rung
   in the weak tail.  Reversible default: a `2NT`@150 `stopper_in(♠)` rung is
   one line if wanted.
4. **`Pattern::node` vs `Pattern::after`.**  A guarded fallback's all-−∞
   rejection does not reach the floor (`Trie::resolve_floored`'s single
   fall-through), so a node meant to fall through must be exact.  Every §N1q
   node is a real table with a finite catch-all, so nothing here relies on
   rejection.

#### Verdict — measured loss, falsifier 2 fired (2026-09-18)

`scripts/ab-landy-strength.sh`, `SEED_BASE=1789675210`, sha `dd2dd0ad`,
4,608,000 boards/arm/vul, all four isolation gates 0 foreign.  Both knobs stay
**off**.

| arm | cell | plain | PD | sd-lead plain / PD |
| --- | --- | --- | --- | --- |
| `str` vs base | NV | +0.0056 ±0.0007 | −0.0115 ±0.0009 | +0.0168 / +0.0026 |
| `str` vs base | both-vul | **−0.0029 ±0.0008** | −0.0192 ±0.0010 | +0.0101 / −0.0040 |
| `strx` vs `str` | NV | +0.0003 ±0.0001 | +0.0005 ±0.0001 | +0.0004 / +0.0006 |
| `strx` vs `str` | both-vul | +0.0001 ±0.0001 | +0.0001 ±0.0001 | +0.0003 / +0.0003 |

Plain DD was the pre-registered primary at both colours and `str` loses it
both-vul; the PD column, read straight as pre-registered, loses everywhere.
`strx` is a small win on its own base and is moot on a losing parent.

**Forensic** (`probe-divergence --imps` + `scripts/divergence-buckets.py` over
both `str` cells, plus per-cell major-length cuts):

* **Falsifier 1 did not fire.**  The weak `- → 2♥` cell wins plain at both
  colours (+6,049 both-vul / +15,286 NV; PD −15,443 / −7,981) and so does
  `2♦ → 2♥`.  The no-notrump weak answer table did its job.
* **Falsifier 2 fired, in §N1p's exact shape.**  `X → 2♠` is −16,865 plain
  both-vul (−0.0037/bd, more than the whole headline) and −1,605 NV, monotone
  in responder's *short* major: per fired both-vul, 2-3 −1.160, 1-4 −1.800,
  2-2 −0.763, 1-3 −0.495 (NV +0.492), against 1-2 +0.255, 0-4 +1.311, 0-5
  +2.873.  By opener's answer the whole loss is the **notrump rungs**:
  `2♠ - 3NT` −10,403 (−5.18/fired; the 3NT fails on 1,434 of 2,009 boards) and
  `2♠ - 2NT` −8,564 (−1.38/fired, half of it responder's pass of a failing
  2NT), while the minor picks read −0.20.  Eight or nine opposite a maximum is
  24–26 combined with both of their majors known, and that is not a game.
* **Two secondary cells, both continuation holes.**  `2♥ → 2♠` (N1j's GF
  takeout hand) −5,110 / −3,754, all of it `2♠ - 3m - 5m`: responder has no
  authored rebid over opener's minor pick ([`landy_strength_inv_rebid`] sits
  only over `2NT`) and the floor jumps to game at −3.2/fired.  `3♣ → 2♠` (the
  6+♦ transfer hand, majors 1-2 / 0-3) −4,712 / −5,406: `2♠`@177 shadows the
  transfer and opener's `3♣` pick strands a six-four in the four-four fit.
  `2NT → 2♠` (the 6+♣ hand) is a small plain win with a PD loss.

**Flip arm — built 2026-09-18, same knob** (the pre-registered repair is a
major term, not a retreat from the band): (a) `2♠` at 8-9 requires a
singleton-or-void major (`len(♥, ..=1) | len(♠, ..=1)`), returning 2-2/2-3 to
the values `X`, and both minors ≤5, returning the six-carders to their
transfers; ten-plus is unchanged (with four-four minors and no short major the
majors are exactly N1j's doubletons).  (b) Opener's `3NT`@160 accept over `2♠`
**removed**: a further cut of the `2♠ - 3NT` boards by responder's short
major showed the accept losing on every 8-9 donor, short major included
(`X → 2♠` short −2.15/fired both-vul, `2♦ → 2♠` −2.77, `- → 2♠` −0.31 plain /
−4.70 PD), so (a) does not rescue it; opener describes `2NT` and responder
raises on ten.  (c) Responder's rebid over the pick was already authored —
lia's table, whose `5m`@100 fires on **ten** — so the fix is the gate:
`landy_strength_pick_rebid` bids `5m` on thirteen.  Same runner into
`ab-results/landy-strength2`, fresh seed.  **Predictions**: the `X → 2♠` cell
shrinks to the short-major donors and reads non-negative; `3♣ → 2♠` and the
`2♠ - 3m - 5m` leg of `2♥ → 2♠` vanish; the `- → 2♥` and `- → 2♠` plain wins
stay.  Falsifier: if the residual `X → 2♠` (short major, 8-9) still loses
through `2♠ - 2NT`, the takeout at 8-9 is simply worse than defending and the
band retreats to ten-plus — which is N1j's takeout merged, and a knob not
worth keeping.

#### Run 2 verdict — the flip arm, a non-win; §N1q closes measured twice (2026-09-19)

`scripts/ab-landy-strength.sh` into `ab-results/landy-strength2`,
`SEED_BASE=1789716492`, sha `dd2dd0ad` plus the uncommitted flip build,
4,608,000 boards/arm/vul, all four isolation gates 0 foreign.  Both knobs stay
**off**.

| arm | cell | plain | PD | sd-lead plain / PD |
| --- | --- | --- | --- | --- |
| `str` vs base | NV | **+0.0067 ±0.0006** | −0.0031 ±0.0008 | +0.0109 / +0.0026 |
| `str` vs base | both-vul | +0.0006 ±0.0007 | −0.0076 ±0.0009 | +0.0050 / −0.0023 |
| `strx` vs `str` | NV | +0.0003 ±0.0001 | +0.0006 ±0.0001 | +0.0004 / +0.0007 |
| `strx` vs `str` | both-vul | +0.0002 ±0.0001 | +0.0002 ±0.0001 | +0.0003 / +0.0004 |

Non-vulnerable is the decision table's **win | loss** row — the plain win is
reaching contracts a competent doubler slaughters — and both-vul is wash |
loss; a default-off knob needs a win, and this is not one on either colour.
`strx` wins every cell on its own base and is moot on a non-winning parent.

**Forensic** (`probe-divergence --imps` + `scripts/divergence-buckets.py`,
plus the same band and major-shape cuts as run 1), against the predictions:

* **What the flip cured, as predicted.**  `3♣ → 2♠` is gone (zero boards —
  the six-carders are back on their transfers), `2♦ → 2♠` flipped from −865
  to **+1,024 / +2,175** plain, the weak rung's plain wins stayed (`- → 2♥`
  +5,359 / +14,375, `2♦ → 2♥` +6,442 / +1,053), and `- → 2♠` is the run's
  biggest cell at **+6,869 / +18,737** plain (PD +2,184 NV — the one cell PD
  likes).  `X → 2♠` halved (9,339 boards both-vul from 20,055) and reads
  **+410 plain / +6,148 PD non-vulnerable**.
* **The pre-registered falsifier fired, both-vul only — and what it loses to
  is §N1m.**  `X → 2♠` is −7,409 plain both-vul, all of it the boards where
  opener answers `2NT`@150 (−7,753 on 4,845 boards, whichever way responder
  continues: pass −1.81/fired, pull to `3♣` −1.62, to `3♦` −1.43), while the
  direct picks read +1.00 (`3♦`) / −0.68 (`3♣`).  The dump with scores (the
  `imps2-*.jsonl` re-solve, `probe-divergence` now writes `score_on/off`,
  `tricks_on/off` and the DD table) says why: on those boards OFF's values
  `X` is **converted to penalty** — §N1m's `landy_opener_px` doubles their
  advance (`X (2♠) X`) — and the OFF side defends `2♥x`/`2♠x` on 3,303 boards
  at −3.22 / −1.94 per fired for us (`2♥x` −2 on 527 boards at −9.1, −3 on 254
  at −12.3; `2♠x` −6 appears), plus a making `3NT` on ~1,000.  Opener's
  both-majors-stopped hand is exactly the hand whose stoppers are trump
  tricks on defense.  Even the DD-best of our three partscores loses to that
  mix vulnerable (−1,655 plain on the `2NT` boards) and wins it non-vulnerable
  (+4,437), where the penalties are smaller (−1.05 / −0.39 per fired).  So
  the pre-registered consequence stands for the vulnerable 8-9 band: the
  values double it displaced is now worth more than it was when §N1q was
  designed, because §N1m shipped in between.
* **`2♥ → 2♠` at ten-plus is an unauthored opener node, not fix (c).**  OFF
  reaches a *making* `3NT` (its `3NT` = / +1 / +2 are the −10-IMP rows); ON's
  responder raises `2NT` to `3NT` correctly and then **opener pulls it to
  `4♣`** at `2♠ - 2NT - 3NT -`, a node nothing authors: 519 of the 521
  both-vul boards in the `we 3NT → we 4♣` pair, 860 / 1,105 boards in all,
  −5,006 / −4,398 plain at −5.8 / −4.0 per fired.  With opener sitting those
  boards read **+171 / +332**.  The pass under the thirteen-point `5m` gate
  is the smaller half (−1,823 / −883 plain, PD +105 / +497); a `3NT` there
  would be +854 / +1,522 plain and −703 / −103 PD, and `5m` −4,574 / −5,085.
* **The weak rung's PD deficit is the floor's `3NT` at its unauthored tails,
  not the three-level partscore.**  The quiet `2♥ - 3m` contracts that stay
  our undoubled `3m` (9,802 / 7,527 boards) read **+13,525 / +12,535 plain
  and −3,300 / +1,822 PD** — PD-neutral, opposite a five-to-seven count.  What
  loses is 3,103 / 4,243 boards that end in *our* `3NT`: −10,758 / −5,198
  plain and **−27,292 / −28,263 PD**, more than the rung's whole deficit.  The
  tails: `2♥ (3♥) X - 3NT` (responder, 5-7 HCP, pulls opener's double to
  `3NT`: 1,125 / 1,123 boards), `2♥ - 3♣ - - (X) - - 3♦ - 3NT` (opener bids
  `3NT` over responder's run from the balancing double: 955 / 1,161),
  `2♥ - 3♣ (3♠) - - 3NT` (196 / 571), `2♥ (2♠) 3♣ (3♠) - - 3NT`, and
  `2♥ - 3♦ - - (X) - - XX - 3NT`.  With the `3NT` bidder sitting: −7,416 /
  −5,115 plain, −9,929 / −6,477 PD.  The same family at 8-9 on the strong
  rung — `2♠ (3♥) 3NT`, `2♠ (3♠) 3NT`, `2♠ - 3♣ - - (3♠) 3NT` — is 1,448 /
  2,754 boards at PD −7,499 / −9,349, and sat reads +2,785 / +4,102.
* **The sit counterfactual** (`scripts/divergence-sit.py $R/imps2-$v.jsonl $v
  --sit 3NT@0-9 --sit 4♣@10-13 --sit 4♦@10-13`, 5,429 / 8,205 boards sat):
  the whole arm reads **NV plain +0.0075 / PD +0.0055; both-vul plain
  +0.0029 / PD −0.0005** — win | win and win | wash — against +0.0067 /
  −0.0031 and +0.0006 / −0.0076 as played.  A sit assumes the opponents'
  calls before the phantom stay put, so it is an estimate of what authored
  `Pass` rails are worth, not a measurement.
* **Flagged, not fixed — a shared-node drift.**  At `2NT - 3♣ - 3M - 3NT -`,
  a node identical in both arms and authored under neither knob state,
  responder pulls to `4♣` in the ON arm only: `- → 4♣` 419 boards −2,795
  both-vul / 510 boards −2,781 NV (−6.7 / −5.5 per fired), and run 1 carried
  the same ~400 boards.  The pull is the floor reading the knob through its
  regime input ([card-manifold.md](../ai-bidder/card-manifold.md)), which makes
  it the base lane's hole, not this knob's.  Proposed reversible default: a
  `multi_signoff_pass()` at that node, its own A/B.

**Third arm — credible, not built (the user's call).**  The deep cut
(2026-09-19, the scored re-solve) reverses the first reading of run 2: the
convention's contracts are fine and the deficit is **three families of floor
phantoms at nodes the knob left unauthored**, which is lia2's finding one lane
over ("the floor *bids* at the unauthored nodes, so the repair is mostly
authored `Pass`").  The arm would be `Pass` rails: (1) opener sits at
`2♠ - 2NT - 3NT -` (a `multi_signoff_pass()`); (2) the weak rung's contested
and balancing tails — responder passes opener's double of their raise
(`2♥ (3M) X -`), opener passes responder's run over the balancing double
(`2♥ - 3m - - (X) - - 3x -`), no redouble at `2♥ - 3♦ - - (X) - -`; (3) no
`3NT` from responder at 8-9 over their raise of `2♠` (`2♠ (3M) -`), and none
over the pick's `(3♠)`.  Optionally `3NT` at 10-12 over the pick (+854 /
+1,522 plain, PD −703 / −103).  What stays structural: the vulnerable 8-9
`X → 2♠` against §N1m's penalty machinery (−7,409; the DD-best partscore still
−1,655), and the weak rung's level cost against the `2♦` escape (`we 2♦ → we
3♦` −1,574 / −2,656).  A colour gate on the 8-9 band (non-vulnerable only)
remains the fallback if the vulnerable cell survives the rails.  Until that
arm is measured, §N1q stands as **measured twice, opt-in, default
byte-identical**.

#### Third arm — built 2026-09-19, same knob; pre-registration

Per-tail cuts of run 2's `imps2-*.jsonl` before the build (both-vul / NV,
plain | PD) settled the two choices the sketch above left open, and corrected
one of its attributions:

* **`2♥ (3♥)`: opener sits — the double is the phantom, not just the pull.**
  `2♥ (3♥) X - 3NT` played −1,213 / −357 | −3,441 / −2,836.  Responder sitting
  the double (`3♥x`) is **−2,645 / −2,687** plain, responder picking its long
  minor at the four level −160 / +591 | −1,820 / −1,056, and opener *not
  doubling* **+245 / −35 | +332 / +12**.  So the rail is opener's `Pass` at
  `2♥ (3♥)` / `2♥ (3♠)`, which reverses the first builds' "floor-owned" split:
  lia2 convicted a sell-out against a floor that passed, and this floor
  doubles.  Under `_doubles` there is deliberately **no** takeout `X` above
  the rail at the three level — the double is what loses.
* **Family 3's `3NT` is the *authored* `3NT`@150**
  ([`landy_strength_inv_overcalled`]), not a floor bid, and the HCP band in
  the sit is responder's (a six-count with shortness is `points(8..)`), which
  opener cannot see.  By opener's HCP the rung loses PD at 15 and 16 and is
  level at 17 (`2♠ (3♥) 3NT`, both-vul: 15 −75 | −585 vs sat +133 | +182; 16
  −93 | −358 vs +37 | +90; 17 −25 | −121 vs −8 | −17).  Non-vulnerable it
  wins plain (+544 over 15-17) and loses PD (−1,355 vs +878 sat).  Build:
  `3NT`@150 needs `hcp(17..)`; `Pass`@145 on the stopper below it, so the
  stopper hand defends rather than falling through to a four-level minor.
* **The balancing double of `3♣`: responder's run stays the floor's.**
  `2♥ - 3♣ - - (X) - -` → `3♦` is 1,067 / 1,007 boards at −2,740 / −1,919 |
  −5,895 / −5,518; sitting `3♣x` instead is plain *worse* (−3,473 / −2,827)
  and PD better (−1,631 / −1,805) — mixed, so not railed.  What is railed is
  opener's call over the run (`… 3♦ -`: `3NT` −1,612 → −766, `4♣` −588 →
  −161 both-vul) and responder's `XX` of the double of `3♦` (−260 → +458).

Rails built (all `multi_signoff_pass()`, all inside `if strength`): opener at
`2♠ - 2NT - 3NT -`; opener at `2♥ (3M)`; opener at `2♥ - 3m (3M) - -`,
`2♥ - 3m - - (3M)`, `2♠ - 3m - - (3M)`; responder at `2♥ (2♠) 3m -` and
opener at `2♥ (2♠) 3m (3♠) - -`; opener at `2♥ - 3♣ - - (X) - - 3♦ -`;
responder at `2♥ - 3♦ - - (X) - -`.  Not built: the optional `3NT` at 10-12
over the pick (a different claim), opener's `4m` at `2♠ - 2NT - 3m (3M)`
(plain-mixed), and the colour gate.

`scripts/ab-landy-strength.sh` into `ab-results/landy-strength3`, fresh seed,
same sizing.  **Plain DD primary at both colours, PD read straight.**
Predictions: `str` vs base lands near the sit counterfactual (NV +0.0075 |
+0.0055, both-vul +0.0029 | −0.0005); the `we 3NT → we 4♣` pair vanishes; the
weak rung's our-`3NT` family shrinks to the balancing-run residue.
**Falsifiers**: (1) if the `2♥ (3M)` sits turn the weak rung PD-positive but
plain-negative, the sell-out was right after all and the opener rail reverts
to a responder-only rail; (2) if the vulnerable `X → 2♠` cell stays below
−5,000 plain, the colour gate on the 8-9 band is the next arm, not a tweak of
this one; (3) if the `hcp(17..)` gate costs NV plain more than it returns in
PD on the `2♠ (3M)` cells, the gate reverts and the rails stand alone.

#### Run 3 verdict — the third arm wins all four cells; the majors ship default-on (2026-09-20)

`ab-results/landy-strength3`, `SEED_BASE=1789833845`, sha `4647befa` plus the
uncommitted rails, 4,608,000 boards/arm/vul, all four isolation gates 0
foreign.

| arm | cell | plain | PD | sd-lead plain / PD |
| --- | --- | --- | --- | --- |
| `str` vs base | NV | **+0.0078 ±0.0006** | **+0.0070 ±0.0007** | +0.0096 / +0.0087 |
| `str` vs base | both-vul | **+0.0050 ±0.0007** | **+0.0026 ±0.0008** | +0.0075 / +0.0054 |
| `strx` vs `str` | NV | +0.0004 ±0.0001 | +0.0003 ±0.0001 | +0.0006 / +0.0005 |
| `strx` vs `str` | both-vul | +0.0000 ±0.0001 | −0.0001 ±0.0001 | +0.0003 / +0.0002 |

`str` is the decision table's **win | win** row at both colours, every cell
clear of its CI on all four scorers: `defense_2c_landy_strength_majors` ships
**default-on**.  It landed on the sit counterfactual's plain prediction
(+0.0075 / +0.0029 predicted) and beat its PD one (+0.0055 / −0.0005) — the
authored `Pass` also moves the opponents' later calls, which a sit cannot.

**Per tail, run 2 → run 3** (both-vul / NV, plain | PD; different seeds, same
sizing):

| node | run 2 | run 3 |
| --- | --- | --- |
| `2♠ - 2NT - 3NT -` (opener) | −3,049 \| −3,306 / −1,633 \| −2,369 | **+4,706 \| +3,956 / +6,020 \| +4,711** |
| `2♥ (3♥)` (opener) | −3,422 \| −9,780 / −930 \| −7,858 | +485 \| +1,587 / −151 \| +393 |
| `2♠ (3♥)` + `2♠ (3♠)` (opener) | +654 \| −3,753 / +4,402 \| −3,372 | +699 \| +2,432 / +148 \| +2,867 |
| `2♥ - 3♣ - - (X) - - 3♦ -` (opener) | −7,409 \| −16,335 / −5,058 \| −15,095 | −4,972 \| −12,041 / −2,912 \| −9,798 |
| `2♥ - 3♦ - - (X) - -` (responder) | +4,528 \| +3,768 / +4,320 \| +3,394 | +6,339 \| +6,333 / +5,483 \| +5,510 |

The sit counterfactual now moves nothing (374 / 1,002 boards sat, plain
+0.0050 → +0.0050 both-vul): the phantoms it was built to find are gone.

**The falsifiers.**

1. *Not fired.*  The weak rung is plain-positive at both colours (`- → 2♥`
   +12,223 / +15,838, `2♦ → 2♥` +12,408 / +3,848), and the `2♥ (3♥)` sit
   itself reads +485 | +1,587 both-vul.  Opener's rail stands.
2. **Fired.**  Vulnerable `X → 2♠` is **−8,563 plain** on 9,625 boards (PD
   −759; NV −1,887 | **+8,222**) — untouched by the rails, as the mechanism
   (§N1m's penalty conversion of OFF's values double) predicted.  It is now a
   losing cell inside a shipped win, so the pre-registered next arm — a
   `!vulnerable()` gate on the 8-9 half of `2♠`, ten-plus unchanged — measures
   against the new default.
3. *Not fired on the pair it names, fired on one cell.*  Over `2♠ (3M)` the
   seventeen-point gate cost NV plain 4,254 IMPs and returned 6,239 PD; but
   the split is by suit — `(3♥)` NV alone is plain +3,143 → −655 and PD −2,774
   → +123 (opener's pass there sells out to `3♥`, 1,698 boards at −2,744 |
   −2,117), while `(3♠)` is +1,259 → +803 | −598 → +2,744.  The gate stands;
   a `(3♥)`-only relaxation non-vulnerable is a sweep candidate, not owed.

**Residue, all of it now inside the default.**  Responder's run from the
balancing double (`2♥ - 3♣ - - (X) - - 3♦`, the table's fourth row) is still
the weak rung's largest hole and stays the floor's — sitting `3♣x` was
plain-worse.  The shared-node drift at `2NT - 3♣ - 3M - 3NT -` (`- → 4♣`,
−3,099 / −2,882 plain, 426 / 510 boards) is unchanged and is no longer an
opt-in arm's problem: the `multi_signoff_pass()` proposed above is worth its
own A/B.

**`strx` stays off.**  It adds doubles, so plain DD arbitrates
(measurement.md's domain addendum): a win non-vulnerable and a wash
vulnerable, down from a win in every cell on runs 1-2.  Its worst boards name
two tails the knob never authored, both reached only through its takeout `X`:
`2♥ (2♠) X (XX)` — responder passes and they play `2♠xx` (−21) — and opener's
seat at `2♥ (2♠) X - 3m (3♠)`, where the floor bids `4♣` into a double.  An
artificial call with an unauthored interfered tail is not complete; the knob
stays opt-in until both are railed and re-measured on the shipped parent.

#### Owed

* ~~The flip arm~~ — built and **measured 2026-09-18/19, a non-win**; verdict
  above.
* ~~The third arm~~ — built 2026-09-19, **measured 2026-09-20, a win in all
  four cells; `defense_2c_landy_strength_majors` shipped default-on**.
* **The colour gate** (falsifier 2): `!vulnerable()` on the 8-9 half of `2♠`,
  against the new default.  **Measured 2026-09-21 and found mis-built** (the
  gate cut on `points`, the `X` it falls back to on `hcp`); repaired the same
  day and **re-measured as `nv2`: a plain win, sd-lead agreeing;
  `defense_2c_landy_strength_nv_invite` shipped default-on** — run 5 verdict
  below.
* ~~`_doubles`' two tails~~ — built and **measured 2026-09-21;
  `defense_2c_landy_strength_doubles` shipped default-on**.
* ~~The shared-node rail~~ — built and **measured 2026-09-21 a win in all four
  cells; `landy_recue_signoff` shipped default-on**.
* **A `4m` rung above the rail** for the thirteen-plus six-card hand (run 4:
  −123 / −324 plain inside the rail's win) — it owes an authored answer
  ([minor-transfer-slam](../minor-transfer-slam.md)); ≈ 0.0001/board, low.
* ~~The A/B~~ — **run 2026-09-18, measured loss; verdict above.**
* ~~A reading gate before launch~~ — **run 2026-09-18, PASSED**.  Read at
  *opener's* seat (the four-call spelling `"1N (2C) 2S P"`; the three-call one
  answers about their overcall instead, which is the trap lia3's gate
  documented): `2♠` → `points 8.. ♣4.. ♦4..`, `2♥` → `points 5..7 ♣4..5 ♦4..5`,
  `3♥` → `points 10.. ♣4.. ♦4.. ♥..1`, `2NT` → `points 2.. ♣6..`.  The values
  `X` gains a **shape** half it never had — `points 8..9 ♣..5 ♦..5` — because
  the four-four-at-8+ hand now bids `2♠`; that falls out of `bid_exclusion`, so
  no new slug and no disclosure decision, exactly as §N1p's `points 8..9` did.

#### The residue arms — pre-registration (2026-09-21; run 4 verdict follows)

Three independent knobs, all against the same control (`main`), so one runner
with a shared `base` arm: `scripts/ab-landy-strength-residue.sh`, results
`ab-results/landy-strength4`, arms `nv` / `strx` / `rail`.  They touch
disjoint nodes and none is measured on top of another; each verdict is its own.

##### `nv` — the colour gate

Knob `competition.defense_2c_landy_strength_nv_invite` (default **on since
2026-09-21**, run 5; a modifier of the shipped majors); `bba-gen` /
`probe-call-reading` `--no-ns-landy-strength-nv-invite` (the ON flag
`--ns-landy-strength-nv-invite` through run 5).  On, the
8-9 half of the `2♠` rule carries `!vulnerable()`; ten-plus is unchanged at
both colours.  Unit test `landy_strength_nv_invite_gates_the_invitation` pins
the fallback: vulnerable, the 8-9 short-major hand is the values `X` again —
not `Pass`, not a transfer.

**Reading.**  Unchanged with the knob on: `2♠` → `points 8.. ♣4.. ♦4..` at
opener's seat.  `vulnerable()` projects nothing and the probe pins `NONE`, so
vulnerable the hull is *sound, not tight* (the truth is `10..`).  Accepted
as-is: a colour-keyed reading is a bidding knob under a neural floor
([reading-drift-handoff](../reading-drift-handoff.md)) and would confound this
arm; it is a follow-up only if the gate ships.

**Arbiter**: plain DD.  The knob *adds doubles* (the vulnerable 8-9 hand
returns to the values `X`), so PD is a blind column here (measurement.md,
domain addendum).

**Prediction.**  Both-vul: `X → 2♠` inverts into a `2♠ → X` cell of roughly
the 8-9 share of run 3's 9,625 boards, and the arm gains up to **+0.0019
plain/board**.  NV: **zero divergent boards** — the isolation gate is the
check, and any divergence there is a build bug, not a result.

**Falsifier.**  Run 3's vulnerable `X → 2♠` was −8,563 plain; the ten-plus
remainder is what this arm leaves in place.  If that remainder (run 3's cell
minus this arm's `2♠ → X` cell, same sign convention) is itself below −3,000
vulnerable, the problem is opener's `2NT`@150 answer, not the band — the next
look is a `2NT`-answer gate, not a wider colour gate.  And if `2♠ → X` reads
plain-negative outright, §N1m's penalty machine is not what run 2's re-solve
said it was: the gate stays off and the cell is closed as *leave it*.

##### `strx` — `_doubles` with its two tails authored

Inside `if landy_strength_doubles`: `2♥ (2♠) X (XX)` takes
`landy_strength_weak_pick` (responder names its five-card minor instead of
passing out `2♠xx`), and `2♥ (2♠) X - 3m (3♠)` takes the `Pass` rail (the floor
bid opener's `4♣` into a double).  Test
`landy_strength_doubles_tails_are_authored`.  **Arbiter plain** (it adds
doubles).  Run 3 read NV +0.0004 / both-vul +0.0000 plain.  **Ship rule**: a
plain win NV and a plain non-loss both-vul.  **Falsifier**: if the two cells
are repaired (no `2♠xx` pass-outs, no `4♣` at that seat in the worst boards)
and both-vul plain is still a wash, the takeout `X`@120 itself is worth
nothing vulnerable and the knob closes as opt-in, no further tails.

##### `rail` — the shared-node rail (`competition.landy_recue_signoff`)

`multi_signoff_pass()` at `2NT - 3♣ - 3M - 3NT -` **and its diamond twin**
`3♣ - 3♦ - 3M - 3NT -` (same table, same wiring loop; only the club node is
measured evidence — −3,099 / −2,882 plain on 426 / 510 boards — so a diamond
cell that reads negative is an over-broad trigger and gets cut).  Home is the
N1j transfer wiring beside `landy_recue_answer`, not the `if strength` block:
the node is unauthored under either knob state and the floor's pull to `4♣`
arrives through the regime input.  `bba-gen --ns-landy-recue-signoff`; test
`landy_recue_signoff_sits_the_game`.  **Flagged cost**: `3M`@150 outranks the
`4m`@130 slam try, so a thirteen-plus six-card hand with exactly one major
stopper cues first and now *passes* `3NT` — no slam continuation exists at that
seat either way (an unauthored `4m` reads as nothing,
[minor-transfer-slam](../minor-transfer-slam.md)), so the rail costs only what
the floor's `4♣` was winning there.  **Prediction**: recovers most of the ≈
+3,000 plain per colour, PD same sign.  **Falsifier**: if `4♣ → -` reads
plain-negative on the thirteen-plus six-card subset by more than the rest
gains, the seat wants a `4m` rung with an authored answer, not a rail.

#### Run 4 verdict (2026-09-21): `rail` and `strx` ship default-on; `nv` was mis-built and is re-armed

`ab-results/landy-strength4`, seed 1789929046, sha `a28d2784`, 4.608M
bd/arm/vul.  **Every isolation gate 0 foreign**; `nv`'s NV cell read **0
divergent boards**, the build check it was there for.  CIs below are exact,
recomputed from the solved divergent sets (`imps.*.jsonl`).

| arm | cell | fired | plain /board | PD /board | sd-lead plain / PD |
| --- | --- | --- | --- | --- | --- |
| `rail` | both | 1,006 | **+0.00116 ±0.00013** | +0.00134 ±0.00015 | +0.0010 / +0.0011 |
| `rail` | none | 1,189 | **+0.00095 ±0.00012** | +0.00117 ±0.00014 | +0.0008 / +0.0009 |
| `strx` | both | 1,184 | +0.00004 ±0.00010 | +0.00007 ±0.00012 | +0.0002 / +0.0002 |
| `strx` | none | 1,528 | **+0.00010 ±0.00010** | +0.00017 ±0.00011 | +0.0001 / +0.0002 |
| `nv` | both | 20,288 | −0.00027 ±0.00045 | −0.00014 ±0.00054 | −0.0019 / −0.0018 |

**`rail` — win | win at both colours, shipped.**  +5,338 / +4,360 plain, more
than the ≈ +3,000 predicted, because the floor's pull was not only `4♣`: the
club leg's `4♣ → -` is +4,448 / +3,934 (632 / 719 boards, +7.04 / +5.47 per
fired) and the pulls to `4♠` and `5♣` add +499 / +306.  **The diamond twin is
positive on its own** (≈ +360 / +140) and stays.  The flagged cost arrived
where it was flagged and is small: the thirteen-plus subset is −123 / −324 on 52 / 100 boards,
nearly all the floor's `6♦` (15 / 29 boards at −12.0 / −9.1 per fired) — the
falsifier ("loses more than the rest gains") does not fire, and the `4m` rung
is recorded under Owed.

**`strx` — plain win NV, plain non-loss both-vul, the pre-registered ship
rule; shipped.**  It adds doubles, so plain arbitrates and sd-lead tie-breaks:
sd is CI-clear positive in all four columns.  Both tails are repaired — **zero
`2♠xx` pass-outs** (the `(XX)` tail is 18 / 43 boards, −55 / −6), and opener's
sit at `2♥ (2♠) X - 3m (3♠)` is +310 / −15 plain, +865 / +827 PD on 385 / 723
boards.  The falsifier's premise (tails repaired, both-vul still a wash) is
met, but its conclusion is not: a CI-clear sd win and a positive point
estimate is a non-loss, not "worth nothing" — it ships on the rule, and the
both-vul cell is recorded as the weak one.  Remaining leak, too small to
author: `X - 3♣ - 3♦` / `- 3NT` (opener moving over the pick), −178 / −43.

**`nv` — a build defect, not a verdict.**  The cell the gate exists for did
what was predicted: `2♠ → X` is **+9,238 ±1,609 plain** on 9,550 boards
(+0.97/fired, +0.0020/board; PD +1,025, the blind column), the mirror of run
3's −8,563.  **Falsifier 2 is refuted** — §N1m's penalty machine is what run
2's re-solve said it was.  But `2♠` is `points(8..)` and the `X`@145 is
`hcp(8..)`: **9,529 seven-counts** (every one exactly 7 HCP, shape-upgraded)
lost `2♠` and had no `X` to fall to — `2♠ → Pass` **−9,234**, and 1,209
six-count 5-5s fell to `2♦` for −1,252.  The two cancel to the wash above.
An over-broad trigger, the iron rule's usual culprit.  **Repair**: the gated
half is `invite & (!vulnerable() | hcp(..=7))` — only a hand the `X` will
take is diverted; test `landy_strength_nv_invite_gates_the_invitation` pins
the seven-count.  Falsifier 1 (the ten-plus remainder) reads −8,563 + 9,238
≈ **+700**: not below −3,000, so no `2NT`-answer gate is indicated.

**The repaired arm, `nv2` — pre-registration.**  Same runner, fresh seed,
`ab-results/landy-strength5`, control = `main` with `rail` and `strx` on
(`base2`).  Arbiter plain.  **Prediction**: both-vul ≈ +0.0020 plain/board on
≈ 9,500 boards, NV zero divergent.  **Falsifier**: run 4's sd-lead read the
*bundled* arm at −0.0019, four times plain's deficit, and cannot be split by
cell after the fact.  If `nv2`'s sd-lead plain is CI-clear negative while
plain DD wins, the `X`'s gain is the clairvoyant lead against `2Mx`, and the
addendum's tie-break goes against the gate: it stays opt-in.

#### Run 5 verdict (2026-09-21): `nv2` ships default-on

`ab-results/landy-strength5`, seed 1789977169, sha `6f66ae69`, 4.608M
bd/arm/vul, control `base2` = `main` with `rail` and `strx` on.  Isolation
gate **0 foreign**; the NV cell read **0 divergent boards**, the build check.

| scorer | fired | plain /board | PD /board |
| --- | --- | --- | --- |
| DD | 9,623 | **+0.0018 ±0.0003** (+8,153, +0.85/fired) | +0.0000 ±0.0004 (+116) |
| sd-lead, 16 worlds | 10,151 | **+0.0013 ±0.0004** (+5,846) | −0.0001 ±0.0004 (−587) |

**A plain win on the arbiter, as predicted** (≈ +0.0020 on ≈ 9,500 boards;
run 4's isolated cell was +9,238 on 9,550).  The repair did what it was for:
every divergent board is *a different bid* (none passed where the baseline
bid), so the seven-count leak to `Pass` is closed.  PD is a wash on both
scorers — the blind column for a knob that adds doubles (4,016 boards doubled
in exactly one arm).  **The falsifier does not fire**: sd-lead plain is
CI-clear *positive*, so the `X`'s gain is not the clairvoyant lead against
`2Mx` — about seven tenths of it survives a blind lead.  Run 4's bundled
sd-lead −0.0019 is thereby attributed to the seven-count `Pass` leak, not the
`X` (by elimination; that cell was never scored alone).  (The two fired
counts differ by construction: `ab-dump-sd` counts auction-divergent boards,
`ab-dump-diff` contract-divergent ones; the 528 extra reach the same contract
by another route.)  Worst boards are the floor's contested tails after the
`X` (`X (2♠) X (3♠) 4♦ - 4♠`, our `3NT` over `X (2M) - - 3♣`) — the usual
unauthored-seat noise, no single cell large enough to author.

### N1r — idea queue for `1NT (2♣) 3♦`+ (**collected 2026-09-21; step 0 — `3NT` vs the values `X` by colour — SHIPPED DEFAULT-ON 2026-09-22 at favourable; step 0b — the misfit-only gate — MEASURED LOSS 2026-09-22, dropped; rows 1/2/5/9 settled 2026-09-23, rows 4/8 closed unbuilt 2026-09-24 — FINISHED**)

The shipped table above `3♣`: **`3♦` is idle** (no rule bids it, and the
`Pass`@0 catch-all means the floor never does either); `3♥`/`3♠` GF both-minor
splinters @179/178; `3NT`@180 (both stoppers) and @168 (ungated); Texas
`4♣`/`4♦` on 6+ major 10+; direct `4♥`/`4♠` NF slam try at **exactly** 15 HCP;
`4NT` and up unassigned.  Sources:
[landy-2c-counter-defense-research.md](../ai-bidder/landy-2c-counter-defense-research.md).
Ranked by frequency × DD-visibility, each with the census that kills it for
free:

| # | Idea | Why it might pay | Step 0 (before any authoring) |
| --- | --- | --- | --- |
| 1 | **Finish the splinter** (**census 2026-09-23**, **arm 1 shipped default-on 2026-09-23**, **arm 2 measured non-win 2026-09-23**, **tails shipped default-on 2026-09-23** — [tails verdict](#row-1-tails-verdict--their-action-over-the-passed-3nt-landy_splinter_tails-shipped-default-on-2026-09-23); [arm 2 verdict](#row-1-arm-2-verdict--openers-3nt-needs-a-double-stopper-landy_splinter_stopper-measured-non-win-2026-09-23-stays-opt-in-default-off); [arm 1 verdict](#row-1-arm-1-verdict--responders-rebids-over-openers-answer-landy_splinter_rebids-shipped-default-on-2026-09-23); census:  [verdict](#row-1-step-0-verdict--the-splinter-census-2026-09-23-wastage-is-the-wrong-axis-the-leak-is-responders-floor-4-over-openers-3nt): half (a) sign-reversed, half (b) dead, and a floor `4♠` leak worth more than both). Opener's answer is `3NT`@150 on *any* `stopper_in(M)` or no minor fit, else `4m`; above `4m` the floor owns a disturbed four level (no RKCB — [minor-transfer-slam.md](../minor-transfer-slam.md)'s survey rows). Two halves: (a) gate the `3NT` on **wastage** — a double stopper / two top honours, not `Axx` opposite a stiff with a five-card suit on lead; (b) author the fit path: opener's `4M` cue = fit, no wastage, maximum vs `4m` = ordinary; responder `5m` / `4NT` RKCB by a `points` floor, controls not HCP on the accept (ledger row P6) | The splinters were 17.1% + 15.2% of the 8+ both-minor band in the §N1q census ≈ **0.86% of all boards** — about 30× Texas's firing rate, and both halves are pure DD (game placement, slam) | From any existing arm dump: splinter boards, split by opener's holding in `M` (no stopper / single / wasted) × final contract × DD of `3NT` vs `5m` vs `6m` |
| 2 | **`3♦` = game values, major-stopper trouble** (**census 2026-09-23: DEAD at step 0** — [verdict](#row-2-step-0-verdict--the-3nt168-stopper-census-2026-09-23-dead-at-step-0-3nt-is-the-right-game-even-without-the-stopper): opener covers the missing major on 88%, and `5m` only ties `3NT` where opener is bare) (the Bessis sheet's artificial `3♦`; Kokish–Kraft and Cohen solve the same thing by making direct `3NT` *deny* full stoppers). Opener: `3NT` both stopped, `3♥`/`3♠` = that major only, `4m` neither. Takes hands out of `3NT`@168 only | `3NT`@168 is ungated opposite a known nine-plus majors on lead; fully DD-visible. **Not** §N1p's `nt` arm — that one rerouted to the `X` and stopped us declaring; this one keeps declaring | `3NT`@168 boards by responder's stopper count (0/1/2) × DD `3NT` vs best minor contract. If the 0-stopper cell still makes as often as the alternative, the idea dies here — honest prior: opener holds both stoppers most of the time and `5m` on 25 is no bargain |
| 3 | **Transfer splinters** (Merienne–Martel): `3♦` = short ♥, `3♥` = short ♠, `3♠` = 5-5 minors. Buys opener one step (`3M` = "doubtful stopper, your call") below `3NT` | Only if #1's census shows the `3NT`/`5m` decision is the leak and wastage alone cannot separate it. Competes with #2 for the `3♦` slot | #1's census |
| 4 | **Direct `4♥` when short in spades.** Texas's recorded residue: every worst board is `4♣ - 4♥ - - (4♠)` — the transfer gives the overcaller a second turn; the mirror never happens to `4♦`→♠. Gate: `len(♥, 6..) & len(♠, ..=1)` jams directly, reclaiming the near-dead exactly-15 `4♥` | Evidence is already measured (52 / 37 boards, 33 / 30 doubles, zero the other way); gain capped at roughly a fifth of Texas's +0.0002/bd | none owed — the Package C reports hold it |
| 5 | **Quantitative `4NT`** (**census 2026-09-23: DEAD at step 0** — [verdict](#row-5-step-0-verdict--the-quantitative-4nt-census-2026-09-23-dead-at-step-0-responder-16-never-bids-3nt): responder 16–17 on 200 / 52 boards in 4.6M) `hcp(16..=17)` balanced, authored answer. Both `3NT` rungs are uncapped, so a 17-count opposite 15-17 plays game | One rule + one answer; but responder 16+ over a Landy overcall is vanishing | count `3NT` boards with responder `hcp(16..)` |
| 6 | `4♠` = both minors, long (K–K weak; BBA plays it 5-5, 3-17 HCP, 0.445%) | Low: at 10+ a 5-5 is **always** a splinter (five cards left for two majors), so only the weak band is new, and a preempt opposite a strong notrump guards a game they rarely have — obstruction the harness cannot price anyway | — |
| 7 | `3♦` natural, INV six-card (Cohen) or GF one five-card major (*Jean Christophe*, opener relays) | Low: the natural `3♦` measured a wash on 26 bd in N1 and **negative at 6+** in lia; a 5-3 major fit into a known 4-1/5-0 break is the wrong game | — |
| 8 | The `4m` rung above the §N1q rail | already owed above, ≈ 0.0001/bd | — |
| 9 | **The values doubler's rebids over their `(2M)` runout under the favourable gate** (**SHIPPED DEFAULT-ON 2026-09-23** — [verdict](#row-9-verdict--the-values-doublers-rebids-over-their-2m-runout-landy_doubler_game-shipped-default-on-2026-09-23)) — `3NT` / natural three-level suit / penalty `X` for the 10+ hand, `3♣` sign-off or pass for the 8–9 hand. Today only the penalty `X` (4+ of their suit) is authored; the floor bids the weak hand's `3♣` with the game hand, and opener reads `X - 3♣` as 8–11 instead of 8–9 and jumps to `5♣` | Step 0b's tax, measured −0.0034 sd at `both` on the full gate; the shipped `ew` gate carries the same tax inside its +0.0042 / +0.0023 sd | none — the tax node census and probes are in step 0b; build, then A/B at `ew` only |

Recommended order: **row 1's census first** (it also decides row 3), then
row 2's.  Row 4 is the only one that needs no census.

**Flagged, 2026-09-23 — rows 4 and 8 sit below measurement.**  With rows 1,
2, 5 and 9 settled, what remains is capped under the A/B's resolution: row 4
at ≈ a fifth of Texas's +0.0002/bd (≈ +0.00004), row 8 at the rail run's
−123 / −324 plain on 52 / 100 boards (≈ 0.00007/bd), against DD CIs of
±0.0003 on 4.6M boards and the ≈ 0.0007 line row 2 died at.  Row 8 is not a
[minor-transfer-slam](../minor-transfer-slam.md) violation: the rail has the
thirteen-plus hand pass `3NT`, so no unauthored `4m` exists at that seat.
Row 4 also collides with the exactly-15 `4♥` slam try, whose answer node
would misread the short-spade jam.  **Closed 2026-09-24 (jdh8): both
unbuilt.**  Arm 2 stays opt-in on the same date (sd-lead arbitrates a
declare-less-`3NT` knob, as for D).  **§N1r is finished.**

**Discrepancy fixed 2026-09-24:** `render-book --their-2c-landy` used to
default `--ns-landy-responder off`, rendering the **N1j** table instead of the
shipped system.  The flag is now optional; absent (or `default`), the knobs
stay at `Agreements::default()`.

#### Step 0 verdict — `3NT` vs the values `X`, all four colours (**`landy_notrump_no_major_favourable` SHIPPED DEFAULT-ON 2026-09-22**)

Before spending the `3♦` slot on `3NT`'s stopper trouble (row 2), price the
alternative §N1p and §N1-lia D both left marginal: `landy_notrump_no_major`
(game hands with a four-card major double instead of declaring) at the two
**asymmetric** colours neither had run.  `scripts/ab-landy-nt-vs-x.sh`,
`SEED_BASE=1789977169`, control `ab8bc884`, 4,608,000 bd/arm/vul, table A
(we sit North/South), all four gates 0 foreign.  IMPs/board, `nt` − `base`:

| colour (ours / theirs) | fired | DD plain | DD PD | sd-lead plain | sd-lead PD | reads |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `ew` — favourable | 0.97% | **+0.0147** ±0.0008 | **+0.0127** ±0.0008 | **+0.0042** ±0.0008 | **+0.0023** ±0.0008 | win on all four |
| `ns` — unfavourable | 0.85% | −0.0107 ±0.0007 | −0.0144 ±0.0008 | −0.0206 ±0.0007 | −0.0235 ±0.0008 | loss on all four |
| `none` | 0.98% | +0.0007 ±0.0006 | −0.0009 ±0.0007 | −0.0108 ±0.0006 | −0.0123 ±0.0007 | DD wash, sd loss |
| `both` | 0.83% | +0.0052 ±0.0007 | +0.0004 ±0.0008 | −0.0054 ±0.0008 | −0.0092 ±0.0008 | D's shape again: DD win, sd loss |

* **The colour flip is real**, as the free re-price of §N1p's dumps predicted
  (ns −0.0318, ew +0.0077).  `3NT` is 400 and down two doubled is 500 at
  favourable; 600 against 300 at unfavourable.
* **The lead-model seam is a constant, not a verdict.**  Plain DD reads the
  knob +0.0105 / +0.0099 / +0.0115 / +0.0106 above plain sd-lead at ew / ns /
  none / both — D measured +0.013…+0.014 on the pre-§N1q table.  Only `ew`
  clears zero *after* paying the seam, which is why it alone ships; `both`'s
  plain-DD win is exactly the row D refused.
* PD sits **below** plain in every cell now — the §N1p auto-double artifact
  (PD above plain) is gone on the strength-sorted table.
* Mechanism is unchanged from §N1p: game reached in the baseline only on
  70.6% of `ew` divergence, 63.7% more room handed to the opponents, the
  worst boards all the floor's `4♦`/`5♣` over their `(2♠)`/`(3♠)` after our
  `X`.  At favourable the penalty pays for it.

**What shipped.**  `competition.landy_notrump_no_major_favourable` (default
**on**; `bba-gen --no-ns-landy-notrump-no-major-favourable`): both `3NT` rungs
are split by complementary `Rules::face` gates — the `no_major` rule live at
favourable, the ungated rule live otherwise.  **No new arm was needed**: a
seeded identity check (`bba-gen --count 3000 --seed 7 --filter-landy`) reads
the gated system byte-identical to `nt` at `ew` and to the off-switch at `ns` /
`none` / `both` on table A, so the `ew` row above *is* its A/B and the other
three cells are 0 divergent by construction.  `landy_notrump_no_major` (every
colour) stays off.

**Build trap worth the paragraph.**  The first spelling put the colour in the
constraint (`no_major | unfavourable`, then a single-atom `& !favourable()`).
Both bid correctly and both **drifted the values `X`'s exclusion reading at
every colour** — 6-7 table-A boards in 3,000 at `none`/`both`/`ns`, the
doubler then raising partner's `3♣` to `5♣` on an eight-count — because the
exclusion fold will not subtract a sibling whose constraint carries a context
atom.  A `face` gate is the rule-level switch the reader honours (a dead face
is skipped), at the price of two same-weight rules per rung, allowlisted in
`KNOWN_WEIGHT_TIES` with the partition argument.

**D's short-major split, re-sliced (answered by step 0b below):** at
favourable the `≤ 2` half is the *larger* part of the win, not the loser; at
both vulnerable both halves lose on sd-lead.  Nothing left to narrow by shape.

#### Step 0b verdict — the misfit-only `3NT` gate (**MEASURED LOSS 2026-09-22; knob `landy_notrump_no_major_misfit` and runner dropped, nothing shipped**)

The arm §N1-lia D's split suggested: whenever *they* are vulnerable, `3NT`
denies only the **misfit** — three-plus in both majors with four-plus in one
(`no_misfit = len(♥, ..=2) | len(♠, ..=2) | (len(♥, ..=3) & len(♠, ..=3))`) —
so a short-major game hand keeps declaring.  Live at `both` (new) and `ew`
(where it *narrows* the shipped gate).  A synthetic arm cut from step 0's
dumps had read it sd-lead +0.0046 / +0.0050 at both and +0.0068 / +0.0073 at
ew, ahead of the full gate's +0.0042 / +0.0023.  `scripts/ab-landy-nt-misfit.sh`,
same seed `1789977169`, control `cfa536b7`, 4,608,000 bd/arm/vul, both gates
0 foreign.  IMPs/board:

| cell | fired | DD plain | DD PD | sd-lead plain | sd-lead PD | reads |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `both`, `misfit` − base | 0.51% | +0.0002 ±0.0005 | −0.0065 ±0.0006 | −0.0017 ±0.0006 | −0.0069 ±0.0006 | wash, then loss on three |
| `ew`, `misfit` − `nt` (= `main`) | 0.51% | −0.0100 ±0.0006 | −0.0118 ±0.0006 | −0.0022 ±0.0006 | −0.0034 ±0.0006 | loss on all four |
| `ew`, `misfit` − base | 0.46% | +0.0047 ±0.0005 | +0.0008 ±0.0006 | +0.0022 ±0.0005 | −0.0008 ±0.0005 | beats base, dominated by the full gate |

**Why the synthetic was wrong, in two halves.**  A shape census of the
divergent boards (responder's major lengths, table A):

* **`ew`: 31,274 of 31,894 divergences are a *short-major* hand moving
  `X → 3NT`** — the half the synthetic handed to `3NT`.  Every worst board is
  `1NT (2♣) 3NT` failing where `1NT (2♣) X (2♥) - - X` collected: the mover
  holds a doubleton (or stiff) in one major and **five** of the other, they run
  into the five-card suit, and the reopening double is 500+.  The synthetic
  saw these boards correctly priced (base's `3NT`), but it was `nt`'s `X` on
  the misfit half it should have been comparing against, not base's.  So the
  short-major half is roughly +0.0100 of the full gate's +0.0147 at
  favourable; the misfit half is the marginal +0.0047.
* **`both`: 64% of the divergence (15,035 of 23,461 boards) has responder's
  `X` unchanged** — a weak values `X` in both arms — and opener's continuation
  moving (`3NT` or `5♣` at calls 8–10 where base passed, a second `X` at
  6–7).  That is the **reading tax**: `3NT`@168 losing `& no_misfit` widens the
  `X`'s exclusion reading to "weak *or* a misfit game hand", and opener can no
  longer tell them apart.  Two synthetic arms re-cut from the *real* dumps
  price the halves on plain DD / PD: the **direct** half (misfit hands
  `3NT → X`, everything else base) **+0.0090 / +0.0095** — 8,697 fired, +4.8
  IMPs per fired board, 74% of them ending in their `2Mx` or `2♣x` — and the
  **tax** half (unchanged `X`, opener's moved continuation) **−0.0089 /
  −0.0160**.  They add to the real arm exactly.  The earlier synthetic was
  the direct half alone.

**The mechanism is one thing at both colours.**  The direct half's best
boards at `both` are the same picture as `ew`'s worst: `632.AQJ65.Q8763.`
doubles, they run to `2♥`, doubled again.  What the values `X` sells is a
**four-plus major behind the runout**; whether the *other* major is short or
three cards is irrelevant.  The full `no_major` gate is already the right
shape axis, and §N1-lia D's `min major` split was noise from a smaller run.

**Both vulnerable is not closed — the tax is an unauthored tail.**  The
same split cut from step 0's *full-gate* dumps at `both` (plain DD / PD):

| synthetic half (rerouted `3NT → X`, everything else base) | fired | plain | PD | per fired |
| --- | ---: | ---: | ---: | ---: |
| direct, any 4+ major (the full gate) | 0.63% | +0.0104 | +0.0105 | +1.66 |
| direct, 4+ with the other major 2+ | 0.47% | +0.0106 | +0.0105 | +2.26 |
| direct, 4+ with the other major 3+ (the misfit) | 0.19% | +0.0090 | +0.0095 | +4.78 |
| tax (unchanged `X`, opener's moved continuation) | 0.32% | −0.0052 | −0.0101 | −2.56 |

So the misfit is the dense core, a doubleton other major adds +0.0016, and a
stiff other major adds nothing; the subset is not the lever, the tax is.  A
node census of the misfit run's 15,035 tax boards puts **61% at opener's
rebid after `1NT (2♣) X (2M) - - 3♣ -`** (`3NT` or `5♣` where base passed or
bid `3NT`), 9% after the doubler's `3♦` instead, and 16% at the doubler's own
rebid over the runout (`X` or `3♣` where base passed).  `probe-decision` on
the top node: opener reads partner `8..9` under base and **`8..11`** under
the gate, every listed call floor-owned (`Provenance { depth: 0, fallback:
Some(0) }`), and the M32 floor answers the wider envelope with `5♣` over a
`3NT` it rated 6.8 in base.  The widening is truthful: a 10–11 doubler
holding four of the *other* major has **no authored rebid over the runout**
(the only book rule at that node is the penalty `X` on `4+` of their suit),
so the floor bids `3♣` with it, the same `3♣` the 8–9 hand signs off in.
That is the incomplete-tails rule from measurement.md: the gate's game hand
was authored one call deep.  Authoring the doubler's rebids over `(2M)` —
`3NT` / a natural three-level suit / the penalty `X` for the game hand, `3♣`
sign-off or pass for the weak one — restores the 8–9 reading on `X - 3♣` and
takes the doubler's own rebid off the floor; three-quarters of the tax boards
pass through those two nodes.  The shipped favourable gate carries the same
tax inside its +0.0147, so the repair has headroom at `ew` too.

**What decides `both`: the sd-lead price of the direct half — and it is
negative.**  The synthetic `direct-all` arm (every 4+ major rerouted, no tax)
scores on sd-lead, 16 worlds, **−0.0020 ±0.0007 plain / −0.0020 ±0.0007 PD**
(`sd.full-direct-all.vs.base.both.txt`) against +0.0104 / +0.0105 on DD: the
whole ≈ 0.012 seam falls on the penalty collection.  We are on lead against
their `2Mx`, and the double-dummy lead is what makes 500+ beat 600; blind,
it does not.  So the tax at `both` is −0.0034 on sd (the arm's −0.0054 less
the direct −0.0020), and removing it entirely still leaves the reroute a
loss on the arbiter.  **`both` is closed by the direct half, not the tax**,
and no shape subset changes that — 4+/2+ is the full gate, 4+/3+ is its
dense core.

**What survives:** the tail repair itself.  The favourable gate's +0.0042 /
+0.0023 sd carries the same unauthored-rebid tax, so authoring the doubler's
rebids over `(2M)` is a real `ew` increment (order of +0.003 sd if the tax
scales), measured on its own — queued as row 9 below.

The knob `landy_notrump_no_major_misfit`, its `bba-gen` flag and
`scripts/ab-landy-nt-misfit.sh` were **dropped** rather than kept opt-in: the
arm is dominated by the full gate at favourable and closed by the direct half
at both, so no re-measure could revive it.  To rebuild, the gate was
`gated & no_misfit()` faced on `vul().contains(THEY)`, with the ungated rule
on the complementary face, at both `3NT` rungs (@180 and @168).
`_favourable` stands.

#### Row 1 step 0 verdict — the splinter census (**2026-09-23: wastage is the wrong axis; the leak is responder's floor `4♠` over opener's `3NT`**)

`examples/probe-landy-splinter-oracle` on step 0's `base-none` / `base-both`
dumps (seed 1789977169; the two colours deal the same boards, so one DD cache
serves both): every `1NT (2♣) 3♥/3♠` board — **39,405 at none (0.86%),
29,038 at both (0.63%)** — priced `3NT` / `4m` / `5m` / `6m` by opener in the
better minor against the live contract and par, cut by opener's holding in
the short major (`nost` no stopper / `stop1` a stopper with at most one of
A-K-Q / `wasted` a stopper with two-plus) × a four-card minor in opener's
hand × responder's HCP.  Reports in `ab-results/landy-splinter-oracle/`.
IMPs per seat board against live, plain / PD, none with both in brackets:

| bucket | boards | `3NT` | `5m` | `6m` |
| --- | ---: | ---: | ---: | ---: |
| `nost fit` | 3,711 (2,924) | −6.64 / −8.60 (−8.44 / −10.67) | +0.43 / +0.24 (+0.62 / +0.35) | +0.08 / −1.21 |
| `stop1 fit` | 15,828 (11,755) | +0.66 / +0.51 (+0.52 / +0.49) | **+1.17 / +1.45** (**+1.21 / +1.67**) | −0.21 / −0.74 |
| `stop1 nofit` | 6,163 (4,531) | **+1.19 / +1.27** (+1.13 / +1.22) | −1.58 / −1.89 | −3.40 / −4.92 |
| `wasted fit` | 8,117 (5,829) | **+1.21 / +1.26** (+1.04 / +1.15) | −0.53 / −0.52 (−0.92 / −0.90) | −2.81 / −3.87 |
| `wasted nofit` | 4,966 (3,527) | **+1.47 / +1.65** (+1.49 / +1.66) | −2.86 / −3.49 | −4.62 / −6.60 |

* **Half (a) as posed is dead — the sign is reversed.**  A *wasted* holding
  (two top honours in the short major) is where `3NT` is best; the one bucket
  where `5m` beats `3NT` is a **single stopper with a four-card minor**:
  +0.50 / +0.93 per seat board at none, +0.69 / +1.18 at both — ≈ +0.0017 /
  +0.0032 (none) and +0.0018 / +0.0030 (both) per board.  That is the
  surviving idea, as an **opener-side gate**: `3NT` needs a double stopper or
  no four-card minor; a single stopper with a minor answers `4m`.  Row 3
  (transfer splinters) buys the same information one step lower and is not
  needed unless this gate fails.
* **Half (b), the slam path, is dead by frequency.**  `6m` beats `3NT` only
  at responder 15+ (157 / 145 boards, +3.8 / +2.0) ≈ 0.0001 per board; at
  12-14 `6m` ≈ `3NT`.
* **The leak the census was not looking for: `1NT (2♣) 3♠ - 3NT - 4♠`.**
  Responder, having splintered in spades, rebids **`4♠`** over opener's
  `3NT` on 2,234 boards at none / 1,140 at both — every one the floor
  (`probe-decision` on `.Q6.A98765.KQ973`: every call `floor / no rule`,
  `4♠` 10.28 over `5♣` 10.25).  Opener holds 2-5 spades; we play `4♠` in a
  three- or four-card fit at −214 plain / −936 PD per board (none), −448 /
  −1,258 (both).  `3NT` on the same boards: **+23,285 / +33,019 IMPs at none
  = +0.0051 / +0.0072 per board; +15,205 / +19,635 at both = +0.0033 /
  +0.0043.**  Hearts are unaffected (the 35 `3♥ - 3NT - 4♠` are 5=0=4=4
  hands bidding a real suit).  It is lia3's phantom-suit class — the floor's
  vector carries the we-bid-this-strain bit and no alert column — at a node
  the book never authored: the splinter table authors opener's answer and its
  doubled twin, nothing for responder over `3NT`.  A `Pass`@0 at
  `P* 1NT (2♣) 3M - 3NT -` in total (every floor continuation there replaced
  by the pass): none +29,100 / +37,719 ≈ **+0.0063 / +0.0082**; both +16,631
  / +20,579 ≈ **+0.0036 / +0.0045**.  The floor's `5♣` pulls are net +4.7k /
  +3.2k for the pass, its `6♦` pulls net −1.9k / −1.8k against it (the
  floor's slam bids are roughly break-even), `5♦` ≈ 0.
* **A second unauthored tail:** responder **passes opener's `4m`** on 518
  boards at none, 8-13 HCP — a passed game force.  `3NT` is not the fix
  (−4.3 there); `5m` vs live on `nost fit` is +0.43.  Small; the `5m` raise
  is the game-force completion and belongs in the same package.

Order: **arm 1** = responder's rebid table over opener's `3NT` (`Pass`
catch-all, `5m` over `4m`), measured at none and both; **arm 2** = the
single-stopper `4m` gate at opener.  The oracle numbers are contract-level
upper bounds; the A/B arbitrates.


#### Row 1 arm 1 verdict — responder's rebids over opener's answer (**`landy_splinter_rebids` SHIPPED DEFAULT-ON 2026-09-23**)

The census's leak, authored: at `P* 1NT (2♣) 3M - 3NT -` responder passes
(`Pass`@0 — the game is reached), at `… 3M - 4m -` responder raises to `5m`
(`5m`@0, the game force's completion in an eight-plus fit); the `(X)` twins
of both nodes carry the same tables, everything else at the seat (their
overcalls over the splinter, the four level) stays the floor's.  No alert,
no reading, no card row: both calls are natural.  The alternative — a
`Trie::tombstone` on `4♠` leaving the floor the rest of the node — priced
lower on the census (the floor's `5♣` pulls were net negative, its `6♦`
pulls break-even), so the node is authored whole.

`scripts/ab-landy-splinter-rebids.sh`, `SEED_BASE=1789977169`, control
`d0f7222c`, 4,608,000 bd/arm/vul; the **base arms are step 0's dumps**
(every commit since `ab8bc884` is byte-identical to them at none and both,
so only the on arms were generated), gates 0 foreign.  IMPs/board, `rebids`
− `base`:

| colour | fired | DD plain | DD PD | sd-lead plain | sd-lead PD | reads |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `none` | 0.18% | **+0.0055** ±0.0003 | **+0.0078** ±0.0004 | **+0.0053** ±0.0003 | **+0.0072** ±0.0004 | win on all four; +3.03 / +4.29 DD per fired |
| `both` | 0.13% | **+0.0039** ±0.0003 | **+0.0047** ±0.0004 | **+0.0035** ±0.0003 | **+0.0042** ±0.0004 | win on all four; +3.04 / +3.67 DD per fired |

* **The oracle was right to the third decimal**: it priced the pass node at
  +0.0063 / +0.0082 (none) and +0.0036 / +0.0045 (both); the A/B, which also
  carries the `5m` raise and every downstream tail, reads +0.0055 / +0.0078
  and +0.0039 / +0.0047.
* **The pre-registered falsifier did not fire**: giving up the floor's `6♦`
  pulls cost nothing visible; the worst boards are all tails *after* the
  pass, where the opponents act and the floor takes over again.  Two of
  them, both floor-owned and both new exposure (the off arm's `4♦`/`4♠`
  pull pre-empted them): their **balancing `(4M)` over the passed `3NT`**
  (≈ 4,400 boards at none; the floor sits on 1,171 of the `(4♥)`s and bids
  `5m` on most of the rest, never doubles) and their **double of the passed
  `3NT`** (≈ 1,100 boards at both; the floor **redoubles** on 545 of them,
  the worst both-vul boards at −21).  `Pass`/`X` at `3M - 3NT - - (4M)` and
  `Pass` at `3M - 3NT - - (X) - -` are the next rows for this seat; sizes
  are board counts, the IMP split is unmeasured.
* Row 1's other survivor, the single-stopper `4m` gate at opener (arm 2),
  is measured on top of this.


#### Row 1 arm 2 verdict — opener's `3NT` needs a double stopper (**`landy_splinter_stopper` MEASURED NON-WIN 2026-09-23; stays opt-in, default off**)

The census's surviving gate: opener's `3NT` over the splinter requires
**two of A-K-Q** in the short major (`stopper_in & top_honors(2..)`) or no
four-card minor; a single stopper with a four-card minor answers `4m`, which
arm 1's responder raises to `5m`.  Only the splinter answers move (N1j's
two-level takeout answers keep the plain stopper).  `scripts/ab-landy-splinter-stopper.sh`,
`SEED_BASE=1789977169`, control `1abbb254`, base arms = arm 1's on-arm dumps
(byte-identical to `main`), 4,608,000 bd/arm/vul, gates 0 foreign.
IMPs/board, `stopper` − `base`:

| colour | fired | DD plain | DD PD | sd-lead plain | sd-lead PD | reads |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `none` | 0.32% | **+0.0020** ±0.0003 | **+0.0022** ±0.0004 | −0.0003 ±0.0003 | −0.0004 ±0.0003 | DD win, sd wash / loss |
| `both` | 0.25% | **+0.0019** ±0.0003 | **+0.0029** ±0.0004 | −0.0008 ±0.0003 | −0.0002 ±0.0004 | DD win, sd loss / wash |

* **The DD win is the census's number** (+0.0017 / +0.0032 predicted at
  none, +0.0018 / +0.0030 at both), and it is **erased on sd-lead** at both
  colours.  The mechanism is §N1-lia D's: the knob *declares `3NT` less*,
  and the clairvoyant-lead scorer is the one that likes that — against a
  single stopper the DD leader finds the right major and the right card
  every time, the blind leader does not, and the ≈ +0.0025 DD − sd seam on
  0.3% of boards is ≈ +0.75 IMPs per fired board of lead luck.  The decision
  table's `win | win` row is written for a knob whose mechanism DD prices
  honestly; this lane has twice ruled (D, step 0's `both`) that for a
  declare-less-`3NT` knob the sd-lead bracket arbitrates.  It reads a wash
  to a slight loss, so **the knob stays opt-in** with the default
  byte-identical — finished code, measured, not parked.
* **Judgment call, flagged**: on the decision table alone this ships.  It is
  one default line (`landy_splinter_stopper: true`) if the sd-lead seam is
  judged unrealistic here — the Landy overcaller on lead holds five-four in
  the majors and leads one in practice, which is the DD leader's choice more
  often than the blind model's.
* Refinement if reopened: the census class is A-K-Q count, so a bare `A`,
  `Kx`, `Qxx` and `Jxxx` share the `stop1` bucket; the pre-registered
  falsifier is a cut by exact stopper type, priced on the same dumps.

#### Row 1 tails verdict — their action over the passed `3NT` (**`landy_splinter_tails` SHIPPED DEFAULT-ON 2026-09-23**)

Arm 1's pass hands the opponents' pass-out seat two floor-owned tails at
`P* 1NT (2♣) 3M - 3NT - - …`.  **Census** (`examples/probe-landy-splinter-tails`,
on arm 1's on-arm dumps; reports `ab-results/landy-splinter-tails/census-*.txt`),
IMPs per seat board over live, plain / PD:

* **`(X)`** (641 boards at none, 1,085 at both): the floor sat on 253 / 340
  and **redoubled** on 116 / 279, then walked into `4♥`@us on the pull
  (−1,000 to −1,400 PD).  Sitting in `3NTx` is +0.96 / +2.01 (none),
  +1.90 / +2.85 (both).  Responder running to its own longer minor beats
  sitting in both columns only on a **void** in the splintered major
  (+2.9 / +1.5 none, +2.1 / +0.2 both); with a singleton it wins plain and
  loses PD (−1.1 / −1.8), so the singleton sits.
* **`(4M)`** (≈ 4,750 boards at none, ~10 at both): doubling is +2.5 to
  +3.5 plain in every bucket (their suit = the splintered major or the
  other, every HCP band); the floor sat on 1,164 `(4♥)`s and bid `5m`
  elsewhere, never doubling.  PD prefers the undoubled `4♥` over the heart
  splinter by 0.5 — the doubled-making cases — but the change *adds*
  doubles, so plain DD arbitrates and PD is the double-blind column.

**Authored**: over `(X)` opener passes, responder runs to `5♦` (diamonds
strictly longer) or `5♣` on a void and passes otherwise, and opener passes the
run (`-` and `(X)` twins); over `(4♥)`/`(4♠)` opener doubles (a `.penalty()`
catch-all), responder passes it, and responder doubles `(4M)` when the
overcaller pulls the `(X)` (`… (X) - (4M)`).  No alerts: every call is natural.

`scripts/ab-landy-splinter-tails.sh`, `SEED_BASE=1789977169`, control
`5c3ee25c` + the knob, base arms = arm 1's on-arm dumps, 4,608,000
bd/arm/vul, gates 0 foreign.  IMPs/board, `tails` − `base` (round 2):

| colour | fired | DD plain | DD PD | sd-lead plain | sd-lead PD | reads |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `none` | 0.11% | **+0.0030** ±0.0003 | **+0.0024** ±0.0003 | **+0.0009** ±0.0002 | **+0.0003** ±0.0002 | win on all four; +2.75 / +2.20 DD per fired |
| `both` | 0.02% | **+0.0006** ±0.0001 | **+0.0007** ±0.0001 | **+0.0003** ±0.0001 | **+0.0004** ±0.0001 | win on all four; +3.33 / +4.09 DD per fired |

* **Round 1** (the same without opener's pass over the run;
  `ab-results/landy-splinter-tails`) read the same to the fourth decimal on
  DD (+0.0030 / +0.0024, +0.0006 / +0.0007), but its worst boards showed
  the floor at opener's seat jumping to six of the other minor over
  responder's `5m` run on 33 of 162 (none) and 47 of 228 (both) runs.  The
  pass was authored and round 2 re-measured; it moved sd-lead at both from
  +0.0002 / +0.0003 to +0.0003 / +0.0004 and nothing else visibly.
* **Worst boards**: our double of their `(4M)` when `6♦` makes (the floor
  used to bid it after `5♣`), −17 each.  A slam branch over the sacrifice
  would need responder's shape and opener's fit at once; not authored.
* **The seat is done** except for arm 2's flagged judgment call and the
  direct `3M - 3NT (4♥)` tail (29 boards at none, left to the floor).

#### Row 9 verdict — the values doubler's rebids over their `(2M)` runout (**`landy_doubler_game` SHIPPED DEFAULT-ON 2026-09-23**)

Step 0b's tax, repaired at the one colour that carries it.  At favourable the
values `X` holds the 10+ hand with a four-card major that `3NT` now denies;
over `1NT (2♣) X (2M) - -` (and the two `X (2♦) - (2M)` legs) the penalty `X`
takes it when the major is theirs, and otherwise it had no authored call.

**The build finding: authoring the game hand alone does not fix the
reading.**  With `3NT`@150 on `points(10..)` in place, `probe-decision` still
read the floor's `X – 3♣` as `8..37`: sibling exclusion narrows only
*authored* calls, so a floor call at a node that rejected the hand keeps
the prior envelope.  The 8–9 hand's five-card `3♣`@100 / `3♦`@99 had to be
authored too — then `X – 3♣` reads `8..9` with at most three of their major.
The general gap (a floor call at a rejecting exact node could read as the
prior minus every live authored rule there) is a reader change in every lane,
so it is left as its own follow-up, not folded in here.

**Authored**, all behind `face(favourable)` and only under the shipped `px`
ladder: `3NT`@150 on `points(10..)` (no stopper gate, like the direct
`3NT`@168 it was diverted from), `3♣`/`3♦` on five cards; opener passes the
`3NT` and answers the minor with `landy_minor_rebid_answer` (`3NT` on 16+
with their major stopped, else pass), on exact nodes so the dead faces fall
through to the floor.  A seeded identity check (`bba-gen --count 3000 --seed 7
--filter-landy`) reads none / both and `ns` table A byte-identical; only
favourable seats move.

`scripts/ab-landy-doubler-game.sh`, `SEED_BASE=1790142988`, control
`9d6a51ae` + the knob, 4,608,000 bd/arm, gate 0 foreign.  IMPs/board,
`game` − `base`:

| colour | fired | DD plain | DD PD | sd-lead plain | sd-lead PD | reads |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `ew` | 0.25% | **+0.0085** ±0.0003 | **+0.0117** ±0.0004 | **+0.0078** ±0.0004 | **+0.0103** ±0.0004 | win on all four; +3.45 / +4.76 DD per fired |

* **No lead seam this time**: DD and sd-lead agree within 0.0015, unlike
  step 0 (≈ 0.011).  The arm mostly *stops* overbidding rather than trading a
  game for a penalty.
* **Where it diverges** (11,444 boards, by first differing call): opener's
  `5m` over `X – 3m` becomes a pass on 33.5% (`2♠` 20.3%, `2♥` 8.2%, `3♦`
  5.0%); the doubler's `3NT` replaces the floor's second `X` on 21.0% and
  the floor's `3♣`/`3♦` on 14.1%; the authored `3♦` replaces a pass or a
  second `X` on 11.4% and the floor's `3♣` on 4.5%; opener's `4♣` over their
  balancing `(3M)` replaces `5♣` on 6.4%.
* **Pre-registered falsifier did not fire in aggregate**: the floor's second
  `X` on the short-trump game hand is 21% of the divergence and the arm still
  wins every column.  **Worst boards** are its tail — `X (2♠) - - 3NT (4♠)`
  where the off arm collected from `X (3♠) X`, −15 each — and their balancing
  `(3M)` over our `3♣` (opener's floor `4♣` where `5♣` pushed them to a
  doubled `5M`).  Both are floor tails over their four-level action; not
  authored.

#### Row 2 step 0 verdict — the `3NT`@168 stopper census (**2026-09-23: dead at step 0; `3NT` is the right game even without the stopper**)

`examples/probe-landy-notrump-stopper` on step 0's `base-none` / `base-both`
dumps (seed 1789977169, one DD cache for both): every `1NT (2♣) 3NT` board
where responder does **not** stop both majors — the ungated `3NT`@168, since
at these colours the @180 head takes every both-stopped hand and the six-card
minors transfer above it — **52,763 at none (1.15%), 36,838 at both (0.80%)**,
against 22,093 / 14,643 for the head.  Advancer passes 97.9% / 99.9%; the
live contract is opener's `3NT` on 51,671 of the 52,763 at none (notrump is
opener's: opener bid it first, so the `3♦` route cannot right-side it
either).  Priced: `3NT` and `5m` in the partnership's best minor against the
live contract and par, and `scheme` — the `3♦` route itself (opener `3NT`
with both majors stopped, `3M` with that one only and responder's `3NT` if it
stops the other, else `5m`).  Cut by responder's stoppers × opener's cover of
the unstopped major(s), and by the minor fit.  Reports in
`ab-results/landy-notrump-stopper/`.  IMPs per seat board vs live, plain /
PD, none with both in brackets:

| bucket | boards | `3NT` | `5m` (opener) |
| --- | ---: | ---: | ---: |
| one stopper, opener covers the other | 40,771 (28,074) | −0.00 / −0.02 (+0.00 / +0.00) | −5.55 / −6.75 (−6.94 / −8.33) |
| no stopper, opener covers both | 5,904 (4,148) | −0.01 / −0.03 (+0.00 / +0.00) | −5.76 / −7.26 (−7.19 / −8.97) |
| no stopper, opener covers one | 1,513 (1,150) | −0.07 / −0.15 (0.00 / 0.00) | −1.70 / −2.31 (−1.94 / −2.47) |
| one stopper, opener lacks the other (`bare`) | 4,573 (3,465) | −0.12 / −0.26 (+0.00 / −0.00) | −0.07 / −0.53 (+0.02 / −0.27) |

* **The honest prior held.**  Opener covers the missing major on 88% of the
  @168 boards, where the `3♦` route bids the same `3NT` by the same hand.
  On the 9% where opener is *bare*, `5m` merely ties `3NT` (their suit is
  split and runs short of five tricks often enough), and with neither major
  stopped by responder and one by opener `5m` loses 1.7-1.9 IMPs.
* **The whole `3♦` scheme loses**: −0.0006 / −0.0011 IMPs/board (none),
  −0.0006 / −0.0010 (both) against the live `3NT` — before pricing what a
  `3♦` hands them (a lead-directing double, room at the three level).
* **Its only positive cell is not reachable.**  With one stopper and an
  eight-card minor fit, the bare boards' `5m` gains — scheme minus `3NT`
  +0.0007 / +0.0008 IMPs/board at none, +0.0006 / +0.0008 at both — but the
  fit is opener's length plus responder's, which neither hand knows after
  `3♦`, and the other bare cells give it back.  An oracle upper bound of
  ≈ 0.0007 before the `3♦`'s own costs is below this lane's build line.
* `3NT` sits ≈ 2.1 / 2.7 IMPs below par per @168 board, but not in any
  minor game: `5m` is −5 IMPs everywhere it is not bare.  The gap is par's
  own (their sacrifices, slams on 25-30 combined), not this row's.

**Row 2 is closed; the `3♦` slot stays idle.**  Row 3 (transfer splinters)
was already not needed after row 1; with row 2 dead nothing competes for
`3♦`.  Remaining rows: 4 (direct `4♥` short in spades, no census owed), 5
(quantitative `4NT`, census owed), 8.

#### Row 5 step 0 verdict — the quantitative `4NT` census (**2026-09-23: dead at step 0; responder 16+ never bids `3NT`**)

`examples/probe-landy-notrump-stopper --quant` on the same `base-none` /
`base-both` dumps, with the @180 head kept (one DD cache,
`ab-results/landy-quant-nt/`): every direct `1NT (2♣) 3NT`, **74,856 at none
(1.62%), 51,481 at both (1.12%)**, cut by responder's HCP × opener's.
Priced against the live contract: `quant` (responder `4NT`, opener `6NT` on
17, else passes `4NT`) and a blind `6NT` by opener.  IMPs per seat board vs
live, plain / PD, none with both in brackets:

| responder HCP | boards | `quant` | blind `6NT` |
| --- | ---: | ---: | ---: |
| ≤ 13 | 72,094 (50,235) | −4.49 / −5.92 (−5.59 / −7.26) | −8.37 / −11.89 |
| 14 | 1,927 (951) | −0.82 / −1.00 (−1.10 / −1.35) | −5.55 / −6.25 |
| 15 | 635 (243) | −0.10 / −0.19 (−0.51 / −0.69) | −2.09 / −2.41 |
| 16 | 182 (51) | −0.24 / −0.30 (−0.12 / −0.28) | +0.68 / +0.48 (±1.6) |
| 17 | 18 (1) | 0.00 / 0.00 | +7.33 / +7.33 (±3.9) |
| 18+ | 0 (0) | — | — |

* **The row's own prior held.**  Responder holds 16–17 on **200 boards in
  4.6M at none (0.004%) and 52 at both** — the rest of a 16+ hand doubles,
  splinters or transfers first, and nothing 18+ bids `3NT` at all.
* **The authored shape loses where it lives**: `quant` is −0.00001 IMPs/board
  on the 16-count, the opener-17 accept almost never co-occurring with it
  (24 / 5 boards at 16 + 16, none at 16 + 17).  Even a blind `6NT` on every
  16–17 `3NT` is ≈ +0.00006 / +0.00001 per board — two orders under the
  ≈ 0.0007 line row 2 died at, and under any A/B's resolution.

**Row 5 is closed.**  Remaining rows: 4 and 8, both capped below the A/B's
resolution (see the queue note above the step 0 verdict).

