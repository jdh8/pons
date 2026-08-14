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

**Confirmation at 3× n (2026-08-14, 230.4k bd/arm/vul, SEED_BASE 1786653231,
`ab-results/landy-counter-v4`).** The ship verdict was re-run because the v3a
run's `landy-on`↔`landy-off` pair did not reproduce it at a fresh seed, and
pooling the two 76.8k runs left only vul PD alive. At 3× boards the shipped
verdict **replicates and strengthens in both vulnerabilities**:

| | plain DD | PD | plain SD | SD-PD |
| --- | --- | --- | --- | --- |
| NV | −0.0002 ±0.0013 | **+0.0032 ±0.0017** (+1.02/fired) | −0.0012 ±0.0014 | **+0.0019 ±0.0017** |
| vul | +0.0003 ±0.0015 | **+0.0028 ±0.0019** (+1.09/fired) | −0.0010 ±0.0016 | +0.0015 ±0.0019 |

Plain wash in all four plain cells, PD CI-clear in three of four (sd-PD vul
straddles by 0.0004) — the decision table's `wash | win` row, at 3× the
evidence and a fresh seed. The pooled-76.8k worry that NV was never there is
retired: NV is the *stronger* vulnerability here. 0.26–0.32% fired.

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

### N1b post-mortem (`probe-divergence`, 2026-08-14) — what the wash was made of

The wash was not one effect. [`examples/probe-divergence`](../examples/probe-divergence/main.rs)
reclassified the measured arms off disk — no generation, no new solve beyond
the 164 divergent boards — and reproduces the published headline exactly
(plain −0.0005/−0.0007, PD −0.0006/−0.0012), then splits it:

| Population | n (NV+vul) | plain | PD | per fired |
| --- | --- | --- | --- | --- |
| **cues** `2♥`/`2♠` (GF minors) | 51 | −90 | −97 | **−1.76 / −1.90** |
| **weak escapes** `3♣`/`3♦` | 46 | +25 | −73 | +0.54 / −1.59 |
| **they opened** — the mirror-read leak | 67 | −24 | +28 | −0.36 / +0.42 |

Three findings, none of them what this section previously reasoned to.

1. **The cues lose; the escapes do not.** −1.76 plain / −1.90 PD per fired
   board, the same sign in both vulnerabilities and under both scorers (CI
   ±1.87 / ±2.25 at n = 51 — consistent, not yet CI-clear). The weak escapes
   are plain-**positive** NV (+1.13/fired) and PD-negative *only vulnerable*
   (−3.26/fired): the signature of going for a number, not of a bad call. And
   `3♦` fired 9–11 times, so `2♦` does **not** shadow the weak diamonds the way
   this doc assumed — the escape band is a live question, the cue is not.
2. **The cue's loss is a missed slam, not a missed game.** Opener's cue answer
   was `2NT`/`3m` — `landy_minor_answer` shifted one level down to match the
   cheaper question. `2NT` collapses into 3NT and the auction dies there; the
   base arm's `4m` leaves a *suit* contract the floor cue-bids over. Three of
   the five worst boards are that swap:

   ```text
   base: 1NT 2♣ 3♦ - 3NT - 4♦ - 4♠ - 4NT - 5♥ - 6♦ - - -   → 6♦
   cues: 1NT 2♣ 2♥ - 2NT - 3NT - - -                        → 3NT   [−12 IMPs]
   ```

   A sub-game answer under a game-forcing call hands the auction to a floor
   whose `Inferences` carry no forcing channel at all (`read.rs:84-118`).
   **Fixed 2026-08-14**: the cue takes `landy_minor_answer` unchanged, so both
   arms answer a game-forcing minor at game level. The game-buckets show no
   systematic missed *game* — that hypothesis was wrong and the boards say so.
3. **38% of the divergent boards are not our 1NT at all** — see the next
   section. This is why the first two rows above are quoted over the
   we-opened-1NT subset only.

### The mirror-read leak — a counter knob changes how we read *their* auctions

`read.rs` gates its 1NT sites on parity **relative to the opener, not to us**
([read.rs:386-389](../src/bidding/inference/read.rs)), and `their_profile`
falls back to *our own* profile whenever no foreign book is declared
([read.rs:333-335](../src/bidding/inference/read.rs)) — which is every arm in
this campaign. So when **they** open 1NT and **we** overcall `2♣` (our own
Landy), their next call is read through *our* `1NT (2♣)` counter table: with
the cues on, their natural `2♥` reads as our game-forcing club cue.

Measured: 33/88 NV and 34/76 vul of N1b's divergent boards (38%), every one
with a `2♥`/`2♠` before the divergence — and **47/227 (21%) of the shipped
N1's**, so this is not specific to the cues. The IMP impact is roughly neutral
(N1b: −24 plain / +28 PD pooled), so no shipped verdict flips; but neither A/B
isolated what its ledger row claims — part of each was a reading change on the
*defensive* side of the same boards.

This is the failure the counter-defense isolation gate exists to catch, one
level up from where the gate looks: the gate checks that a *natural* defense
stays byte-identical, not that our counter stays out of auctions the opponents
opened. Under the house rule that a knob picks what we bid, not how we read
their bidding, the symmetric-opponent fallback quietly makes every counter
knob a reading knob as well.

