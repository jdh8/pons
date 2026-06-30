//! Notrump response structures for the 2/1 game-forcing system
//!
//! This module centralises every notrump continuation:
//!
//! - Responses to a **1NT** opening (Stayman 2♣, Jacoby transfers 2♦/2♥,
//!   notrump raises, and the quantitative 4NT invite).
//! - Responses to a **2NT-strength** notrump — both the direct 2NT opening
//!   (20–21 balanced) and opener's 2NT rebid after a 2♣ opening (22–24
//!   balanced): 3-level Stayman / transfers and the quantitative 4NT.
//! - Simple continuations after opener's **18–19 2NT rebid** over a one-level
//!   new-suit response.
//!
//! The public surface is [`register`], called once by
//! [`american`][super::american] during system assembly.

use super::{call, insert_uncontested, slam};
use crate::bidding::constraint::{
    Cons, Constraint, balanced, described, hcp, len, point_count, points, stopper_in,
};
use crate::bidding::{Alert, Context, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Rank, Strain, Suit};
use std::cell::Cell;

// ---------------------------------------------------------------------------
// 1NT minor-suit response scheme (our-system variant tag)
// ---------------------------------------------------------------------------

/// The **Puppet** 1NT minor scheme — the shipped default
///
/// `2♠` = clubs or a balanced invite, `2NT` = diamonds (transfer), `3♣` = Puppet
/// Stayman.  The variant-selecting [`Alert`] minting the convention (see the
/// `Alert` newtype doc); pass it to [`set_notrump_minors`].
pub const PUPPET: Alert = Alert("puppet");

/// The **European** 1NT minor scheme — opt-in, BBA's Atlantic style
///
/// `2♠` = clubs (transfer), `2NT` = a balanced invite / size ask, `3♣` = diamonds
/// (transfer); no Puppet Stayman.  The standard Polish Club / WJ and common
/// continental response set.  Select with [`set_notrump_minors`].
pub const EUROPEAN: Alert = Alert("european");

// Always-on artificial 1NT responses (present under either minor scheme).  These
// are alerts, not gates: the gate drops only the *dormant* minor scheme, so these
// survive (see `notrump_responses`).
const STAYMAN: Alert = Alert("stayman");
const JACOBY: Alert = Alert("jacoby-transfer");
const BOTH_MAJORS: Alert = Alert("both-majors");
const TEXAS: Alert = Alert("texas");
const SMOLEN: Alert = Alert("smolen");
const SPLINTER: Alert = Alert("splinter");
const SLAM_TRY: Alert = Alert("slam-try");
/// Responder's invitational 5-4-majors rebid after a heart transfer (auctions C/D):
/// `2♠` = single-suited heart invite (denies four spades), `2NT` = five hearts +
/// four spades.  Both are artificial — `2♠` isn't spades, `2NT` pins the 4-card
/// side suit — so the reader decodes them rather than reading natural.
const INV_5CARD: Alert = Alert("inv-5card-major");

thread_local! {
    /// The active 1NT minor-suit response variant, read once at book-construction
    /// time (and by the inference engine, to decode our `2♠`/`2NT`/`3♣`).
    /// [`PUPPET`] by default; flipped to [`EUROPEAN`] by [`set_notrump_minors`].
    static NOTRUMP_MINORS: Cell<Alert> = const { Cell::new(PUPPET) };
}

/// Select the 1NT minor-suit response scheme for books built *after* this call
///
/// Thread-local, read at book-construction time (the [`set_woolsey`]-style knob).
/// Pass [`PUPPET`] (default) or [`EUROPEAN`]; both variants are authored, and only
/// the selected one's `2♠`/`2NT`/`3♣` rules are gated into the trie.
///
/// [`set_woolsey`]: super::set_woolsey
pub fn set_notrump_minors(variant: Alert) {
    NOTRUMP_MINORS.with(|cell| cell.set(variant));
}

/// The active 1NT minor scheme, defaulting to [`PUPPET`]
///
/// Read both at book construction (to gate `2♠`/`2NT`/`3♣` and their
/// continuations) and by the inference engine (to read the artificial calls).
pub(crate) fn notrump_minors() -> Alert {
    NOTRUMP_MINORS.with(Cell::get)
}

// ---------------------------------------------------------------------------
// 1NT response structure
// ---------------------------------------------------------------------------

/// Responses to our 1NT opening: Stayman, Jacoby transfers, the minor-suit
/// scheme, and notrump raises
///
/// Stayman (2♣) needs invitational+ values and a four-card major; Jacoby
/// transfers (2♦/2♥) a five-card major, any strength.  The quantitative 4NT
/// invites slam opposite a balanced 16–17 with no four-card major.
///
/// The minor-suit responses (`2♠`/`2NT`/`3♣`) come in two variants, both authored
/// here behind their [`Alert`] and gated to the active one (`set_notrump_minors`,
/// default [`PUPPET`]): `puppet_minors` (`2♠` = clubs-or-invite, `2NT` = diamonds,
/// `3♣` = Puppet Stayman) and `european_minors` (`2♠` = clubs, `2NT` = balanced
/// invite, `3♣` = diamonds).
#[must_use]
pub fn notrump_responses() -> Rules {
    // Jacoby transfers — any strength, except a game-forcing 5-4 in the majors
    // (the `hcp(..9)` arm denies it): that hand keeps off the transfer and takes
    // the 2♣ Stayman/Smolen route, which right-sides game to the strong notrump.
    // A plain 5-3 still transfers.  2♦ (to hearts) is UNCHANGED by the reroute — a
    // 5♥4♠ invite keeps transferring and shows the spades with a later 2NT/2♠.
    let head = Rules::new()
        .rule(
            Bid::new(2, Strain::Diamonds),
            2.0,
            len(Suit::Hearts, 5..) & (len(Suit::Spades, ..4) | hcp(..9)),
        )
        .alert(JACOBY);
    // 2♥ (to spades): the invitational-5-4 reroute (gated) keeps a 5♠4♥ hand of
    // invitational+ values OFF the transfer so it Staymans; a six-card spade suit
    // (`len(♠,6..)`) and a weaker 5♠4♥ (`hcp(..8)`) still transfer.  Off the flag,
    // the classic any-strength-but-GF-5-4 gate.
    let head = if invitational_5card_majors() {
        head.rule(
            Bid::new(2, Strain::Hearts),
            2.0,
            len(Suit::Spades, 5..) & (len(Suit::Hearts, ..4) | hcp(..8) | len(Suit::Spades, 6..)),
        )
    } else {
        head.rule(
            Bid::new(2, Strain::Hearts),
            2.0,
            len(Suit::Spades, 5..) & (len(Suit::Hearts, ..4) | hcp(..9)),
        )
    }
    .alert(JACOBY);
    head
        // Both-majors 3♦: 5+/5+ in the majors, invitational+.  Outranks the
        // transfers (2.0) so a 5-5 INV+ hand shows both suits in one bid rather
        // than transferring and rebidding; weaker 5-5s (below the `points` floor)
        // still take the transfer route.  `points` (not `hcp`) so the 5-5 shape
        // upgrade counts — these are the unbalanced hands the gauge was built for.
        .rule(
            Bid::new(3, Strain::Diamonds),
            2.1,
            len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & points(8..),
        )
        .alert(BOTH_MAJORS)
        // South African Texas at the four level — a 6-card major.  `4♣/4♦`
        // transfer to the major as the everyday *preemptive* to-play route:
        // jumping straight to game robs the opponents of the two-level a slow
        // Jacoby transfer would leave them to balance in.  A *direct* `4♥/4♠` is a
        // non-forcing slam try (opener passes a minimum, or launches RKCB with a
        // maximum — see [`slam_try_answer`]).  All four outrank the 2.0 Jacoby
        // transfers so the 6-card hand takes the four-level route; the `len(other
        // major, ..5)` guard keeps a 5-5+ two-suiter on the both-majors 3♦, and
        // the strength gate ([`texas_strength_gate`]) routes game-no-slam to the
        // blast (`point_count + length ≥ 14`, lowered from the inherited raw-HCP 9
        // to capture the invitational 7-8 hands — see [`set_texas_game_floor`]) and
        // slam-invitational (15–18) to the direct slam try.
        .rule(
            Bid::new(4, Strain::Clubs),
            2.5,
            len(Suit::Hearts, 6..) & len(Suit::Spades, ..5) & texas_strength_gate(Suit::Hearts),
        )
        .alert(TEXAS)
        .rule(
            Bid::new(4, Strain::Diamonds),
            2.5,
            len(Suit::Spades, 6..) & len(Suit::Hearts, ..5) & texas_strength_gate(Suit::Spades),
        )
        .alert(TEXAS)
        .rule(
            Bid::new(4, Strain::Hearts),
            2.6,
            len(Suit::Hearts, 6..) & len(Suit::Spades, ..5) & hcp(15..=18),
        )
        .alert(TEXAS)
        .rule(
            Bid::new(4, Strain::Spades),
            2.6,
            len(Suit::Spades, 6..) & len(Suit::Hearts, ..5) & hcp(15..=18),
        )
        .alert(TEXAS)
        // Stayman: a four-card major and at least invitational values — but never
        // on a flat 4-3-3-3, which plays better in 3NT than in the 4-4 major fit
        // (no ruffing value), so it invites/forces in notrump directly.
        .rule(
            Bid::new(2, Strain::Clubs),
            1.5,
            (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4)) & hcp(8..) & !flat_4333(),
        )
        .alert(STAYMAN)
        // Quantitative 4NT slam invite (balanced, no four-card major).
        .rule(
            Bid::new(4, Strain::Notrump),
            1.2,
            hcp(16..=17) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        // Natural 3NT game-force, 9+, no five-card major (those transfer).  A
        // balanced hand with a three-card major prefers Puppet (3♣ outranks), so
        // in practice this catches game forces lacking a three-card major and the
        // 18–19 too strong for the quantitative 4NT.  Forcing every 9 (rather than
        // inviting 8–9 and forcing 10+) is A/B-verified worth ≈+1 IMP per
        // divergent board vul none and ≈+3 vul both: opposite a 15–17 opener a 9
        // makes game often enough that the invitational stop loses more by missing
        // games than it gains.  Deciding the 9 by Fifths instead was measured
        // *worse* — even quack-heavy 9s are worth forcing.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(9..) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(
            Call::Pass,
            0.0,
            hcp(..8) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        // Minor-suit responses (2♠/2NT/3♣): both schemes are authored here, each
        // alerted with its variant, and only the active one is gated in.  The gate
        // drops just the dormant minor scheme; every always-on alert (Stayman,
        // Jacoby, …) survives.  Default Puppet.
        .chain(puppet_minors())
        .chain(european_minors())
        // Garbage Stayman (opt-in): a weak 2♣ to escape 1NT.  Same STAYMAN alert,
        // so it survives the minor-scheme gate (which only drops dormant minors).
        .chain(garbage_stayman_rule())
        // Crawling Stayman (superset of garbage): 4-4 majors short in diamonds.
        .chain(crawling_stayman_rule())
        .gated(move |alert| alert != dormant_minors())
}

/// Garbage (drop-dead) Stayman: a weak 2♣ intending to pass opener's answer
///
/// Two tiers, looser the weaker responder is (a broke 1NT rates to be a
/// disaster, so any ~7-card fit is an improvement): both tiers want a four-card
/// major, both majors playable (3+ for a ≥7-card fit on any major answer), and
/// short clubs; the broke tier accepts a thinner 2♦ landing (3+ diamonds), the
/// weak tier insists on 4+.  HCP bands are disjoint from the constructive 2♣
/// (`hcp(8..)`), so no hand matches two 2♣ rules.  Empty when off.
// ponytail: the 0-4/5-7 split and the 3-vs-4 diamond floor are tunable knobs —
// the A/B can tighten or loosen them.
fn garbage_stayman_rule() -> Rules {
    if !garbage_stayman() {
        return Rules::new();
    }
    Rules::new()
        // Broke (0-4): escape at almost any cost; accept a thin 2♦ landing.
        .rule(
            Bid::new(2, Strain::Clubs),
            1.5,
            (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..))
                & len(Suit::Hearts, 3..)
                & len(Suit::Spades, 3..)
                & len(Suit::Clubs, ..3)
                & len(Suit::Diamonds, 3..)
                & hcp(..5)
                & !flat_4333(),
        )
        .alert(STAYMAN)
        // Weak (5-7): insist on a safe 2♦ landing (4+ diamonds).
        .rule(
            Bid::new(2, Strain::Clubs),
            1.5,
            (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..))
                & len(Suit::Hearts, 3..)
                & len(Suit::Spades, 3..)
                & len(Suit::Clubs, ..3)
                & len(Suit::Diamonds, 4..)
                & hcp(5..8)
                & !flat_4333(),
        )
        .alert(STAYMAN)
}

/// Crawling Stayman: a weak 2♣ on 4-4 majors *short in diamonds* (4414/4405)
///
/// The shapes garbage Stayman cannot escape — with ≤1 diamond, passing opener's
/// 2♦ would land in a singleton/void.  Crawling bids 2♣ anyway and crawls 2♦ to
/// 2♥ (see [`stayman_no_major_rebid`]).  4-4 majors with ≤1 diamond forces ≥4
/// clubs, so the 2♥ pass-or-correct (and opener's 3♣ flee) always finds a fit.
/// Weak only (`hcp(..8)`), disjoint from constructive 2♣ and the garbage tiers
/// (which need 3+ diamonds).  Same STAYMAN alert.  Empty when off.
fn crawling_stayman_rule() -> Rules {
    if !crawling_stayman() {
        return Rules::new();
    }
    Rules::new()
        .rule(
            Bid::new(2, Strain::Clubs),
            1.5,
            len(Suit::Hearts, 4..=4)
                & len(Suit::Spades, 4..=4)
                & len(Suit::Diamonds, ..=1)
                & hcp(..8),
        )
        .alert(STAYMAN)
}

/// The minor scheme *not* selected — the one [`notrump_responses`] gates out
fn dormant_minors() -> Alert {
    if notrump_minors() == PUPPET {
        EUROPEAN
    } else {
        PUPPET
    }
}

