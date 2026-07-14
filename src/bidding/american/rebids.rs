//! Opener's rebids (one round) and the forcing-1NT continuations

use super::{call, insert_uncontested};
use crate::bidding::constraint::{
    balanced, fifths, hcp, len, partner_suit_is, points, stopper_in, support,
};
use crate::bidding::{Alert, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};
use std::cell::Cell;

// ponytail: construction-time toggle, read during `register()`; set it before
// building the `Pair`.  A per-classify flag (like `set_fifths_companion`) would
// not work — the adjunct changes which *nodes exist*, baked once at build time.
std::thread_local! {
    /// Whether opener's rebid tables carry the **complete Meckstroth adjunct**:
    /// the artificial game-forcing `2NT` (18+, any shape) with its `3♣`-relay
    /// shape-outs, *and* the invitational `3m` jumps (`1M – 1NT – 3m` and
    /// `1♥ – 1♠ – 3m`).  On by default; both feature sets ship on together.
    static MECKSTROTH: Cell<bool> = const { Cell::new(true) };
}

/// Enable the complete Meckstroth adjunct in books built *after* this call
/// (default **on**)
///
/// After `1M – 1NT` (the forcing notrump), opener's `2NT` is an artificial 18+
/// game force of *any* shape (responder relays `3♣`, opener shape-describes
/// toward game or slam) instead of the natural 18–19 balanced rebid; opener also
/// has the invitational `3m` jumps (5+ minor, 15–17).  Read at book-construction
/// time; set it before building the `Pair` (the `ab-meckstroth-2nt` A/B builds a
/// baseline arm with it off).
///
/// Shipped **on**.  The artificial `2NT` measured a plain-DD win
/// (`ab-meckstroth-2nt`, 200k×2 seeds: plain +0.0075/+0.013, PD +0.006/+0.011,
/// sd-lead +0.010/+0.017 NV/vul, all CI-clean); the `3m` jumps are sd-vindicated
/// (plain wash, PD over-punished, sd-lead +0.0012/+0.0042 NV/vul).
pub fn set_meckstroth_adjunct(on: bool) {
    MECKSTROTH.with(|cell| cell.set(on));
}

/// Whether the Meckstroth adjunct is currently enabled
fn meckstroth() -> bool {
    MECKSTROTH.with(Cell::get)
}

// ponytail: same construction-time toggle as the Meckstroth adjunct above.
std::thread_local! {
    /// Whether opener shows an invitational (15–17) major two-suiter after the
    /// forcing `1NT`: the `1♥ – 1NT – 2♠` reverse (5+ hearts, 4+ spades) and the
    /// `1♠ – 1NT – 3♥` jump (5-5 majors).  Fills the seam between the minimum
    /// natural rebids and the 18+ game force (`set_meckstroth_adjunct`).  Shipped
    /// **on**, sd-vindicated (`ab-forcing-nt-two-suiter`, 1M×2 seeds×2 vuls):
    /// plain wash-NV/+0.0012-vul, PD −0.0017/−0.0010 (over-punished), sd-lead
    /// **+0.0012/+0.0028** NV/vul — all four sd cells CI-clean positive.
    static FORCING_NT_TWO_SUITER: Cell<bool> = const { Cell::new(true) };
}

/// Enable opener's invitational major two-suiter rebids after the forcing `1NT`
/// in books built after this call (default **on**)
///
/// Over the forcing 1NT, opener with 15–17 and a second major suit has no
/// invitational rebid — a 5-4 or 5-5 hand underbids as a minimum natural call.
/// This adds `1♥ – 1NT – 2♠` (reverse: 5+ hearts, 4+ spades) and
/// `1♠ – 1NT – 3♥` (jump: 5-5 majors), both 15–17, with responder's
/// continuations.  Read at book-construction time; set it before building the
/// `Pair` (the `ab-forcing-nt-two-suiter` A/B builds a baseline arm with it off).
pub fn set_forcing_nt_two_suiter(on: bool) {
    FORCING_NT_TWO_SUITER.with(|cell| cell.set(on));
}

/// Whether opener's invitational major two-suiter rebids are enabled
fn forcing_nt_two_suiter() -> bool {
    FORCING_NT_TWO_SUITER.with(Cell::get)
}

// ponytail: same construction-time toggle as the Meckstroth adjunct — read
// during `register()`, so set it before building the `Pair`.
std::thread_local! {
    /// Whether opener rebids `1NT` (not `2m`) with a balanced 12–14 and a
    /// five-card minor after `1m – 1M`.  On by default (shipped);
    /// see [`set_balanced_1nt_rebid`].
    static BALANCED_1NT_REBID: Cell<bool> = const { Cell::new(true) };
}

/// Prefer opener's `1NT` rebid over `2m` on a balanced 12–14 in books built
/// *after* this call
///
/// After `1m – 1M`, a 5332 balanced minimum with the five-card minor otherwise
/// rebids a natural `2m` (weight 0.9) that outranks the balanced `1NT` (0.5),
/// misdescribing the hand and losing the `1NT`-based game placement BBA finds
/// (the largest lever in the Constructive/book/round-2 anchor bucket).  Read at
/// book-construction time; shipped default-on (+0.0093 plain / +0.0101 PD
/// IMPs/board vs BBA, both vuls).
///
/// Natural and folded into base per [docs/bidding-options.md]; retained only
/// as a measurement off-switch, not a user-facing toggle (dropped from the
/// `web` settings registry).
pub fn set_balanced_1nt_rebid(on: bool) {
    BALANCED_1NT_REBID.with(|cell| cell.set(on));
}

/// Whether the balanced-`1NT`-rebid preference is currently enabled
fn balanced_1nt_rebid() -> bool {
    BALANCED_1NT_REBID.with(Cell::get)
}

// ponytail: same construction-time toggle idiom as the Meckstroth adjunct —
// read during `register()`, set it before building the `Pair`.
std::thread_local! {
    /// Whether opener's rebid tables carry the **strength-showing ladder**
    /// after a minor opening and a one-level response: a jump-rebid of opener's
    /// suit, a reverse, and a jump-shift.  Shipped **on** (BBA-gap bucket #3).
    static OPENER_EXTRAS_LADDER: Cell<bool> = const { Cell::new(true) };
}

/// Enable opener's strength-showing rebid ladder in books built after this call
///
/// After a one-level response, opener's only long-suit rebid is a minimum
/// natural `2m`/`2M` with no upper bound (weight 0.9, `len(..5..)`), so a strong
/// single- or two-suiter underbids and the auction dies below game — the
/// largest un-worked lever in the Constructive/book/round-2 anchor bucket.  This
/// adds three rungs above the minimum, disjoint from it by crisp point bands:
///
/// - **Jump-rebid** of opener's suit (`1♦ – 1♠ – 3♦`): a self-sufficient 6+
///   suit, 16+ points, invitational.
/// - **Reverse** into a higher new suit (`1♦ – 1♠ – 2♥`): 5+ first suit, 4+
///   second, 17+ points, forcing.
/// - **Jump-shift** into a new suit (`1♦ – 1♠ – 3♣`): 5-4, 18+ points,
///   game-forcing.
///
/// Read at book-construction time; shipped default-on (+0.0203/+0.0332 plain,
/// +0.0181/+0.0297 PD IMPs/board vs BBA, NV/vul, all CIs>0).  The matching
/// [`Inferences`](crate::bidding::inference) reading gates on the same toggle,
/// narrowing each rung's shape and strength.  The two minor-opening rebid nodes
/// carry the full ladder; the major-opening nodes carry the jump-rebid rung
/// alone (see [`set_opener_major_jump_rebid`]).
pub fn set_opener_extras_ladder(on: bool) {
    OPENER_EXTRAS_LADDER.with(|cell| cell.set(on));
}

/// Whether opener's strength-showing rebid ladder is currently enabled
///
/// Read at book-construction time by `register`, and at classify time by the
/// matching `Inferences` reading (mirrors `rule_of_20_enabled`).
pub(crate) fn opener_extras_ladder() -> bool {
    OPENER_EXTRAS_LADDER.with(Cell::get)
}

// ponytail: same construction-time toggle idiom as the extras ladder above.
std::thread_local! {
    /// Whether opener's major-opening rebid nodes carry the jump-rebid rung of
    /// a six-card major with extras (`1♥ – 1♠ – 3♥`, `1M – 1NT – 3M`) and
    /// responder's continuation over it.  Shipped **on** (BBA-gap bucket #3
    /// residual); see [`set_opener_major_jump_rebid`].
    static OPENER_MAJOR_JUMP_REBID: Cell<bool> = const { Cell::new(true) };
}

/// Enable opener's major jump-rebid rung in books built after this call
///
/// The [extras ladder](set_opener_extras_ladder) covers only the two
/// minor-opening rebid nodes; the major-opening nodes (`1♥ – 1♠` and the
/// forcing-`1NT` rebid) still cap opener's own-major rebid at a minimum `2M`
/// with no upper bound, so a 16+ hand with a strong six-card major underbids
/// and misses the game BBA reaches (the `6+ ♥`/`6+ ♠` residual in the
/// Constructive/book/round-2 anchor bucket — `3♥ → 4♥`, `2♥ → 3♥`, `3♠ → 4♠`).
///
/// This adds the single jump-rebid `3M` (6+ suit, 16+ points), disjoint from
/// the `2M` minimum by a crisp point band, **plus responder's continuation**
/// (`responder_after_major_jump_rebid`: raise `4M` on an 8-card fit, `3NT` with
/// no fit, pass with a minimum).  It is the deferred major-opening half of the
/// extras ladder, scoped to opener's *own* suit to avoid the Meckstroth `3m`
/// collision on the jump-shift-into-a-minor rung.  Natural (names opener's own
/// suit), so unalerted and floor-safe; the matching
/// [`Inferences`](crate::bidding::inference) reading gates on the same toggle.
///
/// Read at book-construction time; shipped default-on (+0.0059/+0.0125 plain,
/// +0.0046/+0.0104 PD IMPs/board vs BBA, NV/vul, all CIs>0).  The bare rung
/// *without* the continuation measured a loss (−0.005/−0.009 plain: responder
/// passed the invitational `3M` and stranded below game) — authoring both sides
/// flipped it to a win.
pub fn set_opener_major_jump_rebid(on: bool) {
    OPENER_MAJOR_JUMP_REBID.with(|cell| cell.set(on));
}

/// Whether opener's major jump-rebid rung is currently enabled
///
/// Read at book-construction time by `register`, and at classify time by the
/// matching `Inferences` reading.
pub(crate) fn opener_major_jump_rebid() -> bool {
    OPENER_MAJOR_JUMP_REBID.with(Cell::get)
}

/// Append opener's jump-rebid of a six-card major with extras
///
/// `major` is opener's opened suit and `highest` responder's call.  The jump
/// `3M` sits above the `2M` minimum by weight, so only a 16+ hand takes it.
/// Gated on [`set_opener_major_jump_rebid`].
fn with_major_jump_rebid(rules: Rules, major: Suit, highest: Bid) -> Rules {
    if !opener_major_jump_rebid() {
        return rules;
    }
    let trump = Strain::from(major);
    let level = cheapest_level_over(highest, trump) + 1;
    rules.rule(Bid::new(level, trump), 1.5, len(major, 6..) & points(16..))
}