**Now enforced**: `probe-divergence --gate-opener ours` exits non-zero unless
every divergent board was opened by our side (`theirs` for a defensive
package, whose ownership is the mirror image). It was pure discipline before —
documented as a gate, implemented nowhere, and both Landy A/Bs shipped
verdicts through it. Run it on every arm pair in this lane.

### N1b-v4 — the INV+ cue with stopper asks (measured 2026-08-14, **two winners and two losers**)

v3a (the game-level cue answer) halved the damage but did not flip the sign:
NV plain −0.0004 ±0.0016 / PD −0.0006 ±0.0019, vul −0.0006 ±0.0017 /
−0.0007 ±0.0020, sd −0.0003/−0.0004, **−0.34…−0.62 per fired** against v2's
−0.43…−1.25 (76.8k bd/arm/vul, SEED_BASE 1786650492). The residue was the
blind `4m` catch-all — opener placing a minor game with no idea whether the
majors were stopped.

v4 replaces the whole structure with an **invitational-or-better** cue and a
strength-carrying ask, per the user's design:

| Responder, over their `2♣` | | Weight |
| --- | --- | --- |
| `3NT` | game values, **both majors stopped, no six-card minor** | **180** |
| `2♥` / `2♠` | **INV+**, 5+ clubs / 5+ diamonds | 173 / 172 |
| `3NT` | game values, ungated — the base arm's rule, untouched | 170 |

The gated `3NT` outranks the cues because **opener declares any notrump
contract** (opener bid 1NT — Law 54), so responder's direct `3NT` costs no
siding. Denying a six-card minor is the 5-vs-6 split: a six-carder is a source
of tricks with slam play, so it always cues and lets opener place the contract.

Opener's answer carries strength **by level** — cheap is minimum, the 3-level
is maximum:

| Opener | Shows |
| --- | --- |
| `3NT` (160) | both majors stopped, maximum |
| `3♥` / `3♠` (155) | maximum, asks for a stopper in the major **opener lacks**, promises 3+ in the minor |
| `4m` (150) | maximum, neither major stopped |
| `2NT` (145) | both majors stopped, minimum |
| `2♠` (140) | minimum ask — **club cue only**, the one rung below the 3-level |
| `3m` (100/20) | minimum, no stopper shown (the finite catch-all) |

Responder answers an ask by showing the stopper (cheaply on a minimum, so
opener still judges game) or retreating to the minor opener's tolerance made
safe. Over opener's minimum `3m` — the only rung showing *no* stopper —
responder may **re-cue** `3♥`/`3♠` with a game force and a stopper worry;
opener bids `3NT` holding it, else takes the minor. Over `2NT` there is
nothing left to ask, so responder passes or bids `3NT`.

Every rung is authored down to the placing call, because `Inferences` has no
forcing channel: a rung left to the floor reads as bare "5+ ♣, 8+ points" with
no notion of an invitation, which is exactly what cost −1.8 IMPs/fired.

**Measured** (SEED_BASE 1786653231, 230.4k bd/arm/vul — 3× v2/v3a — sha
`8873e9c`+dirty, 0.15–0.17% fired). v4 is the first arm of this package to put
a **CI-clear positive** on the board, and the first with no negative cell:

| | plain DD | PD | plain SD | SD-PD |
| --- | --- | --- | --- | --- |
| NV | **+0.0016 ±0.0010** | +0.0001 ±0.0013 | **+0.0018 ±0.0011** | +0.0006 ±0.0013 |
| vul | **+0.0014 ±0.0012** | −0.0000 ±0.0014 | **+0.0024 ±0.0013** | **+0.0016 ±0.0015** |

Against v2's six negative cells and v3a's four, that is a real move. It is
still **not shippable**: the DD pair lands on the decision table's
`win | wash` row (doubling artifact), and the SD pair splits — vul retains 67%
of its plain-SD win CI-clear (*real effect*), NV retains 33% and straddles zero
(*the win was the missing doubling*).

`probe-divergence` says the package is **not one effect** but four, and the
split replicates across both vulnerabilities (IMPs per fired, we-opened boards
only, NV / vul):

| our call | plain | PD | n | reading |
| --- | --- | --- | --- | --- |
| `3♣` weak clubs | **+3.41 / +2.98** | **+0.71 / +0.61** | 56 / 54 | the package's engine — S5's predicted gap, confirmed |
| `2♠` diamond cue | **+1.87 / +1.93** | +0.48 / +0.12 | 114 / 82 | winner both scorers |
| `2♥` club cue | −0.17 / +0.06 | **−0.76 / −1.04** | 102 / 69 | loser, and **9 boards carry all of it** |
| `3♦` weak diamonds | −1.07 / +0.55 | **−1.71 / −0.55** | 28 / 29 | S5 called it — `2♦` already covers weak diamonds |

The doubling artifact is **not** in the doubled boards: the *no double swing*
bucket carries +1.38 / +1.81 plain against −0.17 / −0.27 PD. PD's synthetic
doubles of our own failing undoubled contracts are what erase the win, i.e. we
bid contracts that go down.

