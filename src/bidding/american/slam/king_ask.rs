use super::*;

/// King answers at the 5NT node (for all answer paths — shared table)
///
/// 5NT promises all five keycards; this asks for kings outside trumps.
///
/// For spades: 6♣ (0), 6♦ (1), 6♥ (2), 6♠ signoff (3 kings).
/// For hearts: 6♣ (0), 6♦ (1), 6♥ catch-all signoff (2+).
pub(super) fn king_answers(trump: Suit) -> Rules {
    let mut rules = Rules::new()
        .rule(Bid::new(6, Strain::Clubs), 100, kings_outside(trump, 0..=0))
        .alert(RKCB)
        .rule(
            Bid::new(6, Strain::Diamonds),
            100,
            kings_outside(trump, 1..=1),
        )
        .alert(RKCB);

    match trump {
        Suit::Spades => {
            rules = rules
                .rule(
                    Bid::new(6, Strain::Hearts),
                    100,
                    kings_outside(trump, 2..=2),
                )
                .alert(RKCB)
                // 3 outside kings → 6♠ signoff (counting stops below 7)
                .rule(Bid::new(6, Strain::Spades), 50, hcp(0..));
        }
        Suit::Hearts => {
            // 6♥ is a catch-all signoff for 2+ outside kings
            rules = rules.rule(Bid::new(6, Strain::Hearts), 50, hcp(0..));
        }
        _ => unreachable!("the 5NT king ask is major-only; minors never install it"),
    }
    rules
}

/// Asker's call after a king answer showing `shown` outside kings: seven when
/// the partnership holds `needed` of the three
///
/// `needed` is three in the classic ladder and two on the grand rung (see
/// [`grand_rkcb_rows`]).  6♥ is a king answer only when trumps are spades.
pub(super) fn asker_after_kings(trump: Suit, shown: usize, needed: usize) -> Rules {
    let t = Strain::from(trump);
    Rules::new()
        .rule(
            Bid::new(7, t),
            100,
            kings_outside(trump, needed.saturating_sub(shown)..),
        )
        .rule(Bid::new(6, t), 50, hcp(0..))
}

/// Answers to the grand rung's 5NT: seven with the kings that make two of the
/// three, else the classic count (majors) or six of the trump (minors)
///
/// The asker bids seven itself on two side kings, so a major 5NT shows at most
/// one and two of ours settle it; a minor 5NT is exactly one king short — there
/// is no room to count below six of the trump — and one of ours settles it.
pub(super) fn grand_king_answers(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    if matches!(trump, Suit::Hearts | Suit::Spades) {
        king_answers(trump).rule(Bid::new(7, t), 120, kings_outside(trump, 2..))
    } else {
        Rules::new()
            .rule(Bid::new(7, t), 120, kings_outside(trump, 1..))
            .rule(Bid::new(6, t), 50, hcp(0..))
    }
}
