//! What the partnership has agreed to play — the value a book is built from
//!
//! The knob layer's single source. Every `set_*` knob is one cell of this
//! value; a build captures it **once**
//! ([`Agreements::current`][crate::bidding::agreements::Agreements::current]) and threads
//! `&Agreements` to everything that reads a knob, instead of each reader
//! consulting the thread on its own.
//!
//! That join is the point. Four readers must agree about what we play —
//! [`american_book`][crate::bidding::american::american_book],
//! [`instinct`][crate::bidding::instinct()],
//! [`ConventionCard::capture`][crate::bidding::features::ConventionCard::capture]
//! and [`reading_profile`][crate::bidding::inference] — and until this value
//! existed nothing but call-site discipline made them. Two defects were paid
//! for by that gap: the forced rail froze into a process-wide `LazyLock` built
//! by whichever pair came first, and a card disclosed rows the rules were not
//! playing. Both become unrepresentable once one capture feeds all four.
//!
//! # Layout
//!
//! One field per area of the system — `competition`, `defense`, `notrump`,
//! `opening`, `response`, `rebid`, `game_force`, `instinct` — plus `decision`.
//! No cell appears twice; that is the "one cell, one home" invariant, and
//! `no_knob_lives_in_two_homes` enforces it by scanning this source.
//!
//! `decision` (crate-private until the cells are gone) holds the cells read
//! **per decision**, at classify time, rather than while the books are built.
//! It is split out only because it is the snapshot a
//! [`Stance`][crate::bidding::Stance] pins at
//! [`Pair::against`][crate::bidding::Pair::against], so a stance decides
//! identically on any thread; the eight build-time areas are baked into the
//! rules a build returns and need no such pin.  A cell read at *both* times
//! (there are 24) lives in `decision` and is read from there at build time too.

use super::american::{
    Competitive4333, DoubleShape, DoubleStyle, FreeBidStyle, LebensohlStyle, NegativeDoubleShape,
    NotrumpShape, SizeAskEight, TakeoutSupport, TwoOverOneGate, WeakTwoEval,
};
use super::context::DecisionProfile;