/// Puppet minor-suit responses to 1NT (the default scheme)
///
/// `2♠` = a six-card club one-suiter (weak signoff, or game-going via a later
/// splinter) OR a balanced invitational eight with no four-card major (the bare-8
/// invite relocated here when 2NT became the diamond transfer; min→2NT and max→3NT
/// reproduce the old natural-2NT outcomes).  `2NT` = transfer to diamonds (6+♦, or
/// a 5♦-4♣ minor two-suiter).  `3♣` = Puppet Stayman: game-forcing, balanced, with
/// a three-card major — ranked *above* Stayman so a 4-3 hand takes the Puppet route
/// to catch opener's five-card major in the three-card suit; `balanced()` keeps it
/// off shapely hands, and a balanced no-four-card-major hand almost always has a
/// three-card major, so this routes most balanced game forces through 3♣ (the
/// no-fit case relays back to 3NT).
fn puppet_minors() -> Rules {
    Rules::new()
        // 2♠ = six-card clubs, or the bare-8 balanced invite.  The invite now also
        // takes a flat 4-3-3-3 with a four-card major (it no longer Staymans): the
        // gate is "no four-card major, or flat 4333".
        .rule(
            Bid::new(2, Strain::Spades),
            1.3,
            len(Suit::Clubs, 6..)
                | (hcp(8..=8)
                    & balanced()
                    & ((len(Suit::Hearts, ..4) & len(Suit::Spades, ..4)) | flat_4333())),
        )
        .alert(PUPPET)
        .rule(
            Bid::new(2, Strain::Notrump),
            1.3,
            len(Suit::Diamonds, 6..) | (len(Suit::Diamonds, 5..) & len(Suit::Clubs, 4..)),
        )
        .alert(PUPPET)
        .rule(
            Bid::new(3, Strain::Clubs),
            1.6,
            balanced()
                & hcp(9..=15)
                & (len(Suit::Hearts, 3..=3) | len(Suit::Spades, 3..=3))
                & len(Suit::Hearts, ..5)
                & len(Suit::Spades, ..5)
                // A flat 4-3-3-3 plays 3NT, not the 5-3 major fit — bid notrump.
                & !flat_4333(),
        )
        .alert(PUPPET)
}

/// European minor-suit responses to 1NT (opt-in via [`set_notrump_minors`])
///
/// `2♠` = transfer to clubs (a six-card one-suiter, weak-to-game).  `2NT` = a
/// balanced invitational eight with no four-card major — the size ask, opener
/// accepting game with a maximum.  `3♣` = transfer to diamonds (6+♦, or a 5♦-4♣
/// two-suiter folded in — there is no room below 3♦ to show the clubs).  There is
/// no Puppet Stayman: a game-forcing balanced hand with only a three-card major
/// bids 3NT (the standard continental treatment).
fn european_minors() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Spades), 1.3, len(Suit::Clubs, 6..))
        .alert(EUROPEAN)
        .rule(
            Bid::new(2, Strain::Notrump),
            1.3,
            // The bare-8 size ask, also home to a flat 4-3-3-3 with a four-card
            // major (no Stayman with 4333).
            hcp(8..=8)
                & balanced()
                & ((len(Suit::Hearts, ..4) & len(Suit::Spades, ..4)) | flat_4333()),
        )
        .alert(EUROPEAN)
        .rule(
            Bid::new(3, Strain::Clubs),
            1.3,
            len(Suit::Diamonds, 6..) | (len(Suit::Diamonds, 5..) & len(Suit::Clubs, 4..)),
        )
        .alert(EUROPEAN)
}

/// Opener's answer to Stayman: a four-card major, else 2♦
///
/// `pub(super)` so the competitive book can reuse it as the always-mass catch-all
/// when authoring opener's penalty-pass over a `(2♣)` overcall (systems on).
pub(super) fn stayman_answers() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 1.0, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(2, Strain::Spades),
            1.0,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            0.5,
            len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
}

/// Opener's Stayman answer at the *uncontested* `[1NT, 2♣]` node
///
/// Wraps [`stayman_answers`] with the opt-in max-showing overlays so the shared
/// `stayman_answers` (reused by the competitive book) stays untouched.  With both
/// toggles off this is byte-identical to `stayman_answers`.  A balanced 1NT with
/// a five-card major has ≤3 in the other major, so "both four-card majors" and
/// "five-card major" never overlap; the natural answers (weight 1.0) catch every
/// remaining case (single major, no major, a *minimum* five-card major).
fn stayman_answers_uncontested() -> Rules {
    let mut rules = Rules::new();
    if stayman_both_majors() {
        // Both four-card majors with a *maximum* (16-17, the invite-accepting
        // range): jump to 2NT.  Responder then names their own major (3♣ = hearts,
        // 3♦ = spades) so opener — the strong, concealed hand — declares it
        // (right-siding).  A minimum (15) bids 2♥ naturally, so 2NT only ever costs
        // a step on the maximum.
        let both = len(Suit::Hearts, 4..) & len(Suit::Spades, 4..);
        rules = rules
            .rule(Bid::new(2, Strain::Notrump), 1.1, both & hcp(16..))
            .alert(BOTH_MAJORS);
    }
    if stayman_5card_max() {
        // Five-card major, maximum (16-17): jump.  Natural (names and shows its
        // own suit), so unalerted — alerting would make alert-reading suppress it.
        rules = rules
            .rule(
                Bid::new(3, Strain::Hearts),
                1.1,
                len(Suit::Hearts, 5..) & hcp(16..),
            )
            .rule(
                Bid::new(3, Strain::Spades),
                1.1,
                len(Suit::Spades, 5..) & hcp(16..),
            );
    }
    rules.chain(stayman_answers())
}

/// Responder's relay over opener's max-both-majors `2NT`
///
/// Opener has both four-card majors and a maximum, so responder names *their* own
/// longer major — `3♣` = hearts, `3♦` = spades — asking opener to bid it so the
/// strong concealed hand declares (right-siding).  Both are alerted (artificial).
/// Responder bid Stayman, so always holds a four-card major; the two rules tile
/// every hand, so no catch-all is needed.  A 4-4 tie goes to hearts (the lower
/// major), keeping the most room to escape if an opponent doubles the relay.
fn both_majors_max_responder() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.0,
            described("spades > hearts", |hand: Hand, _: &Context<'_>| {
                hand[Suit::Spades].len() > hand[Suit::Hearts].len()
            }),
        )
        .alert(BOTH_MAJORS)
        .rule(
            Bid::new(3, Strain::Clubs),
            1.0,
            described("hearts ≥ spades", |hand: Hand, _: &Context<'_>| {
                hand[Suit::Hearts].len() >= hand[Suit::Spades].len()
            }),
        )
        .alert(BOTH_MAJORS)
}

/// Opener's forced completion of the both-majors relay (right-siding)
///
/// Responder named a major via `3♣`/`3♦`; opener simply bids it so opener declares.
/// Alerted — it completes the relay and shows nothing beyond the `2NT` already did.
fn both_majors_relay_complete(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::from(major)), 1.0, hcp(0..))
        .alert(BOTH_MAJORS)
}

/// Responder places game over opener's right-siding completion
///
/// Opener's maximum (16-17) and the major fit are both known, so the invite is
/// pre-accepted: bid game when the agreed fit is worth it, else pass the
/// three-level completion (the floor's settle).  The fit is gauged as
/// `points + extra trumps + a fit in the other major`: shape counts now the
/// trump suit is agreed, a fifth trump (the 9-card fit) adds a point, and — since
/// opener showed *both* four-card majors — four in the unnamed major is a known
/// second 4-4 fit worth another.  A flat single 4-4 still needs a full eight; a
/// 5-4 or a double fit reaches game a king lighter.  A bare `points(6..)` on the
/// fifth trump alone overbid the 5-3-3-2 nothing hands this gate now passes.
fn both_majors_relay_placement(major: Suit) -> Rules {
    let other = match major {
        Suit::Spades => Suit::Hearts,
        _ => Suit::Spades,
    };
    Rules::new().rule(
        Bid::new(4, Strain::from(major)),
        1.3,
        described("game values for the agreed major", move |hand: Hand, _| {
            let double_fit = usize::from(hand[other].len() >= 4);
            fit_value(hand, major) + double_fit >= 8
        }),
    )
}

/// Responder's trump-length-adjusted value for a known `major` fit
///
/// Point count plus one per trump beyond the eighth — the ninth and tenth
/// trump are worth a point apiece now the suit is agreed.  No double-fit term:
/// at a plain Stayman answer opener showed only the one major, so a second fit
/// is unknowable.  ([`both_majors_relay_placement`] adds it back where opener
/// *did* show both majors.)
fn fit_value(hand: Hand, major: Suit) -> usize {
    usize::from(point_count(hand)) + hand[major].len().saturating_sub(4)
}

/// Responder's placement over opener's max five-card-major jump (`3♥`/`3♠`)
///
/// With three-card support (an eight-card fit) opposite a maximum, bid game; else
/// sign off in `3NT`.
fn five_card_max_rebid(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 1.3, len(major, 3..))
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(0..))
}

thread_local! {
    /// Whether opener jump super-accepts a Jacoby transfer with four-card support
    /// and a maximum; **off by default** (opt-in A/B).  See
    /// [`set_transfer_super_accept`].
    static TRANSFER_SUPER_ACCEPT: Cell<bool> = const { Cell::new(false) };
}

/// Author opener's jump super-accept of a Jacoby transfer for books built *after*
/// this call (thread-local; **off by default**).
///
/// With four-card support for responder's major and a maximum (17), opener jumps
/// to the three-level instead of merely completing the transfer, so the
/// nine-card fit and the extra values are shown in one call.  Opt-in: a paired
/// double-dummy A/B vs BBA over 640 000 boards found the jump a DD wash leaning
/// negative (−0.055 IMPs/board it fires on) — opposite a transfer that may hold
/// nothing, committing to the three-level overbids — so it stays off by default.
pub fn set_transfer_super_accept(on: bool) {
    TRANSFER_SUPER_ACCEPT.with(|cell| cell.set(on));
}

/// Whether the jump super-accept is currently authored
pub(super) fn transfer_super_accept() -> bool {
    TRANSFER_SUPER_ACCEPT.with(Cell::get)
}

thread_local! {
    /// Responder's single-suited slam try after a Jacoby transfer completes;
    /// **on by default**.  See [`set_transfer_slam_try`].
    static TRANSFER_SLAM_TRY: Cell<bool> = const { Cell::new(true) };
}

/// Author responder's post-transfer single-suited slam try for books built
/// *after* this call (thread-local; **on by default**).
///
/// After a Jacoby transfer completes (`1NT–2♦–2♥` / `1NT–2♥–2♠`), a single-suited
/// five-card major with slam-invitational values (16+ HCP, opposite the 15–17
/// opener) bids the *other* major (`3♠` / `3♥`) as an artificial slam try agreeing
/// the transfer major; opener launches RKCB with a maximum (`4NT`) or signs off in
/// the major game (`4M`), and the [`slam`] 1430 ladder places the slam.  Mirrors
/// the Stayman `3OM` slam try, which the transfer path lacked — so a strong
/// balanced five-card-major responder used to rest in `3NT` while a major slam was
/// cold (the dominant double-dummy leak in our 1NT opening vs BBA).  A paired
/// on/off A/B (320k boards, shared seed, vs the BBA reference) measured **plain
/// +0.0012 IMPs/board (95% CI ±0.0004), PD +0.0012 — +1.42 IMPs/fired in both
/// regimes** (275 fired, 0.09%), every CI excluding 0.
pub fn set_transfer_slam_try(on: bool) {
    TRANSFER_SLAM_TRY.with(|cell| cell.set(on));
}

/// Whether the post-transfer slam try is currently authored
fn transfer_slam_try() -> bool {
    TRANSFER_SLAM_TRY.with(Cell::get)
}

thread_local! {
    /// Garbage (drop-dead) Stayman: a *weak* hand bids 2♣ to escape 1NT into a
    /// major (or diamond) partscore, intending to pass opener's answer.  **On by
    /// default** — a paired DD A/B vs BBA (205k boards, vul none) measured +0.51
    /// IMPs/fired plain (+0.0009/board, 95% CI excl 0) and +0.70 PD.  See
    /// [`set_garbage_stayman`].
    static GARBAGE_STAYMAN: Cell<bool> = const { Cell::new(true) };
    /// Opener jumps to `2NT` over 1NT-2♣ holding *both* four-card majors and a
    /// *maximum* (16-17); a minimum (15) bids 2♥ naturally.  Responder then names own
    /// major (`3♣` = hearts, `3♦` = spades) and opener completes (`3♥`/`3♠`), so the
    /// strong concealed hand declares the known 4-4 fit (right-siding) instead of
    /// responder declaring after a direct raise.  **On by default** — a paired DD
    /// A/B vs BBA (320k boards/arm, vul none) measured +2.18 IMPs/fired plain
    /// (+0.0035/board, 95% CI excl 0) and +2.29 PD *with garbage on*, +2.68/+2.87
    /// with garbage off — a win in every regime, unlike the earlier strength-step
    /// scheme it replaces.  See [`set_stayman_both_majors`].
    static STAYMAN_BOTH_MAJORS: Cell<bool> = const { Cell::new(true) };
    /// Opener jumps `3♥`/`3♠` over 1NT-2♣ holding a *five-card* major and a
    /// maximum (16-17), showing the 5-3/5-4 fit plus extras.  **On by default** —
    /// the cleanest of the three: +3.45 IMPs/fired plain (+0.0007/board, 95% CI
    /// excl 0) and +3.33 PD, holding up at +1.47/+0.90 even with garbage on.  See
    /// [`set_stayman_5card_max`].
    static STAYMAN_5CARD_MAX: Cell<bool> = const { Cell::new(true) };
    /// The invitational 5-4-majors structure: 5♠4♥ invites via Stayman (a 2♠ rebid
    /// over opener's 2♦/2♥), 5♥4♠ via the heart transfer (`2NT` shows the spades,
    /// `2♠` denies them).  **On by default** — a paired A/B vs BBA (1.28M boards/arm,
    /// `--filter-1nt`, vul none) measured **+0.375 IMPs/fired plain (+0.0020/board,
    /// 95% CI ±0.0004) and +0.134 PD (+0.0007/board, 95% CI ±0.0005)**, both excl 0.
    /// The win needed the doubled-2♦ escape (`1NT-2♣-2♦-(X)` systems-on rebase in
    /// `competition.rs`): without it the reroute walked 5♠4♥ into a doubled artificial
    /// 2♦ it passed out, and PD was a wash (−0.0001).  Flipped per
    /// [`set_invitational_5card_majors`].
    static INVITATIONAL_5CARD_MAJORS: Cell<bool> = const { Cell::new(true) };
    /// Crawling Stayman: the superset of garbage Stayman for 4-4 majors *short in
    /// diamonds* (4414/4405).  Garbage needs a safe 2♦ landing (3+ diamonds), so it
    /// cannot escape with a singleton/void diamond; crawling bids 2♣ anyway and, if
    /// opener denies a major (2♦), *crawls* to 2♥ — both majors, pass-or-correct —
    /// rather than passing a doomed diamond partscore.  **On by default.**  See
    /// [`set_crawling_stayman`].
    static CRAWLING_STAYMAN: Cell<bool> = const { Cell::new(true) };
    /// The `point_count + trump length` floor at which a 6-card-major responder
    /// blasts game via South African Texas (`4♣/4♦`) instead of transferring at
    /// the two level.  **Default 14** (a 6-bagger needs 8 points, a 7-bagger 7).
    ///
    /// The book inherited a *raw-HCP* floor of **9** verbatim from the old
    /// transfer-then-game route (only the 15-18 slam edge was ever measured).  A
    /// double-dummy screen (`probe-jacoby-invite-eval`) found that 7-8 HCP 6-card
    /// hands score far better in `4M` than the partscore they stop in, that opener
    /// should *never decline* (so an invite degenerates to a blast), and that the
    /// `3M` invite-landing is a *worse* contract than `2M` at every strength (these
    /// one-suiters make 8 or 10 tricks, rarely 9) — so the choice is binary,
    /// pass-`2M` or blast-`4M`, with no invitational band.  At this *fit-rich*
    /// boundary distribution is a real trick (the 6th trump, ruffs), so the screen
    /// (experiments F/G) ranked `point_count + length` > CCCC > points > raw HCP
    /// for the blast decision — unlike the no-fit invite line
    /// (`probe-nt-invite-eval`) and the slam edge (`probe-texas-slam-eval`) where
    /// honors dominate and HCP won.
    ///
    /// Paired A/Bs vs BBA (1.024M boards/arm, `--filter-1nt`): `point_count+len≥14`
    /// over the old HCP-9 baseline measured **plain +0.0102/board vul none, +0.0171
    /// both; PD +0.0082 / +0.0141**, and over a raw-HCP≥7 floor (the same
    /// aggressiveness) **plain +0.0013 / +0.0018; PD +0.0014 / +0.0019** — every
    /// regime a win, all 95% CI excl 0.  `14` matches the HCP≥7 blast rate while
    /// promoting shapely sixes (a 6-4 makes the cut at a bare 6) and demoting
    /// wasted-honor sevens.  See [`set_texas_game_floor`].
    static TEXAS_GAME_FLOOR: Cell<u8> = const { Cell::new(14) };
    /// The `point_count + trump length` floor at which a six-card-major responder
    /// *invites* game — transfer at the two level, then jump to `3M` — instead of
    /// resting in the passed two-level partscore.  **Default 13** (on): the
    /// invitational band is `[13, `[`TEXAS_GAME_FLOOR`]`)`, i.e. the just-below-blast
    /// sixes route through a `3M` invite; opener accepts on [`SIXCARD_ACCEPT_FLOOR`].
    /// Raise it to [`TEXAS_GAME_FLOOR`] (14) to empty the band and turn the invite
    /// *off*.
    ///
    /// On by default as standard, expected major-suit bidding.  A paired A/B vs BBA
    /// (1.536M boards/arm, `--filter-1nt`, floor 13 over 14, accept floor 18; 1607
    /// fired, 0.10%) measured **plain +0.619 IMPs/fired vul none, +1.820 both (CI
    /// excl 0); PD −0.211 / +0.561** — perfect-defense doubling trims the vul-none
    /// edge (the 3-level tax: the decline branch rests in `3M`), but a 6-card-fit
    /// `3M` partscore is not realistically doubled into a penalty at IMPs, so the
    /// PD-none figure overstates the downside.  Double-dummy can't see the invite's
    /// real edge anyway — the `3M` brake on the thin games real defenders beat — so
    /// the conventional invite is kept on.  `probe-jacoby-invite-eval` experiment I
    /// has the opener-threshold sweep.
    static SIXCARD_INVITE_FLOOR: Cell<u8> = const { Cell::new(13) };
    /// Opener's accept floor for the six-card-major invite (`…3M → 4M`) on
    /// `point_count + trump length`; below it opener passes `3M`.  **Default 18**:
    /// a flat 15 with a doubleton in the major (15 + 2) declines, a 15 with
    /// three-card support (15 + 3) or any 16+ accepts — the ≈15% decline the
    /// probe's opener sweep found optimal.  Consulted only when the invite is on
    /// ([`SIXCARD_INVITE_FLOOR`] < [`TEXAS_GAME_FLOOR`]).
    static SIXCARD_ACCEPT_FLOOR: Cell<u8> = const { Cell::new(18) };
}

