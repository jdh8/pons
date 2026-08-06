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

/// Asker's call after a 6♣ king answer (0 outside kings)
pub(super) fn asker_after_6c(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    Rules::new()
        .rule(Bid::new(7, t), 100, kings_outside(trump, 3..))
        .rule(Bid::new(6, t), 50, hcp(0..))
}

/// Asker's call after a 6♦ king answer (1 outside king)
pub(super) fn asker_after_6d(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    Rules::new()
        .rule(Bid::new(7, t), 100, kings_outside(trump, 2..))
        .rule(Bid::new(6, t), 50, hcp(0..))
}

/// Asker's call after a 6♥ king answer (2 outside kings; only when trump == Spades)
pub(super) fn asker_after_6h(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    Rules::new()
        .rule(Bid::new(7, t), 100, kings_outside(trump, 1..))
        .rule(Bid::new(6, t), 50, hcp(0..))
}
