# 21GF ledger — pons 2/1 vs BBA's `21GF.bbsa`

Target spec for "author pons's 2/1 about as deep as BBA". One row per relevant
`vendor/bba/21GF.bbsa` toggle. The plan that governs this work:
`~/.claude/plans/author-the-2-1-bidding-replicated-mochi.md`.

**Status legend:** `shipped` (authored + tested) · `partial` (some of it) ·
`floor` (handled by `instinct()` only, not authored) · `gap` (absent) ·
`conflict` (pons plays it differently) · `override` (user chose ≠ 21GF) ·
`out` (out of scope).

**Campaign metric:** IMPs/board vs BBA 2/1, plain DD (perfect defense beside).
History: **−2.59** (2000 bd, vul none, S.1) → **−1.997** (4000 bd) → first
**seeded, decomposed anchor** (2026-07-06, sha `62cf5c5`,
`SEED_BASE=1783375064`, 204.8k bd via `scripts/anchor.sh`, replay-verified
**100%**): **vul none −1.675 / vul both −2.310**, pooled **−1.99 plain /
−2.40 PD**. Vul-none improved −2.59→−1.67 across the M6.x + competitive-book
streak; the pooled figure newly folds in the harder both-vul arm.
Re-anchored `4afc985` (2026-07-08, 409.6k bd, same seed): pooled **−1.89 plain /
−2.11 PD** after the bucket-#1 takeout-discipline and bucket-#3 opener-ladder
ships (progress log below).  Latest re-anchor `5f9d6c2` (2026-07-09, same seed):
pooled **−1.758 plain / −1.864 PD** — the five-card-major takeout discipline
(overcall an unbid 5-card major instead of doubling) folded in; def-r1 −127k
plain / −147k PD.  Biggest un-worked prize now: Competitive `fallback@1/@2`
round-1 (−78k plain, two-sided Pillar-D sub-campaign).

**What the first anchor overturns (read before picking work):** the gap is
**book-dominated, not floor-dominated** — `book` −248k IMPs vs the entire
`instinct()` floor ~−160k spread over dozens of rules (largest single floor
rule is `floor#3`, the opaque *pass*, at −38k). By phase it is **Defensive
−171k > Constructive −155k > Competitive −82k** — the "gap concentrates in
competitive auctions" anecdote is **wrong**. Ranked buckets: (1)
**Defensive / book / round-1 −98k** — our overcall / takeout-double /
two-suiter structure vs their opening (PD −136k, i.e. *worse* under good
defense → real overreach, not a doubling artifact; worst boards are our own
3♥x / 4♣x / 2♥x); (2) **Constructive / book / opening −68k**; (3)
**Constructive / book / round-2 −40k** and (4) **round-1 −34k** (splinter /
raise structure missing slams). Balancing is only −11k (2nd-smallest family)
— **deprioritized**, contra the going-in guess. Full report (committed):
`ab-results/anchor/2026-07-06-62cf5c5/report.md`.

