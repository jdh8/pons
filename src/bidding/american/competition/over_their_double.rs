//! Over their takeout double — Jordan/Truscott `2NT`, and our doubled splinter
//!
//! [`set_jordan_truscott`] gives responder the artificial `2NT` limit raise and
//! the redouble structure above it ([`set_redouble_answer`]);
//! [`set_splinter_doubled`] rebases the whole systems-on tree when they double
//! our splinter.

use super::cue_raise::{answer_cue_minor_raise, answer_cue_raise};
use super::*;

thread_local! {
    /// Whether responder's structure over their takeout double of our 1-suit
    /// opening is authored: Jordan/Truscott `2NT`, the value redouble, the
    /// preemptive jump-raise flip, and weak non-forcing 2-level suits — with
    /// the shipped systems-on rebase surviving below it as the catch-all for
    /// every deeper continuation. **Default on** — the campaign's largest
    /// per-board win vs BBA 2/1 (204.8k boards/arm/vul): plain DD
    /// +0.0041/+0.0067 IMPs/board NV/vul, perfect-defense +0.0049/+0.0065;
    /// all four cells' CIs exclude 0 (+0.5…+0.8 IMPs/fired, ~0.8% fired).
    static JORDAN_TRUSCOTT: Cell<bool> = const { Cell::new(true) };
}

/// Author responder's structure over their takeout double for books built
/// *after* this call (thread-local)
///
/// **Default on** (`--no-ns-jordan-truscott` in `bba-gen` for the off arm).
pub fn set_jordan_truscott(on: bool) {
    JORDAN_TRUSCOTT.with(|cell| cell.set(on));
}

/// Whether the over-their-double package is engaged
pub(crate) fn jordan_truscott() -> bool {
    JORDAN_TRUSCOTT.with(Cell::get)
}

thread_local! {
    /// Whether opener's rebid over the value redouble (`1x – (X) – XX – (P)`)
    /// is authored.  **Default on** (fix-vs-shipped, 1M boards/vul, 24.pdd
    /// 16.3M–18.3M: plain DD +0.0056 ± 0.0005 NV / +0.0078 ± 0.0007 vul, PD
    /// +0.0058/+0.0080, ≈ +11..+14 IMPs per divergent board).  Off, the
    /// systems-on rebase strips both the double and the redouble, so opener
    /// replays onto the uncontested tree with responder's shown 10+ unseen,
    /// and the floor blasts stopperless 3NTs / thin games off shaped minimums
    /// — the point-count remnant's single worst per-board family
    /// (−16..−17 IMPs/board vulnerable).  See [`set_redouble_answer`].
    static REDOUBLE_ANSWER: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's rebid over the value redouble (`1x – (X) – XX – (P)`) for
/// books built *after* this call (thread-local); requires
/// [`set_jordan_truscott`] on (the redouble itself)
///
/// **Default on** (measured; see the thread-local above).  The authored node
/// is pass-only — a long-suit minimum sits for the redoubled make, and a 2M
/// escape rung measured −11 IMPs/fired before deletion.  `false` restores the
/// shipped floor for the off arm.
pub fn set_redouble_answer(on: bool) {
    REDOUBLE_ANSWER.with(|cell| cell.set(on));
}

/// Whether opener's answer over the value redouble is authored
fn redouble_answer() -> bool {
    REDOUBLE_ANSWER.with(Cell::get)
}

thread_local! {
    /// Whether a double of our splinter runs systems-on (see
    /// [`set_splinter_doubled`]).
    static SPLINTER_DOUBLED: Cell<bool> = const { Cell::new(true) };
}

/// Play systems-on over their double of our splinter for books built *after*
/// this call (thread-local)
///
/// A splinter (`1M – (P) – double-jump`) is game-forcing, but the double
/// reroutes opener's rebid to the competitive book, where — unauthored — it
/// fell to the floor and *passed*, leaving the game force doubled at the four
/// level (the anchor's Constructive/book/round-1 bucket #4 tail: our monster
/// opener passing a doubled `4♣` splinter while the field bids `7♠`). This
/// rebases the double back onto the undisturbed splinter continuation (4M
/// sign-off floor, RKCB with slam values). **Default on** — measured vs BBA
/// 2/1 (204.8k bd/arm/vul, SEED_BASE 1783439089): plain DD +0.0059/+0.0079
/// IMPs/board NV/vul, perfect-defense +0.0059/+0.0079, all four CIs exclude 0,
/// +15.4/+17.6 IMPs/fired (0.04% fired). Off-switch `--no-ns-splinter-doubled`.
pub fn set_splinter_doubled(on: bool) {
    SPLINTER_DOUBLED.with(|cell| cell.set(on));
}

/// Whether the doubled-splinter systems-on rebase is engaged
fn splinter_doubled() -> bool {
    SPLINTER_DOUBLED.with(Cell::get)
}

/// Section 11 as rows: responder's re-authored first call over their takeout
/// double, and opener's shadow answers (`set_jordan_truscott`)
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
/// * **The value redouble** (behind [`set_redouble_answer`]) would strip to
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
        gate: jordan_truscott,
        entries: || {
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
                    Pattern::after(&key, "2NT (P)"),
                    if is_major {
                        answer_cue_raise(o)
                    } else {
                        answer_cue_minor_raise(o)
                    },
                ));
                let preempt = Pattern::after(&key, &format!("3{o_strain} (P)"));
                if is_major {
                    entries
                        .push(row(preempt.clone(), Bid::new(4, o_strain), 90, points(17..)).into());
                } else {
                    entries
                        .push(row(preempt.clone(), Bid::new(5, o_strain), 90, points(19..)).into());
                }
                entries.push(row(preempt, Call::Pass, 0, hcp(0..)).into());
                if redouble_answer() {
                    entries
                        .push(row(Pattern::after(&key, "XX (P)"), Call::Pass, 60, hcp(0..)).into());
                }
                for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
                    let xs = Strain::from(x);
                    if xs >= o_strain {
                        continue;
                    }
                    let weak = Pattern::after(&key, &format!("2{xs} (P)"));
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
/// ([`set_splinter_doubled`][super::set_splinter_doubled])
///
/// A splinter is game-forcing, but the double reroutes opener into this book,
/// where — unauthored — it fell to the floor and passed out the doubled game
/// force.  The [`FirstIs`][super::super::fallback::FirstIs]`(Double)` rebase strips the double off the whole
/// subtree, so opener (and responder's keycard answers) resolve on the
/// undisturbed splinter continuation.
pub(super) fn splinter_doubled_package() -> Package {
    Package {
        name: "splinter-doubled",
        gate: splinter_doubled,
        entries: || {
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
                    let key = format!("P* 1{m_strain} (P) {}", Bid::new(level, strain));
                    entries.push(rebase(Pattern::first(&key, "X"), ReplaceNext(Call::Pass)));
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
