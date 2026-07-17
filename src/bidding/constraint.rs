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
use super::inference::{Inference, Inferences, Range};
use contract_bridge::eval::{self, HandEvaluator, SimpleEvaluator};
use contract_bridge::{Hand, Holding, Level, Rank, Strain, Suit};
use core::cell::Cell;
use core::fmt;
use core::ops::{BitAnd, BitOr, Bound, Not, RangeBounds};
use std::borrow::Cow;

/// Trait for a logit contribution of a hand feature
///
/// Implementations must not return `f32::INFINITY`: combining `+∞` with the
/// `-∞` of a violated crisp constraint would produce a NaN.
pub trait Constraint: Send + Sync {
    /// Evaluate the constraint into a logit contribution
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32;

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

    /// Project the constraint into the forward [`Inference`] envelope it implies
    ///
    /// The third fold, beside [`eval`][Self::eval] and [`describe`][Self::describe]:
    /// where `eval` scores one hand and `describe` names the meaning, `project`
    /// turns the constraint into the per-suit length and point ranges that every
    /// hand it accepts must fall within — the bidder's *forward* reading of an
    /// authored call, the dual of evaluating a known hand.  Sound by
    /// construction: a finite `eval(hand, context)` implies `hand` lies within
    /// `project(context)`.  The default asserts nothing
    /// ([`Inference::unknown`]), so an opaque predicate stays sound but loose
    /// until a length- or points-bearing primitive overrides it.
    fn project(&self, _context: &Context<'_>) -> Inference {
        Inference::unknown()
    }

    /// Project the constraint into its **two-sided** [`Inference`] envelope
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
    fn project_band(&self, context: &Context<'_>) -> Inference {
        self.project(context)
    }
}

/// Closures are natural constraints
impl<F> Constraint for F
where
    F: Fn(Hand, &Context<'_>) -> f32 + Send + Sync,
{
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        self(hand, context)
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

    fn describe(&self) -> Description {
        self.0.describe()
    }

    fn project(&self, context: &Context<'_>) -> Inference {
        self.0.project(context)
    }

    fn project_band(&self, context: &Context<'_>) -> Inference {
        self.0.project_band(context)
    }
}

/// Sum of two constraints, the logical AND for crisp constraints
#[derive(Clone, Copy, Debug)]
pub struct And<A, B>(A, B);

impl<A: Constraint, B: Constraint> Constraint for And<A, B> {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        self.0.eval(hand, context) + self.1.eval(hand, context)
    }

    fn describe(&self) -> Description {
        self.0.describe().and(self.1.describe())
    }

    fn project(&self, context: &Context<'_>) -> Inference {
        self.0.project(context).intersect(&self.1.project(context))
    }

    fn project_band(&self, context: &Context<'_>) -> Inference {
        self.0
            .project_band(context)
            .intersect(&self.1.project_band(context))
    }
}

/// Maximum of two constraints, the logical OR for crisp constraints
#[derive(Clone, Copy, Debug)]
pub struct Or<A, B>(A, B);

