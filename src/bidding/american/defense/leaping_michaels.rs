//! Leaping Michaels — the `4♣`/`4♦` jumps over their preempt
//!
//! A jump to four of a minor names a 5-5 game-forcing two-suiter: the minor
//! plus the unbid major.  Gated by [`set_leaping_michaels`].
use super::*;

thread_local! {
    /// Whether Leaping Michaels (4♣/4♦ strong two-suiters over their weak two)
    /// is active; see [`set_leaping_michaels`].
    static LEAPING_MICHAELS: Cell<bool> = const { Cell::new(true) };
}

/// Toggle Leaping Michaels for books built *after* this call (thread-local, read
/// once at book-construction time)
///
/// Over their weak two, a jump to `4♣`/`4♦` names a 5-5 two-suiter with
/// game-forcing values: over a major it is a minor plus the *other* major; over
/// `2♦` the `4♦` cue shows both majors and `4♣` shows clubs plus a major.  **On by
/// default** — the authored advances make it a clear DD win (+1.090/+1.452
/// IMPs/board, none/both), and the inference reader lets the live-search bidder
/// price the advance (and reach slam) on top; see `docs/ai-bidder/21gf-ledger.md`.
/// Turn it off to recover the pre-Leaping-Michaels weak-two defense.
pub fn set_leaping_michaels(on: bool) {
    LEAPING_MICHAELS.with(|cell| cell.set(on));
}

/// Whether Leaping Michaels is currently enabled
///
/// Crate-visible so the inference projection pass can condition partner's hand on
/// the two-suiter when the search bidder samples (see `inference::authored_reading`).
pub fn leaping_michaels_enabled() -> bool {
    LEAPING_MICHAELS.with(Cell::get)
}

/// Advancer's response to partner's Leaping Michaels jump over their weak two
///
/// `theirs` is the suit they opened; `lm` is the suit of the jump (Clubs or
/// Diamonds).  The overcall is game-forcing, so every advance reaches game.
/// - Over a **major**, the jump names `lm` plus the *other* major: bid that
///   major game with a fit, else the `lm` minor game.
/// - Over **2♦**, the `4♦` *cue* shows both majors → pick the longer; the `4♣`
///   jump shows clubs + an unknown major → `5♣` with a club fit and no major,
///   else `4♥` pass-or-correct (see [`leaping_michaels_2d_4c_rebid`]).
fn leaping_michaels_advances(theirs: Suit, lm: Suit) -> Rules {
    match theirs {
        // Over a major: lm + the OTHER major, both known.
        Suit::Hearts | Suit::Spades => {
            let major = if theirs == Suit::Hearts {
                Suit::Spades
            } else {
                Suit::Hearts
            };
            // Prefer the major game even on a doubleton (a 7-card fit) — it
            // scores well and needs only ten tricks; retreat to the 5m game only
            // on a genuine major misfit (≤1), where DD has to make eleven.
            Rules::new()
                .rule(Bid::new(4, Strain::from(major)), 130, len(major, 2..))
                .rule(Bid::new(5, Strain::from(lm)), 120, len(major, 0..=1))
        }
        // Over 2♦.
        Suit::Diamonds => match lm {
            // 4♦ cue = both majors: pick the longer (both forced to game).
            Suit::Diamonds => {
                let hearts_longer = at_least_as_long(Suit::Hearts, Suit::Spades);
                let spades_longer = longer_suit(Suit::Spades, Suit::Hearts);
                Rules::new()
                    .rule(Bid::new(4, Strain::Hearts), 130, hearts_longer)
                    .rule(Bid::new(4, Strain::Spades), 130, spades_longer)
            }
            // 4♣ = clubs + a major: 5♣ with a club fit and no major, else 4♥
            // pass-or-correct (partner names their major).
            Suit::Clubs => Rules::new()
                .rule(
                    Bid::new(5, Strain::Clubs),
                    120,
                    len(Suit::Clubs, 3..) & len(Suit::Hearts, 0..=2) & len(Suit::Spades, 0..=2),
                )
                .rule(Bid::new(4, Strain::Hearts), 130, hcp(0..)),
            _ => unreachable!("a Leaping Michaels jump is clubs or diamonds"),
        },
        Suit::Clubs => unreachable!("there is no weak 2♣ opening"),
    }
}

/// Overcaller's rebid after `(2♦) 4♣ - 4♥ -`: pass-or-correct to their major
///
/// `4♣` over `2♦` showed clubs + a major; advancer's `4♥` is pass-or-correct, so
/// the overcaller passes with hearts or corrects to `4♠` with spades.
fn leaping_michaels_2d_4c_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Spades), 130, len(Suit::Spades, 5..))
        .rule(Call::Pass, 100, hcp(0..))
}

/// Advances of Leaping Michaels over their weak two
///
/// The jump is below game, so the advancer is forced on (a fit major game, else
/// the `5m` minor game — never a passed `4m` partscore).
pub(super) fn leaping_michaels_package() -> Package {
    Package {
        name: "leaping-michaels-advance",
        gate: |agreements| agreements.defense.leaping_michaels_enabled,
        entries: |_| {
            let mut entries = Vec::new();
            for suit in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let opening = Bid::new(2, Strain::from(suit));
                for lm in [Suit::Clubs, Suit::Diamonds] {
                    let jump = Bid::new(4, Strain::from(lm));
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* ({opening}) {jump} -")),
                        leaping_michaels_advances(suit, lm),
                    ));
                }
                // Over 2♦, 4♣ shows clubs + an unknown major; advancer's 4♥ is
                // pass-or-correct, so the overcaller names their major in rebid.
                if suit == Suit::Diamonds {
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* ({opening}) 4♣ - 4♥ -")),
                        leaping_michaels_2d_4c_rebid(),
                    ));
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
