//! Responder's strong jump shift `1♥ - 2♠` and the two rounds below it
//!
//! Gated by [`ResponseKnobs::heart_spade_strong_jump_shift`] (default
//! `None`: `2♠` stays the weak jump shift).  `Some(floor)`: an unpassed
//! hand's `2♠` shows five or more spades, at most two hearts and `floor`+
//! HCP, game-forcing; a passed hand keeps the weak one.  BBA plays it from
//! 15; the anchor loss is on 16–17, slams the `1♠` response signs off under
//! at round 2.
//!
//! Opener describes, responder places or asks:
//!
//! | Opener | Meaning | Responder |
//! |--------|---------|-----------|
//! | `3♠`  | 3+ spades | `4NT` RKCB on 18+, else `4♠` |
//! | `3♥`  | 6+ hearts | on two hearts `4NT` RKCB on 18+ else `4♥`; `3♠` on 6+ spades; `3NT` |
//! | `3♦` / `3♣` | 4+ cards | `4m` on 4+ (opener asks on 15+, else `5m`); `3♠` on 6+ spades; `3NT` |
//! | `2NT` | the rest, so 2=5=3=3 | `6NT` on 21+, quantitative `4NT` on 19–20, `4♠` on 6+ spades, `3NT` |
//!
//! Over a placed `4M` opener asks keycards on 15+ points and passes below;
//! over the six-card `3♠` opener bids `4♠` on a doubleton.  Opener's node
//! is authored over their double too; everything else contested, and opener
//! over `3NT`, is the floor's.

use super::super::slam;
use crate::bidding::Rules;
use crate::bidding::constraint::{hcp, len, points};
use crate::bidding::rows::{Entry, Package, Pattern, rows_of};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// The `2♠` rule itself: above the `1♠` response (170) and every 2/1,
/// disjoint from the raises by heart length
pub(super) fn with_strong_jump_shift(rules: Rules, floor: u8) -> Rules {
    rules.rule(
        Bid::new(2, Strain::Spades),
        180,
        len(Suit::Spades, 5..) & len(Suit::Hearts, ..=2) & hcp(floor..),
    )
}

/// Opener over `1♥ - 2♠`: support, six hearts, a second suit, else `2NT`
fn opener() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Spades), 150, len(Suit::Spades, 3..))
        .rule(Bid::new(3, Strain::Hearts), 140, len(Suit::Hearts, 6..))
        .rule(
            Bid::new(3, Strain::Diamonds),
            130,
            len(Suit::Diamonds, 5..) | (len(Suit::Diamonds, 4..) & len(Suit::Clubs, ..=3)),
        )
        .rule(Bid::new(3, Strain::Clubs), 120, len(Suit::Clubs, 4..))
        .rule(Bid::new(2, Strain::Notrump), 0, hcp(0..))
}

/// Responder over the spade raise
fn over_raise() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, hcp(18..))
        .alert(slam::RKCB)
        .rule(Bid::new(4, Strain::Spades), 0, hcp(0..))
}

/// Responder over opener's six-card `3♥`
fn over_hearts() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Notrump),
            160,
            len(Suit::Hearts, 2..) & hcp(18..),
        )
        .alert(slam::RKCB)
        .rule(Bid::new(4, Strain::Hearts), 150, len(Suit::Hearts, 2..))
        .rule(Bid::new(3, Strain::Spades), 130, len(Suit::Spades, 6..))
        .rule(Bid::new(3, Strain::Notrump), 0, hcp(0..))
}

/// Responder over opener's `3m`
fn over_minor(minor: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(minor)), 140, len(minor, 4..))
        .rule(Bid::new(3, Strain::Spades), 130, len(Suit::Spades, 6..))
        .rule(Bid::new(3, Strain::Notrump), 0, hcp(0..))
}

