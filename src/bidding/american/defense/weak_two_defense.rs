//! Defending their weak two
//!
//! Takeout double, natural `2NT`, natural suit overcalls, and the cue — each
//! with its own point band, because a weak two both steals room and bounds
//! their hand.  The `2NT` overcall's advances are here too
//! ([`set_weak_two_notrump_advances`]).

use super::leaping_michaels::leaping_michaels_enabled;
use super::overcall::{TakeoutSupport, takeout_support};
use super::*;

thread_local! {
    /// Whether the direct-seat pass over their weak two documents the strong
    /// tier's complement (`points(..17)`) instead of the `hcp(0..)` catch-all.
    /// **Off by default** — REFUTED by A/B (204.8k bd/vul, `SEED_BASE`
    /// 1785083246: plain DD **−0.0028 ± 0.0017** NV / −0.0012 ± 0.0022 vul, PD
    /// +0.0005 ± 0.0022 / +0.0015 ± 0.0026, 0.45%/0.38% fired).  A sounder
    /// reading that bids worse.  See [`set_weak_two_pass_gate`].
    static WEAK_TWO_PASS_GATE: Cell<bool> = const { Cell::new(false) };
    /// Whether the 2NT overcall of their weak two takes the wider
    /// [`notrump_shape`] instead of strict [`balanced`].  **Off by default** —
    /// WASH over two seeds (204.8k bd/arm/vul; seed 1785085719 plain +0.0008 ±
    /// 0.0008 NV / +0.0008 ± 0.0009 vul, PD +0.0010/+0.0011; seed 1785086925
    /// plain +0.0002 ± 0.0008 / −0.0001 ± 0.0008, PD +0.0005/+0.0002).  Seed 1
    /// went 4/4 positive and did not replicate.  See
    /// [`set_weak_two_notrump_shape`].
    static WEAK_TWO_NOTRUMP_SHAPE: Cell<bool> = const { Cell::new(false) };
    /// Whether a jump in a new suit below 3NT is authored over their weak two:
    /// one trick more, so six-plus cards and three more points.  **Off by
    /// default** — LOST, 4/4 (204.8k bd/vul, seed 1785085719: plain −0.0008 ±
    /// 0.0007 NV / −0.0010 ± 0.0008 vul, PD −0.0012/−0.0011).  Overlapping
    /// bands, see [`set_weak_two_jump_overcall`].
    static WEAK_TWO_JUMP_OVERCALL: Cell<bool> = const { Cell::new(false) };
    /// Whether the natural suit overcall of their weak two demands more when
    /// **we** are vulnerable.  **On by default** — `win | win`, 8/8 cells over
    /// two seeds; see [`set_weak_two_overcall_discipline`].
    static WEAK_TWO_OVERCALL_DISCIPLINE: Cell<bool> = const { Cell::new(true) };
    /// Whether advancer's Gladiator structure over our 2NT overcall of their
    /// weak two in a major is authored.  **Off by default** — measured null and
    /// faintly negative; see [`set_weak_two_notrump_advances`].
    static WEAK_TWO_NOTRUMP_ADVANCES: Cell<bool> = const { Cell::new(false) };
    /// Whether the direct cue of their *major* weak two is authored as Michaels
    /// — the other major plus a minor, 5-5.  **Off by default** — the A/B is
    /// VOID (no advancer node; see [`set_weak_two_cue`]).
    static WEAK_TWO_CUE: Cell<bool> = const { Cell::new(false) };
    /// Inclusive `hcp` band of the 2NT overcall of their weak two; **(16, 17) by
    /// default** — 15-counts pass and 18-counts double, two disjoint wins that
    /// compose.  BBA's own bucket is 15–17 (median 16).  See
    /// [`set_weak_two_notrump_points`].
    static WEAK_TWO_NOTRUMP_POINTS: Cell<(u8, u8)> = const { Cell::new((16, 17)) };
    /// Inclusive `points` bands of the natural suit overcall of their weak two,
    /// by the level it lands on: `(two_lo, two_hi, three_lo, three_hi)`.
    /// **(10, 16, 10, 16) by default** — the shipped flat band at both levels.
    /// See [`set_weak_two_overcall_points`].
    static WEAK_TWO_OVERCALL_POINTS: Cell<(u8, u8, u8, u8)> =
        const { Cell::new((10, 16, 10, 16)) };
}

