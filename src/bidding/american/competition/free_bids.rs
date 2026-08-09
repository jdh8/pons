//! Responder's free bid, and opener's answer to it
//!
//! [`FreeBidStyle`] picks the treatment: `Natural`, `Negative` (capped, with
//! its own continuation), or `Transfer` (responder bids the suit below).  The
//! floors are knobs — [`set_free_bid_floor`] for a suit, [`set_free_1nt_floor`]
//! for the free notrump, [`set_free_bid_quality`] for the suit-quality gate.

use super::negative_double::{NegativeDoubleShape, negative_doubler_rebid};
use super::over_overcall::two_level_slots;
use super::*;

/// The meaning of responder's non-jump 2-level new suit over their overcall
/// (`set_free_bid_style`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreeBidStyle {
    /// Forcing one round — the shipped default (the Fix 1 ruling: 1-level
    /// frees unconditionally forcing, 2-level forcing one round), answered by
    /// the Section-4d `answer_free_bid`.
    Forcing,
    /// Classic negative free bids: 2-level new suits are **non-forcing**,
    /// 5–11 points with a six-card suit or a strong five-carder (two of the
    /// top three honors); every stronger long-suit hand starts with the
    /// widened negative double, and double-then-new-suit is forcing to game.
    Negative,
    /// Cachalot-style 2-level transfers: when exactly two unbid suits sit at
    /// the two level the slots swap — each shows the other suit — and opener
    /// completes (declaring the concealed hand); the wrap slot completes a
    /// level higher. A lone (or three-way, over 1NT) 2-level slot stays
    /// natural-forcing.
    Transfer,
}

thread_local! {
    /// Whether responder's natural free bids over an overcall are authored
    /// (1-level new suit 5+ & 6+, 2-level non-jump 5+ & 10+, 1NT 6–10 / 2NT
    /// 11–12 with a stopper). Default off as a *direct* toggle, but the
    /// shipped `Modern` shape implies them (with opener's forcing answers) —
    /// the default system plays free bids.
    static FREE_BIDS: Cell<bool> = const { Cell::new(false) };

    /// Minimum points/HCP for the 1-level free *suit* bids (new-suit 5+, plus
    /// the Sputnik natural 4+ majors). Default 6 — the shipped floor. The vul-PD
    /// leak of the whole free-bid family lives here; sweep to 8+ and re-measure.
    /// The free 1NT has its own floor (`FREE_1NT_FLOOR`): a forcing suit bid
    /// finds a fit cheaply and is safe light, a limited non-forcing 1NT is not.
    static FREE_BID_FLOOR: Cell<u8> = const { Cell::new(6) };

    /// Minimum HCP for the free 1NT (`1X (1Y) 1NT`), decoupled from the suit
    /// floor above. Default 6 — byte-identical to the historical shared value.
    static FREE_1NT_FLOOR: Cell<u8> = const { Cell::new(6) };

    /// Whether the vulnerable free bids demand quality: a vulnerable 1-level
    /// new suit needs two of the top three honors, and the free 1NT is not
    /// authored vulnerable. The P3b′ floor sweep named the family's vulnerable
    /// leak as plain-DD-visible and strength-independent — a suit-quality
    /// gate, not a floor. Default off while the A/B runs.
    static FREE_BID_QUALITY: Cell<bool> = const { Cell::new(false) };

    /// The 2-level free-bid style — forcing (shipped default), classic
    /// negative free bids, or Cachalot-style transfers. The 1-level free
    /// bids stay forcing in every style.
    static FREE_BID_STYLE: Cell<FreeBidStyle> = const { Cell::new(FreeBidStyle::Forcing) };
}

/// Choose the 2-level free-bid style for books built *after* this call
/// (thread-local)
///
/// Default [`FreeBidStyle::Forcing`] (`--ns-free-bid-style` in `bba-gen` for
/// the other arms).
pub fn set_free_bid_style(style: FreeBidStyle) {
    FREE_BID_STYLE.with(|cell| cell.set(style));
}

