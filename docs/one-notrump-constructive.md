# Our 1NT: constructive structure

This is the canonical ledger for opening and continuing after our strong 1NT:
the shipped structure, the knobs that still expose alternatives, and the
measurements that chose them. Historical measurements retain their original
scorer and population; later refreshes are identified explicitly.

**Read first for contested auctions:**
[Our 1NT: competitive structure](one-notrump-competitive.md). This document is
about auctions in which our side can build its constructive contract.
`docs/bidding-options.md` remains the global current-default index.

## Opening shape, range, and reading

### Shipped opening

- **Range:** raw `hcp(15..=17)`. `set_one_notrump_fifths(true)` restores the
  archived, corrected `fifths(14.5..17.5)` arm; default **off**. The old
  `15.0..18.0` Fifths band was centred half a point too high.
- **Shape:** `NotrumpShape::Wide6322`, selected by `set_notrump_shape`; default
  **Wide6322**. It contains every balanced 4333/4432/5332 hand (including a
  five-card major in 5332), plus 5422 and 6322 with the long suit a minor.
  `Wide` stops at the five-card-minor 5422; `Balanced` is the classic arm.
- **Reading:** opener's majors read 2–5, minors 2–6, and points 15–18. The last
  point is the maximum semi-balanced shape upgrade. The minor widening made
  the 6322 default sound; `opening_inference_contains_the_opener` guards it.
- `set_open_one_notrump(false)` is a diagnostic only: those hands open a minor.

The shape decision was staged. The first constructive/contested study on
2026-06-17 found Wide over Balanced at +0.32 IMPs/divergent constructively and
+0.57 NV / +0.93 vulnerable contested; 6322 was neutral constructively but
+0.52/+0.64 contested over Wide. The reference-opponent refresh that shipped
the default used 204,800 boards/cell at SHA `c6a5643`:

| comparison | plain DD NV / vul | PD NV / vul | single-dummy NV / vul |
| --- | ---: | ---: | ---: |
| Wide − Balanced | +0.0087 / +0.0121 | +0.0060 / +0.0092 | +0.0122 / +0.0171 |
| Wide6322 − Wide, two seeds | +0.0034…0.0048 / +0.0048…0.0050 | +0.0025…0.0033 / +0.0035…0.0039 | +0.0052…0.0054 / +0.0063…0.0078 |

The Wide6322 confirmation seeds were `1783843252` and `1783844868`; all six
cells were positive on both seeds. Wide6322 shipped as the default on
2026-07-12. `american_wide()` and `american_classic()` were the named baselines;
the live system and web radio use `set_notrump_shape`.

### Strength gauges

Plain HCP replaced the old Fifths gate on 2026-06-23. Against the biased old
`fifths(15.0..18.0)` arm it won +0.138/+0.169 IMPs/board NV/vulnerable; against
the centre-matched `fifths(14.5..17.5)` arm it still won +0.067/+0.094 (20,000
filtered paired boards/cell). Fifths at this seam is archived, not queued for
another boundary-by-boundary trial.

`points` was also refuted and removed rather than kept as a losing knob. It is
identical to raw HCP on balanced hands and moves only the Wide6322 additions:
14-HCP semi-balanced hands upgrade into 1NT and 17-HCP hands upgrade out. The
205k-board/vulnerability A/B was plain −0.0008±0.0018 / −0.0024±0.0023 and PD
−0.0048±0.0022 / −0.0078±0.0028 NV/vulnerable. Length bonus is suit-playing
value, not notrump range value.

The once-deferred wide-minor **2NT opening** is settled separately. The opt-in
`set_two_notrump_wide` changes balanced-only to majors 2–4/minors 2–6. A
1,024,000-board/cell confirmation regressed the initial positive lean to
+0.0004…+0.0005 IMPs/board, three of four CIs crossing zero; +0.28/fired at a
0.15% fire rate. Verdict: **wash, default off, settled**. Its strength remains
`fifths(20.0..22.0)`.

## The response ladder

The main undisturbed ladder is:

| response | shipped meaning |
| --- | --- |
| Pass | weak signoff; flat 4333 eights are explicitly included |
| 2♣ | Stayman, including garbage and crawling routes; flat 4333 excluded |
| 2♦ / 2♥ | Jacoby transfer to hearts / spades |
| 2♠ | Puppet scheme: clubs or a balanced size ask |
| 2NT | transfer to diamonds |
| 3♣ | Puppet Stayman, game force |
| 3♦ | 5+/5+ majors, invitational or better |
| 3♥ / 3♠ | BWS/Polish splinter: short bid major, minor-oriented game force |
| 3NT | natural game force |
| 4♣ / 4♦ | South African Texas to hearts / spades in the current slam partition |
| 4♥ / 4♠ | direct major slam-try/signoff tier, as partitioned below |

### Stayman core

Commit `16012ce` fully authored `1NT–2♣` and its continuations. After `2M`,
`3M` invites, `4M` plays, and artificial `3OM` shows the fit with slam/choice
interest; opener signs off, bids 3NT only with a flat maximum, or cues a control.
Without a fit, 2NT/3NT/4NT retain the bare-1NT invite/game/quantitative meanings.
Smolen is present at both the 1NT and 2NT-strength tables. The structural
before/after A/B was +1.38/+2.03 IMPs/divergent NV/vulnerable on 60k boards.

This node had to be authored. The deterministic floor treats any three-level
suit response over our 1NT as forcing and therefore cannot decline an
invitational major raise. This is the boundary of the general “teach the
reading and let the floor judge” rule.

### Garbage, five-card maxima, both majors, and crawling

The 2026 Stayman factorial established that these treatments overlap rather
than add linearly:

| knob | default | structure | measured verdict |
| --- | --- | --- | --- |
| `set_garbage_stayman` | **on** | weak 2♣ escape; 0–4 may pass 2♦ with 3+ diamonds, 5–7 needs 4+ | +0.51 plain / +0.70 PD per fired, 0.17% fire; commit `8b8797f` |
| `set_stayman_5card_max` | **on** | opener jumps 3♥/3♠ with a five-card major and maximum | +3.45 plain / +3.33 PD per fired; still +1.47/+0.90 with garbage on; `8b8797f` |
| `set_stayman_both_majors` | **on** | max-only right-siding relay: 2NT, then 3♣→♥ / 3♦→♠, opener completes | garbage-on +2.18 plain / +2.29 PD per fired, +0.0035/bd plain ±0.0007; 320k/arm |
| `set_crawling_stayman` | **on** | short-diamond 4-4 majors crawl `2♣–2♦–2♥`, pass-or-correct | +1.539 plain / +2.055 PD per fired, +0.0015/+0.0021 per board; 1.6M/arm |

The first three-treatment factorial used 204,800 boards/arm over all eight
combinations. Solo plain gains summed to +0.0023/bd, but the full stack made
+0.0014: they compete for the same both-major hands. The original 2NT/3♣/3♦
15/16/17 both-major steps lost once garbage was on (−0.37/−0.91 per fired)
because responder declared and artificial calls were exposed to doubles. The
right-siding relay fixes both defects. The separate 335 garbage experiment
lost −0.495 plain / −0.631 PD per fired and stays dropped.

Crawling is the short-diamond complement to garbage Stayman. After
`1NT–2♣–2♦–2♥`, opener passes with 3+ hearts, corrects to 2♠ with spades and
short hearts, or bids 3♣ when 2-2 in the majors. The crawl rule floors hearts
only; the auction implies spades. Its doubled tail is covered by the generic
systems-on rebase.

### The curse of 4333

Commit `eb14e05` excludes a flat 4333 responder from ordinary Stayman, Puppet
3♣, and the 2NT-opening 3♣ ask: no ruffing value makes notrump the better home.
Both 2♣ and 3♣ must be gated because Puppet's higher weight otherwise reroutes
the hand. In competitive auctions, `set_competitive_4333` defaults to
`Suppress`; the details and pending vulnerability confirmation belong in
[the competitive ledger](one-notrump-competitive.md).

