//! Responder's answers to opener's jump shift after a minor opening
//!
//! The [extras ladder](super::extras_ladder) jump-shifts `1m - 1M - 3x`
//! (`2♠` over a `1♥` response) on 5+ of the minor, 4+ of the second suit and
//! 18+ points, game-forcing — and left responder's whole node to the floor,
//! which has no forcing channel: the `6141cdb2` anchor's sharpest round-2
//! cluster (`3♦` over `1♣ - 1M`: 321 boards, −1.90/bd) was responder passing
//! the jump shift on a weak hand or bidding `3NT` on a singleton, and opener
//! passing `3NT` with a 6-5.  Gated by
//! [`RebidKnobs::minor_jump_shift_answers`]; every call past the jump shift
//! is authored (`-` and `(X)` tails):
//!
//! | Call | Meaning |
//! |------|---------|
//! | `4m`  | 4+ in opener's minor, 13+ points: opener asks keycards in it |
//! | `4NT` | 4+ in a major second suit, 11+ points: keycards in it (18 + 11 is the 29 a major slam wants, `docs/suit-slam.md`) |
//! | `4s`  | 4+ in the second suit: a 6–10 game when `s` is a major (opener asks on 21+), 13+ and a keycard ask when a minor |
//! | `3M`  | 5+ in responder's own major: opener raises on 3+, else `3NT`; responder pulls `3NT` to `4M` on six, to `6NT` on 15+ HCP |
//! | `5m`  | 4+ in opener's minor and a singleton or void outside it |
//! | `3NT` | everything else; opener pulls to `4m` on a 6-5 |

use super::*;
use crate::bidding::american::slam;

/// One jump-shift lane: opener's minor, responder's major, opener's second
/// suit and the level the jump shift lands on
struct Lane {
    minor: Suit,
    major: Suit,
    second: Suit,
    level: u8,
}

impl Lane {
    fn all() -> Vec<Lane> {
        let mut lanes = Vec::new();
        for minor in [Suit::Clubs, Suit::Diamonds] {
            for major in [Suit::Hearts, Suit::Spades] {
                for second in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                    if second == minor || second == major {
                        continue;
                    }
                    let level =
                        cheapest_level_over(Bid::new(1, Strain::from(major)), Strain::from(second))
                            + 1;
                    if level <= 3 {
                        lanes.push(Lane {
                            minor,
                            major,
                            second,
                            level,
                        });
                    }
                }
            }
        }
        lanes
    }

    /// `P* 1m - 1M - Jx`
    fn node(&self) -> String {
        format!(
            "P* 1{} - 1{} - {}{}",
            Strain::from(self.minor),
            Strain::from(self.major),
            self.level,
            Strain::from(self.second)
        )
    }

    /// Responder's answers: the table in the module doc
    fn responder(&self) -> Rules {
        let m = Strain::from(self.minor);
        let s = Strain::from(self.second);
        let second_is_major = matches!(self.second, Suit::Hearts | Suit::Spades);
        let mut rules = Rules::new().rule(Bid::new(4, m), 150, len(self.minor, 4..) & points(13..));
        rules = if second_is_major {
            rules
                .rule(
                    Bid::new(4, Strain::Notrump),
                    155,
                    len(self.second, 4..) & points(11..),
                )
                .alert(slam::RKCB)
                .rule(Bid::new(4, s), 145, len(self.second, 4..))
        } else {
            rules.rule(Bid::new(4, s), 145, len(self.second, 4..) & points(13..))
        };
        // Own major: the cheapest rebid of it above the jump shift.
        let own = cheapest_level_over(Bid::new(self.level, s), Strain::from(self.major));
        rules
            .rule(
                Bid::new(own, Strain::from(self.major)),
                130,
                len(self.major, 5..),
            )
            .rule(
                Bid::new(5, m),
                120,
                len(self.minor, 4..) & shortness_outside(self.minor),
            )
            .rule(Bid::new(3, Strain::Notrump), 0, hcp(0..))
    }

    /// Opener over responder's major rebid: raise on 3+, else `3NT`
    fn opener_over_major(&self) -> Rules {
        Rules::new()
            .rule(
                Bid::new(4, Strain::from(self.major)),
                100,
                len(self.major, 3..),
            )
            .rule(Bid::new(3, Strain::Notrump), 0, hcp(0..))
    }

    /// Responder over opener's `3NT` after the own-major rebid: six pull to
    /// `4M`, 15+ HCP opposite the 18+ jump shift is the 31 of `6NT`
    /// (`docs/notrump-slam.md`)
    fn responder_over_notrump(&self) -> Rules {
        Rules::new()
            .rule(Bid::new(6, Strain::Notrump), 110, hcp(15..))
            .rule(
                Bid::new(4, Strain::from(self.major)),
                100,
                len(self.major, 6..),
            )
            .rule(Call::Pass, 0, hcp(0..))
    }

    /// Opener over responder's `3NT`: a 6-5 pulls to the minor (natural: the
    /// six-card floor is in opener's own, already-bid suit)
    fn opener_over_notrump(&self) -> Rules {
        Rules::new()
            .rule(
                Bid::new(4, Strain::from(self.minor)),
                100,
                len(self.minor, 6..) & len(self.second, 5..),
            )
            .natural()
            .rule(Call::Pass, 0, hcp(0..))
    }
}

