# The best (GTO) defense to a strong 1NT — a matrix-game tournament

**Status: measured 2026-07-03** on all three brackets — plain DD, perfect
defense, and the **sd-lead** single-dummy-lead scorer added later the same day
(results below; the sd-lead headline: Woolsey is the equilibrium at *both*
vulnerabilities). Harness:
[`examples/ab-nt-defense-matrix`](../../examples/ab-nt-defense-matrix/main.rs).

## Why "best defense" is a game, not an A/B

The value of a defense to 1NT depends on the opening side's *counter-strategy*:
how they run out of a doubled 1NT, what their responder's double of our overcall
means, whether they trap-pass and convert. A single A/B against one fixed
opponent yields a **best response** to that opponent, not an optimum. "GTO" is a
property of a strategy *pair* — an equilibrium neither side can leave profitably.

Bridge here is two-team zero-sum, so over a **finite menu** of strategies per
side the equilibrium is exact and computable:

1. Build the **payoff matrix** `M`: entry `M[i][j]` = expected IMPs/board to the
   defending side when it plays defense `i` against counter-strategy `j`, every
   cell on the *same* boards (common random numbers).
2. Solve the matrix game. For zero-sum games the Nash equilibrium coincides with
   the minimax solution (von Neumann); we use **fictitious play** — each side
   repeatedly best-responds to the other's empirical average strategy — which
   converges for zero-sum games, with the **exploitability gap**
   `max_i (M·ȳ)_i − min_j (x̄ᵀM)_j → 0` as the convergence certificate.
3. The resulting mixture is **GTO within the menu, under this scoring model** —
   the two qualifiers matter and are kept explicit throughout.

## The measure, and its known bias

The two DD scorers assume perfect double-dummy cardplay, bracketed per
`reference_pd-vs-plain-dd-bracket`:

- **plain** (`ns_score_contract`) — the reached contract with its actual penalty;
- **pd** (`ns_score_pd`) — a contract that fails double-dummy is priced doubled,
  real doubles are kept.

**The obstruction wall applies** (`project_preemption-dd-negative`,
[`1nt-defense-dont.md`](1nt-defense-dont.md)): perfect cardplay prices
obstruction, pressure, and "they sit and die" at exactly zero, and prior work
showed always-pass beats every defense on this measure. So the *expected* DD
equilibrium is pass-heavy, and the informative outputs are the relative
structure: which defenses come closest, under which counters, in which buckets.
This is **GTO of the DD game**.

### The sd-lead bracket (added 2026-07-03)

