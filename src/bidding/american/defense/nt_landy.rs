//! Landy — `2♣` for both majors over their `1NT`, and its advances
//!
//! The both-majors `2♣` and everything below it: advancer's preference, the
//! `2NT` game-force ask, the doubled runout and the SOS redouble.
//! [`set_landy`] sets its band; [`set_direct_landy_double`] adds the
//! direct-seat `X`.  Woolsey reuses the same `2♣` call, so the both-majors
//! shapes live here.

use super::nt_defense::{NotrumpDefense, notrump_defense, set_notrump_defense};
use super::nt_woolsey::{set_woolsey_points, woolsey_enabled, woolsey_points};
use super::weak_two_defense::five_four;
use super::*;

thread_local! {
    /// Landy defense to their 1NT: `None` = off (the default natural overcalls +
    /// penalty double); `Some((lo, hi))` = on, with `2♣` = both majors and
    /// `2NT` = both minors on `points(lo..=hi)`.  See [`set_landy`].
    static LANDY: Cell<Option<(u8, u8)>> = const { Cell::new(None) };
}

/// Configure the Landy defense to an opponent's 1NT for books built *after* this
/// call (thread-local, read once at book-construction time)
///
/// `None` (the **default**) keeps today's natural defense: a penalty double
/// (15+ balanced) and natural two-level suit overcalls.  `Some((lo, hi))` turns
/// Landy on: `2♣` shows at least 5-4 in the majors and `2NT` at least 5-4 in the
/// minors, both on `points(lo..=hi)`, at the cost of the natural `2♣` club
/// overcall.  The range is the A/B sweep knob (`examples/ab-landy --ns-majors`);
/// the advancer's invite/game thresholds and the overcaller's min/med/max
/// rebid track it, so a lighter overcall asks more of the advancer.  It also
/// *is* the shared two-suiter band — see [`set_woolsey_points`] — so Landy's and
/// Woolsey's identical both-majors `2♣` always overcall at the same strength.
pub fn set_landy(range: Option<(u8, u8)>) {
    LANDY.with(|cell| cell.set(range));
    // Coupled with Woolsey: the both-majors `2♣` is the identical call in both
    // conventions, so they share one strength band — the [`woolsey_points`] cell.
    // A Landy range feeds that band, so the two can never carry divergent strengths.
    // (Measured: the `:19` cap binds on ~0 hands and the floor barely moves the IMPs,
    // so one knob loses nothing; see `examples/ab-landy` / `bba-gen --ns-landy`.)
    if let Some((lo, hi)) = range {
        set_woolsey_points(lo, hi);
    }
}

/// The configured Landy range, or `None` when Landy is off
///
/// Crate-visible so the inference projection pass and the Landy relay stub can
/// condition partner on the two-suiter (see `inference::authored_reading` and
/// `inference::landy_advance_suppress`).
pub(crate) fn landy_range() -> Option<(u8, u8)> {
    LANDY.with(Cell::get)
}

thread_local! {
    /// The `(min minor length, max length in each major)` gate for the doubled-Landy
    /// minor escapes (`Pass` = clubs, `2♦` = diamonds).  **Default `(6, 2)`**.  See
    /// [`set_doubled_landy_escape`].
    static DOUBLED_LANDY_ESCAPE: Cell<(usize, usize)> = const { Cell::new((6, 2)) };
}

/// Tune the doubled-Landy minor-escape gate for books built *after* this call
/// (thread-local, read once at book-construction time)
///
/// After `(1NT) 2♣ (X)` the advancer may run to a long minor — `Pass` to play `2♣`
/// doubled with clubs, `2♦` to play diamonds — but only with `min_minor`+ in that
/// minor and at most `max_major` in *each* major (a longer major has an 8-card fit
/// opposite the overcaller's 5-carder worth more than a doubled minor).  **The
/// default `(6, 2)`** is the A/B-tuned shipped gate; the knob is
/// `examples/landy-ab --ns-doubled-escape MIN:MAJ`.  Only reachable when Landy is
/// on ([`set_landy`]), so the convention stays opt-in.
pub fn set_doubled_landy_escape(gate: (usize, usize)) {
    DOUBLED_LANDY_ESCAPE.with(|cell| cell.set(gate));
}

