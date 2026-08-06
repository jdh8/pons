//! The Meckstroth adjunct — opener's artificial game-forcing `2NT` and the
//! invitational `3m` jumps
//!
//! Two independent features shipped under one flag ([`set_meckstroth_adjunct`],
//! with [`set_meckstroth_minor_jumps`] isolating the second half):
//!
//! - **`1M - 1NT - 2NT!`** — an 18+ game force of *any* shape, replacing the
//!   natural 18–19 balanced rebid.  Responder relays `3♣`, opener
//!   shape-describes, responder places the contract (with RKCB on the two
//!   major-fit nodes).
//! - **`3♣`/`3♦` jumps** — 5+ minor, 15–17, the medium shapely hand that
//!   otherwise underbids as a natural two-level minor.

use super::*;
use crate::bidding::american::slam;

// ponytail: construction-time toggle, read during `register()`; set it before
// building the `Pair`.  A per-classify flag (like `set_fifths_companion`) would
// not work — the adjunct changes which *nodes exist*, baked once at build time.
std::thread_local! {
    /// Whether opener's rebid tables carry the **complete Meckstroth adjunct**:
    /// the artificial game-forcing `2NT` (18+, any shape) with its `3♣`-relay
    /// shape-outs, *and* the invitational `3m` jumps (`1M - 1NT - 3m` and
    /// `1♥ - 1♠ - 3m`).  On by default; both feature sets ship on together.
    static MECKSTROTH: Cell<bool> = const { Cell::new(true) };
}

/// Enable the complete Meckstroth adjunct in books built *after* this call
/// (default **on**)
///
/// After `1M - 1NT` (the forcing notrump), opener's `2NT` is an artificial 18+
/// game force of *any* shape (responder relays `3♣`, opener shape-describes
/// toward game or slam) instead of the natural 18–19 balanced rebid; opener also
/// has the invitational `3m` jumps (5+ minor, 15–17).  Read at book-construction
/// time; set it before building the `Pair` (the `ab-meckstroth-2nt` A/B builds a
/// baseline arm with it off).
///
/// Shipped **on**.  The artificial `2NT` measured a plain-DD win
/// (`ab-meckstroth-2nt`, 200k×2 seeds: plain +0.0075/+0.013, PD +0.006/+0.011,
/// sd-lead +0.010/+0.017 NV/vul, all CI-clean); the `3m` jumps are sd-vindicated
/// (plain wash, PD over-punished, sd-lead +0.0012/+0.0042 NV/vul).
pub fn set_meckstroth_adjunct(on: bool) {
    MECKSTROTH.with(|cell| cell.set(on));
}

/// Whether the Meckstroth adjunct is currently enabled
pub(super) fn meckstroth() -> bool {
    MECKSTROTH.with(Cell::get)
}

// ponytail: a second gate on the *same* adjunct, so its two halves can be
// measured apart.  One flag shipped both, and the SD-PD re-adjudication
// confirmed only the merged knob — the `3m` leg's own verdict (plain wash, PD
// loss, plain-SD win) is the shape that batch refuted elsewhere.
std::thread_local! {
    /// Whether the adjunct's invitational `3m` jumps are built (default **on**).
    /// Ignored when [`set_meckstroth_adjunct`] is off — the jumps live inside
    /// the adjunct.
    static MECKSTROTH_MINOR_JUMPS: Cell<bool> = const { Cell::new(true) };
}

/// Build the Meckstroth adjunct's invitational `3m` jumps (default **on**)
///
/// The adjunct is two independent features under one flag: the artificial 18+
/// `2NT` game force and the invitational `3m` jumps (5+ minor, 15–17). Turn
/// this off to keep the game force and drop the jumps — the arm that isolates
/// the `3m` leg, whose only positive bracket was plain SD.
///
/// Read at book-construction time, like [`set_meckstroth_adjunct`].
pub fn set_meckstroth_minor_jumps(on: bool) {
    MECKSTROTH_MINOR_JUMPS.with(|cell| cell.set(on));
}

