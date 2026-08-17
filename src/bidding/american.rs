//! A 2/1 game-forcing bidding system
//!
//! [`american()`] assembles a
//! [`System`] for the Two-over-One Game Forcing system, the modern North
//! American standard: five-card majors, a strong 15–17 notrump, the strong
//! artificial 2♣, and — the defining feature — a new suit at the two level in
//! response to a one-of-a-major opening is **game forcing**.
//!
//! The system is authored entirely from the constraint vocabulary
//! ([`constraint`]), the [`Rules`] classifier, and
//! the role-aware books — the strictly uncontested core in a [`Constructive`]
//! book, [`competition()`][crate::bidding::american::competition] over our
//! openings in a [`Competitive`] book, and our actions
//! over their openings in a [`Defensive`] book; nothing here
//! is system infrastructure.
//!
//! # Conventions
//!
//! - **Openings**: 15–17 1NT (balanced, or a 5422 with a five-card minor),
//!   20–21 2NT, strong artificial 2♣ (22+), five-card majors (light in 3rd/4th
//!   seat), better minor, weak twos, three-level preempts.
//! - **Responses**: 2/1 game forces with full continuations to game and the
//!   slam-try level, forcing 1NT (with the three-card limit raise rebid),
//!   Jacoby 2NT with shortness/second-suit rebids, splinters, inverted
//!   minors, weak jump shifts.
//! - **The 2♣ structure**: 2♦ waiting, 2♥ double negative, natural positives;
//!   notrump rebids carry the 2NT machinery ("system on").
//! - **Notrump structures**: Stayman and Jacoby transfers at the two and
//!   three levels, quantitative 4NT at every notrump strength.
//! - **Weak twos**: Ogust 2NT, RONF raises, forcing new suits.
//! - **Slam**: RKCB 1430 with the 5NT king ask
//!   (`slam`) below every major-suit trump agreement.
//! - **Competition**: cue-bid (limit-plus) raises, preemptive jump raises,
//!   negative doubles, system-on over their double, support
//!   doubles/redoubles.
//! - **Defense**: overcalls, takeout doubles, 1NT overcall, Michaels and the
//!   unusual 2NT with advances, advancing partner's takeout double, responsive
//!   doubles, defense to 1NT, and defense to weak twos (takeout double, natural
//!   2NT and suit overcalls).
//! - **Instinct floor**: both contested books carry the
//!   [`instinct`][crate::bidding::instinct()] ladder as a root fallback, so
//!   every contested auction gets a sane natural answer — in particular,
//!   partner's takeout double is never passed without a trump stack.
//!
//! Auctions no authored pass covers fall to the instinct floor, which answers
//! them with a sane natural call; see the crate changelog for what each
//! authored pass added (lebensohl, minor-suit keycard, reopening actions…).
//!
//! # Forcing by omission
//!
//! There is no "forcing" flag.  A bid is forcing when the *next* node for our
//! side carries no [`Pass`][Call::Pass] rule, so passing scores
//! [`f32::NEG_INFINITY`].  Responders keep a pass below their action threshold;
//! opener-rebid nodes after a response omit it entirely.
//!
//! # Weights
//!
//! Within one decision node the highest-weighted *satisfied* call wins (a
//! satisfied crisp constraint contributes `0`, so the logit is its weight).
//! Constraints are kept disjoint where practical; where calls can both apply,
//! the weights order them so the more descriptive bid wins.

use super::agreements::Agreements;
use super::common::{
    call, other_major, with_floor, with_floor_v5, with_floor_v6, with_instinct_floor,
};
use super::{Competitive, Constructive, Defensive, System};