/// The cheapest level at which `strain` may be bid over `highest`
fn cheapest_level_over(highest: Bid, strain: Strain) -> u8 {
    if strain > highest.strain {
        highest.level.get()
    } else {
        highest.level.get() + 1
    }
}

/// Opener's reverse — a higher new suit showing a five-card first suit and extras
const OPENER_REVERSE: Alert = Alert("opener-reverse");
/// Opener's jump-shift — a new suit showing a big two-suiter, game-forcing
const OPENER_JUMP_SHIFT: Alert = Alert("opener-jump-shift");

/// Opener's artificial game-forcing `2NT` — 18+, any shape (real Meckstroth adjunct)
const OPENER_GF_2NT: Alert = Alert("meckstroth-2nt");
/// Responder's `3♣` relay over the game-forcing `2NT` — "describe"
const PUPPET_2NT: Alert = Alert("meckstroth-2nt-relay");
/// Responder's `3NT` over the game-forcing `2NT` — 5+ clubs, doubleton in opener's major
const RESP_CLUBS_2NT: Alert = Alert("meckstroth-2nt-clubs");
/// Opener's `3♦` default shape-out — balanced 18–19 or a four-card minor
const GF_DEFAULT: Alert = Alert("meckstroth-2nt-default");
/// Opener's `3NT` shape-out — five-plus a minor
const GF_MINOR: Alert = Alert("meckstroth-2nt-minor");

/// Append opener's strength-showing ladder to a one-level-response rebid table
///
/// `opener` is opener's opened suit, `highest` responder's one-level call, and
/// `responder` responder's suit when they bid one (a forcing `1NT` bids none).
/// The weights sit above the minimum natural rebid (0.9) but below the
/// support-raises (1.8+), so a hand with four-card support for responder still
/// raises and only a genuine extras hand takes the ladder; the crisp point
/// bands keep a minimum on the natural rebid.  All three rungs name a real
/// suit — natural, unalerted, floor-safe — so the reading (`inference.rs`)
/// narrows their shape and strength rather than an alert projecting it.
fn with_extras_ladder(
    mut rules: Rules,
    opener: Suit,
    highest: Bid,
    responder: Option<Suit>,
) -> Rules {
    if !opener_extras_ladder() {
        return rules;
    }
    let opener_strain = Strain::from(opener);
    // Jump-rebid of opener's suit: a self-sufficient 6+ suit with extras.
    let jump_rebid_level = cheapest_level_over(highest, opener_strain) + 1;
    if jump_rebid_level <= 3 {
        rules = rules.rule(
            Bid::new(jump_rebid_level, opener_strain),
            1.5,
            len(opener, 6..) & points(16..),
        );
    }
    for second in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if second == opener || responder == Some(second) {
            continue;
        }
        let second_strain = Strain::from(second);
        let cheapest = cheapest_level_over(highest, second_strain);
        // Reverse: a non-jump two-level new suit ranking above opener's,
        // forcing partner past a return to opener's suit at the two level.
        // Alerted: the rule floors opener's (unbid-here) first suit, so it is
        // artificial by the house rule and decoded by rule projection.
        if cheapest == 2 && second_strain > opener_strain {
            rules = rules
                .rule(
                    Bid::new(2, second_strain),
                    1.6,
                    len(opener, 5..) & len(second, 4..) & points(17..),
                )
                .alert(OPENER_REVERSE);
        }
        // Jump-shift: a single jump in a new suit, game-forcing.  18+ rather
        // than 19+ so a shapely two-suiter (a 5-5 with controls upgrades past
        // the band) is not stranded in the minimum rebid.  Alerted for the same
        // reason as the reverse (it floors opener's first suit).
        let jump_shift_level = cheapest + 1;
        if jump_shift_level <= 3 {
            rules = rules
                .rule(
                    Bid::new(jump_shift_level, second_strain),
                    1.7,
                    len(opener, 5..) & len(second, 4..) & points(18..),
                )
                .alert(OPENER_JUMP_SHIFT);
        }
    }
    rules
}

/// Whether a rebid is opener's invitational `3♣`/`3♦` jump (the Meckstroth `3m`)
fn is_invitational_minor_jump(rebid: Call) -> bool {
    rebid == call(3, Strain::Clubs) || rebid == call(3, Strain::Diamonds)
}

/// Append the Meckstroth-adjunct invitational minor jumps when enabled
///
/// `3♣`/`3♦` show 5+ cards in the minor and ≈15–17 points — the medium shapely
/// hand that otherwise underbids as a natural two-level minor.  The weight sits
/// above the natural minor (0.9) and the six-card-major rebid (1.0) but below
/// the strong 2NT (1.2), so disjointness is by strength: 18–19 balanced → 2NT;
/// 15–17 with a five-card minor → `3m`; a minimum → the natural two level.
fn with_invitational_minors(mut rules: Rules) -> Rules {
    if meckstroth() {
        for minor in [Suit::Clubs, Suit::Diamonds] {
            rules = rules.rule(
                Bid::new(3, Strain::from(minor)),
                1.05,
                len(minor, 5..) & points(15..=17),
            );
        }
    }
    rules
}

/// Whether a rebid is opener's invitational major two-suiter (`set_forcing_nt_two_suiter`)
///
/// `1♥ – 1NT – 2♠` (the reverse) or `1♠ – 1NT – 3♥` (the 5-5 jump); the other
/// major has no such call.
fn is_forcing_nt_two_suiter(major: Suit, rebid: Call) -> bool {
    match major {
        Suit::Hearts => rebid == call(2, Strain::Spades),
        Suit::Spades => rebid == call(3, Strain::Hearts),
        _ => false,
    }
}

/// Append opener's invitational (15–17) major two-suiter rebid when enabled
///
/// Fills the seam between the minimum natural rebids and the 18+ game force:
/// over `1♥` the `2♠` reverse (5+ hearts, 4+ spades, forcing one round), over
/// `1♠` the `3♥` jump (5-5 majors, invitational).  Both floor opener's first
/// suit, so both are alerted (reused reverse/jump-shift tags) and decoded by
/// rule projection.  Weights sit above the natural minimum rebids (0.9/1.0) but
/// below the `3M` major jump-rebid (1.5) and the 18+ `2NT` (1.6), so the crisp
/// `points(15..=17)` band keeps 18+ hands in the game force.
fn with_forcing_nt_two_suiter(rules: Rules, major: Suit) -> Rules {
    if !forcing_nt_two_suiter() {
        return rules;
    }
    match major {
        Suit::Hearts => rules
            .rule(
                Bid::new(2, Strain::Spades),
                1.1,
                len(Suit::Hearts, 5..) & len(Suit::Spades, 4..) & points(15..=17),
            )
            .alert(OPENER_REVERSE),
        Suit::Spades => rules
            .rule(
                Bid::new(3, Strain::Hearts),
                1.15,
                len(Suit::Spades, 5..) & len(Suit::Hearts, 5..) & points(15..=17),
            )
            .alert(OPENER_JUMP_SHIFT),
        _ => rules,
    }
}

/// Opener's rebid after `1♥ – 1♠`: raise spades, rebid hearts, or show shape
///
/// Forcing on opener — there is no pass rule.
fn rebid_one_heart_one_spade() -> Rules {
    let mut rules = Rules::new()
        .rule(
            Bid::new(4, Strain::Spades),
            2.6,
            support(4..) & points(19..),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            2.2,
            support(4..) & points(16..=18),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            1.8,
            support(4..) & points(12..=15),
        )
        .rule(Bid::new(2, Strain::Hearts), 1.4, len(Suit::Hearts, 6..))
        .rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            fifths(18.0..20.0) & balanced(),
        );
    // Meckstroth adjunct: invitational 3♣/3♦ jumps with a five-card minor.
    rules = with_invitational_minors(rules);
    // Major jump-rebid: 1♥ – 1♠ – 3♥ on a six-card major with extras.
    rules = with_major_jump_rebid(rules, Suit::Hearts, Bid::new(1, Strain::Spades));
    rules
        .rule(Bid::new(2, Strain::Clubs), 0.9, len(Suit::Clubs, 4..))
        .rule(Bid::new(2, Strain::Diamonds), 0.9, len(Suit::Diamonds, 4..))
        // Balanced minimum, and the guaranteed-legal fallback.
        .rule(Bid::new(1, Strain::Notrump), 0.5, fifths(12.0..15.0))
        .rule(Bid::new(1, Strain::Notrump), 0.2, hcp(0..))
}

/// Opener's rebid after `1M – 1NT` (the forcing notrump)
///
/// Forcing on opener.  A five-card-major rebid is the guaranteed-legal
/// fallback when nothing more descriptive fits — a basic simplification.
fn rebid_after_forcing_notrump(major: Suit) -> Rules {
    let trump = Strain::from(major);
    let mut rules = Rules::new();
    // 2NT: the Meckstroth adjunct's artificial 18+ game force (any shape) when
    // enabled, otherwise the natural 18–19 balanced rebid.  Weight 1.6 to outrank
    // the 3M major jump-rebid (1.5), so every 18+ hand routes through the game
    // force while the invitational 3m jumps stay 15–17.
    if meckstroth() {
        rules = rules
            .rule(Bid::new(2, Strain::Notrump), 1.6, points(18..))
            .alert(OPENER_GF_2NT);
    } else {
        rules = rules.rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            fifths(18.0..20.0) & balanced(),
        );
    }
    rules = rules.rule(Bid::new(2, trump), 1.0, len(major, 6..));
    // Meckstroth adjunct: invitational 3♣/3♦ jumps with a five-card minor.
    rules = with_invitational_minors(rules);
    // Major jump-rebid: 1M – 1NT – 3M on a six-card major with extras.
    rules = with_major_jump_rebid(rules, major, Bid::new(1, Strain::Notrump));
    // Invitational two-suiter: 1♥ – 1NT – 2♠ reverse / 1♠ – 1NT – 3♥ jump.
    rules = with_forcing_nt_two_suiter(rules, major);
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        if Strain::from(suit) < trump {
            rules = rules.rule(Bid::new(2, Strain::from(suit)), 0.9, len(suit, 4..));
        }
    }
    // Opener always holds at least five of the major, so this always applies.
    rules.rule(Bid::new(2, trump), 0.3, len(major, 5..))
}