/// The competitive book's build-time knobs
///
/// Each field is one cell, named for the getter it replaces; *derived* readings
/// (`free_bids_engaged`, the natural-floor pair) stay functions of the module
/// that owns them rather than becoming fields, so the "one cell, one home"
/// invariant survives the move.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CompetitionKnobs {
    // --- competition/cue_raise.rs
    /// Answer partner's cue-raise of their major overcall
    ///
    /// **Default on** (`--no-ns-cue-raise-answer` in `bba-gen` for the off
    /// arm).  Without it the cue-raise (`1M (ovc) cue -`) falls through to the
    /// keyless floor, which cannot act on a bid whose *named* suit (the cue)
    /// differs from its *shown* suit (the major), so opener passes and the
    /// cuebid is left in as the contract.
    pub cue_raise_answer: bool,
    /// Answer partner's cue-raise of their minor overcall
    ///
    /// The minor twin of [`cue_raise_answer`][Self::cue_raise_answer]
    /// (`1m (ovc) cue -`), kept as its own cell so the A/B can isolate the
    /// minor contribution over the already-shipped major answer.  **Default
    /// on** (`--no-ns-cue-minor-raise-answer` in `bba-gen` for the off arm).
    pub cue_minor_raise_answer: bool,
    /// Bid (not merely recognize) the delayed cue — 2NT relay, then their suit
    ///
    /// Larry Cohen's fast-denies / slow-shows, adapted to our Transfer
    /// Lebensohl: the *direct* cue of their suit denies a stopper, while a
    /// *delayed* cue (relay through `2NT`, then their suit) is Stayman *with* a
    /// stopper.  It also denies a 5-card unbid major (Smolen / Leaping Michaels
    /// handle those).  Only the single-unbid-major contexts — over `(2♥)` and
    /// `(2♠)` — are affected.  **Off by default**, gated for A/B measurement.
    pub delayed_cue: bool,
    // --- competition/free_bids.rs
    /// Author the free bids directly, rather than only as a negative-double outlet
    ///
    /// Responder's natural free bids over an overcall: 1-level new suit 5+ &
    /// 6+, 2-level non-jump 5+ & 10+, `1NT` 6–10 / `2NT` 11–12 with a stopper.
    /// **Default off** as a *direct* toggle (`--ns-free-bids` in `bba-gen` for
    /// the on arm), but the shipped [`NegativeDoubleShape::Modern`] shape
    /// implies them (with opener's forcing answers) — the default system plays
    /// free bids.
    pub free_bids: bool,
    /// Minimum points/HCP for the 1-level free bids
    ///
    /// Gates the 1-level free *suit* bids (new-suit 5+, plus the Sputnik
    /// natural 4+ majors).  **Default 6** — the shipped floor
    /// (`--ns-free-bid-floor` in `bba-gen`).  The vul-PD leak of the whole
    /// free-bid family lives here; raising it trims that leak — sweep to 8+ and
    /// re-measure.  The free `1NT` has its own floor
    /// ([`free_1nt_floor`][Self::free_1nt_floor]): a forcing suit bid finds a
    /// fit cheaply and is safe light, a limited non-forcing `1NT` is not.
    pub free_bid_floor: u8,
    /// Minimum HCP for the free `1NT`, decoupled from the suit floor
    ///
    /// The free `1NT` of `1X (1Y) 1NT` is a limited, non-forcing commitment to
    /// notrump values; raising this trims light `1NT`s without touching the
    /// forcing 1-level suit bids.  **Default 6**
    /// (`--ns-free-1nt-floor` in `bba-gen`) — byte-identical to the historical
    /// value shared with [`free_bid_floor`][Self::free_bid_floor].
    pub free_1nt_floor: u8,
    /// Require a quality suit for a free bid
    ///
    /// On, a *vulnerable* 1-level free bid demands two of the top three honors
    /// in the bid suit and the free `1NT` is not authored vulnerable;
    /// non-vulnerable rules and the 2-level/`2NT` free bids are unchanged.  The
    /// P3b′ floor sweep named the family's vulnerable leak as plain-DD-visible
    /// and strength-independent — a suit-quality gate, not a floor.  **Default
    /// off** while the A/B runs (`--ns-free-bid-quality` in `bba-gen` for the
    /// on arm).
    pub free_bid_quality: bool,
    /// Whether a free bid is forcing, one-round forcing, or a transfer
    ///
    /// The **2-level** free-bid style — [`FreeBidStyle::Forcing`] (the shipped
    /// default), classic negative free bids, or Cachalot-style transfers.  The
    /// 1-level free bids stay forcing in every style.  `--ns-free-bid-style` in
    /// `bba-gen` for the other arms.
    pub free_bid_style: FreeBidStyle,
    // --- competition/high_overcall.rs
    /// Author responder's structure over their jump / 3-level overcalls
    ///
    /// Covers `2NT < bid ≤ 3♠`, where responder has one round and no room: the
    /// shipped direct-seat package stops at `2♠` (one exact table per overcall
    /// through there, plus their `1NT`) and everything higher falls to the
    /// floor.  **Default off** while the A/B runs (`--ns-high-overcall` in
    /// `bba-gen` for the on arm).
    pub high_overcall_responses: bool,
    // --- competition/lebensohl.rs
    /// Require a stopper for the direct `3NT` over their overcall
    ///
    /// **Default `true`** (status quo): responder's *direct* `3NT` over the
    /// overcall needs its own stopper in their suit.  With `false` a
    /// game-values hand bids `3NT` without a guaranteed stopper, leaning on
    /// opener's `1NT` for the stop — the A/B knob for "does direct `3NT` really
    /// need a stopper, or does `X` show it?".
    pub direct_3nt_stopper: bool,
    /// `(hcp_floor, points_floor)` on responder's weak natural 2-level escape
    ///
    /// The direct natural `2♦`/`2♥`/`2♠` over the overcall is the same weak
    /// 5-card-suit hand as the relay-then-correct sign-off (`2NT`→`3♣`→`3M`),
    /// one level lower — but with no floor it carries no strength gate and
    /// opener cannot raise it.  A non-zero floor makes the two symmetric: it
    /// gates the natural escape (an HCP floor *or* a total-points floor — being
    /// a level lower than the relay, the 2X floor can be lower or
    /// playing-strength oriented) and registers opener's sign-off raise over a
    /// natural *major* escape, so a maximum with a fit stretches to game.
    /// `(hcp, 0)` is an HCP floor, `(0, points)` a points floor, `(0, 0)` no
    /// floor.
    ///
    /// **Default `(5, 0)`** — a 5-HCP floor, with opener's game-raise.  A floor
    /// of any kind beats none by `+0.012`/`+0.016` IMPs/board (none/both), and
    /// — once `(2♣)` went systems-on, leaving the natural escape all *majors*
    /// (every one game-raisable, no raise-less minor) — `5` HCP beats the
    /// relay's `6` by `+2.5`/`+2.3` IMPs/divergent (none/both), all-positive.
    /// `4` HCP is too loose: the raises turn negative (overbidding).  One lower
    /// than the relay's `6`, matching the 2X sitting one level lower.
    pub natural_floor: (u8, u8),
    /// Which Lebensohl package the competitive book carries
    ///
    /// Section 5 of the competitive book; default [`LebensohlStyle::Transfer`].
    pub lebensohl_style: LebensohlStyle,
    /// Read a `(2♦)` overcall of our `1NT` as a Multi
    ///
    /// Responder treats their `2♦` as an unknown single-suited major and
    /// answers with the Multi counter-defense — double = values, everything
    /// else natural, distilled from BBA's Multi-Landy counter
    /// (`docs/ai-bidder/bba-multi-2d.md`) — instead of the natural-diamond
    /// Transfer/Lebensohl package.  It overrides only the `(2♦)` responder
    /// node; the shared `2NT` relay machinery is unchanged.
    ///
    /// **Off by default**, opt-in pending the A/B; faithful for the A/B against
    /// BBA, whose `2♦` over our `1NT` is always a Multi.
    pub defense_2d_multi: bool,
    // --- competition/negative_double.rs
    /// Which negative-double school the minor openings play
    ///
    /// The **minor** openings only — the major-opening double (4+ in the other
    /// major, 8+) is common to all three schools.
    ///
    /// **Default [`NegativeDoubleShape::Modern`]** — shipped default-on
    /// 2026-07-10 together with the forcing free-bid answers: plain +0.0213 NV
    /// / +0.0074 vul (CI>0), sd arbiter +0.42/+0.29 per divergent board (CI>0,
    /// sd>plain, disclosure-corrected); the vul-PD −0.026 is the
    /// perfect-defense doubling artifact on thin vul games.  Pass
    /// `--ns-negative-double-shape both-majors` in `bba-gen` for the old rule.
    pub negative_double_shape: NegativeDoubleShape,
    /// Author the Cachalot answers when the auction is contested
    ///
    /// Opener's raise of a Cachalot `X` transfer when LHO competes over it
    /// ([`NegativeDoubleShape::Cachalot`] only).  **Default on**; off restores
    /// the floored continuation for the A/B (`--no-ns-cachalot-contested-x` in
    /// `bba-gen`).
    pub cachalot_contested_x: bool,
    // --- competition/our_preempts.rs
    /// Author continuations when they contest our weak two
    ///
    /// Responder over their takeout double (business `XX`, systems-on Ogust)
    /// and over their overcall (Ogust-when-legal, values `X`, preemptive
    /// raises).  **Default off** while the A/B runs (`--ns-weak-two-comp` in
    /// `bba-gen` for the on arm).
    pub weak_two_competition: bool,
    /// Author continuations when they contest our strong two
    ///
    /// Our contested `2♣`: systems-on over their double, and over their
    /// overcall a natural-GF / values-`X` / waiting-pass structure backed by
    /// opener's forced reopening.  Without it responder's `X` falls to the
    /// floor's *takeout* reading — with a 22+ opener behind it.
    ///
    /// **Default on** — measured vs BBA 2/1 (204.8k boards/arm/vul): plain DD
    /// +1.86/+2.79 IMPs/fired NV/vul, perfect-defense +2.00/+2.93; all four
    /// cells' CIs exclude 0 (~0.05% fired).  `--no-ns-strong-two-comp` in
    /// `bba-gen` for the off arm.
    pub strong_two_competition: bool,
    // --- competition/over_our_*.rs
    /// Author continuations when they contest our `2NT` diamond transfer
    ///
    /// Opener's replies after they double or overcall `1NT - 2NT` (6+♦, or
    /// 5♦-4♣).  Only the Puppet scheme (the default) plays `2NT` as the diamond
    /// transfer, so the block no-ops under European, where `2NT` is the
    /// balanced size-ask.
    ///
    /// Their `(X)` is lead-directing diamonds; the double frees `Pass` to be
    /// the catch-all "no fit" call, which lets opener's `3♣` shed its
    /// uncontested relay-denies-a-fit meaning and become **natural** (4+♣,
    /// finding responder's 5♦-4♣ club fit): `3♦` = accept with a diamond fit
    /// (3+♦), `3♣` = no fit but 4+♣, `XX` = maximum values without a fit
    /// (penalty-oriented), `Pass` = minimum catch-all.  After a fit-showing
    /// `3♦`/`3♣` responder's rebids match the uncontested tree (strip the `X`
    /// to a Pass); after `Pass`/`XX` (no fit) responder always holds 5+♦ and
    /// signs off in `3♦`.  An overcall is handled naturally: `3♣` leaves room
    /// to complete `3♦` with a fit (else `X` = penalty, Pass = minimum); a
    /// higher overcall keeps `3NT` (max + stopper) / `X` (their suit) / Pass.
    ///
    /// **On by default** (off-switch `--no-ns-comp-over-diamond-transfer` in
    /// `bba-gen`): a paired A/B vs BBA over 1 000 000 `--filter-1nt` boards
    /// (410 fired, 0.04 %) measured a plain-DD **wash** (+0.24 IMPs/board it
    /// fires on, CI straddling 0) and a clear perfect-defense gain (+3.40 PD).
    /// Unlike the `2♠` minor (which won on *both* scorers), the honest-DD
    /// signal is a wash — but it never *loses* on plain DD, and the PD gain is
    /// real value the day the opponents punish the floor's
    /// `X`-then-pull-to-`3NT` overreach, so it ships on.
    pub competition_over_diamond_transfer: bool,
    /// Author continuations when they contest our Jacoby transfer
    ///
    /// Over a `(X)` opener completes the transfer with three-card support, jump
    /// super-accepts with four and a maximum, passes with a doubleton
    /// (declining — responder's `XX` then re-asks, forcing), or redoubles with
    /// the doubled transfer suit as its own.  Over an overcall opener
    /// super-accepts the major with a fit, doubles for cards, else passes.
    ///
    /// **Off by default** (`--ns-comp-over-transfer` in `bba-gen` for the on
    /// arm): unlike the contested `2♣` Stayman, which won +3.5 IMPs/fired, a
    /// paired A/B vs BBA over 640 000 boards found these continuations a DD
    /// **loss** (plain −0.94, PD −0.33 IMPs/board it fires on) — the
    /// super-accept and forcing re-ask drive us into failing contracts the
    /// floor's lower bids avoid.
    pub competition_over_transfer: bool,
    /// Author continuations when they contest our `2♠` minor transfer
    ///
    /// Opener's replies after they double or overcall the two-way `2♠`
    /// (clubs-or-balanced-invite) response.  Only the Puppet `2♠` — a club
    /// one-suiter *or* the balanced invite that asks opener's size — has a
    /// min/max answer to protect, so the block no-ops under the European
    /// pure-transfer scheme.
    ///
    /// Their `(X)` of `2♠` is lead-directing spades, so opener re-encodes its
    /// size-ask answer *and* a spade stopper across four calls: `2NT` = minimum
    /// **with** a stopper, `3♣` = maximum **with** one, `Pass` = minimum **no**
    /// stopper, `XX` = maximum **no** stopper.  After a stopper-showing bid
    /// responder's rebids match the uncontested tree (strip the `X` to a Pass);
    /// after a no-stopper reply responder signs off in `3♣` with clubs.  A
    /// `(2NT)`/`(3♣)` overcall (which steals the size-ask steps) keeps the
    /// signal alive — `3NT` = maximum + stopper, `X` = maximum no stopper, Pass
    /// = minimum; any higher overcall is systems-off (a `X` showing their suit,
    /// else Pass).
    ///
    /// **On by default** (off-switch `--no-ns-comp-over-minor-transfer`).  Like
    /// the contested `2♣` Stayman this is a **constructive** win: a paired A/B
    /// vs BBA over 640 000 boards measured **+4.80 IMPs/board it fires on** on
    /// plain double-dummy (+5.63 under perfect-defense — *higher*, so it is a
    /// sound contract-finding gain, not a doubling artifact), CI excluding 0.
    /// Rare (it fired on 0.03 %): BBA seldom contests our `2♠`.
    pub competition_over_minor_transfer: bool,
    /// Author continuations when they contest our Stayman
    ///
    /// Opener's replies after they double or overcall `1NT - 2♣`.  Over a `(X)`
    /// (lead-directing clubs) opener answers in the *pass-denies-stopper* coded
    /// scheme: a major or `2♦` promises a club stopper, Pass denies one, `XX`
    /// is business clubs; responder's `XX` after opener's pass re-asks Stayman
    /// (forcing).  Over a `(2♦)`/`(2♥)`/`(2♠)` overcall opener bids a 4-card
    /// major naturally if it outranks their suit, doubles for cards, else
    /// passes.
    ///
    /// **On by default** (off-switch `--no-ns-comp-over-stayman`); recorded in
    /// the Jacoby-transfer A/B as winning +3.5 IMPs/fired.
    pub competition_over_stayman: bool,
    // --- competition/over_their_double.rs
    /// Jordan/Truscott `2NT` over their takeout double
    ///
    /// The whole of responder's structure over their takeout double of our
    /// 1-suit opening: Jordan/Truscott `2NT`, the value redouble, the
    /// preemptive jump-raise flip, and weak non-forcing 2-level suits — with
    /// the shipped systems-on rebase surviving below it as the catch-all for
    /// every deeper continuation.
    ///
    /// **Default on** — the campaign's largest per-board win vs BBA 2/1 (204.8k
    /// boards/arm/vul): plain DD +0.0041/+0.0067 IMPs/board NV/vul,
    /// perfect-defense +0.0049/+0.0065; all four cells' CIs exclude 0
    /// (+0.5…+0.8 IMPs/fired, ~0.8% fired).  `--no-ns-jordan-truscott` in
    /// `bba-gen` for the off arm.
    pub jordan_truscott: bool,
    /// Author answers to partner's redouble
    ///
    /// Opener's rebid over the value redouble, `1x (X) XX -`; a no-op unless
    /// [`jordan_truscott`][Self::jordan_truscott] is on, since that authors the
    /// redouble itself.  The authored node is pass-only — a long-suit minimum
    /// sits for the redoubled make, and a `2M` escape rung measured −11
    /// IMPs/fired before deletion.
    ///
    /// **Default on** (fix-vs-shipped, 1M boards/vul, 24.pdd 16.3M–18.3M: plain
    /// DD +0.0056 ± 0.0005 NV / +0.0078 ± 0.0007 vul, PD +0.0058/+0.0080,
    /// ≈ +11..+14 IMPs per divergent board).  Off, the systems-on rebase strips
    /// both the double and the redouble, so opener replays onto the uncontested
    /// tree with responder's shown 10+ unseen, and the floor blasts stopperless
    /// `3NT`s / thin games off shaped minimums — the point-count remnant's
    /// single worst per-board family (−16..−17 IMPs/board vulnerable).
    pub redouble_answer: bool,
    /// Rebase to systems-on when they double our splinter
    ///
    /// A splinter (`1M - double-jump`) is game-forcing, but the double reroutes
    /// opener's rebid to the competitive book, where — unauthored — it fell to
    /// the floor and *passed*, leaving the game force doubled at the four level
    /// (the anchor's Constructive/book/round-1 bucket #4 tail: our monster
    /// opener passing a doubled `4♣` splinter while the field bids `7♠`).  This
    /// rebases the double back onto the undisturbed splinter continuation (`4M`
    /// sign-off floor, RKCB with slam values).
    ///
    /// **Default on** — measured vs BBA 2/1 (204.8k bd/arm/vul, SEED_BASE
    /// 1783439089): plain DD +0.0059/+0.0079 IMPs/board NV/vul, perfect-defense
    /// +0.0059/+0.0079, all four CIs exclude 0, +15.4/+17.6 IMPs/fired (0.04%
    /// fired).  Off-switch `--no-ns-splinter-doubled`.
    pub splinter_doubled: bool,
    // --- competition/penalty_double.rs
    /// Whether a double of their overcall is takeout, optional, or penalty
    ///
    /// Default [`DoubleStyle::Optional`] (2-3 in their suit, 8+); the A/B
    /// verdict that ranks **Optional > Penalty > Takeout** is recorded on
    /// [`DoubleStyle`] itself.
    pub double_style: DoubleStyle,
    /// Opener may leave in responder's penalty double
    ///
    /// Opener sits for `1NT (2X) X -`, defending the doubled natural overcall,
    /// rather than letting the floor read `… X -` as a takeout advance and pull
    /// it — responder's penalty double promised the trumps.  Off restores the
    /// bare floor.
    ///
    /// **On by default**, but a no-op unless the active [`DoubleStyle`] is
    /// `Penalty`/`PenaltyLight`: this is the A/B knob for the "opener pulls
    /// responder's penalty double" leak, the book dual of the penalty latch.
    pub penalty_double_leave_in: bool,
    /// `(min_len, max_len, hcp_floor)` override on responder's penalty double
    ///
    /// An explicit `(min_len, max_len, min_hcp)` in their suit, superseding
    /// [`DoubleStyle`] so an A/B can sweep the penalty/takeout boundary as a
    /// continuum instead of the four discrete styles.  `None` (the default)
    /// uses the named [`DoubleStyle`].
    pub double_override: Option<(usize, usize, u8)>,
    /// `(min_club_len, min_club_hcp, convert_over_major)` on the stolen-Stayman pass
    ///
    /// After `1NT (2♣) X -` — where the systems-on double is the stolen `2♣`
    /// Stayman — opener with this club holding *passes*, defending `2♣` doubled
    /// instead of answering Stayman.  `convert_over_major` decides whether good
    /// clubs outrank a `2♥`/`2♠` major fit (`true`) or yield to it (`false`);
    /// `None` restores the prior flaw, where opener could never convert.
    ///
    /// **Default `Some((4, 4, true))`:** 4+ clubs with 4+ club HCP (an ace or
    /// two honors sitting over the overcaller), converting even with a major
    /// fit.  A/B'd a clear win at every gate tested (`ab-landy`, 2M, Landy off
    /// both arms): **+5.35/+7.28 IMPs/divergent (none/both) on plain DD,
    /// +5.32/+7.09 under perfect defense** — the conversion is a pure penalty
    /// decision, so the two scorers agree.  A looser gate captures more total
    /// IMPs (every gate down to `(4, 0, true)`, and even 3-card clubs, stays
    /// net positive on DD) at lower per-conversion quality; the default trades
    /// a little frequency for a genuine "good clubs" holding.  The A/B knob is
    /// `ab-landy --ns-penalty-pass LEN:HCP[:major]`.
    pub penalty_pass: Option<(usize, u8, bool)>,
    /// Author the trap pass
    ///
    /// Responder *traps* with a too-good stopper: a direct `3NT` additionally
    /// denies **5+ HCP in the overcall suit**, so a strong holding (AQ, KQ,
    /// AKJ…) passes instead — waiting for opener to reopen with a takeout
    /// double and converting it to penalty.  Strong honors in the overcaller's
    /// suit defend better than they declare.
    ///
    /// The `5`-HCP threshold is **distilled from a per-board double-dummy
    /// oracle** (`ab-lebensohl --pd-3nt --log-relay`): comparing `3NT` against
    /// trapping over sampled layouts, the trap rate rises monotonically with
    /// HCP *in their suit* (hcp 4 → 53%, 5 → 77%, 6+ → ~100%) and is
    /// **independent of length** — a long weak holding (e.g. ♠A9642, 4 HCP) is
    /// a running source that wants `3NT`, while a short strong one (♥AQ, 6 HCP)
    /// defends.  The earlier length-based gate (4+ cards) got this backwards
    /// and lost; this honor gate is the fix.
    ///
    /// **On by default** (A/B vs off, isolated, 200k plain DD: the
    /// `1NT`-Lebensohl responder gains `+172`/`+185` IMPs — the original
    /// `resp 3NT` losers, −22/−20, are erased — at a near-wash in the shared
    /// advance-of-takeout-double context; net `+155`/`+230`).
    pub trap_pass: bool,
    // --- competition/rubensohl.rs
    /// How a flat 4-3-3-3 cue-Staymans when our `1NT` is overcalled
    ///
    /// Default [`Competitive4333::Suppress`]; `--ns-competitive-4333` in
    /// `bba-gen` for the other arms.
    pub competitive_4333: Competitive4333,
    // --- competition/support_double.rs
    /// Support doubles/redoubles for the majors
    ///
    /// Extends opener's support double/redouble to the major-major auction
    /// `1♥ - 1♠ (X)` — or an overcall below `2♠`.  The minor-opening pairs are
    /// always on (shipped).
    ///
    /// **Default on** — measured vs BBA 2/1 (204.8k boards/arm/vul): plain DD
    /// wash (−0.0004/+0.0004, CIs straddle 0), perfect-defense +0.97/+1.69
    /// IMPs/fired NV/vul (vul CI excludes 0) — the plain-wash + PD-gain ship
    /// row (~0.10% fired).  `--no-ns-major-support-double` in `bba-gen` for the
    /// off arm.
    pub major_support_double: bool,
    // --- competition/two_suiters.rs
    /// Unusual-vs-unusual over their two-suiter showing both majors
    ///
    /// Responder's structure over the opponents' two-suiters over our `1♥`/`1♠`
    /// opening — their both-minors `(2NT)` and their Michaels cue of our own
    /// major.
    ///
    /// **Default on** — measured vs BBA 2/1 (204.8k boards/arm/vul): plain DD
    /// +0.0019/+0.0018 IMPs/board NV/vul (both CIs exclude 0; +1.43/+1.58
    /// IMPs/fired, ~0.12% fired), perfect-defense the same sign.
    /// `--no-ns-uvu-over-majors` in `bba-gen` for the off arm.
    ///
    /// Book construction only.  This cell once *also* gated the inference
    /// walk's hand-written two-suiter reading; that reader was retired in
    /// favour of the authored rules' own projection (chop 1 of
    /// `docs/reader-retirement.md`), so the reading is now owned by
    /// [`set_table_alert_reading`][crate::bidding::set_table_alert_reading].
    pub uvu_over_majors: bool,
    // --- competition/uvu.rs
    /// Author unusual-vs-unusual at all
    ///
    /// The Unusual-vs-Unusual structure over our `1NT` when an opponent
    /// overcalls a both-minors `2NT` (Section 5d).  Responder's `X` is penalty
    /// ("I can beat ≥1 of their suits"); the constructive answers are cue-bids
    /// — `3♣` = INV+ Stayman or 5+♠, `3♦` = INV+ 5+♥, `4♣`/`4♦` = FG+
    /// 5-5-majors splinters — with symmetric Smolen after the `3♣`→`3♦` denial.
    /// `3NT` is to play, and Pass is the finite catch-all.
    ///
    /// **Default on**: the constructive cues are DD-robust (A/B +0.6–2.6
    /// IMPs/board per call vs the passing floor) and the auction was previously
    /// unauthored.
    pub uvu: bool,
    /// HCP floor on the unusual-vs-unusual double
    ///
    /// Responder's penalty-double HCP floor over `1NT (2NT)` — the A/B sweep
    /// knob for "I can penalize one of their suits".  **Default 9**, the A/B
    /// best.
    pub uvu_x_floor: u8,
    /// Points floor on the unusual-vs-unusual cue
    ///
    /// Responder's INV+ cue-bid floor over `1NT (2NT)`, in points.  **Default
    /// 8**, the A/B best.
    pub uvu_cue_floor: u8,
    /// Length floor on the natural escape over their two-suiter
    ///
    /// Responder's weak natural `3♥`/`3♠` escape over `1NT (2NT)`.  **Default
    /// 6** — a clean six-bagger; `5` lets a five-card major escape when
    /// defending the both-minors overcall looks bad, and is the A/B sweep knob.
    pub uvu_natural_floor: u8,
}

