//! Defending their `1NT` — the system bundle and the natural chain
//!
//! This module holds the selector ([`NotrumpDefense`]), the natural chain every
//! system rests on, and the assembly.  The systems' own calls live in
//! [`super::nt_landy`], [`super::nt_dont`], [`super::nt_meckwell`] and
//! [`super::nt_woolsey`]; the per-call [`Alert`] constants they share are in the
//! parent index.
//!
//! A defensive "system" (Natural, Woolsey, DONT, …) is a *bundle* of per-call
//! conventions: only the call carries a convention, not the system.  "Woolsey"
//! is `X` = Woolsey + `2♣` = Landy + `2♦` = Multi + `2♥`/`2♠` = Muiderberg.  So
//! each artificial `(call, convention)` is authored once as an alerted block, all
//! of them are chained at the `(1NT)` node, and [`Rules::gated`] ships only the
//! active system's calls at book-construction time (the same build-time gate the
//! European 1NT minors use; see `notrump::notrump_responses`).
//!
//! **An [`Alert`] marks an artificial call: only artificial calls carry one.**  An
//! unalerted call is *natural* and *floor-safe* — dropping its book node is at worst
//! suboptimal, because the instinct floor bids it sensibly and reads it right.  An
//! artificial call must be pinned by a book node (and an `Inferences::read`
//! decoding), or the floor misreads the convention and raises a phantom suit into a
//! doubled minus.  So the penalty `X`, the four natural suit overcalls, and `Pass`
//! stay unalerted (authored where they are a measured DD win, via
//! [`chain_natural_base`]); the conventions are the alerts — the same per-call
//! [`Alert`] now carried by every artificial call system-wide (see [`Rules::alert`]).

use super::michaels::semi_balanced;
use super::nt_dont::{direct_dont_one_suiter_min, dont_2c, dont_2d, dont_2h, dont_x};
use super::nt_landy::{landy_2c, landy_x};
use super::nt_meckwell::{meckwell_2c, meckwell_2d, meckwell_natural_major, meckwell_x};
use super::nt_woolsey::{muiderberg, multi_2d, woolsey_2c, woolsey_x};
use super::overcall::DoubleShape;
use super::*;

/// Which mutually-exclusive defense our side plays over the opponents' 1NT opening
///
/// Exactly one system is active at a time.  Storing the choice in a single `Cell`
/// makes the old "two families authored at once" state — previously possible with
/// the independent `NATURAL_DEFENSE` / `DIRECT_DONT` / `MECKWELL` / `WOOLSEY` /
/// `ALWAYS_PASS_DEFENSE` booleans and resolved only by a read-time precedence
/// cascade — unrepresentable.  Read once at book-construction time.
///
/// [`set_notrump_defense`] is the only setter.  The five per-system bool shims
/// that survived the original fold were deleted 2026-08-03: their `false` arms
/// reverted to Natural *only if that system was the active one*, so a harness
/// resetting by calling every `set_*(false)` got an order-dependent result — and
/// keeping them let `bba-gen` and `ab-landy` re-implement the very cascade this
/// cell deleted.  The `DirectLandy` payload keeps a setter
/// ([`set_direct_landy_double`]) because the flat-4-4 flag has no meaning
/// without the double it configures.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum NotrumpDefense {
    /// Natural one-suiter defense: penalty `X` + the four natural two-level overcalls
    /// + the owning `Pass` catch-all.  The **default**.
    #[default]
    Natural,
    /// Direct-seat DONT (one-suiter `X`, two-suiter `2♣`/`2♦`/`2♥`, natural `2♠`).
    DirectDont,
    /// Direct-seat Meckwell (two-way `X`, minor+major `2♣`/`2♦`, natural majors).
    Meckwell,
    /// Woolsey "Multi-Landy" (`X` = 4-card major + longer minor, `2♣` = both majors,
    /// `2♦` = Multi, `2♥`/`2♠` = Muiderberg).
    Woolsey,
    /// Direct-seat both-majors takeout `X` (Landy-style); the 5-4-vs-4-4 shape flag
    /// lives in `DIRECT_LANDY_FOUR_FOUR`.
    DirectLandy,
    /// Author only `Pass` for every hand — our side never competes.
    AlwaysPass,
    /// Author nothing; the `(1NT)` node falls through to the bare instinct floor
    /// (what selecting this variant gives: no authored rules at the `(1NT)` node).
    Off,
}

