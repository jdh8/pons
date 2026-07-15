# Tuning a convention — its best range, and fixing its leaks

Three docs, three questions:

| Question | Doc |
| --- | --- |
| Does this change ship? | [measurement.md](measurement.md) — the A/B playbook |
| Which whole *method* is best (natural vs DONT vs Woolsey…)? | [ai-bidder/gto-1nt-defense.md](ai-bidder/gto-1nt-defense.md) — the matrix-game tournament |
| **I've picked a method — what's its best range, and where does it leak?** | **this doc** |

Everything below is `1NT`-defense-flavored because that is where it was paid
for (the whole `project_natural-1nt-defense` saga), but the loop is
convention-agnostic — the last section maps it onto Stayman, Texas, transfers,
and minor keycard.

Prerequisite: read [measurement.md](measurement.md) first. This doc assumes its
checklist (knob it, complete both sides, paired seeds, dual scoring, reading
opponent) and only adds the parts specific to *tuning a parameter* and
*finding a leak*.

## Two axes

A convention that already exists gets improved along two axes, and they use
different tools:

1. **Range** — a strength/length knob: the overcall floor `(lo, hi)`, the
   penalty-double floor, the one-suiter length minimum, the invite/game
   boundary. You *sweep* it. This is where the obstruction wall bites hardest —
   §"Classify the range before you sweep it".
2. **Leaks** — a specific call that bleeds IMPs. You *bucket and forensic* it:
   which call, is it the action or the continuation, authored or floored.
   §"Per-call forensics".

Most "improve the natural defense" work is axis 2 (find the bad call, fix its
continuation); most "best range of a new defense" work is axis 1.

## The knobs already exist

Every tunable already has a thread-local `set_*` read at book construction and
a harness flag. Sweeping is `for v in range { set_x(v); build; score }` on
paired seeds. The 1NT-defense range knobs (`src/bidding/american/defense.rs`):

