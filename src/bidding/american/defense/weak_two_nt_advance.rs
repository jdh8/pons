//! Advancing our `2NT` overcall of their weak two
//!
//! Advancer's Gladiator structure over our `2NT` overcall of their weak two in
//! a **major** is gated by
//! `agreements.defense.weak_two_notrump_advances_enabled`.  Its game threshold
//! tracks `agreements.defense.weak_two_notrump_points`'s floor, so a widened
//! band raises it instead of silently keeping a tierless structure calibrated for 16.

use super::*;

/// Advancer's Gladiator structure over our `2NT` overcall of their weak two in
/// a **major** (`agreements.defense.weak_two_notrump_advances_enabled`)
///
/// The 1NT-level Gladiator ([`gladiator_advances`]) needs an invitational tier
/// and spends the two level on it.  Here the overcall is narrow — 16–17 by
/// default — so eight points opposite is already game values and the tier
/// vanishes: `3♣` is the weak relay, everything above it is game-forcing, and
/// the cue is Stayman.  The threshold tracks
/// `agreements.defense.weak_two_notrump_points`'s floor, so a widened band raises it
/// instead of silently keeping a tierless structure calibrated for 16.
///
/// Every artificial call states its true meaning in its own rule text, so the
/// `.alert(...)` is the whole reading — `project_authored` decodes the box and
/// suppresses the phantom natural suit at the same index.  No bespoke
/// `Inferences` arm is needed (contrast `gladiator_reading`, whose relay has no
/// sound per-suit floor to project).
fn weak_two_notrump_advances(their_major: Suit, agreements: &Agreements) -> Rules {
    let o = other_major(their_major);
    let m = Strain::from(their_major);
    let os = Strain::from(o);
    // Game values opposite the band's *minimum*: at the default 16–17 that is
    // eight (16 + 8 = 24).  Keyed to the band rather than frozen at 8, so
    // widening it with `agreements.defense.weak_two_notrump_points` cannot leave advancer
    // driving to game on a 23-count — the bias would fall on exactly the hands
    // the widening adds.
    let game = 24u8.saturating_sub(agreements.defense.weak_two_notrump_points.0);

    Rules::new()
        // Cue = Stayman for the *one* unbid major.  A flat (4333) is barred —
        // with no ruffing value a 4-4 fit does not beat 3NT (the 4333 curse).
        .rule(
            Bid::new(3, m),
            140,
            len(o, 4..=4) & points(game..) & !flat_4333(),
        )
        .alert(WEAK_TWO_NT_STAYMAN)
        // Game-forcing naturals: a real five-plus suit.
        .rule(
            Bid::new(3, Strain::Diamonds),
            130,
            len(Suit::Diamonds, 5..) & points(game..),
        )
        .rule(Bid::new(3, os), 130, len(o, 5..) & points(game..))
        // Balanced game, to play — the overcaller holds the stopper, so the
        // notrump is right-sided as it stands.
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            balanced() & points(game..),
        )
        // `3♣` = relay: a weak diamond hand looking for a 3-level partscore.
        // The forced `3♦` is the landing spot; a weak hand with the *other*
        // major has none (its correction would be `3♠`, unauthored) and passes.
        .rule(
            Bid::new(3, Strain::Clubs),
            50,
            points(..game) & len(Suit::Diamonds, 5..),
        )
        .alert(WEAK_TWO_NT_RELAY)
        .rule(Call::Pass, 30, hcp(0..))
}

/// Overcaller's forced `3♦` completion of the `3♣` relay
///
/// Pass-or-correct and utterly blind — it says nothing about diamonds, which is
/// why it is alerted: the alert is what stops the walk floring a phantom suit.
fn weak_two_notrump_relay_reply() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 100, hcp(0..))
        .alert(WEAK_TWO_NT_RELAY_PC)
}

/// Advancer's rebid over the forced `3♦`: pass to play it, or cue their major
/// to say the diamonds are long enough that `4♦` is safe
///
/// Only authored over their `2♥`, where the cue is `3♥`.  Over `2♠` the cue is
/// `3♠` itself, which is left unauthored for now — so the whole node is.
fn weak_two_notrump_relay_rebid(their_major: Suit) -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::from(their_major)),
            100,
            len(Suit::Diamonds, 6..),
        )
        .alert(WEAK_TWO_NT_DIAMONDS)
        .rule(Call::Pass, 50, hcp(0..))
}

/// Advancing our `2NT` overcall of their weak two
/// (`agreements.defense.weak_two_notrump_advances_enabled`)
///
/// Majors only — over `2♦` both majors are unbid, so the cue has no Stayman to
/// be.
pub(super) fn weak_two_notrump_advance_package() -> Package {
    Package {
        name: "weak-two-notrump-advance",
        gate: |agreements| agreements.defense.weak_two_notrump_advances_enabled,
        entries: |agreements| {
            let mut entries = Vec::new();
            for suit in [Suit::Hearts, Suit::Spades] {
                let opening = Bid::new(2, Strain::from(suit));
                let base = format!("P* ({opening}) 2NT -");
                entries.extend(rows_of(
                    Pattern::node(&base),
                    weak_two_notrump_advances(suit, agreements),
                ));
                entries.extend(rows_of(
                    Pattern::node(&format!("{base} 3♣ -")),
                    weak_two_notrump_relay_reply(),
                ));
                // The delayed cue is 3♥ over their 2♥ but 3♠ over their 2♠, and
                // 3♠+ is unauthored — so over 2♠ the node would be Pass alone.
                if suit == Suit::Hearts {
                    entries.extend(rows_of(
                        Pattern::node(&format!("{base} 3♣ - 3♦ -")),
                        weak_two_notrump_relay_rebid(suit),
                    ));
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
