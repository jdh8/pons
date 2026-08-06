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

use super::{call, other_major, slam};
use crate::bidding::constraint::{
    Cons, Constraint, balanced, described, envelope_union_upgrade, equal_length, hcp, len,
    long_suit_box, longer_suit, point_count, points, pred, reads_as, stopper_in,
    support_point_count_in, support_points, top_honors,
};
use crate::bidding::inference::{EnvelopeUnion, Range};
use crate::bidding::instinct::net_break_even_gate;
use crate::bidding::rows::{Package, Pattern, compile_into, expand, rows_of};
use crate::bidding::{Alert, Context, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Holding, Rank, Strain, Suit};
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
/// Thread-local, read at book-construction time (the
/// [`set_notrump_defense`][super::set_notrump_defense]-style knob).
/// Pass [`PUPPET`] (default) or [`EUROPEAN`]; both variants are authored, and only
/// the selected one's `2♠`/`2NT`/`3♣` rules are gated into the trie.
///
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

/// Whether book construction uses the Puppet minor scheme
///
/// This is a function because declarative [`Package`] gates are bare function
/// pointers and cannot capture a local from [`register_one_nt`].
fn puppet_scheme() -> bool {
    notrump_minors() == PUPPET
}

/// The anti-gate of [`puppet_scheme`], for the European packages
fn european_scheme() -> bool {
    !puppet_scheme()
}

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
fn size_ask_eight() -> SizeAskEight {
    SIZE_ASK_EIGHT.with(Cell::get)
}

