//! The `2NT` size ask over a maximum 1NT, and its eight-count seam
//!
//! [`SizeAskEight`] chooses what a *shipped* eight-count does opposite the ask;
//! [`set_size_ask_accept_floor`] sets the HCP at which opener accepts.
use super::*;

/// How a balanced eight with no four-card major responds to our 1NT — the
/// *size ask* class (`hcp(8) & balanced() & no four-card major`).
///
/// The shipped default routes the flat 4-3-3-3 subset to Pass (it plays a level
/// too high — a shape with no ruff and no long suit is its high cards and nothing
/// more) and lets the shapelier eights invite via the `2♠`/`2NT` size ask.  The
/// two poles ([`Invite`][SizeAskEight::Invite], [`Pass`][SizeAskEight::Pass])
/// exist so a measurement harness can price inviting-the-whole-class against
/// passing-it under a realistic scorer — the flat-4333 carve was decided on plain
/// double dummy, which is level-dependently pessimistic on the low contracts in
/// play (very on 1NT, slightly on 3NT).  See [`set_size_ask_eight`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SizeAskEight {
    /// Flat 4-3-3-3 passes, shapelier eights size-ask — the crate default.
    #[default]
    Shipped,
    /// The whole class size-asks (the pre-2026-07-03 behaviour).
    Invite,
    /// The whole class passes.
    Pass,
}

thread_local! {
    /// Routing of the size-ask eight, read once at book-construction time.
    /// [`SizeAskEight::Shipped`] by default; see [`set_size_ask_eight`].
    static SIZE_ASK_EIGHT: Cell<SizeAskEight> = const { Cell::new(SizeAskEight::Shipped) };
}

/// Route the size-ask eight (balanced 8, no four-card major) for books built
/// *after* this call (thread-local; [`SizeAskEight::Shipped`] by default)
///
/// A throwaway measurement knob: `Invite` un-suppresses the flat 4-3-3-3 eight so
/// the whole class size-asks, `Pass` sends the whole class to Pass.  Comparing the
/// two poles under the single-dummy perfect-defense scorer
/// ([`ns_score_pd_tricks`][crate::scoring::ns_score_pd_tricks]) re-prices the
/// flat-4333 carve — and the still-live shapelier eights — with realistic tricks
/// instead of double dummy.  The default is byte-identical to the shipped system.
pub fn set_size_ask_eight(mode: SizeAskEight) {
    SIZE_ASK_EIGHT.with(|cell| cell.set(mode));
}

/// The active size-ask-eight routing, defaulting to [`SizeAskEight::Shipped`]
pub(super) fn size_ask_eight() -> SizeAskEight {
    SIZE_ASK_EIGHT.with(Cell::get)
}

/// The size-ask eight class: a balanced eight with no four-card major — the
/// population the `2♠`/`2NT` size ask and the flat-4333 Pass carve both key on.
pub(super) fn size_ask_eight_class() -> Cons<impl Constraint + Clone> {
    hcp(8..=8) & balanced() & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4)
}

/// The Pass rule for a 1NT response, gated on [`size_ask_eight`].
///
/// The 0-7 pass is unconditional.  The eight's pass leg is knob-dependent:
/// `Shipped`/`Invite` keep the shipped `hcp(8) & flat_4333()` leg **verbatim** (so
/// the default is byte-identical, and a flat 4-3-3-3 with a four-card *major* —
/// which can neither Stayman nor size-ask — always passes); the higher-weight
/// `2♠`/`2NT` size-ask rule outranks this Pass where they overlap, so widening the
/// size ask in the `Invite` arm reroutes the flat-4-minor eight without touching
/// this rule.  The `Pass` arm additionally routes the whole class here so the
/// non-flat eights, whose size-ask rule is dropped, have a home.
pub(super) fn size_ask_eight_pass() -> Rules {
    let base = hcp(..8) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5);
    match size_ask_eight() {
        SizeAskEight::Shipped | SizeAskEight::Invite => {
            Rules::new().rule(Call::Pass, 0, base | (hcp(8..=8) & flat_4333()))
        }
        SizeAskEight::Pass => Rules::new().rule(
            Call::Pass,
            0,
            base | (hcp(8..=8) & flat_4333()) | size_ask_eight_class(),
        ),
    }
}

thread_local! {
    /// Opener's HCP floor to *accept* the balanced-eight size ask, read once at
    /// book-construction time.  `16` by default; see [`set_size_ask_accept_floor`].
    static SIZE_ASK_ACCEPT_FLOOR: Cell<u8> = const { Cell::new(16) };
}

/// Set opener's HCP floor for accepting the balanced-eight size-ask invite for
/// books built *after* this call (thread-local; **default 16**)
///
/// Over the size ask (`2♠` Puppet max-signal `3♣`, or European `2NT`→`3NT`), a
/// maximum accepts game and a minimum declines to `2NT`.  Shipped floor is **16**
/// (accept the 24-HCP game): the earlier double-dummy probe rejected accept-16,
/// but DD over-punishes the accepted `3NT` with doubled failures a realistic
/// blind lead dodges.  Re-priced under the single-dummy perfect-defense scorer
/// ([`ns_score_pd_tricks`][crate::scoring::ns_score_pd_tricks]) accept-16 wins
/// both vulnerabilities (SD-PD −0.85 NV / −2.16 vul IMPs/divergent, plain-DD
/// non-negative), so 16 is default-on.  In the Puppet scheme this signal is
/// shared with the club one-suiter; the club buckets measured benign.  Raise to
/// 17 to restore the pre-2026-07-25 conservative accept.
///
/// The meaningful sweep is `{15, 16, 17}` (opener's 1NT range).  Floor 15 is a
/// **falsification control**, not a shippable setting: accepting on `15 + 8 = 23`
/// turns the invite into a game force, so 15 *should* decline — the harness is
/// expected to price accept-15 as a clear loss, and a measured *win* there is a
/// warning that the scorer (not the treatment) is wrong.  It priced accept-15 at
/// SD-PD +1.11 NV / +0.99 vul (a clear decline), validating the scorer.
pub fn set_size_ask_accept_floor(floor: u8) {
    SIZE_ASK_ACCEPT_FLOOR.with(|cell| cell.set(floor));
}

/// Opener's current accept floor for the balanced-eight size ask (default 16)
pub(super) fn size_ask_accept_floor() -> u8 {
    SIZE_ASK_ACCEPT_FLOOR.with(Cell::get)
}