/// The configured doubled-Landy minor-escape gate
fn doubled_landy_escape() -> (usize, usize) {
    DOUBLED_LANDY_ESCAPE.with(Cell::get)
}

thread_local! {
    /// Whether the Landy `2♣` / unusual `2NT` strength range gauges raw [`hcp`]
    /// rather than the default shape-upgraded [`points`]; see [`set_landy_hcp`].
    static LANDY_HCP: Cell<bool> = const { Cell::new(false) };
}

/// Gauge the two-suiter overcall strength on raw HCP instead of upgraded points,
/// for books built *after* this call (thread-local, read once at book-construction)
///
/// A 5-4/5-5 two-suiter earns a distributional bonus, so [`points`] runs ~2 above
/// HCP — letting thin hands clear the floor.  `true` gauges the `2♣`/`2NT` range on
/// raw [`hcp`] (tighter); `false` (the **default**) keeps [`points`].  An A/B knob
/// (`examples/landy-ab --strength hcp`).
pub fn set_landy_hcp(on: bool) {
    LANDY_HCP.with(|cell| cell.set(on));
}

/// Whether the two-suiter strength range gauges raw HCP
pub(super) fn landy_use_hcp() -> bool {
    LANDY_HCP.with(Cell::get)
}

thread_local! {
    /// Whether the direct-Landy both-majors `X` accepts a flat 4-4 (else 5-4+) — the
    /// payload of the former `DIRECT_LANDY_DOUBLE` `Option`.  No effect unless the
    /// active system is [`NotrumpDefense::DirectLandy`].
    static DIRECT_LANDY_FOUR_FOUR: Cell<bool> = const { Cell::new(false) };
}

thread_local! {
    /// The `points` floor for the direct-seat both-majors double; **15 by default**
    /// — the clean partition just above the natural-overcall ceiling (14), so an
    /// intermediate both-majors hand overcalls a major (8–14) and the `X` is reserved
    /// for the strong hands too good to overcall (15+).  Competing less (fewer thin
    /// doubles to be punished) and carrying more defense when we act both helped on the
    /// A/B sweep, which peaked near 15–16; 15 captures it with no orphaned point-count.
    /// The advancer's invite/game thresholds track it.  See [`set_direct_landy_double_floor`].
    static DIRECT_LANDY_DOUBLE_FLOOR: Cell<u8> = const { Cell::new(15) };
    /// Whether the advancer may **pass the both-majors `X` for penalty** (defend
    /// `1NTx`) at `(1NT) X -`; **off by default**.  On, a hand with no major fit
    /// (both majors ≤2) and enough defense converts the takeout double to penalties
    /// rather than running to a 5-2 major; the threshold tracks the X floor (a
    /// stronger X needs less from the advancer).  See [`set_direct_landy_penalty_pass`].
    static DIRECT_LANDY_PENALTY_PASS: Cell<bool> = const { Cell::new(false) };
}

/// Replace the direct-seat 15+ penalty double of their 1NT with a both-majors
/// takeout double, for books built *after* this call (thread-local, read once at
/// book-construction time)
///
/// `None` (the **default**) keeps the natural penalty-X defense.  `Some(false)`
/// makes `X` show at least 5-4 in the majors at every seat; `Some(true)` accepts a
/// flat 4-4.  The penalty double is dropped entirely (a 15+ balanced hand passes or
/// overcalls), the four natural two-level suit overcalls are kept, and the advancer
/// answers through the Landy machinery (`landy_advances`).  Mutually exclusive
/// with the natural penalty-X arm and the Landy `2♣` overlay (this covers the
/// passed seat too).  The A/B knob for `examples/ab-landy --ns-landy-x`.
///
/// A back-compat shim over [`set_notrump_defense`]: `Some(four_four)` selects
/// [`NotrumpDefense::DirectLandy`] and stores the shape flag; `None` reverts to
/// [`NotrumpDefense::Natural`] when direct-Landy is the active system (else a no-op).
pub fn set_direct_landy_double(shape: Option<bool>) {
    match shape {
        Some(four_four) => {
            set_notrump_defense(NotrumpDefense::DirectLandy);
            DIRECT_LANDY_FOUR_FOUR.with(|cell| cell.set(four_four));
        }
        None if notrump_defense() == NotrumpDefense::DirectLandy => {
            set_notrump_defense(NotrumpDefense::Natural);
        }
        None => {}
    }
}

