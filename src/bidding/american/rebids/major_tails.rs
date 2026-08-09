//! Major-rebid tails: full continuations after `1♥ - 1♠`
//!
//! Below each of opener's four rebids (`2♠`, `3♠`, `2♥`, `2♣`/`2♦`) both sides
//! are authored to game, and — for the two spade-raise auctions — to slam via
//! RKCB.  Gated by [`RebidKnobs::major_rebid_tails`].  Two sub-agreements ride
//! it: fourth-suit forcing ([`RebidKnobs::fourth_suit_forcing`]) and the HCP
//! gauge on the one no-fit rung ([`RebidKnobs::nt_invite_hcp`]).

use super::*;
use crate::bidding::american::slam;

// ponytail: same construction-time-toggle reasoning as `MECKSTROTH` above.
/// Fourth suit forcing — the fourth suit is an artificial game force
const FOURTH_SUIT: Alert = Alert("fourth-suit-forcing");

// ponytail: same construction-time-toggle reasoning as `MECKSTROTH` above.
/// Responder's second call after opener raises to `2♠` in `1♥ - 1♠`
///
/// Opener's `2♠` shows four-card support and a 12–15 point opening.  The
/// `4NT` keycard ask is authored the same way as the 2/1 game force's
/// opener-third rule: the call itself carries the RKCB alert, and
/// `slam::rkcb_rows` authors everything below it.
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 4NT  | 2.0 | Keycard ask: slam interest opposite a maximum (16+ points) |
/// | 4♠   | 1.5 | Sign off in game (12+ points) |
/// | 3♠   | 1.2 | Invitational raise (10–11 points) |
/// | Pass | 0.0 | Minimum, decline any further invitation |
#[must_use]
fn responder_after_spade_raise() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 200, points(16..))
        .alert(slam::RKCB)
        .rule(Bid::new(4, Strain::Spades), 150, points(12..))
        .rule(Bid::new(3, Strain::Spades), 120, points(10..=11))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's second call after opener jumps to `3♠` in `1♥ - 1♠`
///
/// Opener's `3♠` shows four-card support and a strong 16–18 point opening —
/// game is close to guaranteed, so responder's only question is whether to
/// explore slam or sign off.
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 4NT  | 1.5 | Keycard ask: slam interest (14+ points) |
/// | 4♠   | 1.0 | Accept to game (8+ points) |
/// | Pass | 0.0 | Minimum, decline |
#[must_use]
fn responder_after_spade_jump() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 150, points(14..))
        .alert(slam::RKCB)
        .rule(Bid::new(4, Strain::Spades), 100, points(8..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's second call after opener rebids `2♥` in `1♥ - 1♠`
///
/// Opener's `2♥` shows a six-card suit; responder's `1♠` did not deny three
/// hearts, so a heart fit is common at this node.
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 4♥   | 1.5 | Raise to game, 2+ hearts (13+ points) |
/// | 3NT  | 1.3 | Game with no heart fit (13+ points) |
/// | 3♥   | 1.2 | Invitational raise, 2+ hearts (10–12 points) |
/// | 2NT  | 1.0 | Natural notrump invite (10–12 points) |
/// | Pass | 0.0 | Minimum, nothing further |
#[must_use]
fn responder_after_heart_rebid() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Hearts),
            150,
            len(Suit::Hearts, 2..) & points(13..),
        )
        .rule(Bid::new(3, Strain::Notrump), 130, points(13..))
        .rule(
            Bid::new(3, Strain::Hearts),
            120,
            len(Suit::Hearts, 2..) & points(10..=12),
        )
        .rule(Bid::new(2, Strain::Notrump), 100, points(10..=12))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's call over responder's `2NT` notrump invite after `1♥ - 1♠ - 2♥`
