use super::*;

// ---------------------------------------------------------------------------
// Minor-trump asker continuations (plain 4NT; cramped signoff; no king ask)
// ---------------------------------------------------------------------------
//
// The keycard counts mirror the major tables; only the signoff differs, because
// the answers overshoot 5-of-a-minor.  Every table keeps a legal finite call for
// every hand (a `6m` or `Pass` catch-all): a node whose only finite logits are
// *illegal* calls would not fall through to the floor — `Table::next_call` would
// filter them and silently pass, stranding a bad contract.

/// Asker after a 5♣ answer when trumps are a minor
///
/// 3+ keycards (≥5 total) drive to 6-of-the-minor.  To stop: diamonds can sign
/// off in 5♦ (legal over 5♣); clubs must Pass to play partner's 5♣.
pub(super) fn asker_after_5c_minor(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    let rules = Rules::new().rule(Bid::new(6, t), 100, keycards(trump, 3..));
    if trump == Suit::Diamonds {
        rules.rule(Bid::new(5, t), 50, hcp(0..))
    } else {
        rules.rule(Call::Pass, 50, hcp(0..))
    }
}

/// Asker after a 5♦ answer when trumps are a minor
///
/// Diamonds: slam set mirrors the major `asker_after_5d` (≤2 assume partner 3,
/// or 4+); to stop, Pass to play partner's 5♦.  Clubs: no room below 6♣.
pub(super) fn asker_after_5d_minor(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    if trump == Suit::Diamonds {
        Rules::new()
            .rule(
                Bid::new(6, t),
                100,
                keycards(trump, 2..=2) | keycards(trump, 4..),
            )
            .rule(Call::Pass, 50, hcp(0..))
    } else {
        no_room_six(trump)
    }
}

/// Asker with no room to stop below slam: bid 6-of-the-minor
///
/// Used for the 5♥/5♠ answers (both minors) and the clubs 5♦ answer — all sit
/// above 5-of-either-minor, so signing off below slam is impossible.
pub(super) fn no_room_six(trump: Suit) -> Rules {
    Rules::new().rule(Bid::new(6, Strain::from(trump)), 100, hcp(0..))
}