**The `2♥` defect is nine boards, and it is an unauthored continuation.** All
nine have us **declaring hearts** — the opponents' suit — after the cue, in two
modes:

- `1NT (2♣) 2♥ - 3♥ (X)` **passed out** — every rung in the registration block
  ends in `-`, so RHO's double drops us out of book and the floor passes the
  ask. We play `3♥` doubled, −16/−14 plain.
- The floor **bidding `4♥` itself** after the escape is doubled
  (`2♥ - 3♥ - 4♣ (X) 4♥`) or over their own hearts
  (`2♥ - 2NT (3♥) - (3♠) 4♥`). A phantom-suit disaster on a non-book
  continuation; v4 does not create it, it routes traffic into it.

Strip those nine and `2♥`'s remaining 162 boards are +81 plain / −22 PD — a
wash, not a loss. The `2♥`/`2♠` asymmetry is the reverse of what the extra room
predicts (the club cue is the one *with* the cheap `2♠` ask and it is the
loser), so the ask rungs, not the cue itself, are where the next fix goes.

The mirror-read leak is unchanged and still unfixed: 25.2% (NV) / 31.4% (vul)
of divergences were opened by *them*, and they are PD-positive (+0.52 / +0.36
per fired) — noise from a path this package does not own. `--gate-opener ours`
fails on both this pair and the shipped N1 pair.

**Inertness**: `smoke-default --count 20000 --seed 1` SHA-256 `8ea2f567…`
identical at HEAD with both knobs off — the N1 reference hash, unchanged.

## N1c — the club transfer and invitational minors (`defense_2c_landy_transfer`, measured 2026-08-14, **stays opt-in — but it is the best arm this package has produced**)

The earlier N1c sketch was a Lebensohl `2NT` relay, and it was deferred as
mostly redundant with the completed N1b skeleton. The v4 decomposition above
replaces that guess with numbers, and they point somewhere else: keep the cues
verbatim, and re-spend **the two rungs below them** — one is the package's
biggest earner sitting a level too high, the other is its second-worst loser.

| Responder, over their `2♣` | | Weight | vs N1b |
| --- | --- | --- | --- |
| `3NT` | game values, both majors stopped, no six-card minor | 180 | — |
| `3♣` / `3♦` | **INV (8-9), 6+ suit** | **176 / 175** | was the weak escape at 110 |
| `2♥` / `2♠` | INV+, 5+ clubs / 5+ diamonds | 173 / 172 | — |
| `X` | values, 8+ hcp | 145 | — |
| `2♦` | weak, 5+ diamonds | 140 | — |
| `2NT` | **transfer to clubs, weak 6+** | **110** | was the natural invite at 130 |
| `3NT` | game values, ungated | 170 | — |

Three moves, one package:

- **`2NT` = transfer to clubs.** The weak `3♣` escape was the engine
  (+3.41/+2.98 plain, +0.71/+0.61 PD per fired). Their `2♣` is artificial, so
  clubs are ours; transferring puts the escape a level lower *and* right-sides
  it into the 15-17 hand. Opener's completion reuses
  `complete_lebensohl_relay()` — the same forced `3♣` table the relay uses —
  and responder passes it.
- **The natural `2NT` invite is dropped**, which is what pays for the transfer.
  It cost almost nothing: the values `X` at 145 outranks it at 130 on every
  hand with 8+ hcp, so all it ever carried was the 8-9 *point* hand with fewer
  than 8 hcp. That hand now passes unless it has a suit — and shape points are
  exactly what a values double should not be floored on.
- **`3♣`/`3♦` = invitational with six.** The weak `3♦` measured −1.07/+0.55
  plain and −1.71/−0.55 PD, duplicating the `2♦` below it, so both direct 3m
  rungs are re-spent on the invitational six-carder. That hand cues badly: the
  cue's accept/decline tree hunts stoppers on the assumption of a five-card
  suit, where a six-bagger wants a yes/no on 3NT. Opener answers with the same
  size decision as every other 1NT invite (`size_ask_accept_floor`, 16): `3NT`
  from the top with both majors stopped, else sit — minor game is the five
  level and out of reach of a combined 23-26. The game-forcing six-carder
  still cues, since `3m` is capped at 9.

And one repair the user named directly: **`4♣`/`4♦` over opener's minimum
rebids is a slam try** (13+ with the six-card suit), ranked above the `3NT` it
displaces. The cue ladder had no rung for a six-card source of tricks with slam
values — it could only land in game. Opener's continuation is deliberately the
floor's, and this is the one place the measurements say that is right: the
boards that cost the cue's first draft −1.8 IMPs/fired were exactly the ones
where a `4m` suit contract let the floor cue-bid on to `6♦` while the notrump
rung died in 3NT. The slam try is gated on **N1c only**, not on the cues,
because the two rebid tables are shared with N1b and the four-arm A/B only
attributes cleanly if the cues arm stays the structure that was measured.

