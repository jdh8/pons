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
    /// Whether opener's rebid tables carry the **Meckstroth adjunct**: the
    /// invitational `3m` jumps (`1M – 1NT – 3m` and `1♥ – 1♠ – 3m`) and their
    /// responder continuations.  On by default.
    static MECKSTROTH: Cell<bool> = const { Cell::new(true) };
}

/// Enable or disable the Meckstroth adjunct in books built *after* this call
///
/// Read at book-construction time (during `register`); set it before building
/// the `Pair`.  The default is on.  Used by the `meckstroth-abc` A/B example to
/// build a baseline arm (off) and a treatment arm (on).
pub fn set_meckstroth_adjunct(on: bool) {
    MECKSTROTH.with(|cell| cell.set(on));
}

/// Whether the Meckstroth adjunct is currently enabled
fn meckstroth() -> bool {
    MECKSTROTH.with(Cell::get)
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
/// narrowing each rung's shape and strength.  Only the two minor-opening rebid
/// nodes carry it so far; the major-opening nodes (a Meckstroth `3m` collision)
/// are a follow-up.
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
    let mut rules = Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            fifths(18.0..20.0) & balanced(),
        )
        .rule(Bid::new(2, trump), 1.0, len(major, 6..));
    // Meckstroth adjunct: invitational 3♣/3♦ jumps with a five-card minor.
    rules = with_invitational_minors(rules);
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
    rules
        .rule(
            Bid::new(3, Strain::Hearts),
            1.3,
            len(Suit::Hearts, 3..) & points(10..=12),
        )
        .rule(Bid::new(3, m), 1.25, len(minor, 5..) & points(10..=12))
        .rule(Bid::new(2, Strain::Notrump), 1.2, points(10..=12))
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
        // A432.K54.Q54.J32 — 10 points, balanced -> invitational raise.
        assert_eq!(
            best(&trie, AFTER_2S, "A432.K54.Q54.J32"),
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