impl Default for CompetitionKnobs {
    fn default() -> Self {
        Self {
            cue_raise_answer: true,
            cue_minor_raise_answer: true,
            delayed_cue: false,
            free_bids: false,
            free_bid_floor: 6,
            free_1nt_floor: 6,
            free_bid_quality: false,
            free_bid_style: FreeBidStyle::Forcing,
            high_overcall_responses: false,
            direct_3nt_stopper: true,
            natural_floor: (5, 0),
            lebensohl_style: LebensohlStyle::Transfer,
            defense_2d_multi: false,
            negative_double_shape: NegativeDoubleShape::Modern,
            cachalot_contested_x: true,
            weak_two_competition: false,
            strong_two_competition: true,
            competition_over_diamond_transfer: true,
            competition_over_transfer: false,
            competition_over_minor_transfer: true,
            competition_over_stayman: true,
            jordan_truscott: true,
            redouble_answer: true,
            splinter_doubled: true,
            double_style: DoubleStyle::Optional,
            penalty_double_leave_in: true,
            double_override: None,
            penalty_pass: Some((4, 4, true)),
            trap_pass: true,
            competitive_4333: Competitive4333::Suppress,
            major_support_double: true,
            uvu_over_majors: true,
            uvu: true,
            uvu_x_floor: 9,
            uvu_cue_floor: 8,
            uvu_natural_floor: 6,
        }
    }
}