| knob | tunes | default |
| --- | --- | --- |
| `set_natural_overcall_points(lo, hi)` | natural suit-overcall strength | `(8, 14)` |
| `set_natural_double_floor(f)` / `set_natural_double_shape(s)` | penalty-X strength / shape gate | `15` / `Any` |
| `set_woolsey_points(lo, hi)` | Woolsey suit-overcall band (= Landy's) | `(8, 19)` |
| `set_woolsey_double_floor(f)` | Woolsey takeout-X floor | `12` |
| `set_direct_dont_one_suiter_min(n)` | DONT one-suiter length minimum | `5` |
| `set_direct_landy_double_floor(f)` | both-majors-X floor | `15` |

If a new defense needs a range, add its knob the same way (default off / default
system byte-identical, per measurement.md ship rules) before you can sweep it.

## Classify the range before you sweep it

**This is the load-bearing rule.** Double-dummy cannot tune a *competitive*
range. It can tune a *constructive* one. Decide which you have before you read
a single sweep number.

### Constructive range → DD is trustworthy

A boundary between two contracts the hands *actually reach* — invite vs game,
game vs slam-try, which strain, how high to raise. Both arms of the sweep bid to
real contracts DD prices correctly, so an **interior optimum is real** and the
sweep is worth running on self-play or an analytic probe.

The recurring outcome here is a **null**: raw HCP usually wins the boundary and
fancier evaluators wash.

- 1NT-invite boundary: raw HCP beat points/fifths/bumrap/cccc/controls
  (`project_nt-invite-evaluator-sweep`, `examples/probe-nt-invite-eval`).
- Texas slam boundary: responder `fit_value` beat raw HCP by +0.004,
  CI touching 0 — not significant (`project_texas-slam-eval`).

Don't over-read a null: a wash means *keep the simpler evaluator*, which is a
result, not a failure. Report the CI.

### Competitive range → the obstruction wall

A floor/length on an *overcall, preempt, or takeout double* — a call whose value
is obstruction, lead-direction, and table pressure. DD prices all of that at
zero while counting the overbid cost in full (`project_preemption-dd-negative`,
measurement.md §"The obstruction wall"). So the DD-optimal setting is always
**"compete less"** (or, against a fallible opponent it can exploit, "compete
more") — never a bridge-real sweet spot.

The tell is a **monotonic sweep with no interior peak**:

> "the overcall floor wants garbage — perfectly linear no-downside gradient =
> cleanest obstruction-wall proof yet, strong-shapely wants Pass, X dominated,
> balancing wants the passive floor. Only the *constructive* non-vul overcall is
> DD-trustworthy." — `project_natural-1nt-defense`

Raising the natural-overcall floor was *monotonically worse* (8:14 base, 10:14
and 11:14 strictly worse on plain DD) because the light 8–9-point overcalls are
the winners — against BBA, a fallible auction engine our obstruction pushes into
DD-bad contracts. That is **exploitation of a specific opponent**, not a real
range; discount it. In perfect-bidder self-play the same gradient points the
other way (compete less). Neither endpoint is the answer.

The 2026-07-15 probe is the same lesson caught in the act. *Widening* the band
(dropping the floor to 7, removing the 14 cap) looked like a win on plain DD
(`7:37` NV **+0.048**) and even on exact-disclosure sd-lead (**+0.062**) — but
every widened band is perfect-defense **negative** (`7:37` NV PD **−0.042**,
monotonically worse with width), and the vulnerable plain gain is statistically
zero. Re-gauging the identical bands to raw **HCP** or **CCCC** reproduces the
same inversion, so it is evaluator-independent — not a `point_count` quirk.

A follow-up cross sweep around `8:14` (both edges, both vulnerabilities) shows the
mechanism: plain and PD are anti-correlated at every band — no band is positive on
both — and the one apparent win, *tightening* the vul band, is **PD-only**. Its
value is monotone in assumed doubling severity — sd (no double) **−0.021** <
plain (BBA) −0.003 < PD (perfect axe) **+0.043** — so the perfect-double scorer
likes removing the light overcalls while the two scorers that do not assume a
perfect doubler say wash-to-loss. *Either* edge of the band looks good on the
scorer whose opponent-model flatters it: loosen wins on plain/sd, tighten wins on
PD, and `8:14` is the multi-scorer balance (the tell is a one-sided win, not a
band). Widening was **refuted** and nothing shipped — the default is the
disciplined uniform 8–14. Reproduce with `bba-gen --ns-overcall LO:HI`; full
per-gauge and sweep tables are in
[point-count-threshold-campaign.md](archive/point-count-threshold-campaign.md).

**How to actually price a competitive range:**

- **sd-lead** (`single_dummy_leads`, `--sd-worlds`) — the one bias DD gets
  measurably wrong at the 1NT level (the blind opening lead). It gives
  disclosure a price, so an overcall's lead-direction value finally registers.
  This is the right scorer for competitive ranges; it is what flipped Woolsey to
  the equilibrium at both vuls (`project_gto-1nt-defense`, gto-1nt-defense.md
  §"The sd-lead bracket").
- **vs BBA, exploitation discounted** — informative about *this* opponent, not
  GTO. State it as "beats BBA by tuning to its frozen range-model," not "the
  best range."
- **A bridge-sound partition beats chasing the DD peak** — see next.

## The orphaned-points trap

A competitive-range sweep will often peak just above a clean threshold. Do **not**
chase that peak. Raising the Woolsey overcall floor to 16 "peaked" on DD only
because it stranded 15-point both-majors hands into Pass — and passing them is
the worst thing they can do, which the obstruction wall then rewards. The peak is
the wall flattering an unsound hole in the point-count.

Prefer the **clean partition** instead: 8–14 overcall / 15+ penalty-X, no orphaned
range. `set_direct_landy_double_floor` shipped at **15** (not its DD peak of 16)
for exactly this — the partition "8–14 overcall = a direct bid, 15+ X = too
strong to overcall" leaves no point-count homeless, and it matched the peak
within noise anyway (`project_natural-1nt-defense`, the direct-Landy floor
sweep).

The natural-defense overcall band is a cautionary case: it *looks* like there is
a hole to close — the penalty double is balanced-only, so a capped suit band
leaves strong shapely hands in the owning `Pass` rather than routing them into
`X`. Widening the band to admit them won on plain DD and sd-lead, which is
exactly why it is a trap: the win is perfect-defense negative and evaluator-
independent (see above), i.e. it is BBA misdefending the extra overcalls, not a
real gain. The disciplined 8–14 cap remains the default. A monotonic
plain-positive / PD-negative sweep is the tell, whatever the evaluator says.

Corollary (an architecture invariant, [bidding-architecture.md](bidding-architecture.md)):
**every rule table needs a finite catch-all.** A floor set so high the table
rejects a whole point-band drops those hands to the deterministic floor, which
usually competes wrong. A sweep that looks like it wants a high floor is often
really telling you the catch-all is missing.

## Reading a sweep

| Sweep shape | Read |
| --- | --- |
| Monotonic, no interior peak | Suspect. Competitive range hitting the wall, *or* an orphaned point-band. The endpoint is not an optimum — re-price on sd-lead / vs a reading opponent, or pick the sound partition. |
| Interior peak, bridge-sound partition | Trust it (constructive range). This is a real optimum. |
| Flat plateau within CI | The knob is inert on this measure. Pick the simplest/soundest value and move on — don't over-fit noise (the DONT `6:2` doubled-escape gate: the whole cluster tied, only the extreme was clearly worst). |
| Self-play and vs-BBA disagree on direction | Real, and expected. Self-play can't punish or exploit; vs-BBA can do both. State which opponent each verdict is *for*. |

Sub-0.1 IMPs/board is noise unless the sample is hundreds of thousands of boards
(measurement.md decision table). A monotonic gradient of ~0.001/board across the
whole range is second-order — the first-order facts (which method, does it beat
always-pass) rarely move with the knob.

## Per-call forensics — improving a method, not just picking one

To make natural or DONT *better* (not to pick between them), find the one call
that leaks and fix it. The loop:

1. **Bucket by the first defensive call.** `bba-score` re-buckets the we-defend
   boards into `DIRECT (we bid over 1NT)` and `CONT <call>` per first action
   (`examples/bba-score/main.rs:210`). Split conventions that share a syntactic
   call — the defensive `X` bucket was hiding two different conventions until it
   was split into `X (pen)` (direct 15+ penalty) vs `X (PH Landy)` (passed-hand
   both-majors) on the doubler's lane; the combined bucket lied about which one
   was leaking.
2. **Is it the action or the continuation?** A negative bucket is not
   automatically the obstruction wall. Dump the worst boards. The penalty-X
   bucket read as a "−1.2/board single-dummy wall" for weeks; worst-board
   forensics showed most of it was the *doubler mis-pulling its own penalty X*
   — the keyless floor's `we_have_not_bid()` overcall rules fire over the
   opponents' escape (a Double isn't a bid), so a flat 15 doubles, they run, and
   we bid 3NT into a busted partner. Gating those rules off after a penalty latch
   (`set_penalty_no_pull`) moved the both-vul bucket **−2.086 → −0.608 IMPs/X-board**
   — a fixable continuation bug, not a wall.
3. **Authored or floored?** `Trie::classify_with_provenance` (`book.rs:406`)
   tells you: `depth:0, fallback:Some` = the call came from the deterministic
   `instinct()` floor, not your authored node. A bad floored continuation is
   fixed by improving the floor or authoring the node; a bad *authored* one is
   your rule (`feedback_instinct_floor_over_node_authoring`).
4. **Fix the continuation, re-measure paired.** The dominant leak of a
   convention is almost always a missing/ wrong continuation, not the action —
   this is measurement.md's "a half-built convention measures as a loss" seen
   from the tuning side. DONT's whole deficit was one missing escape node
   (`1NT X XX 2♣ X P P P → 2♣x` on a misfit); authoring the doubler's real-suit
   rebid over a doubled relay was worth +0.083/board.

### Two harness techniques (from the point-count campaign)

Both generalize the loop above; full worked examples in
[archive/point-count-threshold-campaign.md](archive/point-count-threshold-campaign.md).

- **Seat-parity worst-board replay.** To dump *many* divergent boards (step 2),
  replay the flagged run with a large `--show N` (e.g. `--show 2000` — the
  default `--show 40` cuts off the long tail) and resolve which arm made which
  call by **seat parity**: the candidate sits EW at the `off` table, so a
  mirror-labelled bucket is the same divergence seen from the other side. Parse
  the acting hand's HCP/shape on each board to name the mechanism.
- **Fix-vs-shipped, both arms on the shipped config.** When the fix is a
  build-time gate knob, measure it as *fix-vs-shipped* — both books built on the
  already-shipped scale/config, differing **only** in the gate under test
  (`ab-point-count --fix <spec>`, the `Arms::GateFix` two-book path). Never
  fix-vs-legacy: an `hcp` swap changes the *legacy* arm's behaviour too (legacy
  `points ⊇ hcp` on floors), so fix-vs-legacy confounds the gate with the scale.
  This isolates a gate-precision fix (every bracket positive, no PD dip) from an
  aggression trade.