/// The size-ask eight class: a balanced eight with no four-card major — the
/// population the `2♠`/`2NT` size ask and the flat-4333 Pass carve both key on.
fn size_ask_eight_class() -> Cons<impl Constraint + Clone> {
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
fn size_ask_eight_pass() -> Rules {
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
fn size_ask_accept_floor() -> u8 {
    SIZE_ASK_ACCEPT_FLOOR.with(Cell::get)
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
    // Direct `4♥/4♠` is the opener-decides slam try; with the Texas slam-drive
    // reroute on it caps at the 15–16 invitational band (17+ Texas-transfers and
    // drives its own RKCB instead — see [`set_texas_slam_drive`]).
    let direct_4m_max: u8 = if texas_slam_drive() { 15 } else { 18 };
    // Jacoby transfers — any strength, except a game-forcing 5-4 in the majors
    // (its weak-only arm denies it): that hand keeps off the transfer and takes
    // the 2♣ Stayman/Smolen route, which right-sides game to the strong notrump.
    // A plain 5-3 still transfers.  Under the longer-major discipline (default;
    // see [`set_transfer_longer_major`]) a two-suiter (both majors 5+) always
    // transfers to the LONGER major, and equal lengths split by strength: weak
    // → hearts (safety), invitational / minimum game force → the both-majors
    // 3♦, slam try → spades (the `1NT–2♥–2♠–3♥` structure).  2♦ (to hearts) is
    // UNCHANGED by the invitational-5-4 reroute — a 5♥4♠ invite keeps
    // transferring and shows the spades with a later 2NT/2♠.
    let prefer_longer = transfer_longer_major();
    let head = if prefer_longer {
        Rules::new().rule(
            Bid::new(2, Strain::Diamonds),
            200,
            len(Suit::Hearts, 5..)
                & (len(Suit::Spades, ..4)
                    | (len(Suit::Spades, 4..=4) & hcp(..9))
                    | (len(Suit::Spades, 5..) & longer_major(Suit::Hearts, Suit::Spades))
                    | (equal_majors() & points(..8))
                    | major_splinter_reroute(Suit::Hearts)),
        )
    } else {
        Rules::new().rule(
            Bid::new(2, Strain::Diamonds),
            200,
            len(Suit::Hearts, 5..)
                & (len(Suit::Spades, ..4) | hcp(..9) | major_splinter_reroute(Suit::Hearts)),
        )
    }
    .alert(JACOBY);
    // 2♥ (to spades): the invitational-5-4 reroute (gated) keeps a 5♠4♥ hand of
    // invitational+ values OFF the transfer so it Staymans; a six-card spade suit
    // (`len(♠,6..)`) and a weaker 5♠4♥ (`hcp(..8)`) still transfer.  Off the flag,
    // the classic any-strength-but-GF-5-4 gate.
    let head = match (prefer_longer, invitational_5card_majors()) {
        (true, true) => head.rule(
            Bid::new(2, Strain::Hearts),
            200,
            len(Suit::Spades, 5..)
                & (len(Suit::Hearts, ..4)
                    | (len(Suit::Hearts, 4..=4) & (hcp(..8) | len(Suit::Spades, 6..)))
                    | (len(Suit::Hearts, 5..) & longer_major(Suit::Spades, Suit::Hearts))
                    | (equal_majors() & slam_55_reroute())),
        ),
        (true, false) => head.rule(
            Bid::new(2, Strain::Hearts),
            200,
            len(Suit::Spades, 5..)
                & (len(Suit::Hearts, ..4)
                    | (len(Suit::Hearts, 4..=4) & hcp(..9))
                    | (len(Suit::Hearts, 5..) & longer_major(Suit::Spades, Suit::Hearts))
                    | (equal_majors() & slam_55_reroute())
                    | major_splinter_reroute(Suit::Spades)),
        ),
        (false, true) => head.rule(
            Bid::new(2, Strain::Hearts),
            200,
            len(Suit::Spades, 5..)
                & (len(Suit::Hearts, ..4) | hcp(..8) | len(Suit::Spades, 6..) | slam_55_reroute()),
        ),
        (false, false) => head.rule(
            Bid::new(2, Strain::Hearts),
            200,
            len(Suit::Spades, 5..)
                & (len(Suit::Hearts, ..4)
                    | hcp(..9)
                    | slam_55_reroute()
                    | major_splinter_reroute(Suit::Spades)),
        ),
    }
    .alert(JACOBY);
    head
        // Both-majors 3♦: 5+/5+ in the majors, invitational+.  Outranks the
        // transfers (2.0) so a 5-5 INV+ hand shows both suits in one bid rather
        // than transferring and rebidding; weaker 5-5s (below the `points` floor)
        // still take the transfer route.  `points` (not `hcp`) so the 5-5 shape
        // upgrade counts — these are the unbalanced hands the gauge was built for.
        // Under the longer-major discipline the bid is *equal lengths only*: a
        // 6-5 hand names its longer suit first via the transfer instead.
        .rule(
            Bid::new(3, Strain::Diamonds),
            210,
            len(Suit::Hearts, 5..)
                & len(Suit::Spades, 5..)
                & points(8..)
                & described(
                    "equal lengths only under the longer-major discipline",
                    move |hand: Hand, _: &Context<'_>| {
                        !prefer_longer || hand[Suit::Hearts].len() == hand[Suit::Spades].len()
                    },
                )
                & described(
                    "both-majors 3♦ capped at minimum game force when the slam reroute is on",
                    |hand: Hand, _: &Context<'_>| {
                        !transfer_gf_majors() || usize::from(point_count(hand)) <= 16
                    },
                ),
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
            250,
            len(Suit::Hearts, 6..)
                & len(Suit::Spades, ..5)
                & texas_strength_gate(Suit::Hearts)
                & not_major_splinter_slam(Suit::Hearts),
        )
        .alert(TEXAS)
        .rule(
            Bid::new(4, Strain::Diamonds),
            250,
            len(Suit::Spades, 6..)
                & len(Suit::Hearts, ..5)
                & texas_strength_gate(Suit::Spades)
                & not_major_splinter_slam(Suit::Spades),
        )
        .alert(TEXAS)
        .rule(
            Bid::new(4, Strain::Hearts),
            260,
            len(Suit::Hearts, 6..)
                & len(Suit::Spades, ..5)
                & hcp(15..=direct_4m_max)
                & not_major_splinter_slam(Suit::Hearts),
        )
        .alert(TEXAS)
        .rule(
            Bid::new(4, Strain::Spades),
            260,
            len(Suit::Spades, 6..)
                & len(Suit::Hearts, ..5)
                & hcp(15..=direct_4m_max)
                & not_major_splinter_slam(Suit::Spades),
        )
        .alert(TEXAS)
        // Stayman: a four-card major and at least invitational values — but never
        // on a flat 4-3-3-3, which plays better in 3NT than in the 4-4 major fit
        // (no ruffing value), so it invites/forces in notrump directly.
        .rule(
            Bid::new(2, Strain::Clubs),
            150,
            (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4)) & hcp(8..) & !flat_4333(),
        )
        .alert(STAYMAN)
        // Quantitative 4NT slam invite (balanced, no four-card major).
        .rule(
            Bid::new(4, Strain::Notrump),
            120,
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
        //
        // Nor can the evaluator net upgrade good sub-9s into the force: rank-
        // calibrated against raw HCP at this seam it scores ≈0, now on all three
        // net generations (v2 2026-07-22; v3 and v4 re-screened 2026-07-28 —
        // eight cells over both vuls and both opener bands, largest |mean| 0.0088
        // IMPs/board, no CI excluding 0).  The null is *structural*, not a stale
        // measurement waiting on a better net: `features_eval_v3`/`v4` extend
        // `features_eval` with the calls tail and partner's shape reading, both of
        // which are **constant** across this class, so every net generation ranks
        // these hands on the same 24-float own-hand honour block.  A responder who
        // can neither Stayman nor transfer, opposite a known balanced 15-17, has
        // only honour texture left — and at fixed HCP that is worth nothing.  The
        // same screen's Stayman class is the positive control: there the net wins,
        // and wins *more* with each version (+0.052 → +0.055 → +0.058 NV).
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            hcp(9..) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        // The source-of-tricks *eight* (opt-in, OFF by default — measured a loss):
        // a running long minor would force 3NT (weight 1.4) rather than transfer,
        // but the transfer reaches the better game.  See `long_minor_force_rule`.
        .chain(long_minor_force_rule())
        // Pass 0-7, and also the flat 4-3-3-3 *eight*: a shape with no ruff and no
        // long suit is its high cards and nothing more, so it plays a level too high
        // opposite a 15-17.  A double-dummy probe (`examples/probe-uninvite-4333`,
        // 16M deals) prices passing over the `2♠` size-ask invite at +0.64 IMPs/board
        // for the whole class, rising to +1.08 for the pure-quack (no ace, no ten)
        // eight — even the ace-holding eights gain.  The *nine* still forces (3NT):
        // the same probe found blanket-inviting it loses −0.33.  The size-ask eight's
        // pass/invite split is knob-gated for re-measurement — see `size_ask_eight`.
        .chain(size_ask_eight_pass())
        // Splinter 3♥/3♠ (opt-in): shortness in the bid major, 2-3 in the other,
        // exactly four diamonds and 5-6 clubs.  See `nt_splinter_rules`.
        .chain(nt_splinter_rules())
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
            150,
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
            150,
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
            150,
            len(Suit::Hearts, 4..=4)
                & len(Suit::Spades, 4..=4)
                & len(Suit::Diamonds, ..=1)
                & hcp(..8),
        )
        .alert(STAYMAN)
}

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
fn long_minor_force_rule() -> Rules {
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

/// Responder's `3♥`/`3♠` splinter over our 1NT — shortness in the *bid* major
///
/// The Bridge World Standard / Polish Club treatment of the only two slots our
/// response ladder leaves empty.  BWS 2017 IV.F(e) words it "three of a major =
/// at most one card in the suit bid", and the BWS 2001 poll gave "both minors
/// with shortness in the bid suit" the expert plurality (37% panel, 29% readers,
/// against 21/25 for natural-and-forcing).
///
/// The shape is pinned rather than merely floored, because every neighbouring
/// slot already owns its half of the residue:
///
/// | axis | value | why not elsewhere |
/// | --- | --- | --- |
/// | bid major | void or low singleton | a stiff A/K un-wastes opener's honors |
/// | other major | 2–3 | four is Stayman's, five transfers |
/// | diamonds | exactly 4 | `♦5+` is the `2NT` transfer |
/// | clubs | 5–6 | `♣7+` keeps the `2♠` transfer |
///
/// so `♣ = 9 − (both majors)` closes the shape at `3-1-4-5`, `1-2-4-6` and
/// `0-3-4-6` (and mirrors).  The `♣6` rows *do* qualify for the `2♠` club
/// transfer, and the rule outranks it deliberately: after `2♠` responder can
/// never show the four diamonds, which is the whole 6-4 slam lane.
///
/// This is a **splinter, not a fragment** — responder names the short major, not
/// the three-card one.  Fragments draw a lead-directing double less often, but
/// the double hurts far more when it comes (the lead runs through responder's
/// real three cards with the doubler's length sitting behind them), whereas an
/// LDX of a splinter directs into a void or singleton and warns us for free.
/// Double dummy is blind to both effects, so the choice is settled on theory.
///
/// BBA/EPBot's `1N-3M splinter` toggle is a *different* convention from the same
/// family — the GIB form, "singleton or void in the suit bid, at least 4 cards
/// in the other 3 suits, no 5-card major", which pins the other major at exactly
/// four and is therefore the hand Stayman already finds.  See
/// `docs/ai-bidder/bba-1nt-splinter.md` for the measured read of it.
///
/// On by default since 2026-07-28, its A/B (`examples/ab-nt-splinter`) having
/// won in all four cells; see [`set_nt_splinter`] for the measured numbers.
fn nt_splinter_rules() -> Rules {
    if !nt_splinter() {
        return Rules::new();
    }
    let floor = nt_splinter_floor();
    let mut rules = Rules::new();
    for (short, other) in [(Suit::Hearts, Suit::Spades), (Suit::Spades, Suit::Hearts)] {
        rules = rules
            .rule(
                Bid::new(3, Strain::from(short)),
                // Above the `2♠`/`2NT` minor transfers (130) so the `♣6` rows
                // route here.  Nothing else can match: Stayman (150) wants a
                // four-card major, Puppet `3♣` (160) wants `balanced()`, the
                // both-majors `3♦` (210) wants 5-5 majors, and the Jacoby
                // transfers (200) want a five-card major.
                170,
                splinter_short(short)
                    & len(other, 2..=3)
                    & len(Suit::Diamonds, 4..=4)
                    & len(Suit::Clubs, 5..=6)
                    & hcp(floor..),
            )
            .alert(SPLINTER);
    }
    rules
}

/// Opener's answer to the `1NT–3M` splinter — place the game
///
/// Authored because the floor **measured inert here**: given the whole pinned
/// shape it bid `3NT` on every hand, including a total-wastage `♥KQJ` and five
/// spades opposite a known three-card holding.  A 600 000-board self-play A/B
/// found 217 firings and 9 divergent boards (−4 IMPs plain, −22 PD) — responder
/// described perfectly and opener ignored every word of it.  Deleting the node
/// hands the position back to the floor, which is exactly where it does nothing.
///
/// The judgement the convention exists for is **3NT versus 5m**, and the pivot
/// is a stopper in the short major.  Responder holds 0–1 there and opener 2–4,
/// so the opponents own eight to eleven cards in it: without a guard they cash
/// out before `3NT` runs nine tricks, while the minor game has a known 8–9 card
/// fit, 24+ combined points and a ruffing value.  With a guard, `3NT` is nine
/// tricks against eleven and wins the level.
///
/// Opener places the *game* rather than probing at the four level, because the
/// probe has no safe landing: `4♣`/`4♦` is below game in a game-forcing auction,
/// so a floor-generated pass would strand the partnership in a partscore.  The
/// 4-4 major fit is deliberately not hunted — responder holds 2–3 in the other
/// major and opener at most five, so the best case is a 5-3 that `3NT` or the
/// minor game usually beats, and hunting it costs the `3♥`/`3♠` symmetry (over
/// `3♠` there is no room below `3NT` at all).  Slam stays with the floor, which
/// reads the shape and keeps its keycard machinery over the minor game.
fn nt_splinter_answer(short: Suit) -> Rules {
    Rules::new()
        // A guard in the short major: `3NT` is the nine-trick game.
        .rule(Bid::new(3, Strain::Notrump), 100, stopper_in(short))
        // No guard.  Prefer the longer fit: responder's 5-6 clubs opposite three
        // is a nine-card trump suit, and only when opener is short in clubs does
        // the 4-4 diamond fit win.
        .rule(
            Bid::new(5, Strain::Clubs),
            120,
            !stopper_in(short) & len(Suit::Clubs, 3..),
        )
        .rule(
            Bid::new(5, Strain::Diamonds),
            120,
            !stopper_in(short) & len(Suit::Clubs, ..3) & len(Suit::Diamonds, 4..),
        )
        // Finite catch-all: no guard, no fit (a 4-4 major hand with a doubleton
        // club and three diamonds) — take the nine-trick game anyway.
        .rule(Bid::new(3, Strain::Notrump), 50, hcp(0..))
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
    // 2♠ = six-card clubs, plus the bare-8 balanced size ask (no four-card major),
    // gated on `size_ask_eight`: `Shipped` excludes the flat 4-3-3-3 (it passes),
    // `Invite` size-asks the whole class, `Pass` drops the invite (clubs only).
    let two_spades = match size_ask_eight() {
        SizeAskEight::Shipped => Rules::new().rule(
            Bid::new(2, Strain::Spades),
            130,
            len(Suit::Clubs, 6..)
                | (hcp(8..=8)
                    & balanced()
                    & len(Suit::Hearts, ..4)
                    & len(Suit::Spades, ..4)
                    & !flat_4333()),
        ),
        SizeAskEight::Invite => Rules::new().rule(
            Bid::new(2, Strain::Spades),
            130,
            len(Suit::Clubs, 6..) | size_ask_eight_class(),
        ),
        SizeAskEight::Pass => {
            Rules::new().rule(Bid::new(2, Strain::Spades), 130, len(Suit::Clubs, 6..))
        }
    };
    two_spades
        .alert(PUPPET)
        .rule(
            Bid::new(2, Strain::Notrump),
            130,
            len(Suit::Diamonds, 6..) | (len(Suit::Diamonds, 5..) & len(Suit::Clubs, 4..)),
        )
        .alert(PUPPET)
        .rule(
            Bid::new(3, Strain::Clubs),
            160,
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
    // 2NT = the bare-8 size ask (no four-card major), gated on `size_ask_eight`:
    // `Shipped` excludes the flat 4-3-3-3 (it passes), `Invite` size-asks the whole
    // class, `Pass` drops the 2NT size ask entirely.
    let size_ask = match size_ask_eight() {
        SizeAskEight::Shipped => Rules::new()
            .rule(
                Bid::new(2, Strain::Notrump),
                130,
                hcp(8..=8)
                    & balanced()
                    & len(Suit::Hearts, ..4)
                    & len(Suit::Spades, ..4)
                    & !flat_4333(),
            )
            .alert(EUROPEAN),
        SizeAskEight::Invite => Rules::new()
            .rule(Bid::new(2, Strain::Notrump), 130, size_ask_eight_class())
            .alert(EUROPEAN),
        SizeAskEight::Pass => Rules::new(),
    };
    Rules::new()
        .rule(Bid::new(2, Strain::Spades), 130, len(Suit::Clubs, 6..))
        .alert(EUROPEAN)
        .chain(size_ask)
        .rule(
            Bid::new(3, Strain::Clubs),
            130,
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
        .rule(Bid::new(2, Strain::Hearts), 100, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(2, Strain::Spades),
            100,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            50,
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
            .rule(Bid::new(2, Strain::Notrump), 110, both & hcp(16..))
            .alert(BOTH_MAJORS);
    }
    if stayman_5card_max() {
        // Five-card major, maximum (16-17): jump.  Natural (names and shows its
        // own suit), so unalerted — alerting would make alert-reading suppress it.
        rules = rules
            .rule(
                Bid::new(3, Strain::Hearts),
                110,
                len(Suit::Hearts, 5..) & hcp(16..),
            )
            .rule(
                Bid::new(3, Strain::Spades),
                110,
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
            100,
            described("spades > hearts", |hand: Hand, _: &Context<'_>| {
                hand[Suit::Spades].len() > hand[Suit::Hearts].len()
            }),
        )
        .alert(BOTH_MAJORS)
        .rule(
            Bid::new(3, Strain::Clubs),
            100,
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
        .rule(Bid::new(3, Strain::from(major)), 100, hcp(0..))
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
        130,
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
    // Fit-known (the major is agreed), so count shortness as support value —
    // in the side suits only, never in trump itself; the
    // length-beyond-the-eighth term is explicit.
    let support = usize::from(support_point_count_in(hand, major));
    support + hand[major].len().saturating_sub(4)
}

/// Responder's placement over opener's max five-card-major jump (`3♥`/`3♠`)
///
/// With three-card support (an eight-card fit) opposite a maximum, bid game; else
/// sign off in `3NT`.
fn five_card_max_rebid(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 130, len(major, 3..))
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
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
pub(crate) fn transfer_super_accept() -> bool {
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
/// the major game (`4M`), and the `slam` 1430 ladder places the slam.  Mirrors
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
    /// Route slam-driving six-card-major hands through Texas + responder RKCB
    /// instead of the opener-decides direct `1NT–4♥/4♠`; **on by default**.
    /// See [`set_texas_slam_drive`].
    static TEXAS_SLAM_DRIVE: Cell<bool> = const { Cell::new(true) };
}

/// Route slam-driving six-card-major hands through a Texas transfer + responder
/// RKCB for books built *after* this call (thread-local; **on by default**).
///
/// The direct `1NT–4♥/4♠` is a *non-forcing* slam try — opener moves only with a
/// maximum, else passes the major game.  That strands the strong responder: a
/// 16+ six-card-major hand opposite a *minimum* 1NT (the majority) has a cold slam
/// the opener vetoes by passing.  When on, the direct `4♥/4♠` is capped at the bare
/// 15 invitational cusp (opener-decides is right there), and a 16+ hand instead
/// Texas-transfers (`4♣/4♦`) and, over opener's completion, drives its own RKCB
/// (`4NT`) — reaching the slam regardless of opener's minimum, exactly as the
/// reference bidder does.  A paired on/off A/B (320k boards, shared seed, vs the
/// BBA reference) measured **plain +0.0024 IMPs/board (95% CI ±0.0006), PD +0.0024
/// — +5.87 IMPs/fired in both regimes** (131 fired, 0.04%), every CI excluding 0.
pub fn set_texas_slam_drive(on: bool) {
    TEXAS_SLAM_DRIVE.with(|cell| cell.set(on));
}

/// Whether the Texas slam-drive reroute is currently authored
fn texas_slam_drive() -> bool {
    TEXAS_SLAM_DRIVE.with(Cell::get)
}

thread_local! {
    /// Responder's game-forcing structure after the spade transfer completes
    /// (`1NT–2♥–2♠`): the natural `3♥` 5-5 slam try, minor side-suits (`3♣`/`3♦`),
    /// and the single-suiter's quantitative `4NT`; **on by default** (A/B vs BBA:
    /// plain +0.0014, PD +0.0016 IMPs/board, both CI ±0.0003, +1.70/+1.90 per fired).
    /// See [`set_transfer_gf_majors`].
    static TRANSFER_GF_MAJORS: Cell<bool> = const { Cell::new(true) };
    /// Within the GF structure, route a minimum five-card-spade game-force holding a
    /// four-card minor into the choice-of-games `3NT` (the floor) rather than showing
    /// the minor, so `3♣`/`3♦` are reserved for slam tries; **off by default**.  See
    /// [`set_minor_min_to_3nt`].
    static MINOR_MIN_TO_3NT: Cell<bool> = const { Cell::new(false) };
    /// Mirror the GF structure onto the *heart* transfer (`1NT–2♦–2♥`): minor
    /// side-suits (`3♣`/`3♦`), the `3♠` spade splinter (plus `4♣`/`4♦`), and the
    /// quantitative `4NT` — the single-suited slam try relocating off `3♠`, just as
    /// spades relocated off `3♥`.  The 5-5 slam try needs no heart slot (it rides the
    /// spade transfer).  No-op unless [`set_transfer_gf_majors`] is also on; **on by
    /// default** (A/B vs BBA, two seeds: plain +0.0015/+0.0017, PD +0.0016/+0.0018
    /// IMPs/board, all CI ±0.0003, +1.83/+2.08 per fired).  See [`set_transfer_gf_hearts`].
    static TRANSFER_GF_HEARTS: Cell<bool> = const { Cell::new(true) };
}

/// Author responder's game-forcing structure after the spade transfer for books
/// built *after* this call (thread-local; **on by default**).
///
/// After `1NT–2♥–2♠`, responder's game-forcing hands otherwise fall to the floor's
/// natural raise.  When on: a natural `3♥` shows 5-5 majors with slam interest
/// (rerouted off the capped both-majors `3♦`), `3♣`/`3♦` show a five-spade hand with
/// a four-card minor, and `4NT` is the single-suiter's quantitative slam invite
/// (relocated off the repurposed `3♥`).  Pass `false` to fall back to the floor.
pub fn set_transfer_gf_majors(on: bool) {
    TRANSFER_GF_MAJORS.with(|cell| cell.set(on));
}

/// Whether the post-transfer game-forcing structure is currently authored
fn transfer_gf_majors() -> bool {
    TRANSFER_GF_MAJORS.with(Cell::get)
}

/// Route minimum five-card-spade game-forces with a four-card minor into the
/// choice-of-games `3NT` instead of showing the minor (thread-local; **off by
/// default**; the E1 A/B arm).  No-op unless [`set_transfer_gf_majors`] is on.
pub fn set_minor_min_to_3nt(on: bool) {
    MINOR_MIN_TO_3NT.with(|cell| cell.set(on));
}

/// Whether minimum five-card-spade game-forces with a minor bid `3NT` (Arm B)
fn minor_min_to_3nt() -> bool {
    MINOR_MIN_TO_3NT.with(Cell::get)
}

/// Mirror the post-transfer game-forcing structure onto the heart transfer for books
/// built *after* this call (thread-local; **on by default**).
///
/// After `1NT–2♦–2♥`, responder shows a five-heart-plus-minor game force (`3♣`/`3♦`),
/// a six-heart splinter (`3♠` short in spades, `4♣`/`4♦` short in a minor), or a
/// single-suited quantitative slam invite (`4NT`, relocated off the `3♠` slam try).
/// The 5-5 majors slam try keeps its single home on the spade transfer.  No effect
/// unless [`set_transfer_gf_majors`] is also on.
pub fn set_transfer_gf_hearts(on: bool) {
    TRANSFER_GF_HEARTS.with(|cell| cell.set(on));
}

/// Whether the heart-transfer mirror is authored (requires the master gate too)
fn transfer_gf_hearts() -> bool {
    transfer_gf_majors() && TRANSFER_GF_HEARTS.with(Cell::get)
}

/// Responder's RKCB drive over opener's Texas completion (`1NT–4♣–4♥–4NT` /
/// `1NT–4♦–4♠–4NT`)
///
/// A 17+ six-card-major hand transferred at the four level and now keycards: `4NT`
/// is RKCB, the [`slam`] 1430 ladder (installed alongside) places the slam.  Weaker
/// (game-only) transfers match no rule and pass opener's `4M`.  Empty unless the
/// reroute is on ([`set_texas_slam_drive`]).
fn texas_slam_drive_rebid() -> Rules {
    if !texas_slam_drive() {
        return Rules::new();
    }
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 140, hcp(16..))
        .alert(slam::RKCB)
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
    /// The Jacoby transfer names the **longer** major, and equal-length
    /// two-suiters split by strength: weak prefers the heart transfer (safety),
    /// invitational and minimum game force show both at once via the
    /// both-majors 3♦, and slam tries prefer the spade transfer for the
    /// `1NT–2♥–2♠–3♥` structure.  **On by default**; off restores the legacy
    /// guards (a 6♠5♥ hand could tie into the heart transfer, and 3♦ fired on
    /// any 5-5+).  See [`set_transfer_longer_major`].
    static TRANSFER_LONGER_MAJOR: Cell<bool> = const { Cell::new(true) };
    /// Responder's continuation after opener cue-bids in cooperation with the `3OM`
    /// slam try (`1NT-2♣-2M-3OM-4x`).  Opener's [`stayman_slam_try_answer`] cues a
    /// control below the trump major with a maximum; without a responder node the
    /// floor *passed the cue* — often below game.  On, responder keycards (`4NT`
    /// RKCB) with slam values or signs off in the major game.  **On by default** —
    /// the cue dead-end was the dominant Stayman leak vs BBA (≈20% of the tail-loss
    /// IMPs, `bba-gen --isolate-opening bba`).  See [`set_stayman_cue_continuation`].
    static STAYMAN_CUE_CONTINUATION: Cell<bool> = const { Cell::new(true) };
    /// Stayman-then-minor slam try: over opener's Stayman answer, responder's
    /// jump-free `3♣`/`3♦` shows a *natural* 5+ minor with slam values (14+) and no
    /// fit for opener's major — the 5-4 two-suiter whose four-card major (the reason
    /// for the 2♣ detour) missed.  Opener cooperates by raising the minor with a
    /// fit + maximum (else `3NT`), and responder keycards.  **On by default** —
    /// the A/B landed +3.29/+4.02 IMPs/fired (none/both, plain DD; PD identical,
    /// no doubling artifact) across 151 fired boards, zero losses.  See
    /// [`set_stayman_minor_slam_try`].
    static STAYMAN_MINOR_SLAM_TRY: Cell<bool> = const { Cell::new(true) };
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

    /// Whether responder's `3♥`/`3♠` splinter over 1NT is authored, read once at
    /// book-construction time (and by the inference engine, to decode it).
    /// **On by default** since 2026-07-28: 5M boards per vulnerability, win in
    /// all four cells (+0.56/+0.67 IMPs per fired board at none, +0.69/+0.81 at
    /// both; plain/PD).  See [`set_nt_splinter`].
    static NT_SPLINTER: Cell<bool> = const { Cell::new(true) };

    /// Responder's HCP floor for the `3♥`/`3♠` splinter, read once at
    /// book-construction time.  `9` by default — BBA's measured floor for the
    /// same slot, and BWS's "strong".  See [`set_nt_splinter_floor`].
    static NT_SPLINTER_FLOOR: Cell<u8> = const { Cell::new(9) };
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

/// Author responder's `3♥`/`3♠` splinter over 1NT for books built *after* this
/// call (thread-local; **on by default** — it won its A/B in all four cells)
///
/// Shortness in the *bid* major, 2–3 in the other, exactly four diamonds and
/// five or six clubs — the Bridge World Standard / Polish Club treatment of the
/// two slots our ladder leaves empty.  The hand class it serves (`3-1-4-5` and
/// mirrors) has no home at all today: too few majors for Stayman, too few
/// diamonds for the `2NT` transfer, too few clubs for the `2♠` transfer, and not
/// `balanced()` for Puppet `3♣`, so at 9+ HCP it blasts 3NT and at 8 it *passes*
/// 1NT holding a singleton opposite 15-17.
///
/// Since `♣ = 9 − (both majors)`, the shape is *closed*: `3-1-4-5`, `1-2-4-6`
/// and `0-3-4-6` (and mirrors).  The `♣6` rows also qualify for the `2♠` club
/// transfer and this outranks it deliberately — after `2♠` responder can never
/// show the four diamonds, which is the whole 6-4 slam lane.
///
/// On, the inference engine also stops reading a three-level major over our 1NT
/// as a natural five-card suit and decodes it off the alert instead, and opener
/// answers from an authored table (`nt_splinter_answer`) rather than the floor,
/// which measured inert here — given the whole pinned shape it bid `3NT` on
/// every hand.  Full rationale and the splinter-versus-fragment argument are on
/// the private `nt_splinter_rules`.  Measured by `examples/ab-nt-splinter`.
pub fn set_nt_splinter(on: bool) {
    NT_SPLINTER.with(|cell| cell.set(on));
}

/// Set responder's HCP floor for the `3♥`/`3♠` splinter for books built *after*
/// this call (thread-local; **default 9**)
///
/// Exists so the 8-versus-9 sweep is a flag rather than a rebuild.  The default
/// matches both BBA's measured floor for the same slot and BWS's "strong"; the
/// eight is the marginal case, since a `3-1-4-5` eight currently *passes* 1NT.
pub fn set_nt_splinter_floor(floor: u8) {
    NT_SPLINTER_FLOOR.with(|cell| cell.set(floor));
}

/// Whether responder's `3♥`/`3♠` splinter over 1NT is currently authored (read
/// by the inference engine too, to decode the call off its alert)
pub(crate) fn nt_splinter() -> bool {
    NT_SPLINTER.with(Cell::get)
}

/// Responder's current HCP floor for the `3♥`/`3♠` splinter
fn nt_splinter_floor() -> u8 {
    NT_SPLINTER_FLOOR.with(Cell::get)
}

/// Whether opener's both-majors max-only relay is currently authored
fn stayman_both_majors() -> bool {
    STAYMAN_BOTH_MAJORS.with(Cell::get)
}

/// Whether opener's max five-card-major jump is currently authored
fn stayman_5card_max() -> bool {
    STAYMAN_5CARD_MAX.with(Cell::get)
}

thread_local! {
    /// See [`set_stayman_net_force`].
    static STAYMAN_NET_FORCE: Cell<bool> = const { Cell::new(false) };
}

/// Price responder's Stayman-rebid invite/force seams with the evaluator net
/// instead of the point tests (thread-local; **off by default — measured a
/// loss**, kept for re-measurement)
///
/// The `probe-nt-invite-eval` screen (30 000 deals per class, seed 1784718391)
/// found the net's game make-probability is the first evaluator to out-rank
/// raw HCP at the 1NT invite/force boundary — but only on the Stayman class
/// (+0.030 ±0.017 IMPs/board vul none, +0.044 ±0.025 vul both, rising to
/// +0.048/+0.069 opposite exactly-15 openers); the balanced no-major seam
/// stays HCP (net ≈ 0, third evaluator family to fail there).  This knob
/// converts exactly the Stayman-rebid seams: with a fit the `4M`/`3M`/`3OM`
/// split, without one the `3NT`/`2NT` revert — each force arm becomes "the
/// net clears the game's IMP break-even at the live vulnerability", its
/// invite twin the declined half.  The 2♣ entry, Smolen, garbage/crawling
/// and the quantitative 4NT are untouched.
///
/// **The live A/B refuted it** (`ab-stayman-net-force --slice`, 200k sliced
/// boards per vul, seed 1784719896): vul none −0.022 plain DD / +0.003 PD,
/// vul both −0.021 plain / −0.027 PD.  The forensic split explains the
/// screen-vs-live reversal: at the *fit* seam the incumbent is not raw HCP
/// but [`fit_value`] — already an upgrade evaluator, and the net loses to it
/// on both scorers in both directions; at the *no-fit NT* seam the net's
/// flips are plain-DD-positive (matching the screen, which scored plain DD)
/// but PD-negative — the DD-trained net promotes 3NTs that die against
/// perfect defense, the decision table's "doubling artifact, don't ship" row.
/// A frequency-matched NT-seam-only gate re-scored under `single_dummy_leads`
/// is the remaining open refinement.
///
/// Unlike its construction-time neighbours, this is read at **classification
/// time** (like [`set_bilans_floor`][crate::bidding::instinct::set_bilans_floor]):
/// flip it on threads that classify through a [`Stance`][crate::bidding::Stance],
/// no book rebuild needed.
#[doc(hidden)]
pub fn set_stayman_net_force(on: bool) {
    STAYMAN_NET_FORCE.with(|cell| cell.set(on));
}

/// Plain-bool read of the Stayman net-force knob (see [`set_stayman_net_force`])
fn stayman_net_force() -> bool {
    STAYMAN_NET_FORCE.with(Cell::get)
}

/// One side of a Stayman-rebid invite/force seam: knob-off exactly
/// `shape & points`; with [`set_stayman_net_force`] on, `shape` plus the net's
/// `want` verdict on `tricks` in `strain` replacing the point test
///
/// The call *reads* as `shape & points` either way ([`reads_as`]) — the net
/// arm is an opaque predicate, and letting it into the projection `Or` would
/// union the authored band out to a vacuous reading.
fn stayman_net_seam(
    shape: Cons<impl Constraint + Clone>,
    points: Cons<impl Constraint + Clone>,
    want: bool,
    strain: Strain,
    tricks: u8,
) -> Cons<impl Constraint + Clone> {
    let off = pred(|_: Hand, _: &Context<'_>| !stayman_net_force());
    let evaluated = shape.clone()
        & ((off & points.clone()) | net_break_even_gate(stayman_net_force, want, strain, tricks));
    reads_as(evaluated, shape & points)
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

/// Author the longer-major transfer discipline for books built *after* this
/// call (thread-local; **on by default**).
///
/// The Jacoby transfer names the longer major (a 6♠5♥ hand transfers to
/// spades, whatever its strength).  With **equal** lengths (5-5, 6-6) the
/// route splits by strength: weak transfers to *hearts* (the safe partscore —
/// nothing shows the spades below it anyway), invitational and minimum game
/// force bid the both-majors `3♦` (which this discipline also restricts to
/// equal lengths — a 6-5 hand prefers naming its longer suit first), and a
/// slam try (17+) transfers to *spades* for the `1NT–2♥–2♠–3♥` natural
/// game-force structure.  Off restores the legacy guards for the A/B.
pub fn set_transfer_longer_major(on: bool) {
    TRANSFER_LONGER_MAJOR.with(|cell| cell.set(on));
}

/// Whether the longer-major transfer discipline is currently authored (read at
/// book construction)
fn transfer_longer_major() -> bool {
    TRANSFER_LONGER_MAJOR.with(Cell::get)
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

/// Author responder's continuation over opener's `3OM`-slam-try cue for books built
/// *after* this call (thread-local; **on by default**).
///
/// Over opener's cue (a control below the trump major, showing a maximum) responder
/// keycards with slam values or signs off in the major game — closing the dead-end
/// where the cue was otherwise passed out below game.  See `stayman_cue_rebid`.
pub fn set_stayman_cue_continuation(on: bool) {
    STAYMAN_CUE_CONTINUATION.with(|cell| cell.set(on));
}

/// Whether responder's `3OM`-cue continuation is currently authored
fn stayman_cue_continuation() -> bool {
    STAYMAN_CUE_CONTINUATION.with(Cell::get)
}

/// Author the Stayman-then-minor slam try for books built *after* this call
/// (thread-local; **on by default** — pass `false` to disable).
///
/// Over opener's Stayman answer, a natural `3♣`/`3♦` shows a 5+ minor with slam
/// values and no major fit; opener raises the minor with a fit + maximum (else
/// `3NT`), and responder keycards over the raise.
pub fn set_stayman_minor_slam_try(on: bool) {
    STAYMAN_MINOR_SLAM_TRY.with(|cell| cell.set(on));
}

/// Whether the Stayman-then-minor slam try is currently authored
fn stayman_minor_slam_try() -> bool {
    STAYMAN_MINOR_SLAM_TRY.with(Cell::get)
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
        // Fit-known: a 6-card major opposite 1NT's 2+ is an 8-card fit.
        // Side-suit shortness counts; the length term is explicit.
        let support = usize::from(support_point_count_in(hand, major));
        support + hand[major].len() >= floor
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
            150,
            len(into, 4..) & hcp(17..),
        );
    }
    rules.rule(Bid::new(2, Strain::from(into)), 100, hcp(0..))
}

/// Complete a four-level Texas transfer by bidding game in the anchor major
///
/// `4♣ → 4♥`, `4♦ → 4♠`.  Responder showed 6+ with game-no-slam values, so
/// opener simply names the game and declares.
fn complete_texas(into: Suit) -> Rules {
    Rules::new().rule(Bid::new(4, Strain::from(into)), 100, hcp(0..))
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
        130,
        len(major, 6..)
            & len(other_major(major), ..5)
            & described("six-card invitational value", move |hand: Hand, _| {
                // Fit-known: 6-card major opposite 1NT's 2+ is an 8-card fit.
                // Side-suit shortness counts; the length term is explicit.
                let support = usize::from(support_point_count_in(hand, major));
                support + hand[major].len() >= floor
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
            100,
            described("accept six-card invite", move |hand: Hand, _| {
                // Fit-known: responder showed six, opener has 2+ — an 8-card fit.
                // A doubleton trump holding earns no phantom ruffing value —
                // the corner where the suit-indexed scale measurably won.
                let support = usize::from(support_point_count_in(hand, major));
                support + hand[major].len() >= floor
            }),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer to a direct four-of-a-major slam try (`1NT–4♥/4♠`)
///
/// Non-forcing: a **maximum** (17) accepts by launching RKCB (`4NT`); a minimum
/// signs off by passing the major game.  The 1430 ladder ([`slam`]) then exchanges
/// keycards and places `6M`, or `5M` when the partnership is missing two.
fn slam_try_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, hcp(17..))
        .alert(slam::RKCB)
        .rule(Call::Pass, 0, hcp(..17))
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
            130,
            len(Suit::Spades, 5..) & hcp(8..),
        );
    }
    let rules = rules
        // Fit: artificial slam try / choice of game (balanced, or 16+); denies 5♠.
        .rule(
            Bid::new(3, other),
            140,
            stayman_net_seam(
                len(major, 4..) & (balanced() | hcp(16..)) & spade_cap.clone(),
                hcp(9..),
                true,
                strain,
                10,
            ),
        )
        .alert(SLAM_TRY)
        // Fit: sign off in the major game — any upgrade past a flat eight (a ninth
        // trump, or working shape) commits to game opposite the 15-17 opener.
        .rule(
            Bid::new(4, strain),
            130,
            stayman_net_seam(
                len(major, 4..) & spade_cap.clone(),
                described("game value for the fit", move |hand: Hand, _| {
                    fit_value(hand, major) >= 9
                }),
                true,
                strain,
                10,
            ),
        )
        // Fit: invitational raise — a flat eight, four-card fit, no upgrade (or,
        // net-priced, any fit hand whose game the net declines).
        .rule(
            Bid::new(3, strain),
            120,
            stayman_net_seam(
                len(major, 4..) & spade_cap.clone(),
                described("invitational value for the fit", move |hand: Hand, _| {
                    fit_value(hand, major) == 8
                }),
                false,
                strain,
                10,
            ),
        )
        // No fit: quantitative 4NT (as if the 2♣ detour never happened).
        .rule(
            Bid::new(4, Strain::Notrump),
            120,
            len(major, ..4) & hcp(16..=17),
        )
        // No fit: game / invitational notrump raise.  The net seams keep the
        // 2♣ entry's 8-HCP floor so a garbage/crawling weak hand never invites.
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            stayman_net_seam(
                len(major, ..4) & hcp(8..),
                hcp(9..),
                true,
                Strain::Notrump,
                9,
            ),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            100,
            stayman_net_seam(
                len(major, ..4) & hcp(8..),
                hcp(8..=8),
                false,
                Strain::Notrump,
                9,
            ),
        );
    // Stayman-then-minor slam try: a natural 5+ minor with slam values (14+) and no
    // fit for opener's major (capped at three, else responder raises or takes the
    // 3OM slam try).  Weight 1.25 outranks the no-fit `3NT`/`4NT` reverts so the
    // two-suiter shows its second suit instead of guessing notrump.  Empty off the
    // gate; the minor is real, so each rule floors only its own strain and stays
    // unalerted (the artificial-alert invariant).
    if stayman_minor_slam_try() {
        rules
            .rule(
                Bid::new(3, Strain::Clubs),
                125,
                len(Suit::Clubs, 5..) & hcp(14..) & len(major, ..4),
            )
            .rule(
                Bid::new(3, Strain::Diamonds),
                125,
                len(Suit::Diamonds, 5..) & hcp(14..) & len(major, ..4),
            )
    } else {
        rules
    }
}

/// A flat 4-3-3-3 — the one balanced shape with no doubleton
pub(super) fn flat_4333() -> Cons<impl Constraint + Clone> {
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
    let mut rules = Rules::new().rule(Bid::new(3, Strain::Notrump), 140, flat_4333());
    // Cheapest control cue with a maximum: each suit ranking below the major.
    let mut weight = 130;
    for cue in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        if Strain::from(cue) < Strain::from(major) {
            rules = rules.rule(
                Bid::new(4, Strain::from(cue)),
                weight,
                hcp(17..) & control_in(cue),
            );
            weight -= 5;
        }
    }
    // Minimum, or a maximum without a cheap control: sign off in game.
    rules.rule(Bid::new(4, Strain::from(major)), 100, hcp(0..))
}

/// Responder's rebid after opener cooperates with the `3OM` slam try by cue-bidding
///
/// Opener's cue (a control ranking below the trump `major`) showed a **maximum**
/// (17) plus slam interest — see [`stayman_slam_try_answer`].  Responder's `3OM` was
/// a wide choice-of-game *or* slam try, so responder resolves it here: a slam-worthy
/// hand keycards (`4NT` RKCB, the [`slam`] 1430 ladder placing the contract),
/// everything else signs off in the major game.  Without this node opener's cue was
/// passed out — often *below* game — the dominant Stayman leak this fixes.  Gated by
/// [`set_stayman_cue_continuation`] (on by default).
fn stayman_cue_rebid(major: Suit) -> Rules {
    Rules::new()
        // Slam values opposite a known maximum plus the shown control: keycard.
        .rule(Bid::new(4, Strain::Notrump), 120, hcp(14..))
        .alert(slam::RKCB)
        // Otherwise the 3OM was only choosing the game: sign off in the major.
        .rule(Bid::new(4, Strain::from(major)), 100, hcp(0..))
}

/// Opener's reply to responder's Stayman-then-minor slam try (`…3♣` / `…3♦`)
///
/// Responder showed a natural 5+ `minor` with slam values (14+) and no major fit.
/// With four-card support *and* a maximum (16-17) opener cooperates by raising to
/// `4m`, setting trump for responder's keycard ask; otherwise — no fit, or a
/// minimum — opener signs off in `3NT`, the game responder's values guarantee.
/// The `3NT` catch-all keeps the table total (the finite-fallback invariant).
fn stayman_minor_answer(minor: Suit) -> Rules {
    Rules::new()
        // Fit + maximum: raise the minor, inviting the keycard ask.
        .rule(
            Bid::new(4, Strain::from(minor)),
            130,
            len(minor, 4..) & hcp(16..),
        )
        // No fit, or a minimum: place game in notrump.
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
}

/// Responder's keycard ask after opener raises the Stayman-then-minor slam try
/// (`…3m–4m`)
///
/// Opener confirmed a four-card fit and a maximum, so responder — who opened the
/// slam try with 14+ — keycards (`4NT` RKCB, the [`slam`] 1430 ladder placing the
/// minor slam or signing off in `5m` when a keycard is missing).  Both hands are
/// known non-minimum before the ask, so — unlike the transfer-then-minor path
/// ([`gf_minor_answer`]) — the five-level response is safe.  Artificial, so the
/// `4NT` carries the [`slam::RKCB`] alert.
fn stayman_minor_slam_rkcb() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, hcp(0..))
        .alert(slam::RKCB)
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
    // The GF-majors structure repurposes the spade `3♥` (natural 5-5 slam try) and —
    // with the heart mirror on — the heart `3♠` (spade splinter), relocating each
    // single-suiter to a quantitative `4NT`, so yield the slot to that structure.
    if (major == Suit::Spades && transfer_gf_majors())
        || (major == Suit::Hearts && transfer_gf_hearts())
    {
        return Rules::new();
    }
    Rules::new()
        .rule(
            Bid::new(3, Strain::from(other_major(major))),
            140,
            len(major, 5..) & len(other_major(major), ..4) & hcp(16..),
        )
        .alert(SLAM_TRY)
}

