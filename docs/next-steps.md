# Next-step candidates, ranked by potential IMP gain

**Ranked 2026-09-26** from the two current anchors — BBA shipping arm at
`c3bb94a7` (2026-09-20, [bba-gap-campaign.md](bba-gap-campaign.md)) and BEN
Tier S at `daa8bf4a` (2026-09-14, [ben-gap-campaign.md](ben-gap-campaign.md)).
Method: pool size on both references × how often the vein has actually
cashed ÷ effort. Figures are IMPs/board unless marked per-fired. Re-rank
after the next re-anchor; each item's ship decision is its own fresh-seed
A/B under [measurement.md](measurement.md).

## 1. Floor junk-action rails — own doc

Active series, best expected value per effort; queue, runbook, and stop
criteria in [floor-rail-campaign.md](floor-rail-campaign.md).

## 2. RKCB / slam accuracy (Constructive / book / round-2)

**Biggest unworked pool on both references; needs a new lever, not a re-run.**

- Pool: #1 by PD on the BBA shipping arm (−45,145 ≈ −0.11/board); vs BEN
  −1.58 plain / −1.75 PD per divergent board, the worst PD/div of the BEN
  top-3, ratio only 1.4–1.5× (i.e. a genuine shared weakness, not a
  BEN-strength artifact).
- Levers on file: the Kickback three-arm A/B
  ([ai-bidder/bba-kickback.md §7](ai-bidder/bba-kickback.md) — blocked on the
  claim guard + grand discipline prerequisites), Stayman major-fit RKCB and
  post-Smolen continuations
  ([one-notrump-constructive.md](one-notrump-constructive.md)), minor-keycard
  grand handling, the parked four-four shape-slam thresholds.
- Every cheap lever here is spent (bucket marked mined-to-residual on the BBA
  side); expect design work. Even a 10% capture ≈ +0.01/board — more than
  any single rail.

## 3. BEN Defensive / book / round-1 — the next slices

Still #2 vs BEN on both scorers (−1.46 / −1.35 per div, 2.0–2.5× the BBA
ratio). The 2♣ wall is **closed** (O4-vul measured 2026-09-25, not shipped —
meta verdict: BEN's direct penalty doubles overprice the lane; do not
re-litigate). Live pieces, all in
[defensive-overcalls.md](defensive-overcalls.md):

- **O1 + the found band clause.** The unbid 5-card major at exactly 15–17
  HCP is 80% of the 1NT-overcall node's −446 plain / −1,002 PD, and BEN bids
  the major on 207/256 such hands. Formally blocked on the failed direct-1NT
  reader, but the 2026-08-13 trigger memo says a narrowed-band arm avoids
  the dilution that sank A5. Ceiling ≈ +0.004 plain / +0.008 PD.
- **O6 three-level jumps** (`P → 3x` slice, −875 plain). Opt-in knob, one
  SD-PD measurement authorized; expect DD pessimism, sd-check first.
- Standing caution from O4-vul: any slice whose PD win is doubling-driven
  needs the meta check before the paired Tier-F run.

## 4. Competitive accountant, session D arm 2

The tight-2m saga's own conclusion: both refutations said the improvable
node is the contested 5-level **declare-vs-defend decision**, not the
overcall band — and BEN, unlike BBA, actually doubles, so the lane is
finally priceable. Pool: the "we act where BEN passes" PD losses (−1,144 PD
over 572 trace boards) plus part of the −29k wall-bound remainder.
Design-heavy. Docs:
[ai-bidder/competitive-accountant.md](ai-bidder/competitive-accountant.md),
[ai-bidder/doubling-calibration.md](ai-bidder/doubling-calibration.md).

## 5. The retrain batch

Measured-and-parked wins gated on the next **matched policy/evaluator
retrain**; individually small, but they ride one retrain for free — schedule
as a batch, not as separate efforts:

- B2.5 axis mask ([bba-gap-campaign.md](bba-gap-campaign.md) open-work 7);
- the phantom-suit rail campaign (refuted 3/3 as output-side rails; the
  input-side fix is the retrain — [ai-bidder/new-suit-veto.md](ai-bidder/new-suit-veto.md));
- the reading-drift `1♦ ♦3` atom
  ([reading-drift-handoff.md](reading-drift-handoff.md));
- per [floor-rail-campaign.md](floor-rail-campaign.md) stop-criterion 5, the
  retrain also re-arbitrates every shipped rail.

~~Blocked on the M5.3 corpus question~~ — **unblocked 2026-09-26** by the
label transplant ([ai-bidder/features-v8.md](ai-bidder/features-v8.md)): a
feature bump no longer needs a relabel. v8 itself measured *suspect* (plain
win, PD erases it; opt-in) — B2.5 and the `1♦ ♦3` atom still ride the next
bump the same way, but a bump alone does not re-arbitrate the rails.

## 6. Cheap insurance + one owed decision

Near-zero direct IMPs; protects banked wins and unblocks measurement:

- Owed fresh-seed confirmations: N3-fit
  ([one-notrump-competitive.md](one-notrump-competitive.md) queue),
  Meckstroth adjunct, forcing-NT two-suiter, pass_exclusion's two seeds.
- **PD-scorer unification** (`ns_score_bid` vs `ns_score_pd`, ≈0.07/board
  definitional skew between the two campaigns' PD columns) — decision owed
  to jdh8; proposed reversible default in
  [ben-gap-campaign.md](ben-gap-campaign.md) §"the two PD scorers disagree".

## Deliberately not ranked

Recorded so future sessions don't re-derive them:

- **PDI's two owed A/Bs** — the `3237e037` anchor proved PDI, `union_hull`
  and the latch detector changed **0 boards in 819,200 table-auctions**;
  inert at scale. Still owed for completeness, not for IMPs
  ([pdi.md](pdi.md)).
- **Anything in the N4-KK lane** — below headline CI by construction (the
  lane fires on 0.06–1.17% of table-auctions).
- **Constructive opening** — mined; the light-open wall and weak-two levers
  are both refuted.
- **N3-x / N2d** (−2.9 to −3.1 per board but 25–43 boards ≈ +0.0005/board
  total) — batch them into the next 1NT-lane visit
  ([one-notrump-competitive.md](one-notrump-competitive.md)).