thread_local! {
    /// The mutually-exclusive 1NT defense in force; **[`Natural`](NotrumpDefense::Natural)
    /// by default**.
    static NOTRUMP_DEFENSE: Cell<NotrumpDefense> = const { Cell::new(NotrumpDefense::Natural) };
}

/// Select the mutually-exclusive 1NT defense for books built *after* this call
/// (thread-local, read once at book-construction time)
pub fn set_notrump_defense(system: NotrumpDefense) {
    NOTRUMP_DEFENSE.with(|cell| cell.set(system));
}

/// The mutually-exclusive 1NT defense currently selected
pub fn notrump_defense() -> NotrumpDefense {
    NOTRUMP_DEFENSE.with(Cell::get)
}

thread_local! {
    /// Whether to also author the natural defense in the *balancing* seat
    /// `(1NT) - - ?`; **off by default** (opt-in A/B). Off leaves the balancing
    /// seat to the instinct floor — the source of the toxic balancing doubles.
    static NOTRUMP_BALANCING: Cell<bool> = const { Cell::new(false) };
}

/// Whether the natural one-suiter defense is currently the active system
#[allow(dead_code)]
pub(super) fn natural_defense_enabled(agreements: &Agreements) -> bool {
    agreements.decision.reading.notrump_defense() == NotrumpDefense::Natural
}

/// Extend the natural 1NT defense to the *balancing* seat `(1NT) - - ?` for books
/// built *after* this call (thread-local; **off by default**). On, the balancing
/// seat reuses `defense_to_notrump` instead of falling to the instinct floor's
/// undisciplined balancing doubles. An A/B knob (`bba-match --ns-balancing`).
pub fn set_notrump_balancing(on: bool) {
    NOTRUMP_BALANCING.with(|cell| cell.set(on));
}

pub(super) fn notrump_balancing_enabled() -> bool {
    NOTRUMP_BALANCING.with(Cell::get)
}

// Each artificial block is a one-rule `Rules` lifting today's cascade verbatim
// (weight, shape, strength).  All twelve are chained unconditionally and then
// gated, so each reads its tuning knobs defensively (an `unwrap_or` placeholder
// band on a gated-out block never reaches the trie).

/// Unusual `2NT`: both minors, 5-5, on its own range (raw HCP or points per
/// [`set_landy_hcp`]).  Additive — compatible with every system.
fn unusual_2nt(agreements: &Agreements) -> Rules {
    let (lo, hi) = agreements.defense.unusual_notrump_range.unwrap_or((0, 37));
    let shape = len(Suit::Clubs, 5..) & len(Suit::Diamonds, 5..);
    if agreements.defense.landy_use_hcp {
        Rules::new().rule(Bid::new(2, Strain::Notrump), 180, shape & hcp(lo..=hi))
    } else {
        Rules::new().rule(Bid::new(2, Strain::Notrump), 180, shape & points(lo..=hi))
    }
}

/// The four natural two-level suit overcalls (five-card suit, `points(8..=14)`),
/// optionally skipping `2♣` when the Landy `2♣` overlay owns that slot.
fn chain_natural_overcalls(mut rules: Rules, skip_clubs: bool, agreements: &Agreements) -> Rules {
    let (oc_lo, oc_hi) = agreements.decision.reading.natural_overcall_points();
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == Suit::Clubs && skip_clubs {
            continue;
        }
        rules = rules.rule(
            Bid::new(2, Strain::from(suit)),
            100,
            len(suit, 5..) & points(oc_lo..=oc_hi),
        );
    }
    rules
}

