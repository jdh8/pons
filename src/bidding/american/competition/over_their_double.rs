//! Over their takeout double — Jordan/Truscott `2NT`, and our doubled splinter
//!
//! `agreements.competition.jordan_truscott` gives responder the artificial `2NT` limit raise and
//! the redouble structure above it (`agreements.competition.redouble_answer`);
//! `agreements.competition.splinter_doubled` rebases the whole systems-on tree when they double
//! our splinter.

use super::cue_raise::{answer_cue_minor_raise, answer_cue_raise};
use super::*;

/// Section 11 as rows: responder's re-authored first call over their takeout
/// double, and opener's shadow answers (`agreements.competition.jordan_truscott`)
///
/// Over a double the meanings genuinely change, so responder's whole first
/// call is re-authored — a total table at the deeper `1x (X)` key; every
/// *deeper* continuation still rides the shipped systems-on rebase below it.
/// Jordan/Truscott `2NT` = limit+ raise (4+ support majors, 5+ minors); `XX`
/// = 10+ without that fit; the jump raise **flips preemptive**; 1-level suits
/// stay forcing-as-uncontested (their continuations rebase onto the
/// uncontested tree); 2-level new suits are weak and non-forcing (2/1 is off
/// over the double); `1NT` natural 6–9.
///
/// Opener's `after` tables shadow exactly the rebase misreads:
///
/// * **Jordan 2NT** would replay as Jacoby — answered by the shared
///   cue-raise tables ([`answer_cue_raise`], [`answer_cue_minor_raise`]).
/// * **The preemptive `3x`** would replay as the uncontested limit raise —
///   game only with genuine extras, else pass the preempt out.
/// * **The weak `2y`** would replay as a 2/1 game force — raise with a fit
///   and real extras, else pass.
/// * **The value redouble** (behind `agreements.competition.redouble_answer`) would strip to
///   an uncontested rebid with responder's 10+ unseen — the floor then
///   re-prices a shaped minimum as game-going and blasts a stopperless 3NT.
///   Sound bridge is **pass**, full stop: even (especially) a long-suit
///   minimum — one-of-a-suit redoubled with six-plus trumps makes with
///   overtricks, while any pull forfeits the redoubled bonus and reopens the
///   auction for their runout (a 2M-escape rung measured −11 IMPs/fired in
///   the smoke A/B before it was deleted).  Extras act naturally on the next
///   round once they run.
pub(super) fn jordan_truscott_package() -> Package {
    Package {
        name: "P4:jordan-truscott",
        gate: |agreements| agreements.competition.jordan_truscott,
        entries: |agreements| {
            let mut entries = Vec::new();
            for o in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let o_strain = Strain::from(o);
                let is_major = matches!(o, Suit::Hearts | Suit::Spades);
                let jordan_min: usize = if is_major { 4 } else { 5 };
                let raise_min: usize = if is_major { 3 } else { 5 };
                let xx_max: usize = if is_major { 3 } else { 4 };
                let key = format!("P* 1{o_strain} (X)");
                let responder = || Pattern::table(&key);

                entries.push(
                    row(
                        responder(),
                        Bid::new(2, Strain::Notrump),
                        200,
                        len(o, jordan_min..) & points(10..),
                    )
                    .alert(JORDAN)
                    .into(),
                );
                entries.push(
                    row(
                        responder(),
                        Call::Redouble,
                        160,
                        hcp(10..) & len(o, ..=xx_max),
                    )
                    .alert(VALUE_REDOUBLE)
                    .into(),
                );
                entries.push(
                    row(
                        responder(),
                        Bid::new(3, o_strain),
                        150,
                        len(o, jordan_min..) & points(..=9),
                    )
                    .into(),
                );
                entries.push(
                    row(
                        responder(),
                        Bid::new(2, o_strain),
                        140,
                        len(o, raise_min..) & points(6..=9),
                    )
                    .into(),
                );
                for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                    if x == o {
                        continue;
                    }
                    let xs = Strain::from(x);
                    entries.push(
                        row(
                            responder(),
                            Bid::new(1, xs),
                            130,
                            min_level_is(1, xs) & len(x, 4..) & points(6..),
                        )
                        .into(),
                    );
                    entries.push(
                        row(
                            responder(),
                            Bid::new(2, xs),
                            120,
                            min_level_is(2, xs) & len(x, 5..) & points(6..=9),
                        )
                        .into(),
                    );
                }
                entries
                    .push(row(responder(), Bid::new(1, Strain::Notrump), 110, hcp(6..=9)).into());
                entries.push(row(responder(), Call::Pass, 0, hcp(0..)).into());

                entries.extend(rows_of(
                    Pattern::after(&key, "2NT -"),
                    if is_major {
                        answer_cue_raise(o)
                    } else {
                        answer_cue_minor_raise(o)
                    },
                ));
                let preempt = Pattern::after(&key, &format!("3{o_strain} -"));
                if is_major {
                    entries
                        .push(row(preempt.clone(), Bid::new(4, o_strain), 90, points(17..)).into());
                } else {
                    entries
                        .push(row(preempt.clone(), Bid::new(5, o_strain), 90, points(19..)).into());
                }
                entries.push(row(preempt, Call::Pass, 0, hcp(0..)).into());
                if agreements.competition.redouble_answer {
                    entries
                        .push(row(Pattern::after(&key, "XX -"), Call::Pass, 60, hcp(0..)).into());
                }
                for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
                    let xs = Strain::from(x);
                    if xs >= o_strain {
                        continue;
                    }
                    let weak = Pattern::after(&key, &format!("2{xs} -"));
                    entries.push(
                        row(
                            weak.clone(),
                            Bid::new(3, xs),
                            90,
                            len(x, 4..) & points(15..),
                        )
                        .into(),
                    );
                    entries.push(row(weak, Call::Pass, 30, hcp(0..)).into());
                }
            }
            entries
        },
    }
}

/// Section 2b as a row package: systems-on over their double of our splinter
/// (`agreements.competition.splinter_doubled`)
///
/// A splinter is game-forcing, but the double reroutes opener into this book,
/// where — unauthored — it fell to the floor and passed out the doubled game
/// force.  The [`FirstIs`][super::super::fallback::FirstIs]`(Double)` rebase strips the double off the whole
/// subtree, so opener (and responder's keycard answers) resolve on the
/// undisturbed splinter continuation.
pub(super) fn splinter_doubled_package() -> Package {
    Package {
        name: "splinter-doubled",
        gate: |agreements| agreements.competition.splinter_doubled,
        entries: |_| {
            let mut entries = Vec::new();
            for major in [Suit::Hearts, Suit::Spades] {
                let m_strain = Strain::from(major);
                let splinter_suits: &[Suit] = if major == Suit::Hearts {
                    &[Suit::Spades, Suit::Clubs, Suit::Diamonds]
                } else {
                    &[Suit::Clubs, Suit::Diamonds, Suit::Hearts]
                };
                for &x in splinter_suits {
                    let (level, strain) = super::super::responses::splinter_bid(major, x);
                    let key = format!("P* 1{m_strain} - {}", Bid::new(level, strain));
                    entries.push(rebase(Pattern::first(&key, "X"), ReplaceNext(Call::Pass)));
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