///
/// Forcing: the `3♥` retreat is always legal below `2NT`, so there is no pass
/// rule.  Accept with 14+ HCP (bid `3NT`), decline with a `3♥` retreat.
#[must_use]
fn opener_after_heart_invite() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(14..))
        .rule(Bid::new(3, Strain::Hearts), 50, hcp(0..))
}

/// Responder's second call after opener rebids a new minor in `1♥ - 1♠`
///
/// Registered at both `1♥ - 1♠ - 2♣` and `1♥ - 1♠ - 2♦` — `minor` is the suit
/// opener rebid, showing 4+ cards on a minimum-ish hand.  Responder's known
/// assets are 4+ spades and 6+ points; heart length is unknown (`1♠` never
/// denied three hearts), so a jump preference to `3♥` outranks the minor
/// raise.
///
/// Fourth-suit-forcing ([`RebidKnobs::fourth_suit_forcing`], riding the
/// major-rebid-tails adjunct) extends this table for `minor == Suit::Clubs`
/// only: a `2♦` response becomes an artificial game force (the fourth suit
/// below `2♣`) instead of natural diamonds.  `minor == Suit::Diamonds` is
/// untouched — the fourth suit there would be a `3♣` jump, out of scope here.
///
/// | Call | Wt   | Meaning |
/// |------|------|---------|
/// | 2♦   | 2.0  | Fourth-suit-forcing game force, 12+ (clubs only, knob-gated) |
/// | 3♥   | 1.3  | Invitational jump preference, 3+ hearts (10–12) |
/// | 3m   | 1.25 | Invitational raise of opener's minor, 5+ (10–12) |
/// | 2NT  | 1.2  | Natural notrump invite (10–12) |
/// | 2♠   | 1.05 | Weak rebid, 6+ spades, to play (≤9) |
/// | 2♥   | 1.0  | Simple preference, 2+ hearts (6–9) |
/// | 3NT  | 0.9  | Game with no fit found (13+) |
/// | Pass | 0.0  | Minimum, nothing further |
#[must_use]
fn responder_after_minor_rebid(minor: Suit, knobs: &RebidKnobs) -> Rules {
    let m = Strain::from(minor);
    let mut rules = Rules::new();
    if minor == Suit::Clubs && knobs.fourth_suit_forcing {
        // Fourth-suit-forcing: an artificial game force.  Points-only on
        // purpose — the projection must claim nothing about diamond length.
        rules = rules
            .rule(Bid::new(2, Strain::Diamonds), 200, points(12..))
            .alert(FOURTH_SUIT);
    }
    rules = rules
        .rule(
            Bid::new(3, Strain::Hearts),
            130,
            len(Suit::Hearts, 3..) & points(10..=12),
        )
        .rule(Bid::new(3, m), 125, len(minor, 5..) & points(10..=12));
    // The one no-fit rung: HCP-gauged when `nt_invite_hcp` is armed (a
    // notrump invite takes no ruffs), else the shipped `points`.
    rules = if knobs.nt_invite_hcp {
        rules.rule(Bid::new(2, Strain::Notrump), 120, hcp(10..=12))
    } else {
        rules.rule(Bid::new(2, Strain::Notrump), 120, points(10..=12))
    };
    rules
        .rule(
            Bid::new(2, Strain::Spades),
            105,
            len(Suit::Spades, 6..) & hcp(..=9),
        )
        .rule(
            Bid::new(2, Strain::Hearts),
            100,
            len(Suit::Hearts, 2..) & hcp(6..=9),
        )
        .rule(Bid::new(3, Strain::Notrump), 90, hcp(13..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's call over responder's raise to `3m` after `1♥ - 1♠ - 2m`
///
/// Accept with 14+ points (bid `3NT`), decline with a pass.  Unlike
/// `opener_accept_limit_raise`, game lives in notrump here — the minor is
/// opener's second suit, not the agreed trump.
#[must_use]
fn opener_accept_minor_raise() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, points(14..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer at `1♥ - 1♠ - 2♣ - 2♦ -`, the fourth-suit-forcing game force
///
/// Forcing — there is no pass rule; the `2♥` catch-all is always legal
/// because opener holds 5+ hearts (guaranteed by the `1♥` opening) and `2♥`
/// outranks `2♦`.
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 2♠   | 1.4 | Delayed three-card raise |
/// | 2♥   | 1.3 | Extra heart length, 6+ |
/// | 2NT  | 1.2 | Notrump with the fourth suit stopped |
/// | 3♣   | 1.1 | A real second suit, 5+ |
/// | 2♥   | 0.2 | Guaranteed-legal catch-all |
#[must_use]
fn opener_after_fourth_suit() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Spades), 140, len(Suit::Spades, 3..))
        .rule(Bid::new(2, Strain::Hearts), 130, len(Suit::Hearts, 6..))
        .rule(
            Bid::new(2, Strain::Notrump),
            120,
            stopper_in(Suit::Diamonds),
        )
        .rule(Bid::new(3, Strain::Clubs), 110, len(Suit::Clubs, 5..))
        .rule(Bid::new(2, Strain::Hearts), 20, len(Suit::Hearts, 5..))
}

/// Responder's placement at `1♥ - 1♠ - 2♣ - 2♦ - answer -`, after opener
/// answers the fourth-suit-forcing game force
///
/// One shared table installed at every answer `X` from
/// [`opener_after_fourth_suit`] — [`partner_suit_is`] reads which answer
/// opener actually gave, so a single table serves all of them.  Forcing to
/// game: `3NT` is always legal since every `X` is at or below `3♣`, so there
/// is no pass rule.
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 4♠   | 1.5 | Opener showed 3-card spade support; 5-3 fit |
/// | 4♥   | 1.2 | Opener opened `1♥` (5+); 5-3 fit |
/// | 4♥   | 1.1 | Opener rebid hearts twice (6+); 6-2 fit |
/// | 3NT  | 0.8 | The game-force landing spot, always legal |
#[must_use]
fn responder_after_fourth_suit_answer() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Spades),
            150,
            partner_suit_is(Suit::Spades) & len(Suit::Spades, 5..),
        )
        .rule(Bid::new(4, Strain::Hearts), 120, len(Suit::Hearts, 3..))
        .rule(
            Bid::new(4, Strain::Hearts),
            110,
            partner_suit_is(Suit::Hearts) & len(Suit::Hearts, 2..),
        )
        .rule(Bid::new(3, Strain::Notrump), 80, hcp(0..))
}

