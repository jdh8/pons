//! Constraint vocabulary for hand classification
//!
//! A [`Constraint`] maps a hand (with its auction [`Context`]) to a logit
//! contribution.  Crisp predicates are the special case returning `0.0` when
//! satisfied and [`f32::NEG_INFINITY`] when violated; fuzzy evaluators can
//! return any other contribution without changing the trait.
//!
//! Constraints compose with operators on the [`Cons`] wrapper that all
//! primitives return:
//!
//! - `a & b` sums contributions (logical AND for crisp constraints,
//!   independent evidence for fuzzy ones),
//! - `a | b` takes the maximum (logical OR),
//! - `!a` is the crisp flip (any finite contribution counts as satisfied).
//!
//! Context-relative primitives such as [`support`] and
//! [`stopper_in_their_suits`] are the generalization mechanism of the crate:
//! one rule written with them applies to every auction whose context fits,
//! instead of one trie path at a time.
//!
//! ```
//! use pons::bidding::Context;
//! use pons::bidding::constraint::{Constraint, balanced, hcp};
//! use contract_bridge::auction::RelativeVulnerability;
//!
//! let strong_notrump = hcp(15..=17) & balanced();
//! let hand = "AQ32.K53.QJ4.A92".parse().unwrap();
//! let context = Context::new(RelativeVulnerability::NONE, &[]);
//! assert_eq!(strong_notrump.eval(hand, &context), 0.0);
//! ```

use super::context::Context;
use super::inference::{Envelope, EnvelopeUnion, Range, ReadingProfile};
use contract_bridge::auction::Call;
use contract_bridge::eval::{self, HandEvaluator, SimpleEvaluator};
use contract_bridge::{Hand, Holding, Level, Rank, Strain, Suit};
use core::cell::Cell;
use core::fmt;
use core::ops::{BitAnd, BitOr, Bound, Not, RangeBounds};
use std::borrow::Cow;

/// Runtime facts a [`Constraint`] may consult.
///
/// The mask is deliberately conservative: downstream constraints that do not
/// override [`Constraint::dependencies`] or
/// [`Constraint::projection_dependencies`] report [`Self::ALL`].  Compilers
/// may use a narrower mask as an optimization hint, but must never use it to
/// change evaluation order or semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstraintDependencies(u8);

impl ConstraintDependencies {
    /// No runtime dependencies.
    pub const NONE: Self = Self(0);
    /// The bidder's hand.
    pub const HAND: Self = Self(1 << 0);
    /// Auction, vulnerability, seat, or another direct [`Context`] fact.
    pub const CONTEXT: Self = Self(1 << 1);
    /// Derived auction [`Inferences`][super::inference::Inferences].
    pub const INFERENCES: Self = Self(1 << 2);
    /// Cached trick estimates.
    pub const TRICKS: Self = Self(1 << 3);
    /// Thread-local bidding/reading profile state.
    pub const PROFILE: Self = Self(1 << 4);
    /// Conservative default for an opaque constraint.
    pub const ALL: Self = Self(
        Self::HAND.0 | Self::CONTEXT.0 | Self::INFERENCES.0 | Self::TRICKS.0 | Self::PROFILE.0,
    );

    /// Combine two dependency masks.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether this mask contains any bit in `other`.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Whether a projection can be frozen for one captured profile.
    ///
    /// [`Self::PROFILE`] is allowed: a compiled projection records the active
    /// reading profile and falls back to the virtual method when it changes.
    /// Hand/context/inference/trick dependencies require live evaluation.
    #[must_use]
    pub const fn projection_context_independent(self) -> bool {
        !self.intersects(
            Self::HAND
                .union(Self::CONTEXT)
                .union(Self::INFERENCES)
                .union(Self::TRICKS),
        )
    }
}

impl BitOr for ConstraintDependencies {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

/// The four independent reading folds a constraint exposes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ProjectionKind {
    /// [`Constraint::project`]
    Forward,
    /// [`Constraint::project_band`]
    Band,
    /// [`Constraint::project_complement`]
    Complement,
    /// [`Constraint::announce`]
    Announcement,
}

/// Dependency masks for each projection fold.
///
/// The folds are separate because wrappers such as [`announced`] deliberately
/// source evaluation/projection from one constraint and disclosure from
/// another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectionDependencies {
    forward: ConstraintDependencies,
    band: ConstraintDependencies,
    complement: ConstraintDependencies,
    announcement: ConstraintDependencies,
    pure: bool,
}

impl ProjectionDependencies {
    /// Build four independently described, observationally pure folds.
    ///
    /// Calling this constructor is the purity promise which permits compiled
    /// readers to freeze or omit an unused fold. Stateful implementations
    /// should return [`Self::OPAQUE`] instead.
    #[must_use]
    pub const fn new(
        forward: ConstraintDependencies,
        band: ConstraintDependencies,
        complement: ConstraintDependencies,
        announcement: ConstraintDependencies,
    ) -> Self {
        Self {
            forward,
            band,
            complement,
            announcement,
            pure: true,
        }
    }

    /// Use one mask for every pure fold.
    #[must_use]
    pub const fn all(mask: ConstraintDependencies) -> Self {
        Self::new(mask, mask, mask, mask)
    }

    /// Conservative dependencies for an opaque constraint.
    pub const OPAQUE: Self = Self {
        forward: ConstraintDependencies::ALL,
        band: ConstraintDependencies::ALL,
        complement: ConstraintDependencies::ALL,
        announcement: ConstraintDependencies::ALL,
        pure: false,
    };

    /// Read one fold's mask.
    #[must_use]
    pub const fn get(self, kind: ProjectionKind) -> ConstraintDependencies {
        match kind {
            ProjectionKind::Forward => self.forward,
            ProjectionKind::Band => self.band,
            ProjectionKind::Complement => self.complement,
            ProjectionKind::Announcement => self.announcement,
        }
    }

    /// Whether the four folds are safe to freeze or omit when unused.
    #[must_use]
    pub const fn is_pure(self) -> bool {
        self.pure
    }

    const fn with_purity(mut self, pure: bool) -> Self {
        self.pure = pure;
        self
    }

    const fn remap(
        self,
        forward: ConstraintDependencies,
        band: ConstraintDependencies,
        complement: ConstraintDependencies,
        announcement: ConstraintDependencies,
    ) -> Self {
        Self::new(forward, band, complement, announcement).with_purity(self.pure)
    }

    /// Union corresponding folds.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self::new(
            self.forward.union(other.forward),
            self.band.union(other.band),
            self.complement.union(other.complement),
            self.announcement.union(other.announcement),
        )
        .with_purity(self.pure && other.pure)
    }
}

/// Trait for a logit contribution of a hand feature
///
/// Implementations must not return `f32::INFINITY`: combining `+∞` with the
/// `-∞` of a violated crisp constraint would produce a NaN.
pub trait Constraint: Send + Sync {
    /// Evaluate the constraint into a logit contribution
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32;

    /// Runtime facts evaluation may consult.
    ///
    /// The default is conservative so adding this hook cannot change the
    /// behavior of downstream implementations.
    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::ALL
    }

    /// Runtime facts each projection fold may consult.
    ///
    /// The default keeps all four folds dynamic.  Implementors should narrow a
    /// fold only when that promise remains true for every value of the type.
    /// A narrowed fold also promises that projection is observationally pure:
    /// the compiler may freeze it or omit a result which the active reading
    /// profile cannot consume. Stateful custom projections must retain the
    /// conservative default.
    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::OPAQUE
    }

    /// Render the constraint's meaning as a [`Description`]
    ///
    /// The inverse of evaluation: instead of scoring a hand, name what the
    /// constraint *requires*.  Primitives describe themselves (`hcp(15..=17)`
    /// → "15–17 HCP"); the combinators compose those descriptions.  The
    /// default is [`Description::Opaque`] — a bare [`pred`] closure carries no
    /// meaning it can recover, so it stays opaque until wrapped by
    /// [`described`].  Independent of the auction: a description is a property
    /// of the authored constraint, not of any one hand or [`Context`].
    fn describe(&self) -> Description {
        Description::Opaque
    }

    /// Project the constraint into the forward [`EnvelopeUnion`] it implies
    ///
    /// The third fold, beside [`eval`][Self::eval] and [`describe`][Self::describe]:
    /// where `eval` scores one hand and `describe` names the meaning, `project`
    /// turns the constraint into unions of the per-suit length and point ranges
    /// that every hand it accepts must fall within — the bidder's *forward*
    /// reading of an authored call, the dual of evaluating a known hand.  Sound by
    /// construction: a finite `eval(hand, context)` implies `hand` lies within
    /// `project(context)`.  The default asserts nothing
    /// ([`EnvelopeUnion::unknown`]), so an opaque predicate stays sound but loose
    /// until a length- or points-bearing primitive overrides it.
    fn project(&self, _context: &Context<'_>) -> EnvelopeUnion {
        EnvelopeUnion::unknown()
    }

    /// Project the constraint into its **two-sided** [`EnvelopeUnion`]
    ///
    /// The ceiling-carrying sibling of [`project`][Self::project]: `project`
    /// deliberately claims floors only for the point gauges (a made call is
    /// read by what it *promises*), while a **declined** call — the negative
    /// inference of a pass — is read by what the gate would have *allowed*,
    /// which needs the ceilings back.  Same soundness contract: a finite
    /// `eval(hand, context)` implies `hand` lies within the band.  The
    /// default reuses `project`, so every constraint whose projection is
    /// already two-sided ([`len`] and the suit-set combinators) or opaque
    /// stays correct; the point gauges and [`balanced`] override it, and
    /// `&`/`|` compose it tightly per arm.
    fn project_band(&self, context: &Context<'_>) -> EnvelopeUnion {
        self.project(context)
    }

    /// Project the **negation** of this constraint into an envelope union
    ///
    /// What [`Flip`] reads: the hands `!self` accepts are exactly the ones
    /// `self` rejects, so a gate that pins one axis to a half-open band pins
    /// the same axis to the other half.  `!len(♠, 4..)` is "at most three
    /// spades", which is a perfectly ordinary box — the information was there
    /// all along, and only the missing fold lost it.
    ///
    /// Two-sided by construction, so [`Flip`] uses it for
    /// [`project_band`][Self::project_band] as well.  The default asserts
    /// nothing, and so does every implementor whose complement is not soundly
    /// expressible as a finite envelope union.  A bounded atom such as
    /// `!hcp(13..=15)` does fit: its low and high halves become two boxes.
    /// Negation is pushed to the leaves before anything is complemented because
    /// `!⊤ = ⊥` is *tighter than the truth*; an off-axis atom like
    /// [`stopper_in`] therefore approximates to ⊤.
    fn project_complement(&self, _context: &Context<'_>) -> EnvelopeUnion {
        EnvelopeUnion::unknown()
    }

    /// Project the constraint into the envelope union its *agreement* announces
    ///
    /// The disclosure twin of [`project`][Self::project], and the two carry
    /// different contracts.  `project` is bound by soundness — a finite `eval`
    /// implies the hand lies inside it — because it is what
    /// [`sample_layouts`][super::sampler::sample_layouts] accepts against, and a
    /// reading tighter than the truth makes the sampler exclude hands we actually
    /// hold.  `announce` is bound by *disclosure*: it is the partnership
    /// agreement, what the call would be explained as at the table, which a
    /// judgment call may stretch below without announcing anything different.
    ///
    /// That is not a licence to be wrong, it is a licence to be *tight*: an
    /// agreement is calibrated to what the criterion does in the population, so
    /// it can name a floor a black-box criterion will occasionally reach past.
    /// [`project`][Self::project] cannot, which is why the evaluator net's gates
    /// publish ⊤ there and will keep doing so.
    ///
    /// Defaults to `project`, so every constraint announces exactly what it
    /// projects until [`announced`] splits the two.
    fn announce(&self, context: &Context<'_>) -> EnvelopeUnion {
        self.project(context)
    }
}

/// The complement of a band within `0..=cap`, as up to two disjoint halves
///
/// `4..` complements to the single low half `0..=3`; a two-sided band like
/// `4..=5` complements to a *union*, `0..=3` ∪ `6..=cap`, which one
/// [`Envelope`] box cannot hold — the caller folds the halves through
/// [`EnvelopeUnion::disjoin`], so knob-off they hull back to the full range
/// (exactly the legacy single-envelope reading) while knob-on both boxes
/// survive.  The full band
/// complements to no halves at all; the caller's fold falls back to ⊤, sound
/// for the truly-empty complement.
fn complement_halves(range: Range, cap: u8) -> impl Iterator<Item = Range> {
    let low = (range.min > 0).then(|| Range::new(0, range.min - 1));
    let high = (range.max < cap).then(|| Range::new(range.max + 1, cap));
    low.into_iter().chain(high)
}

/// Closures are natural constraints
impl<F> Constraint for F
where
    F: Fn(Hand, &Context<'_>) -> f32 + Send + Sync,
{
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        self(hand, context)
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        // Closure constraints use the trait's four vacuous default folds;
        // only their hand evaluation is opaque.
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }
}

/// Describe one box through the same nouns the atom constraints use
///
/// A natively authored box must stay visible to noun-sniffing consumers — the
/// `authored_calls_read_what_they_gate` ratchet reads axes off these rendered
/// atoms — so each non-full axis renders exactly as its atom constraint would:
/// lengths like [`len`] (`"5+ ♠"`), the gauges like [`hcp`]/[`points`]/
/// [`support_points`]/[`suit_hcp`] (`"13+ HCP"`, `"≤10 points"`,
/// `"13+ support points"`, `"5+ HCP in ♠"`).
fn describe_box(envelope: &Envelope) -> Description {
    /// One axis as an atom, `None` when the range is the full (vacuous) one.
    fn axis(range: Range, cap: u8, noun: &str) -> Option<Description> {
        match (range.min, range.max) {
            (0, max) if max >= cap => None,
            (min, max) if max >= cap => Some(describe_int_range(&(min..), noun)),
            (0, max) => Some(describe_int_range(&(..=max), noun)),
            (min, max) => Some(describe_int_range(&(min..=max), noun)),
        }
    }

    let mut parts = Vec::new();
    for suit in Suit::ASC {
        parts.extend(axis(
            envelope.length(suit),
            Range::FULL_LENGTH.max,
            &suit.to_string(),
        ));
    }
    let strength = &envelope.strength;
    parts.extend(axis(strength.hcp, Range::FULL_POINTS.max, "HCP"));
    parts.extend(axis(strength.points, Range::FULL_POINTS.max, "points"));
    let support = strength.support_points;
    if support.iter().all(|slot| *slot == support[0]) {
        // Uniform slots — one band about every fit renders as a scalar.
        parts.extend(axis(support[0], Range::FULL_POINTS.max, "support points"));
    } else {
        // Per-suit slots: name each narrowed trump.
        for suit in Suit::ASC {
            parts.extend(axis(
                support[suit as usize],
                Range::FULL_POINTS.max,
                &format!("support points in {suit}"),
            ));
        }
    }
    // Always per-suit — whole-hand HCP is a different axis, never a shortcut.
    for suit in Suit::ASC {
        parts.extend(axis(
            strength.suit_hcp[suit as usize],
            Range::FULL_SUIT_HCP.max,
            &format!("HCP in {suit}"),
        ));
    }
    match parts.len() {
        0 => Description::atom("any hand"),
        1 => parts.pop().unwrap_or_else(|| unreachable!()),
        _ => Description::All(parts),
    }
}

/// An [`Envelope`] box authored directly as a hand gate
///
/// The reading vocabulary doubles as authoring vocabulary: a box states its
/// per-suit lengths and per-gauge strength bands, and as a gate it enforces
/// exactly that — [`Envelope::accepts`], the strict membership test over
/// **all** gauges (lengths, `points`, raw HCP, `support_points`), ceilings
/// included.  The projection is therefore the identity, exact in both
/// directions: what the gate enforces is literally the box it stores, so the
/// pass reading gets true bands for free.
impl Constraint for Envelope {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        crisp(self.accepts(hand))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND | ConstraintDependencies::PROFILE
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        describe_box(self)
    }

    fn project(&self, _: &Context<'_>) -> EnvelopeUnion {
        EnvelopeUnion::from(*self)
    }
}

