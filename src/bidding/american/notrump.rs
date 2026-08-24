//! Notrump response structures for the 2/1 game-forcing system
//!
//! This module is the **index**.  It holds what every notrump agreement shares
//! — the scheme tag ([`PUPPET`] / [`EUROPEAN`]), the per-call [`Alert`]
//! constants, the trunk table [`notrump_responses`], the base package, and
//! [`register`] — while each agreement lives in its own submodule:
//!
//! | Module | Agreement | Response weight |
//! | --- | --- | --- |
//! | [`stayman`], [`stayman_slam`], [`crawling_stayman`] | the `2♣` ask and what hangs off it | 150 |
//! | [`both_majors`], [`invitational_majors`] | the 5-4 / 5-5 major hands | 210 (`3♦`) |
//! | [`transfers`], [`transfer_gf`], [`transfer_slam`], [`sixcard_invitation`] | Jacoby `2♦`/`2♥` and its continuations | 200 |
//! | [`texas`] | the `4♦`/`4♥` game transfers | 250, 260 (direct `4M`) |
//! | [`splinter`], [`long_minor`] | `3♥`/`3♠` shortness, and the `3m` force | 170, 140 |
//! | [`minor_transfers`], [`puppet_stayman`], [`european`] | the two rival minor schemes | 130, 160 (Puppet `3♣`) |
//! | [`size_ask`] | the `2NT` size ask over a maximum | — (it is a pass) |
//! | [`two_notrump`] | the 2NT-strength structures and the 18–19 rebid | — |
//!
//! # The response weight ladder
//!
//! Every module authors into the one trunk table [`notrump_responses`], so which
//! bid a hand actually makes is decided by *weight*, not by module order.  The
//! whole ladder, highest first — a hand matching several rules takes the highest:
//!
//! | Weight | Response |
//! | --- | --- |
//! | 260 | direct `4♥`/`4♠` — the opener-decides slam try |
//! | 250 | Texas `4♣`/`4♦` — the six-card major game blast |
//! | 210 | both-majors `3♦` — 5-5, invitational+ |
//! | 200 | Jacoby `2♦`/`2♥` |
//! | 170 | `1NT - 3♥`/`3♠` splinter (opt-in) |
//! | 160 | Puppet Stayman `3♣` |
//! | 150 | Stayman `2♣` (garbage and Crawling reuse this) |
//! | 140 | long-minor `3NT` force (opt-in, measured a loss) |
//! | 130 | the minor scheme's `2♠`/`2NT`/`3♣` |
//! | 120 | quantitative `4NT` |
//! | 100 | natural `3NT` |
//! | 0 | pass |
//!
//! Reading the ladder top-down is how a new rule's placement gets argued: the
//! splinter sits at 170 precisely because it must outrank the `130` minor
//! transfers its `♣5-6` shape would otherwise take, while staying under Stayman.
//! Continuation tables (opener's answers, responder's rebids) have their own
//! local weights and do not interact with this ladder.
//!
//! The public surface is [`register`], called once by
//! [`american`][super::american] during system assembly.

use super::{COMPLETION, call, other_major, slam};
use crate::bidding::agreements::Agreements;
use crate::bidding::constraint::{
    Cons, Constraint, balanced, described, envelope_union_upgrade, equal_length, hcp, len,
    long_suit_box, longer_suit, point_count_on, points, pred, reads_as, stopper_in,
    support_point_count_in_on, support_points, top_honors,
};
use crate::bidding::inference::{EnvelopeUnion, Range};
use crate::bidding::instinct::net_break_even_gate;
use crate::bidding::rows::{Bindings, Package, Pattern, compile_into, expand, rows_of};
use crate::bidding::{Alert, Context, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Holding, Rank, Strain, Suit};

mod both_majors;
mod crawling_stayman;
mod european;
mod invitational_majors;
mod long_minor;
mod minor_transfers;
mod puppet_stayman;
mod sixcard_invitation;
mod size_ask;
mod splinter;
mod stayman;
mod stayman_slam;
mod texas;
mod transfer_gf;
mod transfer_slam;
mod transfers;
mod two_notrump;