At the size seam, a flat 4333 **eight** passes. The original 16M-deal plain-DD
probe found pass over invite +0.638/bd ±0.043 (no ace +0.720; no ten +0.990;
neither +1.083). A 2026-07-25 SD-PD re-price softened the verdict: for the whole
balanced size-ask class, pass−invite was +0.283±0.081 NV / +0.092±0.117
vulnerable; flat-only +0.219/+0.019. Thus the shipped flat pass survives as a
marginal NV win/vulnerable wash. `set_size_ask_eight` remains an opt-in A/B
selector (`Shipped` default), not a broader default suppression.

A flat **nine** still forces game overall: invite−force −0.334/bd and pass−force
−0.678. The no-ten tail is the narrow exception (+0.169 invite−force; pure
no-ace/no-ten +0.281), too small to justify a quality gate.

### Invite/game-force seams

The natural-raise inference repair (commit `5e72c9c`) teaches `1NT–2NT` as an
invite and `1NT–3NT` as a game force. The floor then accepts with a maximum and
declines with a minimum. The follow-up `7c6e8c6`/documented shipped change made
**9+ HCP force game**, leaving a bare eight as the invitation: +0.98/+2.91
IMPs/divergent NV/vulnerable on 120k boards/cell. Selecting only “good” nines by
Fifths lost; even low-Fifths nines gained about +0.9/divergent when forced.
`set_nt_responder_game_floor` therefore defaults to **9**, undisturbed only.

`set_size_ask_accept_floor` now defaults to **16** (2026-07-25), 17 restoring
the old arm. The decisive SD-PD comparison, reported as decline−accept, was
−0.849 NV / −2.159 vulnerable for a 16-HCP opener: accepting wins. The embedded
bad control correctly rejected accept-15 at +1.110/+0.994. Boundary: decline 15,
accept 16+.

`set_correct_3nt_to_major` is **on**. The original bare-last-bid trigger lost
−0.037/bd; requiring an undisturbed auction, a known eight-card fit, and ruffing
shortness turned it into +0.0062 plain / +0.0068 PD per board (CI ±0.0005, two
seeds, 0.31% fired, about +2/fired). The paired scope lesson is important: a
negative treatment may be an over-broad trigger, not bad bridge.

Over `1NT-(X)`, `set_suppress_nt_game_force_over_double` defaults **on**:
business redouble or escape is better than a thin direct 3NT (+5.6/fired, about
0.03% of boards). This contested detail is expanded in the competitive ledger.

### Five-card and two-major invitations

`set_invitational_5card_majors` is **on**. Commit `559facd` authored the bare-eight
5-4 routes: 5♠4♥ Staymans, while 5♥4♠ transfers then uses a Muppet-style
2NT/2♠ swap. Commit `6c1d6df` filled the single-suited five-spade hole with
`1NT–2♥–2♠–2NT`; opener places 4♠/3NT/3♠/Pass by strength and fit. The doubled
artificial-2♦ systems-on rebase was load-bearing. The initial package was
+0.0020 plain / +0.0007 PD per board on 1.28M boards; the single-spade addition
on 4.096M/arm was +0.0006/+0.0021 plain and −0.0002/+0.0007 PD NV/vulnerable.

Commit `7ae1ecb` makes `1NT–3♦` 5+/5+ majors, invitational or better, gated on
`points(8..)`. Over a minimum opener the auction stops in the better major;
over a maximum it reaches 4M with a fit or 3NT when 2-2. The 200k-board
structural A/B was +2.17/+2.80 IMPs/divergent NV/vulnerable (~0.05% divergence).

### Puppet and minor transfers

Commit `93352b9` shipped the Puppet/minor scheme:

- `3♣` Puppet: balanced game force with a three-card major; opener shows 3M or
  denies with 3♦, after which responder bids the shorter major Smolen-style.
- `2NT` transfers to diamonds: 6+ diamonds or 5♦4♣; opener supports with 3♦ or
  bids 3♣ pass-or-correct.
