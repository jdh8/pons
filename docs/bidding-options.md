# Bidding options audit

One row per `set_*` bidding knob, with its recorded A/B verdict and the
ship/opt-in decision that verdict justifies. This is the **global option index**;
the rich per-run provenance lives in the campaign ledgers
([competitive-book.md](competitive-book.md), [ai-bidder/21gf-ledger.md](ai-bidder/21gf-ledger.md))
and [CHANGELOG.md](../CHANGELOG.md) — this doc links to them, it does not replace them.

**This is a harvest, not a fresh measurement pass.** Numbers were collected from
MEMORY, CHANGELOG, and the two ledgers. Options with no isolated A/B are marked
`unmeasured` (a worklist, see the bottom); figures predating the 2026-06 PD→plain
harness move (`a6f2206`) or a book-population shift are flagged `stale-*` and are
**re-measure candidates, not live magnitudes**.

## The policy this audit applies

Classify each option on two axes — **natural vs artificial** and **A/B sign vs
the floor** — then read the action:

|            | `> floor`                | `= floor` (CI ⊇ 0)       | `< floor`          |
| ---        | ---                      | ---                      | ---                |
| **Natural**    | fold into base (retire knob) | fold into base       | improve it         |
| **Artificial** | default-on               | opt-in                   | improve, else drop |