/// Author garbage (drop-dead) Stayman for books built *after* this call
/// (thread-local; **on by default**).
///
/// A weak responder with short clubs and a four-card major bids 2♣ to escape a
/// likely-doomed 1NT, passing opener's 2♦/2♥/2♠.  Looser the weaker responder
/// is: broke hands accept a thinner 2♦ landing, since any ~7-card fit beats a
/// broke 1NT.
pub fn set_garbage_stayman(on: bool) {
    GARBAGE_STAYMAN.with(|cell| cell.set(on));
}

/// Author opener's max-only right-siding relay over 1NT-2♣ with both four-card
/// majors for books built *after* this call (thread-local; **on by default**).
pub fn set_stayman_both_majors(on: bool) {
    STAYMAN_BOTH_MAJORS.with(|cell| cell.set(on));
}

/// Author opener's max five-card-major jump over 1NT-2♣ for books built *after*
/// this call (thread-local; **on by default**).
pub fn set_stayman_5card_max(on: bool) {
    STAYMAN_5CARD_MAX.with(|cell| cell.set(on));
}

/// Whether garbage Stayman is currently authored (read by the inference engine
/// too, to widen the 2♣ point range it reads)
pub(crate) fn garbage_stayman() -> bool {
    GARBAGE_STAYMAN.with(Cell::get)
}

/// Whether opener's both-majors max-only relay is currently authored
fn stayman_both_majors() -> bool {
    STAYMAN_BOTH_MAJORS.with(Cell::get)
}

/// Whether opener's max five-card-major jump is currently authored
fn stayman_5card_max() -> bool {
    STAYMAN_5CARD_MAX.with(Cell::get)
}

/// Author the invitational 5-4-majors structure for books built *after* this call
/// (thread-local; **off by default**).
///
/// 5♠4♥ at invitational+ values keeps off the spade transfer and bids Stayman,
/// inviting with a 2♠ rebid over opener's 2♦ (non-forcing) or 2♥ (forcing); 5♥4♠
/// transfers to hearts and rebids `2NT` (showing the four spades) or `2♠` (an
/// artificial relay denying them).  A Muppet-style swap brought down to the
/// two-level over 1NT — see CHANGELOG.
pub fn set_invitational_5card_majors(on: bool) {
    INVITATIONAL_5CARD_MAJORS.with(|cell| cell.set(on));
}

/// Whether the invitational 5-4-majors structure is currently authored (read at
/// book construction to gate the reroute, the Stayman 2♠ rebids, and the
/// heart-transfer invitational node)
fn invitational_5card_majors() -> bool {
    INVITATIONAL_5CARD_MAJORS.with(Cell::get)
}

/// Author Crawling Stayman for books built *after* this call (thread-local; **on
/// by default**).
///
/// A weak 4-4-majors hand short in diamonds (4414/4405) bids 2♣ and, over opener's
/// 2♦ denial, crawls to 2♥ (pass-or-correct between the majors).  The strict
/// superset of garbage Stayman, which cannot escape such hands (it passes 2♦, a
/// singleton/void diamond "fit").
pub fn set_crawling_stayman(on: bool) {
    CRAWLING_STAYMAN.with(|cell| cell.set(on));
}

/// Whether Crawling Stayman is currently authored (read by the inference engine
/// too, to widen the 2♣ point range it reads)
pub(crate) fn crawling_stayman() -> bool {
    CRAWLING_STAYMAN.with(Cell::get)
}

/// Set the South African Texas game-blast floor on `point_count + trump length`
/// (`4♣/4♦`) for books built *after* this call (thread-local; **default 14**).
///
/// Below this floor a 6-card-major hand transfers at the two level (and passes
/// the partscore); at or above it, it jumps to game.  No explicit upper cap: the
/// slam-try `4♥/4♠` (weight 2.6) outranks the game blast (2.5) for the 15-18
/// band, so a slam-interested hand takes the direct slam try regardless.
pub fn set_texas_game_floor(floor: u8) {
    TEXAS_GAME_FLOOR.with(|cell| cell.set(floor));
}

/// The current South African Texas game-blast floor (`point_count + trump length`)
fn texas_game_floor() -> usize {
    usize::from(TEXAS_GAME_FLOOR.with(Cell::get))
}

/// The South African Texas game-blast strength gate for `major`:
/// `point_count + trump length ≥ T` (default `T = 14`).
///
/// Point count plus the full suit length, so a longer trump suit needs fewer
/// points: a 6-bagger blasts at 8 points, a 7-bagger at 7, an 8-bagger at 6.
/// (This is the Stayman [`fit_value`] less its 4-4-fit baseline, which is
/// meaningless for a one-suiter — here the whole suit is the trump length.)  The
/// `len` guards (`6+` in `major`, `≤4` in the other) live with the rule; this is
/// just the strength term.
fn texas_strength_gate(major: Suit) -> Cons<impl Constraint + Clone> {
    let floor = texas_game_floor();
    described("six-card-major game blast", move |hand: Hand, _| {
        usize::from(point_count(hand)) + hand[major].len() >= floor
    })
}

/// Set the six-card-major game-*invite* floor on `point_count + trump length` for
/// books built *after* this call (thread-local; **default 13 = on**).
///
/// At or above [`set_texas_game_floor`]'s value the band is empty (no invite); the
/// default 13 routes the just-below-blast hands through a `3M` invite instead of a
/// passed two-level partscore.  Raise it to 14 to turn the invite off.
pub fn set_sixcard_invite_floor(floor: u8) {
    SIXCARD_INVITE_FLOOR.with(|cell| cell.set(floor));
}

/// Set opener's accept floor for the six-card-major invite (`…3M → 4M`) on
/// `point_count + trump length` for books built *after* this call (thread-local;
/// **default 18**).
pub fn set_sixcard_accept_floor(floor: u8) {
    SIXCARD_ACCEPT_FLOOR.with(|cell| cell.set(floor));
}

/// The current six-card-major game-invite floor (`point_count + trump length`)
fn sixcard_invite_floor() -> usize {
    usize::from(SIXCARD_INVITE_FLOOR.with(Cell::get))
}

/// Opener's current accept floor for the six-card-major invite
fn sixcard_accept_floor() -> usize {
    usize::from(SIXCARD_ACCEPT_FLOOR.with(Cell::get))
}

/// Whether the six-card-major invite is authored: its floor sits below the Texas
/// game-blast floor, so the invitational band `[invite, blast)` is non-empty.
fn sixcard_invite_active() -> bool {
    sixcard_invite_floor() < texas_game_floor()
}

/// Complete a Jacoby transfer by bidding the anchor suit
///
/// With four-card support and a maximum opener instead jumps to the three-level
/// (the super-accept, gated by [`set_transfer_super_accept`]); otherwise it
/// simply names the anchor suit.
// ponytail: a plain jump super-accept; fit-/shortness-showing super-accepts are
// the upgrade path if the A/B asks for them.
pub(super) fn complete_transfer(into: Suit) -> Rules {
    let mut rules = Rules::new();
    if transfer_super_accept() {
        rules = rules.rule(
            Bid::new(3, Strain::from(into)),
            1.5,
            len(into, 4..) & hcp(17..),
        );
    }
    rules.rule(Bid::new(2, Strain::from(into)), 1.0, hcp(0..))
}

/// Complete a four-level Texas transfer by bidding game in the anchor major
///
/// `4♣ → 4♥`, `4♦ → 4♠`.  Responder showed 6+ with game-no-slam values, so
/// opener simply names the game and declares.
fn complete_texas(into: Suit) -> Rules {
    Rules::new().rule(Bid::new(4, Strain::from(into)), 1.0, hcp(0..))
}

/// Responder's invitational jump after a Jacoby transfer completes, holding a
/// six-card major just below the Texas game-blast floor (`1NT–2♦–2♥–3♥` /
/// `1NT–2♥–2♠–3♠`)
///
/// A natural invitational raise of responder's own suit: 6+ in `major`, ≤4 in the
/// other, and `point_count + length` at or above the invite floor.  No upper
/// bound is needed — the blast hands (`≥ 14`) jumped straight to `4♣/4♦` and never
/// transferred, so only the `[invite, 14)` band reaches here.  Opener then accepts
/// game or passes `3M` ([`accept_sixcard_invitation`]).  Empty unless the invite
/// is on ([`set_sixcard_invite_floor`]).  Natural — floors only its own strain, so
/// it stays unalerted (the artificial-alert invariant).
fn sixcard_invite_rebid(major: Suit) -> Rules {
    if !sixcard_invite_active() {
        return Rules::new();
    }
    let floor = sixcard_invite_floor();
    Rules::new().rule(
        Bid::new(3, Strain::from(major)),
        1.3,
        len(major, 6..)
            & len(other_major(major), ..5)
            & described("six-card invitational value", move |hand: Hand, _| {
                usize::from(point_count(hand)) + hand[major].len() >= floor
            }),
    )
}

/// Opener's accept/decline of the six-card-major game invite (`…3M`)
///
/// Accept (`4M`) when `point_count + trump length` reaches
/// [`set_sixcard_accept_floor`]'s value (default 18); otherwise pass `3M`.
/// Authored because the keyless floor reads a three-level raise as forcing and so
/// could not decline.
fn accept_sixcard_invitation(major: Suit) -> Rules {
    let floor = sixcard_accept_floor();
    Rules::new()
        .rule(
            Bid::new(4, Strain::from(major)),
            1.0,
            described("accept six-card invite", move |hand: Hand, _| {
                usize::from(point_count(hand)) + hand[major].len() >= floor
            }),
        )
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's answer to a direct four-of-a-major slam try (`1NT–4♥/4♠`)
///
/// Non-forcing: a **maximum** (17) accepts by launching RKCB (`4NT`); a minimum
/// signs off by passing the major game.  The 1430 ladder ([`slam`]) then exchanges
/// keycards and places `6M`, or `5M` when the partnership is missing two.
fn slam_try_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.0, hcp(17..))
        .alert(slam::RKCB)
        .rule(Call::Pass, 0.0, hcp(..17))
}

/// The other major
const fn other_major(major: Suit) -> Suit {
    match major {
        Suit::Hearts => Suit::Spades,
        _ => Suit::Hearts,
    }
}

/// Opener holds a first- or second-round honour control (an ace or king) in `suit`
fn control_in(suit: Suit) -> Cons<impl Constraint + Clone> {
    // ponytail: A/K only — ignores shortness controls (void/singleton).  A full
    // cue scheme would add them, but a balanced 1NT opener rarely holds one.
    described(
        format!("control in {suit}"),
        move |hand: Hand, _: &Context<'_>| {
            let holding = hand[suit];
            holding.contains(Rank::A) || holding.contains(Rank::K)
        },
    )
}