/// Opener's artificial game-forcing `2NT` — 18+, any shape (real Meckstroth adjunct)
///
/// Attached by the base table [`rebid_after_forcing_notrump`](super) itself,
/// which branches on [`meckstroth`] to choose this rule over the natural 18–19
/// balanced rebid.
pub(super) const OPENER_GF_2NT: Alert = Alert("meckstroth-2nt");
/// Responder's `3♣` relay over the game-forcing `2NT` — "describe"
const PUPPET_2NT: Alert = Alert("meckstroth-2nt-relay");
/// Responder's `3NT` over the game-forcing `2NT` — 5+ clubs, doubleton in opener's major
const RESP_CLUBS_2NT: Alert = Alert("meckstroth-2nt-clubs");
/// Opener's `3♦` default shape-out — balanced 18–19 or a four-card minor
const GF_DEFAULT: Alert = Alert("meckstroth-2nt-default");
/// Opener's `3NT` shape-out — five-plus a minor
const GF_MINOR: Alert = Alert("meckstroth-2nt-minor");

/// Whether a rebid is opener's invitational `3♣`/`3♦` jump (the Meckstroth `3m`)
pub(super) fn is_invitational_minor_jump(rebid: Call) -> bool {
    rebid == call(3, Strain::Clubs) || rebid == call(3, Strain::Diamonds)
}

/// Append the Meckstroth-adjunct invitational minor jumps when enabled
///
/// `3♣`/`3♦` show 5+ cards in the minor and ≈15–17 points — the medium shapely
/// hand that otherwise underbids as a natural two-level minor.  The weight sits
/// above the natural minor (0.9) and the six-card-major rebid (1.0) but below
/// the strong 2NT (1.2), so disjointness is by strength: 18–19 balanced → 2NT;
/// 15–17 with a five-card minor → `3m`; a minimum → the natural two level.
pub(super) fn with_invitational_minors(mut rules: Rules) -> Rules {
    if meckstroth() && MECKSTROTH_MINOR_JUMPS.with(Cell::get) {
        for minor in [Suit::Clubs, Suit::Diamonds] {
            rules = rules.rule(
                Bid::new(3, Strain::from(minor)),
                105,
                len(minor, 5..) & points(15..=17),
            );
        }
    }
    rules
}

/// Responder's call over opener's invitational `3m` jump (Meckstroth adjunct)
///
/// Opener has shown 5+ of the minor and ≈15–17 points.  Responder accepts game
/// with a maximum forcing-1NT (or `1♠`) hand and declines to a preference in
/// opener's five-card major with a minimum.  The `len(major, ..)` guards keep
/// the major-preference rules dead when responder is short, so one table serves
/// both the forcing-1NT auctions and `1♥ - 1♠` (where responder's holding in
/// opener's major is unknown).
///
/// | Call   | Wt  | Meaning |
/// |--------|-----|---------|
/// | 4M     | 1.4 | Accept: 5-3 major game (3+ support, 10+ points) |
/// | 3NT    | 1.2 | Accept: notrump game, no major fit (10+ points) |
/// | 3M     | 1.0 | Decline: preference to opener's major (2+ cards, minimum) |
/// | Pass   | 0.0 | Decline: minimum, short in the major — pass the invite |
fn responder_after_invitational_minor(major: Suit) -> Rules {
    let trump = Strain::from(major);
    Rules::new()
        // Accept to the 5-3 major game.
        .rule(Bid::new(4, trump), 140, len(major, 3..) & points(10..))
        // Accept to notrump game with no major fit.
        .rule(Bid::new(3, Strain::Notrump), 120, points(10..))
        // Decline: preference to opener's five-card major.
        .rule(Bid::new(3, trump), 100, len(major, 2..) & points(..10))
        // Catch-all: minimum, short in the major — pass the invitation.
        // ponytail: a 5m minor game is folded into 3NT; add an explicit 5m raise
        // if the A/B shows it matters.
        .rule(Call::Pass, 0, points(0..))
}

