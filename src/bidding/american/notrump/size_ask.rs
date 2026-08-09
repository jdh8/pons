//! The `2NT` size ask over a maximum 1NT, and its eight-count seam
//!
//! [`SizeAskEight`] chooses what a *shipped* eight-count does opposite the ask;
//! [`NotrumpKnobs::size_ask_accept_floor`][crate::bidding::agreements::NotrumpKnobs::size_ask_accept_floor] sets the HCP at which opener accepts.
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
/// play (very on 1NT, slightly on 3NT).  See [`NotrumpKnobs::size_ask_eight`][crate::bidding::agreements::NotrumpKnobs::size_ask_eight].
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
pub(super) fn size_ask_eight_pass(agreements: &Agreements) -> Rules {
    let base = hcp(..8) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5);
    match agreements.notrump.size_ask_eight {
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
