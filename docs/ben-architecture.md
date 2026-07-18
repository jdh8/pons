# BEN architecture — how the north star actually bids

Reference for the **distillation phase** of the [BEN gap
campaign](ben-gap-campaign.md): a source-verified map of how BEN turns
`(hand, auction, vul)` into a call. Pinned at lorserker/ben **v0.8.8.4**, the
**21GF** (2/1 Game-Forcing) config. Checkout at `/mnt/ssd-data/jdh8/ben`
(symlink `~/ben`). File:line refs are into that checkout unless prefixed
`pons/`.

> **GPL boundary.** BEN is GPL-3.0. This document is *analysis of behaviour*
> (facts, ranges, mechanism) — free to write and to reuse. Never vendor,
> link, or embed BEN's code or weights into the Apache-2.0 pons tree. The
> distillation target is BEN's **output** (its calls), sampled black-box.

## TL;DR

- **Bidder** = a 3-layer stacked **LSTM(128)**, sequence-to-sequence,
  40-way softmax over calls. Trained on **BBA-8730** 2/1 auctions (the
  `-8730-` in every model name is the BBA build), so it is a *distilled BBA*
  with search bolted on.
- **Info net** = the same LSTM trunk with regression heads: it predicts the
  three hidden hands' **expected HCP + shape**. This is BEN's learned,
  continuous analog of pons's `Inferences`.
- **Competitive bidding is emergent**, not a module — the same
  policy→sample→DD machinery handles it, plus a *short* list of
  hand-authored patches (penalty-X, passout, preempt). **No sacrifice
  logic** exists.
- **`opponent_model = bidder_model`** in this config: BEN models the
  opponents as bidding exactly like itself. And there is **no alert
  channel** — it reads opponents from raw call tokens only.
- **Tier F** (what ben-gen mostly runs) = pure policy **argmax**, plus the
  BBA keycard handoff. **Tier S** = policy top-k → info-net-biased sampling →
  double-dummy rollout. The Tier-S−Tier-F delta *is* the search.

## 1. The per-step input tensor (193 dims)

Built by `get_auction_binary` (`src/binary.py:210-294`). It is an **RNN
sequence** `(samples, steps, 193)`; one "step" = one of the player's own
turns. With `model_version=3` and NS/EW systems set:

| idx | field | note |
| --- | --- | --- |
| 0–1 | our / their bidding-system id | both 2/1 in self-play |
| 2–3 | vuln `[us, them]` | vulnerability **is** encoded |
| 4 | own HCP, normalized `(hcp-10)/4` | |
| 5–8 | own shape, normalized `(len-3.25)/1.75` | S,H,D,C |
| 9–32 | own hand, **24 features** | `[A,K,Q,J,T,#small]` × {S,H,D,C} |
| 33–72 | **my** previous call — one-hot(40) | |
| 73–112 | **LHO** call — one-hot(40) | opponent |
| 113–152 | **partner** call — one-hot(40) | |
| 153–192 | **RHO** call — one-hot(40) | opponent |

**Seats are structurally separated.** Each of the four seats gets its own
40-wide one-hot block `[me | LHO | partner | RHO]` — the net can always tell
partner from the two opponents. `PASS=2, X=3, XX=4`, suit bids `1C=5 … 7N=39`
(`src/bidding/bidding.py:9-19`) — doubles/redoubles are distinct tokens, not
overloaded suit bids. **No alert channel** is active
(`alert_supported=False`, `BEN-21GF.conf:75`): opponents' artificial calls
arrive as bare tokens with no meaning annotation. The only "system" signal is
the two scalar id cells. Dealer/seat is encoded implicitly via `PAD_START`
padding before the dealer, not an explicit one-hot.

Hand encoding (`src/binary.py:113-141`): 6 rank-buckets per suit —
`A,K,Q,J,T` one-hot plus a 6th slot that **counts** the small cards (9 down
to 2). "AKQJTx." Cardplay uses 32 = "AKQJT98x."

## 2. The nets