use crawling_stayman::crawling_stayman_rule;
use european::european_minors;
use long_minor::long_minor_force_rule;
use minor_transfers::puppet_minors;
use size_ask::size_ask_eight_pass;
use splinter::nt_splinter_rules;
use stayman::{
    accept_invitation, accept_major_invitation, garbage_stayman_rule, stayman_answers_uncontested,
    stayman_major_rebid, stayman_no_major_rebid,
};
use stayman_slam::stayman_slam_try_answer;
use texas::texas_strength_gate;
use transfer_gf::{
    equal_majors, longer_major, major_splinter_reroute, not_major_splinter_slam, slam_55_reroute,
};
use two_notrump::quantitative_answer;

pub(super) use both_majors::{both_majors_relay, both_majors_three_diamond, five_card_max};
pub(super) use crawling_stayman::crawling;
pub(super) use european::{european_three_club, european_two_notrump, european_two_spade};
pub(super) use invitational_majors::invitational_majors;
pub(super) use minor_transfers::{diamond_transfer, two_spade_two_way};
pub(super) use puppet_stayman::puppet;
pub(super) use sixcard_invitation::sixcard_invite;
pub use size_ask::SizeAskEight;
pub(super) use splinter::notrump_splinter;
pub(super) use stayman::{smolen_at_three, smolen_completion, stayman_answers};
pub(super) use stayman_slam::{cue, minor_slam, slam_try_answer};
pub(super) use texas::{direct_4m_max, texas_drive, texas_transfers};
pub(super) use transfer_gf::{heart_transfer_game_force, spade_transfer_game_force};
pub(super) use transfer_slam::{heart_transfer_slam_try, spade_transfer_slam_try};
pub(super) use transfers::{complete_transfer, heart_transfer_rebids, spade_transfer_rebids};
pub(super) use two_notrump::{two_notrump_rebids, two_notrump_structure};

/// The **Puppet** 1NT minor scheme — the shipped default
///
/// `2♠` = clubs or a balanced invite, `2NT` = diamonds (transfer), `3♣` = Puppet
/// Stayman.  The variant-selecting [`Alert`] minting the convention (see the
/// `Alert` newtype doc); assign it to
/// [`notrump_minors`][field@crate::bidding::inference::ReadingProfile::notrump_minors].
pub const PUPPET: Alert = Alert("puppet");

/// The **European** 1NT minor scheme — opt-in, BBA's Atlantic style
///
/// `2♠` = clubs (transfer), `2NT` = a balanced invite / size ask, `3♣` = diamonds
/// (transfer); no Puppet Stayman.  The standard Polish Club / WJ and common
/// continental response set. Select with
/// [`notrump_minors`][field@crate::bidding::inference::ReadingProfile::notrump_minors].
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

/// Whether book construction uses the Puppet minor scheme
///
/// This is a function because declarative [`Package`] gates are bare function
/// pointers and cannot capture a local from [`register_one_nt`].
fn puppet_scheme(agreements: &Agreements) -> bool {
    agreements.decision.reading.notrump_minors == PUPPET
}

/// The anti-gate of [`puppet_scheme`], for the European packages
fn european_scheme(agreements: &Agreements) -> bool {
    !puppet_scheme(agreements)
}

