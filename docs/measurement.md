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

## Watching a run

A long run is watched by **polling the runner process**, never by grepping the
log for the script's own success line:

```sh
setsid nohup scripts/idle-run.sh scripts/ab-my.sh ab-results/my >ab-results/my.log 2>&1 &
PID=$(pgrep -f ab-my.sh | head -1)
while kill -0 "$PID" 2>/dev/null; do sleep 20; done
tail -5 ab-results/my.log           # then read the diff files in a *separate* command
```

Two failure modes this avoids, both paid for on the Gladiator v6 run:

- **A dead run never writes its "done" line.** `while ! grep -q "A/B done"`
  waits forever on the one outcome you most need to hear about. v6 was killed
  at 18:57 by a mid-run `cargo build` picking up a broken tree (see item 6
  above); the string-watcher was still waiting minutes later. Since then
  `idle-run.sh` also prints `idle-run: <script> exited <status>` on *every*
  path, so a log tail is conclusive even without the PID — but the absence of
  the job's own done line beside a non-zero status is what names the failure.
- **Chaining the report into the watcher loses the report.** Killing a stuck
  watcher throws away the `ab-dump-diff` output it was going to print. Watch,
  then read the results in a separate command.

## Scorers (`src/scoring.rs`)

| Scorer | What it scores | Use for |
| --- | --- | --- |
| `ns_score_contract` | Plain DD: the reached contract with its *actual* table penalty. | Duplicate A/B results — the default verdict. |
| `ns_score_pd` | Perfect defense: a contract that fails DD is scored **doubled** (synthetic X), making ones undoubled. | The pessimistic bracket end: "scored against a competent doubler." |
| `ns_score_bid` | Perfect defense, takes a `Bid` (derives the penalty). | Evaluating a **call** (EV rollouts, contract-choice probes) — never for A/B results. |
| `ns_score_tricks` | **Plain SD**: an explicit single-dummy trick count priced at the contract's *actual* penalty. | The pricing tail of the SD scorers. **Never a verdict on its own** — see below. |
| `ns_score_pd_tricks` | **SD-PD**: the same trick count, but a contract that *fails on those tricks* is scored **doubled**. | The SD arbiter. Report it beside plain SD everywhere the SD bracket is quoted. |
| `single_dummy_leads` (`src/single_dummy.rs`) | MC-DD with a *blind* opening lead chosen from the leader's sampled worlds. | The one known DD bias at 1NT level (DD defenders always find the killing lead, ~+0.3 tricks to 1NT declarers). Re-score close NT-defense verdicts with it — under **both** SD scorers. ⚠ **At slam level this is an UPPER bound** — it removes the lead pessimism and keeps all of DD's play optimism (Pavlicek after-lead: +7pp on slams). Never read it as slam insurance. |
| `single_dummy_playout` (`src/single_dummy.rs`) | The **sd-declarer playout**: blind lead, then declarer chooses every card MC-DD over auction-consistent worlds (show-outs remembered) while the defense plays DD on the actual deal. | The slam-side DD bias (see below): a DD declarer never misguesses, so every DD-play scorer is *optimistic* for the arm bidding more slams. Runners: `ab-dump-sd --sd-declarer`, `ab-slam-entry --sd`. Sequential per board — divergent sets only. Its haircut is ≈1.5× the real one (see Known biases) — the LOWER bound. |
| `sd_blend_imps` (`examples/common/mod.rs`) | The **sd-blend**: one composed run (`single_dummy_declarer_tricks`) returns both endpoints' trick counts, and the blend takes the playout outcome with probability λ(level), the lead-endpoint outcome otherwise, mixed at the IMP level (four `imps` terms — the IMP table is nonlinear). λ per level in `common::SD_BLEND_LAMBDA`, fitted by `probe-sd-calibration`'s λ block so the blend applies exactly Pavlicek's **after-lead** declarer-fallibility shift to the lead endpoint. | The calibrated **point estimate between the two sd endpoints** — built for slam stop-vs-bid buckets, where the decision band is 45–55% make at the 6-level. Grands: quote blend *and* analytic shave as a bracket, never a point (thin data, bigger bias). Validation: `probe-slam-battery` (DD-fair archetypes must not move; third-eye ones must grade). |

