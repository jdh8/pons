# The Dutch system — campaign ledger

Dutch is a **natural 2/1 built around a wide, non-forcing 1♣** — a "lawyer's
Polish Club" that naturalises the Polish 1♣: Polish constructiveness, but
natural and less restricted. It is a **champion candidate**. We copy
[`american()`](../src/bidding/american.rs) and apply the Dutch diff one
measurable phase at a time; Dutch promotes to the shipped default only if it
measures stronger, at which point `american()` demotes to the ablation baseline
(the WBridge5-ships-a-modded-French model).

Read [docs/bidding-architecture.md](bidding-architecture.md) first (the
book/floor/inference layer cake) and [docs/measurement.md](measurement.md) (no
bidding change ships without an A/B).

## Target system

The full bidding spec — openings, the 1♣ response ladder, opener's relay
rebids, and the deep continuation trees — is transcribed from jdh8's Watermelon
Dutch book in **[dutch-spec.md](dutch-spec.md)** (with pons deviations flagged
inline). In one line: a wide non-forcing **1♣** (11–23 catch-all) with a **1♦!
relay** carrying the awkward and the very strong; **1♦** = 5+♦ or the
singleton-club 4441 (never 3♦; every other 4-diamond hand — incl. (xx)45 —
opens 1♣); five-card majors 10–20; a strong artificial **2♣**; and (Phase 3)
Multi / Muiderberg / UNT at the 2-level replacing the weak twos.

## Decisions

1. **Layout = reuse + override.** `dutch()` is a thin factory that reuses
   american's shared module `register()`s (1NT, slam, XYZ, raises, most rebids,
   most competition/defense) and swaps in Dutch modules only for the divergent
   parts. No fork of the ~19k-line tree.
2. **Family = `Family::NATURAL`.** Opponents defend Dutch as natural for now.
3. **2-level openings = lift the shapes, replace weak twos.** Multi/Muiderberg/UNT
   replace american's weak-two 2♦/2♥/2♠ and strong 2NT; strong balanced 20–23
   moves into the wide 1♣. Reuse the existing `woolsey_multi` /
   `woolsey_muiderberg` / `unusual_2nt` shape builders (lifted out of
   1NT-defense gating), re-banded to weak-two strength. (In this codebase
   "Woolsey" is a *defense to their 1NT*, not opening preempts — only the shape
   builders are reused.)

## The central risk — reader/floor are american-tuned

`inference.rs` (the reader) and `instinct.rs` (the floor) are tied to american's
meanings. Two mechanisms:

- **Rule projection** (`set_alert_reading`, default-on) makes every alerted,
  DSL-authored artificial call self-decode by replaying its own `project` fold.
  So Dutch's strong 2♣, Multi 2♦, Muiderberg 2M, UNT 2NT, and the 1♣–1♦ relay
  decode **for free** once authored with projecting combinators + `.alert(...)`.
- The **hardcoded american reading** decodes the natural, unalerted openings.
  Dutch's natural openings differ by range/shape (1♣ 11–23 2+♣; 1♦ 5+/4414; 1M
  10–20), so they get a **soft misread** (biased HCP band), not a phantom-suit
  disaster. Strategy: rely on projection for the artificial calls, accept soft
  natural misreads, **measure the cost**, fix only the worst offenders (Phase 4).

`instinct()` is keyless and only fires off-book, so **completeness of authored
Dutch nodes** (both sides of every convention) is what keeps the floor from
bidding an artificial opening as natural. Dutch keeps american's 15–17 1NT, so
the floor's transfer-completion still holds.

## Phase ledger