/// The 2-level free-bid style in effect
pub(super) fn free_bid_style() -> FreeBidStyle {
    FREE_BID_STYLE.with(Cell::get)
}

/// Author responder's natural free bids over an overcall for books built
/// *after* this call (thread-local)
///
/// Default off (`--ns-free-bids` in `bba-gen` for the on arm).
pub fn set_free_bids(on: bool) {
    FREE_BIDS.with(|cell| cell.set(on));
}

/// Whether the free bids are authored *directly* (the raw knob)
pub(super) fn free_bids() -> bool {
    FREE_BIDS.with(Cell::get)
}

/// Whether the free bids are authored — directly, or implied by a
/// negative-double shape whose tighter double needs the natural outlet
pub(super) fn free_bids_engaged(agreements: &Agreements) -> bool {
    agreements.build.competition.free_bids
        || agreements.build.competition.negative_double_shape != NegativeDoubleShape::BothMajors
}

/// Set the minimum points/HCP for the 1-level free bids (thread-local)
///
/// Default 6 (`--ns-free-bid-floor` in `bba-gen`). Raising it trims the
/// vulnerable-PD leak the free-bid family inherits.
pub fn set_free_bid_floor(min: u8) {
    FREE_BID_FLOOR.with(|cell| cell.set(min));
}

/// The minimum points/HCP for the 1-level free bids
pub(super) fn free_bid_floor() -> u8 {
    FREE_BID_FLOOR.with(Cell::get)
}

/// Set the minimum HCP for the free 1NT (`1X (1Y) 1NT`), decoupled from the
/// suit floor (thread-local)
///
/// Default 6 (`--ns-free-1nt-floor` in `bba-gen`). The free 1NT is a limited,
/// non-forcing commitment to notrump values; raising this trims light 1NTs
/// without touching the forcing 1-level suit bids.
pub fn set_free_1nt_floor(min: u8) {
    FREE_1NT_FLOOR.with(|cell| cell.set(min));
}

/// The minimum HCP for the free 1NT
pub(super) fn free_1nt_floor() -> u8 {
    FREE_1NT_FLOOR.with(Cell::get)
}

/// Gate the vulnerable free bids on suit quality for books built *after* this
/// call (thread-local)
///
/// Default off (`--ns-free-bid-quality` in `bba-gen` for the on arm). When
/// on, a vulnerable 1-level free bid demands two of the top three honors in
/// the bid suit and the free 1NT is not authored vulnerable; non-vulnerable
/// rules and the 2-level/2NT free bids are unchanged.
pub fn set_free_bid_quality(on: bool) {
    FREE_BID_QUALITY.with(|cell| cell.set(on));
}

/// Whether the vulnerable free-bid quality gate is on
pub(super) fn free_bid_quality() -> bool {
    FREE_BID_QUALITY.with(Cell::get)
}