Where DD scoring is most wrong at the 1NT level is *known and measured*:
real declarers beat double-dummy below game and lose to it at slams, because
the DD defender always finds the killing **opening lead** while a real one
leads blind, and the lead matters most at low levels.  Pavlicek's actual-vs-DD
study ([rpbridge.net/8j45.htm](https://www.rpbridge.net/8j45.htm)): 1NT makes
**67.72% at the table vs 60.51% double-dummy** (+7.2pp), 3NT 69.84% vs 65.56%,
while slams flip to 66.34% vs 68.30%.  DD is systematically *pessimistic about
1NT* and *optimistic about slams* — and a tournament whose datum is "their 1NT,
played out" inherits the 1NT half of that bias in full: the datum is priced too
well for the defenders, so every active defense is handicapped against it.

The third scorer, **sd-lead** (`single_dummy_leads`), models exactly that seam
and nothing else: the opening leader chooses a card *single-dummy* — maximizing
mean defensive tricks over `n = 16` layouts consistent with the auction **as
the leader's own book reads it** (`Stance::infer`: trie-routed, alert-decoding,
so a Woolsey 2♥ samples as Muiderberg, not hearts) — and the play thereafter is
double-dummy on the actual deal.  One batched trick-one `Target::Legal` solve
per world prices all thirteen candidate leads; the actual deal rides in the
same batch and converts the chosen lead into declarer tricks.  Under this
scorer, *disclosure finally has a price*: an overcall directs partner's lead,
a penalty double sharpens the sampling posterior, and a silent pass leaves the
lead blind.  Cells are therefore compared at **auction** granularity (the same
contract reached through a different auction can get a different lead), so
sd-lead swings exist where plain/pd swings are structurally zero.

Its ceiling, stated: everything after trick one's first card is still perfect
on both sides, so declarer misguesses (the slam-level bias) and defensive
signalling are not modelled, and within each sampled world the play is still
double-dummy (strategy fusion).  Sanity anchor from the run itself: the datum
declarer takes **+0.31 tricks** more under sd-lead than plain DD (8.16 vs
7.85 over the sd-scored boards) — right in line with Pavlicek's +7.2pp at 1NT.

## The menus

Rows — our defense over their strong 1NT (all shipped knobs):

| row | config |
|---|---|
| always-pass | `set_always_pass_defense` — the DD incumbent, and the datum |
| natural | penalty-X (15+ balanced) + natural overcalls, the shipped default |
| DONT(6+) | `set_direct_dont` + 6-card one-suiter minimum (the parity config) |
| Woolsey | `set_woolsey` — X = 4M+longer minor, 2♣ majors, 2♦ Multi, 2♥/♠ Muiderberg |

Columns — their counters over our interference:

| col | config |
|---|---|
| default | shipped defaults: runout on, `DoubleStyle::Optional`, trap-pass, penalty-conversion |
| penalty-X | responder's double of our overcall = penalty (`DoubleStyle::Penalty`) |
| soft | takeout doubles, no trap-pass, no penalty conversion — never punishes |
| sit | the doubled-1NT runout disabled (`set_one_nt_runout(false)`, + universal) — they sit for 1NTx |

Every cell is scored against the (always-pass, default) datum on the same board,
so the always-pass row is identically zero and each entry reads "IMPs/board this
defense gains over doing nothing, under these counters". One DD solve per board
prices all 16 cells. Boards are kept iff EW actually opens 1NT (the auction
prefix up to the opening is cell-independent, so the kept set is too).

Noise handling: 95% CIs per cell; the equilibrium is **bootstrapped** over
boards (200 resamples → support frequencies and a value band), so a mixture that
flips inside the noise is reported as unresolved rather than believed.

## Rung 1 — best response vs BBA (fixed mature opponent)

The same four defenses vs BBA's 2/1 card (which itself defends 1NT with Woolsey
Multi-Landy and runs out of 1NTx with systems-on), via
`bba-gen --isolate-defense` (table B is the all-BBA reference, so the swing is
pure defense quality), 204.8k kept boards per arm, shared `SEED_BASE` so arms
pair. Natural and DONT advertise natural (the recorded honest protocol); Woolsey
runs unadvertised — BBA reads it natively as its own Multi-Landy.

## Results (2026-07-03, seed 1783025346, 60k boards/matrix, 204.8k/arm vs BBA)

### Self-play matrix, vul none (IMPs/board vs the always-pass datum)

plain DD:

| | default | penalty-X | soft | sit |
|---|---|---|---|---|
| natural | −0.051±0.024 | +0.040±0.023 | +0.099±0.023 | +0.006±0.025 |
| DONT(6+) | −0.115±0.027 | −0.069±0.026 | +0.051±0.026 | −0.084±0.026 |
| Woolsey | **+0.070±0.017** | +0.094±0.016 | +0.080±0.017 | +0.095±0.017 |

perfect defense: natural **+0.028±0.033** / +0.120 / +0.255 / +0.057 across the
columns; DONT −0.10..−0.32; Woolsey −0.05..−0.08.

sd-lead (blind opening lead, DD after; 16 worlds/lead):