**The principle** (jdh8, 2026-06-24): *the threat of a double is a legitimate
deterrent, but a double that never appeared on the table must never enter the
final score.* Hence the bracket: truth sits between plain DD (under-punishes
overbids) and PD (over-punishes — a perfect doubler never doubles a making
contract). Reality is closer to plain DD.

**Plain SD is not an arbiter — it is half a bracket** (2026-07-25). The DD
scorers come in pairs for a reason, and so must the SD ones. Plain SD relaxes
the defenders' opening lead (which *helps declarer*) **and** keeps every failing
contract undoubled (which also helps declarer): it is optimistic on *both* axes
at once, and so is strictly friendlier to aggression than plain DD. That is why
it reliably rehabilitates treatments perfect defense has just killed — for years
a plain-SD win was read as overruling a PD loss, and it shipped several defaults
on that reading.

The fix is the pair **[SD-PD, plain SD]**, exactly mirroring **[PD, plain DD]**:
`ns_score_pd_tricks` keeps the realistic lead but restores the doubled downside,
so the two SD numbers bracket the truth the same way the two DD numbers do.
**Quote both or quote neither**, and read the verdict from SD-PD. A plain-SD win
with an SD-PD loss is the same doubling artifact the decision table already names
on the DD side — not a rehabilitation.

Two caveats on SD-PD. *Level realism*: doubling a failing contract is what a real
opponent does at partscore and game, and is not what happens at slam — nobody
doubles a voluntarily bid six — so SD-PD is a genuine arbiter below slam and a
pessimism stress-test above it. *Identical contracts*: where an A/B's arms reach
the **same** contract and differ only in the cards played (the sd-declarer
playouts, and `ab-dnf-sd-lead`, whose arms vary only the leader's sampled model),
both SD scorers are nondecreasing step functions of one trick count, so every
board's swing keeps its sign and SD-PD adds no independent signal. A verdict
resting on such a harness alone needs a plain+PD re-adjudication, not an SD-PD
row.

**Known limitation — `ab-dump-sd` does not price disclosure** (measured
2026-07-26, unfixed). The dump-rescore harness reads each arm's auctions for the
blind leader via `partnership.infer(relative(vul, leader), …)`, and its `--on-ns-*`
flags are meant to make the leader read the ON arm under the ON arm's system.
**They are inert.** Rescoring `nt-defense-band-9-14` with `--on-ns-overcall
20:37` — claiming the overcaller holds 20–37 HCP rather than 9–14 — reproduces
the flagged run to the last IMP, and so does dropping
`--on-ns-negative-double-shape modern` from `modern-negx`. It is therefore not a
natural-vs-alerted split: no setting on *our* book reaches the leader's model.
The leader is an opponent, so the prime suspect is that inference from the
opposing seat never consults our book (cf. the wrong-seat trap in
`dnf-migration.md`).

Consequences, in order: verdicts from dump rescores **stand** (both arms are read
identically, and the arms still differ in the auctions and contracts the dumps
recorded, so the treatment effect is real); the claim that a dump rescore prices
disclosure or lead-direction value **does not**; and any future harness that
needs disclosure priced must verify it moves the numbers before trusting it — the
one-line check is to re-run with a deliberately absurd band and confirm the
output changes. Fixing it would move every sd number the script has produced, so
it is flagged rather than patched.

**The four brackets are a 2×2, not a ladder** (2026-07-26). It is tempting to
rank them by how harshly they treat the bidding side — sd < plain < PD — and
call a win visible only at the top an artifact. That reasoning shipped a real
verdict (the `8:14` vul overcall band) and is **wrong**, because the brackets
vary two independent things:

|  | fallible/no doubling | perfect doubling |
| --- | --- | --- |
| **double-dummy lead** | plain DD | PD |
| **blind lead** | plain SD | **SD-PD** |

Measured on that band, the two axes separate almost exactly: the doubling axis is
worth the same under either lead model, and the lead axis costs the same under
either doubling model (archive/point-count-threshold-campaign.md). **Plain SD
differs from PD in both coordinates at once**, so a plain-SD↔PD gap tells you
nothing about which axis produced it — that is the whole reason the chain looked
convincing. Two rules follow:

- **Compare along one axis with the other held fixed.** Doubling effect = PD −
  plain DD, or SD-PD − plain SD. Lead effect = plain SD − plain DD, or SD-PD −
  PD. Never plain SD − PD.
- **Plain SD is not the realistic end of anything.** On the doubling axis it is
  the most extreme model available (nobody ever doubles), hence *less* realistic
  than plain DD's fallible doubler. The honest realism pair is **[plain DD,
  SD-PD]**; a treatment those two agree on needs no further argument.