/// Responder's game-forcing rebid after the spade transfer completes
/// (`1NT–2♥–2♠`), under the GF-majors structure
///
/// `3♥` is a natural 5-5 slam try — the slam end of the both-majors hands, rerouted
/// off the capped `1NT–3♦` (the `points(17..)` floor tiles against the `3♦` cap of
/// `points(..=16)`).  `4NT` is the single-suiter's quantitative slam invite (16+,
/// denying a fourth heart — the hand the old artificial `3♥` slam try showed, now
/// relocated here).  `3♥` floors only its own strain (the transfer pins the five
/// spades), so it stays unalerted; `4NT` is conventional and carries [`SLAM_TRY`].
/// `3♣`/`3♦` show five spades and a four-card minor — game-forcing (Arm A), or with
/// [`minor_min_to_3nt`] on slam-only (Arm B), the minimums then resting in the
/// floor's choice-of-games `3NT`.  All three natural calls floor only their own
/// strains, so they stay unalerted; `4NT` is conventional and carries [`SLAM_TRY`].
/// Empty off the gate.
fn transfer_spade_gf_rebid() -> Rules {
    if !transfer_gf_majors() {
        return Rules::new();
    }
    // Arm A shows the minor on any game force; Arm B reserves it for slam tries,
    // routing minimum game-forces to the floor's `3NT` (the `minor_min_to_3nt` A/B).
    let minor_floor: u8 = if minor_min_to_3nt() { 17 } else { 10 };
    Rules::new()
        // The transfer already pins responder's five spades (`transfer_major_reading`),
        // so these natural rebids restate only their *own* second strain — flooring no
        // un-named suit keeps them off the alert list (the `artificial` invariant).
        .rule(
            Bid::new(3, Strain::Hearts),
            150,
            len(Suit::Hearts, 5..) & points(17..),
        )
        .rule(
            Bid::new(3, Strain::Clubs),
            145,
            len(Suit::Clubs, 4..) & len(Suit::Hearts, ..4) & points(minor_floor..),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            145,
            len(Suit::Diamonds, 4..) & len(Suit::Hearts, ..4) & points(minor_floor..),
        )
        // Choice of games: exactly five spades (the transfer pins the floor; `..6`
        // rules out a six-card one-suiter), game values but short of the 16+ slam
        // quant.  Natural (all upper bounds — floors no un-named suit, so
        // unalerted), and being a recognised undisturbed node it lets opener read
        // 3NT as *balanced* rather than guessing — the read only holds undisturbed,
        // which is why opener's correction gates on it (`has_ruffing_shortness`).
        //
        // `hcp(9..16)`, not `points(10..) & hcp(..16)`, and no minor caps: the
        // gate mirrors the instinct floor's game force (`hcp(9..)` undisturbed —
        // the shipped 9-count seam, `nt_responder_game_floor`).  The old tighter
        // gates sent the 9-count and the 4-card-minor game forces below the
        // minor rules' `points(10..)` through to that floor, which bid the
        // *same* 3NT — and under `set_natural_reading` this rule's projected
        // box then excluded the very hands that bid it.  Every hand the old
        // gates admitted was balanced (shape forced by the caps), where
        // `points(10..)` implies `hcp(10..)`, so this is a pure loosening: the
        // bidder is unchanged (hands newly admitted here bid this same 3NT via
        // the floor before, and every rule sharing a 9..15 hand outweighs 1.4).
        .rule(
            Bid::new(3, Strain::Notrump),
            140,
            len(Suit::Spades, ..6) & len(Suit::Hearts, ..4) & hcp(9..16),
        )
        .rule(
            Bid::new(4, Strain::Notrump),
            140,
            len(Suit::Spades, 5..)
                & len(Suit::Hearts, ..4)
                & len(Suit::Clubs, ..4)
                & len(Suit::Diamonds, ..4)
                & hcp(16..),
        )
        .alert(SLAM_TRY)
        // Six-card-spade slam tries with a side-suit splinter — carved off the direct
        // Texas `4♦` / `4♠` (see `is_major_splinter_slam`).  Artificial (the bid names
        // the *short* suit, not a strain to play), so each carries [`SPLINTER`].
        // Responder's own six-card spade suit is the trump, +6.
        .rule(
            Bid::new(4, Strain::Clubs),
            155,
            len(Suit::Spades, 6..)
                & splinter_short(Suit::Clubs)
                & support_points(Suit::Spades, 16..),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(4, Strain::Diamonds),
            155,
            len(Suit::Spades, 6..)
                & splinter_short(Suit::Diamonds)
                & support_points(Suit::Spades, 16..),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(4, Strain::Hearts),
            155,
            len(Suit::Spades, 6..)
                & splinter_short(Suit::Hearts)
                & support_points(Suit::Spades, 16..),
        )
        .alert(SPLINTER)
}