/// An [`EnvelopeUnion`] authored directly as a hand gate
///
/// The `Or`-wall cure at the authoring layer: a disjunctive meaning (a
/// two-suiter, a fit-split) is written as the union of the boxes it is, evals
/// as strict membership of **some** box ([`Envelope::accepts`] per box), and
/// projects as itself — through [`EnvelopeUnion::disjoin`], so the knob-off reading
/// hulls exactly as an equivalent `|`-composite would (byte-identical), while
/// knob-on keeps the boxes.
impl Constraint for EnvelopeUnion {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        crisp(self.boxes().iter().any(|envelope| envelope.accepts(hand)))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND | ConstraintDependencies::PROFILE
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::new(
            ConstraintDependencies::PROFILE,
            ConstraintDependencies::PROFILE,
            ConstraintDependencies::NONE,
            ConstraintDependencies::PROFILE,
        )
    }

    fn describe(&self) -> Description {
        let mut parts: Vec<_> = self.boxes().iter().map(describe_box).collect();
        match parts.len() {
            1 => parts.pop().unwrap_or_else(|| unreachable!()),
            _ => Description::Any(parts),
        }
    }

    fn project(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        self.boxes()
            .iter()
            .map(|&envelope| EnvelopeUnion::from(envelope))
            .reduce(|a, b| a.disjoin_with(b, profile))
            .unwrap_or_else(EnvelopeUnion::unknown)
    }
}

/// Composable wrapper around a [`Constraint`]
///
/// All primitive constraints in this module return this wrapper, which
/// provides the `&`, `|`, and `!` operators.
#[derive(Clone, Copy, Debug)]
pub struct Cons<T>(
    /// The wrapped constraint
    pub T,
);

impl<T: Constraint> Constraint for Cons<T> {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        self.0.eval(hand, context)
    }

    fn dependencies(&self) -> ConstraintDependencies {
        self.0.dependencies()
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        self.0.projection_dependencies()
    }

    fn describe(&self) -> Description {
        self.0.describe()
    }

    fn project(&self, context: &Context<'_>) -> EnvelopeUnion {
        self.0.project(context)
    }

    fn project_band(&self, context: &Context<'_>) -> EnvelopeUnion {
        self.0.project_band(context)
    }

    fn project_complement(&self, context: &Context<'_>) -> EnvelopeUnion {
        self.0.project_complement(context)
    }

    fn announce(&self, context: &Context<'_>) -> EnvelopeUnion {
        self.0.announce(context)
    }
}

/// Sum of two constraints, the logical AND for crisp constraints
#[derive(Clone, Copy, Debug)]
pub struct And<A, B>(A, B);

impl<A: Constraint, B: Constraint> Constraint for And<A, B> {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        self.0.eval(hand, context) + self.1.eval(hand, context)
    }

    fn dependencies(&self) -> ConstraintDependencies {
        self.0.dependencies() | self.1.dependencies()
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        let combined = self
            .0
            .projection_dependencies()
            .union(self.1.projection_dependencies());
        combined.remap(
            combined.get(ProjectionKind::Forward) | ConstraintDependencies::PROFILE,
            combined.get(ProjectionKind::Band) | ConstraintDependencies::PROFILE,
            combined.get(ProjectionKind::Complement) | ConstraintDependencies::PROFILE,
            combined.get(ProjectionKind::Announcement) | ConstraintDependencies::PROFILE,
        )
    }

    fn describe(&self) -> Description {
        self.0.describe().and(self.1.describe())
    }

    fn project(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        self.0
            .project(context)
            .intersect_owned(&self.1.project(context), profile)
    }

    fn project_band(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        self.0
            .project_band(context)
            .intersect_owned(&self.1.project_band(context), profile)
    }

    fn project_complement(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // De Morgan: `!(A & B)` = `!A | !B`, the disjunction of the arm
        // complements.  New precision (the default was ⊤), so the whole
        // reading sits behind the knob — knob-off stays ⊤, today's hull.
        if profile.envelope_union() {
            self.0
                .project_complement(context)
                .disjoin_with(self.1.project_complement(context), profile)
        } else {
            EnvelopeUnion::unknown()
        }
    }

    fn announce(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        self.0
            .announce(context)
            .intersect_owned(&self.1.announce(context), profile)
    }
}

/// Maximum of two constraints, the logical OR for crisp constraints
#[derive(Clone, Copy, Debug)]
pub struct Or<A, B>(A, B);

impl<A: Constraint, B: Constraint> Constraint for Or<A, B> {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        self.0.eval(hand, context).max(self.1.eval(hand, context))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        self.0.dependencies() | self.1.dependencies()
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        let combined = self
            .0
            .projection_dependencies()
            .union(self.1.projection_dependencies());
        combined.remap(
            combined.get(ProjectionKind::Forward) | ConstraintDependencies::PROFILE,
            combined.get(ProjectionKind::Band) | ConstraintDependencies::PROFILE,
            combined.get(ProjectionKind::Complement) | ConstraintDependencies::PROFILE,
            combined.get(ProjectionKind::Announcement) | ConstraintDependencies::PROFILE,
        )
    }

    fn describe(&self) -> Description {
        self.0.describe().or(self.1.describe())
    }

    fn project(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        self.0
            .project(context)
            .disjoin_with(self.1.project(context), profile)
    }

    fn project_band(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        self.0
            .project_band(context)
            .disjoin_with(self.1.project_band(context), profile)
    }

    fn project_complement(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // De Morgan: `!(A | B)` = `!A & !B` — an *intersection*, which always
        // tightens, so the whole reading sits behind the knob (knob-off stays
        // ⊤, today's hull; `EnvelopeUnion::intersect` never consults the knob itself).
        if profile.envelope_union() {
            self.0
                .project_complement(context)
                .intersect_owned(&self.1.project_complement(context), profile)
        } else {
            EnvelopeUnion::unknown()
        }
    }

    fn announce(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        self.0
            .announce(context)
            .disjoin_with(self.1.announce(context), profile)
    }
}

/// Crisp negation of a constraint
///
/// Any finite contribution counts as satisfied and flips to `-∞`; only `-∞`
/// flips to `0.0`.
#[derive(Clone, Copy, Debug)]
pub struct Flip<T>(T);

impl<T: Constraint> Constraint for Flip<T> {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        crisp(self.0.eval(hand, context) == f32::NEG_INFINITY)
    }

    fn dependencies(&self) -> ConstraintDependencies {
        self.0.dependencies()
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        let inner = self.0.projection_dependencies();
        let complement = inner.get(ProjectionKind::Complement);
        inner.remap(
            complement,
            complement,
            inner.get(ProjectionKind::Band) | ConstraintDependencies::PROFILE,
            complement,
        )
    }

    fn describe(&self) -> Description {
        self.0.describe().negate()
    }

    fn project(&self, context: &Context<'_>) -> EnvelopeUnion {
        self.0.project_complement(context)
    }

    fn project_complement(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // `!!A = A`: the double negation accepts exactly `A`'s hands, so the
        // sound (and tight) reading is `A`'s two-sided band.  New precision —
        // knob-gated; knob-off keeps today's ⊤.
        if profile.envelope_union() {
            self.0.project_band(context)
        } else {
            EnvelopeUnion::unknown()
        }
    }
}

impl<A, B> BitAnd<Cons<B>> for Cons<A> {
    type Output = Cons<And<A, B>>;

    fn bitand(self, rhs: Cons<B>) -> Self::Output {
        Cons(And(self.0, rhs.0))
    }
}

impl<A, B> BitOr<Cons<B>> for Cons<A> {
    type Output = Cons<Or<A, B>>;

    fn bitor(self, rhs: Cons<B>) -> Self::Output {
        Cons(Or(self.0, rhs.0))
    }
}

impl<A> Not for Cons<A> {
    type Output = Cons<Flip<A>>;

    fn not(self) -> Self::Output {
        Cons(Flip(self.0))
    }
}

/// Convert a boolean to a crisp logit
const fn crisp(condition: bool) -> f32 {
    if condition { 0.0 } else { f32::NEG_INFINITY }
}

/// A structured, human-readable description of a [`Constraint`]
///
/// The render side of the constraint DSL.  Where [`Constraint::eval`] scores a
/// hand, [`Constraint::describe`] returns one of these trees naming what the
/// constraint *means*, so an authored book can be printed as canonical English
/// instead of staying an opaque `eval`-only closure.  It is the inverse of the
/// planned English→`Constraint` authoring compiler, and the substrate that
/// makes the two directions round-trippable.
///
/// The tree mirrors the combinators: `&` builds [`All`][Self::All], `|` builds
/// [`Any`][Self::Any], `!` builds [`Not`][Self::Not].  [`Display`][fmt::Display]
/// renders it to prose.
///
/// ```
/// use pons::bidding::constraint::{Constraint, balanced, hcp, len};
/// use contract_bridge::Suit;
///
/// assert_eq!((hcp(15..=17) & balanced()).describe().to_string(), "15–17 HCP, and balanced");
/// assert_eq!(len(Suit::Spades, 5..).describe().to_string(), "5+ ♠");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Description {
    /// A leaf meaning, e.g. `"15–17 HCP"`
    Atom(Cow<'static, str>),
    /// A conjunction (from `&` / [`And`]): every part must hold
    All(Vec<Description>),
    /// A disjunction (from `|` / [`Or`]): any part may hold
    Any(Vec<Description>),
    /// A negation (from `!` / [`Flip`])
    Not(Box<Description>),
    /// An unreadable predicate — a bare [`pred`] that carries no label
    Opaque,
}

impl Description {
    /// A leaf description from any string
    fn atom(text: impl Into<Cow<'static, str>>) -> Self {
        Self::Atom(text.into())
    }

    /// Conjoin two descriptions, flattening nested [`All`][Self::All] so that
    /// `a & b & c` reads as one comma list rather than a nested tree.
    fn and(self, other: Self) -> Self {
        let mut parts = self.into_all_parts();
        parts.extend(other.into_all_parts());
        Self::All(parts)
    }

    /// Disjoin two descriptions, flattening nested [`Any`][Self::Any].
    fn or(self, other: Self) -> Self {
        let mut parts = self.into_any_parts();
        parts.extend(other.into_any_parts());
        Self::Any(parts)
    }

    /// Negate, cancelling a double negation.
    fn negate(self) -> Self {
        match self {
            Self::Not(inner) => *inner,
            other => Self::Not(Box::new(other)),
        }
    }

    fn into_all_parts(self) -> Vec<Self> {
        match self {
            Self::All(parts) => parts,
            other => vec![other],
        }
    }

    fn into_any_parts(self) -> Vec<Self> {
        match self {
            Self::Any(parts) => parts,
            other => vec![other],
        }
    }
}

/// Render one list member, parenthesizing a nested conjunction or disjunction
/// so a mixed tree stays unambiguous: `… and (seat 3, or seat 4)`.
fn write_member(f: &mut fmt::Formatter<'_>, member: &Description) -> fmt::Result {
    match member {
        Description::All(_) | Description::Any(_) => write!(f, "({member})"),
        _ => write!(f, "{member}"),
    }
}

/// Join `parts` into a prose list: `"a, b, {last_word} c"`, a single part bare.
fn write_list(f: &mut fmt::Formatter<'_>, parts: &[Description], last_word: &str) -> fmt::Result {
    match parts.split_last() {
        None => Ok(()),
        Some((last, [])) => write_member(f, last),
        Some((last, init)) => {
            for part in init {
                write_member(f, part)?;
                f.write_str(", ")?;
            }
            f.write_str(last_word)?;
            write_member(f, last)
        }
    }
}

impl fmt::Display for Description {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Atom(text) => f.write_str(text),
            Self::Opaque => f.write_str("(opaque condition)"),
            Self::Not(inner) => write!(f, "not ({inner})"),
            Self::All(parts) => write_list(f, parts, "and "),
            Self::Any(parts) => write_list(f, parts, "or "),
        }
    }
}

/// Integer widths that range descriptions normalize through
trait ToU64: Copy {
    fn to_u64(self) -> u64;
}

impl ToU64 for u8 {
    fn to_u64(self) -> u64 {
        u64::from(self)
    }
}

impl ToU64 for usize {
    fn to_u64(self) -> u64 {
        self as u64
    }
}

/// Render an integer [`RangeBounds`] as an [`Atom`][Description::Atom] with a
/// trailing `noun`: `"15–17 HCP"`, `"5+ ♠"`, `"exactly 6 ♠"`, `"≤10 HCP"`.
///
/// Bounds are normalized to inclusive integers, so the half-open `..11` reads
/// as `"≤10 HCP"` rather than exposing the exclusive endpoint.
fn describe_int_range<T: ToU64>(range: &impl RangeBounds<T>, noun: &str) -> Description {
    let lo = match range.start_bound() {
        Bound::Included(&x) => Some(x.to_u64()),
        Bound::Excluded(&x) => Some(x.to_u64() + 1),
        Bound::Unbounded => None,
    };
    let hi = match range.end_bound() {
        Bound::Included(&x) => Some(x.to_u64()),
        Bound::Excluded(&x) => Some(x.to_u64().saturating_sub(1)),
        Bound::Unbounded => None,
    };
    let text = match (lo, hi) {
        (Some(a), Some(b)) if a == b => format!("exactly {a} {noun}"),
        (Some(a), Some(b)) => format!("{a}–{b} {noun}"),
        (Some(a), None) => format!("{a}+ {noun}"),
        (None, Some(b)) => format!("≤{b} {noun}"),
        (None, None) => format!("any {noun}"),
    };
    Description::atom(text)
}

/// Render a floating-point [`RangeBounds`] as an [`Atom`][Description::Atom],
/// e.g. the half-open fifths band `15.0..18.0` → `"15.0–18.0 fifths"`.
///
/// Endpoints print to one decimal as written; the band is shown literally
/// rather than nudged to `"≤17.999"`.
fn describe_real_range(range: &impl RangeBounds<f64>, noun: &str) -> Description {
    let endpoint = |bound: Bound<&f64>| match bound {
        Bound::Included(&x) | Bound::Excluded(&x) => Some(x),
        Bound::Unbounded => None,
    };
    let lo = endpoint(range.start_bound());
    let hi = endpoint(range.end_bound());
    let text = match (lo, hi) {
        (Some(a), Some(b)) => format!("{a:.1}–{b:.1} {noun}"),
        (Some(a), None) => format!("{a:.1}+ {noun}"),
        (None, Some(b)) => format!("≤{b:.1} {noun}"),
        (None, None) => format!("any {noun}"),
    };
    Description::atom(text)
}

