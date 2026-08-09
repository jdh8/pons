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

The dial is **pinned into the stance** at `Pair::against`, so the harness idiom
is unchanged: build our stance at defaults and theirs under the knob, on the
same thread. It was captured per *gauge* at book construction until 2026-08-10,
because before the pin campaign a classify-time read would have leaked the dial
into our own book. A stance now carries its own `ReadingProfile`, so the two
seats cannot see each other's dial — and the gauge constructors (`hcp`,
`points`, `support_points`, called at ~1400 `.rule()` sites) no longer need a
build-time argument to reach it. Only the magnitude is pinned; the direction is
still chosen per decision from the auction.

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

**Not comparable to `scripts/ab-blind-inference.sh`** (−0.65 … −1.27 IMPs/bd,
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

### Full panel — 2026-07-29

Twelve members (the eleven above plus the **BBA 2/1 control**, which the first
roster forgot — without an undeviated baseline the panel has no zero point).
16,000 boards per arm per vul, `SEED_BASE=1785265670`, ~31 minutes. Paired
`seen − blind`, so a positive number is reading value; CIs run ±0.045 to ±0.073.

| member | axis | plain none/both | PD none/both |
| --- | --- | --- | --- |
| **BBA 2/1 (control)** | — | +0.058 / +0.103 | +0.133 / +0.169 |
| WJ (Polish Club) | A | +0.045 / +0.116 | +0.142 / **+0.218** |
| Precision | A | +0.054 / +0.076 | +0.146 / +0.174 |
| Acol | A | +0.066 / +0.075 | +0.141 / +0.154 |
| SAYC | A | +0.061 / +0.094 | +0.133 / +0.160 |
| 2/1 + Multi 2♦ | A | +0.056 / +0.097 | +0.127 / +0.161 |
| 2/1, Cappelletti | A | +0.055 / +0.099 | +0.125 / +0.164 |
| wild weak twos | C | +0.050 / +0.076 | +0.141 / +0.165 |
| dial x=1 | B | +0.030 / +0.044 | +0.126 / +0.150 |
| off-shape 1NT | C | +0.035 / +0.046 | +0.117 / +0.126 |
| 4-card overcalls | C | +0.035 / +0.037 | +0.113 / +0.106 |
| dial x=2 | B | **−0.001** / +0.031 | +0.125 / +0.133 |

**The memorisation hypothesis is refuted.** All 22 PD cells are positive and
clear of zero, across four foreign base systems and two convention swaps. Had
`features_v3` memorised BBA's 2/1, reading value would fall off a cliff when the
opponents switch to Precision or Acol — instead WJ and Precision score *above*
the control. That is transfer, not lookup. It also closes the question
[[project_eval-v5-hcp-ends-refuted]] left open: v3's win over v4 was not
overfitting to one opponent's card.

**What does erode is real and bounded.** The B axis is the one clean
within-family gradient (same engine, one knob, two settings): plain-DD reading
value runs +0.030 → −0.001 (none) and +0.044 → +0.031 (both) from dial 1 to
dial 2, while PD holds near +0.13. Dial 2 is roughly a club player two points
off our card in both directions, and at that distance our reading is worth
nothing under plain DD. Four-card overcalls is the weakest PD member
(+0.113/+0.106 against the control's +0.133/+0.169) — a 25-35% haircut, never a
sign flip.

**Reading is a perfect-defense asset.** Every member scores 2-4× more under PD
than plain, the control included. Consistent with the standing bracket: PD
punishes the overbid, and knowing what the opponents hold is mostly how we
avoid it.

Two caveats the table cannot show:

- The A members are EPBot, the B/C members a pons book, so the *level* gap
  between those groups confounds deviation with opponent engine. Only
  within-axis comparisons are clean.
- These deviations are plausible, not adversarial. A human deliberately
  targeting our reader is a different experiment, and this panel does not
  bound it.

**Verdict against the decision rule above: the layer generalises.** Slack on
opponent boxes is not the next lever — with the asterisk that plain-DD value
does reach zero at dial 2. Promotion to a standing per-change gauge is **not**
taken: at 31 minutes for twelve members it is cheap enough to re-run on demand,
and nothing in the result argues it would move for an ordinary bidding change.
Re-run it when the *reading layer itself* changes.