/// The defensive book's build-time knobs
///
/// Each field is one cell, named for the getter it replaces; *derived* readings
/// stay functions of the module that owns them rather than becoming fields, so
/// the "one cell, one home" invariant survives the move.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DefenseKnobs {
    // --- defense.rs
    /// Prefer the longest suit when advancing partner's takeout double
    pub longest_first_advance_enabled: bool,
    /// Let a weak penalty pass yield to a four-card unbid major
    pub advance_pass_yield_major_enabled: bool,
    // --- defense/overcall.rs
    /// Shape gate for the natural penalty double of their `1NT`
    pub natural_double_shape: DoubleShape,
    /// Logit weight of the natural penalty double of their `1NT`
    pub natural_double_weight: i16,
    /// Support gate on the takeout double's 12+ tier
    pub takeout_support: TakeoutSupport,
    /// Use disciplined strength bands for natural suit overcalls
    pub overcall_discipline: bool,
    /// Allow a good four-card natural overcall
    pub overcall_four_card: bool,
    /// Let a passed hand make the disciplined two-level overcall lighter
    pub passed_hand_overcall: bool,
    /// Demand extra strength for a two-level minor overcall
    pub two_level_minor_overcall_tight: bool,
    /// Bar a five-card major from the natural `1NT` overcall
    pub nt_overcall_no_major: bool,
    /// Optional HCP seam between natural overcalls and the strong double
    pub strong_double_hcp: Option<u8>,
    // --- defense/nt_dont.rs
    /// Raw minimum length cell for direct DONT's one-suiter
    pub direct_dont_one_suiter_min: u8,
    /// Allow four-four two-suiters in direct DONT
    pub direct_dont_four_four: bool,
    /// Raw points-floor cell for direct DONT's double
    pub direct_dont_x_floor: u8,
    // --- defense/nt_woolsey.rs
    // --- defense/weak_two_nt_advance.rs
    /// Author advances of our `2NT` overcall of their weak two
    pub weak_two_notrump_advances_enabled: bool,
    // --- defense/advance_minor_jump.rs
    /// Author invitational minor jumps after partner's takeout double
    pub advance_minor_jump_enabled: bool,
    // --- defense/nt_defense.rs
    /// Extend the notrump defense to the balancing seat
    pub notrump_balancing_enabled: bool,
    // --- defense/leaping_michaels.rs
    /// Author Leaping Michaels over their weak two
    pub leaping_michaels_enabled: bool,
    // --- defense/weak_two_defense.rs
    /// Author the weak-two pass as the complement of stronger actions
    pub weak_two_pass_gate: bool,
    /// Require the `2NT` overcall to have the wide-notrump shape
    pub weak_two_notrump_shape: bool,
    /// Author jump overcalls over their weak two
    pub weak_two_jump_overcall: bool,
    /// Use disciplined bands for suit overcalls of their weak two
    pub weak_two_overcall_discipline: bool,
    /// Author the natural cue-bid over their weak two
    pub weak_two_cue: bool,
    /// Inclusive points band for the `2NT` overcall of their weak two
    pub weak_two_notrump_points: (u8, u8),
    /// Points bands for two- and three-level overcalls of their weak two
    pub weak_two_overcall_points: (u8, u8, u8, u8),
    // --- defense/advance_rubens.rs
    /// Author Rubens advances of partner's takeout double
    pub advance_rubens_enabled: bool,
    // --- defense/nt_landy.rs
    /// Escape thresholds after their double of Landy `2♣`
    pub doubled_landy_escape: (usize, usize),
    /// Gauge the Landy band in HCP rather than points
    pub landy_use_hcp: bool,
    /// Raw four-four-shape cell for direct Landy's double
    pub direct_landy_four_four: bool,
    /// Points floor for direct Landy's double
    pub direct_landy_double_floor: u8,
    /// Author the direct-Landy penalty pass
    pub direct_landy_penalty_pass: bool,
    // --- defense/michaels.rs
    /// Optional strength band for the unusual `2NT`
    pub unusual_notrump_range: Option<(u8, u8)>,
    /// Optional HCP floor for defensive two-suiters
    pub two_suiter_hcp_floor: Option<u8>,
    // --- defense/advance_sohl.rs
    /// Which sohl advance structure partner's takeout double uses
    pub advance_sohl_style: LebensohlStyle,
    // --- defense/nt_meckwell.rs
    /// Allow four-four in Meckwell's minor-major calls
    pub meckwell_minor_major_44: bool,
    /// Allow a four-four two-suiter in Meckwell's double
    pub meckwell_x_four_four: bool,
    /// Raw points floor cell for Meckwell's double
    pub meckwell_x_floor: u8,
    // --- defense/advance_2nt.rs
    /// Author the continuation after advancer's invitational `2NT`
    pub advance_2nt_continuation_enabled: bool,
    // --- defense/nt_their_conventions.rs
    /// Defend their Stayman convention
    pub stayman_defense_enabled: bool,
    /// Shape and strength floor for the natural call over their Stayman
    pub stayman_defense_overcall: (usize, u8),
    /// Defend their major-suit transfers
    pub transfer_defense_enabled: bool,
    /// Defend their minor-suit transfer
    pub minor_transfer_defense_enabled: bool,
    /// Defend their diamond transfer
    pub diamond_transfer_defense_enabled: bool,
    // --- defense/gladiator.rs
    // --- defense/advance_rich.rs
    /// Author the rich advance of partner's takeout double
    pub rich_advance_double_enabled: bool,
    /// Optional HCP gate on advancer's penalty pass
    pub advance_sit_hcp_gate: Option<u8>,
    // --- defense/responsive.rs
    /// Author responsive doubles after partner's takeout double
    pub responsive_takeout_enabled: bool,
    /// Author responsive doubles after partner's natural overcall
    pub responsive_overcall_enabled: bool,
}

impl Default for DefenseKnobs {
    fn default() -> Self {
        Self {
            longest_first_advance_enabled: true,
            advance_pass_yield_major_enabled: false,
            natural_double_shape: DoubleShape::Balanced,
            natural_double_weight: 130,
            takeout_support: TakeoutSupport::Strict,
            overcall_discipline: true,
            overcall_four_card: false,
            passed_hand_overcall: true,
            two_level_minor_overcall_tight: false,
            nt_overcall_no_major: false,
            strong_double_hcp: Some(18),
            direct_dont_one_suiter_min: 5,
            direct_dont_four_four: true,
            direct_dont_x_floor: 0,
            weak_two_notrump_advances_enabled: false,
            advance_minor_jump_enabled: true,
            notrump_balancing_enabled: false,
            leaping_michaels_enabled: true,
            weak_two_pass_gate: false,
            weak_two_notrump_shape: false,
            weak_two_jump_overcall: false,
            weak_two_overcall_discipline: true,
            weak_two_cue: false,
            weak_two_notrump_points: (16, 17),
            weak_two_overcall_points: (10, 16, 10, 16),
            advance_rubens_enabled: false,
            doubled_landy_escape: (6, 2),
            landy_use_hcp: false,
            direct_landy_four_four: false,
            direct_landy_double_floor: 15,
            direct_landy_penalty_pass: false,
            unusual_notrump_range: Some((8, 13)),
            two_suiter_hcp_floor: Some(8),
            advance_sohl_style: LebensohlStyle::Transfer,
            meckwell_minor_major_44: false,
            meckwell_x_four_four: true,
            meckwell_x_floor: 0,
            advance_2nt_continuation_enabled: true,
            stayman_defense_enabled: false,
            stayman_defense_overcall: (6, 14),
            transfer_defense_enabled: false,
            minor_transfer_defense_enabled: false,
            diamond_transfer_defense_enabled: false,
            rich_advance_double_enabled: true,
            advance_sit_hcp_gate: None,
            responsive_takeout_enabled: true,
            responsive_overcall_enabled: false,
        }
    }
}

impl DefenseKnobs {
    /// Capture this thread's defensive build-time knob state
    #[must_use]
    pub fn current() -> Self {
        super::american::defense::capture()
    }
}