/// Responder's call over opener's invitational `3m` (Meckstroth adjunct)
///
/// Covers both the forcing-1NT auctions (`1M - 1NT - 3m`) and the `1♥ - 1♠`
/// auction (`1♥ - 1♠ - 3m`, where opener's major is hearts).  The package gate
/// deliberately follows only the parent Meckstroth knob: with the minor-jump
/// subknob off these continuation nodes remain authored but unreachable, just
/// as they were before the rows port.
pub(crate) fn invitational_minor_continuations() -> Package {
    Package {
        name: "invitational-minor-continuations",
        gate: meckstroth,
        entries: || {
            let three_minors = [call(3, Strain::Clubs), call(3, Strain::Diamonds)];
            let mut entries = Vec::new();

            // Forcing 1NT: 1M - 1NT - 3m, responder's major support unknown.
            for major in [Suit::Hearts, Suit::Spades] {
                let prefix = format!("P* {} - 1NT -", call(1, Strain::from(major)));
                for three_m in three_minors {
                    entries.extend(rows_of(
                        Pattern::node(&format!("{prefix} {three_m} -")),
                        responder_after_invitational_minor(major),
                    ));
                }
            }

            // 1♥ - 1♠ - 3m: opener's major is hearts, responder has shown 4+
            // spades.
            for three_m in three_minors {
                entries.extend(rows_of(
                    Pattern::node(&format!("P* 1♥ - 1♠ - {three_m} -")),
                    responder_after_invitational_minor(Suit::Hearts),
                ));
            }
            entries
        },
    }
}

/// Responder's call over opener's artificial game-forcing `2NT`
///
/// `1M - 1NT - 2NT!` set up a game force (18+, any shape).  Responder shows a
/// fit, a five-card red suit, five clubs (artificially, via `3NT`), or relays
/// `3♣` for opener to describe.  Forcing — the `3♣` relay is the finite
/// catch-all, so there is no `Pass`.
///
/// | Call  | Wt   | Meaning |
/// |-------|------|---------|
/// | 3M    | 1.45 | Fit + slam interest (3+ support, 10+) → RKCB round |
/// | 4M    | 1.40 | Fit, no slam interest (3+ support, ≤9) → to play |
/// | 3♦/3♥ | 1.30 | Natural five-plus red suit (not opener's major) |
/// | 3NT!  | 1.25 | 5+ clubs, doubleton in opener's major (opener may pull) |
/// | 3♣!   | 0.50 | Relay — nothing to show, "you describe" |
fn responder_over_gf_2nt(major: Suit) -> Rules {
    let m = Strain::from(major);
    let mut rules = Rules::new()
        .rule(Bid::new(3, m), 145, len(major, 3..) & points(10..))
        .rule(Bid::new(4, m), 140, len(major, 3..) & points(..=9));
    // Natural five-plus red suits (the game force is set, so free to show).  Over
    // 1♥ only diamonds is available — 1NT denied four spades, and hearts is the fit.
    for red in [Suit::Diamonds, Suit::Hearts] {
        if red != major {
            rules = rules.rule(Bid::new(3, Strain::from(red)), 130, len(red, 5..));
        }
    }
    rules
        // The fourth suit, shown artificially for symmetry with 3♦/3♥: five-plus
        // clubs and exactly a doubleton in opener's major (so opener can pull to
        // a 6-2 game).  Non-forcing — opener may pass 3NT.
        .rule(
            Bid::new(3, Strain::Notrump),
            125,
            len(Suit::Clubs, 5..) & len(major, 2..=2),
        )
        .alert(RESP_CLUBS_2NT)
        // Relay: nothing to show — "you describe".  The finite catch-all.
        .rule(Bid::new(3, Strain::Clubs), 50, points(0..))
        .alert(PUPPET_2NT)
}

