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
use super::inference::Inferences;
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

std::thread_local! {
    /// Whether [`points`] applies its upgrade on top of raw HCP
    static FUZZY_POINTS: Cell<bool> = const { Cell::new(true) };
    /// Whether [`fifths`] evaluates Fifths rather than raw HCP
    static FUZZY_FIFTHS: Cell<bool> = const { Cell::new(true) };
    /// The honor count averaged with Fifths in [`fifths`] (BUM-RAP won the A/B)
    static FIFTHS_COMPANION: Cell<FifthsCompanion> = const { Cell::new(FifthsCompanion::Bumrap) };
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
#[doc(hidden)]
pub fn set_fuzzy_points(enabled: bool) {
    FUZZY_POINTS.with(|flag| flag.set(enabled));
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

/// Raw high card points of a hand
fn raw_hcp(hand: Hand) -> u8 {
    SimpleEvaluator(eval::hcp::<u8>).eval(hand)
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
}

/// Total high card points in the given range
#[must_use]
pub fn hcp(range: impl RangeBounds<u8> + Clone + Send + Sync) -> Cons<impl Constraint + Clone> {
    Cons(Hcp(range))
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

    let mut lengths = holdings.map(Holding::len);
    lengths.sort_unstable();
    u8::from(!is_balanced(hand)) + u8::from(lengths[2] + lengths[3] >= 10)
}

/// Upgraded points as a scalar — raw HCP plus the fuzzy-strength [`upgrade`]
///
/// The number the suit-oriented [`points`] constraint gauges with fuzzy
/// strength on, and the scale [`Inferences`] records its point ranges on.  A
/// reader that needs the value rather than a range — constrained sampling, for
/// one — shares this single definition so it can never drift from the ranges it
/// checks against.
///
/// [`Inferences`]: super::inference::Inferences
#[must_use]
pub fn point_count(hand: Hand) -> u8 {
    raw_hcp(hand) + upgrade(hand)
}

/// Upgraded points in a range (the [`points`] constraint)
#[derive(Clone)]
struct Points<R>(R);

impl<R: RangeBounds<u8> + Clone + Send + Sync> Constraint for Points<R> {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        let bonus = if FUZZY_POINTS.with(Cell::get) {
            upgrade(hand)
        } else {
            0
        };
        crisp(self.0.contains(&(raw_hcp(hand) + bonus)))
    }

    fn describe(&self) -> Description {
        describe_int_range(&self.0, "points")
    }
}

/// Upgraded points — HCP plus [`upgrade`] — in the given range
///
/// The strength gauge for suit-oriented calls.  Notrump-defining ranges use
/// [`fifths`] instead, and ranges indifferent to shape keep [`hcp`].
#[must_use]
pub fn points(range: impl RangeBounds<u8> + Clone + Send + Sync) -> Cons<impl Constraint + Clone> {
    Cons(Points(range))
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
}

/// Length of the given suit in the given range
pub fn len(
    suit: Suit,
    range: impl RangeBounds<usize> + Clone + Send + Sync,
) -> Cons<impl Constraint + Clone> {
    Cons(Len { suit, range })
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
}

/// Balanced shape: 4333, 4432, or 5332
#[must_use]
pub fn balanced() -> Cons<impl Constraint + Clone> {
    Cons(Balanced)
}

/// Kaplan–Rubens CCCC floor (the [`cccc_at_least`] constraint)
#[derive(Clone)]
struct CcccAtLeast(f64);

impl Constraint for CcccAtLeast {
    fn eval(&self, hand: Hand, _: &Context<'_>) -> f32 {
        crisp(eval::cccc(hand) >= self.0)
    }

    fn describe(&self) -> Description {
        Description::atom(format!("CCCC ≥ {}", self.0))
    }
}

/// [Kaplan–Rubens CCCC][eval::cccc] at least the given strength
///
/// CCCC weighs honor placement together with shape, which makes it
/// particularly accurate for suit contracts; prefer [`fifths`] toward
/// notrump.
#[must_use]
pub fn cccc_at_least(points: f64) -> Cons<impl Constraint + Clone> {
    Cons(CcccAtLeast(points))
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
const fn has_stopper(holding: Holding) -> bool {
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
    fn test_points_and_fifths() {
        let context = empty_context();

        // 9 HCP, clean 5-5: counts as 11 upgraded points.
        let two_suiter = hand("KQ765.A8765.32.2");
        assert_pass(points(11..=11).eval(two_suiter, &context));
        assert_reject(points(..=10).eval(two_suiter, &context));

        // Balanced hands score their raw HCP.
        assert_pass(points(15..=15).eval(hand(BALANCED_15), &context));

        // BALANCED_15 is 15 HCP but only 14.6 Fifths: its queens and jacks
        // are worth less toward 3NT.  The banded value averages Fifths with
        // the honor companion (≈14.55 BUM-RAP, 14.8 HCP — same verdict either
        // way), so it still drops out of a 15-17 notrump but stays inside a
        // 12-14 one.
        assert_reject(fifths(15.0..18.0).eval(hand(BALANCED_15), &context));
        assert_pass(fifths(12.0..15.0).eval(hand(BALANCED_15), &context));

        // CCCC of this 4333 is 14.90 (oracle-verified in contract-bridge).
        assert_pass(cccc_at_least(14.9).eval(hand("AQ32.K53.QJ4.A92"), &context));
        assert_reject(cccc_at_least(15.0).eval(hand("AQ32.K53.QJ4.A92"), &context));
    }

    #[test]
    fn test_fifths_companion() {
        let context = empty_context();
        // Quack-heavy 18-count: 18.2 Fifths, 18 HCP, 16.5 BUM-RAP.  The
        // Fifths/HCP average (18.1) tops a 15-17 notrump, but the lighter
        // Fifths/BUM-RAP average (17.35) keeps it inside — the two gauges
        // straddle the band edge.
        let quacky = hand("AQ4.QJT.QJT.KQJT");

        set_fifths_companion(FifthsCompanion::Hcp);
        assert_reject(fifths(15.0..18.0).eval(quacky, &context));

        set_fifths_companion(FifthsCompanion::Bumrap);
        assert_pass(fifths(15.0..18.0).eval(quacky, &context));
    }

    #[test]
    fn test_fuzzy_strength_toggle() {
        let context = empty_context();
        let two_suiter = hand("KQ765.A8765.32.2");

        set_fuzzy_strength(false);
        // Raw HCP: 9 points, and fifths degrades to raw HCP too.
        assert_pass(points(9..=9).eval(two_suiter, &context));
        assert_pass(fifths(15.0..18.0).eval(hand(BALANCED_15), &context));
        assert_reject(fifths(15.5..18.0).eval(hand(BALANCED_15), &context));
        set_fuzzy_strength(true);

        assert_pass(points(11..=11).eval(two_suiter, &context));
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
        // Partner opened 1♦ (3+ diamonds, 12+), RHO passed; we act.
        let auction = [Call::Bid(Bid::new(1, Strain::Diamonds)), Call::Pass];
        let context = Context::new(RelativeVulnerability::NONE, &auction);

        assert_pass(partner_shown_len(Suit::Diamonds, 3..).eval(hand(BALANCED_15), &context));
        assert_reject(partner_shown_len(Suit::Diamonds, 4..).eval(hand(BALANCED_15), &context));
        assert_pass(partner_shown_points(12..).eval(hand(BALANCED_15), &context));
        assert_reject(partner_shown_points(13..).eval(hand(BALANCED_15), &context));

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
        assert_eq!(prose(&cccc_at_least(14.9)), "CCCC ≥ 14.9");
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
