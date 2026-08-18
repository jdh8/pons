//! Their three-level overcall of our `1NT`
//!
//! The census's top loser (`docs/one-notrump-competitive.md` §N3): nothing in
//! the book keys `1NT (3x)`, so responder's new suit is a floor call that reads
//! as *nothing* to opener.  BBA's three-level calls over a 1NT opening are
//! **natural seven-card preempts** (`hcp 4–10`,
//! `docs/ai-bidder/bba-1nt-counter-defense.md` §`(3♣)`–`(3♠)`), so what this
//! lane wants is an ordinary competitive scheme, not a counter-defense.
//!
//! Opt-in via `agreements.competition.nt_high_overcall_responses`; the `(3♣)`
//! transfer variant rides `agreements.competition.nt_3c_transfers` on top.
//!
//! Sibling of [`super::high_overcall`], which covers the same overcall level
//! over our *suit* openings.  It keys `1x`, never `1NT`, so the two never race.

use super::lebensohl::author_direct_3nt;
use super::*;

/// Responder's direct `3NT` over their three-level overcall
///
/// [`author_direct_3nt`] with the *three-level* stopper bit substituted for the
/// shared one: the same paired arm that wants no gate here wants it kept on the
/// lane the shared bit also governs (see
/// [`nt_high_overcall_3nt_stopper`][crate::bidding::agreements::CompetitionKnobs::nt_high_overcall_3nt_stopper]).
fn nt_direct_3nt(rules: Rules, weight: i16, over: Suit, agreements: &Agreements) -> Rules {
    let mut local = *agreements;
    local.competition.direct_3nt_stopper = agreements.competition.nt_high_overcall_3nt_stopper;
    author_direct_3nt(rules, weight, over, &local)
}