- **Natural vs artificial** = the per-rule `Alert`: `None` = natural,
  `Some(_)` = artificial. Backed by the structural predicate `artificial()`
  ([src/bidding/inference.rs:1485](../src/bidding/inference.rs#L1485)) — a bid is
  artificial iff its authoring rule floors an *unnamed* suit — and enforced by the
  invariant test `artificial_calls_are_alerted`
  ([inference.rs:3794](../src/bidding/inference.rs#L3794)). Passes and doubles are
  never artificial.
- **`> / = / < floor`** = the sign of the isolated A/B (`arm --on-ns-<knob>` vs
  `--no-ns-<knob>` → `diffpair`, dual plain/PD scoring; `=` when the CI includes
  0), per [measurement.md](measurement.md).
- **Engine** rows (inference/floor/evaluator toggles) are not a book of calls;
  they read or settle. They are judged by their measured sign, kept default-on
  when they help, and never carry a natural/artificial verdict.
- **Suppress-* discipline** rows gate *out* an over-eager (usually takeout-double)
  action in favour of a natural bid or pass; the suppression is natural-side
  discipline, so they read as Natural.

### The fold-into-base caveat

The matrix says a natural option at/above the floor should be **folded into base
and the knob retired**. In practice the house keeps the shipped default-on knob's
`--no-ns-*` off-switch (and a byte-identical off-state) so the change stays
**re-measurable**. So "fold into base" here is the *system* verdict; the knob may
survive as a `#[doc(hidden)]` measurement switch. Actually retiring a knob is a
follow-up, each with its own re-measure — **out of scope for this document-only
pass.**

### Freshness vocabulary

- `fresh` — measured post-`a6f2206` on the current book population.
- `stale-PD` — pre-`a6f2206` PD-era figure, **not comparable** to plain-DD.
- `stale-pop` — measured before a book-population shift that could flip it.
- `unmeasured` — never isolated (worklist item).

---

## Tier A — book & style options

The natural/artificial × floor policy targets these. Numeric-tuning parameters
that live *inside* a book are in Tier B.

### A1 — Openings, responses, rebids (constructive)

| Option (knob) | CLI | Nat/Art | Default | A/B verdict | Fresh | Policy → action |
| --- | --- | --- | --- | --- | --- | --- |
| set_open_one_notrump | `--no-our-1nt` | Natural | ON | Diagnostic isolation knob; whole-system 1NT gap `--filter-1nt` −1.05 IMPs/bd, leak is constructive ([project_bba-1nt-comparison]) | stale-pop | keep on; "improve" = the constructive-1NT campaign, not knob retirement |
| NotrumpShape (1NT open) | `?` (build enum; `ab-nt-shape`) | Natural | Wide (Balanced/Wide/Wide6322) | 5422-minor Wide vs Balanced classic +0.57/+0.93 (PD-era per-divergent); Wide6322 contested re-test **rejected** | stale-PD | fold into base (Nat≥floor); Wide6322 stays opt-in |
| set_one_notrump_fifths | `--nt-fifths` | Natural | OFF | — unmeasured at the *open* boundary; Fifths refuted at the *invite* boundary ([project_nt-invite-evaluator-sweep]) | unmeasured | needs A/B |
| set_rule_of_20 | `--no-ns-rule-of-20` | Natural | ON | plain +0.0061/+0.0087, PD −0.0056/−0.0034 (doubler artifact), sd-lead +0.0096/+0.0135 (SEED 1783410574, `rule-of-20-ab.sh`) | fresh | fold into base |
| set_longer_major_response | `--no-ns-longer-major-response` | Natural | ON | plain-DD wash, PD −0.12/−0.22 per div; kept by naturalness tiebreak (commit 2ba6b90) | fresh | fold into base |
| set_up_the_line | `--no-ns-up-the-line` | Natural | ON | **coupled with XYZ**: joint plain +0.0382/+0.0559, PD +0.0289/+0.0407; alone a loss −0.91/−1.28 per div | fresh | fold into base *with* XYZ — do not enable independently |
| set_major_game_tries | `--no-ns-major-game-tries` | Natural | ON | plain +0.042/+0.065 (both scorers win); package w/ FSF+limit-accept +0.058/+0.089 ([project_major-continuations]) | fresh | fold into base |
| set_limit_raise_acceptance | `--no-ns-limit-raise-acceptance` | Artificial | ON | plain +0.002/+0.002; load-bearing part is the 4NT keycard ask +4.4/+5.2 IMPs/div | fresh | default-on ✓ |
| set_meckstroth_adjunct | `meckstroth-abc` (no bba-gen flag) | Artificial | ON | — unmeasured (harness exists, no result recorded); 21gf-ledger "complementary to XYZ, keep" | unmeasured | needs A/B |
| set_balanced_1nt_rebid | `--no-ns-balanced-1nt-rebid` | Natural | ON | plain +0.0076/+0.0085 NV, +0.0109/+0.0117 vul ([project_balanced-1nt-rebid]) | fresh | fold into base |
| set_opener_extras_ladder | `--no-ns-opener-extras-ladder` | Natural | ON | plain +0.0203/+0.0332, PD +0.0181/+0.0297 (SEED 1783544590, `opener-extras-ladder-ab.sh`) | fresh | fold into base — but reverse/jump-shift rungs carry a toggle-gated reading |
| set_opener_major_jump_rebid | `--no-ns-opener-major-jump-rebid` | Natural | ON | plain +0.0059/+0.0125, PD +0.0046/+0.0104 (SEED 1783549337). Bare rung LOST; win needed responder's continuation (author both sides) | fresh | fold into base |
| set_major_rebid_tails | `--no-ns-major-rebid-tails` | Natural | ON | plain +0.016/+0.023 NV/vul | fresh | fold into base |
| set_fourth_suit_forcing | `--no-ns-fourth-suit-forcing` | Artificial | ON | plain +0.002/bd both scorers/vuls, atop the tails; part of the +0.058/+0.089 major-continuations package. Rides `set_major_rebid_tails` | fresh | default-on ✓ |
| set_second_suit_agreement | `--no-ns-second-suit-agreement` | Artificial | ON | plain +0.0012/PD +0.0014 NV, +0.0015/+0.0018 vul (marginal; payoff in the RKCB-on-extras tail) | fresh | default-on ✓ |
| set_xyz | `--no-ns-xyz` | Artificial | ON | joint w/ up-the-line +0.0382/+0.0559; XYZ alone +0.504/+0.795 per div plain, +0.332/+0.472 PD | fresh | default-on ✓ |

### A2 — Our 1NT: Stayman, transfers (artificial constructive)

| Option (knob) | CLI | Nat/Art | Default | A/B verdict | Fresh | Policy → action |
| --- | --- | --- | --- | --- | --- | --- |
| set_notrump_minors (PUPPET/EUROPEAN) | `set_notrump_minors(EUROPEAN)` (no flag) | Artificial | PUPPET | Puppet-vs-European never isolated; the scheme as a whole +0.76/+1.15 ([project_minor-transfers-puppet], PD-era) | unmeasured | needs A/B (isolate Puppet vs European) |
| set_transfer_super_accept | `--ns-transfer-super-accept` | Artificial | OFF | DD wash leaning neg, −0.055 IMPs/fired (640k) | stale-PD | opt-in |
| set_transfer_slam_try | `--no-ns-transfer-slam-try` | Artificial | ON | plain +0.0012 ÷ PD +0.0012 (+1.42/fired, 320k, CI>0) | fresh | default-on ✓ |
| set_texas_slam_drive | `--no-ns-texas-slam-drive` | Artificial | ON | plain +0.0024 ÷ PD +0.0024 (+5.87/fired, 320k, CI>0) | fresh | default-on ✓ |
| set_transfer_gf_majors | `--no-ns-transfer-gf-majors` | Artificial | ON | plain +0.0014 ÷ PD +0.0016 (+1.70/+1.90 fired) | fresh | default-on ✓ |
| set_minor_min_to_3nt | `--ns-minor-min-to-3nt` | Artificial | OFF | losing arm B of the gf-majors A/B; show-the-minor default beat it (CHANGELOG) | fresh | opt-in / improve (losing arm) |
| set_transfer_gf_hearts | `--no-ns-transfer-gf-hearts` | Artificial | ON | plain +0.0015/+0.0017 ÷ PD +0.0016/+0.0018 (two seeds) | fresh | default-on ✓ |
| set_garbage_stayman | `--no-ns-garbage-stayman` | Artificial | ON | plain +0.51/fired ÷ PD +0.70/fired (fires 0.17%) | fresh | default-on ✓ |
| set_stayman_both_majors | `--no-ns-stayman-both-majors` | Artificial | ON | plain +2.18/fired ÷ PD +2.29 (right-siding relay, 320k) | fresh | default-on ✓ |
| set_stayman_5card_max | `--no-ns-stayman-5card-max` | Natural | ON | plain +3.45/fired ÷ PD +3.33 (3♥/3♠ jump is natural/unalerted; a winning capability-add) | fresh | default-on ✓ (winning add; keep knob despite Nat label) |
| set_invitational_5card_majors | `--no-ns-invitational-5card-majors` | Artificial | ON | plain +0.375/fired ÷ PD +0.134/fired (1.28M, CI>0). Needed doubled-2♦ systems-on companion | fresh | default-on ✓ |
| set_crawling_stayman | `--no-ns-crawling-stayman` | Artificial | ON | plain +1.539/fired ÷ PD +2.055/fired (1.6M, PD>plain, not a doubling artifact) | fresh | default-on ✓ |
| set_transfer_longer_major | `--no-ns-transfer-longer` | Artificial | ON | DD wash (204.8k, 47 fired); shipped on structural grounds (deterministic route, partner infers longest) | fresh | default-on ✓ (structural, not a measured win) |
| set_stayman_cue_continuation | `--no-ns-stayman-cue-continuation` | Artificial | ON | plain +0.0193 ÷ PD +0.0216 ([project_stayman-slam-deficit] FIX1); closes cue-passed-below-game dead end | fresh | default-on ✓ |
| set_stayman_minor_slam_try | `ab-stayman --treatment` (no flag) | Artificial | ON | plain +3.29/+4.02 IMPs/fired ÷ PD identical (1.5M, 151 fired, zero losses) | fresh | default-on ✓ |
| set_long_minor_force | `ab-long-minor-force` (no flag) | Artificial | OFF | **measured LOSS** plain −7.12 IMPs/fired vs real transfer routing (8M, plain-DD only) | fresh (plain-only) | improve/drop — kept only as A/B instrument |

### A3 — Our 1NT: competition, runouts & escapes

| Option (knob) | CLI | Nat/Art | Default | A/B verdict | Fresh | Policy → action |
| --- | --- | --- | --- | --- | --- | --- |
| set_one_nt_runout | `ab-one-nt-runout` (no flag) | Artificial | ON | qualitative — 5+ escape / XX=business / 2NT=both-minors / SOS ([project_one-nt-doubled-runout]) | unmeasured | default-on ✓ |
| set_one_nt_runout_universal | `ab-one-nt-runout --universal` | Artificial | ON | qualitative — opener also escapes / balancing SOS-XX | unmeasured | default-on ✓ |
| set_unusual_2nt (FourFour/FiveFiveAdd/Direct) | `ab-one-nt-runout --compare …` | Artificial | Direct | phase-2 default Direct (direct minor escape beats relay) | fresh | default-on ✓ (Direct) |
| set_penalty_latch | `--no-ns-penalty-latch` | Natural | ON | X bucket self-play −0.621→−0.464, vs BBA −2.716→−2.329 IMPs/X-bd; no regression | fresh | default-on ✓ (no-op unless natural 1NT-defense on) |
| set_latch_style (Penalty/Optional) | dedicated (no flag) | Natural | Penalty | defensive latch = DD wash ([project_double-style-penalty-leavein]) | fresh | keep Penalty; Optional opt-in (wash) |
| set_penalty_no_pull | `--ns-allow-pull` (**inverted**) | Natural | ON | X bucket −2.312→−1.013 vul; paired +0.058 IMPs/bd CI [+0.030,+0.085] | fresh | default-on ✓ |
| set_advancer_xx_runout | `--no-ns-xx-runout` | Artificial | ON | [project_penalty-x-runout] default-on (advancer escape from `[1NT,X,XX]`) | fresh | default-on ✓ |
| set_doubler_xx_runout | `--no-ns-doubler-run` | Artificial | ON | [project_doubler-xx-runout] default-on; rare | fresh | default-on ✓ |
| set_uvu_encircle | `--uvu` (with set_uvu) | Artificial | ON | [project_uvu-over-1nt-2nt] default-on; dormant unless UvU X bid | fresh | default-on ✓ |
| set_correct_3nt_to_major | `--no-ns-correct-3nt-to-major` | Natural | ON | [project_nt-9count-gameforce-seam], gated default-on (3NT→4M on 8-card fit) | fresh | fold into base |
| set_suppress_nt_game_force_over_double | `--no-ns-suppress-nt-gf-over-double` | Natural | ON | [project_nt-9count-gameforce-seam] default-on (suppress 1NT-(X)-3NT overbid) | fresh | fold into base |
| set_gambling_3nt_over_double | `--ns-gambling-3nt` | Artificial | OFF | net DD-negative (1NTxx baseline too strong) ([project_gambling-games-over-1ntx]) | fresh | stays opt-in (measured loss) |
| set_preempt_4m_over_double | `--ns-preempt-4m` | Artificial | OFF | net DD-negative, same project | fresh | stays opt-in (measured loss) |
| set_penalize_escape_stack | `ab-one-nt-runout --compare escape-stack` | Artificial | ON | on by default, no isolated headline | unmeasured | default-on ✓ |
| set_penalize_escape_values | `ab-one-nt-runout --compare escape-values` | Artificial | ON | on by default, no isolated headline | unmeasured | default-on ✓ |
| set_lebensohl_style (Off/Plain/Transfer) | `ab-lebensohl` (no flag) | Artificial | Transfer | shipped structure (2NT-relay transfers, Smolen, Leaping Michaels); Transfer-vs-Plain never isolated | unmeasured | default-on ✓ (structure) |
| set_lebensohl (shim) | — | — | on=Transfer / off=Off | back-compat shim onto `set_lebensohl_style` | — | see set_lebensohl_style |
| set_penalty_double_leave_in | `ab-lebensohl` (no flag) | Natural | ON | opener's 3NT escape LOST vs sitting: +0.328 vs +0.507 IMPs/div | fresh | fold into base (keep sit) |
| set_double_style (Takeout/Penalty/PenaltyLight/Optional) | `ab-lebensohl` (no flag) | Natural | Optional | Optional > Penalty > Takeout: +1.59 vs penalty, +2.14 vs takeout IMPs/div (robust once opener cooperates) | fresh | fold into base (Optional) |
| set_competitive_4333 (Allow/Suppress/SuppressWithStopper) | `--ns-competitive-4333` | Natural | Suppress | [project_curse-of-4333]: flat 4333 never Staymans (`& !flat_4333()` shipped) | fresh | fold into base (Suppress) |

### A4 — Competitive auctions (they overcall / double our opening)

| Option (knob) | CLI | Nat/Art | Default | A/B verdict | Fresh | Policy → action |
| --- | --- | --- | --- | --- | --- | --- |
| set_negative_double_shape (BothMajors/Modern/Cachalot/Sputnik) | `--ns-negative-double-shape` | Artificial | Modern | Modern shipped: plain +0.0213 NV/+0.0074 vul (CI>0), sd +0.4221/+0.2881; Cachalot loses all 6 cells, Sputnik wash (SEED 1783672667/1783679026) ([project_fallback-competitive-classify]) | fresh | default Modern default-on ✓; Cachalot/Sputnik opt-in (loss/wash) |
| set_free_bid_style (Forcing/Negative/Transfer) | `--ns-free-bid-style` | Natural (Forcing) | Forcing | Negative: plain wash/PD loss/sd-win-only → no ship; Transfer: LOSS all 3 scorers both vuls (SEED 1783681411) | fresh | default Forcing default-on ✓; others opt-in (loss) |
| set_free_bids | `--ns-free-bids` | Natural | OFF | plain +0.29 NV / −0.30 vul (CI<0), PD −0.31/−0.88 (P3b, SEED 1783286814); leak structural | fresh | stays opt-in (measured vul loss) |
| set_free_bid_quality | `--ns-free-bid-quality` | Natural | OFF | gate REFUTED — vul plain −0.0042 (CI<0), suppressed *winning* junk frees (SEED 1783666604) | fresh | stays opt-in (measured loss) |
| set_high_overcall_responses | `--ns-high-overcall` | Artificial | OFF | plain −0.0012/−0.0007, PD −0.0005/−0.0006 (all CI⊇0, SEED 1783286003); named leak: 3-level neg-X too light | fresh | stays opt-in; re-measure candidate |
| set_jordan_truscott | `--no-ns-jordan-truscott` | Artificial | ON | plain +0.0041/+0.0067, PD +0.0049/+0.0065 (all CI>0; campaign's largest per-bd, SEED 1783286386) | fresh | default-on ✓ |
| set_splinter_doubled | `--no-ns-splinter-doubled` | Artificial | ON | plain +0.0059/+0.0079, PD same (FirstIs(Double) systems-on rebase, SEED 1783439089) | fresh | default-on ✓ |
| set_delayed_cue | dedicated (no flag) | Artificial | OFF | gated for measurement, no headline | unmeasured | needs A/B |
| set_major_support_double | `--no-ns-major-support-double` | Artificial | ON | plain −0.0004/+0.0004 wash, PD +0.0009/+0.0016 (vul CI>0, SEED 1783285623) | fresh | default-on ✓ (plain-wash+PD-gain) |
| set_cachalot_contested_x | `--no-ns-cachalot-contested-x` | Artificial | ON | NV win all 3 scorers, vul wash ([project_school-tournament-responses]) — **no-op unless neg-double=Cachalot (opt-in)** | fresh | default-on ✓ (dormant) |
| set_cue_raise_answer | `--no-ns-cue-raise-answer` | Artificial | ON | fixes floor-passes-the-cuebid ([project_cue-raise-answer]) | unmeasured (qualitative) | default-on ✓ |
| set_cue_minor_raise_answer | `--no-ns-cue-minor-raise-answer` | Artificial | ON | minor twin, isolated over shipped major | unmeasured (qualitative) | default-on ✓ |
| set_weak_two_competition | `--ns-weak-two-comp` | Artificial | OFF | plain −0.0012/−0.0015 wash, PD −0.0097/−0.0116 (CI<0, SEED 1783284838); **REFUTED** by fallible-opponent test | fresh | stays opt-in (measured loss) |
| set_strong_two_competition | `--no-ns-strong-two-comp` | Natural | ON | plain +0.0009/+0.0013, PD +0.0010/+0.0014 (all 4 CI>0, SEED 1783285250) | fresh | fold into base |
| set_uvu | `--uvu` | Artificial | ON | +0.6–2.6 IMPs/bd per call vs passing floor (DD-robust) | fresh | default-on ✓ |
| set_uvu_over_majors | `--no-ns-uvu-over-majors` | Artificial | ON | plain +0.0019/+0.0018 (CI>0), PD +0.0009/+0.0006 (P1, SEED 1783284454, sha bc949dc) | fresh | default-on ✓ |
| set_competitive_rebid | `--no-ns-competitive-rebid` | Natural | ON | plain +0.047/+0.037, PD +0.040/+0.023 (all 4 CI>0; P5 largest per-bd, SEED 1783316036) | fresh | fold into base |
| set_reopening_notrump | `--no-ns-reopening-notrump` | Natural | ON | plain +0.0163/+0.0332 (de-confounded ×2, free-1nt-range package) | fresh | fold into base |
| set_rein_advance_raise | `--no-ns-rein-advance-raise` | Natural | ON | part of same package +0.0163/+0.0332 (stops floor over-raising into a doubled game) | fresh | fold into base |
| set_trap_pass | `lebensohl-ab --pd-3nt` (no flag) | Natural | ON | on vs off, 200k plain net +155/+230 IMPs (resp-3NT losers erased); 5-HCP gate PD-distilled | fresh | fold into base (floor-tail fix) |
| set_direct_3nt_stopper | dedicated (no flag) | Natural | ON | status-quo gate, no isolated headline | unmeasured | keep default (needs A/B to retire) |
| set_rubens_advances | `--no-ns-rubens` | Artificial | ON | baseline default; knob is the A/B off-arm ([project_rubens-advances-m63]) | fresh | default-on ✓ |
| set_competition_over_stayman | `--no-ns-comp-over-stayman` | Artificial | ON | Side A +3.5 IMPs/fired ([project_competition-over-stayman]) | fresh | default-on ✓ |
| set_competition_over_transfer | `--ns-comp-over-transfer` | Artificial | OFF | DD loss plain −0.94, PD −0.33 IMPs/bd-fired (640k) | fresh | stays opt-in (measured loss) |
| set_competition_over_minor_transfer | `--no-ns-comp-over-minor-transfer` | Artificial | ON | +4.80 plain / +5.63 PD IMPs/fired (CI>0, 640k; rare 0.03%) | fresh | default-on ✓ |
| set_competition_over_diamond_transfer | `--no-ns-comp-over-diamond-transfer` | Artificial | ON | plain wash +0.24/fired, PD +3.40 (1M boards; never loses plain) | fresh | default-on ✓ |

### A5 — Defending their 1NT & their overcalls

| Option (knob) | CLI | Nat/Art | Default | A/B verdict | Fresh | Policy → action |
| --- | --- | --- | --- | --- | --- | --- |
| set_notrump_defense (Natural/DirectDont/Meckwell/Woolsey/DirectLandy/AlwaysPass/Off) | `--ns-natural`/`--ns-dont`/`--ns-woolsey`/`--ns-landy` (mutually-exclusive shims) | Natural (base) | Natural | GTO matrix mixed equilibrium: NV plain eq Woolsey +0.070, NV PD eq natural +0.029, vul-both eq always-pass, sd-lead eq Woolsey both vuls ([project_gto-1nt-defense]) | fresh | keep default Natural; artificial systems stay opt-in shims |
| ↳ set_natural_defense | `--ns-natural` | Natural | ON (active) | plain +0.744/div NV, +1.276/div vul vs floor (landy-ab) | stale-pop | fold into base |
| ↳ set_woolsey | `--ns-woolsey` | Artificial | OFF | sd-lead equilibrium both vuls (+0.132/+0.071); NV plain eq. Overcall floor probed to 8, X to 12 | fresh | opt-in (sd-only edge; own Woolsey) |
| ↳ set_direct_dont | `--ns-dont` | Artificial | OFF | first artificial 1NT-defense to match natural (−0.196 NV tie / +0.072 win); kept opt-in per jdh8 | stale-pop | opt-in (= floor) |
| ↳ set_meckwell | `ab-nt-defense-matrix` | Artificial | OFF | decisive LOSS plain −0.277, PD −0.522, 0% Nash all brackets ([project_meckwell-defense]) | fresh | stays opt-in (measured loss) |
| ↳ set_landy | `--ns-landy` | Artificial | OFF | DD-lost vs natural (landy-ab) | stale-pop | stays opt-in (measured loss) |
| ↳ set_always_pass_defense | `--ew-always-pass` | (datum) | OFF | the A/B baseline do-nothing defense; not a shipping system | n/a | keep off (measurement datum) |
| set_advance_sohl_style (Off/Plain/Transfer) | `?` (21gf-ledger) | Artificial | Transfer | Transfer clear PD win over flat ladder +0.145/+0.227 IMPs/bd (200k filtered) | fresh | default-on ✓ (Transfer) |
| set_leaping_michaels | `?` (21gf-ledger) | Artificial | ON | +1.090/+1.452 IMPs/bd; inference reader prices slam | fresh | default-on ✓ |
| set_notrump_balancing | `bba-match --ns-balancing` | Artificial | OFF | undisciplined balancing doubles — off | unmeasured | needs A/B |
| set_stayman_defense | `?` | Artificial | OFF | lead-directing (DD-invisible), PD wash | stale-pop | opt-in (DD-blind) |
| set_transfer_defense | `?` (xfer-h/xfer-s) | Artificial | OFF | PD wash (+0.006/fired CI⊇0, 640k); plain loss = light-sac artifact | fresh | opt-in (lead-directing) |
| set_minor_transfer_defense | `?` | Artificial | OFF | lead-directing, rare | unmeasured | opt-in |
| set_diamond_transfer_defense | `?` | Artificial | OFF | clear loss over 1M `--filter-1nt` boards (387 fired) | fresh | stays opt-in (measured loss) |
| set_natural_double_shape (Balanced/SemiBalanced/Any) | `--ns-double-shape …` | Natural | Balanced | Balanced dominates: self-play −0.70/div (−0.92 PD, 16.9k div) vs Any; flipped back 2026-06-26 | stale-pop | keep Balanced (Nat discipline) |
| set_takeout_support (Off/Lenient/Strict) | `--ns-takeout-support …` | Natural | Strict | shipped 3+ card support gate; default-on (21gf-ledger) | fresh | fold into base |
| set_overcall_discipline | `--ns-overcall-discipline …` | Natural | ON | shipped: 1-lvl cap 17, 2-lvl opening-11+ | fresh | fold into base |
| set_passed_hand_overcall | `--ns-passed-hand-overcall` | Natural | OFF | unproven lighter passed-seat overcalls | unmeasured | needs A/B |
| set_two_level_minor_overcall_tight | `--ns-two-level-minor-overcall-tight` | Natural | OFF | tightening LOSES all bands, sd-lead confirms loss real | fresh | stays opt-in (measured loss) |
| set_nt_overcall_no_major | `--ns-nt-overcall-no-major` | Natural | OFF | anchor shows 5cM buried under 1NT overcall | unmeasured | needs A/B |
| set_nt_overcall_systems_on | `--no-ns-nt-overcall-systems-on` | Artificial | ON | shipped default-on: graft opening-1NT responses below [1t,1NT]; sd WIN 4/4 | fresh | default-on ✓ |
| set_nt_overcall_gladiator | `--ns-nt-overcall-gladiator` | Artificial | OFF | completed book WASH vs graft ([project_gladiator-major-overcall]) | fresh | opt-in (= floor) |
| set_responsive_takeout | `responsive-ab --conv takeout` | Artificial | ON | canonical responsive double, shipped default | fresh | default-on ✓ |
| set_responsive_overcall | `responsive-ab --conv overcall` | Artificial | OFF | non-standard extension (BBA's is takeout-only) | unmeasured | needs A/B |
| set_rich_advance_double | `--no-ns-rich-advance` | Artificial | ON | shipped 2026-07-11: byte-identical book, SIG+ after 0.10.0 double-discipline shift (was −0.0011 wash) | fresh | default-on ✓ |
| set_advance_rubens | `--ns-advance-rubens` | Artificial | OFF | DD+sd wash (no effect unless rich on) | fresh | opt-in (= floor) |
| set_longest_first_advance | `--no-ns-longest-advance` | Artificial | ON | shipped w/ rich book: rich+longest SIG+ all 4 scorers; WASH standalone on flat book | fresh | default-on ✓ (paired w/ rich) |
| set_advance_minor_jump | `--no-ns-advance-minor-jump` | Artificial | ON | shipped: bare LOST; both doubler continuations → 2-seed SIG+ all 4 cells, plain ≥ PD | fresh | default-on ✓ (both sides) |
| set_advance_2nt_continuation | `--no-ns-advance-2nt-continuation` | Artificial | ON | shipped: doubler accept/decline of 2NT invite, wash-positive all 4 cells | fresh | default-on ✓ |

### A6 — Floor & inference engine toggles

Not a book of calls — these read the auction, settle the floor, or evaluate a
hand. Judged by measured sign; kept default-on when they help.

| Option (knob) | CLI | Kind | Default | A/B verdict | Fresh | Policy → action |
| --- | --- | --- | --- | --- | --- | --- |
| set_inference_aware | `inference-floor` example | Engine | ON | floor consults auction interpretation vs shape-blind fallback; foundational | unmeasured (net) | keep default-on |
| set_settle_floor | `ab-settle-floor` | Engine | ON | "pass = play top bid": +0.26/+0.37 PD (+0.18/+0.29 plain); Stage-2 TTL reverted | stale-PD | keep default-on |
| set_alert_reading | `ab-alert-reading` | Engine | ON | reads alerted call as artificial: +2.08/+1.59 IMPs/bd | stale-PD | keep default-on |
| set_fallback_projection | BBA A/B | Engine | ON | decodes contested/fallback conventions: plain +0.0006 (+1.03/fired), PD +0.0014 (+2.38/fired), CI>0 | fresh | keep default-on |
| set_control_bid_reading | A/B off arm | Engine | ON | reads high new-suit as control vs to-play (M6.4, BWS-distilled) | fresh | keep default-on |
| set_nt_invite_inference | `nt-invite-abc` | Engine | ON | reads responder invitational (~8-9) vs GF (10+) of our 1NT | unmeasured (net) | keep default-on |
| set_rubens_transfer_reading | `--no-ns-rubens-reading` | Engine | ON | records 1-level Rubens transfer meanings; unblinds overcaller + sampler | unmeasured (net) | keep default-on |
| set_rule_accept | `ab-landy` | Engine | OFF | sampler accepts hand by replaying authoring rule: +0.24/bd on 1NT-defense | fresh | opt-in engine; wider-A/B candidate |
| set_fuzzy_strength | doc-hidden, A/B-only | Engine (eval) | ON | umbrella for points+fifths vs raw HCP; NULL at 1NT-invite boundary (raw HCP best) | unmeasured/NULL | keep default-on |
| set_fuzzy_points | doc-hidden | Engine (eval) | ON | points() upgrade alone | unmeasured | keep default-on |
| set_fuzzy_fifths | doc-hidden | Engine (eval) | ON | fifths() upgrade alone; no help at 1NT invite ([reference_fifths_evaluator]) | unmeasured/NULL | keep default-on |
| set_fifths_companion (Hcp/Bumrap) | doc-hidden | Engine (eval) | Bumrap | honor count averaged into fifths; no isolated net A/B | unmeasured | keep default |

### A7 — Slam & keycard

| Option (knob) | CLI | Nat/Art | Default | A/B verdict | Fresh | Policy → action |
| --- | --- | --- | --- | --- | --- | --- |
| set_floor_rkcb | `--no-ns-floor-rkcb` | Engine/Artificial | ON | 5-round wash ([project_m64-floor-slam]); reaches slams the direct-milestone floor misses | stale-pop | keep default-on (capability-add) |
| minor keycard (no knob) | baked in (commit 99da1b3) | Artificial | ON | vs floor +6.80/+8.76 div; re-measured PD +5.41/+7.05 div [10M, 202 div] HOLDS ([project_minor-keycard]) | stale-PD | already folded in; no knob to retire |

### A-suppress — takeout-double discipline (natural)

Gate out an over-eager takeout double in favour of a natural bid or pass.

| Option (knob) | CLI | Nat/Art | Default | A/B verdict | Fresh | Policy → action |
| --- | --- | --- | --- | --- | --- | --- |
| set_suppress_flat_4333_takeout | `?` | Natural | ON | 12-14 flat 4333 → Pass not X: plain +0.0187/+0.0385, PD +0.0566/+0.0755 (~1.2% fired, CI>0) | fresh | fold into base |
| set_suppress_5332_takeout | `?` | Natural | ON | weak 5332 → bid 5-card suit not X: plain +0.0191/+0.0401, PD +0.0601/+0.0773 | fresh | fold into base |
| set_suppress_5card_major_takeout | `?` | Natural | ON | unbid 5+cM → overcall not X: plain +0.0190/+0.0493, PD +0.0892/+0.1129, sd +0.0124/+0.0413 | fresh | fold into base |
| set_suppress_4432_vs_major | `?` | Natural | OFF | anchor split −3.2…−3.8 IMPs/div over major openings; pending opener-suit A/B | unmeasured | needs A/B |
| set_suppress_4432_vs_minor | `?` | Natural | OFF | mildest slice −1.39 IMPs/div; textbook takeout of a minor | unmeasured | needs A/B; likely keep double (off) |

---

## Tier B — numeric tuning parameters

These live *inside* a book and set a threshold, band, or weight — no
natural/artificial verdict. Documented as current value + what it tunes. Sweep
provenance in [convention-tuning.md](convention-tuning.md) and the cited slugs.

| Knob | Value | Tunes | Sweep / source |
| --- | --- | --- | --- |
| set_nt_responder_game_floor | 9 | pts for NT responder to drive to game | [project_nt-9count-gameforce-seam] (10→9 undisturbed) |
| set_runout_xx_min | 7 | pts to redouble (business) vs run from 1NT-X | [project_one-nt-doubled-runout] |
| set_texas_game_floor | 14 | pts floor for Texas transfers | [project_nt-invite-evaluator-sweep] |
| set_sixcard_invite_floor | 13 | pts to invite with a 6-card major | 1NT invite ladder |
| set_sixcard_accept_floor | 18 | pts for opener to accept the 6-card invite | 1NT invite ladder |
| set_free_bid_floor | 6 | min pts for a natural free bid | [project_free-1nt-range] (floor 6 confirmed) |
| set_free_1nt_floor | 6 | min pts for a free 1NT | [project_free-1nt-range] (band 6–10, DE-CONFOUNDED ×2; `set_free_1nt_floor` opt-in) |
| set_natural_floor | (5, 0) | (hcp, pts) floor for natural competitive bids | [project_fallback-competitive-classify] |
| set_uvu_x_floor | 9 | pts floor for the UvU double | UvU tuning |
| set_uvu_cue_floor | 8 | pts floor for the UvU cue | UvU tuning |
| set_uvu_natural_floor | 6 | pts floor for the UvU natural bid | UvU tuning |
| set_natural_double_floor | 15 | HCP floor for the penalty X of their 1NT | [project_natural-1nt-defense] |
| set_natural_double_weight | 1.3 | logit weight of penalty X vs suit overcalls | [project_natural-1nt-defense] |
| set_natural_overcall_points | (8, 14) | band for natural 2-level overcall of their 1NT | [project_natural-1nt-defense] |
| set_woolsey_points | (8, 19) | Woolsey/Landy shared suit-overcall band | [project_our-woolsey-defense] (floor probed 10→8) |
| set_woolsey_double_floor | 12 | pts floor for the Woolsey takeout X | [project_our-woolsey-defense] |
| set_unusual_notrump_defense | (8, 13) | 2NT both-minors overlay band | defense.rs default |
| set_doubled_landy_escape | (6, 2) | (min minor len, max in each major) after Landy 2♣ doubled | Landy-only |
| set_stayman_defense_overcall | (6, 14) | (min len, pts floor) for the Stayman-defense overcall | defense.rs default |
| set_direct_landy_double_floor | 15 | pts floor for the direct-Landy double | DirectLandy tuning |
| set_direct_dont_one_suiter_min | 5 | min length for a DONT one-suiter | DONT tuning |
| set_direct_dont_x_floor | 0 | pts floor for the DONT double | DONT tuning |
| set_meckwell_x_floor | 0 | pts floor for the Meckwell two-way X (0 = inherit natural 8) | [project_meckwell-defense] (X≥15 sweep near-wash, 0% support) |
| set_gambling_3nt_top_honors | 2 | top honors in the long minor for gambling 3NT | [project_gambling-games-over-1ntx] |
| set_gambling_3nt_require_ace | true | require an ace for gambling 3NT | same |
| set_preempt_4m_floor | 5 | pts floor for the 4m preempt over their X | same |
| set_preempt_4m_top_honors | 2 | top honors for the 4m preempt | same |
| set_preempt_4m_require_ace | true | require an ace for the 4m preempt | same |
| set_double_override | None | override the takeout-double gate (len, len, hcp) | A/B instrument |
| set_penalty_pass | (4, 4, true) | penalty-pass gate (len, hcp, flag) | competition.rs default |
| set_landy_hcp | false | gate Landy on HCP vs points | Landy probe |
| set_direct_landy_double | None | DirectLandy both-majors X accepts flat 4-4 (else 5-4+) | probe (no effect unless landy-X on) |
| set_direct_landy_penalty_pass | false | penalty-pass option inside DirectLandy | probe (parent opt-in) |
| set_meckwell_minor_major_44 | false | Meckwell 2♣/2♦ accept flat 4-4 (else 5-4+) | probe (parent is a loss) |
| set_meckwell_x_four_four | true | Meckwell both-majors X accepts flat 4-4 | probe (parent is a loss) |
| set_direct_dont_four_four | true | DONT two-suiters accept 4-4 | probe (parent opt-in) |

---

## Worklist — unmeasured / stale (needs A/B)

The options whose policy verdict can't be read off a fresh isolated A/B. Work
these buckets per [measurement.md](measurement.md).

**Never isolated (`unmeasured`):** set_notrump_minors (Puppet vs European),
set_one_notrump_fifths, set_meckstroth_adjunct, set_delayed_cue,
set_defense_to_2d_multi, set_notrump_balancing, set_passed_hand_overcall,
set_nt_overcall_no_major, set_responsive_overcall, set_minor_transfer_defense,
set_suppress_4432_vs_major, set_suppress_4432_vs_minor, set_direct_3nt_stopper,
and the qualitative-only cue/runout knobs (set_cue_raise_answer,
set_cue_minor_raise_answer, set_one_nt_runout*, set_penalize_escape_*).

**Stale figures (re-measure before trusting the magnitude):**
- `stale-PD` (pre-`a6f2206` PD-era, not comparable to plain-DD): NotrumpShape
  (+0.57/+0.93), set_alert_reading (+2.08/+1.59), set_settle_floor (+0.26/+0.37),
  set_transfer_super_accept, minor keycard (+6.80/+8.76; PD +5.41/+7.05 is the
  conservative re-measure).
- `stale-pop` (measured before a book-population shift): set_open_one_notrump,
  set_floor_rkcb, set_natural_defense, set_direct_dont, set_landy,
  set_natural_double_shape, set_stayman_defense.