/// Opener's rebid raising responder's new major after a minor opening
///
/// Used at `1m – 1M`.  Forcing on opener; a 1NT rebid is the guaranteed-legal
/// fallback.  Under the up-the-line completion (`set_up_the_line`) opener
/// also shows four spades over a `1♥` response — without it the 4-4 spade
/// fit is lost to the 1NT rebid.
fn rebid_raise_major(responder_major: Suit, opener_minor: Suit) -> Rules {
    let m = Strain::from(responder_major);
    let mut rules = Rules::new()
        .rule(Bid::new(4, m), 2.6, support(4..) & points(19..))
        .rule(Bid::new(3, m), 2.2, support(4..) & points(16..=18))
        .rule(Bid::new(2, m), 1.8, support(4..) & points(12..=15))
        .rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            fifths(18.0..20.0) & balanced(),
        );
    // Balanced 12–14 with a five-card minor: rebid 1NT rather than the natural
    // 2m below it (weight 0.92 — above the 2m rebid, below the up-the-line 1♠
    // so a 4-4 spade fit is still found).  Shipped default-on.
    if balanced_1nt_rebid() {
        rules = rules.rule(
            Bid::new(1, Strain::Notrump),
            0.92,
            fifths(12.0..15.0) & balanced(),
        );
    }
    // Up the line: four spades over a 1♥ response, ahead of the minor rebid
    // and the notrump fallbacks (a heart raise with four-card support still
    // wins on weight).
    if responder_major == Suit::Hearts && super::responses::up_the_line() {
        rules = rules.rule(Bid::new(1, Strain::Spades), 0.95, len(Suit::Spades, 4..));
    }
    // Strength-showing ladder: jump-rebid, reverse, jump-shift (default off).
    rules = with_extras_ladder(rules, opener_minor, Bid::new(1, m), Some(responder_major));
    rules
        .rule(
            Bid::new(2, Strain::from(opener_minor)),
            0.9,
            len(opener_minor, 5..),
        )
        .rule(
            Bid::new(1, Strain::Notrump),
            0.5,
            fifths(12.0..15.0) & balanced(),
        )
        .rule(Bid::new(1, Strain::Notrump), 0.2, hcp(0..))
}

/// Opener's rebid after `1♣ – 1♦`
///
/// Under the up-the-line completion (`set_up_the_line`) a six-plus club suit
/// rebids a natural `2♣` — without it those hands land in the misdescribed
/// 1NT catch-all.
fn rebid_one_club_one_diamond() -> Rules {
    let mut rules = Rules::new()
        .rule(Bid::new(1, Strain::Hearts), 1.3, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(1, Strain::Spades),
            1.3,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.5,
            support(4..) & points(16..=18),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.2,
            support(4..) & points(12..=15),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            1.1,
            fifths(18.0..20.0) & balanced(),
        );
    if super::responses::up_the_line() {
        rules = rules.rule(Bid::new(2, Strain::Clubs), 0.9, len(Suit::Clubs, 6..));
    }
    // Strength-showing ladder: jump-rebid, reverse, jump-shift (default off).
    rules = with_extras_ladder(
        rules,
        Suit::Clubs,
        Bid::new(1, Strain::Diamonds),
        Some(Suit::Diamonds),
    );
    rules
        .rule(
            Bid::new(1, Strain::Notrump),
            0.5,
            fifths(12.0..15.0) & balanced(),
        )
        .rule(Bid::new(1, Strain::Notrump), 0.2, hcp(0..))
}

// ---------------------------------------------------------------------------
// Responder's second call after the forcing 1NT
// ---------------------------------------------------------------------------

/// Responder's options after opener's rebid in the forcing-1NT structure
///
/// One shared table covers every opener rebid; rules for calls that are
/// illegal in a particular sequence simply go dead.  The table in priority
/// order:
///
/// | Call   | Wt  | Meaning |
/// |--------|-----|---------|
/// | 3M     | 1.5 | Three-card limit raise (10–12 HCP) |
/// | 2NT    | 1.2 | Natural notrump invite (11–12 HCP) |
/// | 2x≠M   | 1.1 | Six-card runout, weak (≤ 9 HCP); dead when illegal |
/// | 2M     | 1.0 | Preference to the major (7+ HCP, 2+ cards) |
/// | Pass   | 0.0 | Catch-all: the force was one round only |
fn responder_after_forcing_notrump(major: Suit) -> Rules {
    let trump = Strain::from(major);
    let mut rules = Rules::new()
        // Three-card limit raise — the standard 2/1 route: 1NT then 3M.
        .rule(Bid::new(3, trump), 1.5, len(major, 3..) & hcp(10..=12))
        // Natural notrump invite.
        .rule(Bid::new(2, Strain::Notrump), 1.2, hcp(11..=12))
        // Preference to opener's major.
        .rule(Bid::new(2, trump), 1.0, len(major, 2..) & hcp(7..))
        // Catch-all pass; the forcing 1NT is one round only.
        .rule(Call::Pass, 0.0, hcp(0..));

    // Six-card runouts into a side suit (dead when the call is illegal in
    // the current auction).
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit != major {
            rules = rules.rule(
                Bid::new(2, Strain::from(suit)),
                1.1,
                len(suit, 6..) & hcp(..=9),
            );
        }
    }
    rules
}

/// Responder's call over opener's invitational `3m` jump (Meckstroth adjunct)
///
/// Opener has shown 5+ of the minor and ≈15–17 points.  Responder accepts game
/// with a maximum forcing-1NT (or `1♠`) hand and declines to a preference in
/// opener's five-card major with a minimum.  The `len(major, ..)` guards keep
/// the major-preference rules dead when responder is short, so one table serves
/// both the forcing-1NT auctions and `1♥ – 1♠` (where responder's holding in
/// opener's major is unknown).
///
/// | Call   | Wt  | Meaning |
/// |--------|-----|---------|
/// | 4M     | 1.4 | Accept: 5-3 major game (3+ support, 10+ points) |
/// | 3NT    | 1.2 | Accept: notrump game, no major fit (10+ points) |
/// | 3M     | 1.0 | Decline: preference to opener's major (2+ cards, minimum) |
/// | Pass   | 0.0 | Decline: minimum, short in the major — pass the invite |
fn responder_after_invitational_minor(major: Suit) -> Rules {
    let trump = Strain::from(major);
    Rules::new()
        // Accept to the 5-3 major game.
        .rule(Bid::new(4, trump), 1.4, len(major, 3..) & points(10..))
        // Accept to notrump game with no major fit.
        .rule(Bid::new(3, Strain::Notrump), 1.2, points(10..))
        // Decline: preference to opener's five-card major.
        .rule(Bid::new(3, trump), 1.0, len(major, 2..) & points(..10))
        // Catch-all: minimum, short in the major — pass the invitation.
        // ponytail: a 5m minor game is folded into 3NT; add an explicit 5m raise
        // if the A/B shows it matters.
        .rule(Call::Pass, 0.0, points(0..))
}

/// Responder's call over opener's invitational `3M` jump-rebid
///
/// Opener has shown 6+ of the major and 16+ points.  A forcing-1NT responder
/// is usually short in the major (3+ support would have raised), so the
/// notrump game is the common accept; a doubleton is already an eight-card fit
/// opposite six, so the major-game raise needs only `len(major, 2..)`.  Used at
/// `1M – 1NT – 3M` and `1♥ – 1♠ – 3♥`.
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 4M   | 1.4 | Accept: major game on an 8+ card fit (2+ support, 8+ points) |
/// | 3NT  | 1.2 | Accept: notrump game, no major fit (9+ points) |
/// | Pass | 0.0 | Decline: minimum — play `3M` |
fn responder_after_major_jump_rebid(major: Suit) -> Rules {
    let trump = Strain::from(major);
    Rules::new()
        .rule(Bid::new(4, trump), 1.4, len(major, 2..) & points(8..))
        .rule(Bid::new(3, Strain::Notrump), 1.2, points(9..))
        .rule(Call::Pass, 0.0, points(0..))
}

/// Opener accepts or declines responder's 2NT notrump invite
///
/// Accept with 14+ HCP (bid 3NT), decline with a pass.
fn opener_accept_notrump_invite() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(14..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener accepts or declines responder's 3M limit raise
///
/// Accept with 14+ points (bid game in the major), decline with a pass.
fn opener_accept_limit_raise(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 1.0, points(14..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Register responder's second call and opener's acceptance in the
/// forcing-1NT structure
///
/// For each major and each distinct opener rebid that is NOT 2NT (the 18–19
/// balanced rebid's continuations live in the notrump module) and NOT a
/// Meckstroth `3m` jump (handled by `register_invitational_minor_continuations`),
/// inserts responder's table at `[1M, 1NT, rebid]` and opener's acceptances at
/// `[1M, 1NT, rebid, 2NT]` and `[1M, 1NT, rebid, 3M]`.
fn register_forcing_notrump_continuations(book: &mut Trie) {
    for major in [Suit::Hearts, Suit::Spades] {
        let one_major = call(1, Strain::from(major));
        let one_nt = call(1, Strain::Notrump);

        // Collect distinct rebid calls that take the shared two-level
        // continuation: everything except the 2NT rebid and the `3m` jumps.
        let mut seen: Vec<Call> = Vec::new();
        for rule in rebid_after_forcing_notrump(major).rules() {
            let rebid = rule.call();
            if rebid != call(2, Strain::Notrump)
                && !is_invitational_minor_jump(rebid)
                && !is_forcing_nt_two_suiter(major, rebid)
                && !seen.contains(&rebid)
            {
                seen.push(rebid);
            }
        }

        for rebid in seen {
            insert_uncontested(
                book,
                &[one_major, one_nt, rebid],
                responder_after_forcing_notrump(major),
            );
            insert_uncontested(
                book,
                &[one_major, one_nt, rebid, call(2, Strain::Notrump)],
                opener_accept_notrump_invite(),
            );
            insert_uncontested(
                book,
                &[one_major, one_nt, rebid, call(3, Strain::from(major))],
                opener_accept_limit_raise(major),
            );
        }
    }

    register_invitational_minor_continuations(book);
}

/// Register responder's call over opener's invitational `3m` (Meckstroth adjunct)
///
/// Covers both the forcing-1NT auctions (`1M – 1NT – 3m`) and the `1♥ – 1♠`
/// auction (`1♥ – 1♠ – 3m`, where opener's major is hearts).  A no-op when the
/// adjunct is disabled — opener's tables then carry no `3m` jump to continue.
fn register_invitational_minor_continuations(book: &mut Trie) {
    if !meckstroth() {
        return;
    }
    let three_minors = [call(3, Strain::Clubs), call(3, Strain::Diamonds)];

    // Forcing 1NT: 1M – 1NT – 3m, responder's major support unknown.
    for major in [Suit::Hearts, Suit::Spades] {
        let prefix = [call(1, Strain::from(major)), call(1, Strain::Notrump)];
        for three_m in three_minors {
            insert_uncontested(
                book,
                &[prefix[0], prefix[1], three_m],
                responder_after_invitational_minor(major),
            );
        }
    }

    // 1♥ – 1♠ – 3m: opener's major is hearts, responder has shown 4+ spades.
    let one_heart = call(1, Strain::Hearts);
    let one_spade = call(1, Strain::Spades);
    for three_m in three_minors {
        insert_uncontested(
            book,
            &[one_heart, one_spade, three_m],
            responder_after_invitational_minor(Suit::Hearts),
        );
    }
}

/// Register responder's call over opener's `3M` jump-rebid
///
/// Covers `1M – 1NT – 3M` and `1♥ – 1♠ – 3♥`.  A no-op when the rung is
/// disabled — opener's tables then carry no `3M` jump to continue.
fn register_major_jump_rebid_continuations(book: &mut Trie) {
    if !opener_major_jump_rebid() {
        return;
    }
    for major in [Suit::Hearts, Suit::Spades] {
        insert_uncontested(
            book,
            &[
                call(1, Strain::from(major)),
                call(1, Strain::Notrump),
                call(3, Strain::from(major)),
            ],
            responder_after_major_jump_rebid(major),
        );
    }
    // 1♥ – 1♠ – 3♥: opener's major is hearts, responder has shown 4+ spades.
    insert_uncontested(
        book,
        &[
            call(1, Strain::Hearts),
            call(1, Strain::Spades),
            call(3, Strain::Hearts),
        ],
        responder_after_major_jump_rebid(Suit::Hearts),
    );
}

/// Responder's call over opener's `1♥ – 1NT – 2♠` reverse (5+ hearts, 4+ spades)
///
/// Opener has 15–17 and a real spade suit; responder holds ≤ 3 spades (the
/// forcing 1NT denied four).  Forcing one round — the `2NT` fallback is the
/// finite catch-all, so there is no `Pass`.  Opener's acceptance of a below-game
/// signoff (`3♥`/`2NT`) is left to the deterministic floor (a natural invite).
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 4♥   | 1.5 | 5-3 heart game (3+ hearts, values) |
/// | 4♠   | 1.3 | 4-3 spade game (exactly three spades, values) |
/// | 3NT  | 1.2 | No eight-card fit, values — to play |
/// | 3♥   | 1.0 | Heart preference, minimum |
/// | 2NT  | 0.0 | Guaranteed-legal minimum catch-all |
fn responder_over_forcing_nt_reverse() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Hearts),
            1.5,
            len(Suit::Hearts, 3..) & points(8..),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            1.3,
            len(Suit::Spades, 3..=3) & points(8..),
        )
        .rule(Bid::new(3, Strain::Notrump), 1.2, points(8..))
        .rule(Bid::new(3, Strain::Hearts), 1.0, len(Suit::Hearts, 2..))
        .rule(Bid::new(2, Strain::Notrump), 0.0, points(0..))
}