/// Responder's rebid after opener answers Stayman with a major (`2♥`/`2♠`)
///
/// With a fit (four cards in opener's `major`): an invitational raise (`3M`) on
/// a flat eight, game (`4M`) on any upgrade past it (a ninth trump or working
/// shape — see [`fit_value`]), or — balanced, or slam-interested — the *other*
/// major (`3OM`) as an artificial slam try / choice of game.  Without a fit, the
/// auction reverts to notrump exactly as over a bare 1NT — invite `2NT`, game
/// `3NT`, and the quantitative `4NT` (16–17) — "ignore the 2♣ detour".
fn stayman_major_rebid(major: Suit) -> Rules {
    let other = Strain::from(other_major(major));
    let strain = Strain::from(major);
    // Invitational-5-4 reroute: when on and opener showed *hearts*, a 5♠4♥ hand has
    // its own forcing `2♠` rebid (it Staymaned rather than transferring), so the
    // heart raises and the `3♠` slam-try are capped at four spades — routing that
    // hand to 2♠ and sharpening `3♠` into a balanced slam try that *denies* five
    // spades.  Off the flag (or over a 2♠ answer, where 5♥4♠ transfers and never
    // reaches here) the cap `len(♠,..14)` is a no-op.
    let reroute = invitational_5card_majors() && major == Suit::Hearts;
    let spade_cap = if reroute {
        len(Suit::Spades, ..5)
    } else {
        len(Suit::Spades, ..14)
    };
    let mut rules = Rules::new();
    if reroute {
        // Forcing 5♠4♥, invitational through slam — opener picks ♥ (4-4) or ♠ (5-3)
        // and the level (see `answer_inv_5card_both`).  Responder's four hearts is
        // implied (it Staymaned, opener showed hearts), so `2♠` stays natural-spades
        // — flooring only its own strain keeps it unalerted (the artificial-alert
        // invariant); the spade-capped raises split off the ≤4-spade hands.
        rules = rules.rule(
            Bid::new(2, Strain::Spades),
            1.3,
            len(Suit::Spades, 5..) & hcp(8..),
        );
    }
    rules
        // Fit: artificial slam try / choice of game (balanced, or 16+); denies 5♠.
        .rule(
            Bid::new(3, other),
            1.4,
            len(major, 4..) & hcp(9..) & (balanced() | hcp(16..)) & spade_cap.clone(),
        )
        .alert(SLAM_TRY)
        // Fit: sign off in the major game — any upgrade past a flat eight (a ninth
        // trump, or working shape) commits to game opposite the 15-17 opener.
        .rule(
            Bid::new(4, strain),
            1.3,
            len(major, 4..)
                & described("game value for the fit", move |hand: Hand, _| {
                    fit_value(hand, major) >= 9
                })
                & spade_cap.clone(),
        )
        // Fit: invitational raise — a flat eight, four-card fit, no upgrade.
        .rule(
            Bid::new(3, strain),
            1.2,
            len(major, 4..)
                & described("invitational value for the fit", move |hand: Hand, _| {
                    fit_value(hand, major) == 8
                })
                & spade_cap.clone(),
        )
        // No fit: quantitative 4NT (as if the 2♣ detour never happened).
        .rule(
            Bid::new(4, Strain::Notrump),
            1.2,
            len(major, ..4) & hcp(16..=17),
        )
        // No fit: game / invitational notrump raise.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            len(major, ..4) & hcp(9..),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            1.0,
            len(major, ..4) & hcp(8..=8),
        )
}

/// A flat 4-3-3-3 — the one balanced shape with no doubleton
fn flat_4333() -> Cons<impl Constraint + Clone> {
    balanced()
        & len(Suit::Clubs, 3..)
        & len(Suit::Diamonds, 3..)
        & len(Suit::Hearts, 3..)
        & len(Suit::Spades, 3..)
}

/// Opener's reply to responder's `3OM` slam try / choice of game
///
/// A flat `(4333)` chooses notrump (`3NT`); a maximum (17) cue-bids the cheapest
/// honour control to cooperate; otherwise opener signs off in the major game.
fn stayman_slam_try_answer(major: Suit) -> Rules {
    let mut rules = Rules::new().rule(Bid::new(3, Strain::Notrump), 1.4, flat_4333());
    // Cheapest control cue with a maximum: each suit ranking below the major.
    let mut weight = 1.3;
    for cue in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        if Strain::from(cue) < Strain::from(major) {
            rules = rules.rule(
                Bid::new(4, Strain::from(cue)),
                weight,
                hcp(17..) & control_in(cue),
            );
            weight -= 0.05;
        }
    }
    // Minimum, or a maximum without a cheap control: sign off in game.
    rules.rule(Bid::new(4, Strain::from(major)), 1.0, hcp(0..))
}

/// Responder's artificial slam try after a Jacoby transfer completes
/// (`1NT–2♦–2♥–3♠` / `1NT–2♥–2♠–3♥`)
///
/// A single-suited five-card major with 16+ HCP agrees the transfer major and bids
/// the *other* major to ask for controls — opener cues with a maximum, else signs
/// off in game ([`stayman_slam_try_answer`]).  Denies a four-card other major (a
/// 5-4 hand shows its second suit instead).  Artificial — the bid is *not* that
/// major — so it carries the [`SLAM_TRY`] alert (the artificial-alert invariant).
/// Empty unless the slam try is on ([`set_transfer_slam_try`]).
fn transfer_slam_try_rebid(major: Suit) -> Rules {
    if !transfer_slam_try() {
        return Rules::new();
    }
    Rules::new()
        .rule(
            Bid::new(3, Strain::from(other_major(major))),
            1.4,
            len(major, 5..) & len(other_major(major), ..4) & hcp(16..),
        )
        .alert(SLAM_TRY)
}

/// Opener's answer to the post-transfer slam try (`…3♠` / `…3♥`)
///
/// Mirrors the direct four-major slam try ([`slam_try_answer`]): a **maximum** (17)
/// launches RKCB (`4NT`) and the [`slam`] 1430 ladder places the slam — installed
/// alongside this node — while a **minimum** signs off in the agreed major game
/// (`4M`, *not* pass: responder's `3OM` is artificial, so passing would strand a
/// 3-level part-contract in the wrong strain).
fn transfer_slam_try_answer(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.0, hcp(17..))
        .alert(slam::RKCB)
        .rule(Bid::new(4, Strain::from(major)), 0.0, hcp(..17))
}

/// Responder's rebid after opener denies a major (`1NT–2♣–2♦`)
///
/// Smolen: jump in the four-card major to show *five* in the other, game-forcing,
/// so the strong notrump hand declares.  Lacking 5–4, revert to notrump as if the
/// 2♣ detour never happened — invite `2NT`, game `3NT`, quantitative `4NT`.
fn stayman_no_major_rebid() -> Rules {
    let rules = Rules::new()
        .rule(
            Bid::new(3, Strain::Hearts),
            1.4,
            len(Suit::Hearts, 4..=4) & len(Suit::Spades, 5..) & hcp(9..),
        )
        .alert(SMOLEN)
        .rule(
            Bid::new(3, Strain::Spades),
            1.4,
            len(Suit::Spades, 4..=4) & len(Suit::Hearts, 5..) & hcp(9..),
        )
        .alert(SMOLEN)
        .rule(Bid::new(4, Strain::Notrump), 1.2, hcp(16..=17))
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(9..))
        .rule(Bid::new(2, Strain::Notrump), 1.0, hcp(8..=8));
    let rules = if crawling_stayman() {
        // Crawling Stayman: 4-4 majors short in diamonds (a bare 2♥, weak) — both
        // majors, pass-or-correct (see `answer_crawling_stayman`).  Gated by the
        // diamond shortness (≤1) that brought it here: garbage hands have 3+
        // diamonds and pass 2♦ instead.  Responder's four spades is implied by the
        // crawling 2♣, so 2♥ floors only hearts and stays unalerted natural (like
        // the 2♠ sibling).  Disjoint from every rule above (all need hcp ≥8).
        rules.rule(
            Bid::new(2, Strain::Hearts),
            1.4,
            len(Suit::Hearts, 4..) & len(Suit::Diamonds, ..=1) & hcp(..8),
        )
    } else {
        rules
    };
    if invitational_5card_majors() {
        // 5♠4♥, non-forcing invitational: opener denied hearts, so name the
        // five-card spade suit (natural, outranks the 2NT fallback).  Opener passes
        // a minimum or raises to game (see `answer_inv_5card_spades`).  A 5♠4♥
        // game-force jumped Smolen `3♥` above.  Responder's four hearts is implied
        // (it Staymaned), so `2♠` floors only spades and stays unalerted natural.
        rules.rule(
            Bid::new(2, Strain::Spades),
            1.1,
            len(Suit::Spades, 5..) & hcp(8..=8),
        )
    } else {
        rules
    }
}

/// Opener completes Smolen by bidding game in responder's shown five-card major
pub(super) fn smolen_completion(five_card: Suit) -> Rules {
    let strain = Strain::from(five_card);
    Rules::new()
        // Eight-card fit: bid game in the long major so opener declares.
        .rule(Bid::new(4, strain), 1.0, len(five_card, 3..))
        // No fit: notrump game.
        .rule(Bid::new(3, Strain::Notrump), 0.5, len(five_card, ..3))
}

/// Smolen at the three level: responder's jump after opener denies a major
/// (`…3♣–3♦`).  Game is already forced, so no strength gate.
pub(super) fn smolen_at_three() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Hearts),
            1.4,
            len(Suit::Hearts, 4..=4) & len(Suit::Spades, 5..),
        )
        .alert(SMOLEN)
        .rule(
            Bid::new(3, Strain::Spades),
            1.4,
            len(Suit::Spades, 4..=4) & len(Suit::Hearts, 5..),
        )
        .alert(SMOLEN)
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(0..))
}

/// Opener accepts a no-fit (2NT) Stayman invitation with a maximum, else passes
///
/// Responder invited with a bare 8, so a 1NT opener needs its 17 for game (8+17
/// = 25).  Authored rather than left to the floor: the keyless floor reads a
/// three-level suit response over our 1NT as forcing and so cannot *decline* an
/// invitational raise.
fn accept_invitation(game: Bid) -> Rules {
    Rules::new()
        .rule(game, 1.0, hcp(17..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's acceptance of an invitational major raise
///
/// With a maximum, bid the major game — but choose 3NT on a flat 4-3-3-3, where
/// notrump rates to play as well as the eight-card fit.  A minimum passes.
fn accept_major_invitation(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.1, hcp(17..) & flat_4333())
        .rule(Bid::new(4, Strain::from(major)), 1.0, hcp(17..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's reply to the crawl (`1NT–2♣–2♦–2♥`): drop-dead pass-or-correct
///
/// Opener denied both majors (≤3 each).  Pass the 4-3 heart fit; with only two
/// hearts correct to 2♠ (then ≥3 spades).  Short in *both* majors — only a
/// 5-card-minor 1NT can be 2-2 — flee to 3♣: responder is club-heavy (4414/4405),
/// so it is an 8-9 card fit, far better than a 4-2 major.
fn answer_crawling_stayman() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Clubs),
            1.0,
            len(Suit::Hearts, ..3) & len(Suit::Spades, ..3),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            1.0,
            len(Suit::Hearts, ..3) & len(Suit::Spades, 3..),
        )
        .rule(Call::Pass, 0.0, len(Suit::Hearts, 3..))
}

/// Opener's reply to the non-forcing `2♠` invite (`1NT–2♣–2♦–2♠`, auction A)
///
/// Responder is a bare-8 5♠4♥; opener denied both majors (so 2-3 spades).  With a
/// maximum (17) accept game — `4♠` on three-card support, else `3NT`; a minimum
/// passes the 5-2/5-3 spade partscore.
fn answer_inv_5card_spades() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Spades),
            1.2,
            hcp(17..) & len(Suit::Spades, 3..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            1.1,
            hcp(17..) & len(Suit::Spades, ..3),
        )
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's reply to the forcing `2♠` (`1NT–2♣–2♥–2♠`, auction B)
///
/// Responder is 5♠4♥, invitational through slam; opener has four hearts (so a 4-4
/// heart fit at least) and may hold three spades (a 5-3 spade fit).  Prefer the
/// spade fit when held.  A maximum (17) jumps to game; a minimum (15-16) signs the
/// invite back at the three level for responder to pass (8) or raise (9+).  Slam
/// past game is left to the floor's keycard/search.
// ponytail: a flat min/max split; control-showing replies are the upgrade path.
fn answer_inv_5card_both() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Spades),
            1.3,
            hcp(17..) & len(Suit::Spades, 3..),
        )
        .rule(
            Bid::new(4, Strain::Hearts),
            1.2,
            hcp(17..) & len(Suit::Spades, ..3),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            1.1,
            hcp(..17) & len(Suit::Spades, 3..),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            1.0,
            hcp(..17) & len(Suit::Spades, ..3),
        )
}

/// Responder passes or raises opener's three-level invite-back (auction B min)
///
/// Opener declined to `3♥`/`3♠` (a minimum); responder passes the bare 8 or accepts
/// game with 9+.
// ponytail: 9+ always bids game — slam tries past 4M are left to the floor.
fn inv_5card_raise(strain: Strain) -> Rules {
    Rules::new()
        .rule(Bid::new(4, strain), 1.0, hcp(9..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Responder's invitational 5-4 rebid after the heart transfer completes
/// (`1NT–2♦–2♥`, auctions C/D)
///
/// Both rebids are exactly-8 invitational with five hearts (shown by the
/// transfer).  `2NT` adds a four-card spade suit (auction D); `2♠` is an artificial
/// relay denying it (auction C, a single-suited heart invite).  Weaker and
/// game-forcing hands match no rule and fall through to the floor's natural
/// transfer continuations.
fn transfer_heart_invite_rebid() -> Rules {
    Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            len(Suit::Hearts, 5..) & len(Suit::Spades, 4..=4) & hcp(8..=8),
        )
        .alert(INV_5CARD)
        .rule(
            Bid::new(2, Strain::Spades),
            1.2,
            len(Suit::Hearts, 5..) & len(Suit::Spades, ..4) & hcp(8..=8),
        )
        .alert(INV_5CARD)
}

/// Opener's reply to the artificial single-suited-heart invite (`…2♥–2♠`, C)
///
/// Responder is a bare-8 with five hearts and no four-card spade suit.  A maximum
/// (17) accepts game — `4♥` on three-card support, else `3NT`; a minimum signs off
/// in `3♥` (5-3 fit) or `2NT` (no fit), which responder passes.
fn answer_transfer_heart_single() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Hearts),
            1.4,
            hcp(17..) & len(Suit::Hearts, 3..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            1.3,
            hcp(17..) & len(Suit::Hearts, ..3),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            1.1,
            hcp(..17) & len(Suit::Hearts, 3..),
        )
        .rule(Bid::new(2, Strain::Notrump), 0.0, hcp(0..))
}