/// Responses to our 1NT opening: Stayman, Jacoby transfers, the minor-suit
/// scheme, and notrump raises
///
/// Stayman (2♣) needs invitational+ values and a four-card major; Jacoby
/// transfers (2♦/2♥) a five-card major, any strength.  The quantitative 4NT
/// invites slam opposite a balanced 16–17 with no four-card major.
///
/// The minor-suit responses (`2♠`/`2NT`/`3♣`) come in two variants, both authored
/// here behind their [`Alert`] and gated to the active
/// [`notrump_minors`][field@crate::bidding::inference::ReadingProfile::notrump_minors]
/// field (default [`PUPPET`]): `puppet_minors` (`2♠` = clubs-or-invite, `2NT` = diamonds,
/// `3♣` = Puppet Stayman) and `european_minors` (`2♠` = clubs, `2NT` = balanced
/// invite, `3♣` = diamonds).
#[must_use]
pub fn notrump_responses(agreements: &Agreements) -> Rules {
    let dormant = dormant_minors(agreements);
    // Direct `4♥/4♠` is the opener-decides slam try; with the Texas slam-drive
    // reroute on it caps at the 15–16 invitational band (17+ Texas-transfers and
    // drives its own RKCB instead — see `notrump.texas_slam_drive`).
    let slam_try_max = direct_4m_max(agreements);
    // Jacoby transfers — any strength, except a game-forcing 5-4 in the majors
    // (its weak-only arm denies it): that hand keeps off the transfer and takes
    // the 2♣ Stayman/Smolen route, which right-sides game to the strong notrump.
    // A plain 5-3 still transfers.  Under the longer-major discipline (default;
    // see `notrump.transfer_longer_major`) a two-suiter (both majors 5+) always
    // transfers to the LONGER major, and equal lengths split by strength: weak
    // → hearts (safety), invitational / minimum game force → the both-majors
    // 3♦, slam try → spades (the `1NT - 2♥ - 2♠ - 3♥` structure).  2♦ (to hearts) is
    // UNCHANGED by the invitational-5-4 reroute — a 5♥4♠ invite keeps
    // transferring and shows the spades with a later 2NT/2♠.
    let prefer_longer = agreements.notrump.transfer_longer_major;
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
    let head = match (prefer_longer, agreements.notrump.invitational_5card_majors) {
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
                    |hand: Hand, context: &Context<'_>| {
                        let profile = context.decision_profile();
                        !profile.transfer_gf_majors
                            || usize::from(point_count_on(profile.reading.point_scale, hand)) <= 16
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
        // to capture the invitational 7-8 hands — see `notrump.texas_game_floor`) and
        // slam-invitational (15–18) to the direct slam try.
        .rule(
            Bid::new(4, Strain::Clubs),
            250,
            len(Suit::Hearts, 6..)
                & len(Suit::Spades, ..5)
                & texas_strength_gate(Suit::Hearts, agreements)
                & not_major_splinter_slam(Suit::Hearts),
        )
        .alert(TEXAS)
        .rule(
            Bid::new(4, Strain::Diamonds),
            250,
            len(Suit::Spades, 6..)
                & len(Suit::Hearts, ..5)
                & texas_strength_gate(Suit::Spades, agreements)
                & not_major_splinter_slam(Suit::Spades),
        )
        .alert(TEXAS)
        .rule(
            Bid::new(4, Strain::Hearts),
            260,
            len(Suit::Hearts, 6..)
                & len(Suit::Spades, ..5)
                & hcp(15..=slam_try_max)
                & not_major_splinter_slam(Suit::Hearts),
        )
        .alert(TEXAS)
        .rule(
            Bid::new(4, Strain::Spades),
            260,
            len(Suit::Spades, 6..)
                & len(Suit::Hearts, ..5)
                & hcp(15..=slam_try_max)
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
        .chain(long_minor_force_rule(agreements))
        // Pass 0-7, and also the flat 4-3-3-3 *eight*: a shape with no ruff and no
        // long suit is its high cards and nothing more, so it plays a level too high
        // opposite a 15-17.  A double-dummy probe (`examples/probe-uninvite-4333`,
        // 16M deals) prices passing over the `2♠` size-ask invite at +0.64 IMPs/board
        // for the whole class, rising to +1.08 for the pure-quack (no ace, no ten)
        // eight — even the ace-holding eights gain.  The *nine* still forces (3NT):
        // the same probe found blanket-inviting it loses −0.33.  The size-ask eight's
        // pass/invite split is knob-gated for re-measurement — see `size_ask_eight`.
        .chain(size_ask_eight_pass(agreements))
        // Splinter 3♥/3♠ (opt-in): shortness in the bid major, 2-3 in the other,
        // exactly four diamonds and 5-6 clubs.  See `nt_splinter_rules`.
        .chain(nt_splinter_rules(agreements))
        // Minor-suit responses (2♠/2NT/3♣): both schemes are authored here, each
        // alerted with its variant, and only the active one is gated in.  The gate
        // drops just the dormant minor scheme; every always-on alert (Stayman,
        // Jacoby, …) survives.  Default Puppet.
        .chain(puppet_minors(agreements))
        .chain(european_minors(agreements))
        // Garbage Stayman (opt-in): a weak 2♣ to escape 1NT.  Same STAYMAN alert,
        // so it survives the minor-scheme gate (which only drops dormant minors).
        .chain(garbage_stayman_rule(agreements))
        // Crawling Stayman (superset of garbage): 4-4 majors short in diamonds.
        .chain(crawling_stayman_rule(agreements))
        .gated_out(&[dormant])
}

/// The minor scheme *not* selected — the one [`notrump_responses`] gates out
fn dormant_minors(agreements: &Agreements) -> Alert {
    if agreements.decision.reading.notrump_minors == PUPPET {
        EUROPEAN
    } else {
        PUPPET
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

/// Ungated 1NT responses and Stayman continuations
pub(super) fn base() -> Package {
    Package {
        name: "one-nt-base",
        gate: |_| true,
        entries: |agreements| {
            let mut entries = rows_of(Pattern::node("P* 1NT -"), notrump_responses(agreements));

            // Stayman answers and transfer completions.  The uncontested table
            // folds in the opt-in max-showing overlays.
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ -"),
                stayman_answers_uncontested(agreements),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♦ -"),
                complete_transfer(Suit::Hearts, agreements),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♥ -"),
                complete_transfer(Suit::Spades, agreements),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 4NT -"),
                quantitative_answer(17),
            ));

            // Responder's rebid after opener shows a major, and opener's reply
            // to the artificial 3OM slam try.
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♥ -"),
                stayman_major_rebid(Suit::Hearts, agreements),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♠ -"),
                stayman_major_rebid(Suit::Spades, agreements),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♥ - 3♠ -"),
                stayman_slam_try_answer(Suit::Hearts),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♠ - 3♥ -"),
                stayman_slam_try_answer(Suit::Spades),
            ));

            // Responder's rebid after opener denies a major, and opener's
            // Smolen completion in responder's five-card major.
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♦ -"),
                stayman_no_major_rebid(agreements),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♦ - 3♥ -"),
                smolen_completion(Suit::Spades, agreements),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♦ - 3♠ -"),
                smolen_completion(Suit::Hearts, agreements),
            ));

            // Opener accepts or declines responder's invitations.
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♥ - 3♥ -"),
                accept_major_invitation(Suit::Hearts),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♠ - 3♠ -"),
                accept_major_invitation(Suit::Spades),
            ));
            entries.extend(expand(
                "P* 1NT - 2♣ - 2x - 2NT -",
                |_| true,
                |_| accept_invitation(Bid::new(3, Strain::Notrump)),
            ));

            // Opener's quantitative accept after a no-fit revert to 4NT.
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♥ - 4NT -"),
                quantitative_answer(17),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♠ - 4NT -"),
                quantitative_answer(17),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♦ - 4NT -"),
                quantitative_answer(17),
            ));

            entries
        },
    }
}