/// Responder after our `1NT` and their natural three-level overcall in `over`
///
/// One round, no room.  A five-card suit *above* theirs forces at the three
/// level; a six-card major with no three-level slot plays game; `X` is the 4-4
/// major finder; `3NT` plays.  Strength floors are the Lebensohl lane's —
/// opposite a 15–17 notrump `points(10..)` is game, and the double's
/// `points(8..)` is the census repair (the floor doubles on 6–7 and opener
/// drives to a bad game).
///
/// A five-card *minor* below theirs is the one call priced under `3NT`: it can
/// only be bid at the four level, past the game we most want, so it is what a
/// hand with no stopper and no major falls back on — never a reason to bypass
/// `3NT`.
fn nt_over_high_overcall(over: Suit, agreements: &Agreements) -> Rules {
    let mut rules = Rules::new();

    // Natural game force in the *longest* five-card suit that clears their
    // overcall — `at_least_as_long` keeps a 6-5 from bidding its five-carder,
    // and the rank-ordered weights break the 5-5 tie upward, which leaves the
    // lower suit biddable as a correction.  The arms differ only in how many
    // rivals there are to out-length; each is spelled out because the
    // `&`-chains have distinct types.
    rules = match over {
        Suit::Clubs => {
            let mut rules = rules;
            for (y, weight, a, b) in [
                (Suit::Diamonds, 180, Suit::Hearts, Suit::Spades),
                (Suit::Hearts, 181, Suit::Diamonds, Suit::Spades),
                (Suit::Spades, 182, Suit::Diamonds, Suit::Hearts),
            ] {
                rules = rules.rule(
                    Bid::new(3, Strain::from(y)),
                    weight,
                    len(y, 5..) & points(10..) & at_least_as_long(y, a) & at_least_as_long(y, b),
                );
            }
            rules
        }
        Suit::Diamonds => {
            let mut rules = rules;
            for (y, weight, a) in [
                (Suit::Hearts, 181, Suit::Spades),
                (Suit::Spades, 182, Suit::Hearts),
            ] {
                rules = rules.rule(
                    Bid::new(3, Strain::from(y)),
                    weight,
                    len(y, 5..) & points(10..) & at_least_as_long(y, a),
                );
            }
            rules
        }
        // Only spades clears `(3♥)`, and nothing clears `(3♠)` — no rival to
        // out-length either way.
        Suit::Hearts => rules.rule(
            Bid::new(3, Strain::Spades),
            182,
            len(Suit::Spades, 5..) & points(10..),
        ),
        Suit::Spades => rules,
    };

    // A six-card major to play at game.  Above their suit it is the weak
    // twin of the forcing three-level bid; below it (hearts over `(3♠)`) it is
    // the *only* natural call, so it also carries the strong hands that have
    // no three-level slot left.  Spades outrank hearts on the 6-6.
    for (major, weight) in [(Suit::Hearts, 160), (Suit::Spades, 161)] {
        if major == over {
            continue;
        }
        let bid = Bid::new(4, Strain::from(major));
        rules = if major > over {
            rules.rule(bid, weight, len(major, 6..) & points(6..=9))
        } else {
            rules.rule(
                bid,
                weight,
                (len(major, 6..) & points(6..)) | (len(major, 5..) & points(9..)),
            )
        };
    }

    // Takeout double: at least one four-card major, values.  Alerted like every
    // other negative double — the length and the floor are the whole message.
    rules = match over {
        Suit::Hearts => rules.rule(Call::Double, 150, len(Suit::Spades, 4..) & points(8..)),
        Suit::Spades => rules.rule(Call::Double, 150, len(Suit::Hearts, 4..) & points(8..)),
        _ => rules.rule(
            Call::Double,
            150,
            (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..)) & points(8..),
        ),
    }
    .alert(NEGATIVE_DOUBLE);

    // Direct 3NT to play, riding this lane's stopper bit and the shared
    // trap-pass toggle.
    rules = nt_direct_3nt(rules, 140, over, agreements);

    // Natural forcing five-card minor below their suit — the four-level
    // fallback for a game hand with no stopper and no major.
    rules = match over {
        // Both minors are below `(3♥)`/`(3♠)`, so they compete: the longer
        // one, the higher on a tie.
        Suit::Hearts | Suit::Spades => rules
            .rule(
                Bid::new(4, Strain::Clubs),
                120,
                len(Suit::Clubs, 5..)
                    & points(10..)
                    & at_least_as_long(Suit::Clubs, Suit::Diamonds),
            )
            .rule(
                Bid::new(4, Strain::Diamonds),
                121,
                len(Suit::Diamonds, 5..)
                    & points(10..)
                    & at_least_as_long(Suit::Diamonds, Suit::Clubs),
            ),
        // Only clubs is below `(3♦)`; nothing is below `(3♣)`.
        Suit::Diamonds => rules.rule(
            Bid::new(4, Strain::Clubs),
            120,
            len(Suit::Clubs, 5..) & points(10..),
        ),
        Suit::Clubs => rules,
    };

    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Responder over `1NT (3♣)` with the transfer variant engaged