/// The configured direct-seat both-majors double shape, or `None` when off
pub(crate) fn direct_landy_double() -> Option<bool> {
    (notrump_defense() == NotrumpDefense::DirectLandy)
        .then(|| DIRECT_LANDY_FOUR_FOUR.with(Cell::get))
}

/// Set the `points` floor for the direct-seat both-majors double (default 8), for
/// books built *after* this call.  A higher floor reserves the `X` for stronger
/// hands (lighter both-majors hands overcall a major naturally) — competing less
/// and penalizing more.  The advancer's invite/game thresholds track it.  No effect
/// unless [`set_direct_landy_double`] is on.  The A/B knob for `examples/ab-landy
/// --ns-landy-x-floor`.
pub fn set_direct_landy_double_floor(floor: u8) {
    DIRECT_LANDY_DOUBLE_FLOOR.with(|cell| cell.set(floor));
}

/// The configured both-majors double `points` floor
pub(super) fn direct_landy_double_floor() -> u8 {
    DIRECT_LANDY_DOUBLE_FLOOR.with(Cell::get)
}

/// Allow the advancer to pass the both-majors `X` for penalty (defend `1NTx`) when it
/// has no major fit and enough defense, for books built *after* this call (default
/// off).  No effect unless [`set_direct_landy_double`] is on.  The A/B knob for
/// `examples/ab-landy --ns-landy-x-penalty`.
pub fn set_direct_landy_penalty_pass(on: bool) {
    DIRECT_LANDY_PENALTY_PASS.with(|cell| cell.set(on));
}

pub(super) fn direct_landy_penalty_pass() -> bool {
    DIRECT_LANDY_PENALTY_PASS.with(Cell::get)
}

/// The advancer's action over partner's both-majors `X` (RHO passing, `(1NT) X -`)
///
/// The Landy advance ([`landy_advances`]) plus — when [`set_direct_landy_penalty_pass`]
/// is on — a **penalty pass**: with no major fit (both majors ≤2) and enough defense
/// (`points(22 - lo ..)`, so a stronger `X` asks less), pass and defend `1NTx` rather
/// than run to a 5-2 major.  Weight 1.25 beats the `2NT` game-ask (1.2) and the weak
/// signoffs for exactly these no-fit hands.  After the advancer's pass it is the
/// *opener's* turn, so a following opener pass ends the auction in `1NTx` (declared by
/// them, defended by us) — no doubler node is needed.
fn both_majors_x_advance(lo: u8) -> Rules {
    let base = landy_advances(lo);
    if direct_landy_penalty_pass() {
        let penalty = 22u8.saturating_sub(lo);
        base.rule(
            Call::Pass,
            125,
            len(Suit::Hearts, ..=2) & len(Suit::Spades, ..=2) & points(penalty..),
        )
    } else {
        base
    }
}

/// Both majors: at least 5-4 either way, or a flat 4-4 when `four_four`.  Both majors
/// four-plus, with the longer at least `4` (flat 4-4) or `5` (5-4) — the `and` floors
/// both, the `or` demands the length.
pub(super) fn both_majors_shape(four_four: bool) -> Cons<impl Constraint + Clone> {
    let longer = if four_four { 4 } else { 5 };
    and([Suit::Hearts, Suit::Spades], 4..) & or([Suit::Hearts, Suit::Spades], longer..)
}