- `2♠` shows clubs or a balanced bare-eight invite; opener's 2NT/3♣ step is the
  min/max size answer and remains pass-or-correct for the weak club hand.

Both minor transfers **place games and never ask for slam** — the game boundary
is a hardcoded `8` at every site and opener's splinter answer picks `3NT` or
`5m`, total. The direct quantitative `4NT` is *not* an escape for the strong
long-minor hand: it is weight 120 against the transfers' 130 and the classes
overlap, so `A32.32.AKQ876.K2` bids `2NT`, not `4NT`. The counter-tables
inherited the same gap one level up — the cross-lane census and its queue are in
[minor-transfer-slam.md](minor-transfer-slam.md).

The original scheme beat the natural baseline +0.76/+1.15
IMPs/divergent NV/vulnerable (+0.0072/+0.0109 per board, 60k). The later isolated
Puppet-vs-European comparison stayed positive across four DD cells and the
2026-07-25 SD-PD confirmation was +0.0006±0.0005 / +0.0010±0.0006 per board.
Puppet remains the default; European is an opponent model/opt-in alternative.

The carve is deliberate: Puppet's higher weight claims the balanced 4-3
overlap, while shapely hands use minor transfers. A Puppet-major splinter is
dead under `balanced()` and should not be reintroduced. GF 5-4 majors are carved
out of Jacoby transfers so they can reach Stayman/Smolen. Artificial three-level
continuations must be suppressed from the natural suit reader, and weak retreat
tails need explicit pass-out nodes because the floor otherwise forces game.

### Direct 3M splinters

`set_nt_splinter` shipped **on** at commit `da96c04` (2026-07-28). Ours is the
BWS/Polish form: void or low singleton in the bid major, 2–3 in the other major,
exactly four diamonds, five or six clubs, and 9+ HCP. `set_nt_splinter_floor`
defaults to **9**. Opener's answer must be authored: the floor ignored even the
exact alert-decoded shape, producing only 9 divergences from 217 firings in the
first 600k-board probe.

With `nt_splinter_answer`, 5M boards/vulnerability won every cell: +0.56/+0.67
plain/PD per fired NV and +0.69/+0.81 vulnerable; 0.040% fire, 176 divergent.
The eight-count extension lost −52/−773 IMPs NV and +229/−519 vulnerable
plain/PD across 711 extra firings, so nine stands. BBA/BEN's GIB form instead
has exactly four cards in the other major; see
[the source comparison](ai-bidder/bba-1nt-splinter.md).

## Slam structure

### Jacoby-transfer game forces

The transfer GF package is **on** on both sides:

| knob | commit | structure | verdict |
| --- | --- | --- | --- |
| `set_transfer_gf_majors` | `3b76a9e` | after the spade transfer: 3♥=5-5 slam try, 3m=5♠4m GF, 4-level splinters, 4NT single-suit quantitative | +1.70 plain / +1.90 PD per fired, two seeds |
| `set_transfer_gf_hearts` | `3544b63` | heart mirror; 5-5 stays on the spade route, 3♠ is the cheap spade splinter | +1.83/+2.08 per fired, two seeds |

`set_minor_min_to_3nt` remains **off**: showing the minor beat lumping minimum
game forces into 3NT. Natural rebids must not restate the already-shown transfer
suit; doing so makes an otherwise natural call artificial to the invariant.
The minor answer deliberately has no RKCB: opener blasting slam opposite a
possible bare minimum was a doubled five-level loss.

### Transfer and Texas slam drives

The 2026-07-01 isolate-opening study localized 59% of the constructive loss to
Jacoby transfers, usually our 3NT against BBA's cold major slam. Two fixes
shipped:

- `set_transfer_slam_try`, commit `921b2c9`, default **on**: responder with a
  five-card major and 16+ HCP used the other major as an artificial slam try;
  opener signed off minimum or asked 1430 with a maximum. It measured
  +0.0012/bd plain and PD, +1.42/fired (275/320k). Since the later GF-major
  structure owns this slot it was **inert, 0/320k×2 fired on 2026-07-16**; it
  remains the fallback when the GF package is disabled.