/// Responder's call over opener's `1♠ – 1NT – 3♥` jump (5-5 majors, invitational)
///
/// Opener has 15–17 and 5-5 in the majors; responder accepts to game with a fit
/// or values, else declines.  Non-forcing — `Pass` (heart tolerance) is the
/// finite catch-all.  Opener's acceptance of a `3♠` decline is left to the floor.
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 4♠   | 1.5 | Spade fit game (3+ spades, values) |
/// | 4♥   | 1.4 | Heart fit game (3+ hearts, values) |
/// | 3NT  | 1.2 | Values, no three-card fit — to play |
/// | 3♠   | 1.0 | Spade preference, decline (minimum) |
/// | Pass | 0.0 | Heart tolerance, decline — play `3♥` |
fn responder_over_forcing_nt_5_5() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Spades),
            1.5,
            len(Suit::Spades, 3..) & points(8..),
        )
        .rule(
            Bid::new(4, Strain::Hearts),
            1.4,
            len(Suit::Hearts, 3..) & points(8..),
        )
        .rule(Bid::new(3, Strain::Notrump), 1.2, points(8..))
        .rule(Bid::new(3, Strain::Spades), 1.0, len(Suit::Spades, 2..))
        .rule(Call::Pass, 0.0, points(0..))
}

/// Register responder's continuations over opener's invitational major
/// two-suiter rebids (no-op unless [`set_forcing_nt_two_suiter`] enabled them)
fn register_forcing_nt_two_suiter_continuations(book: &mut Trie) {
    if !forcing_nt_two_suiter() {
        return;
    }
    // 1♥ – 1NT – 2♠ (reverse).
    insert_uncontested(
        book,
        &[
            call(1, Strain::Hearts),
            call(1, Strain::Notrump),
            call(2, Strain::Spades),
        ],
        responder_over_forcing_nt_reverse(),
    );
    // 1♠ – 1NT – 3♥ (5-5 jump).
    insert_uncontested(
        book,
        &[
            call(1, Strain::Spades),
            call(1, Strain::Notrump),
            call(3, Strain::Hearts),
        ],
        responder_over_forcing_nt_5_5(),
    );
}

// ---------------------------------------------------------------------------
// The real Meckstroth adjunct: opener's artificial game-forcing 2NT (opt-in)
// ---------------------------------------------------------------------------

/// The major responder could not have opened — mirrors the 2NT machine
fn other_major(major: Suit) -> Suit {
    match major {
        Suit::Hearts => Suit::Spades,
        _ => Suit::Hearts,
    }
}

/// Responder's call over opener's artificial game-forcing `2NT`
///
/// `1M – 1NT – 2NT!` set up a game force (18+, any shape).  Responder shows a
/// fit, a five-card red suit, five clubs (artificially, via `3NT`), or relays
/// `3♣` for opener to describe.  Forcing — the `3♣` relay is the finite
/// catch-all, so there is no `Pass`.
///
/// | Call  | Wt   | Meaning |
/// |-------|------|---------|
/// | 3M    | 1.45 | Fit + slam interest (3+ support, 10+) → RKCB round |
/// | 4M    | 1.40 | Fit, no slam interest (3+ support, ≤9) → to play |
/// | 3♦/3♥ | 1.30 | Natural five-plus red suit (not opener's major) |
/// | 3NT!  | 1.25 | 5+ clubs, doubleton in opener's major (opener may pull) |
/// | 3♣!   | 0.50 | Relay — nothing to show, "you describe" |
fn responder_over_gf_2nt(major: Suit) -> Rules {
    let m = Strain::from(major);
    let mut rules = Rules::new()
        .rule(Bid::new(3, m), 1.45, len(major, 3..) & points(10..))
        .rule(Bid::new(4, m), 1.40, len(major, 3..) & points(..=9));
    // Natural five-plus red suits (the game force is set, so free to show).  Over
    // 1♥ only diamonds is available — 1NT denied four spades, and hearts is the fit.
    for red in [Suit::Diamonds, Suit::Hearts] {
        if red != major {
            rules = rules.rule(Bid::new(3, Strain::from(red)), 1.30, len(red, 5..));
        }
    }
    rules
        // The fourth suit, shown artificially for symmetry with 3♦/3♥: five-plus
        // clubs and exactly a doubleton in opener's major (so opener can pull to
        // a 6-2 game).  Non-forcing — opener may pass 3NT.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.25,
            len(Suit::Clubs, 5..) & len(major, 2..=2),
        )
        .alert(RESP_CLUBS_2NT)
        // Relay: nothing to show — "you describe".  The finite catch-all.
        .rule(Bid::new(3, Strain::Clubs), 0.50, points(0..))
        .alert(PUPPET_2NT)
}

/// Opener's shape-out over the `3♣` relay (`1M – 1NT – 2NT! – 3♣!`)
///
/// Opener describes toward the right game or slam.  Forcing — `3♦` is the finite
/// catch-all, so there is no `Pass`.
///
/// | Call  | Wt   | Meaning |
/// |-------|------|---------|
/// | 3M    | 1.35 | Six-plus own major (one-suiter) |
/// | 3(oM) | 1.30 | Four-plus the other major (natural) |
/// | 3NT!  | 1.25 | Five-plus a minor |
/// | 3♦!   | 1.20 | Default — balanced 18–19 or a four-card minor; catch-all |
fn opener_shapeout(major: Suit) -> Rules {
    let m = Strain::from(major);
    let other = other_major(major);
    Rules::new()
        .rule(Bid::new(3, m), 1.35, len(major, 6..))
        .rule(Bid::new(3, Strain::from(other)), 1.30, len(other, 4..))
        .rule(
            Bid::new(3, Strain::Notrump),
            1.25,
            len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..),
        )
        .alert(GF_MINOR)
        // Default: balanced 18–19 or a four-card minor — the guaranteed-legal
        // catch-all (opener is 18+, so `points(0..)` always applies).
        .rule(Bid::new(3, Strain::Diamonds), 1.20, points(0..))
        .alert(GF_DEFAULT)
}

/// Responder places over opener's `3♦` default (`… – 2NT! – 3♣! – 3♦!`)
///
/// Opener is balanced 18–19 or has a four-card minor, with exactly five of the
/// major (a sixth would have jumped to `3M`).  Responder raises a 5-3 major fit
/// or signs off in `3NT`.
fn resp_place_over_default(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 1.2, len(major, 3..))
        .rule(Bid::new(3, Strain::Notrump), 1.0, points(0..))
}

/// Responder places over opener's `3(other major)` (four-plus the other major)
///
/// Raises the concealed 4-4 (or 4-3) fit, falls back to opener's five-card own
/// major with three-card support, else `3NT`.
fn resp_place_over_other_major(major: Suit) -> Rules {
    let o = other_major(major);
    Rules::new()
        .rule(Bid::new(4, Strain::from(o)), 1.3, len(o, 4..))
        .rule(Bid::new(4, Strain::from(major)), 1.1, len(major, 3..))
        .rule(Bid::new(3, Strain::Notrump), 0.8, points(0..))
}

/// Responder places over opener's six-plus own major (`… – 3♣! – 3M`)
///
/// An eight-card major fit is near-certain; responder drives slam with a maximum
/// (`4NT` RKCB) or signs off in game.
fn resp_place_over_six(major: Suit) -> Rules {
    let m = Strain::from(major);
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.3, points(11..))
        .alert(super::slam::RKCB)
        .rule(Bid::new(4, m), 1.0, points(0..))
}

/// Responder places over opener's `3NT` (five-plus a minor, i.e. 5-5)
///
/// Non-forcing: responder pulls to a 5-3 major game or passes to play `3NT`.
fn resp_place_over_minor(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 1.1, len(major, 3..))
        .rule(Call::Pass, 0.0, points(0..))
}

/// Opener's call over responder's direct fit slam-try (`1M – 1NT – 2NT! – 3M`)
///
/// Responder agreed the major with slam interest; opener asks keycards on a
/// clear maximum, else signs off in game.
fn opener_over_fit_slamtry(major: Suit) -> Rules {
    let m = Strain::from(major);
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.3, points(20..))
        .alert(super::slam::RKCB)
        .rule(Bid::new(4, m), 0.5, points(0..))
}

/// Opener's call over responder's natural five-plus red suit
///
/// `red` is responder's suit.  Opener raises a heart fit to game, rebids a
/// six-card own major, else places `3NT` (the guaranteed-legal game).
// ponytail: no diamond-slam exploration — a diamond fit lands in 3NT (game
// reached); add a 4♦ slam-try rung if the A/B shows stranded minor slams.
fn opener_over_resp_red(major: Suit, red: Suit) -> Rules {
    let mut rules = Rules::new();
    if red == Suit::Hearts {
        rules = rules.rule(Bid::new(4, Strain::Hearts), 1.3, len(Suit::Hearts, 3..));
    }
    rules
        .rule(Bid::new(3, Strain::from(major)), 1.1, len(major, 6..))
        .rule(Bid::new(3, Strain::Notrump), 0.5, points(0..))
}