`2NT` is the one new artificial call (`comp:landy-transfer`); it projects
`len(♣,6..) & points(..=9)`, tight enough for `project_authored`, so no hand
reader. Opener's completion is natural in the target and unalerted, per the
`complete_advance_transfer` doctrine.

**How much of the transfer the harness can see.** `ab-dump-diff` pairs
`final_contract`, which returns `(Contract, Seat)` — declarer included — and
`ns_score_with` indexes the DD table by declarer, so right-siding *is* priced
to the extent it moves the double-dummy trick count. What DD cannot see is the
**lead** half: it hands the defence a perfect lead against either declarer, so
the tenace protection that is the whole point of transferring is invisible.
That half is what the 16-world SD pass prices, which makes the SD row unusually
load-bearing here. (An earlier draft of this section said the transfer measures
zero by construction. It does not — that was wrong.)

### N1c measured (2026-08-14)

230.4k bd/arm/vul, SEED_BASE 1786657996, sha `f313f3d`+dirty, 0.06–0.07% fired.
The increment over N1b:

| `xfer↔cues` | plain DD | PD | plain SD | SD-PD |
| --- | --- | --- | --- | --- |
| NV | +0.0002 ±0.0006 | **+0.0008 ±0.0008** | −0.0002 ±0.0006 | +0.0002 ±0.0008 |
| vul | +0.0003 ±0.0006 | **+0.0011 ±0.0008** | −0.0003 ±0.0007 | +0.0002 ±0.0008 |

PD **+1.10 / +1.90 per fired** — plain wash with a PD gain replicating at both
vulnerabilities, the shape N1 itself shipped on.

**And it is the first arm here that substantially passes the isolation gate**:
`probe-divergence --gate-opener ours` finds **1 of 132** divergent boards opened
by the opponents (0.8%), against **27%** for N1b. The mirror-read leak barely
touches N1c.

Against the **shipped** counter — the comparison that actually decides shipping,
since N1b is opt-in — pooled over two seeds (1786657996, 1786659297; 460.8k
bd/vul; the second seed run as `landy-xfer ↔ landy-on` only):

| `xfer↔on` | plain DD | PD | plain SD | SD-PD |
| --- | --- | --- | --- | --- |
| NV | **+0.0013 ±0.0007** | +0.0005 ±0.0009 | **+0.0018 ±0.0008** | **+0.0012 ±0.0009** |
| vul | +0.0007 ±0.0008 | −0.0005 ±0.0010 | **+0.0011 ±0.0009** | +0.0000 ±0.0010 |

Four CI-clear positive cells, no CI-clear negative. Seed 1 alone showed a vul PD
of −0.0010 ±0.0014 and it **did not replicate** (seed 2: −0.0001 ±0.0013) — a
reminder to confirm a single-seed negative before designing against it.

**It stays opt-in** for the reason N1b did: plain/SD win with PD a wash is the
decision table's artifact row.

### What is still bleeding — the cue poaches the double

`probe-divergence`, boards *we* opened, vul (NV in parentheses), N1b against the
base counter. Gating on `ours` makes the loss **worse**, not better
(−0.0017/bd of the −0.0020), so this is not the mirror leak:

| base → cues | n | plain/fired | PD/fired |
| --- | --- | --- | --- |
| `X` → `2♠` | 40 | −0.47 | **−3.83** (−2.14) |
| `X` → `2♥` | 38 | +1.05 | **−2.63** (−0.87) |
| `2♥` → `3NT` | 8 | −6.38 | −4.00 |

Plain DD *likes* the cue; PD prices the contract we reach as failing and
doubled, against defending their Landy contract. It scales with vulnerability,
as giving up a red double should. The cue is `points(8..)` at weight 173/172
against the double's 145, so it takes **every** 8+ point hand with a five-card
minor.

N1c wins by pulling hands *off* that call — which is why its gains land exactly
where the design predicted (cues → xfer, vul, boards we opened):

| cues → xfer | n | plain/fired | PD/fired |
| --- | --- | --- | --- |
| `2♠` → `3♦` (INV six-card ♦ off the cue) | 16 | +3.75 | **+6.06** |
| `3♦` → pass (weak `3♦` deleted) | 21 | +0.71 | **+3.38** |
| `2♥` → `3♣` (INV six-card ♣ off the cue) | 23 | +1.78 | **+2.22** |
| `3♣` → `2NT` (the transfer itself) | 61 | −0.52 | +0.67 |

**Named next lever (N1d):** raise the cue's floor from `points(8..)` to
`points(10..)`, leaving the 8-9 five-card-minor hands to the values double.
One line, directly targeted at the −3.83/−2.63, and it makes the cue's INV+
accept/decline tree partly redundant — so price the tree's simplification in the
same A/B.

### Per-bid decomposition against the shipped counter (`xfer↔on`, 2026-08-14)

The tables above price the two increments separately. This one prices the whole
of N1c+N1b against what we ship, on the seed-1786657996 pair — 406 divergent NV
and 321 vul. Boards we opened only; the mirror leak is broken out below. IMPs
per fired board throughout.