/// The family tag of [`ReadingProfile::completion_alerts`][crate::bidding::ReadingProfile::completion_alerts]:
/// forced completions, transfer completions and conventional answers whose
/// face the walk must not read literally.  Attached with
/// [`Rules::alert_if`][crate::bidding::rules::Rules::alert_if] so the tag is
/// absent — not merely inert — when the knob is off; sites that predate the
/// family knob (xyz, Gladiator, weak-two-2NT, Lebensohl) keep their own slugs.
pub(in crate::bidding) const COMPLETION: crate::bidding::rules::Alert =
    crate::bidding::rules::Alert("completion");

pub(in crate::bidding) mod competition;
pub(in crate::bidding) mod defense;
pub(in crate::bidding) mod game_force;
mod nmf;
pub(in crate::bidding) mod notrump;
pub(in crate::bidding) mod openings;
mod raises;
pub(in crate::bidding) mod rebids;
pub(in crate::bidding) mod responses;
pub(in crate::bidding) mod slam;
mod strong_two;
mod weak_twos;
mod xyz;

pub use competition::{
    Competitive4333, DoubleStyle, FreeBidStyle, LebensohlStyle, MultiStopperAsk,
    NegativeDoubleShape, competition,
};
pub use defense::{
    DoubleShape, NotrumpDefense, TakeoutSupport, advance_double, defense_to_suit,
    defense_to_weak_two,
};
pub use notrump::{EUROPEAN, PUPPET, SizeAskEight, notrump_responses};
pub(crate) use openings::notrump_shape;
pub use openings::{NotrumpShape, WeakTwoEval, openings, openings_with};

pub use responses::{TwoOverOneGate, major_responses, minor_responses};

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

/// Build the basic 2/1 game-forcing system as one side's [`System`]
///
/// Bind it with [`bind`][System::bind] for a playable partnership, and seat
/// two systems with [`Table::of_systems`][super::Table::of_systems] for a full
/// table.
///
/// The contested books stand on
/// [`ConfiguredFloorV6`][crate::bidding::neural_floor::ConfiguredFloorV6] —
/// one artifact whose convention-regime input is both partnerships'
/// [`ConventionCard`][super::features::ConventionCard], **captured here, at build
/// time**, from the `agreements` value in the same expression that reads it for
/// [`american_book`].  That is what keeps regime and rules from disagreeing:
/// one value serves the card, the books, and — since the system carries it —
/// the classify-time half that [`System::bind`] pins.  Opponents are modeled as playing our own
/// agreements, matching every other undeclared-opposition default in the
/// crate; a genuinely mixed table wants [`american_with_config`], which also
/// remains the card-input v4 floor's entry point. The honest-reading v6 floor
/// became the default on its 2026-08-18 held-out and playing gates.
///
/// ```
/// use pons::american_default;
/// use pons::bidding::Bidder;
/// use contract_bridge::auction::{Call, RelativeVulnerability};
/// use contract_bridge::{Bid, Strain};
///
/// let partnership = american_default().bind();
/// let hand = "AQ32.K53.QJ4.A92".parse().unwrap(); // 16 HCP, balanced
/// let logits = partnership
///     .classify(hand, RelativeVulnerability::NONE, &[])
///     .expect("an opening decision");
/// let best = (&logits.0)
///     .into_iter()
///     .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
///     .map(|(call, _)| call)
///     .unwrap();
/// assert_eq!(best, Call::Bid(Bid::new(1, Strain::Notrump)));
/// ```
#[must_use]
pub fn american(agreements: &Agreements) -> System {
    with_floor_v6(
        book(agreements),
        super::features::CompactConfig::symmetric(&super::features::ConventionCard::capture(
            agreements, false,
        )),
        agreements,
    )
}