/// Responder's game-forcing rebid after the *heart* transfer completes (`1NT–2♦–2♥`)
///
/// The heart mirror of [`transfer_spade_gf_rebid`], tighter because `2♠`/`2NT` are the
/// single-suited/`5♥4♠` invites and `3♥` is the six-card invite.  So there is **no**
/// 5-5 slot here — a 5-5 slam try rides the spade transfer ([`slam_55_reroute`]) — and
/// the spade shortness splinter drops to a cheap `3♠` (below `4♥`), freed by evicting
/// the single-suited slam try to `4NT`:
///
/// - `3♣`/`3♦` — five hearts and a four-card minor, game-forcing (Arm A).  Natural
///   (the transfer pins the hearts), flooring only the minor, so unalerted.
/// - `3♠` / `4♣` / `4♦` — a six-heart splinter short in spades / clubs / diamonds,
///   carried onto the transfer off the direct Texas `4♣` / `4♥`; each carries
///   [`SPLINTER`].
/// - `4NT` — the single-suiter's quantitative slam invite, [`SLAM_TRY`].
///
/// Empty off the heart gate.
fn transfer_heart_gf_rebid() -> Rules {
    if !transfer_gf_hearts() {
        return Rules::new();
    }
    let minor_floor: u8 = if minor_min_to_3nt() { 17 } else { 10 };
    // The transfer pins responder's five hearts, so these natural rebids restate only
    // their own second strain (floor no un-named suit → off the alert list).
    Rules::new()
        .rule(
            Bid::new(3, Strain::Clubs),
            145,
            len(Suit::Clubs, 4..) & len(Suit::Spades, ..4) & points(minor_floor..),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            145,
            len(Suit::Diamonds, 4..) & len(Suit::Spades, ..4) & points(minor_floor..),
        )
        // Choice of games: exactly five hearts (`..6` rules out a six-card one-suiter),
        // game values short of the 16+ slam quant.  Natural (upper bounds
        // only — unalerted), the undisturbed node that lets opener read 3NT as
        // *balanced* for the ruff-gated correction.  `hcp(9..16)` mirrors the
        // instinct floor's game force so the rule owns every 3NT this node
        // actually produces — see the spade mirror (`transfer_spade_gf_rebid`).
        .rule(
            Bid::new(3, Strain::Notrump),
            140,
            len(Suit::Hearts, ..6) & len(Suit::Spades, ..4) & hcp(9..16),
        )
        // Six-card-heart slam tries with a side-suit splinter — the spade shortness at
        // the cheap `3♠`, the minors at the four level.  Artificial, so each is alerted.
        // Responder's own six-card heart suit is the trump, +6.
        .rule(
            Bid::new(3, Strain::Spades),
            155,
            len(Suit::Hearts, 6..)
                & splinter_short(Suit::Spades)
                & support_points(Suit::Hearts, 16..),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(4, Strain::Clubs),
            155,
            len(Suit::Hearts, 6..)
                & splinter_short(Suit::Clubs)
                & support_points(Suit::Hearts, 16..),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(4, Strain::Diamonds),
            155,
            len(Suit::Hearts, 6..)
                & splinter_short(Suit::Diamonds)
                & support_points(Suit::Hearts, 16..),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(4, Strain::Notrump),
            140,
            len(Suit::Hearts, 5..)
                & len(Suit::Spades, ..4)
                & len(Suit::Clubs, ..4)
                & len(Suit::Diamonds, ..4)
                & hcp(16..),
        )
        .alert(SLAM_TRY)
}