| | default | penalty-X | soft | sit |
|---|---|---|---|---|
| natural | +0.068±0.025 | +0.077±0.024 | +0.131±0.024 | +0.100±0.025 |
| DONT(6+) | +0.066±0.027 | +0.064±0.027 | +0.167±0.026 | +0.075±0.026 |
| Woolsey | **+0.135±0.018** | +0.141±0.017 | +0.131±0.017 | +0.155±0.018 |

**Every active cell turns positive** the moment the opening lead is blind.
(Auction-level divergence is what sd-lead scores: 40.1% of boards for natural,
42.4% DONT, 18.9% Woolsey — larger than contract divergence because the same
contract reached through interference still gets a different lead.)

### Self-play matrix, vul both

plain DD under default counters: natural −0.148, DONT −0.324, **Woolsey −0.010**
(the only defense at parity); perfect defense: natural −0.070, DONT −0.528,
Woolsey −0.168. Every defense is at-or-below zero on the DD brackets —
vulnerable partscore competition is what double-dummy punishes.  **sd-lead:
natural −0.016, DONT −0.116, Woolsey +0.072±0.023** — Woolsey stays positive
even at unfavourable pricing, in every counter column (+0.07..+0.09).

### Equilibria (fictitious play, exploitability gap 0.0000 throughout)

| scenario | defense mixture | counter mixture | value | bootstrap support |
|---|---|---|---|---|
| NV, plain | **pure Woolsey** | pure default | +0.0702 | Woolsey 200/200 |
| NV, pd | **pure natural** | pure default | +0.0285 | natural 196/200 |
| NV, sd-lead | **Woolsey 0.96 · DONT 0.04** | soft 0.66 · default 0.34 | +0.1324 | Woolsey 200/200, DONT 32% |
| both, plain | pure always-pass | pure default | 0 | pass 88%, Woolsey 12% |
| both, pd | pure always-pass | pure default | 0 | pass 100% |
| both, sd-lead | **pure Woolsey** (0.99) | soft 0.68 · default 0.32 | +0.0713 | Woolsey 200/200 |