/// Opener's answer to responder's natural free bid — a new suit over their
/// overcall, **forcing one round** at both levels (the free-bid-quality A/B's
/// worst vulnerable boards were opener passing a game-going `2♦` out)
///
/// Raise partner's suit with 3-card support, bid notrump with a stopper in
/// their suit, show a natural second suit (reverses and 3-level suits need
/// 16+), else rebid the opening suit as the catch-all. No `Pass` rule — the
/// free bid forces by omission; the table is total via the rebid.
pub(super) fn answer_free_bid(opening: Suit, agreements: &Agreements) -> Rules {
    let o = opening;
    let o_strain = Strain::from(o);
    let mut rules = Rules::new();

    // Raise partner's freely bid suit with 3-card support (the free bid
    // promises five). `min_level_is` picks the cheapest legal raise. A raise
    // to *two* answers a 1-level free bid (the only auction whose cheapest
    // raise sits there), and Sputnik's natural 1-level majors promise only
    // four — raising on three would be a Moysian at the two level, so that
    // rung demands four; 2-level frees promise five in every school.
    let two_level_support: usize =
        if agreements.build.competition.negative_double_shape == NegativeDoubleShape::Sputnik {
            4
        } else {
            3
        };
    for y in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if y == o {
            continue;
        }
        let y_strain = Strain::from(y);
        for lvl in 2u8..=3 {
            let min_support = if lvl == 2 { two_level_support } else { 3 };
            rules = rules.rule(
                Bid::new(lvl, y_strain),
                150,
                partner_suit_is(y) & min_level_is(lvl, y_strain) & support(min_support..),
            );
        }
    }

    // Cheapest notrump with a stopper in their suit, minimum balanced range.
    for lvl in 1u8..=2 {
        rules = rules.rule(
            Bid::new(lvl, Strain::Notrump),
            120,
            min_level_is(lvl, Strain::Notrump) & stopper_in_their_suits() & hcp(12..=14),
        );
    }

    // A natural second suit: cheap non-reverse freely, a reverse or 3-level
    // suit shows 16+.
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x == o {
            continue;
        }
        let x_strain = Strain::from(x);
        for lvl in 1u8..=3 {
            let strong = lvl >= 3 || (lvl == 2 && x > o);
            let shape = min_level_is(lvl, x_strain)
                & !partner_suit_is(x)
                & !they_bid(x_strain)
                & len(x, 4..);
            rules = if strong {
                rules.rule(Bid::new(lvl, x_strain), 110, shape & hcp(16..))
            } else {
                rules.rule(Bid::new(lvl, x_strain), 110, shape)
            };
        }
    }

    // Catch-all: rebid the opening suit at the cheapest level (weakest action).
    for lvl in 2u8..=3 {
        rules = rules.rule(
            Bid::new(lvl, o_strain),
            0,
            min_level_is(lvl, o_strain) & hcp(0..),
        );
    }
    rules
}

/// Opener's answer to a *negative* (non-forcing) free bid — 5–11 with a
/// six-carder or a strong five-carder (`FreeBidStyle::Negative`)
///
/// `Pass` is the treatment's whole point: the catch-all drops the capped
/// hand at the two level (mirroring `answer_weak_new_suit`). Raising to
/// three needs a fit and real extras; `2NT` shows a stopper-backed maximum.
fn answer_negative_free_bid(opening: Suit) -> Rules {
    let mut rules = Rules::new();
    for y in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if y == opening {
            continue;
        }
        let ys = Strain::from(y);
        rules = rules.rule(
            Bid::new(3, ys),
            90,
            partner_suit_is(y) & min_level_is(3, ys) & len(y, 3..) & points(15..),
        );
    }
    rules
        .rule(
            Bid::new(2, Strain::Notrump),
            80,
            min_level_is(2, Strain::Notrump) & stopper_in_their_suits() & hcp(13..=14),
        )
        .rule(Call::Pass, 30, hcp(0..))
}

/// Opener's completion of a 2-level free-bid transfer (`FreeBidStyle::
/// Transfer`) — `shown` is responder's real suit, `comp_lvl` where the
/// completion sits (3 on the wrap slot)
///
/// The duty completion is non-forcing and puts opener on play (the
/// right-siding payoff); four trumps with extras super-accept. No notrump
/// option — declining the transfer into notrump re-sides the hand the
/// treatment exists to conceal.
fn free_transfer_completion(shown: Suit, comp_lvl: u8) -> Rules {
    let m = Strain::from(shown);
    Rules::new()
        .rule(
            Bid::new(comp_lvl + 1, m),
            130,
            len(shown, 4..) & points(15..),
        )
        .rule(Bid::new(comp_lvl, m), 120, hcp(0..))
}

/// Responder's clarification after opener completes the 2-level transfer:
/// `Pass` = the weak hand (the NFB equivalent), raise = invitational, the
/// cue of their suit = game force
fn free_transfer_clarify(shown: Suit, comp_lvl: u8, cue: Bid) -> Rules {
    let m = Strain::from(shown);
    Rules::new()
        .rule(cue, 110, points(13..))
        .rule(Bid::new(comp_lvl + 1, m), 100, points(10..=12))
        .rule(Call::Pass, 30, hcp(0..))
}