**Bidder** (`scripts/training/bidding/bidding_nn_keras.py:119-138`; runtime
loads the baked `.keras`, `src/nn/bidder_tf2.py`): 3 × `LSTM(128,
return_sequences=True)` → `Dropout(0.2)` → `BatchNorm`, then
`TimeDistributed(Dense(40, softmax))`. Input 193, output **40 logits/step**
(38 real calls + 2 pad tokens). Inference takes the last step. `model_version
3` adds the player's own previous call as a 4th channel and returns
`(bids, alerts)` natively.

**Info net** (`scripts/training/bidding_info/binfo_nn_keras.py:112-131`;
`src/nn/bid_info_tf2.py`): same LSTM(128)×3 trunk, two **linear** regression
heads (MAE loss): `hcp_output` = 3 values, `shape_output` = 12 values (3
hidden hands × 4 suit lengths), ordered `[LHO, partner, RHO]`. It does **not**
emit full 52-card hands — a compact per-hand HCP+shape *summary* that guides
constrained sampling.

## 3. The decision pipeline (Tier S)

`BotBid.bid()` (`src/botbidder.py:224`; REST `/bid` at `gameapi.py:850-903`):

1. **Full-BBA shortcut** — off (`use_bba=False`).
2. **Candidates** (`get_bid_candidates:849`): first check a **BBA
   keycard/Blackwood handoff** (`:856-870`); else run the policy net
   (`next_bid_np:1030`) and take **top-k by threshold** — repeatedly argmax
   any legal bid scoring `≥ search_threshold[bid_no]`, until below threshold
   and `≥ min_candidates`. `search_threshold == -1` ⇒ exactly one argmax
   candidate, `return` at `:914`.
3. **Sample?** (`:244`) — yes unless single-candidate/no-search.
4. **Sampling** (`sample.py`, §4) — up to 50 consistent deals + a quality.
5. **Rollout?** (`do_rollout:822`) — no if one candidate / no samples.
6. **DD rollout** (`:280-330`): complete each of the ~50 auctions
   (BBA finishes them when `use_bba_rollout`, else the nets), **real
   double-dummy solve** per deal, `expected_score` averaged.
7. **Competitive adjustments** (§4).
8. **Rank** by expected IMP/MP score (or raw NN score if sample quality is
   poor).
9. **Rescue / final-contract check** — gated by `check_final_contract`.

## 4. Competitive bidding — emergent + patched + priced by DD

No competitive module, no sacrifice solver. The strength comes from three
learned parts and the DD evaluator:

- **Learned inference (Info net)** reads opponents' overcalls/doubles into an
  expected-HCP+shape summary of the hidden hands (`sample.py:501-517`).
- **Consistency reweighting** — the key mechanism. A sampled *opponent* hand
  survives only if the **same policy net** would plausibly have made that
  opponent's actual bid: keep-if `pred(actual_bid) ≥ exclude_samples=0.01`,
  scored as the weakest link across the hand's calls
  (`sample.py:519-564`). Because `opponent_model = bidder_model`
  (`nn/models_tf2.py:428-430`), **BEN's model of the opponents is its model
  of itself** — coherent, but it cannot model an opponent on a different
  system (and it has no alerts to tell it there is one).
- **Hand-authored patches** (`botbidder.py:363-494`) — the *one* place BEN
  behaves like a rule set, and a short, transferable list:
  - penalty-X trump control/void/singleton check (`:363-390`);
  - passout `adjust_X`/`adjust_XX` penalties scaled by opponent vul — biases
    *against* speculative doubles (`:404-454`);
  - balancing/`passout`: lower threshold, force ≥2 candidates, assume you'll
    be doubled (`:394-402, 920-925`);
  - after opponents preempt: force a 2nd candidate + lower threshold, comment
    *"After preempts the model is lacking some bidding"* (`:937-948`).
  - **Sacrifice: none** — comment *"we should probably try to detect if they
    are sacrificing"* (`:432`). It falls out of the DD expected-score
    comparison plus the X penalties.
- **`undisturbed()`** routes to a *larger* NN-trust bonus (200 vs 60) —
  BEN trusts the net more when uncontested and leans harder on DD in
  competition (`:355`).

## 5. BBA's role

