# Measuring bidding changes — the A/B playbook

Every rule in this document was paid for with a wrong conclusion. Double-dummy
(DD) A/B measurement has systematic biases; a change that "measures +0.3" can
be an artifact, and a change that "measures −0.6" can be a good idea half-built.
Follow the checklist; the [biases](#known-biases) section explains each rule.

This doc answers *does this change ship?* Two sibling docs answer the adjacent
questions: [convention-tuning.md](convention-tuning.md) (*what is a convention's
best range, and which of its calls leaks?* — sweeping and per-call forensics)
and [ai-bidder/gto-1nt-defense.md](ai-bidder/gto-1nt-defense.md) (*which whole
method is best?* — the matrix-game tournament). The verdict every knob ships on
is indexed in [bidding-options.md](bidding-options.md) — one row per `set_*`
option with its A/B number and ship/opt-in decision.

Mechanics that churn (exact CLI flags, shard counts) are documented where they
live: `<example> --help`, the header of
[`scripts/bba-gen-parallel.sh`](../scripts/bba-gen-parallel.sh), and
[`shared-machine-data-gen.md`](shared-machine-data-gen.md). This document holds
the parts that do not churn: protocol, interpretation, and ship rules.

## The checklist

1. **Pick the honest baseline** — the contract the hands *actually* reach
   without the change (if a hand would transfer, the baseline is the transfer,
   not "pass/invite"). Analytic contract-pricing probes are only valid when the
   hand class genuinely stops in the contracts you price; anything with a long
   suit needs a live A/B.
2. **Complete the treatment before measuring.** Author *both* sides'
   continuations (advances, rebids, break-of-transfer, the doubled/overcalled
   tails). A half-built convention measures as a loss even when the idea wins.
3. **Gate it with a `set_*` knob** (thread-local, read at book construction).
   While measuring, the knob's *off* state must leave the default system
   byte-identical.
4. **Choose the harness.**
   - `examples/ab-*` — self-play seat-swap A/B (fast, both pairs our system).
   - `examples/bba-gen` + `scripts/bba-gen-parallel.sh` + `ab-dump-diff` /
     `bba-score` — versus BBA/EPBot, the reference opponent.
   - Measuring **our own convention** needs an opponent that *reads* it
     (`--advertise-*` flags): self-play with the convention off on the other
     side cannot punish or exploit the disclosure, and over-rates it.
5. **Seed hygiene.** `export SEED_BASE=$(date +%s)` **once per experiment**;
   every arm of the experiment reuses it (paired diffs need identical deals).
   The *next* experiment takes a fresh base. Never replay fixed seeds 0..31.
6. **Run politely and sequentially.** Wrap heavy runs in
   `scripts/idle-run.sh`; run arms one after another, never in parallel (each
   run already saturates the box); **never `cargo build` while an A/B is
   running** (later shards exec the new binary and die on renamed flags).
7. **Score with both scorers** — plain DD (`ns_score_contract`) *and* perfect
   defense (`ns_score_pd`) — and report: IMPs/board, IMPs/fired (or /divergent),
   the fired rate, and vulnerability split (none/both). Two seeds or a
   bootstrap CI before trusting a small edge.
8. **Read the verdict from the [decision table](#the-decision-table).**
9. **Before declaring a loss dead, trace the worst divergent boards.** The
   cause is often an unauthored continuation, an over-broad trigger firing
   outside its intended hand class, or a shared node leaking into a context
   where the treatment doesn't apply — all fixable, none "the idea is bad."
10. **Ship per the [ship rules](#ship-rules)**; record the result in
    `CHANGELOG.md` (and the relevant `docs/ai-bidder/*ledger*` if applicable).

## Scorers (`src/scoring.rs`)

| Scorer | What it scores | Use for |
| --- | --- | --- |
| `ns_score_contract` | Plain DD: the reached contract with its *actual* table penalty. | Duplicate A/B results — the default verdict. |
| `ns_score_pd` | Perfect defense: a contract that fails DD is scored **doubled** (synthetic X), making ones undoubled. | The pessimistic bracket end: "scored against a competent doubler." |
| `ns_score_bid` | Perfect defense, takes a `Bid` (derives the penalty). | Evaluating a **call** (EV rollouts, contract-choice probes) — never for A/B results. |
| `single_dummy_leads` (`src/single_dummy.rs`) | MC-DD with a *blind* opening lead chosen from the leader's sampled worlds. | The one known DD bias at 1NT level (DD defenders always find the killing lead, ~+0.3 tricks to 1NT declarers). Re-score close NT-defense verdicts with it. |
| `single_dummy_playout` (`src/single_dummy.rs`) | The **sd-declarer playout**: blind lead, then declarer chooses every card MC-DD over auction-consistent worlds (show-outs remembered) while the defense plays DD on the actual deal. | The slam-side DD bias (see below): a DD declarer never misguesses, so every DD-play scorer is *optimistic* for the arm bidding more slams. Runners: `ab-dump-sd --sd-declarer`, `ab-slam-entry --sd`. Sequential per board — divergent sets only. |

**The principle** (jdh8, 2026-06-24): *the threat of a double is a legitimate
deterrent, but a double that never appeared on the table must never enter the
final score.* Hence the bracket: truth sits between plain DD (under-punishes
overbids) and PD (over-punishes — a perfect doubler never doubles a making
contract). Reality is closer to plain DD.

## The decision table

| Plain DD | PD | Verdict |
| --- | --- | --- |
| win | win | Real. Ship default-on. |
| wash (CI straddles 0) | win | **Shippable default-on** — a one-sided bet: never loses on the honest scorer, gains when opponents punish. |
| win | wash/loss (PD *erases* a plain win) | Doubling artifact — the "win" is reaching contracts a competent doubler would slaughter. Suspect; don't ship on this evidence. |
| loss | win | Artifact of PD's synthetic X (it credits phantom doubles of contracts we no longer bid). Not a win. |
| loss | loss | Loss — but trace worst boards before declaring dead (step 9). |
| wash | wash, treatment is *additive* (repurposes a useless call, sacrifices nothing) | May ship default-on if its value is DD-invisible (obstruction, lead-direction) — precedent: Unusual 2NT over their 1NT. |
| wash | wash, two methods that *push each other* | Break the tie by **naturalness** (ship rules): a move *toward* established natural theory ships default-on; a *convention* trialled against natural stays opt-in. |

**Slam-boundary addendum** (2026-07-16; reading rule revised same day after
the calibration showed the playout is a 2–4× too-deep pessimist): for a knob
whose ON arm bids **more slams** (a lowered slam gate, a new slam drive, a
keycard capability-add), the verdict still comes from the plain + PD table
above. The insurance against DD's slam optimism is **analytic — Pavlicek's
Δlogit applied to the DD-making slams on the divergent set**: treat a
DD-making small slam as failing with q ≈ 1–3% (6-level odds ratio 0.88–0.95),
a DD-making grand with q ≈ 3–10% (majors/NT; minor-suit grands are
noisy-high). Since a slam-vs-game swing is roughly ±W symmetric, that shaves
the knob's slam-win contribution by a factor (1 − 2q): **2–6% at the 6-level,
~6–20% at the 7-level** — only hair-thin margins or grand-heavy divergent
sets can die of it. Note DD is nearly *calibrated* at the 6-level net of
defender errors; the slam-optimism wall bites grands hardest. The sd-declarer
row (`ab-dump-sd --sd-declarer` / `ab-slam-entry --sd`) stays as the free
robustness bound: a win that survives even the playout is extra-safe; a
playout flip triggers the Pavlicek shave + a divergent-board trace, never an
automatic demotion. See the slam-optimism wall under Known biases.

Sub-0.1 IMPs/board is noise unless the sample is large (hundreds of thousands
of boards); a *fired-rate*-weighted per-fired figure with a CI excluding 0 is
the stronger claim. On contested/filtered harnesses compare **IMPs/divergent**,
not IMPs/board — `--filter` biases the per-board denominator, not the
divergent set.

## Known biases

These produced actual wrong conclusions; each has a memory/ledger trail.

- **The obstruction wall.** DD sees through concealment: preempts, weak jumps,
  lead-direction, "make them guess" value is invisible, while the overbid cost
  is fully counted. Preemptive/obstructive treatments *cannot* measure positive
  here — defer them to single-dummy scoring, don't reject the idea. Conversely,
  **constructive** competitive value (reaching a better strain/level under
  interference) is DD-visible and can win big (Leaping Michaels +1.09/board).
- **The slam-optimism wall — the obstruction wall's mirror.** All three
  standard brackets play tricks 2–13 double-dummy, and a DD declarer picks
  every two-way queen, drops every offside stiff king, finds every squeeze.
  At the 1NT end the dominant seam is the blind lead (DD *pessimistic* for
  declarer; sd-lead corrects it), but that gap tapers to zero with level while
  the misguess seam remains — so at the slam boundary every DD-play scorer is
  **optimistic for the arm bidding more slams** (PD doesn't help: it prices
  doubling, not guessing). Just as a plain-DD *loss* for a preempt is the
  harness, a plain-DD *win* for a slam-aggression knob is suspect until the
  **sd-declarer playout** (`single_dummy_playout`) confirms it doesn't flip
  sign: plain DD is the optimist bracket, sd-declarer the pessimist, and a
  table result lies between. Calibration probe: `probe-sd-calibration`
  (per-level make-rates vs Pavlicek's actual-vs-DD table).

  Calibration (2026-07-16, seed 1784184395, 39,776 self-play contracts, vul
  none, 16 lead × 16 line worlds; OR = odds ratio, base-rate-free):

  | Level | n playout | DD mk% | + blind lead | + fallible declarer | OR(guess) | OR(vs DD) |
  | --- | --- | --- | --- | --- | --- | --- |
  | 1 | 500 | 64.2 | 71.8 | 61.6 | 0.63 | 0.90 |
  | 2 | 500 | 60.0 | 64.2 | 55.2 | 0.69 | 0.82 |
  | 3 | 500 | 56.4 | 63.2 | 55.2 | 0.72 | 0.95 |
  | 4 | 500 | 68.0 | 73.4 | 62.4 | 0.60 | 0.78 |
  | 5 | 391 | 66.2 | 70.3 | 60.6 | 0.65 | 0.78 |
  | 6 | 450 | 72.9 | 81.8 | 68.7 | 0.49 | 0.82 |
  | 7 | 16 | 87.5 | 87.5 | 87.5 | 1.00 | — |

  Two properties to know when reading verdicts. (1) The haircut is **genuine
  ambiguity, not sampling noise**: doubling the worlds to 32×32 on the same
  deals barely moves it (level 6 −13.1pp → −10.9pp, partscores unchanged), so
  k = 16 is the standard setting. (2) The playout is a **deep pessimist**, not
  a table simulator: its guess haircut (OR 0.49–0.72) is 2–4× Pavlicek's
  actual-vs-DD slam net (OR 0.88–0.95 at the 6-level), because the MC declarer
  conditions only on the auction and seen cards — no carding inference, no
  table feel — while the defense stays perfect. Both arms of an A/B wear the
  same haircut, so the *differential* read stands, but treat a bare sign flip
  at tiny magnitude as "suspect, trace the divergent boards", not as an
  automatic death sentence. The realistic arbiter is the analytic Pavlicek
  Δlogit shave in the slam-boundary addendum above; the playout is the
  lower bound.
- **Right-siding alone never wins on DD.** Both arms reach the same contract;
  only the declarer differs, and neither plain DD nor PD sees who declares.
  A convention whose only edge is right-siding measures ≈0 — don't trade real
  constructive value (an auto-drive-to-game, a weak relay) to gain it.
- **The new-information rule.** A constructive structure wins only if it adds
  information the auction doesn't already carry. Rebuilding "advancer bids over
  a takeout double" wins little — the double already advertised the fit. The
  same structure over a balanced 1NT (fits hidden) won big.
- **Self-play can't punish our own conventions.** The opposing book with the
  convention *off* can't double it or use the disclosure constructively —
  strength floors bias light. Use a reading opponent (BBA `--advertise-*`) for
  strength/range tuning. Verified: BBA changes its auction on ~46% of our
  convention boards when advertised, 0% in blind self-play.
- **Analytic probes omit the real routing.** Pricing "force 3NT vs pass/invite"
  for a hand that actually *transfers* to a making 5♣ overstated the force by
  ~7 IMPs/fired. Analytic baselines are valid only for hand classes that
  genuinely stop in the priced contracts (e.g. flat 4333).
- **A half-built convention measures as a loss.** Leaping Michaels was −0.6
  with advances left to the floor, +1.09 once `leaping_michaels_advances` was
  authored. A relay whose saved space is never *spent* (continuations missing)
  adds a doubled artificial target for nothing.
- **Scope artifacts.** An over-broad trigger (a bare `last_bid == 3NT`, a
  responder node shared between "our 1NT overcalled" and "advancing a takeout
  double") fires outside the hand class it was designed for and drags the
  measurement. *Dilution of the per-fired win is the tell.* Gate by context;
  re-measure.
- **Stale seeds oversample one slice.** `StdRng::seed_from_u64` is a fixed
  stream per seed; replaying 0..31 across experiments converges on that slice's
  quirks. Fresh `SEED_BASE` per experiment (see checklist 5).
- **Matchpoint-frequency effects.** A treatment can be +raw-points but −IMPs on
  a seed (frequent small gains vs rare big losses). When the effect is
  frequency-shaped (e.g. the 4333 Stayman suppression), raw points is the
  cleaner signal; report both.
- **PD-era vs plain-era figures don't compare.** The A/B harnesses moved from
  PD to plain DD scoring in 2026-06 (commit `a6f2206`). Ledger figures before
  that are on a different measure.

## Ship rules

- **Knobs**: every treatment gets a `set_*` toggle; CLI wiring in the A/B
  example and/or `bba-gen`. Rejected-but-interesting treatments stay as
  **opt-in knobs, default off, default system byte-identical** — especially
  obstruction-wall rejects, which are single-dummy re-measure candidates, and
  get an off-switch spelled `--no-ns-*` when shipped default-on.
- **Default-on** requires: plain-DD win, or plain wash + PD win, or additive +
  DD-invisible value (table above). **Plain-DD loss never ships default-on.**
- **The wash tiebreak — naturalness.** When two methods push each other (both
  scorers wash), *direction relative to natural bidding* picks the default. The
  default is the least-surprising agreement an unknown American / online partner
  already assumes, so a change that moves us **toward established natural theory
  ships default-on on a wash** — a push is enough. A change **trialling a
  convention** against natural (artificial call, e.g. Cachalot) needs a real
  plain-DD or DD-invisible win; a wash only earns an **opt-in knob**. Naturalness
  is a prior DD can't score (shared understanding, an unknown partner's default),
  and it is the same standing directive that keeps artificial 1NT defenses opt-in
  even when they match the default (convention-tuning.md). *Worked example:*
  `longer_major_response` — bidding the longer major on 5♠4♥ is the established
  American treatment and the arm measured a null, so the tiebreak flips it to the
  default; the unconditional-hearts-first simplification becomes the opt-in.
- Flipping a default that changes `american()` behavior: update the integration
  tests that encode the old default, and say so in the changelog.
- A default flip or new convention needs its **inference reading and alerts**
  shipped in the same change (see
  [bidding-architecture.md](bidding-architecture.md)) — an unread artificial
  call is a floor disaster waiting for competition.

## Harness inventory

Naming convention (see [README](../README.md#examples)): `ab-*` A/B matches,
`probe-*` diagnostics, `dump-*` data generation, `eval-*` evaluator
calibration. Reuse an existing `ab-*` harness before writing one; most new
questions are a flag on an old harness.

sd brackets on existing harnesses: `ab-dump-sd` scores aligned `bba-gen` dumps
with the blind lead (default) or the full sd-declarer playout
(`--sd-declarer`); `ab-slam-entry --sd` adds the playout as a third row beside
plain/PD. `probe-sd-calibration` is the bracket's own calibration (per-level
make-rates vs Pavlicek). The playout is sequential per board (no cross-board
pooling), so reserve it for divergent sets, and note the shared helper
`common::sd_declarer_ns_score` when wiring it into another harness.

New-harness rules (the Rayon pattern, commits `8f549ed`/`eadb654`):

- Deal generation sequential (seeded, reproducible); **bidding** parallelized
  with `rayon::par_iter` (classify is pure; `Stance` is `Sync`).
- The ddss `Solver` stays on the **main thread** — `Solver::lock().solve_deals`
  batches and parallelizes internally; never call it inside a worker.
- Thread-local knobs read at *book construction* are baked in; knobs read at
  *classify time* must be set inside the worker closure.
- Solve only the **divergent** boards; score both plain and PD from the same
  solved table (near-free — loop the summary over both swing vectors).
- Verify determinism: same seed twice → bit-identical summary.
