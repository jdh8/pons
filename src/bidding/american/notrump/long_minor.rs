//! The long-minor force — `1NT - 3m` as a game force with a long minor
//!
//! Opt-in ([`NotrumpKnobs::long_minor_force`][crate::bidding::agreements::NotrumpKnobs::long_minor_force]); off by default, so the `3m` slot stays
//! with whichever minor scheme is armed.
use super::*;

/// The source-of-tricks eight's 3NT force — **opt-in, off by default (a measured
/// loss); kept only to drive the `ab-long-minor-force` A/B** (see
/// [`NotrumpKnobs::long_minor_force`][crate::bidding::agreements::NotrumpKnobs::long_minor_force] for the numbers)
///
/// An 8-count with no four- or five-card major and a long *running* minor jumps to
/// 3NT.  Two shapes qualify: a 7+ card minor (length alone), or a 6-card minor
/// headed by two of the top three honors.  Weight 1.4 would outrank the minor
/// transfers (1.3); the shape is never `balanced()` (a 6+ suit rules it out), so it
/// never collides with the balanced-only size-ask or Puppet Stayman.  Natural 3NT —
/// no alert.  Empty when off, which is the default.
pub(super) fn long_minor_force_rule(agreements: &Agreements) -> Rules {
    if !agreements.notrump.long_minor_force {
        return Rules::new();
    }
    Rules::new().rule(
        Bid::new(3, Strain::Notrump),
        140,
        hcp(8..=8)
            & len(Suit::Hearts, ..4)
            & len(Suit::Spades, ..4)
            & ((len(Suit::Clubs, 6..) & (len(Suit::Clubs, 7..) | top_honors(Suit::Clubs, 2..)))
                | (len(Suit::Diamonds, 6..)
                    & (len(Suit::Diamonds, 7..) | top_honors(Suit::Diamonds, 2..)))),
    )
}