/// Register all notrump continuations into the constructive book
///
/// Registers the 1NT structure (Stayman, transfers, 4NT quantitative), the
/// 2NT-strength structure (3-level Stayman/transfers, 4NT invite) under three
/// base prefixes (direct 2NT opening and the two 2♣ - 2x - 2NT auctions), and
/// simple responses after opener's 18–19 2NT rebid.
pub(super) fn register(book: &mut Trie, agreements: &Agreements) {
    register_one_nt(book, agreements);
    register_two_nt_and_rebids(book, agreements);
}

/// Register the standard 1NT-opening response structure
///
/// Stayman 2♣, Jacoby transfers 2♦/2♥, notrump raises, and the quantitative
/// 4NT invite — the baseline 2/1 treatment.  Factored from the
/// 2NT-strength/18–19-rebid block ([`register_two_nt_and_rebids`]) so an
/// alternative 1NT scheme could replace just this part.
pub(super) fn register_one_nt(book: &mut Trie, agreements: &Agreements) {
    compile_into(
        book,
        agreements,
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
pub(super) fn register_two_nt_and_rebids(book: &mut Trie, agreements: &Agreements) {
    compile_into(
        book,
        agreements,
        &[two_notrump_structure(), two_notrump_rebids()],
    );
}

#[cfg(test)]
mod tests;
