//! Responder's `X` and `Pass` over their overcall, and opener's answer
//!
//! [`DoubleStyle`] says what responder's double of their overcall means —
//! `Optional` (the shipped default), `Penalty`, or `Takeout` — and drives
//! opener's leave-in ([`set_penalty_double_leave_in`]) or cooperation.  The
//! penalty pass ([`set_penalty_pass`]) and the trap pass ([`set_trap_pass`])
//! are the two ways responder passes for value.
use super::*;

/// The meaning of responder's double of the overcall in `1NT (overcall) X`.
///
/// All variants are *authored* in the book (a finite logit), so the instinct
/// floor's own takeout double — whose `hcp(12..)` threshold is too strong here —
/// is shadowed and we control the strength. Opener's continuation is authored to
/// match the style: penalty → `opener_leaves_in_penalty_double` sits; optional →
/// `opener_cooperates_optional` stands on a fit and runs with a doubleton.
/// Gated behind [`set_double_style`]; [`DoubleStyle::Optional`] (2-3/8+) is the
/// default.
///
/// A/B verdict (`ab-lebensohl`, NS vs EW with both pairs Transfer, 200k,
/// ~1500 divergent), once **both** the doubler's partner *and* the takeout
/// baseline are handled fairly: **Optional > Penalty > Takeout**. Optional beats
/// penalty by **+1.59** and takeout by **+2.14 IMPs/divergent**; penalty beats
/// takeout by **+0.51**. The earlier penalty-vs-takeout disagreement (plain DD
/// favored takeout, perfect-defense favored penalty) was an **artifact of opener
/// pulling responder's penalty double** — once opener sits, both measures favor
/// penalty over takeout; once opener also *cooperates* with a 2-3-card optional
/// double (stand on a fit, run with a doubleton) optional wins outright. The
/// ranking is robust to the responder's-double reading. `Takeout`/`Penalty` stay
/// selectable for A/B.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DoubleStyle {
    /// Classic takeout, `len(over, ..=3) & hcp(8..)` (former default; best plain-DD
    /// double only while penalty doubles were pulled — see [`DoubleStyle`]).
    Takeout,
    /// Penalty — length and values in their suit, `len(over, 4..) & hcp(9..)`;
    /// opener sits (see [`set_penalty_double_leave_in`]).
    Penalty,
    /// Penalty at a lower floor: `len(over, 4..) & hcp(7..)`
    PenaltyLight,
    /// Default: cooperative / optional takeout, never short: `len(over, 2..=3) &
    /// hcp(8..)`; opener stands on a fit and runs with a doubleton (see
    /// `opener_cooperates_optional`).
    #[default]
    Optional,
}

thread_local! {
    /// The meaning of responder's double of the overcall (see [`DoubleStyle`]).
    static DOUBLE_STYLE: Cell<DoubleStyle> = const { Cell::new(DoubleStyle::Optional) };
}

/// Select responder's double meaning for books built *after* this call
/// (thread-local, read once at book-construction time)
pub fn set_double_style(style: DoubleStyle) {
    DOUBLE_STYLE.with(|cell| cell.set(style));
}

/// The currently selected double meaning
pub(super) fn double_style() -> DoubleStyle {
    DOUBLE_STYLE.with(Cell::get)
}

thread_local! {
    /// Whether opener leaves in responder's penalty double of a natural overcall of
    /// our 1NT (`1NT (2X) X -`) instead of letting the floor read `… X -` as a
    /// takeout advance and pull it. **On by default**; a no-op unless the active
    /// [`DoubleStyle`] is penalty. Read once at book construction. See
    /// [`set_penalty_double_leave_in`] — the A/B knob for the "opener pulls
    /// responder's penalty double" leak (the book dual of the penalty latch).
    static PENALTY_DOUBLE_LEAVE_IN: Cell<bool> = const { Cell::new(true) };
}

/// Toggle opener leaving in responder's penalty double of a natural overcall of our
/// 1NT, for books built *after* this call (thread-local; **on by default**)
///
/// Only matters when the active [`DoubleStyle`] is `Penalty`/`PenaltyLight`: opener
/// sits for `1NT (2X) X -` (defending the doubled overcall) rather than pulling
/// it, since responder's penalty double promised the trumps.  Off restores the bare
/// floor (which reads the double as takeout and advances).
pub fn set_penalty_double_leave_in(on: bool) {
    PENALTY_DOUBLE_LEAVE_IN.with(|cell| cell.set(on));
}