impl<A: Constraint, B: Constraint> Constraint for Or<A, B> {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        self.0.eval(hand, context).max(self.1.eval(hand, context))
    }

    fn describe(&self) -> Description {
        self.0.describe().or(self.1.describe())
    }

    fn project(&self, context: &Context<'_>) -> Inference {
        self.0.project(context).union(&self.1.project(context))
    }

    fn project_band(&self, context: &Context<'_>) -> Inference {
        self.0
            .project_band(context)
            .union(&self.1.project_band(context))
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

    fn describe(&self) -> Description {
        self.0.describe().negate()
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

/// Which honor-weighted count tempers [`fifths`] (the A/B companion gauge)
///
/// Fifths is tuned for 3NT — it rewards aces and tens and discounts kings and
/// queens — so on its own it misjudges a hand headed for a suit contract.  A
/// notrump-defining range never gauges Fifths alone; it averages Fifths with
/// one of these honor counts, so a tens-rich hand can't reach the band on
/// Fifths and a quack-heavy hand isn't shut out of it.  BUM-RAP is the
/// default — it edged HCP across every vulnerability in the
/// `fifths-companion` A/B match.
#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq)]
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
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PointScale {
    /// Legacy raw HCP + [`upgrade`] (the deposed incumbent, kept opt-in)
    PointCount,
    /// Raw Milton Work 4-3-2-1 HCP (the old `fuzzy_points` off arm)
    Hcp,
    /// Rule of N+8: raw HCP + the two longest suit lengths − 8, so a
    /// `points(12..)` gate is exactly the Rule of 20 (opt-in since the
    /// 4333-floor A/B; its flat downgrade measured worse than the floor)
    RuleOfN,
    /// [`PointScale::RuleOfN`] with the length bonus floored at 0: flat
    /// 4-3-3-3 — plain rule-of-N+8's only downgrade — reads its raw HCP
    /// (the shipped default)
    RuleOfNFloored,
}

std::thread_local! {
    /// The scale [`point_count`] evaluates (the point-scale A/B knob).
    /// **Default [`PointScale::RuleOfNFloored`]**.  The deprecation A/B/C
    /// deposed legacy for rule of N+8 (plain DD +0.031/+0.045 NV/vul, sd-lead
    /// +0.048/+0.064; raw HCP lost plain-DD −0.098/−0.105); the follow-up
    /// 4333-floor A/B then beat plain rule-of-N+8 on the sd-lead tiebreak at
    /// +0.032 ± 0.009 NV / +0.026 ± 0.013 vul (50k boards/vul; 1M-board plain
    /// DD +0.013 NV / wash vul, the campaign's usual PD dip).  Legacy is the
    /// opt-out: `set_point_scale(PointScale::PointCount)`.
    static POINT_SCALE: Cell<PointScale> = const { Cell::new(PointScale::RuleOfNFloored) };
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
}

/// Enable or disable fuzzy strength on the current thread
///
/// For A/B measurement only: with fuzzy strength disabled, [`points`] and
/// [`fifths`] fall back to comparing raw HCP against the same bounds, so one
/// set of books serves as both the baseline and the upgraded system.  The
/// flags are read at classification time and are per-thread; classify on the
/// thread that set them.
#[doc(hidden)]
pub fn set_fuzzy_strength(enabled: bool) {
    set_fuzzy_points(enabled);
    set_fuzzy_fifths(enabled);
}

/// Enable or disable the [`points`] upgrade alone (see [`set_fuzzy_strength`])
///
/// A thin wrapper over [`set_point_scale`] kept for the historical A/B
/// runners: `false` is the raw-HCP arm.
#[doc(hidden)]
pub fn set_fuzzy_points(enabled: bool) {
    set_point_scale(if enabled {
        PointScale::PointCount
    } else {
        PointScale::Hcp
    });
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

/// Enable or disable [`fifths`] alone (see [`set_fuzzy_strength`])
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

/// Raw high card points of a hand
fn raw_hcp(hand: Hand) -> u8 {
    SimpleEvaluator(eval::hcp::<u8>).eval(hand)
}

/// Project a numeric range bound into an inference [`Range`], clamped to `cap`
///
/// The forward dual of [`describe_int_range`]: where that names a bound in
/// prose, this turns it into the `[min, max]` an [`Inference`] records, sharing
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
    // back to the `u8` an `Inference` stores never truncate.
    let clamp = |x: u64| u8::try_from(x.min(cap)).unwrap_or_else(|_| unreachable!());
    Range::new(clamp(min), clamp(max))
}

/// Raw high card points in a range (the [`hcp`] constraint)
#[derive(Clone)]
struct Hcp<R>(R);

impl<R: RangeBounds<u8> + Clone + Send + Sync> Constraint for Hcp<R> {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        crisp(self.0.contains(&raw_hcp(hand)))
    }

    fn describe(&self) -> Description {
        describe_int_range(&self.0, "HCP")
    }

    fn project(&self, _: &Context<'_>) -> Inference {
        // ponytail: floor only — points = raw HCP + upgrade ≥ raw HCP, so an
        // HCP *ceiling* is unsound on the upgraded-points scale an `Inference`
        // records; the floor is exact.  Rule of N+8 reads a flat 4-3-3-3 one
        // under its HCP, so that scale gives the floor back 1.  The ceiling
        // returns in [`project_band`][Constraint::project_band], widened by
        // [`hcp_ceiling_slack`].
        let slack = flat_hcp_slack();
        let floor = bound_range(&self.0, Range::FULL_POINTS.max).min;
        let mut inference = Inference::unknown();
        inference.points = Range::new(floor.saturating_sub(slack), Range::FULL_POINTS.max);
        inference
    }

    fn project_band(&self, context: &Context<'_>) -> Inference {
        // The ceiling an HCP gate owes the upgraded scale: raw HCP plus the
        // scale's maximum upgrade.  The floor half matches `project`.
        let ceiling = bound_range(&self.0, Range::FULL_POINTS.max)
            .max
            .saturating_add(hcp_ceiling_slack())
            .min(Range::FULL_POINTS.max);
        let mut inference = self.project(context);
        inference.points.max = ceiling;
        inference
    }
}

/// Total high card points in the given range
#[must_use]
pub fn hcp(range: impl RangeBounds<u8> + Clone + Send + Sync) -> Cons<impl Constraint + Clone> {
    Cons(Hcp(range))
}

/// The slack an HCP-gated point envelope owes the current scale: rule of N+8
/// reads a flat 4-3-3-3 one under its HCP; the other scales never read under.
/// Shared by [`hcp`]'s projection and the hand-authored NT-opening readings.
pub(crate) fn flat_hcp_slack() -> u8 {
    u8::from(POINT_SCALE.with(Cell::get) == PointScale::RuleOfN)
}

/// The most the active scale's [`point_count`] can exceed raw HCP — the
/// ceiling dual of [`flat_hcp_slack`]: rule of N+8 adds up to
/// `longest_two_suits − 8` ≤ 5, the legacy upgrade at most 2, plain HCP
/// nothing.  Widens an HCP gate's ceiling in
/// [`project_band`][Constraint::project_band].
fn hcp_ceiling_slack() -> u8 {
    match POINT_SCALE.with(Cell::get) {
        PointScale::Hcp => 0,
        PointScale::PointCount => 2,
        PointScale::RuleOfN | PointScale::RuleOfNFloored => 5,
    }
}

/// Whether a short suit (at most two cards) blocks the fuzzy-strength upgrade
///
/// Honors in shortness are wasted: any of A/K/Q/J in a suit of at most two
/// cards blocks the upgrade, except the working holdings Ax and Kx.
const fn blocks_upgrade(holding: Holding) -> bool {
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
/// voids the whole upgrade, except the working holdings Ax and Kx.
#[must_use]
pub fn upgrade(hand: Hand) -> u8 {
    let holdings = Suit::ASC.map(|suit| hand[suit]);

    if holdings.iter().any(|&holding| blocks_upgrade(holding)) {
        return 0;
    }

    u8::from(!is_balanced(hand)) + u8::from(longest_two_suits(hand) >= 10)
}

/// Total length of the two longest suits — the shape kernel shared by
/// [`upgrade`], [`rule_of_20`], and the rule-of-N+8 [`PointScale`]
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
/// Defaults to the **floored rule-of-N+8 scale** — raw HCP plus the two
/// longest suit lengths minus 8, the length bonus never negative, so
/// `points(12..)` is exactly the Rule of 20 and flat 4-3-3-3 reads its raw
/// HCP (see [`PointScale`] for the measured verdicts; the legacy
/// raw-HCP-plus-[`upgrade`] scale is the opt-out).  A reader that needs the
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
    match POINT_SCALE.with(Cell::get) {
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
struct Points<R>(R);

impl<R: RangeBounds<u8> + Clone + Send + Sync> Constraint for Points<R> {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        // Always the shared scalar, whatever scale it is set to — the
        // sampler's soundness invariant (it measures the same number) holds
        // on every arm of the point-scale A/B.
        crisp(self.0.contains(&point_count(hand)))
    }

    fn describe(&self) -> Description {
        describe_int_range(&self.0, "points")
    }

    fn project(&self, _: &Context<'_>) -> Inference {
        // Floor only, matching every hand-written reader (`at_least(floor,
        // CAP)`): sound whether or not the fuzzy-strength upgrade is on, since
        // the upgraded point count is never below the band's floor.
        let floor = bound_range(&self.0, Range::FULL_POINTS.max).min;
        let mut inference = Inference::unknown();
        inference.points = Range::new(floor, Range::FULL_POINTS.max);
        inference
    }

    fn project_band(&self, _: &Context<'_>) -> Inference {
        // Both bounds exact: `points` gauges the shared `point_count` scalar
        // the `Inference` scale records, whatever scale it is set to.
        let mut inference = Inference::unknown();
        inference.points = bound_range(&self.0, Range::FULL_POINTS.max);
        inference
    }
}

/// [`point_count`] in the given range
///
/// The strength gauge for suit-oriented calls.  Notrump-defining ranges use
/// [`fifths`] instead, and ranges indifferent to shape keep [`hcp`].
#[must_use]
pub fn points(range: impl RangeBounds<u8> + Clone + Send + Sync) -> Cons<impl Constraint + Clone> {
    Cons(Points(range))
}

/// [`point_count`] on the fit-known shortness scale when [`set_support_points`]
/// is on, else legacy [`point_count`] — the value-level dual of [`support_points`]
///
/// Only fit-known gates gauge this: a trump fit is known, so counting shortness
/// as support value is sound.  The flag-off default equals [`point_count`], so a
/// gate swapped from [`points`] to [`support_points`] is byte-identical by
/// default.
#[must_use]
pub fn support_point_count(hand: Hand) -> u8 {
    if SUPPORT_POINTS.with(Cell::get) {
        new_point_count(hand)
    } else {
        point_count(hand)
    }
}

/// [`support_point_count`] in a range (the [`support_points`] constraint)
#[derive(Clone)]
struct SupportPoints<R>(R);

impl<R: RangeBounds<u8> + Clone + Send + Sync> Constraint for SupportPoints<R> {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        crisp(self.0.contains(&support_point_count(hand)))
    }

    fn describe(&self) -> Description {
        describe_int_range(&self.0, "support points")
    }

    fn project(&self, _: &Context<'_>) -> Inference {
        // ponytail: unknown() until the sampler needs the floor; see
        // docs/ai-bidder/rule-projection.md.  With the flag on this reads the new
        // scale while every other gate's ranges are recorded on legacy
        // `point_count`, and `new_point_count` is not a lower bound on it (graded
        // shortness can exceed the coarse `upgrade`), so projecting a floor the
        // way `Points::project` does would be unsound.  Claim nothing.
        Inference::unknown()
    }
}

/// [`support_point_count`] in the given range — the fit-known counterpart to
/// [`points`]
///
/// Wire this into a gate only when a trump fit is known; it counts shortness as
/// support value, unsound before a fit.  The invariant is grep-able:
/// `support_points` in a gate ⟹ a fit is known.
#[must_use]
pub fn support_points(
    range: impl RangeBounds<u8> + Clone + Send + Sync,
) -> Cons<impl Constraint + Clone> {
    Cons(SupportPoints(range))
}

/// Fifths in a range (the [`fifths`] constraint)
#[derive(Clone)]
struct Fifths<R>(R);

impl<R: RangeBounds<f64> + Clone + Send + Sync> Constraint for Fifths<R> {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        let value = if FUZZY_FIFTHS.with(Cell::get) {
            // Never Fifths alone: average it with a real-honor count so the
            // 3NT-tuned tens/aces bias is halved toward Milton Work / BUM-RAP.
            let companion = match FIFTHS_COMPANION.with(Cell::get) {
                FifthsCompanion::Hcp => f64::from(raw_hcp(hand)),
                FifthsCompanion::Bumrap => eval::BUMRAP.eval(hand),
            };
            f64::midpoint(eval::FIFTHS.eval(hand), companion)
        } else {
            f64::from(raw_hcp(hand))
        };
        crisp(self.0.contains(&value))
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
// shown (1NT–2NT, 1NT–3NT) is the one place pure Fifths is fine, but those
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

    fn describe(&self) -> Description {
        describe_int_range(&self.range, &self.suit.to_string())
    }

    fn project(&self, _: &Context<'_>) -> Inference {
        // Length is exact — the same `hand[suit].len()` `eval` checks — so both
        // bounds project soundly.
        len_projection(self.suit, &self.range)
    }
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
fn len_projection<R: RangeBounds<usize>>(suit: Suit, range: &R) -> Inference {
    let mut inference = Inference::unknown();
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

    fn describe(&self) -> Description {
        self.suits
            .iter()
            .map(|suit| describe_int_range(&self.range, &suit.to_string()))
            .reduce(|a, b| a.and(b))
            .unwrap_or(Description::Opaque)
    }

    fn project(&self, _: &Context<'_>) -> Inference {
        // Every named suit is floored to `range` (the same exact `len` check), so
        // the projection intersects each suit's bound — sound *and* tight.
        self.suits
            .iter()
            .map(|&suit| len_projection(suit, &self.range))
            .reduce(|acc, inf| acc.intersect(&inf))
            .unwrap_or_else(Inference::unknown)
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

    fn describe(&self) -> Description {
        self.suits
            .iter()
            .map(|suit| describe_int_range(&self.range, &suit.to_string()))
            .reduce(|a, b| a.or(b))
            .unwrap_or(Description::Opaque)
    }

    fn project(&self, _: &Context<'_>) -> Inference {
        // At least one named suit lies in `range`, but not which — the sound
        // envelope is the union of the arms, which widens every suit back to full
        // unless exactly one suit is named (then it floors exactly, like `len`).
        self.suits
            .iter()
            .map(|&suit| len_projection(suit, &self.range))
            .reduce(|acc, inf| acc.union(&inf))
            .unwrap_or_else(Inference::unknown)
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
        crisp(self.range.contains(&eval::hcp::<u8>(hand[self.suit])))
    }

    fn describe(&self) -> Description {
        describe_int_range(&self.range, &format!("HCP in {}", self.suit))
    }
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

    fn describe(&self) -> Description {
        Description::atom("balanced")
    }

    fn project_band(&self, _: &Context<'_>) -> Inference {
        // 4333, 4432, or 5332: every suit two to five cards.
        let mut inference = Inference::unknown();
        inference.lengths = [Range::new(2, 5); 4];
        inference
    }
}

/// Balanced shape: 4333, 4432, or 5332
#[must_use]
pub fn balanced() -> Cons<impl Constraint + Clone> {
    Cons(Balanced)
}

/// Rule-of-Twenty kernel: raw HCP plus the two longest suit lengths total ≥ 20
fn is_rule_of_20(hand: Hand) -> bool {
    raw_hcp(hand) + longest_two_suits(hand) >= 20
}

/// Rule of 20 shape (the [`rule_of_20`] constraint)
#[derive(Clone)]
struct RuleOf20;

impl Constraint for RuleOf20 {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        crisp(is_rule_of_20(hand))
    }

    fn describe(&self) -> Description {
        Description::atom("Rule of 20")
    }
}

/// Rule of 20: raw HCP plus the two longest suits total at least 20
///
/// The classic light-opening test for a borderline 10–11 count with enough
/// shape to open one of a suit.  Gauges *raw* HCP, not upgraded [`points`]:
/// the upgrade voids on a wasted short-suit honor, which would reject exactly
/// the shapely hands this rule is meant to admit.
#[must_use]
pub fn rule_of_20() -> Cons<impl Constraint + Clone> {
    Cons(RuleOf20)
}

/// Kaplan–Rubens CCCC in a range (the [`cccc`] constraint)
#[derive(Clone)]
struct Cccc<R>(R);

impl<R: RangeBounds<f64> + Clone + Send + Sync> Constraint for Cccc<R> {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        crisp(self.0.contains(&eval::cccc(hand)))
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

    fn describe(&self) -> Description {
        describe_int_range(&self.0, "card support for partner")
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
        let shown = Inferences::read(context).partner().length(self.suit);
        crisp(self.range.contains(&shown.min))
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
        let shown = Inferences::read(context).partner().points;
        crisp(self.0.contains(&shown.min))
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

    fn describe(&self) -> Description {
        describe_int_range(&self.range, &format!("of the top honors in {}", self.suit))
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

    fn describe(&self) -> Description {
        Description::atom("vulnerable")
    }
}

/// Our side is vulnerable
#[must_use]
pub fn vulnerable() -> Cons<impl Constraint + Clone> {
    Cons(Vulnerable)
}

/// The opponents are vulnerable (the [`they_vulnerable`] constraint)
#[derive(Clone)]
struct TheyVulnerable;

impl Constraint for TheyVulnerable {
    fn eval(&self, _: Hand, context: &Context<'_>) -> f32 {
        use contract_bridge::auction::RelativeVulnerability;
        crisp(context.vul().contains(RelativeVulnerability::THEY))
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

#[cfg(test)]
mod tests {
    use super::*;
    use contract_bridge::auction::{Call, RelativeVulnerability};
    use contract_bridge::{Bid, Strain};

    /// 15 HCP, 4333 — spades.hearts.diamonds.clubs
    const BALANCED_15: &str = "AKQ2.K53.QJ4.T92";

    fn hand(s: &str) -> Hand {
        s.parse().expect("valid test hand")
    }

    fn empty_context() -> Context<'static> {
        Context::new(RelativeVulnerability::NONE, &[])
    }

    fn assert_pass(logit: f32) {
        assert!(logit.is_finite() && logit.abs() <= f32::EPSILON);
    }

    fn assert_reject(logit: f32) {
        assert!(logit.is_infinite() && logit.is_sign_negative());
    }

    #[test]
    fn project_band_carries_ceilings() {
        let context = empty_context();
        // `points` gauges the shared scalar: both bounds exact.
        assert_eq!(
            points(..12).project_band(&context).points,
            Range::new(0, 11)
        );
        // An HCP ceiling owes the scale its maximum upgrade (rule-of-N+8
        // default: 5); the floor matches `project`.
        assert_eq!(hcp(..6).project_band(&context).points, Range::new(0, 10));
        // `project` itself stays floor-only — the alert path is untouched.
        assert_eq!(hcp(..6).project(&context).points, Range::FULL_POINTS);
        // Composition is tight per arm: the 1NT pass gate (`notrump.rs`) — an
        // off-major weak arm unioned with the flat-eight arm — caps points at
        // 13 and both majors at five.
        let gate = (hcp(..8) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5))
            | (hcp(8..=8)
                & balanced()
                & len(Suit::Clubs, 3..)
                & len(Suit::Diamonds, 3..)
                & len(Suit::Hearts, 3..)
                & len(Suit::Spades, 3..));
        let band = gate.project_band(&context);
        assert_eq!(band.points, Range::new(0, 13));
        assert_eq!(band.length(Suit::Hearts).max, 5);
        assert_eq!(band.length(Suit::Spades).max, 5);
        // A trivial catch-all claims nothing — the trap-pass safeguard.
        assert_eq!(hcp(0..).project_band(&context), Inference::unknown());
    }

    #[test]
    fn test_hcp_and_balanced() {
        let context = empty_context();
        assert_pass(hcp(15..=17).eval(hand(BALANCED_15), &context));
        assert_reject(hcp(16..).eval(hand(BALANCED_15), &context));
        assert_pass(balanced().eval(hand(BALANCED_15), &context));
        assert_reject(balanced().eval(hand("AKQJ2.K543.QJ4.2"), &context));
    }

    #[test]
    fn test_blocks_upgrade() {
        let clean = ["", "2", "32", "A2", "K2", "KT", "AKQ", "QJ2", "J32"];
        let wasted = [
            "A", "K", "Q", "J", "Q2", "J2", "AK", "AQ", "AJ", "KQ", "KJ", "QJ",
        ];

        for text in clean {
            let holding: Holding = text.parse().expect(text);
            assert!(!blocks_upgrade(holding), "{text} should not block");
        }
        for text in wasted {
            let holding: Holding = text.parse().expect(text);
            assert!(blocks_upgrade(holding), "{text} should block");
        }
    }

    #[test]
    fn test_upgrade() {
        // Balanced hands never upgrade, clean doubleton or not.
        assert_eq!(upgrade(hand(BALANCED_15)), 0);
        assert_eq!(upgrade(hand("AQJ32.K53.QJ4.92")), 0);

        // Unbalanced, clean singleton: +1.
        assert_eq!(upgrade(hand("KQ765.A876.532.2")), 1);

        // Two-suiter with 10+ cards in the two longest suits: +2.
        assert_eq!(upgrade(hand("KQ765.A8765.32.2")), 2);
        assert_eq!(upgrade(hand("KQ8765.A876.32.2")), 2);

        // Ax and Kx are working short holdings, not wasted.
        assert_eq!(upgrade(hand("KQ765.87654.A2.2")), 2);

        // Wasted short honors void the whole upgrade.
        assert_eq!(upgrade(hand("KQ765.A876.532.K")), 0); // stiff K
        assert_eq!(upgrade(hand("KQ765.A8765.Q2.2")), 0); // Qx
        assert_eq!(upgrade(hand("KQ765.87654.AK.2")), 0); // AK tight
    }

    #[test]
    fn test_rule_of_20() {
        let context = empty_context();
        let opens = |text: &str| rule_of_20().eval(hand(text), &context);

        // 11 HCP, 5-4: 11 + 9 = 20.  The wasted J9 that voids the points upgrade
        // is irrelevant to the raw-HCP Rule of 20 — that is the whole point of
        // gauging raw HCP here rather than upgraded `points`.
        assert_pass(opens("AK986.J9.QJT6.64"));
        // 11 HCP, 6-6: 11 + 12 = 23.
        assert_pass(opens(".KQ7542.A.Q96542"));
        // 10 HCP, 6-4: 10 + 10 = 20.
        assert_pass(opens("KJ9876.5.KQJ4.32"));
        // Raw HCP, so a 7-count 7-6 also clears (7 + 13); the opening rule's
        // hcp(10..) floor, not this predicate, keeps such freaks out.
        assert_pass(opens("A765432.K76543.."));

        // Flat 11-count 4-3-3-3: 11 + 7 = 18.
        assert_reject(opens("KQ32.K32.Q32.J32"));
        // 11 HCP, 5-3-3-2: 11 + 8 = 19 — still a pass, not a Rule-of-20 opener.
        assert_reject(opens("KQ876.K32.Q32.J2"));
    }

    #[test]
    fn test_unbid_support() {
        // RHO opened 1♥; the unbid suits are ♣ ♦ ♠.
        let auction = [Call::Bid(Bid::new(1, Strain::Hearts))];
        let context = Context::new(RelativeVulnerability::NONE, &auction);

        // 4-1-4-4 short in their suit: 3+ in every unbid suit → passes both gates.
        let shapely = hand("AQ82.5.KJ64.Q975");
        assert_pass(unbid_support(0).eval(shapely, &context));
        assert_pass(unbid_support(1).eval(shapely, &context));

        // 5-3-3-2 with the doubleton in an unbid suit (♠): exactly one unbid suit
        // short → lenient admits, strict rejects.
        let semi = hand("Q2.A54.K54.KJ876");
        assert_reject(unbid_support(0).eval(semi, &context));
        assert_pass(unbid_support(1).eval(semi, &context));

        // 2-3-2-6 one-suiter (6 clubs), short in two unbid suits (♦ ♠): both gates
        // reject — this hand belongs in the 17+ any-shape double tier.
        let one_suiter = hand("K2.A54.Q2.KJ8763");
        assert_reject(unbid_support(0).eval(one_suiter, &context));
        assert_reject(unbid_support(1).eval(one_suiter, &context));
    }

    #[test]
    fn test_points_and_fifths() {
        let context = empty_context();

        // This test exercises the shipped floored rule-of-N+8 default scale;
        // the legacy arms live in `test_point_scale`, and the fit-known
        // candidate rides on `support_points` (see `test_support_points`).

        // 9 HCP, clean 5-5: 9 + 10 − 8 = 11 points (agreeing with the legacy
        // upgrade here).
        let two_suiter = hand("KQ765.A8765.32.2");
        assert_pass(points(11..=11).eval(two_suiter, &context));
        assert_reject(points(..=10).eval(two_suiter, &context));

        // The floor blocks the flat 4-3-3-3 downgrade: raw HCP, not HCP − 1.
        assert_pass(points(15..=15).eval(hand(BALANCED_15), &context));

        // BALANCED_15 is 15 HCP but only 14.6 Fifths: its queens and jacks
        // are worth less toward 3NT.  The banded value averages Fifths with
        // the honor companion (≈14.55 BUM-RAP, 14.8 HCP — same verdict either
        // way), so it still drops out of a 15-17 notrump but stays inside a
        // 12-14 one.  Fifths is default-off now (raw HCP beat it in the A6
        // audit), so this test enables the gauge it exercises.
        set_fuzzy_fifths(true);
        assert_reject(fifths(15.0..18.0).eval(hand(BALANCED_15), &context));
        assert_pass(fifths(12.0..15.0).eval(hand(BALANCED_15), &context));
        set_fuzzy_fifths(false); // restore the shipped default

        // CCCC of this 4333 is 14.90 (oracle-verified in contract-bridge).
        assert_pass(cccc_at_least(14.9).eval(hand("AQ32.K53.QJ4.A92"), &context));
        assert_reject(cccc_at_least(15.0).eval(hand("AQ32.K53.QJ4.A92"), &context));
        assert_pass(cccc(14.0..15.0).eval(hand("AQ32.K53.QJ4.A92"), &context));
        assert_reject(cccc(..14.9).eval(hand("AQ32.K53.QJ4.A92"), &context));

        // Honor location: same 6 HCP, but KQJ concentrated in the 6-card suit
        // versus banished to short suits.  CCCC pays for the concentration;
        // NLTC discounts honors that don't guard length (the doubleton KQ's
        // queen saves no loser — only 3+ card suits check the queen slot).
        let concentrated = hand("KQJ862.943.75.82");
        let scattered = hand("986432.94.KQ.J82");
        assert!(eval::cccc(concentrated) > eval::cccc(scattered));
        assert!(eval::NLTC.eval(concentrated) < eval::NLTC.eval(scattered));
        // NLTC of the concentrated hand: ♠1.5 + ♥3 + ♦2.5 + ♣2.5 = 9.5 losers.
        assert_pass(nltc(..=9.5).eval(concentrated, &context));
        assert_reject(nltc(..9.5).eval(concentrated, &context));
        assert_pass(nltc(9.0..=10.0).eval(concentrated, &context));
    }

    #[test]
    fn test_support_points() {
        let context = empty_context();

        // 9 HCP, clean 5-5-2-1.  The candidate scale counts hcp_plus (useful
        // shortness: +1 doubleton, +2 singleton) plus the long-suit term:
        // 9 + 1 + 2 + 1 = 13, above the legacy raw-HCP-plus-upgrade of 11.
        let two_suiter = hand("KQ765.A8765.32.2");

        // Off (the A/B baseline arm): byte-identical to the global `point_count`
        // that `points` gauges — a gate swapped `points`→`support_points` doesn't
        // move.  (Rule of N+8 and the legacy upgrade agree on this clean 5-5.)
        set_support_points(false);
        assert_eq!(support_point_count(two_suiter), point_count(two_suiter));
        assert_eq!(support_point_count(two_suiter), 11);
        assert_pass(support_points(11..=11).eval(two_suiter, &context));

        // On (the shipped default): the hotter hcp_plus scale, strictly above
        // legacy for a shaped hand (the singleton and doubleton now add).
        set_support_points(true);
        assert_eq!(support_point_count(two_suiter), 13);
        assert!(support_point_count(two_suiter) > point_count(two_suiter));
        assert_pass(support_points(13..=13).eval(two_suiter, &context));
        assert_reject(support_points(..=12).eval(two_suiter, &context));

        // Flat hands carry no useful shortness, so the support scale sticks to
        // raw HCP — and the floored rule-of-N+8 default agrees on a 4-3-3-3.
        let flat = hand("AQ32.K53.QJ4.A92"); // 16 HCP, 4-3-3-3
        assert_eq!(support_point_count(flat), 16);
        assert_eq!(point_count(flat), 16);
        // Left on — the shipped default — for the rest of the suite.
    }

    #[test]
    fn test_fifths_companion() {
        let context = empty_context();
        // Quack-heavy 18-count: 18.2 Fifths, 18 HCP, 16.5 BUM-RAP.  The
        // Fifths/HCP average (18.1) tops a 15-17 notrump, but the lighter
        // Fifths/BUM-RAP average (17.35) keeps it inside — the two gauges
        // straddle the band edge.
        let quacky = hand("AQ4.QJT.QJT.KQJT");

        // The companion only matters inside the Fifths gauge, which is
        // default-off now (raw HCP beat it in the A6 audit) — enable it here.
        set_fuzzy_fifths(true);
        set_fifths_companion(FifthsCompanion::Hcp);
        assert_reject(fifths(15.0..18.0).eval(quacky, &context));

        set_fifths_companion(FifthsCompanion::Bumrap);
        assert_pass(fifths(15.0..18.0).eval(quacky, &context));
        set_fuzzy_fifths(false); // restore the shipped default
    }

    #[test]
    fn test_fuzzy_strength_toggle() {
        let context = empty_context();
        let two_suiter = hand("KQ765.A8765.32.2");

        // This toggle swings `points` between raw HCP and the legacy
        // raw-HCP-plus-upgrade scale (both historical arms now).
        set_fuzzy_strength(false);
        // Raw HCP: 9 points, and fifths degrades to raw HCP too.
        assert_pass(points(9..=9).eval(two_suiter, &context));
        assert_pass(fifths(15.0..18.0).eval(hand(BALANCED_15), &context));
        assert_reject(fifths(15.5..18.0).eval(hand(BALANCED_15), &context));

        // The legacy upgrade arm agrees with rule-of-N+8 on this clean 5-5.
        set_fuzzy_points(true);
        assert_pass(points(11..=11).eval(two_suiter, &context));

        // Restore the shipped default for the rest of the suite.
        set_point_scale(PointScale::RuleOfNFloored);
    }

    #[test]
    fn test_point_scale() {
        let context = empty_context();
        let two_suiter = hand("KQ765.A8765.32.2"); // 9 HCP, 5-5-2-1
        let flat = hand("AQ32.K53.QJ4.A92"); // 16 HCP, 4-3-3-3

        // Rule of N+8: raw HCP + two longest suit lengths − 8, so a
        // `points(12..)` gate is exactly the Rule of 20.
        set_point_scale(PointScale::RuleOfN);
        // Clean 5-5 agrees with the legacy upgrade: 9 + 10 − 8 = 9 + 2.
        assert_eq!(point_count(two_suiter), 11);
        assert_pass(points(11..=11).eval(two_suiter, &context));
        // Flat 4-3-3-3 reads one under its HCP: 16 + 7 − 8.
        assert_eq!(point_count(flat), 15);
        assert_reject(points(16..).eval(flat, &context));
        // A wasted stiff K voids the legacy upgrade but its shape still
        // counts here: 12 + 9 − 8 = 13 vs legacy 12.
        let wasted = hand("KQ765.A876.532.K");
        assert_eq!(point_count(wasted), 13);
        assert_eq!(point_count(wasted), raw_hcp(wasted) + 1);

        // Blocking the downgrade: flat 4-3-3-3 reads its raw HCP, every
        // other shape agrees with plain rule-of-N+8.
        set_point_scale(PointScale::RuleOfNFloored);
        assert_eq!(point_count(flat), 16);
        assert_eq!(point_count(two_suiter), 11);

        set_point_scale(PointScale::Hcp);
        assert_eq!(point_count(two_suiter), 9);

        // The deposed legacy scale stays reachable as the opt-out.
        set_point_scale(PointScale::PointCount);
        assert_eq!(point_count(two_suiter), 11);

        // Restore the shipped default for the rest of the suite.
        set_point_scale(PointScale::RuleOfNFloored);
    }

    #[test]
    fn test_combinators() {
        let context = empty_context();
        let strong_notrump = hcp(15..=17) & balanced();
        assert_pass(strong_notrump.eval(hand(BALANCED_15), &context));

        let either = hcp(16..) | len(Suit::Spades, 4..);
        assert_pass(either.eval(hand(BALANCED_15), &context));

        let neither = hcp(16..) | len(Suit::Spades, 5..);
        assert_reject(neither.eval(hand(BALANCED_15), &context));

        assert_reject((!balanced()).eval(hand(BALANCED_15), &context));
        assert_pass((!hcp(16..)).eval(hand(BALANCED_15), &context));
    }

    #[test]
    fn test_support_and_stoppers() {
        // Partner overcalled 1♥ over their 1♦ opening.
        let auction = [
            Call::Bid(Bid::new(1, Strain::Diamonds)),
            Call::Bid(Bid::new(1, Strain::Hearts)),
            Call::Pass,
        ];
        let context = Context::new(RelativeVulnerability::NONE, &auction);

        assert_pass(support(3..).eval(hand(BALANCED_15), &context));
        assert_reject(support(4..).eval(hand(BALANCED_15), &context));

        // QJ4 of diamonds stops their suit; T92 of clubs would not, but
        // clubs is not their suit.
        assert_pass(stopper_in_their_suits().eval(hand(BALANCED_15), &context));
        assert_reject(stopper_in_their_suits().eval(hand("AKQ2.K53.T92.QJ4"), &context));
    }

    #[test]
    fn test_partner_shown_len_and_points() {
        // Partner opened 1♦ (3+ diamonds, 10+ by Rule of 20), RHO passed; we act.
        let auction = [Call::Bid(Bid::new(1, Strain::Diamonds)), Call::Pass];
        let context = Context::new(RelativeVulnerability::NONE, &auction);

        assert_pass(partner_shown_len(Suit::Diamonds, 3..).eval(hand(BALANCED_15), &context));
        assert_reject(partner_shown_len(Suit::Diamonds, 4..).eval(hand(BALANCED_15), &context));
        assert_pass(partner_shown_points(10..).eval(hand(BALANCED_15), &context));
        assert_reject(partner_shown_points(11..).eval(hand(BALANCED_15), &context));

        // Nothing shown in an unbid suit: the minimum is zero.
        assert_reject(partner_shown_len(Suit::Spades, 1..).eval(hand(BALANCED_15), &context));
    }

    #[test]
    fn test_support_without_partner_suit() {
        let context = empty_context();
        assert_reject(support(0..).eval(hand(BALANCED_15), &context));
    }

    #[test]
    fn test_top_honors_and_stopper_in() {
        let context = empty_context();
        // AKQ2 of spades has all three top honors; T92 of clubs has none.
        assert_pass(top_honors(Suit::Spades, 3..).eval(hand(BALANCED_15), &context));
        assert_pass(top_honors(Suit::Hearts, 1..=1).eval(hand(BALANCED_15), &context));
        assert_reject(top_honors(Suit::Clubs, 1..).eval(hand(BALANCED_15), &context));

        // K53 of hearts stops the suit; T92 of clubs does not.
        assert_pass(stopper_in(Suit::Hearts).eval(hand(BALANCED_15), &context));
        assert_reject(stopper_in(Suit::Clubs).eval(hand(BALANCED_15), &context));
    }

    #[test]
    fn test_partner_suit_and_min_level() {
        // Partner overcalled 1♥ over their 1♦ opening.
        let auction = [
            Call::Bid(Bid::new(1, Strain::Diamonds)),
            Call::Bid(Bid::new(1, Strain::Hearts)),
            Call::Pass,
        ];
        let context = Context::new(RelativeVulnerability::NONE, &auction);

        assert_pass(partner_suit_is(Suit::Hearts).eval(hand(BALANCED_15), &context));
        assert_reject(partner_suit_is(Suit::Spades).eval(hand(BALANCED_15), &context));

        assert_pass(min_level_is(1, Strain::Spades).eval(hand(BALANCED_15), &context));
        assert_pass(min_level_is(2, Strain::Diamonds).eval(hand(BALANCED_15), &context));
        assert_reject(min_level_is(2, Strain::Spades).eval(hand(BALANCED_15), &context));
    }

    #[test]
    fn test_vulnerability_and_seats() {
        let auction = [Call::Pass];
        let context = Context::new(RelativeVulnerability::WE, &auction);

        assert_pass(vulnerable().eval(hand(BALANCED_15), &context));
        assert_reject(they_vulnerable().eval(hand(BALANCED_15), &context));
        assert_pass(nth_seat(2).eval(hand(BALANCED_15), &context));
        assert_reject(nth_seat(1).eval(hand(BALANCED_15), &context));
    }

    /// Render a constraint to its prose, the inverse of evaluation.
    fn prose(constraint: &impl Constraint) -> String {
        constraint.describe().to_string()
    }

    #[test]
    fn test_describe_ranges() {
        // Closed, open-ended, capped, and exact integer bands.
        assert_eq!(prose(&hcp(15..=17)), "15–17 HCP");
        assert_eq!(prose(&hcp(16..)), "16+ HCP");
        assert_eq!(prose(&hcp(..11)), "≤10 HCP"); // half-open → inclusive
        assert_eq!(prose(&points(12..=21)), "12–21 points");
        assert_eq!(prose(&len(Suit::Spades, 5..)), "5+ ♠");
        assert_eq!(prose(&len(Suit::Hearts, 6..=6)), "exactly 6 ♥");
        assert_eq!(prose(&support(3..)), "3+ card support for partner");
        assert_eq!(
            prose(&top_honors(Suit::Spades, 2..)),
            "2+ of the top honors in ♠"
        );
        assert_eq!(
            prose(&partner_shown_len(Suit::Diamonds, 3..)),
            "3+ ♦ shown by partner",
        );
        assert_eq!(
            prose(&partner_shown_points(12..)),
            "12+ points shown by partner"
        );
        // Fifths print as a literal float band, never nudged to "≤17.999".
        assert_eq!(prose(&fifths(15.0..18.0)), "15.0–18.0 fifths");
        assert_eq!(prose(&fifths(20.0..22.0)), "20.0–22.0 fifths");
    }

    #[test]
    fn test_describe_atoms() {
        assert_eq!(prose(&balanced()), "balanced");
        assert_eq!(prose(&cccc_at_least(14.9)), "14.9+ CCCC");
        assert_eq!(prose(&cccc(9.0..13.0)), "9.0–13.0 CCCC");
        assert_eq!(prose(&nltc(..=8.5)), "≤8.5 NLTC");
        assert_eq!(prose(&stopper_in(Suit::Hearts)), "stopper in ♥");
        assert_eq!(prose(&stopper_in_their_suits()), "stopper in their suit(s)");
        assert_eq!(
            prose(&short_in_their_suits()),
            "at most three cards in each of their suits",
        );
        assert_eq!(prose(&they_bid(Strain::Spades)), "opponents bid ♠");
        assert_eq!(prose(&they_bid(Strain::Notrump)), "opponents bid NT");
        assert_eq!(
            prose(&partner_suit_is(Suit::Hearts)),
            "partner's last suit is ♥"
        );
        assert_eq!(
            prose(&min_level_is(2, Strain::Diamonds)),
            "2♦ is the cheapest bid"
        );
        assert_eq!(prose(&passed_hand()), "a passed hand");
        assert_eq!(
            prose(&undisturbed()),
            "the opponents have passed throughout"
        );
        assert_eq!(prose(&vulnerable()), "vulnerable");
        assert_eq!(prose(&they_vulnerable()), "opponents vulnerable");
        assert_eq!(prose(&nth_seat(3)), "opening in seat 3");
    }

    #[test]
    fn test_describe_composition() {
        // `&` flattens into one comma list with a trailing "and".
        assert_eq!(
            prose(&(points(12..=21) & len(Suit::Spades, 5..))),
            "12–21 points, and 5+ ♠",
        );
        assert_eq!(
            prose(&(points(12..=21) & len(Suit::Spades, 5..) & balanced())),
            "12–21 points, 5+ ♠, and balanced",
        );
        // `|` flattens with a trailing "or"; `!` wraps in "not (…)".
        assert_eq!(
            prose(&(len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..))),
            "5+ ♣, or 5+ ♦",
        );
        assert_eq!(prose(&!hcp(16..)), "not (16+ HCP)");
        // Double negation cancels.
        assert_eq!(prose(&!!balanced()), "balanced");
        // A nested group is parenthesized so a mixed tree stays unambiguous.
        assert_eq!(
            prose(&(points(9..=11) & len(Suit::Spades, 5..) & (nth_seat(3) | nth_seat(4)))),
            "9–11 points, 5+ ♠, and (opening in seat 3, or opening in seat 4)",
        );
    }

    #[test]
    fn test_describe_opaque_and_labeled() {
        // A bare predicate carries no recoverable meaning.
        assert_eq!(pred(|_, _| true).describe(), Description::Opaque);
        assert_eq!(prose(&pred(|_, _| true)), "(opaque condition)");
        // Opacity surfaces as one element, not a whole-conjunction collapse.
        assert_eq!(
            prose(&(hcp(15..) & pred(|_, _| true))),
            "15+ HCP, and (opaque condition)",
        );
        // The labeled escape hatch describes to its label and still evaluates.
        let prefers_diamonds = described("prefers diamonds", |hand: Hand, _: &Context<'_>| {
            hand[Suit::Diamonds].len() >= hand[Suit::Clubs].len()
        });
        assert_eq!(prose(&prefers_diamonds), "prefers diamonds");
        assert_pass(prefers_diamonds.eval(hand(BALANCED_15), &empty_context()));
    }
}