| Phase | Scope | Status |
| --- | --- | --- |
| 0 | Scaffold `dutch()`, re-export, 0.000 baseline | **DONE** |
| 1 | Dutch openings: wide 1♣, 1♦ 5+/4441, 1M 10–20, strong 2♣ | **DONE** (code; A/B pending) |
| 2.1 | Wide-1♣ response table + opener's rebid after the `1♦` relay | **MEASURED — LOSS** (see below); on-plan for a half-built system |
| 2.2 | Deep relay continuations (`1♣-1♦-1M/1NT/2♣/2♦`) + `[1♣,2♣]`/`[1♣,2♦]` continuations | **increments 1–2 AUTHORED** — inc.1 `1♣-1♦-1M` + `1♣-1♦-2♣`; inc.2 opener's rebid over `2♣`/`2♦` **+ responder's continuation** (opener-only cut LOST → responder side authored → **re-A/B WIN `+0.0021/bd plain both`**). Rare relay `1NT`/`2♦!` still deferred |
| 3 | 2-level openings (Multi + **BBA's Polish two-suiters**/UNT) + strong-2♣ tree | pending — **Muiderberg superseded**, see below |
| 4 | Reader/floor reconciliation + divergent-opening competitive book | pending |
| 5 | Iterate to champion vs BBA/BEN; promote if it wins | pending |
| WJ-floor | Distil BBA-WJ as the floor over Dutch's divergent minors | **A/B A WON** (floor swap, +0.18/+0.28 plain, shipped); **A/B B LOST** (WJ over 1♦, −0.005/−0.017 PD — inherited overbid); **A/B C LOST** (WJ as *constructive* floor under 1♣, −0.012/−0.029 — nets have no settle rail); both routings removed, net kept; Phase 3's two-level rows are the remaining arm |

Each phase gates on a paired-seed A/B via `examples/bba-gen` (dutch arm vs
american arm), dual-scored (`ns_score_pd` + `ns_score_contract`), fresh
`SEED_BASE`, run sequentially under `scripts/idle-run.sh`. Preemptive phases
(3) are read knowing DD is blind to obstruction — lean on the sd-lead / PD
bracket, not plain DD alone.

### The WJ-floor campaign — BBA's Polish Club as Dutch's teacher

Dutch is not a rival to Polish; it is american with the minor openings replaced.
So the floor decomposes per opening, and BBA's WJ (EPBot system `2`,
`vendor/bba/WJ.bbsa`) is a machine teacher for exactly the subtrees where Dutch
leaves american. Full plan and the measured evidence:
`~/.claude/plans/would-it-be-easier-dynamic-sedgewick.md`.

**Step A — SHIPPED, A/B A won on all four cells.** `dutch()` never got the floor
swap `american()` got on 2026-07-19 — it ran `with_instinct_floor` everywhere, so
the claim "Dutch's majors are bit-identical to american" was false at the floor.
`dutch()` now takes `NeuralFloorBba`; `dutch_instinct()` preserves the old body
as the disclosable reference and A/B baseline. Runner
`scripts/dutch-floor-ab.sh`, 204 800 bd/arm/vul, seed base 1784493655.

| `dutch` − `dutch-instinct` | plain DD | perfect defense | fired |
| --- | --- | --- | --- |
| vul none | **+0.1764** ±0.0136 | **+0.1678** ±0.0166 | 28.11% |
| vul both | **+0.2764** ±0.0169 | **+0.3471** ±0.0203 | 26.31% |

Roughly **double** what the same swap bought `american()` (+0.11/+0.25), which is
what the decomposition predicts: `dutch()` ran the deterministic floor over
strictly more unauthored territory, so it had more to gain. Plain and PD agree at
both vulnerabilities — not a doubling artifact.

**The residual tail is a redouble bug, and it is probably american's too.** Four
of the five worst plain boards and four of the five worst PD boards share one
shape: the net **redoubles** an opponent's double of our artificial call instead
of bidding on —

```
on:  2♥ - 2NT - 3♣ X XX - - -      off: 2♥ - 2NT - 3♣ X 4♥ - - -   [-23 IMP]
on:  - - 1♠ 2♠ 3♥ X XX - - -       off: - - 1♠ 2♠ 3♥ X 4♠ - -      [-24 IMP]
```

These are american weak-two / competitive subtrees that Dutch inherits unchanged.
The node is contested and off-book — the Ogust rows are keyed
`[2♥, P, 2NT, P, 3♣, P]` and trie resolution is exact-depth, so a trailing
`Double` has no child and falls through — so it is `NeuralFloorBba` speaking, and
nothing in the floor path masks or penalises `XX` (only `mask_illegal`).

**Probed, and it is not a defect.** Two hypotheses, both refuted on the A/B's own
dumps (51 200 boards, N/S calls only, `table_b` = the all-BBA reference on the
same deals and seats):

| N/S call | our net floor | all-BBA reference | instinct floor |
| --- | --- | --- | --- |
| `X` | 12.40% | 13.85% | — |
| `XX` | **0.71%** | **0.69%** | 0.45% |

1. *Misaligned `XX` logit index* — refuted. A transposed `X`/`XX` would put `XX`
   at doubling's ~13%, not 0.7%. (The Rust chain is provably consistent anyway:
   one canonical ordering in `array.rs`, and EPBot's differing code space is
   absorbed by a `Call` round-trip rather than crossing as a raw tensor index.)
2. *A defect in the shared net, live in `american()` too* — refuted. The net
   redoubles at **its teacher's own rate**; it is cloning BBA faithfully. What
   differs is `instinct()`, which redoubles less (0.45%).