/// Opener's call over responder's `3NT` (five-plus clubs, doubleton major)
///
/// Non-forcing: opener pulls to a 6-2 major game with a sixth card, else passes
/// to play `3NT`.
fn opener_over_resp_clubs(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 1.0, len(major, 6..))
        .rule(Call::Pass, 0.0, points(0..))
}

/// Register the artificial game-forcing `2NT` adjunct (no-op unless
/// [`set_meckstroth_adjunct`] enabled it)
///
/// Authors both sides below `1M – 1NT – 2NT!`: responder's relay round, opener's
/// shape-out over `3♣`, responder's placement over each shape-out (with RKCB on
/// the two major-fit nodes), and opener's placement over responder's own bids.
/// This **overrides** the natural-2NT continuation `notrump.rs` installed at
/// `[1M, 1NT, 2NT]` — `rebids::register` runs after `notrump::register`, so the
/// on-knob insert wins; with the knob off nothing is authored and the natural
/// handling stands.
fn register_meckstroth_2nt_continuations(book: &mut Trie) {
    if !meckstroth() {
        return;
    }
    for major in [Suit::Hearts, Suit::Spades] {
        let m = Strain::from(major);
        let one_m = call(1, m);
        let one_nt = call(1, Strain::Notrump);
        let two_nt = call(2, Strain::Notrump);
        let three_c = call(3, Strain::Clubs);
        let three_d = call(3, Strain::Diamonds);
        let three_m = call(3, m);
        let three_o = call(3, Strain::from(other_major(major)));
        let three_nt = call(3, Strain::Notrump);
        let base = [one_m, one_nt, two_nt];

        // Responder's relay round over the game-forcing 2NT.
        insert_uncontested(book, &base, responder_over_gf_2nt(major));

        // Opener's shape-out over the 3♣ relay, and responder's placement over
        // each of opener's four shape-outs.
        insert_uncontested(
            book,
            &[one_m, one_nt, two_nt, three_c],
            opener_shapeout(major),
        );
        insert_uncontested(
            book,
            &[one_m, one_nt, two_nt, three_c, three_d],
            resp_place_over_default(major),
        );
        insert_uncontested(
            book,
            &[one_m, one_nt, two_nt, three_c, three_o],
            resp_place_over_other_major(major),
        );
        let six_node = [one_m, one_nt, two_nt, three_c, three_m];
        insert_uncontested(book, &six_node, resp_place_over_six(major));
        super::slam::install_rkcb(book, &six_node, major);
        insert_uncontested(
            book,
            &[one_m, one_nt, two_nt, three_c, three_nt],
            resp_place_over_minor(major),
        );

        // Responder's direct fit slam-try, then RKCB.
        let fit_node = [one_m, one_nt, two_nt, three_m];
        insert_uncontested(book, &fit_node, opener_over_fit_slamtry(major));
        super::slam::install_rkcb(book, &fit_node, major);

        // Opener's placement over responder's natural red suits.
        for red in [Suit::Diamonds, Suit::Hearts] {
            if red != major {
                insert_uncontested(
                    book,
                    &[one_m, one_nt, two_nt, call(3, Strain::from(red))],
                    opener_over_resp_red(major, red),
                );
            }
        }

        // Opener's placement over responder's 3NT clubs (non-forcing).
        insert_uncontested(
            book,
            &[one_m, one_nt, two_nt, three_nt],
            opener_over_resp_clubs(major),
        );
    }
}

// ---------------------------------------------------------------------------
// Major-rebid tails: full continuations after `1♥ – 1♠` (opt-in)
// ---------------------------------------------------------------------------

// ponytail: same construction-time-toggle reasoning as `MECKSTROTH` above.
std::thread_local! {
    /// Whether opener's rebid tables carry the **major-rebid-tails adjunct**:
    /// full responder/opener continuations after `1♥ – 1♠` below opener's
    /// `2♠`/`3♠` raise, `2♥` rebid, and `2♣`/`2♦` minor rebid.  Default on
    /// (measured +0.016/+0.023 IMPs/board NV/vul plain DD).
    static MAJOR_REBID_TAILS: Cell<bool> = const { Cell::new(true) };
}

/// Enable or disable the major-rebid-tails adjunct in books built *after*
/// this call
///
/// Read at book-construction time (during `register`); set it before
/// building the `Pair`.  **Default on** (`--no-ns-major-rebid-tails` in
/// `bba-gen` for the off arm).
pub fn set_major_rebid_tails(on: bool) {
    MAJOR_REBID_TAILS.with(|cell| cell.set(on));
}

/// Whether the major-rebid-tails adjunct is currently enabled
fn major_rebid_tails() -> bool {
    MAJOR_REBID_TAILS.with(Cell::get)
}

/// Fourth suit forcing — the fourth suit is an artificial game force
const FOURTH_SUIT: Alert = Alert("fourth-suit-forcing");

// ponytail: same construction-time-toggle reasoning as `MECKSTROTH` above.
std::thread_local! {
    /// Whether the **fourth-suit-forcing** knob is enabled: at
    /// `1♥ – 1♠ – 2♣`, responder's `2♦` becomes an artificial game force (the
    /// fourth suit) instead of natural diamonds.  Default on (measured
    /// +0.002 IMPs/board on top of the tails, both scorers, both
    /// vulnerabilities).
    ///
    /// This continuation *rides* the major-rebid-tails adjunct — with
    /// [`set_major_rebid_tails`] off, enabling this knob registers nothing.
    static FOURTH_SUIT_FORCING: Cell<bool> = const { Cell::new(true) };
}

/// Enable or disable fourth-suit-forcing in books built *after* this call
///
/// Read at book-construction time (during `register`); set it before
/// building the `Pair`.  **Default on** (`--no-ns-fourth-suit-forcing` in
/// `bba-gen` for the off arm).  This continuation rides the
/// major-rebid-tails adjunct — with [`set_major_rebid_tails`] off, enabling
/// this knob registers nothing.
pub fn set_fourth_suit_forcing(on: bool) {
    FOURTH_SUIT_FORCING.with(|cell| cell.set(on));
}

/// Whether fourth-suit-forcing is currently enabled
fn fourth_suit_forcing() -> bool {
    FOURTH_SUIT_FORCING.with(Cell::get)
}

std::thread_local! {
    /// Whether responder's natural 2NT invite after opener shows two suits
    /// (`1♥ – 1♠ – 2m`) is gauged in raw HCP instead of `points`.  **Default
    /// on** (fix-vs-shipped, 1M boards/vul, 24.pdd 18.3M–20.3M: plain DD
    /// +0.0018 ± 0.0003 NV / +0.0022 ± 0.0005 vul, PD +0.0028/+0.0032).  See
    /// [`set_nt_invite_hcp`].
    static NT_INVITE_HCP: Cell<bool> = const { Cell::new(true) };
}

/// Gauge responder's 2NT invite after `1♥ – 1♠ – 2m` in raw HCP for books
/// built *after* this call
///
/// The 2NT rung is the table's one no-fit call — the hand denied a heart
/// preference and a minor raise, so its long-suit `points` credit prices ruffs
/// that a notrump part-score never takes (the quantitative-6NT reasoning one
/// level down).  Rule-of-N+8 reads a shaped 9-count 10+, invites, and loses
/// both mirror directions (the point-count remnant's 2NT-invite seam).  The
/// fit-showing rungs (`3♥`/`3m` invites) keep `points`, mirroring the 2/1
/// hcp/support-points split.  **Default on** (measured; see the thread-local
/// above); `false` restores the shipped `points` gauge.
pub fn set_nt_invite_hcp(on: bool) {
    NT_INVITE_HCP.with(|cell| cell.set(on));
}

/// Whether the post-two-suit 2NT invite is HCP-gauged
fn nt_invite_hcp() -> bool {
    NT_INVITE_HCP.with(Cell::get)
}

/// Responder's second call after opener raises to `2♠` in `1♥ – 1♠`
///
/// Opener's `2♠` shows four-card support and a 12–15 point opening.  The
/// `4NT` keycard ask is authored the same way as the 2/1 game force's
/// opener-third rule: the call itself carries the RKCB alert, and
/// `slam::install_rkcb` installs everything below it.
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 4NT  | 2.0 | Keycard ask: slam interest opposite a maximum (16+ points) |
/// | 4♠   | 1.5 | Sign off in game (12+ points) |
/// | 3♠   | 1.2 | Invitational raise (10–11 points) |
/// | Pass | 0.0 | Minimum, decline any further invitation |
#[must_use]
fn responder_after_spade_raise() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 2.0, points(16..))
        .alert(super::slam::RKCB)
        .rule(Bid::new(4, Strain::Spades), 1.5, points(12..))
        .rule(Bid::new(3, Strain::Spades), 1.2, points(10..=11))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Responder's second call after opener jumps to `3♠` in `1♥ – 1♠`
///
/// Opener's `3♠` shows four-card support and a strong 16–18 point opening —
/// game is close to guaranteed, so responder's only question is whether to
/// explore slam or sign off.
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 4NT  | 1.5 | Keycard ask: slam interest (14+ points) |
/// | 4♠   | 1.0 | Accept to game (8+ points) |
/// | Pass | 0.0 | Minimum, decline |
#[must_use]
fn responder_after_spade_jump() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.5, points(14..))
        .alert(super::slam::RKCB)
        .rule(Bid::new(4, Strain::Spades), 1.0, points(8..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Responder's second call after opener rebids `2♥` in `1♥ – 1♠`
///
/// Opener's `2♥` shows a six-card suit; responder's `1♠` did not deny three
/// hearts, so a heart fit is common at this node.
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 4♥   | 1.5 | Raise to game, 2+ hearts (13+ points) |
/// | 3NT  | 1.3 | Game with no heart fit (13+ points) |
/// | 3♥   | 1.2 | Invitational raise, 2+ hearts (10–12 points) |
/// | 2NT  | 1.0 | Natural notrump invite (10–12 points) |
/// | Pass | 0.0 | Minimum, nothing further |
#[must_use]
fn responder_after_heart_rebid() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Hearts),
            1.5,
            len(Suit::Hearts, 2..) & points(13..),
        )
        .rule(Bid::new(3, Strain::Notrump), 1.3, points(13..))
        .rule(
            Bid::new(3, Strain::Hearts),
            1.2,
            len(Suit::Hearts, 2..) & points(10..=12),
        )
        .rule(Bid::new(2, Strain::Notrump), 1.0, points(10..=12))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's call over responder's `2NT` notrump invite after `1♥ – 1♠ – 2♥`
///
/// Forcing: the `3♥` retreat is always legal below `2NT`, so there is no pass
/// rule.  Accept with 14+ HCP (bid `3NT`), decline with a `3♥` retreat.
#[must_use]
fn opener_after_heart_invite() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(14..))
        .rule(Bid::new(3, Strain::Hearts), 0.5, hcp(0..))
}