/// Crisp predicate over a hand and its context
///
/// This is the escape hatch for one-off conditions:
///
/// ```
/// use pons::bidding::Context;
/// use pons::bidding::constraint::pred;
/// use contract_bridge::{Hand, Suit};
///
/// let freak = pred(|hand: Hand, _: &Context| {
///     Suit::ASC.into_iter().any(|suit| hand[suit].len() >= 7)
/// });
/// ```
pub fn pred<F>(condition: F) -> Cons<impl Constraint + Clone>
where
    F: Fn(Hand, &Context<'_>) -> bool + Clone + Send + Sync,
{
    Cons(move |hand: Hand, context: &Context<'_>| crisp(condition(hand, context)))
}

/// A labeled crisp predicate
///
/// Carries its own meaning so it describes to `label` instead of the
/// [`Opaque`][Description::Opaque] a bare closure gives.
#[derive(Clone)]
struct Described<F> {
    condition: F,
    label: Cow<'static, str>,
}

impl<F> Constraint for Described<F>
where
    F: Fn(Hand, &Context<'_>) -> bool + Send + Sync,
{
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        crisp((self.condition)(hand, context))
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        // Like `pred`, this evaluator inherits the vacuous default folds.
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        Description::atom(self.label.clone())
    }
}

/// A crisp predicate that knows its own meaning (a labeled [`pred`])
///
/// The same escape hatch as [`pred`], but the one-off condition carries a
/// `label` so it renders to that prose rather than
/// [`Opaque`][Description::Opaque].  Use it on bespoke book predicates the
/// vocabulary has no primitive for:
///
/// ```
/// use pons::bidding::constraint::{Constraint, described};
/// use contract_bridge::Suit;
///
/// let prefers_diamonds = described("prefers diamonds", |hand, _| {
///     hand[Suit::Diamonds].len() >= hand[Suit::Clubs].len()
/// });
/// assert_eq!(prefers_diamonds.describe().to_string(), "prefers diamonds");
/// ```
pub fn described<F>(
    label: impl Into<Cow<'static, str>>,
    condition: F,
) -> Cons<impl Constraint + Clone>
where
    F: Fn(Hand, &Context<'_>) -> bool + Clone + Send + Sync,
{
    Cons(Described {
        condition,
        label: label.into(),
    })
}

/// See [`reads_as`].
#[derive(Clone)]
struct ReadsAs<E, R> {
    evaluated: E,
    reading: R,
}

impl<E: Constraint, R: Constraint> Constraint for ReadsAs<E, R> {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        self.evaluated.eval(hand, context)
    }

    fn dependencies(&self) -> ConstraintDependencies {
        self.evaluated.dependencies()
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        let reading = self.reading.projection_dependencies();
        reading.remap(
            reading.get(ProjectionKind::Forward),
            reading.get(ProjectionKind::Band),
            reading.get(ProjectionKind::Complement),
            // `announce` is not overridden, so its default calls this type's
            // `project`, not `reading.announce`.
            reading.get(ProjectionKind::Forward),
        )
    }

    fn describe(&self) -> Description {
        self.reading.describe()
    }

    fn project(&self, context: &Context<'_>) -> EnvelopeUnion {
        self.reading.project(context)
    }

    fn project_band(&self, context: &Context<'_>) -> EnvelopeUnion {
        self.reading.project_band(context)
    }

    fn project_complement(&self, context: &Context<'_>) -> EnvelopeUnion {
        self.reading.project_complement(context)
    }
}

/// Evaluate as `evaluated`, but describe and project as `reading`
///
/// For a knob-gated classification-time arm (e.g. an evaluator-net gate) that
/// would otherwise vacuous the authored reading: an `Or` with an opaque
/// predicate unions every projection out to the full range, so the call would
/// read as *nothing* — the phantom-reading disaster.  Wrapping the seam keeps
/// the authored band as the declared reading instead.
///
/// Soundness caveat: with the knob **off** the two sides agree exactly and the
/// projection contract holds.  Knob-**on**, the reading is an approximation of
/// the live gate (the disclosable-floor compromise) — acceptable for an
/// opt-in arm whose A/B must vindicate it, not for a default-on conversion.
pub fn reads_as(
    evaluated: Cons<impl Constraint + Clone>,
    reading: Cons<impl Constraint + Clone>,
) -> Cons<impl Constraint + Clone> {
    Cons(ReadsAs { evaluated, reading })
}

/// See [`announced`].
#[derive(Clone)]
struct Announced<J, A> {
    judgment: J,
    agreement: A,
}

impl<J: Constraint, A: Constraint> Constraint for Announced<J, A> {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        self.judgment.eval(hand, context)
    }

    fn dependencies(&self) -> ConstraintDependencies {
        self.judgment.dependencies()
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        let judgment = self.judgment.projection_dependencies();
        let agreement = self.agreement.projection_dependencies();
        ProjectionDependencies::new(
            judgment.get(ProjectionKind::Forward),
            judgment.get(ProjectionKind::Band),
            judgment.get(ProjectionKind::Complement),
            agreement.get(ProjectionKind::Announcement),
        )
        .with_purity(judgment.is_pure() && agreement.is_pure())
    }

    fn describe(&self) -> Description {
        self.agreement.describe()
    }

    fn project(&self, context: &Context<'_>) -> EnvelopeUnion {
        self.judgment.project(context)
    }

    fn project_band(&self, context: &Context<'_>) -> EnvelopeUnion {
        self.judgment.project_band(context)
    }

    fn project_complement(&self, context: &Context<'_>) -> EnvelopeUnion {
        self.judgment.project_complement(context)
    }

    fn announce(&self, context: &Context<'_>) -> EnvelopeUnion {
        self.agreement.announce(context)
    }
}

/// Evaluate as `judgment`, disclose as `agreement`
///
/// The sibling of [`reads_as`], and the one that is safe default-on.  `reads_as`
/// overrides [`project`][Constraint::project], the fold
/// [`sample_layouts`][super::sampler::sample_layouts] accepts against, so an
/// approximate reading there breaks the containment contract and the sampler
/// starts excluding hands we hold — and
/// [`sample_layouts_replay`][super::sampler::sample_layouts_replay] does not
/// rescue it, because replay *conjoins* the range test rather than replacing it.
/// `announced` leaves all three projections on `judgment` — an evaluator-net
/// gate's ⊤ stays ⊤ and the sampler is untouched — and splits only
/// [`announce`][Constraint::announce] and [`describe`][Constraint::describe].
///
/// The use it exists for: a learned criterion decides, and the arithmetic it
/// replaced says what the call *means*.  The agreement must be calibrated to
/// what the criterion does in the population, not inherited from the arithmetic
/// unexamined — at the floor's RKCB ask the net fires some seven points below
/// the point sum it replaced, so announcing that sum would misdescribe the
/// median hand, never mind the tail.
pub fn announced(
    judgment: Cons<impl Constraint + Clone>,
    agreement: Cons<impl Constraint + Clone>,
) -> Cons<impl Constraint + Clone> {
    Cons(Announced {
        judgment,
        agreement,
    })
}

/// Which honor-weighted count tempers [`fifths`] (the A/B companion gauge)
///
/// Fifths is tuned for 3NT — it rewards aces and tens and discounts kings and
/// queens — so on its own it misjudges a hand headed for a suit contract.  A
/// notrump-defining range never gauges Fifths alone; it averages Fifths with
/// one of these honor counts, so a tens-rich hand can't reach the band on
/// Fifths and a quack-heavy hand isn't shut out of it.  BUM-RAP is the
/// default — it edged HCP across every vulnerability in the
/// `fifths-companion` A/B match.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FifthsCompanion {
    /// Milton Work 4-3-2-1 HCP
    Hcp,
    /// BUM-RAP 4.5-3-1.5-0.75-0.25
    Bumrap,
}

/// Which scale the global [`point_count`] evaluates on the current thread —
/// and with it every [`points`] gate, the constrained sampler's acceptance,
/// and the floor's combined counts, all at once
///
/// The point-scale deprecation A/B/C: the arms swap the scalar wholesale so a
/// candidate side's gates, projections, and sampling stay denominated in one
/// scale — the gates-vs-sampler confound of swapping [`points`] alone cannot
/// arise.  Authored ranges are untouched; only their gauge moves.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointScale {
    /// Legacy raw HCP + [`upgrade`] (the deposed incumbent, kept opt-in)
    PointCount,
    /// Raw Milton Work 4-3-2-1 HCP (the old `fuzzy_points` off arm)
    Hcp,
    /// Rule of N+8: raw HCP + the two longest suit lengths − 8, so a
    /// `points(12..)` gate is exactly the Rule of 20 (an opt-out since the
    /// upgrade-linearisation flip; its flat downgrade measured worse than the
    /// floor)
    RuleOfN,
    /// [`PointScale::RuleOfN`] with the length bonus floored at 0: flat
    /// 4-3-3-3 — plain rule-of-N+8's only downgrade — reads its raw HCP
    /// (an opt-out since the upgrade-linearisation flip)
    RuleOfNFloored,
}

std::thread_local! {
    /// The scale [`point_count`] evaluates (the point-scale A/B knob).
    /// **Default [`PointScale::PointCount`]** — raw HCP plus the capped
    /// shape [`upgrade`] (0-2: +1 unbalanced, +1 for a ten-card two-suiter,
    /// −1 per wasted short honor).  History: the deprecation A/B/C once deposed
    /// this legacy scale for rule of N+8, but that was against the *cliff*
    /// upgrade (first wasted honor voided the whole bonus); linearising
    /// [`upgrade`] flipped the verdict back — vs rule-of-N+8-floored, PointCount
    /// measured plain-DD wash both vuls and **PD +0.023 NV / +0.037 vul** (two
    /// disjoint ~90k-board/vul seeds, deterministic floor so the CI is the whole
    /// error budget; no retrain — the net consumes `upgrade` directly, scale-free).
    /// The capped shape reads wild two-suiters/freaks lighter than N+8's
    /// bonus-of-5, cutting the overbids PD punishes.  Rule of N+8 is now the
    /// opt-out: `set_point_scale(PointScale::RuleOfNFloored)` (and
    /// `PointScale::Hcp` for raw HCP).
    static POINT_SCALE: Cell<PointScale> = const { Cell::new(PointScale::PointCount) };
    /// Whether [`fifths`] evaluates Fifths rather than raw HCP.  Default **off**:
    /// the Fifths NT-gauge measured a clean net loss vs raw HCP in the A6 audit
    /// (self-play plain −0.012/−0.018 NV/vul, PD alike, CIs excluding 0), and it
    /// dragged the `points` upgrade (points-only beat points+fifths on both
    /// scorers).  See docs/bidding-options.md A6.
    static FUZZY_FIFTHS: Cell<bool> = const { Cell::new(false) };
    /// The honor count averaged with Fifths in [`fifths`] (BUM-RAP won the A/B)
    static FIFTHS_COMPANION: Cell<FifthsCompanion> = const { Cell::new(FifthsCompanion::Bumrap) };
    /// Whether [`support_points`] gauges the `hcp_plus`-based scale (HCP plus
    /// useful shortness, after BBO GIB) instead of the legacy
    /// raw-HCP-plus-[`upgrade`] [`point_count`]. **Default on.** Shortness is a
    /// ruffing value, real only once a trump fit exists, so the scale is scoped
    /// to the **fit-known** gates only ([`support_points`], never the global
    /// [`point_count`]) — the fit-unknown gates keep legacy [`points`] untouched.
    /// A measured win on every scorer (`examples/ab-point-count`, 200k–500k
    /// boards/vul): plain DD +0.033/+0.054, perfect defense +0.005/+0.020,
    /// sd-lead +0.052 (NV/vul) — all CIs clearing zero.  The unscoped global
    /// flip won bigger (sd-lead +0.28) but broke legacy gates on shaped hands
    /// before a fit; this captures the fit-known fraction without that
    /// regression.  `set_support_points(false)` is the A/B off arm.
    static SUPPORT_POINTS: Cell<bool> = const { Cell::new(true) };
    /// Antisymmetric strength adjustment for a simulated natural bidder.
    /// **Default 0** — byte-identical to the authored card.  Openings and
    /// overcalls are this many points lighter; responses and advances are this
    /// many points heavier.  Captured by strength gauges at book construction.
    ///
    /// Projection deliberately keeps disclosing the undialled authored
    /// meanings: that mismatch is the simulated deviation being measured.
    static STRENGTH_DIAL: Cell<u8> = const { Cell::new(0) };
}

/// Select the global point-count scale on the current thread (see
/// [`PointScale`])
///
/// For A/B measurement only: the scale is read at classification time by
/// [`point_count`] — and therefore by every [`points`] gate, the constrained
/// sampler, and the floor's combined counts together — and is per-thread;
/// classify on the thread that set it.
#[doc(hidden)]
pub fn set_point_scale(scale: PointScale) {
    POINT_SCALE.with(|cell| cell.set(scale));
}

/// The point scale active on the current classification thread
pub(crate) fn point_scale() -> PointScale {
    POINT_SCALE.with(Cell::get)
}

/// Enable or disable [`fifths`] alone
///
/// For A/B measurement only, read at classification time, per-thread; classify
/// on the thread that set it.  The `points` half of the old "fuzzy strength"
/// umbrella is [`set_point_scale`] — the umbrella and its bool `points` wrapper
/// were deleted 2026-08-03: one wrote *two* sibling cells (so flipping it
/// silently moved a knob the caller never named), and the other was a bool over
/// a three-valued scale, unable to name [`PointScale::RuleOfNFloored`] and
/// destroying it on write.
#[doc(hidden)]
pub fn set_fuzzy_fifths(enabled: bool) {
    FUZZY_FIFTHS.with(|flag| flag.set(enabled));
}

/// Choose the honor count averaged into [`fifths`] (see [`FifthsCompanion`])
#[doc(hidden)]
pub fn set_fifths_companion(companion: FifthsCompanion) {
    FIFTHS_COMPANION.with(|cell| cell.set(companion));
}

/// Enable or disable the `hcp_plus`-based [`support_points`] scale on the
/// current thread. **Default on** (the shipped fit-known shortness scale);
/// `false` is the A/B off arm that gauges legacy [`point_count`] instead.
#[doc(hidden)]
pub fn set_support_points(enabled: bool) {
    SUPPORT_POINTS.with(|flag| flag.set(enabled));
}

/// The [`set_fuzzy_fifths`] knob active on this thread
pub(crate) fn fuzzy_fifths_now() -> bool {
    FUZZY_FIFTHS.with(Cell::get)
}

/// The [`set_fifths_companion`] choice active on this thread
pub(crate) fn fifths_companion_now() -> FifthsCompanion {
    FIFTHS_COMPANION.with(Cell::get)
}

/// The [`set_support_points`] knob active on this thread
pub(crate) fn support_points_now() -> bool {
    SUPPORT_POINTS.with(Cell::get)
}

/// Set the antisymmetric strength adjustment for stances subsequently pinned on
/// the current thread (**default 0**, measurement only)
///
/// The deviation panel's B axis (docs/deviation-panel.md): a simulated natural
/// bidder whose openings and overcalls are `dial` points lighter and whose
/// responses and advances are `dial` points heavier.  The antisymmetry is the
/// point — pair-level calibration is preserved, so the partnership still stops
/// in the same places and every authored continuation stays coherent.
///
/// Pinned into the stance at [`Pair::against`][super::Pair::against] like every
/// other gauge setting, so the idiom is unchanged: set it, build the deviant
/// pair, reset.  It used to be captured when each *gauge* was built, because
/// before the pin campaign a classify-time read would have leaked the dial into
/// the book seated opposite on the same thread; a stance now carries its own
/// `ReadingProfile`, so the two seats
/// cannot see each other's dial.  The magnitude is all that is read here — the
/// direction is chosen per decision from the auction (`dial_shift`).
///
/// Projections stay undialled either way: the dial appears only in
/// [`Constraint::eval`], never in the reading folds, so the deviant opponent
/// keeps disclosing the authored meanings and that mismatch is the deviation
/// being measured.
pub fn set_strength_dial(dial: u8) {
    STRENGTH_DIAL.with(|cell| cell.set(dial));
}

pub(crate) fn strength_dial() -> u8 {
    STRENGTH_DIAL.with(Cell::get)
}

/// Direction in which the strength dial moves a measured value
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DialShift {
    /// Make an opening or overcall lighter
    Add(u8),
    /// Make a response or advance heavier
    Subtract(u8),
}

/// Choose the dial direction from the first non-pass call by our side
pub(crate) fn dial_shift(dial: u8, context: &Context<'_>) -> DialShift {
    let auction = context.auction();
    for (index, call) in auction.iter().enumerate() {
        if *call == Call::Pass {
            continue;
        }
        match (auction.len() - index) % 4 {
            0 => return DialShift::Add(dial),
            2 => return DialShift::Subtract(dial),
            _ => {}
        }
    }
    DialShift::Add(dial)
}