**Progress (2026-07-07, re-anchored `57b933b`, same seed → paired):** bucket (1)
traced to the **12+ takeout double weight-shadowing the two-level overcall** —
an off-shape one-suiter with a suit *lower* than theirs doubled (1.3 > 1.0),
got pulled to the 3-level, and landed doubled. The off-shape-X support gate +
2-level overcall discipline (shipped default-on; see the Defensive table and
CHANGELOG `Fixed`) shrank it **−98478 → −67707 plain (−31 %), −136494 → −91569
PD (−33 %)**, with 11.7k fewer boards firing — it now ties
`Constructive / book / opening` (−67689) for #1. Pooled gap **−1.9925 → −1.9778
plain, −2.396 → −2.347 PD** (both arms improved, PD-heavy as expected). Report:
`ab-results/anchor/2026-07-07-57b933b/report.md`. The passed-hand 2-level
overcall carve-out (floor 11→9 when a passed hand; `set_passed_hand_overcall`)
**measured a DD/PD wash** [1783407558, ~0.1 % fired, all CIs straddle 0] — a
lead-directing idea DD/PD are blind to, so it stays **opt-in, an sd-lead
re-measure candidate**. Next: the residual Defensive/round-1 (two-suiter
structure) vs Constructive/opening (now tied for #1).

**Progress (2026-07-08, anchor `5f16e68` re-decompose of `Defensive / book /
round-1`, now −142733 plain / −188939 PD):** decomposed the residual by our
call. The literal **two-suiter structure is only 5.2 %** of the bucket (−7351
plain / −7685 PD, 1897 calls) — small, genuine (plain≈PD), an 8-point Michaels/
Unusual/Leaping overreach concentrated at both-vul; parked as a later opt-in.
The mass is elsewhere: **Pass −23897** (PD *better* → we underbid; obstruction
wall, DD-blind, skip) and **takeout X −22129 plain / −35207 PD** (PD worse →
real overreach). Split the vanilla-X (`12+ HCP` rule, −14605/−22559): **4333
−3507** (bucket #5 kills it, post-anchor), **balanced 4432/5332 ≤13 −3508 /
−6426** (residual), flat-14+ −2146 (has values, leave), unbalanced −5444
(correct doubles, DD-blind, leave). Refinement over bucket #5's thesis: every
4432 that *doubles* has its doubleton in **their** suit (textbook takeout shape,
not a shape-defect — over 1♦ the same 4-4-majors hand already passes for lack of
club support); the 5332 doublers have a biddable 5-suit better overcalled. So
the claim is "a **minimum 12–13 balanced** takeout double overbids vs BBA," not
"no ruffing value" — PD-supported but a real measurement question. A pooled
4432+5332 suppression measured a plain+PD win both vuls (SEED_BASE 1783449013,
409.6k bd/arm/vul: plain +0.0284/+0.0698, PD +0.1082/+0.1449, all CIs>0) — but
**theory review (jdh8) split the two shapes**, so the pooled number does not
ship:

- **5332 → always overcall the five-card suit** (theoretically settled, not a
  measurement call): a 5332 holds *no* 4-card suit, hence no 4-card major, so
  the takeout double cannot do its job (find a 4-4 fit). Bid the suit, minor
  included. `set_suppress_5332_takeout`; A/B confirmatory only.
- **4432 → keep the double** (theory-wrong to suppress): a 4432 short in the
  opponents' suit is the *textbook* takeout — two 4-card suits (often a major),
  the double genuinely finds 4-4 fits. A measured 4432 "win" most likely fingers
  **partner's advance**, not the double → trace the advance rather than suppress.
  `set_suppress_4432_takeout`, held **default off** pending diagnosis.

3-arm split (base / s5332 / s4432, SEED_BASE 1783451581): **both halves win
plain+PD both vuls** — s5332 +0.0191/+0.0401 plain, +0.0601/+0.0773 PD; s4432
+0.0086/+0.0282 plain, +0.0448/+0.0638 PD (all CIs>0). The 4432 win *contradicts*
the textbook-double theory, so decomposed the 4432 loss by which two suits are
the 4-carders (anchor `5f16e68`): **4-4 majors −80 plain (−0.96/div, ~wash) —
the genuine textbook double is fine**; the loss lives in major+minor (−928) and
4-4 minors (−260), which have no real 4-4-major shot and overreach fishing for a
minor. So theory and measurement reconcile: it's not "suppress the textbook
double," it's "the ≤1-major 4432 is the overbid."

**Opener-split (2026-07-08, `ab-results/balanced-takeout-opener/`, SEED_BASE
1783454269, 12.8k bd/arm/vul):** the 4432 anchor-split was by which suit *we*
inferred the opponents held; this re-measures against the **true opener** with
two per-opener knobs (`set_suppress_4432_vs_major` / `_vs_minor`). Result:

- **vs a minor opening** — plain **wash both vuls** (+0.0030 NV [±0.0065] /
  +0.0063 vul [±0.0078], CIs cross 0), PD-only (+0.0130 / +0.0163). The double
  is textbook and gains nothing on plain — **keep it**.
- **vs a major opening** — NV plain wash (+0.0077 [±0.0102]), vul plain
  +0.0269 [±0.0128] (CI>0). But tracing the worst boards, the "win" is *not* the
  takeout double: the worst recurring board is North doubling **2♥ deep in a
  competitive auction** (`1♣(us) 1♥ P P 2♣ 2♥ X`) — the general instinct-floor
  double (`instinct.rs`, `their_live_bid_at_most(3)`), *not* a takeout of their
  opening — and the real leak is our floor's *response* (`X → partner leaps to a
  bad 3NT` instead of pass-and-reopen-to-4♠). Iron-rule "over-broad trigger +
  unauthored continuation," both firing. Suppressing the double papers over a
  floor-continuation bug.

**Shipping**: 5332 default-on (`set_suppress_5332_takeout`; theory-settled +
measured, +0.0191/+0.0401 plain / +0.0601/+0.0773 PD, 409.6k bd/arm/vul). 4432
**not shipped** — the double is sound; the measured over-a-major gain is a
general competitive-double trigger + floor-response leak, not the takeout X.
Knobs `set_suppress_4432_vs_major` / `_vs_minor` stay opt-in (default off,
system byte-identical). The floor's `X → bad-3NT` competitive-double response is
a diffuse instinct-floor continuation candidate for its own campaign, not a
suppression.

**Bucket #2 `Constructive / book / opening` → Rule-of-20 (2026-07-07,
`set_rule_of_20` default-on, `scripts/rule-of-20-ab.sh`, SEED_BASE 1783410574):**
a 1811-board classification (`scratchpad`, plain −3913 ≈ PD −3909, not a
doubling artifact) overturned the "doubled artificial continuation" hypothesis
(the 3 worst boards): **61 % of the loss is sound hands we pass and BBA opens**
(79 % eleven-counts, 46 % Rule-of-20), 33 % strain/level (incl. under-opening
strong hands `1♠`-vs-`2♣`), 6 % weak-2 discipline. Fix = open Rule-of-20 10-11
counts (raw HCP + two longest ≥ 20), one of a suit. **A/B: plain +0.0061 NV /
+0.0087 vul (CI>0), pd −0.0056 / −0.0034 (the doubling-artifact flag), sd-lead
+0.0096 / +0.0135 (CI>0, > plain).** The new `ab-dump-sd` third bracket (blind
opening lead, the realistic middle) rehabilitates the plain win the pd
perfect-doubler bracket erases → **shipped default-on**. The opening inference
floor drops 12→10 to stay sound. Residual bucket-2 levers (strain/weak-2)
un-worked.

**Bucket #4 `Constructive / book / round-1` → doubled-splinter systems-on
(2026-07-07, `set_splinter_doubled` default-on, `scripts/splinter-doubled-ab.sh`,
SEED_BASE 1783439089):** all three of the bucket's worst boards (−22/−23 IMPs)
were **splinters doubled and passed out** — a splinter is game-forcing, but the
double reroutes opener from the constructive book into the competitive book,
where — unauthored — it fell to the floor's *pass* (a four-ace 16-count passing
`4♣x` while the field bids `7♠`). A `FirstIs(Double)` rebase keyed at
`[1M, P, splinter]` strips the double off the whole subtree, so opener and
responder's keycard answers resolve on the undisturbed splinter tree. **A/B:
plain +0.0059 NV / +0.0079 vul (CIs [+0.0047,+0.0071]/[+0.0063,+0.0095]), PD
+0.0059 / +0.0079 (plain ≈ PD — removing our own doubled contracts, no
artifact), +15.4/+17.6 IMPs/fired at 0.04 % fired.** Rare fire, decisive per
board → **shipped default-on**. Known tail: a *second* double (of the keycard
response) still passes out (1 board in 79), the standard rebase-tail limitation.

**Re-anchor (2026-07-08, `4afc985`, same seed → paired, 409.6k bd):** the
bucket-#1 takeout-discipline ships (5332, flat-4333) landed on the metric —
pooled **−1.99 → −1.89 plain, −2.36 → −2.11 PD** (replay-verified 100 %,
`ab-results/anchor/2026-07-08-4afc985/`). Ranking held: (1) Defensive/book/round-1
−2.29/div (was −2.40, PD −189k→−168k), (2) Constructive/book/opening −2.12, (3)
**Constructive/book/round-2 −2.24 (unchanged, never traced)**, (4)
Constructive/book/round-1 −2.49.

**Bucket #3 `Constructive / book / round-2` → opener extras ladder (2026-07-08,
`set_opener_extras_ladder` default-on, `scripts/opener-extras-ladder-ab.sh`,
SEED_BASE 1783544590):** traced the −98k (plain ≈ PD, underbid-heavy: `other`
−86k / missed-game −38k / missed-slam −23k) to opener's minimum natural suit
rebid with **no upper strength bound** — the biggest sub-lever `5+ ♦` (−20k) is
2578/2636 a flat `2♦` on hands up to grand-slam strength (`T64.AJ86.AKQ95.A`
rebids `2♦`, misses a grand). Fix = a strength ladder above the minimum in the
two **minor-opening** rebid nodes: jump-rebid (6+/16+), reverse (5-4/17+,
alerted), jump-shift (5-4/18+, alerted). **A/B: plain +0.0203 NV / +0.0332 vul,
PD +0.0181 / +0.0297 (all CIs>0, plain ≥ PD, ~0.7 % fired, +3–4.5 IMPs/fired)** —
plain-DD win both vuls → **shipped default-on**. The two major-opening nodes
(Meckstroth `3m` collision) and the `5+ ♣`/`6+ ♠`/`6+ ♥` residual are the
follow-up.

**Bucket #3 residual → opener major jump-rebid (2026-07-08,
`set_opener_major_jump_rebid` default-on, `scripts/opener-major-jump-rebid-ab.sh`,
SEED_BASE 1783549337):** re-anchor `c864bad` left the `6+ ♥` (−3.8k) / `6+ ♠`
(−5.7k) major single-suiter underbids (`3♥ → 4♥`, `2♥ → 3♥`, `3♠ → 4♠`, plain ≈
PD) in the major-opening rebid nodes. Added the jump-rebid `3M` (6+/16+, natural)
scoped to opener's own suit — no reverse/jump-shift, so no Meckstroth `3m`
collision. **The bare rung LOST** (plain −0.0051 NV / −0.0091 vul, −1.2…−1.9
IMPs/fired): responder passed the invitational `3M` and stranded below game
(`1♥-1NT-3♥` passed while the slow `2♥-2NT-3NT-4♥` reached game). Authored
responder's continuation (`responder_after_major_jump_rebid`: `4M` on an 8-card
fit, `3NT` no fit, pass a minimum) → re-measure (reusing the byte-identical OFF
arm): **plain +0.0059 NV / +0.0125 vul, PD +0.0046 / +0.0104 (all CIs>0, ~0.35 %
fired, +1.8…+3.4 IMPs/fired)** — plain-DD win both vuls → **shipped default-on**.
The iron-rule lesson (author both sides): a jump whose saved space is never
spent measures as a loss even when the idea wins.

**Scoring basis:** A/B duplicate results are scored **plain double-dummy**
(`scoring::ns_score_contract`, the contract's *actual* auction penalty) as of commit
`a6f2206`. `par` and the `bidding::ev` call-evaluator keep perfect defense
(`ns_score_bid`). Rows carry a **`PD`** tag only where it is still load-bearing.

**A/B notation.** A result reads `±a/±b unit` — IMPs at {none-vul / both-vul} vs the
baseline named in the sentence. `unit` is `board` (per deal) or `div` (per divergent
deal); `[N, p%]` appends sample size and divergence rate when known. Scorer is **plain
double-dummy** unless a figure is tagged **`PD`** (the older perfect-defense scorer,
which auto-doubles failing contracts; kept only where it is the current best or the
decision turns on the PD-vs-plain-DD contrast). The trailing `| <hash>` closes each row.

**Caveat:** plain DD *under-punishes* a failing competitive overbid (PD doubled it), so
a plain-DD figure that flips positive on an overbid is **suspect** — it is recorded, but
is not grounds to change a ship decision (cf. DoubleStyle, Jordan/Truscott).

## Locked decisions (from planning dialogue)

1. Mirror `21GF.bbsa` ON toggles; author by IMP-priority; user vetoes per item.
2. Shipped conflicts: keep pons's, A/B head-to-head; switch only if BBA wins.
3. Keycard: **keep 1430** (not 0314).
4. Jump shifts: **keep weak**; fit-showing only as a competition gadget later.
5. Beyond-21GF additions in scope: **plain-4NT minor keycard**, **Garbage Stayman**.
6. Out of scope: Multi 2♦, Benjamin/French 2D, Gambling, Gazzilli, Ghestem,
   Raptor 1NT, plain Landy, Polish/Wilkosz, Kickback/Crosswood.
7. Overrides: **Woolsey** over their 1NT (≠ Cappelletti); **Rubensohl /
   transfer-Lebensohl SEF-2018+** over interference on our 1NT (≠ Lebensohl).
8. **Kickback** postponed to its own session (spec in the plan).

## Constructive — 1NT/2NT responses

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 9 | 1N-2♠ transfer to clubs | shipped (two-way 2♠) | keep | — | — |
| 12 | 1N-3♣ transfer to ♦ | conflict (Puppet) | keep Puppet, A/B | — | — |
| 14 | 1N-3♦ majors | shipped | keep | — | — |
| 18 | 1N-3M splinter | verify | — | — | — |
| 88 | Minor transfers after 2NT | shipped | keep | — | — |
| 98 | Quantitative 4NT | shipped | keep | — | — |
| 109 | Smolen | shipped | keep | — | — |
| 119 | Texas | conflict (SAT 4♣/4♦) | keep SAT, A/B | — | — |
| 55 | Extended acceptance after NT | partial/verify | — | — | — |
| 115 | Super acceptance after NT | partial/verify | — | — | — |
| — | **Garbage Stayman** | gap (in scope) | add (Batch 3) | — | — |

## Constructive — suit raises & responses

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 7 | 1M-3M inviting (limit raise) | shipped | keep | — | — |
| 70 | Jacoby 2NT | shipped | keep | — | — |
| 113 | Splinter | shipped | keep | — | — |
| 68 | Inverted minors | shipped | keep | — | — |
| 37 | Bergen | conflict (limit+J2NT) | A/B (Batch 3) | — | — |
| 89 | Mixed raise | gap/conflict | A/B w/ Bergen | — | — |
| 56 | Fit showing jumps | conflict (weak JS) | keep weak; comp later | — | — |
| 116 | Support 1NT | verify | — | — | — |

## Constructive — opener rebids / checkback

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 57 | Forcing 1NT | shipped | keep | — | — |
| 125 | Two-Way NMF / XYZ | **shipped** (`set_xyz`, default on, with `set_up_the_line`) | keep; 2NT→3♣ variant unexplored | plain +0.038/+0.056 per bd NV/vul, PD +0.029/+0.041 (`ab-minor-continuations`) | — |
| 58 | Fourth suit forcing | gap (floored) | add (Batch 3) | — | — |
| 124 | Two-way game tries | gap | add (Batch 3) | — | — |
| 52 | Drury | gap | add (Batch 3) | — | — |
| — | Meckstroth adjunct (pons-only) | shipped | keep (complementary to XYZ) | — | — |

## Slam

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 39/40 | Blackwood 0314 / 1430 | shipped 1430 | **keep 1430** | — | — |
| 75 | King ask by 5NT | shipped (majors) | keep | — | — |
| 35 | 5NT pick a slam | verify | — | — | — |
| 64 | Gerber | gap | add (Batch 2) | — | — |
| 53 | Exclusion | gap | add (Batch 2) | — | — |
| 51 | DOPI | gap | add (Batch 2) | — | — |
| 103 | ROPI | gap | add (Batch 2) | — | — |
| 42 | BROMAD | gap (confirm meaning) | Batch 2 | — | — |
| — | **Plain-4NT minor keycard** | shipped (small slam) | keep; grand-in-minor deferred | **vs floor: +5.41/+7.05 div** [`PD` 5611eac, 10M, 202 div] — HOLDS. Plain DD not re-run; constructive (reaches *making* slams), so the `PD` figure is the conservative bound. | 99da1b3 |

## Competitive — our opening contested

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 80 | Lebensohl after 1NT | **shipped** | **`Transfer`** default = Cohen + `(2♦)` `3♣`-Stayman/Smolen/Leaping-Michaels (folded in); `Plain` opt-in (true `Rubensohl` removed 2026-06-20) | **`Transfer` vs floor: +0.080/+0.075 board, +0.789/+0.738 div** [a6f2206, 20393 div] — positive but **suspect** (obstruction overbids no longer auto-doubled; see caveat). `Transfer` stays default on its constructive merit vs `Plain`, not this number. | bfe5e59 (plain), bee9204 (transfer), e234f99, 63af4de, 2a32a89, 6e8694e |
| 105 | Rubensohl after 1m | floor (Rubens advances) | upgrade (Batch 1) | — | — |
| 100 | Responsive double | takeout shipped (toggle); overcall-ext opt-in Off | keep both as-is | `responsive-ab`, 200k/cell vs floor. **Takeout: −0.175/−0.500 div** [a6f2206, ~0.1% div] → stays shipped (drag near-nil + DD-blind obstruction). **Overcall-ext: +0.648/−0.340 div** [a6f2206, ~0.4% div] → stays off (sign-mixed, suspect under the under-punishment caveat). Behind `set_responsive_takeout` (default on) / `set_responsive_overcall` (default off); defaults byte-identical. | (toggles + `responsive-ab`) |
| 83 | Maximal doubles | gap | add (Batch 1) | — | — |
| 71 | Jordan/Truscott 2NT | tried — DD-negative | **keep floor** (don't ship) | **vs floor: −1.0/−1.5 div** [`PD`, jordan-ab 500k] (2NT-only half −4.2/−4.4) — reverted, obstruction is DD-blind. | reverted |
| 117 | Support double/redouble | shipped | keep | — | — |
| 28/30 | 1X-(Y)-2Z forcing/weak | partial | verify | — | — |
| 122 | Transfers if RHO bids clubs | gap | add (Batch 1) | — | — |

**Transfer Lebensohl (80) — Rubensohl take 2, shipped as default.** The first
Rubensohl attempt lost (−1.68/div) by stranding game hands in partscores. Larry
Cohen's *Transfer Lebensohl* fixes that: after `1NT–(2X)` the 3-level bids are
transfers up the line *through* the adverse suit (over `(2♥)`, `3♦` shows
spades), the cue is Stayman, and a transfer to a suit above theirs is INV+ so
opener is **driven to game** (`4M` with a fit, else `3NT`) — the anti-stranding
rule the user specified. Weak hands keep the plain outlets (natural 2-level,
`2NT` relay, penalty double). A/B (`lebensohl-ab`, `--ns transfer`, 200k
filtered/cell): **`Transfer` vs `Plain`: +0.051/+0.084 board, +0.989/+1.624 div**
[a6f2206] — reversing v1's loss (the convention-vs-convention choice is
basis-independent, as expected). Selected by
`LebensohlStyle` (`set_lebensohl_style`); `Transfer` is the default, `Plain` kept
for the A/B and as a fallback. Unlike the preemptive conventions below, the win
is mostly *constructive* (reaching the right game / strain), which the
DD / perfect-defense measure can see; the right-siding (strong `1NT` hand
declares) is invisible on top, so the table value is higher still.

**Naming + the TransferSmolen v1 experiment (80, follow-up — tried & reverted; superseded by v2 below).**
*Rubensohl* proper makes `2NT` an artificial **club** transfer; what ships keeps
the weak `2NT` **relay**, so it is *Transfer Lebensohl* (Cohen). A **TransferSmolen**
hybrid — Cohen over `(2♠)` but the *standard low-Stayman* structure over `(2♦)`/`(2♥)`
(the bid into their suit is Stayman, `3♣`/`3♦`, freeing a Smolen continuation) — was
authored and A/B'd vs `Transfer`. After loosening a too-tight Stayman gate (fire on
one 4-card major) the re-measure was **−1.31/−1.76 div** [`PD`, 300k] — a clear loss.
Standard low-Stayman
reaches DD-worse contracts than Cohen's cue=Stayman (e.g. a 5-5 hand routes through
Stayman→denial→`3NT`, missing the 5-3 major game Cohen's transfer-*through* finds), and
Smolen's right-siding is DD-blind. **Reverted.** `lebensohl-ab` kept a cheap
`--filter-dh` shape pre-filter (concentrates `1NT–(2♦/2♥)` boards ~10× so DD
lands on boards that can diverge) + a worst-board auction diagnostic.

**TransferSmolen v2 (80, follow-up — shipped, later folded into `Transfer`).** The narrowed
retry the user specified *wins*. It keeps Cohen untouched over `(2♥)`/`(2♠)`/`(2♣)`
and changes only the `(2♦)` branch, where `3♣` sits free below the `3♦` cue: `3♣`
becomes game-forcing Stayman (opener answers `3♥`/`3♠`, or `3♦` to deny — leaving
room for responder's Smolen `3♥`/`3♠`, which shows the 5-4), and the 3-level
transfers shift down to direct Jacoby (`3♦`→♥, `3♥`→♠, `3♠`→♣). The `3♠`→♣ leg is a
*forced* game-force — its completion is `4♣`, so `3♣` can never be the contract.
Two Leaping Michaels jumps are added: `4♦` = both majors 5-5, `4♣` = clubs + a 5+
major (classic shapes from `defense.rs`, but only `points(10..)` — partner opened a
15-17 `1NT`, so the 14+ a silent partner needs drops to ≈8 HCP after the 5-5
distribution upgrade). Key authoring subtlety: a 5-4 GF major hand fits both `3♣`
Stayman and a Jacoby transfer, so Stayman is gated to *exactly* a 4-card major and
weighted above the transfers — otherwise the hand would transfer and Smolen could
never fire. A/B (`lebensohl-ab --ns transfersmolen --ew transfer`, 200k filtered/cell):
**vs `Transfer`: +0.020/+0.024 board, +2.286/+2.822 div** [`PD`] — a clean win and a
full reversal of v1's −1.31/−1.76. Why it wins where v1 lost: v1's *standard low-Stayman* reached DD-worse contracts
and leaned on DD-blind right-siding; v2 keeps Cohen's transfer-through value over
the majors, *adds* genuine fit-finding the measure can see (5-3 major games via
Stayman+Smolen, 5-5 major games via Leaping Michaels), and only adds nodes over the
`(2♦)` Cohen base. Promoted to the `set_lebensohl_style` default — and later **folded into `Transfer`**
(the separate `TransferSmolen` name dropped once the package also won after a takeout
double; see the after-double update below), so the default is now plain `Transfer`
= Cohen + this `(2♦)` package, with `Plain`/`Rubensohl` opt-in.

**The top-step clubs transfer (80, follow-up — shipped, theory-correct, DD-marginally-negative).**
Cohen's transfer chain runs *up the line through* the adverse suit, so the highest
3-level step has no suit above it to transfer into and wraps back to **clubs**:
`1NT–(2♦/2♥)–3♠` and `1NT–(2♠)–3♥` are a *forced* game-force transfer to clubs (6+♣,
`points(10..)`; completion `3NT` with a stopper in their suit, else `5♣` — `3♣` is
unplayable below the top step, so game is forced). Previously these fell to the
natural floor, leaving a 6+♣ GF hand with no call: the weak `2NT`→`3♣` relay is
`points(..=8)`, so it cannot carry a game force (bidding it strands the game in `3♣`).
`TransferSmolen` already had the `(2♦)`→`3♠`→♣ leg; this adds the same wrap for
`(2♥)`/`(2♠)` and for plain `Transfer` over `(2♦)`. Lives in the shared
their suit) plus a generalized `clubs_transfer_completion(over)`. A/B (two binaries at
a fixed seed, `--ns transfersmolen --ew off`, 200k filtered/cell): **vs `off`:
−0.0008/−0.0012 board** [`PD`], ≈87 boards changed (0.04%), ≈−1.8/−2.8 IMPs each.
The worst boards are textbook DD-blindness: the transfer reaches a normal
making `3NT` (e.g. 27 combined HCP with a running club source), while the floor
instead makes a *speculative penalty double of the overcall* (`2♦×`/`2♥×`) that
perfect double-dummy defense turns into a giant set — the harness over-credits the
defense, exactly the obstruction-blindness flagged for Lebensohl-vs-floor above. Kept
in the default as a theory-correct completion (the bid a 6+♣ GF hand otherwise lacks),
pending a single-dummy re-measure. (Cohen's full *slow-shows-stopper* layer —
`2NT`→`3♣`→cue = Stayman *with* a stopper — is a separate, unimplemented refinement;
`2NT` here is only the weak relay.) `lebensohl-ab` gains `--seed` (deterministic
two-binary runs) and `--only-topstep` (restrict to top-step boards; note it also
catches floor `3♠`-natural auctions, so the clean isolation is the two-binary delta).

**The `2NT`-role A/B (80, follow-up — kept opt-in).** The `2NT`-role
swap — **true Rubensohl** (`2NT` an artificial **club** transfer) vs the relay
(Transfer Lebensohl) — was authored on the Cohen structure and A/B'd vs `Transfer`.
Design (jdh8's rule): every transfer to a suit *below* the overcall is two-way (weak
transfers then passes; strong continues), so `2NT`=clubs is two-way; transfers to
a suit *above* the overcall stay INV+ (weak hands escape with a natural 2-level
bid), identical to `Transfer` — so opener still auto-drives those to game.
**vs `Transfer`: +0.001/−0.023 board** [`PD` 5611eac, 200k] — neutral non-vul, still
a clear loss vul. Mechanism: for weak hands both arms reach the *same* contract
(right-siding the low-suit partscore is DD-blind), so Rubensohl's only gain is
invisible to DD, while making the low transfers two-way *costs* `Transfer`'s
auto-drive-to-game on invitational hands. PD only doubles *failing*
contracts; it does not let the harness see *who declares*, so it cannot reward
Rubensohl's right-siding edge — the verdict is unchanged. Per the user's gating
("if the cheap probe stays neutral/negative the full standard ladder won't rescue
it, since its extra structure — Smolen, transfer-into-suit, 3♠-minors — is all
right-siding"), the full standard ledger was **not** authored. The variant was kept
as `LebensohlStyle::Rubensohl` (opt-in via `set_lebensohl_style`; the default stayed
`Transfer`) for a future single-dummy / live-search re-measure that could see
right-siding.

**REMOVED (2026-06-20).** True Rubensohl was deleted — `LebensohlStyle::Rubensohl`,
`rubensohl_responder`, `complete_two_way_transfer`, `two_way_transfer_rebid`, and
their dispatch in the `1NT`-overcall and after-double contexts. jdh8 judged it
inferior: its only edge is DD-blind right-siding (never measured a win), and he
prefers the Smolen+LM-over-minors / Cohen-over-majors split that `Transfer` already
carries. The refinements that motivated this revisit (top-step clubs transfer,
delayed cue, `(2♦)` Smolen) don't port to Rubensohl anyway — its `2NT`-club-transfer
and two-way machinery consume the very seams those refinements exploit. Only three
styles remain: `Off`/`Plain`/`Transfer`.

**Responder's double of the overcall (`1NT–(2♦/2♥/2♠)–X`) — penalty stays
default; verdict is measure-dependent.** The status-quo penalty double
(`len(over,4..) & hcp(9..)`) was A/B'd against a takeout double (`≤3 & 7+`), a
cooperative/optional double (`2-3 & 7+/8+`), and a lower-floor penalty (`4+ & 7+`,
plus a looser `3+ & 7+`), via the new `DoubleStyle` toggle (`set_double_style`)
and `lebensohl-ab --ns-dbl/--ew-dbl` (200k filtered, vs penalty 4+/9+, none/both):

- **Perfect-defense** (old `ns_score`): **every alternative loses** — `PenaltyLight`
  4+/7 −0.035/−0.041, `Optional` 2-3/8 −0.039/−0.041, `Optional` 2-3/7
  −0.081/−0.089, `Takeout` ≤3/7 −0.089/−0.092; looser `PenaltyLight` 3+/7 worst
  (−0.100/−0.115).
- **Plain DD** (current A/B scorer, `ns_score_contract` after the scoring split,
  commit a6f2206): the **flip** — `Takeout` **+0.011/+0.018**, `Optional` 2-3/8
  **+0.012/+0.015** go marginally positive (+0.14–0.32 IMPs/div); `PenaltyLight`
  still loses (−0.018/−0.023).

PD auto-doubles the failing takeout/optional overbids; plain DD scores them
undoubled → they look slightly positive, but the edge is near-noise and is
plausibly the overbid under-punishment PD exists to correct (cf. Jordan/Truscott
below, responsive-X). Per the user, **default stays Penalty**; `DoubleStyle` kept
opt-in (best thresholds baked: `PenaltyLight` 4+, `Optional` 8+) for a future
single-dummy re-measure where takeout's competitive value might genuinely pay.
(this commit)

**Jordan/Truscott (71) — tried and rejected (DD-negative).** Authored
`1M–(X)–2NT` = limit-raise-or-better + `3M` = preemptive, with opener's decline
path (`2NT`→`3M` sign-off, responder pass/4M) and a sound `2NT` strength
inference; reused the uncontested `major_responses` for every non-Jordan call;
gated by `set_jordan`. A/B'd vs the system-on baseline (`jordan-ab`, contested
seat-swap duplicate, `Tag::NATURAL` opponents take out double our major).
Result: **vs floor: −1.0/−1.5 div** [`PD`, 500k] (the `2NT`-constructive half alone
−4.2/−4.4). Two causes, both inherent
to the harness: (1) the preemptive `3M`'s obstruction value is invisible to the
double-dummy / perfect-defense measure (the solver sees through it — cf.
`texas-vs-sat` "concealment is single-dummy"), while its overbid cost is counted;
(2) making `2NT` limit-or-better diverts 13+ game-forcing raises out of pons's
rich **Jacoby 2NT** machinery (shortness / slam) into a crude `3M/4M` stub,
reaching worse games and missed slams that the doubler punishes. **Reverted** —
the floor's system-on (`2NT` = Jacoby, `3M` = limit raise) stays. Revisit only
under a single-dummy / IMPs-vs-humans measure where preemption actually pays.

## Defensive — their opening

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 43 | Cappelletti | override → **Woolsey** | add (Batch 1) | — | — |
| 84 | Michaels cuebid | shipped | keep | — | — |
| 127 | Unusual 2NT | shipped | keep | — | — |
| 126 | Unusual 1NT | gap | add (Batch 1) | — | — |
| 79 | Leaping Michaels | **shipped, default ON** | keep on | `4♣/4♦` strong 5-5 two-suiters + authored advances. **vs floor: +1.010/+1.195 board, +3.906/+4.624 div** [a6f2206, 40k, 25.8% div]. Inference reader decodes the two-suiter so `american_search` prices the advance by DD (slam-capable). `set_leaping_michaels(false)` to disable. | (this commit) |
| 123 | Two-suit takeout double | gap | add (Batch 1) | — | — |
| — | **Off-shape X support gate + 2-level overcall discipline** | **shipped, default ON** | anchor bucket-1 fix (traced from `Defensive / book / round-1 −98k`) | **combined vs historical: +0.004/+0.019 board plain, +0.008/+0.026 PD** [1783402635, 102.4k/vul, ~3.6% fired] — no plain loss either vul, both-vul CI>0 on both scorers → default-on. Two additive levers on disjoint boards: `set_takeout_support(Strict)` (12+ X needs 3+ in every unbid suit, else overcall / wait for 17+; **strict alone +0.005/+0.012 plain, +0.004/+0.013 PD**) and `set_overcall_discipline(true)` (2-level overcall = opening 11–17, 1-level cap 17; **disc alone −0.001/+0.007 plain, +0.004/+0.013 PD**). `Off` + `false` reproduce the historical book. | (this commit) |
| 129 | Unusual 4NT | verify | — | — | — |
| 48 | Cue bid | partial | verify | — | — |
| 106 | **Sohl after double** (advancer, weak twos) | **shipped, `Transfer` default ON** (true `Rubensohl` removed 2026-06-20) | `Transfer` default = Cohen + `(2♦)` Smolen (folded in) | **`Transfer` vs `off`: +0.016/+0.102 board, +0.164/+1.052 div** [a6f2206, ~9.8% div] — positive at both vulnerabilities (incl. the `(2♦)` Smolen package). `Transfer` stays default. | (this commit) (`set_advance_sohl_style`) |
| 82 | **Lebensohl after double** (advancer, weak twos; = `Plain`) | measured; opt-in, dominated | `Transfer` (#106) is the default; `Plain` worse | **`Plain` vs `off`: −0.160/−0.153 board, −1.964/−1.840 div** [a6f2206, ~8% div] — negative, and dominated by `Transfer`. Stays opt-in / A/B arm. | a6e7ab9 |

**Lebensohl after a takeout double (advancer over a weak two) — measured;
best variant (`Transfer`) PROMOTED to default.** After `(2X)–X–(P)` the flat `advance_double` ladder can't
distinguish a weak long-suit hand from a constructive one, so the doubler
can't tell when to move. Four sohl structures were authored under the
`(2X)–X–(P)` prefix (reusing the Section-5 builders for `Plain` / `Transfer`,
plus `Pam` = `2NT` shows 5-5 minors and `Lawrence` = three-band
weak/INV/GF strength) and A/B'd on `sohl-after-double-ab` (contested
seat-swap, 200k filtered boards/cell). `Transfer` won (current figures in
rows #106/#82); `Lawrence` and `Pam` both lost to it. Mechanism: a takeout double already advertises
the fit (short in their suit, length elsewhere), so the floor's natural
advancing locates most fits — `Transfer`'s right-siding is DD-blind upside,
`Lawrence` loses 5-card-suit *shape* information by collapsing INV into a
single direct-3X slot, and `Pam`'s 5-5-minors trigger is too rare (~0.4 %
divergence) to recover the slot it eats from weak long-clubs. Stopper-routing ("slow shows /
fast denies") was later tested too and is dead flat on DD (see the
`set_delayed_cue` update below); the strength hypothesis held. This **is** toggle
`#106` and `#82`; the "our opening is doubled" responder case is a *separate*
BBA toggle (`Transfers if RHO doubles`), not this one.

**Update (this commit) — `Transfer` promoted to default + true `Rubensohl`
wired.** The old "DD-neutral → keep `Off`" basis was an artifact of the optimistic
scorer; `Transfer` is positive (current figures in row #106) and is promoted from
opt-in to the **default** advance-of-double sohl. `Plain` (#82) stays dominated, an
opt-in / A/B arm. True `Rubensohl` (the fourth `LebensohlStyle`: `2NT` = artificial
club transfer, the low transfers two-way) is **wired into the `(2X)–X–(P)` context
too** (a verbatim mirror of the Section-5 1NT-context wiring; `--ns rubensohl` on
`sohl-after-double-ab`): head-to-head **`Rubensohl` vs `Transfer`: −0.007/−0.037
board** [`PD`, ~2.5% div] — no gain, **kept opt-in** (its edge is DD-blind
right-siding, exactly the 1NT-context finding). Default is now `Transfer`; `Off` /
`Plain` / `Rubensohl` remain selectable via `set_advance_sohl_style`. Revisit
`Rubensohl` only under a single-dummy measure that can see right-siding.

**Update (this session) — the `(2♦)` Smolen package now carried after the double
too, and `TransferSmolen` folded into `Transfer`.** The `(2♦)`-only `3♣`-Stayman +
Smolen + Jacoby-reshuffle + Leaping-Michaels package that won in the 1NT context
(#80) was wired into the `(2X)–X–(P)` advance as well (verbatim Section-5d reuse,
diamond-only, ~0.8% divergence). Head-to-head vs the plain-Cohen advance
(`sohl-after-double-ab`, 200k filtered/cell): **`Transfer` vs `Plain`:
+0.168/+0.249 board, +3.309/+4.772 div** [a6f2206] — a clean win whose per-div edge
*rises* with vulnerability (reaching better contracts, not right-siding). Winning in
**both** contexts, the experimental `TransferSmolen` style was renamed to `Transfer`
and dropped: `Transfer` *is* Cohen-plus-Smolen-over-`(2♦)` everywhere, styles back to
`Off`/`Plain`/`Transfer`/`Rubensohl` (true `Rubensohl` later removed 2026-06-20;
styles are now `Off`/`Plain`/`Transfer`). Current default `Transfer` vs floor: see
row #106.

**Update (this session) — stopper-routing ("slow shows / fast denies") finally
tested; near-zero on DD, kept opt-in.** The gap flagged above ("not tested per
user direction") is now closed. Larry Cohen's split cue, adapted to our Transfer
Lebensohl: the *direct* cue of their suit denies a stopper, while a *delayed*
cue (relay through `2NT`, then their suit) is Stayman *with* a stopper — and, per
the user, also denies a 5-card unbid major (Smolen / Leaping Michaels own those).
Stopper hands relay slowly and still find the 4-4 major fit (`cue_stayman_answer`,
3NT safe); no-stopper hands keep the fast cue and, lacking a major fit, run to a
minor-suit game instead of a stopperless 3NT (`cue_stayman_answer_no_stopper`).
Authored only in the single-unbid-major contexts — over `(2♥)`/`(2♠)` — behind a
default-off `set_delayed_cue` toggle; `--delayed-cue` on `sohl-after-double-ab`.
Isolation A/B (delayed-cue-`Transfer` NS vs plain-`Transfer` EW, 200k filtered/cell):
**+0.000/+0.001 board, +0.098/+0.387 div** [`PD`, ~0.4% div]. Verdict: **dead flat —
rejected as default, kept opt-in.** Mechanism is exactly as predicted: stopper hands reach the
*same* contract fast or slow (zero swing), so the only divergence is the rare
no-stopper-no-fit hand choosing 4m over 3NT — and the genuine payload of "I hold
their suit stopped" (concealment, right-siding the 3NT) is single-dummy, which the
PD harness looks straight through. Same wall as `TransferSmolen`/`Rubensohl`:
right-siding refinements don't register on DD. Revisit only under a single-dummy
measure. Toggle stays `set_delayed_cue(false)` by default; the shipped system is
unchanged.

*Recognition split from policy (kept default-on).* Because the delayed cue is a
brand-new auction position the floor had no meaning for, the *answer* node is
purely additive and is wired **always-on** in both the `1NT`-overcalled and
`(2X)–X–(P)` contexts (over `(2♥)`/`(2♠)`): the bot answers a partner's delayed
cue (the other major with a fit, else `3NT`) even though it never *bids* one. The
node is unreachable in bot-vs-bot play (the bot's advancer never produces the cue
with the toggle off), so self-play and every A/B are byte-identical — it only
activates opposite a human partner who plays the convention. `set_delayed_cue`
gates only the *bidding* side (the bot routing its own stopper hands through the
delayed cue + reading its own direct cue as stopper-denying). Test:
`tests/american_defense.rs::test_recognize_delayed_cue_major_fit`.

**Leaping Michaels (79) — shipped opt-in, a clear DD win once the advances were
authored.** Over their weak two, a jump to `4♣`/`4♦` names a 5-5 two-suiter with
game-forcing values: over a major it shows a minor + the *other* major; over `2♦`
the `4♦` cue shows both majors and `4♣` shows clubs + a major. Authored in
`defense_to_weak_two` behind `set_leaping_michaels` (default `Off`), with advancer
continuations in `leaping_michaels_advances` (a fit major game — taking even a
7-card fit, which scores well and makes on ten tricks; else the `5m` minor game;
never a passed-out partscore; over `2♦`, `4♥` is pass-or-correct to opener's
major). A/B on `leaping-michaels-ab` (contested seat-swap): a clear win vs the floor
(current figure in row #79).

*The first cut measured −0.6/−0.9* — but that was the **unauthored advancer**,
not the convention: worst-board analysis showed the instinct floor *passing* the
two-suiter, leaving us in `4m` (or, over the `2♦` cue, declaring the opponents'
diamonds). Authoring the advance flipped the sign by +1.7 IMPs/board. The lesson
is the inverse of the obstruction wall (`#71`/`#100`): a constructive competitive
convention can win big on DD when it reaches a *better strain* — but only if the
whole sequence, advances included, is authored; a half-built convention measures
as a loss for a reason that has nothing to do with the idea.

The authored advance is capped at game. To let the bidder reach the slams a big
two-suiter is *for*, `Inferences::read` now decodes the overcall's two suits
(`leaping_michaels_reading`, post-walk like the Rubens cue), so the constrained
sampler conditions partner correctly and the live double-dummy search bidder
(`american_search`, `--features search`) prices the advance — 4M / 5m / slam — by
cardplay EV. The authored length rules become the fast-floor *prior*; DD disposes.
A directional A/B (search+LM NS vs authored-rules+LM EW, 60 filtered boards, trimmed
64-layout search) measured **+2.8 IMPs/board** for search *on top of* the rule floor,
and the auctions show it reaching the slams the game-capped rules cannot (e.g. a
`6♥` off the `2♦` both-majors cue, a `7♣` grand) — at the cost of a few search
overbids (the small-sample / shortlist noise; a larger run would tighten).
**Shipped default ON** (`Cell::new(true)`); `set_leaping_michaels(false)` recovers
the prior weak-two defense. The plan's "spend runtime for better calls" (M2.3)
makes `american_search` the blessed way to play it — the slam upside lives there,
while the fast floor's authored rules bank the row-#79 figure.

**Perfect-defense re-validation sweep (after `ns_score` fix 5611eac).** `ns_score`
now doubles any contract that fails double-dummy (a real defense doubles what it
can beat); the optimistic-undoubled variant is gone. That re-priced every prior
A/B, so the toggle-based competitive conventions were re-measured. PD only changes
scores on divergent boards where one arm reaches a *failing* (now-doubled)
contract, which splits the results cleanly:

- **Convention-vs-convention CHOICES all hold** (both arms share the obstruction
  value, so it cancels and only the constructive/placement edge remains): Leaping
  Michaels on-vs-off **+1.100/+1.445/board** (was +1.090/+1.452, 40k filter);
  Transfer-vs-Plain Lebensohl **+0.46/+0.69/div** (was +0.46/+1.24, both-vul
  shrank, 80k filter-dh); wide-vs-classic 1NT **constructive** **+0.11/+0.32/div**
  (was +0.32 none, `nt-shape-abc` 500k). **No ship decision is overturned.**
- **Convention-vs-NOTHING comparisons flip negative** for the obstructive ones —
  PD sharpens the documented DD-blindness-to-obstruction (the solver sees through
  the obstruction; PD now fully counts the failing-overbid cost): Transfer
  Lebensohl vs the bare floor **−0.66/−0.62/div** (was +0.35/+0.05); wide-vs-classic
  1NT **contested** (`nt-shape-contested`, 100k) **−0.50/−0.61/div** (was
  +0.57/+0.93). Per-board impact is tiny (−0.005…−0.058 — rare auctions). The flip
  is a harness artifact, not a bad convention.
- **sohl-after-double (#106) flipped DD-POSITIVE** — see the row; its opt-in
  rationale is gone (promotion candidate, kept opt-in pending a deeper re-measure).
- **The two-binary *constructive* conventions** (Stayman, 1NT-3♦ #14, Puppet #12,
  SAT #119, M6.1 inferences, minor keycard #75): they reach *making* contracts, so PD
  only doubles the looser baseline's failures → predicted to hold or improve, and they
  shipped with large margins. Re-validating each needs a per-feature worktree rebuild
  (old-arm `--phase bid` → new-arm `--phase score`; the harness file format is stable).
  **minor keycard #75 now re-measured under PD: +5.41/+7.05 div** [10M, 202 div] **—
  HOLDS** (was +6.80/+8.76 optimistic; PD trims but stays clearly
  positive; isolated by reverting just 99da1b3, zero drift in the touched files). The
  other four remain predicted-only (lowest priority; SAT #119 has the most failing-slam
  exposure).

**Rich advance of a takeout double (`[1t, X, P]`, 2026-07-08, `set_rich_advance_double`,
opt-in default-off, `bba-gen --ns-rich-advance`):** the flat `advance_double` floor
gave the advancer only a cheapest natural suit, a `3NT`, and a penalty pass — the whole
10+ invitational-or-better band collapsed into "bid your cheapest suit," flat, with **no
cue and no way to invite or force**. This is the leak behind the 4432-vs-major anchor
finding (advancer leaps to a bad `3NT`; see [[project_5332-4432-takeout-discipline]]).
A BBA distillation (`examples/probe-advance-double`, seat 3, 30k hands/opening) showed
BBA's advance is uniform: **cue of opener's suit = 10–17, ~20%** (the fat workhorse) +
a natural ladder split by level (cheapest = weak-wide, single-jump = 7–12 invite, NT
ladder, weak shapely game jumps, penalty pass). Our floor had **no cue at all**.

Authored `advance_double_rich`: per jdh8's design, a **cue asking for a 4-card unbid
major** (the Stayman-ask), **2NT almost denies a major**, a `1NT`/`3NT` stopper ladder,
weak shapely game jumps, and a **forced 3-card suit when broke**. Plus `answer_advance_cue`
— the doubler's reply, with a finite `2NT` catch-all so the artificial cue is **never
passed out**. Cue alerted (`ADVANCE_CUE`), rule-projected.

**Measured a clean DD-wash** (four-version arc, each 409.6k bd/arm/vul, `--ns-rich-advance`):
`v1` cue-no-answer −0.0020 (passed-out cues → us declaring their suit; the M6.3 trap).
`v2` +doubler answer for *both* RHO-pass **and RHO-doubles-the-cue** (the second passed-out
branch, ~152 disasters/arm, was the whole v1 loss) → −0.0005 wash. `v3` +advancer *drives
to game* after the answer → −0.0015 **worse**: the drive overbid, because the rich builder
had dropped the flat floor's "4-card major + game values → blast 4M" jump, so game hands
diverted into the cue and landed short. `v4` restore the game-blast (12+, above the cue) +
**cap the cue to invitational 10–11** (its floored rest is then correct) → **−0.0001 NV /
−0.0007 both, PD≈plain, CIs include 0** — dead wash. The flat floor already blasts games
well; the cue's invitational precision is real bridge but **DD-invisible** (competitive /
fit-finding). Sound + complete but **not a gap-closer**; kept **opt-in default-off**.

**Decision (jdh8):** build the **jump-cue Rubens** layer anyway (system completeness over
DD gap-closing — accept a likely DD-wash for competitive precision DD can't score).

**Phase 2 — flagship major-transfer (`set_advance_rubens`, opt-in, no-op unless rich is
on; `--ns-advance-rubens`):** a 5+ unbid major (INV+) transfers via the rank below it
(`3♦`→♥, `3♥`→♠), and the doubler **completes and declares** — right-siding the strong
hand. Over `(1♠)` the sole unbid major (hearts) is below the jump-cue, so `3♥` is natural.
Completion has a finite catch-all (never passes the transfer), super-accepts `4M` with a
max; advancer raises/rests. Unit-verified across `(1♣)`/`(1♦)`/`(1♥)`.

**Rubens increment measured DEAD ZERO** (`rubens` vs `rich`, 409.6k bd/arm/vul, SEED_BASE
1783497765): **+0.0000 NV / +0.0001 both plain, +0.0002/+0.0003 PD**, fired 0.03%, all CIs
span 0. This is the textbook signature: **right-siding is invisible to double-dummy** — the
transfer changes *who declares*, not the trick count, so DD sees nothing (the ~+0.0001 is
the rare contract-level change). No regression. See [[project_sd-lead-scorer]].

**sd-lead re-measure (2026-07-08, `ab-dump-sd`, 16 worlds, same dumps):** the middle
bracket **can't find the right-siding value either.** Rubens increment (`rubens` vs `rich`):
**−0.236 IMPs/fired NV [95% CI −1.12..+0.65], −0.240 vul [−1.50..+1.02]** (n=127/129 fired —
too small to resolve). Whole rich advance (`rich` vs `base`, larger n=1154/1179): **−0.231
NV [−0.51..+0.05], −0.219 vul [−0.55..+0.12]** — wash with a faint *negative* lean. The
uniform ≈−0.23 across both layers and both vuls points the **opposite** way from the
right-siding hypothesis: our alerted artificial cue/transfer *discloses* the advancer's
shape+strength to the blind leader, and that concealment cost slightly outweighs the
right-siding gain the transfer buys. No CI cleanly excludes 0 — verdict is **sd-wash**,
matching the DD-wash. Both layers stay opt-in default-off; nothing ships. **Parked:** a
*full* transfer ladder from the *simple* cue up may suit **balancing** doubles — revisit later.

**INV+ cue restructure (2026-07-08, authoring only — knobs unchanged, still opt-in
default-off).** The first cut capped the cue at a Stayman-ask (10–11) and blasted 4M with
game values; jdh8 replaced it with the standard expert ladder (confirmed against
[CSB's *Forcing bids after a takeout double*](https://csbnews.org/en/forcing-bids-after-a-takeout-double/)):
**new-suit jumps MAJORS-ONLY** — a 2-level jump = CONSTRUCTIVE (8–10, 4+), a 3-level jump
= INV (10–12, 5+); a jump in a minor abandons 3NT for a suit that needs eleven tricks and
gets doubled. **1NT = 8–10 stop**, **2NT = 11–12 balanced stop**, **3NT = limited 13–17
stop**; **4M jump always LIMITED** — two-way `points(11..=15)` (shapely-weak *or* min-FG,
distribution-aware) with no Rubens transfer for that major, purely preemptive (0–10) when a
transfer carries the strong hands, so slam tries always cue and `1♠–X–4♥` stays two-way
(hearts can't transfer over 1♠). The **cue is now INV+, forcing one round** (not GF): the
lowest-weighted action above a weak natural suit (`hcp(10..)`, weight 1.05), so every
specific limited bid outranks it and only the shapeless invite-or-better hand lands there.
Because the alerted cue projects only `hcp(10..)`, the floor can't tell invite from force —
so advancer's clarification is **authored** (`advance_cue_rebid`): over the doubler's minimum
answer a GF advancer (13+) raises the shown suit to game or bids 3NT, an INV advancer (10–12)
passes. Wired for every non-game answer (`advance_cue_answers`) × RHO {pass, double}. The
**penalty pass** is widened to `len(theirs, 5..)` alone (or 4 with two top honors) so a weak
5-in-their-suit hand converts instead of being forced into a doubled minor. Tests
`rich_advance_double_cues_and_forces`, `advance_cue_rebid_forces_or_invites`,
`rich_advance_weak_shapely_blasts_game`. **Left to the floor** (flagged, not authored): the
doubler bidding on with extras over advancer's INV rebid, and the cue-then-new-suit
two-suiter GF.

**The no-regression guard earned its keep — it caught a real DD regression.** The first
restructure cut (`4M = hcp(12..=15)`) measured **DD-negative** vs the flat floor, plain≈PD
(*not* a doubling artifact): the pure-MIN-FG 4M had deleted the flat floor's weak shapely
game-blast (`len(4..) & points(11..)`), stranding weak long-major hands below a makeable
game. Worst boards were advancer competing in a **minor** and doubled at the four-level
(`4♣×`/`4♦×`) where the flat floor's `1NT` stayed safe. Fixed in three passes
(`advance-double-v5/6/7`, 102.4k bd/arm/vul, plain NV/vul): **v5** −0.0040/−0.0062 →
**v6** +two-way 4M −0.0024/−0.0031 → **v7** +majors-only jumps +wide penalty pass
**−0.0011/−0.0010 = wash** (PD≈plain, CIs ~±0.0014 straddle 0). Rubens increment stays ~0
(v7 −0.0006/−0.0006 plain). So the book carries the more bridge-correct INV+ cue + authored
rebid at the *same* DD-wash the original had. **Lesson: even an opt-in, DD-invisible book
needs the guard — a restructure can introduce genuinely-worse *reachable* contracts (doubled
minor games), which are DD-visible and distinct from the invisible right-siding value.**
Still opt-in default-off; the default system is byte-identical.

## Openings

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 20/24/26 | 1NT NT-style / 15-17 / 5422 | shipped (default) | keep | — | — |
| 5 | 1m allows 5M | verify | — | — | — |
| 132 | Weak natural 2♦ | shipped | keep | — | — |
| 133 | Weak natural 2M | shipped | keep | — | — |
| 92 | Namyats | conflict (natural 4-lvl) | A/B (Batch 3) | — | — |
| 33 | 4NT opening | gap (rare) | low priority | — | — |

## Jargon to confirm against authoritative text (at authoring time)

BROMAD (42) · Strength Lawrence structure (114) · Mixed raise (89) ·
Maximal doubles (83) · Super/Extended acceptance after NT (55,115) ·
Unusual 4NT (129) · Support 1NT (116).

## Out of scope (21GF turns OFF; user confirmed skip)

Multi 2♦ (90) · Benjamin 2D (36) · French 2D (60) · Gambling (61) ·
Gazzilli (63) · Ghestem (66) · Raptor 1NT (99) · plain Landy (78) ·
Multi-Landy as a whole (91 — but Woolsey's Multi-Landy *structure* is in via
override) · Polish two-suiters (97) · Wilkosz (134) · Kickback (72-74) /
Crosswood (45-47) · New Minor Forcing (94) / plain Checkback (44) ·
Soloway jump shifts (111-112) · Snapdragon (110) · weak jump shifts off in
21GF (130-131 — pons keeps weak JS regardless).