/// Responder's second call after opener rebids a new minor in `1♥ – 1♠`
///
/// Registered at both `1♥ – 1♠ – 2♣` and `1♥ – 1♠ – 2♦` — `minor` is the suit
/// opener rebid, showing 4+ cards on a minimum-ish hand.  Responder's known
/// assets are 4+ spades and 6+ points; heart length is unknown (`1♠` never
/// denied three hearts), so a jump preference to `3♥` outranks the minor
/// raise.
///
/// Fourth-suit-forcing ([`set_fourth_suit_forcing`], riding the
/// major-rebid-tails adjunct) extends this table for `minor == Suit::Clubs`
/// only: a `2♦` response becomes an artificial game force (the fourth suit
/// below `2♣`) instead of natural diamonds.  `minor == Suit::Diamonds` is
/// untouched — the fourth suit there would be a `3♣` jump, out of scope here.
///
/// | Call | Wt   | Meaning |
/// |------|------|---------|
/// | 2♦   | 2.0  | Fourth-suit-forcing game force, 12+ (clubs only, knob-gated) |
/// | 3♥   | 1.3  | Invitational jump preference, 3+ hearts (10–12) |
/// | 3m   | 1.25 | Invitational raise of opener's minor, 5+ (10–12) |
/// | 2NT  | 1.2  | Natural notrump invite (10–12) |
/// | 2♠   | 1.05 | Weak rebid, 6+ spades, to play (≤9) |
/// | 2♥   | 1.0  | Simple preference, 2+ hearts (6–9) |
/// | 3NT  | 0.9  | Game with no fit found (13+) |
/// | Pass | 0.0  | Minimum, nothing further |
#[must_use]
fn responder_after_minor_rebid(minor: Suit) -> Rules {
    let m = Strain::from(minor);
    let mut rules = Rules::new();
    if minor == Suit::Clubs && fourth_suit_forcing() {
        // Fourth-suit-forcing: an artificial game force.  Points-only on
        // purpose — the projection must claim nothing about diamond length.
        rules = rules
            .rule(Bid::new(2, Strain::Diamonds), 2.0, points(12..))
            .alert(FOURTH_SUIT);
    }
    rules = rules
        .rule(
            Bid::new(3, Strain::Hearts),
            1.3,
            len(Suit::Hearts, 3..) & points(10..=12),
        )
        .rule(Bid::new(3, m), 1.25, len(minor, 5..) & points(10..=12));
    // The one no-fit rung: HCP-gauged when `set_nt_invite_hcp` is armed (a
    // notrump invite takes no ruffs), else the shipped `points`.
    rules = if nt_invite_hcp() {
        rules.rule(Bid::new(2, Strain::Notrump), 1.2, hcp(10..=12))
    } else {
        rules.rule(Bid::new(2, Strain::Notrump), 1.2, points(10..=12))
    };
    rules
        .rule(
            Bid::new(2, Strain::Spades),
            1.05,
            len(Suit::Spades, 6..) & hcp(..=9),
        )
        .rule(
            Bid::new(2, Strain::Hearts),
            1.0,
            len(Suit::Hearts, 2..) & hcp(6..=9),
        )
        .rule(Bid::new(3, Strain::Notrump), 0.9, hcp(13..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's call over responder's raise to `3m` after `1♥ – 1♠ – 2m`
///
/// Accept with 14+ points (bid `3NT`), decline with a pass.  Unlike
/// `opener_accept_limit_raise`, game lives in notrump here — the minor is
/// opener's second suit, not the agreed trump.
#[must_use]
fn opener_accept_minor_raise() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.0, points(14..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's answer at `[1♥,1♠,2♣,2♦]`, the fourth-suit-forcing game force
///
/// Forcing — there is no pass rule; the `2♥` catch-all is always legal
/// because opener holds 5+ hearts (guaranteed by the `1♥` opening) and `2♥`
/// outranks `2♦`.
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 2♠   | 1.4 | Delayed three-card raise |
/// | 2♥   | 1.3 | Extra heart length, 6+ |
/// | 2NT  | 1.2 | Notrump with the fourth suit stopped |
/// | 3♣   | 1.1 | A real second suit, 5+ |
/// | 2♥   | 0.2 | Guaranteed-legal catch-all |
#[must_use]
fn opener_after_fourth_suit() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Spades), 1.4, len(Suit::Spades, 3..))
        .rule(Bid::new(2, Strain::Hearts), 1.3, len(Suit::Hearts, 6..))
        .rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            stopper_in(Suit::Diamonds),
        )
        .rule(Bid::new(3, Strain::Clubs), 1.1, len(Suit::Clubs, 5..))
        .rule(Bid::new(2, Strain::Hearts), 0.2, len(Suit::Hearts, 5..))
}

/// Responder's placement at `[1♥,1♠,2♣,2♦,X]`, after opener answers the
/// fourth-suit-forcing game force
///
/// One shared table installed at every answer `X` from
/// [`opener_after_fourth_suit`] — [`partner_suit_is`] reads which answer
/// opener actually gave, so a single table serves all of them.  Forcing to
/// game: `3NT` is always legal since every `X` is at or below `3♣`, so there
/// is no pass rule.
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 4♠   | 1.5 | Opener showed 3-card spade support; 5-3 fit |
/// | 4♥   | 1.2 | Opener opened `1♥` (5+); 5-3 fit |
/// | 4♥   | 1.1 | Opener rebid hearts twice (6+); 6-2 fit |
/// | 3NT  | 0.8 | The game-force landing spot, always legal |
#[must_use]
fn responder_after_fourth_suit_answer() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Spades),
            1.5,
            partner_suit_is(Suit::Spades) & len(Suit::Spades, 5..),
        )
        .rule(Bid::new(4, Strain::Hearts), 1.2, len(Suit::Hearts, 3..))
        .rule(
            Bid::new(4, Strain::Hearts),
            1.1,
            partner_suit_is(Suit::Hearts) & len(Suit::Hearts, 2..),
        )
        .rule(Bid::new(3, Strain::Notrump), 0.8, hcp(0..))
}

/// Register the major-rebid-tails adjunct: full continuations after
/// `1♥ – 1♠` (no-op unless [`set_major_rebid_tails`] enabled it)
///
/// Below each of opener's four rebids this authors both sides' continuations
/// to game, and — for the two spade-raise auctions — to slam via RKCB:
///
/// - `2♠` (raise, 12–15): responder invites, signs off, or asks keycards;
///   opener accepts or declines the `3♠` invite.
/// - `3♠` (jump raise, 16–18): responder signs off or asks keycards.
/// - `2♥` (own suit, 6+): responder invites or signs off; opener accepts,
///   declines, or answers the `2NT` notrump-invite relay.
/// - `2♣`/`2♦` (new minor, 4+, minimum-ish): responder chooses a preference,
///   an invite, or game; opener accepts or declines the invite reached.
/// - `2♣ – 2♦` fourth-suit-forcing ([`set_fourth_suit_forcing`], an
///   additional gate riding this adjunct): opener answers naturally below
///   game; responder places the final contract at game over any answer.
///
/// `1♥ – 1♠ – 2m – 2♥` and `1♥ – 1♠ – 2m – 2♠` are deliberately left to the
/// floor.
fn register_major_rebid_tails(book: &mut Trie) {
    if !major_rebid_tails() {
        return;
    }

    let one_heart = call(1, Strain::Hearts);
    let one_spade = call(1, Strain::Spades);
    let two_spades = call(2, Strain::Spades);
    let three_spades = call(3, Strain::Spades);
    let two_hearts = call(2, Strain::Hearts);
    let three_hearts = call(3, Strain::Hearts);
    let two_nt = call(2, Strain::Notrump);

    // Opener's 2♠ raise (12–15, four-card support): invite/sign-off/RKCB.
    let after_two_spades = [one_heart, one_spade, two_spades];
    insert_uncontested(book, &after_two_spades, responder_after_spade_raise());
    insert_uncontested(
        book,
        &[one_heart, one_spade, two_spades, three_spades],
        opener_accept_limit_raise(Suit::Spades),
    );
    super::slam::install_rkcb(book, &after_two_spades, Suit::Spades);

    // Opener's 3♠ jump raise (16–18, four-card support): sign-off or RKCB.
    let after_three_spades = [one_heart, one_spade, three_spades];
    insert_uncontested(book, &after_three_spades, responder_after_spade_jump());
    super::slam::install_rkcb(book, &after_three_spades, Suit::Spades);

    // Opener's 2♥ rebid (own suit, 6+): invite/sign-off, and the 2NT relay.
    insert_uncontested(
        book,
        &[one_heart, one_spade, two_hearts],
        responder_after_heart_rebid(),
    );
    insert_uncontested(
        book,
        &[one_heart, one_spade, two_hearts, three_hearts],
        opener_accept_limit_raise(Suit::Hearts),
    );
    insert_uncontested(
        book,
        &[one_heart, one_spade, two_hearts, two_nt],
        opener_after_heart_invite(),
    );

    // Opener's 2♣/2♦ new minor (4+, minimum-ish): preference, invite, or game.
    for minor in [Suit::Clubs, Suit::Diamonds] {
        let two_m = call(2, Strain::from(minor));
        let three_m = call(3, Strain::from(minor));
        insert_uncontested(
            book,
            &[one_heart, one_spade, two_m],
            responder_after_minor_rebid(minor),
        );
        insert_uncontested(
            book,
            &[one_heart, one_spade, two_m, two_nt],
            opener_accept_notrump_invite(),
        );
        insert_uncontested(
            book,
            &[one_heart, one_spade, two_m, three_m],
            opener_accept_minor_raise(),
        );
        insert_uncontested(
            book,
            &[one_heart, one_spade, two_m, three_hearts],
            opener_accept_limit_raise(Suit::Hearts),
        );
    }

    // Fourth suit forcing (opt-in, rides this adjunct): `2♦` at
    // `1♥ – 1♠ – 2♣` is an artificial game force; opener answers naturally
    // below game, and responder places the final contract over any answer.
    if fourth_suit_forcing() {
        let two_clubs = call(2, Strain::Clubs);
        let two_diamonds = call(2, Strain::Diamonds);
        let after_fourth_suit = [one_heart, one_spade, two_clubs, two_diamonds];
        insert_uncontested(book, &after_fourth_suit, opener_after_fourth_suit());

        let answers: Vec<Call> = {
            let mut seen = std::collections::HashSet::new();
            opener_after_fourth_suit()
                .rules()
                .iter()
                .filter_map(|r| {
                    let c = r.call();
                    if seen.insert(c) { Some(c) } else { None }
                })
                .collect()
        };
        for answer in answers {
            insert_uncontested(
                book,
                &[one_heart, one_spade, two_clubs, two_diamonds, answer],
                responder_after_fourth_suit_answer(),
            );
        }
    }
}

