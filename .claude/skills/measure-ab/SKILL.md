---
name: measure-ab
description: >
  Run and interpret a bidding A/B experiment in pons — harness choice, seed
  hygiene, polite scheduling, dual scoring, and the ship/reject decision. Use
  whenever the user asks to measure, A/B, benchmark, re-measure, or validate a
  bidding change, or to interpret A/B results.
---

# Running a bidding A/B

[docs/measurement.md](../../../docs/measurement.md) is the source of truth —
read it now if this session hasn't. This skill is the run procedure.

## Procedure

1. **Baseline sanity.** The control arm must be the real routing of the same
   hands (not an analytic pass/invite stand-in), and the treatment must be
   complete (both sides' continuations) — else the number is meaningless.
2. **Pick the harness.** Prefer an existing `examples/ab-*` (self-play
   seat-swap; check `--help` — most questions are a flag away). Use
   `examples/bba-gen` via `scripts/bba-gen-parallel.sh` + `ab-dump-diff` when
   you need the reference opponent — mandatory for tuning our own conventions'
   strength/ranges (self-play can't punish them; pass the matching
   `--advertise-*` so BBA reads the bids).
3. **Seeds.** `export SEED_BASE=$(date +%s)` once; all arms of this experiment
   share it (paired diffs need identical deals). Next experiment → new base.
   Record the base + git SHA in the results note.
4. **Run.** Build first, then hands off the binaries until the run finishes —
   no rebuilds mid-run. Wrap in `scripts/idle-run.sh` (shared box), arms
   strictly sequential; one run already saturates every core. `tmux` or
   `setsid nohup` for long runs. Typical scale: 200k filtered boards/cell
   contested, ~205k/arm (6400 × nproc) for bba-gen; 40k filtered ≈ 1 min/cell.
5. **Score both ways** — plain DD and PD — from the same solved tables, at
   both vulnerabilities (none/both). Report IMPs/board, IMPs/fired (or
   IMPs/divergent on filtered harnesses — never IMPs/board there), fired rate,
   and a CI (two seeds minimum, or bootstrap).
6. **Verdict** from the decision table in docs/measurement.md. Remember the
   two big artifacts: PD-only win = doubling artifact; DD loss on an
   obstructive/concealment idea = harness blindness, park it opt-in for a
   single-dummy re-measure (`single_dummy_leads` prices the blind-lead seam).
7. **Losing? Trace the worst divergent boards before declaring dead.** Look
   for: an unauthored continuation (passed-out cue, unanswered doubled
   transfer), an over-broad trigger (fires outside its hand class — dilution
   of the per-fired win is the tell), or a scope leak via a shared node.
   Fix and re-measure; an unchanged arm's dump can be reused when the change
   can't affect it (same seeds align the pairing).
8. **Write it down**: numbers, board counts, seeds, SHA, verdict, and the
   knob's shipped default — in the CHANGELOG (and ledger doc if tracked).