/// Opener's reply to the `2NT` invite showing five hearts and four spades
/// (`…2♥–2NT`, D)
///
/// Prefer the 5-3 heart fit, then the 4-4 spade fit, then notrump.  A maximum (17)
/// bids game; a minimum signs off at the three level (or passes `2NT`), which
/// responder — a bare 8 — passes.
fn answer_transfer_heart_spade() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Hearts),
            1.6,
            hcp(17..) & len(Suit::Hearts, 3..),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            1.5,
            hcp(17..) & len(Suit::Hearts, ..3) & len(Suit::Spades, 4..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            1.4,
            hcp(17..) & len(Suit::Hearts, ..3) & len(Suit::Spades, ..4),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            1.2,
            hcp(..17) & len(Suit::Hearts, 3..),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            1.1,
            hcp(..17) & len(Suit::Hearts, ..3) & len(Suit::Spades, 4..),
        )
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Responder's invitational single-suited 5-spade rebid after the spade transfer
/// completes (`1NT–2♥–2♠`)
///
/// `2NT` shows five spades (the transfer), no four-card heart suit, and exactly-8
/// invitational values.  Unlike the heart side — where `2NT` is taken by the 5♥4♠
/// invite, forcing the single-suiter through an artificial `2♠` relay — here a 5♠4♥
/// hand Staymans, so `2NT` is free.  It pins the five-card spade suit, so it carries
/// the same `INV_5CARD` alert as its heart cousins (the alert reader decodes it);
/// six-card and game-forcing hands match no rule and fall to the floor.
fn transfer_spade_invite_rebid() -> Rules {
    Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            len(Suit::Spades, 5..) & len(Suit::Hearts, ..4) & hcp(8..=8),
        )
        .alert(INV_5CARD)
}

/// Opener's reply to the single-suited-spade invite (`…2♠–2NT`)
///
/// Responder is a bare-8 with five spades and no four-card heart suit.  A maximum
/// (17) accepts game — `4♠` on three-card support, else `3NT`; a minimum signs off
/// in `3♠` (5-3 fit) or passes `2NT` (no fit), which responder passes.  The 5-3 fit
/// out-scores 3NT even opposite a flat 4-3-3-3 maximum — responder's 5-3-3-2 always
/// brings a ruffing doubleton — so there is no flat-4333→3NT carve here (cf.
/// `accept_major_invitation`'s 4-4 case); see `examples/probe-fivecard-invite-eval`.
fn answer_transfer_spade_single() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Spades),
            1.4,
            hcp(17..) & len(Suit::Spades, 3..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            1.3,
            hcp(17..) & len(Suit::Spades, ..3),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            1.1,
            hcp(..17) & len(Suit::Spades, 3..),
        )
        .rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Puppet Stayman (1NT–3♣)
// ---------------------------------------------------------------------------

/// Opener's answer to Puppet Stayman: a five-card major, else 3♦ to deny
///
/// Puppet is balanced and game-forcing, so opener always cooperates — name a
/// five-card major (`3♥`/`3♠`), otherwise bid `3♦`, denying five but possibly
/// holding a four-card major for the Smolen-style 4-4 hunt below.
fn puppet_answers() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 1.0, len(Suit::Hearts, 5..))
        .rule(
            Bid::new(3, Strain::Spades),
            1.0,
            len(Suit::Spades, 5..) & len(Suit::Hearts, ..5),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            0.5,
            len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
}

/// Responder's rebid after opener names a five-card major over Puppet
///
/// Three-card support is an eight-card fit — bid game in the major so opener
/// declares; otherwise opener's major was responder's short one, so settle in
/// 3NT.  Puppet hands are balanced, so there is no splinter slam-try here (that
/// tool lives in the shapely 2♠ club structure).
fn puppet_major_rebid(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 1.0, len(major, 3..))
        .rule(Bid::new(3, Strain::Notrump), 0.5, len(major, ..3))
}

/// Responder's rebid after opener denies a five-card major (`1NT–3♣–3♦`)
///
/// Smolen-style: a four-card major (so responder is 4-3) bids the *shorter*
/// three-card major to show four in the longer, right-siding game to opener.
/// With no four-card major (3-3, or three and a short major) there is no 4-4 to
/// find — settle in 3NT.
fn puppet_deny_rebid() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Hearts),
            1.0,
            len(Suit::Spades, 4..=4) & len(Suit::Hearts, 3..=3),
        )
        .alert(SMOLEN)
        .rule(
            Bid::new(3, Strain::Spades),
            1.0,
            len(Suit::Hearts, 4..=4) & len(Suit::Spades, 3..=3),
        )
        .alert(SMOLEN)
        .rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..))
}

/// Opener completes the Puppet 4-4 hunt: game in responder's shown major, or 3NT
///
/// Responder's short-major bid named four cards in `shown_major`; raise to game
/// with four-card support, else 3NT.
fn puppet_smolen_completion(shown_major: Suit) -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::from(shown_major)),
            1.0,
            len(shown_major, 4..),
        )
        .rule(Bid::new(3, Strain::Notrump), 0.5, len(shown_major, ..4))
}

// ---------------------------------------------------------------------------
// Both-majors 3♦ (1NT–3♦ = 5+/5+ majors, invitational+)
// ---------------------------------------------------------------------------

/// Opener's answer to the both-majors 3♦: pick the strain by strength
///
/// With a maximum (17) jump to the eight-card major game, or 3NT when 2-2 in the
/// majors leaves only a seven-card fit.  A minimum (15–16) signs off in three of
/// the better major — spades whenever holding three, else hearts — leaving
/// responder to pass an invitation or raise with game values.  Authored, not
/// floored: the keyless floor misreads 3♦ as natural diamonds and forces game.
//
// ponytail: "better major" is spades-with-three, else hearts — it finds an
// eight-card fit when one exists but prefers spades on a tie (e.g. 3♠ on 3-4
// majors).  Good enough; refine only if the A/B asks for it.
fn five_five_major_answer() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Spades),
            1.2,
            hcp(17..) & len(Suit::Spades, 3..),
        )
        .rule(
            Bid::new(4, Strain::Hearts),
            1.2,
            hcp(17..) & len(Suit::Spades, ..3) & len(Suit::Hearts, 3..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            1.2,
            hcp(17..) & len(Suit::Spades, ..3) & len(Suit::Hearts, ..3),
        )
        .rule(Bid::new(3, Strain::Spades), 1.0, len(Suit::Spades, 3..))
        .rule(Bid::new(3, Strain::Hearts), 1.0, len(Suit::Spades, ..3))
}

/// Responder's decision over opener's minimum 3-level signoff
///
/// Opener showed 15–16 by signing off in `major`; responder raises to game with
/// the upper half of the invitational+ range and otherwise passes.  Needed
/// because the floor forces responder to game off the 3♦ opening and so could
/// not pass the invitation.  `points` again — responder is the 5-5 hand.
fn five_five_min_rebid(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 1.0, points(10..))
        .rule(Call::Pass, 0.9, points(..10))
}

// ---------------------------------------------------------------------------
// Minor-suit transfers (1NT–2NT diamonds, 1NT–2♠ clubs/invite)
// ---------------------------------------------------------------------------

/// Opener passes a weak responder retreat
///
/// Authored only to override the keyless floor, which reads a three-level suit
/// response to our 1NT as game-forcing and would otherwise refuse to pass.
fn pass_out() -> Rules {
    Rules::new().rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's reply to the 2NT diamond transfer: complete to 3♦ with a fit, else 3♣
///
/// Three-card diamond support is an assured eight-card fit — complete the
/// transfer.  Short diamonds bid `3♣` instead, pass-or-correct, letting a 5♦4♣
/// responder pick the better minor.
fn diamond_transfer_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 1.0, len(Suit::Diamonds, 3..))
        .rule(Bid::new(3, Strain::Clubs), 0.5, len(Suit::Diamonds, ..3))
}

/// Responder's rebid after opener completes the diamond transfer (`…2NT–3♦`)
///
/// Game values bid 3NT — a long suit bids game on fewer points (`threshold` ≈ 8,
/// below the 9 a balanced hand needs).  Otherwise pass the diamond partscore.
fn diamond_transfer_game(threshold: u8) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(threshold..))
        .rule(Call::Pass, 0.0, hcp(..threshold))
}

/// Responder's rebid after opener's pass-or-correct `3♣` (`…2NT–3♣`, short ♦)
///
/// Game values bid 3NT; a six-card diamond suit retreats to `3♦` (a 6-2 fit beats
/// the possible club misfit); otherwise (5♦4♣) pass and sit for opener's clubs.
fn diamond_transfer_correct(threshold: u8) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(threshold..))
        .rule(
            Bid::new(3, Strain::Diamonds),
            0.5,
            len(Suit::Diamonds, 6..) & hcp(..threshold),
        )
        .rule(Call::Pass, 0.0, len(Suit::Diamonds, ..6) & hcp(..threshold))
}

/// A six-card club one-suiter short in `short` with game values — a splinter shape
fn club_splinter(short: Suit, threshold: u8) -> Cons<impl Constraint + Clone> {
    len(Suit::Clubs, 6..) & hcp(threshold..) & len(short, ..2)
}

/// A six-card club hand with game values and no singleton — game-going, slamless
fn club_no_shortness(threshold: u8) -> Cons<impl Constraint + Clone> {
    len(Suit::Clubs, 6..)
        & hcp(threshold..)
        & len(Suit::Diamonds, 2..)
        & len(Suit::Hearts, 2..)
        & len(Suit::Spades, 2..)
}

/// Opener's reply to the two-way 2♠: `3♣` with a maximum, `2NT` with a minimum
///
/// Showing strength lets responder pass-or-correct safely: the weak club hand
/// lands in `3♣` either way, the balanced invite plays `2NT` (min) or `3NT`
/// (max), and a game-going club hand splinters.
fn two_spade_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Clubs), 1.0, hcp(17..))
        .rule(Bid::new(2, Strain::Notrump), 0.9, hcp(0..))
}

/// Responder's pass-or-correct after opener's minimum `2NT` over the two-way 2♠
fn two_spade_over_min() -> Rules {
    Rules::new()
        // Balanced invite: opener is minimum, settle in 2NT.
        .rule(Call::Pass, 0.0, hcp(8..=8) & balanced())
        // Weak club one-suiter: correct to the club partscore.
        .rule(
            Bid::new(3, Strain::Clubs),
            0.8,
            len(Suit::Clubs, 6..) & hcp(..8),
        )
        // Game-going clubs with a singleton: splinter so opener picks 3NT or 5♣.
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.0,
            club_splinter(Suit::Diamonds, 8),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(3, Strain::Hearts),
            1.0,
            club_splinter(Suit::Hearts, 8),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(3, Strain::Spades),
            1.0,
            club_splinter(Suit::Spades, 8),
        )
        .alert(SPLINTER)
        // Game-going clubs without a singleton: 3NT.
        .rule(Bid::new(3, Strain::Notrump), 0.9, club_no_shortness(8))
        .alert(PUPPET)
}

/// Responder's action after opener's maximum `3♣` over the two-way 2♠
fn two_spade_over_max() -> Rules {
    Rules::new()
        // Weak club one-suiter: pass the club partscore.
        .rule(Call::Pass, 0.0, len(Suit::Clubs, 6..) & hcp(..8))
        // Game-going clubs with a singleton: splinter.
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.0,
            club_splinter(Suit::Diamonds, 8),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(3, Strain::Hearts),
            1.0,
            club_splinter(Suit::Hearts, 8),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(3, Strain::Spades),
            1.0,
            club_splinter(Suit::Spades, 8),
        )
        .alert(SPLINTER)
        // Balanced invite (opener maximum → accept game) or game clubs without a
        // singleton: 3NT.
        .rule(
            Bid::new(3, Strain::Notrump),
            0.9,
            (hcp(8..=8) & balanced()) | club_no_shortness(8),
        )
}

/// Opener picks the game over responder's club splinter: 3NT with the short suit
/// stopped, else 5♣
fn pick_game_over_club_splinter(short: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.0, stopper_in(short))
        .rule(Bid::new(5, Strain::Clubs), 0.9, hcp(0..))
}

// ---------------------------------------------------------------------------
// European minor scheme (1NT–2♠ clubs, 1NT–2NT invite, 1NT–3♣ diamonds)
// ---------------------------------------------------------------------------
//
// ponytail: opener always completes the 2♠/3♣ transfers — no super-accept — and
// the 5♦4♣ two-suiter is folded into the 3♣ diamond transfer (no room below 3♦ to
// show the clubs).  Refine only if an A/B asks for it.

/// Opener completes the European club transfer: `3♣` (the 2♠ bidder has clubs)
fn european_two_spade_answer() -> Rules {
    Rules::new().rule(Bid::new(3, Strain::Clubs), 0.0, hcp(0..))
}

/// Responder's rebid after opener completes the European club transfer (`…2♠–3♣`)
///
/// A weak six-card club one-suiter passes the partscore; a game-going hand
/// splinters in its singleton, or bids 3NT with no shortness.  Reuses the two-way
/// 2♠ club machinery minus its balanced-invite arm — that hand is the European 2NT.
fn european_two_spade_rebid() -> Rules {
    Rules::new()
        .rule(Call::Pass, 0.0, len(Suit::Clubs, 6..) & hcp(..8))
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.0,
            club_splinter(Suit::Diamonds, 8),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            1.0,
            club_splinter(Suit::Hearts, 8),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            1.0,
            club_splinter(Suit::Spades, 8),
        )
        .rule(Bid::new(3, Strain::Notrump), 0.9, club_no_shortness(8))
}

/// Opener's reply to the European 2NT invite: `3NT` with a maximum, else pass
///
/// The 2NT bidder is a balanced eight; opposite a 17 (`25` combined) opener accepts
/// game, otherwise passes and plays 2NT — reproducing the natural-2NT outcome.
fn european_two_nt_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(17..))
        .rule(Call::Pass, 0.0, hcp(..17))
}