/// Opener over a 13+ raise of either suit: always the keycard ask
fn opener_over_raise() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, hcp(0..))
        .alert(slam::RKCB)
}

/// Opener over the 6–10 game raise of a major second suit: keycards on 21+
fn opener_over_major_game() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, points(21..))
        .alert(slam::RKCB)
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's answers, opener's rebids and the RKCB subtrees
pub(crate) fn minor_jump_shift_continuations() -> Package {
    Package {
        name: "minor-jump-shift-answers",
        gate: |a| a.rebid.minor_jump_shift_answers,
        entries: |_| {
            let mut entries = Vec::new();
            for lane in Lane::all() {
                let m = Strain::from(lane.minor);
                let s = Strain::from(lane.second);
                let own = cheapest_level_over(Bid::new(lane.level, s), Strain::from(lane.major));
                for tail in ["-", "(X)"] {
                    let node = format!("{} {tail}", lane.node());
                    entries.extend(rows_of(Pattern::node(&node), lane.responder()));
                    if matches!(lane.second, Suit::Hearts | Suit::Spades) {
                        entries.extend(slam::rkcb_rows(&node, lane.second));
                    }
                    for answer in ["-", "(X)"] {
                        let raise = format!("{node} 4{m} {answer}");
                        entries.extend(rows_of(Pattern::node(&raise), opener_over_raise()));
                        entries.extend(slam::rkcb_rows(&raise, lane.minor));
                        if matches!(lane.second, Suit::Clubs | Suit::Diamonds) {
                            let raise = format!("{node} 4{s} {answer}");
                            entries.extend(rows_of(Pattern::node(&raise), opener_over_raise()));
                            entries.extend(slam::rkcb_rows(&raise, lane.second));
                        } else {
                            let game = format!("{node} 4{s} {answer}");
                            entries.extend(rows_of(Pattern::node(&game), opener_over_major_game()));
                            entries.extend(slam::rkcb_rows(&game, lane.second));
                        }
                        let major = format!("{node} {own}{} {answer}", Strain::from(lane.major));
                        entries.extend(rows_of(Pattern::node(&major), lane.opener_over_major()));
                        for again in ["-", "(X)"] {
                            let pull = format!("{major} 3NT {again}");
                            entries.extend(rows_of(
                                Pattern::node(&pull),
                                lane.responder_over_notrump(),
                            ));
                        }
                        let notrump = format!("{node} 3NT {answer}");
                        entries
                            .extend(rows_of(Pattern::node(&notrump), lane.opener_over_notrump()));
                    }
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
