//! Responses to weak-two openings in the 2/1 game-forcing system
//!
//! Covers three auctions for each weak-two suit M ∈ {♦, ♥, ♠}:
//!
//! 1. **Responder's first bid** (`2M–?`): Ogust 2NT, preemptive raises, and a
//!    one-round-forcing new suit.
//! 2. **Opener's Ogust answers** (`2M–2NT–?`): a conventional five-rung ladder
//!    encoding suit quality and points range.
//! 3. **Asker's Ogust continuations** (`2M–2NT–<answer>–?`): sign-off or game
//!    based on what opener revealed.
//! 4. **Opener's reply to a forcing new suit** (`2M–<new suit>–?`): raise or
//!    rebid.
//!
//! # Raises are to play (RONF)
//!
//! Direct raises of the weak two are non-forcing by design: `2M–3M` and
//! `2M–4M` are both pre-emptive, not invitational.  Hands with game interest
//! and fit use Ogust (2NT) instead.
//!
//! # Forcing by omission
//!
//! Ogust answers carry no [`Pass`][contract_bridge::auction::Call::Pass] rule
//! — the auction is forced after 2NT.  Asker's continuations after a *max*
//! Ogust answer also omit pass (opener showed maximum values; game is
//! obligatory).

use std::cell::Cell;

use super::{call, insert_uncontested};
use crate::bidding::constraint::{hcp, len, longest_unbid, points, suit_hcp, support, top_honors};
use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

thread_local! {
    /// Whether a good five-card major outranks the Ogust ask over a weak 2♦.
    /// Default `true` — off restores the pre-repair Ogust priority.
    static WEAK_TWO_MAJOR_PRIORITY: Cell<bool> = const { Cell::new(true) };
    /// Whether responder's forcing new suit must be their longest.  Default
    /// `true` — off it is the pre-repair argmax race.
    static WEAK_TWO_LONGEST_FIRST: Cell<bool> = const { Cell::new(true) };
}

/// Show the **longest** suit with responder's forcing new suit over a weak two
/// (default `true`; off restores the pre-repair argmax race)
///
/// The new-suit rules all carry weight 1.5, so before the repair the winner was
/// decided by `Table::next_call`'s tie-break — descending sort, first *legal*
/// call — i.e. the **cheapest** bid.  ♠AKT862 ♥AKJ92 therefore responded 2♥ to
/// a weak 2♦, suppressing the longer and higher suit.  On, each rule gains
/// `longest_unbid`, making the choice a constraint: longest first, an
/// equal-length tie to the higher rank.  Same doctrine as
/// [`set_longest_first_advance`][super::set_longest_first_advance] on the
/// advance side, and like it the condition is a `shapes` partition, so knob-on
/// the *reading* also pins the relative length.
///
/// Default-on as a doctrine repair, not a measured win: the collision needs two
/// qualifying five-card suits opposite a weak two, roughly one board in 10⁴, so
/// a random-deal A/B cannot resolve it.  The knob exists to ablate it in the
/// enriched probe (`examples/probe-weak-two-major --mode tie`).
pub fn set_weak_two_longest_first(on: bool) {
    WEAK_TWO_LONGEST_FIRST.with(|cell| cell.set(on));
}

fn weak_two_longest_first() -> bool {
    WEAK_TWO_LONGEST_FIRST.with(Cell::get)
}

/// Bid a good five-card major over partner's weak 2♦ instead of asking Ogust
/// (default `true`; off restores the pre-repair Ogust priority)
///
/// The old node ranked Ogust 2NT (weight 2.0) above every new suit (1.5), so
/// a hand like ♠AKT862 ♥AKJ92 ♦xx asks about diamond quality rather than
/// showing eleven cards in the majors — the major only escapes when responder
/// is short enough in diamonds to fail Ogust's `support(2..)`.  Knob-on, the
/// two major rules outrank Ogust **over 2♦ only**: there 2♥/2♠ is *cheaper*
/// than 2NT and forcing, so the ask is deferred rather than lost.  Over 2♥/2♠
/// a new suit costs 2♠ or the three level, so Ogust keeps priority.
///
/// Expressed as a weight, not as a gate on the Ogust rule: `top_honors` is part
/// of the new-suit gate, so excluding "any five-card major" from Ogust would
/// strand 14+ hands like ♠QJxxx ♦xx, which have no new-suit rule at all.  The
/// cost is that 2NT's projected reading still promises only "14+, 2+♦" and does
/// not deny a major.
///
/// Default-on on measurement: the enriched probe
/// (`examples/probe-weak-two-major --mode ogust`, 20 000 accepted deals,
/// 0.069% trigger density, 85% divergent) scored **+3.048 IMPs/accepted deal**
/// under perfect defense, CI [+2.911, +3.184], and **+1.668** under plain DD,
/// CI [+1.563, +1.773] — a gain under both scorings, so ≈+0.0021 IMPs/board.
pub fn set_weak_two_major_priority(on: bool) {
    WEAK_TWO_MAJOR_PRIORITY.with(|cell| cell.set(on));
}