/// Whether opener's penalty-double leave-in is authored
pub(super) fn penalty_double_leave_in() -> bool {
    PENALTY_DOUBLE_LEAVE_IN.with(Cell::get)
}

/// Opener's reply to responder's **penalty** double of their overcall of our 1NT
/// (`1NT (2X) X -`): always sit and defend, since responder promised length and
/// values in their suit
///
/// A 3NT escape (opener-max with their suit stopped) was A/B'd a clear *loss* vs
/// always sitting (+0.328 vs +0.507 IMPs/divergent on `ab-lebensohl`): defending the
/// doubled overcall beats a fragile notrump game, especially when opener also holds
/// length in their suit — so opener never pulls.
///
/// The book dual of the penalty latch's leave-in: without an authored node here the
/// floor reads `… X -` as a takeout advance and *pulls* the penalty double (opener
/// is usually short in their suit, so its own length-gated leave-in never fires).
pub(super) fn opener_leaves_in_penalty_double() -> Rules {
    Rules::new().rule(Call::Pass, 150, hcp(0..))
}

/// Opener's reply to responder's **optional** (cooperative) double of their `over`
/// overcall of our 1NT (`1NT (2X) X -`): responder showed only 2-3 cards in
/// their suit, so opener *decides* — stand (defend) with a three-card-plus fit, but
/// **run with a doubleton** to a real five-card suit, escaping a thin defense
///
/// The floor would stand only with four-plus behind their suit and pull everything
/// else, so it runs the three-card fits opener should defend — the optional dual of
/// the penalty-double leak.  Without a five-card suit a short opener has nowhere to
/// run, so it sits (the catch-all `Pass`).
pub(super) fn opener_cooperates_optional(over: Suit) -> Rules {
    // Stand by default: a fit defends, and a short hand with no suit has no better.
    let mut rules = Rules::new().rule(Call::Pass, 150, hcp(0..));
    // Run with a doubleton-or-less to a real five-card suit (cheapest legal level).
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == over {
            continue;
        }
        let strain = Strain::from(suit);
        for level in 2..=3 {
            rules = rules.rule(
                Bid::new(level, strain),
                160,
                min_level_is(level, strain) & len(over, ..=2) & len(suit, 5..),
            );
        }
    }
    rules
}

thread_local! {
    /// Optional parametric override of responder's double as
    /// `(min_len, max_len, min_hcp)` in their suit, superseding [`DoubleStyle`]
    /// for A/B sweeps that tune the length/strength threshold directly. `None`
    /// (default) uses the named [`DoubleStyle`]. See [`set_double_override`].
    static DOUBLE_OVERRIDE: Cell<Option<(usize, usize, u8)>> = const { Cell::new(None) };
}

/// Override responder's double with an explicit `(min_len, max_len, min_hcp)` in
/// their suit (for books built *after* this call; thread-local). `None` restores
/// the named [`DoubleStyle`]. Lets an A/B sweep the penalty/takeout boundary as a
/// continuum instead of the four discrete styles.
pub fn set_double_override(spec: Option<(usize, usize, u8)>) {
    DOUBLE_OVERRIDE.with(|cell| cell.set(spec));
}

/// The `(min_len, max_len, hcp_floor)` override on responder's double, if any
pub(super) fn double_override() -> Option<(usize, usize, u8)> {
    DOUBLE_OVERRIDE.with(Cell::get)
}

/// Author responder's double of their `over` overcall per the active
/// [`DoubleStyle`] (or the [`set_double_override`] spec). Shadows the instinct
/// floor's takeout double so the threshold is the one chosen here.
pub(super) fn responder_double(rules: Rules, over: Suit, agreements: &Agreements) -> Rules {
    if let Some((lo, hi, floor)) = agreements.competition.double_override {
        return rules.rule(Call::Double, 155, len(over, lo..=hi) & hcp(floor..));
    }
    // The `len` ranges have distinct types, so author inside each arm.
    match agreements.competition.double_style {
        DoubleStyle::Takeout => rules.rule(Call::Double, 155, len(over, ..=3) & hcp(8..)),
        DoubleStyle::Penalty => rules.rule(Call::Double, 155, len(over, 4..) & hcp(9..)),
        DoubleStyle::PenaltyLight => rules.rule(Call::Double, 155, len(over, 4..) & hcp(7..)),
        DoubleStyle::Optional => rules.rule(Call::Double, 155, len(over, 2..=3) & hcp(8..)),
    }
}

