# The deviation panel

*What is reading the opponents' calls worth against an opponent who does not
play the card we read them with?*

## Why

`Inferences::read` applies **our** meanings to **their** calls. That is sound at
our own two seats — partner really did bid our system — and merely *assumed* at
LHO and RHO. `examples/probe-reading-sound` measured the assumption against BBA
2/1 (10,000 deals, 2026-07-29):

| hidden seat | readings | exclude the truth |
| --- | --- | --- |
| LHO (BBA) | 37,314 | **8.24%** |
| partner (ours) | 42,314 | **3.29%** |
| RHO (BBA) | 47,314 | **8.34%** |

BBA's weak twos head the offender list (2♠ 37%, 2♥ 33%) and their Multi 2♦ over
our 1NT is excluded **100%** — we read diamonds, they hold a major. A box that
excludes the truth is a *wrong* prior, not a loose one, and no amount of extra
columns describing that box helps (`features_v4`'s shape moments are a lossless
reparameterisation of the same hard box — zero epistemic slack, and it lost the
A/B).

That is against one fixed bot. The goal is to beat **humans**, who deviate. So
before designing any slack-on-boxes lever, build the instrument: evaluate
against a *population* of perturbed natural bidders — domain randomisation on
the opponent — and see whether our reading layer's value survives.

## The panel

Three axes, eleven members. `scripts/panel.sh` runs them all.

| # | member | axis | flags |
|---|--------|------|-------|
| 1 | SAYC | A | `--system 1 --their-card vendor/bba/Sayc.bbsa` |
| 2 | WJ (Polish Club) | A | `--system 2 --their-card vendor/bba/WJ.bbsa` |
| 3 | Precision | A | `--system 3 --their-card vendor/bba/PC.bbsa` |
| 4 | Acol | A | `--system 4 --their-card vendor/bba/Acol.bbsa` |
| 5 | 2/1 + Multi 2♦ | A | `--their-conv "Weak natural 2D=0" --their-conv "Multi=1"` |
| 6 | 2/1, Cappelletti | A | `--their-conv "Multi-Landy=0" --their-conv "Cappelletti=1"` |
| 7 | dial x=1 | B | `--their-floor american --their-dial 1` |
| 8 | dial x=2 | B | `--their-floor american --their-dial 2` |
| 9 | 4-card overcalls | C | `--their-floor american --their-overcall-four-card` |
| 10 | off-shape 1NT | C | `--their-floor american --their-offshape-1nt` |
| 11 | wild weak twos | C | `--their-floor american --their-wild-weak-two` |

**Axis A** costs no pons code: EPBot already plays four base systems (the
vendored `.bbsa` headers carry `System type = N`) and ~257 convention toggles.
Do *not* bind `epbot_system_name` to enumerate them — the export exists but
segfaults; read the card headers instead.

**Axis B — the antisymmetric strength dial.** Their openings and overcalls are
`x` points lighter, their responses and advances `x` points heavier. The
antisymmetry is the point: pair-level calibration is preserved, so the
partnership still stops in the same places and every authored continuation
stays coherent. A one-sided "everything lighter" dial would just be a worse
system, and we would be measuring its badness, not our misreading.

The dial is captured at **book construction**, not classification: the harness
builds our book at defaults and theirs under the knob on the same thread. A
classify-time read would leak the dial into our own book.

**Axis C — shape indiscipline**, the concrete deviations a club player makes:
overcalling a good four-card suit, opening 1NT off-shape, and undisciplined
weak twos (the BBA-style wild ones our reader already mis-reads).

For B and C the deviant book keeps **disclosing the undialled meanings**. That
mismatch *is* the simulated deviation — a human who says "15–17 balanced" and
opens 1NT on 5-4-2-2 with 14.

## The statistic

Per member, two arms on identical deals:

- `seen` — the shipped default
- `blind` — `--ns-blind-opponent-reading`: LHO/RHO readings blanked at the
  `Inferences` level (partner and our own stay live)

and the primary number is the paired **`seen − blind`**: what reading *that*
opponent is worth. Dual-scored plain + perfect-defense, per the measurement
iron rule.

Read the paired column, never the absolute score. The absolute anchor against a
deviant member is confounded in the wrong direction: a member playing a weaker
system hands us IMPs *while we misread it more*, so a rising score can hide a
collapsing reading value.

**Not comparable to `scripts/blind-inference-ab.sh`** (−0.65 … −1.27 IMPs/bd,
2026-07-26): that control blanked all four seats and only the nets' feature
vectors. This one blanks the two opponent seats at the source, so the sampler
and the floor go blind with them.

Secondary column: exclusion rate per member, from
`probe-reading-sound --system/--their-card/--their-conv/--their-floor`. For the
B/C members that rate measures deviation from our *own* card.

## How to read the result

| pattern | verdict |
| --- | --- |
| reading value **holds** across the panel | the layer generalises; the 8.2% exclusion is survivable, and slack-on-boxes is not the next lever |
| reading value **collapses** on the realistic members (#7, #9) | our reading is BBA-shaped; fund slack on opponent boxes |
| reading value **flips sign** anywhere | worse than useless there — a wrong box is actively costing IMPs, and that member's offender keys name the site |

Promotion to a standing gauge (re-run per shipped bidding change) is decided
from the first report, not assumed.

## Running it

```sh
setsid nohup scripts/idle-run.sh scripts/panel.sh \
    ab-results/panel >ab-results/panel.log 2>&1 &
```

Resumable; one `SEED_BASE` for the whole panel (`$R/seed`) so every member's
columns sit on identical deals. `MEMBERS=...` restricts the run to a subset.
`ab-dump-diff` pairs `table_a` only, so effective paired boards = `--count`.

## Results

### Pilot — member #9, four-card overcalls (2026-07-29)

The motivating scenario, 8,000 boards per arm per vul, `SEED_BASE=1785265593`.
Paired `seen − blind`, so a positive number is reading value:

| vul | plain | perfect defense |
| --- | --- | --- |
| none | **+0.0454** ±0.0681 | **+0.1394** ±0.0820 |
| both | **+0.0750** ±0.0786 | **+0.1542** ±0.0932 |

All four cells positive; the two PD cells clear zero, the plain ones do not at
this size (the pilot's job was the pipeline, not the verdict). Reading is worth
+0.4 to +0.8 IMPs on each board where it changes a call.

### Full panel

*(running)*
