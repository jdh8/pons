use crate::bidding::Rules;
use crate::bidding::agreements::{Agreements, ResponseKnobs};
use crate::bidding::constraint::{Cons, Constraint, described, len, points};
use crate::bidding::context::Context;
use contract_bridge::{Bid, Hand, Strain, Suit};
use std::cell::Cell;

std::thread_local! {
    /// Whether minor-opening responses pick the **longer major** (equal
    /// lengths: 4-4 up the line to `1♥`, 5-5+ higher-first to `1♠`) instead of
    /// unconditional hearts-first.  Default `true` — the established American
    /// treatment (bid the longer major on 5♠4♥), which measured a null
    /// (`ab-minor-continuations`, 2M boards: plain-DD wash, PD −0.12/−0.22 per
    /// divergent NV/vul; −0.003..−0.005 IMPs/board marginal on the shipped xyz
    /// + up-the-line package).  A push against a natural default goes to the
    /// natural method — the **naturalness tiebreak** (`docs/measurement.md`);
    /// the historic unconditional-hearts-first simplification is the opt-in
    /// (turn this knob *off*).
    static LONGER_MAJOR_RESPONSE: Cell<bool> = const { Cell::new(true) };
}

std::thread_local! {
    /// Whether the natural minor-opening tree is completed **up the line**:
    /// the `1♣ - 1♦` response, opener's `1♠` rebid over `1m - 1♥`, and
    /// opener's natural `2♣` rebid after `1♣ - 1♦`.  Default `true`, shipped
    /// **jointly with XYZ** (`ab-minor-continuations`, 300k boards, with
    /// `set_xyz`: plain +0.0382/+0.0559 IMPs/board NV/vul, PD
    /// +0.0289/+0.0407).  Alone it is a measured **loss** (plain
    /// −0.91/−1.28 per divergent) — the 1♦ response reroutes hands into
    /// auctions only the XYZ round continues; don't enable it with XYZ off.
    static UP_THE_LINE: Cell<bool> = const { Cell::new(true) };
}

/// Author the longer-major response discipline for books built *after* this
/// call (default `true`; off-switch `--no-ns-longer-major-response` in
/// `bba-gen`)
///
/// On (the default): a response to `1♣`/`1♦` names the longer major — `1♠` on
/// 5♠4♥ or any 5-5+, `1♥` up the line only on 4-4 — so partner can infer
/// "spades are not longer than hearts" from `1♥`.  The M6.4 control-bid
/// classifier reads the same discipline at classify time (`classify_high_bid`
/// in `inference.rs`): the response rule, the rebid structure, and the
/// classifier move together (see `docs/bidding-theorems.md`).  Off: the
/// historic unconditional hearts-first pair — measured a null against
/// longer-major, so the naturalness tiebreak (`docs/measurement.md`) keeps the
/// established American treatment as the default.
pub fn set_longer_major_response(on: bool) {
    LONGER_MAJOR_RESPONSE.with(|cell| cell.set(on));
}

/// Whether the longer-major response discipline is active (also read by the
/// inference engine at classify time)
pub(crate) fn longer_major_response() -> bool {
    LONGER_MAJOR_RESPONSE.with(Cell::get)
}

/// Author the up-the-line completion of the natural minor tree for books
/// built *after* this call (default `true`; off-switch `--no-ns-up-the-line`
/// in `bba-gen`)
///
/// On: responder bids `1♦` over `1♣` on four-plus diamonds without a
/// four-card major (off, those hands squeeze into the notrump ladder or fall
/// to the floor), opener rebids `1♠` over `1m - 1♥` on four spades (off, the
/// 4-4 spade fit is lost to a 1NT rebid), and opener rebids a natural `2♣`
/// after `1♣ - 1♦` on six-plus clubs (off, a misdescribed 1NT catch-all).
///
/// Shipped **jointly with [`set_xyz`][super::super::set_xyz]**: the 1♦ response only
/// pays once responder's second round has the XYZ machinery (alone it
/// measured plain −0.91/−1.28 per divergent).
pub fn set_up_the_line(on: bool) {
    UP_THE_LINE.with(|cell| cell.set(on));
}

/// Whether the up-the-line completion is currently authored
pub(crate) fn up_the_line() -> bool {
    UP_THE_LINE.with(Cell::get)
}