/// The notrump book's build-time knobs
///
/// Each field is one cell, named for the getter it replaces; *derived* readings
/// stay functions of the module that owns them rather than becoming fields, so
/// the "one cell, one home" invariant survives the move. The three cells also
/// read at classify time live only in `DecisionProfile`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NotrumpKnobs {
    // --- notrump.rs
    // --- notrump/size_ask.rs
    /// How a balanced eight with no four-card major handles the size ask
    ///
    /// **Default [`SizeAskEight::Shipped`]** — the flat 4-3-3-3 subset passes
    /// (a shape with no ruff and no long suit is its high cards and nothing
    /// more, so it plays a level too high) and the shapelier eights invite via
    /// the `2♠`/`2NT` size ask.
    ///
    /// A throwaway measurement knob: [`Invite`][SizeAskEight::Invite]
    /// un-suppresses the flat 4-3-3-3 eight so the whole class size-asks,
    /// [`Pass`][SizeAskEight::Pass] sends the whole class to Pass.  Comparing
    /// the two poles under the single-dummy perfect-defense scorer
    /// ([`ns_score_pd_tricks`][crate::scoring::ns_score_pd_tricks]) re-prices
    /// the flat-4333 carve — decided on plain double dummy, which is
    /// level-dependently pessimistic on the low contracts in play (very on
    /// `1NT`, slightly on `3NT`) — and the still-live shapelier eights with
    /// realistic tricks.  The default is byte-identical to the shipped system.
    pub size_ask_eight: super::american::SizeAskEight,
    /// Opener's HCP floor for accepting the balanced-eight size ask
    ///
    /// Over the size ask (`2♠` Puppet max-signal `3♣`, or European `2NT`→`3NT`),
    /// a maximum accepts game and a minimum declines to `2NT`.  Shipped floor is
    /// **16** (accept the 24-HCP game): the earlier double-dummy probe rejected
    /// accept-16, but DD over-punishes the accepted `3NT` with doubled failures
    /// a realistic blind lead dodges.  Re-priced under the single-dummy
    /// perfect-defense scorer
    /// ([`ns_score_pd_tricks`][crate::scoring::ns_score_pd_tricks]) accept-16
    /// wins both vulnerabilities (SD-PD −0.85 NV / −2.16 vul IMPs/divergent,
    /// plain-DD non-negative), so 16 is default-on.  In the Puppet scheme this
    /// signal is shared with the club one-suiter; the club buckets measured
    /// benign.  Raise to 17 to restore the pre-2026-07-25 conservative accept.
    ///
    /// The meaningful sweep is `{15, 16, 17}` (opener's `1NT` range).  Floor 15
    /// is a **falsification control**, not a shippable setting: accepting on
    /// `15 + 8 = 23` turns the invite into a game force, so 15 *should* decline
    /// — the harness is expected to price accept-15 as a clear loss, and a
    /// measured *win* there is a warning that the scorer (not the treatment) is
    /// wrong.  It priced accept-15 at SD-PD +1.11 NV / +0.99 vul (a clear
    /// decline), validating the scorer.
    pub size_ask_accept_floor: u8,
    // --- notrump/both_majors.rs
    /// Show both four-card majors in response to Stayman
    ///
    /// Opener jumps to `2NT` over `1NT - 2♣` holding *both* four-card majors and
    /// a *maximum* (16-17); a minimum (15) bids `2♥` naturally.  Responder then
    /// names own major (`3♣` = hearts, `3♦` = spades) and opener completes
    /// (`3♥`/`3♠`), so the strong concealed hand declares the known 4-4 fit
    /// (right-siding) instead of responder declaring after a direct raise.
    ///
    /// **On by default** — a paired DD A/B vs BBA (320k boards/arm, vul none)
    /// measured +2.18 IMPs/fired plain (+0.0035/board, 95% CI excl 0) and +2.29
    /// PD *with garbage on*, +2.68/+2.87 with garbage off — a win in every
    /// regime, unlike the earlier strength-step scheme it replaces.
    pub stayman_both_majors: bool,
    /// Show a five-card major when answering Stayman with a maximum
    ///
    /// Opener jumps `3♥`/`3♠` over `1NT - 2♣` holding a *five-card* major and a
    /// maximum (16-17), showing the 5-3/5-4 fit plus extras.  **On by default**
    /// — the cleanest of the three: +3.45 IMPs/fired plain (+0.0007/board, 95%
    /// CI excl 0) and +3.33 PD, holding up at +1.47/+0.90 even with garbage on.
    pub stayman_5card_max: bool,
    // --- notrump/transfer_gf.rs
    /// Route minimum game-forcing minor side suits directly to `3NT`
    ///
    /// Within the GF structure, a minimum five-card-spade game force holding a
    /// four-card minor takes the choice-of-games `3NT` (the floor) rather than
    /// showing the minor, so `3♣`/`3♦` are reserved for slam tries.  **Off by
    /// default** — the losing arm B of the gf-majors A/B; the show-the-minor
    /// default beat it.  No-op unless `transfer_gf_majors` is on.
    pub minor_min_to_3nt: bool,
    // --- notrump/transfers.rs
    /// Author super-accepts of Jacoby transfers
    ///
    /// With four-card support for responder's major and a maximum (17), opener
    /// jumps to the three-level instead of merely completing the transfer, so
    /// the nine-card fit and the extra values are shown in one call.
    ///
    /// **Opt-in, off by default**: a paired double-dummy A/B vs BBA over 640 000
    /// boards found the jump a DD wash leaning negative (−0.055 IMPs/board it
    /// fires on) — opposite a transfer that may hold nothing, committing to the
    /// three-level overbids.
    pub transfer_super_accept: bool,
    /// Prefer the longer major when both majors can transfer
    ///
    /// The Jacoby transfer names the longer major (a 6♠5♥ hand transfers to
    /// spades, whatever its strength).  With **equal** lengths (5-5, 6-6) the
    /// route splits by strength: weak transfers to *hearts* (the safe partscore
    /// — nothing shows the spades below it anyway), invitational and minimum
    /// game force bid the both-majors `3♦` (which this discipline also restricts
    /// to equal lengths — a 6-5 hand prefers naming its longer suit first), and
    /// a slam try (17+) transfers to *spades* for the `1NT - 2♥ - 2♠ - 3♥`
    /// natural game-force structure.
    ///
    /// **On by default**; off restores the legacy guards for the A/B (a 6♠5♥
    /// hand could tie into the heart transfer, and `3♦` fired on any 5-5+).
    pub transfer_longer_major: bool,
    // --- notrump/crawling_stayman.rs
    // --- notrump/sixcard_invitation.rs
    /// Raw strength floor for inviting with a six-card major
    ///
    /// The `point_count + trump length` floor at which a six-card-major
    /// responder *invites* game — transfer at the two level, then jump to `3M` —
    /// instead of resting in the passed two-level partscore.  **Default 13**
    /// (on): the invitational band is
    /// `[13, `[`texas_game_floor`][Self::texas_game_floor]`)`, i.e. the
    /// just-below-blast sixes route through a `3M` invite; opener accepts on
    /// [`sixcard_accept_floor`][Self::sixcard_accept_floor].  Raise it to
    /// [`texas_game_floor`][Self::texas_game_floor] (14) to empty the band and
    /// turn the invite *off*.
    ///
    /// On by default as standard, expected major-suit bidding.  A paired A/B vs
    /// BBA (1.536M boards/arm, `--filter-1nt`, floor 13 over 14, accept floor
    /// 18; 1607 fired, 0.10%) measured **plain +0.619 IMPs/fired vul none,
    /// +1.820 both (CI excl 0); PD −0.211 / +0.561** — perfect-defense doubling
    /// trims the vul-none edge (the 3-level tax: the decline branch rests in
    /// `3M`), but a 6-card-fit `3M` partscore is not realistically doubled into a
    /// penalty at IMPs, so the PD-none figure overstates the downside.
    /// Double-dummy can't see the invite's real edge anyway — the `3M` brake on
    /// the thin games real defenders beat — so the conventional invite is kept
    /// on.  `probe-jacoby-invite-eval` experiment I has the opener-threshold
    /// sweep.
    pub sixcard_invite_floor: u8,
    /// Raw strength floor for accepting a six-card-major invitation
    ///
    /// Opener's accept floor for the six-card-major invite (`…3M → 4M`) on
    /// `point_count + trump length`; below it opener passes `3M`.  **Default
    /// 18**: a flat 15 with a doubleton in the major (15 + 2) declines, a 15 with
    /// three-card support (15 + 3) or any 16+ accepts — the ≈15% decline the
    /// probe's opener sweep found optimal.  Consulted only when the invite is on
    /// ([`sixcard_invite_floor`][Self::sixcard_invite_floor] <
    /// [`texas_game_floor`][Self::texas_game_floor]).
    pub sixcard_accept_floor: u8,
    // --- notrump/transfer_slam.rs
    /// Author the transfer slam-try structure
    ///
    /// After a Jacoby transfer completes (`1NT - 2♦ - 2♥` / `1NT - 2♥ - 2♠`), a
    /// single-suited five-card major with slam-invitational values (16+ HCP,
    /// opposite the 15–17 opener) bids the *other* major (`3♠` / `3♥`) as an
    /// artificial slam try agreeing the transfer major; opener launches RKCB with
    /// a maximum (`4NT`) or signs off in the major game (`4M`), and the `slam`
    /// 1430 ladder places the slam.  Mirrors the Stayman `3OM` slam try, which
    /// the transfer path lacked — so a strong balanced five-card-major responder
    /// used to rest in `3NT` while a major slam was cold (the dominant
    /// double-dummy leak in our `1NT` opening vs BBA).
    ///
    /// **On by default** — a paired on/off A/B (320k boards, shared seed, vs the
    /// BBA reference) measured **plain +0.0012 IMPs/board (95% CI ±0.0004), PD
    /// +0.0012 — +1.42 IMPs/fired in both regimes** (275 fired, 0.09%), every CI
    /// excluding 0.  Inert while the default-on GF-majors structure owns the same
    /// slot; it is that structure's fallback.
    pub transfer_slam_try: bool,
    // --- notrump/invitational_majors.rs
    /// Author the invitational five-card-major structure
    ///
    /// 5♠4♥ at invitational+ values keeps off the spade transfer and bids
    /// Stayman, inviting with a `2♠` rebid over opener's `2♦` (non-forcing) or
    /// `2♥` (forcing); 5♥4♠ transfers to hearts and rebids `2NT` (showing the
    /// four spades) or `2♠` (an artificial relay denying them).  A Muppet-style
    /// swap brought down to the two-level over `1NT`.
    ///
    /// **On by default** — a paired A/B vs BBA (1.28M boards/arm, `--filter-1nt`,
    /// vul none) measured **+0.375 IMPs/fired plain (+0.0020/board, 95% CI
    /// ±0.0004) and +0.134 PD (+0.0007/board, 95% CI ±0.0005)**, both excl 0.
    /// The win needed the doubled-`2♦` escape (`1NT - 2♣ - 2♦ (X)` systems-on
    /// rebase in `competition.rs`): without it the reroute walked 5♠4♥ into a
    /// doubled artificial `2♦` it passed out, and PD was a wash (−0.0001).
    pub invitational_5card_majors: bool,
    // --- notrump/texas.rs
    /// Route strong Texas hands through the slam-drive continuations
    ///
    /// The direct `1NT - 4♥/4♠` is a *non-forcing* slam try — opener moves only
    /// with a maximum, else passes the major game.  That strands the strong
    /// responder: a 16+ six-card-major hand opposite a *minimum* `1NT` (the
    /// majority) has a cold slam the opener vetoes by passing.  When on, the
    /// direct `4♥/4♠` is capped at the bare 15 invitational cusp (opener-decides
    /// is right there), and a 16+ hand instead Texas-transfers (`4♣/4♦`) and,
    /// over opener's completion, drives its own RKCB (`4NT`) — reaching the slam
    /// regardless of opener's minimum, exactly as the reference bidder does.
    ///
    /// **On by default** — a paired on/off A/B (320k boards, shared seed, vs the
    /// BBA reference) measured **plain +0.0024 IMPs/board (95% CI ±0.0006), PD
    /// +0.0024 — +5.87 IMPs/fired in both regimes** (131 fired, 0.04%), every CI
    /// excluding 0.
    pub texas_slam_drive: bool,
    /// Raw strength floor for the Texas game transfer
    ///
    /// The `point_count + trump length` floor at which a 6-card-major responder
    /// blasts game via South African Texas (`4♣/4♦`) instead of transferring at
    /// the two level.  **Default 14** (a 6-bagger needs 8 points, a 7-bagger 7).
    /// Below the floor the hand transfers at the two level (and passes the
    /// partscore).  No explicit upper cap: the slam-try `4♥/4♠` (weight 2.6)
    /// outranks the game blast (2.5) for the 15-18 band, so a slam-interested
    /// hand takes the direct slam try regardless.
    ///
    /// The book inherited a *raw-HCP* floor of **9** verbatim from the old
    /// transfer-then-game route (only the 15-18 slam edge was ever measured).  A
    /// double-dummy screen (`probe-jacoby-invite-eval`) found that 7-8 HCP 6-card
    /// hands score far better in `4M` than the partscore they stop in, that
    /// opener should *never decline* (so an invite degenerates to a blast), and
    /// that the `3M` invite-landing is a *worse* contract than `2M` at every
    /// strength (these one-suiters make 8 or 10 tricks, rarely 9) — so the choice
    /// is binary, pass-`2M` or blast-`4M`, with no invitational band.  At this
    /// *fit-rich* boundary distribution is a real trick (the 6th trump, ruffs),
    /// so the screen (experiments F/G) ranked `point_count + length` > CCCC >
    /// points > raw HCP for the blast decision — unlike the no-fit invite line
    /// (`probe-nt-invite-eval`) and the slam edge (`probe-texas-slam-eval`) where
    /// honors dominate and HCP won.
    ///
    /// Paired A/Bs vs BBA (1.024M boards/arm, `--filter-1nt`):
    /// `point_count+len≥14` over the old HCP-9 baseline measured **plain
    /// +0.0102/board vul none, +0.0171 both; PD +0.0082 / +0.0141**, and over a
    /// raw-HCP≥7 floor (the same aggressiveness) **plain +0.0013 / +0.0018; PD
    /// +0.0014 / +0.0019** — every regime a win, all 95% CI excl 0.  `14` matches
    /// the HCP≥7 blast rate while promoting shapely sixes (a 6-4 makes the cut at
    /// a bare 6) and demoting wasted-honor sevens.
    pub texas_game_floor: u8,
    // --- notrump/stayman.rs
    // --- notrump/splinter.rs
    /// Responder's HCP floor for the `1NT` splinter
    ///
    /// **Default 9** — BBA's measured floor for the same slot, and BWS's
    /// "strong".  Exists so the 8-versus-9 sweep is a flag rather than a rebuild;
    /// the eight is the marginal case, since a `3-1-4-5` eight currently *passes*
    /// `1NT`.  Consulted only while the splinter itself is authored
    /// (`nt_splinter`).
    pub nt_splinter_floor: u8,
    // --- notrump/stayman_slam.rs
    /// Author the Stayman cue-bid continuation
    ///
    /// Responder's continuation after opener cue-bids in cooperation with the
    /// `3OM` slam try (`1NT - 2♣ - 2M - 3OM - 4x`).  Opener's cue shows a control
    /// below the trump major with a maximum; without a responder node the floor
    /// *passed the cue* — often below game.  On, responder keycards (`4NT` RKCB)
    /// with slam values or signs off in the major game.
    ///
    /// **On by default** — the cue dead-end was the dominant Stayman leak vs BBA
    /// (≈20% of the tail-loss IMPs, `bba-gen --isolate-opening bba`).
    pub stayman_cue_continuation: bool,
    /// Author the Stayman minor-slam try
    ///
    /// Over opener's Stayman answer, responder's jump-free `3♣`/`3♦` shows a
    /// *natural* 5+ minor with slam values (14+) and no fit for opener's major —
    /// the 5-4 two-suiter whose four-card major (the reason for the `2♣` detour)
    /// missed.  Opener cooperates by raising the minor with a fit + maximum (else
    /// `3NT`), and responder keycards over the raise.
    ///
    /// **On by default** — the A/B landed +3.29/+4.02 IMPs/fired (none/both,
    /// plain DD; PD identical, no doubling artifact) across 151 fired boards,
    /// zero losses.
    pub stayman_minor_slam_try: bool,
    // --- notrump/long_minor.rs
    /// Author the source-of-tricks-eight long-minor force
    ///
    /// Whether a *source-of-tricks eight* forces `3NT` over `1NT` instead of
    /// transferring.  The hand: 8 HCP, no four- or five-card major (so it uses
    /// neither Stayman nor Jacoby), and a long minor that runs — a **7+ card
    /// minor**, or a **6-card minor headed by two of the top three honors**.
    ///
    /// **Off by default — measured a LOSS and kept only as an A/B instrument.**
    /// An analytic screen (`probe-force-eight`, 16M deals) looked positive —
    /// forcing `3NT` beat a *notrump* invite/pass by +0.2 to +0.5 IMPs/board —
    /// but that baseline is a fiction: these hands do not stop in notrump, they
    /// *transfer*, and the transfer reaches the suit game.  The live A/B against
    /// the real routing (`ab-long-minor-force`, 8M deals, plain DD, vul none)
    /// measured **−7.12 IMPs/fired** (club source −7.07: the `2♠` transfer drives
    /// to a *making 5♣* that `3NT` throws away; diamond source is a wash — the
    /// `2NT` transfer already reaches `3NT`).  So no shape forces; the transfer
    /// machinery bids these hands strictly better.
    pub long_minor_force: bool,
}