/// Chain the untagged, floor-safe natural calls the active system uses: the owning
/// `Pass`, and — at the slots no artificial alert owns — the penalty `X` and the
/// natural suit overcalls.  Mirrors the pre-tag cascade's natural arms exactly; the
/// conventions are chained and gated separately in [`defense_to_notrump`].  A slot
/// no live system owns is simply not authored and falls to the instinct floor (the
/// natural-off baseline arm).
fn chain_natural_base(rules: Rules, agreements: &Agreements) -> Rules {
    // One active system — the enum makes the old "two families at once" state (which
    // the read-time cascade precedence Woolsey > DONT > Meckwell > direct-Landy >
    // natural used to arbitrate) unrepresentable.
    match agreements.decision.reading.notrump_defense() {
        // Woolsey owns X and every overcall; `Pass` is the only natural call.  Always-
        // pass is the same finite `Pass` logit — it shadows the floor so our side never
        // competes.
        NotrumpDefense::Woolsey | NotrumpDefense::AlwaysPass => rules.rule(Call::Pass, 0, hcp(0..)),
        NotrumpDefense::DirectDont => {
            // DONT keeps the natural `2♠` one-suiter (open-top, length-gated so the
            // one-suiter `X` can exclude spades) below its two-suiters, plus `Pass`.
            let lo = agreements.decision.reading.natural_overcall_points().0;
            let one_min = direct_dont_one_suiter_min(agreements);
            rules
                .rule(
                    Bid::new(2, Strain::Spades),
                    100,
                    len(Suit::Spades, one_min..) & points(lo..),
                )
                .rule(Call::Pass, 0, hcp(0..))
        }
        NotrumpDefense::Meckwell => {
            // Meckwell keeps the natural 5+ single-suited majors (2♥/2♠, disjoint from
            // its two-suiters) below the alerts, plus Pass.  The two-way X / minor+major
            // 2♣/2♦ / both-minors 2NT are the artificial calls.
            let lo = agreements.decision.reading.natural_overcall_points().0;
            rules
                .rule(
                    Bid::new(2, Strain::Hearts),
                    100,
                    meckwell_natural_major(Suit::Hearts) & points(lo..),
                )
                .rule(
                    Bid::new(2, Strain::Spades),
                    100,
                    meckwell_natural_major(Suit::Spades) & points(lo..),
                )
                .rule(Call::Pass, 0, hcp(0..))
        }
        NotrumpDefense::DirectLandy => {
            // The both-majors `X` is the alert; the four natural overcalls and `Pass`
            // are the floor-safe base (a 15+ balanced hand now passes or overcalls).
            chain_natural_overcalls(rules.rule(Call::Pass, 0, hcp(0..)), false, agreements)
        }
        NotrumpDefense::Natural => {
            // Penalty `X` (HCP floor fixed; shape gate per `set_natural_double_shape` —
            // each arm reissues `.rule()` so the differing constraint types unify), the
            // owning `Pass`, and the natural overcalls (ceding `2♣` to a Landy overlay).
            let floor = agreements.decision.reading.natural_double_floor();
            let w = agreements.defense.natural_double_weight;
            let rules = match agreements.defense.natural_double_shape {
                DoubleShape::Balanced => rules.rule(Call::Double, w, hcp(floor..) & balanced()),
                DoubleShape::SemiBalanced => {
                    rules.rule(Call::Double, w, hcp(floor..) & semi_balanced())
                }
                DoubleShape::Any => rules.rule(Call::Double, w, hcp(floor..)),
            };
            chain_natural_overcalls(
                rules.rule(Call::Pass, 0, hcp(0..)),
                agreements.decision.reading.landy_range().is_some(),
                agreements,
            )
        }
        // No system: author nothing, fall to the instinct floor.
        NotrumpDefense::Off => rules,
    }
}