/// Section 4f as a row package: opener completes the 2-level free-bid transfer
/// and responder clarifies ([`FreeBidStyle::Transfer`] only)
///
/// The swap contexts are a closed enumeration — (opening, their overcall,
/// lower slot → shown, wrap slot → shown, completing a level higher on the
/// wrap).
pub(super) fn transfer_free_bid_package() -> Package {
    Package {
        name: "transfer-free-bid",
        gate: |agreements| {
            free_bids_engaged(agreements)
                && agreements.build.competition.free_bid_style == FreeBidStyle::Transfer
        },
        entries: |_| {
            #[allow(clippy::type_complexity)]
            #[rustfmt::skip]
            let swaps: [(Strain, u8, Strain, [(Strain, Suit); 2]); 7] = [
                (Strain::Clubs, 1, Strain::Spades, [(Strain::Diamonds, Suit::Hearts), (Strain::Hearts, Suit::Diamonds)]),
                (Strain::Clubs, 2, Strain::Diamonds, [(Strain::Hearts, Suit::Spades), (Strain::Spades, Suit::Hearts)]),
                (Strain::Diamonds, 1, Strain::Spades, [(Strain::Clubs, Suit::Hearts), (Strain::Hearts, Suit::Clubs)]),
                (Strain::Diamonds, 2, Strain::Clubs, [(Strain::Hearts, Suit::Spades), (Strain::Spades, Suit::Hearts)]),
                (Strain::Hearts, 1, Strain::Spades, [(Strain::Clubs, Suit::Diamonds), (Strain::Diamonds, Suit::Clubs)]),
                (Strain::Hearts, 2, Strain::Clubs, [(Strain::Diamonds, Suit::Spades), (Strain::Spades, Suit::Diamonds)]),
                (Strain::Spades, 2, Strain::Clubs, [(Strain::Diamonds, Suit::Hearts), (Strain::Hearts, Suit::Diamonds)]),
            ];
            let mut entries = Vec::new();
            for (o_strain, ovc_level, ovc_strain, slots) in swaps {
                let key = format!("P* 1{o_strain} ({ovc_level}{ovc_strain})");
                for (slot, shown) in slots {
                    let shown_strain = Strain::from(shown);
                    let comp_lvl = if shown_strain > slot { 2 } else { 3 };
                    let cue_lvl = comp_lvl + u8::from(ovc_strain < shown_strain);
                    let completion = format!("2{slot} -");
                    entries.extend(rows_of(
                        Pattern::after(&key, &completion),
                        free_transfer_completion(shown, comp_lvl),
                    ));
                    entries.extend(rows_of(
                        Pattern::after(&key, &format!("{completion} {comp_lvl}{shown_strain} -")),
                        free_transfer_clarify(shown, comp_lvl, Bid::new(cue_lvl, ovc_strain)),
                    ));
                }
            }
            entries
        },
    }
}

