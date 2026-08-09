//! Their two-suiter over our `1M` — Michaels, the unusual `2NT`, and Cachalot
//!
//! When they name two suits over our major opening, responder's raises and
//! fourth-suit answers all change meaning.  Gated by
//! [`set_uvu_over_majors`].

use super::cue_raise::answer_cue_raise;
use super::lebensohl::unbid_major;
use super::*;

thread_local! {
    /// Whether responder's structure over the opponents' two-suiters over our
    /// 1♥/1♠ opening — their both-minors `(2NT)` and their Michaels cue of our
    /// own major — is authored. **Default on** — measured vs BBA 2/1 (204.8k
    /// boards/arm/vul): plain DD +0.0019/+0.0018 IMPs/board NV/vul (both CIs
    /// exclude 0; +1.43/+1.58 IMPs/fired, ~0.12% fired), perfect-defense the
    /// same sign.
    ///
    /// Book construction only.  This knob once *also* gated the inference
    /// walk's hand-written two-suiter reading; that reader was retired in
    /// favour of the authored rules' own projection (chop 1 of
    /// `docs/reader-retirement.md`), so the reading is now owned by
    /// [`set_table_alert_reading`][crate::bidding::set_table_alert_reading].
    static UVU_OVER_MAJORS: Cell<bool> = const { Cell::new(true) };
}

/// Author responder's structure over their two-suiters over our 1M for books
/// built *after* this call (thread-local)
///
/// Read at book construction. Reading *their* cue / `(2NT)` as a two-suiter is
/// no longer this knob's business — the alerted rules project themselves (see
/// the thread-local's doc).
/// **Default on** (`--no-ns-uvu-over-majors` in `bba-gen` for the off arm).
pub fn set_uvu_over_majors(on: bool) {
    UVU_OVER_MAJORS.with(|cell| cell.set(on));
}

/// Whether the two-suiters-over-our-1M package is authored (book construction)
pub fn uvu_over_majors() -> bool {
    UVU_OVER_MAJORS.with(Cell::get)
}