/// Direct-Landy `X`: both majors (5-4, or flat 4-4 when configured), replacing the
/// 15+ penalty double; weight 1.9 beats the natural 2♥/2♠ so a both-majors hand
/// doubles rather than picking one major.
pub(super) fn landy_x() -> Rules {
    let four_four = direct_landy_double().unwrap_or(false);
    Rules::new().rule(
        Call::Double,
        190,
        both_majors_shape(four_four) & points(direct_landy_double_floor()..),
    )
}

/// Landy `2♣`: both majors, at least 5-4, on the shared two-suiter band
/// ([`woolsey_points`], coupled with Woolsey's identical `2♣`; see [`set_landy`]),
/// gauged as raw HCP or upgraded points per [`set_landy_hcp`].
pub(super) fn landy_2c() -> Rules {
    let (lo, hi) = woolsey_points();
    let shape = five_four(Suit::Hearts, Suit::Spades);
    if landy_use_hcp() {
        Rules::new().rule(Bid::new(2, Strain::Clubs), 190, shape & hcp(lo..=hi))
    } else {
        Rules::new().rule(Bid::new(2, Strain::Clubs), 190, shape & points(lo..=hi))
    }
}

/// Advancer's responses to partner's Landy `2♣` (both majors), per
/// [bridgebum](https://www.bridgebum.com/landy.php)
///
/// `2♦` = equal majors, weak (correct to the longer); `2♥`/`2♠` = preference
/// signoff; `2NT` = game-forcing ask; `3♥`/`3♠` = invitational with 4-card
/// support; `4♥`/`4♠` = to play game with a fit.  The invite/game point
/// thresholds track the `2♣` range — anchored so `lo = 10` reproduces bridgebum's
/// 10–12 invite / 12+ force — so a lighter overcall needs a stronger advancer to
/// reach the same game.
fn landy_advances(lo: u8) -> Rules {
    let invite = 20u8.saturating_sub(lo);
    let game = 22u8.saturating_sub(lo);

    let hearts_longer = at_least_as_long(Suit::Hearts, Suit::Spades);
    let spades_longer = longer_suit(Suit::Spades, Suit::Hearts);
    let equal_majors = equal_length("equal majors", Suit::Hearts, Suit::Spades);

    Rules::new()
        // Game with a known 4-card fit (preferred over the ask).
        .rule(
            Bid::new(4, Strain::Hearts),
            140,
            len(Suit::Hearts, 4..) & points(game..) & hearts_longer.clone(),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            140,
            len(Suit::Spades, 4..) & points(game..) & spades_longer.clone(),
        )
        // Game-forcing ask without a clear 4-card major.
        .rule(Bid::new(2, Strain::Notrump), 120, points(game..))
        // Invitational with 4-card support.
        .rule(
            Bid::new(3, Strain::Hearts),
            110,
            len(Suit::Hearts, 4..) & points(invite..game) & hearts_longer.clone(),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            110,
            len(Suit::Spades, 4..) & points(invite..game) & spades_longer.clone(),
        )
        // Weak: equal majors → 2♦ relay; else preference signoff.
        .rule(
            Bid::new(2, Strain::Diamonds),
            100,
            equal_majors & points(..invite),
        )
        .rule(
            Bid::new(2, Strain::Hearts),
            90,
            hearts_longer & points(..invite),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            90,
            spades_longer & points(..invite),
        )
}