/// Opener completes the European diamond transfer: `3♦`
fn european_three_club_answer() -> Rules {
    Rules::new().rule(Bid::new(3, Strain::Diamonds), 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// 2NT-strength response structure (2NT opening and 2♣–2x–2NT rebid)
// ---------------------------------------------------------------------------

/// Responses to a 2NT-strength notrump (3-level Stayman/transfers, 4NT invite)
///
/// Used after both the direct 2NT opening (20–21 balanced) and opener's 2NT
/// rebid after 2♣ (22–24 balanced).
fn two_notrump_responses() -> Rules {
    Rules::new()
        // 3-level Jacoby transfers.
        .rule(Bid::new(3, Strain::Diamonds), 2.0, len(Suit::Hearts, 5..))
        .alert(JACOBY)
        .rule(Bid::new(3, Strain::Hearts), 2.0, len(Suit::Spades, 5..))
        .alert(JACOBY)
        // 3-level Stayman: a four-card major and at least some values, but never a
        // flat 4-3-3-3 (it bids notrump directly, as over a 1NT opening).
        .rule(
            Bid::new(3, Strain::Clubs),
            1.5,
            (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4)) & hcp(5..) & !flat_4333(),
        )
        // Quantitative 4NT slam invite (balanced, no four-card major).
        .rule(
            Bid::new(4, Strain::Notrump),
            1.2,
            hcp(11..=12) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        // 3NT to play: game values, no major fit.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(5..=10) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(Call::Pass, 0.0, hcp(..5))
}

/// Opener's answer to 3-level Stayman: a four-card major, else 3♦
fn stayman_answers_at_three() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 1.0, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(3, Strain::Spades),
            1.0,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            0.5,
            len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
}

/// Complete a 3-level transfer by bidding the anchor suit
fn complete_transfer_at_three(into: Suit) -> Rules {
    Rules::new().rule(Bid::new(3, Strain::from(into)), 1.0, hcp(0..))
}

/// Opener's answer to the quantitative 4NT: accept or decline the slam invite
///
/// `accept_hcp` is the minimum HCP to accept: 21 after a 2NT opening (20–21),
/// 24 after a 2♣–2x–2NT sequence (22–24).
fn quantitative_answer(accept_hcp: u8) -> Rules {
    Rules::new()
        .rule(Bid::new(6, Strain::Notrump), 1.0, hcp(accept_hcp..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Simple continuations after an 18–19 2NT rebid
// ---------------------------------------------------------------------------

/// Responder's call after opener's 18–19 2NT rebid
///
/// 6+ HCP bids 3NT; 12–13 makes a quantitative 4NT invite; fewer points pass.
fn after_rebid_two_notrump() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.2, hcp(12..=13))
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(6..))
        .rule(Call::Pass, 0.0, hcp(..6))
}

/// Opener's reply to the quantitative raise opposite the 18–19 rebid
///
/// Accept (6NT) with a maximum 19 HCP, decline (pass) otherwise.
fn accept_quantitative_nineteen() -> Rules {
    Rules::new()
        .rule(Bid::new(6, Strain::Notrump), 1.0, hcp(19..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register all notrump continuations into the constructive book
///
/// Registers the 1NT structure (Stayman, transfers, 4NT quantitative), the
/// 2NT-strength structure (3-level Stayman/transfers, 4NT invite) under three
/// base prefixes (direct 2NT opening and the two 2♣–2x–2NT auctions), and
/// simple responses after opener's 18–19 2NT rebid.
pub(super) fn register(book: &mut Trie) {
    register_one_nt(book);
    register_two_nt_and_rebids(book);
}

/// Register the standard 1NT-opening response structure
///
/// Stayman 2♣, Jacoby transfers 2♦/2♥, notrump raises, and the quantitative
/// 4NT invite — the baseline 2/1 treatment.  Factored from the
/// 2NT-strength/18–19-rebid block ([`register_two_nt_and_rebids`]) so an
/// alternative 1NT scheme could replace just this part.
pub(super) fn register_one_nt(book: &mut Trie) {
    let one_nt = call(1, Strain::Notrump);
    let four_nt = call(4, Strain::Notrump);

    let two_c = call(2, Strain::Clubs);
    let two_d = call(2, Strain::Diamonds);
    let two_h = call(2, Strain::Hearts);
    let two_s = call(2, Strain::Spades);
    let three_h = call(3, Strain::Hearts);
    let three_s = call(3, Strain::Spades);

    // The 2♠/2NT/3♣ continuations diverge by minor scheme; the response node
    // itself self-gates inside `notrump_responses`.
    let puppet = notrump_minors() == PUPPET;

    insert_uncontested(book, &[one_nt], notrump_responses());
    // Stayman answers and transfer completions.  The uncontested wrapper folds in
    // the opt-in max-showing overlays (both-majors min/max, max five-card jump).
    insert_uncontested(book, &[one_nt, two_c], stayman_answers_uncontested());
    insert_uncontested(book, &[one_nt, two_d], complete_transfer(Suit::Hearts));
    insert_uncontested(book, &[one_nt, two_h], complete_transfer(Suit::Spades));
    // Quantitative 4NT answer.
    insert_uncontested(book, &[one_nt, four_nt], quantitative_answer(17));

    // --- Stayman continuations ------------------------------------------------
    //
    // Responder's rebid after opener shows a major, and opener's reply to the
    // artificial 3OM slam try.
    insert_uncontested(
        book,
        &[one_nt, two_c, two_h],
        stayman_major_rebid(Suit::Hearts),
    );
    insert_uncontested(
        book,
        &[one_nt, two_c, two_s],
        stayman_major_rebid(Suit::Spades),
    );
    insert_uncontested(
        book,
        &[one_nt, two_c, two_h, three_s],
        stayman_slam_try_answer(Suit::Hearts),
    );
    insert_uncontested(
        book,
        &[one_nt, two_c, two_s, three_h],
        stayman_slam_try_answer(Suit::Spades),
    );
    // Responder's rebid after opener denies a major (Smolen, else revert to NT),
    // and opener's Smolen completion (game in responder's five-card major).
    insert_uncontested(book, &[one_nt, two_c, two_d], stayman_no_major_rebid());
    insert_uncontested(
        book,
        &[one_nt, two_c, two_d, three_h],
        smolen_completion(Suit::Spades),
    );
    insert_uncontested(
        book,
        &[one_nt, two_c, two_d, three_s],
        smolen_completion(Suit::Hearts),
    );
    // Opener accepts or declines responder's invitation (major raise, or the
    // no-fit 2NT) — authored, since the floor reads the three-level raise as
    // forcing and could not decline.
    let two_nt = call(2, Strain::Notrump);
    insert_uncontested(
        book,
        &[one_nt, two_c, two_h, three_h],
        accept_major_invitation(Suit::Hearts),
    );
    insert_uncontested(
        book,
        &[one_nt, two_c, two_s, three_s],
        accept_major_invitation(Suit::Spades),
    );
    for major_answer in [two_h, two_s, two_d] {
        insert_uncontested(
            book,
            &[one_nt, two_c, major_answer, two_nt],
            accept_invitation(Bid::new(3, Strain::Notrump)),
        );
    }
    // Opener's quantitative accept after a no-fit revert to 4NT.
    insert_uncontested(
        book,
        &[one_nt, two_c, two_h, four_nt],
        quantitative_answer(17),
    );
    insert_uncontested(
        book,
        &[one_nt, two_c, two_s, four_nt],
        quantitative_answer(17),
    );
    insert_uncontested(
        book,
        &[one_nt, two_c, two_d, four_nt],
        quantitative_answer(17),
    );

    // Crawling Stayman: opener's pass-or-correct reply to the 2♥ crawl
    // (`1NT–2♣–2♦–2♥`).  The doubled tail `1NT–2♣–2♦–(X)–2♥` is systems-on via the
    // rebase in `competition.rs`.
    if crawling_stayman() {
        insert_uncontested(
            book,
            &[one_nt, two_c, two_d, two_h],
            answer_crawling_stayman(),
        );
    }

    // --- Invitational 5-4 majors (gated; see `set_invitational_5card_majors`) ---
    //
    // 5♠4♥ Staymans and rebids `2♠` over opener's 2♦ (non-forcing) or 2♥ (forcing,
    // through slam); 5♥4♠ transfers to hearts and rebids `2NT` (showing the spades)
    // or an artificial `2♠` (denying them).  Opener's accept/decline is authored —
    // the floor cannot decline an invitation — and the remaining tail (responder
    // passing a chosen partscore) falls to the floor.
    if invitational_5card_majors() {
        // A: opener over the non-forcing 2♠ (1NT–2♣–2♦–2♠).
        insert_uncontested(
            book,
            &[one_nt, two_c, two_d, two_s],
            answer_inv_5card_spades(),
        );
        // B: opener over the forcing 2♠ (1NT–2♣–2♥–2♠), then responder's pass/raise
        // of a minimum's three-level invite-back.
        insert_uncontested(
            book,
            &[one_nt, two_c, two_h, two_s],
            answer_inv_5card_both(),
        );
        insert_uncontested(
            book,
            &[one_nt, two_c, two_h, two_s, three_h],
            inv_5card_raise(Strain::Hearts),
        );
        insert_uncontested(
            book,
            &[one_nt, two_c, two_h, two_s, three_s],
            inv_5card_raise(Strain::Spades),
        );
        // C/D: opener's two replies to responder's invitational heart-transfer
        // rebid (the single-suited `2♠` relay, and the 5♥4♠-showing `2NT`).  The
        // responder rebid itself is inserted below, chained with the six-card
        // invite (both share the `1NT–2♦–2♥` node).
        insert_uncontested(
            book,
            &[one_nt, two_d, two_h, two_s],
            answer_transfer_heart_single(),
        );
        insert_uncontested(
            book,
            &[one_nt, two_d, two_h, two_nt],
            answer_transfer_heart_spade(),
        );
        // E: opener's reply to the single-suited spade invite (`1NT–2♥–2♠–2NT`); the
        // responder rebid itself is inserted below, chained with the six-card invite.
        insert_uncontested(
            book,
            &[one_nt, two_h, two_s, two_nt],
            answer_transfer_spade_single(),
        );
    }

    // --- Six-card-major game invite (gated; see `set_sixcard_invite_floor`) -----
    //
    // Just below the Texas blast floor, responder transfers and jumps to `3M` (a
    // natural invite); opener accepts game or passes `3M` on `point_count + trump
    // length`.  The heart responder node coexists with the 5-4 structure's `2♠`/`2NT`
    // relays — disjoint by HCP (an 8-count 6-bagger has `point_count + length ≥ 14`
    // and blasts `4♣`, never transferring), so the node chains both features.
    if invitational_5card_majors() || sixcard_invite_active() || transfer_slam_try() {
        let mut heart_rebid = Rules::new();
        if invitational_5card_majors() {
            heart_rebid = heart_rebid.chain(transfer_heart_invite_rebid());
        }
        heart_rebid = heart_rebid.chain(sixcard_invite_rebid(Suit::Hearts));
        heart_rebid = heart_rebid.chain(transfer_slam_try_rebid(Suit::Hearts));
        insert_uncontested(book, &[one_nt, two_d, two_h], heart_rebid);
    }
    // The spade-transfer rebid node carries the single-suited 5♠ invite (`2NT` — the
    // spade mirror of the heart `2♠` relay; `2NT` is free here because 5♠4♥ Staymans)
    // and the six-card spade invite (`3♠`), disjoint by strength — exactly like the
    // heart node above.
    if invitational_5card_majors() || sixcard_invite_active() || transfer_slam_try() {
        let mut spade_rebid = Rules::new();
        if invitational_5card_majors() {
            spade_rebid = spade_rebid.chain(transfer_spade_invite_rebid());
        }
        spade_rebid = spade_rebid.chain(sixcard_invite_rebid(Suit::Spades));
        spade_rebid = spade_rebid.chain(transfer_slam_try_rebid(Suit::Spades));
        insert_uncontested(book, &[one_nt, two_h, two_s], spade_rebid);
    }
    // Opener's RKCB-or-sign-off over the post-transfer slam try (`3♠` agrees hearts,
    // `3♥` agrees spades), plus the keycard ladder rooted at each — the same proven
    // machinery as the direct four-major slam try, so the auction never dangles.
    if transfer_slam_try() {
        insert_uncontested(
            book,
            &[one_nt, two_d, two_h, three_s],
            transfer_slam_try_answer(Suit::Hearts),
        );
        insert_uncontested(
            book,
            &[one_nt, two_h, two_s, three_h],
            transfer_slam_try_answer(Suit::Spades),
        );
        slam::install_rkcb(book, &[one_nt, two_d, two_h, three_s], Suit::Hearts);
        slam::install_rkcb(book, &[one_nt, two_h, two_s, three_h], Suit::Spades);
    }
    if sixcard_invite_active() {
        // Opener's accept/decline of the six-card invite for both majors.
        insert_uncontested(
            book,
            &[one_nt, two_d, two_h, three_h],
            accept_sixcard_invitation(Suit::Hearts),
        );
        insert_uncontested(
            book,
            &[one_nt, two_h, two_s, three_s],
            accept_sixcard_invitation(Suit::Spades),
        );
    }

    // --- Opt-in max-showing overlays (both-majors min/max, max five-card jump) -
    //
    // Responder's placement over opener's artificial 2NT/3♣ (both four-card
    // majors) and natural 3♥/3♠ jump (max five-card major).  Opener has limited
    // itself, so its follow-up is the floor's pass.
    if stayman_both_majors() {
        // Max-only, right-siding relay: opener's 2NT shows both four-card majors
        // with a maximum (17); responder names their major (3♣ = hearts, 3♦ =
        // spades); opener completes (3♥/3♠) so the strong hand declares; responder
        // places game.  A minimum opener bids 2♥ naturally (no node needed).
        // ponytail: slam below game left to the floor; author a node if it underbids.
        insert_uncontested(book, &[one_nt, two_c, two_nt], both_majors_max_responder());
        insert_uncontested(
            book,
            &[one_nt, two_c, two_nt, call(3, Strain::Clubs)],
            both_majors_relay_complete(Suit::Hearts),
        );
        insert_uncontested(
            book,
            &[one_nt, two_c, two_nt, call(3, Strain::Diamonds)],
            both_majors_relay_complete(Suit::Spades),
        );
        insert_uncontested(
            book,
            &[one_nt, two_c, two_nt, call(3, Strain::Clubs), three_h],
            both_majors_relay_placement(Suit::Hearts),
        );
        insert_uncontested(
            book,
            &[one_nt, two_c, two_nt, call(3, Strain::Diamonds), three_s],
            both_majors_relay_placement(Suit::Spades),
        );
    }
    if stayman_5card_max() {
        insert_uncontested(
            book,
            &[one_nt, two_c, three_h],
            five_card_max_rebid(Suit::Hearts),
        );
        insert_uncontested(
            book,
            &[one_nt, two_c, three_s],
            five_card_max_rebid(Suit::Spades),
        );
    }

    // --- 3♣ response (Puppet Stayman, or European diamond transfer) -----------
    let three_c = call(3, Strain::Clubs);
    let three_d = call(3, Strain::Diamonds);
    if puppet {
        // Puppet Stayman: opener shows a five-card major (3♥/3♠) or denies with
        // 3♦; responder raises a 5-3 fit, or — Smolen-style after 3♦ — bids the
        // shorter major to find a 4-4 with opener declaring.
        insert_uncontested(book, &[one_nt, three_c], puppet_answers());
        insert_uncontested(
            book,
            &[one_nt, three_c, three_h],
            puppet_major_rebid(Suit::Hearts),
        );
        insert_uncontested(
            book,
            &[one_nt, three_c, three_s],
            puppet_major_rebid(Suit::Spades),
        );
        insert_uncontested(book, &[one_nt, three_c, three_d], puppet_deny_rebid());
        // Responder's shorter-major bid named four cards in the *other* major.
        insert_uncontested(
            book,
            &[one_nt, three_c, three_d, three_h],
            puppet_smolen_completion(Suit::Spades),
        );
        insert_uncontested(
            book,
            &[one_nt, three_c, three_d, three_s],
            puppet_smolen_completion(Suit::Hearts),
        );
    } else {
        // European: 3♣ is a transfer to diamonds — opener completes 3♦, responder
        // bids 3NT with game values or passes the diamond partscore.
        insert_uncontested(book, &[one_nt, three_c], european_three_club_answer());
        insert_uncontested(book, &[one_nt, three_c, three_d], diamond_transfer_game(8));
    }

    // --- Both-majors 3♦ (1NT–3♦) ----------------------------------------------
    //
    // Opener signs off in 3M with a minimum or jumps to game (4M / 3NT) with a
    // maximum; over a minimum signoff responder passes an invitation or raises.
    insert_uncontested(book, &[one_nt, three_d], five_five_major_answer());
    insert_uncontested(
        book,
        &[one_nt, three_d, three_h],
        five_five_min_rebid(Suit::Hearts),
    );
    insert_uncontested(
        book,
        &[one_nt, three_d, three_s],
        five_five_min_rebid(Suit::Spades),
    );

    // --- South African Texas (1NT–4♣/4♦ transfers, 1NT–4♥/4♠ slam tries) ------
    //
    // To-play transfers: opener names the game in the anchor major and declares.
    // Slam tries: opener passes (minimum) or launches RKCB (maximum); the 1430
    // ladder in `slam` registers the keycard exchange and the slam placement.
    let four_c = call(4, Strain::Clubs);
    let four_d = call(4, Strain::Diamonds);
    let four_h = call(4, Strain::Hearts);
    let four_s = call(4, Strain::Spades);
    insert_uncontested(book, &[one_nt, four_c], complete_texas(Suit::Hearts));
    insert_uncontested(book, &[one_nt, four_d], complete_texas(Suit::Spades));
    insert_uncontested(book, &[one_nt, four_h], slam_try_answer());
    insert_uncontested(book, &[one_nt, four_s], slam_try_answer());
    slam::install_rkcb(book, &[one_nt, four_h], Suit::Hearts);
    slam::install_rkcb(book, &[one_nt, four_s], Suit::Spades);

    // --- 2NT response (diamond transfer, or European balanced invite) ---------
    if puppet {
        // Transfer to diamonds: opener completes 3♦ with a fit, else pass-or-correct
        // 3♣; a weak retreat to 3♦ over that 3♣ must be passed.
        insert_uncontested(book, &[one_nt, two_nt], diamond_transfer_answer());
        insert_uncontested(book, &[one_nt, two_nt, three_d], diamond_transfer_game(8));
        insert_uncontested(
            book,
            &[one_nt, two_nt, three_c],
            diamond_transfer_correct(8),
        );
        insert_uncontested(book, &[one_nt, two_nt, three_c, three_d], pass_out());
    } else {
        // European: 2NT is a balanced invite — opener accepts game (3NT) or passes.
        insert_uncontested(book, &[one_nt, two_nt], european_two_nt_answer());
    }

    // --- 2♠ response (two-way clubs/invite, or European club transfer) --------
    if puppet {
        insert_uncontested(book, &[one_nt, two_s], two_spade_answer());
        insert_uncontested(book, &[one_nt, two_s, two_nt], two_spade_over_min());
        insert_uncontested(book, &[one_nt, two_s, three_c], two_spade_over_max());
        // Weak-club correction to 3♣ over opener's minimum 2NT — opener must pass.
        insert_uncontested(book, &[one_nt, two_s, two_nt, three_c], pass_out());
        // Opener picks 3NT/5♣ over the game-going club splinter (3♦/3♥/3♠), after
        // either the minimum (2NT) or maximum (3♣) reply.
        for opener_reply in [two_nt, three_c] {
            for short in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                insert_uncontested(
                    book,
                    &[one_nt, two_s, opener_reply, call(3, Strain::from(short))],
                    pick_game_over_club_splinter(short),
                );
            }
        }
    } else {
        // European: 2♠ is a transfer to clubs — opener completes 3♣, responder
        // passes (weak) or splinters (game); opener picks the game over a splinter.
        insert_uncontested(book, &[one_nt, two_s], european_two_spade_answer());
        insert_uncontested(book, &[one_nt, two_s, three_c], european_two_spade_rebid());
        for short in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            insert_uncontested(
                book,
                &[one_nt, two_s, three_c, call(3, Strain::from(short))],
                pick_game_over_club_splinter(short),
            );
        }
    }
}

/// Register the 2NT-strength structure and the 18–19 2NT-rebid continuations
///
/// The half of the notrump book that an alternative 1NT-opening scheme would
/// keep unchanged — only [`register_one_nt`] varies.
pub(super) fn register_two_nt_and_rebids(book: &mut Trie) {
    let one_nt = call(1, Strain::Notrump);
    let two_nt = call(2, Strain::Notrump);
    let four_nt = call(4, Strain::Notrump);

    // --- 2NT-strength structure ----------------------------------------------
    //
    // Three base prefixes (our calls only; passes are interleaved by
    // `insert_uncontested`):
    //   1. [2NT]                  → direct 2NT opening (20–21), accept_hcp = 21
    //   2. [2♣, 2♦, 2NT]         → 2♣–2♦–2NT sequence (22–24), accept_hcp = 24
    //   3. [2♣, 2♥, 2NT]         → 2♣–2♥–2NT sequence (22–24), accept_hcp = 24

    let bases: &[(&[Call], u8)] = &[
        (&[two_nt], 21),
        (
            &[call(2, Strain::Clubs), call(2, Strain::Diamonds), two_nt],
            24,
        ),
        (
            &[call(2, Strain::Clubs), call(2, Strain::Hearts), two_nt],
            24,
        ),
    ];

    for (base, accept_hcp) in bases {
        // Responses to the 2NT bid.
        insert_uncontested(book, base, two_notrump_responses());

        // Stayman answers and transfer completions at the three level.
        let extend = |tail: Call| -> Vec<Call> {
            base.iter().copied().chain(core::iter::once(tail)).collect()
        };
        insert_uncontested(
            book,
            &extend(call(3, Strain::Clubs)),
            stayman_answers_at_three(),
        );
        insert_uncontested(
            book,
            &extend(call(3, Strain::Diamonds)),
            complete_transfer_at_three(Suit::Hearts),
        );
        insert_uncontested(
            book,
            &extend(call(3, Strain::Hearts)),
            complete_transfer_at_three(Suit::Spades),
        );

        // Quantitative 4NT answer.
        insert_uncontested(book, &extend(four_nt), quantitative_answer(*accept_hcp));

        // Smolen after 3♣ Stayman when opener denies a major (3♦): responder
        // jumps to show 5–4 in the majors, opener completes to game in the long
        // one.  Mirrors the 1NT-level structure one level up.
        let extend2 =
            |a: Call, b: Call| -> Vec<Call> { base.iter().copied().chain([a, b]).collect() };
        let extend3 = |a: Call, b: Call, c: Call| -> Vec<Call> {
            base.iter().copied().chain([a, b, c]).collect()
        };
        let (three_c, three_d) = (call(3, Strain::Clubs), call(3, Strain::Diamonds));
        let (three_h, three_s) = (call(3, Strain::Hearts), call(3, Strain::Spades));
        insert_uncontested(book, &extend2(three_c, three_d), smolen_at_three());
        insert_uncontested(
            book,
            &extend3(three_c, three_d, three_h),
            smolen_completion(Suit::Spades),
        );
        insert_uncontested(
            book,
            &extend3(three_c, three_d, three_s),
            smolen_completion(Suit::Hearts),
        );
    }

    // --- 18–19 2NT rebid continuations --------------------------------------
    //
    // The auctions where opener's existing rebid table carries 2NT = 18–19.
    // Each prefix is [opener's opening, responder's first call] — our side's
    // two calls that precede the rebid.

    let rebid_prefixes: &[&[Call]] = &[
        &[call(1, Strain::Hearts), call(1, Strain::Spades)],
        &[call(1, Strain::Clubs), call(1, Strain::Diamonds)],
        &[call(1, Strain::Clubs), call(1, Strain::Hearts)],
        &[call(1, Strain::Clubs), call(1, Strain::Spades)],
        &[call(1, Strain::Diamonds), call(1, Strain::Hearts)],
        &[call(1, Strain::Diamonds), call(1, Strain::Spades)],
        &[call(1, Strain::Hearts), one_nt],
        &[call(1, Strain::Spades), one_nt],
    ];

    for prefix in rebid_prefixes {
        // Responder's action over opener's 2NT rebid.
        let mut our = prefix.to_vec();
        our.push(two_nt);
        insert_uncontested(book, &our, after_rebid_two_notrump());

        // Opener's reply to the quantitative 4NT raise.
        our.push(four_nt);
        insert_uncontested(book, &our, accept_quantitative_nineteen());
    }
}

#[cfg(test)]
mod tests {
    use crate::american;
    use crate::bidding::{Family, System};
    use contract_bridge::auction::{Call, RelativeVulnerability};
    use contract_bridge::{Bid, Strain};

    const P: Call = Call::Pass;

    fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid::new(level, strain))
    }

    /// The highest-logit call `american()` assigns the hand at the auction
    fn best(auction: &[Call], hand: &str) -> Call {
        let hand = hand.parse().expect("valid test hand");
        let logits = american()
            .against(Family::NATURAL)
            .classify(hand, RelativeVulnerability::NONE, auction)
            .expect("a decision");
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("the logits array is never empty")
    }

    /// The revised South African Texas: 4♣/4♦ to-play transfers and the 4♥/4♠
    /// non-forcing slam try wired into RKCB, end to end through `american()`.
    #[test]
    fn south_african_texas_slam_try() {
        let one_nt = [bid(1, Strain::Notrump), P];

        // Responder, 6 hearts: a 16-count makes the direct 4♥ slam try; a 10-count
        // takes the 4♣ to-play transfer.
        assert_eq!(best(&one_nt, "42.AKJ872.KQ4.K2"), bid(4, Strain::Hearts));
        assert_eq!(best(&one_nt, "42.AKJ872.Q43.32"), bid(4, Strain::Clubs));

        // Opener over the slam try (1NT–P–4♥–P): a maximum (17) launches RKCB, a
        // minimum (15) signs off by passing the major game.
        let over_try = [bid(1, Strain::Notrump), P, bid(4, Strain::Hearts), P];
        assert_eq!(best(&over_try, "KQ3.K53.AQ54.K92"), bid(4, Strain::Notrump));
        assert_eq!(best(&over_try, "KQ3.K53.KQ54.Q92"), P);

        // Opener completes the 4♣ to-play transfer (1NT–P–4♣–P) → 4♥.
        let over_transfer = [bid(1, Strain::Notrump), P, bid(4, Strain::Clubs), P];
        assert_eq!(
            best(&over_transfer, "KQ3.K53.KQ54.Q92"),
            bid(4, Strain::Hearts)
        );

        // RKCB is wired: responder answers keycards over 4NT (A♥+K♥ = 2, no ♥Q → 5♥),
        // then the asker with 3 keycards places the small slam.
        let over_ask = [
            bid(1, Strain::Notrump),
            P,
            bid(4, Strain::Hearts),
            P,
            bid(4, Strain::Notrump),
            P,
        ];
        assert_eq!(best(&over_ask, "42.AKJ872.KQ4.K2"), bid(5, Strain::Hearts));
        let over_answer = [
            bid(1, Strain::Notrump),
            P,
            bid(4, Strain::Hearts),
            P,
            bid(4, Strain::Notrump),
            P,
            bid(5, Strain::Hearts),
            P,
        ];
        assert_eq!(
            best(&over_answer, "KQ3.AK3.AQ54.J92"),
            bid(6, Strain::Hearts)
        );
    }

    /// The opt-in six-card-major game invite: just below the Texas blast floor,
    /// responder transfers and jumps to `3M`; opener accepts game or passes `3M`
    /// on `point_count + trump length`.
    #[test]
    fn sixcard_major_invite() {
        use crate::bidding::american::set_sixcard_invite_floor;

        let one_nt = [bid(1, Strain::Notrump), P];
        // 6 hearts, ♥KQ + ♠J = 6 HCP, 6-3-2-2: point_count 7 (+1 unbalanced),
        // point_count + length = 13 — one below the blast floor (14), so it invites.
        let inv = "J43.KQ8765.32.32";
        // 6 hearts, ♥KQ only = 5 HCP, point_count 6, sum 12 — too weak to invite.
        let weak = "543.KQ8765.32.32";

        // Turned off (floor 14 == blast floor): the invite hand transfers and the
        // floor handles the rebid — no authored 3♥ invite.
        set_sixcard_invite_floor(14);
        let after_transfer = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Diamonds),
            P,
            bid(2, Strain::Hearts),
            P,
        ];
        assert_ne!(best(&after_transfer, inv), bid(3, Strain::Hearts));

        // On by default (floor 13): the invite hand transfers (2♦) then jumps to 3♥;
        // the weak hand stays out of the invite.
        set_sixcard_invite_floor(13);
        assert_eq!(best(&one_nt, inv), bid(2, Strain::Diamonds));
        assert_eq!(best(&after_transfer, inv), bid(3, Strain::Hearts));
        assert_ne!(best(&after_transfer, weak), bid(3, Strain::Hearts));

        // Opener over 1NT–2♦–2♥–3♥: accept (4♥) on point_count + trump length ≥ 18,
        // else pass.  16 with a doubleton (16+2) accepts; a flat 15 with a doubleton
        // (15+2 = 17) passes; a 15 with three-card support (15+3) accepts.
        let over_invite = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Diamonds),
            P,
            bid(2, Strain::Hearts),
            P,
            bid(3, Strain::Hearts),
            P,
        ];
        assert_eq!(
            best(&over_invite, "AK5.32.AQ74.K963"),
            bid(4, Strain::Hearts)
        ); // 16, ♥xx
        assert_eq!(best(&over_invite, "AK5.32.AQ74.Q963"), P); // 15, ♥xx
        assert_eq!(
            best(&over_invite, "AK5.432.AQ7.Q963"),
            bid(4, Strain::Hearts)
        ); // 15, ♥xxx

        // Spade side: 6 spades, ♠KQ + ♥J = 6 HCP transfers (2♥) then jumps to 3♠.
        let spade_inv = "KQ8765.J43.32.32";
        assert_eq!(best(&one_nt, spade_inv), bid(2, Strain::Hearts));
        let after_spade = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Hearts),
            P,
            bid(2, Strain::Spades),
            P,
        ];
        assert_eq!(best(&after_spade, spade_inv), bid(3, Strain::Spades));

        set_sixcard_invite_floor(13); // restore the default (on)
    }

    /// Over a natural (2♣) overcall of our 1NT we play *systems on*, not
    /// Lebensohl: 2♣ steals no room, so responder keeps the uncontested Jacoby
    /// transfers, shows the stolen 2♣ Stayman with a Double, and opener answers in
    /// the uncontested tree (the systems-on rebase in `competition.rs`). There is
    /// no natural 2♦ escape — 2♦ is a transfer.
    #[test]
    fn systems_on_over_two_clubs() {
        use contract_bridge::auction::Auction;
        // The highest-logit *legal* call (what the real bidder picks; the bare
        // `best` helper ignores legality, so it can't drop the now-illegal 2♣).
        let best_legal = |auction: &[Call], hand: &str| -> Call {
            let hand = hand.parse().expect("valid test hand");
            let logits = american()
                .against(Family::NATURAL)
                .classify(hand, RelativeVulnerability::NONE, auction)
                .expect("a decision");
            let mut played = Auction::new();
            for &c in auction {
                played.push(c);
            }
            let mut scored: Vec<_> = (&logits.0)
                .into_iter()
                .filter(|(_, l)| l.is_finite())
                .collect();
            scored.sort_by(|x, y| y.1.partial_cmp(x.1).expect("no NaN"));
            scored
                .into_iter()
                .map(|(c, _)| c)
                .find(|&c| played.can_push(c).is_ok())
                .unwrap_or(Call::Pass)
        };

        let over_2c = [bid(1, Strain::Notrump), bid(2, Strain::Clubs)];
        // 5 hearts → 2♦ transfer; 5 spades → 2♥ transfer (systems on, not natural).
        assert_eq!(
            best_legal(&over_2c, "2.KJ876.5432.432"),
            bid(2, Strain::Diamonds)
        );
        assert_eq!(
            best_legal(&over_2c, "KJ876.2.5432.432"),
            bid(2, Strain::Hearts)
        );
        // 4-4 majors, invitational: the stolen 2♣ Stayman is shown by Double.
        assert_eq!(best_legal(&over_2c, "KJ32.KQ43.432.43"), Call::Double);

        // Opener completes the transfer: 1NT–(2♣)–2♦–(P) → 2♥, via the rebase.
        let over_xfer = [
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            bid(2, Strain::Diamonds),
            P,
        ];
        assert_eq!(
            best_legal(&over_xfer, "KQ3.A53.KQ54.K92"),
            bid(2, Strain::Hearts)
        );

        // Opener answers the stolen Stayman: 1NT–(2♣)–X–(P) → 2♥ with four hearts.
        let over_dbl = [
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Double,
            P,
        ];
        assert_eq!(
            best_legal(&over_dbl, "AQ3.KJ54.KQ4.92"),
            bid(2, Strain::Hearts)
        );
    }

    /// Opener converts the stolen-Stayman Double to penalty with good clubs, and
    /// *only* in the contested context — uncontested forcing Stayman never passes.
    #[test]
    fn penalty_pass_over_two_clubs() {
        use crate::bidding::american::set_penalty_pass;

        // 16 HCP, 5332 with AK-fifth of clubs (5 clubs, 7 club HCP), no 4-card major.
        let opener = "A2.K3.Q42.AK432";
        let over_dbl = [
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Double,
            P,
        ];
        let uncontested_stayman = [bid(1, Strain::Notrump), P, bid(2, Strain::Clubs), P];

        // With the penalty pass enabled, opener sits to defend 2♣ doubled.
        set_penalty_pass(Some((4, 4, true)));
        assert_eq!(best(&over_dbl, opener), Call::Pass);
        // Context-specific: the same hand still answers forcing Stayman (2♦) in the
        // *uncontested* auction — the conversion must not leak onto that shared node.
        assert_eq!(best(&uncontested_stayman, opener), bid(2, Strain::Diamonds));

        // With it off (the default), opener can never convert: answers Stayman 2♦.
        set_penalty_pass(None);
        assert_eq!(best(&over_dbl, opener), bid(2, Strain::Diamonds));
    }

    /// The gated invitational 5-4-majors structure, end to end: 5♠4♥ Staymans and
    /// rebids 2♠; 5♥4♠ transfers and rebids 2NT (with spades) or 2♠ (without).
    #[test]
    fn invitational_five_four_majors() {
        use crate::bidding::american::set_invitational_5card_majors;

        let one_nt = [bid(1, Strain::Notrump), P];
        // 5♠4♥, a bare 8 (♠KQ + ♥Q + ♦J).
        let s5h4 = "KQ864.Q1043.J2.32";
        // 6♠4♥, a bare 8 — a six-card major, so it blasts game via Texas (4♦), not
        // caught by the 5-4 Stayman reroute (which is scoped to five-card majors).
        let s6h4 = "KQ8642.QJ43.32.2";
        // 5♥4♠, a bare 8.
        let h5s4 = "Q1043.KQ864.J2.32";
        // 5 hearts, no four-card spade suit, a bare 8 (the single-suited invite).
        let h5 = "Q3.KQ864.J32.432";

        set_invitational_5card_majors(true);

        // Routing: 5♠4♥/8 now Staymans; 6♠4♥/8 blasts game via Texas (4♦, a six-card
        // major); 5♥4♠/8 still takes the heart transfer (2♦).
        assert_eq!(best(&one_nt, s5h4), bid(2, Strain::Clubs));
        assert_eq!(best(&one_nt, s6h4), bid(4, Strain::Diamonds));
        assert_eq!(best(&one_nt, h5s4), bid(2, Strain::Diamonds));

        // A: 1NT–2♣–2♦–2♠, non-forcing (opener denied a major).
        let stayman_no_major = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Clubs),
            P,
            bid(2, Strain::Diamonds),
            P,
        ];
        assert_eq!(best(&stayman_no_major, s5h4), bid(2, Strain::Spades));

        // B: 1NT–2♣–2♥–2♠, forcing (opener showed hearts); opener with a maximum and
        // three spades accepts in 4♠.
        let stayman_hearts = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Clubs),
            P,
            bid(2, Strain::Hearts),
            P,
        ];
        assert_eq!(best(&stayman_hearts, s5h4), bid(2, Strain::Spades));
        let over_two_s = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Clubs),
            P,
            bid(2, Strain::Hearts),
            P,
            bid(2, Strain::Spades),
            P,
        ];
        assert_eq!(
            best(&over_two_s, "AK4.KQ32.A65.J32"),
            bid(4, Strain::Spades)
        );

        // C/D: after the heart transfer completes, 5♥4♠ rebids 2NT; single-suited
        // five hearts rebids the artificial 2♠.
        let after_transfer = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Diamonds),
            P,
            bid(2, Strain::Hearts),
            P,
        ];
        assert_eq!(best(&after_transfer, h5s4), bid(2, Strain::Notrump));
        assert_eq!(best(&after_transfer, h5), bid(2, Strain::Spades));

        // D opener: a maximum with three hearts accepts the 5♥4♠ invite in 4♥.
        let over_two_nt = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Diamonds),
            P,
            bid(2, Strain::Hearts),
            P,
            bid(2, Strain::Notrump),
            P,
        ];
        assert_eq!(
            best(&over_two_nt, "AK2.A104.KQ32.J2"),
            bid(4, Strain::Hearts)
        );

        // Doubled-2♦ escape: when an opponent doubles opener's artificial 2♦, the
        // 5♠4♥ runs to its real 2♠ (systems on) instead of passing it out doubled.
        let two_d_doubled = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Clubs),
            P,
            bid(2, Strain::Diamonds),
            Call::Double,
        ];
        assert_eq!(best(&two_d_doubled, s5h4), bid(2, Strain::Spades));

        // With the structure off, the same 5♠4♥/8 takes the spade transfer instead.
        set_invitational_5card_majors(false);
        assert_eq!(best(&one_nt, s5h4), bid(2, Strain::Hearts));
        // The doubled-2♦ escape is general (competition-over-Stayman, not the flag):
        // a 4-4 invite runs to 2NT rather than passing the artificial 2♦ doubled.
        assert_eq!(
            best(&two_d_doubled, "KQ32.Q943.J32.43"),
            bid(2, Strain::Notrump)
        );
        set_invitational_5card_majors(true); // restore the default
    }

    /// The single-suited 5-spade invite: `1NT–2♥–2♠–2NT` (the spade mirror of the
    /// heart `2♠` relay — `2NT` is free here since 5♠4♥ Staymans), with opener's
    /// strength-and-fit placement (4♠ / 3NT / 3♠ / pass-2NT).
    #[test]
    fn single_suited_spade_invite() {
        // 5 spades, no four-card heart, a bare 8 (♠KQ + ♥Q + ♦J): single-suited invite.
        let s5 = "KQ864.Q3.J32.432";
        let one_nt = [bid(1, Strain::Notrump), P];

        // Transfers to spades (2♥), then rebids the 2NT invite over 2♠.
        assert_eq!(best(&one_nt, s5), bid(2, Strain::Hearts));
        let after_transfer = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Hearts),
            P,
            bid(2, Strain::Spades),
            P,
        ];
        assert_eq!(best(&after_transfer, s5), bid(2, Strain::Notrump));
        // A weak five-spade hand transfers and passes — it never invites with 2NT.
        assert_ne!(
            best(&after_transfer, "Q9864.32.J32.432"),
            bid(2, Strain::Notrump)
        );

        // Opener over 1NT–2♥–2♠–2NT, by strength and spade support:
        let over_invite = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Hearts),
            P,
            bid(2, Strain::Spades),
            P,
            bid(2, Strain::Notrump),
            P,
        ];
        // max (17) + three spades → 4♠; max + doubleton → 3NT.
        assert_eq!(
            best(&over_invite, "AK3.K32.KQ32.Q32"),
            bid(4, Strain::Spades)
        );
        assert_eq!(
            best(&over_invite, "KQ.AK42.KQ32.432"),
            bid(3, Strain::Notrump)
        );
        // min (16) + three spades → 3♠; min + doubleton → pass (rest in 2NT).
        assert_eq!(
            best(&over_invite, "AK3.Q32.KQ32.Q32"),
            bid(3, Strain::Spades)
        );
        assert_eq!(best(&over_invite, "KQ.Q432.KQ32.A32"), P);
    }

    /// Crawling Stayman: 4-4 majors *short in diamonds* (4414/4405) Stayman and,
    /// over opener's 2♦ denial, crawl to 2♥ — opener passes (heart fit), corrects
    /// to 2♠ (spade fit), or flees to 3♣ (no major fit, a 5-card-minor 1NT).
    #[test]
    fn crawling_stayman_escape() {
        use crate::bidding::american::set_crawling_stayman;

        let one_nt = [bid(1, Strain::Notrump), P];
        // 4414, a weak 5-count (♠QJ + ♥Q): garbage cannot escape it (one diamond).
        let h4414 = "QJ32.Q1043.4.T543";
        // 4405, a weak 5-count, void diamonds.
        let h4405 = "QJ32.Q1043..T9432";

        set_crawling_stayman(true);

        // Both short-diamond 4-4 hands bid 2♣ (crawling), unlike garbage Stayman.
        assert_eq!(best(&one_nt, h4414), bid(2, Strain::Clubs));
        assert_eq!(best(&one_nt, h4405), bid(2, Strain::Clubs));

        // Over opener's 2♦ denial, crawl to 2♥ (both majors, pass-or-correct).
        let two_d = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Clubs),
            P,
            bid(2, Strain::Diamonds),
            P,
        ];
        assert_eq!(best(&two_d, h4414), bid(2, Strain::Hearts));
        assert_eq!(best(&two_d, h4405), bid(2, Strain::Hearts));

        // Opener's reply to the crawl (1NT–2♣–2♦–2♥): three hearts pass the 4-3
        // fit; two hearts/three spades correct to 2♠; short in both majors with a
        // five-card minor flee to 3♣ (an 8-9 card club fit — responder is short
        // diamonds, hence long clubs).
        let crawl = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Clubs),
            P,
            bid(2, Strain::Diamonds),
            P,
            bid(2, Strain::Hearts),
            P,
        ];
        assert_eq!(best(&crawl, "A32.K43.KQ32.A52"), P); // 3-3 majors → pass 2♥
        assert_eq!(best(&crawl, "K43.A2.KQ32.A432"), bid(2, Strain::Spades)); // 3-2 → 2♠
        assert_eq!(best(&crawl, "K2.A2.KJ43.AJ432"), bid(3, Strain::Clubs)); // 2-2-4-5 → 3♣

        // Doubled tail (1NT–2♣–2♦–(X)–2♥) is systems-on via the competition rebase:
        // responder still crawls to 2♥, and opener still corrects (2♠ shown here).
        let two_d_doubled = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Clubs),
            P,
            bid(2, Strain::Diamonds),
            Call::Double,
        ];
        assert_eq!(best(&two_d_doubled, h4414), bid(2, Strain::Hearts));
        let crawl_doubled = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Clubs),
            P,
            bid(2, Strain::Diamonds),
            Call::Double,
            bid(2, Strain::Hearts),
            P,
        ];
        assert_eq!(
            best(&crawl_doubled, "K43.A2.KQ32.A432"),
            bid(2, Strain::Spades)
        );

        // With crawling off, the weak short-diamond 4-4 has no escape and passes.
        set_crawling_stayman(false);
        assert_eq!(best(&one_nt, h4414), P);
        set_crawling_stayman(true); // restore the default
    }

    #[test]
    fn both_majors_relay_game_placement() {
        // 1NT–2♣–2NT (max, both majors) –3♣ (responder names hearts) –3♥: responder
        // places game on `point_count + extra trumps + a fit in the other major`.
        let relay = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Clubs),
            P,
            bid(2, Strain::Notrump),
            P,
            bid(3, Strain::Clubs),
            P,
            bid(3, Strain::Hearts),
            P,
        ];

        // Double 4-4 fit: a flat 7 reaches game (7 + 0 + 1 = 8) — the second major
        // fit is knowable because opener showed both majors.
        assert_eq!(best(&relay, "KQ54.J932.654.J2"), bid(4, Strain::Hearts));
        // Single 8-card fit, 8 HCP: the pre-accepted invite bids game (8 + 0 + 0).
        assert_eq!(best(&relay, "K32.A654.J432.32"), bid(4, Strain::Hearts));
        // Single 9-card fit, a 5-3-3-2 6-count: 6 + 1 + 0 = 7 — passes. This is the
        // hand a bare `points(6..)` on the fifth trump overbid into game.
        assert_eq!(best(&relay, "Q32.KJ954.762.32"), P);
        // Single 8-card fit, only 7 HCP: 7 + 0 + 0 = 7 — passes the partscore.
        assert_eq!(best(&relay, "K32.QJ54.J432.32"), P);
    }

    #[test]
    fn stayman_fit_raise_by_value() {
        // 1NT–2♣–2♥ (opener's four-card major): responder raises on `fit_value`,
        // not raw HCP — any upgrade past a flat eight reaches game.
        let stayman = [
            bid(1, Strain::Notrump),
            P,
            bid(2, Strain::Clubs),
            P,
            bid(2, Strain::Hearts),
            P,
        ];

        // Flat 4-3-3-3 eight, four-card fit, no upgrade: invitational raise (value 8).
        assert_eq!(best(&stayman, "K32.Q654.K32.432"), bid(3, Strain::Hearts));
        // 4-4-4-1 eight with a working singleton: the shape upgrades to value 9, so
        // the same eight now bids game instead of merely inviting.
        assert_eq!(best(&stayman, "Q543.K654.K432.2"), bid(4, Strain::Hearts));
        // Flat 4-3-3-3 seven: value 7, below the invite — passes the partscore.
        assert_eq!(best(&stayman, "K32.Q654.Q32.432"), P);
    }
}