fn weak_two_major_priority() -> bool {
    WEAK_TWO_MAJOR_PRIORITY.with(Cell::get)
}

// ---------------------------------------------------------------------------
// First response to 2M
// ---------------------------------------------------------------------------

/// Responder's first-round options over a weak-two opening in `our`
///
/// Priorities, highest first:
///
/// - **2NT** (Ogust ask, weight 2.0): at least two-card support and opening
///   values; asks opener to describe suit quality and HCP range on the
///   five-rung ladder.
/// - **Game raise** (weight 1.3): four-plus-card support, pre-emptive.  Uses
///   4♦ for M = ♦.
/// - **Forcing new suit** (weight 1.5): the **longest** five-card-plus suit
///   with two of the top three honors and opening values; one-round force.
///   Higher-ranking suits are bid at the two level; lower-ranking suits at the
///   three level.  Under [`set_weak_two_major_priority`] a major outranks Ogust
///   over 2♦.
/// - **Simple raise** (weight 1.2): three-plus-card support, preemptive.
/// - **Pass** (weight 0.0): catch-all.
#[must_use]
pub(super) fn responses(our: Suit) -> Rules {
    let trump = Strain::from(our);
    let mut rules = Rules::new()
        // Ogust 2NT: at least two-card support and opening values.
        .rule(
            Bid::new(2, Strain::Notrump),
            2.0,
            points(14..) & support(2..),
        )
        // Pre-emptive game raise.
        .rule(Bid::new(4, trump), 1.3, support(4..))
        // Pre-emptive simple raise (RONF — raise is to play, not invitational).
        .rule(Bid::new(3, trump), 1.2, support(3..))
        // Pass: the catch-all.
        .rule(Call::Pass, 0.0, hcp(0..));

    // Forcing new suits: each suit other than `our`, with a natural two-level
    // bid (if the suit ranks higher) or three-level bid (if lower).
    //
    // `longest_unbid` makes the choice among them a constraint rather than an
    // argmax race: the rules all shared weight 1.5, so `Table::next_call` broke
    // the tie by *cheapest* and 5-5 majors showed the lower one — ♠AKT862
    // ♥AKJ92 responded 2♥ over 2♦.  Doctrine (shared with the advance side):
    // longest first, an equal-length tie to the higher rank.
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x == our {
            continue;
        }
        let level: u8 = if Strain::from(x) > trump { 2 } else { 3 };
        // Over 2♦ only, and only for the majors, the knob lifts the new suit
        // above the 2.0 Ogust ask.
        let weight =
            if weak_two_major_priority() && our == Suit::Diamonds && Strain::from(x) > trump {
                2.1
            } else {
                1.5
            };
        let gate = len(x, 5..) & top_honors(x, 2..) & points(14..);
        rules = if weak_two_longest_first() {
            rules.rule(
                Bid::new(level, Strain::from(x)),
                weight,
                gate & longest_unbid(x, our),
            )
        } else {
            rules.rule(Bid::new(level, Strain::from(x)), weight, gate)
        };
    }
    rules
}

// ---------------------------------------------------------------------------
// Opener's Ogust answers
// ---------------------------------------------------------------------------