/// Section 4d as a row package: opener answers responder's natural free bid
/// (a non-jump new suit over their overcall ≤ `2♠`), forcing one round at both
/// levels — the free-bid-quality A/B's worst vulnerable-PD boards were opener
/// *passing* a game-going free bid out
///
/// The suffix guard mirrors the free-bid authoring in [`over_their_overcall`]:
/// overcall ≤ `2♠` and not a cue of our suit (4b/4c own the cue-raises), the
/// free bid a cheapest-level new suit that is neither their suit nor ours nor
/// notrump (the free `1NT`/`2NT` are non-forcing).  Cachalot rotates the
/// 1-level calls over its minor openings, so those stay with the Section-9
/// completions (whose deeper keys shadow this entry anyway — the `rotated`
/// conjunct is defense-in-depth and honest rendering); its natural 2-level
/// frees get the forcing answers like every other school's.  The 2-level
/// free-bid *style* carves this node further: [`FreeBidStyle::Negative`] sends
/// the level-2 frees to 4d′ (non-forcing answers, with `Pass`),
/// [`FreeBidStyle::Transfer`] sends the swapped slots to the Section-4f
/// completions (a lone or three-way slot stays natural-forcing and keeps the
/// 4d answers).
///
/// The `free.level < 3` ceiling makes the guard say what the convention says.
/// [`over_their_overcall`] authors free bids at exactly two rungs, each pinned
/// by `cheapest` to one *exact* level — 1 and 2 — so over a
/// two-level overcall every suit below it has no free bid at all and responder
/// never produces a three-level one.  Without the ceiling the guard claimed
/// those auctions anyway, and `answer_free_bid` has no legal rung there (its
/// raise, notrump and catch-all rungs all stop at 3, which a `3♦` free bid has
/// already passed) — an untotal guarded table over auctions that cannot happen.
/// **Verified inert**: no hand bids a three-level free suit.
///
/// Under Cachalot *and* [`FreeBidStyle::Negative`] the entry then admits
/// nothing — rotation claims level 1 and 4d′ claims level 2 — which is what it
/// already was in substance.
pub(super) fn free_bid_answer_package() -> Package {
    Package {
        name: "free-bid-answer",
        gate: free_bids_engaged,
        entries: |agreements| {
            let cachalot =
                agreements.build.competition.negative_double_shape == NegativeDoubleShape::Cachalot;
            let negative = agreements.build.competition.free_bid_style == FreeBidStyle::Negative;
            let transfer = agreements.build.competition.free_bid_style == FreeBidStyle::Transfer;
            let mut entries = Vec::new();
            for opening in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let o_strain = Strain::from(opening);
                let rotated = cachalot && matches!(opening, Suit::Clubs | Suit::Diamonds);
                let key = format!("P* 1{o_strain}");
                // Their overcall `(ix)`, partner's natural free bid `jy` —
                // the domain spells the free-bid grammar the guard used to
                // check: the cheapest rung only, below the 3 level, and not
                // a rung a style knob re-routes (Cachalot rotates the
                // 1 level away, Negative caps and re-answers the 2 level in
                // 4d′ below, a true Transfer pair swaps its slots).  Only a
                // minor opening leaves room for the 1-level rung — so under
                // Negative (which claims the whole 2 level) an opening with
                // no 1-level rung has no columns at all, which the guard
                // expressed by never firing.
                let one_level_room = matches!(opening, Suit::Clubs | Suit::Diamonds) && !rotated;
                if !negative || one_level_room {
                    entries.extend(expand(
                        &format!("{key} (ix) jy -"),
                        move |b| {
                            let ovc = b.bid('x');
                            let free = b.bid('y');
                            ovc <= Bid::new(2, Strain::Spades)
                                && ovc.strain != o_strain
                                && free.strain != o_strain
                                && free.level.get() < 3
                                && free.level.get()
                                    == ovc.level.get() + u8::from(free.strain < ovc.strain)
                                && !(rotated && free.level.get() == 1)
                                && !(negative && free.level.get() == 2)
                                && !(transfer
                                    && free.level.get() == 2
                                    && two_level_slots(o_strain, ovc) == 2)
                        },
                        move |_| answer_free_bid(opening, agreements),
                    ));
                }
                // The guard admitted their 1NT overcall too (1NT ≤ 2♠); every
                // free bid over it sits at the 2 level.
                if !negative {
                    entries.extend(expand(
                        &format!("{key} (1NT) 2y -"),
                        move |b| {
                            Strain::from(b.suit('y')) != o_strain
                                && !(transfer
                                    && two_level_slots(o_strain, Bid::new(1, Strain::Notrump)) == 2)
                        },
                        move |_| answer_free_bid(opening, agreements),
                    ));
                }

                if !negative {
                    continue;
                }

                // 4d′'s notrump column: the capped 2-level free over (1NT).
                entries.extend(expand(
                    &format!("{key} (1NT) 2y -"),
                    move |b| Strain::from(b.suit('y')) != o_strain,
                    move |_| answer_negative_free_bid(opening),
                ));

                // Section 4d′: the capped, non-forcing level-2 frees get
                // answers WITH a Pass catch-all.
                entries.extend(expand(
                    &format!("{key} (ix) 2y -"),
                    move |b| {
                        let ovc = b.bid('x');
                        let free = b.bid('y');
                        ovc <= Bid::new(2, Strain::Spades)
                            && ovc.strain != o_strain
                            && free.strain != o_strain
                            && free.level.get()
                                == ovc.level.get() + u8::from(free.strain < ovc.strain)
                    },
                    move |_| answer_negative_free_bid(opening),
                ));

                // Section 4d″: the doubler's rebid over opener's answer — a new
                // suit is the strong hand the capped free bid could not carry,
                // forcing to game.  This node also claims the ordinary
                // doubler's second turn (previously floored — bucket
                // X-then-Pass vs X-then-suit in the forensics).
                //
                // Stays guarded, with 4d‴ and the rebase carriers: opener's
                // answer is an unconstrained `Bid(_)` — a wildcard, which is
                // the guard verbs' native shape.  Enumerating it costs 640
                // exact nodes (every legal answer above every in-scope
                // overcall, four openings), nearly all of them columns no
                // auction reaches and every one of them a row on the card.
                let over = if opening == Suit::Spades {
                    "(2♥)"
                } else {
                    "(2♠)"
                };
                entries.extend(rows_of(
                    Pattern::guarded(
                        &key,
                        &format!("{over} X - 3♣ -"),
                        described_guard(
                            "(overcall ≤2♠) X - answer -",
                            guard(move |_: &Context<'_>, suffix: &[Call]| {
                                matches!(
                                    suffix,
                                    [
                                        Call::Bid(ovc),
                                        Call::Double,
                                        Call::Pass,
                                        Call::Bid(_),
                                        Call::Pass
                                    ] if *ovc <= Bid::new(2, Strain::Spades)
                                        && ovc.strain != o_strain
                                )
                            }),
                        ),
                    ),
                    negative_doubler_rebid(opening),
                ));

                // Section 4d‴: opener answers the game-forcing rebid with the
                // ordinary forcing-answer table; the guard's `< 3 of the
                // opening suit` scope keeps that table's catch-all legal.
                let fg_sample = match opening {
                    Suit::Clubs => "(1♥) X - 1♠ - 2♦ -",
                    Suit::Diamonds => "(1♥) X - 1♠ - 2♣ -",
                    Suit::Hearts => "(1♠) X - 2♣ - 2♦ -",
                    Suit::Spades => "(1♥) X - 2♣ - 2♦ -",
                };
                entries.extend(rows_of(
                    Pattern::guarded(
                        &key,
                        fg_sample,
                        described_guard(
                            "(overcall ≤2♠) X - answer - FG-suit -",
                            guard(move |_: &Context<'_>, suffix: &[Call]| {
                                matches!(
                                    suffix,
                                    [
                                        Call::Bid(ovc),
                                        Call::Double,
                                        Call::Pass,
                                        Call::Bid(ans),
                                        Call::Pass,
                                        Call::Bid(new),
                                        Call::Pass
                                    ] if *ovc <= Bid::new(2, Strain::Spades)
                                        && ovc.strain != o_strain
                                        && new.strain != Strain::Notrump
                                        && new.strain != ovc.strain
                                        && new.strain != o_strain
                                        && new.strain != ans.strain
                                        && *new < Bid::new(3, o_strain)
                                )
                            }),
                        ),
                    ),
                    answer_free_bid(opening, agreements),
                ));
            }
            entries
        },
    }
}

#[cfg(test)]
mod legacy;
#[cfg(test)]
pub(super) use legacy::free_bid_answer_package_legacy;

#[cfg(test)]
mod tests;