/// Gate the direct-seat pass over their weak two on the strong tier's
/// complement for books built *after* this call
///
/// On, `defense_to_weak_two`'s Pass rule reads `points(..17)` — the negative
/// inference of declining the shape-free `points(17..)` takeout double, exactly
/// as `defense_to_suit` already documents its own tier.  Off restores the
/// `hcp(0..)` catch-all, which projects ⊤ on all five axes the nets read.
///
/// Argmax-inert at the node itself (a 17+ hand already scored 1.2 for the
/// double against 0.0 for the pass), but the reading feeds
/// [`push_inference`][crate::bidding::features], so the neural floor sees
/// different inputs downstream.  The ceiling is sound only because that tier is
/// **shape-free**: it accepts every 17+ hand, so no hand that could have passed
/// is excluded.  A shaped tier would leave holes at every strength and no
/// ceiling would be authorable — which is why the analogous `1NT P` (90.7% ⊤ on
/// all five axes in the census) cannot be fixed this way.
///
/// **Default off — REFUTED** (204.8k bd/vul, `SEED_BASE` 1785083246; numbers on
/// the thread-local above).  Plain DD loses NV with a CI clear of zero and PD
/// washes: `loss | wash` never ships default-on.  The mechanism is the C1
/// encoding failure, not the bridge: capping the passer should make us *more*
/// cautious, yet every one of the five worst boards is the ON arm overbidding
/// into a double (6NT-X, 7♦-X, 5♦-X).  `push_inference` hands the net the raw
/// `{min, max}` pair, so `max/37` moves 1.00 → 0.43 on a seat it was trained to
/// see as ⊤ and it answers out of distribution.  Kept opt-in as a single-dummy
/// and post-retrain re-measure candidate: the reading itself is strictly sounder,
/// and an F2b-style evaluator twin selected on this knob would price it fairly.
pub fn set_weak_two_pass_gate(on: bool) {
    WEAK_TWO_PASS_GATE.with(|cell| cell.set(on));
}

fn weak_two_pass_gate() -> bool {
    WEAK_TWO_PASS_GATE.with(Cell::get)
}

/// Widen the 2NT overcall of their weak two from strict [`balanced`] shape to
/// `two_notrump_wide_shape` (2–4 majors, 2–6 minors) for books built *after* this call
///
/// `balanced()` in this crate is exactly 4333/4432/5332, so today a 6322 with a
/// solid six-card minor and their suit stopped has **no** 2NT — it doubles or
/// passes.  BBA's own 2NT bucket is 88–94% balanced with minors running to five,
/// so the rejected tail is real hands.
///
/// **Default off — WASH over two seeds** (numbers on the thread-local above).
/// Seed 1 came back positive in all four cells (+0.77 to +1.63 IMPs/fired) and
/// seed 2 did not replicate it (one cell mildly negative); pooled, every CI
/// still straddles zero.  The `wash | wash` tiebreak is naturalness, and it
/// argues the *other* way here: Cohen, kwbridge and the St Andrews notes all
/// specify **balanced** for this bid, so the narrow rule is the textbook one and
/// this widening is the trial.  Opt-in.
pub fn set_weak_two_notrump_shape(on: bool) {
    WEAK_TWO_NOTRUMP_SHAPE.with(|cell| cell.set(on));
}

fn weak_two_notrump_shape() -> bool {
    WEAK_TWO_NOTRUMP_SHAPE.with(Cell::get)
}

/// Author the jump in a new suit below 3NT over their weak two for books built
/// *after* this call
///
/// One trick higher than the cheapest overcall, so one trick more of hand:
/// **six-plus cards and three more points** than the natural band — natural,
/// non-forcing, strongly invitational.  Only three such calls exist below 3NT
/// (`3♥`/`3♠` over 2♦, `3♠` over 2♥); every other jump is at the four level.
/// BBA authors none of them, so this is an addition rather than a catch-up.
///
/// **Default off — LOST 4/4** (numbers on the thread-local above, −1.05 to
/// −1.61 IMPs/fired).  The trace is the classic case against *strong* jump
/// overcalls: the jump eats the room the strength wanted.
///
/// ```text
/// on:  2♦ 3♥ - 4♦ - 4♥ - - -     off: 2♦ 2♥ - 6♥ - - -
/// on:  2♦ 3♥ - 4♥ - - -          off: 2♦ 2♥ - 3♣ - 3♥ - 5♣ - - -
/// ```
///
/// The authoring makes it worse than it needs to be: `points(13..=19)` at weight
/// 1.1 **overlaps** the natural `points(10..=16)` at weight 1.0, so every 13–16
/// six-carder stops overcalling cheaply and jumps — precisely the hands that
/// wanted advancer to have room.  A retry should make the bands disjoint (jump
/// 17+, or cap the natural at 12 on six-card hands) before concluding anything
/// about jump overcalls as such.
pub fn set_weak_two_jump_overcall(on: bool) {
    WEAK_TWO_JUMP_OVERCALL.with(|cell| cell.set(on));
}