/// Responder over opener's `2NT`
fn over_notrump() -> Rules {
    Rules::new()
        .rule(Bid::new(6, Strain::Notrump), 160, hcp(21..))
        .rule(
            Bid::new(4, Strain::Notrump),
            150,
            hcp(19..=20) & len(Suit::Spades, ..=5),
        )
        .rule(Bid::new(4, Strain::Spades), 140, len(Suit::Spades, 6..))
        .rule(Bid::new(3, Strain::Notrump), 0, hcp(0..))
}

/// Opener over responder's six-card `3♠`
fn opener_over_spades() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Spades), 100, len(Suit::Spades, 2..))
        .rule(Bid::new(4, Strain::Hearts), 90, len(Suit::Hearts, 7..))
        .rule(Bid::new(3, Strain::Notrump), 0, hcp(0..))
}

/// Opener over the minor raise: keycards on extras, else game
fn opener_over_minor_raise(minor: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, points(15..))
        .alert(slam::RKCB)
        .rule(Bid::new(5, Strain::from(minor)), 0, hcp(0..))
}

/// Opener over the quantitative `4NT`
fn opener_over_quantitative() -> Rules {
    Rules::new()
        .rule(Bid::new(6, Strain::Notrump), 100, hcp(14..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener over a placed `4M`: keycards on 15+ points, else pass
fn ask_or_pass() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, points(15..))
        .alert(slam::RKCB)
        .rule(Call::Pass, 0, hcp(0..))
}

/// A table at `node` whose `4NT` is RKCB in `trump`, with both seats' trees
fn asking(entries: &mut Vec<Entry>, node: &str, rules: Rules, trump: Suit) {
    entries.extend(rows_of(Pattern::node(node), rules));
    entries.extend(slam::rkcb_rows(node, trump));
    entries.extend(slam::rkcb_answerer_rows(node, trump));
}

/// Both rounds below the jump shift, in the unpassed seats only
pub(crate) fn strong_jump_shift_continuations() -> Package {
    Package {
        name: "heart-spade-strong-jump-shift",
        gate: |a| a.response.heart_spade_strong_jump_shift.is_some(),
        entries: |_| {
            let mut entries = Vec::new();
            // Responder is unpassed: opener deals or sits second.
            for (lead, tail) in [("", "-"), ("", "(X)"), ("- ", "-"), ("- ", "(X)")] {
                let node = format!("{lead}1♥ - 2♠ {tail}");
                entries.extend(rows_of(Pattern::node(&node), opener()));

                let fit = format!("{node} 3♠ -");
                asking(&mut entries, &fit, over_raise(), Suit::Spades);
                asking(
                    &mut entries,
                    &format!("{fit} 4♠ -"),
                    ask_or_pass(),
                    Suit::Spades,
                );

                let hearts = format!("{node} 3♥ -");
                asking(&mut entries, &hearts, over_hearts(), Suit::Hearts);
                asking(
                    &mut entries,
                    &format!("{hearts} 4♥ -"),
                    ask_or_pass(),
                    Suit::Hearts,
                );
                entries.extend(rows_of(
                    Pattern::node(&format!("{hearts} 3♠ -")),
                    opener_over_spades(),
                ));

                for minor in [Suit::Clubs, Suit::Diamonds] {
                    let m = Strain::from(minor);
                    let second = format!("{node} 3{m} -");
                    entries.extend(rows_of(Pattern::node(&second), over_minor(minor)));
                    asking(
                        &mut entries,
                        &format!("{second} 4{m} -"),
                        opener_over_minor_raise(minor),
                        minor,
                    );
                    entries.extend(rows_of(
                        Pattern::node(&format!("{second} 3♠ -")),
                        opener_over_spades(),
                    ));
                }

                let notrump = format!("{node} 2NT -");
                entries.extend(rows_of(Pattern::node(&notrump), over_notrump()));
                entries.extend(rows_of(
                    Pattern::node(&format!("{notrump} 4NT -")),
                    opener_over_quantitative(),
                ));
                asking(
                    &mut entries,
                    &format!("{notrump} 4♠ -"),
                    ask_or_pass(),
                    Suit::Spades,
                );
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