**The sd-lead headline: the vulnerability split vanishes.** On the DD brackets
"vulnerable → always-pass" looked like a law; price the opening lead honestly
and Woolsey is the equilibrium at *both* vulnerabilities, with always-pass at
**0/200 bootstrap support** in both scenarios.  The obstruction wall at the 1NT
level was, to first order, the blind-lead bias — the datum ("their 1NT, we
pass") was the cell most flattered by giving the defense a double-dummy lead.
A second structural shift: their counter equilibrium moves off pure-default to
a **default/soft mixture** — blind leads pay declarers on *both* sides, so the
doubled contracts their penalty machinery collects are no longer sure things,
and never-punishing becomes co-optimal.

Two structural findings beyond the headline:

- **Woolsey beats always-passing on plain DD at NV** — the first defense here to
  do so (every earlier "always-pass wins" verdict was natural/DONT-shaped). Its
  edge is structural: every call shows two known suits (accurate advances, +0.5
  to +1.0 IMPs/action-board on all four suit actions), it acts on only 13.7% of
  boards (vs natural's 28% — the marginal actions it skips were the losing ones),
  and it has no penalty X to feed the opener's runout.
- **The counter equilibrium is the shipped default package in all four
  scenarios** — Optional responder-doubles + trap-pass + penalty conversion +
  the doubled-1NT runout. "Sit" (runout off) is their worst counter: it flips
  our penalty-X bucket from −0.45 to **+2.04** IMPs/action-board, an independent
  confirmation that the runout — not the cardplay — is what rescues doubled
  openers. The soft column donates ~+0.15 IMPs/board to the defenders.

### Rung 1 — best response vs BBA (204.8k kept we-defend boards/arm)

IMPs/board vs the all-BBA reference (table B: BBA's own Multi-Landy defends), so
0 would equal BBA's defense; less negative = better:

| arm | NV plain | NV pd | both plain | both pd |
|---|---|---|---|---|
| always-pass | −0.299 | −0.321 | −0.411 | −0.478 |
| natural | **−0.148** | −0.425 | −0.371 | −0.736 |
| DONT(6+) | −0.160 | −0.348 | **−0.354** | −0.620 |
| Woolsey (ours) | −0.230 | −0.444 | −0.440 | −0.730 |

- **Against BBA, defending beats passing on plain DD at every vulnerability**:
  the always-pass arm trails BBA's active Multi-Landy by 0.30 (NV) / 0.41
  (both) IMPs/board. The obstruction wall is therefore partly a *self-play*
  artifact — our own counter package punishes interference well enough to make
  defenses look pointless, BBA's does not.
- Best responses: **natural at NV** (recovers half the passive deficit),
  **DONT at both-vul** on plain; on the pd bracket always-pass stays the best
  response vs BBA (BBA's doubles are perfect under pd, so every active arm gets
  maximally punished).
- **Our Woolsey trails BBA's Woolsey by 0.23 IMPs/board** (NV plain) on
  literally the same convention card — the gap is continuation/advance quality,
  not the card. That is the sharpest concrete improvement target this
  tournament surfaced (`project_our-woolsey-defense`).

## Interpretation

1. **On the DD brackets alone, "the best defense to 1NT" had no
   vulnerability-free answer** (NV → Woolsey plain / natural pd; vulnerable →
   pass), and the plain/pd disagreement at NV sat exactly where the doubling
   model matters (Woolsey's wide 8–19 overcalls are pd-fragile, natural's
   penalty-X package is pd-robust).  **The sd-lead bracket resolves the split:
   Woolsey, both vulnerabilities.** The DD verdicts were leaning on a measured,
   directional bias (the double-dummy opening lead), the sd-lead scorer removes
   exactly that bias and nothing else, and the answer stabilizes on the defense
   that was already winning the least-biased DD cell.  Confidence order for the
   NV headline is therefore sd-lead > plain > pd; for the vulnerable verdict
   the brackets genuinely disagree (pd's perfect doubling still says pass), so
   "Woolsey, both vuls" carries an sd-lead-trusting asterisk.
2. **The equilibrium lens changed the shipped-defaults question.** The
   defender's side of the shipped system (natural default-on) is the pd-bracket
   equilibrium; the opener's side (Optional doubles, trap, conversion, runout)
   is the exact counter-equilibrium in all eight DD scenario×scorer cells.
   Under sd-lead the opener's side softens to a default/soft mixture — the
   first measured crack in the "always punish" counter package, worth its own
   follow-up.  Defaults stay untouched for now: Woolsey remains opt-in
   (`set_woolsey`) at least until its continuation gap vs BBA (−0.23/board on
   the identical card) is closed — shipping the card before the continuations
   would ship the weakest version of the winning defense.
3. **What even sd-lead cannot say:** after trick one's first card, play is
   still perfect on both sides — declarer misguesses (the slam-level DD bias),
   defensive signalling, and within-world strategy fusion remain unpriced, and
   obstruction is still worth zero against perfect later play.  The BBA rung
   (active defense gains ~0.3 IMPs/board even on plain DD against a real,
   merely mature opponent) marks how much more is on the table.  The next
   scorer rung would be a blind *declarer line* (same MC-DD machinery from
   declarer's seat at trick two); the harness re-runs on it unchanged.

## Reproduce

```text
# the matrix + equilibrium, all three brackets (one run per vulnerability;
# --sd-worlds 0 to skip the sd-lead bracket, 16 is the default)
cargo run --release --example ab-nt-defense-matrix -- --count 60000 -v none
cargo run --release --example ab-nt-defense-matrix -- --count 60000 -v both

# rung 1 (one arm; see the flags table above for the others)
export SEED_BASE=$(date +%s)
scripts/bba-gen-parallel.sh out/natural 6400 --isolate-defense --advertise-natural
cargo run --release --features serde --example bba-score -- out/natural/shard-*.json --score plain
```