**Per counter bid.** Every divergent board is keyed twice: once on the call N1c
makes over their `2♣`, and once on the call N1 made on the same hand. Read the
first table as *what each of our rungs is worth when we choose it*, and the
second as *what we gave up to choose it*.

| N1c's call | n (NV/vul) | plain NV | PD NV | plain vul | PD vul |
| --- | --- | --- | --- | --- | --- |
| `3♦` INV six-card ♦ | 18 / 16 | +3.00 | **+3.33** | +4.38 | **+4.44** |
| `3♣` INV six-card ♣ | 33 / 26 | +3.12 | **+3.30** | +3.35 | **+2.46** |
| pass (natural `2NT` deleted) | 5 / 5 | +1.80 | +4.40 | +2.20 | +3.40 |
| `2NT` club transfer | 64 / 57 | **+2.27** | **+1.17** | +1.72 | −0.04 |
| `2♠` ♦ cue | 81 / 53 | +1.33 | −0.37 | +0.47 | **−2.06** |
| `2♥` ♣ cue | 93 / 72 | +0.23 | **−0.84** | −1.01 | **−2.79** |

| N1's call it replaced | n (NV/vul) | plain NV | PD NV | plain vul | PD vul |
| --- | --- | --- | --- | --- | --- |
| `2NT` natural invite | 33 / 28 | +1.88 | **+3.18** | +2.75 | **+2.64** |
| pass (no call existed) | 64 / 57 | +2.27 | +1.17 | +1.72 | −0.04 |
| `2♦` weak ♦ | 31 / 27 | +2.13 | +1.84 | +2.26 | +1.15 |
| `3♦` forcing ♦ | 17 / 11 | +1.59 | +0.59 | +2.18 | +1.18 |
| `X` values | 108 / 78 | +1.53 | **−0.92** | +0.67 | **−2.53** |
| `3NT` | 19 / 9 | −0.16 | +1.79 | −4.22 | −1.89 |
| `3♣` forcing ♣ | 22 / 19 | −1.00 | **−1.09** | −2.95 | **−3.26** |

The two tables agree on one verdict: **the three new invitational/transfer rungs
are all clearly positive on both scorers at both vulnerabilities, and the two
cues are the only negative rungs in the package.** The `X` row is the same
finding seen from the other side — every hand the cue takes from the values
double is worth −0.92 PD NV and −2.53 PD vul.

Keyed on the *first diverging call* instead, so each row is one substitution:

| N1 → N1c | n (NV/vul) | plain NV | PD NV | plain vul | PD vul |
| --- | --- | --- | --- | --- | --- |
| `3NT` → `2♠` | 7 / 1 | +5.14 | **+6.86** | +13.00 | **+15.00** |
| `2NT` → `3♣` (natural invite → INV six-card ♣) | 15 / 14 | +3.07 | **+4.60** | +3.50 | **+3.57** |
| `X` → `3♦` | 6 / 5 | +3.33 | **+5.17** | +3.60 | **+3.80** |
| `2♦` → `3♦` | 12 / 11 | +2.83 | +2.42 | +4.73 | **+4.73** |
| `X` → `3♣` | 16 / 11 | +3.19 | +2.25 | +3.27 | +1.45 |
| `2NT` → pass (natural invite deleted) | 5 / 5 | +1.80 | +4.40 | +2.20 | +3.40 |
| **pass → `2NT`** (the club transfer itself) | **64 / 57** | **+2.27** | **+1.17** | **+1.72** | **−0.04** |
| `3♦` → `2♠` | 16 / 9 | +1.31 | +0.25 | +1.11 | −0.11 |
| `3NT` → `2♥` | 12 / 8 | −3.25 | −1.17 | **−6.38** | **−4.00** |
| `3♣` → `2♥` | 22 / 19 | −1.00 | −1.09 | **−2.95** | **−3.26** |
| `X` → `2♥` | 38 / 28 | +1.26 | **−2.21** | +0.14 | **−4.00** |
| `X` → `2♠` | 48 / 34 | +0.96 | **−1.71** | −0.18 | **−3.53** |

Three things these settle that the increment tables could not:

- **The transfer is the single biggest earner, and it is mostly a new call, not
  a re-rung one.** 64 NV / 57 vul boards where N1 *passed* now bid `2NT` —
  the weak six-card club hand had no call at all under the base counter
  (`X` needs 8 hcp, `2♦` needs diamonds). That is +145 plain / +75 PD NV. It is
  a wash on PD vul, which is the honest reading of a weak escape at red.
- **Every call that migrates *into* the cue loses, and every call that migrates
  *out of* it wins.** Splitting the our-opened set on `call_on ∈ {2♥, 2♠}`:

  | | n (NV/vul) | plain/fired | PD/fired | → IMPs/board |
  | --- | --- | --- | --- | --- |
  | into the cue | 174 / 125 | +0.74 / −0.38 | **−0.62 / −2.48** | −0.0005 / **−0.0013** |
  | everything else | 120 / 104 | +2.59 / +2.56 | **+2.22 / +1.44** | **+0.0012 / +0.0007** |

  The cue is the *whole* vul-PD deficit and then some. N1d is no longer a
  hypothesis about the `X` rows — it is the arithmetic of the package.