///
/// `agreements.competition.nt_3c_transfers` — the minority expert treatment of
/// "systems on over `3♣`", the one three-level overcall that leaves three steps
/// below `3NT`.  `3♦`/`3♥` transfer to the majors (INV+, so the completion is
/// driven to game — the displaced-bid-is-GF simplification), `3♠` transfers to
/// diamonds.  BBA plays all three naturally, so this arm wins or loses on its
/// own merit; what it buys is the invitational five-card major (today `X` or a
/// pass) and right-siding.  Every other rung is [`nt_over_high_overcall`]'s.
fn nt_3c_transfer_responder(agreements: &Agreements) -> Rules {
    let mut rules = Rules::new();
    // The longer major, the higher on a tie — as in the natural table.
    for (bid, target, weight, rival) in [
        (
            Bid::new(3, Strain::Diamonds),
            Suit::Hearts,
            180,
            Suit::Spades,
        ),
        (Bid::new(3, Strain::Hearts), Suit::Spades, 181, Suit::Hearts),
    ] {
        rules = rules
            .rule(
                bid,
                weight,
                len(target, 5..) & points(9..) & at_least_as_long(target, rival),
            )
            .alert(LEBENSOHL_TRANSFER);
    }
    // The diamond transfer sits below 3NT's weight: a long minor with a club
    // stopper still plays notrump.  (`rubensohl`'s minor targets are priced
    // the same way.)
    rules = rules
        .rule(
            Bid::new(3, Strain::Spades),
            145,
            len(Suit::Diamonds, 5..) & points(10..),
        )
        .alert(LEBENSOHL_TRANSFER);

    for (major, weight) in [(Suit::Hearts, 160), (Suit::Spades, 161)] {
        rules = rules.rule(
            Bid::new(4, Strain::from(major)),
            weight,
            len(major, 6..) & points(6..=9),
        );
    }
    rules = rules
        .rule(
            Call::Double,
            150,
            (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..)) & points(8..),
        )
        .alert(NEGATIVE_DOUBLE);
    rules = nt_direct_3nt(rules, 140, Suit::Clubs, agreements);
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Opener's forced answer to responder's game-forcing three-level suit
///
/// A major is raised to game on three-card support, else `3NT`.  A minor
/// (only `3♦` over their `3♣`) looks for `3NT` first — that is the contract the
/// preempt was trying to steal — then a four-card major, then the minor raise.
/// Both arms end in a finite catch-all: a force must be answered.
fn nt_answer_forcing_suit(over: Suit, suit: Suit) -> Rules {
    let strain = Strain::from(suit);
    let game = Bid::new(4, strain);
    let notrump = Bid::new(3, Strain::Notrump);
    if matches!(suit, Suit::Hearts | Suit::Spades) {
        return Rules::new()
            .rule(game, 150, len(suit, 3..))
            .rule(notrump, 140, stopper_in_their_suits())
            .rule(game, 100, hcp(0..));
    }
    let mut rules = Rules::new().rule(notrump, 150, stopper_in_their_suits());
    for major in [Suit::Hearts, Suit::Spades] {
        if major != over {
            rules = rules.rule(Bid::new(3, Strain::from(major)), 140, len(major, 4..));
        }
    }
    rules
        .rule(game, 130, len(suit, 3..))
        .rule(notrump, 100, hcp(0..))
}

/// Opener's forced answer to responder's game-forcing four-level minor
///
/// `3NT` is already gone, so the choice is the minor game on an eight-card fit
/// or a five-card major.  Responder denied both a five-card suit above theirs
/// and a four-card major (the double outranks this call), so the major is at
/// best a 5-3 — the eight-card minor fit wins the tie.
fn nt_answer_forcing_minor(over: Suit, minor: Suit) -> Rules {
    let game = Bid::new(5, Strain::from(minor));
    let mut rules = Rules::new().rule(game, 140, len(minor, 3..));
    for major in [Suit::Hearts, Suit::Spades] {
        if major != over {
            rules = rules.rule(Bid::new(4, Strain::from(major)), 130, len(major, 5..));
        }
    }
    rules.rule(game, 100, hcp(0..))
}

/// Opener's forced answer to responder's takeout double
///
/// `high_overcall::answer_high_neg_double`'s shape, re-floored for a
/// 15–17 notrump: the shown major at its cheapest level with four, jumped to
/// game with a maximum, then `3NT` on a stopper, then the three-card tolerance,
/// with a low-weight `3NT` as the finite catch-all.
fn nt_answer_double(over: Suit) -> Rules {
    let notrump = Bid::new(3, Strain::Notrump);
    let mut rules = Rules::new();
    for major in [Suit::Hearts, Suit::Spades] {
        if major == over {
            continue;
        }
        let strain = Strain::from(major);
        rules = rules.rule(Bid::new(4, strain), 150, len(major, 4..) & points(17..));
        if major > over {
            rules = rules.rule(Bid::new(3, strain), 140, len(major, 4..));
        }
    }
    rules = rules.rule(notrump, 130, stopper_in_their_suits());
    for major in [Suit::Hearts, Suit::Spades] {
        if major == over {
            continue;
        }
        let strain = Strain::from(major);
        if major > over {
            rules = rules.rule(Bid::new(3, strain), 30, len(major, 3..));
        }
        rules = rules.rule(Bid::new(4, strain), 25, len(major, 3..));
    }
    rules.rule(notrump, 15, hcp(0..))
}