impl Default for NotrumpKnobs {
    fn default() -> Self {
        Self {
            size_ask_eight: SizeAskEight::Shipped,
            size_ask_accept_floor: 16,
            stayman_both_majors: true,
            stayman_5card_max: true,
            minor_min_to_3nt: false,
            transfer_super_accept: false,
            transfer_longer_major: true,
            sixcard_invite_floor: 13,
            sixcard_accept_floor: 18,
            transfer_slam_try: true,
            invitational_5card_majors: true,
            texas_slam_drive: true,
            texas_game_floor: 14,
            nt_splinter_floor: 9,
            stayman_cue_continuation: true,
            stayman_minor_slam_try: true,
            long_minor_force: false,
        }
    }
}

/// The opening book's build-time knobs
///
/// Each field is one cell, named for the getter it replaces; *derived* readings
/// stay functions of the module that owns them rather than becoming fields, so
/// the "one cell, one home" invariant survives the move.  `two_notrump_wide` is
/// read at classify time as well and so lives only in `DecisionProfile`,
/// deliberately absent here.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct OpeningKnobs {
    // --- openings/one_notrump.rs
    /// Open our strong `1NT` at all — **default on**
    ///
    /// Off, a strong balanced 15-17 opens a minor instead, so a diagnostic can
    /// isolate our *defense* to an opponent's `1NT` without our own `1NT`
    /// openings polluting the duplicate (`bba-match --no-our-1nt`).
    pub open_one_notrump: bool,
    /// Gauge the `1NT` range in Andrews' fifths rather than plain HCP
    ///
    /// **Default off.**  On restores the legacy `fifths(14.5..17.5)`, centre-matched
    /// to plain HCP 15-17; the shipped plain-HCP gauge opens `1NT` a touch more
    /// often.
    pub one_notrump_fifths: bool,
    /// Which balanced shapes the strong `1NT` opening admits
    ///
    /// **Default [`NotrumpShape::Wide6322`].**  The web Settings shape radio.
    pub notrump_shape: NotrumpShape,
    /// Admit the off-shape `1NT` (a singleton honour in 4441/5431)
    ///
    /// **Default off** (byte-identical); on also admits 5422.  Read by the
    /// generated convention card as well as by the rules.
    pub one_notrump_offshape: bool,
    // --- openings/weak_two.rs
    /// Optional raw-HCP band gauging the weak-two opening
    ///
    /// **Default `None`** — the shipped rule-of-N+8 `points(5..=10)`.  `Some`
    /// gauges `lo..=hi` in raw HCP instead.  The opening is *fit-unknown*, so a
    /// preempt's length is already pinned by the six-card requirement and
    /// gauging its *strength* in shape-crediting `points` double-counts that
    /// length: a six-card suit reads `+max(0, L2−8)`, +0 on 6-2-2-3 up to +2 on
    /// 6-4-2-1, so no single `points` shift restores a clean cutoff.  Only the
    /// fit-unknown *opening* moves — the Ogust min/max answers stay on `points`
    /// because responder's `2NT` promises support, mirroring the 2/1 gate's
    /// hcp/support-points fit-split.
    ///
    /// **Rejected default-on**: `hcp(5..=10)` measured a wash on the sd-lead
    /// scorer (−0.0045 NV / −0.0018 vul, CIs span 0) — a weak two is a preempt,
    /// and the plain-DD remnant the point-count campaign priced here is the
    /// obstruction/disclosure wall, not a fixable gauge.  A major-only carve
    /// measured strictly worse (sd-vul −0.0113).  Retained as a single-dummy
    /// re-measure candidate (`docs/point-count-threshold-campaign.md`).
    pub weak_two_hcp: Option<(u8, u8)>,
    /// Optional honour-location evaluator gauging the weak-two opening
    ///
    /// **Default `None`**; wins over [`weak_two_hcp`][Self::weak_two_hcp] when
    /// both are armed.  Tests the follow-up to the rejected raw-HCP re-gauge:
    /// that *where the honours sit* — concentrated in the six-card suit versus
    /// scattered through the short suits — separates the weak twos worth their
    /// disclosure from the rest.  Like the HCP knob, only the fit-unknown
    /// opening moves.
    pub weak_two_eval: Option<WeakTwoEval>,
    /// Open wild weak twos (five- or six-card suit, `points(3..=12)`)
    ///
    /// **Default off** (byte-identical).
    pub weak_two_wild: bool,
    // --- weak_twos.rs
    /// Prefer a good five-card major to the Ogust ask over a weak `2♦`
    ///
    /// **Default on.**  The old node ranked Ogust `2NT` (weight 2.0) above every
    /// new suit (1.5), so ♠AKT862 ♥AKJ92 ♦xx asked about diamond quality rather
    /// than showing eleven cards in the majors — the major only escaped when
    /// responder was short enough in diamonds to fail Ogust's `support(2..)`.
    /// On, the two major rules outrank Ogust **over `2♦` only**: there `2♥`/`2♠`
    /// is *cheaper* than `2NT` and forcing, so the ask is deferred rather than
    /// lost.  Over `2♥`/`2♠` a new suit costs `2♠` or the three level, so Ogust
    /// keeps priority.
    ///
    /// Expressed as a weight, not as a gate on the Ogust rule: `top_honors` is
    /// part of the new-suit gate, so excluding "any five-card major" from Ogust
    /// would strand 14+ hands like ♠QJxxx ♦xx, which have no new-suit rule at
    /// all.  The cost is that `2NT`'s projected reading still promises only
    /// "14+, 2+♦" and does not deny a major.
    ///
    /// Default-on on measurement: the enriched probe
    /// (`examples/probe-weak-two-major --mode ogust`, 20 000 accepted deals,
    /// 0.069% trigger density, 85% divergent) scored **+3.048 IMPs/accepted
    /// deal** under perfect defense, CI [+2.911, +3.184], and **+1.668** under
    /// plain DD, CI [+1.563, +1.773] — ≈+0.0021 IMPs/board.
    pub weak_two_major_priority: bool,
    /// Answer partner's weak two with the longest suit first
    ///
    /// **Default on**; off restores the pre-repair argmax race.  The new-suit
    /// rules all carry weight 1.5, so before the repair the winner was decided
    /// by `Table::next_call`'s tie-break — descending sort, first *legal* call,
    /// i.e. the **cheapest** bid.  ♠AKT862 ♥AKJ92 therefore responded `2♥` to a
    /// weak `2♦`, suppressing the longer and higher suit.  On, each rule gains
    /// `longest_unbid`, making the choice a constraint: longest first, an
    /// equal-length tie to the higher rank.  Same doctrine as
    /// [`longest_first_advance`][DefenseKnobs::longest_first_advance_enabled] on the
    /// advance side, and like it the condition is a `shapes` partition, so the
    /// *reading* also pins the relative length.
    ///
    /// Default-on as a doctrine repair, not a measured win: the collision needs
    /// two qualifying five-card suits opposite a weak two, roughly one board in
    /// 10⁴, so a random-deal A/B cannot resolve it.  The knob exists to ablate
    /// it in the enriched probe (`examples/probe-weak-two-major --mode tie`).
    pub weak_two_longest_first: bool,
}

impl Default for OpeningKnobs {
    fn default() -> Self {
        Self {
            open_one_notrump: true,
            one_notrump_fifths: false,
            notrump_shape: NotrumpShape::Wide6322,
            one_notrump_offshape: false,
            weak_two_hcp: None,
            weak_two_eval: None,
            weak_two_wild: false,
            weak_two_major_priority: true,
            weak_two_longest_first: true,
        }
    }
}

