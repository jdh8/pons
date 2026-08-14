//! The Puppet-scheme minor structures — `2NT` diamonds and the two-way `2♠`
//!
//! Under [`PUPPET`](super::PUPPET) (the shipped default) `2NT` transfers to
//! diamonds and `2♠` is clubs-or-a-balanced-invite.  The European twin lives in
//! [`super::european`], Puppet Stayman itself in [`super::puppet_stayman`].
//!
//! Both minors run the same lane: weak hands sign off in the three-level minor,
//! game-going hands with a singleton **splinter** in it and let opener place the
//! game (`3NT` with the short suit stopped, else `5m`), and game-going hands
//! without one bid `3NT`.  Two conventions hold that lane together:
//!
//! * The game/partscore boundary is a hardcoded `8` at every site — one below the
//!   `9` a balanced hand needs, since these are all long-suit hands.  It is *not*
//!   a knob; the lane places games, it is not a slam try.
//! * The splinter is only offered where the fit is assured.  Clubs: responder's own
//!   six-card suit.  Diamonds: opener's `3♦` (three-card support) or, over the
//!   `3♣` denial, responder's own six.  A 5♦4♣ hand opposite the denial has no fit
//!   and bids `3NT` — see [`diamond_transfer_correct`].
//!
//! The two minors disagree on one detail: the club side reads shortness as bare
//! `len(short, ..2)`, the diamond side as [`splinter_short`], which excludes a
//! stiff ace or king as a working honor.  Deliberate — aligning them is its own
//! A/B (see [`club_splinter`]).

use super::size_ask::{SizeAskEight, size_ask_eight_class};
use super::transfer_gf::splinter_short;
use super::*;

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
pub(super) fn puppet_minors(agreements: &Agreements) -> Rules {
    // 2♠ = six-card clubs, plus the bare-8 balanced size ask (no four-card major),
    // gated on `size_ask_eight`: `Shipped` excludes the flat 4-3-3-3 (it passes),
    // `Invite` size-asks the whole class, `Pass` drops the invite (clubs only).
    let two_spades = match agreements.notrump.size_ask_eight {
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
        .rule(Bid::new(2, Strain::Notrump), 130, two_notrump_class())
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

/// The hand class shown by the `2NT` diamond transfer: 6+♦, or 5♦ with 4+♣
pub(super) fn two_notrump_class() -> Cons<impl Constraint + Clone> {
    len(Suit::Diamonds, 6..) | (len(Suit::Diamonds, 5..) & len(Suit::Clubs, 4..))
}

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
fn diamond_transfer_answer(agreements: &Agreements) -> Rules {
    let completion_alerts = agreements.decision.reading.completion_alerts;
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 100, len(Suit::Diamonds, 3..))
        .alert_if(completion_alerts, COMPLETION)
        .rule(Bid::new(3, Strain::Clubs), 50, len(Suit::Diamonds, ..3))
        .alert_if(completion_alerts, COMPLETION)
}

/// Responder's major splinters over opener's answer to the `2NT` transfer
///
/// `shape` is the diamond holding the fit rests on, which differs per node — see
/// the two callers.  Empty unless the `diamond_splinter` knob is on.
fn diamond_splinter_rows(
    shape: Cons<impl Constraint + Clone + 'static>,
    threshold: u8,
    on: bool,
) -> Rules {
    if !on {
        return Rules::new();
    }
    Rules::new()
        .rule(
            Bid::new(3, Strain::Hearts),
            100,
            shape.clone() & hcp(threshold..) & splinter_short(Suit::Hearts),
        )
        .alert(SPLINTER)
        .rule(
            Bid::new(3, Strain::Spades),
            100,
            shape.clone() & hcp(threshold..) & splinter_short(Suit::Spades),
        )
        .alert(SPLINTER)
}

/// Responder's rebid after opener completes the diamond transfer (`…2NT - 3♦`)
///
/// Game values bid 3NT — a long suit bids game on fewer points (`threshold` ≈ 8,
/// below the 9 a balanced hand needs).  Otherwise pass the diamond partscore.
///
/// Opener's `3♦` promises three-card support, so *both* members of the `2NT` class
/// hold an eight-card fit: with `splinter` on, a game-forcing hand short in a major
/// bids it and lets opener place the game.
pub(super) fn diamond_transfer_game(threshold: u8, splinter: bool) -> Rules {
    diamond_splinter_rows(two_notrump_class(), threshold, splinter)
        .rule(Bid::new(3, Strain::Notrump), 90, hcp(threshold..))
        .rule(Call::Pass, 0, hcp(..threshold))
}

/// Responder's rebid after opener's pass-or-correct `3♣` (`…2NT - 3♣`, short ♦)
///
/// Game values bid 3NT; a six-card diamond suit retreats to `3♦` (a 6-2 fit beats
/// the possible club misfit); otherwise (5♦4♣) pass and sit for opener's clubs.
///
/// Opener denied diamond support here, so the `splinter` arm needs a self-sufficient
/// six-card suit — the 5♦4♣ hand has no fit to splinter into and bids `3NT`.
fn diamond_transfer_correct(threshold: u8, splinter: bool) -> Rules {
    diamond_splinter_rows(len(Suit::Diamonds, 6..), threshold, splinter)
        .rule(Bid::new(3, Strain::Notrump), 90, hcp(threshold..))
        .rule(
            Bid::new(3, Strain::Diamonds),
            50,
            len(Suit::Diamonds, 6..) & hcp(..threshold),
        )
        .rule(Call::Pass, 0, len(Suit::Diamonds, ..6) & hcp(..threshold))
}

/// A six-card club one-suiter short in `short` with game values — a splinter shape
///
/// ponytail: bare `len(short, ..2)`, so this splinters on a singleton ace — the
/// working honor that [`splinter_short`] (used by the diamond twin above) excludes.
/// Aligning them is its own A/B.
pub(super) fn club_splinter(short: Suit, threshold: u8) -> Cons<impl Constraint + Clone> {
    len(Suit::Clubs, 6..) & hcp(threshold..) & len(short, ..2)
}

/// A six-card club hand with game values and no singleton — game-going, slamless
pub(super) fn club_no_shortness(threshold: u8) -> Cons<impl Constraint + Clone> {
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
fn two_spade_answer(agreements: &Agreements) -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Clubs),
            100,
            hcp(agreements.notrump.size_ask_accept_floor..),
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
pub(super) fn pick_game_over_club_splinter(short: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, stopper_in(short))
        .rule(Bid::new(5, Strain::Clubs), 90, hcp(0..))
}

