//! Sohl after a takeout double — advancing partner's double of a weak two
//!
//! Their weak two steals the room a normal advance needs, so the `2NT` relay
//! comes back: [`set_advance_sohl_style`] picks plain or transfer Sohl, and
//! [`sohl_rows_over`] emits the rows for whichever style is armed.  Shared by
//! [`super::advance_double`] and [`super::gladiator`].

use super::*;

thread_local! {
    /// Which sohl package the advancer carries after partner's takeout double of
    /// a weak two (`(2X)–X–(P)`); see [`set_advance_sohl_style`].
    static ADVANCE_SOHL: Cell<LebensohlStyle> = const { Cell::new(LebensohlStyle::Transfer) };
}

/// Select the sohl package the **advancer** carries after partner's takeout
/// double of a weak two, for books built *after* this call (thread-local, read
/// once at book-construction time)
///
/// Reuses [`LebensohlStyle`]: `Off` keeps the flat [`advance_double`] ladder;
/// `Plain` adds the weak `2NT` relay vs a forcing 3-level suit; `Transfer` (the
/// **default**) adds Larry Cohen's transfers-through + cue-Stayman, plus, over
/// `(2♦)`, `3♣`-Stayman + Smolen + Leaping Michaels. The geometry matches Lebensohl
/// after our overcalled `1NT` (the opponents' suit is at the two level in both),
/// so the Section-5 builders are reused verbatim under the `(2X)–X–(P)` prefix.
/// `Transfer` is the default because it is a clear perfect-defense win over the
/// flat ladder (+0.145/+0.227 IMPs/board none/both, 200k filtered).
/// See `docs/ai-bidder/21gf-ledger.md` for the full A/B numbers.
pub fn set_advance_sohl_style(style: LebensohlStyle) {
    ADVANCE_SOHL.with(|cell| cell.set(style));
}

/// The currently selected advance-of-double sohl package
pub(super) fn advance_sohl_style() -> LebensohlStyle {
    ADVANCE_SOHL.with(Cell::get)
}

/// A Section-5 sohl structure for our side's advancer over a single
/// interfering suit `over`, hung off the auction-string `base` (a three-call
/// prefix ending at the advancer's first turn) — the advancer's responses, the
/// relay completion, and (for `Transfer`) the transfer / cue-Stayman answers
/// plus the `(2♦)` Smolen + Leaping-Michaels package.  Shared by
/// [`advance_of_double_package`] (`P* (2X) X (P)`) and
/// [`gladiator_sohl_package`] (`P* (1M) 1NT (2Y)`).
/// `gate_4333` gates the flat-4333 Stayman/cue carve; callers pass `false` when
/// partner is known short in `over`, `true` when partner is balanced (a 1NT).
pub(super) fn sohl_rows_over(
    base: &str,
    over: Suit,
    style: LebensohlStyle,
    gate_4333: bool,
) -> Vec<Entry> {
    let mut entries = Vec::new();

    // Advancer's first action shadows the floor (the builders end in a 0.0 Pass,
    // which covers the weak and penalty-pass hands).
    let advancer = match style {
        LebensohlStyle::Transfer if over == Suit::Diamonds => {
            transfer_stayman_2d_responder(gate_4333)
        }
        LebensohlStyle::Transfer => transfer_lebensohl_responder(over, gate_4333),
        _ => lebensohl_responder(over),
    };
    entries.extend(rows_of(Pattern::node(base), advancer));

    // Partner completes the 2NT relay with a forced 3♣; advancer then signs off.
    let relay = format!("{base} 2NT (P)");
    entries.extend(rows_of(Pattern::node(&relay), complete_lebensohl_relay()));
    entries.extend(rows_of(
        Pattern::node(&format!("{relay} 3♣ (P)")),
        lebensohl_relay_rebid(over),
    ));

    // Transfer style: partner answers each 3-level transfer / cue. Over (2♦) the
    // Smolen block below owns the 3-level replies, so this covers (2♥)/(2♠).
    if style == LebensohlStyle::Transfer && over != Suit::Diamonds {
        // Over (2♥)/(2♠) the delayed cue (2NT relay, then their suit) is always
        // *recognized* — answered as Stayman with a stopper — even when the bot
        // never bids it itself, so a human partner who plays it gets a sensible
        // reply. `split` (the default-off `set_delayed_cue` toggle) additionally
        // makes the bot *bid* the convention and read the direct cue as denying a
        // stopper (so it is answered without a free 3NT).
        let recognize = matches!(over, Suit::Hearts | Suit::Spades);
        let split = delayed_cue() && recognize;
        for bid_suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let resp = call(3, Strain::from(bid_suit));
            let reply = if bid_suit == over {
                if split {
                    cue_stayman_answer_no_stopper(over)
                } else {
                    cue_stayman_answer(over)
                }
            } else if let Some(target) = transfer_target(bid_suit, over) {
                transfer_completion(target, over)
            } else {
                continue; // the lowest suit has no transfer target — floored
            };
            entries.extend(rows_of(Pattern::node(&format!("{base} {resp} (P)")), reply));
        }
        // Delayed cue: base–2NT–P–3♣–P–3X (their suit) — Stayman with a stopper,
        // answered exactly like the direct cue but with 3NT safe. Wired whenever
        // it could be bid (recognition), independent of whether the bot bids it.
        if recognize {
            let cue = call(3, Strain::from(over));
            entries.extend(rows_of(
                Pattern::node(&format!("{relay} 3♣ (P) {cue} (P)")),
                cue_stayman_answer(over),
            ));
        }
    }

    // Transfer over (2♦): 3♣-Stayman + Smolen, the Jacoby transfers
    // (3♦→♥, 3♥→♠, 3♠→♣), and Leaping Michaels 4♣/4♦ — the diamond-only package
    // ported from the 1NT-(2♦) context. (2♥/2♠ reuse the Transfer completions above.)
    if style == LebensohlStyle::Transfer && over == Suit::Diamonds {
        let nodes: Vec<(&str, Rules)> = vec![
            // 3♣ Stayman, partner's answer; then Smolen after the 3♦ denial.
            ("3♣ (P)", stayman_2d_answer()),
            ("3♣ (P) 3♦ (P)", smolen_at_three()),
            ("3♣ (P) 3♦ (P) 3♥ (P)", smolen_completion(Suit::Spades)),
            ("3♣ (P) 3♦ (P) 3♠ (P)", smolen_completion(Suit::Hearts)),
            // Partner showed a 4-card major over Stayman; advancer places.
            ("3♣ (P) 3♥ (P)", stayman_2d_fit_rebid(Suit::Hearts)),
            ("3♣ (P) 3♠ (P)", stayman_2d_fit_rebid(Suit::Spades)),
            // Jacoby transfers: 3♦→♥, 3♥→♠ (auto-driven), 3♠→♣ (forced GF).
            ("3♦ (P)", transfer_completion(Suit::Hearts, over)),
            ("3♥ (P)", transfer_completion(Suit::Spades, over)),
            ("3♠ (P)", clubs_transfer_completion(over)),
            // Leaping Michaels: 4♦ both majors, 4♣ clubs + a major (ask).
            ("4♦ (P)", lm_2d_both_majors_advance()),
            ("4♣ (P)", lm_2d_clubs_ask()),
            ("4♣ (P) 4♦ (P)", lm_2d_clubs_major()),
        ];
        for (rest, rules) in nodes {
            entries.extend(rows_of(Pattern::node(&format!("{base} {rest}")), rules));
        }
    }
    entries
}

#[cfg(test)]
mod tests;