/// [`american`] against a **declared** opponent — the mixed table
///
/// The two arms of an A/B *play each other*, so at every table one side
/// relocates its asks and the other does not.  That asymmetric cell is in the
/// v4 corpus, and this is how a harness reaches it: build each arm's card from
/// its own knob state, then hand each side both.
///
/// `config` is taken verbatim while the **book comes from `agreements`**, so
/// make them match — a card claiming an agreement the rules do not play is a
/// misdisclosure to the net, and nothing checks it.  [`american`] cannot make
/// that mistake (it derives regime and book from one value in one expression);
/// this entry point can, which is the price of declaring an opponent that value
/// cannot describe.
///
/// Since the 2026-08-08 default-floor swap this is also the only entry point
/// that still builds the 2/1 book over the card-input **v4** floor
/// ([`ConfiguredFloorBba`][crate::bidding::neural_floor::ConfiguredFloorBba]);
/// `american_with_config(Config::symmetric(&american_card()))` reproduces the
/// pre-swap [`american`] exactly.  A declared opponent on the *shipped* floor
/// wants [`american_with_card`] instead — an arm built here and compared
/// against one built by [`american`] measures the two nets, not the declaration.
#[must_use]
pub fn american_with_config(agreements: &Agreements, config: super::features::Config) -> System {
    with_floor(book(agreements), config, agreements)
}

/// [`american`] against a **declared** opponent, on the shipped v6 floor
///
/// The compact-config twin of [`american_with_config`], and a strictly narrower seam: only
/// `theirs` is declared, while our own half is captured from the live knobs in
/// the same expression as the book.  So unlike the v4 entry point this *cannot*
/// misdisclose our own side — the mistake it warns about is unavailable here —
/// and the only judgement left to the caller is what the opposition plays.
///
/// Build `theirs` with [`ConventionCard::capture`][super::features::ConventionCard::capture]
/// under their armed knobs when they are a pons book, or with
/// [`ConventionCard::from_card`][super::features::ConventionCard::from_card] when they
/// are a foreign engine and a card is all there is.  At our own defaults the two
/// agree (`projection_agrees_with_capture_at_defaults`), so declaring an
/// undeviating pons opponent reproduces [`american`] board for board — the
/// inertness gate for this channel.
#[must_use]
pub fn american_with_card(
    agreements: &Agreements,
    theirs: &super::features::ConventionCard,
) -> System {
    with_floor_v6(
        book(agreements),
        super::features::CompactConfig::new(
            &super::features::ConventionCard::capture(agreements, false),
            theirs,
        ),
        agreements,
    )
}

/// [`american`] on the historical v5 floor.
///
/// Kept so harnesses and scripts written against `--our-floor american-v5`
/// can retain the old policy artifact after the Phase-5 v6 swap. Its retired
/// frozen reading view is intentionally unavailable.
#[must_use]
pub fn american_v5(agreements: &Agreements) -> System {
    with_floor_v5(
        book(agreements),
        super::features::CompactConfig::symmetric(&super::features::ConventionCard::capture(
            agreements, false,
        )),
        agreements,
    )
}

/// Alias of [`american`], whose v6 floor shipped on the Phase-5 gate.
#[must_use]
pub fn american_v6(agreements: &Agreements) -> System {
    american(agreements)
}

/// The 2/1 system with the deterministic **instinct** floor (the pre-BBA default)
///
/// Exactly [`american`] but for the floor: the learned
/// [`ConfiguredFloorBba`][crate::bidding::neural_floor::ConfiguredFloorBba]
/// gives way to
/// the deterministic [`instinct`][crate::bidding::instinct()] ladder.  This is the
/// fully-disclosable reference system — every off-book call is a described,
/// natural instinct call — and the fixed baseline the BBA-gap campaign anchors
/// on.  It is also the distillation teacher: the nets clone *this*, never the
/// net-floored [`american`].
#[must_use]
pub fn american_instinct(agreements: &Agreements) -> System {
    with_instinct_floor(book(agreements), agreements)
}

