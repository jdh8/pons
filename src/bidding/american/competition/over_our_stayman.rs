//! Competition over our 2♣ Stayman
//!
//! Opener's replies after the opponents double or overcall our 2♣ Stayman are
//! authored under [`set_competition_over_stayman`].

use super::*;

thread_local! {
    /// Whether opener authors continuations after the opponents contest our 2♣
    /// Stayman (`1NT - 2♣ (X)` or a `2♦`/`2♥`/`2♠` overcall); **on by default**, with an
    /// off-switch for A/B measurement.  See [`set_competition_over_stayman`].
    static COMPETITION_OVER_STAYMAN: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's replies after the opponents double or overcall our 2♣ Stayman,
/// for books built *after* this call (thread-local; **on by default**).
///
/// Over a `(X)` (lead-directing clubs) opener answers in the *pass-denies-stopper*
/// coded scheme: a major or `2♦` promises a club stopper, Pass denies one, `XX` is
/// business clubs; responder's `XX` after opener's pass re-asks Stayman (forcing).
/// Over a `(2♦/2♥/2♠)` overcall opener bids a 4-card major naturally if it
/// outranks their suit, doubles for cards, else passes.
pub fn set_competition_over_stayman(on: bool) {
    COMPETITION_OVER_STAYMAN.with(|cell| cell.set(on));
}

/// Whether competition over our 2♣ Stayman is currently authored
pub fn competition_over_stayman() -> bool {
    COMPETITION_OVER_STAYMAN.with(Cell::get)
}

/// Opener's coded reply after the opponents double our 2♣ Stayman
/// (`1NT - 2♣ (X)`)
///
/// The `(X)` is lead-directing clubs, so the *pass-denies-stopper* scheme spends
/// the free pass on a club-stopper signal: a 4-card major (`2♥`/`2♠`) or `2♦`
/// (no major) promises a club stopper; **Pass denies one** (it may still hide a
/// major, shown after responder re-asks); `XX` is business clubs (offer to play
/// 2♣ doubled-redoubled).  Direct XX is business — distinct from responder's
/// SOS/re-ask XX below.
fn stayman_doubled_opener() -> Rules {
    Rules::new()
        .rule(
            Call::Redouble,
            100,
            len(Suit::Clubs, 5..) & suit_hcp(Suit::Clubs, 5..),
        )
        .rule(
            Bid::new(2, Strain::Hearts),
            100,
            len(Suit::Hearts, 4..) & stopper_in(Suit::Clubs),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            100,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4) & stopper_in(Suit::Clubs),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            50,
            len(Suit::Hearts, ..4) & len(Suit::Spades, ..4) & stopper_in(Suit::Clubs),
        )
        .rule(Call::Pass, 25, !stopper_in(Suit::Clubs))
}

/// Responder's re-ask after opener passed our doubled Stayman to deny a club
/// stopper (`1NT - 2♣ (X) - -`)
///
/// Balancing XX is SOS, not business: `XX` re-asks Stayman (forcing — responder
/// still holds the 4-card major), and opener must answer (`stayman_answers`, no
/// Pass).  An owning Pass is the always-mass catch-all.
fn stayman_redouble_reask() -> Rules {
    Rules::new()
        .rule(
            Call::Redouble,
            100,
            len(Suit::Hearts, 4..) | len(Suit::Spades, 4..),
        )
        .alert(STAYMAN_REDOUBLE)
        .rule(Call::Pass, 10, hcp(0..))
}

/// Opener's natural reply after the opponents overcall our 2♣ Stayman at the
/// 2-level (`1NT - 2♣ (2♦/2♥/2♠)`)
///
/// Show the 4-card major if it outranks their suit; else `X` shows length in
/// their suit (cards/penalty — and, when they overcalled the very major opener
/// holds, the major opener could not bid); else Pass.  Responder stays captain.
fn stayman_overcalled_opener(over: Suit) -> Rules {
    let mut rules = Rules::new();
    if (Suit::Hearts as u8) > (over as u8) {
        rules = rules.rule(Bid::new(2, Strain::Hearts), 100, len(Suit::Hearts, 4..));
    }
    if (Suit::Spades as u8) > (over as u8) {
        rules = rules.rule(
            Bid::new(2, Strain::Spades),
            100,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        );
    }
    rules
        .rule(Call::Double, 60, len(over, 4..))
        .rule(Call::Pass, 20, hcp(0..))
}

/// Competition over our own `2♣` Stayman as a row package
/// ([`set_competition_over_stayman`], default on)
///
/// Opener's replies after they double `1NT - 2♣ (X)` or overcall it.  Keyed
/// at the `1NT - 2♣` node — a distinct trie path from the systems-on
/// `1NT (2♣)` block, where their `2♣` sits at depth 1.
pub(super) fn competition_over_stayman_package() -> Package {
    Package {
        name: "competition-over-stayman",
        gate: |agreements| agreements.build.competition.competition_over_stayman,
        entries: |_| {
            const STAYMAN: &str = "P* 1NT - 2♣";
            // A.1 — our Stayman doubled.  Opener's coded reply, then the
            // systems-on rebase off his stopper-bid.
            let mut entries = rows_of(Pattern::after(STAYMAN, "(X)"), stayman_doubled_opener());
            entries.push(systems_on_over_double(STAYMAN, "2♦"));
            // Opener passed to deny a stopper; responder re-asks, opener must
            // answer — `stayman_answers()` has no Pass rule, and its 2♦ is
            // exactly the artificial "no major" denial.
            entries.extend(rows_of(
                Pattern::after(STAYMAN, "(X) - -"),
                stayman_redouble_reask(),
            ));
            entries.extend(rows_of(
                Pattern::after(STAYMAN, "(X) - - XX -"),
                stayman_answers(),
            ));

            // A.1c — opener's 2-level answer (2♦/2♥/2♠) doubled.  The double
            // steals no room (responder's escapes all sit above 2♦), so
            // responder is systems-on: this is the escape the invitational-5-4
            // reroute needs — a 5♠4♥ that Staymaned bids its 2♠ instead of
            // sitting for a doubled 2♦ — and it also lets a 4-4 hand run to
            // 2NT rather than passing the double out.
            entries.push(rebase(
                Pattern::guarded(
                    STAYMAN,
                    "- 2♦ (X)",
                    described_guard(
                        "- 2♦/2♥/2♠ X …",
                        guard(|_: &Context<'_>, s: &[Call]| {
                            s.first() == Some(&Call::Pass)
                                && matches!(
                                    s.get(1),
                                    Some(Call::Bid(b))
                                        if b.level.get() == 2
                                            && matches!(
                                                b.strain,
                                                Strain::Diamonds | Strain::Hearts | Strain::Spades
                                            )
                                )
                                && s.get(2) == Some(&Call::Double)
                        }),
                    ),
                ),
                described_rewrite(
                    "systems on: their X is stripped to a pass",
                    rewriter(|auction: &[Call], depth: usize| {
                        if auction.get(depth + 2) != Some(&Call::Double) {
                            return None;
                        }
                        let mut rewritten = auction.to_vec();
                        rewritten[depth + 2] = Call::Pass; // strip the X → systems on
                        Some(rewritten)
                    }),
                ),
            ));

            // A.2 — our Stayman overcalled at the 2-level.  Opener's natural reply.
            for over in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                entries.extend(rows_of(
                    Pattern::after(STAYMAN, &format!("(2{})", Strain::from(over))),
                    stayman_overcalled_opener(over),
                ));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