/// Opener picks the game over responder's diamond splinter: 3NT with the short
/// suit stopped, else 5♦
fn pick_game_over_diamond_splinter(short: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, stopper_in(short))
        .rule(Bid::new(5, Strain::Diamonds), 90, hcp(0..))
}

/// Puppet-scheme 1NT - 2NT diamond transfer and its continuations
pub(crate) fn diamond_transfer() -> Package {
    Package {
        name: "diamond-transfer",
        gate: |agreements| puppet_scheme(agreements),
        entries: |agreements| {
            let splinter = agreements.notrump.diamond_splinter;
            let mut entries = rows_of(
                Pattern::node("P* 1NT - 2NT -"),
                diamond_transfer_answer(agreements),
            );
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2NT - 3♦ -"),
                diamond_transfer_game(8, splinter),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2NT - 3♣ -"),
                diamond_transfer_correct(8, splinter),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2NT - 3♣ - 3♦ -"),
                pass_out(),
            ));
            if splinter {
                let major = |b: &Bindings| matches!(b.suit('x'), Suit::Hearts | Suit::Spades);
                entries.extend(expand("P* 1NT - 2NT - 3♦ - 3x -", major, |b| {
                    pick_game_over_diamond_splinter(b.suit('x'))
                }));
                entries.extend(expand("P* 1NT - 2NT - 3♣ - 3x -", major, |b| {
                    pick_game_over_diamond_splinter(b.suit('x'))
                }));
            }
            entries
        },
    }
}

/// Puppet-scheme two-way 1NT - 2♠ structure and club-splinter continuations
pub(crate) fn two_spade_two_way() -> Package {
    Package {
        name: "two-spade-two-way",
        gate: |agreements| puppet_scheme(agreements),
        entries: |agreements| {
            let mut entries = rows_of(Pattern::node("P* 1NT - 2♠ -"), two_spade_answer(agreements));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♠ - 2NT -"),
                two_spade_over_min(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♠ - 3♣ -"),
                two_spade_over_max(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♠ - 2NT - 3♣ -"),
                pass_out(),
            ));
            entries.extend(expand(
                "P* 1NT - 2♠ - 2NT - 3x -",
                |b| b.suit('x') != Suit::Clubs,
                |b| pick_game_over_club_splinter(b.suit('x')),
            ));
            entries.extend(expand(
                "P* 1NT - 2♠ - 3♣ - 3x -",
                |b| b.suit('x') != Suit::Clubs,
                |b| pick_game_over_club_splinter(b.suit('x')),
            ));
            entries
        },
    }
}