/// Opener's answer to responder's quantitative `4NT` (`1NT–2♥–2♠–4NT`)
///
/// Responder is a balanced 16+ single-suited five-spade hand inviting slam.  A
/// maximum (17) accepts — `6M` on three-card support (the known eight-card fit),
/// else `6NT`; a minimum declines by passing `4NT`.  ponytail: blasts the small slam
/// rather than RKCB — the invite is balanced and quantitative, so a keycard detour
/// buys little.
fn gf_quant_answer(major: Suit) -> Rules {
    Rules::new()
        .rule(
            Bid::new(6, Strain::from(major)),
            130,
            hcp(17..) & len(major, 3..),
        )
        .rule(Bid::new(6, Strain::Notrump), 120, hcp(17..))
        .rule(Call::Pass, 0, hcp(..17))
}

/// Opener's answer to responder's five-spade-plus-minor game force (`…3♣` / `…3♦`)
///
/// The five-three major fit is the anchor, and the minor is game-forcing but
/// *undifferentiated* (minimum through slam in Arm A), so opener places game rather
/// than keycarding: `4M` on three-card support — the 5-3 fit's ruffing value
/// out-scores an un-pulled `3NT` — else `3NT`.  ponytail: no RKCB over the minor. A
/// paired A/B caught opener blasting slam on its own maximum opposite a possible bare
/// minimum, doubled into the artificial five-level keycard response; the minor's own
/// slam is left to the (near-impossible) rare hand this treatment already discounts.
fn gf_minor_answer(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 110, len(major, 3..))
        .rule(Bid::new(3, Strain::Notrump), 100, len(major, ..3))
}

/// Opener's answer to responder's six-card-spade splinter (`…4♣` / `…4♦` / `…4♥`)
///
/// The major is agreed (responder showed six) and the shortness is known, so this is
/// game-forcing: a maximum (17) cooperates by launching RKCB in the major (`4NT`), a
/// minimum signs off in the major game (`4M`).  ponytail: the shortness-versus-values
/// judgment (are opener's honors opposite the splinter wasted?) is left to raw
/// strength — a finer wasted-value gate is the upgrade path if the A/B wants it.
fn gf_splinter_answer(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, hcp(17..))
        .alert(slam::RKCB)
        .rule(Bid::new(4, Strain::from(major)), 0, hcp(..17))
}

/// The `longer` major strictly outnumbers `shorter` — the transfer names the
/// longer major (see [`set_transfer_longer_major`])
fn longer_major(longer: Suit, shorter: Suit) -> Cons<impl Constraint + Clone> {
    longer_suit(longer, shorter)
}

/// Both majors of exactly equal length (the 5-5/6-6 two-suiters)
fn equal_majors() -> Cons<impl Constraint + Clone> {
    equal_length("equal-length majors", Suit::Hearts, Suit::Spades)
}

/// A 5-5 majors hand strong enough to drive slam (`point_count ≥ 17`)
///
/// Capped off the both-majors `3♦` (`points(..=16)`) when the GF-majors structure
/// is on, such a hand instead transfers and rebids the natural `3♥` slam try, so
/// this widens the spade-transfer guard to admit it.  Always false off the gate, so
/// the baseline transfer guard is unchanged.
fn slam_55_reroute() -> Cons<impl Constraint + Clone> {
    let legacy = described(
        "5-5 slam reroute to the spade transfer",
        |hand: Hand, _: &Context<'_>| {
            transfer_gf_majors()
                && hand[Suit::Hearts].len() >= 5
                && usize::from(point_count(hand)) >= 17
        },
    );
    // Whenever the gate accepts at all it requires 5+ hearts and 17+ points
    // (and off its treatment it accepts nothing), so the box is sound in both
    // knob states.
    let mut envelope = long_suit_box(
        Suit::Hearts,
        Range::new(5, Range::FULL_LENGTH.max),
        Range::FULL_LENGTH,
    );
    envelope.strength.points = Range::new(17, Range::FULL_POINTS.max);
    envelope_union_upgrade(legacy, EnvelopeUnion::from(envelope))
}

/// A void or low singleton — the shortness a splinter shows
///
/// A singleton ace or king is a *working* honor, not shortness to advertise, so it
/// is excluded (the same wasted-honor principle as [`upgrade`]'s `wasted`).
fn is_splinter_holding(holding: Holding) -> bool {
    holding.is_empty()
        || (holding.len() == 1 && !holding.contains(Rank::A) && !holding.contains(Rank::K))
}

/// Void or a low singleton in `suit`, as a constraint (see [`is_splinter_holding`])
fn splinter_short(suit: Suit) -> Cons<impl Constraint + Clone> {
    let legacy = described(
        "void or a low singleton",
        move |hand: Hand, _: &Context<'_>| is_splinter_holding(hand[suit]),
    );
    // Sound over-approximation: the singleton-A/K exclusion is honor
    // location, which no `Envelope` axis holds — the box keeps `suit ≤ 1`.
    envelope_union_upgrade(
        legacy,
        EnvelopeUnion::from(long_suit_box(suit, Range::new(0, 1), Range::FULL_LENGTH)),
    )
}

/// A six-card-`major` slam hand (16+ points) with a describable side-suit splinter
///
/// Under the GF-majors gate this hand is carved off the direct Texas transfer and
/// direct game-jump onto the Jacoby transfer, where it splinters (spades at the four
/// level, hearts as low as `3♠`).  Gated per major — spades by the master flag, hearts
/// by [`transfer_gf_hearts`] — so each side is false until its own structure is on.
fn is_major_splinter_slam(hand: Hand, major: Suit) -> bool {
    let active = if major == Suit::Hearts {
        transfer_gf_hearts()
    } else {
        transfer_gf_majors()
    };
    active
        && hand[major].len() >= 6
        // Fit-known: 6-card major opposite 1NT's 2+ is an 8-card fit, and this
        // hand has a side-suit splinter — count the shortness as support
        // value, in lockstep with the splinter gates'
        // `support_points(major, 16..)` and the reroute boxes.
        && usize::from(support_point_count_in(hand, major)) >= 16
        && Suit::ASC
            .into_iter()
            .filter(|&suit| suit != major)
            .any(|suit| is_splinter_holding(hand[suit]))
}

/// The splinter reroute as a positive constraint (widens the major's transfer guard)
fn major_splinter_reroute(major: Suit) -> Cons<impl Constraint + Clone> {
    let legacy = described(
        "6-card-major splinter reroute to the transfer",
        move |hand: Hand, _: &Context<'_>| is_major_splinter_slam(hand, major),
    );
    // One box per short side suit: 6+ in the major, 16+ support points, that
    // side suit at most one card (the singleton-honor exclusion is
    // over-approximated away, as in `splinter_short`).  Sound in both
    // treatment states; raw `union`, never `disjoin`, for authoring-time
    // boxes (the C2 precedent — `disjoin` would hull them while the knob is
    // off during book construction).
    let boxes = Suit::ASC
        .into_iter()
        .filter(|&suit| suit != major)
        .map(|short| {
            let mut envelope = long_suit_box(
                major,
                Range::new(6, Range::FULL_LENGTH.max),
                Range::FULL_LENGTH,
            );
            envelope.lengths[short as usize] = Range::new(0, 1);
            envelope.narrow_support_points(major, Range::new(16, Range::FULL_POINTS.max));
            EnvelopeUnion::from(envelope)
        })
        .reduce(EnvelopeUnion::union)
        .unwrap_or_else(EnvelopeUnion::unknown);
    envelope_union_upgrade(legacy, boxes)
}

/// The splinter reroute negated (carves the direct Texas and direct game-jump)
fn not_major_splinter_slam(major: Suit) -> Cons<impl Constraint + Clone> {
    described(
        "not a describable 6-card-major splinter slam",
        move |hand: Hand, _: &Context<'_>| !is_major_splinter_slam(hand, major),
    )
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
        .rule(Bid::new(4, Strain::Notrump), 100, hcp(17..))
        .alert(slam::RKCB)
        .rule(Bid::new(4, Strain::from(major)), 0, hcp(..17))
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
            140,
            len(Suit::Hearts, 4..=4) & len(Suit::Spades, 5..) & hcp(9..),
        )
        .alert(SMOLEN)
        .rule(
            Bid::new(3, Strain::Spades),
            140,
            len(Suit::Spades, 4..=4) & len(Suit::Hearts, 5..) & hcp(9..),
        )
        .alert(SMOLEN)
        .rule(Bid::new(4, Strain::Notrump), 120, hcp(16..=17))
        // The notrump revert seams are net-priced under `set_stayman_net_force`
        // (Smolen and the quantitative 4NT outrank them by weight either way).
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            stayman_net_seam(hcp(8..), hcp(9..), true, Strain::Notrump, 9),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            100,
            stayman_net_seam(hcp(8..), hcp(8..=8), false, Strain::Notrump, 9),
        );
    let rules = if crawling_stayman() {
        // Crawling Stayman: 4-4 majors short in diamonds (a bare 2♥, weak) — both
        // majors, pass-or-correct (see `answer_crawling_stayman`).  Gated by the
        // diamond shortness (≤1) that brought it here: garbage hands have 3+
        // diamonds and pass 2♦ instead.  Responder's four spades is implied by the
        // crawling 2♣, so 2♥ floors only hearts and stays unalerted natural (like
        // the 2♠ sibling).  Disjoint from every rule above (all need hcp ≥8).
        rules.rule(
            Bid::new(2, Strain::Hearts),
            140,
            len(Suit::Hearts, 4..) & len(Suit::Diamonds, ..=1) & hcp(..8),
        )
    } else {
        rules
    };
    let rules = if invitational_5card_majors() {
        // 5♠4♥, non-forcing invitational: opener denied hearts, so name the
        // five-card spade suit (natural, outranks the 2NT fallback).  Opener passes
        // a minimum or raises to game (see `answer_inv_5card_spades`).  A 5♠4♥
        // game-force jumped Smolen `3♥` above.  Responder's four hearts is implied
        // (it Staymaned), so `2♠` floors only spades and stays unalerted natural.
        rules.rule(
            Bid::new(2, Strain::Spades),
            110,
            len(Suit::Spades, 5..) & hcp(8..=8),
        )
    } else {
        rules
    };
    // Stayman-then-minor slam try, opener having denied a major: a natural 5+ minor
    // with slam values (14+).  No major cap — the 2♦ answer already denied the fit.
    // Weight 1.25 outranks the `3NT`/`4NT` reverts; the minor is real, so it floors
    // only its own strain and stays unalerted.  Empty off the gate.  (Smolen owns
    // `3♥`/`3♠`, so `3♣`/`3♦` are free here.)
    if stayman_minor_slam_try() {
        rules
            .rule(
                Bid::new(3, Strain::Clubs),
                125,
                len(Suit::Clubs, 5..) & hcp(14..),
            )
            .rule(
                Bid::new(3, Strain::Diamonds),
                125,
                len(Suit::Diamonds, 5..) & hcp(14..),
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
        .rule(Bid::new(4, strain), 100, len(five_card, 3..))
        // No fit: notrump game.
        .rule(Bid::new(3, Strain::Notrump), 50, len(five_card, ..3))
}

/// Smolen at the three level: responder's jump after opener denies a major
/// (`…3♣–3♦`).  Game is already forced, so no strength gate.
pub(super) fn smolen_at_three() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Hearts),
            140,
            len(Suit::Hearts, 4..=4) & len(Suit::Spades, 5..),
        )
        .alert(SMOLEN)
        .rule(
            Bid::new(3, Strain::Spades),
            140,
            len(Suit::Spades, 4..=4) & len(Suit::Hearts, 5..),
        )
        .alert(SMOLEN)
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
}