/// The artificial alerts live at the `(1NT)` node for the configured system, one
/// per [`NotrumpDefense`] plus the two independent overlays.  Read once at
/// book-construction time.
fn active_alerts(agreements: &Agreements) -> Vec<Alert> {
    let mut alerts = Vec::new();
    let system = agreements.decision.reading.notrump_defense();
    match system {
        // Always-pass authors only `Pass` — no alerts, no overlays.
        NotrumpDefense::AlwaysPass => return alerts,
        NotrumpDefense::Woolsey => {
            alerts.extend([
                WOOLSEY_X,
                WOOLSEY_2C,
                MULTI_2D,
                MUIDERBERG_2H,
                MUIDERBERG_2S,
            ]);
        }
        NotrumpDefense::DirectDont => alerts.extend([DONT_X, DONT_2C, DONT_2D, DONT_2H]),
        NotrumpDefense::Meckwell => alerts.extend([MECKWELL_X, MECKWELL_2C, MECKWELL_2D]),
        NotrumpDefense::DirectLandy => alerts.push(LANDY_X),
        // The natural penalty-X family and the bare floor add no alert of their own.
        NotrumpDefense::Natural | NotrumpDefense::Off => {}
    }
    // The Landy `2♣` overlay is the natural family's one convention, incompatible with
    // DONT / Meckwell / direct-Landy-X / Woolsey (each repurposes the `2♣` slot) — so
    // it rides only on the non-convention arms.
    if agreements.decision.reading.landy_range().is_some()
        && matches!(system, NotrumpDefense::Natural | NotrumpDefense::Off)
    {
        alerts.push(LANDY_2C);
    }
    // Unusual `2NT` is additive — every non-always-pass system.
    if agreements.defense.unusual_notrump_range.is_some() {
        alerts.push(UNUSUAL_2NT);
    }
    alerts
}

/// Our defense to the opponents' 1NT opening, composed from per-call alert tags
///
/// The untagged natural base ([`chain_natural_base`]) and every artificial alert
/// are chained at the `(1NT)` node; [`Rules::gated`] then ships only the active
/// system's alerts (untagged natural rules always survive).  [`active_alerts`]
/// guarantees at most one convention per call, and the natural base skips any slot
/// an alert owns, so no two rules collide at a node.
pub fn defense_to_notrump(agreements: &Agreements) -> Rules {
    let alerts = active_alerts(agreements);
    chain_natural_base(Rules::new(), agreements)
        .chain(woolsey_x(agreements).alert(WOOLSEY_X))
        .chain(landy_x(agreements).alert(LANDY_X))
        .chain(dont_x(agreements).alert(DONT_X))
        .chain(landy_2c(agreements).alert(LANDY_2C))
        .chain(woolsey_2c(agreements).alert(WOOLSEY_2C))
        .chain(dont_2c(agreements).alert(DONT_2C))
        .chain(multi_2d(agreements).alert(MULTI_2D))
        .chain(dont_2d(agreements).alert(DONT_2D))
        .chain(muiderberg(Suit::Hearts, agreements).alert(MUIDERBERG_2H))
        .chain(dont_2h(agreements).alert(DONT_2H))
        .chain(muiderberg(Suit::Spades, agreements).alert(MUIDERBERG_2S))
        .chain(meckwell_x(agreements).alert(MECKWELL_X))
        .chain(meckwell_2c(agreements).alert(MECKWELL_2C))
        .chain(meckwell_2d(agreements).alert(MECKWELL_2D))
        .chain(unusual_2nt(agreements).alert(UNUSUAL_2NT))
        .gated(move |t| alerts.contains(&t))
}

/// Defense to their `1NT` opening, direct and balancing seat
///
/// The balancing entry reuses the direct ranges so `(1NT) - - ?` no longer
/// falls to the instinct floor's undisciplined balancing doubles; a lighter
/// balancing-specific range is a later refinement.  See
/// `set_notrump_balancing`.
pub(super) fn notrump_defense_package() -> Package {
    Package {
        name: "notrump-defense",
        gate: |_| true,
        entries: |agreements| {
            let mut entries = rows_of(Pattern::node("P* (1NT)"), defense_to_notrump(agreements));
            if agreements.defense.notrump_balancing_enabled {
                entries.extend(rows_of(
                    Pattern::node("P* (1NT) - -"),
                    defense_to_notrump(agreements),
                ));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
