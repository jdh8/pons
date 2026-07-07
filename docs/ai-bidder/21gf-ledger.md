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