/// Raw high card points of a hand
pub(crate) fn raw_hcp(hand: Hand) -> u8 {
    SimpleEvaluator(eval::hcp::<u8>).eval(hand)
}

/// Raw high card points held in one suit — the kernel [`SuitHcp`]'s eval and
/// the [`Envelope`][super::inference::Envelope] `suit_hcp` gauge share
pub(crate) fn suit_raw_hcp(hand: Hand, suit: Suit) -> u8 {
    eval::hcp::<u8>(hand[suit])
}

/// Project a numeric range bound into an inference [`Range`], clamped to `cap`
///
/// The forward dual of [`describe_int_range`]: where that names a bound in
/// prose, this turns it into the `[min, max]` an [`Envelope`] records, sharing
/// the same [`ToU64`] so `len` (a `usize` range) and `points`/`hcp` (`u8`)
/// project through one path.  An unbounded end becomes `cap`, the quantity's
/// natural ceiling.
fn bound_range<T: ToU64>(range: &impl RangeBounds<T>, cap: u8) -> Range {
    let cap = u64::from(cap);
    let min = match range.start_bound() {
        Bound::Included(&x) => x.to_u64(),
        Bound::Excluded(&x) => x.to_u64() + 1,
        Bound::Unbounded => 0,
    };
    let max = match range.end_bound() {
        Bound::Included(&x) => x.to_u64(),
        Bound::Excluded(&x) => x.to_u64().saturating_sub(1),
        Bound::Unbounded => cap,
    };
    // `min(cap)` keeps both ends within the quantity's ceiling, so the casts
    // back to the `u8` an `Envelope` stores never truncate.
    let clamp = |x: u64| u8::try_from(x.min(cap)).unwrap_or_else(|_| unreachable!());
    Range::new(clamp(min), clamp(max))
}

/// Raw high card points in a range (the [`hcp`] constraint)
#[derive(Clone)]
struct Hcp<R> {
    range: R,
}

impl<R: RangeBounds<u8> + Clone + Send + Sync> Constraint for Hcp<R> {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        let value = raw_hcp(hand);
        let dial = context.reading_profile().strength_dial();
        if dial == 0 {
            return crisp(self.range.contains(&value));
        }
        let value = match dial_shift(dial, context) {
            DialShift::Add(dial) => value.saturating_add(dial),
            DialShift::Subtract(dial) => value.saturating_sub(dial),
        };
        crisp(self.range.contains(&value))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        // `PROFILE` since the strength dial moved off the constructor: the
        // gauge is still raw HCP, but which band it checks depends on the
        // classifying stance.
        ConstraintDependencies::HAND
            | ConstraintDependencies::CONTEXT
            | ConstraintDependencies::PROFILE
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::PROFILE)
    }

    fn describe(&self) -> Description {
        describe_int_range(&self.range, "HCP")
    }

    fn project(&self, context: &Context<'_>) -> EnvelopeUnion {
        // ponytail: floor only — points = raw HCP + upgrade ≥ raw HCP, so an
        // HCP *ceiling* is unsound on the upgraded-points scale an `Envelope`
        // records; the floor is exact.  Rule of N+8 reads a flat 4-3-3-3 one
        // under its HCP, so that scale gives the floor back 1.  The ceiling
        // returns in [`project_band`][Constraint::project_band], widened by
        // [`hcp_ceiling_slack`].
        let slack = flat_hcp_slack(context.reading_profile().point_scale());
        let floor = bound_range(&self.range, Range::FULL_POINTS.max).min;
        let mut inference = Envelope::unknown();
        inference.strength.points = Range::new(floor.saturating_sub(slack), Range::FULL_POINTS.max);
        // The `hcp` gauge is raw HCP, so its floor is exact — no upgrade slack.
        inference.strength.hcp = Range::new(floor, Range::FULL_POINTS.max);
        EnvelopeUnion::from(inference)
    }

    fn project_band(&self, context: &Context<'_>) -> EnvelopeUnion {
        EnvelopeUnion::from(hcp_band(
            context.reading_profile().point_scale(),
            bound_range(&self.range, Range::FULL_POINTS.max),
        ))
    }

    fn project_complement(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // `!hcp(13..)` is "at most twelve raw HCP", which the upgraded scale
        // then widens upward by the same slack `project_band` owes it.  A
        // two-sided band complements to its two outer halves through
        // `disjoin`: knob-off they hull back to the full range (the legacy
        // single-envelope reading, byte-identical); knob-on both boxes survive.
        complement_halves(
            bound_range(&self.range, Range::FULL_POINTS.max),
            Range::FULL_POINTS.max,
        )
        .map(|half| EnvelopeUnion::from(hcp_band(profile.point_scale(), half)))
        .reduce(|a, b| a.disjoin_with(b, profile))
        .unwrap_or_else(EnvelopeUnion::unknown)
    }
}

/// The upgraded-points envelope a *raw HCP* band implies: slacked down by
/// [`flat_hcp_slack`] (rule of N+8 reads a flat 4-3-3-3 one under its HCP) and
/// up by [`hcp_ceiling_slack`] (the scale's maximum upgrade).
fn hcp_band(scale: PointScale, raw: Range) -> Envelope {
    let mut inference = Envelope::unknown();
    inference.strength.points = Range::new(
        raw.min.saturating_sub(flat_hcp_slack(scale)),
        raw.max
            .saturating_add(hcp_ceiling_slack(scale))
            .min(Range::FULL_POINTS.max),
    );
    // The `hcp` gauge keeps the crisp raw band, unslacked (notrump valuation).
    inference.strength.hcp = raw;
    inference
}

/// Total high card points in the given range
#[must_use]
pub fn hcp(range: impl RangeBounds<u8> + Clone + Send + Sync) -> Cons<impl Constraint + Clone> {
    Cons(Hcp { range })
}

/// The slack an HCP-gated point envelope owes `scale`: rule of N+8 reads a flat
/// 4-3-3-3 one under its HCP; the other scales never read under.  Shared by
/// [`hcp`]'s projection and the hand-authored NT-opening readings.
pub(crate) const fn flat_hcp_slack(scale: PointScale) -> u8 {
    matches!(scale, PointScale::RuleOfN) as u8
}

/// The most `scale`'s [`point_count`] can exceed raw HCP — the ceiling dual of
/// [`flat_hcp_slack`]: rule of N+8 adds up to `longest_two_suits − 8` ≤ 5, the
/// legacy upgrade at most 2, plain HCP nothing.  Widens an HCP gate's ceiling in
/// [`project_band`][Constraint::project_band].
const fn hcp_ceiling_slack(scale: PointScale) -> u8 {
    match scale {
        PointScale::Hcp => 0,
        PointScale::PointCount => 2,
        PointScale::RuleOfN | PointScale::RuleOfNFloored => 5,
    }
}

/// The most `point_count − raw_hcp` can be for a hand whose suit lengths lie in
/// `lengths`, or `None` when the active scale's upgrade is not bounded by shape
/// alone
///
/// The box-local dual of [`hcp_ceiling_slack`], which answers the same question
/// for *any* hand.  The gap that pays is [`is_balanced`]: a balanced hand has
/// at most 9 cards in its two longest suits and never upgrades, so a box whose
/// lengths force balanced reads `points == hcp` where the global slack leaves 2
/// HCP on the table at each end.  Used by
/// [`Envelope::narrow_to_upgrade`][super::inference::Envelope::narrow_to_upgrade].
///
/// Rule-of-N+8 returns `None`: its count can fall *below* raw HCP (a flat
/// 4-3-3-3 reads one under), so the closure's `points >= hcp` leg does not hold
/// and the scale is not worth its own case — nothing measures on it today.
pub(crate) fn upgrade_ceiling(scale: PointScale, lengths: &[Range; 4]) -> Option<u8> {
    match scale {
        PointScale::Hcp => Some(0),
        PointScale::PointCount => Some(if forces_balanced(lengths) { 0 } else { 2 }),
        PointScale::RuleOfN | PointScale::RuleOfNFloored => None,
    }
}

/// Whether every 13-card shape inside this length box is [`is_balanced`]
///
/// ponytail: brute force over the box's own compositions, early-exiting on the
/// first unbalanced one — which is immediate for the overwhelmingly common
/// wide box.  Balanced boxes are narrow (`2..=5` at worst), so the loop stays
/// tiny; swap in a precomputed 560-shape table only if a profile says so.
fn forces_balanced(lengths: &[Range; 4]) -> bool {
    for a in lengths[0].min..=lengths[0].max.min(13) {
        for b in lengths[1].min..=lengths[1].max.min(13 - a) {
            for c in lengths[2].min..=lengths[2].max.min(13 - a - b) {
                let shape = [a, b, c, 13 - a - b - c];
                if lengths[3].contains(shape[3])
                    && !(shape.iter().all(|&len| len >= 2)
                        && shape.iter().filter(|&&len| len == 2).count() <= 1)
                {
                    return false;
                }
            }
        }
    }
    true
}

/// Whether a short suit (at most two cards) holds a **wasted honor**
///
/// Honors in shortness are wasted: any of A/K/Q/J in a suit of at most two
/// cards fails to pull its weight, except the working holdings Ax and Kx.
/// A wasted honor voids the fuzzy [`upgrade`].
const fn wasted(holding: Holding) -> bool {
    holding.len() <= 2
        && (holding.contains(Rank::Q)
            || holding.contains(Rank::J)
            || (holding.contains(Rank::A) && holding.contains(Rank::K))
            || (holding.len() == 1 && (holding.contains(Rank::A) || holding.contains(Rank::K))))
}

/// Fuzzy-strength upgrade on top of raw HCP
///
/// Sharp on shape, fuzzy on strength: an unbalanced hand whose short suits
/// waste no honors (see below) upgrades by 1 point, plus 1 more with ten or
/// more cards in its two longest suits.  Balanced hands never upgrade, so
/// [`points`] coincides with [`hcp`] for them.
///
/// An honor (A, K, Q, or J) in a suit of at most two cards is wasted and
/// costs one point each, except the working holdings Ax and Kx.  The upgrade
/// floors at zero.
#[must_use]
pub fn upgrade(hand: Hand) -> u8 {
    let holdings = Suit::ASC.map(|suit| hand[suit]);
    let nwasted = holdings.iter().filter(|&&holding| wasted(holding)).count() as u32;
    let base = u32::from(!is_balanced(hand)) + u32::from(longest_two_suits(hand) >= 10);
    u8::try_from(base.saturating_sub(nwasted)).unwrap_or_else(|_| unreachable!())
}

/// Total length of the two longest suits — the shape kernel shared by
/// [`upgrade`] and the rule-of-N+8 [`PointScale`]
fn longest_two_suits(hand: Hand) -> u8 {
    let mut lengths = Suit::ASC.map(|suit| hand[suit].len());
    lengths.sort_unstable();
    // Two suit lengths total at most 26, so the cast cannot truncate.
    u8::try_from(lengths[2] + lengths[3]).unwrap_or_else(|_| unreachable!())
}

/// Upgraded points as a scalar — the strength number the suit-oriented
/// [`points`] constraint gauges and the scale [`Inferences`] records its point
/// ranges on
///
/// Defaults to the **raw-HCP-plus-[`upgrade`] scale** ([`PointScale::PointCount`])
/// — raw HCP plus the capped shape upgrade (0-2), which reads wild shapes
/// lighter than rule-of-N+8's bonus-of-5 and measured a plain-DD wash with a
/// PD gain against it (see [`PointScale`] for the verdicts; rule-of-N+8-floored,
/// where `points(12..)` is exactly the Rule of 20, is now the opt-out).  A
/// reader that needs the
/// value rather than a range — constrained sampling, for one — shares this
/// single definition so it can never drift from the ranges it checks against,
/// and [`points`] gauges it directly so the two can never disagree.
/// [`set_point_scale`] swaps the scale wholesale — gates, sampler, and floor
/// together — for the point-scale A/B; the fit-known shortness scale rides on
/// [`support_point_count`] instead.
///
/// [`Inferences`]: super::inference::Inferences
#[must_use]
pub fn point_count(hand: Hand) -> u8 {
    point_count_on(point_scale(), hand)
}

/// [`point_count`] on an explicit scale — what every classify-time caller uses,
/// so the count a stance gauges with is the one pinned into it at build rather
/// than whatever the classifying thread happens to hold.
pub(crate) fn point_count_on(scale: PointScale, hand: Hand) -> u8 {
    match scale {
        PointScale::PointCount => raw_hcp(hand) + upgrade(hand),
        PointScale::Hcp => raw_hcp(hand),
        PointScale::RuleOfN => (raw_hcp(hand) + longest_two_suits(hand)).saturating_sub(8),
        // Flooring the *bonus* at 0 floors the whole count at raw HCP: only
        // flat 4-3-3-3 has its two longest suits under 8 cards.
        PointScale::RuleOfNFloored => raw_hcp(hand) + longest_two_suits(hand).saturating_sub(8),
    }
}

/// The `hcp_plus`-based scale [`support_points`] gauges when its flag is on:
/// `hcp_plus` (HCP plus useful shortness, see [`eval::hcp_plus`]) plus the bare
/// long-suit length term (two longest suits ≥10 cards ≈ an almost-certain double
/// fit).  Closer to BBO GIB's point count than the legacy
/// raw-HCP-plus-[`upgrade`] [`point_count`].
fn new_point_count(hand: Hand) -> u8 {
    SimpleEvaluator(eval::hcp_plus::<u8>).eval(hand) + u8::from(longest_two_suits(hand) >= 10)
}

/// Upgraded points in a range (the [`points`] constraint)
#[derive(Clone)]
struct Points<R> {
    range: R,
}

impl<R: RangeBounds<u8> + Clone + Send + Sync> Constraint for Points<R> {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        // Always the shared scalar, whatever scale it is set to — the
        // sampler's soundness invariant (it measures the same number) holds
        // on every arm of the point-scale A/B.
        let profile = context.reading_profile();
        let value = point_count_on(profile.point_scale(), hand);
        let dial = profile.strength_dial();
        if dial == 0 {
            return crisp(self.range.contains(&value));
        }
        let value = match dial_shift(dial, context) {
            DialShift::Add(dial) => value.saturating_add(dial),
            DialShift::Subtract(dial) => value.saturating_sub(dial),
        };
        crisp(self.range.contains(&value))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND
            | ConstraintDependencies::CONTEXT
            | ConstraintDependencies::PROFILE
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::PROFILE)
    }

    fn describe(&self) -> Description {
        describe_int_range(&self.range, "points")
    }

    fn project(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // Floor only, matching every hand-written reader (`at_least(floor,
        // CAP)`): sound whether or not the fuzzy-strength upgrade is on, since
        // the upgraded point count is never below the band's floor.
        let floor = bound_range(&self.range, Range::FULL_POINTS.max).min;
        let mut inference = Envelope::unknown();
        inference.strength.points = Range::new(floor, Range::FULL_POINTS.max);
        if profile.envelope_union() {
            // `point_count` exceeds raw HCP by at most `hcp_ceiling_slack`,
            // so a points floor implies an HCP floor slacked by the scale's
            // maximum upgrade — without this, a `points | hcp` disjunction
            // whose HCP box the points box (correctly) swallows loses its HCP
            // knowledge entirely.  New precision — knob-gated.
            inference.strength.hcp = Range::new(
                floor.saturating_sub(hcp_ceiling_slack(profile.point_scale())),
                Range::FULL_POINTS.max,
            );
        }
        EnvelopeUnion::from(inference)
    }

    fn project_band(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // Both bounds exact: `points` gauges the shared `point_count` scalar
        // the `Envelope` scale records, whatever scale it is set to.
        let mut inference = Envelope::unknown();
        let band = bound_range(&self.range, Range::FULL_POINTS.max);
        inference.strength.points = band;
        if profile.envelope_union() {
            // The two-sided HCP image of a points band: down by the scale's
            // maximum upgrade, up by its flat-hand under-read (rule of N+8
            // reads a flat 4-3-3-3 one under its HCP).  Knob-gated as above.
            inference.strength.hcp = Range::new(
                band.min
                    .saturating_sub(hcp_ceiling_slack(profile.point_scale())),
                band.max
                    .saturating_add(flat_hcp_slack(profile.point_scale()))
                    .min(Range::FULL_POINTS.max),
            );
        }
        EnvelopeUnion::from(inference)
    }

    fn project_complement(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // Exact on the same scale, so the complement halves are exact too;
        // `disjoin` keeps knob-off hulled to the full range (the legacy
        // single-envelope reading), knob-on both boxes.
        complement_halves(
            bound_range(&self.range, Range::FULL_POINTS.max),
            Range::FULL_POINTS.max,
        )
        .map(|half| {
            let mut inference = Envelope::unknown();
            inference.strength.points = half;
            EnvelopeUnion::from(inference)
        })
        .reduce(|a, b| a.disjoin_with(b, profile))
        .unwrap_or_else(EnvelopeUnion::unknown)
    }
}

