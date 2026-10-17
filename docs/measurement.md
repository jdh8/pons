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

Open corrections from the 2026-10-14 review are tracked in
[research-method-review.md](research-method-review.md): disclosure, population
coverage, asymmetric vulnerability, sampling, SD calibration, dataset and
experiment identity, and the decisions still owed. Issues #6–#9 (deal
independence, non-inferiority, the IMP comparison target, cell counts) are
closed; their ledgers are in
[archive/research-method-review-closed.md](archive/research-method-review-closed.md). Read the affected entry before relying on a historical
verdict or designing its confirmation.

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
5. **Seed hygiene.** `export SEED_BASE=$(date +%s%N)` **once per experiment**;
   every arm of the experiment reuses it (paired diffs need identical deals).
   The *next* experiment takes a fresh base. Never replay fixed seeds 0..31.
   **Deal generator v2 (2026-10-14, [#6](https://github.com/jdh8/pons/issues/6)):**
   board `i` is drawn from `common::board_rng(base, i)`, ChaCha keyed on the
   pair, so two bases share no boards. v1 seeded board `i` with
   `seed_from_u64(base + i)`: a base `k` above another replayed its boards
   `k..` as boards `0..`, so two v1 runs `k < count` apart shared `count − k`
   deals and a "fresh seed" an hour later was mostly the same deals (dealer
   and vulnerability come from the index and line up only when `k ≡ 0 mod 4`).
   Before reading two runs as independent, or pooling them, run
   `deal-overlap` on both (a `bba-gen` dump or shard dir, or
   `seeded:BASE:COUNT`; it counts shared four-hand deals and exits 1 on any)
   and quote its line in the ledger; shared boards are one sample, not two.
   Empty sources are rejected: a missing sample cannot confirm independence.
   For v1 runs that the tool cannot regenerate, check the seed windows
   `[base, base + count)` do not intersect. v1 results reproduce only
   at a commit before the change, and a `smoke-default` byte-identity diff
   must build both sides on the same side of it. The v1 defect did not affect
   `bba-gen` / the anchor, `ben-gen`, or any harness that seeds one `StdRng`
   from `--seed` and draws all its deals from it. However, `bba-gen-parallel.sh`,
   `ben-gen-parallel.sh` and the self-sharding runners (`ab-reading-drift`,
   `ab-diamond-floor`, `ab-walk-shape`, `ab-face-corroborate`) seed shard `i`
   with `SEED_BASE + i`, so two bases closer than the shard count share whole
   shards — fresh bases now use `date +%s%N` (GNU date, nanosecond timestamps).
   The earlier `date +%s` × 1000 spacing still collided within one second;
   check the actual deal intersection rather than assuming clock uniqueness.
   Recorded seeds, the anchor's included, keep their shards. Fixed
   benchmarks keep their v1 keys on purpose (`bidding-corpus`,
   `bidding-performance`), as do `probe-weak-two-eval` (a one-off census)
   and `probe-keycard-reach` (its `seed + i` picks dealer and vulnerability
   over a fixed `.pdd` slice).
   **Vulnerability cells on one seed are one deal set.** `bba-gen` and
   `ben-gen` deal the same boards at every vulnerability (declared-agreement's
   none and both arms: 204 800 of 204 800 shared), which sharpens the
   none-vs-both comparison but makes "4/4 cells positive" one deal set read
   four ways (two vulnerabilities × two scorers, all on the same boards), not
   four samples. Count seeds, not cells, when citing replication. A BEN
   guard run on the BBA run's seed is a check against another opponent, not
   corroboration.
   **Regenerate the control at HEAD, on that same `SEED_BASE`.** A shared seed
   buys *deal*-pairing, not *code*-pairing: diffing a fresh treatment arm
   against an older snapshot's control (e.g. `ab-results/anchor`'s
   `2026-07-28-3c94802` `american-instinct` arm, 87 `src/bidding` commits
   stale) read +0.1905/+0.2832 NV where the same-sha control read
   +0.1745/+0.2540 — a ≈9–11% overstatement, sign not guaranteed. Control
   generation is ~2 min per vul against hours of DD; snapshot arms are for
   *longitudinal* series (the anchor), never same-experiment controls.
   Sibling: when re-measuring to *confirm an older verdict*, diff the **fired
   rate** against the prior run first — `two_level_minor_overcall_tight`'s
   2026-07-26 SD-PD reversal (`daf6c0e`) re-run at `abdafcc` went 1.43%/1.49%
   → 1.76%/1.75% fired (the trigger had widened) and plain DD flipped
   +0.0015 → −0.0102 ±0.0021 NV; a fired-rate drift means the two runs never
   covered the same hands, so the "reversal" was a different experiment.
6. **Pre-count the divergence before spending a solver on a rare feature.**
   Bidding is cheap (~2 min/arm/vul); double-dummy is not. Bid both arms on the
   shared `SEED_BASE`, diff the dumps' `table_a` / `table_b` auctions in plain
   Python — no solver — and count boards whose final contract moved. Below
   roughly 30–50 divergent boards per vulnerability there is no verdict to read
   and the DD arms are wasted; the house remedy for a genuinely rare lane is
   **enrichment** (`--filter-1nt`, `--filter-landy`, `--filter-preempt`, with
   `--wide-1nt` so the scan covers the default 1NT opener), not a bigger
   random draw. This costs minutes and can save a night: the PDI pass
   reading pre-counted at **10 divergent boards in 409,600** and never went to
   DD ([pdi.md](pdi.md), "Why the reading is inert"). Pair it with a
   **reach-maximal negative control** — the same mechanism at deliberately
   over-strong thresholds — to separate "these thresholds are too tight" from
   "this mechanism cannot move the bidder".
7. **Run politely and sequentially.** Wrap heavy runs in
   `scripts/idle-run.sh`; run arms one after another, never in parallel (each
   run already saturates the box); **never `cargo build` while an A/B is
   running** (later shards exec the new binary and die on renamed flags).
8. **Score with both scorers** — plain DD (`ns_score_contract`) *and* perfect
   defense (`ns_score_bid`) — and report: IMPs/board, IMPs/fired (or /divergent),
   the fired rate, and vulnerability split (none/both). Two seeds or a
   bootstrap CI before trusting a small edge. A **competitive, sacrifice or
   doubling** treatment, or a knob that reads `vulnerable()`, is confirmed at
   all four cells: `VULS="none ns ew both"` (ab-lib; we sit N/S, so `ns` is
   unfavourable, `ew` favourable). Each cell rebids its own auctions; a
   verdict names the cells it covers.
9. **Read the verdict from the [decision table](#the-decision-table).**
10. **Before declaring a loss dead, trace the worst divergent boards.** The
   cause is often an unauthored continuation, an over-broad trigger firing
   outside its intended hand class, or a shared node leaking into a context
   where the treatment doesn't apply — all fixable, none "the idea is bad."
11. **Ship per the [ship rules](#ship-rules)**; record the result in
    `CHANGELOG.md` (and the relevant `docs/ai-bidder/*ledger*` if applicable).
12. **A refactor claimed inert must be *proven* inert** — a seeded
    byte-identity diff of the default system across the two commits
    (`smoke-default`), never an argument from knob defaults: the RKCB knob
    cull's `announced()` overlay was inert only because an unrelated global
    split happened to be off ([bba-kickback.md](ai-bidder/bba-kickback.md)
    ledger). The same bar covers a **performance** change: if it moves any
    call it needs the A/B non-inferior on both scorers (worst cell's lower
    bound above −δ, [decision table](#the-decision-table)), not a speed number
    (CLAUDE.md § Objective — KR2/KR3 wins prove KR1 non-inferiority).

`ab-dump-diff --show N` is a **selected worst tail**, not a sample. Use it to
find boards to trace, never to claim a population mechanism until the claim is
counted over the full arm dumps (`boards[].table_a` / `table_b`). This caught
two false Texas stories on 2026-07-20: the full 409,600-auction census showed
SAT's `4♣` was actually the most-doubled call (9.25% vs 8.75%), with doubling
tracking level rather than variant. If a full count is unavailable, label the
tail observation unverified.

## Runtime provenance and resuming arms

`scripts/ab-lib.sh` fingerprints the launcher and the runtime files returned by
`bba-gen --provenance`, plus the arm's flags and file-valued flags. The report
must include `exe PATH` and `lib PATH` rows and a `weights PATH` row for each
external floor-weight blob, in loading order. Every named file must be readable.
A complete or partial arm is refused if its recorded fingerprint differs or is
missing. Source edits and doc-only commits do not substitute for a binary change.

A legacy generator that lacks `--provenance` needs an explicit reporting wrapper
selected through `BBA_GEN`. For example (replace both paths with the actual
runtime files):

```sh
#!/bin/sh
set -eu
exe=/path/to/legacy/bba-gen
lib=${BBA_LIB:-/path/to/libEPBot.so}
if [ "${1:-}" = --provenance ]; then
    printf 'exe %s\nlib %s\n' "$exe" "$lib"
    if [ -n "${PONS_FLOOR_WEIGHTS:-}" ]; then
        printf '%s\n' "$PONS_FLOOR_WEIGHTS" | tr ',' '\n' |
            while IFS= read -r path; do printf 'weights %s\n' "$path"; done
    fi
    exit 0
fi
export BBA_LIB="$lib"
exec "$exe" "$@"
```

The historical `ab-rkcb-asker.sh`, `ab-reading-fallthrough.sh` and
`ab-cue-raise-sign-off.sh` runners select their off build internally. Pass
`OFF_BIN=/path/to/reporting-wrapper` to override that selection and skip its
worktree build; `BBA_GEN` still selects their on arm.

The wrapper must name what it actually runs; a loader wrapper must name the
wrapped executable too. There is no fallback that guesses the default build or
hashes only weight paths. Unsupported, empty or incomplete reports and unreadable
files refuse even a fresh arm. `FORCE_REUSE=1` remains a deliberate override after
manual checking; if provenance fails, the log warns that identity is unverified.
The launcher checksum also makes metadata from before this guard incompatible;
use a fresh results directory or check the old arms before overriding.

`arm.meta` is the arm's **manifest**: the fingerprint above plus the deal set
(`shards:`, `per-shard:`, `seed-base:`), all compared on resume, and `# `
provenance lines that are recorded but never compared (source SHA, a cksum of
the dirty diff, `JOBS`, `BOARDS`, the date). The SHA stays out of the identity
on purpose: the binary's bytes are the code identity, and a doc-only commit
during a long run must not wedge its resume. A manifest mismatch refuses with a
pointer to a fresh results directory. An `arm.meta` written before the manifest
(fingerprint lines only) still resumes on a matching fingerprint, with a
warning, because the shard check below pins the deal set on its own.

Every resume also **verifies the shards** (`bba-gen-parallel.sh --verify`):
exactly `shard-0..SHARDS-1`, each parsing as `PER_SHARD` boards dealt from
`SEED_BASE + i`. An arm that fails is regenerated whole; `FORCE_REUSE=1`
overrides the manifest, never this check. `bba-gen-parallel.sh` exits non-zero
on any failed worker or unverified shard, which stops the runner.

A gate, diff or SD report is reused only while its `.inputs` stamp matches: both
arms' `arm.meta`, the scorer binary's cksum and the scorer flags. A regenerated
arm, a rebuilt scorer or a changed flag rescores the cell; a report with no
stamp (an older results directory) is rescored once. Scoring tools never read
`arm.meta`, so old arms without one stay scoreable directly.

The BEN runners get the same guard by sourcing `ab-lib.sh` with
`GENERATOR=ben-gen` and `JOBS` set to their server count: `ben-gen` writes
shards atomically and answers `--provenance`/`--verify`, and
`ben-gen-parallel.sh` waits on every worker, refuses a fleet whose size is not
`SHARDS`, and verifies `shard-0..N-1` the way `bba-gen-parallel.sh` does. The
BEN servers themselves are outside the fingerprint; the `--note` flag records
the tier conf's sha256. A runner never shadows `arm`/`diffpair`/`sddiff`: the
binary-arm runners export `BBA_GEN` per arm, the split-by-opening runners stamp
their strain-split reports with `report_inputs`. Finally, `load_dump` refuses a
shard dir whose shards were generated with different arguments, ignoring only
the per-shard values (`--seed`, `--output`, `--count`, `--port`,
`--start-board`, `--note`); a scan of every shard dir here and on dl02
(2,723 dirs, 2026-10-16) found none that differ otherwise.

## Watching a run

A long run is watched by **polling the runner process**, never by grepping the
log for the script's own success line:

```sh
setsid nohup scripts/idle-run.sh scripts/ab-my.sh ab-results/my >ab-results/my.log 2>&1 &
PID=$(pgrep -f '^bash scripts/idle-run.sh scripts/ab-my.sh')
tail --pid="$PID" -f /dev/null      # returns when the wrapper exits
tail -5 ab-results/my.log           # then read the diff files in a *separate* command
```

The `^bash scripts/idle-run.sh` anchor is the point, not decoration — see the
self-match note below.  `$!` is not a substitute: under job control `setsid`
forks, and `$!` names a process that is already gone.

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

**Watcher self-match (2026-07-30, again 2026-10-03):** `while pgrep -f
"pass-reading-ab.sh"` matched its *own* command line and looped forever. An
unanchored `pgrep -f <script>` matches every shell whose command line merely
*mentions* the script: the watcher itself, and any stale watcher another
session left behind — so one stuck watcher keeps every later one alive, and
`| head -1` picks the oldest, i.e. the stale one. On 2026-10-03 three
sessions' watchers hung this way on runs that had finished cleanly; the old
recipe above (`pgrep -f ab-my.sh | head -1`) was itself the bug. Anchor the
pattern at the wrapper's own command line (`^bash scripts/idle-run.sh …`),
which no `bash -c` watcher can start with. If a watch seems late, read the
log: `idle-run: <script> exited <n>` is conclusive.

## Scorers (`src/scoring.rs`)

| Scorer | What it scores | Use for |
| --- | --- | --- |
| `ns_score_contract` | Plain DD: the reached contract with its *actual* table penalty. | Duplicate A/B results — the default verdict. |
| `ns_score_bid` | Perfect defense, the **double-dummy-bidding rule**: a contract that fails DD is scored **doubled** (synthetic X), a making one **undoubled**, whatever `X`/`XX` the table carried. | The pessimistic bracket end: "scored as double-dummy opponents would have doubled." Also the call scorer (EV rollouts, contract-choice probes). Until 2026-09-27 the A/B harnesses used `ns_score_pd`, which kept a real double on a making contract; PD columns dated before then differ on exactly those boards (≈0.07 IMPs/board on the BEN anchor). |
| `ns_score_tricks` | **Plain SD**: an explicit single-dummy trick count priced at the contract's *actual* penalty. | The pricing tail of the SD scorers. **Never a verdict on its own** — see below. |
| `ns_score_pd_tricks` | **SD-PD**: the same trick count under the double-dummy-bidding rule — a contract that *fails on those tricks* is scored **doubled**, a making one undoubled (table double erased, as in `ns_score_bid`). | The SD arbiter. Report it beside plain SD everywhere the SD bracket is quoted. |
| `single_dummy_leads` (`src/single_dummy.rs`) | MC-DD with a *blind* opening lead chosen from the leader's sampled worlds. | The one known DD bias at 1NT level (DD defenders always find the killing lead, ~+0.3 tricks to 1NT declarers). Re-score close NT-defense verdicts with it — under **both** SD scorers. ⚠ **At slam level this is an UPPER bound** — it removes the lead pessimism and keeps all of DD's play optimism (Pavlicek after-lead: +7pp on slams). Never read it as slam insurance. |
| `single_dummy_playout` (`src/single_dummy.rs`) | The **sd-declarer playout**: blind lead, then declarer chooses every card MC-DD over worlds inside the auction's readings (show-outs remembered) while the defense plays DD on the actual deal. | The slam-side DD bias (see below): a DD declarer never misguesses, so every DD-play scorer is *optimistic* for the arm bidding more slams. Runners: `ab-dump-sd --sd-declarer`, `ab-slam-entry --sd`. Sequential per board — divergent sets only. Its haircut is ≈1.5× the real one (see Known biases) — the LOWER bound (per arm). Its declarer maximizes **mean tricks**, contract-blind: it will risk a making line for an overtrick; `single_dummy_playout_with` + `DeclarerObjective::Make` is the opt-in safety-play declarer (unmeasured on any harness). |
| `sd_blend_imps` (`examples/common/mod.rs`) | The **sd-blend**: one composed run (`single_dummy_declarer_tricks`) returns both endpoints' trick counts, and the blend takes the playout outcome with probability λ(level), the lead-endpoint outcome otherwise, mixed at the IMP level (four `imps` terms — the IMP table is nonlinear). λ per level in `common::SD_BLEND_LAMBDA`, fitted by `probe-sd-calibration`'s λ block so the blend applies exactly Pavlicek's **after-lead** declarer-fallibility shift to the lead endpoint. | The calibrated **point estimate between the two sd endpoints** — built for slam stop-vs-bid buckets, where the decision band is 45–55% make at the 6-level. Grands: quote blend *and* analytic shave as a bracket, never a point (thin data, bigger bias). Validation: `probe-slam-battery` (DD-fair archetypes must not move; third-eye ones must grade). |

**The principle** (jdh8, 2026-06-24): *the threat of a double is a legitimate
deterrent, but a double that never appeared on the table must never enter the
final score.* Hence the bracket: each arm's score sits between plain DD
(under-punishes overbids) and PD (over-punishes — a perfect doubler never
doubles a making contract). Reality is closer to plain DD. The bracket is per
arm, a pair of scenario checks — **not a bound on the paired A/B delta**:
subtracting like endpoints does not bound the difference. On one NV board with
ON in 4♥ down one and OFF in 3NT down one, the plain delta and the PD delta are
both zero, yet an imperfect doubler who doubles only ON makes the delta −50
(−2 IMPs), outside both.

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
so the two SD numbers bracket each arm the same way the two DD numbers do (and,
like them, do not bound the paired delta).
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

**Fixed limitation — `ab-dump-sd` did not price the disclosure of our
unalerted calls** (measured 2026-07-26, diagnosed and fixed 2026-10-16). The
dump-rescore harness reads each arm's auctions for the blind leader, and its
`--on-ns-*` flags make the leader read the ON arm under the ON arm's system.
On 2026-07-26 rescoring `nt-defense-band-9-14` with `--on-ns-overcall 20:37` —
claiming the overcaller holds 20–37 HCP rather than 9–14 — reproduced the
flagged run to the last IMP, and so did dropping
`--on-ns-negative-double-shape modern` from `modern-negx`.

The cause, confirmed by `examples/ab-dump-sd/tests.rs`: the harness read every
seat through the plain partnership, which decodes the *opponents'* calls only
when alerted (`table_alerts`, at `ReadingScope::Alerted`; the full scope needs a
declared opponent). Our pair sits North–South in a `bba-gen` dump, so whenever
we declare, the East–West leader saw our alerted calls and nothing else of our
book: the unalerted natural `(1NT) 2♥` overcall fell to the natural walk
(`points 8..37`, `♥5+`) under either band. A North–South leader reading its
partner was never affected after 2026-08-16 (`ReadingScope::All`), and the
alerted negative double reaches both leaders on today's reader; its July
inertness is not reproduced and probably came from the pre-2026-08-09 decode
contexts, which read thread-live knobs instead of the arm's pinned ones.

The fix is in the harness only (`common::TableReader`): an East–West reader
declares our own partnership as its opponent (`Partnership::with_opponents`),
so every North–South call decodes off its authoring rule, alerted or not;
North–South readers are unchanged, and the default system's bidding is
byte-identical. The report prints a `leader reading:` line naming the mode, and
`--legacy-leader-reading` reproduces the old numbers. **An `sd.*.txt` without a
`leader reading:` line is legacy.**

The self-play sd harnesses read the same way and were fixed the same day:
`ab-nt-defense-matrix`, `ab-point-count`, `ab-fuzzy-strength`,
`ab-dnf-sd-lead`, `ab-meckstroth-2nt`, `ab-notrump-minors`,
`ab-forcing-nt-two-suiter`, `ab-size-ask-accept`, `ab-size-ask-eight`,
`ab-nt-splinter`, `ab-one-nt-runout`, the sd-declarer rows of `ab-kickback`
(live and `--rescore`), `ab-minor-keycard` and `ab-slam-entry`, and
`probe-sd-calibration`. Both pairs are ours there, so each pair reads with the
*other* pair's partnership declared as its opponent
(`TableReader::self_play`): the arm-pure tables declare the arm's own book, the
duplicate matches the opposing arm's. Six of them used to read every seat
through one fixed book (`ab-point-count` and `ab-fuzzy-strength` the default
book, `ab-one-nt-runout` and `ab-slam-entry` the baseline, `ab-kickback` and
`ab-minor-keycard` the feature); they now read each table with its own
pairs' books. Each prints `leader reading:` and takes
`--legacy-leader-reading`, which restores the old reader exactly (the
regression test is `examples/ab-point-count/tests.rs`: the candidate's
unalerted weak-two HCP band reaches the leader, and the legacy leader reads
`5..10` points at any HCP under every band).

Consequences: legacy verdicts from dump rescores **stand as treatment effects**
(both arms were read identically, and the arms still differ in the auctions and
contracts the dumps recorded); a legacy sd row does **not** price the disclosure
cost or concealment value of our unalerted calls when we declare, and verdicts
whose mechanism rested on that need a re-measure under the disclosed reader
before they are cited as such. Any future harness that needs disclosure priced
must verify it moves the numbers before trusting it — the one-line check is to
re-run with a deliberately absurd band and confirm the output changes.

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
| **higher than plain SD** | Plain SD *understated* it: the treatment is the sounder bidder, so restoring the doubling punishes the **baseline** more | `decision.reading.notrump_minors` (Puppet) — +0.0003/+0.0005 (straddles 0) → **+0.0006/+0.0010, CI-clear both** |

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

## Direct A/B vs reference-relative gain

Two paired quantities come off the same pair of dumps, and they answer
different questions (2026-10-16, [#8](https://github.com/jdh8/pons/issues/8)):

| name | per board | answers | where |
| --- | --- | --- | --- |
| **direct A/B** | `IMP(S_on − S_off)` | does the change beat the old system head to head? — **deployment**, every ship verdict | every `ab-*` harness; `ab-dump-diff` (default `--metric direct`, line `Delta (direct A/B, …)`) |
| **reference-relative IMP gain** | `IMP(S_on − R_on) − IMP(S_off − R_off)` | how much of the gap to the reference does it close? — **anchor gap**, tracking only | `ab-dump-diff --metric reference` (line `Gain (…)`); the move between two same-seed anchor headlines |

**A shared reference does not cancel.** `R` cancels in the raw points
`S_on − S_off`, but the IMP table is nonlinear: `S_on = 420`, `S_off = 400`,
`R = 0` is +1 IMP direct and 0 IMPs gained against the reference
(`common::tests::a_shared_reference_cancels_in_points_but_not_in_imps`). The
metrics agree when the baseline *is* the reference (`R = S_off`) and on
unchanged boards; otherwise neither estimates the other, and same-board pairing
does not make them equivalent.

**They also cover different tables.** In a `bba-gen` dump `table_a` is our pair
NS and `table_b` the reference pair NS against our pair EW. The direct A/B
reads `table_a` only: one table. The anchor's per-board value
(`bba-decompose`) is `IMP(table_a − table_b)`, so the gain's reference `R` is
`table_b`'s contract, and it **moves** whenever the change acts on our EW
bidding; `--metric reference` reports on how many boards it moved. The gain is
the two tables' move together, so a change that acts at both tables gains
about **twice** its direct A/B. On the `7838d8c2` anchor (re-run 2026-10-16),
`--metric reference` on the shipping vs instinct arms reproduces the two
reports' headline difference to the IMP — +116,750 / +143,682 plain IMPs,
+0.5701 / +0.7016 plain and +0.7251 / +1.0179 PD per board (none / both), with
the reference moved on 26.6% / 25.0% of boards — against the floor's direct
worth of +0.2959 / +0.3592 plain: ratios 1.93 / 1.95.

Rules:

- **Ship on the direct A/B.** A reference-relative report prints no `Verdict`
  line; it tracks.
- **Never mix the two in one sum.** `scripts/ab-aggregate.sh` and
  `scripts/ab-score.awk` refuse a file with a `Gain` line.
- **Compare like with like.** Before reading one figure against another (a
  window's paired diff, its headline move, its summed ship A/Bs), name the
  metric and the table count of each. Even under one metric, direct increments
  taken against different baselines need not add up, and combined changes
  interact, so a surplus is not by itself evidence of synergy.
- **One reference definition per comparison.** Plain and PD price all four
  contracts of a board on one DD table, one scorer each, off one solve. If the
  arms faced different reference *bidders* (`their_label`), or were bid at
  different vulnerabilities or dealers, `ab-dump-diff` refuses.

**Historical columns.** Before 2026-10-16 `ab-dump-diff` printed only the
direct A/B, as `Delta (run − sit)`. In [bba-gap-campaign.md](bba-gap-campaign.md)
the *instinct* / *shipping* headline columns and any move read off them are
per-board anchor scores (both tables), so a same-seed move between two of them
is a reference-relative gain. Every "paired to …", "floor's paired worth" and
"summed ship A/Bs" figure is a direct A/B at table A. Raw outputs stay as they
are; recompute a reference-relative gain from the snapshot dumps with
`--metric reference` where one is needed.

## The decision table

**Each scorer's verdict comes first** (2026-10-14,
[#7](https://github.com/jdh8/pons/issues/7)). A "wash" — a CI straddling
0 — says the data have not ruled out zero; it may still hold a loss worth
caring about, so it is no proof of non-inferiority. Every whole-experiment
A/B reporter prints a `Verdict` line per scorer it computes
(`common::verdict`, via `verdict_line` / `report_swings` /
`report_brackets` / `report_sd_brackets`). Forensic subgroup tables
(`ab-dump-bucket`, `ab-dump-cachalot`, `ab-dump-gladiator-bucket`, per-call
and per-bucket breakdowns), the `ab-nt-defense-matrix` game matrix, the
oracle-regret `ab-defend-2sx-or-3nt` and the sum-only
`scripts/ab-aggregate.sh` are not a treatment's verdict and print none.
`ab-dnf-sd-lead` prints none either: it arms the leader's reading on both
sides but signs the swing from NS's chair, so an EW leader's gain counts
against it and the mean cancels in expectation. Nor do the two modeled
diagnostics, which say so in their output: `ab-sat-slam-try` (a hand-coded
`4M` slam-try gadget against the bidder) and `ab-texas-vs-sat` (one `4M`
scored from either declarer). Neither bids a treatment through the system,
and each averages over the configurations of a conditional hand class, not
over random deals, so its mean is no population evidence; a change it
motivates ships on its own A/B.

Each `Verdict` line reads its scorer against:

- **Primary metric**: plain-DD IMPs per board, ON − OFF on paired deals,
  over the population of **random deals**, per vulnerability cell.
- **Margin δ = 0.002 IMPs/board** (`common::NI_MARGIN`): the largest loss
  we accept as no loss. About an eighth of the ships summed over three days
  (anchor `494f0c4b` → `46d0dc14`, paired at +0.016 plain) and 0.05 IMPs per
  24-board match. It was fixed before any result was read against it; **never move
  it after seeing a candidate's numbers**.
- **Test**: the reporters' two-sided 95% CI (`1.96 · SE`); its lower bound
  is the one-sided 97.5% bound.
  - `win`: lower bound > 0.
  - `non-inferior`: lower bound in (−δ, 0], upper bound ≥ 0.
  - `inconclusive`: lower bound ≤ −δ, upper bound ≥ 0; a loss larger than
    δ has not been ruled out.
  - `loss`: upper bound < 0, even inside δ.
  - Under two boards there is no sample variance: `mean_with_ci` returns an
    infinite half-width, and the verdict is `inconclusive`, as is a `NaN`.
- **Enriched runs** measure a conditional population, so the margin per
  measured board is `δ / density`, density = accepted ÷ drawn. The
  conversion holds for the filtered population only; a slice the filter
  misses is outside the verdict (coverage: see the IMPs/divergent
  paragraph below the decision table). A filtered
  dump does not record its density: `ab-dump-diff` / `ab-dump-sd` withhold
  the verdict on a `--filter-*` dump until `--density` is passed. A density
  outside `(0, 1]` is refused (`common::parse_density`; `verdict` panics on
  one): at 0 the margin is infinite and would prove anything non-inferior.
- **Vulnerability cells**: the experiment's verdict on a scorer is its
  **worst cell** (loss < inconclusive < non-inferior < win). Requiring
  every cell to clear the bound is an intersection–union test, so no
  multiplicity correction is owed. A pooled figure is a headline, never a
  rescue for a cell that fails. The cells of one seed deal the same boards
  (checklist 5), so a pooled CI that counts board-cells as independent is
  too narrow; pool per deal (the mean of a deal's cell swings) instead.
- **Confirmation data are fresh**: a run used to choose a threshold or
  pick a candidate cannot confirm it. A re-run on a new seed is independent
  only if `deal-overlap` says so (checklist 5). Fix any extension's board
  count *before* reading the first run; topping up until the bound clears
  inflates the error rate.
- **PD's verdict is PD's.** A PD `non-inferior` is no proof for plain DD;
  each column of the table below reads its own scorer.

| Plain DD | PD | Verdict |
| --- | --- | --- |
| win | win | Real. Ship default-on. |
| non-inferior | win | **Shippable default-on**: plain DD has ruled out a loss beyond δ, and PD gains when opponents punish. |
| inconclusive | win | **Insufficient data**: a plain-DD loss beyond δ is still possible. Run the pre-declared extension on fresh deals; shipping without one is a [recorded trade-off](#recorded-trade-offs), not a proof. |
| win | non-inferior/inconclusive/loss (PD *erases* a plain win) | Doubling artifact — the "win" is reaching contracts a competent doubler would slaughter. Suspect; don't ship on this evidence. |
| loss | win | Artifact of PD's synthetic X (it credits phantom doubles of contracts we no longer bid). Not a win. |
| loss | loss | Loss — but trace worst boards before declaring dead (step 9). |
| non-inferior | non-inferior, treatment is *additive* (repurposes a useless call, sacrifices nothing) | May ship default-on if its value is DD-invisible (obstruction, lead-direction) — precedent: Unusual 2NT over their 1NT. |
| non-inferior | non-inferior, two methods that *push each other* | Break the tie by **naturalness** (ship rules): a move *toward* established natural theory ships default-on; a *convention* trialled against natural stays opt-in. |
| inconclusive | anything but loss | **Insufficient data** for every row above that asks for non-inferior. Extend, or record the trade-off. |

#### Recorded trade-offs

A default can be adopted without a non-inferiority proof — on
naturalness, bridge theory, or engineering cost — but the ledger must say so:
the basis, the plain-DD interval, and the loss the decision accepts (its
lower bound). Never write such a decision up as "proven no loss" or "a
non-loss". The defaults shipped on a wash before 2026-10-14 are inventoried
in [archive/research-method-review-closed.md](archive/research-method-review-closed.md#issue-7-non-inferiority-inventory).

**Domain addendum — name the knob's direction before you read a row**
(2026-08-12, paid for by the competitive accountant). Every row above assumes
a knob that changes **what we bid**. The `win | non-inferior/inconclusive/loss` row in particular
is written for a knob that *bids more*: its rationale is that plain DD lets
**our** overbids off the hook while PD prices them.

For a knob whose mechanism is **doubling them more**, PD is not the
pessimistic end of a bracket — it cannot see the double at all. `ns_score_bid`
re-derives every penalty from the double-dummy outcome, so the arm's own
doubles never reach the score:

| their contract | OFF (we pass) | ON (we double) |
| --- | --- | --- |
| fails | scored doubled **free** | scored doubled — no gain |
| makes | undoubled | undoubled — the wrong double is erased too |

so the knob is invisible on both rows and all PD sees is the run-outs (they
escape to a making contract, scored undoubled, while the contract we *would*
have defended collects a free synthetic X). *(Until 2026-09-27 the PD scorer
kept a real double on a making contract, so the ON arm paid the full cost of
its wrong doubles and could only lose; the conclusion is the same.)* **A knob that adds doubles is
arbitrated on plain DD**, with PD reported as a double-blind column — not
netted against the plain figure, and not rescued or killed by a magnitude
ratio between the two (the scorers are not commensurable). Plain DD's own
known bias, that a better opponent would double more, runs *against* the ON
arm here, so a plain-DD win is a floor. Worked case:
[docs/ai-bidder/competitive-accountant.md](ai-bidder/competitive-accountant.md).

**The addendum's cleanest exemplar** (2026-08-20): the `1NT (3x) X` leave-in's
honor half measured `plain wash | PD win` — the row that shipped
default-on before the 2026-10-14 margin — while concealing a **−2.4 to −3.0 IMPs per fired** single-dummy
loss at both vulnerabilities. Its mechanism is adding doubles, so plain DD
arbitrates, the sd-lead bracket tie-breaks, and PD is a double-blind column
that neither rescues nor kills. Its *length* sibling, priced the same way, was
CI-clear positive on all four columns and shipped. Full rounds:
[one-notrump-competitive-closed.md §N3](archive/one-notrump-competitive-closed.md#n3--measurement-rounds).

**Split a disjunctive gate into separate arms.** When a candidate gate is a
disjunction (`A | B`) and any slice gives its disjuncts different signs, they
are two arms, always — never one bundled knob. Bundling lets a win ship the bad
half or, worse, lets a loss bury the good one: measured on the same lane
(2026-08-20), `len4+ | (len3 & hon2+)` as one gate would have returned a
refutation at both vulnerabilities, and a **+3.4 IMPs/fired** winner would have
been thrown away inside it. Run `A`, then `A|B` against `A` to price the
extension in isolation; a preflight confirming the arms are nested as designed
costs a few thousand boards.

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
robustness check (a pessimist scenario, not a bound on the delta): a win that
survives even the playout is extra-safe; a
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
not IMPs/board — `--filter` changes the per-board denominator. It also
changes the divergent set whenever it rejects a deal on which the arms
would diverge: an enriched run measures `E[Δ | divergence, filter]`, which
equals `E[Δ | divergence]` only if the filter **covers** the trigger, every
hand the changed rule can fire on passing the raw-hand predicate. Check
coverage against the rule's actual population (the 1NT candidate, for
example, against the shipped `NotrumpShape`), not against its usual case,
and state an uncovered slice with the verdict: it is unmeasured, not
non-inferior. Per-fired figures do not recover it.

Measure coverage before reading an enriched verdict globally: bid both arms
**unfiltered** on one seed (bidding only, no solver), then
`filter-coverage` counts the boards whose `table_a` auction (and contract)
moved and splits them by every `--filter-*` scan, strict and `--wide-1nt`,
with the rejected share and examples. The scans are `common::filters`, the
code `bba-gen` draws through, so "accepted" means `bba-gen` would keep it.
On 4,000 boards of `--nt-fifths` against the default, strict `--filter-1nt`
rejected 6 of 48 divergent boards (12.5%: five 5m422 and one 6m322
opener) and the wide candidate none.

**Verdicts drawn on the strict 1NT candidate** (`--filter-1nt`,
`--filter-landy` or `--filter-preempt` without `--wide-1nt`, from Wide6322's
ship on 2026-07-12 to 2026-10-16) are scoped "balanced 1NT openers only".
14.4% of the default's 1NT openers are 5m422 or 6m322 (Monte Carlo, 2M
hands), so a `filter-coverage` preflight misses about that share of nearly
any lane after our 1NT. A coverage threshold therefore cannot sort these verdicts, and the
rule turns on what the treatment touches instead (jdh8, 2026-10-16):

- **The treated call is opener's**, or the arm also authors opener's
  continuation: re-measure with `--wide-1nt` on fresh deals before the
  verdict is relied on (a default flip, a dependent build, a cited
  baseline). The missing shapes change opener's own call, as in Lebensohl,
  where a 17-count 5m422 raised on three trumps.
- **Only responder's, the doubler's or a reading is treated**: the
  "balanced openers only" label stands. The missing openers change the
  routing only.
- **Their 1NT** (our defense): BBA opens 1NT on 98.0% strict-balanced hands
  and 2.0% 5m422, no 6m322 (4,000 `--isolate-defense` boards, seed
  1792160000, none), so the strict scan covers the trigger to within 2%.

The runner-by-runner classification is in
[the filter-scope archive](archive/research-method-review-closed.md#filter-scope-the-rule-and-the-runner-classification).

```text
cargo run --release --features serde --example filter-coverage -- on/ off/ \
  --filter lebensohl-raise --examples 5
```

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
non-inferior/PD-win row, or anywhere ambiguous, re-run SD over a couple of
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
  table make-rate lies between per arm (per level, in aggregate — not per
  board, and not as a bound on a paired delta). Calibration probe: `probe-sd-calibration`
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
  structure, not sample noise. (*#6: the cell fit below, seed 1785794459,
  shares ~88% of this holdout's deals; the owed aligned-cell holdout needs
  a seed outside both windows, or generator v2.*) λ is a property of the **16-world**
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
- **Right-siding: DD prices the lead direction, not the concealment.** The
  solver takes a *(contract, declarer)* pair, so moving declarer moves who is
  on lead — leading *through* the strong hand's tenaces instead of *into* them
  is an honest double-dummy fact, and both plain DD and PD score it. What DD
  cannot see is the other half: that the leader, facing the closed strong hand,
  would have found the killing lead less often. So:
  - **Declarer actually moves → DD sees it**, at roughly the lead-direction
    share of the true edge. §N1-lia package C's Texas transfer won +0.616 /
    +0.711 IMPs per fired board on plain DD with 96% of its divergent boards
    the *same contract from the other seat*, and sd-lead — where the leader is
    blind in both arms — kept the sign at about a third the size (+0.220 /
    +0.352). §N1o's lead 2 is the same result from the other side: CI-clear
    positive exactly where declarer moved.
  - **Declarer does not move → DD sees nothing, and that zero is real.**
    §N1o's notrump endings, where opener declared in both arms, measured
    exactly 0. A convention whose edge is purely concealment measures ≈0 —
    don't trade real constructive value (an auto-drive-to-game, a weak relay)
    to gain it.

  A right-siding idea therefore gets measured like any other. Read its
  divergence first: if the declarer seat flips, a plain-DD zero is a verdict,
  not a blind spot.
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
- **Default-on** requires: plain-DD win, or plain non-inferior + PD win, or
  additive + DD-invisible value on a plain non-inferior (table above) — every
  vulnerability cell's lower bound above −δ. **Plain-DD loss never ships
  default-on.** A plain `inconclusive` ships only as a
  [recorded trade-off](#recorded-trade-offs).
- **Park branch** (`park/<slug>`, CLAUDE.md § Workflow): a non-win whose flip
  needs more *work* — a retrain, unauthored siblings, a redesign — parks the
  whole idea on a branch instead of shipping scaffolding on `main`. Finished
  code whose only owed work is a re-measure stays an opt-in knob (above).
  Rebase onto `main` before re-measuring; control = `main` HEAD on the same
  `SEED_BASE`; merge on a win. Wins already on `main` stand regardless.
- **The wash tiebreak — naturalness.** When two methods push each other (both
  scorers non-inferior), *direction relative to natural bidding* picks the
  default. The
  default is the least-surprising agreement an unknown American / online partner
  already assumes, so a change that moves us **toward established natural theory
  ships default-on on a push** — non-inferior is enough; on an
  `inconclusive` push it may still ship, as a
  [recorded trade-off](#recorded-trade-offs) naming naturalness as its basis. A change **trialling a
  convention** against natural (artificial call, e.g. Cachalot) needs a real
  plain-DD or DD-invisible win; a push only earns an **opt-in knob**. Naturalness
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
to another harness, and read it through `common::TableReader` (every sd
harness prints a `leader reading:` line; one without it is legacy).
`probe-sd-calibration` is the bracket's own calibration
(per-level make-rates vs Pavlicek). The playout is sequential per board (no
cross-board pooling), so reserve it for divergent sets.

Enriched (rejection-sampled) probes score a *conditional* population — see
[Enriched probing](#enriched-probing--when-the-trigger-is-too-rare-for-random-deals)
for when to reach for one and how to read it. Worked example:
`probe-weak-two-major`.

New-harness rules (the Rayon pattern, commits `8f549ed`/`eadb654`):

- Deal generation seeded and reproducible — `common::seeded_deals`, or
  `common::board_rng(base, i)` for a parallel scan; never hand-roll
  `seed_from_u64(base + i)` (checklist 5); **bidding** parallelized with
  `rayon::par_iter` (classify is pure; `Partnership` is `Sync`).
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