**Iron rule, from measurement.md:** *before declaring a measured loss dead,
trace the worst divergent boards.* The usual culprit is an unauthored
continuation or an over-broad trigger, and it is fixable — the wall is only what
survives after the bugs are out.

## When a DD gain still doesn't ship

Tuning can find a real DD gain you still reject — the measure isn't the only
constraint:

- **Disclosure.** The both-majors takeout-X ("Landy X") beat penalty-X on DD
  vs BBA by ~0.036/board, and was **dropped** — a both-majors double of 1NT is
  not honestly disclosable as a penalty double, and disclosability overrides the
  gain (`project_natural-1nt-defense`, "LANDY-X DROPPED"). Ship the honest card.
- **Soundness over the peak** — the orphaned-points trap above: the bridge-sound
  partition wins even at a small DD cost.
- **jdh8's standing directive** — an artificial 1NT defense stays opt-in even
  when it matches or beats the default on the honest measure ("at most opt-in; I
  still have ideas for the natural defense"). Range tuning does not change a
  default-off convention's default.

## This generalizes to any convention

Same two axes, same tools:

| Convention | Range axis (sweep) | Leak axis (forensic) |
| --- | --- | --- |
| Stayman | invite/game boundary via `fit_value` (`project_stayman-fit-value-raise`) — constructive, DD-trustworthy | missing continuation = missed slam (`project_stayman-slam-deficit`) |
| Texas / SAT slam | slam-try boundary evaluator sweep (`project_texas-slam-eval`) — constructive, mostly null | — |
| Jacoby transfers | super-accept strength — *competitive*, so opt-in (`project_transfer-competition`) | GF structure completion (`project_transfer-gf-structure`) |
| Minor keycard | — | every rule table needs a finite catch-all (`project_minor-keycard`) |

The rule of thumb transfers intact: **a boundary between two contracts the hands
reach is constructive (sweep it, trust the peak); a floor on a call whose value
is obstruction is competitive (the wall — sd-lead or a discounted vs-BBA read,
and prefer the sound partition).** Classify first, sweep second.

## The parameter-sweep checklist

1. Add the `set_*` knob + harness flag; default off / default system
   byte-identical (measurement.md).
2. **Classify the range**: constructive (contract boundary) or competitive
   (obstruction). This picks the scorer.
3. Constructive → sweep on self-play or an analytic probe; the interior peak is
   real; expect a null (raw HCP wins). Competitive → **sd-lead**, or vs-BBA with
   exploitation discounted; a monotonic gradient is the wall, not an optimum.
4. Paired seeds across the sweep (`SEED_BASE` once, reused), dual scoring.
5. Read the sweep with the table above. Prefer a bridge-sound partition to a
   DD peak that strands a point-band.
6. Disclosability and soundness can veto a real DD gain — ship the honest card.