/// The response and raise books' build-time knobs
///
/// Each field is one cell, named for the getter it replaces; *derived* readings
/// stay functions of the module that owns them rather than becoming fields, so
/// the "one cell, one home" invariant survives the move.
/// `longer_major_response` is read at classify time as well — the M6.4
/// control-bid classifier reads the same discipline the response rule authors —
/// and so lives only in `DecisionProfile`, deliberately absent here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ResponseKnobs {
    // --- responses/two_over_one.rs
    /// Author the fit leg of the major 2/1 game force
    ///
    /// **Default on** — shipped 2026-07-15 jointly with the `Hcp13` gate
    /// (alone a vul-only plain win; the pair plain +0.0033/+0.0048, PD
    /// +0.0070/+0.0087 NV/vul — the fit leg re-admits with support what the
    /// hcp gate demotes).  Off-switch `--no-ns-two-over-one-fit` in `bba-gen`.
    ///
    /// On: a hand with exactly three-card support and a biddable side suit
    /// enters the 2/1 on `support_points(13..)` — the 2/1 is a preparation for
    /// `4M`, and the fit is privately known (opener promised five), so
    /// shortness counts.  Off: every 2/1 is gauged by the no-fit gate alone.
    pub two_over_one_fit: bool,
    /// The gauge for the no-fit leg of the major 2/1 game force
    ///
    /// **Default [`TwoOverOneGate::Points13`]** — shipped 2026-07-25 under the
    /// PointCount scale (277059f): `points(13..)` re-admits the shapely 11-12
    /// HCP hands that the raw-HCP `Hcp13` gate demoted to a forcing 1NT.
    /// `Hcp13` (shipped 2026-07-15) is the shape-indifferent opt-out.
    /// `--ns-two-over-one-gate` in `bba-gen`.
    pub two_over_one_gate: TwoOverOneGate,
    /// Name natural per-call suit lengths in the major 2/1 game force
    ///
    /// **Default off** (uniform four, book byte-identical); A/B pending.
    /// `--ns-two-over-one-natural-lengths` in `bba-gen`.
    ///
    /// On: `1♠ - 2♥` promises five (a 2/1 into a major is a real five-card
    /// suit) and `1♠ - 2♣` allows three (the cheapest 2/1 is the catch-all);
    /// every other 2/1 keeps its 4+ floor.
    pub two_over_one_natural_lengths: bool,
    /// Force game one HCP light on `1♠ - 2♥`
    ///
    /// **Default off** (book byte-identical); A/B pending.
    /// `--ns-two-over-one-major-discount` in `bba-gen`.
    ///
    /// On: the no-fit leg of `1♠ - 2♥` (the five-card-major 2/1) drops its
    /// `Hcp*` floor by one — `hcp(12..)` at the default `Hcp13` gate — because
    /// the five-card major is worth a game force a shade light, serving both
    /// 3NT and `4♥`.  No effect on the `Points*` gates or on any other 2/1.
    pub two_over_one_major_discount: bool,
    /// Force game on a flat twelve with five hearts on `1♠ - 2♥`
    ///
    /// **Default off**; measured via `ab-point-count --fix
    /// two-over-one-heart-light`.
    ///
    /// On: the no-fit leg of `1♠ - 2♥` becomes `len(♥,5..) & hcp(12..)` (from
    /// the default `points(13..)` at `min_len` four), banking its **ensured
    /// five-card heart suit** — admitting the flat 5=3=3=2 twelve-counts the
    /// `points` scale leaves at a forcing 1NT (they carry no `upgrade`).  The
    /// bet: unlike the minor 2/1s' thin 3NT, a five-card major finds a `4♥`
    /// landing whenever opener holds three — the strain-location fix, not the
    /// upgrade.  The fit leg (exactly-three-card spade support on
    /// `support_points(13..)`) is unchanged.  Unlike
    /// [`two_over_one_major_discount`][Self::two_over_one_major_discount]
    /// (which threads only the `Hcp*` gates), this overrides the `Points*`
    /// default directly.
    ///
    /// **Refuted 2026-07-25** (default stays off): the admitted flat twelves do
    /// not settle in the intended `4♥` on the 5-3 fit — the floor's slam
    /// machinery overshoots to `6♥`/`7♥` because the 2/1 response reads
    /// `0..=37` (the deferred fit-split `Or` erasure; see
    /// `docs/ai-bidder/sampled-projection.md`), so opener cannot see responder
    /// is a minimum.  A/B `ab-point-count --fix`: plain −0.0007/−0.0005, PD
    /// −0.0010/−0.0009 IMPs/board NV/vul.  A **reading-cap** re-measure
    /// candidate — capping the 2/1 reading (a ceiling, not just
    /// `set_two_over_one_slam_strength`'s floor) is the prerequisite.
    pub two_over_one_heart_light: bool,
    // --- responses/longer_major.rs
    /// Complete the natural minor tree up the line
    ///
    /// **Default on**, shipped **jointly with
    /// [`set_xyz`][crate::bidding::american::set_xyz]**
    /// (`ab-minor-continuations`, 300k boards: plain +0.0382/+0.0559
    /// IMPs/board NV/vul, PD +0.0289/+0.0407).  Alone it is a measured **loss**
    /// (plain −0.91/−1.28 per divergent) — the `1♦` response reroutes hands
    /// into auctions only the XYZ round continues; don't enable it with XYZ
    /// off.  Off-switch `--no-ns-up-the-line` in `bba-gen`.
    ///
    /// On: responder bids `1♦` over `1♣` on four-plus diamonds without a
    /// four-card major (off, those hands squeeze into the notrump ladder or
    /// fall to the floor), opener rebids `1♠` over `1m - 1♥` on four spades
    /// (off, the 4-4 spade fit is lost to a 1NT rebid), and opener rebids a
    /// natural `2♣` after `1♣ - 1♦` on six-plus clubs (off, a misdescribed 1NT
    /// catch-all).
    pub up_the_line: bool,
    // --- responses/choice_of_games.rs
    /// Author `1M - 3NT` as a choice of games
    ///
    /// **Default on** — shipped default-on 2026-07-15 (isolated: plain
    /// +0.0006/+0.0011 NV/vul, PD +0.0005/+0.0010, all CIs clear; perfectly
    /// additive atop the 2/1 fit-split, full-package numbers on that knob).
    /// Off-switch `--no-ns-major-choice-of-games` in `bba-gen`.
    ///
    /// On: `3NT` over `1♥`/`1♠` shows 3-4 card support, exactly (4333) and
    /// 12-15 HCP (over `1♥` it also denies four spades — that hand bids `1♠`
    /// first).  Opener passes with a balanced hand and corrects to `4M` with
    /// shape; the alerted reading pins responder's three-card support so later
    /// floor decisions know the fit.  Off: the flat hand routes through its
    /// lone four-card suit as a 2/1 (or Jacoby 2NT / limit raise with four
    /// trumps).
    pub major_choice_of_games: bool,
    // --- raises/game_try.rs
    /// Author the long-suit and general game tries after `1M - 2M`
    ///
    /// **Default on** (+0.042/+0.065 IMPs/board NV/vul, silenced-opponent A/B,
    /// 200k boards/vul, plain-DD + perfect-defense both winning).
    /// `--no-ns-major-game-tries` in `bba-gen` for the off arm.
    ///
    /// Responder's single raise promises three-plus trumps and 6–9 points, so
    /// opener needs real extras to move: a long-suit try, the general re-raise,
    /// or a keycard-asking maximum.
    pub major_game_tries: bool,
    // --- raises/limit_raise.rs
    /// Author opener's acceptance ladder after `1M - 3M`
    ///
    /// **Default on** (+0.002/+0.002 IMPs/board NV/vul — the whole win being
    /// the keycard ask at +4.4/+5.2 IMPs/divergent).
    /// `--no-ns-limit-raise-acceptance` in `bba-gen` for the off arm.
    ///
    /// Opener accepts, asks for keycards, or declines.
    pub limit_raise_acceptance: bool,
}

impl Default for ResponseKnobs {
    fn default() -> Self {
        Self {
            two_over_one_fit: true,
            two_over_one_gate: TwoOverOneGate::Points13,
            two_over_one_natural_lengths: false,
            two_over_one_major_discount: false,
            two_over_one_heart_light: false,
            up_the_line: true,
            major_choice_of_games: true,
            major_game_tries: true,
            limit_raise_acceptance: true,
        }
    }
}

/// The rebid book's build-time knobs
///
/// Each field is one cell, named for the getter it replaces; *derived* readings
/// stay functions of the module that owns them rather than becoming fields, so
/// the "one cell, one home" invariant survives the move.  Three of the area's
/// twelve cells are read at classify time as well — `opener_extras_ladder`,
/// `opener_major_jump_rebid` and `xyz` — and so live only in `DecisionProfile`,
/// deliberately absent here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RebidKnobs {
    // --- rebids.rs
    /// Rebid `1NT` rather than a natural `2m` on a balanced 12-14
    ///
    /// **Default on** (shipped: +0.0093 plain / +0.0101 PD IMPs/board vs BBA,
    /// both vuls).  After `1m - 1M`, a 5332 balanced minimum with the five-card
    /// minor otherwise rebids a natural `2m` (weight 0.9) that outranks the
    /// balanced `1NT` (0.5), misdescribing the hand and losing the `1NT`-based
    /// game placement BBA finds (the largest lever in the
    /// Constructive/book/round-2 anchor bucket).
    ///
    /// Natural and folded into base per `docs/bidding-options.md`; retained
    /// only as a measurement off-switch, not a user-facing toggle (dropped from
    /// the `web` settings registry).
    pub balanced_1nt_rebid: bool,
    // --- rebids/major_tails.rs
    /// Author the full continuations after `1♥ - 1♠`
    ///
    /// **Default on** (measured +0.016/+0.023 IMPs/board NV/vul plain DD;
    /// `--no-ns-major-rebid-tails` in `bba-gen` for the off arm).  Below each
    /// of opener's four rebids (`2♠`, `3♠`, `2♥`, `2♣`/`2♦`) both sides are
    /// authored to game, and — for the two spade-raise auctions — to slam via
    /// RKCB.  Two sub-agreements ride it:
    /// [`fourth_suit_forcing`][Self::fourth_suit_forcing] and
    /// [`nt_invite_hcp`][Self::nt_invite_hcp].
    pub major_rebid_tails: bool,
    /// Author fourth-suit forcing in the `1♥ - 1♠` tail
    ///
    /// **Default on** (measured +0.002 IMPs/board on top of the tails, both
    /// scorers, both vulnerabilities; `--no-ns-fourth-suit-forcing` in
    /// `bba-gen` for the off arm).  At `1♥ - 1♠ - 2♣`, responder's `2♦` becomes
    /// an artificial game force (the fourth suit) instead of natural diamonds.
    ///
    /// This continuation *rides* the major-rebid-tails adjunct — with
    /// [`major_rebid_tails`][Self::major_rebid_tails] off, enabling this knob
    /// registers nothing.
    pub fourth_suit_forcing: bool,
    /// Gauge responder's notrump invitation in raw HCP
    ///
    /// **Default on** (fix-vs-shipped, 1M boards/vul, 24.pdd 18.3M–20.3M:
    /// plain DD +0.0018 ± 0.0003 NV / +0.0022 ± 0.0005 vul, PD
    /// +0.0028/+0.0032); `false` restores the shipped `points` gauge.
    ///
    /// The 2NT rung after opener shows two suits (`1♥ - 1♠ - 2m`) is the
    /// table's one no-fit call — the hand denied a heart preference and a minor
    /// raise, so its long-suit `points` credit prices ruffs that a notrump
    /// part-score never takes (the quantitative-6NT reasoning one level down).
    /// Rule-of-N+8 reads a shaped 9-count 10+, invites, and loses both mirror
    /// directions (the point-count remnant's 2NT-invite seam).  The
    /// fit-showing rungs (`3♥`/`3m` invites) keep `points`, mirroring the 2/1
    /// hcp/support-points split.
    pub nt_invite_hcp: bool,
    // --- rebids/meckstroth.rs
    /// Author the complete Meckstroth adjunct
    ///
    /// **Default on.**  After `1M - 1NT` (the forcing notrump), opener's `2NT`
    /// is an artificial 18+ game force of *any* shape (responder relays `3♣`,
    /// opener shape-describes toward game or slam) instead of the natural 18–19
    /// balanced rebid; opener also has the invitational `3m` jumps (5+ minor,
    /// 15–17).  The `ab-meckstroth-2nt` A/B builds a baseline arm with it off.
    ///
    /// The artificial `2NT` measured a plain-DD win (`ab-meckstroth-2nt`,
    /// 200k×2 seeds: plain +0.0075/+0.013, PD +0.006/+0.011, sd-lead
    /// +0.010/+0.017 NV/vul, all CI-clean); the `3m` jumps are sd-vindicated
    /// (plain wash, PD over-punished, sd-lead +0.0012/+0.0042 NV/vul).
    pub meckstroth_adjunct: bool,
    /// Author the adjunct's invitational `3m` jumps
    ///
    /// **Default on**; ignored when
    /// [`meckstroth_adjunct`][Self::meckstroth_adjunct] is off — the jumps live
    /// inside the adjunct.  Turn this off to keep the game force and drop the
    /// jumps — the arm that isolates the `3m` leg, whose only positive bracket
    /// was plain SD.  One flag shipped both halves, and the SD-PD
    /// re-adjudication confirmed only the merged knob.
    pub meckstroth_minor_jumps: bool,
    // --- rebids/two_suiter.rs
    /// Author opener's two-suiter rebids over the forcing `1NT`
    ///
    /// **Default on**, sd-vindicated (`ab-forcing-nt-two-suiter`, 1M×2 seeds×2
    /// vuls): plain wash-NV/+0.0012-vul, PD −0.0017/−0.0010 (over-punished),
    /// sd-lead **+0.0012/+0.0028** NV/vul — all four sd cells CI-clean
    /// positive.  The A/B builds a baseline arm with it off.
    ///
    /// Over the forcing 1NT, opener with 15–17 and a second major suit has no
    /// invitational rebid — a 5-4 or 5-5 hand underbids as a minimum natural
    /// call.  This adds `1♥ - 1NT - 2♠` (reverse: 5+ hearts, 4+ spades) and
    /// `1♠ - 1NT - 3♥` (jump: 5-5 majors), both 15–17, with responder's
    /// continuations — the seam between the minimum natural rebids and the 18+
    /// game force ([`meckstroth_adjunct`][Self::meckstroth_adjunct]).
    pub forcing_nt_two_suiter: bool,
    // --- xyz.rs
    /// Let opener judge the checkback invitation rather than falling to the floor
    ///
    /// **Default on** (the shipped behavior).  The table is two rules —
    /// `points(14..)` bids the game, else `Pass` — with no shape, fit or
    /// vulnerability term, the same signature as the retired 2/1 game backstop.
    /// Off, it becomes an empty table, which is all-−∞ and so falls through to
    /// `instinct()` by the documented escape hatch.
    ///
    /// The most-*reached* candidate of the constructive book re-audit
    /// (`probe-node-reach`: 0.114% on one key, and the table is registered once
    /// per invite per prefix).  Only the crude `accept_or_decline` copies are
    /// gated; the shaped acceptances (three-card support, the 5♠4♥ hand) always
    /// author.
    pub xyz_invite_judgment: bool,
    // --- nmf.rs
    /// Author New Minor Forcing on the four `1m - 1M - 1NT` slots
    ///
    /// **Default off** — the shipped system uses XYZ.  Off-switch
    /// `--no-ns-new-minor-forcing` in `bba-gen`, opt-in `--nmf` in
    /// `ab-minor-continuations`.  When on it overrides XYZ on those four
    /// prefixes only — XYZ still owns the other six one-level auctions.
    pub new_minor_forcing: bool,
}