/// Advancer's response to a *doubled* Landy `2♣` (`(1NT) 2♣ (X)`)
///
/// The opponents' Double is the stolen `2♣` Stayman, and their opener can sit for
/// `2♣` doubled with good clubs (the [`set_penalty_pass`] conversion) — a disaster
/// for us, since the Landy overcaller is both-majors / short-club.  The Double also
/// hands us an extra step (the Redouble), so we run a richer escape than over a pass:
///
/// - **Redouble** = equal majors, "you pick" — the relay the undoubled `2♦` was.
/// - **Pass** = a long club one-suiter: play `2♣` doubled (the doubler walked in).
/// - **`2♦`** = a long diamond one-suiter, natural and to play (the freed bid).
/// - **`2♥`/`2♠`** = the longer major (weak signoff), as over a pass.
/// - the strong arms (`4M` game, `2NT` game-ask, `3M` invite) are unchanged — the
///   Double buys no room above `2NT`.
///
/// A minor one-suiter (Pass / `2♦`) needs *both majors ≤2*: opposite the overcaller's
/// guaranteed 5-card major a 3-card major has an 8-card fit worth more than a doubled
/// minor, so those hands relay (Redouble) or sign off into the major instead.
///
/// [`set_penalty_pass`]: super::set_penalty_pass
fn landy_advances_over_double(lo: u8) -> Rules {
    let invite = 20u8.saturating_sub(lo);
    let game = 22u8.saturating_sub(lo);

    let hearts_longer = at_least_as_long(Suit::Hearts, Suit::Spades);
    let spades_longer = longer_suit(Suit::Spades, Suit::Hearts);
    let equal_majors = equal_length("equal majors", Suit::Hearts, Suit::Spades);
    // A long minor with both majors short (no 8-card fit opposite the overcaller's
    // 5-carder) outranks a major signoff. Gate A/B-tuned via set_doubled_landy_escape.
    let (min_minor, max_major) = doubled_landy_escape();
    let short_majors = len(Suit::Hearts, ..=max_major) & len(Suit::Spades, ..=max_major);

    Rules::new()
        // Strong arms — identical to the undoubled advance (no room gained above 2NT).
        .rule(
            Bid::new(4, Strain::Hearts),
            140,
            len(Suit::Hearts, 4..) & points(game..) & hearts_longer.clone(),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            140,
            len(Suit::Spades, 4..) & points(game..) & spades_longer.clone(),
        )
        .rule(Bid::new(2, Strain::Notrump), 120, points(game..))
        .rule(
            Bid::new(3, Strain::Hearts),
            110,
            len(Suit::Hearts, 4..) & points(invite..game) & hearts_longer.clone(),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            110,
            len(Suit::Spades, 4..) & points(invite..game) & spades_longer.clone(),
        )
        // Long club one-suiter, no major fit: sit for 2♣ doubled.
        .rule(
            Call::Pass,
            105,
            len(Suit::Clubs, min_minor..) & short_majors.clone(),
        )
        // Long diamond one-suiter, no major fit: natural 2♦, to play.
        .rule(
            Bid::new(2, Strain::Diamonds),
            100,
            len(Suit::Diamonds, min_minor..) & short_majors & points(..game),
        )
        // Equal majors: Redouble asks the overcaller to name the longer one.
        .rule(Call::Redouble, 95, equal_majors & points(..invite))
        .alert(LANDY_SOS)
        // Otherwise sign off in the longer major.
        .rule(
            Bid::new(2, Strain::Hearts),
            90,
            hearts_longer & points(..invite),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            90,
            spades_longer & points(..invite),
        )
}