fn weak_two_jump_overcall() -> bool {
    WEAK_TWO_JUMP_OVERCALL.with(Cell::get)
}

/// Author the direct cue of their **major** weak two as Michaels for books built
/// *after* this call
///
/// `3♥` over 2♥ / `3♠` over 2♠ = the other major plus an unspecified minor, 5-5.
/// This is what BBA bids there (`probe-bba-constraints --mode def2-h`: ♠ 5–6,
/// longest minor 5–6, ♥ 0–2, 0% balanced) and what
/// [`set_cue_reading`][crate::bidding::set_cue_reading] already *reads* a direct
/// cue as — so knob-off the book authors a call the reader is waiting for.
///
/// Deliberately **not** extended to `3♦` over their 2♦: BBA never bids it (no
/// `3♦` bucket at all in `--mode def2-d`), the cheap 2♥/2♠ overcalls already
/// carry a major, and 4♦ Leaping Michaels covers the strong both-majors hand.
///
/// **Default off, and its A/B is VOID** — not a verdict on Michaels.  The
/// advancer has no node: the seat-fanned rows wire continuations for the
/// takeout double and Leaping Michaels only, so `[2♠, 3♠, P]` drops to the
/// floor, which *redoubles the cue* — the phantom-suit disaster in the flesh.
///
/// ```text
/// on:  - 2♠ 3♠ X XX - - -                                    (playing 3♠ redoubled — in their suit)
/// on:  2♠ 3♠ X 4♥ 4♠ - - X XX - - 4NT X 5♦ - 5♥ - 6♥ X - - -
/// ```
///
/// Measured −0.78 to −2.63 IMPs/fired, which is the missing continuation
/// talking.  Author advancer's structure (pick the major, relay for the minor,
/// and an SOS/pass-or-correct after their double) before re-measuring.
pub fn set_weak_two_cue(on: bool) {
    WEAK_TWO_CUE.with(|cell| cell.set(on));
}

fn weak_two_cue() -> bool {
    WEAK_TWO_CUE.with(Cell::get)
}

/// Demand more of the natural suit overcall of their weak two when **we** are
/// vulnerable (default **on**) for books built *after* this call
///
/// On, a vulnerable overcall needs 12–17 at the two level and 15–17 at the
/// three; non-vulnerable keeps the flat band
/// ([`set_weak_two_overcall_points`], default 10–16).  Off, the flat band
/// applies at every vulnerability.
///
/// Shipped on a `win | win`, 8/8 cells over two seeds (`SEED_BASE` 1785092622 /
/// 1785093604, 204.8k bd/arm/vul vs BBA 2/1), pooled:
///
/// | `-v` | fired | plain DD | PD |
/// | --- | --- | --- | --- |
/// | none | 0.00% | **0.0000 ± 0.0000** | 0.0000 ± 0.0000 |
/// | ns | 0.62% | **+0.0026 ± 0.0018** | +0.0136 ± 0.0022 |
/// | both | 0.67% | **+0.0029 ± 0.0020** | +0.0182 ± 0.0024 |
///
/// The `none` row is a free null control rather than a result: with nobody
/// vulnerable the rule reduces to the same `points(lo..=hi)` it replaced, so it
/// *must* read exactly zero on zero divergences, and a non-zero there would
/// have meant the vulnerability conjunct was miswired and the other two rows
/// meaningless.
///
/// The vulnerability conjunct is not a guess — it is what separated the earlier
/// exploratory measurement.  Run flat at 12:17:15:17 against the shipped band,
/// two seeds, 204.8k bd/arm/vul (`SEED_BASE` 1785088050 / 1785088953):
///
/// | `-v` | we vulnerable? | plain DD | PD |
/// | --- | --- | --- | --- |
/// | none | no | −0.0024 / −0.0029 | +0.0136 / +0.0132 |
/// | ns | **yes** | **+0.0048 / +0.0026** | +0.0165 / +0.0137 |
/// | both | **yes** | **+0.0063 / +0.0032** | +0.0223 / +0.0172 |
///
/// `none` and `both` are symmetric vulnerabilities and cannot tell our risk
/// from theirs; `ns` (we vulnerable, they not) is the cell that can, and plain
/// DD splits monotonically on **our** vulnerability with nothing left over —
/// so `vulnerable()` is the predicate and `they_vulnerable()` is refuted.
///
/// Note the PD column wins everywhere, including the cell plain DD loses.  That
/// is PD doing what PD does to a light overcall the field would never double,
/// and on its own it is the doubling artifact; the plain-DD half is the one
/// that flips, and it flips the way bridge says it should.
/// A/B knob (`bba-gen --ns-weak-two-overcall-discipline`).
pub fn set_weak_two_overcall_discipline(on: bool) {
    WEAK_TWO_OVERCALL_DISCIPLINE.with(|cell| cell.set(on));
}