**Reading the SD pair: three outcomes, not two** (2026-07-25, from the first
re-adjudication batch). SD-PD is not a uniformly harsher scorer — it is harsher
on *whichever arm bids the failing contract*. So comparing the two SD rows sorts
a verdict three ways, and the third is easy to miss:

| SD-PD vs plain SD | Reading | Worked example |
| --- | --- | --- |
| ≈ equal (keeps most of its magnitude) | Real effect; plain SD happened to be right | `set_meckstroth_adjunct` — +0.0097/+0.0146 → +0.0073/+0.0120, ~75-80% retained, CI-clear |
| **collapses, often sign-flipping** | The win *was* the missing doubling | `set_forcing_nt_two_suiter` — +0.0005/+0.0018 (CI>0) → **−0.0011 (CI<0) / +0.0000** |
| **higher than plain SD** | Plain SD *understated* it: the treatment is the sounder bidder, so restoring the doubling punishes the **baseline** more | `set_notrump_minors` (Puppet) — +0.0003/+0.0005 (straddles 0) → **+0.0006/+0.0010, CI-clear both** |

The first two rows also kill a grouping the ledger used to make. Knobs whose PD
loss was "redeemed by sd" were filed as one profile; the batch split them —
one kept its win, the other inverted. *"Plain SD rescued it"* was never a
profile, only a coin flip between a real effect and an artifact, and nothing but
SD-PD separates the two.

**Check dump provenance before trusting a rescore** (2026-07-25). Re-scoring
stored arm dumps is the cheapest way to re-adjudicate an old verdict — no
re-bidding, so a moved book cannot contaminate it. The failure mode is that the
dumps may not be the arms you think. A driver that reuses its results directory
regenerates arms into it, and **nothing in the output announces the overwrite**;
`ab-dump-sd` happily pairs an ON arm from one experiment with an OFF arm from
another and prints a confident number. Worked example: `ab-results/free-bid-style`
holds a v2 `negative` arm (sha 3d4fac3, the reverted tempering, shard mtime
19:36) against a v1 `forcing` arm (sha 6d8b0ab, mtime 19:05), because the v2
campaign re-ran `arm negative` into the v1 directory. Rescoring it reproduces the
*v2* verdict, not the v1 one it appears to name — the v1 arms no longer exist, so
that verdict is unre-adjudicatable and needs a fresh A/B.

So: before reading a rescore, (1) compare shard mtimes across the two arm dirs —
they should match the same run, and (2) check the published figure reprints.
Both gates fired usefully in this batch: `set_floor_rkcb` reprinted (+2.36
published vs +2.414 measured per fired) and was trustworthy; `free-bid-negative`
did not, and the mtimes explained why.

**Measure bidding decisions, not gauges** (2026-07-25). When the re-adjudication
queue was drawn up it mixed two kinds of row: knobs that change *which call we
make*, and knobs that merely elect *which scalar strength gauge* the ranges are
denominated in (point scale, HCP-vs-CCCC bands, Fifths-vs-HCP). Only the first
kind is worth DDS-hours. The reading layer now carries several strength axes
side by side on the same `Envelope` — `hcp`, `support_points`, `suit_hcp` — so
the evaluator question has stopped being "which fused scalar wins" and become
"which features does the net get"; electing one scalar by A/B optimises the
wrong layer. Drop gauge-election rows from a queue and record them as
*superseded, not measured*, with the reason.