2/1 GF throughout (`bba_our_cc = BBA/CC/BEN-21GF.bbsa`). Two couplings:

- **Soft blend** (`consult_bba`, `bba_trust=0.2`, `:988-1026`): if BBA's bid
  matches an NN candidate, `+0.2` to its score; else inject BBA's bid as a
  low-weight candidate. BBA also vetoes some passes it reads as forcing/GF.
- **Hard keycard handoff** (`use_bba_to_count_aces`, `:863-870`): on a
  Blackwood/keycard/controlling sequence BEN **returns BBA's bid directly**
  (`insta_score=1`), bypassing the net. **RKCB answers are BBA's, not the
  net's** — so slam/keycard nodes distill as *BBA rules* (already reverse-
  engineered), not the NN's judgment. `find_aces` also feeds ace/king
  placement into sampling.

## 6. Tier F vs Tier S

Tier F = `pons/vendor/ben/BEN-21GF-F.conf`, a two-edit derivation of stock:
`search_threshold=-1`, `check_final_contract=False`.

| | Tier F | Tier S |
| --- | --- | --- |
| competitive call | pure NN **argmax**, first legal bid | top-k → sampling → DD rollout → expected-IMP pick |
| Info-net inference | unused | drives sampling |
| DD search | none | ~50 deals × DD solve per candidate |
| BBA soft blend (+0.2) | **dropped** (returns before the blend at `:914`) | active |
| BBA keycard handoff | **kept** (runs before the argmax branch) | active |

So a normal Tier-F competitive decision is a **single policy forward pass**;
the only non-NN survivor is the BBA keycard handoff.

## 7. What this means for distillation

- **Distill Tier F** (pure policy) through the `probe-brl-book` box-fitter
  (`pons/examples/probe-brl-book`) → the per-node rules-vs-**ceiling** table.
  That is the fair rule-vs-rule comparison against pons's floor, and it
  answers "how many rules / what ceiling."
- **Two residuals, do not conflate:**
  1. *Tier-F ceiling gap* — "policy isn't an (hcp, shape) rule": some is
     honor-location / sequence-specific, distillable with richer features.
  2. *Tier-S − Tier-F* — "policy alone is wrong, DD fixes it": **not
     authorable**, it is search. pons already ships DD; close this with
     search, not rules. Globally measured at **≈0.8–1.1 IMPs/board**
     (ben-gap ledger, Tier-F gap row).
- **Competitive DD-dependence hides in Tier S, not a Tier-F corpus.**
  Measured by the 2026-07-18/19 distillation probes (ben-gap ledger): Tier-F
  competitive-**entry** ceilings are *high* (LHO overcalls 96–99%), and the
  2026-07-19 **contested-biased** corpus extended this *into the deep tail* —
  advancer / competitive-rebid nodes ceiling at **92–100%** too, no collapse.
  So the *policy prior* over competition is ruly at every depth, not just at
  entry. The DD-trick-math dependence shows up only where Tier-S *overrides*
  that ruly prior — which a Tier-F (no-DD) corpus cannot see at any depth.
  Distilling that residual therefore needs Tier S itself, not a deeper Tier-F
  corpus. Two riders from the contested probe: exact-tuple coverage is thin at
  contested depths (~24%, so the ceiling proves ruliness on *seen* shapes and
  wants a generalizing floor, not a lookup), and the axis-aligned box-fitter
  lags the ceiling *wider* in competition (non-Pass ≈65% vs 92–100%) — so the
  distillable competitive rules want richer-than-box features (honor location,
  exact fit length, sequence), more than constructive ones do.
- **Exploitable asymmetry:** no alert channel + `opponent_model =
  bidder_model` ⇒ BEN mis-models opponents on a different or artificial
  system. pons alerts everything and can model the opponents' real system.
  Artificial/unusual pons competitive calls may be systematically
  mis-inferred by BEN's Info net — the standing rationale for keeping BBA as
  the exploit guard, and a possible *edge* worth an A/B.
- **BEN's own competitive patches** (§4) are a short, directly transferable
  rule list — a concrete "distill into documents" win comparable to pons's
  competitive book + floors.