/// Opener completes the `(3♣)`-lane transfer to diamonds
///
/// `rubensohl::transfer_completion`'s minor arm cannot be reused: responder's
/// transfer is `3♠`, so the completion is `4♦` and its `3♦` is illegal — and
/// its `Pass` catch-all would leave us playing `3♠` on a phantom suit.  `3NT`
/// is both the first choice and the finite catch-all instead.
fn nt_3c_diamond_completion(agreements: &Agreements) -> Rules {
    let alerts = agreements.decision.reading.completion_alerts;
    let notrump = Bid::new(3, Strain::Notrump);
    Rules::new()
        .rule(notrump, 150, stopper_in_their_suits())
        .alert_if(alerts, COMPLETION)
        .rule(Bid::new(4, Strain::Diamonds), 130, len(Suit::Diamonds, 3..))
        .alert_if(alerts, COMPLETION)
        .rule(notrump, 100, hcp(0..))
}

/// Opener completes a `(3♣)`-lane transfer the advancer has raised to `4♣`
///
/// The raise takes every step: complete at the four level with tolerance, else
/// double for values.  Responder is invitational-plus, so the floor drives on
/// from there.
fn nt_3c_transfer_squeezed(target: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(target)), 150, len(target, 3..))
        .rule(Call::Double, 100, hcp(0..))
}

/// Section 12 as a row package: their three-level overcall of our `1NT`
/// (`agreements.competition.nt_high_overcall_responses`, default off)
///
/// Responder's one call and opener's one answer to each of them; every
/// interfered tail past the `(3♣)` transfers, and every third call, is the
/// floor's — the lane's book/floor line.
pub(super) fn nt_high_overcall_package() -> Package {
    Package {
        name: "nt-high-overcall-responses",
        gate: |agreements| agreements.competition.nt_high_overcall_responses,
        entries: |agreements| {
            let transfers = agreements.competition.nt_3c_transfers;
            let natural =
                move |bindings: &Bindings| !(transfers && bindings.suit('x') == Suit::Clubs);
            let mut entries = Vec::new();

            entries.extend(expand(
                "P* 1NT (3x)",
                |_| true,
                move |bindings| {
                    let over = bindings.suit('x');
                    if transfers && over == Suit::Clubs {
                        nt_3c_transfer_responder(agreements)
                    } else {
                        nt_over_high_overcall(over, agreements)
                    }
                },
            ));

            // Opener answers the forcing three-level suit.  With the transfers
            // on, their `(3♣)` instance is the completion block below instead.
            entries.extend(expand("P* 1NT (3x) 3y -", natural, |bindings| {
                nt_answer_forcing_suit(bindings.suit('x'), bindings.suit('y'))
            }));

            // ...the forcing four-level minor (only below their suit; the
            // minor above theirs was biddable at the three level).
            entries.extend(expand(
                "P* 1NT (3x) 4m -",
                |bindings| bindings.suit('m') < bindings.suit('x'),
                |bindings| nt_answer_forcing_minor(bindings.suit('x'), bindings.suit('m')),
            ));

            // ...and the takeout double.
            entries.extend(expand(
                "P* 1NT (3x) X -",
                |_| true,
                |bindings| nt_answer_double(bindings.suit('x')),
            ));

            if transfers {
                for (bid, target) in [
                    (Bid::new(3, Strain::Diamonds), Suit::Hearts),
                    (Bid::new(3, Strain::Hearts), Suit::Spades),
                    (Bid::new(3, Strain::Spades), Suit::Diamonds),
                ] {
                    // Their double of the transfer steals no room, so the
                    // completion is the undoubled one verbatim.
                    for their in ["-", "(X)"] {
                        entries.extend(rows_of(
                            Pattern::node(&format!("P* 1NT (3♣) {bid} {their}")),
                            if target == Suit::Diamonds {
                                nt_3c_diamond_completion(agreements)
                            } else {
                                transfer_completion(target, Suit::Clubs, agreements)
                            },
                        ));
                    }
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* 1NT (3♣) {bid} (4♣)")),
                        nt_3c_transfer_squeezed(target),
                    ));
                }
            }

            entries
        },
    }
}

#[cfg(test)]
mod tests;