/// Opener accepts a no-fit (2NT) Stayman invitation with a maximum, else passes
///
/// Responder invited with a bare 8, so a 1NT opener needs its 17 for game (8+17
/// = 25).  Authored rather than left to the floor: the keyless floor reads a
/// three-level suit response over our 1NT as forcing and so cannot *decline* an
/// invitational raise.
fn accept_invitation(game: Bid) -> Rules {
    Rules::new()
        .rule(game, 100, hcp(17..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's acceptance of an invitational major raise
///
/// With a maximum, bid the major game — but choose 3NT on a flat 4-3-3-3, where
/// notrump rates to play as well as the eight-card fit.  A minimum passes.
fn accept_major_invitation(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 110, hcp(17..) & flat_4333())
        .rule(Bid::new(4, Strain::from(major)), 100, hcp(17..))
        .rule(Call::Pass, 0, hcp(0..))
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
            100,
            len(Suit::Hearts, ..3) & len(Suit::Spades, ..3),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            100,
            len(Suit::Hearts, ..3) & len(Suit::Spades, 3..),
        )
        .rule(Call::Pass, 0, len(Suit::Hearts, 3..))
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
            120,
            hcp(17..) & len(Suit::Spades, 3..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            110,
            hcp(17..) & len(Suit::Spades, ..3),
        )
        .rule(Call::Pass, 0, hcp(0..))
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
            130,
            hcp(17..) & len(Suit::Spades, 3..),
        )
        .rule(
            Bid::new(4, Strain::Hearts),
            120,
            hcp(17..) & len(Suit::Spades, ..3),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            110,
            hcp(..17) & len(Suit::Spades, 3..),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            100,
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
        .rule(Bid::new(4, strain), 100, hcp(9..))
        .rule(Call::Pass, 0, hcp(0..))
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
            120,
            len(Suit::Hearts, 5..) & len(Suit::Spades, 4..=4) & hcp(8..=8),
        )
        .alert(INV_5CARD)
        .rule(
            Bid::new(2, Strain::Spades),
            120,
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
            140,
            hcp(17..) & len(Suit::Hearts, 3..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            130,
            hcp(17..) & len(Suit::Hearts, ..3),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            110,
            hcp(..17) & len(Suit::Hearts, 3..),
        )
        .rule(Bid::new(2, Strain::Notrump), 0, hcp(0..))
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
            160,
            hcp(17..) & len(Suit::Hearts, 3..),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            150,
            hcp(17..) & len(Suit::Hearts, ..3) & len(Suit::Spades, 4..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            140,
            hcp(17..) & len(Suit::Hearts, ..3) & len(Suit::Spades, ..4),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            120,
            hcp(..17) & len(Suit::Hearts, 3..),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            110,
            hcp(..17) & len(Suit::Hearts, ..3) & len(Suit::Spades, 4..),
        )
        .rule(Call::Pass, 0, hcp(0..))
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
            120,
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
            140,
            hcp(17..) & len(Suit::Spades, 3..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            130,
            hcp(17..) & len(Suit::Spades, ..3),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            110,
            hcp(..17) & len(Suit::Spades, 3..),
        )
        .rule(Call::Pass, 0, hcp(0..))
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
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 5..))
        .rule(
            Bid::new(3, Strain::Spades),
            100,
            len(Suit::Spades, 5..) & len(Suit::Hearts, ..5),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            50,
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
        .rule(Bid::new(4, Strain::from(major)), 100, len(major, 3..))
        .rule(Bid::new(3, Strain::Notrump), 50, len(major, ..3))
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
            100,
            len(Suit::Spades, 4..=4) & len(Suit::Hearts, 3..=3),
        )
        .alert(SMOLEN)
        .rule(
            Bid::new(3, Strain::Spades),
            100,
            len(Suit::Hearts, 4..=4) & len(Suit::Spades, 3..=3),
        )
        .alert(SMOLEN)
        .rule(Bid::new(3, Strain::Notrump), 50, hcp(0..))
}

/// Opener completes the Puppet 4-4 hunt: game in responder's shown major, or 3NT
///
/// Responder's short-major bid named four cards in `shown_major`; raise to game
/// with four-card support, else 3NT.
fn puppet_smolen_completion(shown_major: Suit) -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::from(shown_major)),
            100,
            len(shown_major, 4..),
        )
        .rule(Bid::new(3, Strain::Notrump), 50, len(shown_major, ..4))
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
            120,
            hcp(17..) & len(Suit::Spades, 3..),
        )
        .rule(
            Bid::new(4, Strain::Hearts),
            120,
            hcp(17..) & len(Suit::Spades, ..3) & len(Suit::Hearts, 3..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            hcp(17..) & len(Suit::Spades, ..3) & len(Suit::Hearts, ..3),
        )
        .rule(Bid::new(3, Strain::Spades), 100, len(Suit::Spades, 3..))
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Spades, ..3))
}

/// Responder's decision over opener's minimum 3-level signoff
///
/// Opener showed 15–16 by signing off in `major`; responder raises to game with
/// the upper half of the invitational+ range and otherwise passes.  Needed
/// because the floor forces responder to game off the 3♦ opening and so could
/// not pass the invitation.  `points` again — responder is the 5-5 hand.
fn five_five_min_rebid(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 100, points(10..))
        .rule(Call::Pass, 90, points(..10))
}

// ---------------------------------------------------------------------------
// Minor-suit transfers (1NT–2NT diamonds, 1NT–2♠ clubs/invite)
// ---------------------------------------------------------------------------

/// Opener passes a weak responder retreat
///
/// Authored only to override the keyless floor, which reads a three-level suit
/// response to our 1NT as game-forcing and would otherwise refuse to pass.
fn pass_out() -> Rules {
    Rules::new().rule(Call::Pass, 0, hcp(0..))
}

/// Opener's reply to the 2NT diamond transfer: complete to 3♦ with a fit, else 3♣
///
/// Three-card diamond support is an assured eight-card fit — complete the
/// transfer.  Short diamonds bid `3♣` instead, pass-or-correct, letting a 5♦4♣
/// responder pick the better minor.
fn diamond_transfer_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 100, len(Suit::Diamonds, 3..))
        .rule(Bid::new(3, Strain::Clubs), 50, len(Suit::Diamonds, ..3))
}

/// Responder's rebid after opener completes the diamond transfer (`…2NT–3♦`)
///
/// Game values bid 3NT — a long suit bids game on fewer points (`threshold` ≈ 8,
/// below the 9 a balanced hand needs).  Otherwise pass the diamond partscore.
fn diamond_transfer_game(threshold: u8) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(threshold..))
        .rule(Call::Pass, 0, hcp(..threshold))
}

/// Responder's rebid after opener's pass-or-correct `3♣` (`…2NT–3♣`, short ♦)
///
/// Game values bid 3NT; a six-card diamond suit retreats to `3♦` (a 6-2 fit beats
/// the possible club misfit); otherwise (5♦4♣) pass and sit for opener's clubs.
fn diamond_transfer_correct(threshold: u8) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(threshold..))
        .rule(
            Bid::new(3, Strain::Diamonds),
            50,
            len(Suit::Diamonds, 6..) & hcp(..threshold),
        )
        .rule(Call::Pass, 0, len(Suit::Diamonds, ..6) & hcp(..threshold))
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
        .rule(
            Bid::new(3, Strain::Clubs),
            100,
            hcp(size_ask_accept_floor()..),
        )
        .rule(Bid::new(2, Strain::Notrump), 90, hcp(0..))
}

/// Responder's pass-or-correct after opener's minimum `2NT` over the two-way 2♠
fn two_spade_over_min() -> Rules {
    Rules::new()
        // Balanced invite: opener is minimum, settle in 2NT.
        .rule(Call::Pass, 0, hcp(8..=8) & balanced())
        // Weak club one-suiter: correct to the club partscore.
        .rule(
            Bid::new(3, Strain::Clubs),
            80,
            len(Suit::Clubs, 6..) & hcp(..8),
        )
        // Game-going clubs with a singleton: splinter so opener picks 3NT or 5♣.
        .rule(
            Bid::new(3, Strain::Diamonds),
            100,
            club_splinter(Suit::Diamonds, 8),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(3, Strain::Hearts),
            100,
            club_splinter(Suit::Hearts, 8),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(3, Strain::Spades),
            100,
            club_splinter(Suit::Spades, 8),
        )
        .alert(SPLINTER)
        // Game-going clubs without a singleton: 3NT.
        .rule(Bid::new(3, Strain::Notrump), 90, club_no_shortness(8))
        .alert(PUPPET)
}

/// Responder's action after opener's maximum `3♣` over the two-way 2♠
fn two_spade_over_max() -> Rules {
    Rules::new()
        // Weak club one-suiter: pass the club partscore.
        .rule(Call::Pass, 0, len(Suit::Clubs, 6..) & hcp(..8))
        // Game-going clubs with a singleton: splinter.
        .rule(
            Bid::new(3, Strain::Diamonds),
            100,
            club_splinter(Suit::Diamonds, 8),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(3, Strain::Hearts),
            100,
            club_splinter(Suit::Hearts, 8),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(3, Strain::Spades),
            100,
            club_splinter(Suit::Spades, 8),
        )
        .alert(SPLINTER)
        // Balanced invite (opener maximum → accept game) or game clubs without a
        // singleton: 3NT.
        .rule(
            Bid::new(3, Strain::Notrump),
            90,
            (hcp(8..=8) & balanced()) | club_no_shortness(8),
        )
}

/// Opener picks the game over responder's club splinter: 3NT with the short suit
/// stopped, else 5♣
fn pick_game_over_club_splinter(short: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, stopper_in(short))
        .rule(Bid::new(5, Strain::Clubs), 90, hcp(0..))
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
    Rules::new().rule(Bid::new(3, Strain::Clubs), 0, hcp(0..))
}

/// Responder's rebid after opener completes the European club transfer (`…2♠–3♣`)
///
/// A weak six-card club one-suiter passes the partscore; a game-going hand
/// splinters in its singleton, or bids 3NT with no shortness.  Reuses the two-way
/// 2♠ club machinery minus its balanced-invite arm — that hand is the European 2NT.
fn european_two_spade_rebid() -> Rules {
    Rules::new()
        .rule(Call::Pass, 0, len(Suit::Clubs, 6..) & hcp(..8))
        .rule(
            Bid::new(3, Strain::Diamonds),
            100,
            club_splinter(Suit::Diamonds, 8),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(3, Strain::Hearts),
            100,
            club_splinter(Suit::Hearts, 8),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(3, Strain::Spades),
            100,
            club_splinter(Suit::Spades, 8),
        )
        .alert(SPLINTER)
        // Game-going clubs without a singleton: 3NT.  Artificial — it shows the
        // club one-suiter, not notrump — and tagged with its own scheme's variant
        // alert, mirroring `two_spade_over_min`'s identical rule under `PUPPET`.
        // Pure disclosure here: this is a continuation node, not one of the
        // `notrump_responses` that `gated(|a| a != dormant_minors())` filters.
        .rule(Bid::new(3, Strain::Notrump), 90, club_no_shortness(8))
        .alert(EUROPEAN)
}

/// Opener's reply to the European 2NT invite: `3NT` with a maximum, else pass
///
/// The 2NT bidder is a balanced eight; opposite a 17 (`25` combined) opener accepts
/// game, otherwise passes and plays 2NT — reproducing the natural-2NT outcome.
fn european_two_nt_answer() -> Rules {
    let floor = size_ask_accept_floor();
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(floor..))
        .rule(Call::Pass, 0, hcp(..floor))
}

/// Opener completes the European diamond transfer: `3♦`
fn european_three_club_answer() -> Rules {
    Rules::new().rule(Bid::new(3, Strain::Diamonds), 0, hcp(0..))
}

// ---------------------------------------------------------------------------
// 2NT-strength response structure (2NT opening and 2♣–2x–2NT rebid)
// ---------------------------------------------------------------------------

/// Responses to a 2NT-strength notrump (3-level Stayman/transfers, 4NT invite)
///
/// Used after both the direct 2NT opening (20–21 balanced) and opener's 2NT
/// rebid after 2♣ (22–24 balanced).
fn two_notrump_responses() -> Rules {
    // The longer-major discipline (see [`set_transfer_longer_major`]): a
    // two-suiter transfers to the longer major, equal lengths to hearts —
    // there is no both-majors bid or slam reroute at this level, so hearts
    // takes every tie.  Off, the old guards tie at 2.0 and the pick between
    // the transfers is arbitrary (a weak 6♠5♥ could transfer to hearts and
    // scramble — the M6.4 A/B caught exactly that board).
    let prefer_longer = transfer_longer_major();
    Rules::new()
        // 3-level Jacoby transfers.
        .rule(
            Bid::new(3, Strain::Diamonds),
            200,
            len(Suit::Hearts, 5..)
                & described(
                    "hearts not outnumbered (longer-major discipline)",
                    move |hand: Hand, _: &Context<'_>| {
                        !prefer_longer || hand[Suit::Hearts].len() >= hand[Suit::Spades].len()
                    },
                ),
        )
        .alert(JACOBY)
        .rule(
            Bid::new(3, Strain::Hearts),
            200,
            len(Suit::Spades, 5..)
                & described(
                    "spades longer (longer-major discipline)",
                    move |hand: Hand, _: &Context<'_>| {
                        !prefer_longer || hand[Suit::Spades].len() > hand[Suit::Hearts].len()
                    },
                ),
        )
        .alert(JACOBY)
        // 3-level Stayman: a four-card major and at least some values, but never a
        // flat 4-3-3-3 (it bids notrump directly, as over a 1NT opening).
        .rule(
            Bid::new(3, Strain::Clubs),
            150,
            (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4)) & hcp(5..) & !flat_4333(),
        )
        // Quantitative 4NT slam invite (balanced, no four-card major).
        .rule(
            Bid::new(4, Strain::Notrump),
            120,
            hcp(11..=12) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        // 3NT to play: game values, no major fit.
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            hcp(5..=10) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(Call::Pass, 0, hcp(..5))
}