/// The major-rebid-tails adjunct: full continuations after `1♥ - 1♠`
///
/// Below each of opener's four rebids this authors both sides' continuations
/// to game, and — for the two spade-raise auctions — to slam via RKCB:
///
/// - `2♠` (raise, 12–15): responder invites, signs off, or asks keycards;
///   opener accepts or declines the `3♠` invite.
/// - `3♠` (jump raise, 16–18): responder signs off or asks keycards.
/// - `2♥` (own suit, 6+): responder invites or signs off; opener accepts,
///   declines, or answers the `2NT` notrump-invite relay.
/// - `2♣`/`2♦` (new minor, 4+, minimum-ish): responder chooses a preference,
///   an invite, or game; opener accepts or declines the invite reached.
/// - `2♣ - 2♦` fourth-suit-forcing ([`RebidKnobs::fourth_suit_forcing`], an
///   additional gate riding this adjunct): opener answers naturally below
///   game; responder places the final contract at game over any answer.
///
/// `1♥ - 1♠ - 2m - 2♥` and `1♥ - 1♠ - 2m - 2♠` are deliberately left to the
/// floor.
pub(crate) fn major_rebid_tail_continuations() -> Package {
    Package {
        name: "major-rebid-tail-continuations",
        gate: |a| a.rebid.major_rebid_tails,
        entries: |agreements| {
            let knobs = &agreements.rebid;
            let base = "P* 1♥ - 1♠ -";
            let mut entries = Vec::new();

            // Opener's 2♠ raise (12–15, four-card support):
            // invite/sign-off/RKCB.
            let after_two_spades = format!("{base} 2♠ -");
            entries.extend(rows_of(
                Pattern::node(&after_two_spades),
                responder_after_spade_raise(),
            ));
            entries.extend(rows_of(
                Pattern::node(&format!("{after_two_spades} 3♠ -")),
                opener_accept_limit_raise(Suit::Spades),
            ));
            entries.extend(slam::rkcb_rows(&after_two_spades, Suit::Spades));

            // Opener's 3♠ jump raise (16–18, four-card support): sign-off or
            // RKCB.
            let after_three_spades = format!("{base} 3♠ -");
            entries.extend(rows_of(
                Pattern::node(&after_three_spades),
                responder_after_spade_jump(),
            ));
            entries.extend(slam::rkcb_rows(&after_three_spades, Suit::Spades));

            // Opener's 2♥ rebid (own suit, 6+): invite/sign-off, and the 2NT
            // relay.
            let after_two_hearts = format!("{base} 2♥ -");
            entries.extend(rows_of(
                Pattern::node(&after_two_hearts),
                responder_after_heart_rebid(),
            ));
            entries.extend(rows_of(
                Pattern::node(&format!("{after_two_hearts} 3♥ -")),
                opener_accept_limit_raise(Suit::Hearts),
            ));
            entries.extend(rows_of(
                Pattern::node(&format!("{after_two_hearts} 2NT -")),
                opener_after_heart_invite(),
            ));

            // Opener's 2♣/2♦ new minor (4+, minimum-ish): preference, invite,
            // or game.
            for minor in [Suit::Clubs, Suit::Diamonds] {
                let after_minor = format!("{base} {} -", call(2, Strain::from(minor)));
                entries.extend(rows_of(
                    Pattern::node(&after_minor),
                    responder_after_minor_rebid(minor, knobs),
                ));
                entries.extend(rows_of(
                    Pattern::node(&format!("{after_minor} 2NT -")),
                    opener_accept_notrump_invite(),
                ));
                entries.extend(rows_of(
                    Pattern::node(&format!("{after_minor} {} -", call(3, Strain::from(minor)),)),
                    opener_accept_minor_raise(),
                ));
                entries.extend(rows_of(
                    Pattern::node(&format!("{after_minor} 3♥ -")),
                    opener_accept_limit_raise(Suit::Hearts),
                ));
            }

            entries
        },
    }
}

