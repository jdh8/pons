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
use contract_bridge::eval::{self, HandEvaluator, SimpleEvaluator};
use contract_bridge::{Hand, Holding, Rank, Suit};
use core::ops::{BitAnd, BitOr, Not, RangeBounds};

/// Trait for a logit contribution of a hand feature
///
/// Implementations must not return `f32::INFINITY`: combining `+∞` with the
/// `-∞` of a violated crisp constraint would produce a NaN.
pub trait Constraint: Send + Sync {
    /// Evaluate the constraint into a logit contribution
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32;
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
}

/// Sum of two constraints, the logical AND for crisp constraints
#[derive(Clone, Copy, Debug)]
pub struct And<A, B>(A, B);

impl<A: Constraint, B: Constraint> Constraint for And<A, B> {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        self.0.eval(hand, context) + self.1.eval(hand, context)
    }
}

/// Maximum of two constraints, the logical OR for crisp constraints
#[derive(Clone, Copy, Debug)]
pub struct Or<A, B>(A, B);

impl<A: Constraint, B: Constraint> Constraint for Or<A, B> {
    fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        self.0.eval(hand, context).max(self.1.eval(hand, context))
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

/// Total high card points in the given range
#[must_use]
pub fn hcp(range: impl RangeBounds<u8> + Clone + Send + Sync) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, _: &Context<'_>| {
        range.contains(&SimpleEvaluator(eval::hcp::<u8>).eval(hand))
    })
}

/// Length of the given suit in the given range
pub fn len(
    suit: Suit,
    range: impl RangeBounds<usize> + Clone + Send + Sync,
) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, _: &Context<'_>| range.contains(&hand[suit].len()))
}

/// Balanced shape: 4333, 4432, or 5332
#[must_use]
pub fn balanced() -> Cons<impl Constraint + Clone> {
    pred(|hand: Hand, _: &Context<'_>| {
        let lengths = Suit::ASC.map(|suit| hand[suit].len());
        lengths.iter().all(|&length| length >= 2)
            && lengths.iter().filter(|&&length| length == 2).count() <= 1
    })
}

/// [New Losing Trick Count][eval::NLTC] at most the given number of losers
#[must_use]
pub fn nltc_at_most(losers: f64) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, _: &Context<'_>| eval::NLTC.eval(hand) <= losers)
}

/// Support for partner's last bid suit in the given range
///
/// Violated when partner has not bid a suit yet.
pub fn support(
    range: impl RangeBounds<usize> + Clone + Send + Sync,
) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        context
            .partner_last_suit()
            .is_some_and(|suit| range.contains(&hand[suit].len()))
    })
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

/// A stopper in every suit the opponents have bid
///
/// Trivially satisfied when the opponents have bid no suit.
#[must_use]
pub fn stopper_in_their_suits() -> Cons<impl Constraint + Clone> {
    pred(|hand: Hand, context: &Context<'_>| {
        context.their_suits().all(|suit| has_stopper(hand[suit]))
    })
}

/// The player to act passed on their first turn
#[must_use]
pub fn passed_hand() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| context.passed_hand())
}

/// The opponents have made nothing but passes
#[must_use]
pub fn undisturbed() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| context.undisturbed())
}

/// Our side is vulnerable
#[must_use]
pub fn vulnerable() -> Cons<impl Constraint + Clone> {
    use contract_bridge::auction::RelativeVulnerability;
    pred(|_: Hand, context: &Context<'_>| context.vul().contains(RelativeVulnerability::WE))
}

/// The opponents are vulnerable
#[must_use]
pub fn they_vulnerable() -> Cons<impl Constraint + Clone> {
    use contract_bridge::auction::RelativeVulnerability;
    pred(|_: Hand, context: &Context<'_>| context.vul().contains(RelativeVulnerability::THEY))
}

/// About to make the first non-pass call in the given seat (1–4)
///
/// This is the exception mechanism for seat-specific openings (e.g. no
/// preempts in 4th seat); 1st/2nd and 3rd/4th seats are otherwise treated
/// alike structurally.
#[must_use]
pub fn nth_seat(seat: u8) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| context.seat_to_open() == Some(seat))
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
    fn test_support_without_partner_suit() {
        let context = empty_context();
        assert_reject(support(0..).eval(hand(BALANCED_15), &context));
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
}