/// Opener's answer to 3-level Stayman: a four-card major, else 3♦
fn stayman_answers_at_three() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(3, Strain::Spades),
            100,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            50,
            len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
}

/// Complete a 3-level transfer by bidding the anchor suit
fn complete_transfer_at_three(into: Suit) -> Rules {
    Rules::new().rule(Bid::new(3, Strain::from(into)), 100, hcp(0..))
}

/// Opener's answer to the quantitative 4NT: accept or decline the slam invite
///
/// `accept_hcp` is the minimum HCP to accept: 21 after a 2NT opening (20–21),
/// 24 after a 2♣–2x–2NT sequence (22–24).
fn quantitative_answer(accept_hcp: u8) -> Rules {
    Rules::new()
        .rule(Bid::new(6, Strain::Notrump), 100, hcp(accept_hcp..))
        .rule(Call::Pass, 0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Simple continuations after an 18–19 2NT rebid
// ---------------------------------------------------------------------------

/// Responder's call after opener's 18–19 2NT rebid
///
/// 6+ HCP bids 3NT; 12–13 makes a quantitative 4NT invite; fewer points pass.
fn after_rebid_two_notrump() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 120, hcp(12..=13))
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(6..))
        .rule(Call::Pass, 0, hcp(..6))
}

/// Opener's reply to the quantitative raise opposite the 18–19 rebid
///
/// Accept (6NT) with a maximum 19 HCP, decline (pass) otherwise.
fn accept_quantitative_nineteen() -> Rules {
    Rules::new()
        .rule(Bid::new(6, Strain::Notrump), 100, hcp(19..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Chain the heart-transfer rebids into their shared table
///
/// The package gate and this table deliberately read the knobs at different arities.
fn heart_transfer_rebid_table() -> Rules {
    let mut heart_rebid = Rules::new();
    if invitational_5card_majors() {
        heart_rebid = heart_rebid.chain(transfer_heart_invite_rebid());
    }
    heart_rebid = heart_rebid.chain(sixcard_invite_rebid(Suit::Hearts));
    heart_rebid = heart_rebid.chain(transfer_slam_try_rebid(Suit::Hearts));
    heart_rebid = heart_rebid.chain(transfer_heart_gf_rebid());
    heart_rebid
}

/// Chain the spade-transfer rebids into their shared table
///
/// The package gate and this table deliberately read the knobs at different arities.
fn spade_transfer_rebid_table() -> Rules {
    let mut spade_rebid = Rules::new();
    if invitational_5card_majors() {
        spade_rebid = spade_rebid.chain(transfer_spade_invite_rebid());
    }
    spade_rebid = spade_rebid.chain(sixcard_invite_rebid(Suit::Spades));
    spade_rebid = spade_rebid.chain(transfer_slam_try_rebid(Suit::Spades));
    spade_rebid = spade_rebid.chain(transfer_spade_gf_rebid());
    spade_rebid
}

/// Whether any treatment contributes to the heart-transfer rebid table
fn heart_transfer_rebid_active() -> bool {
    invitational_5card_majors()
        || sixcard_invite_active()
        || transfer_slam_try()
        || transfer_gf_hearts()
}

/// Whether any treatment contributes to the spade-transfer rebid table
fn spade_transfer_rebid_active() -> bool {
    invitational_5card_majors()
        || sixcard_invite_active()
        || transfer_slam_try()
        || transfer_gf_majors()
}

/// Whether the original heart-agreeing transfer slam-try node owns its path
fn heart_transfer_slam_try_active() -> bool {
    transfer_slam_try() && !transfer_gf_hearts()
}

/// Whether either treatment uses the spade-agreeing transfer slam-try node
fn spade_transfer_slam_try_active() -> bool {
    transfer_slam_try() || transfer_gf_majors()
}

// ---------------------------------------------------------------------------
// Declarative 1NT packages
// ---------------------------------------------------------------------------

/// Ungated 1NT responses and Stayman continuations
pub(super) fn base() -> Package {
    Package {
        name: "one-nt-base",
        gate: || true,
        entries: || {
            let mut entries = rows_of(Pattern::node("P* 1NT (P)"), notrump_responses());

            // Stayman answers and transfer completions.  The uncontested table
            // folds in the opt-in max-showing overlays.
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P)"),
                stayman_answers_uncontested(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♦ (P)"),
                complete_transfer(Suit::Hearts),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♥ (P)"),
                complete_transfer(Suit::Spades),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 4NT (P)"),
                quantitative_answer(17),
            ));

            // Responder's rebid after opener shows a major, and opener's reply
            // to the artificial 3OM slam try.
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♥ (P)"),
                stayman_major_rebid(Suit::Hearts),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♠ (P)"),
                stayman_major_rebid(Suit::Spades),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♥ (P) 3♠ (P)"),
                stayman_slam_try_answer(Suit::Hearts),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♠ (P) 3♥ (P)"),
                stayman_slam_try_answer(Suit::Spades),
            ));

            // Responder's rebid after opener denies a major, and opener's
            // Smolen completion in responder's five-card major.
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♦ (P)"),
                stayman_no_major_rebid(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♦ (P) 3♥ (P)"),
                smolen_completion(Suit::Spades),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♦ (P) 3♠ (P)"),
                smolen_completion(Suit::Hearts),
            ));

            // Opener accepts or declines responder's invitations.
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♥ (P) 3♥ (P)"),
                accept_major_invitation(Suit::Hearts),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♠ (P) 3♠ (P)"),
                accept_major_invitation(Suit::Spades),
            ));
            entries.extend(expand(
                "P* 1NT (P) 2♣ (P) 2x (P) 2NT (P)",
                |_| true,
                |_| accept_invitation(Bid::new(3, Strain::Notrump)),
            ));

            // Opener's quantitative accept after a no-fit revert to 4NT.
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♥ (P) 4NT (P)"),
                quantitative_answer(17),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♠ (P) 4NT (P)"),
                quantitative_answer(17),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♦ (P) 4NT (P)"),
                quantitative_answer(17),
            ));

            entries
        },
    }
}

/// Stayman cue-bid continuations and their RKCB subtrees
pub(super) fn cue() -> Package {
    Package {
        name: "stayman-cue-continuation",
        gate: stayman_cue_continuation,
        entries: || {
            let two_h = call(2, Strain::Hearts);
            let two_s = call(2, Strain::Spades);
            let three_h = call(3, Strain::Hearts);
            let three_s = call(3, Strain::Spades);
            let mut entries = Vec::new();

            for (answer, three_om, major) in [
                (two_h, three_s, Suit::Hearts),
                (two_s, three_h, Suit::Spades),
            ] {
                for cue_suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
                    if Strain::from(cue_suit) >= Strain::from(major) {
                        continue;
                    }
                    let path = format!(
                        "P* 1NT (P) 2♣ (P) {answer} (P) {three_om} (P) {} (P)",
                        call(4, Strain::from(cue_suit)),
                    );
                    entries.extend(rows_of(Pattern::node(&path), stayman_cue_rebid(major)));
                    entries.extend(slam::rkcb_rows(&path, major));
                }
            }

            entries
        },
    }
}

/// Stayman minor slam tries and their RKCB subtrees
pub(super) fn minor_slam() -> Package {
    Package {
        name: "stayman-minor-slam-try",
        gate: stayman_minor_slam_try,
        entries: || {
            let two_d = call(2, Strain::Diamonds);
            let two_h = call(2, Strain::Hearts);
            let two_s = call(2, Strain::Spades);
            let three_c = call(3, Strain::Clubs);
            let three_d = call(3, Strain::Diamonds);
            let mut entries = Vec::new();

            for answer in [two_h, two_s, two_d] {
                for (three_m, minor) in [(three_c, Suit::Clubs), (three_d, Suit::Diamonds)] {
                    let prefix = format!("P* 1NT (P) 2♣ (P) {answer} (P) {three_m} (P)");
                    entries.extend(rows_of(Pattern::node(&prefix), stayman_minor_answer(minor)));

                    let path = format!("{prefix} {} (P)", call(4, Strain::from(minor)));
                    entries.extend(rows_of(Pattern::node(&path), stayman_minor_slam_rkcb()));
                    entries.extend(slam::rkcb_rows(&path, minor));
                }
            }

            entries
        },
    }
}

/// Crawling Stayman pass-or-correct continuation
pub(super) fn crawling() -> Package {
    Package {
        name: "crawling-stayman",
        gate: crawling_stayman,
        entries: || {
            rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♦ (P) 2♥ (P)"),
                answer_crawling_stayman(),
            )
        },
    }
}

/// Invitational five-card-major continuations after Stayman and transfers
pub(super) fn invitational_majors() -> Package {
    Package {
        name: "invitational-five-card-majors",
        gate: invitational_5card_majors,
        entries: || {
            let mut entries = rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♦ (P) 2♠ (P)"),
                answer_inv_5card_spades(),
            );
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♥ (P) 2♠ (P)"),
                answer_inv_5card_both(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♥ (P) 2♠ (P) 3♥ (P)"),
                inv_5card_raise(Strain::Hearts),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2♥ (P) 2♠ (P) 3♠ (P)"),
                inv_5card_raise(Strain::Spades),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♦ (P) 2♥ (P) 2♠ (P)"),
                answer_transfer_heart_single(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♦ (P) 2♥ (P) 2NT (P)"),
                answer_transfer_heart_spade(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♥ (P) 2♠ (P) 2NT (P)"),
                answer_transfer_spade_single(),
            ));
            entries
        },
    }
}

/// Responder's chained rebids after transferring to hearts
pub(super) fn heart_transfer_rebids() -> Package {
    Package {
        name: "heart-transfer-rebids",
        gate: heart_transfer_rebid_active,
        entries: || {
            rows_of(
                Pattern::node("P* 1NT (P) 2♦ (P) 2♥ (P)"),
                heart_transfer_rebid_table(),
            )
        },
    }
}

/// Responder's chained rebids after transferring to spades
pub(super) fn spade_transfer_rebids() -> Package {
    Package {
        name: "spade-transfer-rebids",
        gate: spade_transfer_rebid_active,
        entries: || {
            rows_of(
                Pattern::node("P* 1NT (P) 2♥ (P) 2♠ (P)"),
                spade_transfer_rebid_table(),
            )
        },
    }
}

/// Opener's heart-agreeing transfer slam-try answer and RKCB subtree
pub(super) fn heart_transfer_slam_try() -> Package {
    Package {
        name: "heart-transfer-slam-try",
        gate: heart_transfer_slam_try_active,
        entries: || {
            let path = "P* 1NT (P) 2♦ (P) 2♥ (P) 3♠ (P)".to_owned();
            let mut entries = rows_of(Pattern::node(&path), transfer_slam_try_answer(Suit::Hearts));
            entries.extend(slam::rkcb_rows(&path, Suit::Hearts));
            entries
        },
    }
}

/// Opener's spade-agreeing transfer slam-try answer and RKCB subtree
pub(super) fn spade_transfer_slam_try() -> Package {
    Package {
        name: "spade-transfer-slam-try",
        gate: spade_transfer_slam_try_active,
        entries: || {
            let path = "P* 1NT (P) 2♥ (P) 2♠ (P) 3♥ (P)".to_owned();
            let mut entries = rows_of(Pattern::node(&path), transfer_slam_try_answer(Suit::Spades));
            entries.extend(slam::rkcb_rows(&path, Suit::Spades));
            entries
        },
    }
}

/// Game-forcing spade-transfer continuations and their RKCB subtrees
pub(super) fn spade_transfer_game_force() -> Package {
    Package {
        name: "spade-transfer-game-force",
        gate: transfer_gf_majors,
        entries: || {
            let mut entries = rows_of(
                Pattern::node("P* 1NT (P) 2♥ (P) 2♠ (P) 4NT (P)"),
                gf_quant_answer(Suit::Spades),
            );
            entries.extend(expand(
                "P* 1NT (P) 2♥ (P) 2♠ (P) 3m (P)",
                |_| true,
                |_| gf_minor_answer(Suit::Spades),
            ));
            for short in [Strain::Clubs, Strain::Diamonds, Strain::Hearts] {
                let path = format!("P* 1NT (P) 2♥ (P) 2♠ (P) {} (P)", call(4, short),);
                entries.extend(rows_of(
                    Pattern::node(&path),
                    gf_splinter_answer(Suit::Spades),
                ));
                entries.extend(slam::rkcb_rows(&path, Suit::Spades));
            }
            entries
        },
    }
}

/// Game-forcing heart-transfer continuations and their RKCB subtrees
pub(super) fn heart_transfer_game_force() -> Package {
    Package {
        name: "heart-transfer-game-force",
        gate: transfer_gf_hearts,
        entries: || {
            let mut entries = rows_of(
                Pattern::node("P* 1NT (P) 2♦ (P) 2♥ (P) 4NT (P)"),
                gf_quant_answer(Suit::Hearts),
            );
            entries.extend(expand(
                "P* 1NT (P) 2♦ (P) 2♥ (P) 3m (P)",
                |_| true,
                |_| gf_minor_answer(Suit::Hearts),
            ));
            for splinter in [
                call(3, Strain::Spades),
                call(4, Strain::Clubs),
                call(4, Strain::Diamonds),
            ] {
                let path = format!("P* 1NT (P) 2♦ (P) 2♥ (P) {splinter} (P)");
                entries.extend(rows_of(
                    Pattern::node(&path),
                    gf_splinter_answer(Suit::Hearts),
                ));
                entries.extend(slam::rkcb_rows(&path, Suit::Hearts));
            }
            entries
        },
    }
}

