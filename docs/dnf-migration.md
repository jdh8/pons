# DNF migration ledger

Campaign: every authored bidding constraint yields a tight union-of-boxes
(`Dnf` of `Envelope`) reading instead of hulling to ⊤ at `Or` — then flip
`set_dnf_reading` on after measurement. Why readings collapse today and how to
read a call off the bidder instead:
[ai-bidder/sampled-projection.md](ai-bidder/sampled-projection.md). Layer
invariants: [bidding-architecture.md](bidding-architecture.md).

This file is the **ledger only**: what landed, under which knob, with which
verdict. Design lives in the docs above and the code.

## The migration rules

- New projection precision that changes a **knob-off hull** sits behind
  `dnf_reading()` (precedent: `AnyLen::project`). Multi-box values are born
  via `Dnf::disjoin` (knob-gated), never raw `union` — `Dnf::intersect` does
  not consult the knob and would prune knob-off.
- **The wrong-seat trap** (found by the C2 dump diff): a legacy composite
  whose legs read the auction (`support`, `!support`) replays its projection
  under the *reader's* context, so its knob-off reading is
  **context-sensitive** — ⊤ at one point of the auction (the leg contradicts
  its own `len` and the empty product widens out; this, not the `Or` hull
  alone, is the mechanism of the measured "every 2/1 reads 0..=37" bug) and a
  wrong-suit box at another. No static box list reproduces that, so a
  re-authoring must keep the legacy constraint for eval/describe/knob-off
  reading and swap in the boxes knob-on only — the `dnf_upgrade` adapter
  (constraint.rs). The per-rule ratchet cannot catch reader-context effects
  (it projects under the authoring-time context); only the `bba-gen` dump
  diff can. **Run the dump diff for every re-authored gate.**

## Knob matrix

| Knob | Default | Gates |
| --- | --- | --- |
| `set_dnf_reading` | off | `Dnf::disjoin` (build), `Inferences::admits` (accept), `Dnf::tidy` (hygiene), every ⊤→boxes projection upgrade (`shapes`, `Support::project`, `Balanced::project`, De Morgan complements, `dnf_upgrade`) |
| `set_gauge_membership` | (chop E, not yet built) | `admits`/`contains` also test `hcp` + `support_points` gauges |

Neither knob has a [bidding-options.md](bidding-options.md) row yet — chop F
adds both.

## Chops

Byte-identical chops keep the default system byte-identical (dual-knob
ratchet + `verify::compare`/exhaustive-shape tests + `bba-gen` dump diff).
MEASURED chops follow [measurement.md](measurement.md) and record their
verdict here.

The A→E0 wave landed as **one commit** (this session, 2026-07-23): the chops
were developed against shared files and verified byte-identical as a whole —
dump diff 4000 boards × both vuls + a second 4000-board seed, zero divergent
boards.

| # | What | Status |
| --- | --- | --- |
| pre | `Dnf` machinery + `set_dnf_reading` | cff8919 |
| pre | `Strength` gauges (hcp + support_points axes) | dd90e2c |
| pre | Stage A `Flip` complements | 1e11c49 |
| pre | `authored_calls_read_what_they_gate` leak ratchet | 095ac85 |
| A | this ledger | landed |
| B | ratchet upgrade: shared trie walk, `dutch()`, per-gauge noun columns, dual-knob exact pins, per-box leak predicate | landed |
| C | `impl Constraint for Envelope + Dnf`: strict `Envelope::accepts` eval (all gauges, ceilings), identity projection, noun-compatible `describe` | landed |
| C2 | pilot: 2/1 fit-split boxes via `dnf_upgrade` (legacy eval/describe/knob-off reading kept; exact two-box reading knob-on) | landed |
| D1 | bounded-band complements → 2-box DNF via `disjoin`; De Morgan on `And`/`Or` + `Flip` double-negation `project_complement` (knob-gated); `SupportPoints::project_complement` (new, knob-gated) | landed |
| D1b | exact shape DNFs via `shapes()`: `balanced` (cube {2..=4}⁴ + four 5(332)), `semi_balanced` ({2..=5}⁴ ∪ four {s 6..=7, rest 2..=3}), `notrump_shape` per variant ({M 2..=4, m 2..=cap} ∪ two 5M(332)); each pinned exhaustively over the 560-shape lattice | landed |
| D1c | knob-on box hygiene (`Dnf::tidy`): sum-feasibility prune + containment dedup | landed |
| D2 | `Support::project` forward-boxes partner's suit, knob-gated | landed |
| E0 | book-wide eval ⟹ strict-membership sweep over american()+dutch(), knob-on, forward + band (inference.rs, beside the ratchet) | landed |
| E | **MEASURED** `set_gauge_membership`: gauges get membership teeth; `ab-dnf-sd-lead --arm off\|dnf\|gauge\|both` | parked |
| F | **MEASURED** flip `set_dnf_reading` default-on: full bba-gen/bba-score match both vuls plain+PD, sd-lead arms, regen/retrain evaluation, `--no-ns-dnf`, bidding-options rows, ratchet re-pin | parked |
| G0 | **MEASURED** 2NT shape redesign {M 2..=4, m 2..=6} (drops 5M(332), adds wide minors) — a treatment, not a rewrite | parked |
| G | tail: comparative-shape helpers, `Balanced::project_complement`, the ⊤-`described` arms behind the remaining knob-on leaks, term-merge if `debug_assert!(< 64)` fires | parked |

## Ratchet counts

`authored_calls_read_what_they_gate` pins leak counts (a leak = an authored
rule whose describe names an axis while **no box** of its band constrains
it). Knob-off pins are the byte-identity guard (exact — must not move in
either direction); knob-on pins are the migration meter later chops drive
toward 0. Cells are knob-off/knob-on; knob-off never moved during the wave.

| After chop | HCP | length | points | support | support points |
| --- | --- | --- | --- | --- | --- |
| 095ac85 (old columns) | — | ≤49 | ≤61 (HCP+points mixed) | — | — |
| B | 17/0 | 71/29 | 3/0 | 84/84 | 18/0 |
| D1 | 17/0 | 71/24 | 3/0 | 84/84 | 18/0 |
| D1c | 17/7 | 71/47 | 3/3 | 84/84 | 18/12 |
| D2 (wave tip) | 17/7 | 71/47 | 3/3 | 84/0 | 18/12 |

D1c's knob-on **rise** is the meter getting honest, not a regression: an `Or`
with an opaque arm used to read `[tight-box, ⊤]`, which is membership-wise
just ⊤ — containment dedup collapses it to `[⊤]`, so phantom axis knowledge
no longer counts. The knob-on residue now points at real work: the opaque
`described`/`pred` arms (G) still co-gating rules that name an axis.

To list the rules behind any cell, zero that pin and run the test — the
failure dumps the per-column rule labels. The knob-on lists are chop G's
worklist.

## Irreducible tail (stays ⊤, by design)

Honor-location atoms (stoppers, `top_honors`, `suit_hcp`, alt scales,
keycards) have no `Envelope` axis. Context atoms (`min_level_is`, `they_bid`,
seats, vulnerability) are hand-vacuous — ⊤ is *exact*, not a leak.