/// Opener's shape-out over the `3♣` relay (`1M - 1NT - 2NT! - 3♣!`)
///
/// Opener describes toward the right game or slam.  Forcing — `3♦` is the finite
/// catch-all, so there is no `Pass`.
///
/// | Call  | Wt   | Meaning |
/// |-------|------|---------|
/// | 3M    | 1.35 | Six-plus own major (one-suiter) |
/// | 3(oM) | 1.30 | Four-plus the other major (natural) |
/// | 3NT!  | 1.25 | Five-plus a minor |
/// | 3♦!   | 1.20 | Default — balanced 18–19 or a four-card minor; catch-all |
fn opener_shapeout(major: Suit) -> Rules {
    let m = Strain::from(major);
    let other = other_major(major);
    Rules::new()
        .rule(Bid::new(3, m), 135, len(major, 6..))
        .rule(Bid::new(3, Strain::from(other)), 130, len(other, 4..))
        .rule(
            Bid::new(3, Strain::Notrump),
            125,
            len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..),
        )
        .alert(GF_MINOR)
        // Default: balanced 18–19 or a four-card minor — the guaranteed-legal
        // catch-all (opener is 18+, so `points(0..)` always applies).
        .rule(Bid::new(3, Strain::Diamonds), 120, points(0..))
        .alert(GF_DEFAULT)
}

/// Responder places over opener's `3♦` default (`… - 2NT! - 3♣! - 3♦!`)
///
/// Opener is balanced 18–19 or has a four-card minor, with exactly five of the
/// major (a sixth would have jumped to `3M`).  Responder raises a 5-3 major fit
/// or signs off in `3NT`.
fn resp_place_over_default(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 120, len(major, 3..))
        .rule(Bid::new(3, Strain::Notrump), 100, points(0..))
}

/// Responder places over opener's `3(other major)` (four-plus the other major)
///
/// Raises the concealed 4-4 (or 4-3) fit, falls back to opener's five-card own
/// major with three-card support, else `3NT`.
fn resp_place_over_other_major(major: Suit) -> Rules {
    let o = other_major(major);
    Rules::new()
        .rule(Bid::new(4, Strain::from(o)), 130, len(o, 4..))
        .rule(Bid::new(4, Strain::from(major)), 110, len(major, 3..))
        .rule(Bid::new(3, Strain::Notrump), 80, points(0..))
}

/// Responder places over opener's six-plus own major (`… - 3♣! - 3M`)
///
/// An eight-card major fit is near-certain; responder drives slam with a maximum
/// (`4NT` RKCB) or signs off in game.
fn resp_place_over_six(major: Suit) -> Rules {
    let m = Strain::from(major);
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 130, points(11..))
        .alert(slam::RKCB)
        .rule(Bid::new(4, m), 100, points(0..))
}

/// Responder places over opener's `3NT` (five-plus a minor, i.e. 5-5)
///
/// Non-forcing: responder pulls to a 5-3 major game or passes to play `3NT`.
fn resp_place_over_minor(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 110, len(major, 3..))
        .rule(Call::Pass, 0, points(0..))
}

/// Opener's call over responder's direct fit slam-try (`1M - 1NT - 2NT! - 3M`)
///
/// Responder agreed the major with slam interest; opener asks keycards on a
/// clear maximum, else signs off in game.
fn opener_over_fit_slamtry(major: Suit) -> Rules {
    let m = Strain::from(major);
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 130, points(20..))
        .alert(slam::RKCB)
        .rule(Bid::new(4, m), 50, points(0..))
}