fn weak_two_overcall_discipline() -> bool {
    WEAK_TWO_OVERCALL_DISCIPLINE.with(Cell::get)
}

/// Author advancer's Gladiator structure over our 2NT overcall of their weak
/// two in a **major** (default **off**) for books built *after* this call
///
/// Before this, the 2NT overcall had **no continuations at all** — the book
/// authors advances of the takeout double and of Leaping Michaels, but nothing
/// at `[2M, 2NT, P, ?]`, so advancer dropped to the instinct floor.  That is
/// the same structural hole that voided the `set_weak_two_cue` measurement,
/// except this call is a shipped default rather than an opt-in.
///
/// The scheme is Gladiator lifted one level, minus its invitational tier — at
/// 16–17 opposite there is no room to invite, so it is `3♣` or game:
///
/// ```text
/// 2♥ 2NT P  3♣    relay: weak, 5+ ♦, wants a 3-level partscore
///        P  3♦    game-forcing, 5+ ♦
///        P  3♥    cue = Stayman: exactly 4 ♠, game values, not flat
///        P  3♠    game-forcing, 5+ ♠
///        P 3NT    balanced game, to play
///
/// 2♥ 2NT P  3♣ P 3♦    forced, pass-or-correct, says nothing about diamonds
///                 P 3♥ cue = 6+ ♦, long enough that 4♦ is safe
///                 P  P play 3♦
/// ```
///
/// Two deliberate gaps, both `for now`.  Advancer's `3♠` and above in the relay
/// auction are unauthored, which means a *weak* hand with the other major has
/// no landing spot and passes 2NT — its correction would be exactly that `3♠`.
/// And over their `2♠` the delayed cue *is* `3♠`, so that whole rebid node is
/// omitted rather than half-authored.
///
/// A/B knob (`bba-gen --ns-weak-two-nt-advances`).
pub fn set_weak_two_notrump_advances(on: bool) {
    WEAK_TWO_NOTRUMP_ADVANCES.with(|cell| cell.set(on));
}

fn weak_two_notrump_advances_enabled() -> bool {
    WEAK_TWO_NOTRUMP_ADVANCES.with(Cell::get)
}

/// Set the inclusive `hcp` band of the 2NT overcall of their weak two (default
/// **16–17**) for books built *after* this call
///
/// The literature splits — Cohen and the Bridge Bulletin say 15–18, kwbridge
/// 14–18, the St Andrews notes 16–18 — and BBA's own direct-seat bucket is
/// **15–17, median 16** (`probe-bba-constraints --mode def2-h`).  Measurement
/// says both edges of the old 15–18 were wrong, and *independently* so.  The
/// two one-point trims act on disjoint hand classes, so each diverges from
/// 15–18 only at its own end — and a 15-count is some three times as common as
/// an 18-count, which is why trimming the floor moves twice the mass:
///
/// | band | trims | fired | plain NV/vul | PD NV/vul |
/// | --- | --- | --- | --- | --- |
/// | 15–17 | 18s → double | 0.06% | +0.0009 / +0.0004 | +0.0014 / +0.0007 |
/// | 16–18 | 15s → pass | 0.09% | +0.0006 / +0.0007 | +0.0024 / +0.0018 |
/// | **16–17** | both | 0.16% | **+0.0015 / +0.0011** | **+0.0037 / +0.0025** |
///
/// (IMPs/board, mean of seeds 1785088050 and 1785088953, 204.8k bd/arm/vul vs
/// BBA 2/1; pooled CI ±0.0008 plain, ±0.0009 PD.)  The 16–17 row is the sum of
/// the two above it to within noise on every cell, which is the tell that they
/// compose rather than compete.
///
/// The hands land where the system already wants them: an 18-count meets the
/// takeout double's `points(17..)`, and *that* is the classic double-then-
/// notrump auction.  A balanced 15 with a stopper has no home and passes —
/// facing a preempt with a partner who has not spoken, 2NT was buying a bad
/// 3NT.  A/B knob (`bba-gen --ns-weak-two-nt-points LO:HI`).
pub fn set_weak_two_notrump_points(lo: u8, hi: u8) {
    WEAK_TWO_NOTRUMP_POINTS.with(|cell| cell.set((lo, hi)));
}