/// Overcaller's rebid after advancer's *natural* `2♦` over the doubled Landy
/// (`(1NT) 2♣ (X) 2♦ -`): pass partner's diamonds, but with a singleton/void
/// diamond pull to the longer major (a 5-2 major fit beats a 6-1 diamond one).
fn landy_doubled_2d_rebid() -> Rules {
    let hearts_longer = at_least_as_long(Suit::Hearts, Suit::Spades);
    let spades_longer = longer_suit(Suit::Spades, Suit::Hearts);
    Rules::new()
        .rule(
            Bid::new(2, Strain::Hearts),
            100,
            len(Suit::Diamonds, ..=1) & hearts_longer,
        )
        .rule(
            Bid::new(2, Strain::Spades),
            100,
            len(Suit::Diamonds, ..=1) & spades_longer,
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Overcaller's rebid after the `2♦` relay (`(1NT) 2♣ - 2♦ -`): name the
/// longer major, so the equal-majors advancer plays the right strain
fn landy_2d_rebid() -> Rules {
    let hearts_longer = at_least_as_long(Suit::Hearts, Suit::Spades);
    let spades_longer = longer_suit(Suit::Spades, Suit::Hearts);
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 100, hearts_longer)
        .rule(Bid::new(2, Strain::Spades), 100, spades_longer)
}

/// A Pass-only node: settle, play the contract on the table.  Authoring this where
/// the instinct floor would otherwise run keeps a finite logit on `Pass`, so the
/// floor's over-competition is shadowed (see `project_floor_shadowed_by_book_nodes`).
pub(super) fn sit() -> Rules {
    Rules::new().rule(Call::Pass, 0, hcp(0..))
}

/// Advancer's runout after partner's both-majors `X` is **redoubled** (`(1NT) X (XX)`)
///
/// The redouble forces our side to act (sitting plays `1NTxx`), but it also frees a
/// clean structure: over the redoubled one-level `1NT` our `2♣` sits at the two level,
/// so the advancer has a *natural* rung for every suit.  **`Pass` = "ask back"** — no
/// suit of our own and no major preference, so the doubler names its longer (five-card)
/// major over the opponents' pass; **a bid (`2♣`/`2♦`/`2♥`/`2♠`, or `4♥`/`4♠`) = to
/// play** the natural suit.  No artificial `2♦` relay — that phantom diamond was what
/// let the floor run a doubled major into `3♦x` (the dominant DD leak); here the only
/// `2♦` is real diamonds, so a double of it is sat, not run from.
fn both_majors_x_runout(lo: u8) -> Rules {
    let game = 22u8.saturating_sub(lo);
    let hearts_longer = longer_suit(Suit::Hearts, Suit::Spades);
    let spades_longer = longer_suit(Suit::Spades, Suit::Hearts);
    let short_majors = len(Suit::Hearts, ..=2) & len(Suit::Spades, ..=2);
    Rules::new()
        // To-play game with a big fit in the preferred major.
        .rule(
            Bid::new(4, Strain::Hearts),
            140,
            len(Suit::Hearts, 4..) & points(game..) & hearts_longer.clone(),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            140,
            len(Suit::Spades, 4..) & points(game..) & spades_longer.clone(),
        )
        // Own long minor with no major fit → to play the minor.
        .rule(
            Bid::new(2, Strain::Clubs),
            110,
            len(Suit::Clubs, 5..) & short_majors.clone(),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            110,
            len(Suit::Diamonds, 5..) & short_majors,
        )
        // Major preference → to play.
        .rule(Bid::new(2, Strain::Spades), 100, spades_longer)
        .rule(Bid::new(2, Strain::Hearts), 100, hearts_longer)
        // Equal majors / nothing to say → ask: the doubler names its five-card major.
        .rule(Call::Pass, 50, hcp(0..))
}

/// Overcaller's rebid after the game-forcing `2NT` ask (`(1NT) 2♣ - 2NT -`)
///
/// The sourced min/med/max × 5-4/5-5 ladder, with the strength buckets tracking
/// the `2♣` range (partition `[lo, hi]` into thirds, `hi` capped at 16 when the
/// overcall is open-topped): a 5-5 hand shows `3♥`/`3♠`/`3NT` for min/medium/max;
/// a 5-4 hand shows `3♣` (min-or-medium) / `3♦` (max).
fn landy_2nt_rebid(lo: u8, hi: u8) -> Rules {
    let hi = hi.min(16);
    let step = hi.saturating_sub(lo) / 3;
    let med = lo + step;
    let max = lo + 2 * step;
    let five_five = len(Suit::Hearts, 5..) & len(Suit::Spades, 5..);

    Rules::new()
        // 5-5: 3♥ minimum, 3♠ medium, 3NT maximum.
        .rule(
            Bid::new(3, Strain::Hearts),
            130,
            five_five.clone() & points(lo..med),
        )
        .alert(LANDY_2NT_ANSWER)
        .rule(
            Bid::new(3, Strain::Spades),
            130,
            five_five.clone() & points(med..max),
        )
        .alert(LANDY_2NT_ANSWER)
        .rule(Bid::new(3, Strain::Notrump), 130, five_five & points(max..))
        .alert(LANDY_2NT_ANSWER)
        // 5-4 (the source omits a min-5-4 slot, so 3♣ folds min+medium together).
        .rule(Bid::new(3, Strain::Clubs), 120, points(lo..max))
        .alert(LANDY_2NT_ANSWER)
        .rule(Bid::new(3, Strain::Diamonds), 120, points(max..))
        .alert(LANDY_2NT_ANSWER)
}

/// Advancing partner's Landy `2♣` (both majors) over their `1NT`
///
/// Woolsey's `2♣` is the identical both-majors call on the same shared band,
/// so it reuses this same advance wiring.  The undoubled branch runs the
/// `2♦` "pick a major" relay and the `2NT` game-ask; over their double the
/// advancer runs the richer escape (Redouble = equal-majors relay, Pass =
/// clubs, `2♦` = natural, `2♥`/`2♠` = the longer major).
pub(super) fn landy_advance_package() -> Package {
    Package {
        name: "landy-advance",
        gate: || landy_range().is_some() || woolsey_enabled(),
        entries: || {
            let (lo, hi) = woolsey_points();
            [
                ("P* (1NT) 2♣ -", landy_advances(lo)),
                ("P* (1NT) 2♣ - 2♦ -", landy_2d_rebid()),
                ("P* (1NT) 2♣ - 2NT -", landy_2nt_rebid(lo, hi)),
                ("P* (1NT) 2♣ (X)", landy_advances_over_double(lo)),
                ("P* (1NT) 2♣ (X) XX -", landy_2d_rebid()),
                ("P* (1NT) 2♣ (X) 2♦ -", landy_doubled_2d_rebid()),
                ("P* (1NT) 2♣ (X) 2NT -", landy_2nt_rebid(lo, hi)),
            ]
            .into_iter()
            .flat_map(|(key, rules)| rows_of(Pattern::node(key), rules))
            .collect()
        },
    }
}

/// Direct-seat both-majors `X` advances
/// ([`set_direct_landy_double`][super::set_direct_landy_double])
///
/// The `X` is a Landy-style both-majors takeout double at every seat, so the
/// advancer answers exactly as over a Landy `2♣` — binding `(1NT) X -` is
/// correct here, the direct `X` is both-majors, not penalty.  The `2♦` relay
/// and the `2NT` ask are artificial, so the doubler answers them whether they
/// are passed **or** doubled; over their redouble the relay is dropped
/// entirely (`Pass` = ask back), which kills the phantom-`3♦` run.
pub(super) fn both_majors_double_package() -> Package {
    Package {
        name: "both-majors-double",
        gate: || direct_landy_double().is_some(),
        entries: || {
            // The advancer's invite/game thresholds track the X floor (a
            // stronger X asks less of the advancer), so read it here too.
            let (lo, hi) = (direct_landy_double_floor(), 37u8);
            let mut entries = Vec::new();
            for (key, rules) in [
                ("P* (1NT) X -", both_majors_x_advance(lo)),
                ("P* (1NT) X - 2♦ -", landy_2d_rebid()),
                ("P* (1NT) X - 2♦ (X)", landy_2d_rebid()),
                ("P* (1NT) X - 2NT -", landy_2nt_rebid(lo, hi)),
                ("P* (1NT) X - 2NT (X)", landy_2nt_rebid(lo, hi)),
                ("P* (1NT) X (XX)", both_majors_x_runout(lo)),
                ("P* (1NT) X (XX) - -", landy_2d_rebid()),
            ] {
                entries.extend(rows_of(Pattern::node(key), rules));
            }
            // …then the advancer SITS for that major whether it is passed or
            // doubled — play 2Mx (our real fit), never run.
            for m in ["2♥", "2♠"] {
                for after in ["-", "(X)"] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* (1NT) X (XX) - - {m} {after}")),
                        sit(),
                    ));
                }
            }
            // The undoubled branch keeps the 2♦ relay (Pass there defends 1NT,
            // so it cannot be the ask).  Once the doubler names its major over
            // the (possibly doubled) relay, SIT when the opponents double it —
            // the doubler plays 2Mx instead of running to the phantom 3♦.
            for relay in ["(X)", "-"] {
                for m in ["2♥", "2♠"] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* (1NT) X - 2♦ {relay} {m} (X) - -")),
                        sit(),
                    ));
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