/// Opener's answers to the Ogust 2NT inquiry after a weak-two in `our`
///
/// The five-rung ladder encodes points range and suit quality regardless of
/// which suit was opened:
///
/// | Call | Meaning | Weight |
/// |------|---------|--------|
/// | 3NT  | Solid suit (A-K-Q) | 1.5 |
/// | 3♣   | Minimum points, bad suit (< 5 HCP in trumps) | 1.0 |
/// | 3♦   | Minimum points, good suit (≥ 5 HCP in trumps) | 1.0 |
/// | 3♥   | Maximum points, bad suit | 1.0 |
/// | 3♠   | Maximum points, good suit | 1.0 |
///
/// A safety fallback of 3♣ at weight 0.2 guarantees a legal call when none
/// of the crisp rules fire (which should not happen with a legitimate weak
/// two, but is required by the "forcing by omission" model).
#[must_use]
fn ogust_answers(our: Suit) -> Rules {
    // Min (5–7) / max (8–10) points, each split bad-suit / good-suit.
    //
    // The strength gauge is `points` (rule-of-N+8) not raw HCP: responder's 2NT
    // promised 2+ support, so the fit is known here and opener re-evaluates with
    // the 6th trump and side shape credited — unlike the fit-unknown opening gate
    // (`set_weak_two_hcp`), which is disciplined in raw HCP.  BBA splits min/max
    // on raw HCP 7/8; we deliberately keep ours.
    //
    // The *quality* gate is BBA's, probed exactly (`probe-bba-constraints
    // --mode weak2-{d,h,s}`): "good suit" is trump HCP ≥ 5, which separates its
    // good and bad rungs with zero overlap on all three openings.  Rival
    // predicates do not: A/K/Q count overlaps at 1 (good 28, bad 202) and
    // A/K/Q/J at 2.  Our old `top_honors(our, 2..)` was a strict subset (KQ = 5
    // is the cheapest two of A/K/Q), so the sole hands that change rung are
    // A-J-x-x-x-x — 4 + 1 = 5 HCP with only one top honor.
    Rules::new()
        // Solid six-card suit (A-K-Q present): bid 3NT.  Matches BBA exactly.
        .rule(Bid::new(3, Strain::Notrump), 1.5, top_honors(our, 3..))
        // Minimum values (5–7 points), bad suit (under 5 HCP in trumps).
        .rule(
            Bid::new(3, Strain::Clubs),
            1.0,
            points(5..=7) & suit_hcp(our, ..5),
        )
        // Minimum values, good suit (5+ HCP in trumps).
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.0,
            points(5..=7) & suit_hcp(our, 5..),
        )
        // Maximum values (8–10 points), bad suit.
        .rule(
            Bid::new(3, Strain::Hearts),
            1.0,
            points(8..=10) & suit_hcp(our, ..5),
        )
        // Maximum values, good suit.
        .rule(
            Bid::new(3, Strain::Spades),
            1.0,
            points(8..=10) & suit_hcp(our, 5..),
        )
        // Safety fallback: guarantees a legal response for any legitimate
        // weak-two hand even when the crisp constraints leave a gap.
        .rule(Bid::new(3, Strain::Clubs), 0.2, hcp(0..))
}

// ---------------------------------------------------------------------------
// Asker's continuations after Ogust answers (majors)
// ---------------------------------------------------------------------------

/// Asker's continuation after a *minimum* Ogust answer (3♣ or 3♦) to a
/// major-suit weak two in `our`
///
/// Opener showed a sub-maximum hand.  With 17+ HCP the asker can still make
/// game; otherwise a sign-off in three-of-the-major ends the auction.
///
/// The 3M sign-off may be dead if the Ogust answer already passed 3M; the
/// "forcing by omission" model leaves it dead (scoring −∞) rather than
/// escalating to 4M.
#[must_use]
fn asker_after_min_major(our: Suit) -> Rules {
    let trump = Strain::from(our);
    Rules::new()
        // With game-going values, push to game anyway.
        .rule(Bid::new(4, trump), 1.0, points(17..))
        // Sign-off at three: opener was minimum, asker is short of game force.
        .rule(Bid::new(3, trump), 0.5, hcp(0..))
}

/// Asker's continuation after a *maximum* Ogust answer (3♥ or 3♠) to a
/// major-suit weak two in `our`
///
/// Opener showed maximum weak-two values.  Since the asker held at least 14
/// HCP to invoke Ogust, the combined count is enough for game — bid it.
#[must_use]
fn asker_after_max_major(our: Suit) -> Rules {
    let trump = Strain::from(our);
    // Forcing node (no pass): opener showed maximum, game is in range.
    Rules::new().rule(Bid::new(4, trump), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Asker's continuations after Ogust answers (diamonds)
// ---------------------------------------------------------------------------

/// Asker's continuation after the minimum-bad (3♣) Ogust answer to 2♦
///
/// 3NT on a power hand; otherwise 3♦ as sign-off.
#[must_use]
fn asker_after_diamonds_min_bad() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(17..))
        .rule(Bid::new(3, Strain::Diamonds), 0.5, hcp(0..))
}