fn weak_two_notrump_points() -> (u8, u8) {
    WEAK_TWO_NOTRUMP_POINTS.with(Cell::get)
}

/// Set the inclusive `points` bands of the natural suit overcall of their weak
/// two, separately for the calls that land at the two and three level (default
/// 10–16 at both — the shipped flat band) for books built *after* this call
///
/// A weak two leaves an overcall at either level depending on rank: over 2♥ a
/// spade overcall is `2♠` but a club overcall is `3♣`, and the flat band charges
/// both the same.  The one-opening defense already grades by level
/// ([`set_overcall_discipline`]: 1-level 8–17, 2-level 11–17), and the extra
/// trick has to be paid for somewhere.  BBA grades only slightly (10–16 at the
/// two level, 11–16 at the three).
/// A/B knob (`bba-gen --ns-weak-two-overcall LO2:HI2:LO3:HI3`).
pub fn set_weak_two_overcall_points(two_lo: u8, two_hi: u8, three_lo: u8, three_hi: u8) {
    WEAK_TWO_OVERCALL_POINTS.with(|cell| cell.set((two_lo, two_hi, three_lo, three_hi)));
}

fn weak_two_overcall_points() -> (u8, u8, u8, u8) {
    WEAK_TWO_OVERCALL_POINTS.with(Cell::get)
}