/// The 2/1 system with **no authored book** — every call comes from the floor
///
/// Exactly [`american`] but for the books: all three are empty, so every
/// auction falls straight through to the same floor wiring [`american`] uses —
/// [`ConfiguredFloorV6`][crate::bidding::neural_floor::ConfiguredFloorV6] on
/// the contested books, the deterministic [`instinct`][crate::bidding::instinct()]
/// ladder on the constructive one.  The ablation handle that prices the
/// authored book: `american` − `american_floor` is what [`american_book`] is
/// worth.
///
/// The floor takes the **same** agreements [`american`] would, even though there
/// is no book behind it to play them: the ablation isolates the book only if the
/// floor's inputs are identical on both arms — which means this function has to
/// follow [`american`] onto every future floor, in the same commit.  It did not
/// follow the 2026-08-08 v5 swap, and for one commit `scripts/ab-book-value.sh`
/// priced the book plus a whole net swap (+0.0353 plain DD per board, 3.5× the
/// run's own CI).
///
/// Note it prices the book's *total* contribution.  An empty book also stops
/// projecting authored constraints into
/// [`Inferences`][crate::bidding::inference::Inferences], so the net's
/// inference block collapses to unknown — the measured gap is the
/// book as authored calls **and** as disclosure, not the calls alone.
#[must_use]
pub fn american_floor(agreements: &Agreements) -> System {
    with_floor_v6(
        System::new(
            Constructive::new(),
            Competitive::new(),
            Defensive::new(),
            *agreements,
        ),
        super::features::CompactConfig::symmetric(&super::features::ConventionCard::capture(
            agreements, false,
        )),
        agreements,
    )
}

/// Build the 2/1 system as the **authored books alone**, with no floor
///
/// The book half of [`american`], and the ablation handle for measuring the
/// floor: a driver seating this system passes whenever the books run out — the
/// pre-floor behavior, including passing partner's takeout double on a
/// worthless hand.  [`american`] is exactly this system with the BBA-distilled
/// net attached to both contested books, and [`american_floor`] is the
/// complementary ablation (the floor alone, with no book at all); see the
/// `instinct-floor` example for an A/B match.
///
/// The 1NT [`NotrumpShape`] follows [`OpeningKnobs::notrump_shape`][crate::bidding::agreements::OpeningKnobs::notrump_shape] (default
/// [`NotrumpShape::Wide6322`] — a 5422 or 6322 with a long minor also opens
/// 1NT).
#[must_use]
pub fn american_book(agreements: &Agreements) -> System {
    book(agreements)
}

/// [`american`] on the shipped agreements
///
/// The `_default()` twin exists because [`Agreements`] implements [`Default`];
/// it is that call spelled once, not a second way to configure the system.
#[must_use]
pub fn american_default() -> System {
    american(&Agreements::default())
}

/// [`american_book`] on the shipped agreements — see [`american_default`]
#[must_use]
pub fn american_book_default() -> System {
    american_book(&Agreements::default())
}

/// [`american_instinct`] on the shipped agreements — see [`american_default`]
#[must_use]
pub fn american_instinct_default() -> System {
    american_instinct(&Agreements::default())
}

/// [`american_floor`] on the shipped agreements — see [`american_default`]
#[must_use]
pub fn american_floor_default() -> System {
    american_floor(&Agreements::default())
}

/// [`american_book`] on an explicit capture
///
/// The floor is built from the same value, so a book and the ladder under it
/// can never come from two different reads of the knobs.
pub(in crate::bidding) fn book(agreements: &Agreements) -> System {
    let agreements = *agreements;
    let mut c = Constructive::new();

    openings::register(&mut c, &agreements);
    responses::register(&mut c, &agreements);
    notrump::register(&mut c, &agreements);
    rebids::register(&mut c, &agreements);
    xyz::register(&mut c, &agreements);
    game_force::register(&mut c, &agreements);
    raises::register(&mut c, &agreements);
    strong_two::register(&mut c, &agreements);
    weak_twos::register(&mut c, &agreements);

    System::new(
        c,
        competition::competition(&agreements),
        defense::defensive(&agreements),
        agreements,
    )
}

#[cfg(test)]
mod tests;