/// Asker's continuation after the minimum-good (3♦) Ogust answer to 2♦
///
/// 3NT on a power hand; otherwise pass to play 3♦.
#[must_use]
fn asker_after_diamonds_min_good() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(17..))
        // Pass: 3♦ is already on the table; sign off there.
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Asker's continuation after a *maximum* Ogust answer (3♥ or 3♠) to 2♦
///
/// Opener showed maximum weak-two values.  With game-going HCP, bid 5♦;
/// otherwise 3NT is the practical game in a safe major suit.
#[must_use]
fn asker_after_diamonds_max() -> Rules {
    // Forcing node: game is in range given combined values.
    Rules::new()
        .rule(Bid::new(5, Strain::Diamonds), 1.0, points(17..))
        .rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Opener's reply to a forcing new suit
// ---------------------------------------------------------------------------

/// Opener's reply when responder bids a forcing new suit `x` over weak two `our`
///
/// The response was at `response_level`.  Opener either raises `x` at the
/// cheapest available level (`response_level + 1`) with three-card support,
/// or rebids `our` at the cheapest legal level.
///
/// The rebid level for `our` is 3 when `3M > <response bid>` (i.e. the suit
/// ranks high enough to be legal), otherwise 4.
#[must_use]
fn reply_to_new_suit(our: Suit, x: Suit, response_level: u8) -> Rules {
    let raise_level = response_level + 1;
    let rebid_level: u8 =
        if Bid::new(3, Strain::from(our)) > Bid::new(response_level, Strain::from(x)) {
            3
        } else {
            4
        };

    Rules::new()
        // Raise partner's suit with adequate support.
        .rule(Bid::new(raise_level, Strain::from(x)), 1.0, support(3..))
        // Rebid our suit as the fallback description.
        .rule(Bid::new(rebid_level, Strain::from(our)), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register all weak-two response machinery into the constructive book
///
/// For each weak-two suit M ∈ {♦, ♥, ♠}:
///
/// - First responses at `[2M]`
/// - Ogust answers at `[2M, 2NT]`
/// - Asker's Ogust continuations at `[2M, 2NT, <answer>]`
/// - Opener's reply to each forcing new suit at `[2M, <new suit>]`
pub(super) fn register(book: &mut Trie) {
    for our in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let trump = Strain::from(our);
        let open = call(2, trump);

        // First responses (at [2M]).
        insert_uncontested(book, &[open], responses(our));

        // Ogust: opener's answers (at [2M, 2NT]).
        let ogust = call(2, Strain::Notrump);
        insert_uncontested(book, &[open, ogust], ogust_answers(our));

        // Asker's Ogust continuations (at [2M, 2NT, <answer>]).
        register_ogust_continuations(book, our, open, ogust);

        // Opener's reply to each forcing new suit (at [2M, <new suit>]).
        register_new_suit_replies(book, our, open);
    }
}

/// Register asker's Ogust continuations for a given weak-two suit
fn register_ogust_continuations(book: &mut Trie, our: Suit, open: Call, ogust: Call) {
    let min_bad = call(3, Strain::Clubs); // min, bad suit
    let min_good = call(3, Strain::Diamonds); // min, good suit
    let max_bad = call(3, Strain::Hearts); // max, bad suit
    let max_good = call(3, Strain::Spades); // max, good suit
    // 3NT (solid): no continuation table — pass plays 3NT.

    if our == Suit::Diamonds {
        insert_uncontested(
            book,
            &[open, ogust, min_bad],
            asker_after_diamonds_min_bad(),
        );
        insert_uncontested(
            book,
            &[open, ogust, min_good],
            asker_after_diamonds_min_good(),
        );
        for max_ans in [max_bad, max_good] {
            insert_uncontested(book, &[open, ogust, max_ans], asker_after_diamonds_max());
        }
    } else {
        // Hearts and spades share the same continuation logic.
        for min_ans in [min_bad, min_good] {
            insert_uncontested(book, &[open, ogust, min_ans], asker_after_min_major(our));
        }
        for max_ans in [max_bad, max_good] {
            insert_uncontested(book, &[open, ogust, max_ans], asker_after_max_major(our));
        }
    }
}

/// Register opener's reply to each forcing new suit over `open` (= 2M)
fn register_new_suit_replies(book: &mut Trie, our: Suit, open: Call) {
    let trump = Strain::from(our);
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x == our {
            continue;
        }
        let level: u8 = if Strain::from(x) > trump { 2 } else { 3 };
        let new_suit_call = call(level, Strain::from(x));
        insert_uncontested(
            book,
            &[open, new_suit_call],
            reply_to_new_suit(our, x, level),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::Table;
    use crate::bidding::american::american;
    use contract_bridge::auction::Auction;
    use contract_bridge::{AbsoluteVulnerability, Hand, Seat};

    /// Responder's call over `open`–P, through the **production** tie-break
    ///
    /// `Table::next_call` sorts descending and takes the first legal call, so
    /// equal-weight rules resolve to the *cheapest* bid — the opposite of the
    /// `max_by` argmax the module tests elsewhere use.  The 2♥/2♠ collision is
    /// only visible on this path.
    fn responds(open: Call, hand: &str) -> Call {
        let stance = american().against();
        let table = Table::new(
            stance.clone(),
            stance,
            Seat::North,
            AbsoluteVulnerability::NONE,
        );
        let hand: Hand = hand.parse().expect("valid test hand");
        let mut auction = Auction::new();
        auction.push(open);
        auction.push(Call::Pass);
        table.next_call(hand, &auction)
    }

    /// Board [40] of the kickback phase-5 divergence audit: 6-5 in the majors
    /// opposite a weak 2♦ shows **spades**, the longer and higher suit.  The
    /// shipped node tied both majors at 1.5 and bid 2♥.
    #[test]
    fn five_five_majors_respond_in_the_longer_major() {
        assert_eq!(
            responds(call(2, Strain::Diamonds), "AKT862.AKJ92.4.3"),
            call(2, Strain::Spades)
        );
        // Exactly 5-5: the tie goes to the higher rank.
        assert_eq!(
            responds(call(2, Strain::Diamonds), "AKT86.AKJ92.4.32"),
            call(2, Strain::Spades)
        );
        // Longer hearts win over shorter spades.
        assert_eq!(
            responds(call(2, Strain::Diamonds), "AKT86.AKJ952.4.3"),
            call(2, Strain::Hearts)
        );
    }

    /// Ablating [`set_weak_two_longest_first`] restores the argmax race that
    /// board [40] of the kickback audit was bid under — the probe's `tie` arm.
    #[test]
    fn longest_first_ablation_restores_the_cheaper_major() {
        set_weak_two_longest_first(false);
        let off = responds(call(2, Strain::Diamonds), "AKT862.AKJ92.4.3");
        set_weak_two_longest_first(true);
        assert_eq!(off, call(2, Strain::Hearts));
    }

    /// The tie-break is doctrine, not a diamond-specific patch: it holds over
    /// 2♥ and 2♠ too, where the longer suit may be the *dearer* bid.
    #[test]
    fn longest_first_holds_over_the_major_weak_twos() {
        // Over 2♠, six hearts beat five clubs even though 3♣ is cheaper.
        assert_eq!(
            responds(call(2, Strain::Spades), "3.AKJ942.4.AKQ82"),
            call(3, Strain::Hearts)
        );
        // Six clubs beat five hearts.
        assert_eq!(
            responds(call(2, Strain::Spades), "3.AKJ92.4.AKQ842"),
            call(3, Strain::Clubs)
        );
    }

    /// [`set_weak_two_major_priority`] lifts a good five-card major above the
    /// Ogust ask over 2♦ — and only there.  Knob-off restores Ogust priority.
    #[test]
    fn major_priority_outranks_ogust_over_two_diamonds() {
        // 16 HCP, five good spades, two diamonds: the major by default.
        let hand = "AKQ82.KQ2.43.J65";
        assert_eq!(
            responds(call(2, Strain::Diamonds), hand),
            call(2, Strain::Spades)
        );
        // Over 2♠ the ask keeps priority: a new suit there costs the 3 level.
        assert_eq!(
            responds(call(2, Strain::Spades), "43.AKQ82.KQ2.J65"),
            call(2, Strain::Notrump)
        );

        set_weak_two_major_priority(false);
        let off = responds(call(2, Strain::Diamonds), hand);
        set_weak_two_major_priority(true);
        assert_eq!(off, call(2, Strain::Notrump));
    }
}
