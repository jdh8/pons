//! Invitational minor jumps in the rich advance of partner's takeout double
//!
//! The jump and its continuations are gated by
//! `agreements.defense.advance_minor_jump_enabled`.

use super::*;

/// Doubler's accept-or-decline of the advancer's invitational minor jump
/// (`(1t) X - 3m { - | (X) } ?`, gated by
/// `agreements.defense.advance_minor_jump_enabled`)
///
/// The jump is a *limited* natural invite (10–12, 5+ `minor`, no 4-card unbid
/// major) that does **not** promise a stopper, so — unlike the forcing cue,
/// which the doubler may never pass — the continuation is a natural-invite
/// accept/decline: **Pass** declines (too weak for game), a **new 5+ suit**
/// accepts game-forcing (the advancer places it — [`advance_minor_jump_rebid`]),
/// and **`3NT`** accepts to play *with the doubler's own stopper*.  With game
/// values but **no** stopper and no biddable side suit the doubler instead
/// **cues their suit** — a Western stopper-ask; the advancer supplies the
/// notrump from its side ([`advance_minor_stopper_ask_answer`]), right-siding
/// `3NT` when it holds the stopper.  The cue is the only artificial call here
/// (`ADVANCE_CUE`); the rest are natural.
fn answer_advance_minor_jump(their_opening: Bid, minor: Suit) -> Rules {
    let theirs = their_opening.strain;
    let m = Strain::from(minor);
    let mut rules = Rules::new()
        // Accept to play: 3NT with values and a stopper.
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            hcp(15..) & stopper_in_their_suits(),
        )
        // Too weak for game: decline (the invite is limited, so Pass is safe).
        .rule(Call::Pass, 0, hcp(0..));
    // Accept by showing a new 5+ suit (game-forcing) — any unbid suit above the
    // jump, biddable at the three level.
    for suit in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let s = Strain::from(suit);
        if s == theirs || s <= m {
            continue;
        }
        rules = rules.rule(Bid::new(3, s), 130, points(15..) & len(suit, 5..));
    }
    // Game values but no stopper and no 5-card side suit: cue their suit to ask
    // the advancer for the stopper (a Western cue).  Lowest-weighted of the game
    // tries, so a hand with its own stopper (`3NT`) or a biddable side suit (a
    // new suit) is routed there first; only the shapeless stopperless 15+ lands
    // here.  Always legal — the minor jump exists only *below* their suit, so
    // 3-of-their-suit sits above `3m` and below `3NT`.  Artificial → `ADVANCE_CUE`.
    rules = rules
        .rule(Bid::new(3, theirs), 100, hcp(15..))
        .alert(ADVANCE_CUE);
    rules
}

/// Advancer's placement after the doubler accepts the minor jump with a forcing
/// new suit (`(1t) X - 3m { - | (X) } 3S { - | (X) } ?`, gated by
/// `agreements.defense.advance_minor_jump_enabled`)
///
/// The doubler forced to game showing a 5+ `shown` suit; the advancer (already
/// limited to 10–12) places it: raise to game with three-card support, else
/// `3NT` (a stopper preferred, but the game is on either way).
pub(super) fn advance_minor_jump_rebid(shown: Suit) -> Rules {
    let s = Strain::from(shown);
    let game = if matches!(shown, Suit::Hearts | Suit::Spades) {
        4
    } else {
        5
    };
    Rules::new()
        // Support: raise the doubler's suit to game.
        .rule(Bid::new(game, s), 100, len(shown, 3..))
        // No support: notrump game (stopper preferred, else forced — game is on).
        .rule(Bid::new(3, Strain::Notrump), 60, stopper_in_their_suits())
        .rule(Bid::new(3, Strain::Notrump), 20, hcp(0..))
}

/// Advancer's answer to the doubler's stopper-ask cue after the minor jump
/// (`(1t) X - 3m { - | (X) } 3t { - | (X) } ?`, gated by
/// `agreements.defense.advance_minor_jump_enabled`)
///
/// The doubler cued their suit holding game values but no stopper (and no 5-card
/// side suit); the advancer supplies the notrump decision.  With a stopper the
/// advancer bids **`3NT`** — right-siding it, so the opening lead runs up to the
/// advancer's tenace — otherwise no stopper sits on either side, so the advancer
/// signs off in the **minor game** (both hands have shown game values).  Natural;
/// nothing to alert.
fn advance_minor_stopper_ask_answer(minor: Suit) -> Rules {
    let m = Strain::from(minor);
    Rules::new()
        // Stopper: the right-sided notrump game (the lead comes up to us).
        .rule(Bid::new(3, Strain::Notrump), 130, stopper_in_their_suits())
        // No stopper anywhere: play the minor game (game values are established).
        .rule(Bid::new(5, m), 50, hcp(0..))
}

/// Invitational-minor-jump continuation rows for one opening suit
pub(super) fn advance_minor_jump_rows(
    base: &str,
    theirs: Strain,
    opening: Bid,
    agreements: &Agreements,
) -> Vec<Entry> {
    let mut entries = Vec::new();
    if agreements.defense.advance_minor_jump_enabled {
        for minor in [Suit::Clubs, Suit::Diamonds] {
            let m = Strain::from(minor);
            // A three-level minor jump exists only below their suit.
            if m >= theirs {
                continue;
            }
            let jump = Bid::new(3, m);
            for rho in ["-", "(X)"] {
                let after_jump = format!("{base} {jump} {rho}");
                entries.extend(rows_of(
                    Pattern::node(&after_jump),
                    answer_advance_minor_jump(opening, minor),
                ));
                // The advancer places game over each forcing new suit
                // the doubler can show (any unbid suit above the jump).
                for shown in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                    let s = Strain::from(shown);
                    if s == theirs || s <= m {
                        continue;
                    }
                    let bid = Bid::new(3, s);
                    for rho2 in ["-", "(X)"] {
                        entries.extend(rows_of(
                            Pattern::node(&format!("{after_jump} {bid} {rho2}")),
                            advance_minor_jump_rebid(shown),
                        ));
                    }
                }
                // The advancer answers the doubler's stopper-ask cue
                // (3 of their suit): 3NT with a stopper (right-sided),
                // else the minor game.
                let ask = Bid::new(3, theirs);
                for rho2 in ["-", "(X)"] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("{after_jump} {ask} {rho2}")),
                        advance_minor_stopper_ask_answer(minor),
                    ));
                }
            }
        }
    }
    entries
}

#[cfg(test)]
mod tests;