So the tail is a **selection effect in the worst-boards list**, not a bug: sorting
by largest divergence surfaces exactly the boards where the net redoubled and
instinct bid on, while the +0.18…+0.35 mean says the net's judgement is better
overall. Whether BBA *and* our clone both over-redouble relative to optimum is a
live system question — the aggregate rates match, but that does not prove either
is right in this specific position — and it is a separate investigation from this
campaign.

**Step 0 — SHIPPED as a negative check, not a gate.** `scripts/wj-calibration.sh`
plays BBA-WJ against BBA-2/1 (`--our-card vendor/bba/WJ.bbsa`, so all 134 named
conventions are pinned from the vendored file rather than from engine defaults).
Both sides are EPBot, so this measures the *systems* with bidder quality held
constant. 204 800 bd/vul, seed base 1784496013, ~43–44% divergent.

| BBA-WJ − BBA-2/1 | plain DD | perfect defense |
| --- | --- | --- |
| vul none | **+0.086** [+0.070, +0.102] | −0.001 [−0.019, +0.017] |
| vul both | **+0.086** [+0.066, +0.107] | **−0.038** [−0.061, −0.015] |

**The plain edge is a doubling artifact, and vulnerability proves it.**
`ns_score_pd` differs from `ns_score_contract` *only* by upgrading an undoubled
penalty to doubled when the contract fails double-dummy
([scoring.rs:179](../src/scoring.rs#L179)) — same trick table, so PD is
*punishment*, not better defence. WJ's plain figure is flat across vulnerability
(+0.086 both times) while its PD figure drops from level to a significant loss,
which is exactly what an overbid does: the extra failing contracts are cheap
undoubled, cheap doubled non-vulnerable, and expensive doubled vulnerable. In raw
points at `both`: +953 400 plain → −514 980 PD.

**This does not gate the campaign** (an earlier draft of the plan claimed it did;
jdh8 caught it). Dutch has *already* committed to a Polish-style minor structure,
so the WJ net's job is to be **in distribution** for it — replacing a floor
distilled from a 2/1 teacher that has never seen a Polish 1♦. That gain comes
from distribution match, not systemic edge; an equally-strong teacher speaking the
right language is precisely what is wanted. What Step 0 buys is (a) the negative
check that the chosen structure is not a systemic liability — it passes, WJ is at
worst −0.04 IMPs/bd against a peer — and (b) a competent-WJ reference dump.

**Pre-registered for Step 4**: what transfers is the teacher's *bias*, not only
its strength. Count double-dummy-failing contracts per arm directly from the
dumps. If A/B B wins plain and washes-or-loses on PD, that is this same inherited
overbid arriving in Dutch's 1♦ auctions, not judgement — and it should show up
**worse at `both` than at `none`**, the fingerprint Step 0 just recorded.

**Step B — A/B B LOST. The teacher's overbid transferred; the range mismatch did
not matter.** 204 800 bd/arm/vul, seed base 1784527375, `dutch-wj` (WJ net under
*our* 1♦) against `dutch` (BBA net throughout). Dumps kept at
`ab-results/dutch-wj/`; the runner and the `--our-floor dutch-wj` arm were
removed with the routing, so reproducing this needs both re-added.

| `dutch-wj` − `dutch` | plain DD | perfect defense | fired |
| --- | --- | --- | --- |
| vul none | +0.0019 ±0.0036 | **−0.0095** ±0.0043 | 1.91% |
| vul both | **−0.0052** ±0.0043 | **−0.0173** ±0.0051 | 1.71% |

Three of four cells lose outside their CI, and the shape is **exactly the
pre-registered fingerprint**: plain near-flat, PD losing at both vulnerabilities
and losing *worse* at `both` (−0.0173) than at `none` (−0.0095). Step 0 recorded
that same signature for BBA-WJ against BBA-2/1, so the bias transferred through
the distillation as predicted.

The mechanism is measured, not inferred — over the divergent 1♦ auctions:

| | vul none | vul both |
| --- | --- | --- |
| mean final contract level | 3.606 vs 3.276 (**+0.33**) | 3.458 vs 3.148 (**+0.31**) |
| reached the four-level or higher | 51.4% vs 40.5% | 44.2% vs 35.1% |
| reached slam | 291 vs 212 | 255 vs 173 |

The WJ arm bids a third of a level higher, reaches ~10 percentage points more
games and ~40% more slams. Plain DD barely notices — it rewards an aggressive
contract that happens to make — and PD charges for the failures, vulnerable
most. That is an overbid, not judgement.

**What this refutes: the 18+ range mismatch was never the problem.** The plan
pre-registered that gains should concentrate below 18 HCP, since WJ routes 18+
through its forcing 1♣ and its net has never seen a strong 1♦ opener. The
strength split lands at 10.2%/10.4% 18+, confirming the 9.7% harvest figure — but
the overbid is *larger* in the 11–17 bucket (+0.341 level) than in 18+ (+0.230).
So the loss is uniform teacher bias, not distribution mismatch, and capping the
routing at 17 — the repair the plan ruled out on architectural grounds — would
have removed only a tenth of the mass and the *smaller* half of the effect. It
was never the fix.

**The routing was removed; the net stays** (jdh8, 2026-07-20). `dutch_wj()` and
`DutchFloor` are gone — a measured-loss routing kept "just in case" is dead
surface that invites someone to re-enable it without re-reading the verdict.
`NeuralFloorWj` / `neural::classify_wj` / `weights/wj_bba.f32` remain embedded and
tested, because the net itself is not what failed: it is what Phase 3 wants, once
Dutch adopts BBA's Multi 2♦ and Polish two-suiters (decision below) and book and
teacher share the *same rows* — no range mismatch, no unauthored continuation
tree. Phase 3 wires its own routing over the two-level openings, which is a
different arm from the 1♦ one that was measured. **Assume the overbid follows
there until measured**; it is a property of the teacher, not of the 1♦ subtree.

**Step C — A/B C LOST, and lost harder. The WJ net as the *constructive* floor
under our wide 1♣.** 204 800 bd/arm/vul at `none`, seed base 1784530004, killed
before `both` on the strength of the result. Dumps at `ab-results/dutch-wj1c/`;
the runner, `dutch_wj_club()`, `DutchConstructiveFloor` and the `--our-floor
dutch-wj1c` arm were removed with the routing.

*The premise was right.* jdh8's argument was that a net should read `1♣-1♦` as a
relay, not as diamonds, and that WJ is the teacher that does. Measured over
7 077 EPBot-WJ responder rows, WJ's `1♣-1♦` is a `wildcard response` that
discloses **nothing** — no length range, no point range — with ♦ length running
0–8 including 58 voids, and spades as often the longest suit (2 176) as diamonds
(1 792). EPBot's 2/1 answers 1♣ with a natural `1♦` (`bidable suit`, `D:[4,13]`,
`pts:[6,29]`). So `american_bba` is a net that invents a diamond suit on Dutch's
relay and the WJ net is not. That much held up.

*The slot was the surprise.* `dutch()` is `with_floors(pair, NeuralFloorBba,
instinct())` — the net wraps only the **contested** books, so uncontested
`1♣-P-1♦-P-…` was already floored by deterministic `instinct()`, which reads the
alerted relay through projection and never invents diamonds. The misread the
argument targets was therefore *not happening*; the arm is really the held-back
constructive-floor A/B, scoped to the 1♣ subtree.

| `dutch-wj1c` − `dutch` | plain DD | perfect defense | fired |
| --- | --- | --- | --- |
| vul none | **−0.0119** ±0.0038 | **−0.0293** ±0.0046 | 1.95% |

Both cells lose well outside their CI, and PD loses 2.5× what plain does. The
mechanism, counted over all 4 689 divergent boards (not the worst-N tail):

| | `dutch` | `dutch-wj1c` |
| --- | --- | --- |
| mean final contract level | 2.908 | **3.676** (+0.768) |
| reached slam (≥6) | 76 | **570** (7.5×) |
| reached the five-level or higher | 99 | **862** (8.7×) |
| finished doubled | 5 | **45** (9×) |

Twice A/B B's level inflation and an order of magnitude more slams. In 40% of
divergences the WJ arm *strictly extends* an auction `dutch` had already passed
out, and the extra calls are not junk — the modal one is `3NT` (720 boards),
i.e. `instinct()` genuinely does pass out below cold games under the wide 1♣.
But the same net that finds those games also drives `4NT`/`5NT` into doubled
slams, and the slams cost more than the games earn.

**What this says about the architecture, and it is the durable part.** A learned
net in the **constructive** slot has no rail that tells it to stop. The
contested floors are safe partly because `forced(context)` hands railed
positions back to the deterministic ladder, and partly because a contested
auction has opponents bidding to end it. A constructive auction ends only when
someone passes, and `instinct()` carries an explicit settle floor (pass = play
the top bid) that the nets have no equivalent of. So "swap `instinct()` for a
net in the constructive book" is not a free upgrade anywhere, Dutch or american —
it needs a settle rail first. That reframes the still-open
`scripts/constructive-floor-ab.sh` arm: expect the same runaway from
`NeuralFloorBba` over american, and build the rail before spending the compute.

**The 720 missed games are the real lead.** They are a measured leak in a
*book*, not in a floor: `instinct()` passing out under the wide 1♣ because the
18–20 `1NT` and 21–23 `2♦!` relay continuations are unauthored. The fix is
Phase 2.2 authoring on both sides, which is what the iron rule prescribes
anyway — a book node with finite mass shadows the floor, so authoring removes
the question rather than re-litigating which net floors it.

`dutch()` is unchanged throughout: `NeuralFloorBba` floors every Dutch subtree,
1♦ included.

**Measured facts about BBA-WJ** (274k-board harvest via the surviving
`bba-wj-reference` binary, which records EPBot's own
`get_info_meaning_extended` disclosure beside each hand):

| BBA-WJ opening | What it actually is |
| --- | --- |
| `2♦` (n=9166) | `Multi`, **weak-only** — declared `H/S [0,6]`, pts `[4,10]`; observed HCP 1–10, always a 6+ major. No strong variant at all |
| `2♥`/`2♠` (n=5290) | `Polish two suiters`, **5-5 not 5-4** — the second suit is never four cards. `2♥` may hold 5+ spades (`S [0,13]`); `2♠` caps hearts (`H [0,3]`), since 5-5 majors always open `2♥` |
| `2♣` (n=9143) | a **minimum club hand**: declared `C [5,13]`, `H/S [0,4]`, pts `[11,14]` — traditional WJ, every 5-club hand with a 4-card major |

**Decision (jdh8, 2026-07-20): Phase 3 adopts BBA's two-level openings** — Multi
2♦ + the 5-5 Polish two-suiters, **replacing the spec's planned Muiderberg**
([dutch-spec.md](dutch-spec.md) still says Muiderberg; update that line when
Phase 3 authors the rows). Book and teacher then share the same rows, which
makes the two-level branch the cleanest floor-transfer in the system. Costs a
weak 5-4's opening, gains the weak 5-5 majors an opening Dutch does not have
today (1♠ needs 10+ and Rule of 20).

**Hand-containment, measured** (Dutch's opening table replayed over the same
harvest — exact, since `fuzzy_fifths` is off so `fifths(20.0..22.0)` is literally
HCP 20–21, and `balanced()` is 4333/4432/5332):

| Dutch opens | BBA-WJ agrees | Where the rest goes |
| --- | --- | --- |
| **1♦** (n=17868) | **89.0%** | 9.7% → WJ's 1♣, **all of it 18+** (Dutch's 1♦ is 11–23, WJ's caps ~17) · 1.3% Pass/2♣ |
| **1♣** (n=37754) | **73.0%** | **21.3% → WJ's 2♣** (the minimums above) · 5.4% → WJ's 1♦ |

So "Dutch 1♣ ⊆ Polish 1♣" is **false** — traditional WJ's 2♣ takes the club
minimums. The breach lands on 1♣, which stays an opt-in knob default-off; 1♦ is
the arm to ship, and its residual has no unexplained component. **Never route
2♣**: Dutch's is strong (21–23/24+), WJ's is an 11–14 minimum — same call,
opposite meanings, the one spot the WJ net would be catastrophically wrong.

### Phase 2.1 A/B result — LOSS, as expected for a half-built system

`scripts/dutch-ab.sh` (SEED_BASE 1784374548, sha 945c1ae bidding code plus
harness-only `--our-floor dutch` wiring, 204 800 bd/arm/vul vs the BBA
reference opponent). `dutch − american`:

| vul | plain DD IMPs/bd | PD IMPs/bd | fired | IMPs/fired (plain) |
| --- | --- | --- | --- | --- |
| none | **−0.0278** ±0.0055 | −0.0102 | 6.37% | −0.437 |
| both | **−0.0308** ±0.0074 | −0.0077 | 6.55% | −0.471 |

A clean plain-DD loss (~4σ, not noise) — **not shippable**, and expected: this
is the "half-built convention" bias from [measurement.md](measurement.md), not
a refutation of the wide-1♣ concept. Tracing the worst divergent boards, the
loss splits three ways, all on the roadmap:

1. **Balanced-only 1NT** (`dutch/openings.rs` `hcp(15..=17) & balanced()`) vs
   american's shipped **Wide6322** — 15–17 6322/5422 hands open a *minor* in
   Dutch, `1NT` in american. Recurs as `off: 1NT (X)(XX)` scoring while Dutch's
   minor partscore does not. A Phase-1 lever (flagged below), one-line to flip,
   the cheapest isolation to run next. **FLIPPED 2026-07-18** — Dutch's 1NT now
   reuses american's `notrump_shape(NotrumpShape::Wide6322)`, the A/B-validated
   american default; no fresh gate (the shape carries american's two-seed win).
2. **Half-built competitive continuations** after the wide `1♣` / `1♦` relay —
   contested auctions collapse into doubled Dutch contracts (`5♣X`, `3♠X`,
   `3♦X`) because responder's deeper calls fall to the american-tuned floor/
   reader. This is Phase 2.2 (deep continuations) + Phase 4 (reader) unbuilt.
3. **Strong-opening threshold + slam-continuation gaps** — a 19-count 5–5
   two-suiter opens `1♠` in Dutch but `2♣` in american (cleaner controlled
   auction to game); a slam is missed where american's continuation drives to
   `6♠`. Phase 3 (strong-2♣ tree) + Phase 4.

Nothing here says the wide `1♣` is wrong; it says the system is ~2 phases from
a fair fight. Next diagnostic: isolate factor 1 (Dutch-on-Wide6322 `1NT`) — it
is a within-Dutch A/B, cheap, and sizes how much of the −0.03 is the deliberate
notrump-shape choice vs. the genuinely unbuilt tree.

### Phase 2 notes — the wide-1♣ response structure

Spec tables (responder's calls, opener's relay rebids, the deep continuation
trees) live in **[dutch-spec.md](dutch-spec.md)**. Phase 2.1 authored the first
two nodes — `[1♣]` responses and `[1♣,1♦]` opener rebids; the `2NT!` 5-5-minor
rebid is dropped (unreachable in pons — 5-5 minors open 1♦). This section keeps
only the pons-specific encoding choices and the open items.

Encoding choices (each a small, faithful adaptation — validate in the A/B):

- **Relay constraint = `hcp(5..) | len(♣,..3)`** (constructive values, or too
  short in clubs to pass), sitting at weight 0.3 below every natural and above
  `Pass` (0.0). An OR-disjunction projects to the WALL → floors nothing → the
  alert cleanly suppresses the natural-diamond reading with no phantom suit.
- **Weak jump = exactly six, preempt = seven-plus** (`2M` `6..=6`, `3M` `7..`)
  so `2♥♠` and `3♥♠` partition by length rather than overlapping on 6+.
- **`2NT`/`3NT` deduped at 11** — `2NT` 10–11 invite keeps the 11, `3NT`
  encoded 12–15 to-play (doc lists 3NT as 11–15).
- **Opener's `2♣` spans 11–20**, not 11–17, so an 18–20 unbalanced no-4M no-6♣
  five-club hand has a rebid instead of falling to the `Pass` catch-all; opener
  resolves the exact band on the next round (Phase 2.2).
- **1M responses use up-the-line** (bid the cheaper 4-card major first), pure
  DSL, no dependency on american's `spades_first`/`hearts_first`. Longer-major
  discipline (reader-preferred) is a deferred refinement.

Alerts: only the genuinely artificial calls trip the invariant — `1♦!` relay,
opener's `2♦!`/`2NT!`. The natural-but-meaning-inverted calls (`2♦` GF, `2♣`
INV+, weak jumps, minor invites, major preempts) are *also* alerted so
projection self-decodes them (american's hardcoded reader would otherwise read
`2♦` as a weak jump and `2♣` as an inverted raise). Balanced/notrump naturals
(`1M`, `1NT`, `2NT`, `3NT`) are left unalerted — american's read is close enough.

### Phase 2.2 increment 1 — responder's second call over opener's minimum

Authored `1♣-1♦-1M` (both majors) and `1♣-1♦-2♣` — where the bulk of relay
auctions land (opener minimum, 11–17). The rare 18–20 `1NT` and 21–23 `2♦!`
continuations are **deferred**: opener's strength there self-discloses to the
floor through rule projection (the calls are alerted), so the floor bids toward
the right game without an authored node — a refinement, not a hole. Measure the
common case first, then decide.

Encoding choices (jdh8-confirmed bridge, 2026-07-19):

- **Reverse Flannery** (`2M!`, and `2♥!` over opener's `2♣`) is gated **exactly**
  on `5=♠ & 4–5♥ & 7–9` — the two-suiter deliberately routed through the relay to
  dodge the `1♣-1♠-2♣` rebid squeeze (and to keep a direct `1♣-1♠-2♣-2♥` INV+).
  An ordinary invitational major-raiser never arrives (it raised/bid on round
  one), so no natural-raise row exists; such hands fall to the floor.
- **`2OM!` = both minors** (5+/4+, 9–11 invite). A natural major is impossible
  here (real four-card majors bid up the line on round one), and if a Reverse
  Flannery fit mattered it was already found — so the "other major" is free to
  repurpose. Encoded 4-4 minors with at least one five-bagger.
- **Club support inverted** over `2♣`: the artificial `2♠!` is the *invitational*
  raise (9–11), the natural `3♣` the *minimum* one (7–9).
- **`2NT` = 16+ balanced** is alerted (a meaning inversion vs american's invite)
  so projection discloses the slam-going strength and rightsides the notrump.

**Open questions / deferrals:**

- **✓ Phase-1 spec discrepancy resolved (2026-07-18):** the online `1D.md`
  argument for opening 1♦ on (xx)45 [4♦5♣] is **stale** — jdh8 is no longer
  following it. In pons, **(xx)45 opens 1♣** (the locked `1♦ = 5+♦ | 4441`
  stands) for simplicity and as an experiment. Consequence: the `2NT!` 5-5-minor
  rebid is unreachable (5-5 minors open 1♦) and was **dropped**.
- **Deep continuations — increment 1 authored (2026-07-19):** `1♣-1♦-1M` and
  `1♣-1♦-2♣` (see the increment-1 section above). Still deferred: `1♣-1♦-1NT`
  (18–20 transfer structure, reuses the 1NT machinery) and `1♣-1♦-2♦` (21–23
  transfer structure) — rare, and their strength self-discloses to the floor via
  projection. Opener's third call (after responder's authored second call) still
  falls to the floor — a soft misread, measured not fixed blind.
- **✓ `[1♣,2♣]` / `[1♣,2♦]` overwritten + responder side (increment 2,
  2026-07-19)** — see the increment-2 section below. The opener-only first cut
  left responder to the floor and **measured a loss** (git-arms A/B: the floor
  dropped the game force and blasted slam blind, `−0.0029/bd plain, −1.38
  IMPs/fired`); the **responder side authored** (honour the force, cap at the
  right game) turned it into a **win** — re-A/B `+0.0012/bd plain none,
  +0.0021/bd plain both, PD positive throughout, +0.53…+0.86 IMPs/fired` (the
  responder side alone worth `+1.97…+3.20 IMPs/fired`). Slam beyond game deferred
  to a later RKCB reuse.

### Phase 2.2 increment 2 — opener's rebid after responder's natural 2♣ / 2♦

Overwrites the two continuations american built for its own (different) meanings:
american routes `1♣-2♣` to an **inverted club raise** (forcing) and `1♣-2♦` to a
**weak jump shift** (0–6). Under Dutch, `2♣` is invite+ (5+♣) and `2♦` is
game-forcing (5+♦), so both american nodes misread responder — the GF is treated
as weak (drops games/slams), the invite as forcing (opener can't stop). Authored
`opener_rebids_after_two_diamonds` / `opener_rebids_after_two_clubs`
(`dutch/responses.rs`).

Structure (jdh8-confirmed bridge, 2026-07-19). **Key fact:** after 1♣ (denies a
5-card major — those open 1M) and responder's `2♣`/`2♦` (deny a 4-card major), **no
major fit can exist**, so both auctions are the pure inverted-minors world
(minor-fit / notrump / slam). Opener borrows american's `after_inv_raise` ladder:

- **`2♦` (GF), forcing:** `3♦` = 4-card diamond support (a known nine-card fit —
  and the wide 1♣ hosts most 4-diamond hands, the Dutch enrichment); `3♣` = a real
  5+ club second suit; `3NT` = balanced extras, both majors stopped; `2♥`/`2♠` = a
  single major stopper up the line toward 3NT (`!stopper` in the other, so a
  both-stopped hand takes notrump); `2NT` = the notrump catch-all. No Pass.
- **`2♣` (invite+), non-forcing:** `3NT` = accept (balanced max stopped, or a 17+
  maximum forcing over the 11+ invite = 28+); `3♣` = decline with club support
  (capped ≤16 so a maximum never leaves it in); `2NT` = the balanced-minimum
  decline / catch-all. The help-suit game try (`2♥`/`2♠`) is **dropped** — the
  artificial try needs its own authored responder read; a cheap accept/decline
  lands the same games.

**Responder's continuation: authored (the redo).** The first cut left responder to
the floor on a shallow probe ("drives GF hands to 3NT, passes a dead minimum")
and **it measured a loss.** A git-arms A/B (dutch@inc.2 vs dutch@inc.1, 204 800
bd/arm, SEED_BASE 1784400427) scored `−0.0029/bd plain, −0.0027/bd PD, −1.38
IMPs/fired` (0.21% fired), and the worst boards diagnosed why: the floor **dropped
the game force** — passing opener's forcing `3♣` over `2♦` (`1♣-2♦-3♣` passed out,
a making 6NT missed) — and **blasted slam blind** over opener's `3NT`/stopper-shows
(`2♦-2♠-6NT`, `2♣-3NT-4NT-5♥X`). The probe was too shallow: it checked opener's
placement, not responder's read of the artificial rebids. Lesson re-paid: never
ship on analysis alone; complete both sides before measuring.

`responder_after_two_diamonds` / `responder_after_two_clubs` now author the
responder side to honour the force and cap at the right game:

- **After `2♦` (GF):** `3NT` on every descriptive rebid (`3♦`/`3♣`/`2♥`/`2♠`/`2NT`
  → name the game, never pass the force); Pass over opener's own `3NT` (15+
  balanced, to play).
- **After `2♣` (invite+):** Pass the `3NT` accept; over a non-forcing decline
  (`3♣`/`2NT`) drive `3NT` with the game-forcing end (12+), else pass the invite.

**Re-A/B: the fix wins.** Rebuilt git-arms (SEED_BASE 1784402356, 204 800
bd/arm/vul), complete inc.2 (opener+responder) vs pre-inc.2: `+0.0012/bd plain,
+0.0007/bd PD` (none); `+0.0021/bd plain, +0.0015/bd PD` (both); `+0.53…+0.86
IMPs/fired`, 0.23% fired — a **plain-DD win, both vuls** (3 of 4 CIs exclude
zero), flipping the opener-only loss. Isolating the responder side (head2 vs the
opener-only head1) it is worth `+1.97…+3.20 IMPs/fired` on its own — the exact
mechanism the worst-board trace predicted. (This run also fixed a harness bug in
the *first* A/B, whose `-v both` was dropped, so none/both scored identically;
the redo's vul split is real.)

Slam beyond game is **deferred**: the A/B's dominant loss was blind slam *blasts*,
not missed keycard slams, so the disciplined first cut lands the game cleanly. The
`3♦` diamond-fit branch is the home for a later RKCB reuse (widening
`american::slam::install_rkcb` past `pub(super)`), pending a re-A/B that shows the
game cap leaking slams. Everything responder bids here is natural (notrump / pass,
projecting no suit), so no alert is carried and `dutch_artificial_calls_are_alerted`
passes untouched. The `responder_continues_after_opener_rebid` test locks in the
fix.

### Phase 1 notes

`bare_dutch()` takes a full `bare_american()` pair and **overwrites only the
opening node** (`Trie::insert_arc` replaces the classifier at the opening key)
with `dutch::openings::dutch_openings()`; every american continuation is reused
verbatim. Widening was minimal: `insert_uncontested` and `with_instinct_floor`
in `american.rs` → `pub(in crate::bidding)`. Openings live in
`src/bidding/dutch/openings.rs`.

Design choices to validate in the A/B (each defensible from the spec, cheap to
flip):

- **Rule of 20 is a hard gate** (`hcp(band) & rule_of_20()`), so a flat
  sub-R20 minimum passes out — e.g. a 4-3-3-3 twelve-count (12+4+3 = 19).
- **1NT is american's Wide6322 15–17** (balanced, or a 5422/6322 with a long
  minor) — flipped from the Phase-1 balanced-only choice on 2026-07-18, reusing
  american's `notrump_shape(NotrumpShape::Wide6322)`. The shape is already
  A/B-validated on american (two-seed win, +0.004…0.006 IMPs/bd plain,
  sd-confirmed), so Dutch inherits that evidence by reusing the identical gate;
  this also closes factor 1 of the Phase 2.1 LOSS post-mortem.
- **No 3rd/4th-seat light (9-count) major openers** — american has them; the
  Dutch spec caps majors at a Rule-of-20 10-count. Watch the passed-out-seat
  boards.
- **Strong balanced 20–21 still opens 2NT** (Phase 1 placeholder); only 22–23
  balanced reaches the wide 1♣ until Phase 3 turns 2NT into UNT.

Guards: `dutch_artificial_calls_are_alerted` (inference.rs) walks the Dutch
constructive book; `dutch::tests::opening_partition` pins the six load-bearing
opening cases (incl. the wide 1♣ hosting a 23-count). `dutch()` is re-exported
as `pons::dutch`.

### Phase 0 notes

Scaffolded `dutch()` as a sibling factory (re-exported `pons::dutch`). The
reuse + override seam opened in Phase 1; widening american internals is done
only as each part diverges.