/// Opener's accept-or-decline tables for six-card-major invitations
pub(super) fn sixcard_invite() -> Package {
    Package {
        name: "six-card-major-invite",
        gate: sixcard_invite_active,
        entries: || {
            let mut entries = rows_of(
                Pattern::node("P* 1NT (P) 2♦ (P) 2♥ (P) 3♥ (P)"),
                accept_sixcard_invitation(Suit::Hearts),
            );
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♥ (P) 2♠ (P) 3♠ (P)"),
                accept_sixcard_invitation(Suit::Spades),
            ));
            entries
        },
    }
}

/// Stayman maximum showing both four-card majors, with a right-siding relay
pub(super) fn both_majors_relay() -> Package {
    Package {
        name: "stayman-both-majors-relay",
        gate: stayman_both_majors,
        entries: || {
            let mut entries = rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2NT (P)"),
                both_majors_max_responder(),
            );
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2NT (P) 3♣ (P)"),
                both_majors_relay_complete(Suit::Hearts),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2NT (P) 3♦ (P)"),
                both_majors_relay_complete(Suit::Spades),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2NT (P) 3♣ (P) 3♥ (P)"),
                both_majors_relay_placement(Suit::Hearts),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♣ (P) 2NT (P) 3♦ (P) 3♠ (P)"),
                both_majors_relay_placement(Suit::Spades),
            ));
            entries
        },
    }
}

/// Stayman maximum showing a five-card major
pub(super) fn five_card_max() -> Package {
    Package {
        name: "stayman-five-card-max",
        gate: stayman_5card_max,
        entries: || {
            expand(
                "P* 1NT (P) 2♣ (P) 3M (P)",
                |_| true,
                |b| five_card_max_rebid(b.suit('M')),
            )
        },
    }
}

/// Puppet Stayman responses and continuations after the 1NT–3♣ ask
pub(super) fn puppet() -> Package {
    Package {
        name: "puppet-stayman",
        gate: puppet_scheme,
        entries: || {
            let mut entries = rows_of(Pattern::node("P* 1NT (P) 3♣ (P)"), puppet_answers());
            entries.extend(expand(
                "P* 1NT (P) 3♣ (P) 3M (P)",
                |_| true,
                |b| puppet_major_rebid(b.suit('M')),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 3♣ (P) 3♦ (P)"),
                puppet_deny_rebid(),
            ));

            // The shorter-major bid shows four cards in the other major, so
            // these derived-suit completions stay literal.
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 3♣ (P) 3♦ (P) 3♥ (P)"),
                puppet_smolen_completion(Suit::Spades),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 3♣ (P) 3♦ (P) 3♠ (P)"),
                puppet_smolen_completion(Suit::Hearts),
            ));
            entries
        },
    }
}

/// European 1NT–3♣ diamond transfer and responder's game decision
pub(super) fn european_three_club() -> Package {
    Package {
        name: "european-three-club",
        gate: european_scheme,
        entries: || {
            let mut entries = rows_of(
                Pattern::node("P* 1NT (P) 3♣ (P)"),
                european_three_club_answer(),
            );
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 3♣ (P) 3♦ (P)"),
                diamond_transfer_game(8),
            ));
            entries
        },
    }
}

/// Both-majors 1NT–3♦ answer and responder's decision over a minimum
pub(super) fn both_majors_three_diamond() -> Package {
    Package {
        name: "both-majors-three-diamond",
        gate: || true,
        entries: || {
            let mut entries = rows_of(Pattern::node("P* 1NT (P) 3♦ (P)"), five_five_major_answer());
            entries.extend(expand(
                "P* 1NT (P) 3♦ (P) 3M (P)",
                |_| true,
                |b| five_five_min_rebid(b.suit('M')),
            ));
            entries
        },
    }
}

/// Opener's game placement after responder's opt-in 1NT–3M splinter
pub(super) fn notrump_splinter() -> Package {
    Package {
        name: "nt-splinter",
        gate: nt_splinter,
        entries: || {
            expand(
                "P* 1NT (P) 3M (P)",
                |_| true,
                |b| nt_splinter_answer(b.suit('M')),
            )
        },
    }
}

/// South African Texas transfers, direct slam tries, and their RKCB subtrees
pub(super) fn texas_transfers() -> Package {
    Package {
        name: "texas-transfers",
        gate: || true,
        entries: || {
            let heart_slam = "P* 1NT (P) 4♥ (P)";
            let spade_slam = "P* 1NT (P) 4♠ (P)";
            let mut entries = rows_of(
                Pattern::node("P* 1NT (P) 4♣ (P)"),
                complete_texas(Suit::Hearts),
            );
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 4♦ (P)"),
                complete_texas(Suit::Spades),
            ));
            entries.extend(rows_of(Pattern::node(heart_slam), slam_try_answer()));
            entries.extend(rows_of(Pattern::node(spade_slam), slam_try_answer()));
            entries.extend(slam::rkcb_rows(heart_slam, Suit::Hearts));
            entries.extend(slam::rkcb_rows(spade_slam, Suit::Spades));
            entries
        },
    }
}

/// Responder's Texas slam drive and its RKCB subtrees
pub(super) fn texas_drive() -> Package {
    Package {
        name: "texas-slam-drive",
        gate: texas_slam_drive,
        entries: || {
            let heart_drive = "P* 1NT (P) 4♣ (P) 4♥ (P)";
            let spade_drive = "P* 1NT (P) 4♦ (P) 4♠ (P)";
            let mut entries = rows_of(Pattern::node(heart_drive), texas_slam_drive_rebid());
            entries.extend(rows_of(
                Pattern::node(spade_drive),
                texas_slam_drive_rebid(),
            ));
            entries.extend(slam::rkcb_rows(heart_drive, Suit::Hearts));
            entries.extend(slam::rkcb_rows(spade_drive, Suit::Spades));
            entries
        },
    }
}

/// Puppet-scheme 1NT–2NT diamond transfer and its continuations
pub(super) fn diamond_transfer() -> Package {
    Package {
        name: "diamond-transfer",
        gate: puppet_scheme,
        entries: || {
            let mut entries = rows_of(
                Pattern::node("P* 1NT (P) 2NT (P)"),
                diamond_transfer_answer(),
            );
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2NT (P) 3♦ (P)"),
                diamond_transfer_game(8),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2NT (P) 3♣ (P)"),
                diamond_transfer_correct(8),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2NT (P) 3♣ (P) 3♦ (P)"),
                pass_out(),
            ));
            entries
        },
    }
}

/// European balanced invitation through 1NT–2NT
pub(super) fn european_two_notrump() -> Package {
    Package {
        name: "european-two-notrump",
        gate: european_scheme,
        entries: || {
            rows_of(
                Pattern::node("P* 1NT (P) 2NT (P)"),
                european_two_nt_answer(),
            )
        },
    }
}

/// Puppet-scheme two-way 1NT–2♠ structure and club-splinter continuations
pub(super) fn two_spade_two_way() -> Package {
    Package {
        name: "two-spade-two-way",
        gate: puppet_scheme,
        entries: || {
            let mut entries = rows_of(Pattern::node("P* 1NT (P) 2♠ (P)"), two_spade_answer());
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♠ (P) 2NT (P)"),
                two_spade_over_min(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♠ (P) 3♣ (P)"),
                two_spade_over_max(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♠ (P) 2NT (P) 3♣ (P)"),
                pass_out(),
            ));
            entries.extend(expand(
                "P* 1NT (P) 2♠ (P) 2NT (P) 3x (P)",
                |b| b.suit('x') != Suit::Clubs,
                |b| pick_game_over_club_splinter(b.suit('x')),
            ));
            entries.extend(expand(
                "P* 1NT (P) 2♠ (P) 3♣ (P) 3x (P)",
                |b| b.suit('x') != Suit::Clubs,
                |b| pick_game_over_club_splinter(b.suit('x')),
            ));
            entries
        },
    }
}

/// European 1NT–2♠ club transfer and club-splinter continuations
pub(super) fn european_two_spade() -> Package {
    Package {
        name: "european-two-spade",
        gate: european_scheme,
        entries: || {
            let mut entries = rows_of(
                Pattern::node("P* 1NT (P) 2♠ (P)"),
                european_two_spade_answer(),
            );
            entries.extend(rows_of(
                Pattern::node("P* 1NT (P) 2♠ (P) 3♣ (P)"),
                european_two_spade_rebid(),
            ));
            entries.extend(expand(
                "P* 1NT (P) 2♠ (P) 3♣ (P) 3x (P)",
                |b| b.suit('x') != Suit::Clubs,
                |b| pick_game_over_club_splinter(b.suit('x')),
            ));
            entries
        },
    }
}

/// Responses and continuations shared by the three 2NT-strength sequences
pub(super) fn two_notrump_structure() -> Package {
    Package {
        name: "two-notrump-structure",
        gate: || true,
        entries: || {
            let two_nt = call(2, Strain::Notrump);
            let four_nt = call(4, Strain::Notrump);
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
            let mut entries = Vec::new();

            for (base, accept_hcp) in bases {
                let prefix = core::iter::once("P*".to_owned())
                    .chain(base.iter().map(|call| format!("{call} (P)")))
                    .collect::<Vec<_>>()
                    .join(" ");

                // Responses to the 2NT bid.
                entries.extend(rows_of(Pattern::node(&prefix), two_notrump_responses()));

                // Stayman answers and transfer completions at the three level.
                let extend = |tail: Call| format!("{prefix} {tail} (P)");
                entries.extend(rows_of(
                    Pattern::node(&extend(call(3, Strain::Clubs))),
                    stayman_answers_at_three(),
                ));
                entries.extend(rows_of(
                    Pattern::node(&extend(call(3, Strain::Diamonds))),
                    complete_transfer_at_three(Suit::Hearts),
                ));
                entries.extend(rows_of(
                    Pattern::node(&extend(call(3, Strain::Hearts))),
                    complete_transfer_at_three(Suit::Spades),
                ));

                // Quantitative 4NT answer.
                entries.extend(rows_of(
                    Pattern::node(&extend(four_nt)),
                    quantitative_answer(*accept_hcp),
                ));

                // Smolen after 3♣ Stayman when opener denies a major (3♦):
                // responder jumps to show 5–4 in the majors, opener completes
                // to game in the long one.
                let extend2 = |a: Call, b: Call| format!("{prefix} {a} (P) {b} (P)");
                let extend3 =
                    |a: Call, b: Call, c: Call| format!("{prefix} {a} (P) {b} (P) {c} (P)");
                let (three_c, three_d) = (call(3, Strain::Clubs), call(3, Strain::Diamonds));
                let (three_h, three_s) = (call(3, Strain::Hearts), call(3, Strain::Spades));
                entries.extend(rows_of(
                    Pattern::node(&extend2(three_c, three_d)),
                    smolen_at_three(),
                ));
                entries.extend(rows_of(
                    Pattern::node(&extend3(three_c, three_d, three_h)),
                    smolen_completion(Suit::Spades),
                ));
                entries.extend(rows_of(
                    Pattern::node(&extend3(three_c, three_d, three_s)),
                    smolen_completion(Suit::Hearts),
                ));
            }

            entries
        },
    }
}

/// Continuations after opener's 18–19 2NT rebid
pub(super) fn two_notrump_rebids() -> Package {
    Package {
        name: "two-notrump-rebids",
        gate: || true,
        entries: || {
            let one_nt = call(1, Strain::Notrump);
            let two_nt = call(2, Strain::Notrump);
            let four_nt = call(4, Strain::Notrump);
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
            let mut entries = Vec::new();

            for prefix in rebid_prefixes {
                let prefix = core::iter::once("P*".to_owned())
                    .chain(prefix.iter().map(|call| format!("{call} (P)")))
                    .collect::<Vec<_>>()
                    .join(" ");

                // Responder's action over opener's 2NT rebid.
                let two_nt_rebid = format!("{prefix} {two_nt} (P)");
                entries.extend(rows_of(
                    Pattern::node(&two_nt_rebid),
                    after_rebid_two_notrump(),
                ));

                // Opener's reply to the quantitative 4NT raise.
                let quantitative_raise = format!("{two_nt_rebid} {four_nt} (P)");
                entries.extend(rows_of(
                    Pattern::node(&quantitative_raise),
                    accept_quantitative_nineteen(),
                ));
            }

            entries
        },
    }
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
    compile_into(
        book,
        &[
            base(),
            cue(),
            minor_slam(),
            crawling(),
            invitational_majors(),
            heart_transfer_rebids(),
            spade_transfer_rebids(),
            heart_transfer_slam_try(),
            spade_transfer_slam_try(),
            spade_transfer_game_force(),
            heart_transfer_game_force(),
            sixcard_invite(),
            both_majors_relay(),
            five_card_max(),
            puppet(),
            european_three_club(),
            both_majors_three_diamond(),
            notrump_splinter(),
            texas_transfers(),
            texas_drive(),
            diamond_transfer(),
            european_two_notrump(),
            two_spade_two_way(),
            european_two_spade(),
        ],
    );
}

/// Register the 2NT-strength structure and the 18–19 2NT-rebid continuations
///
/// The half of the notrump book that an alternative 1NT-opening scheme would
/// keep unchanged — only [`register_one_nt`] varies.
pub(super) fn register_two_nt_and_rebids(book: &mut Trie) {
    compile_into(book, &[two_notrump_structure(), two_notrump_rebids()]);
}

#[cfg(test)]
mod tests;