A second, sharper filter applies to any re-measure: **if a default has since
been re-justified by a newer and independent measurement, its old verdict row is
stale evidence and no re-measure can move it.** `set_fuzzy_points` is the worked
example — plain SD shipped it over a negative PD bracket, which is exactly the
artifact above, but its default now rests on the `277059f` scale flip
(plain-DD wash, PD +0.023/+0.037), not on that plain-SD row. Re-measuring it
would produce a number that changes nothing.

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

**Since 2026-08-03 the slam bracket has a calibrated point estimate — the
sd-blend** (`common::sd_blend_imps`; scorer table above). The verdict still
comes from plain + PD; what the blend adds is attribution *inside* a
slam-heavy divergent set: the stop-vs-bid boards DD scores as a coin flip get
a graded make estimate (λ-mixture of the guess-blind and guess-aware
endpoints), so a bucket's loss can be read as "genuine 45–55% judgment calls
resolved against us" versus "accidents no scorer rescues". Small slams are
where the estimate is trusted (the 50% bar under both IMPs and MPs
concentrates the accuracy budget on the 45–55% band); grands are always
reported as the [blend, shave] bracket.

Sub-0.1 IMPs/board is noise unless the sample is large (hundreds of thousands
of boards); a *fired-rate*-weighted per-fired figure with a CI excluding 0 is
the stronger claim. On contested/filtered harnesses compare **IMPs/divergent**,
not IMPs/board — `--filter` biases the per-board denominator, not the
divergent set.

## Enriched probing — when the trigger is too rare for random deals

A random-deal A/B prices a change by burying it in a million boards it cannot
touch. That is the right default when the trigger fires on a percent or more of
boards. Below roughly **10 boards in 10⁴** it stops working: the divergent set is
a few hundred boards, the CI swallows any real effect, and the run spends
essentially all of its CPU bidding and solving deals that were never going to
diverge.

The route for those: **reject-sample to the trigger, then score conditionally.**
Worked example, `examples/probe-weak-two-major` (responder's forcing new suit
over a weak two — a 10⁻⁴ and a 6×10⁻⁴ window).

**The accept test runs on the raw hands, before the bidder.** That is the whole
saving, and it dictates the filter order, because the pipeline's costs are
orders of magnitude apart:

> dealing ≈ free  <  bidding  <  double dummy  ≪  single-dummy playout

1. deal (microseconds) and test cheap hand predicates — shape, point count,
   honors — on the two seats the change involves;
2. bid **only** the survivors, both arms;
3. confirm the auction actually reached the face (the hand predicate only
   approximates the book — over half of the accepted deals may not open what you
   expected);
4. double-dummy **only** the boards whose contracts differ.

**Conditioning costs the duplicate swap.** Accepting on a named seat means the
table-A/table-B rotation no longer measures the same thing; the comparison
becomes the same deal bid twice, our side feature vs baseline, opponents fixed
on the baseline partnership both times.