impl Default for RebidKnobs {
    fn default() -> Self {
        Self {
            balanced_1nt_rebid: true,
            major_rebid_tails: true,
            fourth_suit_forcing: true,
            nt_invite_hcp: true,
            meckstroth_adjunct: true,
            meckstroth_minor_jumps: true,
            forcing_nt_two_suiter: true,
            xyz_invite_judgment: true,
            new_minor_forcing: false,
        }
    }
}

/// The game-forcing book's build-time knobs
///
/// All three gate a *whole table* past round two of a two-over-one auction, and
/// all three trade against the floor rather than against another agreement: off
/// means the node falls through, not that a different rule fires.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GameForceKnobs {
    // --- game_force/backstop.rs
    /// Re-register the retired wildcard game backstop over uncovered nodes
    ///
    /// **Off by default**: every 2/1 continuation the three authored rounds do
    /// not cover falls through to the floor rather than to this table's three
    /// crude rules.
    ///
    /// The backstop was authored against the deterministic `instinct()` ladder;
    /// the floor became the BBA-distilled net on 2026-07-19, and the table
    /// stopped earning its keep.  Deleting it measures **+0.0117/+0.0142 plain,
    /// +0.0132/+0.0160 PD** IMPs/board NV/vul vs BBA (409,600×2, all CI>0)
    /// *paired with*
    /// [`set_two_over_one_force`][crate::bidding::instinct::set_two_over_one_force],
    /// which restores by rule the game force this node used to hold by
    /// omission.  On alone the deletion is worth only +0.005, because the floor
    /// then abandons partner's 2/1 on 24% of the boards it touches.
    ///
    /// Deleting it also cured a replay-sampler starvation: the table is
    /// *partial*, so every call it does not name sat at −∞ while its
    /// unconditional 3NT kept the node's best finite, and the gate rejected
    /// those calls for every hand (`sample_layouts_replay` returned 0%).  With
    /// no node the floor answers, `authored_at` is false, and the gate
    /// abstains.  Kept as a knob so the table can be re-measured if the floor
    /// changes again.
    pub game_backstop: bool,
    // --- game_force/opener_third.rs
    /// Author opener's third call after responder sets trump at `1M - 2r - R - 3M`
    ///
    /// **On by default** — but see the caveat, it is a deletion candidate
    /// blocked on a floor capability, not a settled node.
    ///
    /// Two rules — 4NT RKCB on `points(15..)`, else an unconditional `4M` — the
    /// retired game backstop's signature: a raw point threshold, no shape or
    /// control term, and every cue-bid and five-level call at −∞ at depth 4.
    ///
    /// Deleting it *measures* **+0.437/+0.527 plain, +0.524/+0.637 PD** IMPs
    /// per divergent board NV/vul (`ab-major-continuations`, 2,000,000 boards
    /// per arm per vulnerability, seed 1784484826, 971 divergent = 0.05%) —
    /// +0.0002/+0.0003 per board, the same sign on all four arms.
    ///
    /// **It is not shipped anyway.** With the node gone the floor never asks
    /// keycards here at all: it signs off in `4M` on a 26-count opposite a
    /// game-forcing two-over-one, so slam becomes unreachable at this node.
    /// That is the backstop lesson again — deleting a node deletes the
    /// invariant it held by omission, and here the invariant is "opener can
    /// still try for slam". A +0.0003 IMPs/board gain does not buy a total
    /// capability loss.
    ///
    /// The architecturally correct fix, if this is ever resumed, is the
    /// [`set_two_over_one_force`][crate::bidding::instinct::set_two_over_one_force]
    /// pattern: delete the node *and* teach `instinct()` to ask keycards on a
    /// controls-and-fit test at an agreed-trump game force, which should beat
    /// both arms. Only the raw point threshold is obviously wrong; the ask
    /// itself is load-bearing.
    ///
    /// The RKCB answer rows (`slam::rkcb_rows`) are independent of this knob.
    pub opener_third: bool,
    // --- game_force/second_suit.rs
    /// Author opener's third call after responder raises opener's second suit
    ///
    /// **On by default** — shipped (+0.0012 plain / +0.0014 PD NV, +0.0015 /
    /// +0.0018 vul IMPs/board vs BBA).  `1M - 2r - 2x - 3x` gets an opener
    /// rebid (RKCB on extras, else sign off) instead of falling through to the
    /// floor (it fell to the game backstop until that was deleted).
    pub second_suit_agreement: bool,
}

impl Default for GameForceKnobs {
    fn default() -> Self {
        Self {
            game_backstop: false,
            opener_third: true,
            second_suit_agreement: true,
        }
    }
}

/// The deterministic floor's build-time knobs
///
/// The floor's *other* knobs are read while it classifies and live in
/// `InstinctProfile` alongside the rest of the decision half; these three are
/// read inside [`instinct`][crate::bidding::instinct()]'s table builder, so
/// they are baked into the ladder that comes back.  Membership is decided by
/// where the cell is read, not by what it configures.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InstinctKnobs {
    /// Author the competitive long-suit rebid
    pub competitive_rebid: bool,
    /// Author opener's balanced-18-19 notrump actions in a contested auction
    pub reopening_notrump: bool,
    /// Author the doubler's runout after their redoubled penalty double
    pub doubler_xx_runout: bool,
}

impl Default for InstinctKnobs {
    fn default() -> Self {
        Self {
            competitive_rebid: true,
            reopening_notrump: true,
            doubler_xx_runout: true,
        }
    }
}

impl InstinctKnobs {
    /// Capture this thread's floor build-time knob state
    #[must_use]
    pub fn current() -> Self {
        super::instinct::capture_build()
    }
}

/// Everything the partnership has agreed to play
///
/// Constructed once per build and threaded down by reference. Cloning is cheap
/// but pointless: the whole design is that one capture serves a whole build.
///
/// One field per area of the system, plus `decision` for the cells read while
/// classifying rather than while building.  That last split is by *when* a
/// value is read, not by what it means — a build-time area and `decision` are
/// equally "what we agreed" — so it buys the `Stance` a small `Copy` snapshot
/// to pin and nothing else.
#[derive(Clone, Copy, PartialEq)]
pub struct Agreements {
    /// The classify-time cells, pinned into the stance at `Pair::against`
    pub(crate) decision: DecisionProfile,
    /// What we play when they contest our auction
    pub competition: CompetitionKnobs,
    /// What we play when they open the auction
    pub defense: DefenseKnobs,
    /// What we play after our notrump openings and rebids
    pub notrump: NotrumpKnobs,
    /// What we open, and how partner answers a weak two
    pub opening: OpeningKnobs,
    /// How partner answers our one-level opening, and how the raises continue
    pub response: ResponseKnobs,
    /// What opener rebids, and the checkback machinery over it
    pub rebid: RebidKnobs,
    /// Which continuations the game-forcing auction authors past round two
    pub game_force: GameForceKnobs,
    /// Which rules the deterministic floor's ladder authors
    pub instinct: InstinctKnobs,
}

impl Default for Agreements {
    /// The shipped system — what `american_default()` plays
    ///
    /// Equal to [`Agreements::current`] on a virgin thread
    /// (`build_defaults_match_the_cells`, `decision_defaults_match_the_cells`),
    /// which is what lets the cells be deleted without moving a bid.
    fn default() -> Self {
        Self {
            decision: DecisionProfile::default(),
            competition: CompetitionKnobs::default(),
            defense: DefenseKnobs::default(),
            notrump: NotrumpKnobs::default(),
            opening: OpeningKnobs::default(),
            response: ResponseKnobs::default(),
            rebid: RebidKnobs::default(),
            game_force: GameForceKnobs::default(),
            instinct: InstinctKnobs::default(),
        }
    }
}

impl Agreements {
    /// Capture this thread's knob state — the one read a build performs
    ///
    /// Every knob getter consulted here is consulted *here only*, so the
    /// readers downstream cannot disagree about what we play.
    #[must_use]
    pub fn current() -> Self {
        Self {
            decision: DecisionProfile::current(),
            competition: CompetitionKnobs::default(),
            defense: super::american::defense::capture(),
            notrump: NotrumpKnobs::default(),
            opening: OpeningKnobs::default(),
            response: ResponseKnobs::default(),
            rebid: RebidKnobs::default(),
            game_force: GameForceKnobs::default(),
            instinct: super::instinct::capture_build(),
        }
    }
}

#[cfg(test)]
mod tests;