- **The `3♣` → `2♥` row is a design error of mine, not a floor gap.** The
  partition I authored sends the GF six-card minor to the cue on the 5-vs-6
  doctrine ("`3m` is capped at 9, so the six-card GF still cues"). Measured, the
  base counter's forcing `3♣` beat that on both scorers at both vulnerabilities
  (−1.09 / −3.26 PD). N1d should raise the cue's floor **and** restore a
  forcing `3♣`/`3♦` above it, not merely trim the bottom.

The mirror leak (`read.rs:333-335`) carried 112 of 406 NV and 92 of 321 vul
divergent boards — 27.6% / 28.7%, unchanged from N1b, exactly as predicted. It
is worth −73 plain / +9 PD NV and −71 / −71 vul, i.e. it *depresses* the
headline; the our-opened figures above are the honest ones.

### The interference hole, now priced

Every registered suffix still ends in `-`, so the moment an opponent bids over
one of our rungs the auction drops to the floor. That is not hypothetical: in
the divergent set the opponents bid over our cue on 47 boards — `2♥` → `(2♠)`
28 times, a raise to `3♥`/`3♠` 18 times, `(4♠)` once. Splitting the cue-entry
boards on whether the cue was interfered with:

| | n (NV/vul) | plain/fired | PD/fired |
| --- | --- | --- | --- |
| cue interfered | 25 / 22 | −1.48 / **−3.91** | −0.52 / **−4.68** |
| cue clean | 149 / 103 | +1.11 / +0.37 | −0.64 / −2.01 |

So the hole is real and expensive vul — but it is the *smaller* problem. The
clean cue loses PD on its own (−0.64 NV, −2.01 vul) on six times the volume.
Authoring `{cue} (X)` and `{cue} (2♠/3M)` continuations would recover perhaps a
fifth of the cue's deficit; raising its floor addresses the rest. Do N1d first,
then re-price the hole against whatever cue survives.

