# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.10.0] — Unreleased

### Added

- **Rich advance of a takeout double** (`set_rich_advance_double`, **opt-in,
  default-off**; `bba-gen --ns-rich-advance`). The flat advance floor gave the
  advancer of `(1t)–X–(P)` only a cheapest natural suit, `3NT`, and a penalty
  pass — no cue, no way to invite or force. This adds an invitational cue of
  opener's suit asking for a 4-card unbid major (game hands blast `4M`
  directly), a `1NT`/`2NT`/`3NT` stopper ladder, weak shapely game jumps, a
  forced 3-card response when broke, and the doubler's answer to the cue (with a
  finite catch-all so the artificial cue is never passed out). Measured a clean
  **DD-wash** vs the flat floor (−0.0001 NV / −0.0007 vul plain, PD≈plain, CIs
  include 0, 409.6k bd/arm/vul) — sound and complete, but DD cannot see the
  advance's competitive value, so it stays opt-in as the base for the jump-cue
  Rubens layer. Distilled from BBA via `examples/probe-advance-double`. The
  default system is byte-identical.

- **Jump-cue Rubens transfers** on the advance of a takeout double
  (`set_advance_rubens`, **opt-in, default-off**; `bba-gen --ns-advance-rubens`;
  no-op unless `set_rich_advance_double` is also on). A 5+ unbid major
  (invitational-or-better) transfers via the rank below it, so the doubler
  completes and *declares* — right-siding the strong hand. Measured a **dead-zero
  DD increment** (+0.0000 NV / +0.0001 vul plain, CIs span 0) — expected, since
  right-siding is invisible to double-dummy (same trick count whoever declares);
  the value is an sd-lead (single-dummy) re-measure candidate. Default
  byte-identical.

- **5-3-3-2 takeout-double discipline** (`set_suppress_5332_takeout`,
  **shipped default-on**; off-switch `bba-gen --no-ns-suppress-5332-takeout`).
  A takeout double exists to find a 4-4 major fit, but a 5-3-3-2 has no 4-card
  suit — so it can never hold a 4-card major, and `X` cannot find a fit it does
  not have. With a five-card suit it is almost always better to overcall it (or
  the five-card minor at the two level) than to double and let partner pick a
  three-card major. Both the instinct floor's takeout rule and the book takeout
  doubles (over a suit opening and over a weak two) now route a sub-14-HCP
  5-3-3-2 to Pass/overcall instead of doubling; the strong `points(17..)`
  doubles and the 15–18 balanced `1NT` overcall are untouched. Split out of the
  anchor's balanced-takeout lever alongside flat-4333. Measured vs BBA 2/1
  (409.6k bd/arm/vul, both vuls): **plain-DD win both vuls** (+0.0191 NV /
  +0.0401 vul) and **perfect-defense confirms** larger (+0.0601 / +0.0773) —
  plain-positive rules out a doubling artifact, PD amplifies because it punishes
  the doubled overbids we now avoid. ~1.2% fired, +1.5/+3.4 IMPs/fired plain.
  (The sibling **4-4-3-2** suppression is *not* shipped: a 4432 short in their
  suit is a textbook takeout with a real 4-card major to find, and the
  opener-split A/B traced its apparent gain to an over-broad instinct-floor
  competitive-double trigger plus an unauthored `X`-response continuation, not
  to the double being unsound — left opt-in as `set_suppress_4432_vs_major` /
  `set_suppress_4432_vs_minor`, default off.)

- **Flat-4333 takeout-double discipline** (`set_suppress_flat_4333_takeout`,
  **shipped default-on**; off-switch `bba-gen --no-ns-suppress-flat-4333-takeout`).
  A flat 4-3-3-3 has no ruffing value, so a takeout double on 12–14 HCP flat
  4333 overbids: partner advances into a flat 4-3/4-4 with nothing to ruff, and
  when the opponents double our stretch we go for a number. Both the instinct
  floor's takeout rule and the book takeout doubles (over a suit opening and
  over a weak two) now route a sub-15-HCP flat 4333 to Pass instead; the strong
  `points(17..)` doubles and the 15–18 balanced `1NT` overcall are untouched.
  Surfaced as the largest own-side lever inside the anchor's competitive
  round-1 buckets (bucket #5 neighbourhood): our takeout `X` on a
  flat-4333-below-1NT hand ran −10.8k plain / −17.7k PD across the divergent
  union, all 12–14 HCP (sound values, wrong shape). Measured vs BBA 2/1 (409.6k
  bd/arm/vul, both vuls, SEED_BASE 1783443667): **plain-DD win both vuls**
  (+0.0187 NV / +0.0385 vul, CIs [+0.0168,+0.0207] / [+0.0361,+0.0409]) and
  **perfect-defense confirms** larger (+0.0566 / +0.0755) — plain-positive rules
  out a doubling artifact, PD amplifies because it punishes the doubled
  overbids we now avoid. ~1.2% fired, +1.5/+3.3 IMPs/fired plain.

- **Systems-on over their double of our splinter** (`set_splinter_doubled`,
  **shipped default-on**; off-switch `bba-gen --no-ns-splinter-doubled`). A
  splinter (`1M – (P) – double-jump`) is game-forcing, but a double reroutes
  opener's rebid from the constructive book (where the splinter continuation is
  authored) to the competitive book, where it was **unauthored** and fell to
  the floor — which *passed*, leaving the game force doubled at the four level
  (opener holding a four-ace monster passing `4♣x` while the field bids `7♠`).
  A `FirstIs(Double)` rebase now strips the double off the whole subtree, so
  opener — and responder's keycard answers — resolve on the undisturbed
  splinter tree (`4M` sign-off floor, RKCB with slam values). The anchor's #4
  bucket (`Constructive / book / round-1`) tail: all three of its worst boards
  were doubled splinters passed out. Measured vs BBA 2/1 (204.8k bd/arm/vul,
  both vuls, SEED_BASE 1783439089): **plain-DD win both vuls** (+0.0059 NV /
  +0.0079 vul, CIs [+0.0047,+0.0071] / [+0.0063,+0.0095]) and **perfect-defense
  confirms** (+0.0059 / +0.0079, plain ≈ PD — no doubling artifact, we are
  *removing* our own doubled contracts). Rare but decisive: +15.4/+17.6
  IMPs/fired at 0.04% fired.

- **Second-suit agreement in 2/1 auctions** (`set_second_suit_agreement`,
  **shipped default-on**; off-switch `bba-gen --no-ns-second-suit-agreement`).
  After `1M – 2r – 2x – 3x` — responder raises opener's second suit `x` to the
  three level — opener now has a third-call table (4NT RKCB on extras, else a
  sign-off: four of an agreed major, or `3NT`/`5x` for a minor). Previously this
  node had no table and fell to the game backstop, which reverted to game in the
  *major* — playing a 5-2 major fit after the side had found a 4-4+ fit in `x`.
  The last uncovered lever in the anchor's #3 bucket (`Constructive / book /
  round-2`); the bucket is diffuse, so the remaining swing is small. Measured vs
  BBA 2/1 (204.8k bd/arm/vul, both vuls, SEED_BASE 1783434297): **plain-DD win
  both vuls** (+0.0012 NV / +0.0015 vul) and **perfect-defense confirms**
  (+0.0014 / +0.0018), all CI>0, PD ≥ plain (no doubling artifact).

- **Balanced `1NT` rebid after `1m – 1M`** (`set_balanced_1nt_rebid`, **shipped
  default-on**; off-switch `bba-gen --no-ns-balanced-1nt-rebid`). A balanced
  12–14 with a five-card minor now rebids `1NT` instead of the natural `2m`: the
  `2m` rebid (weight 0.9) outranked the balanced-`1NT` rebid (0.5), so a 5332
  minimum misdescribed itself as an unbalanced minor and lost BBA's `1NT`-based
  game placement. Only the 5332-with-five-card-minor shape changes (4333/4432
  hold no five-card minor); the boost sits at 0.92, above the `2m` rebid but
  below the up-the-line `1♠` so a 4-4 spade fit is still found. Traced from the
  anchor's #3 bucket `Constructive / book / round-2` as its single largest lever
  (the `5+♦`/`5+♣` rules, ≈28 % of the bucket, ≈−2150 IMPs). Measured vs BBA 2/1
  (204.8k bd/arm/vul, both vuls, SEED_BASE 1783421467): **plain-DD win both
  vuls** (+0.0076 NV / +0.0109 vul) and **perfect-defense confirms**
  (+0.0085 / +0.0117), all CI>0, PD ≥ plain (no doubling artifact) — a clean
  constructive gain, ships default-on.

- **Rule-of-20 light openings** (`set_rule_of_20`, **shipped default-on**;
  off-switch `bba-gen --no-ns-rule-of-20`). Opens a sound 10-11 count that
  satisfies the Rule of 20 (raw HCP + the two longest suits ≥ 20) with one of a
  suit — five-card major first, else the better minor — instead of passing. It
  gauges *raw* HCP, not upgraded `points`, whose wasted-honor penalty rejects
  exactly the shapely eleven-counts the rule admits (e.g. `AK986.J9.QJT6.64`,
  a Rule-of-20 hand that `points` values at 11). Traced from the anchor's #2
  bucket `Constructive / book / opening`: 61 % of its loss is sound hands we
  passed and BBA opened — 79 % of them eleven-counts, 46 % Rule-of-20.
  Measured vs BBA 2/1 (204.8k bd/arm/vul, both vuls, SEED_BASE 1783410574): a
  **plain-DD win** (+0.0061 NV / +0.0087 vul, both CI>0) the perfect-defense
  scorer erases (−0.0056 / −0.0034) — a doubling artifact by the decision
  table, except the new **sd-lead** bracket (a realistic blind opening lead)
  *confirms and amplifies* the plain win (+0.0096 / +0.0135, both CI>0). The pd
  loss is the over-pessimistic perfect-doubler bracket; reality (nearer
  plain/sd) is a clear gain, so it ships default-on. Opener's disclosed
  one-level suit floor drops 12→10 to match (`apply_opening`).

- **`ab-dump-sd`** — the third A/B scorer bracket (a Pillar C down-payment; see
  [docs/bba-gap-campaign.md](docs/bba-gap-campaign.md)). Re-scores two aligned
  `bba-gen` dumps with a *single-dummy opening lead*: the leader picks blind
  over `--sd-worlds` (default 16) sampled worlds read from the auction, and play
  is double-dummy thereafter — the realistic middle between plain DD (perfect
  killing lead) and perfect defense. Reuses `single_dummy_leads`; compares at
  auction granularity (a different auction to the same contract can draw a
  different lead). `scripts/rule-of-20-ab.sh` orchestrates the paired run.

- **`set_passed_hand_overcall`** (opt-in, default off — byte-identical). Relaxes
  the disciplined two-level overcall floor 11→9 for a *passed hand*: it cannot
  hold opening values, so the 11+ floor all but forbids the safe, useful light
  overcall (lead-directing, partner a limited captain); the one-level floor and
  unpassed hands are untouched. Measured vs BBA 2/1 (102.4k bd/arm, both vuls,
  SEED_BASE 1783407558) as a **DD/PD wash** at ~0.1 % fired (|Δ| ≤ 0.001
  IMPs/board, all four CIs straddle 0) — the expected result for a lead-directing
  idea the current scorers are blind to (DD/PD ignore lead-direction and
  obstruction). Kept opt-in as a **single-dummy-lead re-measure candidate**
  (`bba-gen --ns-passed-hand-overcall`).

- **The BBA gap campaign: anchor + decomposition tooling**
  ([docs/bba-gap-campaign.md](docs/bba-gap-campaign.md), the standing plan for
  closing the pons↔BBA gap, floor-first). The gap (−2.59 → −1.997 IMPs/board
  at last measure, then a long unmeasured win streak) had never been
  *attributed* — no seeded anchor was persisted and the only buckets were
  1NT-centric. This ships Pillar A:
  - `bba-gen` dumps now record their **seed and full command line**
    (`Dump.seed`/`Dump.gen_args`, serde-defaulted so old dumps still parse) —
    every dump is self-describing and regenerable forever.
  - **`Stance::explain_call`** (with the new `ExplainedRule`): resolve one
    decision exactly as classification does and name the winning rule —
    provenance (exact node / guarded fallback / instinct floor) plus the
    rule's index, label, and rendered constraint. The underlying
    `Trie::resolve_floored` is the same mass fall-through
    `classify_floored` uses, so attribution can never disagree with play.
  - **`examples/bba-decompose`**: turns anchor shard dumps into a ranked
    IMP-loss bucket report (`report.md` + machine-readable `boards.jsonl`).
    Buckets key on *phase / provenance / auction family* with a
    direction-of-loss triage, dual-scored (plain + PD, flagged when they
    disagree), per-bucket CIs, IMP histogram, right-siding divergences
    counted and excluded, and top-N worst boards per losing bucket with the
    winning rule named per board. Attribution is **derived by replay** (our
    side is deterministic), guarded by a printed replay-verification rate
    that must be 100% — a 40-board smoke test verified 411/411 our-side
    calls reproduced. A `--dd-cache` keyed on deals makes re-anchoring take
    minutes (the warm-cache rerun of the smoke test: 0.09 s).
  - **`scripts/anchor.sh`**: the orchestrator — both vulnerability arms at a
    persistent series `SEED_BASE` (the documented exception to
    fresh-seed-per-experiment), 16×6,400 boards/arm by default, then
    decompose. Runbook in the campaign doc; ship decisions stay per-fix
    fresh-seed A/Bs.
  - **First anchor run (2026-07-06, `SEED_BASE=1783375064`, 204.8k boards,
    replay-verified 100%):** vul none **−1.675** / vul both **−2.310**
    IMPs/board plain DD (pooled **−1.99**; perfect defense **−2.40**). The
    decomposition overturns the going-in anecdote: the gap is
    **book-dominated** (`book` −248k IMPs vs the whole `instinct()` floor
    ~−160k), and by phase **Defensive (−171k) > Constructive (−155k) >
    Competitive (−82k)** — *not* concentrated in competitive auctions. The
    single worst bucket is our **defensive book's first-round overcall /
    takeout-double / two-suiter structure** (−98k; PD −136k). This re-ranks
    the campaign (defensive book first, balancing deprioritized).

### Fixed

- **Off-shape takeout doubles and light 2-level overcalls over their one-suit
  opening — anchor bucket-1 fix.** The first BBA-gap anchor's #1 IMP-loss bucket
  was `Defensive / book / round-1` (−98k plain / −136k PD); tracing it, the
  dominant leak was our **12+ takeout double**. It gated only on shortness in
  *their* suit, so an off-shape one-suiter with a suit *lower* than theirs
  doubled (weight 1.3, out-shadowing the 1.0 two-level overcall), got pulled to
  the three-level, and landed doubled — worst boards our own 3♥×/4♣×/2♥×. Two
  grounded, independent, additive fixes, both **default-on**:
  - **Support gate on the 12+ double** (`set_takeout_support`, default `Strict`):
    the 12+ tier now demands 3+ cards in *every unbid suit*, so an off-shape
    one-suiter overcalls (or waits for the 17+ any-shape tier) instead of
    doubling-and-pulling — matching BBA's two-regime X (12+ with three-suit
    support, else 17+) and expert theory (Cohen/Lawrence: off-shape "double and
    pull" wants 17–18+). `Off` reproduces the historical book; `Lenient`
    tolerates one doubleton.
  - **Two-level overcall discipline** (`set_overcall_discipline`, default on): a
    below-their-suit two-level overcall now needs opening values
    (`points(11..=17)` vs the flat 8–16) because partner can raise us to the
    three-level; the one-level cap rises to 17. `false` reverts to the flat 8–16.
  Measured vs BBA 2/1 (102.4k boards/arm, both vulnerabilities, SEED_BASE
  1783402635, the two levers combined): plain DD **+0.004 / +0.019** IMPs/board
  NV/vul (both-vul CI excludes 0, NV a positive wash), perfect-defense
  **+0.008 / +0.026** (both-vul CI excludes 0). No plain-DD loss at either
  vulnerability; the PD-heavy gains are the signature of "stop overreaching into
  doubled minuses." The two levers touch disjoint boards (doubles vs overcalls)
  and add up (~3.6 % fired combined). `set_takeout_support` /
  `set_overcall_discipline` are the A/B / off-arm knobs (`bba-gen
  --ns-takeout-support off|lenient|strict --ns-overcall-discipline on|off`).

- **Opener's/overcaller's competitive long-suit rebid.** Once our side had bid,
  the keyless instinct floor could only *raise partner* or make a *takeout
  double* in competition — so a self-sufficient one-suiter (e.g. `1♦ (1♥) P
  (2♥)` holding `AKJT984`) was stuck doubling, misdescribing a takeout it does
  not have and missing games the auction is cold for. Now a suit we *personally*
  bid and hold six-plus in is rebid at the cheapest legal level, outranking that
  double; the existing raise ladder carries responder to game. Seat-scoped
  (partner's transfer cannot license a phantom rebid of a suit we never bid) and
  level-capped (never a unilateral game jump over their three-level bid). The
  two-level rebid is unconditional; the more committal three-level rebid demands
  a real source of tricks — **seven cards, or a good six (two of the top three
  honors)**. This split is measured, not assumed: the blanket three-level rebid
  was a *vulnerable loss* (opener 3-level PD −0.016 IMPs/board), and the quality
  gate flips it to a clear win (+0.007) while neutralising the overcaller
  three-level to a wash. Measured vs BBA 2/1 (102.4k boards/arm, both
  vulnerabilities, SEED_BASE 1783316036): plain DD **+0.047 / +0.037**
  IMPs/board NV/vul, perfect-defense **+0.040 / +0.023** (all four cells' CIs
  exclude 0; +0.67…+1.37 IMPs/fired, 3.4 % fired). Default-on
  (`--no-ns-competitive-rebid` / `set_competitive_rebid(false)` for the off arm).

- **Opener's answer to partner's cue-raise.** After `1M – (ovc) – cue – P`,
  opener had no authored rebid, so the auction fell through to the keyless
  instinct floor — whose raise ladder needs partner's *named* suit (the cue) and
  *shown* suit (the major) to agree. A cue-raise decouples them (named = the
  opponents' suit, shown = the major), so no rule fired and opener **passed the
  cuebid out**, declaring e.g. 3♣ on a four-card fit. Now opener accepts to game
  (`4M`, 13+ points) or declines by signing off in `3M` — never Pass. Majors
  only; the trigger excludes the opponents cue-bidding our *own* major (a
  Michaels `1♠-(2♠)`, where responder's `3♠` is a natural raise, not a
  cue-raise). No keycard ask — offering `4NT` here would strand it (the
  contested node has no authored RKCB responses), so a strong opener blasts
  game. Measured vs BBA 2/1 (409.6k boards/arm, both vulnerabilities): plain DD
  **+0.029 / +0.037** IMPs/board NV/vul, perfect-defense **+0.042 / +0.048**
  (all four CIs exclude 0; +8.6…+14.5 IMPs/fired, 0.34 % fired). Default-on
  (`--no-ns-cue-raise-answer` / `set_cue_raise_answer(false)` for the off arm).

- **Opener's answer to a *minor*-opening cue-raise.** The same passed-out-cue
  bug for `1m – (ovc) – cue – P`. The minor answer differs because minor game
  (`5m`) is remote: opener accepts to **3NT** (14+ points *and* a stopper in
  their suit, so we are not run in the overcall suit) or signs off in our minor.
  The sign-off level floats — `3m` when it is still available, else `4m` when a
  three-level cue leaves `3m` below it (e.g. a club opening) — and, since the
  engine does not mask illegal calls, each rung is legality-anchored with
  `min_level_is` so exactly one fires. A separate knob so the A/B isolated it
  over the shipped major answer. Measured vs BBA 2/1 (614.4k boards/arm, both
  vulnerabilities): plain DD **+0.014 / +0.019** IMPs/board NV/vul,
  perfect-defense **+0.022 / +0.026** (all four CIs exclude 0; +6.3…+11.8
  IMPs/fired, 0.22 % fired). Default-on (`--no-ns-cue-minor-raise-answer` /
  `set_cue_minor_raise_answer(false)` for the off arm). The 3NT accept always
  prefers notrump to `5m`; splitting the two on fit length is a future probe.

- **RKCB 1430 five-keycard answer.** The queen-distinguished responses now
  show **2 or 5** keycards on the same step (`5♥` without the trump queen,
  `5♠` with it), matching standard 1430. The answer table previously matched
  exactly 2, so a responder holding all five keycards (four aces + the trump
  king) fell through to the floor instead of giving the keycard answer. The
  ambiguity is harmless to the asker — partner can hold 5 only when the asker
  has 0, who would not be asking — so the asker's continuations are unchanged.
  The fix covers minor trumps too (the answer table is shared).

### Added

- **Web UI: a settings tab (`web/`, "Settings" tab).** Toggle bidding
  conventions like a calculator's basic vs. scientific mode: a dozen curated
  on/off treatments a casual player recognizes (open 1NT, garbage Stayman,
  fourth-suit-forcing, XYZ, Lebensohl, responsive doubles, …), with a native
  **"More…"** expander revealing the full boolean system — 91 `set_*` knobs
  grouped by area (openings, notrump, competition, defense to 1NT, floor,
  inference, fuzzing) — plus **Reset to defaults** (behind a confirmation
  prompt, since it discards every override in one click). The five mutually
  exclusive defense-to-1NT families (natural, DONT, Meckwell, Woolsey, always
  pass) are a radio group instead of independent checkboxes, so exactly one
  is ever active — previously nothing stopped checking more than one, and the
  Rust side's fixed priority order would silently drop all but the winner.
  Every toggle persists to `localStorage` (only the deltas from default,
  replayed onto the wasm bidder at page load) so choices survive a reload.
  New wasm entry point `set_option(key, on)` dispatches each key to its
  `pons::bidding::…::set_*` flag; because a deal rebuilds `american()` and
  wasm is single-threaded, a toggle is picked up from the next Practice/Demo
  board. `lebensohl` is special-cased to preserve the default `Transfer`
  style rather than the wrapper's lossy `Plain`. The default system is
  byte-identical — every box starts at its shipped default, so an untouched
  Settings tab changes nothing.

- **Web UI: a deal editor (`web/`, "Edit" tab).** A PBN text field two-way-synced
  with a 4×13 card palette (the lichess analysis-board idiom) — type or paste a
  PBN deal to fill the board, or click cards to cycle each through
  N→E→S→W→unassigned, and the field rewrites to canonical `N:…` PBN live
  (copyable). **"Bid it out in Demo →"** hands a completed deal to the Demo tab,
  where the 2/1 bots bid it and the double-dummy table scores the result — the
  first way to run the bots on a *chosen* deal rather than a random one. New wasm
  entry point `WebTable::deal_pbn` (parses the editor's PBN to a `FullDeal`, bids
  it out, or returns `null` on a non-full deal); the editor and PBN round-trip are
  pure client-side JS, no wasm involvement for editing.

- **Over their takeout double** (`set_jordan_truscott`, **default on** —
  measured vs BBA 2/1, 204.8k boards/arm/vul, SEED_BASE 1783286386, sha
  bc949dc: plain DD **+0.0041/+0.0067** IMPs/board NV/vul, perfect-defense
  **+0.0049/+0.0065**, all four CIs exclude 0; +0.5…+0.8 IMPs/fired at ~0.8%
  fired — the campaign's largest per-board win. `--no-ns-jordan-truscott` for
  the off arm). Responder's first call over
  `1x-(X)` is re-authored at the deeper `[1x, X]` key — the shipped
  systems-on rebase survives below it for every deeper continuation:
  Jordan/Truscott `2NT` = limit+ raise (4+ majors / 5+ minors, alerted and
  projection-decoded), value `XX` = 10+ without the fit, the jump raise
  **flips preemptive**, 1-level suits stay forcing-as-uncontested, 2-level
  new suits go weak non-forcing (2/1 is off over the double), `1NT` natural
  6–9. Opener nodes shadow exactly the three rebase misreads: Jordan landing
  on Jacoby 2NT (reuses the shipped cue-raise answers), the preemptive `3x`
  landing on a limit raise, and the weak `2y` landing on a 2/1.

- **Extended overcall responses** — four knobs, measured separately (P3 of
  [docs/competitive-book.md](docs/competitive-book.md), 204.8k boards/arm/vul
  each, sha bc949dc):
  - `set_major_support_double` — **default on** (plain-DD wash, CIs straddle
    0; perfect-defense +0.97/**+1.69** IMPs/fired NV/vul, vul CI > 0 — the
    plain-wash + PD-gain ship row; ~0.10% fired;
    `--no-ns-major-support-double` for the off arm): opener's support
    double/redouble extends to `1♥-(P)-1♠` (exactly three spades), reusing the
    shipped minor-opening tables verbatim.
  - `set_free_bids` (**stays opt-in**, `--ns-free-bids`: plain +0.29 NV but
    **−0.30 vul** IMPs/fired, PD −0.31/−0.88, CIs exclude 0): responder's
    natural free bids over an overcall — 1-level new suit 5+ & 6+, 2-level
    non-jump 5+ & 10+, `1NT` 6–10 / `2NT` 11–12 with a stopper.
  - `set_free_bid_floor` (**stays opt-in / default 6, byte-identical**,
    `--ns-free-bid-floor`): the minimum points/HCP for the 1-level free bids
    (new-suit 5+, `1NT`, and the Sputnik natural 4+ majors). Added to test the
    standing "the vul-PD leak is the 6-count floor — sweep to 8+" hypothesis;
    **refuted.** Sweeping 6→7→8 vs off leaves every loss intact (`free8` vs
    off still −0.0128 vul-plain / −0.0212 vul-PD / −0.0066 NV-PD, all CIs < 0)
    while *discarding* free6's NV-plain win (+0.0028 → +0.0001 ~0). `free8` vs
    `free6` shows the removed 6–7 counts were net-**positive** (NV-plain
    −0.0026, vul-plain −0.0019, CIs < 0) and bought **nothing** at vul-PD
    (+0.0012, CI straddles 0); `modern8` vs `modern6` is the same bad trade.
    The vul weakness is **structural and plain-DD-visible**, not a
    floor-height artifact — free-bidding these shapes into a live auction is
    punished vulnerable whatever the strength. Kept as a tuning handle for a
    future *shape/suit-quality* gate (204.8k boards/arm/vul, SEED_BASE
    1783315917, sha c5a0b44).
  - `set_negative_double_shape` (**`BothMajors` stays the default**,
    `--ns-negative-double-shape both-majors|modern|cachalot|sputnik`): the
    negative-double school over our minor openings. `Modern` = BWS/Cohen
    (over `(1♦)` 4-4+ at 6+, over `(1♥)` **exactly** four spades, over `(1♠)`
    4+ hearts at 8+); `Cachalot` = transfer Walsh in competition (X = 4+
    adjacent major, `1♥` = 4+ spades, `1♠` = residual takeout hand), with
    opener's rotated answers authored — the 1-level completion shows
    **exactly three** trumps, forcing; `Sputnik` = Roth–Stone's **residual**
    double, 7+ *denying* a 1-level-biddable major (over `(1♦)` ≤3 in both
    majors, over `(1♥)` ≤3 spades), with the natural free 1-level major
    lowered to 4+ and opener answering via `cachalot_takeout_answer` (NT /
    minor rebid — the double is the *inverse* of a major-showing one, so
    opener must **not** raise the denied suit). All three imply the free bids
    and inherit the free-bid floor leak vs off. The clean inner signals:
    **Modern beats bare free-bids** (+0.95/+1.36 IMPs/fired plain, CIs > 0
    both vuls; PD same sign), **Cachalot ties Modern** (NV wash; vul PD −0.41),
    and **Sputnik ties Modern too** — all four cells wash, CIs straddle 0
    (204.8k boards/arm/vul, SEED_BASE 1783290254, sha ad79b3e). Sputnik was a
    clear loss until opener's answers were authored: an unauthored opener let
    the floor read the residual double with its classic major-*showing*
    instinct and jump the phantom **denied** suit to a doubled 4♠ (−0.017
    vul-PD vs Modern — the entire shape gap; fixed, then wash). Sputnik beats
    bare free-bids on plain DD (+0.005/+0.004 NV/vul, CIs > 0) and wins
    NV-plain vs off (+0.012, CI > 0) but loses vul-PD (−0.021, CI < 0) — the
    shared free-bid leak, not the shape. The `set_free_bid_floor` sweep
    (below) proves that leak is **structural, not the floor height**:
    `modern8` vs off still loses vul-PD −0.0183 (CI < 0). The family's
    default-on unblock is a shape/suit-quality gate on *which* free bids to
    make, not a strength floor; Cachalot's/Sputnik's right-siding wants an
    sd-lead bracket.
  - `set_high_overcall_responses` (**stays opt-in**, `--ns-high-overcall`:
    plain −0.63/−0.35, PD −0.24/−0.33 IMPs/fired, all CIs straddle 0; the
    worst-board bucket is the minor-opening 3-level negative double's
    one-major `or`-shape at 10+ — try 12+ or 4-4 and re-measure): responder
    over their jump/3-level overcalls (`2NT < bid ≤ 3♠`), where the book
    previously stopped at 2♠ — negative X through 3♠, forcing 3-level new
    suits, `3NT` with a stopper, raises — plus opener's forced answer to the
    3-level double.

- **Contested weak twos** (`set_weak_two_competition`, **stays opt-in** —
  measured vs BBA 2/1, 204.8k boards/arm/vul, SEED_BASE 1783284838: plain DD
  wash (−0.0012/−0.0015, CIs straddle 0) but perfect-defense
  **−0.0097/−0.0116** IMPs/board NV/vul (CIs < 0; −1.5/−1.9 IMPs/fired at
  ~0.6% fired). Worst-board buckets: the values-X over their overcall (no
  trump gate) and the contested Ogust (too eager at 14+) are authorable
  leaks to fix before a re-measure; the preemptive raises over (X) are the
  obstruction wall — park for sd-lead. `--ns-weak-two-comp` for the on arm).
  Over their takeout double the uncontested
  responses ride (Ogust still asks, raises stay preemptive — RONF) plus a
  business `XX` on 13+ without the Ogust fit, and every deeper continuation is
  systems-on (their X stripped to a pass), so opener's five-rung Ogust ladder
  and the forcing-new-suit replies survive intact. Over their overcall (≤3♠):
  Ogust-when-legal, a penalty-leaning values `X` (11+, the settle floor
  answers it), and any-strength preemptive raises; a targeted rebase routes
  the contested Ogust to opener's undisturbed answers.

- **Contested strong 2♣** (`set_strong_two_competition`, **default on** —
  measured vs BBA 2/1, 204.8k boards/arm/vul, SEED_BASE 1783285250, sha
  bc949dc: plain DD **+1.86/+2.79** IMPs/fired NV/vul, perfect-defense
  **+2.00/+2.93**, all four CIs exclude 0 at ~0.05% fired.
  `--no-ns-strong-two-comp` for the off arm). Over their double: systems on (the
  X steals no room). Over their overcall: natural game-forcing new suits (the
  uncontested positive shape, legality-anchored), `2NT`/`3NT` balanced
  positives with their suit stopped, `X` = "cards" 6+ — shadowing the floor's
  *takeout* reading of a double made in front of a 22+ opener, the live bug —
  and a waiting Pass backed by opener's forced reopening (natural rebids,
  notrump with a stopper, finite catch-all `X`): 22+ never sells out.

- **Their two-suiters over our 1M** (`set_uvu_over_majors`, **default on** —
  measured vs BBA 2/1, 204.8k boards/arm/vul, SEED_BASE 1783284454, sha
  bc949dc: plain DD **+0.0019/+0.0018** IMPs/board NV/vul (CIs exclude 0;
  +1.43/+1.58 IMPs/fired, ~0.12% fired), perfect-defense +0.0009/+0.0006 the
  same sign. `--no-ns-uvu-over-majors` in `bba-gen` for the off arm).
  Responder structure over their both-minors `(2NT)` — unusual vs unusual:
  `3♣` = limit+ raise, `3♦` = GF 5+ other major (both alerted and
  projection-decoded), `3NT` with both minors stopped, `X` = values + a
  punishable minor — and over their Michaels cue of our own major: the
  known-suit cue (`2♠`/`3♥`) = limit+ raise, `X` = values, direct raises stay
  natural. Opener answers reuse the shipped cue-raise table; the `3♦` force
  gets a raise/3NT/rebid answer node. The inference walk gains the two-suiter
  reading: their direct cue of our opened major no longer reads as a natural
  5-card suit *in our major* (the sampler had been dealing the cue-bidder
  length in the one suit Michaels all but denies), and the `(2NT)` records
  both minors 5-5. Off-state byte-identical.

- **The competitive book renders.** The book pretty-printer (`render-book`) and
  the web book tab printed only exact trie nodes, so the competitive book —
  ~3,000 lines of guarded-fallback machinery (cue-raises, negative doubles,
  support doubles, Lebensohl/Transfer-Lebensohl, UvU, the contested
  Stayman/transfer packages) — rendered as an **empty section**. Guards and
  rebases now describe themselves (`Guard::describe`/`Rewrite::describe`, the
  new exact-suffix `SuffixIs` guard, `described_guard`/`described_rewrite`
  label wrappers) and both renderers walk the new `Trie::fallbacks()`
  enumeration: the competitive section goes from 0 to ~100 sections (e.g.
  `1♠ (overcall ≤2♠)` with the cue-raise and negative-double rules,
  `1♠ X … → systems on`). Render-only — the classification behavior and the
  existing nodes' render output are byte-identical; the invariant test
  `competitive_fallbacks_are_renderable` pins every future guard to stay
  self-describing. Campaign plan for the coverage gaps this exposed:
  [docs/competitive-book.md](docs/competitive-book.md).

- **Web UI (`web/`): the human-facing examples in the browser.** A standalone
  wasm crate (mirroring the gin-rummy-engine site: static
  `index.html`/`app.js`/`style.css` + `wasm-pack`, no server, no framework)
  with three tabs — **Practice** (bid one seat with a bidding box against the
  `american()` bots, per-call top-3 feedback with probabilities, full reveal
  after the auction), **Demo** (watch the bots bid a random board), and
  **Book** (the authored 2/1 books rendered from `Rule::describe()`, live in
  wasm, filterable). Cards render suit-first (`♣Q`); suit colors are CSS
  variables with **orange diamonds** (a "red suit" must stay warm) and
  tentatively dark-blue clubs. Deployed by `.github/workflows/pages.yml`.
- **Double dummy in the browser, via pure-Rust `pons-dds`.** The web crate
  drives `pons-dds` strictly on its single-threaded paths (no rayon on wasm)
  and only **after** the auction: a cached full 5×4 DD table once the hands
  are revealed (demo, and practice reveal — ~0.1 s in wasm), plus a practice
  **oracle** that judges the reached contract over 100 reshuffles of the two
  unseen opposing hands (makes %, tricks min/mean/max, mean score from the
  human's side) — fairness instead of actual-layout hindsight. Needed a
  wasm fix in pons-dds (an always-on `Instant::now` in the bisection loop
  panics on wasm) and a 16 MiB wasm shadow stack for the solver's deep
  recursion.
- **`dd` cargo feature (default on, default build byte-identical).** The
  native C++ double-dummy stack (`ddss`) is now optional: `gib`,
  `single_dummy`, `stats`, `bidding::ev`, and the DD scorers in `scoring` are
  gated behind `dd`, and `search` implies it. `default-features = false`
  leaves the pure-Rust bidding stack, which is what the web crate compiles to
  wasm.

### Changed

- **`practice-bidding` no longer prints the actual-layout DD verdict or par**
  — both are hindsight: they judge the luck of the one true layout, not the
  bidding. The reshuffled-opponent simulation (makes %, mean score over
  reshuffles of the two unseen hands) stays, as it judges the contract on what
  the bidder could actually know; `--simulations 0` still disables it.

- **Stayman-then-minor slam try, default on (`set_stayman_minor_slam_try`).**
  After a Stayman answer
  (`1NT–2♣–2♥/2♠/2♦`), responder's natural `3♣`/`3♦` shows a 5+ card minor
  with slam values (14+) and no fit for opener's major — the 5-4 two-suiter
  whose four-card major (the reason for the 2♣ detour) missed. Opener
  cooperates by raising the minor with a four-card fit and a maximum (else
  signs off in `3NT`), and responder keycards (1430 RKCB in the minor). Unlike
  the deliberately RKCB-free transfer-then-minor path (`gf_minor_answer`), both
  hands are known non-minimum before the four-level, so the five-level keycard
  response is safe. Targets the measured Stayman slam deficit (missed minor
  slams). **A/B (`ab-stayman --treatment`, 1.5M boards/vul, opponents
  silenced): +3.29 / +4.02 IMPs/fired (none/both, plain DD; perfect defense
  identical — no doubling artifact) across 151 fired boards, zero losses.**
  Fires rarely (≈1 in 10k deals) but is pure upside when it does.
- **Meckwell defense to their 1NT, opt-in (`set_meckwell`, default off,
  off-state byte-identical).** DONT's cousin, added *alongside* the shipped
  classic DONT (`set_direct_dont`), not replacing it: the double is a two-way
  **single 6+ minor OR both majors**, `2♣`/`2♦` = a minor + a major (5-4 either
  way), `2♥`/`2♠` = a natural 5+ single-suited major, `2NT` = both minors (the
  Unusual overlay). Both sides authored — the two-way `X` gets a `2♣`
  pass-or-correct relay (the doubler names its minor, or shows both majors as
  `2♥` and the advancer corrects to `2♠`), with doubled/redoubled escapes so we
  never sit in `1NTxx` or a doubled misfit relay; the `2♣`/`2♦` reuse DONT's
  pass-or-correct advances. The two-way `X` reads as a **points floor only**
  (the long-minor and both-majors arms share no sound per-suit fact, exactly
  like the Woolsey double); `2♣`/`2♦` pin the real 4+ minor. Two **probe** knobs
  expose the tunable shape boundaries for measurement:
  `set_meckwell_minor_major_44` (the `2♣`/`2♦` 5-4-vs-4-4 boundary, default 5-4)
  and `set_meckwell_x_four_four` (the both-majors `X`, default 4-4). Wired into
  `bba-gen` (`--ns-meckwell` + the two probe flags) and added as a fifth row of
  the `ab-nt-defense-matrix` GTO tournament (plain-DD / perfect-defense / sd-lead
  brackets + Nash mixture). **Measured a decisive loss and ships off by default,
  like DONT and Woolsey** (`ab-nt-defense-matrix`, 60k boards, vul none, seed
  1783107051): the Meckwell row scores **plain-DD −0.277/−0.238/−0.185/−0.033,
  perfect-defense −0.522/−0.483/−0.389/−0.362, sd-lead −0.068/−0.070/−0.020/+0.123
  IMPs/board** (default / penalty-X / soft / sit counters) and takes **0%
  equilibrium support in all three brackets** (plain and sd-lead pick Woolsey,
  PD picks natural). The dominant leak is the broad two-way `X`: it fires on
  6588 boards (11%) at **−2.28 IMPs/board** plain-DD, versus Woolsey's narrow
  4-major-plus-minor `X` (1318 boards, −0.29) and DONT's one-suiter `X` (3016
  boards, −0.19) — the two-way double at an 8+ floor over a *strong* 1NT is a
  heavier, far more frequent sacrifice, and its points-only reading leaves the
  contested continuations shape-blind (the loss collapses to −0.06 when the
  opponents cannot run). Kept as an opt-in probe target rather than a shipped
  default.

- **Stronger-`X` sweep for Meckwell and DONT** — new opt-in floor knobs
  `set_meckwell_x_floor` and `set_direct_dont_x_floor` (both default `0` =
  inherit the natural overcall floor of 8, **off-state byte-identical**), so the
  broad two-way Meckwell double and the DONT one-suiter double can require strong
  hands only. Measured (`ab-nt-defense-matrix` 8-row, 60k boards, vul none, seed
  1783114165): raising **Meckwell's** two-way `X` floor is a large monotone gain
  — default-column plain-DD/PD/sd-lead **−0.262/−0.498/−0.070 (X≥8) →
  −0.056/−0.153/+0.052 (X≥12) → −0.021/−0.065/+0.064 (X≥15)**, the `X` bucket
  shrinking 6588→2064→526 boards; at X≥15 Meckwell is a plain-DD near-wash and
  *sd-lead positive*, **but still takes 0% Nash support** (Woolsey stays
  +0.077 plain / +0.15 sd — the narrow, lead-directing defense wins, and a
  0.9%-frequency `X` is "Meckwell that mostly passes"). Raising **DONT's**
  one-suiter `X` floor is the *wrong* trade: plain/PD improve (PD −0.290→−0.211)
  but sd-lead **drops +0.107→+0.079**, and DONT(6+) at floor 8 holds 10% of the
  sd-lead equilibrium and 88% bootstrap support while the X≥12 variant gets 0% —
  DONT's one-suiter `X` earns its keep by *directing the opening lead*, which a
  higher floor deletes. Both floors ship at their measured-best default (Meckwell
  inherits 8 as textbook, tunable up; DONT stays at 8).

- **Major-opening continuations, default on** — four knobs complete the
  constructive tree after `1♥`/`1♠`, the families that previously fell to the
  instinct floor (which cannot hold an invitational dialogue: it moves only
  at combined 25 counting partner's shown *minimum*). Package measured
  (`ab-major-continuations`, silenced opponents, 200k boards per cell,
  NV/both-vul, replicated on a second seed base): **plain DD +0.058..+0.061
  (NV) / +0.089..+0.092 (vul) IMPs/board, perfect-defense +0.067/+0.100** —
  a win on both scorers at both vulnerabilities, ~4.2% divergence,
  +1.4/+2.2 IMPs per divergent board.
  - `set_major_game_tries` (off: `--no-ns-major-game-tries`): after
    `1M – 2M`, opener's long-suit game tries (`len 4+`, 16–18, natural and
    unalerted so the free 4+ reading stays sound), the `3M` general re-raise
    try, direct `4M` at 19+, and a `4NT` keycard ask at 22+; responder
    accepts a suit try with a maximum, shortness, or two top honors in the
    try suit; opener may still push on with 18+ over a decline. RKCB
    installed at `[1M, 2M]`. Alone: plain **+0.042/+0.065** NV/vul, PD
    +0.053/+0.081 — the biggest single gap closed.
  - `set_limit_raise_acceptance` (off: `--no-ns-limit-raise-acceptance`):
    after `1M – 3M`, opener accepts with 13+, asks keycards with 19+, else
    passes; RKCB installed at `[1M, 3M]`. The accept threshold is a measured
    story: the textbook 14 (and a 15 retry) **lost** −4.6/−5.2 IMPs per
    divergent board — every divergent board was the table *under-bidding*
    the floor, whose raise-partner ladder already accepts at 13+ and whose
    aggression DD endorses (a known nine-card fit at 23 combined is a clear
    game). At floor-parity 13 the node's whole added value is the keycard
    ask: **+4.4/+5.2 IMPs/divergent** (78 boards/200k, CI excludes 0, both
    scorers). Traced with the new `probe-limit-raise` example.
  - `set_major_rebid_tails` (off: `--no-ns-major-rebid-tails`): both-sides
    continuations under `1♥ – 1♠` for opener's `2♠`/`3♠` raises (invite,
    sign-off, keycards; RKCB installed below both), the `2♥` rebid
    (preference / `3♥` invite / `2NT` invite with acceptances), and the
    `2♣`/`2♦` rebids (jump `3♥` preference on three hearts, minor raise,
    `2NT` invite, weak `2♠` rebid, simple preference, `3NT`), with opener's
    acceptance tables; `1♥–1♠–2m–2♥/2♠` deliberately stay with the floor.
    Alone: plain **+0.016/+0.023**, PD +0.014/+0.020.
  - `set_fourth_suit_forcing` (off: `--no-ns-fourth-suit-forcing`; rides the
    tails knob — inert without it): at `1♥ – 1♠ – 2♣`, responder's `2♦`
    (alerted `fourth-suit-forcing`, points-only constraint so the projection
    claims no diamonds) is an artificial game force; opener answers
    naturally (three-card spade raise first, then extra hearts, a diamond
    stopper, a real second club suit), and responder places the game. The
    `1♥–1♠–2♦` fourth suit (`3♣`, a level higher) is out of scope. Marginal
    on top of the tails: **+0.002** on both scorers at both vulnerabilities.
  Out of scope this round, documented in the module docs: weak-jump-shift
  continuations (obstruction-wall class), a 12-point limit-raise accept
  (out-bidding the floor), alerted 3-card help-suit tries, counter-tries,
  and FSF after the `2♦` rebid. Seeds 1783099705 + 1783101472, uncommitted
  tree at `b7ae839`+.

- **XYZ two-way checkback + the up-the-line minor completion, default on**
  (`set_xyz`, `set_up_the_line`; off-switches `--no-ns-xyz` /
  `--no-ns-up-the-line` in `bba-gen`). XYZ (`src/bidding/american/xyz.rs`)
  covers the ten uncontested `1x – 1y – 1z` auctions: responder's `2♣`
  (alerted `xyz-relay`) puppets opener to `2♦` (alerted `xyz-completion`) for
  a weak diamond sign-off or any invitation, `2♦` (alerted `xyz-game-force`)
  is an artificial game force answered naturally (three-card support first),
  direct two-level rebids are weak, and the invite round carries opener's
  accept/decline tables — both sides authored, readings free via
  alert-projection. The up-the-line completion fills the natural gaps the
  structure needs: the `1♣ – 1♦` response (previously squeezed into the
  notrump ladder or floored), opener's `1♠` rebid over `1m – 1♥` (the 4-4
  spade fit was lost to a 1NT rebid), and opener's natural `2♣` after
  `1♣ – 1♦` on six clubs. Measured jointly (`ab-minor-continuations`, 300k
  boards, both scorers, NV/vul): **+0.038/+0.056 IMPs/board plain DD,
  +0.029/+0.041 perfect-defense**, +0.55/+0.80 per divergent board (6.96%
  divergence) — a win on both ends of the bracket. XYZ alone is
  +0.016/+0.025 plain per board; **up-the-line alone is a loss** (plain
  −0.91/−1.28 per divergent — the 1♦ response reroutes hands into auctions
  only the XYZ round continues), hence the joint ship. The known XYZ cost
  (the natural `2♣` sign-off orphan) is priced into the measurement.

- **Longer-major response discipline (opt-in, default off)**
  (`set_longer_major_response`; `--ns-longer-major-response` in `bba-gen`).
  Responses to `1♣`/`1♦` name the longer major — `1♠` on 5♠4♥/6♠5♥ or any
  5-5, `1♥` up the line only on 4-4 — built as the trio the theorems doc
  prescribes: the response pair, opener's rebid structure, and the M6.4
  control-bid classifier's bypass rule (`classify_high_bid` now swaps its
  two major-response verdicts under the discipline, gated at classify time).
  Measured a **null**: alone it is a plain-DD wash with a perfect-defense
  loss at vulnerable (−0.12/−0.22 per divergent NV/vul, 2M boards, 0.46%
  fired), and its marginal on top of the shipped xyz + up-the-line package
  is −0.003..−0.005 IMPs/board on both scorers — opener's up-the-line `1♠`
  rebid already recovers the concealed 4-4 spade fits, and longest-first
  costs a level on the heart fits. Hearts-first stays the default; the knob
  and its classifier reading remain for re-measurement.

- **Contributor docs and session skills.** Two new documents distill the
  working knowledge of the bidding campaign: `docs/bidding-architecture.md`
  (the book/floor/inference layer cake and the invariants that keep it sound —
  shadowing, the floor partition, alerts and readings, the Constraint DSL,
  samplers, knob conventions) and `docs/measurement.md` (the A/B playbook:
  the scorers, the plain-DD/perfect-defense decision table, the catalog of
  measurement biases, and the ship rules). `CLAUDE.md` was rewritten as the
  repo operating manual indexing them, and two checked-in skills
  (`.claude/skills/author-convention`, `.claude/skills/measure-ab`) encode the
  end-to-end procedures for authoring and measuring a convention. No code
  changes. The README gained a pointer to the docs, and its BBA benchmarking
  section — which still referenced the retired `bba-match` example — now
  documents the current `bba-gen` / `bba-score` pair, the `serde` feature they
  require, and the `scripts/bba-gen-parallel.sh` sharding wrapper.

- **Single-dummy solver** (`pons::single_dummy`, `examples/single-dummy`). Given
  a declarer's hand and dummy — the 26 cards a declarer actually sees — estimate
  how a strain will play by Monte-Carlo double-dummy: deal the two hidden defender
  hands uniformly `n` times, solve each layout double-dummy, and fold the results
  into a `stats::HistogramTable`. Two new readers on `HistogramTable`,
  `expected_tricks(seat, strain)` and `make_probability(seat, bid)`, turn the trick
  histogram into the mean trick count and the fraction of layouts a contract makes.
  It reuses the existing sampler primitives (`Builder` + `fill_deals`) and the
  double-dummy solver, so it needs no new dependency or feature flag. This is the
  standard DD-averaging estimator (as used by GIB); a true imperfect-information
  single-dummy search is out of scope.

- **Single-dummy opening-lead scorer** (`single_dummy_leads`,
  `single_dummy_lead_tricks`, `LeadQuestion` in `pons::single_dummy`;
  `Stance::infer` in `pons::bidding`). Prices the one information seam
  double-dummy scoring is known (Pavlicek, actual-vs-DD) to get most wrong at
  the 1NT level: the DD defender always finds the killing opening lead, so DD
  underrates 1NT declarers by ~7pp of make rate. The scorer has the opening
  leader choose a *blind* lead — maximizing mean defensive tricks over layouts
  sampled consistent with the auction as the leader's own book reads it
  (`Stance::infer` attaches the trie so alerted conventions decode correctly:
  a Woolsey 2♥ samples as Muiderberg, not hearts) — then plays the actual deal
  double-dummy from that card. One trick-one `Target::Legal` solve per sampled
  world prices all 13 candidate leads at once, and the batch API pools
  thousands of positions into single `solve_boards` calls (~5× faster than
  per-position solving, which straggles on slow boards). Validated in-run:
  datum 1NT declarers gain +0.30 tricks over plain DD, matching Pavlicek's
  gap.

- **sd-lead bracket in the GTO 1NT-defense tournament**
  (`ab-nt-defense-matrix --sd-worlds`, default 16; the study doc has the full
  results). Re-scoring the tournament with the blind-lead scorer **dissolves
  the vulnerability split**: Woolsey Multi-Landy is the equilibrium defense at
  *both* vulnerabilities (+0.132 IMPs/board NV, +0.071 vul-both, bootstrap
  200/200; always-passing drops to 0/200 support everywhere) — the
  "vulnerable → pass" law of the DD brackets was, to first order, the
  blind-lead bias flattering the pass-out datum. Every active defense cell
  turns positive at NV. Their counter-equilibrium also shifts from the pure
  shipped-default package to a default/soft mixture: with blind leads paying
  declarers on both sides, doubled contracts are no longer sure things and
  never-punishing becomes co-optimal. Cells are compared at auction (not
  contract) granularity under this scorer, since the same contract reached
  through a different auction gets a different lead. Defaults untouched —
  Woolsey stays opt-in until its continuation gap vs BBA is closed.

- **GTO 1NT-defense tournament** (`examples/ab-nt-defense-matrix`, study:
  `docs/ai-bidder/gto-1nt-defense.md`). "What is the best defense to their
  strong 1NT?" is a *game*, not an A/B — the answer depends on the opening
  side's counters — so this harness plays the full payoff matrix (4 defenses ×
  4 counter-strategies, every cell on identical boards, one DD solve pricing
  all 16 cells against an always-pass datum) and solves the zero-sum matrix
  game by fictitious play, with a bootstrap over boards certifying the
  equilibrium support. Measured at 60k boards per vulnerability, plus the same
  defense menu vs BBA (`--isolate-defense`, 204.8k boards/arm). Headlines:
  **Woolsey Multi-Landy is the nonvulnerable plain-DD equilibrium defense
  (+0.070 IMPs/board over always-passing — the first defense here to beat
  doing nothing on plain DD)**, natural is the perfect-defense-bracket
  equilibrium (+0.029), and vulnerable both revert to always-pass; the
  **counter-side equilibrium is the shipped default package** (Optional
  doubles + trap-pass + penalty conversion + the doubled-1NT runout) in all
  eight scenario×scorer cells — disabling the runout alone flips our
  penalty-X bucket from −0.45 to +2.04 IMPs/action-board. Vs BBA, *defending
  beats passing at every vulnerability on plain DD* (always-pass trails BBA's
  own Multi-Landy by 0.30–0.41 IMPs/board), locating the obstruction wall
  partly in self-play's too-strong counters; our own Woolsey trails BBA's by
  0.23 IMPs/board on the same card — a continuation-quality gap and the
  sharpest improvement target the tournament surfaced. No library changes;
  defaults untouched.
- **Source-of-tricks-eight 3NT force** (`set_long_minor_force`, **off by
  default — a measured loss, kept only as an A/B instrument**). Bridge theory
  says an 8-count with no four- or five-card major and a running long minor
  (7+ cards, or a six-card minor with two top honors) should gamble 3NT on the
  source of tricks, and an analytic screen agreed — pricing 3NT against a
  *notrump* invite/pass looked worth +0.2 to +0.5 IMPs/board. But these hands
  do not stop in notrump, they **transfer**, and the transfer reaches the suit
  game: a live A/B against the real routing (`examples/ab-long-minor-force`,
  8M deals, plain DD, vul none) measured **−7.12 IMPs/fired** — club source
  −7.07 (the `2♠` transfer drives to a *making 5♣* that 3NT throws away),
  diamond source a wash (the `2NT` transfer already reaches 3NT). So no eight
  shape forces; the knob ships off. (Contrast the flat-4-3-3-3 eight, which has
  no long suit and genuinely does stop in notrump — its pass demotion stands.)
- **Longer-major transfer discipline** (`set_transfer_longer_major`, **on by
  default**; `bba-gen --no-ns-transfer-longer`). The Jacoby transfer now
  names the **longer** major — a 6♠5♥ hand transfers to spades whatever its
  strength, where the legacy guards let it *tie* into the heart transfer
  (weak) or fired the both-majors 3♦ (invitational+), losing the sixth spade
  either way (the M6.4 A/B caught a weak 6♠5♥ transferring to hearts and
  scrambling into a phantom 4♠).  With **equal** lengths (5-5, 6-6) the route
  splits by strength: weak transfers to *hearts* for safety, invitational and
  minimum game force show both suits at once via the both-majors `3♦` (now
  restricted to equal lengths), and a slam try (17+) transfers to *spades*
  for the natural game-forcing `1NT–2♥–2♠–3♥` structure.  The 2NT-strength
  table (2NT opening and `2♣–2x–2NT`) follows the same discipline — longer
  major, hearts on every tie (it has no both-majors bid or slam reroute).
  The 5♠4♥/5♥4♠ Stayman reroutes and the splinter reroute are untouched.
  Paired A/B (204.8k 1NT-filtered boards, 47 fired): plain −0.0000 ± 0.0003,
  perfect-defense +0.0000 ± 0.0003 — a double-dummy wash; the discipline
  ships on structural grounds (the deterministic route replaces an arbitrary
  weight tie, and partner can now infer the transferred suit is the longest).
- **M6.4 — slam machinery on the floor: RKCB 1430** (`set_floor_rkcb`, **on by
  default**; `bba-gen --no-ns-floor-rkcb`). With a known eight-card **major**
  fit (three-plus our side) and combined small-slam values (33) the instinct
  floor now *asks* `4NT` before committing, instead of blasting the direct
  milestone six: partner answers the book's 1430 ladder (the keycard counting
  is shared with `american::slam`; the floor's 5♣ also covers **all five**
  keycards, a hole the book's `{1,4}` ladder leaves open and a 2♣ rock hits),
  and the asker signs off at five with two keycards missing, bids six missing
  at most one, or seven holding all five in the grand zone — so **instinct
  decodes instinct on both sides**, with no book node installed. The agreed
  trump is *derived*, not installed: the major maximizing our length plus
  partner's shown floor, with the answerer falling back to the partnership's
  genuinely shown five-plus major — Bridge World Standard's "an agreed suit
  makes 4NT keycard" (else it stays quantitative: a 4NT raise of our own
  notrump is never answered with keycards) — and the ask only fires when that
  trump is *decodable* (a shown-5+ suit, or partner would pass the ask out).
  The ask and answers carry a `floor:rkcb` alert, so the projection
  suppresses their phantom suits from the reading; the answerer respects the
  asker's placement when holding at most one keycard (with two-plus the
  combined-33 correction stays live — the asker may have read an ambiguous
  answer low, and the book's own tables sign off pessimistically).  Five A/B
  rounds against the no-RKCB floor (204.8k boards each, paired) drove the
  scoping: minor and thin 6-2 asks *lost* to the milestone 6NT power-blast
  (double-dummy monetizes honors at 33-plus), majors-with-a-real-holding
  ended a clean wash with the safety net kept — final round 4 fired /
  204.8k, delta exactly zero, plain and perfect-defense alike.
- **M6.4 — control bids vs to-play, a deterministic reading**
  (`set_control_bid_reading`, **on by default**; `bba-gen
  --no-ns-control-bid-reading`). A four-plus-level new-suit bid in an
  undisturbed auction is a **control bid iff the bidder *bypassed* the suit** —
  it was biddable more cheaply (same level, lower strain) at their first
  suit-showing call and they chose another suit: `1♦–1♠–2♦–4♥` had 1♥
  available under 1♠, so hearts are short and 4♥ agrees diamonds (support and
  slam-try values recorded, the phantom hearts suppressed) — likewise
  `1NT–2♥–2♠–4♥` through the transfer overlay. Everything else — a bidder who
  has shown nothing (`1♦–4♥`), a suit *above* the first-shown one
  (`1♣–1♥–2♣–4♠`, the post-transfer `1NT–2♦–2♥–4♠`) — reads **to play:
  suppressed, with nothing floored**.  Two A/B rounds fixed each half: the
  naive "shown another suit ⟹ can't be longest" rule bled −6.1 IMPs per fired
  board pulling natural 4♠s to the "agreed" minor (this system's response and
  transfer styles bid the cheaper suit first holding a longer higher one —
  probed: it responds 1♥ to 1♣ on 6♠5♥), and flooring the to-play bid at six
  rerouted combined-33 hands from the winning 6NT power-blast into thin 6-2
  suit slams — so the bypass rule decides *control*, and to-play stays
  unfloored.  The classification itself now rides on
  `Inferences::control_bid` (an unread to-play bid and a control bid are
  otherwise indistinguishable), and the floor gains the matching **signoff**:
  partner's control bid is never passed out — instinct returns to the agreed
  suit at the cheapest level, and with slam-zone values the RKCB ask takes
  over, closing the loop the Rubens A/B taught (unauthored continuations pass
  out artificial calls).
- **Rubens advances become a knob** (`set_rubens_advances`, **on by default** —
  unchanged behavior; `bba-gen --no-ns-rubens`). The transfer advances of
  partner's simple overcall shipped unconditionally, so the convention itself was
  never A/B-measurable — only the book→floor migration was (neutral). Off
  recovers a *natural-advances baseline*: the limit-plus raise reverts to the
  natural raise ladder (the limit distinction is the honest natural price), and a
  knob-off **natural two-level new-suit advance** is authored covering exactly
  the new-suit transfer's hand class (five-plus cards between their suit and
  partner's, ten-plus upgraded points) — without it the floor had *no* call for
  those hands (the natural five-card-suit rule is anchored on "our side has not
  bid") and the A/B would have measured Rubens against a pass. The `Inferences`
  reading shares the knob: off, an advance in the band reads as a genuine suit.
  **The first A/B found real leaks** (204.8k boards vs BBA, paired, 2217 fired:
  plain −0.0111 ± 0.0022, perfect-defense −0.0240 ± 0.0030 IMPs/board),
  concentrated in three unauthored tails — a quarter of the divergent boards
  *passed out the two-level cue-raise to play the opponents' suit*, opener's
  lead-directing double silenced the mechanical completion (leaving advancer in
  the phantom suit doubled, −14 a board), and takeout-double structures were
  misdetected as overcall advances. With the tails authored the re-run (204.8k
  boards, fresh seeds, 1378 fired) recovered to **plain +0.0012 ± 0.0016
  (wash), perfect-defense −0.0029 ± 0.0019**; with the **both-sides
  continuations** below the third run (204.8k boards, fresh seeds, 1144 fired)
  reaches **plain +0.0016 ± 0.0015 (a small win, CI excluding zero),
  perfect-defense −0.0009 ± 0.0017 (wash)** — the useful-space structure beats
  natural advances once completely authored, and stays **on by default**.
- **The Rubens tails are authored** (part of the knob work above, on whenever
  Rubens advances are on): the overcaller *answers* the two-level cue-raise —
  retreat to three of the overcall suit as the guaranteed action, or place the
  game with a maximum (`4♥` on the heart fit, `3NT` over a minor with their
  suit stopped) — the mechanical completion fires *through* opener's double
  (bidder and reading suppression both), and `overcall_shape` requires the
  overcall to be the side's **first action**, so a double-then-bid auction is an
  advance-of-double structure, never a Rubens one.
- **Both Rubens partners continue with extras** (the useful-space cash-in): the
  into-partner completion is *graded* — `2Y` denies extras, `3Y` super-accepts
  at 13–14, a 15+ maximum places the game — and the raiser drives on with 14+
  over the minimum completion. The new-suit completion covers exactly the hands
  that would have *passed* a natural non-forcing `2 target`; with a three-card
  fit and values the overcaller bids what it would have bid over that natural
  call (invite raise 13–14, game 15+), and the transferee clarifies its
  wide-yet-unlimited transfer over the completion — 12–13 re-raises as the
  invite the natural non-forcing treatment never had, 14+ bids the six-card
  major game or `3NT` behind a stopper. This is what let the transfer subsume
  both sides of the natural forcing/non-forcing debate on the A/B.
- **One-level Rubens transfers now record their meaning**
  (`set_rubens_transfer_reading`, **on by default**; `bba-gen
  --no-ns-rubens-reading`). The transfers were suppress-only — after
  `(1♣) 1♠ (P) 2♥` the overcaller read *nothing* from the limit-plus raise,
  so game acceptance, the constrained sampler, and the neural features (whose
  `Inferences` block shifts values on these auctions — no layout change, no
  `FEATURES_VERSION` bump) were all blind to it; only the two-level cue-raise
  recorded. Now the transfer into partner's suit records three-plus cards in
  the overcall suit, a new-suit transfer records five-plus in its target, both
  ten-plus points, at the transfer index — the completion is *not* required,
  since the shown values matter most when fourth hand intervenes over the
  transfer. Recorded for the advancer's own side only: an opponent's in-band
  advance may be a genuine suit (BBA's advances are natural), and asserting
  length in the suit above would poison the sampler. A/B (204.8k boards vs BBA,
  paired, Rubens on in both arms, 42 fired — the reading only changes the
  overcaller's *decision*): **plain +0.0005 ± 0.0004 IMPs/board (+2.55 per
  fired), perfect-defense +0.0003 ± 0.0005** — a small real win; it converts
  cue/transfer passouts into bid games.
- **Responder's continuation after opener's Stayman slam-try cue**
  (`set_stayman_cue_continuation`, **on by default**). After `1NT–2♣–2M–3OM`,
  opener cue-bids a control (`4♣`/`4♦`/`4♥`) to accept the slam try with a maximum —
  but responder had no authored rebid, so the floor *passed the cue out*, frequently
  **below the major game**. This was the single dominant leak in our Stayman auction
  against BBA (an `--isolate-opening bba` study found the uncontested `2♣` continuation
  losing ≈0.24 IMPs/board, ~20% of the tail-loss IMPs concentrated in this passed-out
  cue). When on, responder resolves the (choice-of-game *or* slam) `3OM` bid: a
  slam-worthy hand launches **RKCB** (`4NT`, the 1430 ladder placing the contract),
  everything else **signs off in the major game**. A paired A/B vs BBA (384k
  boards/arm, `--isolate-opening bba`, seed-fresh) measured **+0.0193 IMPs/board plain
  and +0.0216 perfect-defense** (both 95% CI ±0.0015; +8.73/+9.74 per fired board,
  850 fired = 0.22%), both CIs excluding 0 — one of the largest per-fired gains in the
  Stayman structure, since it converts a below-game passout into a game or slam.
- **A game-forcing structure after the spade transfer** (`set_transfer_gf_majors`,
  **on by default**). After `1NT–2♥–2♠`, responder's game-forcing hands previously
  fell to the floor's natural raise, with no way to show a two-suiter, a minor side
  suit, or a splinter. Against BBA (960k boards, seed-fresh) the structure gains
  **+0.0014 IMPs/board plain and +0.0016 par-doubled** (both 95% CI ±0.0003,
  +1.70/+1.90 per fired board). When on:
  - `3♥` is a natural **5-5 majors slam try** — the slam end of the both-majors hands,
    rerouted off the direct `1NT–3♦` jump (now capped at `point_count ≤ 16`, a clean
    invitational-through-minimum-game-force range). Opener agrees spades and launches
    RKCB with a maximum, or signs off in `4♠`.
  - `3♣`/`3♦` show **five spades and a four-card minor**. By default they are shown on
    any game force (Arm A); with `set_minor_min_to_3nt` on (Arm B) they are reserved
    for slam tries, minimum game-forces instead resting in the floor's choice-of-games
    `3NT`. Opener places game on the five-three spade fit — `4♠` with support (its
    ruffing value beats an un-pulled `3NT`), else `3NT`.
  - `4♣`/`4♦`/`4♥` are **splinters** — a six-card-spade slam try short in the bid suit
    (a void or a low singleton; a singleton ace or king is a working honor, not a
    splinter). These are carved off the direct Texas `4♦` and `4♠` slam routes onto the
    transfer, so `set_texas_slam_drive` keeps the no-splinter hands unchanged.
  - `4NT` is the single-suiter's **quantitative slam invite** (a balanced 16+
    five-spade hand), relocated off the now-repurposed `3♥`; opener accepts to `6♠`/
    `6NT` or passes.

  The choice-of-games `3NT` is now an authored balanced-hand node and opener corrects
  it to the eight-card fit by default — see **Changed**. Disable with
  `set_transfer_gf_majors(false)` (or `bba-gen --no-ns-transfer-gf-majors`). `set_minor_min_to_3nt` (Arm B) stays **off** —
  the A/B refuted it: showing the minor beat lumping minimums into `3NT`.
- **The game-forcing structure mirrored onto the heart transfer**
  (`set_transfer_gf_hearts`, **on by default**). After `1NT–2♦–2♥`, responder shows a
  five-heart-plus-minor game force (`3♣`/`3♦`), a six-heart splinter (`3♠` short in
  spades — cheap, below `4♥` — or `4♣`/`4♦` short in a minor), or a single-suited
  quantitative slam invite (`4NT`, relocated off the `3♠` slam try, just as spades
  relocated off `3♥`). The node is tighter than spades — `2♠` and `2NT` are the
  single-suited/`5♥4♠` invites — so the 5-5 majors slam try keeps its single home on the
  spade transfer rather than duplicating a heart slot. Against BBA (960k boards, two
  seeds) the mirror gains **+0.0015/+0.0017 IMPs/board plain and +0.0016/+0.0018
  par-doubled** (all 95% CI ±0.0003, +1.83/+2.08 per fired). No effect unless
  `set_transfer_gf_majors` is also on; disable with `set_transfer_gf_hearts(false)` (or
  `bba-gen --no-ns-transfer-gf-hearts`).
- **A Texas + responder-RKCB slam drive for six-card majors**
  (`set_texas_slam_drive`, **on by default**). The direct `1NT–4♥/4♠` was a
  *non-forcing* slam try — opener moved only with a maximum, else passed the major
  game — which stranded the strong responder: a 16+ six-card-major hand opposite a
  *minimum* 1NT (the majority) held a cold slam the opener vetoed by passing, while
  the reference bidder reached it via a Texas transfer and keycards. Now the direct
  `4♥/4♠` is capped at the bare-15 invitational cusp, and a 16+ hand instead
  South-African-Texas-transfers (`4♣/4♦`) and, over opener's completion, drives its
  own RKCB (`4NT`); the 1430 ladder places the slam regardless of opener's minimum.
  A double-dummy survey localised this as the mirror of the Jacoby leak (the direct
  `4♥/4♠` continuation cost ≈−6 IMPs/board, all stranded slams). A paired on/off
  A/B (320 000 boards, shared seed, vs the BBA reference) measured **plain +0.0024
  IMPs/board (95% CI ±0.0006), PD +0.0024 — +5.87 IMPs/fired in both regimes** (131
  fired, 0.04%), every CI excluding 0. Disable with `set_texas_slam_drive(false)`
  (or `bba-gen --no-ns-texas-slam-drive`).
- **A slam try after a Jacoby transfer** (`set_transfer_slam_try`, **on by
  default**). Once a transfer completes (`1NT–2♦–2♥` / `1NT–2♥–2♠`), a
  single-suited five-card major with 16+ HCP bids the *other* major (`3♠` / `3♥`,
  artificial) to agree the transfer suit and try for slam; opener launches RKCB
  with a maximum (`4NT`) or signs off in the major game, and the existing 1430
  ladder places the contract. *Why:* the transfer path had **no** slam machinery —
  a strong balanced five-card-major responder transferred and then rested in `3NT`
  while a major slam was cold. A double-dummy survey localised the bulk of our
  1NT-opening deficit vs BBA to exactly this: of the 50 worst `1NT–2♦/2♥` boards we
  played `3NT` on 43 while BBA reached a slam on 34. A paired on/off A/B (320 000
  boards, shared seed, vs the BBA reference) measured **plain +0.0012 IMPs/board
  (95% CI ±0.0004), PD +0.0012 — +1.42 IMPs/fired in both regimes** (275 fired,
  0.09%), every CI excluding 0. Mirrors the Stayman `3OM` slam try, which already
  existed. Disable with `set_transfer_slam_try(false)` (or `bba-gen
  --no-ns-transfer-slam-try`).

### Changed

- **A flat 4-3-3-3 eight now passes `1NT` instead of inviting.** The invitational
  eight-count with a `(4333)` shape used to bid the `2♠` size ask (Puppet scheme) or
  `2NT` (European) like any other balanced eight; it now passes. A flat 4-3-3-3 has
  no ruff and no long suit — its tricks *are* its high cards — so it plays a level too
  high opposite a 15-17: `2NT` fails opposite a minimum where `1NT` would have held,
  and `3NT` opposite a maximum still lacks a trick source. A double-dummy probe
  (`examples/probe-uninvite-4333`, 16M deals, uncontested) prices passing over the
  invite at **+0.638 IMPs/board** for the whole class (95% CI ±0.043, 21165 boards),
  rising monotonically as card quality drops — **+0.720** with no ace, **+0.990** with
  no ten, **+1.083** for the pure-quack no-ace-no-ten eight — because a flat hand's
  tricks are exactly the cards HCP over-credits (the quacks) and under-credits (aces,
  tens that cash). Even the ace-holding eights gain (≈+0.585), so no card-quality gate
  is needed. The game-forcing **nine** is unchanged: the same probe found blanket-
  inviting the flat nine *loses* −0.334 IMPs/board (forcing `3NT` is right in
  aggregate); only the quack tail (no ten) leans toward inviting (+0.28), too small and
  gate-heavy to act on. The analytic invite the probe prices was validated to equal the
  system's own auction on all 21165 eights before the switch (0/21165 mismatch).
- **Opener now corrects a choice-of-games `3NT` to `4M`, gated on the ruff**
  (`set_correct_3nt_to_major`, flipped **on by default**). After a Jacoby transfer,
  responder's balanced game force offers a choice of games with `3NT` (now an authored
  node on both majors — exactly five in the transferred major, denies a four-card side
  suit, below the 16+ `4NT` quant), so opposite three-card support opener holds an
  eight-card fit. The pull was previously **off** (**−0.037 IMPs/board**): keyed on the
  bare `3NT`, it fired in *any* auction — walking into penalty doubles in contested
  sequences and pulling flat 4-3-3-3 hands with no ruffing value. Two gates fix it —
  `undisturbed` (a `3NT` reads as *balanced* only undisturbed; contested it walks into a
  double) and a **ruffing doubleton** (a flat 4-3-3-3 opener has no ruff, so `3NT` keeps
  its ninth trick against `4M`'s tenth). Gated, against BBA (1.6M boards/arm, two seeds)
  it wins **+0.0062 IMPs/board plain and +0.0068 par-doubled** (both 95% CI ±0.0005,
  +2.0/+2.2 per fired, 0.31% fired). Disable with `set_correct_3nt_to_major(false)` (or
  `bba-gen --no-ns-correct-3nt-to-major`).
- **A strong-1NT responder now forces game with 9 HCP in an undisturbed auction**
  (`set_nt_responder_game_floor`, **default 9**, was 10). *Why:* the authored
  direct-3NT response already forces on `hcp(9..)`, but a 9-count holding a
  *single five-card major* cannot bid it — it must Jacoby-transfer, and after the
  transfer completes (`1NT–2♦–2♥` / `1NT–2♥–2♠`) the authored rebid table only
  covers the exactly-8 invite, so the game force fell to the instinct floor, whose
  trigger was 10. The 9-count therefore stalled in a partscore and missed game.
  Lowering the floor to 9 closes the seam; gated on an *undisturbed* auction
  because a double-dummy A/B (`bba-gen --filter-1nt`, 204.8k boards/arm vs BBA)
  showed forcing a thin 9 over a suit *overcall* loses (the enemy lead/shape beats
  the thin 3NT). The undisturbed change measured **plain +0.0048 IMPs/board (95%
  CI ±0.0020), PD wash**.
- **Responder no longer pulls to 3NT over a double of our 1NT** — the unlimited
  business redouble (`1NT–(X)–XX`, "we make it") or a long-suit escape governs
  instead (`set_suppress_nt_game_force_over_double`, **on by default**). *Why:* the
  floor's natural-3NT game force was firing over a penalty double of our 1NT for
  every strong balanced hand, bypassing the redouble; a paired A/B isolated this
  case at **+5.6 IMPs/fired in both plain and PD** (rare, ~0.03%) — XX is strictly
  superior. Over a suit *overcall* responder still bids game as usual (no redouble
  available, the opponents are not penalizing).
- **Opener can correct a choice-of-games `3NT` to `4M` with a known eight-card
  major fit** (`set_correct_3nt_to_major`, **opt-in, off by default**). After a
  transfer responder's five-card major is already shown, so a follow-up `3NT`
  (in any continuation, contested or not) is a choice of games that opener could
  pass or correct to the 5-3 fit. *Why off:* the ruffing-doubleton edge is
  single-dummy lore — a double-dummy A/B (204.8k boards/arm vs BBA) measured the
  correction at **−0.037 IMPs/board in both plain and PD** (CI excludes 0),
  because perfect play cashes the ninth trick in `3NT` on finesses and squeezes
  that `4M` cannot turn into a tenth. Kept as a knob for single-dummy play.
- **Auctions now display pass as `-` instead of `P`** so `X` (double) and `XX`
  (redouble) stand out when scanning a bidding sequence. This rides on
  `contract-bridge` 0.1.4 — the new `auction::display_calls` slice adapter and
  `Auction`'s `Display`/serde now emit `-`; the `render-book` and `bba-score`
  example renderers route through it. A lone `Call` and per-call serde arrays
  (e.g. the `dump-corpus` `["1H","P"]`) still use `P`. Older auction strings/JSON
  written with `P` still parse.
- **South African Texas game floor lowered and re-gauged on `point_count + trump
  length`** (`set_texas_game_floor`, **default 14**, was a raw-HCP floor of 9). A
  6-card-major responder opposite our 1NT now blasts game (`4♣/4♦`) when its point
  count plus suit length reaches 14 (a 6-bagger at 8 points, a 7-bagger at 7)
  instead of needing ~9 HCP and otherwise transferring to a partscore. *Why:* the
  old 9-HCP floor was inherited
  verbatim from the pre-Texas transfer-then-game route and never measured at its
  lower edge, leaving the invitational 7-8 HCP one-suiters stranded in 2M. A
  double-dummy screen (`examples/probe-jacoby-invite-eval`) established three
  things: (1) opposite a known 6-2+ fit opener should **never decline** an invite
  (a 6-card suit is too much playing strength), so the "transfer then invite"
  structure degenerates to a blast; (2) the would-be `3M` invite landing is a
  *worse* contract than `2M` at every strength — these one-suiters make 8 or 10
  tricks, rarely 9 — so the decision is binary, pass-`2M` or blast-`4M`, with no
  invitational band; and (3) at this *fit-rich* boundary distribution is a real
  trick (the 6th trump, ruffs), so a fit-adjusted gauge ranks the game hands
  better than raw HCP — `fit_value` > CCCC > points > HCP — unlike the no-fit
  invite line and the slam edge where honors dominate. Paired A/Bs vs BBA (1.024M
  boards/arm, `--filter-1nt`): `fit_value≥10` over the old HCP-9 baseline measured
  **plain +0.0102 IMPs/board vul none, +0.0171 both; PD +0.0082 / +0.0141**
  (+2.4…+5.0 IMPs/fired), and over an interim raw-HCP≥7 floor of the same
  aggressiveness **plain +0.0013 / +0.0018; PD +0.0014 / +0.0019** — every regime
  a win, all 95% CIs exclude 0. *Impact:* a bare-7 6-card major (and a shapely
  6-count, e.g. 6-4) now reaches game where it previously sold out to a partscore;
  the direct `4♥/4♠` slam try (15-18) and every other 1NT response are unchanged.
  No competition nodes were needed — the blast is a single jump to game, so there
  is no transfer-completion auction for the opponents to contest.
- **Tightened dependency lower bounds to their real minimums** so a
  minimal-version build resolves and compiles: `bitflags` `2` → `2.2.1`, `serde`
  `1` → `1.0.225` (the `serde_core` facade split, needed by transitive
  `serde_with`), and the dev-deps `anyhow` `1` → `1.0.98`, `clap` `4` → `4.5`,
  `serde_json` `1` → `1.0.100`. The bare-major bounds were below what the code
  actually needs; `cargo +nightly update -Z direct-minimal-versions` previously
  failed to resolve (or built deps that no longer compile, e.g. `anyhow` 1.0.0).
  A new `minimal-versions` CI job now guards this. *Impact:* none for normal
  builds (the committed lockfile already pins newer versions); downstream
  consumers using minimal-version selection are no longer broken.

### Added

- **Gambling games over a double of our 1NT** (`set_gambling_3nt_over_double` /
  `set_preempt_4m_over_double`, **both opt-in, off by default**). Over a penalty
  double of our 1NT, responder may bid the direct games as long-suit hands rather
  than balanced game forces. A gambling game is a six-plus suit that is **semi-solid
  and headed by its own ace** — the suit ace cashes and buffs total tricks — opposite
  the strong notrump: a long **minor** bids `3NT` (run the suit), a long **major**
  bids `4♥`/`4♠` (the trump ace is a sure trick). The strong *balanced* hands these
  carve out of `3NT` still defend the business redouble (XX); the `3NT` is alerted so
  the inference reader decodes the long-minor run, not a balanced 3NT. Knobs:
  `set_gambling_3nt_top_honors` / `set_preempt_4m_top_honors` (the semi-solid floor,
  default 2 of A/K/Q; `0` = length only), `set_gambling_3nt_require_ace` /
  `set_preempt_4m_require_ace` (the suit/trump ace, default on), `set_preempt_4m_floor`
  (the 4M HCP floor, default 5). The undisturbed and over-an-overcall `3NT`/`4M` are
  unchanged; the same `--ns-gambling-3nt` / `--ns-preempt-4m` family is exposed on
  `bba-gen` for a vs-BBA cross-check.

  *Why off:* this is the gambling-3NT-over-(X) nuance deferred when the suppress
  default shipped. A self-play seat-swap A/B (`examples/ab-one-nt-runout --compare …
  --filter-1nt`, 2M boards/arm, one fresh seed, scored both plain DD and
  perfect-defense, reported per divergent board) verified the structure but found the
  net double-dummy-negative:
  - **XX catches all strong balanced** — confirmed by `--coverage`: of 3160 strong
    (7+ HCP) balanced responders, **100% redoubled, 0 gambled** (a long minor is
    structurally impossible in a balanced hand).
  - **Suit quality, not raw length, is load-bearing** — a length-only `3NT` is a
    disaster (−4.4 plain / −9.9 PD), but the semi-solid gate flips it to a clear win
    over length-only (`gambling-semisolid` **+4.6 plain / +10.3 PD**).
  - **The crucial ace is the *suit/trump* ace, not an outside ace** — requiring it
    (`gambling-ace`, semi-solid + suit ace vs semi-solid alone) measured
    **+2.9 plain / +7.6 PD** (it rejects the KQ-headed-no-ace gambles); the
    outside-ace variant diverged on only 4/2M boards (the wrong card).
  - **Net, even done right, the gamble does not beat the redouble** — `3NT` alone vs
    suppress was −0.6 plain / −4.0 PD, the quality `4M` −0.5 plain / −5.3 PD, the
    whole package −0.6 plain / −4.4 PD. Opposite a 15–17 opener, `1NT` redoubled is
    such a strong spot (it usually makes for a big score) that giving it up to gamble
    a game rarely gains, and double-dummy is **blind to the obstruction value** the
    "partly preemptive" framing rests on. The self-play A/B faces our own *penalty*
    double (the worst case — the gamble surrenders a making `1NTxx`); against a
    *conventional* takeout / two-suiter double (the opponents are not penalizing and
    will not defend `1NT`, e.g. BBA's Woolsey) there is no redouble to give up and the
    jump denies them their two-suiter auction — the regime where the gamble is
    expected to pay. Authored as opt-in knobs for single-dummy / matchpoint play and
    competition against a conventional double, with the gates the A/B validated.
- **Six-card-major game invite** (`set_sixcard_invite_floor` /
  `set_sixcard_accept_floor`, **default on**). With a 6-card major just *below* the
  Texas blast floor, responder transfers (`2♦`/`2♥`) and jumps to `3M` — a natural
  game invite — and opener accepts `4M` on `point_count + trump length ≥ 18` (a flat
  15 with a doubleton declines, a 15 with three-card support or any 16+ accepts) or
  passes `3M`. The invitational band is `[13, 14)`; raise
  `set_sixcard_invite_floor` to the blast floor (14) to empty the band and turn the
  invite off. This is the invitational band the Texas change above deliberately left
  out, restored as standard major-suit bidding. A paired A/B vs BBA (1.536M
  boards/arm, `--filter-1nt`, floor 13, accept floor 18; 1607 fired, 0.10%) measured
  **plain +0.619 IMPs/fired vul none, +1.820 both** (95% CIs exclude 0) and **PD
  −0.211 / +0.561**: perfect-defense doubling trims the vul-none edge (the 3-level
  tax — the decline branch rests in `3M`), but a 6-card-fit `3M` partscore is not
  realistically doubled into a penalty at IMPs, so the PD-none figure overstates the
  downside. Double-dummy can't price the invite's real edge anyway — the `3M` brake
  on the thin games real defenders beat (`examples/probe-jacoby-invite-eval`
  experiment I shows DD sees only +0.09 IMP/bd over a blast) — so the conventional
  invite is kept on.
- **`probe-jacoby-invite-eval` diagnostic example** — the double-dummy screen
  behind the Texas floor change above. Buckets 6-card-major responders opposite a
  15-17 1NT and reports: the responder pass/invite/blast cut by HCP (R/R2), the
  opener accept-vs-decline sweep (O, which degenerates to "always accept"), the
  floor-evaluator comparison (F/G — `fit_value` > CCCC > points > raw HCP at
  the 2M/4M boundary, +0.11/+0.18 IMPs/board over HCP at a matched blast rate), and
  the realistic-invite-vs-binary head-to-head (I).
- **Single-suited 5-card-major invite completed for spades** (rides
  `set_invitational_5card_majors`, **default on**). A responder with *exactly* five
  spades, no four-card heart suit, and a bare-8 invitation now transfers (`2♥`) and
  rebids `2NT` (`1NT–2♥–2♠–2NT`) — the spade mirror of the heart single-suiter's `2♠`
  relay, using the free `2NT` step (5♠4♥ Staymans, so `2NT` is not needed for the 5-4
  invite). Opener places the contract by strength and fit: a maximum bids `4♠` with
  three-card support or `3NT` with a doubleton; a minimum rests in `3♠` (the 5-3 fit) or
  passes `2NT`. This fills the one gap in the 5-card-major invite structure — 5♥
  single-suited, 5♥4♠, and 5♠4♥ were already authored; the 5♠ single-suiter alone fell
  to the floor. A double-dummy screen (`examples/probe-fivecard-invite-eval`) confirmed
  the opener table and, notably, that the 5-3 fit out-scores `3NT` even opposite a flat
  4-3-3-3 maximum (**+1.4/+1.6 IMPs** on those boards, 95% CIs exclude 0) — responder's
  5-3-3-2 always brings a ruffing doubleton — so, unlike the 4-4 Stayman acceptance,
  there is **no** flat-4333→`3NT` carve (the existing heart single-suiter, correctly, has
  none either). A paired A/B vs BBA (two binaries, HEAD with vs without the node, same
  seed, 4.096M boards/arm, `--filter-1nt`; 11880 fired, 0.29%) measured **plain +0.0006
  IMPs/board vul none, +0.0021 both** (95% CIs exclude 0; +0.221 / +0.734 per fired) and
  **PD −0.0002 / +0.0007**: the tiny PD-none dip is the standard constructive game-invite
  artifact (perfect-defense doubling the thin invited games — the same shape as the
  six-card invite, but milder, since the rest is a 1NT-level or `3♠` partscore), and a
  24-26-HCP game is not realistically doubled at IMPs, so the convention stays on.
  *Impact:* a bare-8 five-spade one-suiter now invites precisely instead of guessing
  between a floored 2NT and an over-bid 3NT; `set_invitational_5card_majors(false)` turns
  the whole 5-card structure off. The bid is alerted (`INV_5CARD`); every other 1NT
  response is unchanged.
- **`probe-fivecard-invite-eval` diagnostic example** — the double-dummy screen behind
  the spade single-suiter's opener table. Buckets exactly-5 single-suited major
  responders opposite a 15-17 1NT and reports opener's rebid by strength / support /
  shape (O — settling the flat-4333 question above) and the responder invite band by
  HCP (R).

### Fixed

- **Fallback-authored conventions are now decoded under second-round
  intervention** (`set_fallback_projection`, **on by default**). The floor's
  projection (`project_authored`) read meaning off a prior call only when it was
  authored at an *exact* trie node (via `common_prefixes`). But every contested
  convention — Transfer-Lebensohl transfers, Leaping Michaels, the Lebensohl cue —
  is authored as a *guarded fallback*, which `common_prefixes` walks straight past.
  So once the opponents bid again over an artificial call (e.g. `1NT-(2♠)-3♦`→♥
  followed by `(3♠)`), the floor lost the convention's meaning, read the call as a
  *natural* long suit, and raised the phantom suit into a doubled contract (the
  BBA-match tail had `1NT-(2♠)-3♦-(3♠)-5♦x` catastrophes). `project_authored` now
  re-resolves each prior call's *authoring* classifier through the same
  node-then-fallback chain that bid it (`Trie::authoring_classifier`), so any
  alerted convention is decoded the same way it was made. This is the general
  replacement for the per-convention hand readers; the structural
  `transfer_lebensohl_reading` band-aid is retired in favor of it. BBA A/B (paired,
  1.6M `--filter-1nt`, on vs `--no-ns-fallback-projection`): plain **+0.0014
  IMPs/board** (+2.0/fired), PD **+0.0022** (+3.2/fired), both 95% CIs exclude 0;
  small per-board because rare (0.07% fired). The OR-disjunction two-suiters (DONT,
  Woolsey, Multi) and doubles still need their hand readers — their union projects
  no single suit.

### Added

- **Three Stayman (1NT-2♣) treatments, measured with paired same-deal A/B vs BBA
  (vul none).** Each is a rarely firing subset of the 1NT-2♣ subtree; all three
  are individually DD-positive on *both* plain and perfect-defense (so not a
  doubling artifact), and they are partial **substitutes** — a treatment's
  per-fired value softens (but stays clearly positive) when another is on, as they
  fight over the same both-major hands. **All three ship on by default.**
  - **Garbage (drop-dead) Stayman** (`set_garbage_stayman`, **default on**;
    off-switch `bba-gen --no-ns-garbage-stayman`): a *weak* responder (below the
    constructive 8-HCP floor) with short clubs and a four-card major bids 2♣ to
    escape a likely-doomed 1NT, passing opener's 2♦/2♥/2♠. Two tiers, looser the
    weaker responder is — a broke 1NT rates to be a disaster, so any ~7-card fit
    is an improvement: the 0-4 tier accepts a thin 2♦ landing (3+ diamonds), the
    5-7 tier insists on 4+ diamonds. Responder's pass is the floor default (no
    authoring). The inference engine no longer assumes 2♣ shows 8+ when this is
    on, so the floor doesn't misjudge a weak escaper. **+0.51 IMPs/fired plain
    (+0.0009/board, 95% CI ±0.0005, excl 0), +0.70 PD**; fires 0.17% of boards.
  - **Opener shows a maximum five-card major** (`set_stayman_5card_max`,
    **default on**; off-switch `bba-gen --no-ns-stayman-5card-max`): over 1NT-2♣
    holding a five-card major and a maximum, jump `3♥`/`3♠` (our balanced 1NT
    *can* hold a five-card major — a 5332 is balanced and outranks the
    one-of-a-major opening on weight), showing the 5-3/5-4 fit plus extras so
    responder drives game/slam; a minimum bids `2♥`/`2♠` naturally. The jump names
    and shows its own suit, so it stays natural (unalerted). The cleanest of the
    three: **+3.45 IMPs/fired plain (+0.0007/board, 95% CI ±0.0004, excl 0),
    +3.33 PD**, holding up at +1.47/+0.90 even with garbage on; fires 0.02%.
  - **Opener shows both four-card majors with a max-only right-siding relay**
    (`set_stayman_both_majors`, **default on**; off-switch
    `bba-gen --no-ns-stayman-both-majors`): over 1NT-2♣ holding both majors and a
    *maximum* (16-17), jump to `2NT`; a minimum (15) bids `2♥` naturally.
    Responder then names *their own longer* major — `3♣` = hearts, `3♦` = spades
    (hearts on a 4-4 tie, the lower major leaving room to escape a double) — and
    opener completes (`3♥`/`3♠`), so the strong concealed hand declares the known
    fit (right-siding) instead of responder declaring after a direct raise. Responder always holds a four-card major
    (they bid Stayman), so the two relays tile every hand and need no catch-all.
    Responder then places game: because opener's maximum and the major fit are
    both known, the invite is pre-accepted, so game goes when
    `point_count + extra trumps + a fit in the other major ≥ 8` — shape counts now
    the trump suit is agreed, a fifth trump (the 9-card fit) adds a point, and four
    in the unnamed major is a *knowable* second 4-4 fit (opener showed both) worth
    another — else responder passes the partscore. The `2NT`, the `3♣`/`3♦`
    relays, and the `3♥`/`3♠` completions
    are alerted (artificial) and auto-decoded by `project_authored`. The
    right-siding is the win the earlier strength-step scheme lacked — it is
    **DD-positive in every regime**: **+2.18 IMPs/fired plain (+0.0035/board, 95%
    CI ±0.0007, excl 0), +2.29 PD with garbage on** and +2.68/+2.87 with garbage
    off (320 000 boards/arm). Replaces the earlier strength-step scheme (`2NT`/`3♣`/`3♦`
    = 15/16/17), which lost to garbage (−0.37/−0.91) because responder declared.
    Known gap: with no `4NT`-quant over the `2NT` relay, a big responding hand
    signs off in game where the natural `2♥` auction could reach a quantitative
    slam (the `-11 IMP` tail) — a future add.
- **Invitational 5-4 in the majors after 1NT** (`set_invitational_5card_majors`,
  **on by default**; off-switch `bba-gen --no-ns-invitational-5card-majors`): a
  deliberate, asymmetric structure for the one purely-invitational both-majors band
  (a bare 8, since we force every 9 to game and 5-5 already has the `3♦` relay).
  Previously every invitational 5-4 transferred to its five-card major and buried
  the four-card one (the transfer-completion rebid was not even authored — the
  floor invited). Paired same-deal A/B vs BBA (1.28M boards/arm, `--filter-1nt`,
  vul none, 0.53% fired): **plain +0.375 IMPs/fired (+0.0020/board, 95% CI ±0.0004)
  and PD +0.134/fired (+0.0007/board, 95% CI ±0.0005)**, both excluding 0 (a 640k
  pilot agreed: +0.0022 / +0.0008).
  Reaching that took a companion fix to a contested gap the reroute exposed (below):
  a first cut without it was plain +0.0013 but a PD wash (−0.0001), because ~21 of
  the fired boards walked a 5♠4♥ hand into a **doubled artificial 2♦ it passed out**
  — the same doubled-passout pattern that sank "335" garbage Stayman, below.
  Gated:
  - **5♠4♥** keeps off the spade transfer (a 6+♠ suit and the weaker 5♠4♥ still
    transfer) and bids Stayman, rebidding `2♠` over opener's `2♦` (non-forcing,
    opener passes the partscore or accepts game) or over `2♥` (forcing,
    invitational through slam — opener picks ♥/♠ and the level). With 5♠4♥ now
    routed to `2♠`, the existing `1NT-2♣-2♥-3♠` slam try is capped to **deny five
    spades**.
  - **5♥4♠** transfers to hearts and, after the completion, rebids `2NT` (showing
    the four spades) or an artificial `2♠` (a single-suited heart invite denying
    them) — a Muppet-style swap (2NT shows the side suit, the cheaper relay denies
    it) brought down to the two-level over 1NT. Both rebids are alerted and
    auto-decoded by `project_authored`. Game-forcing 5-4 still uses Smolen.
  - **Doubled-2♦ escape** (general — in `competition.rs`, gated by
    `competition_over_stayman`, so flag-independent and present for the baseline
    too). When an opponent doubles opener's *artificial* Stayman answer
    (`1NT-2♣-(2♦/2♥/2♠)-(X)`), responder's rebids are now systems-on: strip the `X`
    to a Pass and re-key onto the uncontested tree, so the 5♠4♥ runs to its real
    `2♠` and a 4-4 invite to `2NT` instead of passing the artificial bid out
    doubled. Alert-reading decodes the 2♦ for *inference*; it does not make the
    keyless floor *escape*, which is why the authored rebase was needed.
- **Crawling Stayman** (`set_crawling_stayman`, **on by default**; off-switch
  `bba-gen --no-ns-crawling-stayman`): the strict superset of garbage Stayman for
  the 4-4-majors hands garbage *cannot* escape — those short in diamonds
  (`4=4=1=4` / `4=4=0=5`). Garbage needs a safe `2♦` landing (3+ diamonds), so a
  singleton/void diamond would sit doubled in a misfit; crawling bids `2♣` anyway
  and, over opener's `2♦` denial, *crawls* to `2♥` — both majors, pass-or-correct.
  Opener passes with three hearts (a 4-3 fit), corrects to `2♠` with only two
  (then ≥3 spades), or — short in *both* majors, which only a five-card-minor 1NT
  can be (the 5422-minor open) — flees to `3♣`, an 8-9 card club fit since a
  short-diamond 4-4 is necessarily club-heavy. The `2♣` is the existing STAYMAN
  alert; the `2♥` crawl floors only hearts (responder's spades is implied by the
  crawling `2♣`, like the invitational `2♠` sibling) so it stays unalerted
  natural. The doubled tail `1NT-2♣-2♦-(X)-2♥` is handled for free by the
  systems-on rebase above (the `X` is stripped and the auction re-keyed onto the
  uncontested crawl). The inference engine no longer assumes `2♣` shows 8+ when
  this is on. Paired same-deal A/B vs BBA (1.6M boards/arm, `--filter-1nt`, vul
  none, on vs `--no-ns-crawling-stayman`, 0.10% fired): **plain +1.539 IMPs/fired
  (+0.0015/board, 95% CI ±0.0002) and PD +2.055/fired (+0.0021/board, 95% CI
  ±0.0003)**, both excluding 0 — and PD *exceeds* plain, so the win is not a
  doubling artifact.
- **Tried and dropped: looser ("335") garbage Stayman.** A broke (0-4) hand with
  no four-card major but five diamonds and `3=3` majors escaping `1NT` via `2♣`
  (sound drop-dead — a 4-3 major or long-diamond fit on every answer, no pull
  needed). Measured net-negative for the same reason as above: the weak `2♣` is
  **doubled and passed out** (`1NT P 2♣ X P P P`, a short-club `2♣x`), **−0.495
  IMPs/fired plain, −0.631 PD** (320 000 boards, vul none). The loss is the
  `(2♣)-X` tail, not the escape; with no near-term runout to author, the variant
  was removed rather than kept as a dead knob. A runout over `(2♣)-X` is the
  prerequisite to revisit it.
- **`examples/probe-nt-invite-eval` — does any hand evaluator beat raw HCP at the
  1NT-response invite/force boundary, per responder shape?** A double-dummy screen
  that deals boundary-band responders (7–10 HCP) opposite a 15–17 balanced opener,
  splits them into the two shape classes that actually route differently —
  **Stayman** (a four-card major, via `2♣`) and **no-4-major** (the `2♠`-invite vs
  `3♣`-Puppet hand; a balanced hand with no four-card major always has a three-card
  major) — and scores each action (pass `1NT` / invite / force) by DD, with the
  misfit→notrump branch modelled. Each candidate evaluator (`points`, `fifths`,
  `bumrap`, `cccc`, `controls`) is **rank-calibrated** to HCP's exact action
  frequencies, so the IMP delta is a pure ranking test, not an aggression change.
  Result (50 000 deals/class, both vulnerabilities): **raw HCP wins per shape
  class** — every other evaluator is a wash or significantly worse. Suit/control
  evaluators are markedly worse (`controls` −0.43 to −0.75 IMPs/board, `cccc` −0.06
  to −0.23); `fifths`/`bumrap` are mildly-to-significantly worse (`fifths` −0.01 to
  −0.04); `points` is a wash (it equals HCP on the balanced bulk, ≈0 on the shapely
  Stayman hands). The faint "fifths helps the notrump hands" hint at small samples
  **flipped to negative** at scale, refuting a shape-dependent evaluator. This
  *confirms and extends* the existing note in `notrump.rs` ("deciding the 9 by Fifths
  was measured worse"): HCP is best per-class, so the aggregate test was not masking
  a hidden shape effect. No system change — the boundary stays on raw HCP.

- **`set_competitive_4333(Competitive4333)` toggles the flat-4333 rule in
  competition** (`Allow` / `Suppress` / `SuppressWithStopper`; default `Suppress`).
  Gates whether a flat 4-3-3-3 still cue-Staymans when our 1NT is overcalled, or
  diverts to 3NT — the competitive arm of the constructive 4333 change (see
  *Changed*), with `--ns-competitive-4333` wired into `bba-gen` for the A/B.
  `SuppressWithStopper` (suppress only with a stopper in their suit) and `Allow` (the
  old behaviour) stay selectable for re-measurement.

- **The Michaels cue-bid is now alerted** (`Alert("michaels")`). First increment
  of retiring the structural `artificial()` decode heuristic in favor of
  alert-by-disclosed-meaning: a call is read as conventional because its authoring
  rule carries an `.alert(...)`, not because its projection happens to floor a
  foreign suit. Disclosure-only and a **decode no-op** — while `artificial()`
  remains in the gate it already kept these calls, so the floor reads them
  identically; only the alert output changes. Shrinks the retirement worklist
  (`artificial_calls_are_alerted`, ignored) from 172 to 156. No measurement: the
  behavior change is deferred to the final increment that drops `artificial()`.

- **The unusual 2NT overcall is now alerted** (`Alert("unusual-2nt")`). Second
  increment of the `artificial()` retirement: the `[1♦] 2NT` / `[1♠] 2NT`
  two-lowest-unbid-suits overcall (the suit-defense unusual notrump, distinct
  from the already-alerted defense to a 1NT opening) now carries its own alert.
  Same disclosure-only **decode no-op** as the Michaels increment; shrinks the
  worklist from 156 to 140.

- **The Leaping Michaels overcall is now alerted** (`Alert("leaping-michaels")`).
  Third increment of the `artificial()` retirement: the 4♣/4♦ jumps over a weak
  two (`[2♥] 4♣`/`4♦`, `[2♦] 4♦`, …) — the 5-5 game-forcing two-suiter authored
  in `defense_to_weak_two` behind `leaping_michaels_enabled()` — now carry an
  alert (distinct from the responder-side `comp:leaping-michaels`). Same
  disclosure-only **decode no-op**; shrinks the worklist from 140 to 120.

- **Responsive doubles are now alerted, and `artificial()` is bid-only — the
  Pass/Double half of the `artificial()` retirement, with a policy correction.**
  The structural witness `artificial()` flags any call whose projection floors a
  suit it did not name; for a pass or double — which name no suit — that flagged the
  natural *defend-it* calls, because a trap pass or penalty double floors the
  *opponents'* suit precisely when it wants to defend the contract on the table. The
  artificial call there is the **takeout double** (it asks partner to pick a suit),
  not the pass that sits for it.
  - The responsive double (`[1♦ X 2♦] X` / `[1♦ X 3♦] X`, both the takeout-double and
    overcall variants) now carries `Alert("responsive-double")` — honest disclosure
    that it is artificial (a takeout call asking partner to pick a suit). An exact
    **no-op for inference**: the floor's read is identical before and after (the
    structural `artificial()` and the alert pass the same projection gate).
  - `artificial()` is now **bid-only** (returns `false` for Pass/Double). Passes and
    doubles are decoded by alert alone — the artificial doubles by their alert
    (responsive here), the defend-it passes by the settle floor ("pass = play the top
    bid"), the penalty doubles by their existing post-walk readers
    (`penalty_x_reading` / `penalty_latch_double_reading`). This **naturalizes the
    trap pass** (`[1♦ X P] P`) — the one real behavior change, deferred to the
    final-drop measurement (watch those boards and the `1NT-(2M)-P-(P)` reopening).
  - Shrinks the worklist from 120 to **68**; all remaining counterexamples are bids
    (#6 transfers over 2NT, #7 puppet / two-way relay).

- **`artificial()` retired — alerts now carry the decode signal exhaustively.**
  Final increments of the retirement. The last 68 counterexamples were swept:
  - *#6 — transfers over a 2NT-strength notrump* (`[2NT] 3♦/3♥`, and the same after
    a `2♣–2x–2NT` rebid) now carry `Alert("jacoby-transfer")`. The 3♣ Stayman beside
    them is an OR-disjunction the structural witness never flagged, so it is left
    natural — alerting **exactly** the structural set keeps the drop a true no-op.
  - *#7 — puppet / two-way-relay continuations*: the club splinters under the 2♠
    two-way relay (`[1NT 2♠ 2NT] 3♦/3♥/3♠`, `[1NT 2♠ 3♣] 3♦/3♥/3♠`) carry
    `Alert("splinter")`, the slamless 6-card-club `3NT` carries `Alert("puppet")`,
    the Stayman 3-other-major artificial slam try (`[1NT 2♣ 2♥] 3♠`, `[1NT 2♣ 2♠]
    3♥`) carries `Alert("slam-try")`, and the Puppet-Stayman 4-4 hunt
    (`[1NT 3♣ 3♦] 3♥/3♠`) carries `Alert("smolen")`.
  - **The drop:** with the worklist at zero, `|| artificial(p, made)` is removed
    from the decode gate (`project_authored`) — alerts alone decide whether a call
    is read as conventional. This is a **provable bit-identical no-op** under the
    default `set_alert_reading(true)`, not merely a statistical wash: the gate
    projects the *union* of a node's rules for a call, and whenever that union
    floors a non-named suit (the structural witness) **every** matching rule floors
    it too, so every matching rule is alerted (the now-permanent
    `artificial_calls_are_alerted` test proves the book has no unalerted
    artificial bid), making the `alerted` term already true. Old gate
    (`alerted || artificial`) and new gate (`alerted`) therefore fire on exactly
    the same calls — no BBA measurement needed, the invariant test is the proof.
    The `artificial` detector survives `#[cfg(test)]`-only as that invariant guard,
    so a future artificial bid added without an `.alert(...)` fails the test rather
    than silently losing its decoding. (With `set_alert_reading(false)` these calls
    now go undecoded, where the structural fallback formerly caught them regardless;
    `alert_reading` stays the master switch.)

- **Competition over our 2NT diamond transfer, both sides — authored and
  A/B-measured vs BBA (both opt-in).** The PUPPET 2NT diamond transfer (6+♦, or 5♦-4♣) and its
  fourth-hand contest previously fell through to the instinct floor; both sides are
  now authored behind toggles, modeled on the 2♠ minor / contested-Stayman
  machinery. No-ops under the EUROPEAN scheme (where 2NT is the balanced size-ask).
  - *Our 2NT is contested* (`set_competition_over_diamond_transfer`, **on by default**,
    off-switch `bba-gen --no-ns-comp-over-diamond-transfer`). Their `(X)` is lead-directing
    diamonds; the double frees `Pass` to be the catch-all "no fit" call, which lets
    opener's `3♣` shed its uncontested relay-denies-a-fit meaning and become
    **natural** 4+♣ (landing responder's 5♦-4♣ in the club fit): `3♦` = accept with
    3+♦, `3♣` = no fit but 4+♣, `XX` = maximum values (no fit, penalty-oriented),
    `Pass` = minimum catch-all. After a fit-showing `3♦`/`3♣` responder's rebids
    match the uncontested tree (strip the `X`); after `Pass`/`XX` responder always
    holds 5+♦ and signs off in `3♦`. A `(3♣)` overcall keeps the `3♦` completion
    legal (else `X` = clubs, Pass = minimum); a higher overcall keeps `3NT` (max +
    stopper) / `X` (their suit) / Pass.
  - *They bid a 2NT diamond transfer* (`set_diamond_transfer_defense`, `bba-gen
    --ns-diamond-transfer-defense`; default off). `X` = lead-directing diamonds (the
    shown suit, not takeout); `3♦` (cueing their diamond anchor) = both majors (5-5,
    Michaels), weighted above the `X`; natural `3♣`/`3♥`/`3♠` six-card one-suiters.
  - A paired A/B vs BBA over 1 000 000 `--filter-1nt` boards (≈400 fired/side, 0.04 %):
    Side A is a plain-DD **wash** (+0.24 IMPs/fired, CI straddling 0) with a clear PD gain
    (+3.40) — it never loses on honest DD and the PD gain is real value when opponents
    punish the floor's `X`-then-pull-to-`3NT` overreach, so it ships **on**. Side B is a
    clear **loss** on both scorers (−1.91 plain, −2.32 PD), the light-sacrifice cost of
    doubling/cueing into a strong-1NT auction, so it stays **opt-in**.

- **Competition over our two-way 2♠ minor response, both sides — authored and
  A/B-measured vs BBA over 640 000 boards.** The PUPPET 2♠ (6+ clubs *or* a balanced
  size-ask) and its fourth-hand contest previously fell through to the instinct
  floor; both sides are now authored, modeled on the contested-Stayman /
  Jacoby-transfer machinery. Both contests are **rare** — BBA seldom contests our
  2♠, and seldom bids a 2♠ minor transfer of its own — so the per-board impact is
  tiny, but the per-fired signal is clear and the two sides split:
  - *Our 2♠ is contested* — **on by default** (`set_competition_over_minor_transfer`,
    off-switch `bba-gen --no-ns-comp-over-minor-transfer`). Because the opponents'
    `X` of 2♠ is lead-directing spades, opener re-encodes its size-ask answer **and**
    a spade stopper across four calls: `2NT` = minimum with a stopper, `3♣` = maximum
    with a stopper (after either, responder's rebids match the uncontested tree),
    `Pass` = minimum no stopper, `XX` = maximum no stopper (after either, responder
    signs off in `3♣` with clubs). A `(2NT)`/`(3♣)` overcall — the bids that steal
    the size-ask steps — keeps the signal alive (`3NT` = maximum + stopper, `X` =
    maximum no stopper, Pass = minimum); any higher overcall is systems-off (`X`
    shows their suit, else Pass). No-ops under the EUROPEAN pure-transfer 2♠ (no
    min/max answer to protect). Like the contested 2♣ Stayman this is a
    **constructive** win: it fired on 0.03 % of boards for **+4.80 IMPs/board it
    fires on** on plain double-dummy (**+5.63 under perfect-defense** — *higher*, so
    a sound contract-finding gain, not a doubling artifact), CI excluding 0.
  - *Their 2♠ is the contest* — **opt-in (default off)** (`set_minor_transfer_defense`,
    `bba-gen --ns-minor-transfer-defense`). Our fourth-hand defense: `X` =
    lead-directing spades (the bid suit, not takeout); `2NT` = the two lowest unbid
    suits (diamonds + hearts, 5-5); `3♣`, cueing their clubs anchor, = the
    top-and-bottom two-suiter (spades + diamonds, 5-5) weighted above the `X` so a
    genuine two-suiter shows rather than lead-directs; natural `3♦`/`3♥` six-card
    one-suiters at `points(14..)`. Measured a **wash leaning slightly negative**
    (50 fired, −1.74 IMPs/fired plain, −2.00 PD; per-board CI straddles 0) — the
    value is mostly lead-directing, invisible to the double-dummy harness — so it
    ships off, like the Stayman/transfer defenses.

- **Jacoby-transfer competition and super-accepts, both sides — authored, opt-in
  (default off).** Three transfer auction families that previously fell through to
  the instinct floor are now authored behind toggles. A paired double-dummy A/B vs
  BBA over 640 000 boards (the `ab-dump-diff` paired scorer, where the shared BBA
  reference cancels) measured each, and **none earns its way on by default** — so,
  unlike the contested 2♣ Stayman, all three ship opt-in:
  - *Uncontested super-accept.* With four-card support for responder's major and a
    maximum, opener jumps to the three-level (`1NT-2♦-3♥` / `1NT-2♥-3♠`) instead of
    merely completing the transfer. Measured a **DD wash leaning negative** (−0.055
    IMPs/board it fires on) — opposite a transfer that may hold nothing, committing
    to the three-level overbids. Enable with `set_transfer_super_accept(true)`
    (`bba-gen --ns-transfer-super-accept`).
  - *Our transfer is contested.* After `1NT-(P)-2♦/2♥-(X)` opener **completes** with
    three-card support, **jump super-accepts** with four and a maximum, **Pass**
    declines with a doubleton (responder's `XX` is then a **forcing re-ask** opener
    must answer), or `XX` shows the doubled transfer suit as its own; after a 2-/3-
    level overcall opener super-accepts the major with a fit, doubles for cards, else
    passes. Unlike the contested 2♣ Stayman (which won +3.5 IMPs/fired), these
    continuations measured a **DD loss** (plain −0.94, PD −0.33 IMPs/board it fires
    on; plain-DD CI excludes 0) — the super-accept and forcing re-ask drive us into
    failing contracts the floor's lower bids avoid. Enable with
    `set_competition_over_transfer(true)` (`bba-gen --ns-comp-over-transfer`).
  - *Defending their transfer.* Over `(1NT)-P-(2♦/2♥)`: `X` = lead-directing the bid
    (transfer) suit (not takeout); a cue of the suit they showed = the other major +
    a minor (Michaels 5-5); natural one-suiter overcalls (six-card, `points(14..)`,
    the Stayman-defense floor); the transfer suit's own 3-level bid is a strong
    declare above the `X`. Distilled from BBA (`probe-bba-constraints --mode
    xfer-h`/`xfer-s`: `X` = the bid suit, natural overcalls, ~80% Pass). Like the
    Stayman defense its value is mostly lead-directing (invisible to the harness),
    and the A/B confirms a **PD wash** (+0.006 IMPs/board it fires on, CI straddles
    0; the plain-DD loss is the light-sacrifice artifact PD prices away). Enable with
    `set_transfer_defense(true)` (`bba-gen --ns-transfer-defense`).

- **Competition over the 2♣ Stayman, on both sides.** Two auction families that
  previously fell through to the instinct floor are now authored:
  - *Our Stayman is contested.* After `1NT-(P)-2♣-(X)` opener answers in the
    expert *pass-denies-stopper* coded scheme — a major or `2♦` promises a club
    stopper, **Pass denies one**, `XX` is business clubs — and when opener passes,
    responder's `XX` is a **forcing re-ask** (direct XX is business, balancing XX
    is SOS) that opener must answer (the artificial `2♦` denies both majors).
    After a `(2♦/2♥/2♠)` overcall opener bids a 4-card major naturally if it
    outranks their suit, doubles for cards (or the major they could not bid),
    else passes. **Default on** — a paired double-dummy A/B vs BBA over 256 000
    boards scored **+3.5 IMPs/board it fires on** (CI excludes 0). The off-switch
    `set_competition_over_stayman(false)` (`bba-gen --no-ns-comp-over-stayman`)
    isolates it for A/B.
  - *Defending their Stayman.* A new **opt-in** defense to `(1NT)-P-(2♣)`:
    `X` = lead-directing clubs (the bid suit, not takeout); `2♦/2♥/2♠` = a natural
    **6-card** suit, `points(14..)`; `3♣` = a **strong** natural club one-suiter
    (declare, not preempt); no Michaels cue (their 2♣ is artificial). The overcall
    length and strength were **A/B-searched** (`set_stayman_defense_overcall` /
    `bba-gen --ns-staydef-overcall LEN:FLOOR`), not copied from BBA: a paired
    perfect-defense (PD) sweep over 1 M boards/setting is monotone in the floor —
    over a *strong* 1NT the bidding side holds the points, so a light natural
    overcall into their auction is PD-negative; the 8–13 floors lose, **14** is
    where the overcalls turn DD-harmless, and tightening further only deletes the
    sound overcalls that carry the (DD-invisible) competitive value. The sweep also
    prefers **length-6 over length-5** (the 5-card overcalls' plain-DD edge is the
    light-sacrifice artifact PD prices away). The original loose placeholders
    (5-card / `points(8..)`, weak `3♣` preempt) are gone. **Off by default** —
    enable with `set_stayman_defense(true)` (`bba-gen --ns-defense-to-their-stayman`).
    Its value is mostly lead-directing (invisible to the double-dummy harness), so
    the whole convention stays a DD wash vs not bidding and remains opt-in. A
    speculative Unusual 2NT (both minors) was tried and measured **−4.9 IMPs/board
    it fires on**, so it was dropped.

- **`ab-dump-diff` gains `--score plain|pd`.** The paired delta can now re-price
  with perfect-defense doubling (`ns_score_pd`) instead of only honest double-dummy.
  For a *competitive* feature, a plain-DD edge that the PD pass erases is the
  light-sacrifice / doubling artifact, so reporting both numbers is the honest read.

- **`bba-gen` can now isolate our 1NT *opening* against BBA, defense held
  constant** (`--isolate-opening <off|bba|pons>`, the mirror of
  `--isolate-defense`). It keeps only boards where *our* pair (N/S) opens 1NT and
  bids the two tables so the **defender is identical in both arms** — so the swing
  is the opening (and its constructive continuations) alone, with the
  opening-vs-defense confound the umbrella `--filter-1nt` duplicate carries removed.
  `bba` holds the defender to BBA's Multi-Landy (table B is the all-BBA reference);
  `pons` holds it to our natural defense (table A is all-pons, table B is
  BBA-opens / we-defend). The comparison is *our 1NT auction* vs *whatever BBA does
  with the same N/S cards* (BBA may not open those hands 1NT at all). Result
  (320 000 boards each, `--filter-1nt`, vulnerability none): our 1NT opening
  **trails BBA's robustly** — defender BBA: plain **−0.442** IMPs/board (95% CI
  [−0.455, −0.428]), PD **−0.525** ([−0.540, −0.510]); defender pons: plain
  **−0.123** ([−0.138, −0.107]), PD **−0.572** ([−0.591, −0.552]). All four CIs
  exclude 0. The deficit is concentrated in the constructive continuations — Stayman
  `2♣` (≈−1.7/board), Jacoby transfers `2♦`/`2♥` (≈−1.4), and direct `3NT` (≈−2.7) —
  while Puppet/minor `3♣` is mildly *positive* (+0.2). So it is the 1NT-system
  machinery, not the 15-17-balanced opening decision itself, that lags BBA.

- **`bba-gen` board generation can now use every core, via processes.** EPBot's
  FFI is thread-unsafe, so the bidding half has always been single-threaded. The
  new `scripts/bba-gen-parallel.sh` sidesteps that by sharding across **processes**
  instead of threads — one `bba-gen --seed i` per core, each with its own address
  space, `.so`, and thread-locals (no shared state to race on). `bba-score` now
  accepts **multiple dump files** and concatenates their boards (rejecting shards
  whose labels or vulnerability disagree), so the shards merge back into one match:
  `scripts/bba-gen-parallel.sh out 1000 --isolate-defense` then `bba-score
  out/shard-*.json --score pd`. Single-file and stdin invocations are unchanged.

- **`bba-gen` can now measure our Landy honestly against BBA.** A new `--ns-landy
  LO:HI` overlays Landy on our natural 1NT defense (`2♣` = both majors, `2NT` = both
  minors), and `--advertise-landy` discloses it by setting BBA's opponent model to
  read our `2♣` as both majors and `2♦`/`2♥`/`2♠` as natural — the honest mirror of
  the overlay (vs `--advertise-natural`, which would misread `2♣` as clubs). This
  matters because the prior self-play harness has **no counter-defense to Landy**: the
  opposing side can neither penalty-double a light both-majors `2♣` nor compete in its
  now-implied minor fit, so it flatters the convention. Validated on 1 000 paired
  boards: BBA changes its auction on ~46% of the boards we bid Landy `2♣` (doubling or
  competing on the disclosure), where the self-play opponent never does.

- **A `doubled` probe mode for `probe-bba-1nt`.** Mirrors the existing `responder`
  mode but with BBA *opening* the 1NT and receiving a penalty double, reading BBA's
  runout style across a strength/shape battery. Finding: BBA plays **systems on** —
  transfers (`2♦`→♥, `2♥`→♠), minor transfers (`2♠`→♣), and Stayman (`2♣`) run on
  top of the double exactly as uncontested; weak balanced hands `Pass` to defend the
  doubled 1NT, and strong balanced hands `XX` (business/values). It is *not* a
  natural scramble. Vulnerability did not change any call in the sample. Run with
  `cargo run --release --example probe-bba-1nt doubled`.

- **A second 1NT minor-suit response scheme, "European", selectable per book.**
  Alongside the default "Puppet" scheme (`2♠` = clubs-or-invite, `2NT` = diamond
  transfer, `3♣` = Puppet Stayman), the opt-in European scheme plays `2♠` = clubs
  (transfer), `2NT` = a balanced invitational eight (the size ask, opener accepting
  game with a maximum), and `3♣` = diamonds (transfer), with no Puppet Stayman — so
  a game-forcing balanced hand with only a three-card major bids 3NT and a 4-3 game
  force takes Stayman. This is the standard Polish Club / WJ and common continental
  treatment (and BBA's "Atlantic" style). Select it with
  `set_notrump_minors(EUROPEAN)` (default `PUPPET`, the prior behavior): both
  schemes' `2♠`/`2NT`/`3♣` rules and continuations are authored and gated by alert
  at book-construction time, and the floor reads the European calls (2NT = balanced
  invite not a transfer; 2♠ clubs / 3♣ diamonds artificial). Covered by the new
  `tests/american_european_minors.rs`; the Puppet default is unchanged.

- **A weak advancer now runs from their redoubled penalty double (`[1NT, X, XX]`).**
  After our natural penalty double of their 1NT, their redouble is business in
  every system we face (BBA and our own: "we make 1NT redoubled"), so a broke
  advancer escapes to its longest five-plus-card suit rather than sit for a making
  `1NTxx`; a values advancer (≥ 8 HCP) passes to defend and beat it. The defensive
  mirror of the existing responder runout, default on, with an off-switch
  (`set_advancer_xx_runout` / `bba-gen --no-ns-xx-runout`) for the A/B. (Five-plus
  suits only for now — a 4-4 bust still sits.) Paired A/B vs BBA's 2/1 (isolated
  1NT-defense match, 16000 we-defend both-vulnerable boards/seed): the penalty-X
  (`X`) bucket goes −174 → −67 IMPs (−0.328 → −0.125 IMPs/X-board), **+107 IMPs**
  recovered and isolated by construction to the boards where it fires. Restricting
  to the *immediate* `[1NT,X,XX]` is deliberate: extending the run to the balancing
  redouble (`[1NT,X,P,P,XX]`) regresses to −202 (−135 vs immediate), because there
  the advancer already passed its first turn — it chose to defend, and the
  redouble's announced max does not undo that.

- **The *doubler* now runs once that redouble travels back around
  (`[1NT, X, XX, P, P]`).** The companion to the advancer runout above, two calls
  later: after their business redouble of our 15+ penalty double, the advancer and
  opener pass it back to the doubler, who — holding a five-plus-card suit — escapes
  to it rather than defend a (usually-making) `1NTxx`; a 4-3-3-3/4-4-3-2 doubler with
  nowhere to run still sits. There is **no HCP cap** (the penalty double already
  promised 15+), only the five-plus-suit gate. Default on, with an off-switch
  (`set_doubler_xx_runout(false)` / `bba-gen --no-ns-doubler-run`), read once at book
  construction so a duplicate A/B bakes the rule into the on book alone. The auction
  is **rare** — a 15+ *balanced* doubler seldom holds a five-card suit (only 5-3-3-2),
  and the redouble has to come all the way back — but it is **measured positive
  wherever it fires**:
  - vs BBA's 2/1 (isolate-defense, 200000 boards, vulnerable, paired same-seed delta
    via the new `ab-dump-diff`): the any-shape double fires on 212 boards (0.11%) for
    **+0.0067 IMPs/board [95% CI ±0.0018, excludes 0]**, +6.3 IMPs per fired board.
  - DD self-play (5,000,000 boards/cell, natural defense on both arms, only the runout
    differing): **+8.6 to +10.2 IMPs per divergent board** across both double shapes
    (any: 979/5M; balanced: 275/5M) and both vulnerabilities — the win is escaping a
    redoubled 1NT that beats us where we sit, a little larger when vulnerable.
  It fires on only 0.005–0.11% of boards, so the per-board effect is small
  (≈+0.0005 to +0.0067 IMPs/board) — directionally-correct insurance against the rare
  redoubled-1NT disaster, shipped on like the advancer runout it mirrors. The new
  `ab-dump-diff` example scores the paired delta of two same-seed `bba-gen` dumps,
  double-dummy-solving only the handful of boards a rare feature actually touched.

### Changed

- **Responder raises opener's Stayman major on adjusted points, not raw HCP.**
  After `1NT-2♣-2M` with a fit, the invite-versus-game choice now runs through a
  shared `notrump::fit_value` — point count plus a point for each trump past the
  eighth (the ninth and tenth trump, worth a trick apiece once the suit is
  agreed). A flat eight (4-3-3-3, no ruffing value) still only invites (`3M`);
  *any upgrade past it* now bids game (`4M`) opposite the 15-17 opener — a
  working singleton or void (`point_count` already lifts a 4-4-4-1 eight to nine)
  or a fifth trump. Previously both raised on raw `hcp` alone, so the shapely
  eight under-bid the flat one. `fit_value` also backs the both-majors relay
  placement (which adds the *knowable* second-major fit on top). Paired same-deal
  A/B vs BBA (128k boards/arm, `--filter-1nt`, vul none, 0.09% fired):
  **plain +1.874 IMPs/fired (+0.0016/board, 95% CI ±0.0008) and PD +1.523/fired
  (+0.0013/board, 95% CI ±0.0009)**, both excluding 0.
- **Responder no longer uses any form of Stayman with a flat 4-3-3-3.** Per
  [Pavlicek's statistics](https://www.rpbridge.net/8j05.htm), a `(4333)` hand plays
  better in 3NT than in a major-suit game even with a 4-4 (or 5-3) fit — flat, it
  has no ruffing value, so the trump suit buys no extra trick. A flat 4-3-3-3
  responder now bids notrump directly instead of `2♣` Stayman, `3♣` Puppet Stayman,
  or 3-level Stayman over a `2NT` opening: it invites with the natural `2♠` (Puppet)
  / `2NT` (European) size-ask at a bare 8, forces `3NT` at game values, and bids the
  quantitative `4NT` at 16–17. Only the flat shape is diverted (gated by the
  existing `notrump::flat_4333`) — a `4-4-3-2` or a non-flat 4-3 still
  Staymans/Puppets. Seeded before/after A/B (`examples/ab-stayman`, opponents
  silenced, double-dummy, 200k boards × 3 seeds × {none, both}): the ~0.14% of
  boards it diverts net **positive on raw score every run** (+2.2k…+7.2k points per
  200k) and net positive at IMPs — the matchpoint-frequency signature of the 4333
  curse. **In competition** (our 1NT overcalled) the same diversion now applies to
  the Transfer-Lebensohl cue-Stayman and the `3♣`-over-`(2♦)` Stayman, settled by a
  paired BBA A/B as the new `set_competitive_4333` default (`Suppress`): 960k boards
  vul none, PD **+3.8 IMPs/fired** (95% CI excludes 0), plain a wash-to-win.
  *Advancing* partner's takeout double is left untouched — there partner is short in
  their suit, so the 4-4 fit keeps a ruffing value and the curse does not apply.

- **Landy's and Woolsey's both-majors `2♣` now share one strength band.** The two
  conventions bid the identical both-majors `2♣`, so rather than carry two
  independent `points` ranges, `set_landy(Some((lo, hi)))` now feeds the same band
  that Woolsey's `2♣`/`2♦`/`2♥`/`2♠` read (`set_woolsey_points`), and `landy_2c`
  overcalls on `woolsey_points()` — one knob instead of two. No behavior change to
  either convention's default. Honest measurement (BBA reading our Landy via the new
  `--advertise-landy`, 16 000 paired boards) shows the band is nearly inert anyway:
  the `:19` cap binds on **1 board in 16 000**, the floor barely moves the IMPs
  (`8+`↔`12+` is −0.003 IMPs/board, CI straddles 0), and Landy trails our natural
  defense at every band (≈−0.17 IMPs per board it fires). The self-play preference
  for a light floor (`6+` best) was the artifact of an opponent that cannot punish a
  light `2♣`; once BBA reads it, the gradient flattens and mildly favors *sound*.

- **Defense to their 1NT is now composed from per-call alerts instead of a
  per-system `if`/`else if` cascade.** A defensive "system" is a bundle of per-call
  conventions — "Woolsey" is really `X` = Woolsey + `2♣` = Landy + `2♦` = Multi +
  `2♥`/`2♠` = Muiderberg — so each artificial call is now authored once as an
  `Alert`-stamped block, all are chained at the `[1NT]` node, and `Rules::gated` ships
  only the active system's calls at book-construction time (the same build-time gate
  as the Puppet/European 1NT split). The guiding invariant: **an alert marks an
  artificial call, so only artificial calls carry one** — the penalty `X`, the four
  natural suit overcalls, and `Pass` stay unalerted and floor-safe (dropping their
  node is at worst suboptimal; the instinct floor bids them sensibly), while every
  convention is pinned by its alert. Purely internal: all public setters
  (`set_woolsey`, `set_direct_dont`, `set_direct_landy_double`, `set_landy`,
  `set_unusual_notrump_defense`, the tuning knobs) and every defended auction are
  unchanged — a new test asserts the `[1NT]` node authors at most one rule per call
  in each named config, and the existing routing/inference suites pin parity. (The
  diverged building blocks — Woolsey's `2♣` is `passed_two_suiter`, the standalone
  Landy `2♣` is `five_four` — are kept as distinct alerts, since the ≤5-major cap
  routes a 6-card major to the Multi `2♦` and is load-bearing for the bundle's
  disjoint shapes.)

- **The inference floor now reads an alerted call as its convention, driving
  per-call defense switching (`set_alert_reading`, default on).** `project_authored`
  decided "artificial" purely structurally — a call floors a suit it does not name
  (Jacoby `2♦` → 5+♥) — which misses the *strength*-showing artificials that floor no
  foreign suit: the strong `2♣` opening (22+, no shape), its `2♦` waiting / `2♥`
  double negative, and Puppet `3♣`. Those were misread as a natural suit, so partner
  (and the keyless floor behind it) thought opener held that suit. The reader now
  also treats a call as artificial when its authoring rule carries an `Alert`, on top
  of the structural test — a union that only adds coverage, never dropping a read the
  structural test already made — so the floor suppresses the phantom-suit read and
  projects the convention instead, for our own *and* the opponents' alerted calls.
  Constructive A/B (`ab-alert-reading`, paired self-play, opponents silenced, honest
  plain-DD, 24000 boards/seed): **+2.08 IMPs/divergent vul none, +1.59 vul both**,
  consistently positive with no sign flip, on the ~0.4% of boards a strength-showing
  artificial swings the contract. Contested A/B vs BBA's 2/1
  (`bba-gen --no-alert-reading` + `bba-score`, paired, `--advertise-natural`, 20000
  boards/seed): **+181 IMPs vul none, +216 IMPs vul both** recovered — same sign, no
  regression. Toggle off with `set_alert_reading(false)` to recover the
  structural-only reading.

- **The system identity (`Family`) and a new per-call `Alert` are now two distinct
  types.** The opponent-visible system label keeps its name, role, and API —
  `Family::NATURAL` / `STRONG_CLUB` / `WEAK_NOTRUMP`, the `Pair::family` field,
  selected via `Pair::against` and the `competitive_vs` / `defensive_vs` overrides —
  the convention card opponents pick a *base* defense against. Split out from it, a
  `Rule` may carry an optional `Alert` (an open `&'static str` newtype): the per-call
  dual that names the artificial convention a call shows. `Rules::alert(a)` stamps
  the most recently added rule (mirroring `Rules::note`), and `Rules::gated(active)`
  drops the rules whose alert is inactive at trie insertion — so one book authors
  several convention variants and ships only the selected one (the Puppet/European
  1NT split, the Woolsey/DONT defense selection). The two never mix: `Family` keys
  `Pair`/`against`, `Alert` keys `Rule`/`gated`.

- **Every artificial constructive call now carries an `Alert`.** The strong `2♣`
  opening and its `2♦` waiting / `2♥` double-negative responses; Stayman, Jacoby
  transfers, South African Texas, both-majors `3♦`, Smolen, and the Puppet/European
  minor schemes over 1NT; splinters, weak jump shifts, inverted minors, and the 2/1
  game force; Jacoby `2NT` and opener's shortness rebids; and the RKCB keycard
  responses — each is stamped with the convention it shows. Pure disclosure metadata
  on its own (the 1NT-response gate is widened so always-on alerts survive variant
  gating); the behavior it unlocks is the alert-reading change below. The
  competitive book (negative/support doubles, cue-bids, Lebensohl) and the scattered
  `4NT` keycard *ask* are a follow-up.

### Removed

- **The passed-hand 1NT-defense subsystem is deleted.** After the passed-hand
  both-majors double was made opt-in (default off, losing to BBA), the machinery
  was dead by default, so it is removed entirely for a blank slate: the
  `PassedHandDefense` enum, `set_passed_hand_defense` and the thread-local it
  drove, both 1NT-defense book match arms (the reassigned double, the full passed
  DONT, their Landy/DONT advances and doubled-`2♦`-relay completions), the
  passed-hand shape helpers, the `bba-score` `X (PH Landy)` / `X (pen)` bucket
  split (our direct `X` over their 1NT now buckets as a single `X`), and the
  `bba-gen --ns-passed-landy` / `ab-landy --ns-passed-dbl` flags. Direct Landy
  (`landy_advances` / `landy_2d_rebid`), the direct-seat DONT defense, the natural
  penalty double, and `inference::penalty_x_reading` are untouched.

### Changed

- **The natural penalty double of their 1NT defaults to `Balanced` shape again
  (was `Any`).** The 15+ HCP floor is unchanged; only a *flat* 15+ hand now
  doubles, where before any 15+ shape did ([`set_natural_double_shape`] /
  `--ns-double-shape`). A flat hand has no escape for the opener to punish and
  genuinely wants to defend `1NT` doubled; a shapely 15+ hand would rather declare
  its own suit, and the opponents simply run from the double into a making
  contract. The `Any` default rested on a cosmetic tiebreak of a within-noise
  `bba-match --isolate-defense` edge ("a 15+ hand has no overcall outlet, so just
  double"); the axis had never been cleanly isolated in self-play. It now has:
  isolated plain-DD self-play (`ab-landy --ns-double-shape any --ns-majors ""
  --ns-minors ""`, 100k filtered, 16.9k divergent) prefers `Balanced` by **−0.70
  IMPs/divergent** (−0.92 under perfect-defense doubling). Against BBA the edge is
  a wash that the change does not cost: the head-to-head over 138 divergent
  isolate-defense boards is +0.33 IMPs/divergent for `Any` with a CI straddling 0.
  `Balanced` strictly dominates — it wins self-play and ties BBA.
- **The penalty-double latch no longer un-latches when our side bids.** After our
  natural penalty double of their 1NT, the "once penalty, always penalty" stance
  ([`set_penalty_latch`], default on) now holds for the *rest of the auction* — a
  contract bid of our own (e.g. the advancer running to a suit) used to abandon
  the stance, turning later doubles back into takeout; now they stay penalty. The
  floor (`penalty_latched`) and the matching inference reading
  (`penalty_latch_double_reading`) are updated in lockstep.
- **A passed hand's both-majors `X` of their 1NT is now opt-in (default `None`,
  the historic dead double).** The reassignment of a passed hand's otherwise-dead
  penalty double to both majors was promoted to default-on on a *self-play* DD
  win — but that A/B measured it against an *always-pass* baseline, which only
  proves DD rewards competing a partscore it cannot see defended (the
  obstruction-blind artifact). Re-measured against BBA's 2/1 on the isolated
  1NT-defense match (paired, 4000 we-defend boards/seed, both-vulnerable), the
  passed-hand `X` bucket is a net loss — doubled passed-partscores (`1NTxx`,
  `2♥xx`, `2♦xx` on misfits) outweigh the gains — and disabling it recovers
  **+43 IMPs / 4000 boards**, isolated by construction to the ~20 boards where it
  fired. `set_passed_hand_defense(Some(NaturalLandyDouble))` (or `bba-gen
  --ns-passed-landy`) restores it; the convention, its Landy advances, and the
  doubled-relay completion are all retained as the opt-in. The direct-seat 15+
  penalty double is untouched.
- **A passed hand's both-majors `X` of their 1NT no longer strands a doubled `2♦`
  relay.** After `[P,P,P,1NT,X,(XX),2♦]` the `2♦` is the artificial equal-majors
  relay ("you pick a major"), but only the *passed*-relay continuation
  (`…2♦,P → name the major`) was authored; when the opponents *doubled* the relay
  the doubler had no rule and sat, declaring `2♦x` on a 4-2 misfit. The doubler
  now corrects to its longer major whether the relay is passed or doubled (the
  `…2♦,X` twins, matching the direct-seat both-majors branch). Isolated DD effect
  vs BBA's 2/1 (the six affected boards/seed): the penalty/both-majors `X` bucket
  improves −1.588 → −1.448 IMPs/X-board non-vulnerable and −1.013 → −0.903
  vulnerable, no regression elsewhere.
- **After our natural penalty double of their 1NT, the doubler stops pulling its
  own double** (`set_penalty_no_pull`, **on by default**). A double is not a bid,
  so the keyless instinct floor's overcall-shaped rules (the 15–18 balanced
  notrump overcall and the five-card suit overcall) still fired for the doubler
  over the opponents' escape — a 15+ balanced hand "competed" to 2NT/3NT/a major
  opposite a likely-broke partner (they opened a strong 1NT, so our 15 is offset
  and partner is usually busted), the single worst defense leak. Now, while the
  penalty latch holds (`penalty_latched` — we doubled their 1NT and have bid no
  contract since), those two overcall rules step aside and the doubler defends
  (Pass) or latch-doubles the runout instead. DD-measured against BBA's 2/1 on
  the isolated 1NT-defense match (8000 we-defend boards/seed): the penalty-X
  bucket goes −2.312 → −1.013 IMPs/X-board vulnerable (paired **+0.058 IMPs/board
  overall, 95 % CI [+0.030, +0.085]**), neutral non-vulnerable (+0.007, CI
  straddles 0); the swing is isolated to the X bucket. `bba-gen --ns-allow-pull`
  restores the old pulling behaviour for the off arm of the A/B. `bba-score`
  gains `--action <label>` to filter the worst-board dump to a single defensive
  call (e.g. `--action X` for penalty-double boards) and splits the defensive `X`
  bucket into `X (pen)` (the direct 15+ penalty double) and `X (PH Landy)` (a
  passed hand's both-majors takeout) — completely different conventions whose
  separate DD results the combined bucket hid.
- **The `bba-match` BBA-reference example splits into `bba-gen` + `bba-score`,
  exchanging a JSON board dump.** `bba-gen` does only the single-threaded EPBot
  bidding — it owns every `set_*` / convention knob that shapes the auctions
  (`--isolate-defense`, `--ns-woolsey-range`, `--advertise-natural`, …) — and
  writes the bid boards to `-o <path>` or stdout. `bba-score` reads them (a path
  or stdin), solves the divergent boards double dummy, and prints the IMPs/board
  report; it never loads `libEPBot.so`. Both build only with `--features serde`.
  Piping `bba-gen … | bba-score` reproduces the old one-shot match byte for byte;
  saving a board file instead lets a tuning loop re-score the *same* bids many
  ways (`bba-score --score plain|pd`, or `-v` to re-price at another
  vulnerability) **without re-bidding** — the bidding is ~0.9 s but the
  double-dummy scoring is ~70 s of CPU, so caching the bids is the real win, and
  the single-threaded `bba-gen` can now run beside a core-saturating self-play
  sweep. The shared scoring core (divergent → DD solve → IMPs + 95 % CI) is
  factored into a `score_boards` helper in `examples/common/`, reused by
  `ab-landy`. (`bba-score` additionally hides empty report buckets and takes the
  scoring vulnerability at score time; the per-convention numbers are unchanged.)
- **The Woolsey suit-overcall floor drops from 10 to 8** (`WOOLSEY_POINTS`
  default `(8, 19)`; `examples/ab-landy`/`bba-match --ns-woolsey-range` default
  `8:19`), level with the natural-overcall floor. A re-probe (now that the
  continuations are fully authored, M6.2) swept the floor 6–13 three ways:
  honest plain-DD self-play vs our natural defense **peaks at 8** (+0.459
  IMPs/divergent) and flattens below it (6/7 add nothing), and the BBA
  isolate-defense head-to-head agrees (best at 8, −0.207 IMPs/board). The old
  "lower floor always loses more" verdict was an artifact of *unauthored*
  follow-ups, not the light overcalls themselves. Perfect-defense (PD) still
  mildly prefers 10, but PD over-deters by assuming a flawless doubler; the two
  honest measures, plus the principle that the conventions only rearrange *which*
  call shows a hand (same hands, same opponents), put the floor at natural's 8.
  The takeout-`X` floor is unchanged at 12 — lowering *it* hurts plain-DD, since
  the double is the constructive action, not a preempt. Woolsey stays **opt-in**
  (`set_woolsey` default off), so the default system is unchanged.
- **The opaque DONT / Woolsey / Multi 1NT-defense shapes are re-authored as
  transparent `or`/`and` constraints, and DONT now defends a traditional 4-4
  (AI-bidder M6.2d).** The seven shapes that hid behind the `described(label,
  closure)` escape hatch — DONT's one-suiter / minor-major / both-majors, Woolsey's
  Multi / Muiderberg / takeout-double, and the direct-Landy both-majors `X` —
  projected *nothing* (an opaque closure is invisible to `project`). They are now
  stated with the new `or`/`and` suit-set combinators (DONT both majors =
  `and([♥,♠],4..)`, Landy = `and([♥,♠],4..) & or([♥,♠],5..)`, Multi =
  `or([♥,♠],6..) & and([♣,♦],..=4)`), so each reads like its bridge spec *and*
  projects its real shape off its own rule. Two deliberate **behavior changes** come
  with the move to traditional shapes: **DONT two-suiters now accept a flat 4-4 by
  default** (`set_direct_dont_four_four` flips on — DONT is traditionally a 4-4
  method; set it off for the old 5-4+), and **Woolsey's Multi `2♦` drops its
  strictly-longer-major / no-6-6 guard**, so a 6-5 or 6-6 major hand competes with
  `2♦` instead of passing. Muiderberg keeps its exactly-5 + other-major-≤3 caps (the
  Woolsey structure relies on disjoint shapes so its uniform 1.9 weights never tie).
  Both conventions stay **opt-in** (`set_direct_dont` / `set_woolsey` default off), so
  the default system is unchanged. **A/B** (`ab-landy`, 60k filtered, none-vul,
  self-play vs the natural defense): DONT 4-4 is DD-negative — −0.362 IMPs/divergent
  plain, −1.397 PD (4-4 competes on far more hands; the obstruction value is
  single-dummy, invisible to the perfect-defense DD harness — the recurring
  obstruction-wall result). Woolsey with the wider Multi is modestly DD-positive on
  both scorings — +0.414/divergent plain, +0.065 PD. Each new shape is verified
  behavior-faithful to its intended spec by a `verify::compare` guard (8k sampled
  hands per shape). See `docs/ai-bidder/rule-projection.md`.

- **The three declarative `*_reading` decoders are retired — an artificial call's
  meaning is now read straight off its authored rule (AI-bidder M6.2c).** The
  generic `authored_reading` projection pass is wired into production:
  `SearchBook::classify` now prefixes its search context (the one keyless leak that
  fed `features` and the EV sampler's `Inferences::read`), and `Inferences::read`
  folds the projection in — the same `project` artificial-detector (a call whose
  projection floors a suit it did not name) drives *both* the suppression of an
  artificial call from the natural reading *and* the recording of its shape. With
  that, `transfer_major_reading`, `leaping_michaels_reading`, and `landy_reading`
  (and `LandyReading`) are **deleted**: the authored rule is the single source of
  truth, no longer mirrored by hand. Only the Landy/Woolsey advancer's `2♦` relay
  keeps a small `landy_advance_suppress` stub (a relay names no length, so its rule
  projects nothing for the detector to catch). **Impact on bidders:** the
  projection is sound but in two spots reads differently from the old hand decoders
  — it pins a completed transfer's *five*-card floor but drops the old reader's
  *six*-card upgrade off a follow-up jump/raise (a natural-suit raise is outside the
  projection's artificial-only scope; soundness over tightness), and it reads
  Woolsey's `2♣` majors as the rule's true **4-5** rather than the old loose 4+
  (Woolsey sends a six-card major to its Multi/Muiderberg calls). The deterministic
  `instinct()` ladder bids by rule and is unchanged; only search-based bidders read
  partner's projected shape. Architectural payoff, IMP-neutral by design. See
  `docs/ai-bidder/rule-projection.md`.

### Added

- **`or` / `and` suit-set length combinators in the constraint DSL (AI-bidder
  M6.2d).** `and(suits, range)` requires *every* listed suit in `range` — projecting
  each suit's floor (tight); `or(suits, range)` requires *some* listed suit in
  `range` — projecting the sound union of the arms (loose, washing to no-info for two
  or more suits). They generalize `len` from one suit to a set, so a two-suiter
  states its own lengths declaratively — `and([♥,♠],4..)` = 4-4, `… & or([♥,♠],5..)` =
  5-4, `and([♥,♠],5..)` = 5-5 — and the shape is both readable and projectable. Folds
  into `eval` (crisp `all`/`any`), `describe`, and `project`; the projection soundness
  property test covers both.

- **Rule projection now reads an artificial call's meaning straight off its rule
  (AI-bidder M6.2b — validation).** `Rule::project` joins `eval`/`describe` as the
  reading-side fold, and a generic `authored_reading` pass walks the auction's
  authored trie nodes and, at each *artificial* call (one whose projection floors a
  suit it did not name — a transfer, a two-suiter, a Landy 2♣), records that
  projected shape against the bidder's seat. This is the single mechanism meant to
  replace the seven hand-written `*_reading` decoders. A new equivalence test proves
  the pass reproduces, exactly (signature suit lengths *and* points), the three
  declarative readers — `transfer_major_reading`, `leaping_michaels`, and `landy`
  core — on prefixed contexts built from the real book. **No behavior change:** the
  pass is `#[cfg(test)]`-only (no production caller yet); wiring it into the keyless
  sampler/features paths and retiring the readers is M6.2c. See
  `docs/ai-bidder/rule-projection.md`.

- **Responder's double of an overcall of our 1NT is now optional by default, and
  opener cooperates with it.** `DoubleStyle`'s default flips `Takeout → Optional`:
  over `[1NT,(2X)]`, responder's double shows 2-3 cards and values in their suit —
  cooperative, not pure penalty and not short-suit takeout. The documented "takeout
  is the best plain-DD double" verdict turned out to be an **artifact of opener
  mishandling responder's double** — opener had no authored continuation, so the
  floor read `[…,X,P]` as a takeout advance and either pulled a penalty double or
  ran a 3-card optional fit. Two book duals fix it: `set_penalty_double_leave_in`
  (default on) makes opener **sit** for a penalty double, and
  `opener_cooperates_optional` makes opener **stand on a fit and run only with a
  doubleton** for an optional double. Once both the doubler's partner *and* the
  takeout baseline are handled fairly, the ranking is **Optional > Penalty >
  Takeout** on `ab-lebensohl` (NS vs EW, both Transfer Lebensohl, 200k, ~1500
  divergent): optional beats penalty by **+1.59** and takeout by **+2.14
  IMPs/divergent**, penalty beats takeout by **+0.51** — robust to the responder's-
  double reading. (Waypoints along the way: with opener merely *sitting*, penalty-
  vs-takeout already flipped **−1.207 → +0.328**, a sign flip; a 3NT escape for an
  opener-max with a stopper A/B'd a *loss* vs sitting, so opener never pulls a
  penalty double — defending the doubled partscore beats a fragile game on a stack.)
  `Takeout`/`Penalty`/`PenaltyLight` stay selectable via `set_double_style`;
  `ab-lebensohl` gains `--ns-penalty-leave-in on|off`. Responder's double now also
  carries an 8+ HCP floor reading (`responder_overcall_double_reading`) for the
  sampler/search bidders.

- **The penalty-double latch ("once penalty, always penalty"), default on.** New
  `instinct::set_penalty_latch` (thread-local, **on by default**) models the human
  rule that once a side makes a penalty double, its later doubles are penalty too —
  never takeout. After our side's natural penalty double of their 1NT (the one
  penalty double the floor classifies today, via `penalty_x_reading`) the floor
  reads our later doubles as penalty: it doubles their runout for penalty on a
  trump stack instead of for takeout on shortness, and partner leaves our double in
  rather than advancing it — the mirror of the existing 1NT-runout encircle. Same-
  side only (the opponents' penalty doubles do not latch us), and a constructive bid
  of ours since the double unlatches it. The latch ships with a matching
  `Inferences::read` decoding (`penalty_latch_double_reading`): each later penalty
  double is read as four-plus cards in the doubled suit (the floor makes them only on
  a trump stack), so a sampling bidder reads them as penalty rather than takeout and
  does not pull — the floor action and its meaning stay in lock-step. A no-op unless
  the natural defense is on. DD-measured a win in the penalty-X bucket with no
  regression elsewhere — self-play (natural vs always-pass, 100k filtered) improves
  the X bucket −0.621 → −0.464 IMPs/action-board, and vs BBA (`--advertise-natural`,
  6k 1NT-filtered) −2.716 → −2.329 IMPs/X-board; the whole-system delta is noise
  (the latch fires only in penalty-X auctions, ~1% of deals). Disable with
  `set_penalty_latch(false)` (the off arm of the A/B). The `ab-landy` and `bba-match`
  examples gain `--ns-penalty-latch` to sweep it. The latched second double's *style*
  is now an opt-in A/B knob, `instinct::set_latch_style(LatchStyle::Penalty|Optional)`
  (default `Penalty`): `Optional` makes `(1NT)−X−(2Y)−X` a 2-3-card cooperative
  double (partner stands on a fit, runs when short) instead of a trump-stack penalty
  (partner sits) — the defensive mirror of `DoubleStyle`. A/B'd (`ab-landy
  --ns-latch-style`, two seeds 100k+300k): unlike the we-open side, optional is a
  **wash** on defense — Δ(opt−pen) was +179 IMPs at 100k but −34 at 300k under `Any`
  double-shape, and a faint +163/+60 under `Balanced`. The disciplined penalty stack
  is already near-optimal opposite an any-shape penalty doubler, so `Penalty` stays
  the default.

- **Floor reading for the natural penalty double of their 1NT.** A double of an
  opponent's 1NT names no suit, so the inference walk's takeout branch (which needs
  a *suit* opening) read it as nothing — leaving the floor to sample the doubler as
  a random weak hand and the advancer to pull a phantom suit. The floor now reads
  the direct-seat penalty double as the `set_natural_double_floor` points floor
  (15+ by default), mirroring the Woolsey/DONT double readings. It fires only when
  a double of their 1NT actually *means* penalty: the natural defense is on and no
  convention has repurposed the double (DONT, direct Landy, Woolsey each keep their
  own reading), and the doubler is not a passed hand (whose double is the
  both-majors passed-hand call). DD-measured neutral-to-slightly-positive with no
  regression — the natural-penalty-X bucket improves vs BBA (defense-isolated, 10k
  boards: −1.241 → −1.209 IMPs/X-board) and in self-play (+0.479 → +0.491); a
  consistency fix banked like the other 1NT-defense readers, not an IMPs win.

- **`Constraint::project` — a rule's forward shown-range envelope.** A third fold
  on the constraint DSL beside `eval` (score a hand) and `describe` (name the
  meaning): `project(context) -> Inference` turns a constraint into the per-suit
  length and point ranges every hand it accepts must fall within — the *forward*
  dual of evaluating a known hand. Length/points primitives project their band
  (`len` keeps both bounds, exact; `points`/`hcp` are floor-only, sound whether or
  not the fuzzy-strength upgrade is on and matching the hand-written readers'
  `at_least(floor, cap)`); `&` intersects, `|` takes the loosest span (soundness
  over tightness, so a Landy-style `(5♥4♠)|(4♥5♠)` projects the sound 4-4 floor);
  negation and opaque `pred`/`described` predicates project no information. The
  default is no-info, so every existing constraint compiles unchanged. Sound by
  construction — a finite `eval(hand, context)` implies the hand lies within
  `project(context)` — verified by a property test over the disjoint-suit
  disjunctions and the opaque escape hatch. New `Inference::intersect`/`union` and
  `Range::union` support the fold. This is the data substrate for reading an
  authored call's meaning straight off its rule; the forward reader and sampler
  keep their keyless `*_reading` decoders for now, as those run without a trie to
  project from.
- **Rule-replay layout acceptance for the search sampler — opt-in, default off
  (`set_rule_accept`).** Instead of projecting the auction into the hand-written
  per-convention `Inferences` ranges, the sampler now *reads each bid by the rule
  that authored it*: for every prior call a player made, the policy is re-run on a
  candidate hand at the node before that call, and the hand is kept only if the
  policy would rank that call within a margin of its best legal call. A bid's
  meaning is thus frozen at its node and survives later competition for free, and
  artificial calls suppress themselves — no `*_reading` decoder per convention.
  Replay only enforces at **authored** nodes (new `System::authored_at`):
  resolution follows the book's `Rebase` fallbacks to the canonical node, so
  authored responses, raises, Stayman, transfers, and 2/1 all count, while a bid
  only the keyless floor answers — a competitive raise/rebid with no authored node,
  where a `-∞` is mere absence of an opinion, not a real "don't bid that" —
  abstains and is left to the range reading. So replay *tightens* the old range
  reading wherever a rule answers, rather than replacing it. New
  `sample_layouts_replay` (public, sibling to `sample_layouts`); the `ev_all`
  search rollout uses it behind the flag. Replay is tighter than the loose ranges,
  so it draws from a far larger budget — up to `REPLAY_DRAW_CAP` (50M deals, ~10-20
  s, in tempo for a human bid) — because generating a deal is ~0.3 µs, negligible
  beside the double-dummy solve each *accepted* layout pays. A `REPLAY_DRY_LIMIT`
  consecutive-reject early-out distinguishes a *budget*-limited auction (keep
  drawing) from a *feasibility*-limited one (e.g. a penalty double needs the
  doubler to hold 15+, impossible when the actor is strong — bail within tempo);
  only then does `ev_all` top up with the range reader. `probe-replay-yield`
  measures the replay-vs-range fill (the no-DD pre-check). Distilled from the observation
  that the sampler is already a rejection filter and a `Constraint`/`System` is
  already a hand predicate. **Measured** on a paired search-floor A/B over 300
  filtered 1NT-defense boards (`ab-search-floor --filter --rule-accept`): replay
  moved the search floor **+0.24 IMPs/board** vs the deterministic floor (−0.94 →
  −0.70), changing 46% of decisions, but the 95% CI [−0.35, +0.83] still straddles
  zero. Neutral-to-positive with healthy yield — **kept default off** pending a
  larger run, matching the opt-in precedent for not-yet-conclusive measures.
- **Our own Woolsey "Multi-Landy" defense to their 1NT — opt-in, default off
  (`set_woolsey`).** Authors BBA's distilled structure at every seat, with our own
  (tunable) strength bands: `X` = a 4-card major + a longer (5-6) minor (takeout,
  *not* penalty), `2♣` = both majors (5-4 / 5-5, advanced via the existing Landy
  machinery), `2♦` = Multi (a single 6+ major), `2♥`/`2♠` = Muiderberg (exactly 5
  in the major + a 4+ minor), and an owning `Pass` for everything else — including
  strong balanced, so there is no penalty double. The shapes are disjoint, so they
  reproduce BBA's pass cases for free (a flat 22-count, a bare 5332 major, a 4-4 or
  six-card-minor one-suiter all pass). **Continuations authored in full** (so the
  structure never bleeds to the instinct floor): the Multi `2♦` (BBA's two-strength
  pass-or-correct with the `2♠` → `2NT` heart-relay, plus a game-force `2NT` ask),
  the Muiderberg `2♥`/`2♠` (invitational/game raises + the `2NT` minor-ask answered
  `3♣`/`3♦`), and the takeout `X` (relay `2♣` to the long minor / own 5+ major /
  `2NT` ask). Every artificial call has a doubled / redoubled escape so the
  opponents can never trap us in a doubled artificial contract (the `X`
  doubled/redoubled run alone moved that bucket from −2.0 to −0.19 IMPs/board vs
  BBA). Two tuning knobs: `set_woolsey_points(lo, hi)` (suit overcalls, **default
  10–19**) and `set_woolsey_double_floor(floor)` (the `X`, **default 12**) — our
  own floors, one above BBA's 9. `american()`'s default defense is **unchanged**
  (natural penalty-X + overcalls); Woolsey acts only on an explicit opt-in.
  Harness: `ab-landy --ns-woolsey on [--ns-woolsey-range LO:HI --ns-woolsey-x-floor
  N]` and `bba-match --isolate-defense --ns-woolsey`. **Measured DD-negative under
  honest scoring, and worse than the natural defense vs BBA** — like every
  preempt-flavoured convention here, the DD harness is blind to its obstruction
  value. On `ab-landy` (vs our natural defense): plain-DD ≈neutral (+0.22
  IMPs/divergent nv) but perfect-defense doubling (`--score pd`) is **−0.46**, and a
  floor-sweep is monotonic (the DD-positive setting is the one that stops
  competing). On `bba-match --isolate-defense` (vs BBA self-play, the realistic
  measure): our Woolsey started at **−0.29 IMPs/board**, statistically
  indistinguishable from doing nothing (always-pass −0.30) and worse than our
  natural defense (−0.20). **Floor readings then closed the artificial-bid leaks**
  (see below), lifting it to **−0.237**; the residual deficit is the overcall
  decisions + our floors passing where BBA competes (the `Pass` bucket, −1.04),
  which range tuning does not move. Still worse than natural, so kept opt-in.
- **Floor readings for every Woolsey artificial call** (`Inferences::read`), so the
  deterministic `instinct()` floor never misreads the convention once the opponents
  intervene and the auction leaves the authored book. Without them the floor read
  our artificial `2♣`/`2♦` as *natural* clubs/diamonds and raised the phantom suit
  into doubled disasters (`1NT 2♣ 3NT 4♣x` −1500). Added: `multi_reading` (the
  `2♦` Multi suppresses its diamond reading + caps both minors; the `2♥`/`2♠`
  Muiderberg pins major = 5, other major ≤ 3); `landy_reading` now fires for
  Woolsey's both-majors `2♣`; the advancer's preference (`2♥`/`2♠` over `2♣`/`2♦`,
  a pick among partner's majors — *not* own length, inverse over the Multi) and the
  `X`-advance `2♣` minor relay are suppressed, the suppression living for the whole
  read (covering doubled / contested runouts); and the takeout `X` records its
  `set_woolsey_double_floor` points floor (its 4-major-+-5-6-minor *shape* is a
  double disjunction the per-suit framework cannot pin, but the points floor alone
  stops the floor sampling the doubler as a random hand). The floor's syntactic
  competitive raises were also gated on `partner_shown_len` (the reading) instead of
  the literal bid suit, so it trusts the decode. This drove the `2♣`/`2♦` buckets
  from −2.46/−2.05 to −0.95/−0.93 IMPs/board vs BBA.
- **`probe-bba-constraints` — BBA's 1NT defense fully distilled (it's Woolsey
  "Multi-Landy").** New `--mode`s read the rest of the structure from real EPBot
  hands: `muider-h`/`muider-s` (the advances over the `2♥`/`2♠` Muiderberg) and
  `rebid-d`/`rebid-d2s`/`rebid-h`/`rebid-s` (the overcaller's rebid, which
  disambiguates each relay/ask), plus a longest-major / longest-minor read-out so
  the Multi and the 5-4 majors read at a glance. Findings in
  [`docs/ai-bidder/bba-1nt-defense.md`](docs/ai-bidder/bba-1nt-defense.md): **`X`
  is Woolsey — a 4-card major + a longer minor, 12–19 HCP, *not* penalty** (a flat
  22-count passes); **`2♣`** = ≥ 5-4 majors; **`2♦`** = a 6+ single major (Multi);
  **`2♥`/`2♠`** = Muiderberg (exactly 5 in the major + a 4+ minor); the Muiderberg
  **`2NT` advance is an artificial minor-ask** (overcaller answers `3♣`/`3♦`), and a
  *direct* `3♣`/`3♦` advance is vestigial. No library change — analysis tooling only.
- **Conventional DONT defense to their 1NT — opt-in, default off
  (`set_direct_dont`).** Replaces the natural penalty-X + overcalls at every seat
  with DONT: `X` = a one-suiter (♣/♦/♥; spade one-suiters bid `2♠` directly), `2♣`
  = clubs + a higher major, `2♦` = diamonds + a major, `2♥` = both majors, `2♠` =
  natural spades, `2NT` = both minors (the Unusual overlay), plus an owning `Pass`.
  Two tuning knobs: `set_direct_dont_one_suiter_min` (5 = classic; **6** = insist
  only on a six-card suit, passing five-card one-suiters) and
  `set_direct_dont_four_four` (let the two-suiters accept a flat 4-4).
  `american()`'s default defense is **unchanged** (natural penalty-X + overcalls)
  — DONT acts only on an explicit opt-in. Reuses the passed-hand DONT shape
  predicates and advance relays. Harness: `bba-match --ns-dont`
  [`--ns-dont-one-suiter-min N`/`--ns-dont-four-four`] and `ab-landy --ns-dont`.
  Two escapes (bug fixes, active only when DONT is on) keep the artificial `X` out
  of doubled misfits: a redoubled one-suiter `X` relays out of `1NTxx` rather than
  sitting, and — the dominant fix — over a **doubled `2♣` relay** the doubler now
  names its real suit (`[1NT,X,(XX),2♣,X]`) instead of being floored into `2♣x` on
  a hand that need not hold clubs (the relay is artificial). The escape is worth
  **+0.083 IMPs/board** on the honest measure.
- **Floor readings for the DONT artificial calls** (`Inferences::read`,
  `dont_reading`), so the deterministic `instinct()` floor never misreads DONT once
  the auction leaves the authored book — the same fix the Woolsey calls got. The
  generic walk read the `2♥` two-suiter as a *single* 5+ heart suit, the `2♣`/`2♦`
  as a *natural* 5+ minor (they can be only four), the one-suiter `X` as *nothing*
  (a random hand), and each advancer relay (`2♣` over `X`; `2♦`/`2♥`/`2♠` over
  `2♣`/`2♦`/`2♥`) as own length. Now: `2♥` pins both majors ≥ 4 (a Landy
  two-suiter); `2♣`/`2♦` re-pin the real minor ≥ 4 (the unknown major falls to the
  residual, surfacing naturally if later named); the `X` records the overcall
  points floor + `spades ≤ 3` (the one sound per-suit fact of a ♣/♦/♥ one-suiter);
  and every relay's natural reading is suppressed for the whole read (covering
  contested runouts via the shared `advancer_artificial` jump). The natural `2♠`
  keeps its genuine spade reading. Seat-isolated `bba-match --isolate-defense
  --advertise-natural --ns-dont --ns-dont-one-suiter-min 6` (10 000 boards, seed 42,
  paired): the defense bucket improves **−0.492 → −0.428 IMPs/board** with no bucket
  regressing — the wins are the two worst-misread calls, `2♥` (−0.003 → **+0.301**)
  and `X` (−0.983 → **−0.659**).
- **The honest DONT verdict — it reaches DD parity-to-win, but stays opt-in.**
  Seat-isolated `bba-match --isolate-defense --advertise-natural` (20 000 boards,
  seed-paired) vs natural's **−0.187 (none) / −0.480 (both)**: full DONT *loses*
  (−0.327 none) — the conceding one-suiter-`X` relay is the leak — but the
  **six-card one-suiter min + the doubled-relay escape** lift it to **−0.196 (none,
  tied) / −0.408 (both, +0.072 ahead)**, the first artificial 1NT defense to
  match-or-beat natural on this harness (DONT/Landy/Meckwell were all DD-lost
  before). The conventional `X` bucket (−1.23/bd none, −2.03 both) is better than
  the penalty double's (−1.42 / −2.76), and vulnerability — where doubled
  contracts bite — favors the conventional `X`. The spread across all variants is
  narrow (the obstruction wall — DD under-prices defensive pressure), so the real
  DONT-vs-natural verdict is single-dummy; **DONT stays opt-in, natural remains the
  default.** (Aside, same harness: natural with the penalty-X dropped entirely —
  15+ balanced just passes — scores −0.174 none, statistically tied with natural,
  confirming the X is DD-dominated but its value single-dummy.)
- **`bba-match --advertise-natural` — tell BBA our 1NT defense is natural, so the
  defense measure is honest.** BBA's 2/1 card assumes the defenders play
  *Multi-Landy* (its `2♣` = both majors, `2♦` = a Multi), so when our pair makes a
  *natural* two-level overcall BBA mis-reads it and mis-defends. The flag disables
  BBA's 1NT-defense conventions (`Multi-Landy`/`Cappelletti`/`Landy`) on the
  opponent bot **at our table only** — the all-BBA reference keeps BBA's genuine
  Multi-Landy — so it reads our overcalls naturally. Seat-isolated re-measure of
  the shipped defense (`--isolate-defense`, 20 000 boards, seed-paired): advertising
  natural moves the swing from **−0.005 → −0.181 IMPs/board (none)** and **−0.285 →
  −0.484 (both)** — the ~0.18 IMPs/board the old measure showed was a BBA-confusion
  artifact (it read our natural `2♣` as a both-majors two-suiter and gifted +1.50
  IMPs/board on that bucket). The honest finding: our natural *overcalls* are sound
  (`2♣/2♦/2♥/2♠` net **+81 IMPs**, `2♦` a clear **+475**); the entire deficit is the
  penalty double of 1NT (**−1.43 IMPs/board**, redoubled or run into a doubled
  partial) and the floor's undisciplined *continuation* doubling (a correct Pass,
  then a doubling war into `3NTx` down) — the obstruction wall, the single-dummy
  pressure a double-dummy harness scores as a dead loss.
- **Direct-seat both-majors double of their 1NT — opt-in, default off
  (`set_direct_landy_double`).** Replaces the 15+ penalty double at every seat with
  a Landy-style takeout double: `X` = both majors (`Some(false)` = at least 5-4,
  `Some(true)` = a flat 4-4 accepted), the four natural `2♣/♦/♥/♠` overcalls kept,
  the advancer answering through the existing Landy machinery (`2♦` relay / `2NT`
  game-ask / direct major). `None` (the default) keeps the natural penalty-X
  defense; `american()` is **unchanged**. Reuses `landy_advances`/`landy_2d_rebid`/
  `landy_2nt_rebid` — no new responder code. Harness: `ab-landy --ns-landy-x
  off|5-4|4-4`. **Runout (the bulk of the value):** worst-board forensics showed the
  dominant loss was a doubled major *run into a phantom `3♦`* — the advancer's
  artificial `2♦` relay made the floor think our side held diamonds, so after we
  named our major and they doubled it the floor bolted to `3♦x` (every −17/−18 board
  was this `… 2♦ X 2M X … 3♦`). Fixed in two layers: (a) over a **redoubled** `X`
  (`[1NT,X,XX]`) the advancer now runs *cleanly* — **`Pass` = ask back** (the
  redouble forces partner to bid, so the doubler names its five-card major), **a bid
  = to play** the natural suit (`2♣` sits at the two level over the redoubled `1NT`,
  giving a club one-suiter a home) — no artificial relay, no phantom diamond; (b)
  everywhere a named major is then doubled, an authored `Pass` node makes us **sit in
  our real major fit** instead of running. Progression on the 5-4 plain-DD result vs
  natural: relay-then-run −0.047 → sit-Pass +0.621 → clean runout **+0.705**.
- **The honest verdict — the 5-4 double is a DD win, but it is the penalty-X's
  known blind spot, so it stays opt-in.** `ab-landy --filter` vs the natural default
  (200 000 boards, seed-fixed): the **5-4** double scores **+0.705 IMPs/divergent
  (plain-DD) / +0.624 (PD)** — both positive, and PD *below* plain (so a genuine DD
  result, not a synthetic-doubling artifact). But the per-shape split shows the win
  is *abandoning the penalty double*, not the convention: the both-majors `X` action
  itself **loses** (the 5422/5431 two-suiters negative, the obstruction wall), while
  the whole +IMPs come from the **balanced 15+** rows — exactly the hands that no
  longer penalty-X. Measured against the **always-pass** baseline (do nothing over
  their 1NT) the 5-4 double **loses −0.176 (plain) / −0.375 (PD)** — *but so does the
  natural defense* (−0.293 / −0.457), and by *more*: on DD the best 1NT defense is to
  pass (the obstruction wall — DD under-prices lead-direction and competing for the
  partscore), and among defenses that *do* compete the both-majors `X` beats natural.
  This corroborates the established finding (the 1NT penalty double is a −1.43
  IMPs/board DD leak whose value is *single-dummy*), so dropping it for a both-majors
  `X` is the same DD-blind trade as preempts — **kept opt-in, natural stays default.**
  **5-4 ≫ 4-4:** the flat **4-4** scores **+0.050 (plain) but −0.823 (PD)** — the
  textbook overbid artifact (it reaches failing contracts that only escape when the
  opponents don't double), so the looser shape is a real loss; insist on 5-4.
- **Strength floor `set_direct_landy_double_floor` (default 15) + penalty-pass
  `set_direct_landy_penalty_pass` (default off).** Two strength refinements to the
  both-majors `X`. The **floor** partitions the both-majors hands: 8–14 overcall a
  major naturally (the "direct bid"), 15+ make the `X` (too strong to overcall). The
  A/B floor sweep (`ab-landy --ns-landy-x-floor`, 100k filtered) improves monotonically
  with strength — vs natural +0.0050→+0.0061/raw, vs always-pass −0.0070→−0.0052/raw
  across floors 8→16 — and peaks near 15–16 (competing less means fewer thin doubles
  the obstruction wall punishes). **15 is the shipped default**: it captures the peak
  with no orphaned point-count (floor 16+ strands the 15-counts into a pass, which
  flatters the DD number for the wrong reason). The **penalty-pass** lets the advancer
  convert the takeout `X` to penalty (`Pass` to defend `1NTx`) with no major fit and
  enough defense (threshold tracks the floor); the `[1NT,X,P]` node now carries a gated
  `Pass`, where it had been forcing. A/B-neutral on DD (the penalty you collect when
  they sit is single-dummy, invisible here), so it ships **off** but available — sound
  bridge for the strong-`X` style. `ab-landy --ns-landy-x-floor N --ns-landy-x-penalty
  on|off`.
- **`bba-match --ns-double-shape` now defaults to `any`** (was `balanced`), matching
  the shipped `american()` so a no-flag run measures the real default. Re-running the
  X-only-*balanced* restriction honestly (advertise-natural, 20 000, seed-paired) is
  **within noise** of `any` — **−0.188 vs −0.181 (none), −0.474 vs −0.484 (both)** —
  confirming b73dd6a's `Any`: restricting the penalty double to balanced hands only
  *relocates* the loss (the shapely 15+ hands move from the Pass bucket to the X
  bucket; the totals match), it does not fix it.
- **`bba-match --no-settle-floor` — A/B the settle floor's effect on defense.**
  Toggles `instinct::set_settle_floor` (the 9badc15 "pass = play the top bid"
  change, default on) so a seed-paired run isolates the floor update. With natural
  advertised, the update *helps* defense **+0.048 IMPs/board (none)** and **+0.055
  (both)** over 20 000 boards — small but the same direction at both vulnerabilities
  (CIs nearly disjoint), on top of the takeout-double-continuation gain measured in
  `ab-settle-floor`.
- **`scoring::ns_score_pd` — a perfect-defense scorer that carries the table
  `X`/`XX`.** Like `ns_score_bid` it doubles a contract that fails double-dummy
  (opponents always hold the red card), but a double or redouble already on the table
  is locked in and kept even when the contract *makes* — `X`/`XX` cannot be taken
  back. This is the correct scorer once a side may **defend by passing** (the settle
  floor above), which puts real doubled contracts on the table; the new
  `examples/ab-settle-floor` A/B uses it. `instinct::set_settle_floor` is its A/B
  knob (default on).
- **`american_neural_v3()` — a distilled neural floor that bids from *disclosable*
  information only.** Duplicate-bridge ethics require full disclosure: a call is
  explained to opponents by the partnership's *agreement*, never by the bidder's
  specific cards. So the new `features::features_v3` extractor (88 floats) drops
  every card-specific value the v1 vector carried (the 13 per-suit rank bits, the
  top-honor and stopper flags) and keeps only what a bidder could lawfully
  disclose: per-suit length and HCP (suit quality), global HCP and shape
  (`points − HCP`), and the public-auction / inferred-range / vulnerability
  context shared byte-for-byte with v1. `neural::classify_v3` and the
  `NeuralFloorV3` safety shell (same forced-rail delegation + legality mask as the
  other learned floors) wire it in. Gated behind `neural-floor`; `american()` and
  the v1/v2/search floors are untouched. The net was distilled from `american()`
  over the 100 000-deal GIB database (`ddss-sys/vendor/hands/sol100000.txt`).
  **Impact:** the ethical restriction is essentially free. Held-out top-1
  agreement with `american()` is **95.3%** (above v1's 93.8% — the disclosable
  summary is a sufficient statistic, and dropping card-detail noise generalises
  better). Against BBA's 2/1 over 1000 boards it scores **−1.94 IMPs/board**,
  indistinguishable from the full-information `american()` floor (−2.10); a direct
  DD duplicate vs `american()` costs only **−0.057 IMPs/board** (diverges on 9% of
  boards) while still beating bare books by +0.81.
- **Tooling for the v3 net:** `dump-teacher` gains `--features-version 3` and a
  `--deals <file>` source that bids out every deal in a GIB solution file (e.g.
  `sol100000.txt`) instead of random boards; `bba-match` gains
  `--our-floor {american|neural-v3}` to seat the new net against BBA; and the
  `ab-neural-floor` example gains a v3 cost-of-restriction section. The off-crate
  `trainer/` accepts feature version 3 unchanged (it sizes the model from the dump
  sidecar).
- **First-class GIB double-dummy files: a `gib` tool and a shared codec.** A GIB
  line is `<West-first PBN>:<20 hex DD digits>` — the double-dummy table cached as
  free I/O, so a database produced once is reused without ever re-solving. The
  table codec now lives upstream (`ddss::TrickCountTable::gib`/`from_gib`, ddss
  0.1.3) next to the existing `hex()`; the new `pons::gib` module adds the
  line-level `parse_line`/`format_line`; and the new `gib` example wraps them in
  three subcommands: `read` (pretty-print deal + DD grid), `generate` (deal random
  boards, solve once, write GIB — the previously-missing encode path), and
  `verify` (re-solve and confirm every cached tail, exit non-zero on any mismatch).
  `generate` is deterministic in its `--seed`, so each machine can produce a shard
  independently and the shards just concatenate (`cat shard-*.txt > all.txt`) — a
  GIB database needs no online fleet, only `cat`. `eval-calibrate` drops its
  private `decode_table` for the shared `from_gib`.
- **The teacher dump captures the cached DD table as a value target, and the
  trainer learns it.** With `--deals`, `dump-teacher` now appends the board's
  double-dummy table — re-oriented to the acting seat (`gib::relativized_tricks`,
  20 floats normalised by 13) — to each row (random boards have no free DD and
  omit it; sidecar records `dd_len`). The off-crate `trainer/` grows an optional
  value head (`H → 20`) off the shared trunk that regresses this table by MSE
  alongside the policy cross-entropy (`--dd-weight`, default 1), giving a
  policy-plus-value net in one pass. The value head is **train-only**: it is not
  exported, so the policy weights and the M1.2 parity fixture stay byte-identical.
  **Impact:** the double-dummy signal that was already sitting unused in the GIB
  file becomes a free auxiliary target that shapes the shared representation; on a
  1000-deal v3 dump the held-out DD MSE falls steadily during training with no
  change to the exported policy.

### Changed

- **The instinct floor now treats "Pass = play the top bid": advancing partner's
  takeout double is no longer 100% forcing.** Previously the floor *had* to advance a
  live takeout double — penalty-passing only on a genuine trump stack — which climbed
  to captive doubled contracts (`4♣x`, −800…) on a bust. The "settle floor"
  (`instinct::set_settle_floor`, **default on**) recasts Pass as playing the contract
  on the table: with four-plus cards behind their doubled suit a hand **defends**
  (pass plays their doubled contract — the better penalty), and a four-level advance
  becomes a *free bid* requiring values (~11+) since you could have defended. A hand
  that cannot beat their contract still advances exactly as before, so the
  anti-blunder rail (never pass a takeout double on a worthless hand into their
  contract) is preserved. The `forced_advance` predicate is renamed
  `advancing_a_double` to reflect that it is now a *context*, not a mandate. **Impact:**
  a clear win on the perfect-defense duplicate measure — **+0.264 IMPs/board vul none
  (95% CI [+0.251, +0.276]) and +0.372 vul both ([+0.357, +0.387])** over 200 000
  boards (9.35% divergent), larger vulnerable where defending doubled contracts and
  dodging doubled overbids both pay more. The change is contained to takeout-double
  advances, so constructive and other competitive auctions are untouched; the gated
  neural/search floors inherit it via the safety shell's forced-rail delegation.
  `set_settle_floor(false)` recovers the old always-advance floor (the A/B baseline,
  `examples/ab-settle-floor`).
- **`examples/ab-settle-floor` now reports both scorers — perfect-defense
  (`ns_score_pd`) and plain double-dummy (`ns_score_contract`) — side by side.** A
  perfect-defense score prices any contract that fails double-dummy as *doubled*, which
  is honest only when the `X`/`XX` is real on the table; for a change that suppresses
  bidding it over-credits by synthetically doubling the contracts we *didn't* reach. So
  the A/B prints both: the plain-DD column carries only the penalty that actually sat on
  the table and is the honest verdict, while the gap between the two exposes how much of
  a result is the synthetic double. Rechecked this way, the settle floor stands —
  **+0.178 IMPs/board vul none ([+0.168, +0.187]) and +0.294 vul both ([+0.282,
  +0.306])** on plain double-dummy over 200 000 boards (vs. the perfect-defense
  +0.264/+0.372), so roughly two-thirds of its gain is real defense of partner's
  genuinely doubled contracts.
- **The penalty double of an opponent's 1NT is gated by a configurable shape
  (`DoubleShape`), defaulting to `Any`: every 15+ hand doubles, regardless of shape.**
  The scheme is clean — 15+ doubles, 8–14 with a five-card suit overcalls — and since
  the overcall range stops at 14 (and the double's weight 1.3 outranks the overcall's
  1.0), a 15+ hand has no overcall to make, so it doubles on any shape. A `Balanced`
  (4333/4432/5332) gate was briefly the default after `bba-match --isolate-defense`
  suggested the shapely doubles leaked, but a deeper seat-isolated re-measure put that
  difference within noise (the leak was the *redoubled* doubler's reopened continuation,
  not the shape), so the cleaner `Any` scheme is restored as the default.
  `american::set_natural_double_shape` selects `Balanced` / `SemiBalanced` / `Any`;
  the HCP floor (15+) is unchanged.
- **New 1NT-defense A/B knobs** (defaults unchanged besides the shape flip above):
  `set_natural_double_floor` (HCP floor, default 15), `set_natural_double_weight`
  (logit weight, default 1.3 — drop below the 1.0 overcall to make suit overcalls
  outrank the double), `set_natural_overcall_points` (overcall `points` range, default
  8–14), and `set_notrump_balancing` (extend the defense to the balancing seat
  `(1NT) P P ?`, default off — an A/B showed it loses to the instinct floor's
  passivity on DD). Surfaced in `bba-match` as `--ns-double-shape`/`--ns-double-floor`/
  `--ns-double-weight`/`--ns-overcall`/`--ns-balancing`. These exist because the DD
  isolate-defense measure cannot honestly tune the defense's *competitive* parameters —
  every such lever slides toward "compete less" (the obstruction wall); they are the
  dials for a future single-dummy re-measure.

- **The strong 1NT opening now gauges plain HCP 15-17 instead of Andrews' fifths.**
  The shipped `fifths(15.0..18.0)` gate sat at centre 16.5 — half a point above the
  natural "15-17 HCP" band — so it under-opened 1NT on honor-heavy 15-counts. A
  seed-paired A/B against BBA's 2/1 (`bba-match --filter-1nt`, 20k boards) gives
  plain `hcp(15..=17)` **+0.138 (none) / +0.169 (both) IMPs/board** over the old
  gate, opening ~27% more 1NTs. About half that edge is range bias: a centre-matched
  `fifths(14.5..17.5)` closes the gap to +0.067 / +0.094, so plain HCP wins on its
  own merit too. New `american::set_one_notrump_fifths(true)` restores the
  corrected fifths gauge; `set_open_one_notrump(false)` suppresses the 1NT opening
  outright (a diagnostic hook — those hands then open a minor).

- **`bba-match` gains 1NT-defense isolation tooling.** `--isolate-defense` keeps only
  boards where BBA opens 1NT and our pair defends, scoring each against an all-BBA
  reference table (same BBA opener and responses, only the defender differs) — a
  clean double-dummy measure of our defense, free of the other-table constructive
  confound that `--no-our-1nt` leaves. The we-defend report now splits the swing by
  auction shape (our DIRECT action over 1NT vs the CONTinuation after they respond)
  and dumps the worst we-defend auctions. Finding: with the opener held constant our
  defense is ~neutral on DD (−0.09 none / −0.39 both IMPs/board); the leaks are the
  penalty double and over-competition that gets doubled — the obstruction wall, which
  DD cannot price.

- **Lint hygiene: the crate is now clean under `clippy`, `rustdoc -D warnings`,
  and a `clippy::pedantic` run of `src/`.** No public API change. The one
  behaviour-relevant fix is the Fifths-companion average in `constraint.rs`,
  which now uses overflow-safe `f64::midpoint` instead of `(a + b) / 2.0`.
  Several public defense/EV builders (`defense_to_suit`, `defense_to_weak_two`,
  `advance_double`, `ev_all`) gain `# Panics` docs naming the precondition they
  assume (a suit — not notrump — opening; a legal prior auction). The `pedantic`
  families that are noise on a numerics-heavy engine (integer casts, long match
  tables, similar suit names) are `#![allow]`ed crate-wide rather than rewritten;
  no `clippy::pedantic = "warn"` is added to the repo, so the default CI lint set
  is unchanged.

- **The `examples/` tree is reorganized so user-facing demos stand out from dev
  tooling.** Bare names are now the runnable demos (`american`, `practice-bidding`,
  `render-book`, `average-ns-par`); every development/research harness carries a
  **category prefix** — `ab-` (A/B match), `dump-` (training-data generator),
  `eval-` (hand-evaluator calibration), `probe-` (diagnostic), `bba-` (BBA/EPBot
  benchmark). So `landy-ab`→`ab-landy`, `stayman-abc`→`ab-stayman`,
  `teacher-dump`→`dump-teacher`, `search-dump`→`dump-search`, `check-nltc`→`eval-nltc`,
  `grand-probe`→`probe-grand`, etc. Update any `cargo run --example <name>` invocation
  to the new name (the README and `scripts/` are already updated). The ~310 lines of
  helper code copy-pasted across the A/B harnesses (`next_call`, `bid_out`,
  `bid_uncontested`, `seat_to_act`, `hand_hcp`) now live once in
  `examples/common/mod.rs`, pulled in via `#[path]` (no `main.rs`, so Cargo never
  builds it as a standalone example). Three obsolete BBA spikes — `bba-floor-probe`
  (self-marked throwaway), `bba-conv-probe`, and `bba-oracle` — are removed; their
  work is folded into `bba-match`.

- **The natural penalty double of their 1NT now fires on *any* 15+ hand, not only
  balanced ones.** The authored double was gated `hcp(15..) & balanced()`, so a
  strong *shapely* hand — which qualifies for neither the balanced double nor the
  `8–14` natural overcall — silently *passed* the floor's catch-all. The new
  `set_natural_double_shape(DoubleShape)` knob widens the shape gate (the 15+ HCP
  floor is unchanged): `Balanced` (the historic gate), `SemiBalanced` (also
  5422/6322/7222), and **`Any` — every 15+ hand, now the default**. A/B'd vs the
  balanced-only double (`examples/landy-ab --ns-majors "" --ns-double-shape any`,
  contested seat-swap, plain double-dummy, 500k filtered, ~66k divergent):
  **+0.951 IMPs/divergent (+0.0018/raw deal) non-vul, +1.185 (+0.0022/raw) both
  vul** — every doubler shape is net positive, monotonically more so the longer
  the suit (5422 +0.32/+0.43, 6322 +2.12/+2.67, 7222 +4.89/+6.01,
  one-suiters with 8+ cards +7 to +16 per board). `landy-ab` now also prints an
  **IMPs-won-per-doubler-shape** breakdown (sorted-length buckets), so each shape's
  marginal gain over the balanced baseline reads straight off one run. *Caveat:* the
  baseline **passes** these hands, so this measures double-vs-pass, not
  double-vs-natural-suit-bid — passing a 15+ one-suiter over their 1NT is the worst
  case, which is why even a blunt forced penalty double wins big; letting the very
  strongest one-suiters bid their suit instead is a possible future refinement.
  `set_natural_double_shape(DoubleShape::Balanced)` restores the old behavior.

- **`scoring::ns_score` split into two scorers for two questions** —
  `ns_score_contract` (plain double-dummy, the contract's *actual* penalty) and
  `ns_score_bid` (perfect-defense doubling: a contract that fails double-dummy is
  scored doubled, a making one undoubled, so it takes a `Bid` not a `Contract`).
  The old single `ns_score` was a hybrid (PD-double on failure but honor-the-
  auction-penalty on make) that fit neither job. **Scoring a reached contract** (a
  duplicate A/B result) now honors the penalty actually bid (`ns_score_contract`);
  **evaluating a call** (the `bidding::ev` rollout, contract-choice probes) uses
  perfect defense (`ns_score_bid`). The A/B duplicate harnesses move from PD to
  plain DD — a measurement re-baseline: prior PD-era A/B figures are not directly
  comparable, and findings driven by PD auto-doubling failing overbids (the
  obstruction-wall sweep) soften under plain DD. `stats::average_ns_par` keeps its
  perfect-defense `min(undoubled, doubled)` assumption (par is inherently a
  best-defense concept).

- **Over a `(2♣)` overcall of our `1NT` we now play *systems on*, not Lebensohl.**
  A 2♣ overcall steals no room — every transfer and relay still sits above it — so
  imposing the Lebensohl relay/transfer-through structure was wrong (and bred a
  losing "natural 2♦" escape that has no opener game-raise). Responder now keeps
  the **uncontested** 1NT structure: Jacoby transfers (`2♦`→♥, `2♥`→♠), the minor
  transfers, the 2NT/3-level responses — and shows the now-unbiddable **2♣ Stayman
  with a Double** (X inherits the 2♣ rule's exact logit, so it never drifts). The
  book reuses `notrump_responses()` by rebasing `1NT–(2♣)–…` onto the uncontested
  tree (the 2♣ overcall maps to the opponent's pass; a Double maps to the stolen
  2♣). Lebensohl proper now applies only over `(2♦/2♥/2♠)`, the overcalls that
  actually take away room.
- **Responder's weak natural `2♦/2♥/2♠` escape is now floored at 5 HCP, and opener
  game-raises it** — the relay sign-off's treatment (`lebensohl_relay_shape` +
  `lebensohl_signoff_raise`) extended to the one-level-lower direct escape, since
  they are the same weak 5-card-suit hand. A/B (floored vs unfloored, 300k
  unfiltered, perfect-defense): **+0.012/+0.016 IMPs/board (none/both)**, every
  mechanism positive — the floor sends sub-5 hands to defend (`resp P`, the largest
  share), opener stops overbidding a known-weak signoff (`late P`), and a maximum
  with a fit reaches game (`4♥/4♠`). The level was tuned *after* `(2♣)` went
  systems-on (below), which leaves the natural escape all *majors* — every one
  game-raisable, with no raise-less minor: `5` HCP then beats the relay's `6` by
  +2.5/+2.3 IMPs/divergent (none/both), all-positive, while `4` HCP overbids (the
  game-raises turn negative). One lower than the relay, matching the 2X sitting one
  level lower. Gated behind `set_natural_floor(hcp_floor, points_floor)` for A/B.
- **Opener's Lebensohl sign-off raise is now gauged by points *plus* trump length,
  and calibrated to each floor.** `lebensohl_signoff_raise` previously raised flat
  `17+ points & 3+ support` regardless of which sign-off it answered — so the
  5-HCP-floored *direct* `2X` escape inherited a bar tuned for the relay's 6-HCP
  floor, one point too light for the weaker hand. It now takes the responder floor
  and stretches to 4M when `opener points + trump support` reach a combined target
  of 23 (a Law-of-Total-Tricks dummy adjustment: one point lighter per trump beyond
  three, one point heavier per point of missing responder floor). The relay's old
  `17/3` boundary is preserved exactly and *gains* lighter big-fit raises (`16/4`,
  `15/5`); the natural escape's bare-3-card bar rises to the correct `18` (and
  likewise gains `17/4`, `16/5`). A/B against the flat old rule (both pairs on
  Transfer with the 5-HCP floor, so only the raise gauge differs; 1M filtered, PD):
  **+0.007/+0.006 IMPs/board (none/both), +1.56/+1.39 IMPs/divergent**. The swing
  splits exactly along the two changes: opener *passing* the natural `17/3` hands it
  used to raise (the 17-vs-18 boundary) is worth +1.69/+1.40 IMPs/board — raising a
  17-count opposite a known weak 5-8 overbids — and the length-driven lighter raises
  (`16/4`, `15/5`) add +0.06–1.48/board. (The pass magnitudes are PD-inflated: the
  measure doubles the failing 4M overbids; the direction holds before doubling.)

### Added

- **Multi counter-defense over our `1NT − (2♦)` (opt-in, `set_defense_to_2d_multi`).**
  BBA's 2/1 card defends a 1NT opening with Multi-Landy, whose `2♦` is a *Multi* —
  an unknown single-suited major (confirmed by the probe below). Our default `(2♦)`
  handling (the Transfer/Smolen package) instead reads it as **natural diamonds**,
  which is wrong-sided against a hand that actually holds a major. The new
  `set_defense_to_2d_multi` knob (**default off**) swaps responder's `(2♦)` action
  in `competition.rs` for a Multi-aware set distilled from BBA's own counter
  (`docs/ai-bidder/bba-multi-2d.md`): **`X` = values / takeout** of the unknown
  major (BBA's 41 %-of-the-time workhorse), natural weak `2♥`/`2♠`, forcing natural
  3-level suits (including a natural `3♦` — diamonds is not their suit, so no
  Stayman cue), the shared `2NT` Lebensohl relay, `3NT` to play, else Pass. Wired
  into `examples/bba-match` as `--defense-2d-multi` (pair with
  `--their-conv "Multi-Landy=1"`, so BBA actually bids the Multi). Kept **opt-in**:
  the obstruction-wall prior says competitive/defensive conventions usually do not
  clear plain DD, and much of the Multi-awareness is DD-blind right-siding; a
  large-N A/B is the gate for any promotion. A `tests/american_competition.rs` unit
  test pins the behavior (a values hand doubles only with the toggle on).

- **`examples/probe-bba-constraints` — distill any BBA convention into the DSL.**
  Sample-and-probe against the real EPBot engine (the union of `probe-bba-1nt`'s FFI
  recipe and `probe-extract-constraints`'s renderer): deal random actor hands, drive
  BBA for a fixed `(seat, auction)`, bucket each hand by the call it returns, and
  render each bucket as a candidate DSL `sketch:`. Three `--mode`s read the
  Multi-Landy `2♦` structure — `multi` (the overcaller), `advance` (the
  pass-or-correct relay that resolves the major), `counter` (BBA's own defense to
  the Multi, vulnerability-split). Multi-Landy is forced on all seats so BBA both
  bids and interprets the `2♦` as a Multi. The distilled constraints are written up
  in `docs/ai-bidder/bba-multi-2d.md`.

- **Unusual vs Unusual over our `1NT − (2NT)` (default on).** When an opponent
  overcalls our 1NT with a both-minors `2NT` (e.g. BBA's Multi-Landy), responder
  previously had no authored call and the auction fell to the instinct floor
  (Pass/guess). `competition.rs` §5d now adds a responder structure, gated by the
  new `set_uvu` knob (**default on**):
  - **`X`** — penalty, an (either-or) *suit* penalty: values plus a trick in a
    minor (4+ length or 4+ HCP — AJ/KJ/KQ/AQ — in either minor). Floor
    `set_uvu_x_floor` (HCP, default `9`).
  - **`3♣`** — INV+, Stayman (a 4-card major) or 5+♠; **`3♦`** — INV+, 5+♥. Floor
    `set_uvu_cue_floor` (points, default `8`). Symmetric Smolen after the
    `3♣`→`3♦` denial (`3♥` = 5+♠, `3♠` = 5+♥; neither promises the other major,
    as the denial already killed any 4-4 fit).
  - **`4♣`/`4♦`** — FG+ 5-5-majors splinters (every 5-5 hand is short in exactly
    one minor, so the splinters cover them all; 5-5 never goes through Stayman).
  - weak natural `3♥`/`3♠`, to-play `3NT`, else Pass.

  The opener's answers, Smolen completions and splinter advance reuse the
  existing `(2♦)` Transfer-Lebensohl machinery. An **encircling** penalty chase
  (`instinct.rs`, `set_uvu_encircle`, default on but dormant unless our `X` was
  bid) doubles the opponents' runout from our `X` — every partnership double is
  penalty from the first `X`, and a pass conveys inability to punish.

  Measured (`examples/ab-uvu`, a shape-filtered Rayon self-A/B, plain DD): per
  counter-measure vs the passing floor, **`3♣` +0.67, `3♦` +0.61, `4♣` +2.6,
  `4♦` +2.4 IMPs/board** (vul none; similar at both) — DD-robust, like Transfer
  Lebensohl. Against BBA (seed-paired `bba-match`), the full structure trims the
  `1NT-(2NT)` loss (`+35`/`+32` IMPs over 20k boards, none/both); the subset
  still loses (~`−1.3` IMPs/board — the obstruction wall is single-dummy), but
  the cues recover ~`+1` IMP/board over passing. The penalty `X` is inherently
  rare over a both-minors `2NT` (you cannot stack a suit they hold 5-5), so its
  value lives in the encircling chase, not the immediate double — single-dummy
  territory the DD harness cannot price.

- **Runout when our own both-minors `2NT` overcall is doubled (bug fix).** The
  `set_unusual_notrump_defense` `2NT` (default on) had no authored continuation
  over `[1NT, 2NT, X]`, so a penalty double left the advancer with no escape — the
  auction fell to the floor (Pass) and we hung in a hopeless `2NT` doubled. The
  advancer now always **runs to the longer minor** (`3♣`/`3♦`); it never sits,
  because the doubler holds values behind a 15-17 1NT. This also de-biases the
  `ab-uvu` penalty-`X` measurement: the passive baseline used to sit in `2NT-X`
  and get slaughtered double-dummy, flattering our `X` to a flat ~`+11` IMPs/board;
  with both sides running, the `X`'s value scales with strength (~`+5`/board at the
  default floor) — the honest signal.

- **`examples/ab-uvu` gains `--natural-floor`.** Sweeps the length floor of
  responder's weak `3♥`/`3♠` escape over `(2NT)`. Lowering it to 5 (a five-card
  major escaping a bad defence) measured DD-negative (the marginal escapers lose
  ~1–2 IMPs/board — the obstruction wall), so the default stays 6.

- **`bba-match` gains `--uvu` (+ `--uvu-x-floor` / `--uvu-cue-floor`), `--seed`,
  and a `1NT-(2NT)` focus report; `examples/ab-uvu` is new.** `--uvu` forces the
  UvU structure + encircling on at the given floors; `--seed` makes the deals
  reproducible so an on/off comparison is paired (the boards UvU never touches
  cancel). The new focus report buckets the `[1NT, (2NT)]` divergent boards by
  our response. `ab-uvu` is the Rayon self-A/B (shape-filtered for density,
  sweeps the X / cue floors, per-call attribution). `examples/probe-bba-1nt`
  gains `responder` and `runout` modes that read BBA's own Unusual-vs-Unusual
  handling from real hands (BBA plays `X` = ~11+ values, not suit-specific; its
  suit penalty is a delayed double of the runout).

- **`examples/probe-bba-1nt` — read BBA's actual 1NT defense from real hands.**
  A small probe that feeds crafted archetype hands to the live EPBot engine
  (system 0, the card `bba-match` uses) and prints its direct-seat call over a
  `(1NT)` opening. It exists because the `.so` **ignores the `vendor/bba/*.bbsa`
  cards** (strace: it opens no data file — those drive `BBA.exe`, not the FFI), so
  the compiled-in system can disagree with the config. Concretely it revealed BBA
  defends our `1NT` with **Multi-Landy** (`2♣` = both majors, `2♦` = a one-suited
  major, `2♥`/`2♠` = that major + a minor, `2NT` = both minors, balanced hands
  pass) — even though `21GF.bbsa` labels the card `Cappelletti=1`. The
  `create → set_system → new_hand → set_bid → get_bid` recipe generalizes to
  verifying any BBA convention from real hands.

- **`examples/bba-match` gains a `--filter-1nt` flag and a per-subset 1NT
  report.** To answer "how does our `1NT` (opening and continuations) stack up
  against BBA?", the harness now isolates the `1NT` territory of the duplicate.
  It splits the divergent boards into two subsets — **our `1NT` openings** (we
  open `1NT`) and **our defense vs their `1NT`** (they open, we compete) — and
  reports IMPs/board for each, broken down by our first call (Stayman / transfer
  / Lebensohl / penalty `X` / …) so a leak localizes to a single continuation.
  Bucketing keys on table A, where our pair always sits North/South. The optional
  `--filter-1nt` pre-filter keeps only deals with a balanced 15-17 HCP hand
  somewhere (a `1NT`-opener candidate), raising the yield of `1NT` boards;
  `--count` then counts kept boards. Default off — runs without the flag are
  unchanged, and the report is purely additive.

- **A natural runout when our `1NT` is doubled (`[1NT, (X)]`), on by default.**
  The instinct floor had no agreement here, so responder fell to the catch-all
  **Pass** — sitting a hand that may be broke for an effectively-penalty double,
  the `−500`/`−800`/`−1100` disaster a runout exists to prevent. The floor now
  runs a natural escape, the structure mirroring our own defense to *their* `1NT`
  (the penalty double makes the doubler "a `1NT` opener"). It is **universal** —
  the whole partnership runs out, not just the weak responder:
  - **`2♣`/`2♦`/`2♥`/`2♠`** = natural, weak, to play — escape to the longest
    five-plus-card suit (longer suits and majors preferred);
  - **`2NT`** = unusual, both minors (4-4, no five-card suit to run to): opener
    names the better minor;
  - **direct Redouble** = values, to play `1NT` redoubled — keyed on raw
    (defensive) HCP at or above the `set_runout_xx_min` floor (default `7`);
  - **opener escapes too** — in the balancing seat (`1NT-X-P-P`), a minimum-ish
    opener with a five-card suit runs it, or **SOS-redoubles** (the *balancing*
    redouble) with none, forcing responder to bid its longest suit (four-card
    suits included);
  - **whoever ran is captain** — partner passes the escape / SOS answer, at a
    weight that *outranks the `1.5` transfer completion*, so a `2♦`/`2♥` escape
    is never misread as a Jacoby transfer and "completed" into the wrong suit.
  The double need not be penalty: left in, any double of `1NT` plays for the
  penalty, so the runout fires over a conventional double too. Knobs (all
  per-thread): `set_one_nt_runout(bool)` (the whole runout), `set_runout_xx_min(u8)`
  (the run/redouble HCP boundary), and `set_one_nt_runout_universal(bool)` (opener
  escape + SOS vs responder-direct only); the new `examples/ab-one-nt-runout`
  harness A/B's them (seat-swap duplicate, plain double-dummy, `--show`,
  `--xx-min`, `--no-universal`). Measured vs the old Pass floor (500k–1M boards):
  the responder-direct escape alone is **+2.43 IMPs/divergent non-vul, +4.95 both
  vul**; the redouble's marginal value is monotonic the lower the `xx-min` floor
  (probed `6`–`12`), best near `7`; and the universal layer adds a further
  **+1211 IMPs non-vul, +2048 both** over direct-only (500k, `xx-min 7`). The full
  default system runs **`+0.011` IMPs/raw deal non-vul, `+0.020` both vul**, the
  vul edge ≈ 2× non-vul (escaping a doubled penalty). The suit escapes are
  double-dummy-*robust* (fleeing a doubled penalty wins under any measure); the
  redouble and the SOS 4-3-fit rescues lean on double-dummy declarer play, so the
  `xx-min` floor sits at `7` (not lower) as a hedge and the universal layer is a
  candidate to re-confirm under a future single-dummy measure. The both-minor
  action and the penalty double of the opponents' escape are now tunable — see
  the next two entries.

- **The 4-4-minor bust now runs *direct* to its longer minor, not through `2NT`.**
  The Phase-1 `2NT` scramble (relay to opener's better minor) kept the auction
  alive an extra round and landed at the three level — two fresh chances for the
  opponents to double, and a level higher. The new default (`Unusual2nt::Direct`)
  skips the relay: the bust bids its own longer minor (ties to diamonds) at the
  two level, one double-exposure instead of two. A/B'd vs the relay
  (`examples/ab-one-nt-runout --compare direct`, seat-swap, plain double-dummy,
  2M boards × two seeds): **+0.6–0.7 IMPs/divergent non-vul, +2.0–2.2 both vul**,
  the most frequent runout-shape axis. The relay survives as opt-in
  `set_unusual_2nt(FourFour)`; a third mode, `FiveFiveAdd` (route 5-5 minors
  through `2NT` so opener picks the better fit), A/B'd a clear **loss** (−4.5/−8.4
  IMPs/divergent) and stays off. Knob: `set_unusual_2nt(Unusual2nt)`.

- **We now double the opponents' escape from our (re)doubled `1NT` for penalty
  (default on).** When they run from our `1NT-X` (the advancer pulls partner's
  penalty double) or our `1NT-X-XX` (they flee the business redouble), the floor
  used to take the run out as if it were a takeout double. It now *doubles them*
  — and keeps doubling as they keep running (the chase recurses) — with partner
  leaving the double in rather than advancing it. Two arms, each a per-thread
  knob: `set_penalize_escape_stack(bool)` (a trump stack — 4+ cards, two top
  honors — in their suit, sound in any seat) and `set_penalize_escape_values(bool)`
  (general values once responder's business redouble has shown them, no personal
  stack). A/B'd (`--compare escape-stack` / `escape-values`, 2M × two seeds):
  **+5–7 IMPs/divergent across both arms and both vulnerabilities**, never
  negative — but rare (the opponent bots seldom escape, so the per-board figure is
  ≈ `0`; real opponents run more, so the harness understates the frequency). The
  doubled penalties are scored as bid, so this is double-dummy-visible, not the
  obstruction-blind trap. The `ab-one-nt-runout` harness gains a `--compare` axis
  (`runout` | `escape-stack` | `escape-values` | `minors5` | `direct`) that flips
  one feature between the two tables, holding the rest at baseline.

- **The Landy advancer now has responses to a doubled `2♣` (`[1NT, 2♣, X]`).**
  When we overcall their `1NT` with Landy `2♣` (both majors, short clubs) and the
  opponents double — the stolen `2♣` Stayman — their opener can sit for `2♣`
  doubled with good clubs (the `set_penalty_pass` conversion shipped just above).
  Previously the advancer had no node there and **passed the floor**, leaving us
  declaring `2♣` doubled in a both-majors / short-club misfit. The advancer now
  runs a **richer escape that the Double's extra step (the Redouble) pays for**:
  - **Redouble** = equal majors, "you pick" — the relay the undoubled `2♦` was;
  - **Pass** = a long club one-suiter (play `2♣` doubled, the doubler walked in);
  - **`2♦`** = a long diamond one-suiter, natural and to play (the freed bid);
  - **`2♥`/`2♠`** = the longer major (weak signoff); strong arms (`4M`, `2NT` ask,
    `3M`) are unchanged. The minor escapes (Pass / `2♦`) require *6+ in the minor and
    both majors ≤2* — since the overcaller already promised 9+ cards in the majors a
    long-minor/short-major advancer is the rare hand with no major fit, and a 3-card
    major would have an 8-card fit beating a doubled minor. Overcaller's rebids: name
    the longer major over the Redouble relay; pass (or pull a singleton diamond to a
    major) over the natural `2♦`; answer the game ask over `2NT`. The `(6, 2)` gate is
    a new A/B knob, `set_doubled_landy_escape((min_minor, max_major))` /
    `examples/landy-ab --ns-doubled-escape MIN:MAJ`.
  Effect on the (opt-in, off-by-default) Landy defense, measured in the shipped
  `set_penalty_pass(4:4:major)` world (`examples/landy-ab --ns-penalty-pass
  4:4:major --ew-penalty-pass 4:4:major`, 40k filtered, ~4.1k divergent): the
  doubled-`2♣` leak the penalty-pass revamp had opened against Landy is closed —
  the Landy-vs-natural figure goes from **−0.946** (no responses) to **−0.156**
  (simple escape) to **−0.138 IMPs/divergent** with the Redouble/natural-`2♦`
  refinement (a clean paired +0.018/divergent on identical boards), `2♣`-action
  row **−1.098 → −0.162 IMPs/action-board**. Landy stays mildly DD-negative (the
  known obstruction-blindness wall, see the `1NT` defense notes) so it remains
  opt-in/off, but the misfit disaster is gone. The gate was A/B-swept (100k filtered,
  same-seed paired) over `MIN ∈ {5,6,7} × MAJ ∈ {1,2}`: `6:2` is the best of a tight
  field (it beats the `7:2`/`5:1`/`6:1`/`7:1` cluster by only ≈0.0006 IMPs/board —
  noise, as expected when the escape is this rare), and the lone clear signal is that
  `5:2` is distinctly worst (a 5-card minor with 2-2 majors over-escapes into bad
  doubled spots). Live-search note: the escapes are normal advancer bids; the rare
  `Pass`-to-defend is terminal with no bid to decode.

- **Opener can now convert the systems-on Double of a `(2♣)` overcall to penalty
  with good clubs.** Over our `1NT`, a `(2♣)` overcall is *systems on* and
  responder's Double is the stolen `2♣` Stayman — but opener was forced to *answer*
  it (`2♥/2♠/2♦`) and could never sit, so `1NT–(2♣)–X–(P)` left a big penalty on the
  table when opener held length and strength in clubs behind the overcaller (our
  23+ combined HCP routinely sets a vulnerable `2♣` doubled multiple tricks). Opener
  now **passes** that Double — defending `2♣` doubled — when holding the
  `set_penalty_pass` gate, authored as a context-specific fallback at the
  `[1NT, 2♣]` node (so it is reached before the systems-on rebase and never leaks
  onto the shared *uncontested* forcing Stayman, which still never passes).
  **Default `(4, 4, true)`:** 4+ clubs with 4+ club HCP (an ace or two honors),
  converting even when responder's Double promised a 4-card major (good clubs beat
  the fit). A/B'd a clear win at every gate (`examples/landy-ab --ns-penalty-pass
  4:4:major`, contested seat-swap, Landy off both arms, 2M boards, 594 divergent):
  **+5.35 IMPs/divergent non-vul, +7.28 both vul on plain double-dummy; +5.32 /
  +7.09 under perfect-defense** scoring — the two scorers agree because converting
  is a pure penalty decision (we defend a reached `2♣x`), not an obstruction or
  overbid the measures treat differently. Bigger vulnerable, as expected (a doubled
  set of a vulnerable overcaller scores more). The whole-game effect is tiny
  (+0.002 IMPs/raw deal — the auction is rare, ~0.03% of deals) but strongly and
  robustly positive per conversion, and every gate from the default down to
  `(4, 0, true)` (any 4 clubs) — and even 3-card clubs — stays net positive on DD,
  so the gate trades frequency for a defensible "good clubs" holding rather than
  guarding a losing region. `set_penalty_pass(None)` restores the prior behavior;
  `set_penalty_pass(Some((len, hcp, over_major)))` retunes the gate.
  `examples/landy-ab` gained `--ns-penalty-pass` / `--ew-penalty-pass
  LEN:HCP[:major]`. **Side effect on the opponents' overcall:** re-measuring the
  natural `2♣` overcall of our `1NT` once the opener can punish it (`--ew-always-pass
  on --ew-penalty-pass 4:4:major`, the `2♣`-action row), its value drops from
  **+0.944 → +0.552 IMPs/action-board non-vul (−42%)** and **+1.183 → +0.662 both
  vul (−44%)** — the conversion claws back roughly two-fifths of what a natural `2♣`
  overcall used to gain.
  *(Live-search note: the conversion is a terminal Pass with no partner bid to
  decode, so `american_search` prices it directly from the book — no inference work
  needed.)*

- **A passed hand now reassigns its dead penalty double of their 1NT to both
  majors (new default behavior).** A passed hand cannot hold the 15+ HCP a penalty double of
  their 1NT needs, so over `[P,P,P,1NT]` (RHO opens 1NT in fourth seat) the
  natural double is dead weight. `set_passed_hand_defense(Some(
  PassedHandDefense::NaturalLandyDouble))` keeps every natural overcall but
  reassigns that freed double to show both majors (≥5-4, `points(6..)`, **neither
  major six-plus** — a six-card major would have opened a weak two in first seat,
  so it shows that suit with the natural overcall instead), advanced via the
  existing Landy machinery (the advancer — also a passed hand — signs off in a
  two-level major; the invite/game/2NT-ask arms are unreachable below opening
  strength). Gated on `passed_hand()`, so the direct-seat penalty double is
  byte-identical. A/B'd vs leaving the double dead (`examples/landy-ab
  --ns-passed-dbl landy --ns-majors "" --ns-minors ""`, contested seat-swap, 200k
  filtered, ~2.2k divergent): **+1.12 IMPs/divergent non-vul, +1.25 both vul on
  plain double-dummy; +1.27 / +1.25 under perfect-defense scoring** (`--score
  pd`). Unusually for a conventional 1NT defense — Landy, DONT, and Meckwell all
  *lost* on double-dummy — this one *wins*, and wins *at least as much* under
  perfect defense, not less: a passed hand's penalty double (the one DD-visible
  weapon that made natural beat every convention) is impossible, so reassigning it
  costs nothing and adds a pure two-suited fit-find, which is DD-visible; perfect
  defense additionally punishes the baseline's single-suit overcall when it
  overbids into a doubled misfit. The whole-game effect is tiny (+0.0004–0.0005
  IMPs/raw deal — the auction is rare) but strictly positive at both
  vulnerabilities and both scorings. **On by default** (now that it clears both
  the plain-DD and perfect-defense bars); `set_passed_hand_defense(None)` restores
  the historic dead double. `examples/landy-ab` gained `--ns-passed-dbl` and a
  `--score plain|pd` knob (the latter re-scores any A/B under perfect-defense
  doubling, to catch a plain-DD positive that is really an under-punished overbid).
  *(Live-search note: the inference reader does not yet decode the passed-hand
  double, so `american_search` under-narrows that rare leaf — safe, just not yet
  exploited; the production `american` bidder reads it straight from the book.)*

- **Full DONT is available as a second passed-hand 1NT defense (opt-in, *not*
  default).** `set_passed_hand_defense(Some(PassedHandDefense::Dont))` gives a
  passed hand the namesake convention: `X` = a one-suiter (advancer relays `2♣`,
  the doubler then names it), `2♣` = clubs + a higher suit, `2♦` = diamonds + a
  major, `2♥` = both majors — every advance a two-level pass-or-correct signoff,
  since both partners passed. The motivation: a passed hand cannot *preempt* a
  two-suiter (our only preempts are one-suited weak twos / three-level openings),
  so DONT's two-suiter coverage targets exactly the shapes that had no first-seat
  voice. **Measured worse than `NaturalLandyDouble`, though**: vs leaving the
  double dead it is +0.47 / +0.45 IMPs/divergent on plain double-dummy but
  **+0.05 / +0.01 under perfect defense** — the plain-DD edge is almost entirely
  the under-punishment of overbids (`ns_score_bid` doubles them away). DONT acts
  on far more, and weaker, hands than the disciplined both-majors double, and most
  of that extra activity is overbidding; `NaturalLandyDouble` survives perfect
  defense (+1.25 / +1.25) precisely because it is narrow and uses the `X`→`2♦`
  relay to find the 4-4 major fit, where DONT's `2♥` can land in a 4-3. Kept
  opt-in for a future single-dummy re-measure (obstruction and lead-direction are
  DD-blind). `examples/landy-ab --ns-passed-dbl dont`.

- **An always-pass defense to their 1NT as a true do-nothing baseline.**
  `set_always_pass_defense(true)` authors only `Pass` at the `[1NT]` node (a finite
  logit for every hand shadows the instinct floor), so our side never competes
  over their 1NT — distinct from `set_natural_defense(false)`, which falls to the
  floor (and the floor still competes a little). This isolates the *full* value of
  having any 1NT defense. A/B vs the natural defense (`examples/landy-ab
  --ns-natural on --ew-always-pass on --ns-majors "" --ns-minors ""`, plain
  double-dummy, 200k filtered): natural beats always-passing by **+0.566
  IMPs/divergent (+0.0166/raw deal) non-vul, +0.609 (+0.0178/raw) both vul**. The
  ordering is `always-pass < floor < natural`: per raw deal natural is *further*
  ahead of always-passing (+0.0166) than ahead of the floor (+0.0104), so the
  floor's own competition over their 1NT is worth something — always-passing is
  the worst option. `examples/landy-ab` also now prints an **IMPs-won-per-natural-
  defensive-action breakdown** (`X` / `2♣` / `2♦` / `2♥` / `2♠`), attributing each
  divergent board's swing to the overcall or double that caused it. **Every action
  is net positive vs always-passing** (IMPs/action-board, non-vul / both-vul):
  `2♣` +0.98 / +1.21, `2♥` +0.78 / +0.86, `2♠` +0.52 / +0.54, `2♦` +0.30 / +0.17,
  `X` (penalty double) +0.20 / +0.44 — the natural minor/major overcalls carry
  most of the value, the penalty double the least per board but still a gain.

- **The natural defense to their 1NT is now distilled-confirmed, A/B-validated,
  and toggleable.** The authored natural defense (penalty double 15+ balanced,
  natural two-level overcalls on a five-card suit `8–14` points) had never been
  measured standalone. Probing the distilled net over `(1NT)`
  (`examples/extract-constraints --auction "1NT"`) reproduces the authored ranges
  almost exactly — `hcp(15..=21) & balanced()` for the double, `hcp(7..=14) &
  len(suit, 5..)` per overcall (the net's `hcp(7)` floor matches the authored
  `points(8)` once the five-card length point is counted) — so no constants
  changed. A new `set_natural_defense(bool)` toggle (on by default) drops the
  whole natural arm so the `[1NT]` node falls to the bare instinct floor, enabling
  a standalone contested A/B (`examples/landy-ab --ns-natural on --ew-natural off`,
  plain double-dummy, 200k filtered): the natural defense is a **clear win vs the
  floor — +0.744 IMPs/divergent (+0.010/raw deal) non-vul, +1.276 (+0.018/raw)
  both vul**. Unlike pure obstruction (Lebensohl-vs-floor measured negative), a
  natural defense adds DD-visible constructive value — fit-finding, sacrifices, and
  penalty doubles that cash — so it survives the obstruction-blind measure. Kept on
  by default; `set_natural_defense(false)` reverts to the floor.

- **Unusual `2NT` (both minors) over an opponent's 1NT — now the default
  defense.** A natural `2NT` over their strong notrump is nearly worthless, so the
  bid is repurposed as a both-minors (5-5) two-suiter, `8–13` points, advanced by
  picking the longer minor. Purely additive — it sacrifices no natural call. A/B'd
  vs the bare floor (`examples/landy-ab --ns-minors`, contested seat-swap, plain
  double-dummy, 100k filtered): a vulnerability-dependent wash (≈+0.0001 IMPs/board
  non-vul, ≈−0.0001 vul), shipped on because it is additive and its
  obstruction/lead-direction value is invisible to the DD measure (same call as the
  shipped takeout responsive double). Best-measured settings: the 5-5 shape and the
  `13` ceiling both helped (capping strong hands, which do better doubling/passing),
  and `points` beat `hcp` for the strength gauge. `set_unusual_notrump_defense(None)`
  reverts to the floor's natural `2NT`.

- **Landy `2♣` (both majors) over their 1NT — opt-in, off by default.**
  `set_landy(Some((lo, hi)))` turns `2♣` into both majors (at least 5-4), on
  `points(lo..=hi)` — replacing the natural `2♣` club overcall (a club one-suiter
  then passes or doubles). The advancer's responses are authored per the canonical
  structure (`2♦` = equal majors weak, correct to the longer; `2♥/2♠` = preference
  signoff; `2NT` = game-forcing ask; `3♥/3♠` = invitational; `4♥/4♠` = to play),
  and the overcaller answers both the `2♦` relay and the `2NT` ask (the min/med/max
  × 5-4/5-5 rebid ladder). Response strengths and the rebid buckets **track the
  configured `2♣` range** (a lighter overcall asks more of the advancer; anchored
  so `lo = 10` reproduces the textbook 10–12 invite / 12+ force). **Off by
  default** — A/B'd it loses at every floor (it gives up the natural club overcall —
  the obstruction-blind-DD wall), so it stays opt-in for a future single-dummy
  re-measure. `set_landy_hcp(true)` gauges either two-suiter's range on raw HCP
  instead of shape-upgraded points (default points; points measured better). The
  inference reader decodes both two-suiters (suppressing the `2♣` club and `2♦`
  relay natural readings) so the live-search bidder conditions partner correctly.
  New `examples/landy-ab` is the contested seat-swap A/B
  (`--ns-majors`/`--ns-minors LO[:HI]`, `--strength points|hcp`).

- **Responder's double of an overcall (`1NT–(2♦/2♥/2♠)–X`) is now a takeout
  double (`≤3` in their suit, `8+` HCP) by default**, replacing the old penalty
  double (`4+/9`). Selected via the `DoubleStyle` toggle (`set_double_style`,
  default now `Takeout`); penalty and the other meanings stay opt-in. Isolating
  just the double on the A/B harness (both pairs Transfer, NS varies the style,
  200k, plain double-dummy, none/both), the penalty double is monotone-bad: every
  *extra* penalty double loses (`4+/8` −0.001/−0.002, `4+/7` −0.002) and every
  *removed* one gains (`4+/11` and `5+/9` +0.002/+0.003), while takeout beats
  penalty outright (`≤3/8` **+0.004/+0.005**, `≤3/7` +0.004/+0.005; the floor moves
  to `8` because `≤3/9` already loses on the double itself and `≤3/8` has the
  cleanest per-board gain). Against the bare floor the change converts the single
  worst response bucket — the penalty double leaked −4.0/−4.8 IMPs/board — into a
  small winner (+0.12/+0.15) and erases the penalty-*pass* leak (short-suit values
  hands now double instead of passing), lifting the whole package +0.004/+0.005
  IMPs/board (`+0.010/+0.011` vs floor). **Measure caveat:** under perfect-defense
  scoring the flip reverses — PD auto-doubles the takeout overbid, so penalty wins
  there (`Takeout` ≤3/7 −0.089/−0.092); the measure-robust part is only that the
  *marginal* penalty double (4-card, 8–10 HCP) is a net loser. The shipped A/B
  scorer is plain DD, so takeout is the default; re-measure under a single-dummy
  scorer before treating it as final. Opener needs no new continuation — the
  instinct floor pulls the double (or passes with a trump stack). *(Known weight
  interaction: the takeout double, 1.55, outranks direct 3NT, 1.5, and the
  top-step clubs transfer, 1.45, so short-suit one-suiter / stopper-game hands are
  pulled into the double; the X bucket stays net-positive even so, but lowering the
  takeout weight below the constructive bids is a deferred refinement to A/B.)*
- **Responder *traps* with a too-good stopper instead of declaring `3NT`
  (`set_trap_pass`, on by default).** A direct `3NT` over the overcall now denies
  **5+ HCP in the opponents' suit** (`suit_hcp(over, ..=4)`): a strong holding
  (♥AQ86, ♥AQ754) passes and waits for opener to reopen with a takeout double,
  converting it to penalty, while an adequate-but-not-too-good stopper or a long
  *weak* source of tricks (♠A9642, 4 HCP) still bids `3NT`. The `5`-HCP threshold
  is **distilled from a per-board double-dummy oracle** (`lebensohl-ab --pd-3nt
  --log-relay`, which compares `3NT` vs trapping over sampled layouts): the trap
  rate rises monotonically with HCP *in their suit* (4 → 53%, 5 → 77%, 6+ → ~100%)
  and is independent of length — a length-based gate gets it backwards. A/B (vs
  off, isolated, 200k plain DD): the 1NT-Lebensohl responder gains **+172/+185
  IMPs** (none/both — the prior `resp 3NT` losers, −22/−20, are erased) at a
  near-wash in the shared advance-of-a-takeout-double context; net **+155/+230**.
  New general constraint `constraint::suit_hcp(suit, range)` (suit-specific HCP).
- **A/B knobs on `lebensohl-ab` for tuning the double and 3NT.** `--ns-dbl/--ew-dbl`
  now also accept a parametric spec `LEN:HCP` (`set_double_override`) — `LEN` is
  `LO-HI`/`LO+`/`LO` in their suit, `HCP` the floor (e.g. `4+:9` = penalty,
  `0-3:8` = takeout) — sweeping the penalty/takeout boundary as a continuum instead
  of the four named styles. `--ns-3nt-stopper on|off` (`set_direct_3nt_stopper`)
  drops responder's own-stopper requirement for a direct `3NT`, leaning on opener's
  1NT; measured ≈ neutral (+0.001 none / −0.000 both vs needing a stopper), so the
  default keeps the stopper. `--ns-trap on|off` (`set_trap_pass`) toggles the trap
  pass above; `--pd-3nt` is the double-dummy oracle that distilled it (`--log-relay`
  emits `THREENT` decisions); `--show-bucket "<label>"` dumps every board in a
  `--diverge-diff` bucket with the deal, both auctions, and the DD makes grid.
- **`lebensohl-ab --diverge-diff`: per-call attribution of the A/B swing.** Buckets
  every divergent board by the measured (`--ns`) pair's *first* call the baseline
  (`--ew`) would not have made — tagged `resp` (responder's action directly over
  `1NT–(2X)`) or `late` (e.g. opener completing a transfer) — and reports
  boards/IMPs/contribution per call (the `contrib` column sums to the headline
  IMPs/board). Isolates which call drives the result. Finding (transfer vs the
  bare floor, 200k unfiltered, none-vul, perfect-defense): the penalty double is
  the single worst call (−5.05 IMPs/board × 201 boards, −0.005/board), the weak
  natural 2-level escapes also lose, and the `2NT` relay + `3♦`/`3♥` transfers are
  the positive drivers — i.e. the competitive outlets, not the constructive ones,
  carry the PD loss against the floor.
- **`lebensohl-ab --pd-natural`: PD-gate + distill for the weak natural escape.**
  Mirrors `--pd-relay` for responder's natural `2♦/2♥/2♠` over the overcall
  (double-dummy compare bidding vs defending; `--log-relay` emits `NATURAL` lines).
  Distill (12k filtered boards, 64 layouts, perfect-defense): unlike the `2NT`
  relay, the weak natural has **no rescuing floor** — bidding loses to defending at
  *every* HCP 0–8 (mean EV margin −34 to −119 score points, never positive) and at
  every suit length (6-card −34, 5-card −53). The least-bad slice is a 6+ length
  gate, but even that stays PD-negative — there is no HCP crossover to distill.
  Per the standing DD-blind-to-obstruction caveat, this is **not** taken as a
  signal to floor or drop the escape (its obstructive value is invisible to
  perfect defense); deferred to a single-dummy measure.
- **Plain Lebensohl gains a direct cue-bid (Stayman) and good-5 sign-offs, to
  compete on even terms with Transfer Lebensohl.** After `1NT` is overcalled and
  `LebensohlStyle::Plain` is selected, responder's cue of the opponents' suit is
  now game-forcing Stayman with a 4-card unbid major and no 5-card suit of its
  own (the cue outranks a direct `3NT`, so a 4-4 major fit is found even with a
  stopper); opener answers it via the existing cue-Stayman machinery. Previously
  such hands could only bid `3NT`, never finding the major fit. The weak `2NT`
  relay now also admits a 5-card suit below the overcall — relay then correct
  `3♣`→`3M` as a 3-level sign-off — instead of requiring 6+, **gated by a 6+ HCP
  floor**; and a stack in *their* suit no longer wrongly relays (it is a penalty
  pass). The 5-card relay, the HCP floor, and the their-suit exclusion apply to
  **both** Plain and Transfer Lebensohl; the new cue is Plain-only (Transfer
  already cues). The 6-HCP floor is **PD-distilled**: a perfect-defense gate
  (`lebensohl-ab --pd-relay`, double-dummy comparing relay vs defend per board)
  beats blanket all-5-card relaying by +0.023 to +0.032 IMPs/board, and a plain
  `hcp(6..)` floor — adverse-suit values were *not* predictive; overall weakness
  is the driver — recovers ~60–80% of that gain with zero runtime cost (pushing a
  near-bust to the 3 level loses on DD even with a 6-card suit). A/B steps
  (lebensohl-ab, vs floor): any-5 beats good-5 (two top-three honors) by +0.010
  to +0.022, then the HCP floor adds the PD gain on top. *(Note: advancing a
  takeout double reuses the Plain responder table, so the advancer can now bid the
  cue, but the doubler's reply there is still floored — the point/shape re-tuning
  for the lighter, shapier double is deferred to a future session.)*

- **A maximum 1NT opener can now stretch a weak Lebensohl sign-off to game.**
  After responder relays `2NT` weakly and corrects `3♣`→`3M` (a 6–9 sign-off in
  a major, the floor above), a *maximum* opener (17, in the 15–17 range) holding
  three-card support raises to game instead of passing — the relay's 6-HCP floor
  makes the combined count high enough to reach `4M` on a long-trump dummy.
  Applies to both Lebensohl styles, majors only (a minor's game is the 5 level,
  out of reach for a 6–9 hand). A/B (lebensohl-ab, 500k unfiltered, vs floor):
  +0.0010 to +0.0012 IMPs/board across plain/transfer × non-vul/vul — a rare node
  (the sign-off-to-major sequence only arises over a `2♠` overcall) but positive
  in every cell and never negative. *(Re-confirmed that the relay floor belongs in
  raw HCP, not points: a perfect-defense crossover analysis shows the
  relay-vs-defend boundary is sharp at 6 HCP — 21%→80% — but mushy on the
  shape-upgraded points scale, because the upgrade drags light, shapely
  defend-hands into the relay bucket. The driver is defensive values, not playing
  strength.)*

- **Transfer Lebensohl now *recognizes* a partner's delayed cue, and can
  optionally bid it (ledger #106).** Larry Cohen's stopper-split cue: a *delayed*
  cue (relay through `2NT`, then the opponents' suit) is Stayman *with* a stopper
  in their suit and exactly a 4-card unbid major (denying 5 — Smolen / Leaping
  Michaels keep those), versus the plain *direct* cue. Two layers, split so the
  shipped system is byte-identical in self-play:
  - **Recognition is on by default.** Over `(2♥)`/`(2♠)`, in both the `1NT`-overcalled
    and the `(2X)–X–(P)` advance contexts, the bot now answers a delayed cue (show
    the other major at game with a fit, else `3NT` — partner's stopper makes it
    safe), so a human partner who plays the convention gets a sensible reply. The
    bot never *bids* the delayed cue itself, so this node is dormant in bot-vs-bot
    play and changes no measurement; it only activates opposite a partner who bids it.
  - **Bidding it is opt-in**, behind the new
    [`set_delayed_cue`][pons::bidding::american::set_delayed_cue] toggle (default
    `false`); `--delayed-cue` on the `sohl-after-double-ab` example. When on, the
    bot also routes its stopper hands through the delayed cue and reads its own
    direct cue as denying a stopper (running no-stopper, no-fit hands to a minor
    game over a stopperless `3NT`). Isolation A/B (delayed-cue-`Transfer` vs
    plain-`Transfer`, perfect-defense, 200k filtered boards/cell): **+0.000 / +0.001
    IMPs/board (none/both)** on ~0.4 % divergence — **dead flat, so it stays
    opt-in, not default.** Stopper hands reach the same contract fast or slow, and
    the real value of showing a stopper (concealment, right-siding the notrump) is
    single-dummy, which the double-dummy / perfect-defense measure looks through —
    the same wall that sank the reverted `TransferSmolen` and the removed `Rubensohl`.

- **Transfer Lebensohl now plays a richer structure over a `(2♦)` overcall of our
  `1NT` (ledger #80); behavior over `(2♥)`/`(2♠)`/`(2♣)` is unchanged.** When an
  opponent overcalls our `1NT` with `2♦`, responder now plays more than bare Cohen:
  `3♣` is game-forcing Stayman with a Smolen continuation
  (`1NT–(2♦)–3♣–P–3♦–P–3♥/3♠` shows 5-4 majors), the 3-level transfers shift down to
  direct Jacoby (`3♦`→♥, `3♥`→♠, `3♠`→♣ — the club leg a *forced* game-force, since
  its `4♣` completion leaves `3♣` unplayable), and `4♦`/`4♣` are Leaping Michaels
  (both majors 5-5 / clubs + a 5+ major). Over a `2♥`/`2♠`/`2♣` overcall it is
  byte-identical to bare Cohen. This `(2♦)` package is part of the default
  [`LebensohlStyle::Transfer`][pons::bidding::american::LebensohlStyle]
  ([`set_lebensohl_style`][pons::bidding::american::set_lebensohl_style]); `Plain`
  remains opt-in. A/B of the package vs bare Cohen-over-`(2♦)`
  (`examples/lebensohl-ab`, perfect-defense `ns_score`, 200k filtered boards/cell):
  **+0.020/+0.024 IMPs/board, +2.286/+2.822 IMPs/divergent (none/both)** — a clean
  win that reverses an earlier, reverted standard-Stayman+Smolen hybrid
  (−1.31/−1.76/div). The gain is genuine fit-finding the double-dummy / perfect-defense
  measure can credit (5-3 major games through Stayman+Smolen, 5-5 major games through
  Leaping Michaels), not the DD-blind right-siding that sank the earlier attempt.
- **Transfer Lebensohl's top step is now a forcing transfer to clubs (ledger #80).**
  Cohen's transfers run *up the line through* the adverse suit, so the highest 3-level
  step has no suit above it and wraps back to clubs: `1NT–(2♦/2♥)–3♠` and `1NT–(2♠)–3♥`
  are now a *forced* game-force transfer to clubs (6+♣, game values, no stopper in
  their suit; opener completes `3NT` with a stopper, else `5♣`). These previously fell
  to the natural instinct floor, leaving a 6+♣ game-forcing hand with no call — the
  weak `2NT`→`3♣` relay is limited to ≤8 points and can't carry a game force. Applies
  to `Transfer` over every overcall, and to the `(2♦)` Smolen leg's own `3♠`→♣
  transfer. Perfect-defense A/B
  (two binaries at a fixed `--seed`, `transfer` vs the bare floor, 200k
  filtered/cell): **−0.0008/−0.0012 IMPs/board (none/both)** — a tiny, consistent loss
  confined to ≈0.04% of boards. Those boards are normal making games (`3NT`/`5♣`) that
  double-dummy scores below the floor's *speculative penalty doubles of the overcall*
  under perfect defense — the harness's known blindness to competition/obstruction
  (cf. Lebensohl-vs-floor #80), not a transfer misjudgment. Kept in the default as a
  theory-correct completion, pending a single-dummy re-measure. `examples/lebensohl-ab`
  gains `--seed` (deterministic two-binary runs) and `--only-topstep` (restrict to
  top-step boards).
- **Responsive double re-measured under perfect defense (ledger #100); two opt-in
  toggles, defaults unchanged.** The shipped responsive double after partner's
  *takeout* double and their raise (`(1t)–X–(2t)–X` — the canonical convention, and
  BBA's single `Responsive double` toggle, on in `21GF.bbsa`) is now gated by
  [`set_responsive_takeout`][pons::bidding::american::set_responsive_takeout]
  (default **on**), and a non-standard *overcall* extension (`(1t)–overcall–(2t)–X`,
  nearest to BBA's `Snapdragon Double`, off in 21GF) by
  [`set_responsive_overcall`][pons::bidding::american::set_responsive_overcall]
  (default **off**). The new `examples/responsive-ab` A/B (200k filtered/cell,
  perfect-defense `ns_score`) measures both against the bare instinct floor: takeout
  **−1.18/−1.89 IMPs/divergent** (−0.0003/−0.0006 per raw deal, none/both),
  overcall-ext **−2.16/−3.53** (−0.0020/−0.0032 per raw deal). Both stay as they
  were — the overcall extension remains rejected (the new scoring does not rescue the
  reverted −0.034/−2.37 result; it is slightly *worse* vulnerable, as perfect defense
  punishes the doubled-down overbids), and the takeout part stays shipped: its per-deal
  drag is negligible and its competitive/obstruction value is invisible to the
  double-dummy measure (the same reason `Lebensohl`-vs-floor was kept despite flipping
  negative under perfect defense). **The `american()` default is byte-identical to
  before** (takeout on, overcall off).

- **The advancer after a takeout double of a weak `(2♦)` now plays Transfer's `(2♦)`
  Smolen package (ledger #80).** After `(2♦)–X–(P)`, the default
  [`set_advance_sohl_style(LebensohlStyle::Transfer)`][pons::bidding::american::set_advance_sohl_style]
  advance now answers with `3♣`-Stayman + Smolen, direct Jacoby transfers, and Leaping
  Michaels `4♣`/`4♦` — the same package the 1NT context plays — instead of bare Cohen
  transfers-through; `(2♥)`/`(2♠)` advances are unchanged. It reuses the Section-5d
  builders verbatim under the `(2X)–X–(P)` prefix. Head-to-head vs the prior
  plain-Cohen advance (`examples/sohl-after-double-ab`, perfect-defense `ns_score`,
  200k filtered/cell): **+0.014/+0.019 IMPs/board, +1.77/+2.52 IMPs/divergent
  (none/both)** — a clean win whose per-divergent edge *rises* with vulnerability, the
  signature of reaching better contracts (which the measure credits) rather than
  right-siding (which it cannot see). With the package now winning in **both**
  contexts, the experimental `TransferSmolen` style is folded into `Transfer` (it
  never shipped as a separate variant): `Transfer` *is* Cohen-plus-Smolen-over-`(2♦)`,
  so the styles are again `Off`/`Plain`/`Transfer`.
- **Search at every authored leaf (AI-bidder M7.0) — `american_search_book`.**
  A new gated bidder, [`SearchBook`][pons::bidding::search_floor::SearchBook] /
  [`american_search_book`], that prices **authored book leaves by double-dummy
  cardplay**, not only the off-book auctions. Today
  [`american_search`][pons::american_search] runs the live search only where the
  book is silent (the contested fallback floor); `SearchBook` widens it to every
  *non-forced* book leaf: the leaf's authored logits become the search *prior*
  (rather than the final word), and DD re-judges among the calls the rule proposes
  **∪ the net's top-`k` natural alternatives** — so it can override an inflexible
  one-call rule — over sampled layouts, then bids the highest-EV call. "Rules
  propose, DD disposes," at every leaf. The authored *constraints* (meaning) are
  untouched — an opening still forbids `Pass` — only the authored *weights*
  (judgement) are overridden by the specific cards. The EV-pricing core
  (`price_and_blend`) is shared with `SearchFloor` (extracted byte-identically), and
  every §0 safety invariant is inherited verbatim: a forced auction delegates to the
  deterministic stance before any search, legality masking is unchanged, and the
  rollout RNG is seeded from the decision (determinism). It wraps a *bound* `Stance`
  (build it with `american_search_book(them)`); [`american`] and `instinct()` are
  untouched and stay the default. This is the **treatment arm** of the M7 A/B
  against `american_search` (DD off-book only) — on a measured win it folds into
  `american_search` as a default-on knob rather than living on as a twin. Strong but
  *very* slow (it searches every non-forced on-book decision); gated behind the
  `search` feature, with [`examples/search-book`](examples/search-book/main.rs) as
  the IMPs/board A/B harness. **Measured (120 boards, vul none, seed 1,
  perfect-defense scoring): wrapping *every* leaf as-is REGRESSES** — −2.958
  IMPs/board vs `american` (95% CI [−4.605, −1.312], excludes 0 — a clear loss) and
  −1.700 vs `american_search` ([−3.552, +0.152], point estimate firmly negative). The
  losses concentrate in *competitive* auctions where the convention is undecoded:
  the layout sampler then deals partner ranges too wide, so double-dummy over-values
  doubled grands (the divergent dump shows leaf-pricing reaching failing `7♣xx`). The
  fix is the M7.1 `Inferences::read` decode sweep (skip the search on any leaf with no
  usable decode); until then this is a recorded negative result, not a default. No
  effect on `american()`/`instinct()` (untouched, still the default and baseline).
- **Constraint extraction from the trained net — sample-and-probe (new example):**
  added [`examples/extract-constraints`](examples/extract-constraints/main.rs),
  which recovers human-readable *candidate* bidding constraints for **any
  auction, including competitive ones**. For a fixed auction prefix (`--auction
  "1♦ 1♠"`) it deals random actor hands — filtered to the actor's own shown
  shape — runs the real distilled bidder (`american_neural_search` by default, or
  `--net v2`/`neural`; legality mask and forced rails included), buckets each hand
  by the call it produces, and summarises every bucket in the DSL's own vocabulary:
  a chosen-share %, an HCP range, per-suit length bands, balanced %, and a
  copy-pasteable `sketch:` line such as `hcp(15..=18) & balanced()` (1NT) or, after
  `1♦ (1♠)`, `hcp(2..=9) & len(Diamonds, 5..)` (the preemptive `3♦` raise) versus
  `hcp(9..=18) & len(Diamonds, 5..)` (the `2♠` cue-raise). This replaces an earlier
  weight-linearization + data-dump approach that ignored the ReLU and matched a
  fixed corpus by exact context — brittle exactly where it mattered, contested
  auctions. The net's output depends only on `(actor hand, auction)`, so probing
  it directly is both simpler and faithful. Output is verifier-ready hypotheses,
  not proof: check a `sketch` with [`bidding::verify`] before authoring a rule.
  Builds under the `search` feature.
- **Leaping Michaels over a weak two — now on by default.** Over an opponent's
  weak two, a jump to `4♣`/`4♦` names a 5-5 two-suiter with game-forcing values:
  over a major it shows a minor + the *other* major; over `2♦` the `4♦` cue shows
  both majors and `4♣` shows clubs + a major. Advancer continuations are authored
  too — a fit major game (taking even a 7-card fit, which scores well and needs
  only ten tricks), else the `5m` minor game, never a passed-out partscore; over
  `2♦`, `4♥` is pass-or-correct to opener's major. *Measure* (`leaping-michaels-ab`
  contested seat-swap A/B, 40 000 filtered boards): **+1.090 / +1.452 IMPs/board
  (none / both) vs the prior weak-two defense** — a clear win, so it ships **on**;
  [`set_leaping_michaels`]`(false)` recovers the old behavior. (An earlier cut that
  left the advance to the instinct floor measured *negative*: the floor passed the
  two-suiter, stranding us in `4m` or the opponents' suit; authoring the advance
  flipped the sign.) The inference reader
  ([`Inferences::read`][pons::bidding::inference]) also decodes the overcall's
  two-suiter, so the live double-dummy search bidder (`american_search`,
  `--features search`) chooses the advance by cardplay EV — adding **+2.8
  IMPs/board over the authored rules** in a directional A/B and reaching the slams
  (`6♥`, `7♣`) the game-capped rules cannot. See the ledger (toggle 79).
- **Transfer Lebensohl — now the default over our overcalled 1NT** (Larry Cohen's
  version). A first attempt at transfer-Lebensohl lost (−1.7 IMPs/divergent — see
  the plain-Lebensohl entry below) by stranding game hands in partscores; Cohen's
  structure fixes exactly that. (Naming: this keeps the weak `2NT` *relay*, which
  makes it Transfer Lebensohl; *Rubensohl* proper makes `2NT` an artificial club
  transfer — tried on this structure and **not adopted**: the `2NT`-role swap
  measured −0.017 / −0.046 IMPs/board (none / both, 200k each), because
  right-siding the low-suit partscore is double-dummy-blind while two-way low
  transfers cost the auto-drive-to-game; see the ledger.) After `1NT–(2X)`,
  responder's three-level bids are **transfers up the line, *through* the adverse
  suit** (over `(2♥)`, `3♦` shows spades — skipping their hearts), the **cue is
  Stayman**, and a transfer to a suit *above* theirs is invitational-or-better, so
  **opener is driven to game** (`4M` with a fit, else `3NT`) — never a three-level
  partscore. Weak hands keep the plain outlets (natural two-level, `2NT` relay to
  `3♣`, penalty double). Selected by a new [`LebensohlStyle`]
  (`Off`/`Plain`/`Transfer`) via [`set_lebensohl_style`]; [`set_lebensohl`]`(bool)`
  stays as a `Plain`/`Off` shim. *Why it matters:* right-siding the strong `1NT`
  hand as declarer and describing shape more precisely under interference reaches
  better contracts. *Measure* (`lebensohl-ab` contested seat-swap A/B, 200 000
  boards per cell): **+0.46 / +1.24 IMPs/divergent (none / both) vs plain
  Lebensohl** (the incumbent default; 1 637 / 1 781 divergent, ~0.8–0.9 % of
  boards; +0.004 / +0.011 IMPs/board), and **+0.35 / +0.05 vs the bare floor** — so
  the earlier −1.7 loss is gone, now a gain. The double-dummy measure is blind to
  the right-siding effect, so the real table value is higher still. Tracked in
  [`docs/ai-bidder/21gf-ledger.md`](docs/ai-bidder/21gf-ledger.md).
- **Lebensohl after a takeout double (advancer over a weak two) — measured,
  opt-in.** Plain / Transfer / Pam (pick-a-minor) / Lawrence (three-band
  strength) sohl structures were authored over the `(2X)–X–(P)` advancer
  prefix and A/B'd against the `advance_double` floor on `sohl-after-double-ab`
  (contested seat-swap, 200k filtered boards/cell). At best DD-neutral vs the
  floor (a takeout double already advertises the fit, so natural advancing
  finds most of it). `Transfer` (the best) is kept behind
  [`set_advance_sohl_style`] as an opt-in (**default `Off`** — the floor);
  `Plain` is the A/B arm, while `Pam` / `Lawrence` were rejected and not
  retained. Tracked in
  [`docs/ai-bidder/21gf-ledger.md`](docs/ai-bidder/21gf-ledger.md).
- **Plain-4NT minor-suit keycard** (Roman Keycard Blackwood 1430 for an agreed
  minor — Batch 2 of the "author 2/1 as deep as BBA" effort). [`install_rkcb`] was
  major-only, so a minor fit carrying slam values could not ask for keycards at
  all: the strong-`2♣` minor raise blind-jumped to `6m`/`5m` on raw HCP, and
  inverted minor raises topped out at `3NT`. The ask now works for clubs and
  diamonds with the same `5♣/5♦/5♥/5♠` answers and a *cramped-signoff* asker — it
  signs off in 5-of-the-minor when that call is still legal (diamonds over a `5♣`
  answer), passes when partner's answer *is* 5-of-the-minor, and otherwise has no
  room and bids the small slam. Wired into the two cleanest minor-agreement
  auctions: the **strong-`2♣` minor raise** (`2♣–2♦–3m–4m`, opener launches `4NT`
  with 28+) and the **inverted minor raise** (`1m–2m–3NT`, responder launches
  `4NT` with slam values over opener's 18–19). *Why it matters:* a cold minor slam
  the floor could never bid is a ~12-IMP swing. *Measure* (`stayman-abc`
  constructive A/B, new vs the pre-change floor, 2 000 000 boards):
  **+6.80/+8.76 IMPs/divergent (none/both)** over 46 divergent boards (~1 in
  43 000 — minor-slam auctions are rare; +0.0002 IMPs/board). *Scope:* plain 4NT
  only — the 5NT king ask is **major-only** (over a minor, 5NT misreads as the ask
  and the `6♣/6♦` king answers collide with the trump slam), so grand slams in a
  minor stay under-bid; Kickback, the usual remedy, is out of scope. Tracked in
  [`docs/ai-bidder/21gf-ledger.md`](docs/ai-bidder/21gf-ledger.md).
- **Lebensohl after our 1NT is overcalled** (toward BBA's 21GF depth — first
  competitive convention of the "author 2/1 as deep as BBA" effort). When we open
  `1NT` and an opponent overcalls at the two level, responder previously fell to
  the natural [`instinct()`] floor. A new competitive-book section gives plain
  Lebensohl: a weak hand relays through `2NT` to a forced `3♣` (sign off in clubs
  or correct to a six-card suit), while a game hand bids a **forcing** three-level
  suit or a to-play `3NT` directly — so a game is never stranded in a partscore.
  Penalty doubles and weak natural two-level bids round out the table.
  *Why it matters:* the floor competes naturally but cannot relay a weak hand to
  the right partscore or force game cleanly under interference. *Measure*
  (contested seat-swap A/B, Lebensohl vs the floor, opponents overcall,
  `lebensohl-ab` 200 000 boards vul none): **+0.26 IMPs/divergent** (1 764
  divergent, ~0.9 % of boards; +0.002 IMPs/board — a small, correct gain
  concentrated in the rare overcalled-1NT auctions). A first attempt at
  *Rubensohl* (transfer-Lebensohl) measured a **net loss** (−1.7 IMPs/divergent):
  its transfers stranded game hands in partscores because the rebid re-evaluated
  too conservatively, and it shadowed the floor's penalty doubles — recorded in
  [`docs/ai-bidder/21gf-ledger.md`](docs/ai-bidder/21gf-ledger.md). Plain
  Lebensohl is now the [`LebensohlStyle::Plain`] option; the corrected Transfer
  Lebensohl (see above) is the default. The ledger tracks pons's 2/1 against the
  `21GF.bbsa` convention card.
- **A deeper deterministic floor — Milestone 6.1: parametric auction
  inferences.** The keyless [`instinct()`] floor now *derives* responder's
  major-suit length from a completed Jacoby transfer rather than going silent on
  it ([`Inferences`]): `1NT–2♦–2♥` shows five-plus hearts, and a follow-up jump to
  game (`…–4♥`, the canonical case) or raise of the suit (`…–3♥`, which also pins
  invitational strength) shows **six** — responder bypassed the choice-of-games
  `3NT`. A new six-two arm in the floor's `known_major_fit` lets opener act on
  that shown six-card suit opposite a doubleton — the fit the prior bidder could
  not see after a transfer (`known_major_fit` needed three-card support on one
  side). Both majors, over 1NT and 2NT; uncontested only. *Why it matters:* the
  floor can now accept a transfer invitation with a maximum (`1NT–2♦–2♥–3♥` →
  `4♥` on a six-two fit) instead of always passing, and the sampler behind the
  search floor deals layouts consistent with the shown six-card suit. *Measure*
  (seeded constructive A/B, baseline vs M6.1 `american()`, opponents silenced,
  200 000 boards): **+1.94 IMPs/divergent vul none, +2.25 vul both** (306
  divergent boards; +0.003 IMPs/board — the gain concentrates in the rare
  transfer/limit auctions it touches). No regression: the whole inference floor
  stays **+0.05 IMPs/board** (`inference-floor`, 20 000 boards, both
  vulnerabilities). Derived, not authored — no node per sequence.
- **`bba-match --our-system` — BBA-vs-BBA system comparison** (AI-bidder side
  experiment). The 2/1 eval-anchor example gains an optional flag that drives
  *our* side with a second EPBot card instead of the [`american`] floor, so the
  same engine bids both tables and every swing is a pure methods difference, with
  no bidding-skill confound. *Finding* (WJ / Polish Club, system 2, vs 2/1 Game
  Force, system 0; 10 000 boards each; double-dummy scored; swing credited to
  WJ): **+0.123 IMPs/board non-vulnerable** (95% CI [+0.051, +0.196]) and
  **−0.022 both-vulnerable** (CI [−0.111, +0.068], a statistical tie) — WJ holds
  a small constructive edge non-vul that washes out when both sides risk the big
  penalties; double-dummy is blind to WJ's obstruction value, so this reads as a
  *floor* on its real-table edge. Default (flag unset) keeps the S.1 anchor
  (`american` vs BBA 2/1) unchanged; no change to the crate's default build or
  dependencies.
- **A reverse-engineering study of BBA/EPBot's *floor*** (AI-bidder side study):
  a new report [`docs/ai-bidder/bba-floor.md`](docs/ai-bidder/bba-floor.md)
  answering how a mature engine bids where authoring runs out — the analogue of
  pons's [`instinct()`] floor. Findings: (1) `strace` shows `libEPBot.so` loads
  **no** external data files, so `MB.TXT` is a compiled-in export, not the
  runtime source; (2) static classification of all 6094 `MB.TXT` rules shows
  **66% are generic/parametric** (suit-variable templates, char-class ranges,
  constraint-only catch-alls) and specific literal-auction nodes are **shallow**
  (1–3 calls, vanishing past depth 5); weights are bimodal so broad floor rules
  (weight 0–9) always lose to specifics (90–99); (3) a live probe
  (`examples/bba-floor-probe`) confirms the compiled engine is **programmatic** —
  on deep off-book auctions its call escalates monotonically with the hand and it
  labels its own floor bids `"calculated bid"`. Throwaway reproducers
  (`scripts/bba_floor_stats.py`, `examples/bba-floor-probe/`) reuse the existing
  `bba-match` FFI; no change to the crate's default build, dependencies, or
  `instinct()` baseline.
- **South African Texas over 1NT — `4♣/4♦` to-play transfers and `4♥/4♠`
  non-forcing slam tries.** A 6-card-major responder gains a four-level structure
  on top of the two-level Jacoby transfers ([`american::notrump`]):
  - **`4♣ → 4♥`, `4♦ → 4♠`** (game, no slam, 9–14 HCP): jump transfers that put
    the opener in `4M` — identical in placement to the old `2♦/2♥`-then-game route,
    but *preemptive*, denying the opponents the two level they would otherwise
    balance in.
  - **direct `4♥/4♠`** (slam-invitational, 15–18): a non-forcing slam try. Opener
    passes the game with a minimum or launches RKCB 1430 with a maximum (17),
    reusing the existing [`american::slam`] ladder to place `6M` (or `5M` when
    missing two keycards). Because responder names the major first, the slam is
    responder-declared.

  The carve (weights 2.5/2.6 over the 2.0 Jacoby transfers; a `len(other major,
  ..5)` guard keeps 5-5+ two-suiters on the both-majors `3♦`; an HCP split routes
  game to the transfer and slam-invitational to the direct try) sends each 6-card
  hand to the right level. *Why it matters:* the prior bidder **could not bid a
  major slam after a Jacoby transfer** — the floor's `6M` milestone needs a
  `known_major_fit` (`partner_shown_len(major, 3..)`), which a transfer's
  completion never establishes — so those slams went unbid. The direct `4♥/4♠`
  defines the 6-card fit explicitly and bypasses the gap. *Measured* (`sat-ab`,
  seeded before/after duplicate match, 10M deals, opponents silent, double-dummy):
  **+2.53 IMPs/divergent board vul none, +3.78 vul both** — the divergent boards
  are the slam-invitational hands (21% of the 6-major class; the to-play `4♣/4♦`
  hands reach the same `4M`-by-opener as before, so they do not diverge); +0.54 /
  +0.81 IMPs per 6-major board.
- **`sat-slam-try` diagnostic example — the revised SAT `4♥/4♠` non-forcing slam
  try.** A follow-up to `texas-vs-sat`: in the *swapped* South African Texas,
  `4♣/4♦` are the everyday transfers (opener declares — declarer-equivalent to
  Texas) and a *direct* `4♥/4♠` is a non-forcing slam try, opener passing with a
  minimum or launching RKCB with a maximum. The to-play hands no longer diverge
  from Texas, so the example measures only the **slam-invitational** ones — where
  double-dummy *can* see the difference. Baseline = the current bidder; gadget =
  the opener-max keycard slam, modeled by hand. **Finding** (12M deals per
  vulnerability, ~3,600 configs each): **+1.38 IMPs/board vul none, +1.57 vul
  both.** The gadget reaches `6M` on ~17% of configs at a **94% double-dummy
  make-rate**, while the current bidder reaches slam only ~3%. *The catch:* much
  of the gain is the current bidder's **structural slam hole** — the floor's `6M`
  milestone requires a `known_major_fit` (`partner_shown_len(major, 3..)`), which
  a Jacoby transfer's completion never establishes, so the system cannot bid a
  major slam after a transfer. The gadget bypasses it; patching the floor would
  recover much of the same gain for *every* transfer auction. Now authored — see
  the South African Texas entry above; the seeded `sat-ab` A/B confirms the
  modeled gain on the real bidder.
- **`texas-vs-sat` diagnostic example — Texas vs South African Texas, the
  declarer question.** Measures the one *double-dummy-visible* difference between
  the two 4-level-transfer schemes: who declares `4M` on a 6-card-major
  game-but-not-slam hand opposite a strong 1NT — Texas (and the crate's current
  transfer-then-game) puts the **opener** in; South African Texas's direct
  `1NT–4♥/4♠` puts **responder** in. The DD/perfect-defense scorer is blind to
  *concealment* — the textbook reason Texas exists — so the example isolates only
  the residual opening-lead swing, and says so. **Finding** (600k deals per
  vulnerability, ~4,300 qualifying configs each): responder declaring scores
  **−0.052 IMPs/board vul none, −0.088 vul both** — opener declaring (Texas) is
  better *even on the concealment-blind metric*, the same direction as the larger
  effect double-dummy cannot see, and no hand feature (responder shortness
  included) flips the sign. The current treatment already declares from opener,
  so **no system change** — South African Texas is not adopted.
- **Both-majors response (1NT–3♦) — 5+/5+ in the majors, invitational+.** A 5-5
  major two-suiter previously had no one-bid home: it transferred and rebid the
  other major (clumsy, and game-forcing 5-5s fell through to the floor entirely).
  New nodes in [`american::notrump`]: responder bids `3♦` to show both majors
  (gated `points(8..)` so the 5-5 shape upgrade counts and weak 5-5s still take
  the transfer route; weight `2.1` outranks the `2.0` transfers). Opener picks the
  strain by strength — a **minimum (15–16)** signs off in three of the better
  major (`3♥/3♠`, spades-with-three else hearts), a **maximum (17)** jumps to the
  eight-card major game (`4♥/4♠`) or `3NT` when 2-2 in the majors. Over a minimum
  signoff responder passes the invitation or raises to game (`points(10..)`).
  Authored, not floored, for the usual reason — the keyless floor misreads `3♦`
  as natural diamonds and force-bids game. *Measured* (`stayman-abc`, seeded
  before/after duplicate match, opponents silenced, 200k boards, double-dummy):
  **+2.17 IMPs/divergent board vul none, +2.80 vul both** (5-5 INV+ is rare, ~0.05%
  of boards diverge). The `points(8..)` floor was tuned on the A/B (beats `7..`
  on per-divergent at tied total IMPs, and `9..` on both counts).
- **Puppet Stayman (1NT–3♣) and the minor-suit transfers (1NT–2NT diamonds,
  1NT–2♠ clubs/invite).** Three new constructive structures fill 1NT-response
  slots that previously carried no precision — a weak long-minor hand just passed
  1NT, a balanced game force blasted 3NT, and a 5-3 major fit was missed. New
  nodes in [`american::notrump`]:
  - **Puppet Stayman (`3♣`):** a game-forcing balanced hand with a three-card
    major hunts opener's five-card major. Opener shows it (`3♥/3♠`) or denies
    (`3♦`); over the denial responder bids the *shorter* major Smolen-style to
    show four in the longer, finding a 4-4 with opener declaring, else 3NT. The
    **2♣-vs-3♣ carve**: a 4-3 game force Puppets (it holds both a four- and a
    three-card major); a 4-4 or 4-(0-2) takes plain Stayman; invitational hands
    always take `2♣` (Puppet is game-forcing). `balanced()` keeps Puppet off
    shapely hands, which use the minor transfers instead.
  - **Diamond transfer (`2NT`):** 6+♦, or 5♦4♣. Opener completes to `3♦` only
    with three-card support (an assured eight-card fit), else `3♣` pass-or-correct
    so a 5♦4♣ hand can pick the better minor.
  - **Two-way `2♠`:** a club one-suiter (weak signoff, or game-going) or a
    balanced invitational eight. Opener shows strength — `3♣` maximum, `2NT`
    minimum — so responder pass-or-corrects safely: the invite plays `2NT`/`3NT`,
    weak clubs land in `3♣`, and a game-going club hand splinters (`3♦/3♥/3♠`) for
    opener to pick `3NT` vs `5♣`. The bare-8 invite **relocated here** from the
    old natural `2NT` (now the diamond transfer); min→2NT and max→3NT reproduce
    the old accept/decline outcomes, so that win is preserved, not reverted.
  - **Smolen reachability:** a game-forcing 5-4 in the majors now keeps off the
    Jacoby transfer (its `hcp(..9)` arm) and takes `2♣` Stayman, so the existing
    Smolen jump right-sides game to the strong notrump (the jump's floor dropped
    10→9 to match "force every 9"). A plain 5-3 still transfers.

  `Inferences::read` now reads a `2NT` response as the diamond transfer (5+♦), not
  an 8–9 points raise, and suppresses the new artificial relays/puppets/splinters
  from the natural suit reading so the floor and the search sampler are not
  misled. *Measured* (`stayman-abc`, a seeded before/after duplicate match — the
  change is structural, two binaries rather than a runtime toggle — opponents
  silenced, 60k boards, double-dummy): **+0.76 IMPs/divergent board vul none,
  +1.15 vul both** (~1.0% of boards diverge, so +0.0072 / +0.0109 IMPs/board
  overall), every divergent class net positive; the Smolen-reachability lever
  alone adds +0.0022 / +0.0030 IMPs/board. The `american_minor_transfers` test
  suite pins the new behaviour.
- **Stayman (1NT–2♣) is now fully authored — further bidding, Smolen, and the
  "ignore 2♣ ⇒ revert to notrump" rule.** Previously only opener's `2♥/2♠/2♦`
  answer was in the book; every continuation fell to the keyless floor, which
  misbid them — it reads any three-level suit response over our 1NT as
  *forcing*, so it force-bid game over an invitational Stayman raise and could
  never decline. New constructive nodes ([`american::notrump`]):
  - **After opener shows a major (`2♥/2♠`):** invitational raise (`3M`), game
    (`4M`), or — balanced or slam-interested — the **other major (`3OM`)** as an
    artificial slam try / choice of game. Opener answers `3OM` with `3NT` on a
    flat 4-3-3-3, the cheapest control cue on a maximum, else the major game.
    Opener accepts the invitational raise into the major game with a maximum
    (`3NT` only on a flat 4-3-3-3), passes a minimum.
  - **Without a fit, "ignore the 2♣ detour":** `2NT` invites, and `3NT`/`4NT`
    are bid exactly as over a bare 1NT — so `4NT` is quantitative (16–17), opener
    accepting `6NT` with a max.
  - **Smolen:** with game-forcing 5–4 in the majors, responder jumps in the
    four-card major to show *five* in the other (`1NT–2♣–2♦–3♥/3♠`), so the strong
    notrump declares; opener completes to game in the long major. Mirrored at the
    **2NT-strength** level (`…3♣–3♦–3♥/3♠`).

  The judgement that *is* sound for the keyless floor stays there: `Inferences::read`
  now reads the 1NT–2♣ auction (opener's answer pins a four-card major or denies
  both; responder's `2♣` and invitational continuations pin strength), feeding the
  sampler behind `american_search()` and any competitive fallback, while the
  artificial `3OM`/Smolen jumps are suppressed from the natural suit reading rather
  than misread as long suits. *Measured* (`stayman-abc`, a seeded before/after
  duplicate match — the change is structural, so the two arms are two binaries
  rather than a runtime toggle — opponents silenced, 60k boards, double-dummy as in
  `nt-invite-abc`): **+1.38 IMPs/divergent board vul none, +2.03 vul both**
  (~0.9% of boards diverge, so +0.013 / +0.019 IMPs/board overall), every divergent
  board class net positive. The `american_stayman` test suite pins the new behaviour.
- **Opener accepts a 1NT–2NT invitation — via the inference, not a node.**
  `american()` previously *passed* a `1NT–2NT` invite even with a maximum: opener
  was blind to responder's strength because `Inferences::read`'s notrump-raise
  reading was gated to one-of-a-suit openings, so a raise of our *own* 1NT opening
  showed nothing. Teaching the inference that `1NT–2NT` shows an invitational ≈8 and
  `1NT–3NT` is game-going 9+ (naturally; the artificial Stayman/transfers stay
  silent) lets the **keyless floor judge game itself** — it already knew "bid game
  when the combined range suffices", it just couldn't see responder. With the fix,
  **both `american()` (the deterministic instinct floor) and `american_search()`
  accept opposite a maximum (3NT) and decline opposite a minimum (Pass)** — no
  hand-authored acceptance node, in keeping with "smarten the floor, don't author a
  node per bid". *Measured* (`nt-invite-abc`, opponents silenced, 60k boards/cell):
  consistently positive, **+1.96 IMPs/divergent board vul none, +4.48 vul both**
  (~0.1% of boards, so +0.002–0.004 IMPs/board overall), zero regression. Gated by
  `set_nt_invite_inference(bool)` (default on) for the A/B and as a regression
  guard. *Deferred* (future session): apply the same inference treatment to the
  other partially-authored notrump continuations — invitational/game sequences
  after transfers and Stayman, and natural raises of the 2NT opening (the
  `nt-range-split` diagnostic below still shows ~23 such hands the book under-bids).
- **Responder forces game with 9+ over 1NT (was: invite 8–9, force 10+).** A/B
  verification of "upgrade the 9-count to a game force": opposite a 15–17 notrump a
  flat 9 makes game often enough that the invitational stop loses more by missing
  games (opener declining with a useful minimum) than the occasional 24-count game
  costs. *Measured* (constructive A/B, opponents silenced, 120k boards/cell, forcing
  every 9 vs inviting 8–9): **+0.98 IMPs/divergent board vul none, +2.91 vul both**
  (+0.0016 / +0.0046 IMPs/board), zero regression. Deciding the 9 by Thomas Andrews's
  tempered **Fifths** instead (force good 9s, invite quack-heavy ones) was measured
  *worse* — even low-Fifths 9s gain ≈+0.9 IMPs/divergent when forced, so the
  selective threshold just leaves games unbid (matching Andrews's own caveat that
  the fractional valuation does not help at the 1NT invitation boundary). So the
  blunt HCP threshold wins; responder's 2NT is now a bare-8 invitation and `3NT`
  shows 9+. The inference (above) was updated to match.
- **`Inferences::narrowed_points` + the `nt-range-split` diagnostic (AI-bidder).**
  The new `Inferences::narrowed_points(who, range)` returns a copy with one player's
  shown points intersected to a sub-range — the seam for splitting a 1NT opener's
  shown range into halves and sampling layouts from each (`sample_layouts`). The
  `nt-range-split` example uses it as an *oracle*: opposite openers from each half it
  scores the best NS game against the best NS partscore double-dummy (game good
  opposite both → FG, the upper half only → INV, neither → PASS — the meaning of an
  invitation), and compares that verdict to where `american()` lands by bidding the
  `1NT–Pass` auction out. This is what *found* the invite-acceptance gap above (the
  empty INV column); after the inference fix its disagreement drops 26.4% → 22.8%
  (the residual is the deferred transfer/Stayman continuations). Plan:
  `docs/ai-bidder/`.
- **Meckstroth adjunct — opener's invitational `3m` jump after a forcing 1NT
  (and `1♥–1♠`), now the default.** After `1M–1NT` or `1♥–1♠`, opener's
  medium *shapely* hands (5-5 / 6-5, ≈15–17 points) previously had no
  descriptive rebid and underbid as a natural two-level minor; opener now jumps
  to **`3♣`/`3♦` to show 5+ of the minor, invitational**, and responder accepts
  game with a maximum (4M with a 5-3 fit, else 3NT) or declines to a preference
  in opener's five-card major. The strong 2NT rebid is left as the
  forcing-to-game/strong slice (its existing natural continuation already drives
  responder to game opposite a 6+ forcing 1NT, so the full artificial relay
  ladder is deferred). Disjoint by strength: 18–19 balanced → 2NT, 15–17 with a
  five-card minor → `3m`, a minimum → the natural two level. Gated by
  `set_meckstroth_adjunct(bool)` (default on); the new `meckstroth-abc` example
  is the constructive A/B harness. *Measured* (opponents silenced, 50k boards
  per cell): consistently positive across all three vulnerabilities on both
  points and IMPs — **+0.67 IMPs/divergent board vul none, +1.38 vul ns, +1.43
  vul both**, with zero regression. The situation is rare (~0.25% of boards), so
  the whole-match figure is +0.002–0.004 IMPs/board; the win is real but
  low-frequency.
- **`american_wide_6322()` — experimental 6322-minor 1NT option.** A
  `NotrumpShape` enum (`Balanced` / `Wide` / `Wide6322`) now selects the 1NT
  opening shape; `Wide6322` adds a 6322 with a six-card minor on top of the
  shipped `Wide` (5422-minor) default. Kept as an option, **not** the default: a
  constructive ablation had found the 6322 addition net-neutral, but a
  *contested* re-test (`nt-shape-contested --baseline wide --redesign wide6322`,
  100k boards) shows it is worth **+0.52 IMPs/divergent board vul none and +0.64
  vul both** (~2.8–3.6σ) — the 6-card minor's preemptive value pays off only in
  competition. Adopting it as the default is gated on the deferred inference pass
  (a 6-card suit breaks the current "1NT opener is 2–5 in every suit" inference,
  which a 5422 satisfied but a 6322 does not). The `nt-shape-contested` example
  gained `--baseline`/`--redesign` flags to compare any two shape policies.
- **Wider 1NT opening shape, now the default.** The strong 1NT (`american`)
  opens not only the balanced patterns (4333/4432/5332) but also a **5422 with a
  five-card minor** — a five-card major still prefers a one-of-a-major opening it
  can rebid, and a 6322 (either suit) keeps opening its long suit (an A/B
  ablation showed any-5422 *loses* by burying the major fit, and the 6322
  addition was net-neutral, so both are left out). Strength (`fifths` 15–17) is
  unchanged; this is a shape-only change. The pre-change balanced-only system is
  preserved as `american_classic()` (the A/B baseline). *Measured*
  (5422-minor wide vs balanced classic): **constructive** A/B (`nt-shape-abc`,
  opponents silenced) +0.32 IMPs per divergent board; **contested** A/B
  (`nt-shape-contested`, opponents bidding, 100k boards) +0.57 IMPs/divergent vul
  none and **+0.93 vul both** — a clear, statistically solid win that grows with
  competition and vulnerability, exactly the modern rationale. The shape fires on
  ~1% of boards, so the whole-match figure is +0.006–0.009 IMPs/board. The
  opening inference stays **sound** (the `opening_inference_contains_the_opener`
  proptest passes at 200k cases — a 5422 fits the existing "each suit 2–5, 14–19
  points" 1NT inference); *tightening* it to convey the possible five-card minor
  is left for the deferred evaluation/inference pass.

- **Rubens (transfer) advances of a simple overcall, in the instinct floor.**
  When partner makes a *simple* (non-jump) suit overcall of a one-level opening
  and RHO passes, advancer's calls from the cue up to a two-level raise are
  transfers to the next suit — a new-suit transfer shows a five-card suit and
  **10+ upgraded points** (a *good* 9 and all 10+, since the transfer commits
  partner to the two level), and the transfer that lands in partner's suit is a
  limit-plus raise; over a two-level overcall, where there is no room for the
  ladder, the cue itself is the limit-plus raise. Jump overcalls are preemptive and advance naturally, as
  do advances of preemptive openings. The convention is implemented once,
  *programmatically*, in the keyless floor: `overcall_shape` /
  `advance_of_overcall` derive the transfer band for every (opening, overcall)
  pair, so one rule set serves all of them (a per-suit authored table could
  not), and the meaning is mirrored in `Inferences` — the transfer/cue suit is a
  relay, not a holding (suppressed), while a cue-raise is read as three-plus
  support and ten-plus points so the overcaller can still reach game. Partner's
  instinct therefore completes the transfer mechanically and never misreads it.
  The floor now **owns advancing a simple overcall**: the books' raises-only
  `advances()` — which returned a degenerate result on hands it could not
  classify, such as a five-card side suit with no support — is removed from
  the 2/1 defensive book, and the floor's Rubens transfers,
  natural raises, and a weak preemptive jump cover the position. *Measured:*
  floor worth preserved at **+1.03 IMPs/board** (instinct-floor A/B, 8000 boards,
  vul none), with transfers confirmed firing in the off-book telemetry; against
  BBA's 2/1 (`bba-match`, 2000 boards, vul none) **−2.02 IMPs/board vs −2.13
  pre-Rubens** — neutral-to-slightly-positive, no regression from moving
  advances to the floor. Eleven tests: floor and inference unit tests plus four
  full-system integration rails (new-suit transfer, limit-raise transfer,
  preemptive raise, two-level cue-raise).
- **`constructive-abc` example + `american_constructive_floor` builder.**
  The neural/search floors only ever own the *contested* books — unbooked
  *constructive* auctions are always answered by the deterministic `instinct()`
  milestone ladder. This A/B/C harness measures whether that partition leaves
  points on the table: it silences the opponents (East/West always pass) so every
  auction is constructive, bids each board three times — `instinct()`,
  `SearchFloor`, `NeuralFloorSearch` floored onto the *constructive* book — over
  the same deal, solves it once double dummy, and reports the pairwise IMP swings.
  The new `bidding::american::american_constructive_floor(floor)` builder
  (gated `neural-floor`) exposes the constructive-floor knob the standard
  constructors hard-wire to `instinct()`; the example is gated `search` and the
  search arm dominates runtime (~seconds/board), so `--layouts`/`--shortlist`
  trade strength for speed. No change to any shipping bidder — purely a new
  measurement path and an added builder. First run (2000 boards, none vul, 45%
  divergent): the bare net `NeuralFloorSearch` loses **0.8 IMPs/board** to
  `instinct()` on constructive auctions, while the live `SearchFloor` ties it
  (+0.002) — the search rescues a weak constructive prior with real cardplay,
  the bare net cannot. Confirms the partition: the milestone ladder is the right
  constructive floor and the learned floors stay contested-only.
- **`scripts/fleet/` — distributed data-gen.** A small ssh harness that spreads
  the CPU-bound double-dummy dumps (`search-dump` / `teacher-dump`) across several
  machines without manual syncing. Because a dump is deterministic given
  `(git SHA, seed)` and its `.f32`/`.tags` rows are independent and concatenable,
  distribution needs no daemon or queue: `run.sh` partitions the seed space into
  one shard per seed and dispatches them with GNU `parallel --sshloginfile`
  (each remote run still wrapped in `scripts/idle-run.sh` for SCHED_IDLE
  politeness), pulls the shards back with `rsync`, and `merge.sh` validates the
  sidecars agree (feature/layout/system/SHA) and `cat`s them into one dump the
  off-crate trainer reads unchanged. It pins the coordinator's SHA and refuses
  any host not on it (skew would silently corrupt the dataset), builds on each
  host (so it is arch-agnostic), `--resume`s incomplete shards on re-run, and
  self-balances across heterogeneous hosts (`-j1` = one all-core solver per box,
  faster boxes grab more shards). Copy `hosts.example` to `hosts` to use.
- **`calibrate-eval` example**: regresses double-dummy tricks on the partnership
  hand evaluators (HCP, Fifths, BUM-RAP, LTC, NLTC, Zar, CCCC) using the
  precomputed 100k-deal database `sol100000.txt` — **no DD solving**, so it is
  fast and cannot overfit the bidder. It decodes that file's **GIB** format
  (West-first deal; 20 hex DD cells over strains `NT,S,H,D,C` × declarers
  `E,N,W,S`, with East/West stored as `13 − tricks`; verified against the
  solver) and reports, per evaluator and per context (notrump vs an 8+-card
  trump fit), the trick mapping (slope, intercept, residual σ, R²) and a
  concentration term `c·|eᴺ − eˢ|` that measures non-additivity. Hands taking
  fewer than 6 double-dummy tricks are dropped (never bid), and the survivors
  are further split into partscore (6–8), game (9–11) and slam (12–13) zones,
  each reporting the strength `Σ` it requires and its own linear `tricks = b·Σ + a`
  fit. Findings: the evaluators are essentially perfectly additive (`c ≈ 0`);
  BUM-RAP/Fifths fit notrump tricks best while Zar/CCCC win the suit fit; NLTC
  clearly beats LTC; the textbook "tricks = 24 − losers" base is empirically
  closer to 22 (NLTC) / 20 (LTC); and the slope flattens sharply toward slam
  (an extra notrump HCP is worth ≈ 0.4 tricks overall but only ≈ 0.05 in the
  slam zone — slams are decided by controls and fit, not raw points).

- **Tag features for the neural floor** (AI-bidder **M5.1**): a second, opt-in
  feature-spec version that feeds the **WBF tags of the recent calls** into the
  policy net as categorical inputs. A new `bidding::tags` module lifts the
  structural tag reader (`derive`, `infer_book`, the `TAGS` vocabulary, and
  `tag_multihot`) out of `examples/export-corpus` so the corpus exporter and the
  featurizer share one source of truth (the exporter's output is byte-identical).
  `features::features_v2` returns the v1 160-float vector followed by a
  multi-hot of the last `TAG_WINDOW` (= 4) calls' tags over the 21-tag
  vocabulary — **244 floats**, version 2 — with each prior call read by the same
  `derive`, recovering its book from the auction via `infer_book`. The net is
  trained, distilled, and embedded exactly as v1: `bidding::neural::classify_v2`
  (hand-rolled forward pass, now dimension-parameterized and bit-matched to the
  trainer on a fixture), the `NeuralFloorV2` safety shell (same forced-rail
  delegation + legality mask), and `american_neural_v2()` (gated behind
  `neural-floor`; `american()` stays the baseline and `american_neural()`
  the v1 floor — an added option, never a removal). The off-crate trainer and
  `teacher-dump` are now **layout-agnostic**: `teacher-dump --features-version 2`
  emits the v2 dump, the trainer sizes the model input from the dump sidecar, and
  **v1 dumps still load unchanged**.
  **Measured** (20 000-board duplicate A/B, vul none): the tag block improves the
  *distillation fidelity* (held-out top-1 agreement with the teacher 95.0% vs the
  v1 net's 93.8%, val cross-entropy 0.235 vs 0.249) but lands at **parity on
  IMPs/board vs v1** (−0.016 IMPs/board, 95% CI [−0.039, +0.007] — within noise),
  while preserving the floor's worth over bare books (+0.540 IMPs/board, CI
  [+0.495, +0.585], containing the +0.5 baseline) and the teacher-clone parity
  (−0.015 vs the deterministic floor). The expected reading: for *pure teacher
  distillation* the deterministic teacher is the ceiling, so richer inputs buy
  fidelity, not table result; the tag infrastructure is in place to pay off when
  the floor is distilled toward a better-than-teacher search target (M3.2). New
  artifact `src/bidding/weights/american_v2.{f32,json,fixture.json}`; no new
  crate dependencies; the default build is unchanged.
- **Search-target neural floor** (AI-bidder **M3.2**, round 1): a third distilled
  floor, trained toward the **double-dummy search teacher's** EV-grounded targets
  (the M3.1 `search-dump`) instead of the deterministic teacher — same v1 features
  and 160→256→256→38 shape, only the training target differs. `neural::classify_search`
  (hand-rolled forward pass, bit-matched to the trainer on a fixture), the
  `NeuralFloorSearch` safety shell (the *same* forced-rail delegation to `instinct()`
  + legality mask as the v1/v2 floors), and `american_neural_search()` (gated
  behind `neural-floor`; `american()` and `american_neural()` stay the
  baselines — an added option, never a removal). Trained on the 10 000-board dump
  (97 701 rows, git_sha `1d43577`): held-out fit to the *richer* search target is
  val-CE 0.776, top-1 89.4 % constructive / 73.8 % contested — looser than the
  near-deterministic teacher clone *by design*, since the search softmax is a
  higher-entropy distribution. **Measured** (20 000-board duplicate A/B, vul none):
  **+0.787 IMPs/board vs the v1 teacher-distilled net** (95 % CI [+0.718, +0.857]),
  and +0.700 vs the deterministic floor ([+0.630, +0.770]) and +0.816 vs bare
  books — a decisive gain, concentrated **off-book/competitive**. The high divergence
  (75 % of boards) is the net reaching makeable games/slams the conservative v1
  misses, *not* overbidding: against double-dummy par (small slam makeable on
  10.65 % of deals, grand 2.94 %) the search net in fact **under**-bids slams
  (4.0 %/1.7 %) while v1 is the pathological under-bidder (≈0). A
  perfect-defense-doubling rescore — DD the optimistic bound, PD the pessimistic —
  **brackets** the gain and it survives both: search vs v1 holds at 0.35 IMPs/board
  under PD (vs 0.79–0.86 under DD across two 20k samples), CI excluding 0; search vs
  the deterministic floor 0.40 under PD. The `examples/neural-floor` A/B now prints
  both views per matchup. Only the single-dummy haircut (deferred — needs a cardplay
  engine) stays unquantified. New artifact
  `src/bidding/weights/american_v1_search.{f32,json,fixture.json}`; no new crate
  dependencies; the default build is unchanged. Iteration (round 2: regenerate
  targets with this net as the search policy) is deferred.
- **`scoring::ns_score_doubling_failures`** — a sibling of `ns_score` that scores
  a contract under **perfect-defense doubling**: any contract failing
  double-dummy is scored *doubled*, a making one keeps its auction penalty. This
  is the per-deal form of the par scorer's long-standing `min(undoubled,
  doubled)` heuristic in `stats::average_ns_par` (now documented and named at
  both sites). It is the EV evaluator's new scorer (see *Fixed*). Also adds a
  gated `grand-probe` diagnostic example (behind the `search` feature) that
  replays the search self-play, measures the DD make-rate and a points-vs-IMP
  recompute at each 7NT node, and has a `--census` mode tallying the
  advancing-call level histogram — the regression check that the grand flood
  stays gone.
- **Search-improved distillation targets** (AI-bidder **M3.1**): a new gated
  `search-dump` example (behind the `search` feature) that bids out random boards
  with the M2.3 live double-dummy search floor and records, at every decision, a
  training row of `(features, search_target)` — the improved call distribution the
  net is distilled toward in M3.2. The output is **byte-identical in layout to
  `teacher-dump`** (a flat little-endian `f32` file of `160 + 38 = 198` floats per
  row, a `.json` sidecar, and a `.tags` file), so the off-crate trainer consumes it
  unchanged; the only difference is the target, which the search improves on the
  teacher exactly where the books were silent (the file is a trainer-compatible
  *superset* of `teacher-dump`, identical on book nodes and upgraded off-book). The
  `.tags` byte gains a second bit (`bit1` = off-book / search fired, alongside the
  existing `bit0` = contested phase). The example also prints and records **the
  M3.1 measure**: at each row it classifies the deterministic teacher
  (`american`) and the raw net prior (`american_neural`) and reports, split
  by off-book/on-book and contested/constructive, the arg-max disagreement rate and
  the mean total-variation distance — confirming the targets differ from the teacher
  *mainly off-book* (on-book rows are `0` by construction; a 40-board smoke run shows
  ~51 % arg-max disagreement and ~0.53 mean TV off-book vs `0`/`0` on-book). A small
  additive constructor, **`american_search_with(SearchFloor)`** (gated `search`,
  re-exported at the crate root), lets data-generation runs trade strength for speed
  via the `--layouts`/`--shortlist`/`--temperature` knobs; `american_search()`
  is now exactly `american_search_with(SearchFloor::default())`. No change to the
  default build, the safety shell, or the `instinct`/`search_floor` rails; no new
  crate dependencies.
- A **BBA/EPBot eval anchor** (AI-bidder Side-track S, S.1): a new `bba-match`
  example that pits our deterministic `american()` floor against **BBA's own
  2/1 Game Force card** (EPBot system 0, verified by name) in an A/B duplicate
  match — apples-to-apples, so every divergence is a pure quality gap in our DSL,
  not a difference of methods. A `BbaOracle` implements pons's public `System`
  trait by driving a fresh EPBot bot per decision (configure all four seats,
  deal the actor's hand, replay the auction with `epbot_set_bid`, read the call
  with `epbot_get_bid`), with the dealer canonicalized to position 0 so
  `classify` stays a pure function of `(hand, vul, auction)`. The S.0 ABI was
  generalized to full auctions: `epbot_set_bid(bot, position, bid, meaning)` and
  `epbot_set_system_type(bot, position, system)` were decompiled and confirmed,
  the ten is EPBot-canonical `T` (`Holding`'s `Display`, verified via
  `epbot_get_cards`), and an earlier "crash on the second bot" was traced to a
  pointer-truncation bug in a throwaway probe, not the library. The harness
  reuses the `instinct-floor`/`scoring`/`ddss` machinery, reports IMPs/board with
  a 95% confidence interval, and dumps the worst divergent boards (the deal plus
  both tables' auctions) as concrete authoring targets. Measured at 2000 boards,
  vul none: **−2.59 IMPs/board, 95% CI [−2.83, −2.35]** — our floor trails a
  mature engine by ≈ 2.6 IMPs/board, the gap concentrated in competitive/
  contested auctions (where the books are thinnest). Still purely external
  tooling: a `libloading` **dev-dependency** only, the proprietary binary stays
  git-ignored under `/vendor/`, and the crate's default build, dependencies, and
  `instinct()` baseline are untouched.
- A **BBA/EPBot reference-bidder spike** (AI-bidder Side-track S, S.0): a new
  `bba-oracle` example that drives Edward Piwowar's EPBot engine as a black-box
  bidding oracle to benchmark our bidding against a mature, rule-based system —
  the way the open-source BEN engine was trained on BBA-bid deals. EPBot ships a
  self-contained native Linux library (`libEPBot.so`, .NET-NativeAOT), so the
  example `dlopen`s it and calls the `epbot_*` C ABI **directly — no Wine, no
  .NET runtime, no subprocess**. The undocumented ABI was recovered (objdump +
  a pure-Python decompile of `EPBotFFI`) and is documented inline: `epbot_create`/
  `_new_hand`(7 args; the four holdings as one `'\n'`-joined string)/`_get_bid`/
  `_destroy`, plus the bid-code encoding (`0/1/2 = Pass/X/XX`, contract =
  `5 + (level-1)*5 + strain`). The spike bids known hands to their textbook 2/1
  openings. Purely external tooling: a `libloading` **dev-dependency** only, the
  proprietary binary stays git-ignored under `/vendor/`, and the crate's default
  build, dependencies, and `instinct()` baseline are untouched. The full harness
  (complete auctions + a `BbaOracle` `System` + the 2/1 A/B match) is S.1, planned
  in [`docs/ai-bidder/plan.md`](docs/ai-bidder/plan.md) "Side-track S".
- A **behavioral constraint verifier** (M4.2 of the AI-bidder effort): a new
  ungated `bidding::verify` module that checks a candidate `Constraint` *accepts
  the right hands*, complementing M4.1's round-trip check that it *renders* to the
  right gloss. M4.1's `describe().to_string() == gloss` is a string compare, so it
  is blind to the body of a `described("label", closure)` escape hatch (only the
  label renders) and to whether a primitive's bounds match looser human intent
  when porting. `verify::compare(reference, candidate, rng, n)` samples `n` random
  hands and returns a `Report` (accept rates plus a bounded sample of
  counterexample hands) of where the two disagree; `accepts`/`predicate` adapt a
  `Constraint` to the comparison, a book `Rule`'s public `eval` serves as the
  porting oracle (`compare_against_rules`), and `check_examples` checks a
  constraint against hand-labeled intent. A new `tests/dsl_verify.rs` is the
  milestone measure — it catches a battery of deliberately-broken constraints
  (the canonical "5+ ♥" mis-compiled to `len(♥, 4..)`, off-by-one bands, a
  swapped `&`/`|`, dropped/extra clauses, and a `described` closure that uses `>`
  where intent is `≥`) while faithful recompiles agree. A new `verify-constraint`
  example runs the M4.3 porting loop on real book data: it pulls the 1♠ opening
  from the 2/1 books and shows a faithful recompile (0 disagreements) versus a
  broken one (caught, every counterexample a four-card spade hand), then the
  escape-hatch blind spot (two "prefers diamonds" closures that render
  identically yet disagree on equal-length hands). Offline tooling; nothing
  learned ships, and the instinct/neural/search rails stay green.
- A **DSL authoring-compiler spec** (M4.1 of the AI-bidder effort):
  [`docs/ai-bidder/dsl-spec.md`](docs/ai-bidder/dsl-spec.md) is a precise,
  pasteable English→`Constraint` prompt — the grammar (the `&`/`|`/`!` tree and
  how `describe()` renders it), a vocabulary table for every primitive with
  their exact glosses and range conventions, the `described(...)` escape-hatch
  discipline, gold `(English, Rust)` pairs harvested from the live 2/1 books, and
  explicit compile instructions. It turns book authoring into "write the meaning,
  verify, commit": an LLM proposes a
  `Constraint`, deterministic Rust verifies it. The spec is offline tooling;
  nothing learned ships. A new `tests/dsl_roundtrip.rs` is that mechanical check —
  it pins every primitive gloss and the combinator/range rendering against
  `describe()`, and reproduces 12 held-out real rules from their gloss alone (100%
  exact round-trip), so the spec is provably sufficient and `describe()` cannot
  drift from it unnoticed. The behavioral verifier (accept/reject over random
  hands) is the next milestone, M4.2.
- A **self-describing constraint DSL** (M4 of the AI-bidder effort, the
  authoring compiler's foundation): `Constraint::describe()` now renders any
  authored constraint to canonical English, the inverse of `eval()`. Until now a
  `Constraint` was eval-only and opaque — once built as `Arc<dyn Constraint>` you
  could run it but never read what it *meant*, and the corpus exporter had to
  re-guess descriptions structurally from the bid shape, divorced from the real
  logic. Now every primitive names itself (`hcp(15..=17)` → "15–17 HCP",
  `len(Spades, 5..)` → "5+ ♠", `support(3..)` → "3+ card support for partner")
  and the combinators compose those: `&` reads as a comma list ("12–21 points,
  and 5+ ♠"), `|` as ", or", `!` as "not (…)", with nested groups parenthesized.
  The new public `Description` tree carries the structure (so a terse WBF-tag
  renderer can be added later without touching primitives) and `impl Display`
  prints the prose. This is the readable face of a book — the meaning is read
  straight from the logic it bids on, so author and reader cannot drift, and it
  is the verification substrate the later English→`Constraint` LLM compiler will
  round-trip against. **Non-breaking and behaviour-preserving:** `describe()` has
  a default (`Opaque`) so external impls compile unchanged, the ~21 primitives
  were turned from anonymous closures into named structs with byte-identical
  `eval`, and the full instinct/neural/search rails stay green.
- `bidding::constraint::described(label, condition)` — a labeled escape hatch: a
  one-off predicate that carries its own meaning, where a bare `pred()` renders
  `Opaque`. Used to label the books' bespoke predicates (better-minor selection,
  Michaels/Unusual length comparisons, the RKCB keycard/queen/king holdings), so
  every node in the 2/1 corpus now describes truthfully.
- `bidding::rules::Rule::describe()` — the meaning of a rule's call, read from
  its constraint.
- A `render-book` example: prints the floor-less 2/1 books as readable prose —
  each auction, then per call its weight and the constraint's own English
  description — including the full RKCB 1430 ladder ("exactly 2 keycards, and
  holds the ♠ queen"). A stderr coverage metric counts any rules still opaque (0
  for the corpus books).
- The `export-corpus` exporter now emits a truthful `constraint` field from
  `Rule::describe()` and makes it the default `description` (precedence: a
  hand-authored `note()` label, then the truthful constraint render, then — only
  for a bare opaque predicate — the structural gloss). At 770 nodes / 2314
  records the corpus is now **0 opaque**: every record carries its real meaning,
  not a re-guessed one. The `tags` field (the controlled WBF vocabulary) is
  unchanged.
- `bidding::search_floor::SearchFloor` and `american_search()` behind a new
  `search` feature — the gated live double-dummy search bidder (M2.3 of the
  AI-bidder effort, completing Milestone 2). This is "simulations in action": at
  each non-forced decision the floor *thinks* before it bids. It wears the same
  deterministic safety shell as the neural floor — auction-determined forced
  situations delegate to `instinct()` verbatim, so the §0.4 rails hold by
  construction — but in the judgement middle it no longer trusts the net's single
  forward pass. Instead the net is only a *prior*: it shortlists the top-`k` legal
  calls, prices each by cardplay over `n` sampled layouts with `ev_all` (the M2.2
  evaluator), and re-seats the evaluated calls onto an EV-ranked band above the
  prior tail, so the driver bids the highest-EV call while every legal call keeps
  a sane fallback logit and `Pass` stays finite. "Net proposes, search disposes."
  The rollouts finish under self-play with our own distilled net
  (`american_neural()`) — the continuation policy M3.2 will iterate. The knobs
  (`layouts`, `shortlist`, `temperature`) default to *strength, not latency*
  (`n = 128`, `k = 8`, ≈ 1.4 s per decision — `n` and `k` raised together so the
  wider shortlist's extra candidates are scored against tight EV estimates, not
  noise); shrink them for a faster, noisier bidder. `classify` stays
  a pure function despite sampling: the rollout RNG is seeded deterministically
  from the decision's feature vector, so the same hand and auction always yield
  the same logits (invariant §0.5). The `search` feature implies `neural-floor`
  (it needs the prior net and the forced-rails shell); the default build,
  `instinct()`, and `american()` are untouched — this is an added gated
  option, never a replacement. Seven gated tests cover the five §0.4 rails against
  the shelled search bidder, determinism, and the EV-band ordering. A gated
  `search-floor` example A/Bs it against the deterministic floor (it should beat
  the hand-written ladder) and against the distilled net (search should beat the
  raw policy it proposes from). The search is slow by design — every decision is a
  double-dummy search — so a real interval needs a long run; the live bidder is
  also the teacher whose improved EVs M3 will distill back into a fast forward
  pass.
- `bidding::ev::{ev, ev_all}` — the call-EV evaluator (M2.2 of the AI-bidder
  effort). For a candidate call it answers the question the rule books never
  could: *what is this call worth?* — by Monte-Carlo rollout grounded in
  cardplay. It samples layouts with `sample_layouts`, seeds the candidate onto
  the prior auction, lets a continuation policy bid each layout out, scores the
  contract reached double dummy, and averages the result in the actor's favour.
  `ev_all` scores a slate of candidates over the *same* layouts and a single
  double-dummy solve per layout, so the cost is `n` solves rather than `k · n`;
  `ev` is the one-call wrapper. The continuation policy is a `System` parameter,
  not hardwired — callers pass the deterministic `american()` for now, and
  the M3 search-improvement loop will swap in successive nets without touching
  this code; all four seats bid the same policy (a self-play assumption). EVs are
  average scores in points (positive good for the actor); a call illegal in the
  prior auction, or an auction so tight no layout can be sampled, scores `NaN`
  (read as *no signal*). The evaluator is ungated. Five tests cover the ranking
  sanity (a sound game out-values a hopeless grand, which prices out negative),
  determinism under a fixed seed, the illegal-candidate and infeasible-auction
  `NaN` paths, and the empty-slate case. This evaluator is the shared engine
  behind both the M2.3 live search bidder and the M3 offline training targets.
- `bidding::sampler::sample_layouts` — the constrained layout sampler (M2.1 of
  the AI-bidder effort, starting Milestone 2). The inverse of `Inferences`: given
  the player to act, their hand, and their seat, it deals the other three hands
  at random so that LHO, partner, and RHO each fall within their shown length and
  point ranges. It pins the actor's thirteen cards into a partial deal and
  rejection-samples on top of `contract_bridge::deck::fill_deals`, so accepted
  layouts satisfy every range by construction; an attempt budget bounds tight or
  infeasible auctions, which may return fewer layouts than requested rather than
  loop forever. This is the substrate the M2.2 call-EV evaluator will score each
  candidate call over by double dummy. The sampler is ungated — the natural
  completion of `Inferences`, which was built for exactly this — and takes the
  caller's RNG, so the learned floor stays deterministic. Six tests cover
  soundness (a property test over random hands), the count being met on feasible
  auctions, non-degenerate coverage, and termination on an infeasible auction.
- `bidding::constraint::point_count` — the upgraded-points scalar (raw HCP plus
  the fuzzy-strength `upgrade`) that the suit-oriented `points` constraint gauges
  and that `Inferences` records its point ranges on. A single definition now
  shared by the new sampler and the `Inferences` soundness test, so the value can
  never drift from the ranges checked against it.
- `rand` is now a direct dependency (`0.10`). It was already compiled
  transitively via `contract-bridge`'s `rand` feature, so the dependency tree is
  unchanged; the sampler simply names it directly to take the caller's RNG.
- `bidding::neural_floor::NeuralFloor` and `american_neural()` behind the
  `neural-floor` feature — the safety shell that makes the distilled net usable
  as a floor (M1.3 of the AI-bidder effort, completing Milestone 1). The shell is
  a drop-in `Classifier`: in auction-determined forced situations (partner's live
  takeout double, an auction that forces game, a just-made transfer over our
  strong notrump) it delegates to the deterministic `instinct()` ladder verbatim
  — the learned net is never trusted on the rails — and everywhere else it
  returns the net's logits legality-masked with `Auction::can_push`, keeping
  `Pass` finite so a distribution always exists. `american_neural()` mirrors
  `american()` with this floor swapped in; the deterministic `instinct()`
  floor stays the default and baseline (nothing is removed, an option is added).
  Five gated tests pin the five §0.4 safety properties against the shelled net.
  Hand-conditioned game forces are left to the net as judgement, measured by the
  example below, not hard-railed.
- `examples/neural-floor` behind the `neural-floor` feature — the A/B measurement
  for the learned floor (M1.4 of the AI-bidder effort). Two duplicate matches
  with 95% confidence intervals: the neural floor against the deterministic floor
  (the distillation parity target) and against bare books (the floor's worth).
  At 8000 boards (vul none) the neural floor is at parity with the deterministic
  floor — −0.01 IMPs/board, CI [−0.05, +0.03], containing zero — while preserving
  the floor's gain over bare books (+0.59 IMPs/board, CI [+0.52, +0.66], against
  the hand-built floor's recorded ≈ +0.5). The distilled floor *equals* the
  hand-written one on the harness; the machine now does the floor's job.
- `bidding::neural` behind the new `neural-floor` cargo feature — the in-crate
  forward pass for the distilled floor (M1.2 of the AI-bidder effort). A
  hand-rolled `f32` matmul + ReLU that embeds the trained `american_v1`
  weights with `include_bytes!` and evaluates `classify(features) -> Logits`
  with no ML dependency. The feature is off by default, so the standard build is
  byte-for-byte unchanged. A parity test reproduces the trainer's candle logits
  on the exported fixture within `1e-3` and matches the arg-max (chosen call)
  exactly. The deterministic safety shell — legality masking and forced-situation
  overrides — follows in M1.3.
- `trainer/` — the off-crate distillation trainer (M1.1 of the AI-bidder
  effort). A self-contained cargo workspace built with `candle` that is never
  compiled by the `pons` build (an empty `[workspace]` table decouples it, and
  the crate carries no ML dependency). It fits a `160 → 256 → 256 → 38` MLP to
  the teacher's softmax by soft-target cross-entropy and exports the weights as
  a flat little-endian `f32` artifact plus a versioned sidecar and a
  forward-pass parity fixture into `src/bidding/weights/`. On a 484k-row dataset
  it reaches ≈94% held-out top-1 agreement with `american()` (validation
  cross-entropy 0.25 against a 0.20-nat teacher-entropy floor). The weights ship
  in-repo for M1.2 to embed and run by a hand-rolled forward pass; the library
  itself is unchanged.
- `examples/teacher-dump` — the distillation dataset generator (M0.4 of the
  AI-bidder effort, completing Milestone 0). Bids out random boards with
  `american()` and writes one `(features, teacher_softmax)` row per decision
  to a flat little-endian `f32` file (160 features + 38-way softmax = 198 floats
  per row) plus a JSON sidecar pinning the feature version, teacher, seed, git
  SHA, and counts, plus a sibling `.tags` file (one `u8` per row marking
  contested-phase decisions) so the trainer can split held-out agreement by
  phase. A dev tool, not part of the library API.
- `examples/export-corpus` — the description-corpus exporter (M0.2 of the
  AI-bidder effort). Walks the floorless 2/1 books, recovers each authored node
  through `Classifier::as_rules()`, and emits one JSONL record per `(node, call)`
  with WBF tags derived structurally from the auction (and the rule's `note`
  label as the description where present). A dev tool, not part of the library
  API.
- `bidding::rules`: rules can now carry a human-readable `label` (M0.1 of the
  AI-bidder effort). `Rules::note("…")` chains after `rule(…)` to label the
  preceding rule, `Rule::label()` reads it back (empty by default, so the
  authored books are unchanged), and `Classifier::as_rules()` downcasts a
  type-erased trie classifier back to its authored `Rules` — the hook the
  description-corpus exporter uses to recover each node's calls and labels.
- `bidding::features` — a versioned feature extractor (M0.3 of the AI-bidder
  effort). `features(hand, context)` returns a `Vec<f32>` of exactly 160
  values (`FEATURES_LEN`) encoding five blocks: per-suit rank/length/honor
  indicators (76), global hand evaluations (HCP, upgraded points, Fifths,
  CCCC, NLTC, balanced flag — 6), laws-only auction facts (our/their strains,
  contract-to-beat, partner's last bid, penalty, seat, we-opened — 36),
  inferences per seat (length ranges, point ranges from `Inferences::read` —
  40), and relative vulnerability (2). Layout is versioned as
  `FEATURES_VERSION = 1`; block offsets are exported as `OFFSET_*` /
  `LEN_*` constants.
- `bidding::constraint::cccc_at_least`, gating on the Kaplan–Rubens CCCC
  ("Four C's") evaluation newly available as `contract_bridge::eval::cccc`
  (validated bit-for-bit against Richard Pavlicek's published distribution
  over all 635,013,559,600 hands). CCCC weighs honor placement together with
  shape, making it the right gauge for suit contracts; Fifths remains the
  gauge toward notrump, especially 3NT. The `check-nltc` example gains a CCCC
  column: on 2000-deal pair sums it correlates 0.72 with par suit-contract
  tricks, in the same league as BUM-RAP+ (0.75) and ahead of NLTC.
- `bidding::constraint::points`, the fuzzy-strength gauge: raw HCP plus an
  `upgrade` (also exported) for clean unbalanced hands. An unbalanced hand
  whose short suits (≤ 2 cards) waste no honors gets +1, and +1 more with ten
  or more cards in its two longest suits; any A/K/Q/J in shortness voids the
  upgrade, except the working Ax and Kx. Balanced hands never upgrade, so
  `points` coincides with `hcp` for them.
- `bidding::constraint::fifths`, gating on Thomas Andrews's computed point
  count for 3NT (an `f64` range on the same 40-point scale as HCP).
- The `fuzzy-strength` example: an A/B duplicate match where the same 2/1
  books bid with fuzzy strength on one side and raw HCP on the other, via a
  per-thread ablation hook read at classification time. `--policy
  points|fifths|both` ablates the two gauges separately.
- `bidding::inference`, a per-player summary of what the calls have shown —
  each suit's length range and the point range — accumulated across an auction
  and derived purely from the calls under standard 2/1 meanings (`Inferences`,
  `Inference`, the inclusive `Range`, and a `Relative` seat). It fills the gap
  the eval-only `Constraint` leaves: a `len(..)` rule's length cannot be read
  back out, so the summary is reconstructed from the calls (like the instinct
  floor's `Interpretation`). Every range only ever *narrows*, so a hand that
  made the calls always falls within it — the soundness the future constrained
  sampler relies on; the deriver stays silent on artificial structures
  (Stayman, transfers, the strong-2♣ responses) rather than misread them.
- `bidding::constraint::partner_shown_len` and `partner_shown_points`, crisp
  predicates reading partner's guaranteed minimum from `Inferences` — what
  partner's calls *promised*, where `support` grades what *our* hand holds.
- The `inference-floor` example: an A/B duplicate match measuring the
  inference-aware floor against the pre-inference floor via the
  `set_inference_aware` ablation hook.

### Changed

- **Refreshed the Transfer Lebensohl A/B numbers in the 21GF ledger (ledger #80)
  after the relay/floor/stretch updates above.** Re-measured on the current book
  (`lebensohl-ab`, 200k filtered/cell, seed 20260620, perfect-defense): `Transfer`
  vs `off` is now −0.010/−0.022 IMPs/board (−0.103/−0.226/div, none/both), up from
  −0.048/−0.065; `Transfer` vs `plain` is +0.049/+0.082 board (+0.873/+1.463/div).
  Documentation only — no code change.

- **Transfer Lebensohl after a takeout double is now the default advance (was
  opt-in `Off`).** After `(2X)–X–(P)` the advancer now carries `Transfer`
  Lebensohl by default — `set_advance_sohl_style`'s default flips from `Off` (the
  flat `advance_double` ladder) to `LebensohlStyle::Transfer`. A deeper
  perfect-defense re-measure (200k filtered boards/cell, both vulnerabilities)
  makes it a clear win over the flat ladder: **+0.145 / +0.227 IMPs/board**
  (none/both; reproduced +0.139/+0.234). The earlier "DD-neutral → keep opt-in"
  verdict was an artifact of the optimistic scorer (corrected by the `ns_score`
  perfect-defense fix). `Plain` also flips DD-positive (+0.089/+0.139, was
  −0.108/−0.050) but stays dominated. `set_advance_sohl_style(LebensohlStyle::Off)`
  recovers the prior flat-ladder default; `Plain` / `Rubensohl` remain selectable.
- **`scoring::ns_score` now assumes perfect-defense doubling (breaking); the
  optimistic variant is removed.** A contract that fails double dummy is now scored
  *doubled* — a competent defense always doubles what it can beat, so in a
  double-dummy model the opponents always hold the red card and pricing a failing
  overbid undoubled modelled an opponent who *cannot* double. This folds the old
  `ns_score_doubling_failures` behavior into `ns_score` and **removes
  `ns_score_doubling_failures`** (it is now redundant). Every A/B harness that
  scored with plain `ns_score` (`instinct-floor`, `inference-floor`,
  `leaping-michaels-ab`, `lebensohl-ab`, `stayman-abc`, `nt-shape-contested`, …)
  is corrected to perfect defense by this one change; `neural-floor`'s dual
  optimistic/perfect-defense reporting collapses to the single (now correct)
  measure. The cardplay EV evaluator (`bidding::ev`) is unaffected — it already
  used the doubling behavior. *Consequence:* convention A/Bs previously reported on
  the optimistic bound (Leaping Michaels, Transfer Lebensohl, Stayman, the 1NT
  redesigns, the BBA gap, …) should be re-validated under perfect defense, since
  competitive conventions can shift most where overbids go doubled.
- **Renamed the `two_over_one*` system to `american*` (breaking).** The 2/1
  game-force system is now named by identity: `two_over_one` → `american`
  throughout. The module `bidding::two_over_one`
  → `bidding::american`; every function follows (`two_over_one()` → `american()`,
  `_classic`, `_wide_6322`, `_neural`, `_neural_v2`, `_neural_search`, `_search`,
  `_search_with`, `_constructive_floor`, and `bare_two_over_one()` →
  `bare_american()`), as do the crate-root re-exports. The `export-corpus`
  `--system` value `two-over-one` is now `american`,
  and the example `cargo run --example two-over-one` is now `--example american`.
  Bundled neural weights were renamed in step (`weights/american_v{1,2}.f32`,
  `american_v1_search.f32`). Prose still calls the system "2/1" / "Two-over-One
  Game Forcing" — only identifiers and names changed.
- **The optional search-target neural floor (`american_neural_search()`) is
  now the round-2 net (AI-bidder M3.2 / M3.3).** The M3.1 search-dump was
  regenerated with the round-1 net as the rollout continuation policy *and* the
  doubling-aware `ev_all` (104 476 rows / 10k boards), then distilled into the
  same `160→256→256→38` net at the same hyperparameters. The new weights replace
  the old ones in place — the public API, the safety shell, and `instinct()` (the
  default and baseline) are untouched. **A/B (20 000 boards), round-2 vs round-1,
  on the default perfect-defense measure** (failing contracts priced doubled, as
  real opponents would): **+1.661 IMPs/board vul none** (CI [+1.550, +1.772]),
  **+2.069 vul both** (CI [+1.957, +2.181]) — round 2 learned to *stop reaching
  doubled-down contracts*, the discipline its doubling-aware targets reward.
  Beats the deterministic floor on the same measure (+0.178 vul none, +1.716 vul
  both; CIs exclude 0), so it is the M3.3 champion. On the optimistic double-dummy
  bound (which scores down contracts undoubled) the round-1→round-2 step is parity
  vul none (+0.046, CI [−0.034, +0.127]) and a gain vul both (+0.424), so round 2
  is never worse on either bound.
- **The neural-floor A/B harness now reports perfect-defense doubling as the
  default measure, double-dummy as the optimistic bound.** `examples/neural-floor`
  used to headline the double-dummy swing — which prices every failing contract
  undoubled, i.e. as if the opponents passed it out instead of doubling what they
  can beat — and relegate perfect-defense doubling to a sub-line. The head-to-head
  verdict now runs on the PD swing; the DD swing is still printed as the optimistic
  bound. (The vs-bare floor-worth target stays on DD: PD vs a passing opponent is a
  scorer artifact, since bare never owns a failing contract.)
- **The `fifths` strength gauge no longer scores Fifths alone — it averages
  Fifths with an honor-weighted companion.** Fifths is tuned for 3NT (it
  rewards aces and tens, discounts kings and queens), so it misjudges an
  *initial* notrump bid that may yet land in a suit contract. Every
  `fifths(...)` range — the 1NT/2NT openings, opener's 1NT/2NT rebids, and the
  balanced descriptions in the game-force, Jacoby, and strong-2♣ structures —
  now bands the **average of Fifths and a companion count**, halving the 3NT
  bias toward a real-honor scale. The notrump *raises* (1NT–2NT, 1NT–3NT,
  quantitative 4NT) are unaffected; they already gauge plain `hcp`, which is
  where Fifths-alone would have been fine anyway. A new `FifthsCompanion` enum
  and `set_fifths_companion` hook (both `#[doc(hidden)]`, A/B only) pick the
  companion: **BUM-RAP is the default**, chosen by the new `fifths-companion`
  A/B example — it edged Milton Work HCP across every vulnerability (combined
  −0.28 IMPs per divergent board to the HCP team over 120k boards, ~2.9σ;
  whole-match ≈ −0.01 IMPs/board, as only ~2% of boards diverge). User-visible
  effect: a tens-rich light hand can no longer sneak into a notrump range on
  Fifths, and a quack-heavy hand is no longer shut out of one.
- **The live-search floor's rollout continuation is now self-play against the
  search-target net (AI-bidder M3.2 round 2).** `SearchFloor`'s `POLICY` — the
  policy that finishes every rollout auction so `ev_all` can score a candidate —
  was the teacher-distilled `american_neural`; it is now
  `american_neural_search` (the M3.2 round-1 net). Each round's distillation
  targets are thus scored by the previous round's policy: "feed the improved net
  back into the continuations." Behind the `search` feature; affects
  `american_search` only.
- **The `practice-bidding` example now bids with the learned floor by default.**
  A new `--floor` flag selects the bots' (and the "Bot's opinion" feedback's)
  floor: `neural-search` (the M3.2 search-distilled net) or `instinct` (the
  deterministic ladder). The default is `neural-search` when built with
  `--features neural-floor`, else `instinct`, so the no-feature build still
  compiles and runs. The deterministic floor was only ever the pre-AI-bidder
  default; with the net compiled in, practice now feels what the net plays.
- **The instinct floor is now a milestone bidder, and it floors the constructive
  book too.** Two coupled changes, worth **+1.12 IMPs/board** (instinct-floor A/B,
  4000 boards, up from ≈+0.5 for the old contested-only floor):
  - *General game/slam selection.* The floor's game bidding was special-cased to
    three forced auctions (strong-notrump responder, strong 2♣, takeout-double
    advance). It now also fires on a general trigger: our own `point_count` plus
    the **sound floor** of partner's shown points (`Inferences::partner().points.min`)
    reaching a milestone — 25 for game, 33/37 for small/grand slam. Below game it
    takes the cheapest milestone (a known eight-card major fit → 4M, else 3NT,
    dropping to 5m only when a suit they bid is unstopped); in the slam zone it
    bids 6M/6NT/7M/7NT. Because the trigger reads the *guaranteed* minimum, it
    never overbids a hand that could be weaker than counted — it only stops
    passing out cold games.
  - *Constructive flooring.* `with_floor` now attaches the deterministic instinct
    ladder to the **constructive** book as well, not just the contested books.
    Uncontested off-book auctions previously fell through to a pass — e.g.
    `1♦–1♥–1NT` was passed out on a balanced 16 opposite the 12–14 rebid, a cold
    3NT (the learned neural/search floors don't help here: they are wired onto the
    contested books only). They now reach the milestone. `american_strawberry`
    floored the constructive book by hand for the same reason; that is now the
    default and its bespoke block is gone.
- **The auction inference reads limited rebids and raises.** `Inferences` now
  narrows the shown point range for opener's 1NT rebid (12–16) and **jump** 2NT
  rebid (18–21, the slam-enabling minimum), the single (6–10) and limit (10–12)
  raise, and the 1NT response (6–12) — each a sound bound, read only when the
  opponents stay silent (a competitive 2NT or raise can be off-meaning). A latent
  bug in the highest-contract tracker (`outranks` ranked strain before level, so
  `2♣` did not outrank `1♠`) was fixed in passing; it gated the jump detection the
  new readings rely on. Sharper partner-strength is what lets the milestone floor
  reach slams.
- **BBA/EPBot is now bundled as a git submodule.** With redistribution permitted
  by its author (free for non-commercial use), the reference engine that the
  `bba-match` and `bba-oracle` examples benchmark
  against ships as the `vendor/bba` submodule (`github.com/EdwardPiwowar/BBA`,
  pinned). Fetch it with `git submodule update --init vendor/bba` and the
  examples' default library path resolves — no manual download or `BBA_LIB`
  needed. The submodule is excluded from the published crate, so the packaged
  tarball is unchanged. Published comparison numbers credit EPBot as the
  reference engine.
- **The 2/1 system is now sharp on shape, fuzzy on strength.** Roughly a
  hundred rule sites across openings, responses, raises, rebids, the
  game-force structure, Stenberg, weak twos and their Ogust ladder, strong
  2♣, competition, defense, Rubens advances, and the instinct floor swap raw
  `hcp` for `points` wherever strength gates a suit-oriented call — including
  caps, so a clean shapely maximum upgrades *out* of a weak two. Boundary
  pairs (an overcall cap and its double-then-bid floor) convert together so
  no upgraded hand falls between bands.
- Notrump-defining ranges (1NT/2NT openings, opener's balanced 12–14 / 18–19
  / 22–24 / 25–27 rebids, the 13–15 and 15–17 balanced 3NT rebids) gauge
  `fifths` over half-open bands (`hcp(15..=17)` → `fifths(15.0..18.0)`), so a
  queen-heavy 20-count now opens 1♣ planning a 2NT rebid while a ten-rich
  14-count upgrades into the strong notrump. Responder's notrump ladders
  (including the whole BTU structure) intentionally stay on raw HCP for a
  follow-up. The last-resort 1NT rebid fallbacks became unconditional
  (`hcp(0..)`) so light or off-band openers always retain a book call.
- **The instinct floor now reads partner's shown shape.** In a forced-to-game
  auction it bids a *known* eight-card-plus major fit (our five-card suit
  opposite partner's shown three-card support, or our support opposite
  partner's shown five-card suit — e.g. opposite our 1NT after partner's
  natural, forcing three-level major) rather than the shape-blind 3NT.
  Measured by the `inference-floor` example over 20,000-board A/B matches, it
  scores non-negative at every vulnerability (+0.00 to +0.01 IMPs/board, ~1.5
  to 3 IMPs per divergent board) while diverging on only ~0.3% of boards — the
  triggering auctions are rare, so the gain is small but real and never a
  regression.
- Measured by the `fuzzy-strength` example over 20,000-board A/B matches per
  configuration, the combined policy scores level with raw HCP (runs between
  −0.04 and +0.03 IMPs/board, within noise at this sample size) while
  diverging on ~21% of boards. Ablated at 20,000 boards apiece, each half
  alone also measures −0.01 IMPs/board (`points` diverging on 17% of boards,
  `fifths` on 5%). The policy is kept for its descriptive value — sharper
  announced ranges at equal measured strength — and the ablation hooks stay
  for tuning the halves separately.

### Removed

- **True Rubensohl (`LebensohlStyle::Rubensohl`).** The `2NT`-club-transfer /
  two-way-low-transfer variant of Lebensohl — added earlier this cycle as an opt-in
  fourth style — is removed, along with `rubensohl_responder`,
  `complete_two_way_transfer`, `two_way_transfer_rebid`, and their wiring in both the
  overcalled-`1NT` and takeout-double-advance contexts. It never measured a win
  (its only edge over the default `Transfer` is DD-blind right-siding:
  `+0.001/−0.023 IMPs/board` head-to-head, neutral non-vul / a loss vul), and the
  `Transfer` refinements that prompted a fresh look (top-step clubs transfer, delayed
  cue, `(2♦)` Smolen) don't port onto it — its transfer machinery consumes the very
  seams those refinements use. The remaining styles are `Off`/`Plain`/`Transfer`;
  the default and `american()` are unchanged. The `--ns rubensohl` / `--ew rubensohl`
  options on `lebensohl-ab` and `sohl-after-double-ab` are gone.
- **The strawberry 2/1 variant (`american_strawberry`,
  `bare_american_strawberry`) and its three convention modules.** This was a
  `NATURAL`-family 2/1 with a few polish.club conventions layered on — Strawberry
  Stenberg 2NT (`stenberg`), BTU strong-1NT responses (`btu_notrump`), and a
  **book** overlay of Rubens transfer raises (`american::rubens`). The book
  Rubens collided with the new floor Rubens: it authored the transfer raise only
  for 10–12 points, so a game-strength raise leaked past it to the floor's
  cue-raise, giving the *same* limit-plus raise two different (and strength
  inverted) calls. Rather than maintain two Rubens implementations, the variant
  is dropped; the keyless floor's Rubens is the single source of truth. The
  conventions remain in git history if ever wanted.
- **The in-development second system and the `nltc` / `nltc_at_most` DSL
  constraints.** The project refocuses on a single mature 2/1 (`american`)
  system, on top of which other systems can later be built. The second authored
  system (`polish_club()` / `bare_polish_club()`, the `Tag::POLISH_CLUB`
  constant), its `polish-club-reference` and `bba-wj-reference` examples, and the
  `export-corpus --system` selector are dropped — `export-corpus` now always
  walks the 2/1 books. The two NLTC *constraint* primitives `nltc(range)` and
  `nltc_at_most(losers)`, used only by that system, are removed; the NLTC hand
  *evaluator* (`eval::NLTC`, the `bba-match`/`bba-oracle` 2/1 reference, and the
  `calibrate-eval` example) is unaffected. `nltc_at_most` shipped in 0.9.0, so
  its removal is breaking for any direct user. Everything remains in git history.
- **The direct `serde_with` dependency.** It was wired into the `serde` feature
  but never referenced — no `#[serde_as]`, no custom (de)serialization anywhere
  in the crate. Dropped from `[dependencies]` and from the `serde` feature list.
  No API change: `serde` derives on the stats and inference types are unaffected,
  and `serde_with` is still pulled transitively by `contract-bridge`/`ddss` (for
  *their* serde impls) when `--features serde` is enabled.

### Fixed

- **A passed hand's both-majors double of an opponent's 1NT now escapes the
  opponents' redouble instead of sitting in `1NTxx`.** The passed-hand
  `NaturalLandyDouble` is a *takeout* double (both majors, ≥5-4), advanced like
  Landy `2♣` — but only the `[P,P,P,1NT,X,P]` advance (they pass our double) was
  authored. When the 1NT side **redoubled** (`[P,P,P,1NT,X,XX]`) the advance fell
  to the floor, which passed, leaving us in `1NTxx` for a routine −760/−1000. The
  redoubled node now mirrors the pass case: the advancer runs to the longer major
  (or `2♦` relay → doubler names the major), since a takeout double must never be
  left in. Measured against BBA on isolated defense-to-1NT
  (`bba-match --isolate-defense --our-floor neural-v3`): on the identical
  2000-board seed the eight `1NTxx` catastrophes drop to **zero** and our defense
  recovers from **−0.034 → −0.003 IMPs/board** (the entire +63-IMP gain is the
  penalty-double bucket, −95 → −32); an independent seed scores **+0.043**.
  `american_neural_v3` inherits the node for free — it is a floored leaf. The
  *direct*-seat 15+ penalty double redoubled is left to the floor: running from a
  penalty (not takeout) double is single-dummy judgment, not the unambiguous run
  a takeout double demands.
- **`examples/grand-probe` now declares `required-features = ["search"]`** so
  `cargo build --examples` (no features) no longer fails resolving the
  `search`-gated `search_floor`/`american_search_with` imports it uses.
- **A partial book node no longer shadows the floor: a hand it rejects now
  falls through to the floor instead of producing a degenerate all-`-∞`
  result.** A book node admits only the calls whose constraints match the hand,
  so a deliberately partial node — the codebase's "partial nodes, the floor
  catches the rest" design — left a hand it did not cover with no probability
  mass. Resolution returns the most specific (exact) node first and only walks up
  to the `Always`-guarded floor when *no* exact node matches, and the classify
  wrappers did not check the result for mass — so the partial node shadowed the
  total floor and the driver, finding no finite call, silently passed (the 7NT
  degenerate-result report). Resolution is now **mass-aware**: `Trie` gains
  `classify_floored`, which consults the exact node first and, only when it
  yields no mass for the hand, walks up to the fallback chain — reaching the
  always-total instinct floor. The bare-book ablation (no floor attached) is
  unchanged: it still returns no mass and the driver passes. Telemetry benefits
  too — `classify_with_provenance` now attributes the floor (`depth == 0,
  fallback == Some(_)`) on a fall-through, the "next node worth authoring"
  signal. No authored node changed; the only behavior difference is for hands
  that previously degenerated. Regression tests in `trie`.
- **The instinct floor's 3NT game milestone now requires a stopper in their
  suits, so it never bids notrump game into an unstopped enemy suit in a
  competitive auction.** The milestone game/slam ladder already fires in
  competition (an overcall shows 8+, a takeout double 11+, and the combined-points
  trigger has no "uncontested" guard), and its slam and minor-game rules already
  gate on `stopper_in_their_suits()` — but the plain 3NT game rule did not, so a
  game-values hand opposite a competitive overcall could bid 3NT with the
  opponents' long suit wide open. The guard is **vacuous when uncontested** (no
  suit of theirs to stop), so constructive auctions are unchanged; only contested
  ones differ — a stopperless, fitless game-values hand now passes (or competes)
  rather than bidding a doomed notrump. Floor worth on the `instinct-floor` A/B is
  preserved (6000 boards: **+1.10 IMPs/board** vul none, **+0.40** both), and the
  telemetry confirms the floor's competitive judgement firing — milestone games
  (`3NT  1♦ P 1♥ P 2♦ P`) and reopening takeout doubles (`X  1♦ 1♠ P P`) — in the
  off-book tail it owns.
  openings gate on `fifths` (which downgrades quack-heavy hands), but the
  inference layer recorded their point ranges on the raw-HCP scale, so a balanced
  quack-heavy 19-count (e.g. ♠KQJx ♥KQx ♦KQx ♣Kxx — 17.6 fifths) opened 1NT yet
  fell outside the announced 14–18. Because `hcp − fifths ≤ 1.6`, the sound
  envelopes are 1NT **14–19** (was 14–18) and 2NT **19–23** (was 19–22); the
  inference now uses them. Caught by the `opening_inference_contains_the_opener`
  proptest (intermittent: it needed a quack-heavy draw). Announced NT point
  ranges widen by one at the top; no bids change.
- **`grand-probe` example and the AI-bidder doc comments compile cleanly under
  the CI lint gates** — the `grand-probe` example tripped `needless_range_loop`
  and `redundant_closure` (clippy `-D warnings`), and ~20 intra-doc links in the
  bidding modules were unresolved or ambiguous under rustdoc `-D warnings`. Both
  gates had been red since the AI-bidder modules landed; fixed with no behavior
  change.
- **Call-EV evaluator now prices contracts under perfect-defense doubling**
  (AI-bidder, M3.1 follow-up). The cardplay rollout already assumes optimal
  defense, but it left the *doubling* decision to the weak continuation policy,
  which under-doubles — so a failing sacrifice priced at its cheap *undoubled*
  penalty and the live double-dummy search chased phantom saves into runaway
  competitive auctions (probing the M3.1 `search-dump` targets found 7NT chosen
  as often as games, on ~0%-making hands, purely because the rollout never
  doubled the save). `bidding::ev::ev_all` now scores every layout with
  `scoring::ns_score_doubling_failures`: a contract that fails double-dummy is
  scored *doubled*, extending the perfect-defense assumption to the penalty. In a
  50-board census this cut slam-or-higher advancing calls from **12.6% to 0.4%**
  (grand slams **6.2% → 0%**) and the count of off-book search decisions by
  **62%** — the auctions stop spiralling once saves are priced honestly. This
  changes only the gated `search` floor and the distillation targets it produces;
  `instinct` and the distilled neural floor are untouched.
- The instinct floor no longer passes a game-forcing 2♣ auction out below 3NT.
  The strong 2♣ opening is forcing for one round only; it is responder's
  *answer* that settles the game force — the 0–3 HCP double negative (2♥) keeps
  the option to stop short, while the waiting 2♦ or any positive commits both
  partners to at least game. The floor now reads that force off responder's
  call (its second convention, after the strong notrump), so off-book
  continuations such as `2♣ – 2♦ – 2NT` reach 3NT or the cheapest major game
  instead of dying in a partscore, while `2♣ – 2♥ – 2NT` may still be passed.
  The forced-to-game rules (for both the 2♣ and strong-notrump conventions)
  also step aside whenever we are penalizing the opponents with a double of our
  own, so a game force never pulls partner's penalty double of a partscore.
  Instinct now reconstructs this forcing state through a small `Interpretation`
  of the auction it owns, keeping system interpretation out of the mechanical
  `Context`.

### Removed

- **`scripts/fleet/` — the distributed-dump ssh harness.** Superseded by the
  `gib` tool: the expensive double-dummy work is now produced as a portable GIB
  file, and GIB files from distinct seeds concatenate with `cat`, so spreading
  the work across machines no longer needs an orchestrator (run a per-seed
  `gib generate` shard on each box and merge). The remaining seed-shard-and-merge
  workflow is documented in `docs/shared-machine-data-gen.md`; `scripts/idle-run.sh`
  (the SCHED_IDLE scavenger) stays.

## [0.9.0] — 2026-06-13

### Added

- `scoring`: per-board scoring primitives promoted from the `instinct-floor`
  example so every simulation harness shares one scorer —
  `final_contract(auction, dealer)` (the last bid with its doubles and the
  absolute declarer), `ns_score(result, tricks, vulnerability)` (the signed
  NS score of a final contract priced from a double-dummy `TrickCountTable`,
  0 for a pass-out), and `imps(diff)` (the standard WBF scale). The
  `instinct-floor`, `practice-bidding`, and `defend-2sx-or-3nt` examples now
  use these instead of private copies.
- `Table::bid_out_from`: continue a seeded auction (positioned from the
  dealer) until it ends; `Table::bid_out` is now this with an empty seed.
  This is the driver for forcing an auction prefix and letting the systems
  finish the board, as the `defend-2sx-or-3nt` example does with its
  `(2♠) X (P) + decision` seeds.
- `american_strawberry()` (with its floor-less `bare_american_strawberry()`
  ablation), an opt-in variant of the 2/1 system that layers in three optional
  conventions from the author's *Strawberry Polish Club* notes
  (<https://polish.club>), each chosen to stay applicable to a 2/1 framework,
  while leaving the canonical `american()` untouched for A/B comparison.
  Exported at the crate root alongside `american`. To avoid authoring a node
  per artificial continuation, the variant also floors its *constructive* book;
  with the strong-notrump instinct rules a game-forcing 1NT auction reaches game
  even where the book stops (covered by an end-to-end test that plays full
  auctions through the stance).
  - **Strawberry Stenberg 2NT** (`stenberg`) replaces Jacoby 2NT as opener's
    rebid after `1M – 2NT`: the cheapest step shows a minimum, every other rebid
    a maximum that describes a side fragment, a five-card side suit, or a
    two-suiter, with RKCB 1430 below the agreed major.
  - **BTU responses to the strong 1NT** (`btu_notrump`) replace the baseline
    1NT response block — BTU Stayman 2♣ (with the invitational 5=♠ relay and
    Smolen), Jacoby transfers 2♦/2♥ with super-accepts, minor-suit transfers
    2♠/2NT, Puppet Stayman 3♣, splinters, South African Texas, and the
    quantitative ladder — while keeping the 2NT-strength and 18–19-rebid
    structures shared with the baseline.
  - **Rubens (transfer) advances** (`rubens`) overlay the defensive book: over
    partner's overcall, the step just below partner's suit is a limit-plus
    transfer raise that partner completes.
- The `practice-bidding` example: an interactive harness for judging the
  bidding books by sitting at the table. It shuffles random boards — optionally
  rejection-sampled against `--min-hcp` / `--max-hcp` / `--min-suit` bounds on
  your hand — and lets you bid one seat while pons bots bid the others:
  `--bots 3` seats bots at all three other chairs, `--bots 1` gives you a bot
  partner against silently passing opponents for uncontested practice. After
  each of your calls it prints the book's top three candidates with softmax
  weights and tracks your agreement rate with the bot's first choice. When the
  auction ends it reveals the deal and renders two double-dummy verdicts via
  `ddss`: the final contract scored on the actual layout (with the par score
  for reference, both signed from your side), and its make rate, mean score,
  and trick range across `--simulations` reshuffles of the two unseen opposing
  hands with your side's cards held fixed.
- `bidding::instinct`: the instinct bidder, a keyless floor for off-book
  auctions. `instinct()` is one context-driven `Rules` ladder that answers
  *every* auction with a sane natural action: penalty pass only with a trump
  stack, raises of partner's suit with three-card support and rising strength
  per level, notrump and five-card-suit overcalls when we have not bid,
  takeout doubles of their low suit bids, and pass. Partner's *live takeout
  double* (the auction ends `… (bid) X (Pass)` with their suit bid at the
  three level or below doubled) is recognized mechanically and never passed
  without a trump stack — the advance ladder guarantees an action down to a
  cheapest-notrump escape. Every instinct call is natural, so the floor stays
  coherent when both partners land on it; strength-showing cue-bids are
  deliberately excluded until both sides of the convention can be authored.
  Floor activations are observable as `Provenance { depth: 0, fallback:
  Some(_), .. }` from `Trie::resolve` — in simulation, the most-hit auctions
  are the next nodes worth authoring properly.
- `bidding::constraint`: `they_bid(strain)` and `short_in_their_suits()`
  (takeout shape: at most three cards in each suit the opponents have bid),
  promoted from private helpers in the 2/1 books.
- `Stance::classify_with_provenance`: the same routing and logits as the
  `System` implementation, plus the resolution `Provenance` — the telemetry
  hook for counting instinct-floor activations (`depth == 0` with `fallback`
  set) and ranking which off-book auctions to author next.
- `bidding::american::bare_american`: the 2/1 pair *without* the
  instinct floor — the ablation handle; `american()` is now this pair
  with the floor attached.
- `instinct-floor` example: an A/B duplicate match (floored vs bare 2/1 on
  identical boards, swings scored double dummy and credited to the floored
  team in points and IMPs) plus floor telemetry (activation rate and the
  most-hit off-book auctions). First run: the floor is worth about +0.5
  IMPs/board against its own absence, and the telemetry's top entries —
  later-seat openings falling off the defensive book — drove the seat-fan
  fix above.
- `bidding::american::defense_to_weak_two` and `advance_double`: defense to
  the opponents' weak twos and advancing partner's takeout double, filling the
  one gap the `defend-2sx-or-3nt` example needed. The defensive book now
  answers a `(2♦/2♥/2♠)` opening with a takeout double, a natural 15–18 2NT
  overcall, and cheapest-level suit overcalls; `advance_double` answers
  `(opening) X (P)` for advancer with a penalty pass on a trump stack, a
  major-suit game jump, 3NT with a stopper, cheapest-level new suits, and a
  lebensohl-style escape to the cheapest notrump. Bid levels are derived from
  the opening, so the one advancer builder serves both one-bids and weak twos
  (it is now also registered after `(1x) X (P)`).
- `bidding::context`: `Context`, the mechanical auction context passed to
  classifiers and constraints — vulnerability (relative to the side to act),
  the raw table auction, and facts derived from it (bid strains per side,
  partner's last bid, the contract to beat, doubling state, passed-hand and
  seat facts, `min_level`). Also `context::relative(AbsoluteVulnerability,
  Seat)`, the only vulnerability conversion in the crate: drivers convert
  once per `classify` call, and systems pass the relative value through
  unchanged.
- `bidding::constraint`: a composable constraint vocabulary for authoring
  rules. A `Constraint` maps `(Hand, &Context)` to a logit contribution;
  crisp predicates return `0.0`/`-∞`. Primitives: `hcp`, `len`, `balanced`,
  `nltc_at_most`, the context-relative `support`, `stopper_in_their_suits`,
  `passed_hand`, `undisturbed`, `vulnerable`, `they_vulnerable`, `nth_seat`,
  and the `pred` escape hatch. Constraints compose with `&` (sum, AND for
  crisp), `|` (max, OR), and `!` (crisp flip) on the `Cons` wrapper.
- `bidding::rules`: `Rules`, an ordered rule list acting as a `Classifier`.
  Each `Rule` ties a call to a constraint with a weight (soft priority); a
  call's logit is the **max** of `weight + constraint` over its rules.
  `Rules::explain` reports the winning rule per call — "why did you bid
  that".
- `bidding::fallback`: guarded fallbacks generalizing the trie over
  competitive auctions. `Trie::fallback_at` attaches ordered `(Guard,
  Fallback)` entries to a node; `Trie::resolve` answers from the exact book
  first, then walks up from the deepest reachable node taking the first
  admitted fallback, reporting a `Provenance` (depth, entry index, rebase
  count). `Fallback::Rebase` rewrites the auction and re-resolves (at most
  `REBASE_LIMIT` times) — "system on over their double" is
  `FirstIs(Call::Double)` + `ReplaceNext(Call::Pass)` instead of a copied
  subtree. Stock guards: `Always`, `Undisturbed`, `FirstIs`,
  `OvercallAtMost`.
- `bidding::compose`: lazy `System` combinators. `a.vs(b)` composes a table
  where `a`'s partnership is the dealer's side, dispatching purely by
  auction-length parity; the opposing slot is also where an approximate
  opponent model goes. `a.or_else(b)` layers `a` over a fallback system,
  falling through on `None` or logits without probability mass. A blanket
  `impl System for &S` lets `(&a).vs(&a)` work without cloning.
- `bidding::book`: role-aware pair books, split three ways by the `Phase` of
  the auction. A `Constructive` book covers the strictly uncontested auctions
  (openings keyed per seat by their leading passes, and the continuations
  while the opponents only pass), a `Competitive` book covers the auctions
  where we open and they intervene (negative doubles, "system on" rebases),
  and a `Defensive` book covers the auctions where they open (overcalls,
  doubles, defense). All three wrap and deref to `Trie`; each adds a *gated*
  `System` impl that answers only for its phase. `Phase::of(&[Call])` is the
  single routing point that assumes a standard pass — forcing/strong-pass
  openings stay out of scope (use a bare `Trie`).
- `bidding::book`: `Pair` and `Stance` — a pair's authored system and its
  bound form. A `Pair` assembles the three books with a `Tag` identity (an
  open `&'static str` newtype with stock constants such as `NATURAL` and
  `STRONG_CLUB`; downstream systems mint their own) plus optional
  per-opponent-family overrides (`competitive_vs`, `defensive_vs`), because
  what the competitive and defensive books mean depends on the opponents'
  system. Binding happens once at table assembly: `pair.against(them)`
  selects the books for that opposing family and returns a `Stance`, the
  `System` that classifies by `Phase`. `against` merges a structural clone of
  the constructive trie into the bound competitive trie (classifiers stay
  `Arc`-shared), so a competitive rebase such as "system on over their
  double" lands in the uncontested core; constructive-phase queries use the
  unmerged constructive trie, so no competitive fallback can leak into
  undisturbed auctions. Seat-dependent systems (e.g. transfer openings in
  1st/2nd seat only) need no extra machinery — each seat already has its own
  subtree under its leading-pass prefix — and vulnerability-dependent
  agreements are authored with the `vulnerable()`/`they_vulnerable()`
  constraints.
- `bidding::table`: `Table`, the absolute-seat table driver. It seats two
  systems as North/South and East/West with a dealer and an
  `AbsoluteVulnerability`, rotates the seat to act, converts the
  vulnerability once per call, filters illegal calls with the contract-bridge
  law (`Auction::can_push`), and bids a deal out (`classify`, `next_call`,
  `bid_out`). `Table::of_pairs(ns, ew, dealer, vul)` binds two `Pair`s
  against each other's families and seats them. `Versus` remains the lazy,
  dealer-relative composer; `Table` is the full-board driver and deliberately
  not a `System` (the trait speaks relative vulnerability).
- `Trie::merge`: structural union for assembling a system from separately
  authored fragments (uncontested core + competitive packages). On collision
  `self` keeps its classifier and the keys are reported back; fallback lists
  concatenate with `self`'s first; `Arc`s are reused.
- `classifier`, `guard`, and `rewriter`: identity functions giving plain
  closures the higher-ranked `&Context`/`&[Call]` signature the compiler
  cannot generalize on its own.
- `bidding::american`: the first concrete, reusable system —
  `american()` builds a `Pair` (family `NATURAL`) for basic Two-over-One
  Game Forcing (re-exported as `pons::american`). It covers the uncontested openings
  (strong 15–17 / 20–21 notrumps, strong artificial 2♣, five-card majors,
  better-minor 1♣/1♦, weak twos, preempts, a lighter 3rd/4th-seat major), the
  first response to every one-level opening (the 2/1 game force, the forcing
  1NT, single/limit/Jacoby-2NT/weak-jump raises, 1♠ over 1♥, four-card majors
  up the line over a minor), the 1NT structure (Stayman and Jacoby transfers
  with their opener completions), one round of opener's rebids (after a
  one-level new suit and the forcing 1NT), negative doubles with "system on
  over their double", and a defensive book (natural overcalls, takeout
  doubles, the 15–18 1NT overcall, simple advances, and a penalty-oriented
  defense to their 1NT). The public sub-builders `openings`, `major_responses`,
  `minor_responses`, `notrump_responses`, and `defense_to_suit` return `Rules`
  for reuse and testing, and `competition()` returns the `Competitive` book.
  Authored entirely from the existing vocabulary; no new infrastructure. A
  `american` example (`cargo run --example american`) bids out random
  boards end to end with both sides playing the system, seated via
  `Table::of_pairs`.
- `bidding::american`: the system is now a complete 2/1 card rather than
  a basic slice. New in this pass, each in its own submodule:
  - **2/1 game-force continuations** through the slam-try level: opener's
    rebids after every two-level response (jump rebid, raise, six-card
    rebid, 12–14/18–19 2NT, new suits up the line), responder's rebids
    (trump-setting 3M, second-suit raises, 3NT), opener's 4NT hook once
    trump is set, and an `Undisturbed`-guarded *game backstop* so no
    uncovered game-forcing continuation dies below game.
  - **Forcing 1NT continuations**: the three-card limit raise (1NT then
    3M), 2NT invites, weak six-card runouts, preference, and opener's
    accept/decline.
  - **Jacoby 2NT rebids**: side-suit shortness at the three level, a good
    five-card second suit at the four level, 3M (18+) / 3NT (15–17) with no
    shortness, 4M minimum; responder drives with 4NT.
  - **Splinters** (double jump = four trumps, 10–13, singleton/void),
    **inverted minors**, and **weak jump shifts** (6-card suit, 2–5 HCP),
    with opener continuations.
  - **The strong 2♣ structure**: 2♦ waiting, 2♥ double negative (0–3),
    natural five-card positives and the 2NT balanced positive; opener's
    natural rebids (2NT = 22–24, 3NT = 25–27) and responder continuations.
  - **The 2NT machinery** at every strength: three-level Stayman and Jacoby
    transfers over the 2NT opening *and* the 2♣–2x–2NT rebids ("system
    on"), plus quantitative 4NT raises over 1NT, 2NT, and the 18–19 2NT
    rebid, with opener's graded accept/decline.
  - **Weak-two continuations**: Ogust 2NT (min/max × bad/good suit, 3NT =
    solid), RONF preemptive raises, forcing new suits with opener's reply.
  - **RKCB 1430** below every major-suit trump agreement (after Jacoby
    rebids, splinters, game-force trump setting, and the 2♣ major raise):
    the 1430 answers, asker continuations with a documented 0/3–1/4
    ambiguity policy, the 5NT king ask, and grand-slam decisions.
  - **Fuller competition**: cue-bid (limit-plus) raises, preemptive jump
    raises, competitive raises, negative doubles over all four openings
    (system on over their double everywhere), weak jump shifts in
    competition, support doubles/redoubles (exactly three-card support),
    and opener's forced answer to a negative double.
  - **Two-suited defense**: Michaels cue-bids and the unusual 2NT with
    longer-suit advances and game jumps, plus responsive doubles after
    partner's takeout double and their raise.

  Still left for later passes: lebensohl and reopening actions in deeper
  competitive auctions, responder's natural rebids after `1m–1M–2m`, and
  minor-suit keycard.
- `bidding::constraint`: four new public primitives — `top_honors(suit,
  range)` (count of A/K/Q for suit quality), `stopper_in(suit)`,
  `partner_suit_is(suit)` (pins partner's last bid suit, where `support`
  only grades it), and `min_level_is(level, strain)` (the legality anchor
  for rules whose call sits at a dynamic level, such as cue bids).

### Changed

- **Breaking** (within this release's 2/1 card, never published in an earlier
  version): raise meanings moved to the
  modern defaults. `1m–2m` is now the strong inverted raise (10+, forcing)
  and `1m–3m` the weak preemptive one; direct `1M–3M` limit raises promise
  four trumps, with the three-card limit raise routed through the forcing
  1NT.

- Reduced Clippy noise across bidding internals and tests: several small
  closure-coercion/context helpers are now `const fn`, builder-style
  constraint constructors gained `#[must_use]`, doc-code link formatting was
  cleaned up, and float assertions in tests were refactored to robust helper
  predicates instead of direct float equality.

- **Breaking:** `Partnership` is replaced by `Pair` + `Stance`, and book
  routing is now three-way via `Phase::of`. `Constructive` answers *only*
  strictly undisturbed auctions; competition over our openings moves out of
  the constructive trie into the new `Competitive` book (`american`'s
  negative doubles and system-on now live in its `competition()` builder).
  Assemble the three books with `Pair::new(family, constructive, competitive,
  defensive)` and bind once against the opponents' family —
  `pair.against(them)` returns the `Stance` that implements `System`; a
  `Pair` itself is authoring material, not a `System`. `american()` now
  returns the `Pair`.
- pons now requires `contract-bridge` 0.1.2 for the newly public
  `Auction::can_push`, the dry-run legality check behind `Table::next_call`.

- **Breaking:** `bidding::trie::Classifier::classify` now takes
  `(Hand, &Context)` instead of
  `(Hand, RelativeVulnerability, CommonPrefixes)`. The context carries the
  vulnerability and (optionally) the common prefixes. Closure classifiers
  change from `|hand, vul| …` to `classifier(|hand, context| …)`.
- **Breaking:** the vulnerability-indexed `Forest` (`[Trie; 4]`, with `from_fn`
  and the `Index<RelativeVulnerability>` impls) and its `SeatClasses` mask are
  removed in favor of `bidding::book`. Vulnerability conditions live in
  constraints (`vulnerable()` / `they_vulnerable()`), seats are explicit leading
  passes in the book key, and seat-dependent strength is a constraint
  (`nth_seat` / `passed_hand`). Author a pair's notes with the role-aware
  books assembled into a `Pair` (see above) instead of a bare `Trie`, which
  stays as the low-level table-model escape hatch (and the way to express
  forcing passes).
- `System for Trie` resolves through fallbacks (`Trie::resolve`) instead of
  exact lookup only, so a trie with fallbacks now answers auctions outside
  its book. `get`, `longest_prefix`, `common_prefixes`, and `suffixes` are
  unchanged. The `System` docs pin the vulnerability convention: `vul` is
  relative to the side to act, and composite systems pass it through
  unchanged.

- **Breaking:** `stats::average_ns_par`'s vulnerability parameter is now
  `contract_bridge::AbsoluteVulnerability` instead of `ddss::Vulnerability`.
  `AbsoluteVulnerability` is a new NS/EW bit set in `contract-bridge` 0.1.1 (now
  the minimum required version) that mirrors the existing `RelativeVulnerability`
  for symmetry. The four values map one-to-one — replace
  `ddss::Vulnerability::{NONE, NS, EW, ALL}` with the same constants on
  `contract_bridge::AbsoluteVulnerability`. The double-dummy solver is unchanged
  (still `ddss`).
- `bidding::instinct` learned to bid opposite our own strong notrump: it
  completes a standard transfer (Jacoby 2♦/2♥, 3♦/3♥ over 2NT, South African
  Texas 4♣/4♦) and, rather than pass a *forced* game out, bids the cheapest
  game — a six-card major, else 3NT — when a responder holds game values
  opposite a 15–17 1NT / 20–21 2NT, or when opener is forced past invitation.
  This is the one convention instinct reads, so the deep BTU / strawberry 1NT
  structures stay sound even where the book does not author every continuation.
  Authored rules and weak hands are unaffected: the floor is reached last and
  still defaults to Pass below an invitation.
- `notrump::register` split into `register_one_nt` (the 1NT-opening response
  block) and `register_two_nt_and_rebids` (the 2NT-strength and 18–19-rebid
  structures), so the strawberry variant can swap in BTU for the former while
  reusing the latter. `american()` is unchanged.
- `american()` attaches the instinct floor (see `bidding::instinct` under
  *Added*) to its competitive and defensive books as a root `Always`
  fallback, so the bound stance never falls off the book in a contested
  auction. Auctions that previously classified as `None` — and so were passed
  by drivers, including passing partner's takeout double on a worthless hand
  — now get a natural answer: their three-level preempts, jump overcalls past
  the negative-double range, and deep competitive continuations among them.
  Authored rules are unaffected: resolution reaches the root fallback last.
  The standalone `competition()` and `defensive()` builders stay floor-less.
- The `defend-2sx-or-3nt` example is now a flavor-comparison harness for the
  `(2♠) X (P)` defend-vs-declare decision. West's weak-two opening still comes
  from the real `american` system, while North's takeout double and South's
  Pass-vs-3NT advance are swept across alternative *flavors* — Shape / Support /
  Sound doubles and Defense / Balanced / Offense responses — each written as a
  crisp constraint in the `bidding::constraint` vocabulary. It reports
  per-double-flavor population stats and per-response-policy regret against a
  double-dummy oracle.
- The `defend-2sx-or-3nt` example studies a *realistic* population and plays
  realistic auctions. Deals are accepted through a four-gate funnel — West
  opens 2♠ per the system (at the table's actual vulnerability, via `Table`),
  North doubles by a swept flavor, *East's pass over the double is the
  system's own call* (deals where East would raise never reach South), and
  *South's decision is live* (the system's advance over `(2♠) X (P)` is Pass
  or 3NT, not a suit bid or an escape). Neither branch assumes the auction
  stops at South's call: the table bids both continuations out with
  `Table::bid_out_from` — West may run from the penalty pass, East/West may
  double 3NT or sacrifice — and the *final* contract is scored with
  `scoring::final_contract` / `scoring::ns_score`. New divergence telemetry
  reports how often (and where) the bid-outs left the nominal 2♠×/3NT
  contracts. The funnel is far tighter than the old gate, so
  `--max-attempts-per-deal` now defaults to 20000 (was 5000); numbers are not
  comparable with earlier runs — the live population is markedly more
  NS-favorable, and the swept response policies now beat both trivial
  baselines instead of losing to "always 3NT".
- docs.rs now documents the crate with `--all-features`
  (`[package.metadata.docs.rs]`), so the `serde` impls appear in the rendered
  docs.

### Fixed

- The defensive book's entry tables are now seat-fanned. `defense_to_suit`,
  `defense_to_weak_two`, `defense_to_notrump`, and the advances of natural
  overcalls were keyed only at the raw opening with no leading passes, so
  they answered only when the opponents opened in *first seat*; with any
  leading pass — `(P) 1♦`, our dealer passing first — the same decisions fell
  off the book (and before the instinct floor, were silently passed). Found
  by the first run of the `instinct-floor` telemetry.
- Broken intra-doc links in `bidding::american`: replaced the unresolvable
  `[`slam`]` reference (a private module) with plain backtick notation, and
  qualified `[`Pair::against`]` with its full crate path so rustdoc can resolve
  it from `competition`. The strawberry builder's links to its private
  convention modules (`stenberg`, `btu_notrump`, `rubens`, the `notrump`
  register blocks) likewise became plain backticks, and the `bidding::instinct`
  links now disambiguate the module from the `instinct()` function.
- Seat-fan coverage gaps: responses and continuations now answer after
  4th-seat openings (leading-pass fan extended to three passes), and the
  defensive book answers when their opening arrives after leading passes —
  previously `[P, 1♦]` and kin were silently off-book.

## [0.8.0] — 2026-05-24

### Changed

- **Breaking:** Replace the `dds-bridge` dependency with `ddss` (a
  performance-oriented DDS fork) and the `dds-bridge-sys` dev-dependency
  with `ddss-sys`. Most public types are structurally compatible — `Par`,
  `ParContract`, `TrickCountTable`, `TrickCountRow`, and `Vulnerability` all
  live at the same paths under `ddss::*` — so downstream callers usually
  only need to swap the crate name in imports. Two shape changes:
  - `dds_bridge::Solver::default()` → `ddss::Solver::lock()`. The new
    handle holds a reentrant lock, so its solve methods take `&self` (drop
    the `mut`) and the type is `!Send`.
  - The free `dds_bridge::solve_deals(&deals)` is now a method that takes
    a non-empty strain selector: `Solver::lock().solve_deals(&deals,
    NonEmptyStrainFlags::ALL)` reproduces the old all-strains behavior.
  `calculate_par` remains a free function with the same signature and can
  be called with or without a held `Solver` (it acquires the global ddss
  lock internally; the lock is reentrant per thread).
- **Breaking:** Auction primitives (`Call`, `Auction`, `IllegalCall`,
  `RelativeVulnerability`, and their parse errors), the entire `eval`
  module (`HandEvaluator`, `SimpleEvaluator`, `hcp`, `shortness`,
  `fifths`, `bumrap`, `ltc`, `nltc`, `zar`, `hcp_plus`, `FIFTHS`,
  `BUMRAP`, `BUMRAP_PLUS`, `NLTC`), and the entire `deck` module
  (`Deck`, `full_deal`, `FillDeals`, `fill_deals`) move into the new
  `contract-bridge` crate. Update imports such as
  `use pons::bidding::Call;` → `use contract_bridge::auction::Call;`,
  `use pons::eval::hcp;` → `use contract_bridge::eval::hcp;`, and
  `use pons::deck::full_deal;` →
  `use contract_bridge::deck::full_deal;`.
- **Breaking:** `pons` no longer re-exports bridge data types
  (`Hand`, `Strain`, `Bid`, `Seat`, etc.) — these live in the new
  `contract-bridge` crate, not `dds-bridge`. Replace
  `use dds_bridge::Hand;` with `use contract_bridge::Hand;`.
- Track `dds-bridge`'s flattening of the `solver` module to the crate
  root: `dds_bridge::solver::*` imports become `dds_bridge::*` (e.g.
  `dds_bridge::solver::Vulnerability` → `dds_bridge::Vulnerability`).
- Relocated tests that exercised only lower-crate APIs out of `pons`,
  so failures point at the crate they actually cover. `tests/eval.rs`,
  `tests/deck.rs`, `tests/proptest_roundtrip.rs`, and `tests/solver.rs`
  are removed; the auction block in `tests/bidding.rs` and the
  contract-bridge/ddss serde tests in `tests/serde.rs` are removed in
  place, leaving only `Array`/`Map`/`Logits` tests and pons stats serde
  respectively. The moved tests now live in `contract-bridge` (auction,
  deck, eval, proptest, serde) and `ddss`/`dds-bridge` (large-batch
  solver). No behavior or public-API change in pons.
- Dev-dependencies pruned: `approx` and `ddss-sys` are no longer used
  by anything in `pons` and are removed from `Cargo.toml`.

### Removed

- `pons::deck` and `pons::eval` modules (moved to `contract-bridge`).
- The crate-root re-exports `Deck`, `full_deal`, `HandEvaluator`,
  `Auction`, `Call` (moved to `contract-bridge`).
- The `generate-deals` and `notrump-tricks` examples. They no longer
  depended on anything in `pons` and now live with the crates they
  actually need: `generate-deals` in
  [`contract-bridge`](https://github.com/jdh8/contract-bridge/tree/main/examples/generate-deals)
  and `notrump-tricks` in
  [`ddss`](https://github.com/jdh8/ddss/tree/main/examples/notrump-tricks)
  (with a [parallel
  copy](https://github.com/jdh8/dds-bridge/tree/main/examples/notrump-tricks)
  in `dds-bridge`).

### Fixed

- README's `average_ns_par` doctest no longer overflows the stack on
  Windows. The fix is in `ddss` 0.1.2 (now the minimum required
  version): the batch solver's FFI packs are allocated directly on the
  heap via `Box::new_zeroed`, instead of routing through a stack
  temporary as `Box::default()` does at opt-level 0.

### Internal

- Set `[profile.dev.package."*"]` to `opt-level = 2`, so dependencies —
  most notably `ddss-sys`'s C++ DDS engine via `cc` — are optimized in
  dev builds. Pons's own Rust stays at opt-level 0 so any future
  stack-temp-class bug in this crate's own code still surfaces under
  `cargo test`. Big speedup for the `average_ns_par` doctest and
  `tests/par.rs`.

## [0.7.0] — 2026-05-20

### Changed

- **Breaking:** Bump `dds-bridge` to **0.19** and `dds-bridge-sys` to **3.0**
  (the latter is a dev-dependency used only by `tests/solver.rs`). The
  underlying DDS C++ library moves to v3.0.0 with PascalCase struct names
  and snake_case fields; `pons`'s own safe API is unaffected, but downstream
  users who pin to older versions of these dependencies should also bump
  them in lockstep. See the `dds-bridge-sys` v3.0.0 and `dds-bridge` v0.19.0
  changelogs for the rename map.

### Added
- New `defend-2sx-or-3nt` example: compares the expected NS score from
  defending 2♠× vs declaring 3NT after the auction `(2♠) X (P)`. The
  bidding system is a single `Trie` with three classifiers — West's
  weak-two opening at `[]`, North's takeout double at `[2♠]`, and South's
  natural call at `[2♠, X, P]` (which may be Pass, 3NT, or an
  out-of-scope call such as a 3-level new suit, jump in hearts, or
  Lebensohl 2NT). South's classifier is used only as an eligibility
  filter: deals are rejection-sampled so only those where West opens 2♠,
  North doubles, *and* South naturally faces a P-or-3NT decision are
  kept and double-dummy solved. Each accepted deal is scored under three
  strategies — always defend 2♠×, always declare 3NT, and a per-deal
  oracle that picks the higher of the two — giving an upper bound on
  what any policy keyed on South's hand could achieve. Scoring uses
  `dds_bridge::Contract::score`. Accepts an optional `--south` for
  hand-specific analysis (errors if the hand falls out of scope) or
  randomizes all four seats when omitted.

## [0.6.1] — 2026-04-25

### Changed
- Updated `dds-bridge` dependency to 0.18
- `full_deal` now returns `FullDeal` (was `Deal`)
- `fill_deals` now takes a pre-validated `PartialDeal`; no longer returns `Result`
- Track `dds-bridge`'s trick-count rename: `solver::TricksTable` → `solver::TrickCountTable` in `stats::HistogramTable`'s `FromIterator` impl and in the `check-zar` / `check-nltc` examples. Pure rename on the consumer side.
- The `serde` feature now also pulls in `serde_with` (optional dep).

### Internal
- Replaced the last hand-written `serde_impl` submodule (on `Deck`) with `serde_with::SerializeDisplay` / `DeserializeFromStr` derives. No change to the serialized form.
- Replaced non-const `.unwrap()` in tests and the `Auction::declarer` doctest with `?` propagation. Tests with a single fallible error type return `Result<(), E>`; tests mixing error types or unwrapping `Option` return `anyhow::Result<()>`.
- Moved inline `mod tests` blocks in `bidding.rs` and `deck.rs` into dedicated `bidding/tests.rs` and `deck/tests.rs` files. No behavior change.

## [0.6.0] — 2026-04-19

### Added
- Optional `serde` feature for serialization/deserialization support
- `Display` and `FromStr` implementations for `Deck` and bidding types
- `Classifier` promoted to a trait (was a plain `fn` in 0.5.0)
- Constructors for `Forest`
- `FusedIterator` implementation for `Trie` iterators
- `Debug` on `Trie` and iterator types
- Slicing API for `Auction`; `Index<Range<Bid>>` and bid-range indexing on `Array`
- `Logits::softmax` (replaces `to_odds`); returns `None` when all logits are `-∞`
- `fill_deals` helper
- Criterion benchmarks for shuffle, trie, and parallel solving
- proptest-based roundtrip and histogram invariant tests

### Changed
- `System::classify` now takes a slice
- `Auction::push` is panicking; confusing `force_push` removed
- `Deck` rejects duplicate cards
- `RelativeVulnerability` renamed from previous type
- Converters borrow instead of consuming
- Public fields replaced with getters
- Error types marked `#[non_exhaustive]`
- `average_ns_par` return type improved; redundant count parameter removed
- Random deal generation moved to `dds-bridge`; local `solver` module renamed to `random`
- Deterministic stats moved to `mod stats`
- MSRV pinned to 1.93
- Updated `dds-bridge` dependency to 0.16

### Fixed
- Memory leak in `Array::try_map`
- `hcp_plus` calculation

### Internal
- Added `#[inline]` to trivial getters on `Copy` types
- Aligned `HistogramRow::count` to take `self` (non-breaking: `HistogramRow: Copy`)
- Deduplicated `Map::get_mut`
- Bidding context lives with the stored classifier; shared API between systems and classifiers
- Hardened GitHub workflow; CI enforces `fmt`, `clippy`, and doc warnings
- Expanded README; documented the `map` module

## [0.5.0] — 2026-03-25

### Added
- `Array<T>` modeling `Call -> T`, with `Array`-like and full iterator API
- `Map` with iteration over keys, values, and entries; separated iteration for arrays
- `Logits` module (under `mod array`); `Logits::to_odds`
- Abstract bidding table supporting multiple calls per node
- Classifier concept (as a plain `fn`) replacing the filter-based approach
- Own `bidding::Vulnerability` type
- Absolute `bidding::Frequency` for easier filtering
- Different indices for X (double) and XX (redouble)

### Changed
- Edition updated to Rust 2024
- Magic number 38 replaced with a named constant

## [0.3.1] — 2025-05-31

### Fixed
- `Strategy` now requires `RefUnwindSafe` so `Trie` stays `UnwindSafe`

### Internal
- Inlined small functions for optimization

## [0.3.0] — 2025-05-30

### Added
- Core bridge data structures: `Card`, `Suit`, `Hand`, `Deck`, `Holding`
- `SmallSet` trait for `Holding` and `Hand`
- DDS (double-dummy solver) integration via `dds-bridge`
- Contract scoring
- Bitset operators for `Holding` and `Hand`
- Basic CLI to solve random deals
- Hand evaluation (LTC, NLTC, BUM-RAP, Zar points)
- `Auction` with `push`, `pop`, and `truncate`
- `Trie` for bidding strategies, with depth-first iteration, suffix and prefix iterators
- Statistics utilities for evaluators; histograms

[0.9.0]: https://github.com/jdh8/pons/compare/0.8.0...0.9.0
[0.8.0]: https://github.com/jdh8/pons/compare/0.7.0...0.8.0
[0.7.0]: https://github.com/jdh8/pons/compare/0.6.1...0.7.0
[0.6.1]: https://github.com/jdh8/pons/compare/0.6.0...0.6.1
[0.6.0]: https://github.com/jdh8/pons/releases/tag/0.6.0
[0.5.0]: https://github.com/jdh8/pons/releases/tag/0.5.0
[0.3.1]: https://github.com/jdh8/pons/releases/tag/0.3.1
[0.3.0]: https://github.com/jdh8/pons/releases/tag/0.3.0