/// [`point_count`] in the given range
///
/// The strength gauge for suit-oriented calls.  Notrump-defining ranges use
/// [`fifths`] instead, and ranges indifferent to shape keep [`hcp`].
#[must_use]
pub fn points(range: impl RangeBounds<u8> + Clone + Send + Sync) -> Cons<impl Constraint + Clone> {
    Cons(Points { range })
}

/// The **suit-blind** support scalar: [`point_count`] on the fit-known
/// shortness scale when [`set_support_points`] is on, else legacy
/// [`point_count`]
///
/// Superseded by [`support_point_count_in`] at every gate that knows its
/// trump statically — this sums `hcp_plus` over *all four* suits, crediting
/// even a short trump holding with ruffing value it cannot have.  It remains
/// for the sites where the trump is dynamic (`slam_entry_reached` resolves
/// its trump per call — migration is a ledger follow-up) and for the
/// diagnostic probes.
#[must_use]
pub fn support_point_count(hand: Hand) -> u8 {
    support_point_count_on(support_points_now(), point_scale(), hand)
}

/// [`support_point_count`] on explicit scales (see [`point_count_on`])
pub(crate) fn support_point_count_on(support: bool, scale: PointScale, hand: Hand) -> u8 {
    if support {
        new_point_count(hand)
    } else {
        point_count_on(scale, hand)
    }
}

/// The suit-indexed support scale every [`support_points`] gate gauges: the
/// trump suit is worth plain HCP — trumps are trumps, not ruffs, so a short
/// trump holding earns no shortness value — while the side suits keep
/// `hcp_plus` (HCP plus useful shortness) and the double-fit term stays as in
/// the suit-blind [`support_point_count`].  Trump *length* is deliberately
/// not in the scale: the sites where fit length decides games
/// (`fit_sum_game`, Texas, the six-card invites, `fit_value`) add it
/// explicitly, as they always have.
///
/// Whenever trump length ≥ 3 (every fit-known gate), this is exactly
/// `support_point_count(hand)`; the scales diverge only on a short trump
/// holding, where the suit-blind scale wrongly counted ruffing shortness *in
/// trump* — a doubleton reads a point lower here, a stiff two.  Under
/// [`set_support_points`]`(false)` (the historical A/B off arm) it falls back
/// to legacy [`point_count`], which counts no shortness anywhere.
#[must_use]
pub fn support_point_count_in(hand: Hand, trump: Suit) -> u8 {
    support_point_count_in_on(support_points_now(), point_scale(), hand, trump)
}

/// [`support_point_count_in`] on explicit scales (see [`point_count_on`])
pub(crate) fn support_point_count_in_on(
    support: bool,
    scale: PointScale,
    hand: Hand,
    trump: Suit,
) -> u8 {
    if !support {
        return point_count_on(scale, hand);
    }
    let holdings = Suit::ASC.map(|suit| {
        if suit == trump {
            eval::hcp::<u8>(hand[suit])
        } else {
            eval::hcp_plus::<u8>(hand[suit])
        }
    });
    holdings.iter().sum::<u8>() + u8::from(longest_two_suits(hand) >= 10)
}

/// Write a support band into an [`Envelope`]'s per-suit slots: the named
/// trump's slot alone — the band is a claim about *that* fit — while the
/// rest stay full.
fn support_slots(trump: Suit, band: Range) -> [Range; 4] {
    let mut slots = [Range::FULL_POINTS; 4];
    slots[trump as usize] = band;
    slots
}

/// [`support_point_count_in`] in a range (the [`support_points`] constraint)
#[derive(Clone)]
struct SupportPoints<R> {
    suit: Suit,
    range: R,
}

impl<R: RangeBounds<u8> + Clone + Send + Sync> SupportPoints<R> {
    /// The authored band, bounded onto the scale
    fn band(&self) -> Range {
        bound_range(&self.range, Range::FULL_POINTS.max)
    }
}

impl<R: RangeBounds<u8> + Clone + Send + Sync> Constraint for SupportPoints<R> {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        // Clamp the measured value at the scale cap: a capped ceiling means
        // "unbounded", so the clamp keeps a freak hand inside every
        // floor-only band.
        let profile = context.reading_profile();
        let value = support_point_count_in_on(
            profile.support_points(),
            profile.point_scale(),
            hand,
            self.suit,
        );
        let dial = profile.strength_dial();
        if dial == 0 {
            return crisp(self.band().contains(value.min(Range::FULL_POINTS.max)));
        }
        let value = match dial_shift(dial, context) {
            DialShift::Add(dial) => value.saturating_add(dial),
            DialShift::Subtract(dial) => value.saturating_sub(dial),
        }
        .min(Range::FULL_POINTS.max);
        crisp(self.band().contains(value))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND
            | ConstraintDependencies::CONTEXT
            | ConstraintDependencies::PROFILE
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::PROFILE)
    }

    fn describe(&self) -> Description {
        // The gate names no suit in disclosure: every gate conjoins 3+ trumps,
        // where the suit-indexed value equals the familiar suit-blind count.
        describe_int_range(&self.range, "support points")
    }

    fn project(&self, _: &Context<'_>) -> EnvelopeUnion {
        // Floor into the dedicated per-suit `support_points` gauge, which
        // measures this same value — so, unlike a projection into the legacy
        // `points` gauge (which records `point_count`, no lower bound on the
        // shortness scale), the floor is exact.
        // Read behind Edit 1's knob; the `points`/`admits` gauge is untouched.
        let floor = self.band().min;
        let mut inference = Envelope::unknown();
        inference.strength.support_points =
            support_slots(self.suit, Range::new(floor, Range::FULL_POINTS.max));
        EnvelopeUnion::from(inference)
    }

    fn project_band(&self, _: &Context<'_>) -> EnvelopeUnion {
        // Both bounds exact on the dedicated `support_points` gauge — the
        // band is precisely the eval's test.
        let mut inference = Envelope::unknown();
        inference.strength.support_points = support_slots(self.suit, self.band());
        EnvelopeUnion::from(inference)
    }

    fn project_complement(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // Exact on the dedicated gauge.  No complement existed before the
        // envelope-union wave (even one-sided reads were ⊤), so the whole reading is new
        // precision and sits behind the knob — knob-off stays ⊤.
        if !profile.envelope_union() {
            return EnvelopeUnion::unknown();
        }
        complement_halves(self.band(), Range::FULL_POINTS.max)
            .map(|half| {
                let mut inference = Envelope::unknown();
                inference.strength.support_points = support_slots(self.suit, half);
                EnvelopeUnion::from(inference)
            })
            .reduce(|a, b| a.disjoin_with(b, profile))
            .unwrap_or_else(EnvelopeUnion::unknown)
    }
}

/// [`support_point_count_in`] in the given range — the fit-known counterpart
/// to [`points`]
///
/// Wire this into a gate only when a trump fit is known; it counts shortness as
/// support value, unsound before a fit.  The invariant is grep-able:
/// `support_points` in a gate ⟹ a fit is known.
///
/// `suit` is the agreed trump — written explicitly, because the context's
/// partner-last-suit is the *wrong* trump at Jacoby rebids (partner's last
/// call was 2NT) and transfer-GF rebids.  The gate tests
/// [`support_point_count_in`]`(hand, suit)`, denying a short trump holding
/// the phantom ruffing value the suit-blind scale credited it.
#[must_use]
pub fn support_points(
    suit: Suit,
    range: impl RangeBounds<u8> + Clone + Send + Sync,
) -> Cons<impl Constraint + Clone> {
    Cons(SupportPoints { suit, range })
}

/// Fifths in a range (the [`fifths`] constraint)
#[derive(Clone)]
struct Fifths<R>(R);

impl<R: RangeBounds<f64> + Clone + Send + Sync> Constraint for Fifths<R> {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        let profile = context.decision_profile();
        let value = if profile.fuzzy_fifths {
            // Never Fifths alone: average it with a real-honor count so the
            // 3NT-tuned tens/aces bias is halved toward Milton Work / BUM-RAP.
            let companion = match profile.fifths_companion {
                FifthsCompanion::Hcp => f64::from(raw_hcp(hand)),
                FifthsCompanion::Bumrap => eval::BUMRAP.eval(hand),
            };
            f64::midpoint(eval::FIFTHS.eval(hand), companion)
        } else {
            f64::from(raw_hcp(hand))
        };
        crisp(self.0.contains(&value))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND | ConstraintDependencies::PROFILE
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        describe_real_range(&self.0, "fifths")
    }
}

/// Tempered [Fifths][eval::FIFTHS] in the given range
///
/// Thomas Andrews's computed point count for 3NT, on the same 40-point scale
/// as HCP (A&nbsp;=&nbsp;4, K&nbsp;=&nbsp;2.8, Q&nbsp;=&nbsp;1.8,
/// J&nbsp;=&nbsp;1, T&nbsp;=&nbsp;0.4).  The strength gauge for
/// notrump-defining ranges, but never on its own: Fifths is too 3NT-oriented,
/// so the value banded here is the *average* of Fifths and an honor-weighted
/// companion ([`FifthsCompanion`], HCP or BUM-RAP) — half the 3NT tens/aces
/// bias.  Convert an integer HCP band to a half-open interval, e.g.
/// `hcp(15..=17)` becomes `fifths(15.0..18.0)` so adjacent bands keep tiling.
// ponytail: blended unconditionally — every current `fifths` site is an
// *initial* NT bid, where the 3NT bias hurts.  Raising a notrump partner has
// shown (1NT - 2NT, 1NT - 3NT) is the one place pure Fifths is fine, but those
// rules gate on `hcp` today; add a pure-Fifths variant only when one needs it.
#[must_use]
pub fn fifths(range: impl RangeBounds<f64> + Clone + Send + Sync) -> Cons<impl Constraint + Clone> {
    Cons(Fifths(range))
}

/// Length of a suit in a range (the [`len`] constraint)
#[derive(Clone)]
struct Len<R> {
    suit: Suit,
    range: R,
}

impl<R: RangeBounds<usize> + Clone + Send + Sync> Constraint for Len<R> {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        crisp(self.range.contains(&hand[self.suit].len()))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::new(
            ConstraintDependencies::NONE,
            ConstraintDependencies::NONE,
            ConstraintDependencies::PROFILE,
            ConstraintDependencies::NONE,
        )
    }

    fn describe(&self) -> Description {
        describe_int_range(&self.range, &self.suit.to_string())
    }

    fn project(&self, _: &Context<'_>) -> EnvelopeUnion {
        // Length is exact — the same `hand[suit].len()` `eval` checks — so both
        // bounds project soundly.
        EnvelopeUnion::from(len_projection(self.suit, &self.range))
    }

    fn project_complement(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        len_complement(self.suit, &self.range, profile)
    }
}

/// The projection of `!len(suit, range)` — `suit` bounded to the complement of
/// `range`, every other suit full.  A two-sided `range` complements to its two
/// outer halves through [`EnvelopeUnion::disjoin`]: knob-off they hull back to
/// the full suit (the legacy single-envelope reading, byte-identical); knob-on
/// both boxes survive.
/// Shared with [`Support`], whose suit comes from the auction.
fn len_complement<R: RangeBounds<usize>>(
    suit: Suit,
    range: &R,
    profile: ReadingProfile,
) -> EnvelopeUnion {
    complement_halves(
        bound_range(range, Range::FULL_LENGTH.max),
        Range::FULL_LENGTH.max,
    )
    .map(|half| {
        let mut inference = Envelope::unknown();
        inference.lengths[suit as usize] = half;
        EnvelopeUnion::from(inference)
    })
    .reduce(|a, b| a.disjoin_with(b, profile))
    .unwrap_or_else(EnvelopeUnion::unknown)
}

/// Length of the given suit in the given range
pub fn len(
    suit: Suit,
    range: impl RangeBounds<usize> + Clone + Send + Sync,
) -> Cons<impl Constraint + Clone> {
    Cons(Len { suit, range })
}

/// The projection of a single `len(suit, range)` — `suit` floored to `range`,
/// every other suit full.  Shared by [`AllLen`] (intersected) and [`AnyLen`]
/// (unioned), and by [`Len::project`]'s sibling logic.
fn len_projection<R: RangeBounds<usize>>(suit: Suit, range: &R) -> Envelope {
    let mut inference = Envelope::unknown();
    inference.lengths[suit as usize] = bound_range(range, Range::FULL_LENGTH.max);
    inference
}

/// Length of *every* suit in `suits` within `range` (the [`and`] combinator)
#[derive(Clone)]
struct AllLen<const N: usize, R> {
    suits: [Suit; N],
    range: R,
}

impl<const N: usize, R: RangeBounds<usize> + Clone + Send + Sync> Constraint for AllLen<N, R> {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        crisp(
            self.suits
                .iter()
                .all(|&suit| self.range.contains(&hand[suit].len())),
        )
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        self.suits
            .iter()
            .map(|suit| describe_int_range(&self.range, &suit.to_string()))
            .reduce(|a, b| a.and(b))
            .unwrap_or(Description::Opaque)
    }

    fn project(&self, context: &Context<'_>) -> EnvelopeUnion {
        // Every named suit is floored to `range` (the same exact `len` check), so
        // the projection intersects each suit's bound — sound *and* tight.
        let scale = context.reading_profile().point_scale();
        EnvelopeUnion::from(
            self.suits
                .iter()
                .map(|&suit| len_projection(suit, &self.range))
                .reduce(|acc, inf| acc.intersect_on(&inf, scale))
                .unwrap_or_else(Envelope::unknown),
        )
    }
}

/// Every suit in `suits` falls in `range` — the suit-set conjunction
///
/// `and([♥, ♠], 4..)` is both majors at least four (the flat 4-4 two-suiter);
/// `and([♥, ♠], 4..) & or([♥, ♠], 5..)` is the 5-4-either-way Landy shape.  The
/// many-suit generalization of [`len`], and the tight dual of [`or`]: its
/// projection floors every named suit, where [`or`]'s washes out.
#[must_use]
pub fn and<const N: usize>(
    suits: [Suit; N],
    range: impl RangeBounds<usize> + Clone + Send + Sync,
) -> Cons<impl Constraint + Clone> {
    Cons(AllLen { suits, range })
}

/// Length of *some* suit in `suits` within `range` (the [`or`] combinator)
#[derive(Clone)]
struct AnyLen<const N: usize, R> {
    suits: [Suit; N],
    range: R,
}

