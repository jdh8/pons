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
  ([src/bidding/inference.rs:1494](../src/bidding/inference.rs#L1494)) — a call is
  artificial iff it points partner at a suit it did not name: a **bid** floors an
  *unnamed* suit; a **double/redouble** floors an *unbid* suit (takeout — "pick a
  suit") rather than the *doubled* strain (penalty/business → natural, "play what's
  on the table"); a **pass** and a **transfer completion** never do. Enforced by the
  invariant test `artificial_calls_are_alerted`
  ([inference.rs:3853](../src/bidding/inference.rs#L3853)). The structural witness is
  *sufficient, not complete*: a takeout double whose authoring rule floors nothing
  (opaque shape predicates, e.g. the direct takeout double) is artificial by meaning
  and classified by its `.alert(...)` instead.
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
**re-measurable**. So "fold into base" here is the *system* verdict, and
"retiring" a knob means removing it as a **user-facing** choice, not deleting it:

- Drop its row from the `web` settings registry ([web/src/lib.rs](../web/src/lib.rs),
  the `SETTINGS` table) — that is the only user-facing surface, and it has no
  automatic sync to the engine setters, so this is the edit that matters.
- Keep the `pub set_*` fn and its `--no-ns-*` CLI wiring for measurement. A
  "retired" knob only faces developers, so it stays in rustdoc (no
  `#[doc(hidden)]`) — the off-switch should be documented for whoever re-measures.

Retired this way so far: `balanced_1nt_rebid` and the fresh
natural-≥-floor batch (`major_game_tries`, `longer_major_response`,
`major_rebid_tails`, `competitive_rebid`, `suppress_nt_game_force_over_double`,
`correct_3nt_to_major`, `overcall_discipline`, `trap_pass`,
`penalty_double_leave_in`, `strong_two_competition`, and the three
`suppress_*_takeout` knobs). `xyz` and `up_the_line` are retired **together**:
XYZ is the de-facto modern checkback (it displaced New Minor Forcing), so the
artificial convention folds into base on naturalness, and up_the_line — whose
trigger fires independent of `xyz()` and loses standalone — is only safe to
force on when XYZ is forced on with it, which retiring both as a pair
guarantees.

**Deleted outright, not retired: `rule_of_20`.** The one case where the
off-switch is not worth keeping is a knob whose two arms became *the same
predicate*. Since `points` shipped on the rule-of-N+8 scale, `points(12..)` is
the Rule of 20 wherever the two longest suits reach eight cards, so the light
`hcp(10..=11) & rule_of_20()` openings were a strict subset of the
`points(12..=21)` openings above them at identical weights, and the knob's off
arm no longer bit. Deleted 2026-07-21 with the `rule_of_20()` constraint, the
CLI flags, and `scripts/rule-of-20-ab.sh`; the identity is pinned by
`points_twelve_is_the_rule_of_20` and `sound_eleven_counts_open_one_of_a_suit`.
Its last verdict, for the record: plain +0.0061/+0.0087, PD −0.0056/−0.0034
(doubler artifact), sd-lead +0.0096/+0.0135 (SEED 1783410574).

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
| NotrumpShape (1NT open) | `set_notrump_shape` (web radio, Settings → Openings) | Natural | **Wide6322** (Balanced/Wide/Wide6322) | **SHIPPED Wide6322 as default 2026-07-12**, contested vs BBA (204.8k/cell ×2 vuls, SHA c6a5643, `nt-shape-ab.sh` + `nt-shape-confirm-ab.sh`). (1) Wide vs Classic (5422-minor widening) plain **+0.0087/+0.0121**, PD +0.0060/+0.0092, **sd +0.0122/+0.0171** NV/vul — all 6 cells positive, sd>plain>PD. (2) Wide6322 vs Wide, **two seeds** (1783843252 + 1783844868): plain +0.0034…0.0048/+0.0048…0.0050, PD +0.0025…0.0033/+0.0035…0.0039, sd +0.0052…0.0054/+0.0063…0.0078 — all 6 cells positive both seeds, plain wins on its own. Soundness block cleared: 1NT inference now reads opener's minors 2–6 (majors 2–5), proptest passes | shipped | **folded into base**; the `american_wide()`/`american_classic()` baselines and the `--nt-shape` flag were retired once the decision folded in — `set_notrump_shape` still reaches every shape |
| set_one_notrump_fifths | `--nt-fifths` | Natural | OFF | **archived** — already measured *losing* at the open boundary: plain HCP beat the centre-corrected fifths gauge **+0.067/+0.094** NV/vul (de-confounded ×2, 2026-06-23; HCP is the shipped default — [project_1nt-aggressive-redesign]), and Fifths is also refuted at the invite/responder boundary ([project_nt-invite-evaluator-sweep]). A refuted evaluator stays archived, not re-litigated per boundary | archived | keep opt-in off; no A/B |
| set_longer_major_response | `--no-ns-longer-major-response` | Natural | ON | plain-DD wash, PD −0.12/−0.22 per div; kept by naturalness tiebreak (commit 2ba6b90) | fresh | fold into base |
| set_up_the_line | `--no-ns-up-the-line` | Natural | ON | **coupled with XYZ**: joint plain +0.0382/+0.0559, PD +0.0289/+0.0407; alone a loss −0.91/−1.28 per div | fresh | folded into base *with* XYZ (web toggle retired as a pair) |
| set_major_game_tries | `--no-ns-major-game-tries` | Natural | ON | plain +0.042/+0.065 (both scorers win); package w/ FSF+limit-accept +0.058/+0.089 ([project_major-continuations]) | fresh | fold into base |
| set_limit_raise_acceptance | `--no-ns-limit-raise-acceptance` | Artificial | ON | plain +0.002/+0.002; load-bearing part is the 4NT keycard ask +4.4/+5.2 IMPs/div | fresh | default-on ✓ |
| set_major_choice_of_games | `--no-ns-major-choice-of-games`; `ab-major-continuations --choice-of-games` | Artificial | ON | 1M-3NT = 3-4 trumps, (4333), 12-15 HCP; opener passes balanced / corrects 4M with shape. Isolated plain +0.0006/+0.0011 NV/vul, PD +0.0005/+0.0010, all CI-clear (1M bd/vul, seed 1784056362); exactly additive atop the 2/1 fit-split | fresh | default-on ✓ (both scorers win) |
| set_two_over_one_fit | `--no-ns-two-over-one-fit`; `ab-major-continuations --two-over-one-fit` | Artificial | ON | 2/1 fit leg: exactly-3-card support enters on `support_points(13..)` (fit known — opener promised five). Alone NV wash / vul plain +0.0010; **complementary with Hcp13**: the pair plain +0.0033/+0.0048, PD +0.0070/+0.0087 NV/vul, all CI-clear (1M bd/vul, seed 1787056851) | fresh | default-on ✓ jointly with Hcp13 |
| set_two_over_one_gate | `--ns-two-over-one-gate hcp13\|hcp12\|points13`; `ab-major-continuations --two-over-one-gate` + `--baseline-gate` | Natural | **Hcp13** | no-fit 2/1 gauge — the remnant report's shape-indifferent prescription (shaped 11-12s back to 1NT). hcp13 plain +0.0019/+0.0018 PD +0.0065/+0.0069 vs legacy; h2h hcp12-vs-hcp13: NV PD −0.0034, vul plain +0.0026 but PD −0.0020 (thin-game doubling signature) → 13 | fresh | default Hcp13 ✓; hcp12-vul = sd-probe candidate; Points13 = legacy opt-out |
| set_meckstroth_adjunct | `ab-meckstroth-2nt` (no bba-gen flag) | Artificial | ON | The **complete Meckstroth adjunct**, one knob: (1) opener's `1M–1NT–2NT` = artificial 18+ GF (any shape) + `3♣`-relay shape-outs, both sides + RKCB — plain **+0.0075/+0.013**, PD +0.006/+0.011, **sd-lead +0.010/+0.017** NV/vul (200k×2 seeds, all CI-clean; +2.7/+4.4 IMPs/div; fires 0.4%); and (2) opener's invitational `3m` jumps (5+ minor, 15–17) — plain wash, PD −0.0036/−0.0019 (over-punished), **sd-lead +0.0012/+0.0042** NV/vul (2 seeds ×200k, SHA 22364c9), sd-vindicated. Repro: `ab-meckstroth-2nt --sd`. *(Was two knobs `set_meckstroth_adjunct` (3m jumps) + `set_meckstroth_2nt` (the 2NT machine); merged 2026-07-12, the shipped book byte-identical.)* | fresh | keep default-on ✓ |
| set_forcing_nt_two_suiter | `ab-forcing-nt-two-suiter` (no bba-gen flag) | Artificial | ON | Opener's invitational (15–17) major two-suiter over the forcing 1NT: `1♥–1NT–2♠` reverse (5+♥ 4+♠) and `1♠–1NT–3♥` jump (5-5 majors), both sides authored. plain wash-NV/**+0.001-vul** (never negative), PD −0.0017/−0.0010 (over-punished), **sd-lead +0.0012/+0.0013 NV, +0.0026/+0.0029 vul** (1M×2 seeds×2 vuls, SHA 293ed53; all four sd cells CI-clean; +0.8/+2.1 IMPs/div; fires 0.14%). sd redeems the PD loss — the `set_meckstroth_adjunct` profile. Repro: `ab-forcing-nt-two-suiter --sd` | fresh | keep default-on ✓ |
| set_balanced_1nt_rebid | `--no-ns-balanced-1nt-rebid` | Natural | ON | plain +0.0076/+0.0085 NV, +0.0109/+0.0117 vul ([project_balanced-1nt-rebid]) | fresh | fold into base |
| set_opener_extras_ladder | `--no-ns-opener-extras-ladder` | Natural | ON | plain +0.0203/+0.0332, PD +0.0181/+0.0297 (SEED 1783544590, `opener-extras-ladder-ab.sh`) | fresh | fold into base — but reverse/jump-shift rungs carry a toggle-gated reading |
| set_opener_major_jump_rebid | `--no-ns-opener-major-jump-rebid` | Natural | ON | plain +0.0059/+0.0125, PD +0.0046/+0.0104 (SEED 1783549337). Bare rung LOST; win needed responder's continuation (author both sides) | fresh | fold into base |
| set_major_rebid_tails | `--no-ns-major-rebid-tails` | Natural | ON | plain +0.016/+0.023 NV/vul | fresh | fold into base |
| set_fourth_suit_forcing | `--no-ns-fourth-suit-forcing` | Artificial | ON | plain +0.002/bd both scorers/vuls, atop the tails; part of the +0.058/+0.089 major-continuations package. Rides `set_major_rebid_tails` | fresh | default-on ✓ |
| set_second_suit_agreement | `--no-ns-second-suit-agreement` | Artificial | ON | plain +0.0012/PD +0.0014 NV, +0.0015/+0.0018 vul (marginal; payoff in the RKCB-on-extras tail). **Re-audit 2026-07-20 (candidate #1): vindicated.** Deleting the node measures **−2.777/−3.351 plain, −2.749/−3.328 PD** IMPs/div NV/vul (self-play 2,000,000×2, seed 1784484826, 704 div = 0.04%) — the node clearly beats the floor, keep it | fresh | default-on ✓ |
| set_xyz_invite_judgment | `--no-ns-xyz-invite-judgment` | Natural | **ON** | opener judges the XYZ invitations that stop below game (`points(14..)` bids it, else Pass). **Re-audit 2026-07-20 (candidate #3): vindicated, and the most-reached node in the sweep** (0.75% divergent, 15× #2). Deleting it costs **−0.0086/−0.0175 plain, −0.0035/−0.0106 PD** IMPs/board NV/vul (self-play 2,000,000×2, seed 1784484826) — −1.145/−2.333 and −0.466/−1.406 per divergent. Two crude rules on a raw point count that comfortably beat the floor: the crude-node signature is a search hint, not a verdict | fresh | keep default-on ✓ |
| set_xyz | `--no-ns-xyz` | Artificial | ON | joint w/ up-the-line +0.0382/+0.0559; XYZ alone +0.504/+0.795 per div plain, +0.332/+0.472 PD | fresh | folded into base (de-facto modern checkback; web toggle retired) |
| set_game_backstop | `--ns-game-backstop` (restores it) | Natural | **OFF** | the retired 2/1 game backstop (three rules: 4♥/4♠/3NT over every uncovered GF continuation). Authored against an earlier, weaker `instinct()`, which floors the *constructive* book (the BBA-distilled net floors only the contested books). **Deleted 2026-07-20**, paired with `set_two_over_one_force`: **plain +0.0117/+0.0142, PD +0.0132/+0.0160** NV/vul (409,600×2, all CI>0, fires 0.57%, +2.1-2.8 IMPs/div). Deletion *alone* is only +0.005 — the floor then abandons partner's 2/1 on 24% of divergences. Also cures a `sample_layouts_replay` 0% fill (partial table ⟹ unnamed calls at −∞ with the node still holding mass) | fresh | **improved: default→off** (knob kept for re-measure) |
| set_opener_third | `--no-ns-opener-third` | Natural | ON | opener's third call after trump is agreed at `1M–2r–R–3M`: 4NT RKCB on `points(15..)`, else an unconditional `4M`, every cue-bid and five-level call at −∞. **Re-audit candidate #2 — CLOSED 2026-07-20, node stands.** Its apparent +0.437/+0.527 IMPs/div in self-play was the deletion routing around a *starved* slam-entry gate, not around a bad call: the alerted 2/1 read as zero points, so the floor could not explore. With `set_two_over_one_slam_strength` supplying the missing reading, deleting the node on top is worth **+0.0003/+0.0004 plain, +0.0003/+0.0005 PD** NV/vul with the CI straddling zero (409,600×2 vs BBA) — i.e. nothing. Fix the reading, keep the node | fresh | keep default-on ✓ (deletion priced at zero once the floor can see) |
| set_two_over_one_force | `--no-ns-two-over-one-force` | Natural | **ON** | tells the *floor* an uncontested 2/1 forces game, so it takes the cheapest game milestone instead of passing out a partscore — the invariant `game_force`'s tables held by omission and the floor never learned. On top of the deletion: **plain +0.0067/+0.0102, PD +0.0060/+0.0094** NV/vul (all CI>0), firing on exactly the 606/622 boards that abandoned the force, at +4.5/+6.7 IMPs each. Costs routing those nodes through the deterministic ladder (the shell delegates wholesale on a forced auction); priced in | fresh | keep default-on ✓ (WIN/WIN) |
| set_two_over_one_slam_strength | `--no-ns-two-over-one-slam-strength` | Natural | **ON** | floors partner's shown strength at the 13 points a two-over-one promised, on the floor's slam-entry gate. The 2/1 response is alerted (`GAME_FORCE`), so the inference walk skips its natural reading and defers to the rule's projection — and `points(13..)` on the rule-of-N+8 scale soundly projects to *no* high-card floor (a 13-pointer can be an eight-count with a six-card suit). Partner therefore read as **zero** through an established game force and the 29-point slam entry could never fire: opener with a 26-count counted 26+0 and signed off in game. **plain +0.0032/+0.0042, PD +0.0031/+0.0041** NV/vul (409,600×2 vs BBA, all CI>0, fires 0.08%/0.09% at +3.8/+4.8 IMPs each). Applies only when *partner* made the 2/1 | fresh | keep default-on ✓ (WIN/WIN) |

### A2 — Our 1NT: Stayman, transfers (artificial constructive)

| Option (knob) | CLI | Nat/Art | Default | A/B verdict | Fresh | Policy → action |
| --- | --- | --- | --- | --- | --- | --- |
| set_notrump_minors (PUPPET/EUROPEAN) | `ab-notrump-minors --sd` (web `puppet_stayman` toggle) | Artificial | PUPPET | **Puppet ≥ European, isolated** (the scheme-as-a-whole was +0.76/+1.15 vs a natural baseline, PD-era [project_minor-transfers-puppet]): plain +0.18…+0.44 IMPs/divergent — 4 cells {NV,vul}×2 seeds, **all positive** (+213…+502 IMPs / ~1.2k div each), PD positive throughout, sd-lead +0.0002…+0.0006/bd (weakly positive, CI straddles 0 at vul); fires 0.3%; 400k×4, SHA 82840a5 | fresh | default-on ✓ (Puppet the default; European stays opt-in — Puppet never loses) |
| set_transfer_super_accept | `--ns-transfer-super-accept` | Artificial | OFF | DD wash leaning neg, −0.055 IMPs/fired (640k) | stale-PD | opt-in |
| set_transfer_slam_try | `--no-ns-transfer-slam-try` | Artificial | ON | shipped on plain +0.0012 ÷ PD +0.0012 (+1.42/fired, 320k); a7-run 2026-07-16: **0 fired in 320k×2 — INERT by design**: `transfer_slam_try_rebid` yields its slot to the default-on GF-majors structure (notrump.rs), which relocated the single-suiters to a quantitative 4NT.  Live only with `--no-ns-transfer-gf-majors`/`-hearts` | fresh | default-on ✓ (inert while the GF structure is on; keep as that structure's fallback, like nt_invite under Puppet) |
| set_texas_slam_drive | `--no-ns-texas-slam-drive` | Artificial | ON | a7-run: plain **+5.04/+5.85 per fired** ÷ PD +5.17/+6.03 (320k×2, fires 0.02%, CI>0), sd-lead +2.67/+2.68, sd-declarer +2.89/+3.69 — positive in all four brackets | fresh | default-on ✓ |
| set_transfer_gf_majors | `--no-ns-transfer-gf-majors` | Artificial | ON | plain +0.0014 ÷ PD +0.0016 (+1.70/+1.90 fired) | fresh | default-on ✓ |
| set_minor_min_to_3nt | `--ns-minor-min-to-3nt` | Artificial | OFF | losing arm B of the gf-majors A/B; show-the-minor default beat it (CHANGELOG) | fresh | opt-in / improve (losing arm) |
| set_transfer_gf_hearts | `--no-ns-transfer-gf-hearts` | Artificial | ON | plain +0.0015/+0.0017 ÷ PD +0.0016/+0.0018 (two seeds) | fresh | default-on ✓ |
| set_garbage_stayman | `--no-ns-garbage-stayman` | Artificial | ON | plain +0.51/fired ÷ PD +0.70/fired (fires 0.17%) | fresh | default-on ✓ |
| set_stayman_both_majors | `--no-ns-stayman-both-majors` | Artificial | ON | plain +2.18/fired ÷ PD +2.29 (right-siding relay, 320k) | fresh | default-on ✓ |
| set_stayman_5card_max | `--no-ns-stayman-5card-max` | Natural | ON | plain +3.45/fired ÷ PD +3.33 (3♥/3♠ jump is natural/unalerted; a winning capability-add) | fresh | default-on ✓ (keep knob: **matrix exception** — off *removes a distinct call* (the 3♥/3♠ jump), a system variant not a style tweak, so it stays user-facing where the folded natural batch re-routed among existing calls) |
| set_invitational_5card_majors | `--no-ns-invitational-5card-majors` | Artificial | ON | plain +0.375/fired ÷ PD +0.134/fired (1.28M, CI>0). Needed doubled-2♦ systems-on companion | fresh | default-on ✓ |
| set_crawling_stayman | `--no-ns-crawling-stayman` | Artificial | ON | plain +1.539/fired ÷ PD +2.055/fired (1.6M, PD>plain, not a doubling artifact) | fresh | default-on ✓ |
| set_transfer_longer_major | `--no-ns-transfer-longer` | Artificial | ON | DD wash (204.8k, 47 fired); shipped on structural grounds (deterministic route, partner infers longest) | fresh | default-on ✓ (structural, not a measured win) |
| set_stayman_cue_continuation | `--no-ns-stayman-cue-continuation` | Artificial | ON | plain +0.0193 ÷ PD +0.0216 ([project_stayman-slam-deficit] FIX1); closes cue-passed-below-game dead end | fresh | default-on ✓ |
| set_stayman_minor_slam_try | `ab-stayman --treatment` (no flag) | Artificial | ON | plain +3.29/+4.02 IMPs/fired ÷ PD identical (1.5M, 151 fired, zero losses) | fresh | default-on ✓ |
| set_long_minor_force | `ab-long-minor-force` (no flag) | Artificial | OFF | **measured LOSS** plain −7.12 IMPs/fired vs real transfer routing (8M, plain-DD only) | fresh (plain-only) | improve/drop — kept only as A/B instrument |

### A3 — Our 1NT: competition, runouts & escapes

| Option (knob) | CLI | Nat/Art | Default | A/B verdict | Fresh | Policy → action |
| --- | --- | --- | --- | --- | --- | --- |
| set_one_nt_runout | `ab-one-nt-runout --compare runout --filter-1nt` | Artificial | ON | **isolated vs the passing floor** (1M/cell ×2 seeds ×2 vuls, SHA 03d981f, `scripts/a3-run.sh`): plain **+0.039/+0.053**, PD **+0.023/+0.031** NV/vul — all 8 cells positive both seeds, PD holds (not a doubling artifact); fires 1.58%, +2.5/+3.4 IMPs/div plain (weak responder escapes the doubled 1NT) ([project_one-nt-doubled-runout]) | fresh | default-on ✓ |
| set_one_nt_runout_universal | `ab-one-nt-runout --compare runout [--no-universal]` | Artificial | ON | **marginal = full runout − direct-only** (`--no-universal` off-arm, same 1M×2×2): plain **+0.009/+0.011** but PD **−0.004/−0.005** NV/vul — obstruction split (opener escape / balancing SOS-XX gains on plain, PD over-doubles it), tiny magnitude; sd-lead would arbitrate. Kept default-on as a sub-feature of the winning runout | fresh | default-on ✓ (obstruction split; sd-lead candidate) |
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
| set_penalize_escape_stack | `ab-one-nt-runout --compare escape-stack --filter-1nt` | Artificial | ON | **isolated** (5M×2 vuls, SHA 03d981f): fires **0.01%** (498/5M); plain **+5.5/+7.3 IMPs/div** (penalty-X cashes) but only **+0.001/board**, PD ≈0/slightly-neg → negligible net. Penalty double of their escape on a trump stack; kept default-on qualitatively | fresh | default-on ✓ (rare, negligible net) |
| set_penalize_escape_values | `ab-one-nt-runout --compare escape-values --filter-1nt` | Artificial | ON | **isolated** (5M×2 vuls, SHA 03d981f): fires **0.03%** (1466/5M); plain **+4.2/+5.7 IMPs/div** (+0.001–0.002/board), PD **−0.36/−0.42 IMPs/div** (≈0/board) → negligible net. Penalty-X of their escape on values after a business XX; kept default-on qualitatively | fresh | default-on ✓ (rare, negligible net) |
| set_lebensohl_style (Off/Plain/Transfer) | `ab-lebensohl --ns transfer --ew plain` | Artificial | Transfer | **Transfer ≥ Plain, isolated** (contested self-play, 400k/cell ×2 seeds ×2 vuls, plain-DD only, SHA 03d981f, `scripts/a3-run.sh`): plain **+0.002 NV / +0.003–0.004 vul** IMPs/board — all 4 cells positive both seeds (+0.30/+0.55 IMPs/div, 0.6% fire, systems-on over 2♣ excluded). Transfer default vindicated; Plain/Off stay opt-in. Structure: 2NT-relay transfers, Smolen, Leaping Michaels | fresh | default-on ✓ (Transfer) |
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
| set_high_overcall_responses | `--ns-high-overcall` | Artificial | OFF | plain −0.0012/−0.0007, PD −0.0005/−0.0006 (all CI⊇0, SEED 1783286003); named leak: 3-level neg-X too light | fresh | stays opt-in; Lebensohl re-measure candidate (see competitive-book.md P3a) |
| set_jordan_truscott | `--no-ns-jordan-truscott` | Artificial | ON | plain +0.0041/+0.0067, PD +0.0049/+0.0065 (all CI>0; campaign's largest per-bd, SEED 1783286386) | fresh | default-on ✓ |
| set_splinter_doubled | `--no-ns-splinter-doubled` | Artificial | ON | plain +0.0059/+0.0079, PD same (FirstIs(Double) systems-on rebase, SEED 1783439089) | fresh | default-on ✓ |
| set_delayed_cue | dedicated (no flag) | Artificial | OFF | gated for measurement, no headline | unmeasured | needs A/B |
| set_major_support_double | `--no-ns-major-support-double` | Artificial | ON | plain −0.0004/+0.0004 wash, PD +0.0009/+0.0016 (vul CI>0, SEED 1783285623) | fresh | default-on ✓ (plain-wash+PD-gain) |
| set_cachalot_contested_x | `--no-ns-cachalot-contested-x` | Natural | ON | NV win all 3 scorers, vul wash ([project_school-tournament-responses]) — **no-op unless neg-double=Cachalot (opt-in)**; opener's raise of the shown major is unalerted/natural | fresh | default-on ✓ (dormant) |
| set_cue_raise_answer | `--no-ns-cue-raise-answer` | Natural | ON | **A4 pass** (`scripts/a4-run.sh`, JOBS=12, SHA 3dc5cbe + non-behavioral audit edits): plain **+0.0256/+0.0348**, PD **+0.0377/+0.0462** NV/vul — all 4 cells CI>0, PD≥plain (fires 0.33%, +7.8…+13.5 IMPs/fired; thin fired set, first pass). Fixes floor-passes-the-cuebid ([project_cue-raise-answer]); opener's 3M/4M raise is unalerted/natural | fresh | default-on ✓ (Natural capability-add: off strands the cuebid) |
| set_cue_minor_raise_answer | `--no-ns-cue-minor-raise-answer` | Natural | ON | **A4 pass** (same run): plain **+0.0134/+0.0184**, PD **+0.0211/+0.0262** NV/vul — all 4 cells CI>0, PD≥plain (fires 0.25%, +5.4…+10.5 IMPs/fired; thin fired set). Minor twin; 3NT/3m-4m replies unalerted/natural | fresh | default-on ✓ (Natural capability-add) |
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
| set_advance_sohl_style (Off/Plain/Transfer) | `ab-sohl-after-double --ns off\|plain\|transfer` | Artificial | Transfer | Transfer clear PD win over flat ladder +0.145/+0.227 IMPs/bd (200k filtered) | fresh | default-on ✓ (Transfer) |
| set_leaping_michaels | `ab-leaping-michaels --ns on\|off` | Artificial | ON | +1.090/+1.452 IMPs/bd; inference reader prices slam | fresh | default-on ✓ |
| set_notrump_balancing | `--ns-balancing` | Artificial | OFF | **A5 pass** (`scripts/a5-run.sh`, JOBS=12, sha 54a1afa): plain +0.0004/−0.0003, PD −0.0002/−0.0013, sd +0.0008/+0.0003 NV/vul — wash on every scorer (all cells CI⊇0), sd shows no real edge (SEED 1783882108) | fresh | opt-in (= floor) |
| set_stayman_defense | `--ns-defense-to-their-stayman` | Artificial | OFF | lead-directing (DD-invisible), PD wash | stale-pop | opt-in (DD-blind) |
| set_transfer_defense | `--ns-transfer-defense` | Artificial | OFF | PD wash (+0.006/fired CI⊇0, 640k); plain loss = light-sac artifact | fresh | opt-in (lead-directing) |
| set_minor_transfer_defense | `--ns-minor-transfer-defense` | Artificial | OFF | **A5 pass** (`scripts/a5-run.sh`, `--isolate-defense --filter-1nt`): measured LOSS all scorers — plain −0.0041/−0.0064, PD −0.0060/−0.0082, sd(floor) −0.0041/−0.0060 (every cell CI<0; −3.7…−7.0 IMPs/fired). sd is a floor (ab-dump-sd can't disclose the transfer) yet still negative → the lead-direction can't pay its cost (SEED 1783882432) | fresh | stays opt-in (measured loss) |
| set_diamond_transfer_defense | `--ns-diamond-transfer-defense` | Artificial | OFF | clear loss over 1M `--filter-1nt` boards (387 fired) | fresh | stays opt-in (measured loss) |
| set_natural_double_shape (Balanced/SemiBalanced/Any) | `--ns-double-shape …` | Natural | Balanced | Balanced dominates: self-play −0.70/div (−0.92 PD, 16.9k div) vs Any; flipped back 2026-06-26 | stale-pop | keep Balanced (Nat discipline) |
| set_takeout_support (Off/Lenient/Strict) | `--ns-takeout-support …` | Natural | Strict | shipped 3+ card support gate; default-on (21gf-ledger) | fresh | fold into base |
| set_overcall_discipline | `--ns-overcall-discipline …` | Natural | ON | shipped: 1-lvl cap 17, 2-lvl opening-11+ | fresh | fold into base |
| set_passed_hand_overcall | `--no-ns-passed-hand-overcall` | Natural | ON | **A5 pass**: plain +0.0008/+0.0011, PD +0.0006/+0.0012, sd +0.0009/+0.0009 NV/vul — consistently wash-positive on all 6 cells (each CI⊇0, closest NV plain [−0.0001,+0.0017]; thin 0.09–0.15% fired, sd does not contradict). **Folded into base 2026-07-13** (Natural × ≥floor per matrix; captain-limited passed hand, never negative) (SEED 1783882270) | fresh | folded into base ✓ |
| set_two_level_minor_overcall_tight | `--ns-two-level-minor-overcall-tight` | Natural | OFF | tightening LOSES all bands, sd-lead confirms loss real | fresh | stays opt-in (measured loss) |
| set_nt_overcall_no_major | `--ns-nt-overcall-no-major` | Natural | OFF | **A5 pass**: plain +0.0004/+0.0006, PD +0.0013/+0.0012 NV/vul — all 4 cells wash-positive with PD≥plain, but each CI⊇0 (thin, 0.15% fired, ±0.0020–0.0031; +0.28…+0.85 IMPs/fired). Discipline is bridge-sound (don't bury a 5cM under a 1NT overcall) (SEED 1783881962) | fresh | fold-into-base candidate; keep OFF pending a deeper re-measure to clear CI>0 |
| set_nt_overcall_systems_on | `--no-ns-nt-overcall-systems-on` | Artificial | ON | shipped default-on: graft opening-1NT responses below [1t,1NT]; sd WIN 4/4 | fresh | default-on ✓ |
| set_nt_overcall_gladiator | `--ns-nt-overcall-gladiator` | Artificial | OFF | completed book WASH vs graft ([project_gladiator-major-overcall]) | fresh | opt-in (= floor) |
| set_responsive_takeout | `responsive-ab --conv takeout` | Artificial | ON | canonical responsive double, shipped default | fresh | default-on ✓ |
| set_responsive_overcall | `ab-responsive --conv overcall` | Artificial | OFF | **A5 pass** (`ab-responsive --conv overcall`, 400k×2 filtered, PD self-play vs floor): NV **+0.928 IMPs/divergent** (+0.009/filtered), vul **−0.178/divergent** (−0.002/filtered); 1.0% divergent (~4.1k boards). Clear NV win, small vul loss — mixed by vul; non-standard extension (BBA's is takeout-only) | fresh | opt-in (NV edge; loses vul) |
| set_rich_advance_double | `--no-ns-rich-advance` | Artificial | ON | shipped 2026-07-11: byte-identical book, SIG+ after 0.10.0 double-discipline shift (was −0.0011 wash) | fresh | default-on ✓ |
| set_advance_rubens | `--ns-advance-rubens` | Artificial | OFF | DD+sd wash (no effect unless rich on) | fresh | opt-in (= floor) |
| set_longest_first_advance | `--no-ns-longest-advance` | Artificial | ON | shipped w/ rich book: rich+longest SIG+ all 4 scorers; WASH standalone on flat book | fresh | default-on ✓ (paired w/ rich) |
| set_advance_minor_jump | `--no-ns-advance-minor-jump` | Artificial | ON | shipped: bare LOST; both doubler continuations → 2-seed SIG+ all 4 cells, plain ≥ PD | fresh | default-on ✓ (both sides) |
| set_advance_2nt_continuation | `--no-ns-advance-2nt-continuation` | Artificial | ON | shipped: doubler accept/decline of 2NT invite, wash-positive all 4 cells | fresh | default-on ✓ |

### A6 — Floor & inference engine toggles

Not a book of calls — these read the auction, settle the floor, or evaluate a
hand. Judged by measured sign; kept default-on when they help. **A6 pass closed
2026-07-13** (`scripts/a6-run.sh`, SHA e688d82, self-play 1M boards/cell ×2 vuls
dual-scored, bba diffpair for rubens): every `unmeasured`/`stale-PD` cell landed
on the current plain-DD harness. Headline: `fuzzy_fifths` flipped **default-off**
(net loss vs raw HCP).

| Option (knob) | CLI | Kind | Default | A/B verdict | Fresh | Policy → action |
| --- | --- | --- | --- | --- | --- | --- |
| set_inference_aware | `ab-inference-floor` (scripts/a6-run.sh) | Engine | ON | floor consults auction interpretation vs shape-blind fallback (foundational): **plain +0.0270/+0.0347, PD +0.0242/+0.0309** NV/vul (1M×2, all CI>0, fires 1.86%, +1.3-1.9 IMPs/div) | fresh | keep default-on ✓ (WIN/WIN) |
| set_settle_floor | `ab-settle-floor` (seeded+dual) | Engine | ON | "pass = play top bid": **DD +0.047/+0.092, PD +0.062/+0.107** NV/vul (1M×2, all CI>0, PD≥DD, fires 6.2%). Refreshes the stale-PD +0.26/+0.37 | fresh | keep default-on ✓ (WIN/WIN) |
| set_alert_reading | `ab-alert-reading` (seeded+dual) | Engine | ON | reads alerted call as artificial: **plain +0.0171/+0.0234, PD +0.0207/+0.0278** NV/vul (1M×2, all CI>0, PD≥plain, +2.0-3.3 IMPs/div). Refreshes the stale-PD +2.08/+1.59; contested defense-switch value is extra | fresh | keep default-on ✓ (WIN/WIN) |
| set_fallback_projection | BBA A/B | Engine | ON | decodes contested/fallback conventions: plain +0.0006 (+1.03/fired), PD +0.0014 (+2.38/fired), CI>0 | fresh | keep default-on |
| set_control_bid_reading | A/B off arm | Engine | ON | reads high new-suit as control vs to-play (M6.4, BWS-distilled) | fresh | keep default-on |
| set_nt_invite_inference | `ab-nt-invite` (seeded+dual) | Engine | ON | reads responder invitational (~8-9) vs GF (10+) of our 1NT — **INERT: 0 divergent at 1M×2**. Puppet Stayman routes the 8-9 invite through `1NT-2♠` (two-way clubs-or-invitational), so the natural `1NT-2NT` this reads is never reached | fresh (NULL) | keep default-on (dead in the current book; harmless) |
| set_rubens_transfer_reading | `--no-ns-rubens-reading` (bba diffpair) | Engine | ON | records 1-level Rubens transfer meanings; unblinds overcaller + sampler: **WASH** vs BBA — plain +0.0003/+0.0004 (CI⊇0), PD +0.0002/+0.0001; fires 0.01% (9-10/76.8k, per-fired set too thin to price) | fresh | keep default-on (structural; net ≈0) |
| set_rule_accept | `ab-landy` | Engine | **ON** | sampler accepts hand by replaying authoring rule: +0.24/bd on 1NT-defense. **Flipped default-on in `74d783d`** (sound-search Phase 1b). Known hole: a *partial* fallback table leaves unnamed calls at −∞ while the table itself keeps mass, so the gate rejects them for every hand and replay yields 0% — the 2/1 game backstop is the found instance, see [sound-search.md](ai-bidder/sound-search.md) | fresh | keep default-on |
| set_fuzzy_strength | `ab-fuzzy-strength --policy both` | Engine (eval) | ON | umbrella (points+fifths) vs raw HCP: **plain +0.0947/+0.0994, PD −0.0469/−0.0557** NV/vul (1M×2) — the plain win is the `points` half; the `fifths` half is a net drag (see below), so points-only dominates points+fifths on both scorers | fresh | keep default-on (= points-only now that fifths is off) |
| set_fuzzy_points | `ab-fuzzy-strength --policy points [--sd]` | Engine (eval) | ON | points() suit upgrade alone: **plain +0.1060/+0.1163, PD −0.0363/−0.0399** NV/vul (1M×2). Plain-win / PD-erases (doubling-artifact shape), but **sd-lead arbitrates for it** (`--sd`, +0.1639/+0.1939 NV/vul, 300k×2, both CI>0) — the aggression survives a realistic blind lead | fresh (sd-vindicated) | keep default-on ✓ (plain+sd win; PD is the pessimist bracket) |
| set_fuzzy_fifths | `ab-fuzzy-strength --policy fifths` | Engine (eval) | **OFF** | fifths() NT-gauge vs raw HCP: **plain −0.0118/−0.0177, PD −0.0110/−0.0165** NV/vul (1M×2, all CI<0) — LOSS/LOSS, and it dragged the umbrella. **Flipped default-off 2026-07-13** (raw HCP for NT ranges; consistent w/ archived 1NT-open fifths loss + [project_nt-invite-evaluator-sweep]) | fresh | **improved: default→off** (raw HCP wins; knob kept for re-measure) |
| set_fifths_companion (Hcp/Bumrap) | `ab-fifths-companion` | Engine (eval) | Bumrap | honor count averaged into fifths (HCP−BUMRAP swing): **−0.0044/−0.0074 plain, −0.0037/−0.0064 PD** NV/vul (1M×2, CI<0) → BUM-RAP beats HCP. Now **dormant** (fifths default-off) but the default is right when fifths is manually enabled | fresh | keep default Bumrap (dormant) |

### A7 — Slam & keycard

**Pass CLOSED 2026-07-16.** The A7 rows sit behind the **slam-optimism
wall** — every DD-play scorer hands declarer the guesses (docs/measurement.md,
slam-boundary addendum).  Reading rule as revised after calibration: the
verdict comes from **plain + PD**; the insurance is the **analytic Pavlicek
Δlogit shave** (2–6% off the slam-win contribution at the 6-level, ~6–20% at
grands — DD is nearly calibrated at small slams, the wall bites grands); the
**sd-declarer playout** (`single_dummy_playout`, a deliberate deep-pessimist:
guess haircut 2–4× Pavlicek's) is the free robustness lower bound, never an
auto-demoter.  Campaign `scripts/a7-run.sh`, results `ab-results/a7`
(sha 23d3768): **five experiments, four confirms, one inert, nothing
demoted** — and no verdict even needed the shave, since every win survived
the playout bound outright (the one flip, floor-rkcb NV, was a statistical
wash).  `set_floor_slam_entry` 29-vs-33 re-arbitrated under the new brackets:
plain +0.004/+0.004, PD +0.003/+0.004, sd-declarer +0.001/+0.002 (1M×2, 2,453
div, all CI>0) — the shortness slams don't hinge on guesses; 29 confirmed
default-on.

| Option (knob) | CLI | Nat/Art | Default | A/B verdict | Fresh | Policy → action |
| --- | --- | --- | --- | --- | --- | --- |
| set_floor_rkcb | `--no-ns-floor-rkcb` | Engine/Artificial | ON | a7-run: plain +1.01/+1.03 per fired (320k×2, fires 0.15%, NV CI>0, vul borderline), PD +0.84/+0.77, sd-lead +2.36/+2.93 (the strongest bracket — right-siding + lead-proofing value); sd-declarer NV −0.22/fired (CI straddles 0, a wash not a loss), vul +0.12 | fresh | default-on ✓ (capability-add; the one playout flip of the pass, retained per the Pavlicek rule — a ±0.0013 CI around −0.0003 is noise) |
| set_minor_keycard | `--no-ns-minor-keycard` | Artificial | ON | a7-run self-play [10M×2, 847 div]: plain **+5.23/+6.68 per div** ÷ PD +5.22/+6.68, and keeps ~75% even under the deep-pessimist playout (+3.87/+5.04) — keycard's value is *staying out* of slams off two aces, line-independent.  *Knob added 2026-07-16*: the original off arm was a worktree revert of `99da1b3` that no longer applies, so the off-state is now authored (strong-2♣ blind 6m jump on 27+, inverted-minor 3NT top-out) and byte-identical default-on | fresh | default-on ✓ |

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
| set_fit_sum_game | 31 | combined pts + trump length to game a known-fit major (0 = off, flat 25-gate) | [project_fit-sum-game-gate] (31 vs off default-on; swept 34→27, deconfounded vs flat gate) |
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

**Never isolated (`unmeasured`):** set_delayed_cue,
set_defense_to_2d_multi, set_suppress_4432_vs_major, set_suppress_4432_vs_minor,
set_direct_3nt_stopper.
*(A5 pass closed 2026-07-13: set_nt_overcall_no_major, set_notrump_balancing,
set_passed_hand_overcall, set_minor_transfer_defense (bba-gen arm/diffpair, +sd
for the DD-blind trio) and set_responsive_overcall (ab-responsive self-play)
isolated via `scripts/a5-run.sh` — all fresh, see A5. minor_transfer_defense a
decisive loss; notrump_balancing wash and responsive_overcall NV-win/vul-loss →
opt-in; nt_overcall_no_major + passed_hand_overcall wash-positive but thin
(CI⊇0). **passed_hand_overcall folded into base default-on 2026-07-13** (Natural
× ≥floor per matrix, never negative on any scorer); nt_overcall_no_major still an
open fold-in candidate pending a deeper re-measure. The six A5 `?` CLI cells were
resolved to their real flags in the same pass. A5 follow-up: **improve advances
to our 1NT overcall** — enrich the advancer structure over
set_nt_overcall_systems_on (today a systems-on graft of opening-1NT responses);
set_nt_overcall_gladiator is an opt-in wash and set_nt_overcall_no_major an open
fold-in candidate. Author, then re-measure.)*
*(A4 pass closed 2026-07-13: set_cue_raise_answer + set_cue_minor_raise_answer
isolated via `scripts/a4-run.sh` — both fresh, clean wins, see A4. The two
remaining A4 knobs, set_delayed_cue and set_direct_3nt_stopper, have no bba-gen
flag and need bespoke self-play distillation — still unmeasured.)* *(A3 pass
closed 2026-07-12: set_one_nt_runout*, set_penalize_escape_*, and
set_lebensohl_style isolated via `scripts/a3-run.sh` — all fresh, see A3.)*
*(A6 pass closed 2026-07-13: every A6 engine toggle isolated via
`scripts/a6-run.sh` — see A6. inference_aware / alert_reading / settle_floor all
WIN/WIN (the last two refresh their stale-PD figures); nt_invite_inference INERT
(0 divergent — Puppet Stayman `1NT-2♠` routes the invite off `1NT-2NT`);
rubens_transfer_reading a bba WASH; **fuzzy_fifths flipped default-off** (net loss
vs raw HCP, dragged the umbrella); fuzzy_points kept default-on (plain+sd win, PD
is the doubling-artifact bracket); fifths_companion Bumrap confirmed but now
dormant. Five self-play harnesses (ab-inference-floor / ab-nt-invite /
ab-fuzzy-strength / ab-fifths-companion / ab-alert-reading) were brought to
seeded + dual-scored (`seeded_deals` + `report_brackets`) in the same pass, and
ab-fuzzy-strength gained a `--sd` blind-lead arbitrator.)*

**Stale figures (re-measure before trusting the magnitude):**
- `stale-PD` (pre-`a6f2206` PD-era, not comparable to plain-DD):
  set_transfer_super_accept, minor keycard (+6.80/+8.76; PD +5.41/+7.05 is the
  conservative re-measure). *(NotrumpShape shipped Wide6322 as default 2026-07-12 — fresh, see A1. set_alert_reading + set_settle_floor refreshed fresh in the A6 pass 2026-07-13.)*
- `stale-pop` (measured before a book-population shift): set_open_one_notrump,
  set_floor_rkcb, set_natural_defense, set_direct_dont, set_landy,
  set_natural_double_shape, set_stayman_defense.
