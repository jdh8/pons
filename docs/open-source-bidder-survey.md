# Open-source bridge bidder survey (July 2026)

**Question:** is pons the strongest open-source bridge bidder?

**Verdict: no — on the best available evidence, pons is not currently the
strongest open-source bidder.** Two genuine open-source rivals exist — BEN
and brl — and BBA's own website (found after the initial sweep, § 3a)
anchors EPBot against WBridge5, BEN, GIB, and Lia in cross-engine bidding
tables. That closes the chain the first pass flagged as missing: pons trails
EPBot by ≈1.7–1.9 IMPs/board (private anchor), EPBot trails WBridge5 by
≈0.4 (DD) and **loses to current BEN by ≈0.35–0.38 (DD)**, and brl carries a
peer-reviewed +1.24 over WBridge5. Chained nominally, pons sits ≈2.1 behind
BEN and ≈3.4 behind brl — rough transitivity across different harnesses, but
every link points the same way with margins far above quoted noise. A live
pons-vs-BEN match (§ Feasibility) remains the way to measure the gap
honestly rather than infer it.

Produced by a 100-agent deep-research sweep (18 sources fetched, 88 claims
extracted, 25 adversarially verified: 20 confirmed / 5 refuted), with the
load-bearing claims re-checked by hand against primary sources on
2026-07-16, plus a manual dig that recovered BBA's cross-engine tables from
its test-results page (the sweep's summarizer had missed them).

## 1. Open-source engines

Genuinely open-source — license, code, and (where applicable) weights:

| Engine | License | Approach | System | Activity | Measured strength |
| --- | --- | --- | --- | --- | --- |
| **BEN** ([lorserker/ben](https://github.com/lorserker/ben)) | GPL-3.0 | Neural call policy trained on human deals + DD solver (python-dds) sampling/rollout | Configurable via convention card; published match model trained on SAYC | Active: 615 commits, 17 releases, v0.8.8.4 (2026-06-12) | **None self-published.** Third-party: WBridge5 5.12 (level 4) beat BEN v0.2 by 12 IMPs over 160 boards (≈0.075 IMPs/bd) on Camrose 2024 deals — near parity, but old BEN version, informal method ([BridgeWinners](https://bridgewinners.com/article/view/ben-vs-wbridge5-in-the-camrose-trophy-2024/)) |
| **brl** ([harukaki/brl](https://github.com/harukaki/brl)) | Apache-2.0 | SL pretrain + PPO with fictitious self-play (JAX/pgx, 4×1024 MLP); weights released | Learned, non-human system | Research baseline, 146 commits; reference impl of Kita et al. 2024 | **+1.24 ±0.19 IMPs/bd vs WBridge5** (highest level, 1K boards), peer-reviewed (IEEE CoG 2024) |
| **pons** (this repo) | MIT/Apache-2.0 | Rule book (2/1 GF) + inference engine + learned floors + DD search | 2/1 game forcing | Active | ≈−1.7 to −1.9 IMPs/bd vs BBA/EPBot (private anchor); chains to ≈−2.1 vs WBridge5 via § 3a |
| zizhang-qiu/BridgeBidding | Apache-2.0 | Supervised imitation (PyTorch) | Learned | Training codebase, not a complete bidder | Relationship to Qiu et al. 2024 (+0.98) **unresolved** — do not cite as its release |
| saycbridge, GNUBridge-era relics | various | Rules | SAYC etc. | Dormant | None credible |

Free-but-closed binaries — **yardsticks, not competitors**: BBA/EPBot, GIB,
WBridge5, Jack, Argine, Q-Plus, Bridge Baron.

Notes:

- BEN publishes **zero** quantitative strength claims itself (verified 6-0
  and re-checked by hand) — but BBA's site measures it: current BEN
  v0.8.8.4 beats EPBot by 0.35–0.38 IMPs/deal DD (§ 3a), making BEN the
  measured open-source front-runner among human-system bidders.
- brl's model weights ship in-repo (`model-pretrained-rl-with-fsp.pkl`,
  README table: 1.24 IMPs/b); its WBridge5 evaluation harness requires
  Windows (WBridge5 only runs there).
- An engine-agreement study (bridge-llm-bench) found WBridge5, BEN (SAYC),
  and saycbridge agree on only 44-64% of calls pairwise — "accuracy vs a
  reference engine" is a fragile strength proxy.

## 2. Publications

| Paper | Year/venue | Result vs WBridge5 | Sample | Code/weights |
| --- | --- | --- | --- | --- |
| Ginsberg — GIB | 1999-2001 | (predates; GIB is the classic yardstick) | — | closed |
| Yeh & Lin | 2016 | (self-play / DD framework, no WBridge5 match) | — | no |
| Rong et al. (ENN+PNN) | 2019 arXiv/AAMAS | +0.25 | **64 boards, manual**, DD-scored; SE ≈0.7 dwarfs the margin | **no** (public program listed as future work, never released) |
| Gong et al. | 2019 | +0.41 ±0.27 | 64 boards | no |
| Tian et al. (Joint Policy Search) | 2020 NeurIPS | +0.63 ±0.22 | 1K boards | partial (JPS code) |
| Lockhart et al. (OpenSpiel) | 2020 | +0.85 ±0.05 | 10K boards | partial |
| Qiu et al. (RL + belief MC search) | 2024 IEEE/CAA JAS | +0.98 | 10K deals | **unresolved** (see above) |
| **Kita et al. (brl)** | 2024 IEEE CoG | **+1.24 ±0.19** | 1K boards | **yes — code + weights** |

### Measurement pitfalls (weigh every row above by these)

- **All rungs are bidding-only, DD-scored.** The auction runs live (brl uses
  a localhost network match with duplicate boards) but the final contract is
  scored by double-dummy analysis, not actual play. A claim that brl's
  evaluation was a full-play duplicate match was **refuted 0-3** in
  verification. This is exactly the protocol pons's own iron rules flag as
  inflating: DD is blind to obstruction, concealment, and right-siding.
- Early rungs are statistically hollow: Rong and Gong used 64 boards with
  SE ≈0.7 IMPs/bd against claimed margins of 0.25-0.41.
- Kita's 0.39 edge over Lockhart is only ~2 combined standard errors, and
  the paper itself flags a fairness asymmetry (the agent consults DDS during
  bidding; WBridge5 does not).
- Protocols are heterogeneous — the ladder rows are only loosely comparable
  to each other, and **not directly comparable to pons's plain-DD/PD/sd
  regime at all**.

## 3. Strength ladder

Public, WBridge5-anchored (all caveats above apply):

```text
WBridge5 (closed, multi-time WCBC champion)
  +0.25  Rong 2019        (64 boards, DD-scored, no code)
  +0.41  Gong 2019        (64 boards, no code)
  +0.63  Tian/JPS 2020    (1K boards)
  +0.85  Lockhart 2020    (10K boards)
  +0.98  Qiu 2024         (10K deals, code status unresolved)
  +1.24  Kita/brl 2024    (1K boards, OPEN: code + weights)
  ≈0     BEN v0.2 2024    (−12 IMPs / 160 boards, informal third-party match)
```

## 3a. BBA's own cross-engine tables — the missing rung

BBA's [test-results page](https://sites.google.com/view/bbaenglish/test-results)
carries HTML tables (recovered by hand 2026-07-16; the sweep's page
summarizer missed them) that anchor EPBot to the public ladder. Protocol:
two 4-bot teams bid random hands, each bot sees only its own cards, **the
lead and play are not analyzed** — contracts are scored by a single-dummy
estimate (leader's-view) and by double dummy. Quoted accuracy ≈±0.04
IMP/deal at 20K hands.

**Table 1 — EPBot v.8741 vs other engines** (10K hands, IMP/deal from
EPBot's side, SD / DD):

| Opponent | CC | SD | DD |
| --- | --- | --- | --- |
| BEN v0.8.8.4 | BEN-21GF | −0.51 | **−0.38** |
| BEN v0.8.8.4 | BEN-Sayc | −0.47 | **−0.35** |
| GIB v6.1 | GIB | +0.37 | +0.15 |
| GIB v6.2 | GIB | +0.38 | +0.07 |
| Lia v2.6.2 | LIA-21GF | −0.38 | −0.24 |
| Lia v2.6.0 | LIA-Sayc | −0.19 | −0.10 |
| WBridge5 v5.12 L4 | WB5-Sayc | −0.57 | −0.32 |
| WBridge5 v5.12 L4 | WB5-SEF | −0.64 | −0.35 |
| WBridge5 v5.12 L4 | WBridge5 | −0.69 | **−0.40** |

**Table 2 — other bots head-to-head** (WBridge5 at level 2 only, old BEN):
BEN v0.2 loses to WBridge5-L2 by 0.27–0.41; GIB 6.x loses by ≈0.34–0.74.

The chained picture (DD bracket, nominal transitivity):

```text
  +1.24      brl            (vs WBridge5 L4, its own protocol)
   0.00      WBridge5 L4
  −0.35…−0.38  BEN v0.8.8.4 beats EPBot  →  BEN ≈ WBridge5-level (matches Camrose near-parity)
  −0.40      EPBot v.8741   (vs WBridge5 L4, BBA's protocol)
  −0.5…−0.6  GIB 6.x        (EPBot beats GIB by 0.07–0.15 DD)
  −2.1…−2.3  pons           (−1.73/−1.89 vs EPBot, our harness)
```

Caveats on the chain: three different harnesses (BBA's SD/DD bidding-only,
Kita's protocol, our anchor), different deal distributions, and our vendored
EPBot build's vintage vs the site's v.8741 is unverified (our DLL reports
only assembly version 11.0.0.0). Nominal sums are indicative, not
measurements. But every link has the same sign in both SD and DD brackets,
and the margins are 10–50× the quoted noise.

Corroborations: BBA's own Table 3 (systems, not engines) has WJ beating
2/1GF by +0.038/+0.054 SD/DD — the same magnitude and direction as our
long-standing "WJ ≈ 2/1" note; and GIB placing weakest among the yardsticks
matches its reputation.

### Honest reading

- The claim "pons is the strongest open-source bidder" is **refuted at
  moderate confidence**: BEN (open-source, GPL-3.0) beats EPBot outright in
  BBA's own tables, and pons trails that same EPBot by ≈1.7–1.9. brl sits
  higher still via its WBridge5 win. Only a wholesale failure of
  cross-harness transitivity would rescue the claim.
- The DD-scoring caveat still matters for the *size* of the gaps: bidding-
  only DD protocols reward thin games and ignore play, so the ≈2.1-behind-
  BEN figure is an estimate, not a measurement. It does not plausibly flip
  the sign.
- brl plays a learned, non-human system with DDS consultation — strong at
  the protocol it was scored under, but it is not a disclosable-system
  bidder in the sense pons and BEN are. For "strongest human-system
  bidder", BEN is the rival that matters — and it now has a number.

## 4. Head-to-head feasibility (no code yet)

> **Update 2026-07-16:** this section graduated into design docs — the
> campaign plan is [ben-gap-campaign.md](ben-gap-campaign.md) and the
> harness design is [ben-gen-design.md](ben-gen-design.md).

**BEN — cheapest and most meaningful match.**

- Interfaces: documented localhost REST API for bidding/play/analysis
  (default port 8085, `README-api.md`); websocket gameserver (4443) +
  HTTP appserver split; `table_manager_client.py` speaking the Blue Chip
  table-manager TCP protocol v18; `game.py` self-play CLI.
- System: configurable convention card; stock models trained on human deals
  (SAYC-flavored in the published match).
- Integration: a Rust process bridge driving the REST API is the obvious
  path — same shape as the existing `bba-gen` harness, but HTTP instead of
  FFI. Rough cost: a `ben-gen` binary (serde JSON client + the existing
  board/scoring pipeline) — days, not weeks; BEN's Python server runs
  locally on this box (mind shared-machine etiquette for the NN inference
  load).
- Protocol choice matters: score the match with our own dual bracket
  (plain DD + PD, sd where relevant), not the literature's DD-only-bidding
  protocol, and report both.

**brl — most directly comparable on bidding, clunkier ops.**

- Python/JAX checkpoints, no serving API; its WBridge5 harness assumes
  Windows for WBridge5 but a pons match doesn't need WBridge5 — a thin
  Python driver loading the pgx env + checkpoint and speaking JSON lines to
  a Rust harness would do.
- Caveat: brl bids a private learned system. There are no disclosures/alerts
  to read, so pons's inference engine plays blind against it in competition
  — that asymmetry is itself part of what a match would measure.

**Do not** rely on OpenSpiel's BlueChip-protocol wrapper as a ready-made
interop path — the claim that it is verified against WBridge5 was refuted
0-3 in verification; re-verify before building on it.

## Open questions

1. ~~Is BBA/EPBot anchored to any public benchmark?~~ **Answered** — BBA's
   own test-results tables (§ 3a): EPBot −0.40 DD vs WBridge5 L4, −0.35…
   −0.38 DD vs BEN v0.8.8.4, +0.07…+0.15 DD vs GIB 6.x.
2. How much do the DD-scored ladder results shrink under real card play or
   a perfect-defense bracket — would brl still beat WBridge5 in full play?
3. A live pons-vs-BEN match over the REST API remains the direct
   measurement — the § 3a chain estimates pons ≈2.1 behind current BEN, but
   across three different harnesses.
4. Does Qiu et al. 2024 have a genuine public code/weights release?
5. Which build is our vendored EPBot (v.7340-era or v.8741-era)? The DLL
   reports only assembly version 11.0.0.0; the site's tables use build
   numbers. Affects how our private anchor chains onto § 3a.

## Sources

Primary: [lorserker/ben](https://github.com/lorserker/ben) ·
[BEN project site](https://lorserker.github.io/ben/) ·
[harukaki/brl](https://github.com/harukaki/brl) ·
[Kita et al. 2024, arXiv:2406.10306](https://arxiv.org/abs/2406.10306) (IEEE CoG 2024) ·
[Qiu et al. 2024, IEEE/CAA JAS](https://www.ieee-jas.net/article/doi/10.1109/JAS.2024.124488) ·
[Rong et al. 2019, arXiv:1903.00900](https://arxiv.org/abs/1903.00900) ·
[Tian et al. 2020, arXiv:2008.06495](https://arxiv.org/abs/2008.06495) ·
[Lockhart et al. 2020, arXiv:2011.14124](https://arxiv.org/abs/2011.14124) ·
[zizhang-qiu/BridgeBidding](https://github.com/zizhang-qiu/BridgeBidding) ·
[google-deepmind/open_spiel](https://github.com/google-deepmind/open_spiel)

Primary (added after the sweep): [BBA test results — cross-engine tables](https://sites.google.com/view/bbaenglish/test-results)

Secondary: [BridgeWinners: BEN vs WBridge5, Camrose 2024](https://bridgewinners.com/article/view/ben-vs-wbridge5-in-the-camrose-trophy-2024/) ·
[Computer bridge (Wikipedia)](https://en.wikipedia.org/wiki/Computer_bridge) ·
[World Computer-Bridge Championship](https://bridgebotchampionship.com/home/world-computer-bridge-championship/) ·
[bridge-llm-bench](https://github.com/albertogerli/bridge-llm-bench)

Repo facts (versions, licenses, weights) verified as of 2026-07-16 and will
drift.