/// Opener's call over responder's natural five-plus red suit
///
/// `red` is responder's suit.  Opener raises a heart fit to game, rebids a
/// six-card own major, else places `3NT` (the guaranteed-legal game).
// ponytail: no diamond-slam exploration — a diamond fit lands in 3NT (game
// reached); add a 4♦ slam-try rung if the A/B shows stranded minor slams.
fn opener_over_resp_red(major: Suit, red: Suit) -> Rules {
    let mut rules = Rules::new();
    if red == Suit::Hearts {
        rules = rules.rule(Bid::new(4, Strain::Hearts), 130, len(Suit::Hearts, 3..));
    }
    rules
        .rule(Bid::new(3, Strain::from(major)), 110, len(major, 6..))
        .rule(Bid::new(3, Strain::Notrump), 50, points(0..))
}

/// Opener's call over responder's `3NT` (five-plus clubs, doubleton major)
///
/// Non-forcing: opener pulls to a 6-2 major game with a sixth card, else passes
/// to play `3NT`.
fn opener_over_resp_clubs(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 100, len(major, 6..))
        .rule(Call::Pass, 0, points(0..))
}

/// The artificial game-forcing `2NT` adjunct
///
/// Authors both sides below `1M - 1NT - 2NT!`: responder's relay round, opener's
/// shape-out over `3♣`, responder's placement over each shape-out (with RKCB on
/// the two major-fit nodes), and opener's placement over responder's own bids.
/// This **overrides** the natural-2NT continuation `notrump.rs` installed at
/// `1M - 1NT - 2NT` — `rebids::register` runs after `notrump::register`, so the
/// on-knob insert wins; with the knob off nothing is authored and the natural
/// handling stands.
pub(crate) fn meckstroth_two_notrump_continuations() -> Package {
    Package {
        name: "meckstroth-two-notrump-continuations",
        gate: meckstroth,
        entries: || {
            let mut entries = Vec::new();
            for major in [Suit::Hearts, Suit::Spades] {
                let m = Strain::from(major);
                let base = format!("P* {} - 1NT - 2NT -", call(1, Strain::from(major)),);

                // Responder's relay round over the game-forcing 2NT.
                entries.extend(rows_of(Pattern::node(&base), responder_over_gf_2nt(major)));

                // Opener's shape-out over the 3♣ relay, and responder's
                // placement over each of opener's four shape-outs.
                let relay = format!("{base} 3♣ -");
                entries.extend(rows_of(Pattern::node(&relay), opener_shapeout(major)));
                entries.extend(rows_of(
                    Pattern::node(&format!("{relay} 3♦ -")),
                    resp_place_over_default(major),
                ));
                entries.extend(rows_of(
                    Pattern::node(&format!(
                        "{relay} {} -",
                        call(3, Strain::from(other_major(major))),
                    )),
                    resp_place_over_other_major(major),
                ));
                let six_node = format!("{relay} {} -", call(3, m));
                entries.extend(rows_of(
                    Pattern::node(&six_node),
                    resp_place_over_six(major),
                ));
                entries.extend(slam::rkcb_rows(&six_node, major));
                entries.extend(rows_of(
                    Pattern::node(&format!("{relay} 3NT -")),
                    resp_place_over_minor(major),
                ));

                // Responder's direct fit slam-try, then RKCB.
                let fit_node = format!("{base} {} -", call(3, m));
                entries.extend(rows_of(
                    Pattern::node(&fit_node),
                    opener_over_fit_slamtry(major),
                ));
                entries.extend(slam::rkcb_rows(&fit_node, major));

                // Opener's placement over responder's natural red suits.
                for red in [Suit::Diamonds, Suit::Hearts] {
                    if red != major {
                        entries.extend(rows_of(
                            Pattern::node(&format!("{base} {} -", call(3, Strain::from(red)),)),
                            opener_over_resp_red(major, red),
                        ));
                    }
                }

                // Opener's placement over responder's 3NT clubs (non-forcing).
                entries.extend(rows_of(
                    Pattern::node(&format!("{base} 3NT -")),
                    opener_over_resp_clubs(major),
                ));
            }
            entries
        },
    }
}