/// Register opener's rebids after a one-level new suit and the forcing 1NT
pub(super) fn register(book: &mut Trie) {
    register_forcing_notrump_continuations(book);
    register_major_jump_rebid_continuations(book);
    register_forcing_nt_two_suiter_continuations(book);
    register_meckstroth_2nt_continuations(book);
    insert_uncontested(
        book,
        &[call(1, Strain::Hearts), call(1, Strain::Spades)],
        rebid_one_heart_one_spade(),
    );
    register_major_rebid_tails(book);
    for major in [Suit::Hearts, Suit::Spades] {
        insert_uncontested(
            book,
            &[call(1, Strain::from(major)), call(1, Strain::Notrump)],
            rebid_after_forcing_notrump(major),
        );
    }
    insert_uncontested(
        book,
        &[call(1, Strain::Clubs), call(1, Strain::Diamonds)],
        rebid_one_club_one_diamond(),
    );
    for minor in [Suit::Clubs, Suit::Diamonds] {
        for responder_major in [Suit::Hearts, Suit::Spades] {
            insert_uncontested(
                book,
                &[
                    call(1, Strain::from(minor)),
                    call(1, Strain::from(responder_major)),
                ],
                rebid_raise_major(responder_major, minor),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::System;
    use contract_bridge::Hand;
    use contract_bridge::auction::RelativeVulnerability;

    /// Build a Trie with the major-rebid-tails adjunct on but
    /// fourth-suit-forcing off, then restore both knobs to their (on)
    /// defaults (mirrors `slam::tests::rkcb_trie`).
    fn tails_trie() -> Trie {
        set_major_rebid_tails(true);
        set_fourth_suit_forcing(false);
        let mut trie = Trie::new();
        register_major_rebid_tails(&mut trie);
        set_fourth_suit_forcing(true);
        trie
    }

    /// Build a Trie with both the major-rebid-tails and fourth-suit-forcing
    /// knobs on (the shipped defaults).
    fn fsf_trie() -> Trie {
        set_major_rebid_tails(true);
        set_fourth_suit_forcing(true);
        let mut trie = Trie::new();
        register_major_rebid_tails(&mut trie);
        trie
    }

    /// Build the full rebid Trie with the opener extras ladder on (the shipped
    /// default).
    fn ladder_trie() -> Trie {
        set_opener_extras_ladder(true);
        let mut trie = Trie::new();
        register(&mut trie);
        trie
    }

    /// The raw table auction `[1♦, P, 1♠, P]` (opener to rebid).
    const AFTER_1D_1S: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Diamonds)),
        Call::Pass,
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Pass,
    ];

    #[test]
    fn opener_extras_ladder_shows_strength() {
        let trie = ladder_trie();
        let b = |hand| best(&trie, AFTER_1D_1S, hand);
        // Self-sufficient 6+ diamonds, 16 HCP → jump-rebid 3♦.
        assert_eq!(
            b("653.K3.AKQT854.A"),
            Call::Bid(Bid::new(3, Strain::Diamonds))
        );
        // 5♦ 4♥, 18 HCP → jump-shift 3♥ (game-forcing two-suiter).
        assert_eq!(
            b("T64.AJ86.AKQ95.A"),
            Call::Bid(Bid::new(3, Strain::Hearts))
        );
        // 5-5 in the minors, 18 HCP → jump-shift 3♣.
        assert_eq!(b("K.62.AQJ94.AKJ85"), Call::Bid(Bid::new(3, Strain::Clubs)));
        // A dead minimum still takes the natural 2♦ rebid.
        assert_eq!(
            b("K54.Q3.KJ8542.32"),
            Call::Bid(Bid::new(2, Strain::Diamonds))
        );
    }

    #[test]
    fn opener_extras_ladder_reverts_when_off() {
        set_opener_extras_ladder(false);
        let mut trie = Trie::new();
        register(&mut trie);
        set_opener_extras_ladder(true);
        // Knob off: the 16-count monster reverts to the minimum 2♦ rebid.
        assert_eq!(
            best(&trie, AFTER_1D_1S, "653.K3.AKQT854.A"),
            Call::Bid(Bid::new(2, Strain::Diamonds))
        );
    }

    /// Build the full rebid Trie with opener's major jump-rebid rung on (the
    /// shipped default).
    fn major_jump_trie() -> Trie {
        set_opener_major_jump_rebid(true);
        let mut trie = Trie::new();
        register(&mut trie);
        trie
    }

    /// The raw table auction `[1♥, P, 1♠, P]` (opener to rebid).
    const AFTER_1H_1S: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Pass,
    ];

    /// The raw table auction `[1♠, P, 1NT, P]` (opener rebids over forcing 1NT).
    const AFTER_1S_1NT: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Pass,
        Call::Bid(Bid::new(1, Strain::Notrump)),
        Call::Pass,
    ];

    #[test]
    fn opener_major_jump_rebid_shows_strength() {
        let trie = major_jump_trie();
        // 6+ hearts, 16 HCP, no spade fit → jump-rebid 3♥.
        assert_eq!(
            best(&trie, AFTER_1H_1S, "3.AKQJ72.KQ5.J54"),
            Call::Bid(Bid::new(3, Strain::Hearts))
        );
        // A minimum 6-card heart hand still takes the natural 2♥.
        assert_eq!(
            best(&trie, AFTER_1H_1S, "A2.KQ9872.Q43.J5"),
            Call::Bid(Bid::new(2, Strain::Hearts))
        );
        // The forcing-1NT node carries the same rung: 1♠ – 1NT – 3♠.
        assert_eq!(
            best(&trie, AFTER_1S_1NT, "AKQJ72.3.KQ5.J54"),
            Call::Bid(Bid::new(3, Strain::Spades))
        );
    }

    #[test]
    fn opener_major_jump_rebid_reverts_when_off() {
        // Knob off: the 16-count 6-heart hand reverts to the minimum 2♥ rebid.
        set_opener_major_jump_rebid(false);
        let mut trie = Trie::new();
        register(&mut trie);
        set_opener_major_jump_rebid(true);
        assert_eq!(
            best(&trie, AFTER_1H_1S, "3.AKQJ72.KQ5.J54"),
            Call::Bid(Bid::new(2, Strain::Hearts))
        );
    }

    /// `[1♥, P, 1♠, P, 3♥, P]` — responder to act over opener's jump-rebid.
    const AFTER_1H_1S_3H: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Pass,
        Call::Bid(Bid::new(3, Strain::Hearts)),
        Call::Pass,
    ];

    /// `[1♠, P, 1NT, P, 3♠, P]` — responder to act over opener's jump-rebid.
    const AFTER_1S_1NT_3S: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Pass,
        Call::Bid(Bid::new(1, Strain::Notrump)),
        Call::Pass,
        Call::Bid(Bid::new(3, Strain::Spades)),
        Call::Pass,
    ];

    #[test]
    fn responder_accepts_major_jump_rebid() {
        let trie = major_jump_trie();
        // Fit (3 hearts) + values → raise to game 4♥.
        assert_eq!(
            best(&trie, AFTER_1H_1S_3H, "KQ85.K76.542.J43"),
            Call::Bid(Bid::new(4, Strain::Hearts))
        );
        // No heart fit + values → notrump game 3NT.
        assert_eq!(
            best(&trie, AFTER_1H_1S_3H, "KQ85.6.KJ43.Q642"),
            Call::Bid(Bid::new(3, Strain::Notrump))
        );
        // Minimum → pass the invitational jump, play 3♥.
        assert_eq!(best(&trie, AFTER_1H_1S_3H, "Q985.42.J8532.K4"), Call::Pass);
        // Forcing-1NT node: a doubleton spade fit (8 cards) + values → 4♠.
        assert_eq!(
            best(&trie, AFTER_1S_1NT_3S, "87.KQ86.KJ43.T92"),
            Call::Bid(Bid::new(4, Strain::Spades))
        );
    }

    /// The highest-logit call the trie makes for a hand at an auction
    fn best(trie: &Trie, auction: &[Call], hand: &str) -> Call {
        let hand: Hand = hand.parse().expect("valid test hand");
        let logits = trie
            .classify(hand, RelativeVulnerability::NONE, auction)
            .expect("trie covers this auction");
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("logits array is never empty")
    }

    /// The raw table auction `[1♥, P, 1♠, P, 2♠, P]` (opener in seat 1).
    const AFTER_2S: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Spades)),
        Call::Pass,
    ];

    /// The raw table auction `[1♥, P, 1♠, P, 2♥, P]`.
    const AFTER_2H: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Hearts)),
        Call::Pass,
    ];

    /// The raw table auction `[1♥, P, 1♠, P, 2♥, P, 2NT, P]`.
    const AFTER_2H_2NT: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Notrump)),
        Call::Pass,
    ];

    /// The raw table auction `[1♥, P, 1♠, P, 2♣, P]`.
    const AFTER_2C: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Clubs)),
        Call::Pass,
    ];

    /// The raw table auction `[1♥, P, 1♠, P, 2♣, P, 2♦, P]`
    /// (fourth-suit-forcing).
    const AFTER_2C_2D: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Clubs)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Diamonds)),
        Call::Pass,
    ];

    /// The raw table auction `[1♥, P, 1♠, P, 2♣, P, 2♦, P, 2♠, P]`.
    const AFTER_2C_2D_2S: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Clubs)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Diamonds)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Spades)),
        Call::Pass,
    ];

    /// The raw table auction `[1♥, P, 1♠, P, 2♣, P, 2♦, P, 2NT, P]`.
    const AFTER_2C_2D_2NT: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Clubs)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Diamonds)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Notrump)),
        Call::Pass,
    ];

    /// The off state: `register` inserts nothing with the knob off.
    #[test]
    fn off_state_inserts_nothing() {
        set_major_rebid_tails(false);
        let mut trie = Trie::new();
        register_major_rebid_tails(&mut trie);
        set_major_rebid_tails(true); // restore the shipped default
        let hand: Hand = "K432.AQ5.432.Q32".parse().expect("valid test hand");
        assert!(
            trie.classify(hand, RelativeVulnerability::NONE, AFTER_2S)
                .is_none(),
            "the adjunct must insert zero nodes while off"
        );
    }

    /// C1: responder's second call after opener's `2♠` raise picks by points.
    #[test]
    fn spade_raise_responder_picks_by_points() {
        let trie = tails_trie();

        // A432.KQ5.K54.J32 — 13 points, balanced (4333) -> accept to game.
        assert_eq!(
            best(&trie, AFTER_2S, "A432.KQ5.K54.J32"),
            Call::Bid(Bid::new(4, Strain::Spades)),
            "13 points -> 4♠"
        );
        // A432.K542.Q54.J3 — 10 points (4-4-3-2) -> invitational raise.  A
        // flat 4333 10-count reads 9 on the shipped scale and passes.
        assert_eq!(
            best(&trie, AFTER_2S, "A432.K542.Q54.J3"),
            Call::Bid(Bid::new(3, Strain::Spades)),
            "10 points -> 3♠"
        );
        // A432.432.Q54.J32 — 7 points, balanced -> pass.
        assert_eq!(
            best(&trie, AFTER_2S, "A432.432.Q54.J32"),
            Call::Pass,
            "7 points -> pass"
        );
    }

    /// C4: responder's second call after opener's `2♥` rebid prefers the fit.
    #[test]
    fn heart_rebid_responder_prefers_the_fit() {
        let trie = tails_trie();

        // K987.AQ.9876.Q32 — 11 points, 2 hearts -> invite in hearts, beats 2NT.
        assert_eq!(
            best(&trie, AFTER_2H, "K987.AQ.9876.Q32"),
            Call::Bid(Bid::new(3, Strain::Hearts)),
            "11 points, 2 hearts -> 3♥ beats 2NT"
        );
        // AJ98.K.7654.QJ87 — 11 points, singleton heart -> the notrump invite.
        assert_eq!(
            best(&trie, AFTER_2H, "AJ98.K.7654.QJ87"),
            Call::Bid(Bid::new(2, Strain::Notrump)),
            "11 points, 1 heart -> 2NT"
        );
    }

    /// C6: opener's call over responder's `2NT` invite goes by raw HCP.
    #[test]
    fn opener_answers_the_heart_invite_by_hcp() {
        let trie = tails_trie();

        // AK32.KQ32.A32.32 — 16 HCP -> accept with 3NT.
        assert_eq!(
            best(&trie, AFTER_2H_2NT, "AK32.KQ32.A32.32"),
            Call::Bid(Bid::new(3, Strain::Notrump)),
            "16 HCP -> 3NT"
        );
        // K432.KQ32.Q32.Q3 — 12 HCP -> decline with the 3♥ retreat.
        assert_eq!(
            best(&trie, AFTER_2H_2NT, "K432.KQ32.Q32.Q3"),
            Call::Bid(Bid::new(3, Strain::Hearts)),
            "12 HCP -> 3♥ retreat"
        );
    }

    /// C7: responder's second call after opener's `2♣` rebid picks by weight.
    #[test]
    fn minor_rebid_responder_picks_by_weight() {
        let trie = tails_trie();

        // K432.AQ5.432.Q32 — 11 points, 3 hearts -> the jump preference
        // outranks the 2NT invite (both are live at 10-12 points).
        assert_eq!(
            best(&trie, AFTER_2C, "K432.AQ5.432.Q32"),
            Call::Bid(Bid::new(3, Strain::Hearts)),
            "11 points, 3 hearts -> 3♥"
        );
        // K432.98.76.AQJ54 — 11 points (10 HCP + unbalanced upgrade), 5 clubs,
        // only 2 hearts (no 3-heart holding) -> raise opener's minor.
        assert_eq!(
            best(&trie, AFTER_2C, "K432.98.76.AQJ54"),
            Call::Bid(Bid::new(3, Strain::Clubs)),
            "11 points, 5 clubs, no 3-heart holding -> 3♣"
        );
        // K432.Q3.K432.432 — 8 points, balanced, 2 hearts -> simple preference.
        assert_eq!(
            best(&trie, AFTER_2C, "K432.Q3.K432.432"),
            Call::Bid(Bid::new(2, Strain::Hearts)),
            "8 points, 2 hearts -> 2♥"
        );
        // K98765.4.Q32.J32 — 6 HCP (7 points), 6 spades, singleton heart ->
        // too weak for any invite; the weak spade rebid is the only live call
        // besides pass.
        assert_eq!(
            best(&trie, AFTER_2C, "K98765.4.Q32.J32"),
            Call::Bid(Bid::new(2, Strain::Spades)),
            "weak hand, 6 spades -> 2♠"
        );
        // AK32.Q54.K54.Q32 — 14 points, balanced, no fit found -> the game route.
        assert_eq!(
            best(&trie, AFTER_2C, "AK32.Q54.K54.Q32"),
            Call::Bid(Bid::new(3, Strain::Notrump)),
            "14 points, no fit -> 3NT"
        );
    }

    /// D0: fourth-suit-forcing fires at 12+ points; below that floor the
    /// existing jump-preference table is unchanged.
    #[test]
    fn fourth_suit_forcing_fires_at_twelve_points() {
        let trie = fsf_trie();

        // AK32.Q54.K54.Q32 — 14 points, no fit found -> 2♦ fourth-suit-forcing
        // beats the old 3NT game route (weight 2.0 vs 0.9).
        assert_eq!(
            best(&trie, AFTER_2C, "AK32.Q54.K54.Q32"),
            Call::Bid(Bid::new(2, Strain::Diamonds)),
            "14 points -> 2♦ fourth-suit-forcing"
        );
        // K432.AQ5.432.Q32 — 11 points, 3 hearts -> below the 12-point FSF
        // floor, so the jump preference to 3♥ still wins.
        assert_eq!(
            best(&trie, AFTER_2C, "K432.AQ5.432.Q32"),
            Call::Bid(Bid::new(3, Strain::Hearts)),
            "11 points, 3 hearts -> 3♥ (below the FSF floor)"
        );
    }

    /// D1: opener's answer to the fourth-suit-forcing game force picks by
    /// weight; the guaranteed-legal `2♥` catches every remaining hand.
    #[test]
    fn fourth_suit_forcing_opener_answers_by_weight() {
        let trie = fsf_trie();

        // KQ4.AJ76.A32.987 — 3 spades and a diamond stopper (the ace) ->
        // the delayed raise (1.4) beats the notrump answer (1.2).
        assert_eq!(
            best(&trie, AFTER_2C_2D, "KQ4.AJ76.A32.987"),
            Call::Bid(Bid::new(2, Strain::Spades)),
            "3 spades + diamond stopper -> 2♠ beats 2NT"
        );
        // 98.K8765.432.876 — no 3-card spade support, no 6th heart, no
        // diamond stopper, no 5-card club suit -> the guaranteed-legal
        // catch-all.
        assert_eq!(
            best(&trie, AFTER_2C_2D, "98.K8765.432.876"),
            Call::Bid(Bid::new(2, Strain::Hearts)),
            "none of the above -> 2♥ catch-all"
        );
    }

    /// D2: responder places the contract at game over opener's answer.
    #[test]
    fn fourth_suit_forcing_responder_places_the_contract() {
        let trie = fsf_trie();

        // AKJ87.654.32.432 — 5 spades after opener's 2♠ answer -> 4♠.
        assert_eq!(
            best(&trie, AFTER_2C_2D_2S, "AKJ87.654.32.432"),
            Call::Bid(Bid::new(4, Strain::Spades)),
            "5 spades after 2♠ -> 4♠"
        );
        // Q432.KJ8.654.J32 — 3 hearts after opener's 2NT answer -> 4♥.
        assert_eq!(
            best(&trie, AFTER_2C_2D_2NT, "Q432.KJ8.654.J32"),
            Call::Bid(Bid::new(4, Strain::Hearts)),
            "3 hearts after 2NT -> 4♥"
        );
        // Q432.K8.6543.J32 — neither a spade fit nor 3 hearts -> 3NT.
        assert_eq!(
            best(&trie, AFTER_2C_2D_2NT, "Q432.K8.6543.J32"),
            Call::Bid(Bid::new(3, Strain::Notrump)),
            "neither fit -> 3NT"
        );
    }

    /// Fourth-suit-forcing rides the major-rebid-tails adjunct: with tails
    /// off, turning FSF on still inserts nothing (the whole adjunct — not
    /// just FSF's own nodes — is gated by `major_rebid_tails()` first).
    #[test]
    fn fourth_suit_forcing_without_tails_inserts_nothing() {
        set_major_rebid_tails(false);
        set_fourth_suit_forcing(true);
        let mut trie = Trie::new();
        register_major_rebid_tails(&mut trie);
        set_major_rebid_tails(true); // restore the shipped defaults

        let hand: Hand = "K432.AQ5.432.Q32".parse().expect("valid test hand");
        assert!(
            trie.classify(hand, RelativeVulnerability::NONE, AFTER_2C)
                .is_none(),
            "fourth-suit-forcing must not register without the tails adjunct"
        );
    }

    #[test]
    fn nt_invite_hcp_gauges_the_no_fit_rung() {
        // 1♥ – 1♠ – 2♦ (the remnant report's 2NT-invite seam): a 9-HCP
        // six-spade hand reads 10 points and invites 2NT by default — a
        // notrump invite priced in ruffs it will never take.  HCP-gauged it
        // takes the weak 2♠ rebid instead; a flat-ish 10-count invites on
        // either gauge.
        let after_2d: &[Call] = &[
            Call::Bid(Bid::new(1, Strain::Hearts)),
            Call::Pass,
            Call::Bid(Bid::new(1, Strain::Spades)),
            Call::Pass,
            Call::Bid(Bid::new(2, Strain::Diamonds)),
            Call::Pass,
        ];
        let shaped = "KT8642.7.QJ4.QJ3"; // 9 HCP, 10 points
        let flat = "AT86.97.QJ42.QJ3"; // 10 HCP, 10 points
        let two_nt = Call::Bid(Bid::new(2, Strain::Notrump));

        let default_trie = fsf_trie();
        assert_eq!(
            best(&default_trie, after_2d, shaped),
            Call::Bid(Bid::new(2, Strain::Spades)),
            "default (HCP-gauged): the shaped 9 takes the weak rebid"
        );
        assert_eq!(
            best(&default_trie, after_2d, flat),
            two_nt,
            "a real 10-count still invites"
        );

        set_nt_invite_hcp(false);
        let legacy_trie = fsf_trie();
        set_nt_invite_hcp(true);
        assert_eq!(
            best(&legacy_trie, after_2d, shaped),
            two_nt,
            "the points gauge (off arm) invites the shaped 9"
        );
        assert_eq!(best(&legacy_trie, after_2d, flat), two_nt);
    }

    /// The fourth-suit-forcing `2♦` rule carries the alert.
    #[test]
    fn fourth_suit_forcing_rule_is_alerted() {
        set_fourth_suit_forcing(true);
        let rules = responder_after_minor_rebid(Suit::Clubs);

        let fsf_rule = rules
            .rules()
            .iter()
            .find(|r| r.call() == Call::Bid(Bid::new(2, Strain::Diamonds)))
            .expect("the fourth-suit-forcing rule is present");
        assert!(
            fsf_rule.alert().is_some(),
            "fourth-suit-forcing must carry an alert"
        );
    }

    /// After `1♦ – 1♥`, a balanced 12–14 with a five-card diamond suit rebids
    /// the natural `2♦` by default but `1NT` once `set_balanced_1nt_rebid` is
    /// on — the only shape the knob moves (4333/4432 hold no five-card minor).
    #[test]
    fn balanced_1nt_rebid_knob_flips_2m_to_1nt() {
        let one_d_one_h = &[
            call(1, Strain::Diamonds),
            Call::Pass,
            call(1, Strain::Hearts),
            Call::Pass,
        ];
        // ♠KQ4 ♥Q3 ♦AK762 ♣853 — 3=2=5=3, 14 HCP, no four-card heart support.
        let hand = "KQ4.Q3.AK762.853";
        let build = || {
            let mut trie = Trie::new();
            insert_uncontested(
                &mut trie,
                &[call(1, Strain::Diamonds), call(1, Strain::Hearts)],
                rebid_raise_major(Suit::Hearts, Suit::Diamonds),
            );
            trie
        };

        set_balanced_1nt_rebid(false);
        assert_eq!(best(&build(), one_d_one_h, hand), call(2, Strain::Diamonds));

        set_balanced_1nt_rebid(true); // the shipped default
        let on = build();
        assert_eq!(best(&on, one_d_one_h, hand), call(1, Strain::Notrump));
    }
}