/// Whether the fourth-suit tail's nested pair of construction-time gates is on
fn fourth_suit_forcing_continuations_enabled(knobs: &RebidKnobs) -> bool {
    knobs.major_rebid_tails && knobs.fourth_suit_forcing
}

/// Opener's answers and responder's placements after fourth-suit forcing
pub(crate) fn fourth_suit_forcing_continuations() -> Package {
    Package {
        name: "fourth-suit-forcing-continuations",
        gate: |a| fourth_suit_forcing_continuations_enabled(&a.rebid),
        entries: |_| {
            let prefix = "P* 1♥ - 1♠ - 2♣ - 2♦ -";
            let opener_rules = opener_after_fourth_suit();
            let answers: Vec<Call> = {
                let mut seen = std::collections::HashSet::new();
                opener_rules
                    .rules()
                    .iter()
                    .filter_map(|rule| {
                        let answer = rule.call();
                        if seen.insert(answer) {
                            Some(answer)
                        } else {
                            None
                        }
                    })
                    .collect()
            };
            let mut entries = rows_of(Pattern::node(prefix), opener_rules);
            for answer in answers {
                entries.extend(rows_of(
                    Pattern::node(&format!("{prefix} {answer} -")),
                    responder_after_fourth_suit_answer(),
                ));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