/// Our action over their weak-two opening
///
/// A weak two steals a level of room, so the toolkit is leaner than over a
/// one-bid: a takeout double (the workhorse), a natural 2NT overcall (15–18
/// with a stopper), and natural suit overcalls at the cheapest legal level.
/// Strong hands (17+) still double first, planning to bid again.
///
/// Overcall levels are derived from `their_opening`, so the suits higher than
/// theirs sit at the opening level and the lower ones one rung up — over 2♥, a
/// spade overcall is 2♠ but a club overcall is 3♣.
///
/// # Panics
///
/// Panics if `their_opening` is a notrump bid; pass a suit opening.
#[must_use]
pub fn defense_to_weak_two(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let level = their_opening.level.get();

    let (nt_lo, nt_hi) = weak_two_notrump_points();
    let mut rules = Rules::new();
    // The wide arm is `balanced() | (two_notrump_wide_shape() & two top honours in their
    // suit)`: the extra shapes are the 6322 and long-minor hands, which have a
    // trick source but one fewer stopper-guarded entry than a flat hand, so they
    // are asked for a *real* holding rather than the crisp Jxxx that
    // `stopper_in_their_suits` accepts.  Balanced hands keep today's gate, so the
    // knob only ever adds.
    rules = if weak_two_notrump_shape() {
        let theirs_suit = theirs.suit().expect("weak two is a suit bid");
        rules.rule(
            Bid::new(2, Strain::Notrump),
            150,
            hcp(nt_lo..=nt_hi)
                & stopper_in_their_suits()
                & (balanced() | (two_notrump_wide_shape() & top_honors(theirs_suit, 2..))),
        )
    } else {
        rules.rule(
            Bid::new(2, Strain::Notrump),
            150,
            hcp(nt_lo..=nt_hi) & balanced() & stopper_in_their_suits(),
        )
    };

    // 12+ takeout double, optionally gated on unbid-suit support (see
    // [`set_takeout_support`]); the 17+ tier catches off-shape strong hands.
    rules = match takeout_support() {
        TakeoutSupport::Off => rules.rule(
            Call::Double,
            130,
            hcp(12..) & short_in_their_suits() & takeout_double_shape_ok(),
        ),
        TakeoutSupport::Lenient => rules.rule(
            Call::Double,
            130,
            hcp(12..) & short_in_their_suits() & unbid_support(1) & takeout_double_shape_ok(),
        ),
        TakeoutSupport::Strict => rules.rule(
            Call::Double,
            130,
            hcp(12..) & short_in_their_suits() & unbid_support(0) & takeout_double_shape_ok(),
        ),
    }
    .alert(TAKEOUT_DOUBLE);

    // The pass gate documents the 17+ tier's complement, exactly as
    // `defense_to_suit` does — "strong hands double first regardless".
    // Byte-identical to the old `hcp(0..)` catch-all: below the floor both score
    // 0.0, above it the shape-free tier is finite at weight 1.2 and always
    // outscores a weight-0 pass.  Authored so the pass reading
    // (`set_pass_reading`) has a band to project; the ⊤ census found the
    // direct-seat pass over their weak two reading *nothing* on all five axes.
    rules = rules
        .rule(Call::Double, 120, points(17..))
        .alert(TAKEOUT_DOUBLE);
    rules = if weak_two_pass_gate() {
        rules.rule(Call::Pass, 0, points(..17))
    } else {
        rules.rule(Call::Pass, 0, hcp(0..))
    };

    // Natural overcalls: five-card suit, 10–16 points, at the cheapest legal level.
    // The jump one rung above is the same call with a trick more of hand — six-plus
    // cards and three more points — but only where it still fits under 3NT; every
    // other jump is at the four level, where Leaping Michaels already lives.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain != theirs {
            let overcall_level = if strain > theirs { level } else { level + 1 };
            // The extra trick has to be paid for: the band is graded by the level
            // the overcall lands on (`set_weak_two_overcall_points`; default flat).
            let (two_lo, two_hi, three_lo, three_hi) = weak_two_overcall_points();
            let (lo, hi) = if overcall_level <= 2 {
                (two_lo, two_hi)
            } else {
                (three_lo, three_hi)
            };
            // Red-vs-white is where a light overcall gets punished, and the
            // measurement splits on *our* vulnerability alone — so the
            // discipline is authored as a vulnerability conjunct rather than a
            // flat band (`set_weak_two_overcall_discipline`).
            rules = if weak_two_overcall_discipline() {
                let (vul_lo, vul_hi) = if overcall_level <= 2 {
                    (12, 17)
                } else {
                    (15, 17)
                };
                rules.rule(
                    Bid::new(overcall_level, strain),
                    100,
                    len(suit, 5..) & points_by_vul(lo..=hi, vul_lo..=vul_hi),
                )
            } else {
                rules.rule(
                    Bid::new(overcall_level, strain),
                    100,
                    len(suit, 5..) & points(lo..=hi),
                )
            };
            if weak_two_jump_overcall() && overcall_level < 3 {
                rules = rules.rule(
                    Bid::new(overcall_level + 1, strain),
                    110,
                    len(suit, 6..) & points(13..=19),
                );
            }
        }
    }

    // The direct cue of their MAJOR weak two: the other major plus a minor, 5-5 —
    // what BBA bids and what `set_cue_reading` already reads.  Over 2♦ the cue is
    // deliberately absent (see `set_weak_two_cue`).
    //
    // Game-forcing, the same `points(14..)` Leaping Michaels demands, because the
    // cue *is* a game force by geometry: over 2♠ advancer cannot bid 3♥ under it,
    // so a heart preference costs 4♥ and every other answer is 3NT or four of a
    // minor.  A `points(8..)` Michaels here would commit an 8-count to the four
    // level.
    let cue_major = match theirs {
        Strain::Hearts => Some(Suit::Spades),
        Strain::Spades => Some(Suit::Hearts),
        _ => None,
    };
    if weak_two_cue()
        && let Some(other) = cue_major
    {
        rules = rules
            .rule(
                Bid::new(3, theirs),
                160,
                len(other, 5..) & (len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..)) & points(14..),
            )
            .alert(MICHAELS);
    }

    // Leaping Michaels: a jump to 4♣/4♦ showing a 5-5 two-suiter with
    // game-forcing values.  These are all 4-level jumps, so they never collide
    // with the natural overcalls above (which sit at the 2/3 level), and 4♦ over
    // 2♦ is a cue the natural loop skips.
    if leaping_michaels_enabled() {
        let t = theirs.suit().expect("weak two is a suit bid");
        let gf = points(14..);
        match t {
            // Over a major: a minor plus the OTHER major.  Superseded by the cue
            // when that is on — the cue shows the same hand a level cheaper, and
            // at the same `points(14..)` every Leaping hand also satisfies it, so
            // leaving both would author a rung the weights can never reach.  BBA
            // makes the same choice: `--mode def2-h` shows a `3♥` cue bucket and
            // no `4♣`/`4♦` bucket at all.
            Suit::Hearts | Suit::Spades if !weak_two_cue() => {
                let other = if t == Suit::Hearts {
                    Suit::Spades
                } else {
                    Suit::Hearts
                };
                for minor in [Suit::Clubs, Suit::Diamonds] {
                    rules = rules
                        .rule(
                            Bid::new(4, Strain::from(minor)),
                            200,
                            len(minor, 5..) & len(other, 5..) & gf.clone(),
                        )
                        .alert(LEAPING);
                }
            }
            // Over 2♦: 4♣ = clubs + a major; 4♦ (cue) = both majors.  Advancer's
            // continuation (incl. the 4♣ major-ask) is authored in
            // `leaping_michaels_advances`.
            Suit::Diamonds => {
                rules = rules
                    .rule(
                        Bid::new(4, Strain::Clubs),
                        200,
                        len(Suit::Clubs, 5..)
                            & (len(Suit::Hearts, 5..) | len(Suit::Spades, 5..))
                            & gf.clone(),
                    )
                    .alert(LEAPING)
                    .rule(
                        Bid::new(4, Strain::Diamonds),
                        200,
                        len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & gf.clone(),
                    )
                    .alert(LEAPING);
            }
            Suit::Clubs => {} // no weak 2♣ in our system
            // Majors with the cue on: the guarded arm above declined them.
            Suit::Hearts | Suit::Spades => {}
        }
    }
    rules
}

