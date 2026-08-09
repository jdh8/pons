//! The invitational `2NT` continuation in the rich advance of partner's takeout double
//!
//! The accept-or-decline structure is gated by
//! `agreements.defense.advance_2nt_continuation_enabled`.

use super::advance_minor_jump::advance_minor_jump_rebid;
use super::*;

/// Doubler's accept-or-decline of the advancer's invitational `2NT`
/// (`(1t) X - 2NT { - | (X) } ?`, gated by
/// `agreements.defense.advance_2nt_continuation_enabled`)
///
/// The `2NT` invite is a limited balanced 11–12 with a stopper (the advancer
/// supplies the notrump stopper), so the doubler — sitting on the wide takeout
/// range — simply answers a natural invite: **Pass** declines with a minimum,
/// **`3NT`** accepts to play, and a **new 5-card major** accepts game-forcing so
/// the advancer can choose the 4-4/5-3 major game over `3NT` (the advancer places
/// it — [`advance_minor_jump_rebid`], the same accept-a-forcing-suit logic).  A
/// 5-card *minor* is not shown: with the advancer's stopper `3NT` is almost
/// always right, so only the fit-seeking majors are worth the detour.  All
/// natural; nothing artificial to alert.
fn answer_advance_2nt(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let mut rules = Rules::new()
        // Accept to play: 3NT with a maximum (the advancer holds the stopper).
        .rule(Bid::new(3, Strain::Notrump), 120, hcp(14..))
        // Minimum: decline the invite, play 2NT.
        .rule(Call::Pass, 0, hcp(0..));
    // Accept game-forcing by showing a 5-card major to seek the fit.
    for major in [Suit::Hearts, Suit::Spades] {
        let s = Strain::from(major);
        if s == theirs {
            continue;
        }
        rules = rules.rule(Bid::new(3, s), 130, points(14..) & len(major, 5..));
    }
    rules
}

/// Invitational-`2NT` continuation rows for one opening suit
pub(super) fn advance_2nt_rows(
    base: &str,
    theirs: Strain,
    opening: Bid,
    agreements: &Agreements,
) -> Vec<Entry> {
    let mut entries = Vec::new();
    if agreements.defense.advance_2nt_continuation_enabled {
        for rho in ["-", "(X)"] {
            let after_2nt = format!("{base} 2NT {rho}");
            entries.extend(rows_of(
                Pattern::node(&after_2nt),
                answer_advance_2nt(opening),
            ));
            // The advancer places game over each forcing major the
            // doubler can show (an unbid major at the three level).
            for major in [Suit::Hearts, Suit::Spades] {
                let s = Strain::from(major);
                if s == theirs {
                    continue;
                }
                let bid = Bid::new(3, s);
                for rho2 in ["-", "(X)"] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("{after_2nt} {bid} {rho2}")),
                        advance_minor_jump_rebid(major),
                    ));
                }
            }
        }
    }
    entries
}

#[cfg(test)]
mod tests;