**Reading the verdict.** The headline is IMPs per *accepted* deal, and the
[decision table](#the-decision-table) applies to it unchanged — plain DD and PD
both, same shapes, same traps. What does *not* transfer is comparability: a
conditional +3 IMPs/deal is not a +3/board result. Publish the per-board
equivalent alongside it,

> per-board = conditional mean × trigger density,

with the density measured by the probe itself (boards reaching the face ÷ draws)
— scale the CI bounds the same way. Stack **that** number against the campaign
ledger, never the conditional one.

**Caveats.** The conditional population is exactly as good as the accept
predicate: too narrow and the measurement answers a question nobody asked, too
wide and the enrichment evaporates. State the predicate in the harness doc
comment. And a conditional CI is tight enough to over-read — a significant
conditional effect on a 10⁻⁴ trigger is still worth ~10⁻⁴ of it per board.

Leave the single-dummy row out of the first pass. If the verdict lands on the
plain-wash/PD-gain row, or anywhere ambiguous, re-run SD over a couple of
thousand accepted deals — enrichment is what makes that affordable at all.

## Known biases

These produced actual wrong conclusions; each has a memory/ledger trail.

- **Series breaks: the default floor has moved twice.** Almost every `ab-*`
  harness builds `american()` as its baseline without naming a floor, so a
  floor swap silently re-baselines all of them. Within one A/B this is harmless
  — both arms rebuild from the same source — but a number quoted *across* a
  swap is comparing floors, not treatments. The two breaks: **2026-07-19**
  (`instinct()` → the v3 BBA-distilled net) and **2026-08-05** (v3 → the
  configured `american_bba_v4`, which reads both convention cards). Numbers in
  `docs/bidding-options.md` marked `fresh` before 2026-08-05 keep their deltas
  but no longer describe the shipped population.

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

  **The two clairvoyances, decomposed** (Pavlicek's after-the-lead table,
  fetched 2026-08-03 — the half of 8j45 the paragraph above could not use).
  Full-deal DD nets a killing *lead* (pessimistic for declarer) against
  clairvoyant *play* (optimistic), and at the 6-level they nearly cancel —
  which is why "DD is almost calibrated at six" above is true but misleading.
  Conditioned on the **actual** opening lead, the play-side bias stands alone:

  | level | actual mk% | DD-after-lead mk% | shift (log-odds) |
  | --- | --- | --- | --- |
  | 4 | 66.66 | 71.02 | −0.203 |
  | 5 | 49.84 | 52.87 | −0.121 |
  | 6 | 66.70 | 73.67 | **−0.334** |
  | 7 | 64.84 | 72.14 | **−0.339** (n = 768) |

  (Full per-level table, with counts, typed into `probe-sd-calibration`.)
  Three consequences. **sd-lead is an UPPER bound at slam level** — it is
  exactly the "DD-after-lead" column's bias position: realistic lead,
  clairvoyant play, +7pp / +0.21 tricks at the six level. It exists to fix
  the 1NT seam and it does; it must never be quoted as slam insurance.
  **The playout is the lower bound** — its measured haircut (−0.49 log-odds
  at L6, fitted 2026-08-04 on the fixed playout; the pre-fix scorer measured
  −0.71, ≈2.1×, most of the excess being the current-trick sequence-collapse
  bug) is ≈1.5× the real after-lead quantum (−0.334). **The sd-blend
  interpolates between them**: λ(level) is fitted so the blend shifts the
  lead endpoint's make-logit by exactly the after-lead column
  (`probe-sd-calibration` prints the fit; `common::SD_BLEND_LAMBDA` carries
  it — fitted λ by level, 1–7: 0.089 / 0.242 / 0.298 / 0.539 / 0.474 /
  0.664 / =λ(6)). The after-lead shift is flat across 6 and 7 (−0.334 vs
  −0.339), so λ(7) inherits λ(6) by design — the fit's own 0.696 rests on 54
  grands, and the grands' extra full-deal bias lives on the lead side, which
  the shared lead endpoint models itself. A per-board caveat rides along:
  `probe-slam-battery` measures the playout leaking 20–40% of seeds on
  DD-fair slam deals through mean-max line detours, so the blend is honest
  on population averages while individual boards carry that variance.
  **Holdout-validated** (2026-08-04, fresh seed 1785789480, 39,648
  contracts): the shipped λ applied to endpoints it never saw reproduces the
  Pavlicek targets within 0.6pp at every level ≤ 6 (binomial noise ≈ 1.5pp
  at n = 1000), and the λ(4) > λ(5) inversion replicates — real 5-level
  structure, not sample noise. λ is a property of the **16-world**
  emulation (the misguess rate shrinks with worlds); re-fit before changing
  the world count.
- **λ is bucketed by level, but level is not the whole story — and his cells
  are not our cells.** Split Pavlicek's per-contract table by strain class
  (`probe-sd-calibration` now fits `(level, m/M/NT)` cells, 2026-08-04, seed
  1785794459, 158,534 contracts, 600 playouts per cell), and the after-lead
  shift spans −0.075 (4m) to −0.217 (4M) *within* level 4; 6NT (−0.396) is the
  most DD-optimistic cell on the board, more so than any suit slam (6m −0.318,
  6M −0.341). But the per-cell fit also exposes why importing those targets is
  mostly **illegitimate**: the `align` column (our sd-lead make% − his
  DD-after-lead make%, both DD play after a real lead, so equal populations
  must agree) misses by −28pp at 6m, −9.7pp at 5m, −5.9pp at 4M, +8.5pp at 3m.
  Expert contract *choice* selects the hands — they bid 5m only when the hand
  screams it, take saves we never take, and reach 3-level partscores
  competitively where ours are constructive — so a cell label alone does not
  identify the same population. **Only import a cell whose `align` is within
  ≈2pp.** Two things survive that filter and matter: (i) the 4M-guessier-than-5m
  ordering *replicates on our own population* in the corpus-free column — our
  playout's own haircut is −0.321 at 4M vs −0.262 at 5m (log-odds), a different
  hand-selection and a different declarer model reproducing his ordering, which
  is what answers the "his corpus is skewed toward easy 4M" objection; and
  (ii) at the slam level the two aligned cells are exactly the ones the blend
  exists for — 6M (align −0.8pp, λ 0.554) and 6NT (align +0.8pp, λ 0.457),
  against shipped λ(6) = 0.664 fitted with 6m in the pool, whose λ clamps to
  1.0 on a population that does not transfer at all. Fitting on 6M+6NT alone
  gives **λ(6) = 0.510**, i.e. the shipped blend is ≈0.15 too pessimistic on
  slam stop-vs-bid — the kickback §7.15 decision cell. Owed before that number
  ships: a fresh-seed holdout of the aligned-cell fit. One discrepancy stands
  unresolved: this run's level-4 pooled λ is 0.644 against the shipped 0.539
  (≈1 se at these counts, but larger than the ≤0.02 the first holdout moved).
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
(`--sd-declarer`); `ab-slam-entry --sd` adds the playout as rows beside
plain/PD; `ab-point-count --sd` is the `.pdd`-bank harness. **Every one of them
now prints the SD pair** (plain and perfect-defense) from a single trick count —
`common::report_sd_brackets` does the dual print, and
`common::sd_declarer_ns_score` returns `[plain, pd]`; wire both when adding SD
to another harness. `probe-sd-calibration` is the bracket's own calibration
(per-level make-rates vs Pavlicek). The playout is sequential per board (no
cross-board pooling), so reserve it for divergent sets.

Enriched (rejection-sampled) probes score a *conditional* population — see
[Enriched probing](#enriched-probing--when-the-trigger-is-too-rare-for-random-deals)
for when to reach for one and how to read it. Worked example:
`probe-weak-two-major`.

New-harness rules (the Rayon pattern, commits `8f549ed`/`eadb654`):

- Deal generation sequential (seeded, reproducible); **bidding** parallelized
  with `rayon::par_iter` (classify is pure; `Partnership` is `Sync`).
- The ddss `Solver` stays on the **main thread** — `Solver::lock(None).solve_deals`
  batches and parallelizes internally; never call it inside a worker.
- **Arm the knobs, then build; one partnership per arm.** Both kinds of knob —
  book-construction and classify-time — are captured into the `Partnership` at
  build, so a `set_*` inside a worker closure reaches nothing and both arms
  bid identically (a clean wash on every board, meaning nothing by it). If an
  arm must differ only at eval time, build both on the defaults and edit each
  one's own pin with `Partnership::profile_mut`.
- Solve only the **divergent** boards; score both plain and PD from the same
  solved table (near-free — loop the summary over both swing vectors).
- Verify determinism: same seed twice → bit-identical summary.