/// [`defense_to_weak_two`] over each weak-two opening, as rows
///
/// The exact-node pilot of the row layer: `P* (2♦)` and friends lower to the
/// plain seat-fanned insert.  Clubs is omitted — a 2♣ opening is the strong
/// artificial bid, not a weak two.
pub(super) fn weak_two_defense_package() -> Package {
    Package {
        name: "weak-two-defense",
        gate: || true,
        entries: || {
            [Suit::Diamonds, Suit::Hearts, Suit::Spades]
                .into_iter()
                .flat_map(|suit| {
                    let opening = Bid::new(2, Strain::from(suit));
                    rows_of(
                        Pattern::node(&format!("P* ({opening})")),
                        defense_to_weak_two(opening),
                    )
                })
                .collect()
        },
    }
}

/// Advancer's Gladiator structure over our `2NT` overcall of their weak two in
/// a **major** ([`set_weak_two_notrump_advances`])
///
/// The 1NT-level Gladiator ([`gladiator_advances`]) needs an invitational tier
/// and spends the two level on it.  Here the overcall is narrow — 16–17 by
/// default — so eight points opposite is already game values and the tier
/// vanishes: `3♣` is the weak relay, everything above it is game-forcing, and
/// the cue is Stayman.  The threshold tracks
/// [`set_weak_two_notrump_points`]'s floor, so a widened band raises it
/// instead of silently keeping a tierless structure calibrated for 16.
///
/// Every artificial call states its true meaning in its own rule text, so the
/// `.alert(...)` is the whole reading — `project_authored` decodes the box and
/// suppresses the phantom natural suit at the same index.  No bespoke
/// `Inferences` arm is needed (contrast `gladiator_reading`, whose relay has no
/// sound per-suit floor to project).
fn weak_two_notrump_advances(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let m = Strain::from(their_major);
    let os = Strain::from(o);
    // Game values opposite the band's *minimum*: at the default 16–17 that is
    // eight (16 + 8 = 24).  Keyed to the band rather than frozen at 8, so
    // widening it with [`set_weak_two_notrump_points`] cannot leave advancer
    // driving to game on a 23-count — the bias would fall on exactly the hands
    // the widening adds.
    let game = 24u8.saturating_sub(weak_two_notrump_points().0);

    Rules::new()
        // Cue = Stayman for the *one* unbid major.  A flat (4333) is barred —
        // with no ruffing value a 4-4 fit does not beat 3NT (the 4333 curse).
        .rule(
            Bid::new(3, m),
            140,
            len(o, 4..=4) & points(game..) & !flat_4333(),
        )
        .alert(WEAK_TWO_NT_STAYMAN)
        // Game-forcing naturals: a real five-plus suit.
        .rule(
            Bid::new(3, Strain::Diamonds),
            130,
            len(Suit::Diamonds, 5..) & points(game..),
        )
        .rule(Bid::new(3, os), 130, len(o, 5..) & points(game..))
        // Balanced game, to play — the overcaller holds the stopper, so the
        // notrump is right-sided as it stands.
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            balanced() & points(game..),
        )
        // `3♣` = relay: a weak diamond hand looking for a 3-level partscore.
        // The forced `3♦` is the landing spot; a weak hand with the *other*
        // major has none (its correction would be `3♠`, unauthored) and passes.
        .rule(
            Bid::new(3, Strain::Clubs),
            50,
            points(..game) & len(Suit::Diamonds, 5..),
        )
        .alert(WEAK_TWO_NT_RELAY)
        .rule(Call::Pass, 30, hcp(0..))
}