- `set_texas_slam_drive`, commit `c0c459c`, default **on**: direct 4M is capped
  at the opener-decides cusp; stronger responders transfer at the four level
  and drive their own RKCB. The shipping A/B was +0.0024/bd plain and PD,
  +5.87/fired (131/320k). The later A7 refresh was +5.04/+5.85 plain and
  +5.17/+6.03 PD per fired NV/vulnerable, with positive single-dummy brackets.

In both cases the strong responder must be allowed to ask; asking a minimum
opener whether it likes slam strands combined 32+ HCP hands in game.

### South African Texas

Commit `d9ad0b1` authored the South African Texas family. Its original
partition used `4♣/4♦` as 9–14 to-play transfers to hearts/spades and direct
`4♥/4♠` as 15–18 non-forcing slam tries, with a maximum opener asking 1430.
The 10M-deal structural A/B was +2.53/+3.78 IMPs/divergent NV/vulnerable.
`set_texas_game_floor` is currently 14; `set_texas_slam_drive` provides the
later strong-responder partition above.

Standard Texas versus South African labels was a null on 2026-07-20 (204,800
boards/arm/vulnerability, seed `1784525480`): plain +0.0001/−0.0001 and PD
+0.0002/−0.0000 per board, every CI crossing zero. SAT stays as incumbent and
the temporary `set_standard_texas` knob was deleted. Auction divergence (0.85%)
was about 20× score divergence (0.04%): size this comparison by scored outcomes.

A fit-adjusted evaluator at the SAT game/slam seam was also null (seed
`1782732665`, 300k boards, both vulnerabilities). For the balanced opener,
`points == hcp` and had zero divergence; a trump-support bonus leaned −0.003
with CI ±0.037. Responder `fit_value` led at +0.0042/+0.0050 per board but its
±0.0046/±0.0055 CIs included zero. Raw HCP gates stand.

### Stayman slam work

The isolate-opening diagnosis at `e54493f` (2026-07-02, seed `1782939501`)
found uncontested Stayman at −0.239 plain / −0.242 PD over 19,260 boards. In the
top 500 losses, about 69% was missed slam: major game versus slam 36%, 3NT
versus slam 20%, and a passed-out control cue 20%.

Two repairs shipped:

- `set_stayman_cue_continuation`, default **on**: after the `3OM` try and
  opener's cue, responder signs off 4M or asks 1430. It gained +0.0193/bd plain
  / +0.0216 PD, +8.73/+9.74 per fired (850/384k, CI ±0.0015).
- `set_stayman_minor_slam_try`, default **on** (from `7745e6b`): natural 3m
  after a Stayman answer shows 5+ minor, 14+ HCP and no major fit; opener raises
  with four and a maximum or bids 3NT, with 1430 only after both hands have
  shown non-minimums. It gained +3.29/+4.02 per fired NV/vulnerable, 151 firings
  in 1.5M/vulnerability, zero losses; PD was identical.

The remaining constructive questions are major-fit RKCB and a post-Smolen slam
continuation; they are kept as transient work, not silently declared solved by
the generic floor.

## Evaluator verdicts at the 1NT seams

### Raw HCP versus analytic counts

`examples/probe-nt-invite-eval` rank-calibrated each evaluator to HCP's exact
pass/invite/force frequencies. On 50k deals per class at both vulnerable, raw
HCP won in both the Stayman and no-four-major classes: controls −0.43…−0.75
IMPs/bd, CCCC −0.06…−0.23, BUM-RAP −0.07…−0.14, Fifths −0.01…−0.04, and points
approximately zero. On balanced no-major hands, points equals HCP exactly.
This is a raw-25 decision opposite a known balanced 15–17, not a suit-fit
evaluation problem.

