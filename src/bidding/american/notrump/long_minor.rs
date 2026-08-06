//! The long-minor force — `1NT – 3m` as a game force with a long minor
//!
//! Opt-in ([`set_long_minor_force`]); off by default, so the `3m` slot stays
//! with whichever minor scheme is armed.
use super::*;

/// The source-of-tricks eight's 3NT force — **opt-in, off by default (a measured
/// loss); kept only to drive the `ab-long-minor-force` A/B** (see
/// [`LONG_MINOR_FORCE`] for the numbers)
///
/// An 8-count with no four- or five-card major and a long *running* minor jumps to
/// 3NT.  Two shapes qualify: a 7+ card minor (length alone), or a 6-card minor
/// headed by two of the top three honors.  Weight 1.4 would outrank the minor
/// transfers (1.3); the shape is never `balanced()` (a 6+ suit rules it out), so it
/// never collides with the balanced-only size-ask or Puppet Stayman.  Natural 3NT —
/// no alert.  Empty when off, which is the default.
pub(super) fn long_minor_force_rule() -> Rules {
    if !long_minor_force() {
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

thread_local! {
    /// Whether a *source-of-tricks eight* forces 3NT over 1NT instead of
    /// transferring.  The hand: 8 HCP, no four- or five-card major (so it uses
    /// neither Stayman nor Jacoby), and a long minor that runs — a **7+ card
    /// minor**, or a **6-card minor headed by two of the top three honors**.
    ///
    /// **Off by default — measured a LOSS and kept only as an A/B instrument.**  An
    /// analytic screen (`probe-force-eight`, 16M deals) looked positive — forcing
    /// 3NT beat a *notrump* invite/pass by +0.2 to +0.5 IMPs/board — but that
    /// baseline is a fiction: these hands do not stop in notrump, they *transfer*,
    /// and the transfer reaches the suit game.  The live A/B against the real
    /// routing (`ab-long-minor-force`, 8M deals, plain DD, vul none) measured
    /// **−7.12 IMPs/fired** (club source −7.07: the `2♠` transfer drives to a
    /// *making 5♣* that 3NT throws away; diamond source is a wash — the `2NT`
    /// transfer already reaches 3NT).  So no shape forces; the transfer machinery
    /// bids these hands strictly better.  See [`set_long_minor_force`].
    static LONG_MINOR_FORCE: Cell<bool> = const { Cell::new(false) };
}

/// Author the source-of-tricks-eight 3NT force for books built *after* this call
/// (thread-local; **off by default — measured a loss**).
///
/// An 8-count with no four- or five-card major and a running long minor (7+ cards,
/// or a 6-card minor with two top honors) jumps to 3NT rather than transferring.
/// The transfer already reaches the better spot (5♣ game on club hands, 3NT on
/// diamond hands), so this *loses* (`examples/ab-long-minor-force` measured −7.12
/// IMPs/fired) — it exists only to re-run that A/B.
pub fn set_long_minor_force(on: bool) {
    LONG_MINOR_FORCE.with(|cell| cell.set(on));
}

/// Whether the source-of-tricks-eight 3NT force is currently authored
fn long_minor_force() -> bool {
    LONG_MINOR_FORCE.with(Cell::get)
}