/// Responder after our 1M and their both-minors `(2NT)` — unusual vs unusual
///
/// The two cues split by strength and direction: `3♣` (their lower suit) is
/// the limit-plus raise of our major, `3♦` a game force with 5+ in the other
/// major. `3NT` is to play with both minors stopped; `X` shows values and a
/// minor we can punish (the shape [`uvu_responder`] measured over our
/// overcalled 1NT); the direct raises stay natural — `3M` competitive, `4M`
/// preemptive. Written with `len` rather than `support` so the alerted cues
/// project (the opening major is known here).
fn uvu_major_responder(major: Suit) -> Rules {
    let m = Strain::from(major);
    let om = unbid_major(major).expect("a major opening has an unbid major");

    Rules::new()
        .rule(
            Bid::new(3, Strain::Clubs),
            200,
            len(major, 3..) & points(10..),
        )
        .alert(UVU_MAJOR_RAISE)
        .rule(
            Bid::new(3, Strain::Diamonds),
            190,
            len(om, 5..) & points(13..),
        )
        .alert(UVU_MAJOR_FOURTH)
        .rule(
            Bid::new(3, Strain::Notrump),
            150,
            points(13..) & stopper_in(Suit::Clubs) & stopper_in(Suit::Diamonds),
        )
        .rule(
            Call::Double,
            140,
            hcp(10..)
                & (len(Suit::Clubs, 4..)
                    | suit_hcp(Suit::Clubs, 4..)
                    | len(Suit::Diamonds, 4..)
                    | suit_hcp(Suit::Diamonds, 4..)),
        )
        .rule(Bid::new(3, m), 130, len(major, 3..) & points(6..=9))
        .rule(Bid::new(4, m), 125, len(major, 4..) & points(..=9))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder after our 1M and their Michaels cue of our own major (`1♥ (2♥)`
/// / `1♠ (2♠)` — 5+ in the other major and 5+ in an unknown minor)
///
/// The cue of their *known* suit (`2♠` over `1♥ (2♥)`, `3♥` over `1♠ (2♠)`)
/// is the limit-plus raise; `X` shows values (their runout has nowhere quiet
/// to land); the direct raises keep their natural meaning — the guard in
/// Section 4b always excluded their cue of our own major precisely because
/// `3M` here is a raise, not a cue-raise. `3♣`/`3♦` are natural weak escapes
/// (their minor is unknown, so both are biddable).
fn michaels_cue_responder(major: Suit) -> Rules {
    let m = Strain::from(major);
    let om_cue = if major == Suit::Hearts {
        Bid::new(2, Strain::Spades)
    } else {
        Bid::new(3, Strain::Hearts)
    };

    Rules::new()
        .rule(om_cue, 200, len(major, 3..) & points(10..))
        .alert(UVU_MAJOR_RAISE)
        .rule(Call::Double, 160, hcp(10..))
        .rule(Bid::new(3, m), 130, len(major, 3..) & points(6..=9))
        .rule(Bid::new(4, m), 125, len(major, 4..) & points(..=9))
        .rule(
            Bid::new(3, Strain::Clubs),
            110,
            len(Suit::Clubs, 6..) & points(2..=9),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            110,
            len(Suit::Diamonds, 6..) & points(2..=9),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer after `1M (2NT) 3♦ -` — partner's game force with
/// 5+ in the other major
///
/// Raise the shown major to game with 3+, else `3NT` with both minors
/// stopped, else rebid a 6-card opening major; the low-weight `3NT` is the
/// finite catch-all (the node is forced — partner's `3♦` is unbounded).
/// A slow forcing `3OM` probe is a deferral; opposite 13+ the blast is sound.
fn uvu_fourth_suit_answer(major: Suit) -> Rules {
    let m = Strain::from(major);
    let om = unbid_major(major).expect("a major opening has an unbid major");
    let om_strain = Strain::from(om);

    Rules::new()
        .rule(Bid::new(4, om_strain), 150, len(om, 3..))
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            stopper_in(Suit::Clubs) & stopper_in(Suit::Diamonds),
        )
        .rule(Bid::new(4, m), 100, len(major, 6..))
        .rule(Bid::new(3, Strain::Notrump), 20, hcp(0..))
}

/// Section 6 as a row package: their two-suiters over our `1M`
/// ([`set_uvu_over_majors`][super::set_uvu_over_majors])
///
/// Unusual-vs-unusual over their both-minors `2NT`, and a raise structure over
/// their Michaels cue of our own major.  Keyed at the deeper `1M (their call)`
/// tables — their cue and their `2NT` are single concrete calls — so these
/// shadow the `1M` direct-seat package (whose negative double misfires over a
/// Michaels cue) with no declaration-order race.  Both cue answers reuse the
/// shipped cue-raise table: its accept/decline shape is cue-agnostic.
pub(super) fn uvu_over_majors_package() -> Package {
    Package {
        name: "uvu-over-majors",
        gate: |agreements| agreements.build.competition.uvu_over_majors,
        entries: |_| {
            let mut entries = Vec::new();
            for major in [Suit::Hearts, Suit::Spades] {
                let trump = Strain::from(major);
                let unusual = format!("P* 1{trump} (2NT)");
                let michaels = format!("P* 1{trump} (2{trump})");
                let om_cue = if major == Suit::Hearts {
                    "2♠"
                } else {
                    "3♥"
                };

                // Their (2NT): responder, then opener's answers to the two cues.
                entries.extend(rows_of(
                    Pattern::table(&unusual),
                    uvu_major_responder(major),
                ));
                entries.extend(rows_of(
                    Pattern::after(&unusual, "3♣ -"),
                    answer_cue_raise(major),
                ));
                entries.extend(rows_of(
                    Pattern::after(&unusual, "3♦ -"),
                    uvu_fourth_suit_answer(major),
                ));

                // Their Michaels cue of our major: responder, then opener's
                // answer to the other-major cue.
                entries.extend(rows_of(
                    Pattern::table(&michaels),
                    michaels_cue_responder(major),
                ));
                entries.extend(rows_of(
                    Pattern::after(&michaels, &format!("{om_cue} -")),
                    answer_cue_raise(major),
                ));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