/// Spades take the first response: strictly longer, or equal length five-plus
///
/// The longer-major discipline's selector — 5-5 responds `1♠` planning to
/// show hearts next; 4-4 responds `1♥` up the line.
fn spades_first() -> Cons<impl Constraint + Clone> {
    described(
        "spades longer than hearts, or equal five-plus",
        |hand: Hand, _: &Context<'_>| {
            spades_take_first(hand[Suit::Spades].len(), hand[Suit::Hearts].len())
        },
    )
}

/// The [`spades_first`] predicate on the two major lengths alone
fn spades_take_first(spades: usize, hearts: usize) -> bool {
    spades > hearts || (spades == hearts && spades >= 5)
}

/// Hearts take the first response: strictly longer, or equal length below five
///
/// The exact complement of [`spades_first`] — the 1♥ response fires precisely
/// when spades do not. Phrased positively so the book renders "hearts longer
/// than spades, or equal below five" rather than a negated `spades_first`.
fn hearts_first() -> Cons<impl Constraint + Clone> {
    described(
        "hearts longer than spades, or equal below five",
        |hand: Hand, _: &Context<'_>| {
            hearts_take_first(hand[Suit::Spades].len(), hand[Suit::Hearts].len())
        },
    )
}

/// The [`hearts_first`] predicate on the two major lengths alone
fn hearts_take_first(spades: usize, hearts: usize) -> bool {
    hearts > spades || (hearts == spades && spades < 5)
}

pub(super) fn with_major_selection(rules: Rules, agreements: &Agreements) -> Rules {
    let mut rules = rules;
    // Major selection between 4+ majors, per the longer-major knob (default on).
    rules = if agreements.decision.reading.longer_major_response() {
        // Longer-major discipline (the default, `set_longer_major_response`): the response
        // names the longer major — 1♠ on 5♠4♥/6♠5♥ or any 5-5+, 1♥ up the
        // line only on 4-4 — so 1♥ denies longer spades and the M6.4
        // control-bid classifier can read `1♣ - 1♥ - 2♣ - 4♠` as a control bid.
        rules
            .rule(
                Bid::new(1, Strain::Spades),
                150,
                len(Suit::Spades, 4..) & points(6..) & spades_first(),
            )
            .rule(
                Bid::new(1, Strain::Hearts),
                140,
                len(Suit::Hearts, 4..) & points(6..) & hearts_first(),
            )
    } else {
        // Opt-in pair (`set_longer_major_response(false)`) — unconditional
        // hearts-first: any four-plus hearts responds 1♥ even with longer
        // spades (5♠4♥, 6♠5♥), so partner can only infer "1♠ denies four
        // hearts", never the converse, and the M6.4 classifier must read a
        // later jump into the suit *above* the response as natural to play (the
        // first M6.4 A/B round assumed longest-first here and lost 6 IMPs per
        // fired board).  This simplification measured a null against the
        // longer-major default and stays available as a knob; see
        // `set_longer_major_response` and `docs/bidding-theorems.md`.
        rules
            .rule(
                Bid::new(1, Strain::Hearts),
                150,
                len(Suit::Hearts, 4..) & points(6..),
            )
            .rule(
                Bid::new(1, Strain::Spades),
                140,
                len(Suit::Spades, 4..) & points(6..) & len(Suit::Hearts, ..4),
            )
    };
    rules
}

pub(super) fn with_up_the_line(rules: Rules, minor: Suit, knobs: &ResponseKnobs) -> Rules {
    let mut rules = rules;
    // Up-the-line completion (`set_up_the_line`): a natural 1♦ over 1♣ on
    // four-plus diamonds without a four-card major.  Weight 1.2 sits below
    // the majors (1.5/1.4) and the inverted raise (1.25), above the notrump
    // ladder (1.0) — so diamond hands stop mislabeling themselves as
    // balanced notrump responses or falling to the floor.
    if minor == Suit::Clubs && knobs.up_the_line {
        rules = rules.rule(
            Bid::new(1, Strain::Diamonds),
            120,
            len(Suit::Diamonds, 4..)
                & points(6..)
                & len(Suit::Hearts, ..4)
                & len(Suit::Spades, ..4),
        );
    }
    rules
}

#[cfg(test)]
mod tests;