thread_local! {
    /// Opener's penalty-pass over a `(2♣)` overcall, as
    /// `(min_club_len, min_club_hcp, convert_over_major)`. After `1NT (2♣) X -`
    /// — where the systems-on Double is the stolen `2♣` Stayman — opener with this
    /// club holding *passes* to defend `2♣` doubled instead of answering Stayman.
    /// `convert_over_major` decides whether good clubs outrank a `2♥`/`2♠` major
    /// fit (`true`) or yield to it (`false`).
    ///
    /// **Default `Some((4, 4, true))`:** 4+ clubs with 4+ club HCP (an ace or two
    /// honors sitting over the overcaller), converting even with a major fit. A/B'd
    /// a clear win at every gate tested (`landy-ab`, 2M, Landy off both arms):
    /// **+5.35/+7.28 IMPs/divergent (none/both) on plain DD, +5.32/+7.09 under
    /// perfect defense** — the conversion is a pure penalty decision, so the two
    /// scorers agree. `None` restores the prior flaw (opener could never convert).
    /// See [`set_penalty_pass`].
    static PENALTY_PASS: Cell<Option<(usize, u8, bool)>> =
        const { Cell::new(Some((4, 4, true))) };
}

/// Set opener's penalty-pass of the stolen-Stayman Double over a `(2♣)` overcall,
/// gated on `(min_club_len, min_club_hcp, convert_over_major)` (for books built
/// *after* this call; thread-local, read once at construction). `None` restores
/// the historic behaviour where opener can never convert. A looser gate captures
/// more total IMPs (every gate down to `(4, 0, true)` and even 3-card clubs stays
/// net positive on DD) at lower per-conversion quality; the default trades a
/// little frequency for a genuine "good clubs" holding. The A/B knob is
/// `landy-ab --ns-penalty-pass LEN:HCP[:major]`.
pub fn set_penalty_pass(spec: Option<(usize, u8, bool)>) {
    PENALTY_PASS.with(|cell| cell.set(spec));
}

/// Opener's currently selected penalty-pass gate over `(2♣)`
pub(super) fn penalty_pass() -> Option<(usize, u8, bool)> {
    PENALTY_PASS.with(Cell::get)
}

thread_local! {
    /// Whether responder *traps* with a too-good stopper: a direct `3NT`
    /// additionally denies **5+ HCP in the overcall suit**, so a strong holding
    /// (AQ, KQ, AKJ…) passes instead — waiting for opener to reopen with a takeout
    /// double and converting it to penalty. On by default. See [`set_trap_pass`].
    static TRAP_PASS: Cell<bool> = const { Cell::new(true) };
}

/// Enable the trap pass: with a too-good stopper (5+ HCP in their suit) responder
/// passes rather than declaring `3NT` (for books built *after* this call;
/// thread-local). Strong honors in the overcaller's suit defend better than they
/// declare — sit, let opener reopen with a takeout double, and convert to penalty.
///
/// The `5`-HCP threshold is **distilled from a per-board double-dummy oracle**
/// (`lebensohl-ab --pd-3nt --log-relay`): comparing `3NT` against trapping over
/// sampled layouts, the trap rate rises monotonically with HCP *in their suit*
/// (hcp 4 → 53%, 5 → 77%, 6+ → ~100%) and is **independent of length** — a long
/// weak holding (e.g. ♠A9642, 4 HCP) is a running source that wants `3NT`, while a
/// short strong one (♥AQ, 6 HCP) defends. The earlier length-based gate (4+ cards)
/// got this backwards and lost; this honor gate is the fix. **On by default**
/// (A/B vs off, isolated, 200k plain DD: the 1NT-Lebensohl responder gains
/// `+172`/`+185` IMPs — the original `resp 3NT` losers, −22/−20, are erased — at a
/// near-wash in the shared advance-of-takeout-double context; net `+155`/`+230`).
pub fn set_trap_pass(on: bool) {
    TRAP_PASS.with(|cell| cell.set(on));
}

/// Whether responder traps (passes) with a too-good stopper instead of `3NT`
pub(super) fn trap_pass() -> bool {
    TRAP_PASS.with(Cell::get)
}

#[cfg(test)]
mod tests;