impl<const N: usize, R: RangeBounds<usize> + Clone + Send + Sync> Constraint for AnyLen<N, R> {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        crisp(
            self.suits
                .iter()
                .any(|&suit| self.range.contains(&hand[suit].len())),
        )
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::new(
            ConstraintDependencies::PROFILE,
            ConstraintDependencies::PROFILE,
            ConstraintDependencies::NONE,
            ConstraintDependencies::PROFILE,
        )
    }

    fn describe(&self) -> Description {
        self.suits
            .iter()
            .map(|suit| describe_int_range(&self.range, &suit.to_string()))
            .reduce(|a, b| a.or(b))
            .unwrap_or(Description::Opaque)
    }

    fn project(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // At least one named suit lies in `range`, but not which — the sound
        // envelope is the union of the arms, which widens every suit back to full
        // unless exactly one suit is named (then it floors exactly, like `len`).
        //
        // C2 (`envelope_union_reading` on) keeps the arms as separate `EnvelopeUnion` boxes so
        // `or([♥, ♠], 6..)` pins the two shapes instead of widening both suits
        // to full — the `Or`-wall fix; off, it hulls to one box (byte-identical).
        self.suits
            .iter()
            .map(|&suit| EnvelopeUnion::from(len_projection(suit, &self.range)))
            .reduce(|a, b| a.disjoin_with(b, profile))
            .unwrap_or_else(EnvelopeUnion::unknown)
    }
}

/// At least one suit in `suits` falls in `range` — the suit-set disjunction
///
/// `or([♥, ♠], 6..)` is a six-plus card major, unknown which (a Multi one-suiter);
/// `or([♣, ♦], 4..)` is a four-plus minor (the Muiderberg side suit).  The dual of
/// [`and`]: its projection is the union of the arms — sound but loose, since a
/// one-of-N suit cannot floor any single suit.
#[must_use]
pub fn or<const N: usize>(
    suits: [Suit; N],
    range: impl RangeBounds<usize> + Clone + Send + Sync,
) -> Cons<impl Constraint + Clone> {
    Cons(AnyLen { suits, range })
}

/// High card points held *in one suit* in a range (the [`suit_hcp`] constraint)
#[derive(Clone)]
struct SuitHcp<R> {
    suit: Suit,
    range: R,
}

impl<R: RangeBounds<u8> + Clone + Send + Sync> Constraint for SuitHcp<R> {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        crisp(self.range.contains(&suit_raw_hcp(hand, self.suit)))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::new(
            ConstraintDependencies::NONE,
            ConstraintDependencies::NONE,
            ConstraintDependencies::PROFILE,
            ConstraintDependencies::NONE,
        )
    }

    fn describe(&self) -> Description {
        describe_int_range(&self.range, &format!("HCP in {}", self.suit))
    }

    fn project(&self, _: &Context<'_>) -> EnvelopeUnion {
        // Both bounds are exact — the axis records the very scalar the gate
        // evaluates, so unlike `Hcp` (whose ceiling is unsound on the upgraded
        // points scale) the *forward* projection keeps its ceiling.  Intended;
        // do not "fix" the asymmetry in either direction.
        EnvelopeUnion::from(suit_hcp_box(
            self.suit,
            bound_range(&self.range, SUIT_HCP_CAP),
        ))
    }

    fn project_band(&self, context: &Context<'_>) -> EnvelopeUnion {
        self.project(context)
    }

    fn project_complement(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // A two-sided band complements to its two outer halves through
        // `disjoin`: knob-off they hull back to the full range (the legacy
        // single-envelope reading, byte-identical); knob-on both boxes survive.
        complement_halves(bound_range(&self.range, SUIT_HCP_CAP), SUIT_HCP_CAP)
            .map(|half| EnvelopeUnion::from(suit_hcp_box(self.suit, half)))
            .reduce(|a, b| a.disjoin_with(b, profile))
            .unwrap_or_else(EnvelopeUnion::unknown)
    }
}

/// The cap of the [`Envelope`]'s per-suit HCP axis (AKQJ)
const SUIT_HCP_CAP: u8 = Range::FULL_SUIT_HCP.max;

/// An otherwise-unknown box whose `suit_hcp` slot for `suit` is `band`
fn suit_hcp_box(suit: Suit, band: Range) -> Envelope {
    let mut inference = Envelope::unknown();
    inference.strength.suit_hcp[suit as usize] = band;
    inference
}

/// High card points held in the given suit, in the given range
///
/// Suit-specific HCP (A=4, K=3, Q=2, J=1). Distinguishes a *too-good stopper* —
/// strong honors in the opponents' suit that defend better than they declare —
/// from a thin one or a long running source; see the Lebensohl trap pass.
#[must_use]
pub fn suit_hcp(
    suit: Suit,
    range: impl RangeBounds<u8> + Clone + Send + Sync,
) -> Cons<impl Constraint + Clone> {
    Cons(SuitHcp { suit, range })
}

/// Balanced shape kernel shared by [`balanced`] and [`upgrade`]
fn is_balanced(hand: Hand) -> bool {
    let lengths = Suit::ASC.map(|suit| hand[suit].len());
    lengths.iter().all(|&length| length >= 2)
        && lengths.iter().filter(|&&length| length == 2).count() <= 1
}

/// Balanced shape (the [`balanced`] constraint)
#[derive(Clone)]
struct Balanced;

impl Constraint for Balanced {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        crisp(is_balanced(hand))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::PROFILE)
    }

    fn describe(&self) -> Description {
        Description::atom("balanced")
    }

    fn project(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // The forward reading was historically ⊤ (only the band was boxed), so
        // the exact union is new precision — knob-gated; knob-off stays ⊤.
        if profile.envelope_union() {
            balanced_union(profile)
        } else {
            EnvelopeUnion::unknown()
        }
    }

    fn project_band(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // 4333, 4432, or 5332 — exactly: the {2..=4}⁴ cube (4333/4432, the
        // 13-card sum excludes the rest) plus four 5(332) pan-handles, born
        // via `disjoin` so knob-off they hull to the historical
        // every-suit-2..=5 box, byte-identically.
        balanced_union(profile)
    }

    fn project_complement(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // Unbalanced, exactly: a singleton-or-void suit, a six-plus suit, or
        // two suits of 5+/4+ (5422 and the 5-4/5-5 family the first two boxes
        // miss) — 4 + 4 + 12 boxes, pinned exhaustively against
        // `!is_balanced` over the length lattice.  New precision (the default
        // was ⊤), so the whole reading sits behind the knob.
        if !profile.envelope_union() {
            return EnvelopeUnion::unknown();
        }
        let extremes = Suit::ASC.into_iter().flat_map(|suit| {
            [
                long_suit_box(suit, Range::new(0, 1), Range::FULL_LENGTH),
                long_suit_box(
                    suit,
                    Range::new(6, Range::FULL_LENGTH.max),
                    Range::FULL_LENGTH,
                ),
            ]
        });
        let two_suiters = Suit::ASC.into_iter().flat_map(|a| {
            Suit::ASC
                .into_iter()
                .filter(move |&b| a != b)
                .map(move |b| {
                    let mut lengths = [Range::FULL_LENGTH; 4];
                    lengths[a as usize] = Range::new(5, Range::FULL_LENGTH.max);
                    lengths[b as usize] = Range::new(4, Range::FULL_LENGTH.max);
                    length_box(lengths)
                })
        });
        extremes
            .chain(two_suiters)
            .map(EnvelopeUnion::from)
            .reduce(|a, b| a.disjoin_with(b, profile))
            .unwrap_or_else(EnvelopeUnion::unknown)
    }
}

/// The exact 5-box `balanced` union: the `{2..=4}⁴` cube (4333/4432) plus four
/// 5(332) pan-handles, one per five-card suit
fn balanced_boxes() -> [Envelope; 5] {
    let mut boxes = [length_box([Range::new(2, 4); 4]); 5];
    for suit in Suit::ASC {
        boxes[1 + suit as usize] = long_suit_box(suit, Range::new(5, 5), Range::new(2, 3));
    }
    boxes
}

/// [`balanced_boxes`] folded through [`EnvelopeUnion::disjoin`]: knob-off the single
/// 2..=5 hull, knob-on the exact 5-box union
fn balanced_union(profile: ReadingProfile) -> EnvelopeUnion {
    balanced_boxes()
        .into_iter()
        .map(EnvelopeUnion::from)
        .reduce(|a, b| a.disjoin_with(b, profile))
        .unwrap_or_else(EnvelopeUnion::unknown)
}

/// A pure length box: per-suit ranges in [`Suit::ASC`] order (♣ ♦ ♥ ♠),
/// strength unconstrained
pub(crate) fn length_box(lengths: [Range; 4]) -> Envelope {
    let mut envelope = Envelope::unknown();
    envelope.lengths = lengths;
    envelope
}

/// A one-long-suit box: `suit` in `long`, every other suit in `rest` — the
/// "thin pan-handle" beside a shape union's cube
pub(crate) fn long_suit_box(suit: Suit, long: Range, rest: Range) -> Envelope {
    let mut envelope = length_box([rest; 4]);
    envelope.lengths[suit as usize] = long;
    envelope
}

/// Balanced shape: 4333, 4432, or 5332
#[must_use]
pub fn balanced() -> Cons<impl Constraint + Clone> {
    Cons(Balanced)
}

/// A pure shape predicate as an explicit union of length boxes (the [`shapes`]
/// constraint)
#[derive(Clone)]
struct Shapes {
    label: String,
    boxes: Vec<Envelope>,
}

impl Constraint for Shapes {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        crisp(self.boxes.iter().any(|envelope| envelope.accepts(hand)))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND | ConstraintDependencies::PROFILE
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::new(
            ConstraintDependencies::PROFILE,
            ConstraintDependencies::PROFILE,
            ConstraintDependencies::NONE,
            ConstraintDependencies::PROFILE,
        )
    }

    fn describe(&self) -> Description {
        Description::atom(self.label.clone())
    }

    fn project(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // New precision over the `described` closures this replaces (their
        // projection was ⊤), so the union sits behind the knob — knob-off
        // reads ⊤ exactly as the closure arm did.
        if profile.envelope_union() {
            self.boxes
                .iter()
                .map(|&envelope| EnvelopeUnion::from(envelope))
                .reduce(EnvelopeUnion::union)
                .unwrap_or_else(EnvelopeUnion::unknown)
        } else {
            EnvelopeUnion::unknown()
        }
    }
}

/// A pure shape predicate as an explicit, exact union of length boxes
///
/// The envelope-union-native replacement for a `described` shape closure:
/// every pure shape predicate is a finite subset of the 13-card length lattice, hence an
/// exact finite union of boxes — usually a bigger cube plus thin pan-handles
/// ([`long_suit_box`]), with the 13-card sum doing the excluding.  Evaluates
/// as strict box membership; describes as the bare `label` (no axis nouns,
/// like the closures this replaces); projects ⊤ knob-off and the exact union
/// knob-on.
pub(crate) fn shapes(
    label: impl Into<String>,
    boxes: Vec<Envelope>,
) -> Cons<impl Constraint + Clone> {
    Cons(Shapes {
        label: label.into(),
        boxes,
    })
}

/// The exact staircase union for "`a` outnumbers `b` by at least `gap`":
/// `∪ₖ {b ≤ k, a ≥ k + gap}` over `k = 0..=6`
///
/// Every feasible hand with `a ≥ b + gap` lands on the step `k = b` (two
/// suits of seven don't fit in thirteen cards, so `b ≤ 6`), and each fat step
/// only contains hands satisfying the comparison, so the union is **exact** —
/// the axis-aligned cover of a triangle in the length lattice.
fn staircase(a: Suit, b: Suit, gap: u8) -> Vec<Envelope> {
    (0..=6u8)
        .map(|k| {
            let mut lengths = [Range::FULL_LENGTH; 4];
            lengths[b as usize] = Range::new(0, k);
            lengths[a as usize] = Range::new(k + gap, Range::FULL_LENGTH.max);
            length_box(lengths)
        })
        .collect()
}

/// `a` strictly longer than `b` — an exact [`staircase`] of length boxes
///
/// The envelope-union-native replacement for the "`a` longer than `b`"
/// `described` closures; identical label and (exhaustively pinned) identical eval.
pub(crate) fn longer_suit(a: Suit, b: Suit) -> Cons<impl Constraint + Clone> {
    shapes(format!("{a} longer than {b}"), staircase(a, b, 1))
}

/// `a` at least as long as `b` — the non-strict [`staircase`]
pub(crate) fn at_least_as_long(a: Suit, b: Suit) -> Cons<impl Constraint + Clone> {
    shapes(format!("{a} at least as long as {b}"), staircase(a, b, 0))
}

/// `suit` is the longest suit outside `excluded`, an equal-length tie going to
/// the higher rank
///
/// Crisp, and a partition: of the three suits outside `excluded` exactly one
/// satisfies its instance — `suit` must strictly out-length a higher-ranking
/// rival (which would win the tie) and at least equal a lower-ranking one.  The
/// excluded suit — theirs when advancing a takeout double, partner's when
/// responding to a weak two — never competes.  An exact [`shapes`] union, one
/// box per own-length floor `k` (`suit` ≥ k, each rival capped at k or k−1 by
/// rank; `k ≤ 7` suffices — two suits of eight don't fit in thirteen cards), so
/// knob-on the reading pins the relative-length claim and knob-off it stays ⊤,
/// leaving the companion `len` floor as the whole legacy reading.
pub(crate) fn longest_unbid(suit: Suit, excluded: Suit) -> Cons<impl Constraint + Clone> {
    let boxes = (0..=7u8)
        .filter_map(|k| {
            let mut lengths = [Range::FULL_LENGTH; 4];
            lengths[suit as usize] = Range::new(k, Range::FULL_LENGTH.max);
            for rival in Suit::ASC {
                if rival == suit || rival == excluded {
                    continue;
                }
                let cap = if rival > suit { k.checked_sub(1)? } else { k };
                lengths[rival as usize] = Range::new(0, cap);
            }
            Some(length_box(lengths))
        })
        .collect();
    shapes(format!("{suit} the longest unbid suit"), boxes)
}

/// `a` and `b` of exactly equal length — the lattice diagonal, one thin box
/// per feasible common length (at most six of each)
///
/// `label` keeps the caller's shipped prose ("equal majors",
/// "equal-length majors").
pub(crate) fn equal_length(
    label: impl Into<String>,
    a: Suit,
    b: Suit,
) -> Cons<impl Constraint + Clone> {
    let boxes = (0..=6u8)
        .map(|k| {
            let mut lengths = [Range::FULL_LENGTH; 4];
            lengths[a as usize] = Range::new(k, k);
            lengths[b as usize] = Range::new(k, k);
            length_box(lengths)
        })
        .collect();
    shapes(label, boxes)
}

/// An envelope-union re-authoring of a legacy composite gate, knob-switched at
/// the reading
///
/// Everything the table sees stays the legacy constraint's — `eval` (the
/// weight race), `describe` (disclosure and the ratchet's noun sniffer), and
/// the **knob-off projections**, which reproduce the shipped reading exactly,
/// *including its accidents*: a composite whose `support` legs replay under
/// the reader's seat can read ⊤ or a wrong-suit box depending on where in the
/// auction it is read from, and no static box list reproduces that.  Knob-on,
/// the projections read the exact `boxes` instead — the envelope-union cure.  The
/// authoring invariant that makes the swap sound (legacy-accepted hands lie
/// in some box) is the E0 sweep's `eval ⟹ membership` check.
#[derive(Clone)]
struct EnvelopeUnionUpgrade<T> {
    legacy: T,
    boxes: EnvelopeUnion,
}

impl<T: Constraint> Constraint for EnvelopeUnionUpgrade<T> {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        self.legacy.eval(hand, context)
    }

    fn dependencies(&self) -> ConstraintDependencies {
        self.legacy.dependencies()
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        let legacy = self.legacy.projection_dependencies();
        let boxes = self.boxes.projection_dependencies();
        ProjectionDependencies::new(
            legacy.get(ProjectionKind::Forward)
                | boxes.get(ProjectionKind::Forward)
                | ConstraintDependencies::PROFILE,
            legacy.get(ProjectionKind::Band)
                | boxes.get(ProjectionKind::Band)
                | ConstraintDependencies::PROFILE,
            legacy.get(ProjectionKind::Complement),
            // The default `announce` calls this type's profile-switched
            // `project`.
            legacy.get(ProjectionKind::Forward)
                | boxes.get(ProjectionKind::Forward)
                | ConstraintDependencies::PROFILE,
        )
        .with_purity(legacy.is_pure() && boxes.is_pure())
    }

    fn describe(&self) -> Description {
        self.legacy.describe()
    }

    fn project(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        if profile.envelope_union() {
            self.boxes.project(context)
        } else {
            self.legacy.project(context)
        }
    }

    fn project_band(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        if profile.envelope_union() {
            self.boxes.project_band(context)
        } else {
            self.legacy.project_band(context)
        }
    }

    fn project_complement(&self, context: &Context<'_>) -> EnvelopeUnion {
        self.legacy.project_complement(context)
    }
}

