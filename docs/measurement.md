# Measuring bidding changes — the A/B playbook

Every rule in this document was paid for with a wrong conclusion. Double-dummy
(DD) A/B measurement has systematic biases; a change that "measures +0.3" can
be an artifact, and a change that "measures −0.6" can be a good idea half-built.
Follow the checklist; the [biases](#known-biases) section explains each rule.

This doc answers *does this change ship?* Two sibling docs answer the adjacent
questions: [convention-tuning.md](convention-tuning.md) (*what is a convention's
best range, and which of its calls leaks?* — sweeping and per-call forensics)
and [ai-bidder/gto-1nt-defense.md](ai-bidder/gto-1nt-defense.md) (*which whole
method is best?* — the matrix-game tournament).

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
