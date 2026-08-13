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
| **N1** | **Landy `(2♣)` counter** | `their.two_clubs_landy` | **SHIPPED 2026-08-14** (PD +0.0032/+0.0043); was the top loser, both scorers |
| N2 | Muiderberg `(2♠)` calibration | — | −0.66/bd, PD −0.58 NV |
| N3 | `(3+)` overcalls of our 1NT | new | floor-only today; −0.63/bd |
| N4 | Multi `(2♦)` — finish + measure | `defense_2d_multi` | exists, half-built, gate bug above; plain-only loss; **owed the `their` disclosure migration N1 got** |
| N5 | Complete Jacoby, re-measure | `competition_over_transfer` | default-off on a measured loss *while missing its two most-fired cells* — a half-built loss, resumable |
| N6 | `(2NT)` penalty discipline | `uvu_encircle` et al. | worst per-board rate, n=118 — needs boards before it needs code |
| N7 | Absent responses contested | new | Puppet `3♣`, `3♦`, splinters, `3NT`, Texas, `4NT` — rarest in the system |

## N1 — the Landy `(2♣)` counter (**SHIPPED 2026-08-14** via the disclosure channel `their.two_clubs_landy`)

**Not a knob.** What their `2♣` means is a fact about the opponents, so the
engagement bit lives in `Agreements::their` — the disclosure channel — never
in our own knob space (`competition.defense_2c_landy` existed for one day and
was deleted; `defense_2d_multi` is owed the same migration). Undeclared
defaults to natural (systems-on rebase; self-play and unknown fields), and a
harness that knows its opponent derives the declaration: `bba-gen`'s
`their_2c_landy` plays explicit `--their-card`/`--their-conv` Landy-family
rows at **face value** (a declared no-Landy set reads natural — deviations
behind a declaration are *their* infraction) and, with no declaration,
defaults the 2/1 reference to Landy from its **measured behavior**, because
its own card lies: `21GF.bbsa` declares `Cappelletti=1, Landy=0,
Multi-Landy=0` while the engine bids Multi-Landy regardless (551-board
census). `bba-decompose` applies the same correction when replaying BBA dumps
(derived from the dump's opponent label; `--landy-counter false` for
pre-ship dumps), keeping the replay contract exact — and the re-homing is
proven inert: shard-0 of the measured on-arm regenerates board-for-board
under `--their-2c-landy true` at the shipped sha.

Responder's table at `1NT (2♣)`, replacing the systems-on rebase
(`landy_responder`, `competition/lebensohl.rs`):

| Call | Meaning | Weight |
| --- | --- | --- |
| `3♣` / `3♦` | natural forcing, **6+** suit, `points(10..)` | 175 |
| `3NT` | game values, `points(10..)`, **no stopper gate** | 170 |
| `X` | **values**, `hcp(8..)`, penalty-oriented — alert `comp:landy-values` | 145 |
| `2♦` | weak natural, 5+, `points(..=9)` + the `natural_floor` | 140 |
| `2NT` | natural invite, `points(8..=9)` | 130 |
| Pass | finite catch-all | 0 |

Opener's answers, one per responder call (`landy_natural_answers`, authored
2026-08-14 after the first A/B traced the loss to their absence — the floor
has no net input slot for the knob, so it completed each call as the
default-system gadget it replaced):