/// Upgrade a legacy composite gate with an exact knob-on box reading (see
/// [`EnvelopeUnionUpgrade`])
pub(crate) fn envelope_union_upgrade(
    legacy: Cons<impl Constraint + Clone>,
    boxes: EnvelopeUnion,
) -> Cons<impl Constraint + Clone> {
    Cons(EnvelopeUnionUpgrade { legacy, boxes })
}

/// Kaplan–Rubens CCCC in a range (the [`cccc`] constraint)
#[derive(Clone)]
struct Cccc<R>(R);

impl<R: RangeBounds<f64> + Clone + Send + Sync> Constraint for Cccc<R> {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        crisp(self.0.contains(&eval::cccc(hand)))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        describe_real_range(&self.0, "CCCC")
    }

    // No `project` override: CCCC is not a bound on `point_count`, so like
    // `support_points` it claims nothing — soundness comes from the `len` /
    // `points` legs that co-gate the call plus rule-replay acceptance.
}

/// [Kaplan–Rubens CCCC][eval::cccc] in the given range
///
/// CCCC weighs honor placement together with shape — honors in long suits
/// count more — which makes it particularly accurate for suit contracts;
/// prefer [`fifths`] toward notrump.
#[must_use]
pub fn cccc(range: impl RangeBounds<f64> + Clone + Send + Sync) -> Cons<impl Constraint + Clone> {
    Cons(Cccc(range))
}

/// [Kaplan–Rubens CCCC][eval::cccc] at least the given strength
#[must_use]
pub fn cccc_at_least(points: f64) -> Cons<impl Constraint + Clone> {
    cccc(points..)
}

/// New Losing Trick Count in a range (the [`nltc`] constraint)
#[derive(Clone)]
struct Nltc<R>(R);

impl<R: RangeBounds<f64> + Clone + Send + Sync> Constraint for Nltc<R> {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        crisp(self.0.contains(&eval::NLTC.eval(hand)))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        describe_real_range(&self.0, "NLTC")
    }

    // No `project` override, same reasoning as `Cccc`.
}

/// [New Losing Trick Count][eval::NLTC] in the given range
///
/// Graded losers (missing A&nbsp;=&nbsp;1.5, K&nbsp;=&nbsp;1, Q&nbsp;=&nbsp;0.5
/// over the first three cards of each suit): *fewer* is stronger, and honors
/// only count where they guard length, so scattered short-suit queens are
/// discounted.  A suit-contract gauge; meaningless toward notrump.
#[must_use]
pub fn nltc(range: impl RangeBounds<f64> + Clone + Send + Sync) -> Cons<impl Constraint + Clone> {
    Cons(Nltc(range))
}

/// Fit for partner's last suit (the [`support`] constraint)
#[derive(Clone)]
struct Support<R>(R);

impl<R: RangeBounds<usize> + Clone + Send + Sync> Constraint for Support<R> {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        crisp(
            context
                .partner_last_suit()
                .is_some_and(|suit| self.0.contains(&hand[suit].len())),
        )
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND | ConstraintDependencies::CONTEXT
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        let live = ConstraintDependencies::CONTEXT | ConstraintDependencies::PROFILE;
        ProjectionDependencies::new(live, live, live, live)
    }

    fn describe(&self) -> Description {
        describe_int_range(&self.0, "card support for partner")
    }

    fn project(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // `support(3..)` is a plain length bound on partner's suit once the
        // auction names it — the same box `len` projects.  The forward reading
        // was historically ⊤, so resolving the suit is new precision and sits
        // behind the knob; knob-off stays ⊤.  With no suit named, `eval`
        // rejects every hand, so ⊤ is (loosely) sound there too.
        if profile.envelope_union() {
            context
                .partner_last_suit()
                .map_or_else(EnvelopeUnion::unknown, |suit| {
                    EnvelopeUnion::from(len_projection(suit, &self.0))
                })
        } else {
            EnvelopeUnion::unknown()
        }
    }

    fn project_complement(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // `!support(4..)` is "at most three of partner's suit" — a box once the
        // auction names the suit.  With no suit named, `eval` rejects every
        // hand, so the negation accepts every hand and ⊤ is the exact reading.
        context
            .partner_last_suit()
            .map_or_else(EnvelopeUnion::unknown, |suit| {
                len_complement(suit, &self.0, profile)
            })
    }
}

/// Support for partner's last bid suit in the given range
///
/// Violated when partner has not bid a suit yet.
pub fn support(
    range: impl RangeBounds<usize> + Clone + Send + Sync,
) -> Cons<impl Constraint + Clone> {
    Cons(Support(range))
}

/// Length partner has shown in a suit (the [`partner_shown_len`] constraint)
#[derive(Clone)]
struct PartnerShownLen<R> {
    suit: Suit,
    range: R,
}

impl<R: RangeBounds<u8> + Clone + Send + Sync> Constraint for PartnerShownLen<R> {
    fn eval(&self, _: Hand, context: &Context<'_>) -> f32 {
        let shown = context.inferences().partner().length(self.suit);
        crisp(self.range.contains(&shown.min))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::CONTEXT
            | ConstraintDependencies::INFERENCES
            | ConstraintDependencies::PROFILE
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        describe_int_range(&self.range, &format!("{} shown by partner", self.suit))
    }
}

/// Partner has shown at least the given length in `suit` (see [`Inferences`])
///
/// Where [`support`] grades *our* fit for partner's last suit, this reads what
/// partner's calls have *promised* in `suit` — the guaranteed minimum length
/// from [`Inferences::read`], tested against `range`.  Comparing the shown
/// minimum (not the maximum) keeps the constraint sound: it fires only on
/// length partner cannot lack.
///
/// [`Inferences`]: super::inference::Inferences
/// [`Inferences::read`]: super::inference::Inferences::read
pub fn partner_shown_len(
    suit: Suit,
    range: impl RangeBounds<u8> + Clone + Send + Sync,
) -> Cons<impl Constraint + Clone> {
    Cons(PartnerShownLen { suit, range })
}

/// Points partner has shown (the [`partner_shown_points`] constraint)
#[derive(Clone)]
struct PartnerShownPoints<R>(R);

impl<R: RangeBounds<u8> + Clone + Send + Sync> Constraint for PartnerShownPoints<R> {
    fn eval(&self, _: Hand, context: &Context<'_>) -> f32 {
        let shown = context.inferences().partner().strength.points;
        crisp(self.0.contains(&shown.min))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::CONTEXT
            | ConstraintDependencies::INFERENCES
            | ConstraintDependencies::PROFILE
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        describe_int_range(&self.0, "points shown by partner")
    }
}

/// Partner has shown at least the given points (see [`partner_shown_len`])
///
/// Reads the guaranteed minimum of partner's shown point range and tests it
/// against `range`, on the same upgraded [`points`] scale.
pub fn partner_shown_points(
    range: impl RangeBounds<u8> + Clone + Send + Sync,
) -> Cons<impl Constraint + Clone> {
    Cons(PartnerShownPoints(range))
}

/// Count of top honors in a suit (the [`top_honors`] constraint)
#[derive(Clone)]
struct TopHonors<R> {
    suit: Suit,
    range: R,
}

impl<R: RangeBounds<usize> + Clone + Send + Sync> Constraint for TopHonors<R> {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        let holding = hand[self.suit];
        let count = [Rank::A, Rank::K, Rank::Q]
            .into_iter()
            .filter(|&rank| holding.contains(rank))
            .count();
        crisp(self.range.contains(&count))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::new(
            ConstraintDependencies::PROFILE,
            ConstraintDependencies::PROFILE,
            ConstraintDependencies::NONE,
            ConstraintDependencies::PROFILE,
        )
    }

    fn describe(&self) -> Description {
        describe_int_range(&self.range, &format!("of the top honors in {}", self.suit))
    }

    fn project(&self, context: &Context<'_>) -> EnvelopeUnion {
        let profile = context.reading_profile();
        // `n` of A/K/Q needs `n` cards in the suit and at least the cheapest
        // `n` honors — Q, then +K, then +A: HCP floors 2/5/9, on both the
        // whole-hand gauge and the suit's own `suit_hcp` axis.  Still an
        // over-approximation (the box admits any cards worth that much in the
        // suit — AJ passes a `top_honors(2..)` box), which is all soundness
        // asks of a projection.  A top-honor ceiling pins nothing, so the
        // default `project_band` (= this) is already the band.  New precision
        // (the default was ⊤) — knob-gated.
        if !profile.envelope_union() {
            return EnvelopeUnion::unknown();
        }
        let n = bound_range(&self.range, 3).min;
        if n == 0 {
            return EnvelopeUnion::unknown();
        }
        let mut envelope = long_suit_box(
            self.suit,
            Range::new(n, Range::FULL_LENGTH.max),
            Range::FULL_LENGTH,
        );
        envelope.strength.hcp = Range::new([0, 2, 5, 9][n as usize], Range::FULL_POINTS.max);
        // The same cheapest-honors floor lands on the suit's own HCP axis,
        // where it is far tighter: the honors live *in this suit*.
        envelope.strength.suit_hcp[self.suit as usize] =
            Range::new([0, 2, 5, 9][n as usize], SUIT_HCP_CAP);
        EnvelopeUnion::from(envelope)
    }
}

/// Count of top honors (A, K, Q) in the given suit, in the given range
///
/// Suit quality for preempts, positives, and asking bids: "two of the top
/// three honors" is `top_honors(suit, 2..)`.
pub fn top_honors(
    suit: Suit,
    range: impl RangeBounds<usize> + Clone + Send + Sync,
) -> Cons<impl Constraint + Clone> {
    Cons(TopHonors { suit, range })
}

/// Whether a holding stops the suit for notrump purposes
///
/// The crisp textbook definition: A, Kx, Qxx, or Jxxx.
pub(crate) const fn has_stopper(holding: Holding) -> bool {
    holding.contains(Rank::A)
        || (holding.contains(Rank::K) && holding.len() >= 2)
        || (holding.contains(Rank::Q) && holding.len() >= 3)
        || (holding.contains(Rank::J) && holding.len() >= 4)
}

/// A stopper in a specific suit (the [`stopper_in`] constraint)
#[derive(Clone)]
struct StopperIn(Suit);

impl Constraint for StopperIn {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        crisp(has_stopper(hand[self.0]))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        Description::atom(format!("stopper in {}", self.0))
    }
}

/// A stopper in the given suit
///
/// The same crisp textbook definition as [`stopper_in_their_suits`]: A, Kx,
/// Qxx, or Jxxx.
#[must_use]
pub fn stopper_in(suit: Suit) -> Cons<impl Constraint + Clone> {
    Cons(StopperIn(suit))
}

/// A stopper in every suit the opponents bid (the
/// [`stopper_in_their_suits`] constraint)
#[derive(Clone)]
struct StopperInTheirSuits;

impl Constraint for StopperInTheirSuits {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        crisp(context.their_suits().all(|suit| has_stopper(hand[suit])))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND | ConstraintDependencies::CONTEXT
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        Description::atom("stopper in their suit(s)")
    }
}

/// A stopper in every suit the opponents have bid
///
/// Trivially satisfied when the opponents have bid no suit.
#[must_use]
pub fn stopper_in_their_suits() -> Cons<impl Constraint + Clone> {
    Cons(StopperInTheirSuits)
}

/// The opponents have bid a strain (the [`they_bid`] constraint)
#[derive(Clone)]
struct TheyBid(Strain);

impl Constraint for TheyBid {
    fn eval(&self, _: Hand, context: &Context<'_>) -> f32 {
        crisp(context.they_bid(self.0))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::CONTEXT
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        Description::atom(format!("opponents bid {}", self.0))
    }
}

/// The opponents have bid the given strain
#[must_use]
pub fn they_bid(strain: Strain) -> Cons<impl Constraint + Clone> {
    Cons(TheyBid(strain))
}

/// Takeout shape against their suits (the [`short_in_their_suits`] constraint)
#[derive(Clone)]
struct ShortInTheirSuits;

impl Constraint for ShortInTheirSuits {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        crisp(context.their_suits().all(|suit| hand[suit].len() <= 3))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND | ConstraintDependencies::CONTEXT
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        Description::atom("at most three cards in each of their suits")
    }
}

/// Takeout shape: at most three cards in each suit the opponents have bid
///
/// Trivially satisfied when the opponents have bid no suit.
#[must_use]
pub fn short_in_their_suits() -> Cons<impl Constraint + Clone> {
    Cons(ShortInTheirSuits)
}

std::thread_local! {
    /// Whether [`takeout_double_shape_ok`] routes a weak flat 4-3-3-3 to Pass
    static SUPPRESS_FLAT_4333_TAKEOUT: Cell<bool> = const { Cell::new(true) };
    /// Whether [`takeout_double_shape_ok`] routes a weak 5-3-3-2 (12–13 HCP) to
    /// its natural overcall instead of a takeout double — bid the five-card suit.
    /// **Shipped default-on** (a 5-3-3-2 holds no 4-card suit, so the double
    /// cannot find a 4-4 fit — its whole purpose is moot).
    static SUPPRESS_5332_TAKEOUT: Cell<bool> = const { Cell::new(true) };
    /// Whether [`takeout_double_shape_ok`] routes a weak 4-4-3-2 (12–13 HCP) to
    /// Pass **when the opponents opened a major**: they have announced a fit, so
    /// our minimum double is outgunned and partner is forced to the two level
    /// (anchor split: the worst 4-4-3-2 slice, −3.2 to −3.8 IMPs/div, and one
    /// unbid 4-card major does not rescue it).
    static SUPPRESS_4432_VS_MAJOR: Cell<bool> = const { Cell::new(false) };
    /// Whether [`takeout_double_shape_ok`] routes a weak 4-4-3-2 (12–13 HCP) to
    /// Pass **when the opponents opened a minor** — the classic "double the minor
    /// with the majors", the mildest 4-4-3-2 slice (−1.39 IMPs/div; the 4-4-majors
    /// subset a wash).  Likely kept; here for the opener-suit A/B.
    static SUPPRESS_4432_VS_MINOR: Cell<bool> = const { Cell::new(false) };
    /// Whether [`takeout_double_shape_ok`] routes a hand with an unbid five-card
    /// **major** to its natural overcall instead of a takeout double — show the
    /// major directly rather than doubling and risking partner bidding our short
    /// suit.  **Shipped default-on** (only the 12–16 HCP shapely double is
    /// redirected; 17+ hands fall through to the separate `points(17..)` rule).
    static SUPPRESS_5CARD_MAJOR_TAKEOUT: Cell<bool> = const { Cell::new(true) };
}

/// Suppress our takeout double on a flat 4-3-3-3 weaker than a 1NT opening
///
/// **Shipped default-on**: a flat 4-3-3-3 has no ruffing value, so a takeout
/// double on 12–14 HCP flat 4333 overbids.  [`takeout_double_shape_ok`] rejects
/// those hands so they route to Pass instead.  A paired BBA A/B (409.6k bd/arm/
/// vul, SEED_BASE 1783443667) scored it a plain-DD **and** perfect-defense win
/// at both vulnerabilities, every 95% CI excluding 0: plain +0.0187 (NV) /
/// +0.0385 (vul), PD +0.0566 / +0.0755 IMPs/board; ~1.2% fired.  Pass `false`
/// to revert to doubling.  Read at classification time and per-thread — the flag
/// is consulted for books built after this call; classify on the thread that set
/// it.
#[doc(hidden)]
pub fn set_suppress_flat_4333_takeout(on: bool) {
    SUPPRESS_FLAT_4333_TAKEOUT.with(|flag| flag.set(on));
}