Sub-integer Fifths ordering did not rescue either invite or acceptance. Three
frequency-matched arms (`fifInv`, `fifAcc`, `fifInv2s`) were null in every cell,
|mean|≤0.013. On 2026-07-28, eval-net v2/v3/v4 likewise closed the no-major
class: eight cells all crossed zero, largest |mean| 0.0088/bd. Conditioning on
the class makes the newer nets' extra shape context constant.

### Fifths reference

Thomas Andrews's Fifths scale is A=4, K=2.8, Q=1.8, J=1, T=0.4 on a 40-point
deck, with a simulated 3NT threshold around 24.2–24.4. The crate's `fifths()`
is a blend with an HCP/BUM-RAP companion rather than the pure scale. Andrews
warned that fuzzy invitation boundaries after 1NT may not improve practical
decisions; the opening and responder measurements above confirm that here.

### Fit value and the net gate

`fit_value(hand, major) = point_count + trumps past eight` is the right tool at
the known-fit Stayman invite/game seam. Commit `4c07362` shipped it on
`1NT–2♣–2M`: +1.874 plain / +1.523 PD per fired (+0.0016/+0.0013 per board,
111/128k, both CIs excluding zero). It also underlies the both-major relay,
where the known second fit adds value. It does not generalize to already-forced
Puppet/Smolen or to a 5-4 invitation where opener denied the major.

`set_stayman_net_force` remains **off**. The rank-calibrated screen was the
first evaluator to beat HCP in the Stayman class (+0.030/+0.044 per board,
+0.048/+0.069 opposite exactly-15 openers), but the live 200k/vulnerability A/B
lost: NV −0.022 plain/+0.003 PD; vulnerable −0.021 plain/−0.027 PD. At the fit
seam it displaced the stronger incumbent `fit_value`; at the no-fit seam it
promoted 3NTs that won plain DD and lost under perfect defense.

### A6 engine audit (2026-07-13)

The A6 self-play audit used 1M boards/cell over both vulnerabilities and dual
plain/PD scoring:

- `fuzzy_fifths` flipped **off**: Fifths NT gauging lost plain
  −0.0118/−0.0177 and PD −0.0110/−0.0165 NV/vulnerable. Raw HCP is the default.
- The then-named fuzzy-points arm stayed on after plain +0.106/+0.116 and
  single-dummy +0.1639/+0.1939; its PD loss was a DD lead artifact. The current
  option model is recorded in `docs/bidding-options.md`; this is the historical
  A6 verdict, not a promise that the old setter still exists.
- `inference_aware`, alert reading, and settle-floor all won and stayed on;
  alert was +0.017/+0.023 plain and settle +0.047/+0.092, with PD at least plain.
- `nt_invite_inference` was inert (0 divergent at 1M×2) because Puppet routes
  the invite through 2♠, away from natural 2NT.
- `rubens_transfer_reading` was a wash (about 0.01% fire) and stayed structural.

## Comparison with BBA

The early 2026-06-22 `bba-match --filter-1nt` umbrella (10k boards/cell) put the
whole 1NT territory at −1.39/−2.20 IMPs/board NV/vulnerable, but it mixed our
opening, continuations, and defense. The first headline that constructive
gadgets were at parity was therefore not causal.

`--isolate-opening` fixed the attribution by holding the defender constant. On
320k boards/cell (2026-06-28), our whole 1NT auction trailed BBA with BBA
defending by −0.442 plain / −0.525 PD, and with pons defending by −0.123 plain /
−0.572 PD; every CI excluded zero. The loss concentrated in Stayman, Jacoby
transfers, and direct 3NT, while Puppet/minor 3♣ was mildly positive. The
2026-07-01 refresh after intervening fixes narrowed the plain result to
−0.290 (BBA defense) / −0.064 (pons defense), with PD −0.427/−0.541, and
localized the Jacoby slam hole described above.

Continuation buckets are therefore the correct unit: isolate the opening,
hold defense fixed, bucket `1NT P <response>`, and inspect the worst boards.
The contested Multi-Landy diagnosis and the completed `2♦`-Multi counter work
now live only in [the competitive ledger](one-notrump-competitive.md); the
older "pending A/B" note is superseded.