/// Overcaller's forced `3♦` completion of the `3♣` relay
///
/// Pass-or-correct and utterly blind — it says nothing about diamonds, which is
/// why it is alerted: the alert is what stops the walk floring a phantom suit.
fn weak_two_notrump_relay_reply() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 100, hcp(0..))
        .alert(WEAK_TWO_NT_RELAY_PC)
}

/// Advancer's rebid over the forced `3♦`: pass to play it, or cue their major
/// to say the diamonds are long enough that `4♦` is safe
///
/// Only authored over their `2♥`, where the cue is `3♥`.  Over `2♠` the cue is
/// `3♠` itself, which is left unauthored for now — so the whole node is.
fn weak_two_notrump_relay_rebid(their_major: Suit) -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::from(their_major)),
            100,
            len(Suit::Diamonds, 6..),
        )
        .alert(WEAK_TWO_NT_DIAMONDS)
        .rule(Call::Pass, 50, hcp(0..))
}

/// At least 5-4 (or 4-5) in the two named suits — the Landy two-suiter shape
pub(crate) fn five_four(a: Suit, b: Suit) -> Cons<impl Constraint + Clone> {
    (len(a, 5..) & len(b, 4..)) | (len(a, 4..) & len(b, 5..))
}

/// A *passed-hand* two-suiter in `a`+`b`: at least 5-4, but with neither suit
/// six-plus.  A passed hand holding a six-card suit would have opened a weak two
/// or a three-level preempt in first seat (see `openings.rs`), so those openable
/// shapes are excluded from the passed-hand 1NT defense — leaving the genuine
/// two-suiters that had no first-seat voice.  (A 5-4 two-suiter has at most four
/// cards in any third suit, so capping `a`/`b` at five bars every six-card suit.)
pub(crate) fn passed_two_suiter(a: Suit, b: Suit) -> Cons<impl Constraint + Clone> {
    five_four(a, b) & len(a, ..=5) & len(b, ..=5)
}

/// Advancing our `2NT` overcall of their weak two ([`set_weak_two_notrump_advances`])
///
/// Majors only — over `2♦` both majors are unbid, so the cue has no Stayman to
/// be.
pub(super) fn weak_two_notrump_advance_package() -> Package {
    Package {
        name: "weak-two-notrump-advance",
        gate: weak_two_notrump_advances_enabled,
        entries: || {
            let mut entries = Vec::new();
            for suit in [Suit::Hearts, Suit::Spades] {
                let opening = Bid::new(2, Strain::from(suit));
                let base = format!("P* ({opening}) 2NT (P)");
                entries.extend(rows_of(
                    Pattern::node(&base),
                    weak_two_notrump_advances(suit),
                ));
                entries.extend(rows_of(
                    Pattern::node(&format!("{base} 3♣ (P)")),
                    weak_two_notrump_relay_reply(),
                ));
                // The delayed cue is 3♥ over their 2♥ but 3♠ over their 2♠, and
                // 3♠+ is unauthored — so over 2♠ the node would be Pass alone.
                if suit == Suit::Hearts {
                    entries.extend(rows_of(
                        Pattern::node(&format!("{base} 3♣ (P) 3♦ (P)")),
                        weak_two_notrump_relay_rebid(suit),
                    ));
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