/// Whether the weak-flat-4333 takeout suppression is active
fn suppress_flat_4333_takeout() -> bool {
    SUPPRESS_FLAT_4333_TAKEOUT.with(Cell::get)
}

/// Suppress our takeout double on a weak 5-3-3-2 — bid the five-card suit instead
///
/// **Shipped default-on.**  A 12–13 HCP 5-3-3-2 holds *no* 4-card suit, hence no
/// 4-card major, so a takeout double cannot do its job — find a 4-4 fit; it just
/// buries the unbid five-card suit.  With the knob on, [`takeout_double_shape_ok`]
/// rejects the double so the hand routes to its natural overcall, matching BBA.
/// A paired BBA A/B (409.6k bd/arm/vul, SEED_BASE 1783451581) scored the 5-3-3-2
/// half a plain-DD **and** perfect-defense win at both vulnerabilities, every
/// 95% CI excluding 0: plain +0.0191 (NV) / +0.0401 (vul), PD +0.0601 / +0.0773
/// IMPs/board; ~1.2% fired.  Pass `false` to revert to doubling.  Read at
/// classification time and per-thread, like its 4333 sibling.
#[doc(hidden)]
pub fn set_suppress_5332_takeout(on: bool) {
    SUPPRESS_5332_TAKEOUT.with(|flag| flag.set(on));
}

/// Whether the weak-5332 takeout suppression is active
fn suppress_5332_takeout() -> bool {
    SUPPRESS_5332_TAKEOUT.with(Cell::get)
}

/// Suppress our weak 4-4-3-2 takeout double when the opponents opened a **major**
///
/// A 12–13 HCP 4-4-3-2 short in the opponents' suit is a takeout shape, but the
/// anchor split (opener = the takeout-short suit) shows the loss lives over
/// **major** openings — −3.2 to −3.8 IMPs/div whether or not we hold the one
/// unbid 4-card major, because the opponents have announced a fit and our
/// minimum double gets outgunned, partner forced to the two level.  With the
/// knob on, [`takeout_double_shape_ok`] rejects the double so the hand routes to
/// Pass.  **Default off** pending the opener-suit A/B; pass `true` to enable.
/// Read at classification time and per-thread.
#[doc(hidden)]
pub fn set_suppress_4432_vs_major(on: bool) {
    SUPPRESS_4432_VS_MAJOR.with(|flag| flag.set(on));
}

/// Whether the weak-4432-over-a-major takeout suppression is active
fn suppress_4432_vs_major() -> bool {
    SUPPRESS_4432_VS_MAJOR.with(Cell::get)
}

/// Suppress our weak 4-4-3-2 takeout double when the opponents opened a **minor**
///
/// The mildest 4-4-3-2 slice (−1.39 IMPs/div; the 4-4-majors subset a wash) — the
/// classic takeout of a minor showing the majors, which is textbook and likely
/// kept.  Provided for the opener-suit A/B; **default off**.  Read at
/// classification time and per-thread.
#[doc(hidden)]
pub fn set_suppress_4432_vs_minor(on: bool) {
    SUPPRESS_4432_VS_MINOR.with(|flag| flag.set(on));
}

/// Whether the weak-4432-over-a-minor takeout suppression is active
fn suppress_4432_vs_minor() -> bool {
    SUPPRESS_4432_VS_MINOR.with(Cell::get)
}

/// Suppress our takeout double when we hold an unbid five-card major — overcall it
///
/// With a five-card (or longer) major we can name the suit directly, so a takeout
/// double only risks partner responding in our short suit.  Over a one-level
/// opening the natural major overcall already outranks the double; the leak is
/// over a **weak two**, where the 12+ shapely double (weight 1.3) outguns the
/// two-level major overcall (weight 1.0).  With the knob on (the default),
/// [`takeout_double_shape_ok`] rejects the double so the hand routes to its
/// natural overcall — only the 12–16 HCP range is redirected, since a 17+ hand
/// falls through to the separate `points(17..)` double (too strong for a simple
/// overcall).  **Shipped default-on**: a paired BBA A/B (409.6k bd/arm/vul,
/// SEED_BASE 1783631820) scored a plain-DD **and** perfect-defense **and**
/// single-dummy-lead win at both vulnerabilities, every 95% CI excluding 0: plain
/// +0.0190 (NV) / +0.0493 (vul), PD +0.0892 / +0.1129, sd-lead +0.0124 / +0.0413
/// IMPs/board; ~2% fired.  Pass `false` to revert to doubling.  Read at
/// classification time and per-thread.
#[doc(hidden)]
pub fn set_suppress_5card_major_takeout(on: bool) {
    SUPPRESS_5CARD_MAJOR_TAKEOUT.with(|flag| flag.set(on));
}

/// Whether the unbid-five-card-major takeout suppression is active
fn suppress_5card_major_takeout() -> bool {
    SUPPRESS_5CARD_MAJOR_TAKEOUT.with(Cell::get)
}

/// Gate ANDed into each takeout-double rule to suppress a weak flat 4-3-3-3
///
/// A no-op unless [`set_suppress_flat_4333_takeout`] is on (the default): when
/// off it is satisfied for every hand, reverting to the old double.  When on it
/// is satisfied *unless* the hand is a flat 4-3-3-3 with fewer than 15 HCP (12–14),
/// which a takeout double overbids for lack of ruffing value — those hands route
/// to Pass instead.  Four suits all 3 or 4 cards long sum to 13 only as a
/// 4-3-3-3, so that test *is* "flat 4333".  The flag is read once at
/// construction, so the closure captures a `bool`.
#[must_use]
pub(crate) fn takeout_double_shape_ok() -> Cons<impl Constraint + Clone> {
    let suppress_4333 = suppress_flat_4333_takeout();
    let suppress_5332 = suppress_5332_takeout();
    let suppress_4432_major = suppress_4432_vs_major();
    let suppress_4432_minor = suppress_4432_vs_minor();
    let suppress_5card_major = suppress_5card_major_takeout();
    described(
        "not a weak balanced hand diverted to Pass",
        move |hand: Hand, context: &Context<'_>| {
            let mut lens = [0usize; 4];
            for (slot, suit) in Suit::ASC.into_iter().enumerate() {
                lens[slot] = hand[suit].len();
            }
            lens.sort_unstable_by(|a, b| b.cmp(a));
            let hcp = raw_hcp(hand);
            // Unbid five-card major: overcall it rather than double (doubling
            // buries the major and risks partner bidding our short suit).
            let reject_5card_major = suppress_5card_major
                && Suit::ASC.into_iter().any(|suit| {
                    Strain::from(suit).is_major()
                        && hand[suit].len() >= 5
                        && !context.their_suits().any(|their| their == suit)
                });
            // Flat 4-3-3-3: no doubleton at all — suppressed 12–14 (its own knob).
            let reject_4333 = suppress_4333 && lens == [4, 3, 3, 3] && hcp < 15;
            // 5-3-3-2: bid the five-card suit instead of doubling — 12–13.
            let reject_5332 = suppress_5332 && lens == [5, 3, 3, 2] && hcp < 14;
            // 4-4-3-2, split by what the opponents opened (real auction context,
            // not inferred): the loss lives over major openings — 12–13.
            let their_major = context
                .their_suits()
                .any(|suit| Strain::from(suit).is_major());
            let reject_4432 = lens == [4, 4, 3, 2]
                && hcp < 14
                && (if their_major {
                    suppress_4432_major
                } else {
                    suppress_4432_minor
                });
            !(reject_4333 || reject_5332 || reject_4432 || reject_5card_major)
        },
    )
}

/// Takeout support for the unbid suits (the [`unbid_support`] constraint)
#[derive(Clone)]
struct UnbidSupport {
    max_short: usize,
}

impl Constraint for UnbidSupport {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        let short = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
            .into_iter()
            .filter(|&suit| context.their_suits().all(|theirs| theirs != suit))
            .filter(|&suit| hand[suit].len() < 3)
            .count();
        crisp(short <= self.max_short)
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::HAND | ConstraintDependencies::CONTEXT
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        Description::atom(if self.max_short == 0 {
            "at least three cards in each unbid suit".to_owned()
        } else {
            format!(
                "at most {} unbid suit(s) shorter than three cards",
                self.max_short
            )
        })
    }
}

/// Takeout support: at most `max_short` of the unbid suits hold fewer than three
/// cards
///
/// The companion of [`short_in_their_suits`]: where that gates shortness in the
/// opponents' suit(s), this gates *length* in the suits they have **not** bid —
/// the support a takeout double promises partner.  `max_short == 0` demands 3+ in
/// every unbid suit (a textbook shapely double); `max_short == 1` tolerates one
/// doubleton (admitting 4-4-3-2 and 5-3-3-2 patterns while still rejecting a
/// one-suiter short in two unbid suits, which belongs in the 17+ any-shape tier).
#[must_use]
pub fn unbid_support(max_short: usize) -> Cons<impl Constraint + Clone> {
    Cons(UnbidSupport { max_short })
}

/// Which suit partner bid last (the [`partner_suit_is`] constraint)
#[derive(Clone)]
struct PartnerSuitIs(Suit);

impl Constraint for PartnerSuitIs {
    fn eval(&self, _: Hand, context: &Context<'_>) -> f32 {
        crisp(context.partner_last_suit() == Some(self.0))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::CONTEXT
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        Description::atom(format!("partner's last suit is {}", self.0))
    }
}

/// Partner's last bid suit is the given one
///
/// Violated when partner has not bid a suit yet.  Where [`support`] grades
/// *how well* we fit partner's suit, this pins down *which* suit partner bid
/// last — the anchor for raises of a specific second suit.
#[must_use]
pub fn partner_suit_is(suit: Suit) -> Cons<impl Constraint + Clone> {
    Cons(PartnerSuitIs(suit))
}

/// The cheapest legal level for a strain (the [`min_level_is`] constraint)
#[derive(Clone)]
struct MinLevelIs {
    level: u8,
    strain: Strain,
}

impl Constraint for MinLevelIs {
    fn eval(&self, _: Hand, context: &Context<'_>) -> f32 {
        crisp(context.min_level(self.strain) == Some(Level::new(self.level)))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::CONTEXT
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        Description::atom(format!("{}{} is the cheapest bid", self.level, self.strain))
    }
}

/// The strain's cheapest legal level is exactly the given one
///
/// The legality anchor for rules whose call sits at a dynamic level (cue
/// bids, competitive raises): `min_level_is(2, their_strain)` admits the rule
/// only when the two-level cue is exactly the cheapest available.
#[must_use]
pub fn min_level_is(level: u8, strain: Strain) -> Cons<impl Constraint + Clone> {
    Cons(MinLevelIs { level, strain })
}

/// The actor passed on their first turn (the [`passed_hand`] constraint)
#[derive(Clone)]
struct PassedHand;

impl Constraint for PassedHand {
    fn eval(&self, _: Hand, context: &Context<'_>) -> f32 {
        crisp(context.passed_hand())
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::CONTEXT
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        Description::atom("a passed hand")
    }
}

/// The player to act passed on their first turn
#[must_use]
pub fn passed_hand() -> Cons<impl Constraint + Clone> {
    Cons(PassedHand)
}

/// The opponents have only passed (the [`undisturbed`] constraint)
#[derive(Clone)]
struct Undisturbed;

impl Constraint for Undisturbed {
    fn eval(&self, _: Hand, context: &Context<'_>) -> f32 {
        crisp(context.undisturbed())
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::CONTEXT
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        Description::atom("the opponents have passed throughout")
    }
}

/// The opponents have made nothing but passes
#[must_use]
pub fn undisturbed() -> Cons<impl Constraint + Clone> {
    Cons(Undisturbed)
}

/// Our side is vulnerable (the [`vulnerable`] constraint)
#[derive(Clone)]
struct Vulnerable;

impl Constraint for Vulnerable {
    fn eval(&self, _: Hand, context: &Context<'_>) -> f32 {
        use contract_bridge::auction::RelativeVulnerability;
        crisp(context.vul().contains(RelativeVulnerability::WE))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::CONTEXT
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        Description::atom("vulnerable")
    }
}

/// Our side is vulnerable
#[must_use]
pub fn vulnerable() -> Cons<impl Constraint + Clone> {
    Cons(Vulnerable)
}

/// Two [`points`] bands split by our vulnerability: `nv` white, `vul` red
///
/// Pure sugar for the two-range idiom, expanding to exactly
/// `(points(nv) & !vulnerable()) | (points(vul) & vulnerable())` — the same
/// combinator tree, so eval, the DNF boxes, and the rendered `Description`
/// are byte-for-byte what the long-hand spelling produced.
#[must_use]
pub fn points_by_vul(
    nv: impl RangeBounds<u8> + Clone + Send + Sync,
    vul: impl RangeBounds<u8> + Clone + Send + Sync,
) -> Cons<impl Constraint + Clone> {
    (points(nv) & !vulnerable()) | (points(vul) & vulnerable())
}

/// The opponents are vulnerable (the [`they_vulnerable`] constraint)
#[derive(Clone)]
struct TheyVulnerable;

impl Constraint for TheyVulnerable {
    fn eval(&self, _: Hand, context: &Context<'_>) -> f32 {
        use contract_bridge::auction::RelativeVulnerability;
        crisp(context.vul().contains(RelativeVulnerability::THEY))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::CONTEXT
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        Description::atom("opponents vulnerable")
    }
}

/// The opponents are vulnerable
#[must_use]
pub fn they_vulnerable() -> Cons<impl Constraint + Clone> {
    Cons(TheyVulnerable)
}

/// About to open in a specific seat (the [`nth_seat`] constraint)
#[derive(Clone)]
struct NthSeat(u8);

impl Constraint for NthSeat {
    fn eval(&self, _: Hand, context: &Context<'_>) -> f32 {
        crisp(context.seat_to_open() == Some(self.0))
    }

    fn dependencies(&self) -> ConstraintDependencies {
        ConstraintDependencies::CONTEXT
    }

    fn projection_dependencies(&self) -> ProjectionDependencies {
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    }

    fn describe(&self) -> Description {
        Description::atom(format!("opening in seat {}", self.0))
    }
}

/// About to make the first non-pass call in the given seat (1–4)
///
/// This is the exception mechanism for seat-specific openings (e.g. no
/// preempts in 4th seat); 1st/2nd and 3rd/4th seats are otherwise treated
/// alike structurally.
#[must_use]
pub fn nth_seat(seat: u8) -> Cons<impl Constraint + Clone> {
    Cons(NthSeat(seat))
}

/// Test-only: every composition of 13 into four suit lengths (`[♣, ♦, ♥, ♠]`,
/// the [`Suit::ASC`] order), each realized as a synthetic hand of top cards —
/// the exhaustive ground the shape-union equivalence tests enumerate (560
/// shapes).
#[cfg(test)]
pub(crate) fn for_each_shape(mut f: impl FnMut([u8; 4], Hand)) {
    fn top(n: u8) -> &'static str {
        &"AKQJT98765432"[..n as usize]
    }
    for clubs in 0..=13u8 {
        for diamonds in 0..=13 - clubs {
            for hearts in 0..=13 - clubs - diamonds {
                let spades = 13 - clubs - diamonds - hearts;
                let text = format!(
                    "{}.{}.{}.{}",
                    top(spades),
                    top(hearts),
                    top(diamonds),
                    top(clubs),
                );
                let hand = text.parse().unwrap_or_else(|_| panic!("unparsable {text}"));
                f([clubs, diamonds, hearts, spades], hand);
            }
        }
    }
}

#[cfg(test)]
mod tests;