| After | Opener answers | Was (floor, audited in `ab-results/landy-counter`) |
| --- | --- | --- |
| `X -` | Pass, sitting for the values double | phantom stolen-Stayman answer |
| `2♦ -` | Pass, always — a minor sign-off is never raised (`lebensohl_signoff_raise` doctrine) | phantom Jacoby `2♥` **82%** (742/902, `3♥`×149 super-accepts) |
| `2NT -` | `3NT` at `hcp(size_ask_accept_floor..)` (default 16, the uncontested invite's own knob), else Pass | phantom minor transfers **23%** (`3♦`×27/`3♣`×19/`3♥3♠`×17 in 277) |
| `3♣`/`3♦ -` | `3NT` with both majors stopped, else raise (the raise is the finite catch-all) | `3♣`: phantom Puppet `3♦` **85%** (272/320); `3♦`: passed the force 62% |
| `3NT -` | *no node* — audited clean (530/530 passed out) | — |

Everything deeper is the floor's, which is where we want it — the floor
already encircles their escape from our double
(`penalize_escape_stack`/`penalize_escape_values`, both default-on).

**Three design points, two of them found by the smoke run rather than by
reasoning:**

1. **Either/or with the rebase, not an overlay.** Leaving the systems-on rebase
   registered would strip their `2♣` and remap our values `X` back onto the
   stolen `2♣` Stayman one round later, routing `1NT (2♣) X (2♥)` into the
   contested-Stayman package. That is the same defect as the `defense_2d_multi`
   gate bug above, so the knob switches the whole node.
2. **`3NT` takes no stopper gate.** The first draft used `author_direct_3nt`,
   which keys its stopper *and* its trap-pass test on the overcall's suit — but
   their `2♣` is artificial and promises no clubs, so a club stopper is not the
   question and a club honour-stack is no reason to trap. Demanding a *major*
   stopper is no use either: they hold both. Leaving `3NT` at plain game values
   also keeps it where systems-on had it, so the A/B tests the double rather
   than 3NT discipline.
3. **`X` is the residual, floored on `hcp` not `points`.** It ranks below every
   hand with an offensive direction (the 6-card minors, `3NT`) and above those
   without (`2♦`, `2NT`). Defending does not care about distribution, and a
   `points` floor would drag the shapely weak hands that belong in `2♦` into a
   double.

The 3-level minors are ranked *above* `3NT` on purpose: with both majors against
us, whether `3NT` is playable turns on opener's major holdings, which only opener
can see, so showing a six-card source of tricks and letting opener choose beats
guessing. Opener's reply to that is the floor's, and reads correctly because the
bid is natural.

**Smoke, 4,000 `--filter-1nt` boards paired (seed 424242, vul none):** 46 of
8,000 tables diverge (0.57%). Responder's call over their `2♣` moves
`2♠` **11 → 0** — the two-way minor transfer fired at a both-majors overcall is
gone — with those hands redistributing to Pass (+11) and `3NT` (+9). Sample
divergence, the census mechanism reversing:

```text
on : 1NT 2♣ X - - -                 (we double and defend)
off: 1NT 2♣ X - 2♦ 2♠ - - -         (opener answers stolen Stayman; they find 2♠)
```

**First verdict (2026-08-14, 76.8k bd/arm/vul, SEED_BASE 1786642613): LOSS on
all six cells** — two named leaks: opener's unauthored answers (the floor
phantom-completes the counter's natural calls as the gadgets they replaced),
and the census misread (systems-on's minor transfers were *winning* the
minor-partial boards). The follow-up audit widened leak 1 to every non-`X`
call (the table above), and `landy_natural_answers` closed it.

**Re-measure verdict (2026-08-14, 76.8k bd/arm/vul, SEED_BASE 1786644715,
sha 40a0946, `ab-results/landy-counter-v2`): plain wash + PD CI-clear WIN in
both vuls — SHIPPED.** NV plain +0.0005 ±0.0022 / PD **+0.0032 ±0.0028**
(+1.10/fired), vul plain +0.0013 ±0.0026 / PD **+0.0043 ±0.0032**
(+1.65/fired); 0.26–0.30% fired; SD-PD agrees (+0.0017/+0.0030, plain sd
wash). The unauthored continuations were the entire first loss and then
some. Shipped as the derived declaration above, not an engine default — the
engine's undeclared read stays natural, which the first A/B's self-play
argument demands (our own tables' `2♣` overcalls *are* natural).

**Inertness proven**: `smoke-default --count 20000 --seed 1` SHA-256
`8ea2f5678a733cfe3ead79411d9cb31b8e95d37de52236e597fc38f9dec82bbb`, identical at
HEAD and with the change. The default system is byte-identical.

**Disclosure**: BBA's `.bbsa` schema has rows for whether *we* play Landy /
Multi-Landy against *their* 1NT, but none for our counter to *their* Landy over
*our* 1NT. Nothing to wire in `card.rs`; the golden cards and
`alert-sites.txt` are unchanged (the counter rides the disclosure channel,
which is undeclared in the default system, so the alert site is not in it).

## N1b — the GF minor cues (`defense_2c_landy_cues`, measured 2026-08-14, stays opt-in)

The overlay that makes N1's remaining free space earn its keep, and the
falsifiable half of the **`1♣ (2♣)` analogy** (the theory that a counter to a
both-majors overcall of a balanced-ish partner should carry the
Unusual-vs-Unusual cue skeleton — see the verdict below). Replaces N1's
3-level minor structure; a knob of *ours* (which counter structure we play),
riding the `their.two_clubs_landy` declaration:

| Call | Meaning | Weight |
| --- | --- | --- |
| `2♥` (cue) | GF, **5+ clubs** — alert `comp:landy-cue` | 173 |
| `2♠` (cue) | GF, **5+ diamonds** — alert `comp:landy-cue` | 172 |
| `3♣` / `3♦` | natural **weak** escape, 6+ suit, `points(2..=9)` — replaces N1's forcing 175 | 110 |

As first authored (measured 2026-08-14) the overlay was a pure addition below
N1's forcing naturals, so six-carders bypassed the cue and weak minor hands
still had no call — a half transplant. The user's review fixed it: with a GF
cue below it a forcing 3m is redundant, so the cues carry *every* GF minor
one-suiter (six-carders included, showing extras later) and the direct
`3♣`/`3♦` flip to the weak escape, exactly the `michaels_cue_responder` twin
(`two_suiters.rs`). The weak `3♦` is mostly shadowed by the cheaper `2♦`
(140 > 110), surviving only below the 2♦ escape's hcp floor; a GF hand with
6♦5♣ cues `2♥` (173 > 172) and shows the shorter minor — rare, left alone.
The cues sit above the ungated `3NT` (170), so the game hands start low and
let opener place the contract. Opener's one answer (`landy_cue_answer`):
`2NT` with both majors stopped, else raise the named minor (the raise is the
finite catch-all — opener is balanced, so it never lands on fewer than two);
opener **passes** the weak `3♣`/`3♦` (`landy_signoff_answer`, replacing the
base arm's forcing-minor answer). Everything deeper is the floor's.

**Documentary basis (web survey 2026-08-14).** No published source draws the
`1♣ (2♣)` analogy outright — the UvU literature scopes itself to suit
openings — but the strongest expert counter-Landy structures independently
reproduce its two load-bearing components: `X` = values / "can double at
least one of their suits" (Cohen, Walker — N1's authored `X`), and cues = the
two unshown suits, exactly this scheme (Cohen's advanced structure: `2♥` = GF
clubs, `2♠` = GF diamonds). Where the record diverges is where the captaincy
disanalogy predicts: over a wide-range nebulous `1♣` the UvU cues grade
*raises* and the double stays informational (WJ2005 Sputnik), while opposite
a narrow 15-17 the raise half is re-spent on the `2NT`/`3NT` ladder and the
penalty axis is promoted. So the theory holds as *isomorphic, not identical*,
and the published cue meanings are four-way incompatible (minors / stoppers /
natural / other-major Stayman) — which is what the third A/B arm decides.

**First verdict (2026-08-14, pure cue addition, same run as N1's loss): WASH
in all six cells** (PD leaned positive, sd negative, every CI ⊇ 0), read next
to a floor that phantom-answered the cue's sibling calls — unpriceable.

**Re-measure verdict (2026-08-14, full skeleton, same run as N1's win,
cues↔on): WASH in all six cells, now leaning uniformly negative — stays
opt-in.** NV plain −0.0005 ±0.0013 / PD −0.0006 ±0.0018, vul plain −0.0007
±0.0016 / PD −0.0012 ±0.0021, sd −0.0004/−0.0012; −0.43…−1.25/fired on
76–88 divergent boards (0.10–0.11% fired). Measured on a sane base this
time — opener answers every sibling call by book — so the reading stands:
the `1♣ (2♣)` analogy's delta is ≈0 with a negative tilt; the base counter's
`X`-and-naturals core carries all the value, and the cue overlay adds
nothing the 15-17 captaincy hasn't already re-spent. The minor-opening side
of the same skeleton is P7 in [competitive-book.md](competitive-book.md)
(`set_uvu_over_minors`) — authored for coherence, unmeasurable vs the anchor
(BBA never cues over a minor; def1-c/d probes 2026-08-14).

**Deferred candidate N1c**: a Lebensohl `2NT` relay (weak sign-offs at `3♣`/
`3♦`). The completed N1b skeleton covers most of its ground — the cues arm
now has direct weak escapes — so the relay's residual value is the *base*
arm's weak minor hands (which still pass) and rescuing the invite band; it
would move the natural invite, so it is its own package and its own A/B,
only worth boards if the census still shows weak-minor passes leaking after
N1/N1b ship or die.

**Inertness**: `smoke-default --count 20000 --seed 1` SHA-256 `8ea2f567…`
identical at HEAD with both knobs off — the N1 reference hash, unchanged.

## Measurement discipline

- **Counter-defense isolation gate:** on identical seeded deals, configuring
  the candidate against a natural defense must leave the auction dump
  byte-identical to the natural baseline.  Require natural interference to
  occur and the targeted artificial-defense arm to diverge, so the check is
  not vacuous; a face-call-wide reinterpretation (`2♣` always means Landy)
  fails this gate.
- **Enriched probing** is the default here: `bba-gen --filter-1nt` (raw-hand
  gate, balanced 15-17 somewhere, applied *before* any bidding). Headline is
  IMPs per **accepted** deal; publish `per-board = conditional mean × trigger
  density` alongside and scale the CI the same way. Compare IMPs/divergent, not
  IMPs/board.
- One knob = one measured change; arms **sequential**, fresh
  `SEED_BASE=$(date +%s)` per experiment shared across its arms, never rebuild
  in flight.
- Any **reading** change is a second, separate A/B on the same enriched boards,
  so a loss attributes to calls or to reading, never to their sum.
- Ship rule: standard gate. Plain-DD wash + PD gain ships default-on; a CI-clear
  plain loss stays opt-in with the default byte-identical, and the leak gets
  named in the ledger.

## Ledger

| Package | Knob | Status | Verdict (plain / PD, IMPs) |
| --- | --- | --- | --- |
| census tool | — | **shipped** | read-only; picked N1 over the pre-census guess |
| N1 Landy `(2♣)` counter | `their.two_clubs_landy` (disclosure, not a knob; `defense_2c_landy` deleted) | **SHIPPED 2026-08-14** — engine undeclared=natural; `bba-gen` derives the declaration (2/1 reference → Landy, its card lies) and `bba-decompose` replays it; re-homing proven board-identical on the measured arm | **v2 (with `landy_natural_answers`, full audit fix)** on↔off: NV plain +0.0005 ±0.0022 / PD **+0.0032 ±0.0028** (+1.10/fired) / sd −0.0006 ±0.0025, SD-PD +0.0017 ±0.0030; vul plain +0.0013 ±0.0026 / PD **+0.0043 ±0.0032** (+1.65/fired) / sd +0.0001 ±0.0029, SD-PD +0.0030 ±0.0034. 0.26–0.30% fired, 76.8k bd/arm/vul, SEED_BASE 1786644715, sha 40a0946 — plain wash + PD CI-clear both vuls = ship. **v1 (sha 8bc465a, SEED_BASE 1786642613): LOSS all six cells** (NV plain −0.0050 ±0.0024, vul −0.0049 ±0.0028, every CI<0) — leak 1: opener's answers unauthored, audit: phantom Jacoby `2♥` 82% over `2♦`, phantom minor-transfers 23% over `2NT`, phantom Puppet `3♦` 85% over `3♣`, passed force 62% over `3♦` (only `3NT` clean); leak 2: the census misread (systems-on's minor transfers were winning the minor-partial boards). |
| N1b GF minor cues | `defense_2c_landy_cues` | **measured 2026-08-14 ×2 — WASH both times, stays opt-in**; v2 = the *full* UvU skeleton (cues carry all GF one-suiters, direct 3m weak) on the fixed base — the analogy's delta reads ≈0 with a negative tilt | **v2 (full skeleton, N1-win run)** cues↔on: NV plain −0.0005 ±0.0013 / PD −0.0006 ±0.0018, vul plain −0.0007 ±0.0016 / PD −0.0012 ±0.0021, sd −0.0004/−0.0012 (−0.43…−1.25/fired, 0.10–0.11% fired, all CIs ⊇ 0, every cell leaning negative). **v1 (pure cue addition, N1-loss run):** NV plain −0.0001 / PD +0.0004, vul plain +0.0001 / PD +0.0005, sd negative — unpriceable next to phantom sibling answers. |