`{cue} - 3NT -` and `{cue} - 4m -` (opener's maximum rungs) also remain
unauthored, and did not surface in this dump.

### The fit forensic — where the cue's contracts actually land (2026-08-14)

The user's hypothesis for the cue's deficit was 7-card fits: `landy_cue_answer`
was *designed* to raise on a doubleton ("opener is balanced, so it never lands
on fewer than two"), so a 5-card cue opposite a doubleton catch-all is a 5-2
partial by construction. `probe-divergence` now stamps every record with
`fit_on`/`fit_off` (combined trump length of the declaring pair, null for
notrump or a pass-out), so the question is a jq filter over the same
`xfer↔on` probe files. Cue-entry boards we declare (151 NV / 115 vul), IMPs
per fired:

| final contract | n NV | plain | PD | n vul | plain | PD |
| --- | --- | --- | --- | --- | --- | --- |
| notrump | 82 | +0.93 | **−1.72** | 64 | −0.52 | **−3.86** |
| 8-card suit fit | 27 | −0.15 | −0.85 | 18 | −0.11 | −1.00 |
| 9-card suit fit | 24 | +2.88 | +3.04 | 16 | +3.63 | +3.25 |
| 10+ | 7 | +3.29 | +3.14 | 6 | −0.67 | −1.50 |
| **≤7-card "fit"** | **11** | **−6.27** | **−10.0** | **11** | **−5.18** | **−8.18** |

The answer is **mostly no, but the yes half is catastrophic**. A 5-2 (or worse)
final is 7% of the cue's boards and −10/−8 PD *each* — roughly −0.6 IMPs of the
cue's per-fired deficit at both vulnerabilities, a fifth to a quarter of it.
The majority of cue boards land in notrump, and that bucket's steady PD bleed
(−1.72/−3.86) is the poached-values-double arithmetic of the previous section,
not a fit problem.

The ≤7 bucket itself is two distinct defects, half and half:

- **The doubleton catch-all raise, working as authored.** `2♥ - 3♣ - - -` on
  five clubs opposite two, at both vulnerabilities; one goes for 500 under a
  reopening double (`3♦ - - X`, −9). Two of the nine such partials *win* — a
  5-2 minor partial is sometimes the right spot — but the bucket nets heavily
  negative, and one board rides the 4m slam try up to `6♣` on a seven-card fit.
- **Interference-hole artifacts, the worst boards in the whole divergent set.**
  Every registered suffix ends in `-`, so their `3♠` raise (or a balancing
  `3♥`) drops the auction to the floor mid-convention, and the floor — reading
  bare envelopes with no forcing channel — bids a 3-5 card **major** at the
  four or five level: `2♠ - 3♦ 3♠ 4♦ - 4♠ X` on a 3-card fit (−15),
  `2♥ - 3♣ 3♠ 4♦ - 4♠ X` on four (−15/−17), `2♥ - 2NT - - 3♥ 4♥` on five
  (−15). These are the −14…−18 PD boards.

So the two fixes named by this forensic are exactly N1e and N1f below: an
answer structure that offers notrump on a doubleton instead of manufacturing a
5-2 raise, and authored continuations so an opponent's call over the cue stops
handing the auction to a floor that cannot know the cue was forcing.

## N1d / N1e / N1f — the cue repairs (authored 2026-08-14)

Three toggles, one increment each, stacked in evidence order on top of N1c
(each implies `defense_2c_landy_transfer`; all default off, default system
byte-identical, smoke SHA `8ea2f567…` unchanged):

- **N1d `defense_2c_landy_cue_floor`** — the cues go `points(8..)` →
  `points(10..)`, one line.  The per-bid decomposition's arithmetic: every
  hand that migrated `X` → cue measured −0.92/−2.53 PD per fired, because the
  cue at weight 173/172 against the double's 145 took *every* 8+ point hand
  with a five-card minor.  The 8-9s go back to the double; sub-8-hcp shapely
  hands fall to `2♦`/the transfer/Pass.  The decomposition's *other* N1d idea —
  restoring a forcing `3♣`/`3♦` above the cue for the GF six-carders — is
  **deferred**: one `3m -` suffix gets one answer table, and no table can both
  sit on an 8-9 invitation and never pass a game force.  The cue route for
  six-carders may also heal under N1e/N1f (its losses were part answer-tree,
  part interference); re-read the `3♣` → `2♥` row after this round, and if it
  still bleeds, that is its own arm.
- **N1e `defense_2c_landy_fit_answers`** — the fit forensic's fix, per the
  user's design: `landy_cue_answer`'s notrump rungs become *(both majors
  stopped, or ≤2-card support)* at both strength levels — `3NT` maximum, `2NT`
  minimum, level still carrying strength — and the terminal catch-all flips
  from `3m` to `2NT`, so every raise and ask now **promises 3+** and the 5-2
  raise (−10.0/−8.2 PD per fired) stops existing.  A stopper is guaranteed
  only alongside a fit; responder knows which story the rung told from the
  rung that told it.  The re-cue node is fixed transitively — it only exists
  over a genuine raise now.
- **N1f `defense_2c_landy_competition`** — the interference hole's fix, three
  shapes: their `X` of a cue or an ask takes no room, so opener/responder
  answer **verbatim** (the immediate table re-registered on the `(X)` suffix)
  and every deeper X-then-bid tail is stripped back onto the clean subtree by
  a `systems_on_over_double` rebase (the contested-Stayman idiom — one entry
  covers the whole doubled subtree, asks, rebids and re-cues included); their
  raise over a cue (`2♠`/`3♥`/`3♠` — 47 priced boards) gets a compressed
  ladder (`3NT` = both stopped + max, the raise = 3+ by size, Pass = the rest,
  safe because responder is INV+ and guaranteed another turn); and the doubled
  club transfer is still completed, with the sign-off intact.  `(4♠)`,
  overcalls of opener's answers, and everything deeper stay the floor's,
  deliberately.

### N1d/e/f measured (2026-08-14) — **SHIPPED DEFAULT-ON, the package's first `win | win`**

Seed 1786694464, 230.4k bd/arm/vul, arms `xfer → d → e → f → on`, sha
`d3df592`+dirty; ship pair confirmed on a second seed 1786695954
(`scripts/ab-landy-confirm.sh`).  The whole five-arm, two-vul round — the
census's original loser to a shipped default — ran in 19 minutes.

**The increments.**  N1d is the package's engine; the other two are the
priced hygiene they were designed to be:

| increment | fired (NV/vul) | plain NV | PD NV | plain vul | PD vul |
| --- | --- | --- | --- | --- | --- |
| `d↔xfer` cue floor | 169 / 136 | −0.0002 ±0.0006 | **+0.0009 ±0.0008** | +0.0002 ±0.0007 | **+0.0015 ±0.0009** |
| `e↔d` doubleton-NT | 3 / 1 | ~0 | ~0 | ~0 | ~0 |
| `f↔e` interfered tails | 17 / 11 | +0.0001 ±0.0003 | +0.0000 ±0.0004 | −0.0000 ±0.0003 | +0.0001 ±0.0003 |

N1d lands on the decision table's `wash | win` row at both vulnerabilities —
the shape the base counter itself shipped on — at +1.21/+2.49 PD per fired.
Keyed per substitution, **cue→X is 55-60% of every divergence and carries
+2.0…+5.1 PD per fired**: the `X`→cue poaching rows, reversed, exactly as the
per-bid decomposition predicted.  Its sd-plain NV cell is CI-clear negative
(−0.0009 ±0.0007), but sd-plain relaxes the lead *and* drops the doubling —
it cannot overrule (measurement.md §"Plain SD is not an arbiter") — and
SD-PD, the sd arbiter, is a wash (−0.0000/+0.0007).  N1e fired **three times
NV and once vul**: the cue floor had already removed most doubleton-raise
cues, so the fix it was designed for barely has a population left — it ships
on the naturalness tiebreak (raises promise 3+ support; the alternative was a
priced −10/−8 PD per fired 5-2).  N1f's own increment is the expected
CI-wide wash; it ships as convention-completion (the iron rule's interfered
tails).

**The isolation gate** (`probe-divergence --gate-opener ours`): `e↔d` and
`f↔e` pass at **zero** foreign boards — pure book changes.  `d↔xfer` fails at
18-21% and `f↔on` at 41-43%: N1d edits the cue *constraints*, i.e. the same
`2♥`/`2♠` readings the mirror leak (`read.rs:333-335`) reflects onto auctions
where **they** open 1NT and **we** overcall `2♣` — N1c's own 28% plus the new
cue-floor delta.  As with N1/N1b, the leak is quantified and it *depresses*
the headline rather than carrying it (foreign boards: −26/−34 PD on `d↔xfer`;
+15/+52 on `f↔on` — noise both ways), so the our-opened figures below are the
honest ones and they are **stronger** than the headline.

**The ship comparison, pooled over both seeds** (460.8k bd/vul; all-boards
and the our-opened honest subset):

| `f↔on` pooled | plain DD | PD |
| --- | --- | --- |
| NV all | **+0.00068 ±0.00062** | +0.00075 ±0.00077 |
| NV ours | **+0.00091 ±0.00052** | **+0.00077 ±0.00064** |
| vul all | **+0.00085 ±0.00072** | **+0.00100 ±0.00087** |
| vul ours | **+0.00075 ±0.00058** | +0.00060 ±0.00070 |

Six of eight cells CI-clear positive, the other two short by a hair, every
sd cell positive at both seeds (8/8), **no negative cell in 24 readings** —
the decision table's `win | win` row.  Per-seed: seed 1 all-positive with
plain NV at the boundary; seed 2 replicated and strengthened
(vul PD **+0.0014 ±0.0012** CI-clear alone).

**Shipped 2026-08-14**: `defense_2c_landy_transfer`, `defense_2c_landy_cue_floor`,
`defense_2c_landy_fit_answers` and `defense_2c_landy_competition` default
**true**.  All four engage only under the `their.two_clubs_landy` declaration,
so the default *system* is untouched — `smoke-default` SHA `8ea2f567…`
unchanged — and the engine's undeclared read stays natural, as the N1 ship
demanded.  Harness contract: `bba-gen`'s four stack flags became
`Option<bool>` (unset = engine default; a pre-ship arm is spelled
`--defense-2c-landy-<knob> false`), and `bba-decompose` gained
`--landy-stack false` for replaying dumps generated between the base-counter
ship and the stack ship — the same correction `--landy-counter false` gives
the pre-N1 era.  `ab-landy-counter.sh`'s arms are re-spelled in post-flip
terms (landy-f *is* the default; landy-on switches the stack off).

**Named residue.**  Their *second* call still drops us to the floor: N1f's
worst board is `1NT (2♣) 2♥ (2♠) 3♣ (3♠) 4♦ - 4♠ X` — the authored answer to
the first raise, then their re-raise, then the floor bidding a phantom `4♠`
on Q3 one level deeper than the hole N1f closed (−17 PD).  One authored round
was the priced fix; the deeper fix is the floor's phantom-suit discipline,
per the architecture doc's smarter-floor rule, not another node ring.  And
the `3♣`→`2♥` row (GF six-carders through the cue) is still open — re-read it
against the shipped stack before considering a forcing-3m arm.

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
| N1d/e/f cue repairs | `defense_2c_landy_cue_floor` + `_fit_answers` + `_competition` (each implies `_transfer`; all four now default **true**) | **SHIPPED DEFAULT-ON 2026-08-14** — the package's first `win \| win`. Stack vs shipped base (`f↔on`), pooled seeds 1786694464 + 1786695954, 460.8k bd/vul: **six of eight DD cells CI-clear positive, 8/8 sd cells positive, no negative cell in 24 readings** (table in §N1d/e/f measured). Engages only under the `their.two_clubs_landy` declaration — default system byte-identical, smoke `8ea2f567…` unchanged; `bba-gen` stack flags are `Option<bool>` (pre-ship arm = `--defense-2c-landy-<knob> false`), `bba-decompose --landy-stack false` replays between-ships dumps. Increment attribution: **N1d is the engine** (`d↔xfer` plain wash + PD **+0.0009 ±0.0008** NV / **+0.0015 ±0.0009** vul, cue→X = 55-60% of divergence at +2.0…+5.1 PD/fired — the poached-double rows reversed); N1e fired 3+1 boards post-floor (ships on naturalness: raises promise 3+); N1f the expected CI-wide wash (ships as the iron rule's convention-completion). Isolation gate: e/f pass at 0 foreign; d and f↔on fail at 18-43% (the cue-constraint mirror leak), foreign boards *depress* the headline — our-opened figures are stronger. Residue: their **second** call still floors us (phantom `4♠` one level deeper, −17 PD, 1 board); the `3♣`→`2♥` GF-six-carder row unread against the shipped stack. | `f↔on` pooled: NV plain **+0.00068 ±0.00062** / PD +0.00075 ±0.00077, vul plain **+0.00085 ±0.00072** / PD **+0.00100 ±0.00087**; ours-only NV plain **+0.00091 ±0.00052** / PD **+0.00077 ±0.00064**, vul plain **+0.00075 ±0.00058** / PD +0.00060 ±0.00070. |
